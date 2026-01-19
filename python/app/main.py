"""BlueprintX AI Service - FastAPI Application."""

import uuid
from contextlib import asynccontextmanager
from typing import AsyncGenerator

from fastapi import FastAPI, HTTPException, Request
from fastapi.middleware.cors import CORSMiddleware

from app.cache.redis import close_redis_cache, init_redis_cache
from app.config import get_settings
from app.errors import (
    APIError,
    api_error_handler,
    http_exception_handler,
    unhandled_exception_handler,
)
from app.logging import get_logger, set_request_id, setup_logging
from app.routes import documents, health, jobs, plan, qna, tenders

# Initialize logging
setup_logging()
logger = get_logger(__name__)


@asynccontextmanager
async def lifespan(app: FastAPI) -> AsyncGenerator[None, None]:
    """Application lifespan handler."""
    settings = get_settings()
    logger.info(
        "Starting BlueprintX AI Service",
        env=settings.env,
        host=settings.server_host,
        port=settings.server_port,
    )

    # Initialize Redis cache
    cache = await init_redis_cache()
    if cache:
        logger.info("Redis cache initialized")
    else:
        logger.warning("Redis cache not available - running without caching")

    yield

    # Cleanup
    await close_redis_cache()
    logger.info("Shutting down BlueprintX AI Service")


# Create FastAPI application
app = FastAPI(
    title="BlueprintX AI Service",
    description="Internal LLM service for plan analysis and document processing",
    version="0.1.0",
    docs_url="/docs" if get_settings().env != "prod" else None,
    redoc_url="/redoc" if get_settings().env != "prod" else None,
    lifespan=lifespan,
)


# Register exception handlers
# Note: type: ignore needed due to FastAPI's overly strict ExceptionHandler typing
app.add_exception_handler(APIError, api_error_handler)  # type: ignore[arg-type]
app.add_exception_handler(HTTPException, http_exception_handler)  # type: ignore[arg-type]
app.add_exception_handler(Exception, unhandled_exception_handler)  # type: ignore[arg-type]


# CORS middleware (internal service, but useful for debugging)
if get_settings().env == "dev":
    app.add_middleware(
        CORSMiddleware,
        allow_origins=["*"],
        allow_credentials=True,
        allow_methods=["*"],
        allow_headers=["*"],
    )


@app.middleware("http")
async def request_id_middleware(request: Request, call_next):
    """Extract or generate request ID and propagate it."""
    # Get request ID from header (set by Rust) or generate new one
    request_id = request.headers.get("x-request-id")
    if not request_id:
        request_id = str(uuid.uuid4())

    # Set in context for logging
    set_request_id(request_id)

    # Process request
    response = await call_next(request)

    # Add request ID to response headers
    response.headers["x-request-id"] = request_id

    return response


@app.middleware("http")
async def logging_middleware(request: Request, call_next):
    """Log requests and responses."""
    logger.info(
        "Request started",
        method=request.method,
        path=request.url.path,
        client=request.client.host if request.client else None,
    )

    response = await call_next(request)

    logger.info(
        "Request completed",
        method=request.method,
        path=request.url.path,
        status_code=response.status_code,
    )

    return response


# Register routers
app.include_router(health.router)
app.include_router(plan.router, prefix="/v1/plan", tags=["Plan Analysis"])
app.include_router(tenders.router, prefix="/v1/tenders", tags=["Tenders"])
app.include_router(qna.router, prefix="/v1", tags=["Q&A"])
app.include_router(jobs.router, prefix="/v1/jobs", tags=["Jobs"])
app.include_router(documents.router, prefix="/v1/documents", tags=["Documents"])


if __name__ == "__main__":
    import uvicorn

    settings = get_settings()
    uvicorn.run(
        "app.main:app",
        host=settings.server_host,
        port=settings.server_port,
        reload=settings.env == "dev",
    )
