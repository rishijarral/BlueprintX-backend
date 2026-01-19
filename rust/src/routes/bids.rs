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
use crate::domain::CreateBidRequest;

/// Submit a bid for a tender
pub async fn create_bid(
    auth: RequireAuth,
    State(_state): State<Arc<AppState>>,
    Path(tender_id): Path<Uuid>,
    Json(req): Json<CreateBidRequest>,
) -> (StatusCode, Json<MessageResponse>) {
    tracing::info!(
        user_id = %auth.user_id,
        tender_id = %tender_id,
        company_name = %req.company_name,
        bid_amount = req.bid_amount,
        "Creating bid"
    );

    // TODO: Implement actual bid creation with database
    (
        StatusCode::CREATED,
        Json(MessageResponse::with_code(
            format!(
                "Bid from '{}' for ${:.2} creation placeholder for tender {}",
                req.company_name,
                req.bid_amount as f64 / 100.0,
                tender_id
            ),
            "BID_CREATED",
        )),
    )
}

/// List bids for a tender
pub async fn list_bids(
    auth: RequireAuth,
    State(_state): State<Arc<AppState>>,
    Path(tender_id): Path<Uuid>,
    Query(pagination): Query<PaginationParams>,
) -> Json<MessageResponse> {
    tracing::info!(
        user_id = %auth.user_id,
        tender_id = %tender_id,
        page = pagination.page(),
        per_page = pagination.per_page(),
        "Listing bids"
    );

    // TODO: Implement actual bid listing with database
    Json(MessageResponse::with_code(
        format!(
            "Bids list placeholder for tender {} (page {}, per_page {})",
            tender_id,
            pagination.page(),
            pagination.per_page()
        ),
        "BIDS_LIST",
    ))
}
