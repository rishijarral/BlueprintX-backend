//! Redis caching service for optimizing API performance.
//!
//! Provides a type-safe caching layer with:
//! - Automatic serialization/deserialization via serde
//! - Configurable TTL
//! - Cache invalidation patterns
//! - Connection pooling via ConnectionManager

use anyhow::{Context, Result};
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use serde::{de::DeserializeOwned, Serialize};
use std::time::Duration;
use tracing::{debug, error, instrument, warn};

/// Redis cache client with connection pooling.
#[derive(Clone)]
pub struct RedisCache {
    conn: ConnectionManager,
    default_ttl: Duration,
}

impl RedisCache {
    /// Create a new Redis cache connection.
    pub async fn new(redis_url: &str, default_ttl_seconds: u64) -> Result<Self> {
        let client = redis::Client::open(redis_url)
            .context("Failed to create Redis client")?;

        let conn = ConnectionManager::new(client)
            .await
            .context("Failed to connect to Redis")?;

        tracing::info!("Redis cache connected");

        Ok(Self {
            conn,
            default_ttl: Duration::from_secs(default_ttl_seconds),
        })
    }

    /// Get a value from cache.
    #[instrument(skip(self), fields(cache_hit))]
    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        let mut conn = self.conn.clone();

        match conn.get::<_, Option<String>>(key).await {
            Ok(Some(data)) => {
                match serde_json::from_str(&data) {
                    Ok(value) => {
                        debug!(key = key, "Cache hit");
                        tracing::Span::current().record("cache_hit", true);
                        Some(value)
                    }
                    Err(e) => {
                        warn!(key = key, error = %e, "Failed to deserialize cached value");
                        tracing::Span::current().record("cache_hit", false);
                        None
                    }
                }
            }
            Ok(None) => {
                debug!(key = key, "Cache miss");
                tracing::Span::current().record("cache_hit", false);
                None
            }
            Err(e) => {
                error!(key = key, error = %e, "Redis get error");
                tracing::Span::current().record("cache_hit", false);
                None
            }
        }
    }

    /// Set a value in cache with default TTL.
    #[instrument(skip(self, value))]
    pub async fn set<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        self.set_with_ttl(key, value, self.default_ttl).await
    }

    /// Set a value in cache with custom TTL.
    #[instrument(skip(self, value))]
    pub async fn set_with_ttl<T: Serialize>(
        &self,
        key: &str,
        value: &T,
        ttl: Duration,
    ) -> Result<()> {
        let mut conn = self.conn.clone();

        let data = serde_json::to_string(value)
            .context("Failed to serialize value for cache")?;

        conn.set_ex::<_, _, ()>(key, data, ttl.as_secs())
            .await
            .context("Failed to set cache value")?;

        debug!(key = key, ttl_secs = ttl.as_secs(), "Cached value");
        Ok(())
    }

    /// Delete a specific key from cache.
    #[allow(dead_code)]
    #[instrument(skip(self))]
    pub async fn delete(&self, key: &str) -> Result<bool> {
        let mut conn = self.conn.clone();

        let deleted: i32 = conn.del(key).await.context("Failed to delete cache key")?;

        debug!(key = key, deleted = deleted > 0, "Cache delete");
        Ok(deleted > 0)
    }

    /// Delete all keys matching a pattern (e.g., "project:123:*").
    #[instrument(skip(self))]
    pub async fn delete_pattern(&self, pattern: &str) -> Result<usize> {
        let mut conn = self.conn.clone();

        // Use SCAN to find keys matching pattern (production-safe)
        let keys: Vec<String> = redis::cmd("SCAN")
            .cursor_arg(0)
            .arg("MATCH")
            .arg(pattern)
            .arg("COUNT")
            .arg(1000)
            .query_async(&mut conn)
            .await
            .map(|(_, keys): (u64, Vec<String>)| keys)
            .unwrap_or_default();

        if keys.is_empty() {
            return Ok(0);
        }

        let deleted: i32 = conn.del(&keys).await.context("Failed to delete cache keys")?;

        debug!(pattern = pattern, deleted = deleted, "Cache pattern delete");
        Ok(deleted as usize)
    }

    /// Check if Redis is healthy.
    pub async fn health_check(&self) -> Result<()> {
        let mut conn = self.conn.clone();
        let _: String = redis::cmd("PING")
            .query_async(&mut conn)
            .await
            .context("Redis health check failed")?;
        Ok(())
    }
}

/// Cache key builders for consistent key formats.
///
/// These functions provide standardized cache key patterns for all cacheable
/// entities in the system. Some keys are scaffolded for future use.
#[allow(dead_code)]
pub mod keys {
    use uuid::Uuid;

    /// Project cache key
    pub fn project(project_id: Uuid) -> String {
        format!("project:{}", project_id)
    }

    /// Project list cache key (for a user)
    pub fn project_list(user_id: Uuid, page: u32) -> String {
        format!("projects:user:{}:page:{}", user_id, page)
    }

    /// Document cache key
    pub fn document(document_id: Uuid) -> String {
        format!("document:{}", document_id)
    }

    /// Document list cache key
    pub fn document_list(project_id: Uuid) -> String {
        format!("documents:project:{}", project_id)
    }

    /// Tender cache key
    pub fn tender(tender_id: Uuid) -> String {
        format!("tender:{}", tender_id)
    }

    /// Tender list cache key
    pub fn tender_list(project_id: Uuid) -> String {
        format!("tenders:project:{}", project_id)
    }

    /// Bid cache key
    pub fn bid(bid_id: Uuid) -> String {
        format!("bid:{}", bid_id)
    }

    /// Bid list cache key
    pub fn bid_list(tender_id: Uuid) -> String {
        format!("bids:tender:{}", tender_id)
    }

    /// Plan summary cache key
    pub fn plan_summary(project_id: Uuid) -> String {
        format!("ai:summary:project:{}", project_id)
    }

    /// Trade scopes cache key
    pub fn trade_scopes(project_id: Uuid) -> String {
        format!("ai:scopes:project:{}", project_id)
    }

    /// Q&A cache key (hash of question for uniqueness)
    pub fn qna(project_id: Uuid, question_hash: &str) -> String {
        format!("ai:qna:project:{}:{}", project_id, question_hash)
    }

    /// Pattern to invalidate all project-related caches
    pub fn project_pattern(project_id: Uuid) -> String {
        format!("*:project:{}*", project_id)
    }

    /// Pattern to invalidate all AI caches for a project
    pub fn ai_pattern(project_id: Uuid) -> String {
        format!("ai:*:project:{}*", project_id)
    }
}
