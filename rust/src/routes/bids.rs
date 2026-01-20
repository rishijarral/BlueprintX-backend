//! Bid routes
//!
//! Bid submission and management for tenders.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use rust_decimal::prelude::*;
use std::sync::Arc;
use uuid::Uuid;

use crate::api::pagination::PaginationParams;
use crate::api::response::{DataResponse, Paginated};
use crate::app::AppState;
use crate::auth::RequireAuth;
use crate::domain::bids::{BidResponse, BidStatus, CreateBidRequest};
use crate::error::ApiError;

/// Database row for bid
#[derive(Debug, sqlx::FromRow)]
struct BidRow {
    id: Uuid,
    tender_id: Uuid,
    bidder_id: Option<Uuid>,
    company_name: String,
    contact_name: Option<String>,
    contact_email: Option<String>,
    contact_phone: Option<String>,
    bid_amount: rust_decimal::Decimal,
    status: String,
    notes: Option<String>,
    submitted_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<BidRow> for BidResponse {
    fn from(row: BidRow) -> Self {
        let status = match row.status.as_str() {
            "submitted" => BidStatus::Submitted,
            "under_review" => BidStatus::UnderReview,
            "shortlisted" => BidStatus::Shortlisted,
            "awarded" => BidStatus::Awarded,
            "rejected" => BidStatus::Rejected,
            "withdrawn" => BidStatus::Withdrawn,
            _ => BidStatus::Draft,
        };

        // Convert decimal to cents
        let bid_amount = (row.bid_amount * rust_decimal::Decimal::from(100))
            .to_i64()
            .unwrap_or(0);

        Self {
            id: row.id,
            tender_id: row.tender_id,
            company_name: row.company_name,
            contact_name: row.contact_name,
            contact_email: row.contact_email,
            contact_phone: row.contact_phone,
            bid_amount,
            status,
            notes: row.notes,
            submitted_at: row.submitted_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// POST /api/tenders/:tender_id/bids
///
/// Submit a bid for a tender.
pub async fn create_bid(
    auth: RequireAuth,
    State(state): State<Arc<AppState>>,
    Path(tender_id): Path<Uuid>,
    Json(req): Json<CreateBidRequest>,
) -> Result<impl IntoResponse, ApiError> {
    tracing::info!(
        user_id = %auth.user_id,
        tender_id = %tender_id,
        company_name = %req.company_name,
        bid_amount = req.bid_amount,
        "Creating bid"
    );

    // Verify tender exists and is open
    let tender_status: Option<String> =
        sqlx::query_scalar("SELECT status FROM tenders WHERE id = $1")
            .bind(tender_id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    match tender_status.as_deref() {
        None => return Err(ApiError::not_found("Tender not found")),
        Some(status) if status != "open" && status != "draft" => {
            return Err(ApiError::bad_request("Tender is not accepting bids"));
        }
        _ => {}
    }

    // Convert cents to decimal
    let bid_amount =
        rust_decimal::Decimal::from(req.bid_amount) / rust_decimal::Decimal::from(100);

    let bid = sqlx::query_as::<_, BidRow>(
        r#"
        INSERT INTO bids (tender_id, bidder_id, company_name, contact_name, contact_email, contact_phone, bid_amount, status, notes, submitted_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, 'submitted', $8, NOW())
        RETURNING id, tender_id, bidder_id, company_name, contact_name, contact_email, contact_phone, bid_amount, status, notes, submitted_at, created_at, updated_at
        "#,
    )
    .bind(tender_id)
    .bind(auth.user_id)
    .bind(&req.company_name)
    .bind(&req.contact_name)
    .bind(&req.contact_email)
    .bind(&req.contact_phone)
    .bind(bid_amount)
    .bind(&req.notes)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to create bid: {}", e)))?;

    let response: BidResponse = bid.into();
    Ok((StatusCode::CREATED, Json(DataResponse::new(response))))
}

/// GET /api/tenders/:tender_id/bids
///
/// List bids for a tender. Only the tender owner (project owner) can see all bids.
pub async fn list_bids(
    auth: RequireAuth,
    State(state): State<Arc<AppState>>,
    Path(tender_id): Path<Uuid>,
    Query(pagination): Query<PaginationParams>,
) -> Result<impl IntoResponse, ApiError> {
    tracing::info!(
        user_id = %auth.user_id,
        tender_id = %tender_id,
        "Listing bids"
    );

    // Verify user owns the project that this tender belongs to
    let is_owner: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM tenders t
            JOIN projects p ON t.project_id = p.id
            WHERE t.id = $1 AND p.owner_id = $2
        )
        "#,
    )
    .bind(tender_id)
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    if !is_owner {
        return Err(ApiError::forbidden(
            "Only the project owner can view all bids",
        ));
    }

    let offset = pagination.offset() as i64;
    let limit = pagination.limit() as i64;

    // Get total count
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM bids WHERE tender_id = $1")
        .bind(tender_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    // Get bids
    let bids = sqlx::query_as::<_, BidRow>(
        r#"
        SELECT id, tender_id, bidder_id, company_name, contact_name, contact_email, contact_phone, bid_amount, status, notes, submitted_at, created_at, updated_at
        FROM bids
        WHERE tender_id = $1
        ORDER BY bid_amount ASC, submitted_at ASC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(tender_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let data: Vec<BidResponse> = bids.into_iter().map(Into::into).collect();
    Ok(Json(Paginated::new(data, &pagination, total as u64)))
}
