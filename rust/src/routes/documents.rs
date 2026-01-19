use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::api::{MessageResponse, PaginationParams};
use crate::app::AppState;
use crate::auth::RequireAuth;
use crate::domain::CreateDocumentRequest;

/// Upload a document to a project
pub async fn create_document(
    auth: RequireAuth,
    State(_state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
    Json(req): Json<CreateDocumentRequest>,
) -> (StatusCode, Json<MessageResponse>) {
    tracing::info!(
        user_id = %auth.user_id,
        project_id = %project_id,
        document_name = %req.name,
        document_type = ?req.document_type,
        "Creating document"
    );

    // TODO: Implement actual document creation with file upload
    (
        StatusCode::CREATED,
        Json(MessageResponse::with_code(
            format!(
                "Document '{}' upload placeholder for project {}",
                req.name, project_id
            ),
            "DOCUMENT_CREATED",
        )),
    )
}

/// List documents for a project
pub async fn list_documents(
    auth: RequireAuth,
    State(_state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
    Query(pagination): Query<PaginationParams>,
) -> Json<MessageResponse> {
    tracing::info!(
        user_id = %auth.user_id,
        project_id = %project_id,
        page = pagination.page(),
        per_page = pagination.per_page(),
        "Listing documents"
    );

    // TODO: Implement actual document listing with database
    Json(MessageResponse::with_code(
        format!(
            "Documents list placeholder for project {} (page {}, per_page {})",
            project_id,
            pagination.page(),
            pagination.per_page()
        ),
        "DOCUMENTS_LIST",
    ))
}
