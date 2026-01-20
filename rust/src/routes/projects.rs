//! Project routes
//!
//! CRUD operations for construction projects.

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
use crate::api::response::{DataResponse, Paginated, PaginationMeta};
use crate::app::AppState;
use crate::auth::RequireAuth;
use crate::domain::{CreateProjectRequest, ProjectResponse, ProjectStatus, UpdateProjectRequest};
use crate::error::ApiError;

/// Database row for project
#[derive(Debug, sqlx::FromRow)]
struct ProjectRow {
    id: Uuid,
    owner_id: Uuid,
    name: String,
    description: Option<String>,
    address: Option<String>,
    city: Option<String>,
    state: Option<String>,
    zip_code: Option<String>,
    status: String,
    estimated_value: Option<rust_decimal::Decimal>,
    bid_due_date: Option<DateTime<Utc>>,
    start_date: Option<DateTime<Utc>>,
    end_date: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<ProjectRow> for ProjectResponse {
    type Error = ApiError;

    fn try_from(row: ProjectRow) -> Result<Self, Self::Error> {
        let status = match row.status.as_str() {
            "draft" => ProjectStatus::Draft,
            "active" => ProjectStatus::Active,
            "bidding" => ProjectStatus::Bidding,
            "awarded" => ProjectStatus::Awarded,
            "in_progress" => ProjectStatus::InProgress,
            "completed" => ProjectStatus::Completed,
            "cancelled" => ProjectStatus::Cancelled,
            _ => ProjectStatus::Draft,
        };

        // Convert decimal to cents (i64)
        let estimated_value = row
            .estimated_value
            .map(|d| (d * rust_decimal::Decimal::from(100)).to_i64().unwrap_or(0));

        Ok(Self {
            id: row.id,
            name: row.name,
            description: row.description,
            address: row.address,
            city: row.city,
            state: row.state,
            zip_code: row.zip_code,
            status,
            estimated_value,
            bid_due_date: row.bid_due_date,
            start_date: row.start_date,
            end_date: row.end_date,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

/// POST /api/projects
///
/// Create a new project. Only GCs can create projects.
pub async fn create_project(
    auth: RequireAuth,
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateProjectRequest>,
) -> Result<impl IntoResponse, ApiError> {
    tracing::info!(
        user_id = %auth.user_id,
        project_name = %req.name,
        "Creating project"
    );

    // Convert cents to decimal for storage
    let estimated_value = req
        .estimated_value
        .map(|cents| rust_decimal::Decimal::from(cents) / rust_decimal::Decimal::from(100));

    let project = sqlx::query_as::<_, ProjectRow>(
        r#"
        INSERT INTO projects (owner_id, name, description, address, city, state, zip_code, status, estimated_value, bid_due_date, start_date, end_date)
        VALUES ($1, $2, $3, $4, $5, $6, $7, 'draft', $8, $9, $10, $11)
        RETURNING id, owner_id, name, description, address, city, state, zip_code, status, estimated_value, bid_due_date, start_date, end_date, created_at, updated_at
        "#,
    )
    .bind(auth.user_id)
    .bind(&req.name)
    .bind(&req.description)
    .bind(&req.address)
    .bind(&req.city)
    .bind(&req.state)
    .bind(&req.zip_code)
    .bind(estimated_value)
    .bind(req.bid_due_date)
    .bind(req.start_date)
    .bind(req.end_date)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to create project: {}", e)))?;

    let response: ProjectResponse = project.try_into()?;
    Ok((StatusCode::CREATED, Json(DataResponse::new(response))))
}

/// GET /api/projects
///
/// List projects for the authenticated user.
pub async fn list_projects(
    auth: RequireAuth,
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<PaginationParams>,
) -> Result<impl IntoResponse, ApiError> {
    tracing::info!(
        user_id = %auth.user_id,
        page = pagination.page(),
        per_page = pagination.per_page(),
        "Listing projects"
    );

    let offset = pagination.offset() as i64;
    let limit = pagination.limit() as i64;

    // Get total count
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM projects WHERE owner_id = $1")
        .bind(auth.user_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    // Get projects
    let projects = sqlx::query_as::<_, ProjectRow>(
        r#"
        SELECT id, owner_id, name, description, address, city, state, zip_code, status, estimated_value, bid_due_date, start_date, end_date, created_at, updated_at
        FROM projects
        WHERE owner_id = $1
        ORDER BY created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(auth.user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let data: Vec<ProjectResponse> = projects
        .into_iter()
        .filter_map(|row| row.try_into().ok())
        .collect();

    Ok(Json(Paginated::new(data, &pagination, total as u64)))
}

/// GET /api/projects/:project_id
///
/// Get a specific project by ID.
pub async fn get_project(
    auth: RequireAuth,
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    tracing::info!(
        user_id = %auth.user_id,
        project_id = %project_id,
        "Getting project"
    );

    let project = sqlx::query_as::<_, ProjectRow>(
        r#"
        SELECT id, owner_id, name, description, address, city, state, zip_code, status, estimated_value, bid_due_date, start_date, end_date, created_at, updated_at
        FROM projects
        WHERE id = $1 AND owner_id = $2
        "#,
    )
    .bind(project_id)
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
    .ok_or_else(|| ApiError::not_found("Project not found"))?;

    let response: ProjectResponse = project.try_into()?;
    Ok(Json(DataResponse::new(response)))
}

/// PUT /api/projects/:project_id
///
/// Update a project.
pub async fn update_project(
    auth: RequireAuth,
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
    Json(req): Json<UpdateProjectRequest>,
) -> Result<impl IntoResponse, ApiError> {
    tracing::info!(
        user_id = %auth.user_id,
        project_id = %project_id,
        "Updating project"
    );

    // First check ownership
    let exists: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM projects WHERE id = $1 AND owner_id = $2"
    )
    .bind(project_id)
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    if exists.is_none() {
        return Err(ApiError::not_found("Project not found"));
    }

    // Convert status to string
    let status = req.status.as_ref().map(|s| match s {
        ProjectStatus::Draft => "draft",
        ProjectStatus::Active => "active",
        ProjectStatus::Bidding => "bidding",
        ProjectStatus::Awarded => "awarded",
        ProjectStatus::InProgress => "in_progress",
        ProjectStatus::Completed => "completed",
        ProjectStatus::Cancelled => "cancelled",
    });

    // Convert cents to decimal
    let estimated_value = req
        .estimated_value
        .map(|cents| rust_decimal::Decimal::from(cents) / rust_decimal::Decimal::from(100));

    let project = sqlx::query_as::<_, ProjectRow>(
        r#"
        UPDATE projects SET
            name = COALESCE($3, name),
            description = COALESCE($4, description),
            address = COALESCE($5, address),
            city = COALESCE($6, city),
            state = COALESCE($7, state),
            zip_code = COALESCE($8, zip_code),
            status = COALESCE($9, status),
            estimated_value = COALESCE($10, estimated_value),
            bid_due_date = COALESCE($11, bid_due_date),
            start_date = COALESCE($12, start_date),
            end_date = COALESCE($13, end_date),
            updated_at = NOW()
        WHERE id = $1 AND owner_id = $2
        RETURNING id, owner_id, name, description, address, city, state, zip_code, status, estimated_value, bid_due_date, start_date, end_date, created_at, updated_at
        "#,
    )
    .bind(project_id)
    .bind(auth.user_id)
    .bind(&req.name)
    .bind(&req.description)
    .bind(&req.address)
    .bind(&req.city)
    .bind(&req.state)
    .bind(&req.zip_code)
    .bind(status)
    .bind(estimated_value)
    .bind(req.bid_due_date)
    .bind(req.start_date)
    .bind(req.end_date)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to update project: {}", e)))?;

    let response: ProjectResponse = project.try_into()?;
    Ok(Json(DataResponse::new(response)))
}

/// DELETE /api/projects/:project_id
///
/// Delete a project.
pub async fn delete_project(
    auth: RequireAuth,
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    tracing::info!(
        user_id = %auth.user_id,
        project_id = %project_id,
        "Deleting project"
    );

    let result = sqlx::query("DELETE FROM projects WHERE id = $1 AND owner_id = $2")
        .bind(project_id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Project not found"));
    }

    Ok(StatusCode::NO_CONTENT)
}
