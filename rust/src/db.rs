//! Database connection pool management

#![allow(dead_code)]

use anyhow::{Context, Result};
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions},
    PgPool,
};
use std::str::FromStr;
use std::time::Duration;

use crate::config::Settings;

/// Create a PostgreSQL connection pool with optimized settings
pub async fn create_pool(settings: &Settings) -> Result<PgPool> {
    let connect_options = PgConnectOptions::from_str(&settings.database_url)
        .context("Invalid DATABASE_URL")?
        .application_name("blueprintx-backend");

    let pool = PgPoolOptions::new()
        .max_connections(settings.database_max_connections)
        .min_connections(1)
        .acquire_timeout(Duration::from_secs(5))
        .idle_timeout(Duration::from_secs(300))
        .max_lifetime(Duration::from_secs(1800))
        .connect_with(connect_options)
        .await
        .context("Failed to connect to PostgreSQL")?;

    tracing::info!(
        max_connections = settings.database_max_connections,
        "Database connection pool established"
    );

    Ok(pool)
}

/// Lightweight health check for database connectivity
pub async fn health_check(pool: &PgPool) -> bool {
    sqlx::query("SELECT 1").fetch_one(pool).await.is_ok()
}
