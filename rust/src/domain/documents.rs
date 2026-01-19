use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Document type enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DocumentType {
    Plan,
    Specification,
    Addendum,
    Contract,
    ChangeOrder,
    Submittal,
    Rfi,
    Other,
}

impl Default for DocumentType {
    fn default() -> Self {
        Self::Other
    }
}

/// Document version status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DocumentStatus {
    Draft,
    Active,
    Superseded,
    Archived,
}

impl Default for DocumentStatus {
    fn default() -> Self {
        Self::Draft
    }
}

/// Document entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: Uuid,
    pub project_id: Uuid,
    pub uploaded_by: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub document_type: DocumentType,
    pub file_path: String,
    pub file_size: i64,
    pub mime_type: String,
    pub version: i32,
    pub status: DocumentStatus,
    pub checksum: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request DTO for uploading a document
#[derive(Debug, Clone, Deserialize)]
pub struct CreateDocumentRequest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub document_type: DocumentType,
}

/// Response DTO for document
#[derive(Debug, Clone, Serialize)]
pub struct DocumentResponse {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub document_type: DocumentType,
    pub file_size: i64,
    pub mime_type: String,
    pub version: i32,
    pub status: DocumentStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Document> for DocumentResponse {
    fn from(d: Document) -> Self {
        Self {
            id: d.id,
            project_id: d.project_id,
            name: d.name,
            description: d.description,
            document_type: d.document_type,
            file_size: d.file_size,
            mime_type: d.mime_type,
            version: d.version,
            status: d.status,
            created_at: d.created_at,
            updated_at: d.updated_at,
        }
    }
}
