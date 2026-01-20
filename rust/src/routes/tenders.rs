use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use uuid::Uuid;

use crate::api::pagination::PaginationParams;
use crate::api::response::{DataResponse, MessageResponse, Paginated, PaginationMeta};
use crate::app::AppState;
use crate::auth::RequireAuth;
use crate::domain::CreateTenderRequest;
use crate::error::ApiError;

/// Database row for tender
#[derive(Debug, sqlx::FromRow)]
struct TenderRow {
    id: Uuid,
    project_id: Uuid,
    name: String,
    description: Option<String>,
    trade_category: String,
    scope_of_work: Option<String>,
    status: String,
    bid_due_date: Option<DateTime<Utc>>,
    estimated_value: Option<i64>,
    bids_received: Option<i32>,
    bids_invited: Option<i32>,
    awarded_to: Option<String>,
    priority: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

/// Response DTO for tender
#[derive(Debug, serde::Serialize)]
pub struct TenderResponse {
    id: Uuid,
    project_id: Uuid,
    name: String,
    description: Option<String>,
    trade_category: String,
    scope_of_work: Option<String>,
    status: String,
    bid_due_date: Option<DateTime<Utc>>,
    estimated_value: Option<i64>,
    bids_received: Option<i32>,
    bids_invited: Option<i32>,
    awarded_to: Option<String>,
    priority: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<TenderRow> for TenderResponse {
    fn from(row: TenderRow) -> Self {
        Self {
            id: row.id,
            project_id: row.project_id,
            name: row.name,
            description: row.description,
            trade_category: row.trade_category,
            scope_of_work: row.scope_of_work,
            status: row.status,
            bid_due_date: row.bid_due_date,
            estimated_value: row.estimated_value,
            bids_received: row.bids_received,
            bids_invited: row.bids_invited,
            awarded_to: row.awarded_to,
            priority: row.priority,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// Create a new tender package for a project
pub async fn create_tender(
    auth: RequireAuth,
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
    Json(req): Json<CreateTenderRequest>,
) -> Result<impl IntoResponse, ApiError> {
    tracing::info!(
        user_id = %auth.user_id,
        project_id = %project_id,
        tender_name = %req.name,
        trade_category = ?req.trade_category,
        "Creating tender"
    );

    let trade_category = match req.trade_category {
        crate::domain::TradeCategory::GeneralConditions => "general_conditions",
        crate::domain::TradeCategory::SiteworkExcavation => "sitework_excavation",
        crate::domain::TradeCategory::Concrete => "concrete",
        crate::domain::TradeCategory::Masonry => "masonry",
        crate::domain::TradeCategory::Metals => "metals",
        crate::domain::TradeCategory::WoodPlastics => "wood_plastics",
        crate::domain::TradeCategory::ThermalMoisture => "thermal_moisture",
        crate::domain::TradeCategory::DoorsWindows => "doors_windows",
        crate::domain::TradeCategory::Finishes => "finishes",
        crate::domain::TradeCategory::Specialties => "specialties",
        crate::domain::TradeCategory::Equipment => "equipment",
        crate::domain::TradeCategory::Furnishings => "furnishings",
        crate::domain::TradeCategory::SpecialConstruction => "special_construction",
        crate::domain::TradeCategory::ConveyingSystems => "conveying_systems",
        crate::domain::TradeCategory::Mechanical => "mechanical",
        crate::domain::TradeCategory::Electrical => "electrical",
        crate::domain::TradeCategory::Plumbing => "plumbing",
        crate::domain::TradeCategory::Hvac => "hvac",
        crate::domain::TradeCategory::FireProtection => "fire_protection",
        crate::domain::TradeCategory::Other => "other",
    };

    let tender = sqlx::query_as::<_, TenderRow>(
        r#"
        INSERT INTO tenders (project_id, name, description, trade_category, scope_of_work,
                            status, bid_due_date, estimated_value, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, 'draft', $6, $7, NOW(), NOW())
        RETURNING id, project_id, name, description, trade_category, scope_of_work,
                  status, bid_due_date, estimated_value, bids_received, bids_invited,
                  awarded_to, priority, created_at, updated_at
        "#,
    )
    .bind(project_id)
    .bind(&req.name)
    .bind(&req.description)
    .bind(trade_category)
    .bind(&req.scope_of_work)
    .bind(&req.bid_due_date)
    .bind(&req.estimated_value)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let response: TenderResponse = tender.into();
    Ok((StatusCode::CREATED, Json(DataResponse::new(response))))
}

/// List tenders for a project
pub async fn list_tenders(
    _auth: RequireAuth,
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
    Query(pagination): Query<PaginationParams>,
) -> Result<impl IntoResponse, ApiError> {
    let page = pagination.page.unwrap_or(1).max(1);
    let per_page = pagination.per_page.unwrap_or(20).min(100);
    let offset = ((page - 1) * per_page) as i64;

    // Get total count
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tenders WHERE project_id = $1")
        .bind(project_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    // Get tenders
    let tenders = sqlx::query_as::<_, TenderRow>(
        r#"
        SELECT id, project_id, name, description, trade_category, scope_of_work,
               status, bid_due_date, estimated_value, 
               COALESCE(bids_received, 0) as bids_received,
               COALESCE(bids_invited, 0) as bids_invited,
               awarded_to, priority, created_at, updated_at
        FROM tenders
        WHERE project_id = $1
        ORDER BY created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(project_id)
    .bind(per_page as i64)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let data: Vec<TenderResponse> = tenders.into_iter().map(Into::into).collect();
    let total_pages = ((total as f64) / (per_page as f64)).ceil() as u32;

    let response = Paginated {
        data,
        pagination: PaginationMeta {
            page,
            per_page,
            total_items: total as u64,
            total_pages,
            has_next: page < total_pages,
            has_prev: page > 1,
        },
    };

    Ok(Json(response))
}

/// List all tenders for the current user across all projects
pub async fn list_all_tenders(
    auth: RequireAuth,
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<PaginationParams>,
) -> Result<impl IntoResponse, ApiError> {
    let page = pagination.page.unwrap_or(1).max(1);
    let per_page = pagination.per_page.unwrap_or(20).min(100);
    let offset = ((page - 1) * per_page) as i64;

    // Get total count for projects owned by user
    let total: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM tenders t
        JOIN projects p ON t.project_id = p.id
        WHERE p.owner_id = $1
        "#,
    )
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    // Get tenders
    let tenders = sqlx::query_as::<_, TenderRow>(
        r#"
        SELECT t.id, t.project_id, t.name, t.description, t.trade_category, t.scope_of_work,
               t.status, t.bid_due_date, t.estimated_value,
               COALESCE(t.bids_received, 0) as bids_received,
               COALESCE(t.bids_invited, 0) as bids_invited,
               t.awarded_to, t.priority, t.created_at, t.updated_at
        FROM tenders t
        JOIN projects p ON t.project_id = p.id
        WHERE p.owner_id = $1
        ORDER BY t.created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(auth.user_id)
    .bind(per_page as i64)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let data: Vec<TenderResponse> = tenders.into_iter().map(Into::into).collect();
    let total_pages = ((total as f64) / (per_page as f64)).ceil() as u32;

    let response = Paginated {
        data,
        pagination: PaginationMeta {
            page,
            per_page,
            total_items: total as u64,
            total_pages,
            has_next: page < total_pages,
            has_prev: page > 1,
        },
    };

    Ok(Json(response))
}
