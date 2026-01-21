"""Job management endpoints."""

from fastapi import APIRouter, BackgroundTasks
from pydantic import BaseModel

from app.config import get_settings
from app.dependencies import (
    DeadLetterStoreDep,
    GeminiClientDep,
    GeminiEmbeddingsDep,
    JobStoreDep,
    VectorStoreDep,
)
from app.errors import BadRequestError, NotFoundError
from app.jobs.models import (
    CreateJobRequest,
    JobResponse,
    JobStatus,
)
from app.jobs.runner import JobRunner
from app.logging import get_logger
from app.security import InternalAuth

logger = get_logger(__name__)

router = APIRouter()


# =============================================================================
# Response Models
# =============================================================================


class JobListResponse(BaseModel):
    """Response for job list."""

    jobs: list[JobResponse]
    total: int


# =============================================================================
# Endpoints
# =============================================================================


@router.post("", response_model=JobResponse)
async def create_job(
    request: CreateJobRequest,
    _auth: InternalAuth,
    job_store: JobStoreDep,
) -> JobResponse:
    """
    Create a new job.

    Creates a job in QUEUED status. Use POST /jobs/{job_id}/run
    to execute it.

    Job types:
    - document_ingest: Ingest and embed a document
    - plan_summary: Generate project summary
    - trade_scope_extract: Extract trade scopes
    - tender_scope_doc: Generate tender scope document
    - qna: Answer a question

    Jobs support automatic retries with exponential backoff on
    transient failures. Failed jobs are moved to the dead letter
    queue for later analysis and retry.

    Requires internal authentication (X-Internal-Token header).
    """
    settings = get_settings()

    logger.info(
        "Creating job",
        type=request.type,
        project_id=request.project_id,
    )

    job = await job_store.create(
        job_type=request.type,
        input_data=request.input,
        project_id=request.project_id,
        document_id=request.document_id,
        max_retries=settings.job_max_retries,
    )

    return JobResponse.from_job(job)


@router.get("/{job_id}", response_model=JobResponse)
async def get_job(
    job_id: str,
    _auth: InternalAuth,
    job_store: JobStoreDep,
) -> JobResponse:
    """
    Get job status and details.

    Requires internal authentication (X-Internal-Token header).
    """
    job = await job_store.get(job_id)
    if not job:
        raise NotFoundError(f"Job not found: {job_id}")

    return JobResponse.from_job(job)


@router.post("/{job_id}/run", response_model=JobResponse)
async def run_job(
    job_id: str,
    _auth: InternalAuth,
    job_store: JobStoreDep,
    dlq_store: DeadLetterStoreDep,
    gemini: GeminiClientDep,
    embeddings: GeminiEmbeddingsDep,
    vector_store: VectorStoreDep,
) -> JobResponse:
    """
    Run a job synchronously with retry logic.

    Executes the job immediately with automatic retries on transient failures.
    Failed jobs are moved to the dead letter queue after max retries.

    Requires internal authentication (X-Internal-Token header).
    """
    logger.info("Running job", job_id=job_id)

    job = await job_store.get(job_id)
    if not job:
        raise NotFoundError(f"Job not found: {job_id}")

    if job.status != JobStatus.QUEUED:
        raise BadRequestError(
            f"Job cannot be run: status is {job.status}, expected 'queued'"
        )

    settings = get_settings()

    # Create runner with DLQ support and execute
    runner = JobRunner(
        job_store=job_store,
        gemini_client=gemini,
        embeddings=embeddings,
        vector_store=vector_store,
        dlq_store=dlq_store,
        settings=settings,
    )

    updated_job = await runner.run_job_with_immediate_retry(job_id)
    if not updated_job:
        raise NotFoundError(f"Job not found after execution: {job_id}")

    return JobResponse.from_job(updated_job)


@router.post("/{job_id}/run-async", response_model=JobResponse)
async def run_job_async(
    job_id: str,
    background_tasks: BackgroundTasks,
    _auth: InternalAuth,
    job_store: JobStoreDep,
    dlq_store: DeadLetterStoreDep,
    gemini: GeminiClientDep,
    embeddings: GeminiEmbeddingsDep,
    vector_store: VectorStoreDep,
) -> JobResponse:
    """
    Run a job asynchronously (in background) with retry logic.

    Returns immediately with job status. Poll GET /jobs/{job_id}
    to check completion. Jobs are automatically retried on transient
    failures and moved to DLQ after max retries.

    Requires internal authentication (X-Internal-Token header).
    """
    logger.info("Scheduling async job", job_id=job_id)

    job = await job_store.get(job_id)
    if not job:
        raise NotFoundError(f"Job not found: {job_id}")

    if job.status != JobStatus.QUEUED:
        raise BadRequestError(
            f"Job cannot be run: status is {job.status}, expected 'queued'"
        )

    settings = get_settings()

    # Create runner with DLQ support
    runner = JobRunner(
        job_store=job_store,
        gemini_client=gemini,
        embeddings=embeddings,
        vector_store=vector_store,
        dlq_store=dlq_store,
        settings=settings,
    )

    # Schedule background execution with immediate retries
    background_tasks.add_task(runner.run_job_with_immediate_retry, job_id)

    # Update status to indicate it's been scheduled
    job.status = JobStatus.QUEUED  # Still queued until background picks it up

    return JobResponse.from_job(job)


@router.get("", response_model=JobListResponse)
async def list_jobs(
    _auth: InternalAuth,
    job_store: JobStoreDep,
    status: JobStatus | None = None,
    project_id: str | None = None,
    limit: int = 100,
) -> JobListResponse:
    """
    List jobs with optional filtering.

    Query parameters:
    - status: Filter by job status
    - project_id: Filter by project
    - limit: Maximum results (default 100)

    Requires internal authentication (X-Internal-Token header).
    """
    jobs = await job_store.list_by_status(
        status=status,
        project_id=project_id,
        limit=limit,
    )

    return JobListResponse(
        jobs=[JobResponse.from_job(j) for j in jobs],
        total=len(jobs),
    )


@router.delete("/{job_id}")
async def delete_job(
    job_id: str,
    _auth: InternalAuth,
    job_store: JobStoreDep,
) -> dict:
    """
    Delete a job.

    Only completed, failed, or cancelled jobs can be deleted.

    Requires internal authentication (X-Internal-Token header).
    """
    job = await job_store.get(job_id)
    if not job:
        raise NotFoundError(f"Job not found: {job_id}")

    if job.status in (JobStatus.QUEUED, JobStatus.RUNNING):
        raise BadRequestError(
            f"Cannot delete job with status '{job.status}'. "
            "Wait for completion or cancel first."
        )

    deleted = await job_store.delete(job_id)
    return {"deleted": deleted, "job_id": job_id}
