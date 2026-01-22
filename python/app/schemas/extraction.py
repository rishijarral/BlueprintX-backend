"""Pydantic schemas for AI extraction pipelines."""

from datetime import datetime
from enum import Enum
from typing import Any

from pydantic import BaseModel, Field


# ============================================================================
# Enums
# ============================================================================


class ExtractionStepKey(str, Enum):
    """Keys for extraction pipeline steps."""

    PARSING = "parsing"
    OCR = "ocr"
    MATERIALS = "materials"
    ROOMS = "rooms"
    MILESTONES = "milestones"
    TRADE_SCOPES = "trade_scopes"
    EMBEDDING = "embedding"


class StepStatus(str, Enum):
    """Status for individual extraction steps."""

    PENDING = "pending"
    RUNNING = "running"
    COMPLETED = "completed"
    FAILED = "failed"
    SKIPPED = "skipped"


# ============================================================================
# Base Extraction Models
# ============================================================================


class RoomFinishes(BaseModel):
    """Finish specifications for a room."""

    floor: str | None = Field(default=None, description="Floor finish type")
    walls: str | None = Field(default=None, description="Wall finish type")
    ceiling: str | None = Field(default=None, description="Ceiling finish type")
    base: str | None = Field(default=None, description="Base/baseboard type")
    paint_color: str | None = Field(default=None, description="Paint color or code")


class ScopeItem(BaseModel):
    """A scope inclusion or exclusion item."""

    item: str = Field(description="The scope item description")
    details: str | None = Field(default=None, description="Additional details")


# ============================================================================
# Material Extraction
# ============================================================================


class ExtractedMaterialItem(BaseModel):
    """A single extracted material from blueprints."""

    name: str = Field(description="Material name")
    description: str | None = Field(default=None, description="Material description")
    quantity: float | None = Field(default=None, ge=0, description="Quantity extracted")
    unit: str | None = Field(default=None, description="Unit of measurement")
    location: str | None = Field(default=None, description="Location in building")
    room: str | None = Field(default=None, description="Room or space name")
    specification: str | None = Field(default=None, description="Spec section or details")
    trade_category: str | None = Field(default=None, description="Trade category")
    csi_division: str | None = Field(default=None, description="CSI division code")
    source_page: int | None = Field(default=None, ge=1, description="Source page number")
    confidence: float = Field(default=0.5, ge=0.0, le=1.0, description="Extraction confidence")


class MaterialsExtractionOutput(BaseModel):
    """Output from material extraction pipeline."""

    project_id: str
    document_id: str | None = None
    materials: list[ExtractedMaterialItem] = Field(default_factory=list)
    total_count: int = Field(default=0, ge=0)
    extraction_notes: list[str] = Field(default_factory=list)
    confidence: float = Field(default=0.5, ge=0.0, le=1.0)


# ============================================================================
# Room Extraction
# ============================================================================


class ExtractedRoomItem(BaseModel):
    """A single extracted room from blueprints."""

    room_name: str = Field(description="Room name")
    room_number: str | None = Field(default=None, description="Room number or ID")
    room_type: str | None = Field(default=None, description="Room type classification")
    floor: str | None = Field(default=None, description="Floor level")
    area_sqft: float | None = Field(default=None, ge=0, description="Area in square feet")
    ceiling_height: float | None = Field(default=None, ge=0, description="Ceiling height in feet")
    perimeter_ft: float | None = Field(default=None, ge=0, description="Perimeter in feet")
    finishes: RoomFinishes = Field(default_factory=RoomFinishes)
    fixtures: list[str] = Field(default_factory=list, description="Fixtures in room")
    notes: str | None = Field(default=None, description="Additional notes")
    source_page: int | None = Field(default=None, ge=1, description="Source page number")
    confidence: float = Field(default=0.5, ge=0.0, le=1.0, description="Extraction confidence")


