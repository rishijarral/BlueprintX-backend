use axum::{http::HeaderValue, Router};
use sqlx::PgPool;
use std::sync::Arc;
use tower_http::{
    cors::{AllowHeaders, AllowMethods, CorsLayer},
    trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer},
};
use tracing::Level;

use crate::auth::JwksCache;
use crate::config::Settings;
use crate::middleware::request_id_layer;
use crate::routes;
use crate::services::{AiClient, RedisCache};

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub settings: Settings,
    pub jwks_cache: JwksCache,
    pub cache: RedisCache,
    pub ai_client: AiClient,
    /// Shared HTTP client for external API calls (Supabase, etc.)
    /// Reusing a single client avoids expensive per-request allocations
    pub http_client: reqwest::Client,
}

impl AppState {
    pub fn new(
        db: PgPool,
        settings: Settings,
        jwks_cache: JwksCache,
        cache: RedisCache,
        ai_client: AiClient,
        http_client: reqwest::Client,
    ) -> Arc<Self> {
        Arc::new(Self {
            db,
            settings,
            jwks_cache,
            cache,
            ai_client,
            http_client,
        })
    }
}

/// Build the complete application with all middleware
pub fn create_app(state: Arc<AppState>) -> Router {
    // Build CORS layer
    let cors = build_cors_layer(&state.settings);

    // Build trace layer (use DEBUG for spans to reduce overhead at INFO level)
    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(DefaultMakeSpan::new().level(Level::DEBUG))
        .on_request(DefaultOnRequest::new().level(Level::DEBUG))
        .on_response(DefaultOnResponse::new().level(Level::DEBUG));

    // Request ID layers
    let (set_request_id, propagate_request_id) = request_id_layer();

    // Build router (routes at root level, no /api prefix)
    Router::new()
        .merge(routes::api_router())
        // Middleware stack (applied bottom-up)
        .layer(propagate_request_id)
        .layer(trace_layer)
        .layer(set_request_id)
        .layer(cors)
        .with_state(state)
}

fn build_cors_layer(settings: &Settings) -> CorsLayer {
    let origins: Vec<HeaderValue> = settings
        .cors_allow_origins
        .iter()
        .filter_map(|origin| origin.parse().ok())
        .collect();

    // In dev mode, use longer preflight cache to reduce OPTIONS requests
    let max_age = if settings.env.is_dev() {
        // Cache preflight for 24 hours in development
        std::time::Duration::from_secs(86400)
    } else {
        // 1 hour in production
        std::time::Duration::from_secs(3600)
    };

    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods(AllowMethods::list([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::PATCH,
            axum::http::Method::DELETE,
            axum::http::Method::OPTIONS,
        ]))
        .allow_headers(AllowHeaders::list([
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
            axum::http::header::ACCEPT,
            axum::http::HeaderName::from_static("x-request-id"),
            // Allow cache-related headers for better performance
            axum::http::header::CACHE_CONTROL,
            axum::http::header::IF_NONE_MATCH,
            axum::http::header::IF_MODIFIED_SINCE,
        ]))
        .allow_credentials(true)
        .max_age(max_age)
}
