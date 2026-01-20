//! Task routes
//!
//! Project task management endpoints.

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
use crate::domain::tasks::{
    CreateTaskRequest, TaskPriority, TaskResponse, TaskStatus, UpdateTaskRequest,
};
use crate::error::ApiError;

/// Database row for task
#[derive(Debug, sqlx::FromRow)]
struct TaskRow {
    id: Uuid,
    project_id: Uuid,
    title: String,
    description: Option<String>,
    status: String,
    priority: String,
    assignee: Option<String>,
    assignee_id: Option<Uuid>,
    due_date: Option<DateTime<Utc>>,
    category: Option<String>,
    progress: Option<i32>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<TaskRow> for TaskResponse {
    fn from(row: TaskRow) -> Self {
        Self {
            id: row.id,
            project_id: row.project_id,
            title: row.title,
            description: row.description,
            status: match row.status.as_str() {
                "in_progress" => TaskStatus::InProgress,
                "completed" => TaskStatus::Completed,
                _ => TaskStatus::Todo,
            },
            priority: match row.priority.as_str() {
                "low" => TaskPriority::Low,
                "high" => TaskPriority::High,
                "urgent" => TaskPriority::Urgent,
                _ => TaskPriority::Medium,
            },
            assignee: row.assignee,
            assignee_id: row.assignee_id,
            due_date: row.due_date,
            category: row.category,
            progress: row.progress,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// GET /api/projects/:project_id/tasks
///
/// List tasks for a project.
pub async fn list_tasks(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
    Query(pagination): Query<PaginationParams>,
    _auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    let page = pagination.page.unwrap_or(1).max(1);
    let per_page = pagination.per_page.unwrap_or(20).min(100);
    let offset = ((page - 1) * per_page) as i64;

    // Get total count
    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM tasks WHERE project_id = $1",
    )
    .bind(project_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    // Get tasks
    let tasks = sqlx::query_as::<_, TaskRow>(
        r#"
        SELECT t.id, t.project_id, t.title, t.description, t.status, t.priority,
               p.first_name || ' ' || p.last_name as assignee, t.assignee_id,
               t.due_date, t.category, t.progress, t.created_at, t.updated_at
        FROM tasks t
        LEFT JOIN profiles p ON t.assignee_id = p.id
        WHERE t.project_id = $1
        ORDER BY t.created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(project_id)
    .bind(per_page as i64)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let data: Vec<TaskResponse> = tasks.into_iter().map(Into::into).collect();
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

/// GET /api/tasks
///
/// List all tasks for the current user across all projects.
pub async fn list_all_tasks(
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<PaginationParams>,
    auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    let page = pagination.page.unwrap_or(1).max(1);
    let per_page = pagination.per_page.unwrap_or(20).min(100);
    let offset = ((page - 1) * per_page) as i64;

    // Get total count for projects owned by user
    let total: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM tasks t
        JOIN projects pr ON t.project_id = pr.id
        WHERE pr.owner_id = $1
        "#,
    )
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    // Get tasks
    let tasks = sqlx::query_as::<_, TaskRow>(
        r#"
        SELECT t.id, t.project_id, t.title, t.description, t.status, t.priority,
               p.first_name || ' ' || p.last_name as assignee, t.assignee_id,
               t.due_date, t.category, t.progress, t.created_at, t.updated_at
        FROM tasks t
        JOIN projects pr ON t.project_id = pr.id
        LEFT JOIN profiles p ON t.assignee_id = p.id
        WHERE pr.owner_id = $1
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

    let data: Vec<TaskResponse> = tasks.into_iter().map(Into::into).collect();
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

/// GET /api/projects/:project_id/tasks/:task_id
///
/// Get a specific task.
pub async fn get_task(
    State(state): State<Arc<AppState>>,
    Path((project_id, task_id)): Path<(Uuid, Uuid)>,
    _auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    let task = sqlx::query_as::<_, TaskRow>(
        r#"
        SELECT t.id, t.project_id, t.title, t.description, t.status, t.priority,
               p.first_name || ' ' || p.last_name as assignee, t.assignee_id,
               t.due_date, t.category, t.progress, t.created_at, t.updated_at
        FROM tasks t
        LEFT JOIN profiles p ON t.assignee_id = p.id
        WHERE t.id = $1 AND t.project_id = $2
        "#,
    )
    .bind(task_id)
    .bind(project_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
    .ok_or_else(|| ApiError::not_found("Task not found"))?;

    let response: TaskResponse = task.into();
    Ok(Json(DataResponse::new(response)))
}

/// POST /api/projects/:project_id/tasks
///
/// Create a new task.
pub async fn create_task(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
    _auth: RequireAuth,
    Json(req): Json<CreateTaskRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let status = match req.status {
        TaskStatus::InProgress => "in_progress",
        TaskStatus::Completed => "completed",
        TaskStatus::Todo => "todo",
    };

    let priority = match req.priority {
        TaskPriority::Low => "low",
        TaskPriority::High => "high",
        TaskPriority::Urgent => "urgent",
        TaskPriority::Medium => "medium",
    };

    let task = sqlx::query_as::<_, TaskRow>(
        r#"
        INSERT INTO tasks (project_id, title, description, status, priority, 
                          assignee_id, due_date, category, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW(), NOW())
        RETURNING id, project_id, title, description, status, priority,
                  NULL as assignee, assignee_id, due_date, category, progress,
                  created_at, updated_at
        "#,
    )
    .bind(project_id)
    .bind(&req.title)
    .bind(&req.description)
    .bind(status)
    .bind(priority)
    .bind(&req.assignee_id)
    .bind(&req.due_date)
    .bind(&req.category)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let response: TaskResponse = task.into();
    Ok((StatusCode::CREATED, Json(DataResponse::new(response))))
}

/// PUT /api/projects/:project_id/tasks/:task_id
///
/// Update a task.
pub async fn update_task(
    State(state): State<Arc<AppState>>,
    Path((project_id, task_id)): Path<(Uuid, Uuid)>,
    _auth: RequireAuth,
    Json(req): Json<UpdateTaskRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let status = req.status.map(|s| match s {
        TaskStatus::InProgress => "in_progress",
        TaskStatus::Completed => "completed",
        TaskStatus::Todo => "todo",
    });

    let priority = req.priority.map(|p| match p {
        TaskPriority::Low => "low",
        TaskPriority::High => "high",
        TaskPriority::Urgent => "urgent",
        TaskPriority::Medium => "medium",
    });

    let task = sqlx::query_as::<_, TaskRow>(
        r#"
        UPDATE tasks SET
            title = COALESCE($3, title),
            description = COALESCE($4, description),
            status = COALESCE($5, status),
            priority = COALESCE($6, priority),
            assignee_id = COALESCE($7, assignee_id),
            due_date = COALESCE($8, due_date),
            category = COALESCE($9, category),
            progress = COALESCE($10, progress),
            updated_at = NOW()
        WHERE id = $1 AND project_id = $2
        RETURNING id, project_id, title, description, status, priority,
                  NULL as assignee, assignee_id, due_date, category, progress,
                  created_at, updated_at
        "#,
    )
    .bind(task_id)
    .bind(project_id)
    .bind(&req.title)
    .bind(&req.description)
    .bind(status)
    .bind(priority)
    .bind(&req.assignee_id)
    .bind(&req.due_date)
    .bind(&req.category)
    .bind(&req.progress)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
    .ok_or_else(|| ApiError::not_found("Task not found"))?;

    let response: TaskResponse = task.into();
    Ok(Json(DataResponse::new(response)))
}

/// DELETE /api/projects/:project_id/tasks/:task_id
///
/// Delete a task.
pub async fn delete_task(
    State(state): State<Arc<AppState>>,
    Path((project_id, task_id)): Path<(Uuid, Uuid)>,
    _auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    let result = sqlx::query("DELETE FROM tasks WHERE id = $1 AND project_id = $2")
        .bind(task_id)
        .bind(project_id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Task not found"));
    }

    Ok((
        StatusCode::OK,
        Json(MessageResponse::new("Task deleted successfully")),
    ))
}
