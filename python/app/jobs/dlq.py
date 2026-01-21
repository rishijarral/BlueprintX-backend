"""Dead Letter Queue models and store implementations.

The DLQ captures failed jobs after max retries for later analysis and reprocessing.
"""

import uuid
from abc import ABC, abstractmethod
from datetime import datetime, timedelta
from enum import Enum
from typing import Any

import redis.asyncio as redis
from pydantic import BaseModel, Field

from app.jobs.models import Job, JobType
from app.logging import get_logger

logger = get_logger(__name__)


class FailureReason(str, Enum):
    """Categorized failure reasons for DLQ entries."""

    MAX_RETRIES_EXCEEDED = "max_retries_exceeded"
    PERMANENT_ERROR = "permanent_error"  # Non-retryable errors
    TIMEOUT = "timeout"
    INVALID_INPUT = "invalid_input"
    EXTERNAL_SERVICE_ERROR = "external_service_error"
    UNKNOWN = "unknown"


class DeadLetterEntry(BaseModel):
    """A job that has been moved to the dead letter queue."""

    dlq_id: str = Field(default_factory=lambda: str(uuid.uuid4()))

    # Original job information
    original_job_id: str
    job_type: JobType
    job_input: dict[str, Any]
    project_id: str | None = None
    document_id: str | None = None
    created_by: str | None = None

    # Failure information
    failure_reason: FailureReason = FailureReason.UNKNOWN
    error_message: str
    error_details: str | None = None  # Full stack trace or extended info
    attempt_count: int = 1

    # Timestamps
    original_job_created_at: datetime
    first_failure_at: datetime
    last_failure_at: datetime
    dlq_created_at: datetime = Field(default_factory=datetime.utcnow)

    # Processing status
    processed: bool = False
    processed_at: datetime | None = None
    requeued_job_id: str | None = None  # If retried, the new job ID

    class Config:
        use_enum_values = True

    @classmethod
    def from_job(
        cls,
        job: Job,
        error_message: str,
        failure_reason: FailureReason = FailureReason.MAX_RETRIES_EXCEEDED,
        error_details: str | None = None,
        attempt_count: int = 1,
    ) -> "DeadLetterEntry":
        """Create a DLQ entry from a failed job."""
        now = datetime.utcnow()
        return cls(
            original_job_id=job.job_id,
            job_type=job.type,
            job_input=job.input,
            project_id=job.project_id,
            document_id=job.document_id,
            created_by=job.created_by,
            failure_reason=failure_reason,
            error_message=error_message,
            error_details=error_details,
            attempt_count=attempt_count,
            original_job_created_at=job.created_at,
            first_failure_at=job.completed_at or now,
            last_failure_at=now,
        )


class DeadLetterEntryResponse(BaseModel):
    """API response model for DLQ entry."""

    dlq_id: str
    original_job_id: str
    job_type: str
    job_input: dict[str, Any]
    project_id: str | None
    document_id: str | None
    failure_reason: str
    error_message: str
    error_details: str | None
    attempt_count: int
    original_job_created_at: datetime
    first_failure_at: datetime
    last_failure_at: datetime
    dlq_created_at: datetime
    processed: bool
    processed_at: datetime | None
    requeued_job_id: str | None

    @classmethod
    def from_entry(cls, entry: DeadLetterEntry) -> "DeadLetterEntryResponse":
        """Create response from DeadLetterEntry."""
        return cls(
            dlq_id=entry.dlq_id,
            original_job_id=entry.original_job_id,
            job_type=entry.job_type,
            job_input=entry.job_input,
            project_id=entry.project_id,
            document_id=entry.document_id,
            failure_reason=entry.failure_reason,
            error_message=entry.error_message,
            error_details=entry.error_details,
            attempt_count=entry.attempt_count,
            original_job_created_at=entry.original_job_created_at,
            first_failure_at=entry.first_failure_at,
            last_failure_at=entry.last_failure_at,
            dlq_created_at=entry.dlq_created_at,
            processed=entry.processed,
            processed_at=entry.processed_at,
            requeued_job_id=entry.requeued_job_id,
        )


# =============================================================================
# Dead Letter Store Interface
# =============================================================================


