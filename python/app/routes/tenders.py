"""Tender scope document generation endpoints."""

from fastapi import APIRouter
from pydantic import BaseModel, Field

from app.dependencies import GeminiClientDep
from app.errors import BadRequestError
from app.gemini.schemas import TenderScopeDoc
from app.graphs.analysis import create_analysis_graph
from app.logging import get_logger
from app.security import InternalAuth

logger = get_logger(__name__)

router = APIRouter()


# =============================================================================
# Request/Response Models
# =============================================================================


class TenderScopeDocRequest(BaseModel):
    """Request to generate a tender scope document."""

    project_id: str
    trade: str = Field(description="Trade name (e.g., 'Electrical', 'Plumbing')")
    scope_data: dict = Field(
        description="Extracted scope data (from /plan/trade-scopes)"
    )
    project_context: str | None = Field(
        default=None,
        description="Project name, location, and type",
    )
    bid_due_date: str | None = Field(
        default=None,
        description="Bid due date for inclusion in document",
    )
    gc_contact: str | None = Field(
        default=None,
        description="GC contact information",
    )


class TenderScopeDocResponse(BaseModel):
    """Response with generated tender scope document."""

    project_id: str
    trade: str
    document: TenderScopeDoc


# =============================================================================
# Endpoints
# =============================================================================


@router.post("/scope-doc", response_model=TenderScopeDocResponse)
async def generate_tender_scope_doc(
    request: TenderScopeDocRequest,
    _auth: InternalAuth,
    gemini: GeminiClientDep,
) -> TenderScopeDocResponse:
    """
    Generate a formal Scope of Work document for a tender package.

    Takes extracted scope data (from /plan/trade-scopes) and generates
    a professional document with:
    - Overview
    - Inclusions/Exclusions
    - Allowances and Alternates
    - Submittals required
    - Schedule notes and lead times
    - Bid instructions
    - RFI questions

    Requires internal authentication (X-Internal-Token header).
    """
    logger.info(
        "Tender scope doc request",
        project_id=request.project_id,
        trade=request.trade,
    )

    if not request.scope_data:
        raise BadRequestError("scope_data is required")

    if not request.trade:
        raise BadRequestError("trade is required")

    # Create analysis pipeline
    pipeline = create_analysis_graph(gemini)

    # Generate document
    result = await pipeline.run_tender_doc(
        project_id=request.project_id,
        trade=request.trade,
        scope_data=request.scope_data,
        project_context=request.project_context,
        bid_due_date=request.bid_due_date,
    )

    if result["status"] == "failed":
        raise BadRequestError(result.get("error", "Tender doc generation failed"))

    return TenderScopeDocResponse(
        project_id=request.project_id,
        trade=request.trade,
        document=TenderScopeDoc.model_validate(result["result"]),
    )
