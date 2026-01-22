//! Database connection pool management

#![allow(dead_code)]

use anyhow::{Context, Result};
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions},
    PgPool,
};
use std::str::FromStr;
use std::time::Duration;

use crate::config::{Environment, Settings};

/// Create a PostgreSQL connection pool with optimized settings
pub async fn create_pool(settings: &Settings) -> Result<PgPool> {
    let connect_options = PgConnectOptions::from_str(&settings.database_url)
        .context("Invalid DATABASE_URL")?
        .application_name("blueprintx-backend");

    // Optimize pool settings based on environment
    let (min_connections, acquire_timeout, idle_timeout) = match settings.env {
        Environment::Dev => {
            // In dev, keep more warm connections for faster response
            // Longer acquire timeout since we might be debugging
            (2, Duration::from_secs(10), Duration::from_secs(600))
        }
        Environment::Staging | Environment::Prod => {
            // Production: conservative settings
            (1, Duration::from_secs(5), Duration::from_secs(300))
        }
    };

    let pool = PgPoolOptions::new()
        .max_connections(settings.database_max_connections)
        .min_connections(min_connections)
        .acquire_timeout(acquire_timeout)
        .idle_timeout(idle_timeout)
        .max_lifetime(Duration::from_secs(1800))
        // Test connections before use to avoid stale connection errors
        .test_before_acquire(true)
        .connect_with(connect_options)
        .await
        .context("Failed to connect to PostgreSQL")?;

    tracing::info!(
        max_connections = settings.database_max_connections,
        min_connections = min_connections,
        env = ?settings.env,
        "Database connection pool established"
    );

    Ok(pool)
}

/// Lightweight health check for database connectivity
pub async fn health_check(pool: &PgPool) -> bool {
    sqlx::query("SELECT 1").fetch_one(pool).await.is_ok()
}
