use axum::Json;
use serde::Serialize;
use uuid::Uuid;

use crate::auth::RequireAuth;

#[derive(Serialize)]
pub struct MeResponse {
    pub user_id: Uuid,
    pub email: Option<String>,
    pub role: Option<String>,
    pub issuer: String,
    pub audience: String,
}

/// Get current authenticated user info
pub async fn get_me(auth: RequireAuth) -> Json<MeResponse> {
    Json(MeResponse {
        user_id: auth.user_id,
        email: auth.email.clone(),
        role: auth.role.clone(),
        issuer: auth.issuer.clone(),
        audience: auth.audience.clone(),
    })
}