class RoomsExtractionOutput(BaseModel):
    """Output from room extraction pipeline."""

    project_id: str
    document_id: str | None = None
    rooms: list[ExtractedRoomItem] = Field(default_factory=list)
    total_count: int = Field(default=0, ge=0)
    total_area_sqft: float | None = Field(default=None)
    extraction_notes: list[str] = Field(default_factory=list)
    confidence: float = Field(default=0.5, ge=0.0, le=1.0)


# ============================================================================
# Milestone Extraction
# ============================================================================


class MilestoneStatus(str, Enum):
    """Status for project milestones."""

    PENDING = "pending"
    IN_PROGRESS = "in_progress"
    COMPLETED = "completed"
    DELAYED = "delayed"
    CANCELLED = "cancelled"


class ExtractedMilestoneItem(BaseModel):
    """A single extracted milestone from blueprints."""

    name: str = Field(description="Milestone name")
    description: str | None = Field(default=None, description="Milestone description")
    phase: str | None = Field(default=None, description="Construction phase")
    phase_order: int = Field(default=0, ge=0, description="Order within phase")
    estimated_duration_days: int | None = Field(default=None, ge=0, description="Duration in days")
    dependencies: list[str] = Field(default_factory=list, description="Dependency names")
    trades_involved: list[str] = Field(default_factory=list, description="Trades required")
    deliverables: list[str] = Field(default_factory=list, description="Expected deliverables")
    confidence: float = Field(default=0.5, ge=0.0, le=1.0, description="Extraction confidence")


class MilestonesExtractionOutput(BaseModel):
    """Output from milestone extraction pipeline."""

    project_id: str
    phases: list[str] = Field(default_factory=list, description="Identified phases")
    milestones: list[ExtractedMilestoneItem] = Field(default_factory=list)
    total_count: int = Field(default=0, ge=0)
    estimated_total_days: int | None = Field(default=None)
    extraction_notes: list[str] = Field(default_factory=list)
    confidence: float = Field(default=0.5, ge=0.0, le=1.0)


# ============================================================================
# Trade Scope Extraction
# ============================================================================


class ExtractedTradeScopeItem(BaseModel):
    """A single extracted trade scope from blueprints."""

    trade: str = Field(description="Trade name")
    trade_display_name: str | None = Field(default=None, description="Display name")
    csi_division: str | None = Field(default=None, description="CSI division code")
    inclusions: list[ScopeItem] = Field(default_factory=list, description="Included scope items")
    exclusions: list[ScopeItem] = Field(default_factory=list, description="Excluded scope items")
    required_sheets: list[str] = Field(default_factory=list, description="Required drawing sheets")
    spec_sections: list[str] = Field(default_factory=list, description="Spec sections")
    rfi_needed: list[str] = Field(default_factory=list, description="RFIs needed")
    assumptions: list[str] = Field(default_factory=list, description="Assumptions made")
    confidence: float = Field(default=0.5, ge=0.0, le=1.0, description="Extraction confidence")


class TradeScopesExtractionOutput(BaseModel):
    """Output from trade scope extraction pipeline."""

    project_id: str
    document_id: str | None = None
    trade_scopes: list[ExtractedTradeScopeItem] = Field(default_factory=list)
    total_count: int = Field(default=0, ge=0)
    general_notes: list[str] = Field(default_factory=list)
    extraction_notes: list[str] = Field(default_factory=list)
    confidence: float = Field(default=0.5, ge=0.0, le=1.0)


# ============================================================================
# Progress Events for SSE
# ============================================================================


class ProgressEvent(BaseModel):
    """Base class for progress events sent via SSE."""

    job_id: str
    timestamp: datetime = Field(default_factory=datetime.utcnow)


class JobStatusChangedEvent(ProgressEvent):
    """Event when job status changes."""

    type: str = "job_status_changed"
    status: str
    progress: float = Field(ge=0.0, le=1.0)
    current_step: str | None = None


class StepStartedEvent(ProgressEvent):
    """Event when a step starts."""

    type: str = "step_started"
    step_key: str
    step_name: str
    step_order: int


