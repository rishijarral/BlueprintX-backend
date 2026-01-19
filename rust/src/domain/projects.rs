use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Project status enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectStatus {
    Draft,
    Active,
    Bidding,
    Awarded,
    InProgress,
    Completed,
    Cancelled,
}

impl Default for ProjectStatus {
    fn default() -> Self {
        Self::Draft
    }
}

/// Project entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub address: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub zip_code: Option<String>,
    pub status: ProjectStatus,
    pub estimated_value: Option<i64>, // cents
    pub bid_due_date: Option<DateTime<Utc>>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request DTO for creating a project
#[derive(Debug, Clone, Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub city: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub zip_code: Option<String>,
    #[serde(default)]
    pub estimated_value: Option<i64>,
    #[serde(default)]
    pub bid_due_date: Option<DateTime<Utc>>,
    #[serde(default)]
    pub start_date: Option<DateTime<Utc>>,
    #[serde(default)]
    pub end_date: Option<DateTime<Utc>>,
}

/// Request DTO for updating a project
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateProjectRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub city: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub zip_code: Option<String>,
    #[serde(default)]
    pub status: Option<ProjectStatus>,
    #[serde(default)]
    pub estimated_value: Option<i64>,
    #[serde(default)]
    pub bid_due_date: Option<DateTime<Utc>>,
    #[serde(default)]
    pub start_date: Option<DateTime<Utc>>,
    #[serde(default)]
    pub end_date: Option<DateTime<Utc>>,
}

/// Response DTO for project
#[derive(Debug, Clone, Serialize)]
pub struct ProjectResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub address: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub zip_code: Option<String>,
    pub status: ProjectStatus,
    pub estimated_value: Option<i64>,
    pub bid_due_date: Option<DateTime<Utc>>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Project> for ProjectResponse {
    fn from(p: Project) -> Self {
        Self {
            id: p.id,
            name: p.name,
            description: p.description,
            address: p.address,
            city: p.city,
            state: p.state,
            zip_code: p.zip_code,
            status: p.status,
            estimated_value: p.estimated_value,
            bid_due_date: p.bid_due_date,
            start_date: p.start_date,
            end_date: p.end_date,
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}
