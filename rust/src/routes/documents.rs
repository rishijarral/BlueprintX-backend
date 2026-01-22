//! Document routes
//!
//! CRUD operations for project documents/blueprints including file upload.

use axum::{
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use crate::api::pagination::PaginationParams;
use crate::api::response::{DataResponse, Paginated};
use crate::app::AppState;
use crate::auth::RequireAuth;
use crate::domain::{CreateDocumentRequest, DocumentResponse, DocumentStatus, DocumentType};
use crate::error::ApiError;

/// Database row for document
#[allow(dead_code)]
#[derive(Debug, sqlx::FromRow)]
struct DocumentRow {
    id: Uuid,
    project_id: Uuid,
    name: String,
    description: Option<String>,
    document_type: String,
    file_path: Option<String>,
    file_size: Option<i64>,
    mime_type: Option<String>,
    version: Option<i32>,
    status: String,
    category: Option<String>,
    revised: Option<String>,
    author: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<DocumentRow> for DocumentResponse {
    type Error = ApiError;

    fn try_from(row: DocumentRow) -> Result<Self, Self::Error> {
        let document_type = match row.document_type.as_str() {
            "plan" => DocumentType::Plan,
            "specification" => DocumentType::Specification,
            "addendum" => DocumentType::Addendum,
            "contract" => DocumentType::Contract,
            "change_order" => DocumentType::ChangeOrder,
            "submittal" => DocumentType::Submittal,
            "rfi" => DocumentType::Rfi,
            _ => DocumentType::Other,
        };

        let status = match row.status.as_str() {
            "draft" => DocumentStatus::Draft,
            "active" => DocumentStatus::Active,
            "superseded" => DocumentStatus::Superseded,
            "archived" => DocumentStatus::Archived,
            _ => DocumentStatus::Draft,
        };

        Ok(Self {
            id: row.id,
            project_id: row.project_id,
            name: row.name,
            description: row.description,
            document_type,
            file_size: row.file_size.unwrap_or(0),
            mime_type: row.mime_type.unwrap_or_default(),
            version: row.version.unwrap_or(1),
            status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

/// Verify project ownership
async fn verify_project_ownership(
    state: &AppState,
    project_id: Uuid,
    user_id: Uuid,
) -> Result<(), ApiError> {
    let exists: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM projects WHERE id = $1 AND owner_id = $2")
            .bind(project_id)
            .bind(user_id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    if exists.is_none() {
        return Err(ApiError::not_found("Project not found"));
    }

    Ok(())
}

/// POST /api/projects/:project_id/documents
///
/// Create a document metadata entry (without file).
pub async fn create_document(
    auth: RequireAuth,
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
    Json(req): Json<CreateDocumentRequest>,
) -> Result<impl IntoResponse, ApiError> {
    tracing::info!(
        user_id = %auth.user_id,
        project_id = %project_id,
        document_name = %req.name,
        "Creating document"
    );

    verify_project_ownership(&state, project_id, auth.user_id).await?;

    let document_type = match req.document_type {
        DocumentType::Plan => "plan",
        DocumentType::Specification => "specification",
        DocumentType::Addendum => "addendum",
        DocumentType::Contract => "contract",
        DocumentType::ChangeOrder => "change_order",
        DocumentType::Submittal => "submittal",
        DocumentType::Rfi => "rfi",
        DocumentType::Other => "other",
    };

    let document = sqlx::query_as::<_, DocumentRow>(
        r#"
        INSERT INTO documents (project_id, name, description, document_type, status)
        VALUES ($1, $2, $3, $4, 'draft')
        RETURNING id, project_id, name, description, document_type, file_path, file_size, mime_type, version, status, category, revised, author, created_at, updated_at
        "#,
    )
    .bind(project_id)
    .bind(&req.name)
    .bind(&req.description)
    .bind(document_type)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to create document: {}", e)))?;

    let response: DocumentResponse = document.try_into()?;
    Ok((StatusCode::CREATED, Json(DataResponse::new(response))))
}

/// POST /api/projects/:project_id/documents/upload
///
/// Upload a document file (multipart form).
pub async fn upload_document(
    auth: RequireAuth,
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, ApiError> {
    tracing::info!(
        user_id = %auth.user_id,
        project_id = %project_id,
        "Uploading document"
    );

    verify_project_ownership(&state, project_id, auth.user_id).await?;

    let mut file_data: Option<Vec<u8>> = None;
    let mut file_name: Option<String> = None;
    let mut content_type: Option<String> = None;
    let mut document_type = "other".to_string();

    // Process multipart fields
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::bad_request(format!("Failed to read multipart: {}", e)))?
    {
        let name = field.name().unwrap_or("").to_string();

        match name.as_str() {
            "file" => {
                file_name = field.file_name().map(|s| s.to_string());
                content_type = field.content_type().map(|s| s.to_string());
                file_data = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|e| ApiError::bad_request(format!("Failed to read file: {}", e)))?
                        .to_vec(),
                );
            }
            "document_type" => {
                let value = field
                    .text()
                    .await
                    .map_err(|e| ApiError::bad_request(format!("Failed to read field: {}", e)))?;
                document_type = value;
            }
            _ => {}
        }
    }

    let file_data =
        file_data.ok_or_else(|| ApiError::bad_request("No file provided in upload"))?;
    let file_name =
        file_name.ok_or_else(|| ApiError::bad_request("No filename provided in upload"))?;

    // Validate file type (only allow PDFs for blueprints)
    let mime = content_type.as_deref().unwrap_or("application/octet-stream");
    if !mime.contains("pdf") && !mime.contains("octet-stream") {
        // Allow PDFs and generic binary
        tracing::warn!(mime_type = %mime, "Non-PDF file uploaded, allowing but logging");
    }

    // Validate file size (max 100MB)
    let file_size = file_data.len() as i64;
    if file_size > 100 * 1024 * 1024 {
        return Err(ApiError::bad_request("File too large (max 100MB)"));
    }

    // Create upload directory
    let upload_dir = format!("./uploads/documents/{}", project_id);
    fs::create_dir_all(&upload_dir)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to create upload directory: {}", e)))?;

    // Generate unique filename
    let file_uuid = Uuid::new_v4();
    let safe_filename = file_name
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '.' || *c == '-' || *c == '_')
        .collect::<String>();
    let stored_filename = format!("{}_{}", file_uuid, safe_filename);
    let file_path = format!("{}/{}", upload_dir, stored_filename);

    // Write file to disk
    let mut file = fs::File::create(&file_path)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to create file: {}", e)))?;
    file.write_all(&file_data)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to write file: {}", e)))?;
    file.flush()
        .await
        .map_err(|e| ApiError::internal(format!("Failed to flush file: {}", e)))?;

    // Insert document record
    let document = sqlx::query_as::<_, DocumentRow>(
        r#"
        INSERT INTO documents (project_id, name, document_type, file_path, file_size, mime_type, status)
        VALUES ($1, $2, $3, $4, $5, $6, 'active')
        RETURNING id, project_id, name, description, document_type, file_path, file_size, mime_type, version, status, category, revised, author, created_at, updated_at
        "#,
    )
    .bind(project_id)
    .bind(&file_name)
    .bind(&document_type)
    .bind(&file_path)
    .bind(file_size)
    .bind(mime)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to create document record: {}", e)))?;

    let response: DocumentResponse = document.try_into()?;
    Ok((StatusCode::CREATED, Json(DataResponse::new(response))))
}

/// GET /api/projects/:project_id/documents
///
/// List documents for a project.
pub async fn list_documents(
    auth: RequireAuth,
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
    Query(pagination): Query<PaginationParams>,
) -> Result<impl IntoResponse, ApiError> {
    tracing::info!(
        user_id = %auth.user_id,
        project_id = %project_id,
        "Listing documents"
    );

    verify_project_ownership(&state, project_id, auth.user_id).await?;

    let offset = pagination.offset() as i64;
    let limit = pagination.limit() as i64;

    // Get total count
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM documents WHERE project_id = $1")
        .bind(project_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    // Get documents
    let documents = sqlx::query_as::<_, DocumentRow>(
        r#"
        SELECT id, project_id, name, description, document_type, file_path, file_size, mime_type, version, status, category, revised, author, created_at, updated_at
        FROM documents
        WHERE project_id = $1
        ORDER BY created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(project_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let data: Vec<DocumentResponse> = documents
        .into_iter()
        .filter_map(|row| row.try_into().ok())
        .collect();

    Ok(Json(Paginated::new(data, &pagination, total as u64)))
}

/// GET /api/projects/:project_id/documents/:document_id
///
/// Get a specific document.
pub async fn get_document(
    auth: RequireAuth,
    State(state): State<Arc<AppState>>,
    Path((project_id, document_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, ApiError> {
    tracing::info!(
        user_id = %auth.user_id,
        project_id = %project_id,
        document_id = %document_id,
        "Getting document"
    );

    verify_project_ownership(&state, project_id, auth.user_id).await?;

    let document = sqlx::query_as::<_, DocumentRow>(
        r#"
        SELECT id, project_id, name, description, document_type, file_path, file_size, mime_type, version, status, category, revised, author, created_at, updated_at
        FROM documents
        WHERE id = $1 AND project_id = $2
        "#,
    )
    .bind(document_id)
    .bind(project_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
    .ok_or_else(|| ApiError::not_found("Document not found"))?;

    let response: DocumentResponse = document.try_into()?;
    Ok(Json(DataResponse::new(response)))
}

/// DELETE /api/projects/:project_id/documents/:document_id
///
/// Delete a document and its file.
pub async fn delete_document(
    auth: RequireAuth,
    State(state): State<Arc<AppState>>,
    Path((project_id, document_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, ApiError> {
    tracing::info!(
        user_id = %auth.user_id,
        project_id = %project_id,
        document_id = %document_id,
        "Deleting document"
    );

    verify_project_ownership(&state, project_id, auth.user_id).await?;

    // Get file path before deleting
    let file_path: Option<String> = sqlx::query_scalar(
        "SELECT file_path FROM documents WHERE id = $1 AND project_id = $2",
    )
    .bind(document_id)
    .bind(project_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
    .flatten();

    // Delete from database
    let result =
        sqlx::query("DELETE FROM documents WHERE id = $1 AND project_id = $2")
            .bind(document_id)
            .bind(project_id)
            .execute(&state.db)
            .await
            .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Document not found"));
    }

    // Delete file from disk (ignore errors)
    if let Some(path) = file_path {
        let _ = fs::remove_file(&path).await;
    }

    Ok(StatusCode::NO_CONTENT)
}
