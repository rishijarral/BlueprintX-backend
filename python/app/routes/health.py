"""Health check endpoint."""

from fastapi import APIRouter
from pydantic import BaseModel

from app.cache import get_redis_cache

router = APIRouter()


class HealthResponse(BaseModel):
    """Health check response."""

    status: str = "ok"
    service: str = "blueprintx-ai"
    version: str = "0.1.0"
    redis: str = "not_configured"


@router.get("/health", response_model=HealthResponse)
async def health_check() -> HealthResponse:
    """
    Health check endpoint.

    Returns service status. Does not require authentication.
    Used by load balancers and container orchestrators.
    """
    # Check Redis connectivity
    redis_status = "not_configured"
    cache = get_redis_cache()
    if cache:
        if await cache.ping():
            redis_status = "ok"
        else:
            redis_status = "error"

    # Determine overall status
    overall_status = "ok"
    if redis_status == "error":
        overall_status = "degraded"

    return HealthResponse(
        status=overall_status,
        redis=redis_status,
    )
