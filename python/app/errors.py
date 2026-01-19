"""Unified error handling with consistent JSON responses."""

from typing import Any

from fastapi import HTTPException, Request
from fastapi.responses import JSONResponse
from pydantic import BaseModel

from app.logging import get_logger, get_request_id

logger = get_logger(__name__)


class ErrorResponse(BaseModel):
    """Standard error response format matching Rust backend."""

    code: str
    message: str
    request_id: str | None = None


class APIError(Exception):
    """Base API error with status code and error code."""

    def __init__(
        self,
        status_code: int,
        code: str,
        message: str,
        details: dict[str, Any] | None = None,
    ) -> None:
        self.status_code = status_code
        self.code = code
        self.message = message
        self.details = details or {}
        super().__init__(message)


class BadRequestError(APIError):
    """400 Bad Request."""

    def __init__(self, message: str, details: dict[str, Any] | None = None) -> None:
        super().__init__(400, "BAD_REQUEST", message, details)


class UnauthorizedError(APIError):
    """401 Unauthorized."""

    def __init__(self, message: str = "Unauthorized") -> None:
        super().__init__(401, "UNAUTHORIZED", message)


class ForbiddenError(APIError):
    """403 Forbidden."""

    def __init__(self, message: str = "Forbidden") -> None:
        super().__init__(403, "FORBIDDEN", message)


class NotFoundError(APIError):
    """404 Not Found."""

    def __init__(self, message: str = "Resource not found") -> None:
        super().__init__(404, "NOT_FOUND", message)


class ConflictError(APIError):
    """409 Conflict."""

    def __init__(self, message: str) -> None:
        super().__init__(409, "CONFLICT", message)


class ValidationError(APIError):
    """422 Validation Error."""

    def __init__(self, message: str, details: dict[str, Any] | None = None) -> None:
        super().__init__(422, "VALIDATION_ERROR", message, details)


class InternalError(APIError):
    """500 Internal Server Error."""

    def __init__(
        self,
        message: str = "An internal error occurred",
        details: dict[str, Any] | None = None,
    ) -> None:
        super().__init__(500, "INTERNAL_ERROR", message, details)


class LLMError(APIError):
    """500 LLM Error - Gemini API failures."""

    def __init__(self, message: str = "LLM processing failed") -> None:
        super().__init__(500, "LLM_ERROR", message)


class DocumentProcessingError(APIError):
    """500 Document Processing Error."""

    def __init__(self, message: str = "Document processing failed") -> None:
        super().__init__(500, "DOCUMENT_PROCESSING_ERROR", message)


async def api_error_handler(request: Request, exc: APIError) -> JSONResponse:
    """Handle APIError exceptions."""
    request_id = get_request_id()

    # Log error (don't leak internal details for 5xx)
    if exc.status_code >= 500:
        logger.error(
            "Internal error",
            code=exc.code,
            message=exc.message,
            details=exc.details,
            status_code=exc.status_code,
        )
    else:
        logger.warning(
            "API error",
            code=exc.code,
            message=exc.message,
            status_code=exc.status_code,
        )

    return JSONResponse(
        status_code=exc.status_code,
        content=ErrorResponse(
            code=exc.code,
            message=exc.message,
            request_id=request_id,
        ).model_dump(),
    )


async def http_exception_handler(request: Request, exc: HTTPException) -> JSONResponse:
    """Handle FastAPI HTTPException."""
    request_id = get_request_id()

    # Map status codes to error codes
    code_map = {
        400: "BAD_REQUEST",
        401: "UNAUTHORIZED",
        403: "FORBIDDEN",
        404: "NOT_FOUND",
        409: "CONFLICT",
        422: "VALIDATION_ERROR",
        500: "INTERNAL_ERROR",
    }

    code = code_map.get(exc.status_code, "ERROR")
    message = str(exc.detail) if exc.detail else "An error occurred"

    logger.warning(
        "HTTP exception",
        code=code,
        message=message,
        status_code=exc.status_code,
    )

    return JSONResponse(
        status_code=exc.status_code,
        content=ErrorResponse(
            code=code,
            message=message,
            request_id=request_id,
        ).model_dump(),
    )


async def unhandled_exception_handler(request: Request, exc: Exception) -> JSONResponse:
    """Handle unhandled exceptions."""
    request_id = get_request_id()

    logger.exception(
        "Unhandled exception",
        exc_info=exc,
    )

    return JSONResponse(
        status_code=500,
        content=ErrorResponse(
            code="INTERNAL_ERROR",
            message="An internal error occurred",
            request_id=request_id,
        ).model_dump(),
    )
