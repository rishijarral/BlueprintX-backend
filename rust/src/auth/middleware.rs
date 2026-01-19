use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use std::sync::Arc;

use super::AuthContext;
use crate::app::AppState;
use crate::error::ErrorResponse;

/// Extractor that requires authentication
/// Use this in route handlers to require a valid JWT
///
/// Example:
/// ```ignore
/// async fn protected_route(auth: RequireAuth) -> impl IntoResponse {
///     format!("Hello, user {}", auth.user_id)
/// }
/// ```
#[derive(Debug, Clone)]
pub struct RequireAuth(pub AuthContext);

impl std::ops::Deref for RequireAuth {
    type Target = AuthContext;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
pub enum AuthError {
    MissingToken,
    InvalidFormat,
    #[allow(dead_code)]
    InvalidToken(String),
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AuthError::MissingToken => (StatusCode::UNAUTHORIZED, "Missing authorization token"),
            AuthError::InvalidFormat => (StatusCode::UNAUTHORIZED, "Invalid authorization format"),
            AuthError::InvalidToken(_) => (StatusCode::UNAUTHORIZED, "Invalid or expired token"),
        };

        let body = ErrorResponse {
            code: "UNAUTHORIZED".to_string(),
            message: message.to_string(),
            request_id: None,
        };

        (status, Json(body)).into_response()
    }
}

#[async_trait]
impl FromRequestParts<Arc<AppState>> for RequireAuth {
    type Rejection = AuthError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        // Extract Authorization header
        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .ok_or(AuthError::MissingToken)?
            .to_str()
            .map_err(|_| AuthError::InvalidFormat)?;

        // Parse Bearer token
        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or(AuthError::InvalidFormat)?;

        if token.is_empty() {
            return Err(AuthError::MissingToken);
        }

        // Verify token
        let claims = state.jwks_cache.verify_token(token).await.map_err(|e| {
            tracing::warn!(error = %e, "JWT verification failed");
            AuthError::InvalidToken(e.to_string())
        })?;

        // Build auth context
        let context = AuthContext::from_claims(&claims).map_err(|e| {
            tracing::warn!(error = %e, "Failed to build auth context");
            AuthError::InvalidToken(e.to_string())
        })?;

        Ok(RequireAuth(context))
    }
}
