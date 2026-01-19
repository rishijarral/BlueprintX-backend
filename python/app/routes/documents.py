"""Document upload and management endpoints."""

import uuid
from pathlib import Path

from fastapi import APIRouter, File, Form, UploadFile
from pydantic import BaseModel

from app.config import get_settings
from app.dependencies import (
    GeminiClientDep,
    GeminiEmbeddingsDep,
    JobStoreDep,
    VectorStoreDep,
)
from app.errors import BadRequestError
from app.jobs.models import JobType
from app.jobs.runner import JobRunner
from app.logging import get_logger
from app.security import InternalAuth

logger = get_logger(__name__)

router = APIRouter()

# Upload directory
UPLOAD_DIR = Path("/data/uploads")


# =============================================================================
# Response Models
# =============================================================================


class DocumentUploadResponse(BaseModel):
    """Response for document upload."""

    document_id: str
    job_id: str
    filename: str
    file_size: int
    status: str = "queued"
    message: str = "Document uploaded. Processing job created."


class DocumentStatusResponse(BaseModel):
    """Response for document status."""

    document_id: str
    project_id: str
    filename: str
    status: str
    chunks_count: int | None = None
    error: str | None = None


# =============================================================================
# Endpoints
# =============================================================================


@router.post("/upload", response_model=DocumentUploadResponse)
async def upload_document(
    _auth: InternalAuth,
    job_store: JobStoreDep,
    settings: Settings = Depends(get_settings),
    file: UploadFile = File(...),
    project_id: str = Form(...),
    document_id: str | None = Form(default=None),
    run_immediately: bool = Form(default=False),
    gemini: GeminiClientDep = None,
    embeddings: GeminiEmbeddingsDep = None,
    vector_store: VectorStoreDep = None,
) -> DocumentUploadResponse:
    """
    Upload a document (PDF) for processing.

    Creates an ingestion job that will:
    1. Convert PDF pages to images
    2. Run OCR using Gemini Vision
    3. Chunk the extracted text
    4. Generate embeddings
    5. Store in vector database

    Form parameters:
    - file: PDF file to upload
    - project_id: Project this document belongs to
    - document_id: Optional custom document ID (generated if not provided)
    - run_immediately: If true, runs the job synchronously

    Requires internal authentication (X-Internal-Token header).
    """
    # Validate file type
    if not file.filename:
        raise BadRequestError("Filename is required")

    if not file.filename.lower().endswith(".pdf"):
        raise BadRequestError("Only PDF files are supported")

    # Check file size
    file_content = await file.read()
    file_size = len(file_content)

    if file_size > settings.max_upload_size_bytes:
        raise BadRequestError(
            f"File too large. Maximum size is {settings.max_upload_size_mb}MB"
        )

    # Generate document ID if not provided
    doc_id = document_id or str(uuid.uuid4())

    logger.info(
        "Document upload",
        project_id=project_id,
        document_id=doc_id,
        filename=file.filename,
        size=file_size,
    )

    # Create upload directory
    doc_dir = UPLOAD_DIR / project_id / doc_id
    doc_dir.mkdir(parents=True, exist_ok=True)

    # Save file
    file_path = doc_dir / file.filename
    with open(file_path, "wb") as f:
        f.write(file_content)

    logger.info("File saved", path=str(file_path))

    # Create ingestion job
    job = await job_store.create(
        job_type=JobType.DOCUMENT_INGEST,
        input_data={
            "file_path": str(file_path),
            "filename": file.filename,
        },
        project_id=project_id,
        document_id=doc_id,
    )

    # Run immediately if requested
    if run_immediately and gemini and embeddings and vector_store:
        runner = JobRunner(
            job_store=job_store,
            gemini_client=gemini,
            embeddings=embeddings,
            vector_store=vector_store,
        )
        await runner.run_job(job.job_id)
        job = await job_store.get(job.job_id)

    return DocumentUploadResponse(
        document_id=doc_id,
        job_id=job.job_id,
        filename=file.filename,
        file_size=file_size,
        status=job.status if job else "queued",
    )


@router.get("/{document_id}/status", response_model=DocumentStatusResponse)
async def get_document_status(
    document_id: str,
    project_id: str,
    _auth: InternalAuth,
    job_store: JobStoreDep,
    vector_store: VectorStoreDep,
) -> DocumentStatusResponse:
    """
    Get document processing status.

    Returns the status of the most recent ingestion job for this document
    and the number of chunks stored in the vector database.

    Requires internal authentication (X-Internal-Token header).
    """
    # Find related jobs
    jobs = await job_store.list_by_status(project_id=project_id)
    doc_jobs = [j for j in jobs if j.document_id == document_id]

    # Get most recent job
    latest_job = doc_jobs[0] if doc_jobs else None

    # Count chunks in vector store
    chunks_count = await vector_store.count(
        filter_metadata={
            "project_id": project_id,
            "document_id": document_id,
        }
    )

    # Determine status
    if latest_job:
        status = latest_job.status
        error = latest_job.error
    elif chunks_count > 0:
        status = "processed"
        error = None
    else:
        status = "unknown"
        error = None

    # Get filename from job input
    filename = "unknown"
    if latest_job and latest_job.input:
        filename = latest_job.input.get("filename", "unknown")

    return DocumentStatusResponse(
        document_id=document_id,
        project_id=project_id,
        filename=filename,
        status=status,
        chunks_count=chunks_count,
        error=error,
    )


@router.delete("/{document_id}")
async def delete_document(
    document_id: str,
    project_id: str,
    _auth: InternalAuth,
    vector_store: VectorStoreDep,
) -> dict:
    """
    Delete a document and its embeddings.

    Removes:
    - All embeddings from vector store
    - Uploaded file (if exists)

    Requires internal authentication (X-Internal-Token header).
    """
    logger.info(
        "Deleting document",
        project_id=project_id,
        document_id=document_id,
    )

    # Delete from vector store
    deleted_count = await vector_store.delete(
        filter_metadata={
            "project_id": project_id,
            "document_id": document_id,
        }
    )

    # Delete uploaded files
    doc_dir = UPLOAD_DIR / project_id / document_id
    files_deleted = 0
    if doc_dir.exists():
        import shutil

        shutil.rmtree(doc_dir)
        files_deleted = 1

    logger.info(
        "Document deleted",
        document_id=document_id,
        embeddings_deleted=deleted_count,
        files_deleted=files_deleted,
    )

    return {
        "document_id": document_id,
        "embeddings_deleted": deleted_count,
        "files_deleted": files_deleted,
    }
