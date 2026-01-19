"""Redis cache implementation for the AI service."""

import hashlib
import json
from contextlib import asynccontextmanager
from typing import Any

import redis.asyncio as redis
from pydantic import BaseModel

from app.config import get_settings
from app.logging import get_logger

logger = get_logger(__name__)


class RedisCache:
    """
    Async Redis cache wrapper.

    Provides a simple interface for caching with automatic JSON serialization.
    """

    def __init__(self, redis_client: redis.Redis, default_ttl: int = 3600) -> None:
        self._client = redis_client
        self._default_ttl = default_ttl
        self._connected = False

    @classmethod
    async def create(cls, redis_url: str, default_ttl: int = 3600) -> "RedisCache":
        """Create a new RedisCache instance."""
        client = redis.from_url(
            redis_url,
            encoding="utf-8",
            decode_responses=True,
            socket_connect_timeout=5,
            socket_timeout=5,
        )
        cache = cls(client, default_ttl)
        await cache._check_connection()
        return cache

    async def _check_connection(self) -> bool:
        """Check if Redis is connected."""
        try:
            await self._client.ping()
            self._connected = True
            logger.info("Redis connection established")
            return True
        except Exception as e:
            self._connected = False
            logger.warning("Redis connection failed", error=str(e))
            return False

    async def close(self) -> None:
        """Close the Redis connection."""
        await self._client.close()
        self._connected = False
        logger.info("Redis connection closed")

    @property
    def is_connected(self) -> bool:
        """Check if cache is connected."""
        return self._connected

    async def ping(self) -> bool:
        """Ping Redis to check connectivity."""
        try:
            await self._client.ping()
            return True
        except Exception:
            self._connected = False
            return False

    # -------------------------------------------------------------------------
    # Core operations
    # -------------------------------------------------------------------------

    async def get(self, key: str) -> Any | None:
        """
        Get a value from cache.

        Returns None if key doesn't exist or on error.
        """
        if not self._connected:
            return None

        try:
            value = await self._client.get(key)
            if value is None:
                logger.debug("Cache miss", key=key)
                return None

            logger.debug("Cache hit", key=key)
            return json.loads(value)
        except Exception as e:
            logger.warning("Cache get error", key=key, error=str(e))
            return None

    async def set(
        self,
        key: str,
        value: Any,
        ttl: int | None = None,
    ) -> bool:
        """
        Set a value in cache.

        Args:
            key: Cache key
            value: Value to cache (must be JSON serializable)
            ttl: Time-to-live in seconds (uses default if not specified)

        Returns:
            True if successful, False otherwise
        """
        if not self._connected:
            return False

        ttl = ttl or self._default_ttl

        try:
            # Handle Pydantic models
            if isinstance(value, BaseModel):
                serialized = value.model_dump_json()
            else:
                serialized = json.dumps(value, default=str)

            await self._client.setex(key, ttl, serialized)
            logger.debug("Cache set", key=key, ttl=ttl)
            return True
        except Exception as e:
            logger.warning("Cache set error", key=key, error=str(e))
            return False

    async def delete(self, key: str) -> bool:
        """Delete a key from cache."""
        if not self._connected:
            return False

        try:
            result = await self._client.delete(key)
            logger.debug("Cache delete", key=key, deleted=bool(result))
            return bool(result)
        except Exception as e:
            logger.warning("Cache delete error", key=key, error=str(e))
            return False

    async def exists(self, key: str) -> bool:
        """Check if a key exists in cache."""
        if not self._connected:
            return False

        try:
            return bool(await self._client.exists(key))
        except Exception:
            return False

    async def delete_pattern(self, pattern: str) -> int:
        """Delete all keys matching a pattern."""
        if not self._connected:
            return 0

        try:
            keys = []
            async for key in self._client.scan_iter(match=pattern):
                keys.append(key)

            if keys:
                deleted = await self._client.delete(*keys)
                logger.info("Cache pattern delete", pattern=pattern, deleted=deleted)
                return deleted
            return 0
        except Exception as e:
            logger.warning("Cache pattern delete error", pattern=pattern, error=str(e))
            return 0

    # -------------------------------------------------------------------------
    # Key builders
    # -------------------------------------------------------------------------

    @staticmethod
    def build_key(*parts: str) -> str:
        """Build a cache key from parts."""
        return ":".join(parts)

    @staticmethod
    def hash_content(content: str) -> str:
        """Create a hash of content for cache keys."""
        return hashlib.sha256(content.encode()).hexdigest()[:16]

    # -------------------------------------------------------------------------
    # Convenience methods for AI service patterns
    # -------------------------------------------------------------------------

    def plan_summary_key(self, project_id: str, document_id: str) -> str:
        """Build cache key for plan summary."""
        return self.build_key("ai", "plan_summary", project_id, document_id)

    def trade_scopes_key(self, project_id: str, document_id: str) -> str:
        """Build cache key for trade scopes."""
        return self.build_key("ai", "trade_scopes", project_id, document_id)

    def scope_doc_key(self, tender_id: str, trade: str) -> str:
        """Build cache key for scope document."""
        return self.build_key("ai", "scope_doc", tender_id, trade)

    def embedding_key(self, text_hash: str) -> str:
        """Build cache key for embeddings."""
        return self.build_key("ai", "embedding", text_hash)

    def qna_key(self, project_id: str, question_hash: str) -> str:
        """Build cache key for Q&A responses."""
        return self.build_key("ai", "qna", project_id, question_hash)


# -----------------------------------------------------------------------------
# Global cache instance management
# -----------------------------------------------------------------------------

_cache_instance: RedisCache | None = None


async def init_redis_cache() -> RedisCache | None:
    """Initialize the global Redis cache instance."""
    global _cache_instance

    settings = get_settings()

    if not settings.redis_url:
        logger.info("Redis URL not configured, caching disabled")
        return None

    try:
        _cache_instance = await RedisCache.create(
            settings.redis_url,
            default_ttl=settings.redis_cache_ttl_seconds,
        )
        return _cache_instance
    except Exception as e:
        logger.error("Failed to initialize Redis cache", error=str(e))
        return None


async def close_redis_cache() -> None:
    """Close the global Redis cache instance."""
    global _cache_instance

    if _cache_instance:
        await _cache_instance.close()
        _cache_instance = None


def get_redis_cache() -> RedisCache | None:
    """Get the global Redis cache instance."""
    return _cache_instance


@asynccontextmanager
async def redis_lifespan():
    """Context manager for Redis lifecycle."""
    await init_redis_cache()
    try:
        yield get_redis_cache()
    finally:
        await close_redis_cache()
