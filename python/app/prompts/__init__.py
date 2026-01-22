"""Prompt templates for Gemini LLM interactions."""

from app.prompts.plan_summary import PLAN_SUMMARY_PROMPT, build_plan_summary_prompt
from app.prompts.qna import QNA_PROMPT, build_qna_prompt
from app.prompts.tender_scope_doc import TENDER_SCOPE_DOC_PROMPT, build_tender_scope_doc_prompt
from app.prompts.trade_scopes import TRADE_SCOPES_PROMPT, build_trade_scopes_prompt
from app.prompts.vision_ocr import VISION_OCR_PROMPT, build_vision_ocr_prompt
from app.prompts.materials import (
    MATERIALS_EXTRACTION_PROMPT,
    MATERIALS_AGGREGATION_PROMPT,
    build_materials_prompt,
    build_materials_aggregation_prompt,
)
from app.prompts.rooms import (
    ROOMS_EXTRACTION_PROMPT,
    ROOMS_AGGREGATION_PROMPT,
    build_rooms_prompt,
    build_rooms_aggregation_prompt,
    normalize_room_type,
    ROOM_TYPE_MAPPINGS,
)
from app.prompts.milestones import (
    MILESTONES_EXTRACTION_PROMPT,
    MILESTONES_INFERENCE_PROMPT,
    build_milestones_prompt,
    build_milestones_inference_prompt,
    get_standard_phases,
    estimate_duration_by_building_type,
    STANDARD_PHASES,
)

__all__ = [
    # Plan summary
    "PLAN_SUMMARY_PROMPT",
    "build_plan_summary_prompt",
    # Trade scopes
    "TRADE_SCOPES_PROMPT",
    "build_trade_scopes_prompt",
    # Tender scope doc
    "TENDER_SCOPE_DOC_PROMPT",
    "build_tender_scope_doc_prompt",
    # Q&A
    "QNA_PROMPT",
    "build_qna_prompt",
    # Vision OCR
    "VISION_OCR_PROMPT",
    "build_vision_ocr_prompt",
    # Materials extraction
    "MATERIALS_EXTRACTION_PROMPT",
    "MATERIALS_AGGREGATION_PROMPT",
    "build_materials_prompt",
    "build_materials_aggregation_prompt",
    # Rooms extraction
    "ROOMS_EXTRACTION_PROMPT",
    "ROOMS_AGGREGATION_PROMPT",
    "build_rooms_prompt",
    "build_rooms_aggregation_prompt",
    "normalize_room_type",
    "ROOM_TYPE_MAPPINGS",
    # Milestones extraction
    "MILESTONES_EXTRACTION_PROMPT",
    "MILESTONES_INFERENCE_PROMPT",
    "build_milestones_prompt",
    "build_milestones_inference_prompt",
    "get_standard_phases",
    "estimate_duration_by_building_type",
    "STANDARD_PHASES",
]
