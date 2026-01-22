"""Pydantic schemas for AI extraction and processing."""

from .extraction import (
    # Enums
    ExtractionStepKey,
    StepStatus,
    MilestoneStatus,
    # Base models
    RoomFinishes,
    ScopeItem,
    # Material extraction
    ExtractedMaterialItem,
    MaterialsExtractionOutput,
    # Room extraction
    ExtractedRoomItem,
    RoomsExtractionOutput,
    # Milestone extraction
    ExtractedMilestoneItem,
    MilestonesExtractionOutput,
    # Trade scope extraction
    ExtractedTradeScopeItem,
    TradeScopesExtractionOutput,
    # Progress events
    ProgressEvent,
    JobStatusChangedEvent,
    StepStartedEvent,
    StepProgressEvent,
    StepCompletedEvent,
    StepFailedEvent,
    JobCompletedEvent,
    JobFailedEvent,
    JobPausedEvent,
    JobResumedEvent,
    JobCancelledEvent,
    HeartbeatEvent,
    ExtractionEvent,
    # State
    ExtractionStepState,
    ExtractionState,
)

__all__ = [
    # Enums
    "ExtractionStepKey",
    "StepStatus",
    "MilestoneStatus",
    # Base models
    "RoomFinishes",
    "ScopeItem",
    # Material extraction
    "ExtractedMaterialItem",
    "MaterialsExtractionOutput",
    # Room extraction
    "ExtractedRoomItem",
    "RoomsExtractionOutput",
    # Milestone extraction
    "ExtractedMilestoneItem",
    "MilestonesExtractionOutput",
    # Trade scope extraction
    "ExtractedTradeScopeItem",
    "TradeScopesExtractionOutput",
    # Progress events
    "ProgressEvent",
    "JobStatusChangedEvent",
    "StepStartedEvent",
    "StepProgressEvent",
    "StepCompletedEvent",
    "StepFailedEvent",
    "JobCompletedEvent",
    "JobFailedEvent",
    "JobPausedEvent",
    "JobResumedEvent",
    "JobCancelledEvent",
    "HeartbeatEvent",
    "ExtractionEvent",
    # State
    "ExtractionStepState",
    "ExtractionState",
]
