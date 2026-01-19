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
use crate::domain::CreateProjectRequest;

/// Create a new project
pub async fn create_project(
    auth: RequireAuth,
    State(_state): State<Arc<AppState>>,
    Json(req): Json<CreateProjectRequest>,
) -> (StatusCode, Json<MessageResponse>) {
    tracing::info!(
        user_id = %auth.user_id,
        project_name = %req.name,
        "Creating project"
    );

    // TODO: Implement actual project creation with database
    (
        StatusCode::CREATED,
        Json(MessageResponse::with_code(
            format!("Project '{}' creation placeholder", req.name),
            "PROJECT_CREATED",
        )),
    )
}

/// List projects for the authenticated user
pub async fn list_projects(
    auth: RequireAuth,
    State(_state): State<Arc<AppState>>,
    Query(pagination): Query<PaginationParams>,
) -> Json<MessageResponse> {
    tracing::info!(
        user_id = %auth.user_id,
        page = pagination.page(),
        per_page = pagination.per_page(),
        "Listing projects"
    );

    // TODO: Implement actual project listing with database
    Json(MessageResponse::with_code(
        format!(
            "Projects list placeholder (page {}, per_page {})",
            pagination.page(),
            pagination.per_page()
        ),
        "PROJECTS_LIST",
    ))
}

/// Get a specific project by ID
pub async fn get_project(
    auth: RequireAuth,
    State(_state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
) -> Json<MessageResponse> {
    tracing::info!(
        user_id = %auth.user_id,
        project_id = %project_id,
        "Getting project"
    );

    // TODO: Implement actual project retrieval with database
    Json(MessageResponse::with_code(
        format!("Project {} placeholder", project_id),
        "PROJECT_GET",
    ))
}
