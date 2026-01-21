"""Job runner for executing jobs with retry logic and DLQ support."""

import asyncio
import traceback
from datetime import datetime, timedelta
from typing import Any

from app.config import Settings
from app.gemini.client import GeminiClient
from app.gemini.embeddings import GeminiEmbeddings
from app.graphs.analysis import AnalysisPipeline, create_analysis_graph
from app.graphs.ingest import IngestPipeline, create_ingest_graph
from app.graphs.qna import QnAPipeline, create_qna_graph
from app.jobs.dlq import (
    DeadLetterEntry,
    DeadLetterStore,
    FailureReason,
)
from app.jobs.models import Job, JobStatus, JobType
from app.jobs.store import JobStore
from app.logging import get_logger
from app.vectorstore.base import VectorStore

logger = get_logger(__name__)


# Error categories for determining retry behavior
PERMANENT_ERROR_PATTERNS = [
    "invalid_input",
    "validation error",
    "invalid document",
    "file not found",
    "permission denied",
    "authentication failed",
    "invalid api key",
    "quota exceeded",  # Quota errors typically need manual intervention
]

TIMEOUT_ERROR_PATTERNS = [
    "timeout",
    "timed out",
    "deadline exceeded",
    "connection timeout",
]


def classify_error(error_message: str) -> FailureReason:
    """
    Classify an error message to determine the failure reason.

    Returns a FailureReason enum value based on error patterns.
    """
    error_lower = error_message.lower()

    for pattern in TIMEOUT_ERROR_PATTERNS:
        if pattern in error_lower:
            return FailureReason.TIMEOUT

    for pattern in PERMANENT_ERROR_PATTERNS:
        if pattern in error_lower:
            return FailureReason.PERMANENT_ERROR

    # Check for input validation errors
    if "input" in error_lower and ("missing" in error_lower or "required" in error_lower):
        return FailureReason.INVALID_INPUT

    # Check for external service errors (API errors that might be transient)
    if any(
        p in error_lower
        for p in ["api error", "service unavailable", "rate limit", "500", "502", "503", "504"]
    ):
        return FailureReason.EXTERNAL_SERVICE_ERROR

    return FailureReason.UNKNOWN


def is_retryable_error(failure_reason: FailureReason) -> bool:
    """
    Determine if an error is retryable based on its classification.

    Permanent errors and invalid input errors are not retryable.
    Transient errors (timeout, external service) are retryable.
    """
    return failure_reason not in (
        FailureReason.PERMANENT_ERROR,
        FailureReason.INVALID_INPUT,
    )


def calculate_retry_delay(
    attempt: int,
    base_delay: float,
    max_delay: float,
    multiplier: float,
) -> float:
    """
    Calculate exponential backoff delay for retry.

    Args:
        attempt: Current attempt number (1-based)
        base_delay: Base delay in seconds
        max_delay: Maximum delay cap in seconds
        multiplier: Backoff multiplier

    Returns:
        Delay in seconds before next retry
    """
    delay = base_delay * (multiplier ** (attempt - 1))
    return min(delay, max_delay)


