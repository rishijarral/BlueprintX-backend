"""Pydantic schemas for Gemini API interactions."""

from enum import Enum
from typing import Any

from pydantic import BaseModel, Field


class GeminiModel(str, Enum):
    """Available Gemini models."""

    FLASH = "gemini-3-flash-preview"
    FLASH_LITE = "gemini-3-flash-preview-lite"
    PRO = "gemini-1.5-pro"
    PRO_LATEST = "gemini-1.5-pro-latest"
    FLASH_8B = "gemini-1.5-flash-8b"


class GenerationConfig(BaseModel):
    """Configuration for text generation."""

    temperature: float = Field(default=0.7, ge=0.0, le=2.0)
    top_p: float = Field(default=0.95, ge=0.0, le=1.0)
    top_k: int = Field(default=40, ge=1)
    max_output_tokens: int = Field(default=8192, ge=1, le=65536)
    response_mime_type: str | None = None
    response_schema: dict[str, Any] | None = None


class VisionInput(BaseModel):
    """Input for vision-based analysis."""

    image_bytes: bytes
    mime_type: str = "image/png"
    prompt: str


class GeminiResponse(BaseModel):
    """Response from Gemini API."""

    text: str
    model: str
    finish_reason: str | None = None
    usage: dict[str, int] | None = None


class EmbeddingResponse(BaseModel):
    """Response from embedding API."""

    embedding: list[float]
    model: str


# ============================================================================
# Structured output schemas for plan analysis
# ============================================================================


class PlanSummary(BaseModel):
    """Structured plan summary output."""

    building_type: str = Field(description="Type of building (e.g., commercial, residential)")
    project_name: str | None = Field(default=None, description="Name of the project if identified")
    floors: int | None = Field(default=None, description="Number of floors")
    total_area_sqft: int | None = Field(default=None, description="Total area in square feet")
    key_materials: list[str] = Field(default_factory=list, description="Primary construction materials")
    major_systems: list[str] = Field(default_factory=list, description="Major building systems identified")
    structural_system: str | None = Field(default=None, description="Primary structural system")
    risks: list[str] = Field(default_factory=list, description="Identified risks or concerns")
    assumptions: list[str] = Field(default_factory=list, description="Assumptions made during analysis")
    confidence: float = Field(default=0.5, ge=0.0, le=1.0, description="Confidence score")


class TradeScopeItem(BaseModel):
    """Scope item for a single trade."""

    trade: str = Field(description="Trade name (e.g., Electrical, Plumbing)")
    csi_division: str | None = Field(default=None, description="CSI division number")
    inclusions: list[str] = Field(default_factory=list, description="Work items included")
    exclusions: list[str] = Field(default_factory=list, description="Work items excluded")
    required_sheets: list[str] = Field(default_factory=list, description="Referenced drawing sheets")
    spec_sections: list[str] = Field(default_factory=list, description="Referenced specification sections")
    rfi_needed: list[str] = Field(default_factory=list, description="Items needing clarification")
    assumptions: list[str] = Field(default_factory=list, description="Assumptions made")


class TradeScopesOutput(BaseModel):
    """Output for trade scope extraction."""

    project_id: str | None = None
    trades: list[TradeScopeItem] = Field(default_factory=list)
    general_notes: list[str] = Field(default_factory=list)
    confidence: float = Field(default=0.5, ge=0.0, le=1.0)


class TenderScopeDoc(BaseModel):
    """Generated tender scope document."""

    trade: str
    overview: str
    inclusions: list[str]
    exclusions: list[str]
    allowances: list[str] = Field(default_factory=list)
    alternates: list[str] = Field(default_factory=list)
    submittals: list[str] = Field(default_factory=list)
    schedule_notes: list[str] = Field(default_factory=list)
    lead_times: list[str] = Field(default_factory=list)
    bid_instructions: list[str] = Field(default_factory=list)
    rfi_questions: list[str] = Field(default_factory=list)
    markdown: str = Field(description="Full document in Markdown format")


class QnAResponse(BaseModel):
    """Response for Q&A queries."""

    answer: str
    citations: list[str] = Field(default_factory=list, description="Source citations")
    confidence: float = Field(default=0.5, ge=0.0, le=1.0)
    followups: list[str] = Field(default_factory=list, description="Suggested follow-up questions")


class VisionOCRResult(BaseModel):
    """Result from vision OCR on a drawing page."""

    page_number: int
    sheet_number: str | None = Field(default=None, description="Drawing sheet number if found")
    sheet_title: str | None = Field(default=None, description="Drawing title if found")
    drawing_type: str | None = Field(default=None, description="Type of drawing (plan, section, detail, etc.)")
    discipline: str | None = Field(default=None, description="Discipline (architectural, structural, MEP, etc.)")
    text_content: str = Field(description="Extracted text content")
    annotations: list[str] = Field(default_factory=list, description="Annotations and callouts")
    dimensions: list[str] = Field(default_factory=list, description="Key dimensions found")
    notes: list[str] = Field(default_factory=list, description="Drawing notes")
    materials: list[str] = Field(default_factory=list, description="Materials mentioned")
    references: list[str] = Field(default_factory=list, description="References to other drawings")
