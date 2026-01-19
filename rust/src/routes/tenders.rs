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
use crate::domain::CreateTenderRequest;

/// Create a new tender package for a project
pub async fn create_tender(
    auth: RequireAuth,
    State(_state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
    Json(req): Json<CreateTenderRequest>,
) -> (StatusCode, Json<MessageResponse>) {
    tracing::info!(
        user_id = %auth.user_id,
        project_id = %project_id,
        tender_name = %req.name,
        trade_category = ?req.trade_category,
        "Creating tender"
    );

    // TODO: Implement actual tender creation with database
    (
        StatusCode::CREATED,
        Json(MessageResponse::with_code(
            format!(
                "Tender '{}' creation placeholder for project {}",
                req.name, project_id
            ),
            "TENDER_CREATED",
        )),
    )
}

/// List tenders for a project
pub async fn list_tenders(
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
        "Listing tenders"
    );

    // TODO: Implement actual tender listing with database
    Json(MessageResponse::with_code(
        format!(
            "Tenders list placeholder for project {} (page {}, per_page {})",
            project_id,
            pagination.page(),
            pagination.per_page()
        ),
        "TENDERS_LIST",
    ))
}