class JobRunner:
    """
    Job execution engine with retry logic and dead letter queue support.

    Jobs are retried with exponential backoff on transient failures.
    After max retries, failed jobs are moved to the dead letter queue.
    """

    def __init__(
        self,
        job_store: JobStore,
        gemini_client: GeminiClient,
        embeddings: GeminiEmbeddings,
        vector_store: VectorStore,
        dlq_store: DeadLetterStore | None = None,
        settings: Settings | None = None,
    ) -> None:
        self.job_store = job_store
        self.gemini = gemini_client
        self.embeddings = embeddings
        self.vector_store = vector_store
        self.dlq_store = dlq_store

        # Retry configuration from settings or defaults
        if settings:
            self.max_retries = settings.job_max_retries
            self.retry_base_delay = settings.job_retry_base_delay_seconds
            self.retry_max_delay = settings.job_retry_max_delay_seconds
            self.retry_backoff_multiplier = settings.job_retry_backoff_multiplier
        else:
            self.max_retries = 3
            self.retry_base_delay = 5
            self.retry_max_delay = 300
            self.retry_backoff_multiplier = 2.0

        # Initialize pipelines lazily
        self.ingest_pipeline: IngestPipeline | None = None
        self.analysis_pipeline: AnalysisPipeline | None = None
        self.qna_pipeline: QnAPipeline | None = None

    def _get_ingest_pipeline(self) -> IngestPipeline:
        """Get or create ingest pipeline."""
        if not self.ingest_pipeline:
            self.ingest_pipeline = create_ingest_graph(
                self.gemini,
                self.embeddings,
                self.vector_store,
            )
        return self.ingest_pipeline

    def _get_analysis_pipeline(self) -> AnalysisPipeline:
        """Get or create analysis pipeline."""
        if not self.analysis_pipeline:
            self.analysis_pipeline = create_analysis_graph(
                self.gemini,
                self.vector_store,
            )
        return self.analysis_pipeline

    def _get_qna_pipeline(self) -> QnAPipeline:
        """Get or create Q&A pipeline."""
        if not self.qna_pipeline:
            self.qna_pipeline = create_qna_graph(
                self.gemini,
                self.embeddings,
                self.vector_store,
            )
        return self.qna_pipeline

    async def _move_to_dlq(
        self,
        job: Job,
        error_message: str,
        failure_reason: FailureReason,
        error_details: str | None = None,
    ) -> DeadLetterEntry | None:
        """
        Move a failed job to the dead letter queue.

        Args:
            job: The failed job
            error_message: Error message (truncated)
            failure_reason: Categorized failure reason
            error_details: Full stack trace or extended error info

        Returns:
            The created DLQ entry, or None if DLQ is not configured
        """
        if not self.dlq_store:
            logger.warning(
                "DLQ not configured, failed job will not be preserved",
                job_id=job.job_id,
            )
            return None

        entry = DeadLetterEntry.from_job(
            job=job,
            error_message=error_message,
            failure_reason=failure_reason,
            error_details=error_details,
            attempt_count=job.attempt_count,
        )

        await self.dlq_store.add(entry)

        logger.info(
            "Job moved to DLQ",
            job_id=job.job_id,
            dlq_id=entry.dlq_id,
            failure_reason=failure_reason,
            attempt_count=job.attempt_count,
        )

        return entry

    async def run_job(self, job_id: str) -> Job | None:
        """
        Execute a job with retry logic.

        On failure:
        - Retryable errors: Increment attempt count, schedule retry
        - Non-retryable or max retries exceeded: Move to DLQ, mark as FAILED

        Args:
            job_id: ID of the job to run

        Returns:
            Updated job or None if not found
        """
        job = await self.job_store.get(job_id)
        if not job:
            logger.error("Job not found", job_id=job_id)
            return None

        if job.status != JobStatus.QUEUED:
            logger.warning(
                "Job not in queued state",
                job_id=job_id,
                status=job.status,
            )
            return job

        # Increment attempt count and mark as running
        current_attempt = job.attempt_count + 1
        await self.job_store.update(
            job_id,
            status=JobStatus.RUNNING,
            attempt_count=current_attempt,
            next_retry_at=None,  # Clear any scheduled retry
        )

        logger.info(
            "Starting job",
            job_id=job_id,
            type=job.type,
            attempt=current_attempt,
            max_retries=job.max_retries,
        )

        try:
            # Route to appropriate handler
            if job.type == JobType.DOCUMENT_INGEST:
                result = await self._run_ingest(job)
            elif job.type == JobType.PLAN_SUMMARY:
                result = await self._run_plan_summary(job)
            elif job.type == JobType.TRADE_SCOPE_EXTRACT:
                result = await self._run_trade_scopes(job)
            elif job.type == JobType.TENDER_SCOPE_DOC:
                result = await self._run_tender_doc(job)
            elif job.type == JobType.QNA:
                result = await self._run_qna(job)
            else:
                raise ValueError(f"Unknown job type: {job.type}")

            # Mark as completed successfully
            updated_job = await self.job_store.complete(job_id, result)

            logger.info(
                "Job completed successfully",
                job_id=job_id,
                type=job.type,
                attempt=current_attempt,
            )

            return updated_job

        except Exception as e:
            error_msg = str(e)[:500]  # Truncate error message
            error_details = traceback.format_exc()
            failure_reason = classify_error(error_msg)

            logger.error(
                "Job execution failed",
                job_id=job_id,
                type=job.type,
                attempt=current_attempt,
                error=error_msg,
                failure_reason=failure_reason,
            )

            # Determine if we should retry
            can_retry = (
                is_retryable_error(failure_reason)
                and current_attempt < job.max_retries
            )

            if can_retry:
                # Schedule retry with exponential backoff
                delay = calculate_retry_delay(
                    attempt=current_attempt,
                    base_delay=self.retry_base_delay,
                    max_delay=self.retry_max_delay,
                    multiplier=self.retry_backoff_multiplier,
                )
                next_retry_at = datetime.utcnow() + timedelta(seconds=delay)

                await self.job_store.update(
                    job_id,
                    status=JobStatus.QUEUED,  # Back to queued for retry
                    progress=0.0,  # Reset progress
                    last_error=error_msg,
                    next_retry_at=next_retry_at,
                )

                logger.info(
                    "Job scheduled for retry",
                    job_id=job_id,
                    attempt=current_attempt,
                    next_retry_at=next_retry_at.isoformat(),
                    delay_seconds=delay,
                )

                return await self.job_store.get(job_id)
            else:
                # No more retries - move to DLQ and mark as failed
                # Refresh job to get latest state
                job = await self.job_store.get(job_id)
                if job:
                    await self._move_to_dlq(
                        job=job,
                        error_message=error_msg,
                        failure_reason=(
                            FailureReason.MAX_RETRIES_EXCEEDED
                            if current_attempt >= job.max_retries
                            else failure_reason
                        ),
                        error_details=error_details,
                    )

                await self.job_store.fail(job_id, error_msg)

                logger.error(
                    "Job failed permanently",
                    job_id=job_id,
                    type=job.type if job else "unknown",
                    attempt=current_attempt,
                    failure_reason=failure_reason,
                    moved_to_dlq=self.dlq_store is not None,
                )

                return await self.job_store.get(job_id)

    async def run_job_with_immediate_retry(self, job_id: str) -> Job | None:
        """
        Execute a job with immediate in-process retries.

        This method handles retries within the same execution context,
        useful for background task execution where we want to complete
        all retries before returning.

        Args:
            job_id: ID of the job to run

        Returns:
            Updated job or None if not found
        """
        job = await self.job_store.get(job_id)
        if not job:
            logger.error("Job not found", job_id=job_id)
            return None

        while True:
            result = await self.run_job(job_id)
            if not result:
                return None

            # Check if job completed or failed permanently
            if result.status in (JobStatus.SUCCEEDED, JobStatus.FAILED, JobStatus.CANCELLED):
                return result

            # Job is queued for retry - wait for the delay and retry
            if result.next_retry_at:
                delay = (result.next_retry_at - datetime.utcnow()).total_seconds()
                if delay > 0:
                    logger.info(
                        "Waiting for retry delay",
                        job_id=job_id,
                        delay_seconds=delay,
                    )
                    await asyncio.sleep(delay)

    async def _run_ingest(self, job: Job) -> dict[str, Any]:
        """Run document ingestion job."""
        pipeline = self._get_ingest_pipeline()

        result = await pipeline.run(
            job_id=job.job_id,
            project_id=job.project_id or job.input.get("project_id", ""),
            document_id=job.document_id or job.input.get("document_id", ""),
            file_path=job.input.get("file_path"),
            file_bytes=job.input.get("file_bytes"),
        )

        if result["status"] == "failed":
            raise Exception(result.get("error", "Ingestion failed"))

        return {
            "status": result["status"],
            "pages_processed": result.get("pdf_metadata", {}).get("page_count", 0)
            if result.get("pdf_metadata")
            else 0,
            "chunks_created": len(result.get("chunks", [])),
            "embeddings_stored": result.get("embeddings_stored", 0),
        }

    async def _run_plan_summary(self, job: Job) -> dict[str, Any]:
        """Run plan summary job."""
        pipeline = self._get_analysis_pipeline()

        result = await pipeline.run_summary(
            project_id=job.project_id or job.input.get("project_id", ""),
            document_text=job.input.get("document_text", ""),
            instructions=job.input.get("instructions"),
        )

        if result["status"] == "failed":
            raise Exception(result.get("error", "Summary generation failed"))

        return result.get("result", {})

    async def _run_trade_scopes(self, job: Job) -> dict[str, Any]:
        """Run trade scope extraction job."""
        pipeline = self._get_analysis_pipeline()

        result = await pipeline.run_trade_scopes(
            project_id=job.project_id or job.input.get("project_id", ""),
            document_text=job.input.get("document_text", ""),
            trades=job.input.get("trades"),
        )

        if result["status"] == "failed":
            raise Exception(result.get("error", "Trade scope extraction failed"))

        return result.get("result", {})

    async def _run_tender_doc(self, job: Job) -> dict[str, Any]:
        """Run tender scope document generation job."""
        pipeline = self._get_analysis_pipeline()

        result = await pipeline.run_tender_doc(
            project_id=job.project_id or job.input.get("project_id", ""),
            trade=job.input.get("trade", ""),
            scope_data=job.input.get("scope_data", {}),
            project_context=job.input.get("project_context"),
            bid_due_date=job.input.get("bid_due_date"),
        )

        if result["status"] == "failed":
            raise Exception(result.get("error", "Tender doc generation failed"))

        return result.get("result", {})

    async def _run_qna(self, job: Job) -> dict[str, Any]:
        """Run Q&A job."""
        pipeline = self._get_qna_pipeline()

        result = await pipeline.run(
            project_id=job.project_id or job.input.get("project_id", ""),
            question=job.input.get("question", ""),
            document_id=job.document_id or job.input.get("document_id"),
            document_text=job.input.get("document_text"),
        )

        if result["status"] == "failed":
            raise Exception(result.get("error", "Q&A failed"))

        qna_result = result.get("result")
        if qna_result:
            return qna_result.model_dump()
        return {}

    async def process_pending_jobs(self, max_jobs: int = 10) -> int:
        """
        Process pending jobs (for background worker mode).

        Processes jobs that are queued and either:
        - Have no scheduled retry time (first attempt)
        - Have a retry time that has passed

        Args:
            max_jobs: Maximum number of jobs to process

        Returns:
            Number of jobs processed
        """
        pending = await self.job_store.list_by_status(
            status=JobStatus.QUEUED,
            limit=max_jobs,
        )

        now = datetime.utcnow()
        processed = 0

        for job in pending:
            # Skip jobs scheduled for future retry
            if job.next_retry_at and job.next_retry_at > now:
                continue

            await self.run_job_with_immediate_retry(job.job_id)
            processed += 1

        return processed

    async def process_retry_jobs(self) -> int:
        """
        Process jobs that are due for retry.

        Finds queued jobs with next_retry_at in the past and processes them.

        Returns:
            Number of jobs processed
        """
        pending = await self.job_store.list_by_status(
            status=JobStatus.QUEUED,
            limit=100,
        )

        now = datetime.utcnow()
        processed = 0

        for job in pending:
            # Only process jobs with retries due
            if job.next_retry_at and job.next_retry_at <= now:
                await self.run_job(job.job_id)
                processed += 1

        if processed > 0:
            logger.info("Processed retry jobs", count=processed)

        return processed
