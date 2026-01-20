//! RFI (Request for Information) domain types
//!
//! RFIs for project clarifications and communications.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// RFI status enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RFIStatus {
    Open,
    Answered,
    Closed,
}

impl Default for RFIStatus {
    fn default() -> Self {
        Self::Open
    }
}

/// RFI priority enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RFIPriority {
    Low,
    Medium,
    High,
    Urgent,
}

impl Default for RFIPriority {
    fn default() -> Self {
        Self::Medium
    }
}

/// RFI entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RFI {
    pub id: Uuid,
    pub project_id: Uuid,
    pub number: i32, // Sequential RFI number
    pub title: String,
    pub description: String,
    pub status: RFIStatus,
    pub priority: RFIPriority,
    pub requester: String, // Display name
    pub requester_id: Uuid,
    pub assignee: String, // Display name
    pub assignee_id: Option<Uuid>,
    pub category: Option<String>,
    pub due_date: Option<DateTime<Utc>>,
    pub responses_count: i32,
    pub attachments_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// RFI Response entity (answers to RFIs)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RFIResponseEntity {
    pub id: Uuid,
    pub rfi_id: Uuid,
    pub content: String,
    pub author: String, // Display name
    pub author_id: Uuid,
    pub created_at: DateTime<Utc>,
}

/// Request DTO for creating an RFI
#[derive(Debug, Clone, Deserialize)]
pub struct CreateRFIRequest {
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub priority: RFIPriority,
    #[serde(default)]
    pub assignee_id: Option<Uuid>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub due_date: Option<DateTime<Utc>>,
}

/// Request DTO for updating an RFI
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateRFIRequest {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub status: Option<RFIStatus>,
    #[serde(default)]
    pub priority: Option<RFIPriority>,
    #[serde(default)]
    pub assignee_id: Option<Uuid>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub due_date: Option<DateTime<Utc>>,
}

/// Request DTO for creating an RFI response
#[derive(Debug, Clone, Deserialize)]
pub struct CreateRFIResponseRequest {
    pub content: String,
}

/// Response DTO for RFI
#[derive(Debug, Clone, Serialize)]
pub struct RFIResponse {
    pub id: Uuid,
    pub project_id: Uuid,
    pub number: i32,
    pub title: String,
    pub description: String,
    pub status: RFIStatus,
    pub priority: RFIPriority,
    pub requester: String,
    pub requester_id: Uuid,
    pub assignee: String,
    pub assignee_id: Option<Uuid>,
    pub category: Option<String>,
    pub due_date: Option<DateTime<Utc>>,
    pub responses_count: i32,
    pub attachments_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<RFI> for RFIResponse {
    fn from(r: RFI) -> Self {
        Self {
            id: r.id,
            project_id: r.project_id,
            number: r.number,
            title: r.title,
            description: r.description,
            status: r.status,
            priority: r.priority,
            requester: r.requester,
            requester_id: r.requester_id,
            assignee: r.assignee,
            assignee_id: r.assignee_id,
            category: r.category,
            due_date: r.due_date,
            responses_count: r.responses_count,
            attachments_count: r.attachments_count,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

/// Response DTO for RFI response (answer)
#[derive(Debug, Clone, Serialize)]
pub struct RFIResponseDTO {
    pub id: Uuid,
    pub rfi_id: Uuid,
    pub content: String,
    pub author: String,
    pub author_id: Uuid,
    pub created_at: DateTime<Utc>,
}

impl From<RFIResponseEntity> for RFIResponseDTO {
    fn from(r: RFIResponseEntity) -> Self {
        Self {
            id: r.id,
            rfi_id: r.rfi_id,
            content: r.content,
            author: r.author,
            author_id: r.author_id,
            created_at: r.created_at,
        }
    }
}
