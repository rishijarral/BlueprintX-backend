"""Prompt templates for Gemini LLM interactions."""

from app.prompts.plan_summary import PLAN_SUMMARY_PROMPT, build_plan_summary_prompt
from app.prompts.qna import QNA_PROMPT, build_qna_prompt
from app.prompts.tender_scope_doc import TENDER_SCOPE_DOC_PROMPT, build_tender_scope_doc_prompt
from app.prompts.trade_scopes import TRADE_SCOPES_PROMPT, build_trade_scopes_prompt
from app.prompts.vision_ocr import VISION_OCR_PROMPT, build_vision_ocr_prompt

__all__ = [
    "PLAN_SUMMARY_PROMPT",
    "build_plan_summary_prompt",
    "TRADE_SCOPES_PROMPT",
    "build_trade_scopes_prompt",
    "TENDER_SCOPE_DOC_PROMPT",
    "build_tender_scope_doc_prompt",
    "QNA_PROMPT",
    "build_qna_prompt",
    "VISION_OCR_PROMPT",
    "build_vision_ocr_prompt",
]