class DeadLetterStore(ABC):
    """
    Abstract dead letter store interface.

    Stores failed jobs for later analysis and reprocessing.
    """

    @abstractmethod
    async def add(self, entry: DeadLetterEntry) -> DeadLetterEntry:
        """Add an entry to the dead letter queue."""
        pass

    @abstractmethod
    async def get(self, dlq_id: str) -> DeadLetterEntry | None:
        """Get a DLQ entry by ID."""
        pass

    @abstractmethod
    async def get_by_job_id(self, job_id: str) -> DeadLetterEntry | None:
        """Get a DLQ entry by original job ID."""
        pass

    @abstractmethod
    async def list(
        self,
        processed: bool | None = None,
        job_type: JobType | None = None,
        project_id: str | None = None,
        limit: int = 100,
        offset: int = 0,
    ) -> list[DeadLetterEntry]:
        """List DLQ entries with optional filtering."""
        pass

    @abstractmethod
    async def count(
        self,
        processed: bool | None = None,
        job_type: JobType | None = None,
        project_id: str | None = None,
    ) -> int:
        """Count DLQ entries matching filters."""
        pass

    @abstractmethod
    async def mark_processed(
        self,
        dlq_id: str,
        requeued_job_id: str | None = None,
    ) -> DeadLetterEntry | None:
        """Mark an entry as processed (retried or acknowledged)."""
        pass

    @abstractmethod
    async def delete(self, dlq_id: str) -> bool:
        """Delete a DLQ entry."""
        pass

    @abstractmethod
    async def purge(
        self,
        processed_only: bool = True,
        older_than_hours: int | None = None,
    ) -> int:
        """
        Purge entries from the DLQ.

        Args:
            processed_only: If True, only delete processed entries
            older_than_hours: If set, only delete entries older than this

        Returns:
            Number of entries deleted
        """
        pass


# =============================================================================
# Memory Implementation
# =============================================================================


class MemoryDeadLetterStore(DeadLetterStore):
    """
    In-memory DLQ store for development.

    Note: Entries are lost on restart.
    """

    def __init__(self) -> None:
        self._entries: dict[str, DeadLetterEntry] = {}
        self._job_id_index: dict[str, str] = {}  # job_id -> dlq_id mapping
        logger.info("MemoryDeadLetterStore initialized")

    async def add(self, entry: DeadLetterEntry) -> DeadLetterEntry:
        """Add an entry to the DLQ."""
        self._entries[entry.dlq_id] = entry
        self._job_id_index[entry.original_job_id] = entry.dlq_id

        logger.info(
            "DLQ entry added",
            dlq_id=entry.dlq_id,
            original_job_id=entry.original_job_id,
            job_type=entry.job_type,
            failure_reason=entry.failure_reason,
        )

        return entry

    async def get(self, dlq_id: str) -> DeadLetterEntry | None:
        """Get a DLQ entry by ID."""
        return self._entries.get(dlq_id)

    async def get_by_job_id(self, job_id: str) -> DeadLetterEntry | None:
        """Get a DLQ entry by original job ID."""
        dlq_id = self._job_id_index.get(job_id)
        if dlq_id:
            return self._entries.get(dlq_id)
        return None

    async def list(
        self,
        processed: bool | None = None,
        job_type: JobType | None = None,
        project_id: str | None = None,
        limit: int = 100,
        offset: int = 0,
    ) -> list[DeadLetterEntry]:
        """List DLQ entries with optional filtering."""
        entries = list(self._entries.values())

        # Apply filters
        if processed is not None:
            entries = [e for e in entries if e.processed == processed]

        if job_type is not None:
            entries = [e for e in entries if e.job_type == job_type]

        if project_id is not None:
            entries = [e for e in entries if e.project_id == project_id]

        # Sort by dlq_created_at descending (newest first)
        entries.sort(key=lambda e: e.dlq_created_at, reverse=True)

        # Apply pagination
        return entries[offset : offset + limit]

    async def count(
        self,
        processed: bool | None = None,
        job_type: JobType | None = None,
        project_id: str | None = None,
    ) -> int:
        """Count DLQ entries matching filters."""
        entries = list(self._entries.values())

        if processed is not None:
            entries = [e for e in entries if e.processed == processed]

        if job_type is not None:
            entries = [e for e in entries if e.job_type == job_type]

        if project_id is not None:
            entries = [e for e in entries if e.project_id == project_id]

        return len(entries)

    async def mark_processed(
        self,
        dlq_id: str,
        requeued_job_id: str | None = None,
    ) -> DeadLetterEntry | None:
        """Mark an entry as processed."""
        entry = self._entries.get(dlq_id)
        if not entry:
            return None

        entry.processed = True
        entry.processed_at = datetime.utcnow()
        entry.requeued_job_id = requeued_job_id

        logger.info(
            "DLQ entry marked processed",
            dlq_id=dlq_id,
            requeued_job_id=requeued_job_id,
        )

        return entry

    async def delete(self, dlq_id: str) -> bool:
        """Delete a DLQ entry."""
        entry = self._entries.get(dlq_id)
        if not entry:
            return False

        del self._entries[dlq_id]
        self._job_id_index.pop(entry.original_job_id, None)

        logger.info("DLQ entry deleted", dlq_id=dlq_id)
        return True

    async def purge(
        self,
        processed_only: bool = True,
        older_than_hours: int | None = None,
    ) -> int:
        """Purge entries from the DLQ."""
        cutoff = None
        if older_than_hours is not None:
            cutoff = datetime.utcnow() - timedelta(hours=older_than_hours)

        to_delete = []
        for dlq_id, entry in self._entries.items():
            if processed_only and not entry.processed:
                continue
            if cutoff and entry.dlq_created_at >= cutoff:
                continue
            to_delete.append(dlq_id)

        for dlq_id in to_delete:
            entry = self._entries.pop(dlq_id)
            self._job_id_index.pop(entry.original_job_id, None)

        if to_delete:
            logger.info(
                "DLQ entries purged",
                count=len(to_delete),
                processed_only=processed_only,
                older_than_hours=older_than_hours,
            )

        return len(to_delete)


