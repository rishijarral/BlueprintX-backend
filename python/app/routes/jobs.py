"""Job management endpoints."""

from fastapi import APIRouter, BackgroundTasks
from pydantic import BaseModel

from app.dependencies import (
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

    Requires internal authentication (X-Internal-Token header).
    """
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
    gemini: GeminiClientDep,
    embeddings: GeminiEmbeddingsDep,
    vector_store: VectorStoreDep,
) -> JobResponse:
    """
    Run a job synchronously.

    Executes the job immediately and returns when complete.
    For long-running jobs (like document ingestion), consider
    using async mode (not yet implemented).

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

    # Create runner and execute
    runner = JobRunner(
        job_store=job_store,
        gemini_client=gemini,
        embeddings=embeddings,
        vector_store=vector_store,
    )

    updated_job = await runner.run_job(job_id)
    if not updated_job:
        raise NotFoundError(f"Job not found after execution: {job_id}")

    return JobResponse.from_job(updated_job)


@router.post("/{job_id}/run-async", response_model=JobResponse)
async def run_job_async(
    job_id: str,
    background_tasks: BackgroundTasks,
    _auth: InternalAuth,
    job_store: JobStoreDep,
    gemini: GeminiClientDep,
    embeddings: GeminiEmbeddingsDep,
    vector_store: VectorStoreDep,
) -> JobResponse:
    """
    Run a job asynchronously (in background).

    Returns immediately with job status. Poll GET /jobs/{job_id}
    to check completion.

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

    # Create runner
    runner = JobRunner(
        job_store=job_store,
        gemini_client=gemini,
        embeddings=embeddings,
        vector_store=vector_store,
    )

    # Schedule background execution
    background_tasks.add_task(runner.run_job, job_id)

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
