"""Job store implementations."""

import uuid
from abc import ABC, abstractmethod
from datetime import datetime, timedelta
from typing import Any

import redis.asyncio as redis

from app.jobs.models import Job, JobStatus, JobType
from app.logging import get_logger

logger = get_logger(__name__)


class JobStore(ABC):
    """
    Abstract job store interface.

    Allows swapping between in-memory, Redis, or database-backed stores.
    """

    @abstractmethod
    async def create(
        self,
        job_type: JobType,
        input_data: dict[str, Any],
        project_id: str | None = None,
        document_id: str | None = None,
        created_by: str | None = None,
        max_retries: int = 3,
    ) -> Job:
        """Create a new job."""
        pass

    @abstractmethod
    async def get(self, job_id: str) -> Job | None:
        """Get a job by ID."""
        pass

    @abstractmethod
    async def update(
        self,
        job_id: str,
        status: JobStatus | None = None,
        progress: float | None = None,
        output: dict[str, Any] | None = None,
        error: str | None = None,
        attempt_count: int | None = None,
        last_error: str | None = None,
        next_retry_at: datetime | None = None,
    ) -> Job | None:
        """Update a job."""
        pass

    @abstractmethod
    async def list_by_status(
        self,
        status: JobStatus | None = None,
        project_id: str | None = None,
        limit: int = 100,
    ) -> list[Job]:
        """List jobs filtered by status and/or project."""
        pass

    @abstractmethod
    async def delete(self, job_id: str) -> bool:
        """Delete a job."""
        pass

    async def start(self, job_id: str) -> Job | None:
        """Mark a job as started."""
        job = await self.get(job_id)
        if job:
            job.status = JobStatus.RUNNING
            job.started_at = datetime.utcnow()
            return await self.update(
                job_id,
                status=JobStatus.RUNNING,
            )
        return None

    async def complete(
        self,
        job_id: str,
        output: dict[str, Any],
    ) -> Job | None:
        """Mark a job as completed successfully."""
        return await self.update(
            job_id,
            status=JobStatus.SUCCEEDED,
            progress=1.0,
            output=output,
        )

    async def fail(
        self,
        job_id: str,
        error: str,
    ) -> Job | None:
        """Mark a job as failed."""
        return await self.update(
            job_id,
            status=JobStatus.FAILED,
            error=error,
        )


class MemoryJobStore(JobStore):
    """
    In-memory job store for development and MVP.

    Note: Jobs are lost on restart. For production, use Redis or database.
    """

    def __init__(self) -> None:
        self._jobs: dict[str, Job] = {}
        logger.info("MemoryJobStore initialized")

    async def create(
        self,
        job_type: JobType,
        input_data: dict[str, Any],
        project_id: str | None = None,
        document_id: str | None = None,
        created_by: str | None = None,
        max_retries: int = 3,
    ) -> Job:
        """Create a new job."""
        job_id = str(uuid.uuid4())

        job = Job(
            job_id=job_id,
            type=job_type,
            status=JobStatus.QUEUED,
            input=input_data,
            project_id=project_id,
            document_id=document_id,
            created_by=created_by,
            max_retries=max_retries,
        )

        self._jobs[job_id] = job

        logger.info(
            "Job created",
            job_id=job_id,
            type=job_type,
            project_id=project_id,
            max_retries=max_retries,
        )

        return job

    async def get(self, job_id: str) -> Job | None:
        """Get a job by ID."""
        return self._jobs.get(job_id)

    async def update(
        self,
        job_id: str,
        status: JobStatus | None = None,
        progress: float | None = None,
        output: dict[str, Any] | None = None,
        error: str | None = None,
        attempt_count: int | None = None,
        last_error: str | None = None,
        next_retry_at: datetime | None = None,
    ) -> Job | None:
        """Update a job."""
        job = self._jobs.get(job_id)
        if not job:
            return None

        if status is not None:
            job.status = status
            if status == JobStatus.RUNNING and not job.started_at:
                job.started_at = datetime.utcnow()
            elif status in (JobStatus.SUCCEEDED, JobStatus.FAILED, JobStatus.CANCELLED):
                job.completed_at = datetime.utcnow()

        if progress is not None:
            job.progress = progress

        if output is not None:
            job.output = output

        if error is not None:
            job.error = error

        if attempt_count is not None:
            job.attempt_count = attempt_count

        if last_error is not None:
            job.last_error = last_error

        if next_retry_at is not None:
            job.next_retry_at = next_retry_at

        logger.debug(
            "Job updated",
            job_id=job_id,
            status=job.status,
            progress=job.progress,
            attempt_count=job.attempt_count,
        )

        return job

    async def list_by_status(
        self,
        status: JobStatus | None = None,
        project_id: str | None = None,
        limit: int = 100,
    ) -> list[Job]:
        """List jobs filtered by status and/or project."""
        jobs = list(self._jobs.values())

        if status:
            jobs = [j for j in jobs if j.status == status]

        if project_id:
            jobs = [j for j in jobs if j.project_id == project_id]

        # Sort by created_at descending
        jobs.sort(key=lambda j: j.created_at, reverse=True)

        return jobs[:limit]

    async def delete(self, job_id: str) -> bool:
        """Delete a job."""
        if job_id in self._jobs:
            del self._jobs[job_id]
            logger.info("Job deleted", job_id=job_id)
            return True
        return False

    async def cleanup_old_jobs(self, max_age_hours: int = 24) -> int:
        """Remove jobs older than max_age_hours."""
        cutoff = datetime.utcnow() - timedelta(hours=max_age_hours)
        to_delete = [
            job_id
            for job_id, job in self._jobs.items()
            if job.created_at < cutoff
            and job.status in (JobStatus.SUCCEEDED, JobStatus.FAILED, JobStatus.CANCELLED)
        ]

        for job_id in to_delete:
            del self._jobs[job_id]

        if to_delete:
            logger.info("Old jobs cleaned up", count=len(to_delete))

        return len(to_delete)


