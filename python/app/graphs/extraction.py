"""LangGraph pipeline for blueprint extraction: Materials, Rooms, Milestones, Trade Scopes."""

import json
import time
from datetime import datetime
from typing import Any, Callable

from langgraph.graph import END, StateGraph

from app.gemini.client import GeminiClient
from app.gemini.schemas import VisionOCRResult
from app.logging import get_logger
from app.prompts.materials import build_materials_prompt, build_materials_aggregation_prompt
from app.prompts.milestones import build_milestones_prompt, build_milestones_inference_prompt
from app.prompts.rooms import build_rooms_prompt, build_rooms_aggregation_prompt
from app.prompts.trade_scopes import build_trade_scopes_prompt
from app.schemas.extraction import (
    ExtractionState,
    ExtractionStepKey,
    ExtractionStepState,
    ExtractedMaterialItem,
    ExtractedMilestoneItem,
    ExtractedRoomItem,
    ExtractedTradeScopeItem,
    JobCompletedEvent,
    JobFailedEvent,
    JobStatusChangedEvent,
    ScopeItem,
    StepCompletedEvent,
    StepFailedEvent,
    StepProgressEvent,
    StepStartedEvent,
    StepStatus,
)

logger = get_logger(__name__)


class ExtractionPipeline:
    """
    Blueprint extraction pipeline using LangGraph.

    Pipeline stages:
    1. extract_materials: Extract materials from OCR results
    2. extract_rooms: Extract rooms and spaces
    3. extract_milestones: Extract project milestones
    4. extract_trade_scopes: Extract trade scopes
    """

    def __init__(
        self,
        gemini_client: GeminiClient,
        progress_callback: Callable[[Any], None] | None = None,
    ) -> None:
        self.gemini = gemini_client
        self.progress_callback = progress_callback
        self.graph = self._build_graph()

    def _emit_event(self, event: Any) -> None:
        """Emit a progress event if callback is set."""
        if self.progress_callback:
            try:
                self.progress_callback(event)
            except Exception as e:
                logger.warning("Failed to emit progress event", error=str(e))

    def _build_graph(self) -> StateGraph:
        """Build the LangGraph state machine."""
        graph = StateGraph(ExtractionState)

        # Add nodes
        graph.add_node("extract_materials", self._extract_materials)
        graph.add_node("extract_rooms", self._extract_rooms)
        graph.add_node("extract_milestones", self._extract_milestones)
        graph.add_node("extract_trade_scopes", self._extract_trade_scopes)
        graph.add_node("handle_error", self._handle_error)

        # Set entry point
        graph.set_entry_point("extract_materials")

        # Add edges - sequential execution with error handling
        graph.add_conditional_edges(
            "extract_materials",
            self._check_step,
            {
                "success": "extract_rooms",
                "error": "handle_error",
            },
        )

        graph.add_conditional_edges(
            "extract_rooms",
            self._check_step,
            {
                "success": "extract_milestones",
                "error": "handle_error",
            },
        )

        graph.add_conditional_edges(
            "extract_milestones",
            self._check_step,
            {
                "success": "extract_trade_scopes",
                "error": "handle_error",
            },
        )

        graph.add_conditional_edges(
            "extract_trade_scopes",
            self._check_step,
            {
                "success": END,
                "error": "handle_error",
            },
        )

        graph.add_edge("handle_error", END)

        return graph.compile()

    def _check_step(self, state: ExtractionState) -> str:
        """Check if the current step succeeded."""
        if state.status == "failed":
            return "error"
        return "success"

    def _update_step_state(
        self,
        state: ExtractionState,
        step_key: ExtractionStepKey,
        **updates: Any,
    ) -> None:
        """Update the state for a specific step."""
        for step in state.steps:
            if step.step_key == step_key:
                for key, value in updates.items():
                    setattr(step, key, value)
                break

    def _get_ocr_text(self, state: ExtractionState) -> str:
        """Get combined OCR text from all pages."""
        texts = []
        for ocr in state.ocr_results:
            if isinstance(ocr, dict):
                text = ocr.get("text_content", "")
                page = ocr.get("page_number", 0)
            elif isinstance(ocr, VisionOCRResult):
                text = ocr.text_content
                page = ocr.page_number
            else:
                continue

            if text:
                texts.append(f"--- Page {page} ---\n{text}")

        return "\n\n".join(texts)

    # =========================================================================
    # Material Extraction
    # =========================================================================

    async def _extract_materials(self, state: ExtractionState) -> dict[str, Any]:
        """Extract materials from OCR results."""
        step_key = ExtractionStepKey.MATERIALS
        start_time = time.time()

        logger.info(
            "Starting material extraction",
            job_id=state.job_id,
            pages=len(state.ocr_results),
        )

        # Emit step started event
        self._emit_event(
            StepStartedEvent(
                job_id=state.job_id,
                step_key=step_key.value,
                step_name="Material Takeoff",
                step_order=1,
            )
        )

        try:
            all_materials = []
            total_pages = len(state.ocr_results)

            # Process each page
            for i, ocr in enumerate(state.ocr_results):
                if isinstance(ocr, dict):
                    text = ocr.get("text_content", "")
                    page = ocr.get("page_number", i + 1)
                else:
                    text = ocr.text_content
                    page = ocr.page_number

                if not text or len(text.strip()) < 50:
                    continue

                # Build prompt and call Gemini
                prompt = build_materials_prompt(
                    document_text=text,
                    page_number=page,
                    document_id=state.document_id,
                    project_id=state.project_id,
                )

                try:
                    response = await self.gemini.generate_json(prompt)
                    materials = response.get("materials", [])

                    for mat in materials:
                        item = ExtractedMaterialItem(
                            name=mat.get("name", "Unknown"),
                            description=mat.get("description"),
                            quantity=mat.get("quantity"),
                            unit=mat.get("unit"),
                            location=mat.get("location"),
                            room=mat.get("room"),
                            specification=mat.get("specification"),
                            trade_category=mat.get("trade_category"),
                            csi_division=mat.get("csi_division"),
                            source_page=mat.get("source_page", page),
                            confidence=mat.get("confidence", 0.5),
                        )
                        all_materials.append(item)

                except Exception as e:
                    logger.warning(
                        "Material extraction failed for page",
                        page=page,
                        error=str(e),
                    )

                # Emit progress
                progress = (i + 1) / total_pages
                self._emit_event(
                    StepProgressEvent(
                        job_id=state.job_id,
                        step_key=step_key.value,
                        progress=progress,
                        items_processed=i + 1,
                        items_total=total_pages,
                        message=f"Processed page {page}",
                    )
                )

            duration_ms = int((time.time() - start_time) * 1000)

            logger.info(
                "Material extraction complete",
                job_id=state.job_id,
                materials_found=len(all_materials),
                duration_ms=duration_ms,
            )

            # Emit step completed
            self._emit_event(
                StepCompletedEvent(
                    job_id=state.job_id,
                    step_key=step_key.value,
                    duration_ms=duration_ms,
                )
            )

            return {
                "materials": all_materials,
                "current_step": ExtractionStepKey.ROOMS,
                "progress": 0.25,
            }

        except Exception as e:
            logger.error("Material extraction failed", error=str(e))

            self._emit_event(
                StepFailedEvent(
                    job_id=state.job_id,
                    step_key=step_key.value,
                    error=str(e),
                    can_retry=True,
                )
            )

            return {
                "status": "failed",
                "error": f"Material extraction failed: {str(e)}",
            }

    # =========================================================================
    # Room Extraction
    # =========================================================================

    async def _extract_rooms(self, state: ExtractionState) -> dict[str, Any]:
        """Extract rooms from OCR results."""
        step_key = ExtractionStepKey.ROOMS
        start_time = time.time()

        logger.info(
            "Starting room extraction",
            job_id=state.job_id,
            pages=len(state.ocr_results),
        )

        self._emit_event(
            StepStartedEvent(
                job_id=state.job_id,
                step_key=step_key.value,
                step_name="Room Breakdown",
                step_order=2,
            )
        )

        try:
            all_rooms = []
            finish_legends: dict[str, str] = {}
            total_pages = len(state.ocr_results)

            for i, ocr in enumerate(state.ocr_results):
                if isinstance(ocr, dict):
                    text = ocr.get("text_content", "")
                    page = ocr.get("page_number", i + 1)
                else:
                    text = ocr.text_content
                    page = ocr.page_number

                if not text or len(text.strip()) < 50:
                    continue

                prompt = build_rooms_prompt(
                    document_text=text,
                    page_number=page,
                    document_id=state.document_id,
                    project_id=state.project_id,
                )

                try:
                    response = await self.gemini.generate_json(prompt)
                    rooms = response.get("rooms", [])

                    # Merge finish legend
                    legend = response.get("finish_legend", {})
                    finish_legends.update(legend)

                    for room in rooms:
                        finishes = room.get("finishes", {})
                        from app.schemas.extraction import RoomFinishes

                        item = ExtractedRoomItem(
                            room_name=room.get("room_name", "Unknown"),
                            room_number=room.get("room_number"),
                            room_type=room.get("room_type"),
                            floor=room.get("floor"),
                            area_sqft=room.get("area_sqft"),
                            ceiling_height=room.get("ceiling_height"),
                            perimeter_ft=room.get("perimeter_ft"),
                            finishes=RoomFinishes(
                                floor=finishes.get("floor"),
                                walls=finishes.get("walls"),
                                ceiling=finishes.get("ceiling"),
                                base=finishes.get("base"),
                                paint_color=finishes.get("paint_color"),
                            ),
                            fixtures=room.get("fixtures", []),
                            notes=room.get("notes"),
                            source_page=room.get("source_page", page),
                            confidence=room.get("confidence", 0.5),
                        )
                        all_rooms.append(item)

                except Exception as e:
                    logger.warning(
                        "Room extraction failed for page",
                        page=page,
                        error=str(e),
                    )

                progress = (i + 1) / total_pages
                self._emit_event(
                    StepProgressEvent(
                        job_id=state.job_id,
                        step_key=step_key.value,
                        progress=progress,
                        items_processed=i + 1,
                        items_total=total_pages,
                        message=f"Processed page {page}",
                    )
                )

            duration_ms = int((time.time() - start_time) * 1000)

            logger.info(
                "Room extraction complete",
                job_id=state.job_id,
                rooms_found=len(all_rooms),
                duration_ms=duration_ms,
            )

            self._emit_event(
                StepCompletedEvent(
                    job_id=state.job_id,
                    step_key=step_key.value,
                    duration_ms=duration_ms,
                )
            )

            return {
                "rooms": all_rooms,
                "current_step": ExtractionStepKey.MILESTONES,
                "progress": 0.5,
            }

        except Exception as e:
            logger.error("Room extraction failed", error=str(e))

            self._emit_event(
                StepFailedEvent(
                    job_id=state.job_id,
                    step_key=step_key.value,
                    error=str(e),
                    can_retry=True,
                )
            )

            return {
                "status": "failed",
                "error": f"Room extraction failed: {str(e)}",
            }

    # =========================================================================
    # Milestone Extraction
    # =========================================================================

    async def _extract_milestones(self, state: ExtractionState) -> dict[str, Any]:
        """Extract milestones from document content."""
        step_key = ExtractionStepKey.MILESTONES
        start_time = time.time()

        logger.info(
            "Starting milestone extraction",
            job_id=state.job_id,
        )

        self._emit_event(
            StepStartedEvent(
                job_id=state.job_id,
                step_key=step_key.value,
                step_name="Project Milestones",
                step_order=3,
            )
        )

        try:
            # Combine OCR text for milestone analysis
            combined_text = self._get_ocr_text(state)

            # Build prompt
            prompt = build_milestones_prompt(
                document_text=combined_text,
                document_type="construction drawings",
                project_id=state.project_id,
            )

            self._emit_event(
                StepProgressEvent(
                    job_id=state.job_id,
                    step_key=step_key.value,
                    progress=0.3,
                    items_processed=0,
                    items_total=1,
                    message="Analyzing document for milestones...",
                )
            )

            response = await self.gemini.generate_json(prompt)
            milestones_data = response.get("milestones", [])

            all_milestones = []
            for ms in milestones_data:
                item = ExtractedMilestoneItem(
                    name=ms.get("name", "Unknown"),
                    description=ms.get("description"),
                    phase=ms.get("phase"),
                    phase_order=ms.get("phase_order", 0),
                    estimated_duration_days=ms.get("estimated_duration_days"),
                    dependencies=ms.get("dependencies", []),
                    trades_involved=ms.get("trades_involved", []),
                    deliverables=ms.get("deliverables", []),
                    confidence=ms.get("confidence", 0.5),
                )
                all_milestones.append(item)

            duration_ms = int((time.time() - start_time) * 1000)

            logger.info(
                "Milestone extraction complete",
                job_id=state.job_id,
                milestones_found=len(all_milestones),
                duration_ms=duration_ms,
            )

            self._emit_event(
                StepCompletedEvent(
                    job_id=state.job_id,
                    step_key=step_key.value,
                    duration_ms=duration_ms,
                )
            )

            return {
                "milestones": all_milestones,
                "current_step": ExtractionStepKey.TRADE_SCOPES,
                "progress": 0.75,
            }

        except Exception as e:
            logger.error("Milestone extraction failed", error=str(e))

            self._emit_event(
                StepFailedEvent(
                    job_id=state.job_id,
                    step_key=step_key.value,
                    error=str(e),
                    can_retry=True,
                )
            )

            return {
                "status": "failed",
                "error": f"Milestone extraction failed: {str(e)}",
            }

    # =========================================================================
    # Trade Scope Extraction
    # =========================================================================

    async def _extract_trade_scopes(self, state: ExtractionState) -> dict[str, Any]:
        """Extract trade scopes from document content."""
        step_key = ExtractionStepKey.TRADE_SCOPES
        start_time = time.time()

        logger.info(
            "Starting trade scope extraction",
            job_id=state.job_id,
        )

        self._emit_event(
            StepStartedEvent(
                job_id=state.job_id,
                step_key=step_key.value,
                step_name="Trade Scopes",
                step_order=4,
            )
        )

        try:
            combined_text = self._get_ocr_text(state)

            prompt = build_trade_scopes_prompt(
                document_text=combined_text,
                project_id=state.project_id,
            )

            self._emit_event(
                StepProgressEvent(
                    job_id=state.job_id,
                    step_key=step_key.value,
                    progress=0.3,
                    items_processed=0,
                    items_total=1,
                    message="Extracting trade scopes...",
                )
            )

            response = await self.gemini.generate_json(prompt)
            trades_data = response.get("trades", [])

            all_scopes = []
            for trade in trades_data:
                # Convert inclusions/exclusions to ScopeItem format
                inclusions = []
                for inc in trade.get("inclusions", []):
                    if isinstance(inc, str):
                        inclusions.append(ScopeItem(item=inc))
                    elif isinstance(inc, dict):
                        inclusions.append(ScopeItem(
                            item=inc.get("item", str(inc)),
                            details=inc.get("details"),
                        ))

                exclusions = []
                for exc in trade.get("exclusions", []):
                    if isinstance(exc, str):
                        exclusions.append(ScopeItem(item=exc))
                    elif isinstance(exc, dict):
                        exclusions.append(ScopeItem(
                            item=exc.get("item", str(exc)),
                            details=exc.get("details"),
                        ))

                item = ExtractedTradeScopeItem(
                    trade=trade.get("trade", "Unknown"),
                    trade_display_name=trade.get("trade"),
                    csi_division=trade.get("csi_division"),
                    inclusions=inclusions,
                    exclusions=exclusions,
                    required_sheets=trade.get("required_sheets", []),
                    spec_sections=trade.get("spec_sections", []),
                    rfi_needed=trade.get("rfi_needed", []),
                    assumptions=trade.get("assumptions", []),
                    confidence=trade.get("confidence", 0.5),
                )
                all_scopes.append(item)

            duration_ms = int((time.time() - start_time) * 1000)

            logger.info(
                "Trade scope extraction complete",
                job_id=state.job_id,
                scopes_found=len(all_scopes),
                duration_ms=duration_ms,
            )

            self._emit_event(
                StepCompletedEvent(
                    job_id=state.job_id,
                    step_key=step_key.value,
                    duration_ms=duration_ms,
                )
            )

            # Calculate total duration
            total_duration_ms = int((time.time() - (state.started_at or datetime.utcnow()).timestamp()) * 1000)

            # Emit job completed
            self._emit_event(
                JobCompletedEvent(
                    job_id=state.job_id,
                    duration_ms=total_duration_ms,
                    results_summary={
                        "materials_count": len(state.materials),
                        "rooms_count": len(state.rooms),
                        "milestones_count": len(state.milestones),
                        "trade_scopes_count": len(all_scopes),
                    },
                )
            )

            return {
                "trade_scopes": all_scopes,
                "status": "completed",
                "progress": 1.0,
                "completed_at": datetime.utcnow(),
            }

        except Exception as e:
            logger.error("Trade scope extraction failed", error=str(e))

            self._emit_event(
                StepFailedEvent(
                    job_id=state.job_id,
                    step_key=step_key.value,
                    error=str(e),
                    can_retry=True,
                )
            )

            return {
                "status": "failed",
                "error": f"Trade scope extraction failed: {str(e)}",
            }

    # =========================================================================
    # Error Handler
    # =========================================================================

    async def _handle_error(self, state: ExtractionState) -> dict[str, Any]:
        """Handle pipeline errors."""
        logger.error(
            "Extraction pipeline error",
            job_id=state.job_id,
            error=state.error,
            current_step=state.current_step,
        )

        self._emit_event(
            JobFailedEvent(
                job_id=state.job_id,
                error=state.error or "Unknown error",
                failed_step=state.current_step.value if state.current_step else None,
                can_retry=state.can_retry,
            )
        )

        return {
            "status": "failed",
        }

    # =========================================================================
    # Public API
    # =========================================================================

    async def run(
        self,
        job_id: str,
        project_id: str,
        ocr_results: list[dict[str, Any] | VisionOCRResult],
        document_id: str | None = None,
    ) -> ExtractionState:
        """
        Run the extraction pipeline.

        Args:
            job_id: Unique job identifier
            project_id: Project this document belongs to
            ocr_results: List of OCR results from ingestion pipeline
            document_id: Optional document identifier

        Returns:
            Final pipeline state with all extractions
        """
        # Initialize step states
        steps = [
            ExtractionStepState(
                step_key=ExtractionStepKey.MATERIALS,
                step_name="Material Takeoff",
                step_order=1,
            ),
            ExtractionStepState(
                step_key=ExtractionStepKey.ROOMS,
                step_name="Room Breakdown",
                step_order=2,
            ),
            ExtractionStepState(
                step_key=ExtractionStepKey.MILESTONES,
                step_name="Project Milestones",
                step_order=3,
            ),
            ExtractionStepState(
                step_key=ExtractionStepKey.TRADE_SCOPES,
                step_name="Trade Scopes",
                step_order=4,
            ),
        ]

        initial_state = ExtractionState(
            job_id=job_id,
            project_id=project_id,
            document_id=document_id,
            status="running",
            current_step=ExtractionStepKey.MATERIALS,
            progress=0.0,
            steps=steps,
            ocr_results=[
                r.model_dump() if isinstance(r, VisionOCRResult) else r
                for r in ocr_results
            ],
            started_at=datetime.utcnow(),
        )

        logger.info(
            "Starting extraction pipeline",
            job_id=job_id,
            project_id=project_id,
            pages=len(ocr_results),
        )

        # Emit job started
        self._emit_event(
            JobStatusChangedEvent(
                job_id=job_id,
                status="running",
                progress=0.0,
                current_step=ExtractionStepKey.MATERIALS.value,
            )
        )

        result = await self.graph.ainvoke(initial_state)
        return result


def create_extraction_pipeline(
    gemini_client: GeminiClient,
    progress_callback: Callable[[Any], None] | None = None,
) -> ExtractionPipeline:
    """Factory function to create an extraction pipeline."""
    return ExtractionPipeline(gemini_client, progress_callback)
