"""Security middleware and dependencies for internal API authentication."""

import secrets
from typing import Annotated

from fastapi import Depends, Header

from app.config import Settings, get_settings
from app.errors import UnauthorizedError
from app.logging import get_logger

logger = get_logger(__name__)

# Header name for internal token (matches Rust convention)
INTERNAL_TOKEN_HEADER = "X-Internal-Token"


async def verify_internal_token(
    x_internal_token: Annotated[str | None, Header(alias=INTERNAL_TOKEN_HEADER)] = None,
    settings: Settings = Depends(get_settings),
) -> None:
    """
    Verify the internal API token from Rust backend.

    This dependency ensures that only the Rust backend can call this service.
    Uses constant-time comparison to prevent timing attacks.

    Raises:
        UnauthorizedError: If token is missing or invalid
    """
    if x_internal_token is None:
        logger.warning("Missing internal token header")
        raise UnauthorizedError("Missing internal token")

    # Constant-time comparison to prevent timing attacks
    if not secrets.compare_digest(x_internal_token, settings.internal_api_token):
        logger.warning("Invalid internal token")
        raise UnauthorizedError("Invalid internal token")


# Type alias for dependency injection
InternalAuth = Annotated[None, Depends(verify_internal_token)]
