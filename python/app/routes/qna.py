"""Q&A endpoint with RAG support."""

from fastapi import APIRouter
from pydantic import BaseModel, Field

from app.dependencies import (
    GeminiClientDep,
    GeminiEmbeddingsDep,
    VectorStoreDep,
)
from app.errors import BadRequestError
from app.graphs.qna import create_qna_graph
from app.logging import get_logger
from app.security import InternalAuth

logger = get_logger(__name__)

router = APIRouter()


# =============================================================================
# Request/Response Models
# =============================================================================


class QnARequest(BaseModel):
    """Request for Q&A."""

    project_id: str
    question: str = Field(description="Question to answer")
    document_id: str | None = Field(
        default=None,
        description="Specific document to search within",
    )
    document_text: str | None = Field(
        default=None,
        description="If provided, use this text instead of vector search",
    )


class QnAResponseModel(BaseModel):
    """Response for Q&A."""

    project_id: str
    question: str
    answer: str
    citations: list[str]
    confidence: float
    followups: list[str]


# =============================================================================
# Endpoints
# =============================================================================


@router.post("/qna", response_model=QnAResponseModel)
async def answer_question(
    request: QnARequest,
    _auth: InternalAuth,
    gemini: GeminiClientDep,
    embeddings: GeminiEmbeddingsDep,
    vector_store: VectorStoreDep,
) -> QnAResponseModel:
    """
    Answer a question about project documents using RAG.

    If document_text is provided, uses that directly.
    Otherwise, performs vector search to find relevant chunks
    from previously ingested documents.

    Requires internal authentication (X-Internal-Token header).
    """
    logger.info(
        "Q&A request",
        project_id=request.project_id,
        question_length=len(request.question),
        has_document_text=request.document_text is not None,
    )

    if not request.question.strip():
        raise BadRequestError("question cannot be empty")

    # Create Q&A pipeline
    pipeline = create_qna_graph(
        gemini,
        embeddings,
        vector_store,
    )

    # Run Q&A
    result = await pipeline.run(
        project_id=request.project_id,
        question=request.question,
        document_id=request.document_id,
        document_text=request.document_text,
    )

    if result["status"] == "failed":
        raise BadRequestError(result.get("error", "Q&A failed"))

    qna_result = result.get("result")
    if not qna_result:
        raise BadRequestError("No answer generated")

    return QnAResponseModel(
        project_id=request.project_id,
        question=request.question,
        answer=qna_result.answer,
        citations=qna_result.citations,
        confidence=qna_result.confidence,
        followups=qna_result.followups,
    )
