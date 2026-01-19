use axum::{extract::State, http::StatusCode, Json};
use serde::Serialize;
use std::sync::Arc;

use crate::app::AppState;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub services: ServiceHealth,
}

#[derive(Serialize)]
pub struct ServiceHealth {
    pub database: String,
    pub redis: String,
    pub ai_service: String,
}

/// Health check endpoint - public
pub async fn health_check(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<HealthResponse>) {
    // Check all services in parallel
    let (db_result, redis_result, ai_result) = tokio::join!(
        sqlx::query("SELECT 1").fetch_one(&state.db),
        state.cache.health_check(),
        state.ai_client.health_check(),
    );

    let db_status = if db_result.is_ok() { "ok" } else { "error" };
    let redis_status = if redis_result.is_ok() { "ok" } else { "error" };
    let ai_status = if ai_result.is_ok() { "ok" } else { "error" };

    // Determine overall status
    let status = if db_result.is_ok() && redis_result.is_ok() && ai_result.is_ok() {
        "healthy"
    } else if db_result.is_ok() {
        // DB is critical, others are degraded
        "degraded"
    } else {
        "unhealthy"
    };

    // Return 503 if unhealthy (critical service down)
    let status_code = if status == "unhealthy" {
        StatusCode::SERVICE_UNAVAILABLE
    } else {
        StatusCode::OK
    };

    (
        status_code,
        Json(HealthResponse {
            status: status.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            services: ServiceHealth {
                database: db_status.to_string(),
                redis: redis_status.to_string(),
                ai_service: ai_status.to_string(),
            },
        }),
    )
}
