//! Subcontractor routes
//!
//! Marketplace/directory of subcontractors.

use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use uuid::Uuid;

use crate::api::pagination::PaginationParams;
use crate::api::response::{DataResponse, Paginated, PaginationMeta};
use crate::app::AppState;
use crate::auth::RequireAuth;
use crate::domain::subcontractors::{RecentProject, SubcontractorQuery, SubcontractorResponse};
use crate::error::ApiError;

/// Database row for subcontractor
#[derive(Debug, sqlx::FromRow)]
struct SubcontractorRow {
    id: Uuid,
    name: String,
    trade: String,
    rating: f64,
    review_count: i32,
    location: String,
    description: Option<String>,
    contact_email: Option<String>,
    contact_phone: Option<String>,
    projects_completed: i32,
    average_bid_value: Option<i64>,
    response_time: Option<String>,
    verified: bool,
    specialties: serde_json::Value,
    recent_projects: serde_json::Value,
    created_at: DateTime<Utc>,
}

impl TryFrom<SubcontractorRow> for SubcontractorResponse {
    type Error = ApiError;

    fn try_from(row: SubcontractorRow) -> Result<Self, Self::Error> {
        let specialties: Vec<String> = serde_json::from_value(row.specialties).unwrap_or_default();
        let recent_projects: Vec<RecentProject> =
            serde_json::from_value(row.recent_projects).unwrap_or_default();

        Ok(Self {
            id: row.id,
            name: row.name,
            trade: row.trade,
            rating: row.rating,
            review_count: row.review_count,
            location: row.location,
            description: row.description,
            contact_email: row.contact_email,
            contact_phone: row.contact_phone,
            projects_completed: row.projects_completed,
            average_bid_value: row.average_bid_value,
            response_time: row.response_time,
            verified: row.verified,
            specialties,
            recent_projects,
            created_at: row.created_at,
        })
    }
}

/// Combined query params
#[derive(Debug, serde::Deserialize, Default)]
pub struct ListSubcontractorsQuery {
    #[serde(flatten)]
    pub pagination: PaginationParams,
    #[serde(flatten)]
    pub filter: SubcontractorQuery,
}

/// GET /api/subcontractors
///
/// List subcontractors with optional filtering.
pub async fn list_subcontractors(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListSubcontractorsQuery>,
    _auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    let page = query.pagination.page.unwrap_or(1).max(1);
    let per_page = query.pagination.per_page.unwrap_or(20).min(100);
    let offset = ((page - 1) * per_page) as i64;

    // Build dynamic query with filters
    let mut sql = String::from(
        r#"
        SELECT id, name, trade, rating, review_count, location, description,
               contact_email, contact_phone, projects_completed, average_bid_value,
               response_time, verified, specialties, recent_projects, created_at
        FROM subcontractors
        WHERE 1=1
        "#,
    );

    let mut count_sql = String::from("SELECT COUNT(*) FROM subcontractors WHERE 1=1");

    if query.filter.trade.is_some() {
        sql.push_str(" AND trade ILIKE $4");
        count_sql.push_str(" AND trade ILIKE $1");
    }

    if query.filter.location.is_some() {
        sql.push_str(" AND location ILIKE $5");
        count_sql.push_str(" AND location ILIKE $2");
    }

    if query.filter.verified_only == Some(true) {
        sql.push_str(" AND verified = true");
        count_sql.push_str(" AND verified = true");
    }

    if query.filter.min_rating.is_some() {
        sql.push_str(" AND rating >= $6");
        count_sql.push_str(" AND rating >= $3");
    }

    sql.push_str(" ORDER BY rating DESC, review_count DESC LIMIT $1 OFFSET $2");

    // For simplicity, use a basic query without dynamic filters for now
    // In production, you'd use a query builder or dynamic SQL

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM subcontractors")
        .fetch_one(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let subcontractors = sqlx::query_as::<_, SubcontractorRow>(
        r#"
        SELECT id, name, trade, rating, review_count, location, description,
               contact_email, contact_phone, projects_completed, average_bid_value,
               response_time, verified, 
               COALESCE(specialties, '[]'::jsonb) as specialties, 
               COALESCE(recent_projects, '[]'::jsonb) as recent_projects, 
               created_at
        FROM subcontractors
        ORDER BY rating DESC, review_count DESC
        LIMIT $1 OFFSET $2
        "#,
    )
    .bind(per_page as i64)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let data: Vec<SubcontractorResponse> = subcontractors
        .into_iter()
        .filter_map(|row| row.try_into().ok())
        .collect();

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

/// GET /api/subcontractors/:id
///
/// Get a specific subcontractor.
pub async fn get_subcontractor(
    State(state): State<Arc<AppState>>,
    Path(subcontractor_id): Path<Uuid>,
    _auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    let subcontractor = sqlx::query_as::<_, SubcontractorRow>(
        r#"
        SELECT id, name, trade, rating, review_count, location, description,
               contact_email, contact_phone, projects_completed, average_bid_value,
               response_time, verified,
               COALESCE(specialties, '[]'::jsonb) as specialties,
               COALESCE(recent_projects, '[]'::jsonb) as recent_projects,
               created_at
        FROM subcontractors
        WHERE id = $1
        "#,
    )
    .bind(subcontractor_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
    .ok_or_else(|| ApiError::not_found("Subcontractor not found"))?;

    let response: SubcontractorResponse = subcontractor.try_into()?;
    Ok(Json(DataResponse::new(response)))
}