class RedisJobStore(JobStore):
    """
    Redis-backed job store for production.

    Jobs are stored in Redis with automatic expiration.
    Supports persistence across restarts.
    """

    # Redis key prefixes
    KEY_PREFIX = "job:"
    INDEX_PREFIX = "jobs:"

    def __init__(
        self,
        redis_client: redis.Redis,
        job_ttl_hours: int = 72,
    ) -> None:
        self._client = redis_client
        self._job_ttl = timedelta(hours=job_ttl_hours)
        logger.info("RedisJobStore initialized", ttl_hours=job_ttl_hours)

    @classmethod
    async def create(cls, redis_url: str, job_ttl_hours: int = 72) -> "RedisJobStore":
        """Create a new RedisJobStore instance."""
        client = redis.from_url(
            redis_url,
            encoding="utf-8",
            decode_responses=True,
            socket_connect_timeout=5,
            socket_timeout=5,
        )
        # Verify connection
        await client.ping()
        return cls(client, job_ttl_hours)

    def _job_key(self, job_id: str) -> str:
        """Build Redis key for a job."""
        return f"{self.KEY_PREFIX}{job_id}"

    def _status_index_key(self, status: JobStatus) -> str:
        """Build Redis key for status index."""
        return f"{self.INDEX_PREFIX}status:{status.value}"

    def _project_index_key(self, project_id: str) -> str:
        """Build Redis key for project index."""
        return f"{self.INDEX_PREFIX}project:{project_id}"

    async def create(
        self,
        job_type: JobType,
        input_data: dict[str, Any],
        project_id: str | None = None,
        document_id: str | None = None,
        created_by: str | None = None,
        max_retries: int = 3,
    ) -> Job:
        """Create a new job."""
        job_id = str(uuid.uuid4())

        job = Job(
            job_id=job_id,
            type=job_type,
            status=JobStatus.QUEUED,
            input=input_data,
            project_id=project_id,
            document_id=document_id,
            created_by=created_by,
            max_retries=max_retries,
        )

        # Store job
        job_key = self._job_key(job_id)
        ttl_seconds = int(self._job_ttl.total_seconds())
        await self._client.setex(
            job_key,
            ttl_seconds,
            job.model_dump_json(),
        )

        # Add to indices
        score = job.created_at.timestamp()
        await self._client.zadd(
            self._status_index_key(JobStatus.QUEUED),
            {job_id: score},
        )
        if project_id:
            await self._client.zadd(
                self._project_index_key(project_id),
                {job_id: score},
            )

        logger.info(
            "Job created",
            job_id=job_id,
            type=job_type,
            project_id=project_id,
            max_retries=max_retries,
        )

        return job

    async def get(self, job_id: str) -> Job | None:
        """Get a job by ID."""
        job_data = await self._client.get(self._job_key(job_id))
        if not job_data:
            return None
        return Job.model_validate_json(job_data)

    async def update(
        self,
        job_id: str,
        status: JobStatus | None = None,
        progress: float | None = None,
        output: dict[str, Any] | None = None,
        error: str | None = None,
        attempt_count: int | None = None,
        last_error: str | None = None,
        next_retry_at: datetime | None = None,
    ) -> Job | None:
        """Update a job."""
        job = await self.get(job_id)
        if not job:
            return None

        old_status = job.status

        if status is not None:
            job.status = status
            if status == JobStatus.RUNNING and not job.started_at:
                job.started_at = datetime.utcnow()
            elif status in (JobStatus.SUCCEEDED, JobStatus.FAILED, JobStatus.CANCELLED):
                job.completed_at = datetime.utcnow()

        if progress is not None:
            job.progress = progress

        if output is not None:
            job.output = output

        if error is not None:
            job.error = error

        if attempt_count is not None:
            job.attempt_count = attempt_count

        if last_error is not None:
            job.last_error = last_error

        if next_retry_at is not None:
            job.next_retry_at = next_retry_at

        # Update job in Redis
        job_key = self._job_key(job_id)
        ttl = await self._client.ttl(job_key)
        if ttl > 0:
            await self._client.setex(job_key, ttl, job.model_dump_json())
        else:
            await self._client.setex(
                job_key,
                int(self._job_ttl.total_seconds()),
                job.model_dump_json(),
            )

        # Update status index if status changed
        if status is not None and status != old_status:
            await self._client.zrem(self._status_index_key(old_status), job_id)
            await self._client.zadd(
                self._status_index_key(status),
                {job_id: datetime.utcnow().timestamp()},
            )

        logger.debug(
            "Job updated",
            job_id=job_id,
            status=job.status,
            progress=job.progress,
        )

        return job

    async def list_by_status(
        self,
        status: JobStatus | None = None,
        project_id: str | None = None,
        limit: int = 100,
    ) -> list[Job]:
        """List jobs filtered by status and/or project."""
        if project_id:
            # Get job IDs from project index
            job_ids = await self._client.zrevrange(
                self._project_index_key(project_id),
                0,
                limit - 1,
            )
        elif status:
            # Get job IDs from status index
            job_ids = await self._client.zrevrange(
                self._status_index_key(status),
                0,
                limit - 1,
            )
        else:
            # Get all job IDs (scan keys)
            job_ids = []
            async for key in self._client.scan_iter(match=f"{self.KEY_PREFIX}*"):
                job_ids.append(key.replace(self.KEY_PREFIX, ""))
                if len(job_ids) >= limit:
                    break

        # Fetch jobs
        jobs = []
        for job_id in job_ids:
            job = await self.get(job_id)
            if job:
                # Apply status filter if project was used
                if status and job.status != status:
                    continue
                jobs.append(job)

        # Sort by created_at descending
        jobs.sort(key=lambda j: j.created_at, reverse=True)

        return jobs[:limit]

    async def delete(self, job_id: str) -> bool:
        """Delete a job."""
        job = await self.get(job_id)
        if not job:
            return False

        # Remove from indices
        await self._client.zrem(self._status_index_key(job.status), job_id)
        if job.project_id:
            await self._client.zrem(
                self._project_index_key(job.project_id),
                job_id,
            )

        # Delete job
        result = await self._client.delete(self._job_key(job_id))

        if result:
            logger.info("Job deleted", job_id=job_id)
            return True
        return False

    async def cleanup_old_jobs(self, max_age_hours: int = 24) -> int:
        """Remove jobs older than max_age_hours (handled by Redis TTL)."""
        # Redis TTL handles expiration automatically
        # This method is for compatibility with the base class
        logger.info("Cleanup called - Redis TTL handles expiration")
        return 0
