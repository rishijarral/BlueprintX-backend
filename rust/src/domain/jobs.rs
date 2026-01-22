//! Processing job domain types
//!
//! Types for tracking document ingestion and AI processing jobs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Processing job status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "text", rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobStatus::Queued => write!(f, "queued"),
            JobStatus::Running => write!(f, "running"),
            JobStatus::Paused => write!(f, "paused"),
            JobStatus::Completed => write!(f, "completed"),
            JobStatus::Failed => write!(f, "failed"),
            JobStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Processing step status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "text", rename_all = "snake_case")]
pub enum StepStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

impl std::fmt::Display for StepStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StepStatus::Pending => write!(f, "pending"),
            StepStatus::Running => write!(f, "running"),
            StepStatus::Completed => write!(f, "completed"),
            StepStatus::Failed => write!(f, "failed"),
            StepStatus::Skipped => write!(f, "skipped"),
        }
    }
}

/// Processing step keys (used for identifying steps)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StepKey {
    Upload,
    Validate,
    ExtractPages,
    OcrPages,
    ChunkText,
    GenerateEmbeddings,
    StoreVectors,
    ExtractTradeScopes,
    ExtractMaterials,
    ExtractRooms,
    GenerateMilestones,
    Finalize,
}

impl std::fmt::Display for StepKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StepKey::Upload => write!(f, "upload"),
            StepKey::Validate => write!(f, "validate"),
            StepKey::ExtractPages => write!(f, "extract_pages"),
            StepKey::OcrPages => write!(f, "ocr_pages"),
            StepKey::ChunkText => write!(f, "chunk_text"),
            StepKey::GenerateEmbeddings => write!(f, "generate_embeddings"),
            StepKey::StoreVectors => write!(f, "store_vectors"),
            StepKey::ExtractTradeScopes => write!(f, "extract_trade_scopes"),
            StepKey::ExtractMaterials => write!(f, "extract_materials"),
            StepKey::ExtractRooms => write!(f, "extract_rooms"),
            StepKey::GenerateMilestones => write!(f, "generate_milestones"),
            StepKey::Finalize => write!(f, "finalize"),
        }
    }
}

/// Processing job response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingJobResponse {
    pub id: Uuid,
    pub document_id: Uuid,
    pub project_id: Uuid,
    pub status: String,
    pub current_step: Option<String>,
    pub progress: f64,
    pub total_steps: i32,
    pub completed_steps: i32,
    pub error_message: Option<String>,
    pub error_step: Option<String>,
    pub can_retry: bool,
    pub retry_count: i32,
    pub steps: Vec<ProcessingStepResponse>,
    pub paused_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Processing step response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingStepResponse {
    pub id: Uuid,
    pub step_name: String,
    pub step_key: String,
    pub step_order: i32,
    pub status: String,
    pub progress: f64,
    pub message: Option<String>,
    pub details: serde_json::Value,
    pub items_total: i32,
    pub items_processed: i32,
    pub error_message: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Start processing request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartProcessingRequest {
    pub auto_start: Option<bool>,
}

/// Job control action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobControlAction {
    Pause,
    Resume,
    Cancel,
    RetryStep { step_key: String },
    RetryJob,
}

/// Job control request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobControlRequest {
    pub action: JobControlAction,
}

/// SSE event types for job progress
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum JobProgressEvent {
    /// Job status changed
    JobStatusChanged {
        job_id: Uuid,
        status: String,
        progress: f64,
        current_step: Option<String>,
    },
    /// Step started
    StepStarted {
        job_id: Uuid,
        step_key: String,
        step_name: String,
        step_order: i32,
    },
    /// Step progress updated
    StepProgress {
        job_id: Uuid,
        step_key: String,
        progress: f64,
        items_processed: i32,
        items_total: i32,
        message: Option<String>,
    },
    /// Step completed
    StepCompleted {
        job_id: Uuid,
        step_key: String,
        duration_ms: i64,
    },
    /// Step failed
    StepFailed {
        job_id: Uuid,
        step_key: String,
        error: String,
        can_retry: bool,
    },
    /// Job completed
    JobCompleted { job_id: Uuid, duration_ms: i64 },
    /// Job failed
    JobFailed {
        job_id: Uuid,
        error: String,
        failed_step: Option<String>,
        can_retry: bool,
    },
    /// Job paused
    JobPaused {
        job_id: Uuid,
        current_step: Option<String>,
    },
    /// Job resumed
    JobResumed { job_id: Uuid },
    /// Job cancelled
    JobCancelled { job_id: Uuid },
    /// Heartbeat to keep connection alive
    Heartbeat { timestamp: DateTime<Utc> },
}

/// Default processing steps for document ingestion
pub fn default_ingestion_steps() -> Vec<(StepKey, &'static str, i32)> {
    vec![
        (StepKey::Upload, "Uploading Document", 1),
        (StepKey::Validate, "Validating Document", 2),
        (StepKey::ExtractPages, "Extracting Pages", 3),
        (StepKey::OcrPages, "OCR Processing", 4),
        (StepKey::ChunkText, "Chunking Text", 5),
        (StepKey::GenerateEmbeddings, "Generating Embeddings", 6),
        (StepKey::StoreVectors, "Storing Vectors", 7),
        (StepKey::ExtractTradeScopes, "Extracting Trade Scopes", 8),
        (StepKey::ExtractMaterials, "Extracting Materials", 9),
        (StepKey::ExtractRooms, "Extracting Rooms", 10),
        (StepKey::GenerateMilestones, "Generating Milestones", 11),
        (StepKey::Finalize, "Finalizing", 12),
    ]
}
