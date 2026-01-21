"""Dead Letter Queue management endpoints."""

from pydantic import BaseModel, Field

from fastapi import APIRouter

from app.config import get_settings
from app.dependencies import (
    DeadLetterStoreDep,
    GeminiClientDep,
    GeminiEmbeddingsDep,
    JobStoreDep,
    VectorStoreDep,
)
from app.errors import BadRequestError, NotFoundError
from app.jobs.dlq import (
    DeadLetterEntryResponse,
    FailureReason,
)
from app.jobs.models import JobResponse, JobType
from app.jobs.runner import JobRunner
from app.logging import get_logger
from app.security import InternalAuth

logger = get_logger(__name__)

router = APIRouter()


# =============================================================================
# Request/Response Models
# =============================================================================


class DLQListResponse(BaseModel):
    """Response for DLQ entry list."""

    entries: list[DeadLetterEntryResponse]
    total: int
    unprocessed_count: int


class DLQStatsResponse(BaseModel):
    """DLQ statistics."""

    total_entries: int
    unprocessed_entries: int
    processed_entries: int
    by_failure_reason: dict[str, int]
    by_job_type: dict[str, int]


class RetryRequest(BaseModel):
    """Request to retry a DLQ entry."""

    max_retries: int | None = Field(
        default=None,
        description="Override max retries for the new job (uses original if not set)",
    )


class RetryResponse(BaseModel):
    """Response from retrying a DLQ entry."""

    dlq_entry: DeadLetterEntryResponse
    new_job: JobResponse


class PurgeRequest(BaseModel):
    """Request to purge DLQ entries."""

    processed_only: bool = Field(
        default=True,
        description="If true, only purge processed entries",
    )
    older_than_hours: int | None = Field(
        default=None,
        description="Only purge entries older than this (hours)",
    )


class PurgeResponse(BaseModel):
    """Response from purging DLQ entries."""

    deleted_count: int


# =============================================================================
# Endpoints
# =============================================================================


@router.get("", response_model=DLQListResponse)
async def list_dlq_entries(
    _auth: InternalAuth,
    dlq_store: DeadLetterStoreDep,
    processed: bool | None = None,
    job_type: JobType | None = None,
    project_id: str | None = None,
    limit: int = 100,
    offset: int = 0,
) -> DLQListResponse:
    """
    List dead letter queue entries.

    Query parameters:
    - processed: Filter by processed status
    - job_type: Filter by original job type
    - project_id: Filter by project
    - limit: Maximum results (default 100)
    - offset: Pagination offset (default 0)

    Requires internal authentication (X-Internal-Token header).
    """
    entries = await dlq_store.list(
        processed=processed,
        job_type=job_type,
        project_id=project_id,
        limit=limit,
        offset=offset,
    )

    total = await dlq_store.count(
        processed=processed,
        job_type=job_type,
        project_id=project_id,
    )

    unprocessed_count = await dlq_store.count(processed=False)

    return DLQListResponse(
        entries=[DeadLetterEntryResponse.from_entry(e) for e in entries],
        total=total,
        unprocessed_count=unprocessed_count,
    )


@router.get("/stats", response_model=DLQStatsResponse)
async def get_dlq_stats(
    _auth: InternalAuth,
    dlq_store: DeadLetterStoreDep,
) -> DLQStatsResponse:
    """
    Get DLQ statistics.

    Returns counts by failure reason and job type.

    Requires internal authentication (X-Internal-Token header).
    """
    total = await dlq_store.count()
    unprocessed = await dlq_store.count(processed=False)
    processed = await dlq_store.count(processed=True)

    # Get counts by failure reason
    by_failure_reason: dict[str, int] = {}
    for reason in FailureReason:
        # We need to count entries by iterating (not ideal for large DLQs)
        # In production, consider adding indexed counts
        entries = await dlq_store.list(processed=False, limit=1000)
        count = sum(1 for e in entries if e.failure_reason == reason.value)
        if count > 0:
            by_failure_reason[reason.value] = count

    # Get counts by job type
    by_job_type: dict[str, int] = {}
    for job_type in JobType:
        count = await dlq_store.count(job_type=job_type)
        if count > 0:
            by_job_type[job_type.value] = count

    return DLQStatsResponse(
        total_entries=total,
        unprocessed_entries=unprocessed,
        processed_entries=processed,
        by_failure_reason=by_failure_reason,
        by_job_type=by_job_type,
    )


