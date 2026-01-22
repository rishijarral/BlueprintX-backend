//! Authentication routes
//!
//! These routes proxy authentication requests to Supabase Auth.

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;

use crate::api::response::DataResponse;
use crate::app::AppState;
use crate::auth::RequireAuth;
use crate::domain::auth::{
    AuthResponse, RefreshTokenRequest, SessionResponse, SignInRequest, SignUpRequest,
    SignupPendingResponse, SupabaseAuthResponse, SupabaseErrorResponse, SupabaseSignupResponse, User,
};
use crate::error::ApiError;

/// POST /api/auth/signup
/// 
/// Register a new user with Supabase and create a profile.
pub async fn sign_up(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SignUpRequest>,
) -> Result<impl IntoResponse, ApiError> {
    // Prepare the Supabase signup request with user metadata
    let supabase_req = serde_json::json!({
        "email": req.email,
        "password": req.password,
        "data": {
            "user_type": match req.user_type {
                crate::domain::auth::UserType::Gc => "gc",
                crate::domain::auth::UserType::Sub => "sub",
            }
        }
    });

    let response = state
        .http_client
        .post(format!("{}/auth/v1/signup", state.settings.supabase_url))
        .header("apikey", &state.settings.supabase_anon_key)
        .header("Content-Type", "application/json")
        .json(&supabase_req)
        .send()
        .await
        .map_err(|e| ApiError::internal(format!("Failed to connect to auth service: {}", e)))?;

    if !response.status().is_success() {
        let error: SupabaseErrorResponse = response.json().await.unwrap_or_else(|_| {
            SupabaseErrorResponse {
                code: None,
                error_code: None,
                msg: None,
                error: Some("Unknown error".to_string()),
                error_description: None,
                message: None,
            }
        });
        return Err(ApiError::bad_request(error.get_message()));
    }

    // Get the response body as text first to handle both response formats
    let response_text = response.text().await.map_err(|e| {
        ApiError::internal(format!("Failed to read auth response: {}", e))
    })?;

    let user_type_str = match req.user_type {
        crate::domain::auth::UserType::Gc => "gc",
        crate::domain::auth::UserType::Sub => "sub",
    };

    // Try to parse as full auth response (tokens included - email confirmation disabled)
    if let Ok(auth_response) = serde_json::from_str::<SupabaseAuthResponse>(&response_text) {
        // Create profile in database
        let user_id: uuid::Uuid = auth_response.user.id.parse().map_err(|_| {
            ApiError::internal("Invalid user ID from auth service")
        })?;

        sqlx::query(
            r#"
            INSERT INTO profiles (id, email, user_type, company_name, created_at, updated_at)
            VALUES ($1, $2, $3, $4, NOW(), NOW())
            ON CONFLICT (id) DO UPDATE SET
                email = EXCLUDED.email,
                user_type = EXCLUDED.user_type,
                updated_at = NOW()
            "#
        )
        .bind(user_id)
        .bind(&req.email)
        .bind(user_type_str)
        .bind(&req.company_name)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to create profile: {}", e)))?;

        let user: User = auth_response.user.into();
        let response = AuthResponse {
            access_token: auth_response.access_token,
            refresh_token: auth_response.refresh_token,
            expires_in: auth_response.expires_in,
            user,
        };

        return Ok((StatusCode::CREATED, Json(serde_json::to_value(DataResponse::new(response)).unwrap())));
    }

    // Try to parse as signup response (email confirmation required - new Supabase format)
    if let Ok(signup_response) = serde_json::from_str::<SupabaseSignupResponse>(&response_text) {
        // Create profile in database
        let user_id: uuid::Uuid = signup_response.id.parse().map_err(|_| {
            ApiError::internal("Invalid user ID from auth service")
        })?;

        sqlx::query(
            r#"
            INSERT INTO profiles (id, email, user_type, company_name, created_at, updated_at)
            VALUES ($1, $2, $3, $4, NOW(), NOW())
            ON CONFLICT (id) DO UPDATE SET
                email = EXCLUDED.email,
                user_type = EXCLUDED.user_type,
                updated_at = NOW()
            "#
        )
        .bind(user_id)
        .bind(&req.email)
        .bind(user_type_str)
        .bind(&req.company_name)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to create profile: {}", e)))?;

        let pending_response: SignupPendingResponse = signup_response.into();
        return Ok((StatusCode::CREATED, Json(serde_json::to_value(DataResponse::new(pending_response)).unwrap())));
    }

    // Neither format matched
    Err(ApiError::internal(format!("Failed to parse auth response: unexpected format")))
}

