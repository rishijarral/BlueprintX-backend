"""Job runner for executing jobs."""

from typing import Any

from app.gemini.client import GeminiClient
from app.gemini.embeddings import GeminiEmbeddings
from app.graphs.analysis import AnalysisPipeline, create_analysis_graph
from app.graphs.ingest import IngestPipeline, create_ingest_graph
from app.graphs.qna import QnAPipeline, create_qna_graph
from app.jobs.models import Job, JobStatus, JobType
from app.jobs.store import JobStore
from app.logging import get_logger
from app.vectorstore.base import VectorStore

logger = get_logger(__name__)


class JobRunner:
    """
    Job execution engine.

    Runs jobs synchronously or can be extended for async queue processing.
    """

    def __init__(
        self,
        job_store: JobStore,
        gemini_client: GeminiClient,
        embeddings: GeminiEmbeddings,
        vector_store: VectorStore,
    ) -> None:
        self.job_store = job_store
        self.gemini = gemini_client
        self.embeddings = embeddings
        self.vector_store = vector_store

        # Initialize pipelines
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

    async def run_job(self, job_id: str) -> Job | None:
        """
        Execute a job synchronously.

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

        # Mark as running
        await self.job_store.start(job_id)

        logger.info(
            "Starting job",
            job_id=job_id,
            type=job.type,
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

            # Mark as completed
            updated_job = await self.job_store.complete(job_id, result)

            logger.info(
                "Job completed",
                job_id=job_id,
                type=job.type,
            )

            return updated_job

        except Exception as e:
            # Mark as failed
            error_msg = str(e)[:500]  # Truncate error message
            await self.job_store.fail(job_id, error_msg)

            logger.error(
                "Job failed",
                job_id=job_id,
                type=job.type,
                error=error_msg,
            )

            return await self.job_store.get(job_id)

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

        Args:
            max_jobs: Maximum number of jobs to process

        Returns:
            Number of jobs processed
        """
        pending = await self.job_store.list_by_status(
            status=JobStatus.QUEUED,
            limit=max_jobs,
        )

        processed = 0
        for job in pending:
            await self.run_job(job.job_id)
            processed += 1

        return processed
