"""Job models for async task processing."""

from datetime import datetime
from enum import Enum
from typing import Any

from pydantic import BaseModel, Field


class JobStatus(str, Enum):
    """Job execution status."""

    QUEUED = "queued"
    RUNNING = "running"
    SUCCEEDED = "succeeded"
    FAILED = "failed"
    CANCELLED = "cancelled"


class JobType(str, Enum):
    """Types of jobs the service can process."""

    DOCUMENT_INGEST = "document_ingest"
    PLAN_SUMMARY = "plan_summary"
    TRADE_SCOPE_EXTRACT = "trade_scope_extract"
    TENDER_SCOPE_DOC = "tender_scope_doc"
    QNA = "qna"


class Job(BaseModel):
    """Job record."""

    job_id: str
    type: JobType
    status: JobStatus = JobStatus.QUEUED
    input: dict[str, Any] = Field(default_factory=dict)
    output: dict[str, Any] | None = None
    error: str | None = None
    progress: float = Field(default=0.0, ge=0.0, le=1.0)

    # Metadata
    project_id: str | None = None
    document_id: str | None = None
    created_by: str | None = None

    # Timestamps
    created_at: datetime = Field(default_factory=datetime.utcnow)
    started_at: datetime | None = None
    completed_at: datetime | None = None

    class Config:
        use_enum_values = True


class CreateJobRequest(BaseModel):
    """Request to create a new job."""

    type: JobType
    input: dict[str, Any] = Field(default_factory=dict)
    project_id: str | None = None
    document_id: str | None = None


class JobResponse(BaseModel):
    """Job response for API."""

    job_id: str
    type: str
    status: str
    progress: float
    input: dict[str, Any]
    output: dict[str, Any] | None
    error: str | None
    project_id: str | None
    document_id: str | None
    created_at: datetime
    started_at: datetime | None
    completed_at: datetime | None

    @classmethod
    def from_job(cls, job: Job) -> "JobResponse":
        """Create response from Job model."""
        return cls(
            job_id=job.job_id,
            type=job.type,
            status=job.status,
            progress=job.progress,
            input=job.input,
            output=job.output,
            error=job.error,
            project_id=job.project_id,
            document_id=job.document_id,
            created_at=job.created_at,
            started_at=job.started_at,
            completed_at=job.completed_at,
        )