# =============================================================================
# Redis Implementation
# =============================================================================


class RedisDeadLetterStore(DeadLetterStore):
    """
    Redis-backed DLQ store for production.

    Supports persistence and distributed access.
    """

    KEY_PREFIX = "dlq:"
    INDEX_PREFIX = "dlq_index:"

    def __init__(
        self,
        redis_client: redis.Redis,
        entry_ttl_days: int = 30,  # Keep DLQ entries for 30 days by default
    ) -> None:
        self._client = redis_client
        self._entry_ttl = timedelta(days=entry_ttl_days)
        logger.info("RedisDeadLetterStore initialized", ttl_days=entry_ttl_days)

    @classmethod
    async def create(
        cls,
        redis_url: str,
        entry_ttl_days: int = 30,
    ) -> "RedisDeadLetterStore":
        """Create a new RedisDeadLetterStore instance."""
        client = redis.from_url(
            redis_url,
            encoding="utf-8",
            decode_responses=True,
            socket_connect_timeout=5,
            socket_timeout=5,
        )
        await client.ping()
        return cls(client, entry_ttl_days)

    def _entry_key(self, dlq_id: str) -> str:
        """Build Redis key for a DLQ entry."""
        return f"{self.KEY_PREFIX}{dlq_id}"

    def _job_id_index_key(self) -> str:
        """Build Redis key for job ID -> dlq_id index."""
        return f"{self.INDEX_PREFIX}by_job_id"

    def _unprocessed_index_key(self) -> str:
        """Build Redis key for unprocessed entries index."""
        return f"{self.INDEX_PREFIX}unprocessed"

    def _project_index_key(self, project_id: str) -> str:
        """Build Redis key for project index."""
        return f"{self.INDEX_PREFIX}project:{project_id}"

    def _type_index_key(self, job_type: JobType) -> str:
        """Build Redis key for job type index."""
        return f"{self.INDEX_PREFIX}type:{job_type}"

    async def add(self, entry: DeadLetterEntry) -> DeadLetterEntry:
        """Add an entry to the DLQ."""
        entry_key = self._entry_key(entry.dlq_id)
        ttl_seconds = int(self._entry_ttl.total_seconds())

        # Store entry
        await self._client.setex(
            entry_key,
            ttl_seconds,
            entry.model_dump_json(),
        )

        score = entry.dlq_created_at.timestamp()

        # Update indices
        await self._client.hset(
            self._job_id_index_key(),
            entry.original_job_id,
            entry.dlq_id,
        )

        if not entry.processed:
            await self._client.zadd(
                self._unprocessed_index_key(),
                {entry.dlq_id: score},
            )

        await self._client.zadd(
            self._type_index_key(entry.job_type),
            {entry.dlq_id: score},
        )

        if entry.project_id:
            await self._client.zadd(
                self._project_index_key(entry.project_id),
                {entry.dlq_id: score},
            )

        logger.info(
            "DLQ entry added",
            dlq_id=entry.dlq_id,
            original_job_id=entry.original_job_id,
            job_type=entry.job_type,
            failure_reason=entry.failure_reason,
        )

        return entry

    async def get(self, dlq_id: str) -> DeadLetterEntry | None:
        """Get a DLQ entry by ID."""
        data = await self._client.get(self._entry_key(dlq_id))
        if not data:
            return None
        return DeadLetterEntry.model_validate_json(data)

    async def get_by_job_id(self, job_id: str) -> DeadLetterEntry | None:
        """Get a DLQ entry by original job ID."""
        dlq_id = await self._client.hget(self._job_id_index_key(), job_id)
        if dlq_id:
            return await self.get(dlq_id)
        return None

    async def list(
        self,
        processed: bool | None = None,
        job_type: JobType | None = None,
        project_id: str | None = None,
        limit: int = 100,
        offset: int = 0,
    ) -> list[DeadLetterEntry]:
        """List DLQ entries with optional filtering."""
        # Determine which index to use
        if processed is False:
            # Use unprocessed index
            dlq_ids = await self._client.zrevrange(
                self._unprocessed_index_key(),
                offset,
                offset + limit - 1,
            )
        elif job_type is not None:
            dlq_ids = await self._client.zrevrange(
                self._type_index_key(job_type),
                offset,
                offset + limit - 1,
            )
        elif project_id is not None:
            dlq_ids = await self._client.zrevrange(
                self._project_index_key(project_id),
                offset,
                offset + limit - 1,
            )
        else:
            # Scan all entries
            dlq_ids = []
            async for key in self._client.scan_iter(match=f"{self.KEY_PREFIX}*"):
                dlq_ids.append(key.replace(self.KEY_PREFIX, ""))
                if len(dlq_ids) >= offset + limit:
                    break
            dlq_ids = dlq_ids[offset : offset + limit]

        # Fetch entries
        entries = []
        for dlq_id in dlq_ids:
            entry = await self.get(dlq_id)
            if entry:
                # Apply additional filters
                if processed is not None and entry.processed != processed:
                    continue
                if job_type is not None and entry.job_type != job_type:
                    continue
                if project_id is not None and entry.project_id != project_id:
                    continue
                entries.append(entry)

        return entries

    async def count(
        self,
        processed: bool | None = None,
        job_type: JobType | None = None,
        project_id: str | None = None,
    ) -> int:
        """Count DLQ entries matching filters."""
        if processed is False:
            return await self._client.zcard(self._unprocessed_index_key())
        elif job_type is not None:
            return await self._client.zcard(self._type_index_key(job_type))
        elif project_id is not None:
            return await self._client.zcard(self._project_index_key(project_id))
        else:
            # Count all entries (expensive, avoid in production)
            count = 0
            async for _ in self._client.scan_iter(match=f"{self.KEY_PREFIX}*"):
                count += 1
            return count

    async def mark_processed(
        self,
        dlq_id: str,
        requeued_job_id: str | None = None,
    ) -> DeadLetterEntry | None:
        """Mark an entry as processed."""
        entry = await self.get(dlq_id)
        if not entry:
            return None

        entry.processed = True
        entry.processed_at = datetime.utcnow()
        entry.requeued_job_id = requeued_job_id

        # Update entry
        entry_key = self._entry_key(dlq_id)
        ttl = await self._client.ttl(entry_key)
        if ttl > 0:
            await self._client.setex(entry_key, ttl, entry.model_dump_json())
        else:
            await self._client.setex(
                entry_key,
                int(self._entry_ttl.total_seconds()),
                entry.model_dump_json(),
            )

        # Remove from unprocessed index
        await self._client.zrem(self._unprocessed_index_key(), dlq_id)

        logger.info(
            "DLQ entry marked processed",
            dlq_id=dlq_id,
            requeued_job_id=requeued_job_id,
        )

        return entry

    async def delete(self, dlq_id: str) -> bool:
        """Delete a DLQ entry."""
        entry = await self.get(dlq_id)
        if not entry:
            return False

        # Remove from all indices
        await self._client.hdel(self._job_id_index_key(), entry.original_job_id)
        await self._client.zrem(self._unprocessed_index_key(), dlq_id)
        await self._client.zrem(self._type_index_key(entry.job_type), dlq_id)
        if entry.project_id:
            await self._client.zrem(
                self._project_index_key(entry.project_id),
                dlq_id,
            )

        # Delete entry
        result = await self._client.delete(self._entry_key(dlq_id))

        if result:
            logger.info("DLQ entry deleted", dlq_id=dlq_id)
            return True
        return False

    async def purge(
        self,
        processed_only: bool = True,
        older_than_hours: int | None = None,
    ) -> int:
        """Purge entries from the DLQ."""
        cutoff = None
        if older_than_hours is not None:
            cutoff = datetime.utcnow() - timedelta(hours=older_than_hours)

        deleted_count = 0
        async for key in self._client.scan_iter(match=f"{self.KEY_PREFIX}*"):
            dlq_id = key.replace(self.KEY_PREFIX, "")
            entry = await self.get(dlq_id)
            if not entry:
                continue

            if processed_only and not entry.processed:
                continue

            if cutoff and entry.dlq_created_at >= cutoff:
                continue

            if await self.delete(dlq_id):
                deleted_count += 1

        if deleted_count > 0:
            logger.info(
                "DLQ entries purged",
                count=deleted_count,
                processed_only=processed_only,
                older_than_hours=older_than_hours,
            )

        return deleted_count