@router.get("/{dlq_id}", response_model=DeadLetterEntryResponse)
async def get_dlq_entry(
    dlq_id: str,
    _auth: InternalAuth,
    dlq_store: DeadLetterStoreDep,
) -> DeadLetterEntryResponse:
    """
    Get a specific DLQ entry.

    Requires internal authentication (X-Internal-Token header).
    """
    entry = await dlq_store.get(dlq_id)
    if not entry:
        raise NotFoundError(f"DLQ entry not found: {dlq_id}")

    return DeadLetterEntryResponse.from_entry(entry)


@router.get("/by-job/{job_id}", response_model=DeadLetterEntryResponse)
async def get_dlq_entry_by_job(
    job_id: str,
    _auth: InternalAuth,
    dlq_store: DeadLetterStoreDep,
) -> DeadLetterEntryResponse:
    """
    Get a DLQ entry by original job ID.

    Requires internal authentication (X-Internal-Token header).
    """
    entry = await dlq_store.get_by_job_id(job_id)
    if not entry:
        raise NotFoundError(f"DLQ entry not found for job: {job_id}")

    return DeadLetterEntryResponse.from_entry(entry)


@router.post("/{dlq_id}/retry", response_model=RetryResponse)
async def retry_dlq_entry(
    dlq_id: str,
    _auth: InternalAuth,
    dlq_store: DeadLetterStoreDep,
    job_store: JobStoreDep,
    gemini: GeminiClientDep,
    embeddings: GeminiEmbeddingsDep,
    vector_store: VectorStoreDep,
    request: RetryRequest | None = None,
) -> RetryResponse:
    """
    Retry a failed job from the DLQ.

    Creates a new job with the same parameters and optionally runs it.
    The DLQ entry is marked as processed.

    Requires internal authentication (X-Internal-Token header).
    """
    entry = await dlq_store.get(dlq_id)
    if not entry:
        raise NotFoundError(f"DLQ entry not found: {dlq_id}")

    if entry.processed:
        raise BadRequestError(
            f"DLQ entry already processed. Requeued job ID: {entry.requeued_job_id}"
        )

    settings = get_settings()

    # Create a new job from the DLQ entry
    max_retries = (
        request.max_retries
        if request and request.max_retries is not None
        else settings.job_max_retries
    )

    new_job = await job_store.create(
        job_type=entry.job_type,
        input_data=entry.job_input,
        project_id=entry.project_id,
        document_id=entry.document_id,
        created_by=entry.created_by,
        max_retries=max_retries,
    )

    # Mark the DLQ entry as processed
    updated_entry = await dlq_store.mark_processed(
        dlq_id=dlq_id,
        requeued_job_id=new_job.job_id,
    )

    logger.info(
        "DLQ entry retried",
        dlq_id=dlq_id,
        original_job_id=entry.original_job_id,
        new_job_id=new_job.job_id,
    )

    return RetryResponse(
        dlq_entry=DeadLetterEntryResponse.from_entry(updated_entry or entry),
        new_job=JobResponse.from_job(new_job),
    )


