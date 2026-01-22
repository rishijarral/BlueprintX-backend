mod api;
mod app;
mod auth;
mod config;
mod db;
mod domain;
mod error;
mod logging;
mod middleware;
mod routes;
mod services;

use anyhow::Result;

use services::{AiClient, RedisCache};

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables
    dotenvy::dotenv().ok();

    // Load configuration
    let settings = config::Settings::from_env()?;

    // Initialize logging
    logging::init_logging(&settings.env);

    tracing::info!(
        env = ?settings.env,
        server_addr = %settings.server_addr,
        "Starting BlueprintX backend"
    );

    // Create database pool
    let pool = db::create_pool(&settings).await?;

    // Create Redis cache
    let cache = RedisCache::new(&settings.redis_url, settings.redis_cache_ttl_seconds).await?;
    tracing::info!("Redis cache initialized");

    // Create AI service client
    let ai_client = AiClient::new(
        &settings.ai_service_url,
        &settings.ai_service_token,
        settings.ai_service_timeout_seconds,
    )?;

    // Optionally check AI service health (non-blocking)
    tokio::spawn({
        let ai_client = ai_client.clone();
        async move {
            match ai_client.health_check().await {
                Ok(()) => tracing::info!("AI service is healthy"),
                Err(e) => tracing::warn!(error = %e, "AI service health check failed - will retry on first request"),
            }
        }
    });

    // Create shared HTTP client (reused across all requests to avoid expensive allocations)
    // Optimized for local development with larger connection pool and TCP keepalive
    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        // Increase connection pool for localhost development
        .pool_max_idle_per_host(if settings.env.is_dev() { 20 } else { 10 })
        .pool_idle_timeout(std::time::Duration::from_secs(if settings.env.is_dev() { 300 } else { 90 }))
        // Enable TCP keepalive to prevent connection drops on localhost
        .tcp_keepalive(std::time::Duration::from_secs(60))
        // Enable TCP nodelay for faster small requests (reduces latency for API calls)
        .tcp_nodelay(true)
        .build()
        .expect("Failed to create HTTP client");
    tracing::info!(
        env = ?settings.env,
        pool_size = if settings.env.is_dev() { 20 } else { 10 },
        "Shared HTTP client initialized"
    );

    // Create JWKS cache for JWT verification (uses shared HTTP client)
    let jwks_cache = auth::JwksCache::new(
        settings.supabase_jwt_jwks_url.clone(),
        settings.supabase_jwt_issuer.clone(),
        settings.supabase_jwt_audience.clone(),
        settings.jwks_cache_ttl_seconds,
        http_client.clone(),
    );

    // Optionally warm the JWKS cache
    if let Err(e) = jwks_cache.warm_cache().await {
        tracing::warn!(error = %e, "Failed to warm JWKS cache - will fetch on first request");
    }

    // Create application state
    let state = app::AppState::new(pool, settings.clone(), jwks_cache, cache, ai_client, http_client);

    // Build application
    let app = app::create_app(state);

    // Start server
    let listener = tokio::net::TcpListener::bind(&settings.server_addr).await?;
    tracing::info!("Listening on {}", settings.server_addr);

    axum::serve(listener, app).await?;

    Ok(())
}