/// POST /api/auth/signin
/// 
/// Sign in with email and password.
pub async fn sign_in(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SignInRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let response = state
        .http_client
        .post(format!(
            "{}/auth/v1/token?grant_type=password",
            state.settings.supabase_url
        ))
        .header("apikey", &state.settings.supabase_anon_key)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "email": req.email,
            "password": req.password
        }))
        .send()
        .await
        .map_err(|e| ApiError::internal(format!("Failed to connect to auth service: {}", e)))?;

    if !response.status().is_success() {
        let error: SupabaseErrorResponse = response.json().await.unwrap_or_else(|_| {
            SupabaseErrorResponse {
                code: None,
                error_code: None,
                msg: None,
                error: Some("Invalid credentials".to_string()),
                error_description: None,
                message: None,
            }
        });
        return Err(ApiError::unauthorized(error.get_message()));
    }

    let auth_response: SupabaseAuthResponse = response.json().await.map_err(|e| {
        ApiError::internal(format!("Failed to parse auth response: {}", e))
    })?;

    // Ensure profile exists (handles users created directly in Supabase)
    let user_id: uuid::Uuid = auth_response.user.id.parse().map_err(|_| {
        ApiError::internal("Invalid user ID from auth service")
    })?;

    let user_type_str = auth_response
        .user
        .user_metadata
        .as_ref()
        .and_then(|m| m.get("user_type"))
        .and_then(|v| v.as_str())
        .unwrap_or("gc");

    sqlx::query(
        r#"
        INSERT INTO profiles (id, email, user_type, created_at, updated_at)
        VALUES ($1, $2, $3, NOW(), NOW())
        ON CONFLICT (id) DO NOTHING
        "#
    )
    .bind(user_id)
    .bind(&req.email)
    .bind(user_type_str)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to ensure profile: {}", e)))?;

    let user: User = auth_response.user.into();
    let response = AuthResponse {
        access_token: auth_response.access_token,
        refresh_token: auth_response.refresh_token,
        expires_in: auth_response.expires_in,
        user,
    };

    Ok(Json(DataResponse::new(response)))
}

/// POST /api/auth/signout
/// 
/// Sign out the current user.
pub async fn sign_out(
    State(state): State<Arc<AppState>>,
    auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    // Call Supabase logout endpoint
    let _ = state
        .http_client
        .post(format!("{}/auth/v1/logout", state.settings.supabase_url))
        .header("apikey", &state.settings.supabase_anon_key)
        .header("Authorization", format!("Bearer {}", auth.token()))
        .send()
        .await;

    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/auth/session
/// 
/// Get the current session/user info.
pub async fn get_session(
    State(state): State<Arc<AppState>>,
    auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    let claims = auth.claims();

    // Fetch additional user data from Supabase
    let response = state
        .http_client
        .get(format!("{}/auth/v1/user", state.settings.supabase_url))
        .header("apikey", &state.settings.supabase_anon_key)
        .header("Authorization", format!("Bearer {}", auth.token()))
        .send()
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch user: {}", e)))?;

    if !response.status().is_success() {
        return Err(ApiError::unauthorized("Invalid session"));
    }

    let supabase_user: crate::domain::auth::SupabaseUser = response.json().await.map_err(|e| {
        ApiError::internal(format!("Failed to parse user response: {}", e))
    })?;

    let user: User = supabase_user.into();
    let session = SessionResponse {
        user,
        access_token: auth.token().to_string(),
        expires_at: claims.exp,
    };

    Ok(Json(DataResponse::new(session)))
}

/// POST /api/auth/refresh
/// 
/// Refresh the access token.
pub async fn refresh_token(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RefreshTokenRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let response = state
        .http_client
        .post(format!(
            "{}/auth/v1/token?grant_type=refresh_token",
            state.settings.supabase_url
        ))
        .header("apikey", &state.settings.supabase_anon_key)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "refresh_token": req.refresh_token
        }))
        .send()
        .await
        .map_err(|e| ApiError::internal(format!("Failed to connect to auth service: {}", e)))?;

    if !response.status().is_success() {
        let error: SupabaseErrorResponse = response.json().await.unwrap_or_else(|_| {
            SupabaseErrorResponse {
                code: None,
                error_code: None,
                msg: None,
                error: Some("Invalid refresh token".to_string()),
                error_description: None,
                message: None,
            }
        });
        return Err(ApiError::unauthorized(error.get_message()));
    }

    let auth_response: SupabaseAuthResponse = response.json().await.map_err(|e| {
        ApiError::internal(format!("Failed to parse auth response: {}", e))
    })?;

    let user: User = auth_response.user.into();
    let response = AuthResponse {
        access_token: auth_response.access_token,
        refresh_token: auth_response.refresh_token,
        expires_in: auth_response.expires_in,
        user,
    };

    Ok(Json(DataResponse::new(response)))
}
