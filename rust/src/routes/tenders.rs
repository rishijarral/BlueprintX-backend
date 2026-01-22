//! Tender routes
//!
//! CRUD operations for tender packages (bid invitations) with Redis caching.

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
use crate::domain::tenders::{CreateTenderRequest, TradeCategory, UpdateTenderRequest};
use crate::error::ApiError;
use crate::services::cache::{keys as cache_keys, ttl as cache_ttl};

/// Database row for tender with computed bid counts
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
    estimated_value: Option<rust_decimal::Decimal>,
    awarded_to: Option<Uuid>,
    priority: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    bids_received: Option<i64>,
}

/// Response DTO for tender
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
    bids_received: i32,
    bids_invited: i32,
    awarded_to: Option<String>,
    priority: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

/// Cached paginated response for tenders
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct CachedTenderList {
    data: Vec<TenderResponse>,
    total: u64,
}

impl From<TenderRow> for TenderResponse {
    fn from(row: TenderRow) -> Self {
        // Convert decimal to cents
        let estimated_value = row
            .estimated_value
            .map(|d| (d * rust_decimal::Decimal::from(100)).to_i64().unwrap_or(0));

        Self {
            id: row.id,
            project_id: row.project_id,
            name: row.name,
            description: row.description,
            trade_category: row.trade_category,
            scope_of_work: row.scope_of_work,
            status: row.status,
            bid_due_date: row.bid_due_date,
            estimated_value,
            bids_received: row.bids_received.unwrap_or(0) as i32,
            bids_invited: 0, // Not tracked in current schema
            awarded_to: row.awarded_to.map(|id| id.to_string()),
            priority: row.priority,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
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

fn trade_category_to_string(cat: &TradeCategory) -> &'static str {
    match cat {
        TradeCategory::GeneralConditions => "general_conditions",
        TradeCategory::SiteworkExcavation => "sitework_excavation",
        TradeCategory::Concrete => "concrete",
        TradeCategory::Masonry => "masonry",
        TradeCategory::Metals => "metals",
        TradeCategory::WoodPlastics => "wood_plastics",
        TradeCategory::ThermalMoisture => "thermal_moisture",
        TradeCategory::DoorsWindows => "doors_windows",
        TradeCategory::Finishes => "finishes",
        TradeCategory::Specialties => "specialties",
        TradeCategory::Equipment => "equipment",
        TradeCategory::Furnishings => "furnishings",
        TradeCategory::SpecialConstruction => "special_construction",
        TradeCategory::ConveyingSystems => "conveying_systems",
        TradeCategory::Mechanical => "mechanical",
        TradeCategory::Electrical => "electrical",
        TradeCategory::Plumbing => "plumbing",
        TradeCategory::Hvac => "hvac",
        TradeCategory::FireProtection => "fire_protection",
        TradeCategory::Other => "other",
    }
}

/// POST /api/projects/:project_id/tenders
///
/// Create a new tender package for a project. Invalidates tender list caches.
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
        "Creating tender"
    );

    verify_project_ownership(&state, project_id, auth.user_id).await?;

    let trade_category = trade_category_to_string(&req.trade_category);

    // Convert cents to decimal
    let estimated_value = req
        .estimated_value
        .map(|cents| rust_decimal::Decimal::from(cents) / rust_decimal::Decimal::from(100));

    let tender = sqlx::query_as::<_, TenderRow>(
        r#"
        INSERT INTO tenders (project_id, name, description, trade_category, scope_of_work, status, bid_due_date, estimated_value)
        VALUES ($1, $2, $3, $4, $5, 'draft', $6, $7)
        RETURNING id, project_id, name, description, trade_category, scope_of_work, status, bid_due_date, estimated_value, awarded_to, priority, created_at, updated_at,
                  (SELECT COUNT(*) FROM bids WHERE tender_id = id) as bids_received
        "#,
    )
    .bind(project_id)
    .bind(&req.name)
    .bind(&req.description)
    .bind(trade_category)
    .bind(&req.scope_of_work)
    .bind(req.bid_due_date)
    .bind(estimated_value)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to create tender: {}", e)))?;

    let response: TenderResponse = tender.into();

    // Invalidate tender list caches
    let _ = state.cache.delete_pattern(&cache_keys::tender_list_pattern(project_id)).await;
    let _ = state.cache.delete(&cache_keys::tender_count(project_id)).await;
    let _ = state.cache.delete_pattern(&cache_keys::tender_user_pattern(auth.user_id)).await;
    let _ = state.cache.delete(&cache_keys::tender_count_all(auth.user_id)).await;
    // Invalidate dashboard
    let _ = state.cache.delete(&cache_keys::dashboard_stats(auth.user_id)).await;

