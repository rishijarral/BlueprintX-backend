"""Plan analysis endpoints: summary and trade scope extraction."""

from fastapi import APIRouter
from pydantic import BaseModel, Field

from app.dependencies import GeminiClientDep
from app.errors import BadRequestError
from app.gemini.schemas import PlanSummary, TradeScopesOutput
from app.graphs.analysis import create_analysis_graph
from app.logging import get_logger
from app.prompts.trade_scopes import STANDARD_TRADES
from app.security import InternalAuth

logger = get_logger(__name__)

router = APIRouter()


# =============================================================================
# Request/Response Models
# =============================================================================


class PlanSummaryRequest(BaseModel):
    """Request for plan summary."""

    project_id: str
    document_url: str | None = None
    document_text: str | None = None
    instructions: str | None = None


class PlanSummaryResponse(BaseModel):
    """Response for plan summary."""

    project_id: str
    summary: PlanSummary


class TradeScopesRequest(BaseModel):
    """Request for trade scope extraction."""

    project_id: str
    document_text: str
    trades: list[str] | None = Field(
        default=None,
        description="Trades to extract. Defaults to standard construction trades.",
    )


class TradeScopesResponse(BaseModel):
    """Response for trade scope extraction."""

    project_id: str
    scopes: TradeScopesOutput


# =============================================================================
# Endpoints
# =============================================================================


@router.post("/summary", response_model=PlanSummaryResponse)
async def generate_plan_summary(
    request: PlanSummaryRequest,
    _auth: InternalAuth,
    gemini: GeminiClientDep,
) -> PlanSummaryResponse:
    """
    Generate a structured summary of a construction plan.

    Analyzes the provided document text and extracts:
    - Building type and basic info
    - Key materials and systems
    - Risks and assumptions

    Requires internal authentication (X-Internal-Token header).
    """
    logger.info(
        "Plan summary request",
        project_id=request.project_id,
        has_text=request.document_text is not None,
    )

    # Validate input
    if not request.document_text:
        if request.document_url:
            raise BadRequestError(
                "document_url processing not yet implemented. Provide document_text."
            )
        raise BadRequestError("document_text is required")

    # Create analysis pipeline
    pipeline = create_analysis_graph(gemini)

    # Run analysis
    result = await pipeline.run_summary(
        project_id=request.project_id,
        document_text=request.document_text,
        instructions=request.instructions,
    )

    if result["status"] == "failed":
        raise BadRequestError(result.get("error", "Summary generation failed"))

    return PlanSummaryResponse(
        project_id=request.project_id,
        summary=PlanSummary.model_validate(result["result"]),
    )


@router.post("/trade-scopes", response_model=TradeScopesResponse)
async def extract_trade_scopes(
    request: TradeScopesRequest,
    _auth: InternalAuth,
    gemini: GeminiClientDep,
) -> TradeScopesResponse:
    """
    Extract scope information for each trade from a construction document.

    For each trade, extracts:
    - Inclusions and exclusions
    - Required drawing sheets and spec sections
    - Items needing RFI/clarification
    - Assumptions

    Requires internal authentication (X-Internal-Token header).
    """
    logger.info(
        "Trade scopes request",
        project_id=request.project_id,
        trades_count=len(request.trades) if request.trades else "default",
    )

    if not request.document_text:
        raise BadRequestError("document_text is required")

    # Create analysis pipeline
    pipeline = create_analysis_graph(gemini)

    # Run extraction
    result = await pipeline.run_trade_scopes(
        project_id=request.project_id,
        document_text=request.document_text,
        trades=request.trades,
    )

    if result["status"] == "failed":
        raise BadRequestError(result.get("error", "Trade scope extraction failed"))

    return TradeScopesResponse(
        project_id=request.project_id,
        scopes=TradeScopesOutput.model_validate(result["result"]),
    )


@router.get("/trades", response_model=list[str])
async def list_standard_trades(
    _auth: InternalAuth,
) -> list[str]:
    """
    Get list of standard construction trades.

    Returns the default list of trades used for scope extraction
    when no specific trades are provided.
    """
    return STANDARD_TRADES
