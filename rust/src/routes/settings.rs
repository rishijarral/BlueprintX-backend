//! User settings routes
//!
//! User preferences and notification settings endpoints.

use axum::{extract::State, response::IntoResponse, Json};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use uuid::Uuid;

use crate::api::response::DataResponse;
use crate::app::AppState;
use crate::auth::RequireAuth;
use crate::domain::settings::{NotificationSettings, UpdateUserSettingsRequest, UserSettingsResponse};
use crate::error::ApiError;

/// Database row for user settings
#[derive(Debug, sqlx::FromRow)]
struct UserSettingsRow {
    user_id: Uuid,
    notification_settings: serde_json::Value,
    updated_at: DateTime<Utc>,
}

impl TryFrom<UserSettingsRow> for UserSettingsResponse {
    type Error = ApiError;

    fn try_from(row: UserSettingsRow) -> Result<Self, Self::Error> {
        let notification_settings: NotificationSettings =
            serde_json::from_value(row.notification_settings)
                .map_err(|e| ApiError::internal(format!("Failed to parse settings: {}", e)))?;

        Ok(Self {
            user_id: row.user_id,
            notification_settings,
            updated_at: row.updated_at,
        })
    }
}

/// GET /api/settings
///
/// Get user settings.
pub async fn get_settings(
    State(state): State<Arc<AppState>>,
    auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    let settings = sqlx::query_as::<_, UserSettingsRow>(
        r#"
        SELECT user_id, notification_settings, updated_at
        FROM user_settings
        WHERE user_id = $1
        "#,
    )
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    match settings {
        Some(row) => {
            let response: UserSettingsResponse = row.try_into()?;
            Ok(Json(DataResponse::new(response)))
        }
        None => {
            // Return default settings if none exist
            let default = UserSettingsResponse {
                user_id: auth.user_id,
                notification_settings: NotificationSettings::default(),
                updated_at: Utc::now(),
            };
            Ok(Json(DataResponse::new(default)))
        }
    }
}

/// PUT /api/settings
///
/// Update user settings.
pub async fn update_settings(
    State(state): State<Arc<AppState>>,
    auth: RequireAuth,
    Json(req): Json<UpdateUserSettingsRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let notification_settings = serde_json::to_value(&req.notification_settings)
        .map_err(|e| ApiError::internal(format!("Failed to serialize settings: {}", e)))?;

    let settings = sqlx::query_as::<_, UserSettingsRow>(
        r#"
        INSERT INTO user_settings (user_id, notification_settings, updated_at)
        VALUES ($1, $2, NOW())
        ON CONFLICT (user_id) DO UPDATE SET
            notification_settings = EXCLUDED.notification_settings,
            updated_at = NOW()
        RETURNING user_id, notification_settings, updated_at
        "#,
    )
    .bind(auth.user_id)
    .bind(&notification_settings)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let response: UserSettingsResponse = settings.try_into()?;
    Ok(Json(DataResponse::new(response)))
}
