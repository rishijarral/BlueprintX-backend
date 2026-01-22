//! Notification routes
//!
//! Endpoints for in-app notifications: list, mark read, delete.

use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::api::pagination::PaginationParams;
use crate::api::response::{DataResponse, Paginated, PaginationMeta};
use crate::app::AppState;
use crate::auth::RequireAuth;
use crate::domain::notifications::*;
use crate::error::ApiError;

// ============================================================================
// Database Row Types
// ============================================================================

#[derive(Debug, sqlx::FromRow)]
struct NotificationRow {
    id: Uuid,
    user_id: Uuid,
    #[sqlx(rename = "type")]
    notification_type: String,
    title: String,
    message: Option<String>,
    data: serde_json::Value,
    is_read: bool,
    read_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

// ============================================================================
// Query Types
// ============================================================================

#[derive(Debug, Deserialize, Default)]
pub struct NotificationQueryParams {
    #[serde(flatten)]
    pub pagination: PaginationParams,
    #[serde(flatten)]
    pub filter: NotificationQuery,
}

// ============================================================================
// Notification Endpoints
// ============================================================================

/// GET /api/notifications
///
/// List notifications for the current user with pagination and filtering.
pub async fn list_notifications(
    State(state): State<Arc<AppState>>,
    Query(query): Query<NotificationQueryParams>,
    auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = auth.user_id();
    let page = query.pagination.page.unwrap_or(1).max(1);
    let per_page = query.pagination.per_page.unwrap_or(20).min(100);
    let offset = ((page - 1) * per_page) as i64;

    let unread_only = query.filter.unread_only.unwrap_or(false);

    // Count total
    let total: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM notifications
        WHERE user_id = $1
        AND ($2::bool = false OR is_read = false)
        AND ($3::text IS NULL OR type = $3)
        "#,
    )
    .bind(user_id)
    .bind(unread_only)
    .bind(&query.filter.notification_type)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    // Fetch notifications
    let rows = sqlx::query_as::<_, NotificationRow>(
        r#"
        SELECT id, user_id, type, title, message, data, is_read, read_at, created_at
        FROM notifications
        WHERE user_id = $1
        AND ($2::bool = false OR is_read = false)
        AND ($3::text IS NULL OR type = $3)
        ORDER BY created_at DESC
        LIMIT $4 OFFSET $5
        "#,
    )
    .bind(user_id)
    .bind(unread_only)
    .bind(&query.filter.notification_type)
    .bind(per_page as i64)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let data: Vec<NotificationResponse> = rows
        .into_iter()
        .map(|r| NotificationResponse {
            id: r.id,
            notification_type: r.notification_type,
            title: r.title,
            message: r.message,
            data: r.data,
            is_read: r.is_read,
            read_at: r.read_at,
            created_at: r.created_at,
        })
        .collect();

    let total_pages = ((total as f64) / (per_page as f64)).ceil() as u32;

    Ok(Json(Paginated {
        data,
        pagination: PaginationMeta {
            page,
            per_page,
            total_items: total as u64,
            total_pages,
            has_next: page < total_pages,
            has_prev: page > 1,
        },
    }))
}

/// GET /api/notifications/unread-count
///
/// Get the count of unread notifications for the current user.
pub async fn get_unread_count(
    State(state): State<Arc<AppState>>,
    auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = auth.user_id();

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM notifications WHERE user_id = $1 AND is_read = false",
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    Ok(Json(UnreadCountResponse { count }))
}

/// GET /api/notifications/:id
///
/// Get a single notification by ID.
pub async fn get_notification(
    State(state): State<Arc<AppState>>,
    Path(notification_id): Path<Uuid>,
    auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = auth.user_id();

    let row = sqlx::query_as::<_, NotificationRow>(
        r#"
        SELECT id, user_id, type, title, message, data, is_read, read_at, created_at
        FROM notifications
        WHERE id = $1 AND user_id = $2
        "#,
    )
    .bind(notification_id)
    .bind(user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
    .ok_or_else(|| ApiError::not_found("Notification not found"))?;

    let response = NotificationResponse {
        id: row.id,
        notification_type: row.notification_type,
        title: row.title,
        message: row.message,
        data: row.data,
        is_read: row.is_read,
        read_at: row.read_at,
        created_at: row.created_at,
    };

    Ok(Json(DataResponse::new(response)))
}

/// PUT /api/notifications/:id/read
///
/// Mark a single notification as read.
pub async fn mark_notification_read(
    State(state): State<Arc<AppState>>,
    Path(notification_id): Path<Uuid>,
    auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = auth.user_id();

    let result = sqlx::query(
        r#"
        UPDATE notifications 
        SET is_read = true, read_at = NOW() 
        WHERE id = $1 AND user_id = $2 AND is_read = false
        "#,
    )
    .bind(notification_id)
    .bind(user_id)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    if result.rows_affected() == 0 {
        // Check if it exists
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM notifications WHERE id = $1 AND user_id = $2)",
        )
        .bind(notification_id)
        .bind(user_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

        if !exists {
            return Err(ApiError::not_found("Notification not found"));
        }
        // Already read, that's fine
    }

    Ok(Json(serde_json::json!({ "success": true })))
}

/// PUT /api/notifications/read-all
///
/// Mark all notifications as read for the current user.
pub async fn mark_all_read(
    State(state): State<Arc<AppState>>,
    auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = auth.user_id();

    let result = sqlx::query(
        r#"
        UPDATE notifications 
        SET is_read = true, read_at = NOW() 
        WHERE user_id = $1 AND is_read = false
        "#,
    )
    .bind(user_id)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    Ok(Json(serde_json::json!({ 
        "success": true, 
        "marked_count": result.rows_affected() 
    })))
}

/// POST /api/notifications/mark-read
///
/// Mark specific notifications as read (batch operation).
pub async fn mark_batch_read(
    State(state): State<Arc<AppState>>,
    auth: RequireAuth,
    Json(input): Json<MarkReadRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = auth.user_id();

    let notification_ids = input.notification_ids.unwrap_or_default();
    if notification_ids.is_empty() {
        return Err(ApiError::bad_request("notification_ids is required"));
    }

    let result = sqlx::query(
        r#"
        UPDATE notifications 
        SET is_read = true, read_at = NOW() 
        WHERE user_id = $1 AND id = ANY($2) AND is_read = false
        "#,
    )
    .bind(user_id)
    .bind(&notification_ids)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    Ok(Json(serde_json::json!({ 
        "success": true, 
        "marked_count": result.rows_affected() 
    })))
}

/// DELETE /api/notifications/:id
///
/// Delete a single notification.
pub async fn delete_notification(
    State(state): State<Arc<AppState>>,
    Path(notification_id): Path<Uuid>,
    auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = auth.user_id();

    let result = sqlx::query("DELETE FROM notifications WHERE id = $1 AND user_id = $2")
        .bind(notification_id)
        .bind(user_id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Notification not found"));
    }

    Ok(Json(serde_json::json!({ "success": true })))
}

/// DELETE /api/notifications
///
/// Delete all read notifications for the current user.
pub async fn delete_all_read(
    State(state): State<Arc<AppState>>,
    auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = auth.user_id();

    let result = sqlx::query("DELETE FROM notifications WHERE user_id = $1 AND is_read = true")
        .bind(user_id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    Ok(Json(serde_json::json!({ 
        "success": true, 
        "deleted_count": result.rows_affected() 
    })))
}
