//! Profile routes
//!
//! User profile management endpoints with Redis caching.

use axum::{extract::State, response::IntoResponse, Json};
use chrono::{DateTime, Utc};
use std::sync::Arc;

use crate::api::response::DataResponse;
use crate::app::AppState;
use crate::auth::RequireAuth;
use crate::domain::auth::UserType;
use crate::domain::profiles::{ProfileResponse, UpdateProfileRequest};
use crate::error::ApiError;
use crate::services::cache::{keys as cache_keys, ttl as cache_ttl};

/// Database row for profile
#[derive(Debug, sqlx::FromRow)]
struct ProfileRow {
    id: uuid::Uuid,
    email: String,
    user_type: String,
    company_name: Option<String>,
    first_name: Option<String>,
    last_name: Option<String>,
    phone: Option<String>,
    title: Option<String>,
    bio: Option<String>,
    location: Option<String>,
    updated_at: DateTime<Utc>,
}

impl From<ProfileRow> for ProfileResponse {
    fn from(row: ProfileRow) -> Self {
        Self {
            id: row.id,
            email: row.email,
            user_type: match row.user_type.as_str() {
                "sub" => UserType::Sub,
                _ => UserType::Gc,
            },
            company_name: row.company_name,
            first_name: row.first_name,
            last_name: row.last_name,
            phone: row.phone,
            title: row.title,
            bio: row.bio,
            location: row.location,
            updated_at: row.updated_at,
        }
    }
}

/// GET /api/profiles/me
///
/// Get the current user's profile. Uses Redis cache for performance.
pub async fn get_my_profile(
    State(state): State<Arc<AppState>>,
    auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    let cache_key = cache_keys::profile(auth.user_id);

    // Try cache first
    if let Some(cached) = state.cache.get::<ProfileResponse>(&cache_key).await {
        tracing::debug!(user_id = %auth.user_id, "Profile cache hit");
        return Ok(Json(DataResponse::new(cached)));
    }

    // Cache miss - fetch from database
    let profile = sqlx::query_as::<_, ProfileRow>(
        r#"
        SELECT id, email, user_type, company_name, first_name, last_name,
               phone, title, bio, location, updated_at
        FROM profiles
        WHERE id = $1
        "#,
    )
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
    .ok_or_else(|| ApiError::not_found("Profile not found"))?;

    let response: ProfileResponse = profile.into();

    // Cache the result
    let _ = state.cache.set_with_ttl(&cache_key, &response, cache_ttl::PROFILE).await;

    Ok(Json(DataResponse::new(response)))
}

/// PUT /api/profiles/me
///
/// Update the current user's profile. Invalidates cache on update.
pub async fn update_my_profile(
    State(state): State<Arc<AppState>>,
    auth: RequireAuth,
    Json(req): Json<UpdateProfileRequest>,
) -> Result<impl IntoResponse, ApiError> {
    // Build dynamic update query
    let profile = sqlx::query_as::<_, ProfileRow>(
        r#"
        UPDATE profiles SET
            first_name = COALESCE($2, first_name),
            last_name = COALESCE($3, last_name),
            phone = COALESCE($4, phone),
            company_name = COALESCE($5, company_name),
            title = COALESCE($6, title),
            bio = COALESCE($7, bio),
            location = COALESCE($8, location),
            updated_at = NOW()
        WHERE id = $1
        RETURNING id, email, user_type, company_name, first_name, last_name,
                  phone, title, bio, location, updated_at
        "#,
    )
    .bind(auth.user_id)
    .bind(&req.first_name)
    .bind(&req.last_name)
    .bind(&req.phone)
    .bind(&req.company_name)
    .bind(&req.title)
    .bind(&req.bio)
    .bind(&req.location)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
    .ok_or_else(|| ApiError::not_found("Profile not found"))?;

    let response: ProfileResponse = profile.into();

    // Invalidate cache after update
    let cache_key = cache_keys::profile(auth.user_id);
    let _ = state.cache.delete(&cache_key).await;

    // Cache the new value
    let _ = state.cache.set_with_ttl(&cache_key, &response, cache_ttl::PROFILE).await;

    Ok(Json(DataResponse::new(response)))
}