    Ok((StatusCode::CREATED, Json(DataResponse::new(response))))
}

/// GET /api/projects/:project_id/tenders
///
/// List tenders for a project. Uses Redis cache for performance.
pub async fn list_tenders(
    auth: RequireAuth,
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
    Query(pagination): Query<PaginationParams>,
) -> Result<impl IntoResponse, ApiError> {
    verify_project_ownership(&state, project_id, auth.user_id).await?;

    let page = pagination.page();
    let per_page = pagination.per_page();
    let cache_key = cache_keys::tender_list(project_id, page, per_page);

    // Try cache first
    if let Some(cached) = state.cache.get::<CachedTenderList>(&cache_key).await {
        tracing::debug!(project_id = %project_id, "Tenders list cache hit");
        return Ok(Json(Paginated::new(cached.data, &pagination, cached.total)));
    }

    let offset = pagination.offset() as i64;
    let limit = pagination.limit() as i64;

    // Get total count (with caching)
    let count_cache_key = cache_keys::tender_count(project_id);
    let total: i64 = if let Some(cached_count) = state.cache.get::<i64>(&count_cache_key).await {
        cached_count
    } else {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tenders WHERE project_id = $1")
            .bind(project_id)
            .fetch_one(&state.db)
            .await
            .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;
        let _ = state.cache.set_with_ttl(&count_cache_key, &count, cache_ttl::COUNT).await;
        count
    };

    // Get tenders with bid count
    let tenders = sqlx::query_as::<_, TenderRow>(
        r#"
        SELECT t.id, t.project_id, t.name, t.description, t.trade_category, t.scope_of_work,
               t.status, t.bid_due_date, t.estimated_value, t.awarded_to, t.priority,
               t.created_at, t.updated_at,
               (SELECT COUNT(*) FROM bids WHERE tender_id = t.id) as bids_received
        FROM tenders t
        WHERE t.project_id = $1
        ORDER BY t.created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(project_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let data: Vec<TenderResponse> = tenders.into_iter().map(Into::into).collect();

    // Cache the result
    let cached = CachedTenderList { data: data.clone(), total: total as u64 };
    let _ = state.cache.set_with_ttl(&cache_key, &cached, cache_ttl::LIST).await;

    Ok(Json(Paginated::new(data, &pagination, total as u64)))
}

/// GET /api/tenders
///
/// List all tenders for the current user across all projects. Uses Redis cache.
pub async fn list_all_tenders(
    auth: RequireAuth,
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<PaginationParams>,
) -> Result<impl IntoResponse, ApiError> {
    let page = pagination.page();
    let per_page = pagination.per_page();
    let cache_key = cache_keys::tender_list_all(auth.user_id, page, per_page);

    // Try cache first
    if let Some(cached) = state.cache.get::<CachedTenderList>(&cache_key).await {
        tracing::debug!(user_id = %auth.user_id, "All tenders list cache hit");
        return Ok(Json(Paginated::new(cached.data, &pagination, cached.total)));
    }

    let offset = pagination.offset() as i64;
    let limit = pagination.limit() as i64;

    // Get total count (with caching)
    let count_cache_key = cache_keys::tender_count_all(auth.user_id);
    let total: i64 = if let Some(cached_count) = state.cache.get::<i64>(&count_cache_key).await {
        cached_count
    } else {
        let count: i64 = sqlx::query_scalar(
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
        let _ = state.cache.set_with_ttl(&count_cache_key, &count, cache_ttl::COUNT).await;
        count
    };

    // Get tenders with bid count
    let tenders = sqlx::query_as::<_, TenderRow>(
        r#"
        SELECT t.id, t.project_id, t.name, t.description, t.trade_category, t.scope_of_work,
               t.status, t.bid_due_date, t.estimated_value, t.awarded_to, t.priority,
               t.created_at, t.updated_at,
               (SELECT COUNT(*) FROM bids WHERE tender_id = t.id) as bids_received
        FROM tenders t
        JOIN projects p ON t.project_id = p.id
        WHERE p.owner_id = $1
        ORDER BY t.created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(auth.user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let data: Vec<TenderResponse> = tenders.into_iter().map(Into::into).collect();

    // Cache the result
    let cached = CachedTenderList { data: data.clone(), total: total as u64 };
    let _ = state.cache.set_with_ttl(&cache_key, &cached, cache_ttl::LIST).await;

    Ok(Json(Paginated::new(data, &pagination, total as u64)))
}

/// GET /api/projects/:project_id/tenders/:tender_id
///
/// Get a specific tender.
pub async fn get_tender(
    auth: RequireAuth,
    State(state): State<Arc<AppState>>,
    Path((project_id, tender_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, ApiError> {
    verify_project_ownership(&state, project_id, auth.user_id).await?;

    let tender = sqlx::query_as::<_, TenderRow>(
        r#"
        SELECT t.id, t.project_id, t.name, t.description, t.trade_category, t.scope_of_work,
               t.status, t.bid_due_date, t.estimated_value, t.awarded_to, t.priority,
               t.created_at, t.updated_at,
               (SELECT COUNT(*) FROM bids WHERE tender_id = t.id) as bids_received
        FROM tenders t
        WHERE t.id = $1 AND t.project_id = $2
        "#,
    )
    .bind(tender_id)
    .bind(project_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
    .ok_or_else(|| ApiError::not_found("Tender not found"))?;

    let response: TenderResponse = tender.into();
    Ok(Json(DataResponse::new(response)))
}

/// PUT /api/projects/:project_id/tenders/:tender_id
///
/// Update a tender.
pub async fn update_tender(
    auth: RequireAuth,
    State(state): State<Arc<AppState>>,
    Path((project_id, tender_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateTenderRequest>,
) -> Result<impl IntoResponse, ApiError> {
    verify_project_ownership(&state, project_id, auth.user_id).await?;

    // Check tender exists
    let exists: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM tenders WHERE id = $1 AND project_id = $2")
            .bind(tender_id)
            .bind(project_id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    if exists.is_none() {
        return Err(ApiError::not_found("Tender not found"));
    }

    let trade_category = req.trade_category.as_ref().map(trade_category_to_string);
    let status = req.status.as_ref().map(|s| match s {
        crate::domain::tenders::TenderStatus::Draft => "draft",
        crate::domain::tenders::TenderStatus::Published => "open",
        crate::domain::tenders::TenderStatus::Closed => "closed",
        crate::domain::tenders::TenderStatus::Awarded => "awarded",
        crate::domain::tenders::TenderStatus::Cancelled => "cancelled",
    });

    let estimated_value = req
        .estimated_value
        .map(|cents| rust_decimal::Decimal::from(cents) / rust_decimal::Decimal::from(100));

    let tender = sqlx::query_as::<_, TenderRow>(
        r#"
        UPDATE tenders SET
            name = COALESCE($3, name),
            description = COALESCE($4, description),
            trade_category = COALESCE($5, trade_category),
            scope_of_work = COALESCE($6, scope_of_work),
            status = COALESCE($7, status),
            bid_due_date = COALESCE($8, bid_due_date),
            estimated_value = COALESCE($9, estimated_value),
            updated_at = NOW()
        WHERE id = $1 AND project_id = $2
        RETURNING id, project_id, name, description, trade_category, scope_of_work, status, bid_due_date, estimated_value, awarded_to, priority, created_at, updated_at,
                  (SELECT COUNT(*) FROM bids WHERE tender_id = id) as bids_received
        "#,
    )
    .bind(tender_id)
    .bind(project_id)
    .bind(&req.name)
    .bind(&req.description)
    .bind(trade_category)
    .bind(&req.scope_of_work)
    .bind(status)
    .bind(req.bid_due_date)
    .bind(estimated_value)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to update tender: {}", e)))?;

    let response: TenderResponse = tender.into();

    // Invalidate tender list caches
    let _ = state.cache.delete_pattern(&cache_keys::tender_list_pattern(project_id)).await;
    let _ = state.cache.delete_pattern(&cache_keys::tender_user_pattern(auth.user_id)).await;
    // Invalidate dashboard
    let _ = state.cache.delete(&cache_keys::dashboard_stats(auth.user_id)).await;

    Ok(Json(DataResponse::new(response)))
}

/// DELETE /api/projects/:project_id/tenders/:tender_id
///
/// Delete a tender. Invalidates tender list caches.
pub async fn delete_tender(
    auth: RequireAuth,
    State(state): State<Arc<AppState>>,
    Path((project_id, tender_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, ApiError> {
    verify_project_ownership(&state, project_id, auth.user_id).await?;

    let result = sqlx::query("DELETE FROM tenders WHERE id = $1 AND project_id = $2")
        .bind(tender_id)
        .bind(project_id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Tender not found"));
    }

    // Invalidate tender list caches
    let _ = state.cache.delete_pattern(&cache_keys::tender_list_pattern(project_id)).await;
    let _ = state.cache.delete(&cache_keys::tender_count(project_id)).await;
    let _ = state.cache.delete_pattern(&cache_keys::tender_user_pattern(auth.user_id)).await;
    let _ = state.cache.delete(&cache_keys::tender_count_all(auth.user_id)).await;
    // Invalidate dashboard
    let _ = state.cache.delete(&cache_keys::dashboard_stats(auth.user_id)).await;

    Ok(StatusCode::NO_CONTENT)
}
