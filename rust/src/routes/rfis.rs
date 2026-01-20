//! RFI routes
//!
//! Request for Information management endpoints.

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
use crate::domain::rfis::{
    CreateRFIRequest, CreateRFIResponseRequest, RFIPriority, RFIResponse, RFIResponseDTO,
    RFIStatus, UpdateRFIRequest,
};
use crate::error::ApiError;

/// Database row for RFI
#[derive(Debug, sqlx::FromRow)]
struct RFIRow {
    id: Uuid,
    project_id: Uuid,
    number: i32,
    title: String,
    description: String,
    status: String,
    priority: String,
    requester: Option<String>,
    requester_id: Uuid,
    assignee: Option<String>,
    assignee_id: Option<Uuid>,
    category: Option<String>,
    due_date: Option<DateTime<Utc>>,
    responses_count: i32,
    attachments_count: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<RFIRow> for RFIResponse {
    fn from(row: RFIRow) -> Self {
        Self {
            id: row.id,
            project_id: row.project_id,
            number: row.number,
            title: row.title,
            description: row.description,
            status: match row.status.as_str() {
                "answered" => RFIStatus::Answered,
                "closed" => RFIStatus::Closed,
                _ => RFIStatus::Open,
            },
            priority: match row.priority.as_str() {
                "low" => RFIPriority::Low,
                "high" => RFIPriority::High,
                "urgent" => RFIPriority::Urgent,
                _ => RFIPriority::Medium,
            },
            requester: row.requester.unwrap_or_default(),
            requester_id: row.requester_id,
            assignee: row.assignee.unwrap_or_default(),
            assignee_id: row.assignee_id,
            category: row.category,
            due_date: row.due_date,
            responses_count: row.responses_count,
            attachments_count: row.attachments_count,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// Database row for RFI response
#[derive(Debug, sqlx::FromRow)]
struct RFIResponseRow {
    id: Uuid,
    rfi_id: Uuid,
    content: String,
    author: Option<String>,
    author_id: Uuid,
    created_at: DateTime<Utc>,
}

impl From<RFIResponseRow> for RFIResponseDTO {
    fn from(row: RFIResponseRow) -> Self {
        Self {
            id: row.id,
            rfi_id: row.rfi_id,
            content: row.content,
            author: row.author.unwrap_or_default(),
            author_id: row.author_id,
            created_at: row.created_at,
        }
    }
}

/// GET /api/projects/:project_id/rfis
///
/// List RFIs for a project.
pub async fn list_rfis(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
    Query(pagination): Query<PaginationParams>,
    _auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    let page = pagination.page.unwrap_or(1).max(1);
    let per_page = pagination.per_page.unwrap_or(20).min(100);
    let offset = ((page - 1) * per_page) as i64;

    // Get total count
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM rfis WHERE project_id = $1")
        .bind(project_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    // Get RFIs
    let rfis = sqlx::query_as::<_, RFIRow>(
        r#"
        SELECT r.id, r.project_id, r.number, r.title, r.description, r.status, r.priority,
               req.first_name || ' ' || req.last_name as requester, r.requester_id,
               asg.first_name || ' ' || asg.last_name as assignee, r.assignee_id,
               r.category, r.due_date,
               (SELECT COUNT(*)::int FROM rfi_responses WHERE rfi_id = r.id) as responses_count,
               0 as attachments_count,
               r.created_at, r.updated_at
        FROM rfis r
        LEFT JOIN profiles req ON r.requester_id = req.id
        LEFT JOIN profiles asg ON r.assignee_id = asg.id
        WHERE r.project_id = $1
        ORDER BY r.created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(project_id)
    .bind(per_page as i64)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let data: Vec<RFIResponse> = rfis.into_iter().map(Into::into).collect();
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

/// GET /api/rfis
///
/// List all RFIs for the current user across all projects.
pub async fn list_all_rfis(
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<PaginationParams>,
    auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    let page = pagination.page.unwrap_or(1).max(1);
    let per_page = pagination.per_page.unwrap_or(20).min(100);
    let offset = ((page - 1) * per_page) as i64;

    // Get total count
    let total: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM rfis r
        JOIN projects p ON r.project_id = p.id
        WHERE p.owner_id = $1
        "#,
    )
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    // Get RFIs
    let rfis = sqlx::query_as::<_, RFIRow>(
        r#"
        SELECT r.id, r.project_id, r.number, r.title, r.description, r.status, r.priority,
               req.first_name || ' ' || req.last_name as requester, r.requester_id,
               asg.first_name || ' ' || asg.last_name as assignee, r.assignee_id,
               r.category, r.due_date,
               (SELECT COUNT(*)::int FROM rfi_responses WHERE rfi_id = r.id) as responses_count,
               0 as attachments_count,
               r.created_at, r.updated_at
        FROM rfis r
        JOIN projects p ON r.project_id = p.id
        LEFT JOIN profiles req ON r.requester_id = req.id
        LEFT JOIN profiles asg ON r.assignee_id = asg.id
        WHERE p.owner_id = $1
        ORDER BY r.created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(auth.user_id)
    .bind(per_page as i64)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let data: Vec<RFIResponse> = rfis.into_iter().map(Into::into).collect();
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

/// GET /api/projects/:project_id/rfis/:rfi_id
///
/// Get a specific RFI.
pub async fn get_rfi(
    State(state): State<Arc<AppState>>,
    Path((project_id, rfi_id)): Path<(Uuid, Uuid)>,
    _auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    let rfi = sqlx::query_as::<_, RFIRow>(
        r#"
        SELECT r.id, r.project_id, r.number, r.title, r.description, r.status, r.priority,
               req.first_name || ' ' || req.last_name as requester, r.requester_id,
               asg.first_name || ' ' || asg.last_name as assignee, r.assignee_id,
               r.category, r.due_date,
               (SELECT COUNT(*)::int FROM rfi_responses WHERE rfi_id = r.id) as responses_count,
               0 as attachments_count,
               r.created_at, r.updated_at
        FROM rfis r
        LEFT JOIN profiles req ON r.requester_id = req.id
        LEFT JOIN profiles asg ON r.assignee_id = asg.id
        WHERE r.id = $1 AND r.project_id = $2
        "#,
    )
    .bind(rfi_id)
    .bind(project_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
    .ok_or_else(|| ApiError::not_found("RFI not found"))?;

    let response: RFIResponse = rfi.into();
    Ok(Json(DataResponse::new(response)))
}

/// POST /api/projects/:project_id/rfis
///
/// Create a new RFI.
pub async fn create_rfi(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
    auth: RequireAuth,
    Json(req): Json<CreateRFIRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let priority = match req.priority {
        RFIPriority::Low => "low",
        RFIPriority::High => "high",
        RFIPriority::Urgent => "urgent",
        RFIPriority::Medium => "medium",
    };

    // Get next RFI number for this project
    let next_number: i32 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(number), 0) + 1 FROM rfis WHERE project_id = $1",
    )
    .bind(project_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let rfi = sqlx::query_as::<_, RFIRow>(
        r#"
        INSERT INTO rfis (project_id, number, title, description, status, priority,
                         requester_id, assignee_id, category, due_date, created_at, updated_at)
        VALUES ($1, $2, $3, $4, 'open', $5, $6, $7, $8, $9, NOW(), NOW())
        RETURNING id, project_id, number, title, description, status, priority,
                  NULL as requester, requester_id, NULL as assignee, assignee_id,
                  category, due_date, 0 as responses_count, 0 as attachments_count,
                  created_at, updated_at
        "#,
    )
    .bind(project_id)
    .bind(next_number)
    .bind(&req.title)
    .bind(&req.description)
    .bind(priority)
    .bind(auth.user_id)
    .bind(&req.assignee_id)
    .bind(&req.category)
    .bind(&req.due_date)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let response: RFIResponse = rfi.into();
    Ok((StatusCode::CREATED, Json(DataResponse::new(response))))
}

/// PUT /api/projects/:project_id/rfis/:rfi_id
///
/// Update an RFI.
pub async fn update_rfi(
    State(state): State<Arc<AppState>>,
    Path((project_id, rfi_id)): Path<(Uuid, Uuid)>,
    _auth: RequireAuth,
    Json(req): Json<UpdateRFIRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let status = req.status.map(|s| match s {
        RFIStatus::Answered => "answered",
        RFIStatus::Closed => "closed",
        RFIStatus::Open => "open",
    });

    let priority = req.priority.map(|p| match p {
        RFIPriority::Low => "low",
        RFIPriority::High => "high",
        RFIPriority::Urgent => "urgent",
        RFIPriority::Medium => "medium",
    });

    let rfi = sqlx::query_as::<_, RFIRow>(
        r#"
        UPDATE rfis SET
            title = COALESCE($3, title),
            description = COALESCE($4, description),
            status = COALESCE($5, status),
            priority = COALESCE($6, priority),
            assignee_id = COALESCE($7, assignee_id),
            category = COALESCE($8, category),
            due_date = COALESCE($9, due_date),
            updated_at = NOW()
        WHERE id = $1 AND project_id = $2
        RETURNING id, project_id, number, title, description, status, priority,
                  NULL as requester, requester_id, NULL as assignee, assignee_id,
                  category, due_date,
                  (SELECT COUNT(*)::int FROM rfi_responses WHERE rfi_id = rfis.id) as responses_count,
                  0 as attachments_count,
                  created_at, updated_at
        "#,
    )
    .bind(rfi_id)
    .bind(project_id)
    .bind(&req.title)
    .bind(&req.description)
    .bind(status)
    .bind(priority)
    .bind(&req.assignee_id)
    .bind(&req.category)
    .bind(&req.due_date)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
    .ok_or_else(|| ApiError::not_found("RFI not found"))?;

    let response: RFIResponse = rfi.into();
    Ok(Json(DataResponse::new(response)))
}

/// DELETE /api/projects/:project_id/rfis/:rfi_id
///
/// Delete an RFI.
pub async fn delete_rfi(
    State(state): State<Arc<AppState>>,
    Path((project_id, rfi_id)): Path<(Uuid, Uuid)>,
    _auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    let result = sqlx::query("DELETE FROM rfis WHERE id = $1 AND project_id = $2")
        .bind(rfi_id)
        .bind(project_id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("RFI not found"));
    }

    Ok((
        StatusCode::OK,
        Json(MessageResponse::new("RFI deleted successfully")),
    ))
}

/// POST /api/projects/:project_id/rfis/:rfi_id/responses
///
/// Add a response to an RFI.
pub async fn add_rfi_response(
    State(state): State<Arc<AppState>>,
    Path((project_id, rfi_id)): Path<(Uuid, Uuid)>,
    auth: RequireAuth,
    Json(req): Json<CreateRFIResponseRequest>,
) -> Result<impl IntoResponse, ApiError> {
    // Verify RFI exists
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM rfis WHERE id = $1 AND project_id = $2)",
    )
    .bind(rfi_id)
    .bind(project_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    if !exists {
        return Err(ApiError::not_found("RFI not found"));
    }

    let response = sqlx::query_as::<_, RFIResponseRow>(
        r#"
        INSERT INTO rfi_responses (rfi_id, content, author_id, created_at)
        VALUES ($1, $2, $3, NOW())
        RETURNING id, rfi_id, content, NULL as author, author_id, created_at
        "#,
    )
    .bind(rfi_id)
    .bind(&req.content)
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let dto: RFIResponseDTO = response.into();
    Ok((StatusCode::CREATED, Json(DataResponse::new(dto))))
}

/// GET /api/projects/:project_id/rfis/:rfi_id/responses
///
/// Get responses for an RFI.
pub async fn get_rfi_responses(
    State(state): State<Arc<AppState>>,
    Path((project_id, rfi_id)): Path<(Uuid, Uuid)>,
    _auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    // Verify RFI exists
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM rfis WHERE id = $1 AND project_id = $2)",
    )
    .bind(rfi_id)
    .bind(project_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    if !exists {
        return Err(ApiError::not_found("RFI not found"));
    }

    let responses = sqlx::query_as::<_, RFIResponseRow>(
        r#"
        SELECT r.id, r.rfi_id, r.content, 
               p.first_name || ' ' || p.last_name as author, r.author_id, r.created_at
        FROM rfi_responses r
        LEFT JOIN profiles p ON r.author_id = p.id
        WHERE r.rfi_id = $1
        ORDER BY r.created_at ASC
        "#,
    )
    .bind(rfi_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let data: Vec<RFIResponseDTO> = responses.into_iter().map(Into::into).collect();
    Ok(Json(DataResponse::new(data)))
}
