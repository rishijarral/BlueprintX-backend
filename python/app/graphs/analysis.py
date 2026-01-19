"""LangGraph pipeline for document analysis: summary, trade scopes, etc."""

from typing import Any, TypedDict

from langgraph.graph import END, StateGraph

from app.gemini.client import GeminiClient
from app.gemini.schemas import (
    GenerationConfig,
    PlanSummary,
    TenderScopeDoc,
    TradeScopesOutput,
)
from app.logging import get_logger
from app.prompts.plan_summary import build_plan_summary_prompt
from app.prompts.tender_scope_doc import build_tender_scope_doc_prompt
from app.prompts.trade_scopes import build_trade_scopes_prompt
from app.vectorstore.base import VectorStore

logger = get_logger(__name__)


class AnalysisState(TypedDict):
    """State for analysis pipeline."""

    # Input
    project_id: str
    document_text: str | None
    instructions: str | None
    analysis_type: str  # summary, trade_scopes, tender_scope_doc

    # For trade scope extraction
    trades: list[str] | None

    # For tender scope doc
    trade: str | None
    scope_data: dict | None
    project_context: str | None
    bid_due_date: str | None

    # Output
    result: dict | None
    status: str
    error: str | None


class AnalysisPipeline:
    """
    Document analysis pipeline using LangGraph.

    Supports multiple analysis types:
    - summary: Generate project summary
    - trade_scopes: Extract trade-specific scope
    - tender_scope_doc: Generate tender scope document
    """

    def __init__(
        self,
        gemini_client: GeminiClient,
        vector_store: VectorStore | None = None,
    ) -> None:
        self.gemini = gemini_client
        self.vector_store = vector_store
        self.graph = self._build_graph()

    def _build_graph(self) -> StateGraph:
        """Build the analysis graph."""
        graph = StateGraph(AnalysisState)

        # Add nodes
        graph.add_node("route_analysis", self._route_analysis)
        graph.add_node("generate_summary", self._generate_summary)
        graph.add_node("extract_trade_scopes", self._extract_trade_scopes)
        graph.add_node("generate_tender_doc", self._generate_tender_doc)
        graph.add_node("handle_error", self._handle_error)

        # Set entry point
        graph.set_entry_point("route_analysis")

        # Add conditional routing
        graph.add_conditional_edges(
            "route_analysis",
            self._get_analysis_type,
            {
                "summary": "generate_summary",
                "trade_scopes": "extract_trade_scopes",
                "tender_scope_doc": "generate_tender_doc",
                "error": "handle_error",
            },
        )

        # All analysis nodes go to END
        graph.add_edge("generate_summary", END)
        graph.add_edge("extract_trade_scopes", END)
        graph.add_edge("generate_tender_doc", END)
        graph.add_edge("handle_error", END)

        return graph.compile()

    async def _route_analysis(self, state: AnalysisState) -> dict[str, Any]:
        """Validate input and prepare for routing."""
        logger.info(
            "Routing analysis",
            project_id=state["project_id"],
            analysis_type=state["analysis_type"],
        )

        # Validate we have document text for text-based analysis
        if state["analysis_type"] in ["summary", "trade_scopes"]:
            if not state.get("document_text"):
                return {
                    "status": "failed",
                    "error": "document_text is required for this analysis type",
                }

        # Validate tender doc requirements
        if state["analysis_type"] == "tender_scope_doc":
            if not state.get("trade") or not state.get("scope_data"):
                return {
                    "status": "failed",
                    "error": "trade and scope_data are required for tender_scope_doc",
                }

        return {"status": "processing"}

    def _get_analysis_type(self, state: AnalysisState) -> str:
        """Determine which analysis to run."""
        if state.get("status") == "failed":
            return "error"
        return state["analysis_type"]

    async def _generate_summary(self, state: AnalysisState) -> dict[str, Any]:
        """Generate project summary from document text."""
        logger.info("Generating plan summary", project_id=state["project_id"])

        try:
            prompt = build_plan_summary_prompt(
                document_text=state["document_text"],
                instructions=state.get("instructions"),
            )

            config = GenerationConfig(
                temperature=0.3,  # Lower for more consistent output
                max_output_tokens=4096,
            )

            result = await self.gemini.generate_structured(
                prompt,
                PlanSummary,
                config=config,
            )

            logger.info(
                "Summary generated",
                project_id=state["project_id"],
                confidence=result.confidence,
            )

            return {
                "result": result.model_dump(),
                "status": "completed",
            }

        except Exception as e:
            logger.error("Summary generation failed", error=str(e))
            return {
                "status": "failed",
                "error": f"Summary generation failed: {str(e)}",
            }

    async def _extract_trade_scopes(self, state: AnalysisState) -> dict[str, Any]:
        """Extract trade-specific scope from document."""
        logger.info(
            "Extracting trade scopes",
            project_id=state["project_id"],
            trades=state.get("trades"),
        )

        try:
            prompt = build_trade_scopes_prompt(
                document_text=state["document_text"],
                trades=state.get("trades"),
                project_id=state["project_id"],
            )

            config = GenerationConfig(
                temperature=0.3,
                max_output_tokens=8192,
            )

            result = await self.gemini.generate_structured(
                prompt,
                TradeScopesOutput,
                config=config,
            )

            logger.info(
                "Trade scopes extracted",
                project_id=state["project_id"],
                trade_count=len(result.trades),
            )

            return {
                "result": result.model_dump(),
                "status": "completed",
            }

        except Exception as e:
            logger.error("Trade scope extraction failed", error=str(e))
            return {
                "status": "failed",
                "error": f"Trade scope extraction failed: {str(e)}",
            }

    async def _generate_tender_doc(self, state: AnalysisState) -> dict[str, Any]:
        """Generate tender scope document."""
        logger.info(
            "Generating tender scope document",
            project_id=state["project_id"],
            trade=state["trade"],
        )

        try:
            prompt = build_tender_scope_doc_prompt(
                trade=state["trade"],
                scope_data=state["scope_data"],
                project_context=state.get("project_context"),
                bid_due_date=state.get("bid_due_date"),
            )

            config = GenerationConfig(
                temperature=0.4,
                max_output_tokens=8192,
            )

            result = await self.gemini.generate_structured(
                prompt,
                TenderScopeDoc,
                config=config,
            )

            logger.info(
                "Tender doc generated",
                project_id=state["project_id"],
                trade=state["trade"],
            )

            return {
                "result": result.model_dump(),
                "status": "completed",
            }

        except Exception as e:
            logger.error("Tender doc generation failed", error=str(e))
            return {
                "status": "failed",
                "error": f"Tender doc generation failed: {str(e)}",
            }

    async def _handle_error(self, state: AnalysisState) -> dict[str, Any]:
        """Handle pipeline errors."""
        logger.error(
            "Analysis error",
            project_id=state["project_id"],
            error=state.get("error"),
        )
        return {"status": "failed"}

    async def run_summary(
        self,
        project_id: str,
        document_text: str,
        instructions: str | None = None,
    ) -> dict[str, Any]:
        """Run plan summary analysis."""
        state: AnalysisState = {
            "project_id": project_id,
            "document_text": document_text,
            "instructions": instructions,
            "analysis_type": "summary",
            "trades": None,
            "trade": None,
            "scope_data": None,
            "project_context": None,
            "bid_due_date": None,
            "result": None,
            "status": "pending",
            "error": None,
        }
        return await self.graph.ainvoke(state)

    async def run_trade_scopes(
        self,
        project_id: str,
        document_text: str,
        trades: list[str] | None = None,
    ) -> dict[str, Any]:
        """Run trade scope extraction."""
        state: AnalysisState = {
            "project_id": project_id,
            "document_text": document_text,
            "instructions": None,
            "analysis_type": "trade_scopes",
            "trades": trades,
            "trade": None,
            "scope_data": None,
            "project_context": None,
            "bid_due_date": None,
            "result": None,
            "status": "pending",
            "error": None,
        }
        return await self.graph.ainvoke(state)

    async def run_tender_doc(
        self,
        project_id: str,
        trade: str,
        scope_data: dict,
        project_context: str | None = None,
        bid_due_date: str | None = None,
    ) -> dict[str, Any]:
        """Generate tender scope document."""
        state: AnalysisState = {
            "project_id": project_id,
            "document_text": None,
            "instructions": None,
            "analysis_type": "tender_scope_doc",
            "trades": None,
            "trade": trade,
            "scope_data": scope_data,
            "project_context": project_context,
            "bid_due_date": bid_due_date,
            "result": None,
            "status": "pending",
            "error": None,
        }
        return await self.graph.ainvoke(state)


def create_analysis_graph(
    gemini_client: GeminiClient,
    vector_store: VectorStore | None = None,
) -> AnalysisPipeline:
    """Factory function to create an analysis pipeline."""
    return AnalysisPipeline(gemini_client, vector_store)