class StepProgressEvent(ProgressEvent):
    """Event for step progress updates."""

    type: str = "step_progress"
    step_key: str
    progress: float = Field(ge=0.0, le=1.0)
    items_processed: int = Field(default=0, ge=0)
    items_total: int = Field(default=0, ge=0)
    message: str | None = None


class StepCompletedEvent(ProgressEvent):
    """Event when a step completes."""

    type: str = "step_completed"
    step_key: str
    duration_ms: int = Field(ge=0)


class StepFailedEvent(ProgressEvent):
    """Event when a step fails."""

    type: str = "step_failed"
    step_key: str
    error: str
    can_retry: bool = True


class JobCompletedEvent(ProgressEvent):
    """Event when job completes."""

    type: str = "job_completed"
    duration_ms: int = Field(ge=0)
    results_summary: dict[str, Any] = Field(default_factory=dict)


class JobFailedEvent(ProgressEvent):
    """Event when job fails."""

    type: str = "job_failed"
    error: str
    failed_step: str | None = None
    can_retry: bool = True


class JobPausedEvent(ProgressEvent):
    """Event when job is paused."""

    type: str = "job_paused"
    current_step: str | None = None


class JobResumedEvent(ProgressEvent):
    """Event when job is resumed."""

    type: str = "job_resumed"


class JobCancelledEvent(ProgressEvent):
    """Event when job is cancelled."""

    type: str = "job_cancelled"


class HeartbeatEvent(BaseModel):
    """Heartbeat event for SSE connection keepalive."""

    type: str = "heartbeat"
    timestamp: datetime = Field(default_factory=datetime.utcnow)


# Union type for all events
ExtractionEvent = (
    JobStatusChangedEvent
    | StepStartedEvent
    | StepProgressEvent
    | StepCompletedEvent
    | StepFailedEvent
    | JobCompletedEvent
    | JobFailedEvent
    | JobPausedEvent
    | JobResumedEvent
    | JobCancelledEvent
    | HeartbeatEvent
)


# ============================================================================
# Extraction State
# ============================================================================


class ExtractionStepState(BaseModel):
    """State for a single extraction step."""

    step_key: ExtractionStepKey
    step_name: str
    step_order: int
    status: StepStatus = StepStatus.PENDING
    progress: float = Field(default=0.0, ge=0.0, le=1.0)
    items_total: int = Field(default=0, ge=0)
    items_processed: int = Field(default=0, ge=0)
    message: str | None = None
    error: str | None = None
    started_at: datetime | None = None
    completed_at: datetime | None = None
    output: dict[str, Any] = Field(default_factory=dict)


class ExtractionState(BaseModel):
    """Full state for extraction pipeline."""

    job_id: str
    project_id: str
    document_id: str | None = None
    status: str = "queued"  # queued, running, paused, completed, failed, cancelled
    current_step: ExtractionStepKey | None = None
    progress: float = Field(default=0.0, ge=0.0, le=1.0)
    steps: list[ExtractionStepState] = Field(default_factory=list)

    # Inputs
    file_path: str | None = None
    file_bytes: bytes | None = None
    ocr_results: list[dict[str, Any]] = Field(default_factory=list)

    # Extraction outputs
    materials: list[ExtractedMaterialItem] = Field(default_factory=list)
    rooms: list[ExtractedRoomItem] = Field(default_factory=list)
    milestones: list[ExtractedMilestoneItem] = Field(default_factory=list)
    trade_scopes: list[ExtractedTradeScopeItem] = Field(default_factory=list)

    # Metadata
    error: str | None = None
    can_retry: bool = True
    retry_count: int = Field(default=0, ge=0)
    paused_at: datetime | None = None
    started_at: datetime | None = None
    completed_at: datetime | None = None
    created_at: datetime = Field(default_factory=datetime.utcnow)

    class Config:
        arbitrary_types_allowed = True