@router.post("/{dlq_id}/retry-and-run", response_model=RetryResponse)
async def retry_and_run_dlq_entry(
    dlq_id: str,
    _auth: InternalAuth,
    dlq_store: DeadLetterStoreDep,
    job_store: JobStoreDep,
    gemini: GeminiClientDep,
    embeddings: GeminiEmbeddingsDep,
    vector_store: VectorStoreDep,
    request: RetryRequest | None = None,
) -> RetryResponse:
    """
    Retry a failed job from the DLQ and run it immediately.

    Creates a new job, marks the DLQ entry as processed, and executes the job.
    Returns the completed job result.

    Requires internal authentication (X-Internal-Token header).
    """
    entry = await dlq_store.get(dlq_id)
    if not entry:
        raise NotFoundError(f"DLQ entry not found: {dlq_id}")

    if entry.processed:
        raise BadRequestError(
            f"DLQ entry already processed. Requeued job ID: {entry.requeued_job_id}"
        )

    settings = get_settings()

    # Create a new job from the DLQ entry
    max_retries = (
        request.max_retries
        if request and request.max_retries is not None
        else settings.job_max_retries
    )

    new_job = await job_store.create(
        job_type=entry.job_type,
        input_data=entry.job_input,
        project_id=entry.project_id,
        document_id=entry.document_id,
        created_by=entry.created_by,
        max_retries=max_retries,
    )

    # Mark the DLQ entry as processed
    updated_entry = await dlq_store.mark_processed(
        dlq_id=dlq_id,
        requeued_job_id=new_job.job_id,
    )

    # Run the job with retries
    runner = JobRunner(
        job_store=job_store,
        gemini_client=gemini,
        embeddings=embeddings,
        vector_store=vector_store,
        dlq_store=dlq_store,
        settings=settings,
    )

    completed_job = await runner.run_job_with_immediate_retry(new_job.job_id)

    logger.info(
        "DLQ entry retried and run",
        dlq_id=dlq_id,
        original_job_id=entry.original_job_id,
        new_job_id=new_job.job_id,
        new_job_status=completed_job.status if completed_job else "unknown",
    )

    return RetryResponse(
        dlq_entry=DeadLetterEntryResponse.from_entry(updated_entry or entry),
        new_job=JobResponse.from_job(completed_job or new_job),
    )


@router.post("/{dlq_id}/acknowledge")
async def acknowledge_dlq_entry(
    dlq_id: str,
    _auth: InternalAuth,
    dlq_store: DeadLetterStoreDep,
) -> dict:
    """
    Acknowledge a DLQ entry without retrying.

    Marks the entry as processed so it won't appear in unprocessed lists.
    Use this when you've manually handled the failure or determined
    the job should not be retried.

    Requires internal authentication (X-Internal-Token header).
    """
    entry = await dlq_store.get(dlq_id)
    if not entry:
        raise NotFoundError(f"DLQ entry not found: {dlq_id}")

    if entry.processed:
        raise BadRequestError("DLQ entry already processed")

    await dlq_store.mark_processed(dlq_id=dlq_id)

    logger.info(
        "DLQ entry acknowledged",
        dlq_id=dlq_id,
        original_job_id=entry.original_job_id,
    )

    return {"acknowledged": True, "dlq_id": dlq_id}


@router.delete("/{dlq_id}")
async def delete_dlq_entry(
    dlq_id: str,
    _auth: InternalAuth,
    dlq_store: DeadLetterStoreDep,
) -> dict:
    """
    Delete a specific DLQ entry.

    Use with caution - the entry will be permanently removed.

    Requires internal authentication (X-Internal-Token header).
    """
    entry = await dlq_store.get(dlq_id)
    if not entry:
        raise NotFoundError(f"DLQ entry not found: {dlq_id}")

    deleted = await dlq_store.delete(dlq_id)

    logger.info(
        "DLQ entry deleted",
        dlq_id=dlq_id,
        original_job_id=entry.original_job_id,
    )

    return {"deleted": deleted, "dlq_id": dlq_id}


@router.post("/purge", response_model=PurgeResponse)
async def purge_dlq_entries(
    request: PurgeRequest,
    _auth: InternalAuth,
    dlq_store: DeadLetterStoreDep,
) -> PurgeResponse:
    """
    Purge DLQ entries in bulk.

    By default, only purges processed entries. Use with caution.

    Requires internal authentication (X-Internal-Token header).
    """
    deleted_count = await dlq_store.purge(
        processed_only=request.processed_only,
        older_than_hours=request.older_than_hours,
    )

    logger.info(
        "DLQ entries purged",
        deleted_count=deleted_count,
        processed_only=request.processed_only,
        older_than_hours=request.older_than_hours,
    )

    return PurgeResponse(deleted_count=deleted_count)
