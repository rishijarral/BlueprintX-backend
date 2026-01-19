//! AI-related domain models matching Python AI service schemas.
//!
//! These models mirror the Pydantic schemas from the Python service
//! to ensure type-safe communication.

use serde::{Deserialize, Serialize};

/// Structured plan summary output from AI analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanSummary {
    pub building_type: String,
    pub project_name: Option<String>,
    pub floors: Option<i32>,
    pub total_area_sqft: Option<i32>,
    pub key_materials: Vec<String>,
    pub major_systems: Vec<String>,
    pub structural_system: Option<String>,
    pub risks: Vec<String>,
    pub assumptions: Vec<String>,
    pub confidence: f64,
}

/// Scope item for a single trade.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeScopeItem {
    pub trade: String,
    pub csi_division: Option<String>,
    pub inclusions: Vec<String>,
    pub exclusions: Vec<String>,
    pub required_sheets: Vec<String>,
    pub spec_sections: Vec<String>,
    pub rfi_needed: Vec<String>,
    pub assumptions: Vec<String>,
}

/// Output for trade scope extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeScopesOutput {
    pub project_id: Option<String>,
    pub trades: Vec<TradeScopeItem>,
    pub general_notes: Vec<String>,
    pub confidence: f64,
}

/// Generated tender scope document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenderScopeDoc {
    pub trade: String,
    pub overview: String,
    pub inclusions: Vec<String>,
    pub exclusions: Vec<String>,
    pub allowances: Vec<String>,
    pub alternates: Vec<String>,
    pub submittals: Vec<String>,
    pub schedule_notes: Vec<String>,
    pub lead_times: Vec<String>,
    pub bid_instructions: Vec<String>,
    pub rfi_questions: Vec<String>,
    pub markdown: String,
}

/// Response for Q&A queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QnAResponse {
    pub project_id: String,
    pub question: String,
    pub answer: String,
    pub citations: Vec<String>,
    pub confidence: f64,
    pub followups: Vec<String>,
}

/// Vision OCR result from a drawing page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionOCRResult {
    pub page_number: i32,
    pub sheet_number: Option<String>,
    pub sheet_title: Option<String>,
    pub drawing_type: Option<String>,
    pub discipline: Option<String>,
    pub text_content: String,
    pub annotations: Vec<String>,
    pub dimensions: Vec<String>,
    pub notes: Vec<String>,
    pub materials: Vec<String>,
    pub references: Vec<String>,
}

// =============================================================================
// Request/Response DTOs for API endpoints
// =============================================================================

/// Request for plan summary generation.
#[derive(Debug, Clone, Deserialize)]
pub struct PlanSummaryRequest {
    pub document_text: String,
    #[serde(default)]
    pub instructions: Option<String>,
}

/// Response for plan summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanSummaryResponse {
    pub project_id: String,
    pub summary: PlanSummary,
    pub cached: bool,
}

/// Request for trade scope extraction.
#[derive(Debug, Clone, Deserialize)]
pub struct TradeScopesRequest {
    pub document_text: String,
    #[serde(default)]
    pub trades: Option<Vec<String>>,
}

/// Response for trade scopes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeScopesResponse {
    pub project_id: String,
    pub scopes: TradeScopesOutput,
    pub cached: bool,
}

/// Request for tender scope document generation.
#[derive(Debug, Clone, Deserialize)]
pub struct TenderScopeDocRequest {
    pub trade: String,
    pub scope_data: serde_json::Value,
    #[serde(default)]
    pub project_context: Option<String>,
    #[serde(default)]
    pub bid_due_date: Option<String>,
}

/// Response for tender scope document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenderScopeDocResponse {
    pub project_id: String,
    pub trade: String,
    pub document: TenderScopeDoc,
}

/// Request for Q&A.
#[derive(Debug, Clone, Deserialize)]
pub struct QnARequest {
    pub question: String,
    #[serde(default)]
    pub document_id: Option<uuid::Uuid>,
    #[serde(default)]
    pub document_text: Option<String>,
}

/// Standard trades list response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardTradesResponse {
    pub trades: Vec<String>,
}

// =============================================================================
// Job-related types
// =============================================================================

/// Job status enum matching Python service.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

/// Job type enum matching Python service.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobType {
    DocumentIngest,
    PlanSummary,
    TradeScopeExtract,
    TenderScopeDoc,
    Qna,
}

/// Job information for tracking async operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobInfo {
    pub job_id: String,
    pub job_type: JobType,
    pub status: JobStatus,
    pub progress: f64,
    pub error: Option<String>,
    pub created_at: String,
    pub completed_at: Option<String>,
}

/// Request to create a document ingestion job.
#[derive(Debug, Clone, Deserialize)]
pub struct IngestJobRequest {
    pub document_id: uuid::Uuid,
}

/// Response for job creation/status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobResponse {
    pub job_id: String,
    pub status: String,
    pub progress: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<serde_json::Value>,
}
