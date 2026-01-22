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
/// entities in the system.
#[allow(dead_code)]
pub mod keys {
    use uuid::Uuid;

    // =========================================================================
    // Profile / User keys
    // =========================================================================
    
    /// User profile cache key
    pub fn profile(user_id: Uuid) -> String {
        format!("profile:{}", user_id)
    }

    /// Pattern to invalidate all user-related caches
    pub fn user_pattern(user_id: Uuid) -> String {
        format!("*:user:{}*", user_id)
    }

    // =========================================================================
    // Project keys
    // =========================================================================

    /// Project cache key
    pub fn project(project_id: Uuid) -> String {
        format!("project:{}", project_id)
    }

    /// Project list cache key (for a user with pagination)
    pub fn project_list(user_id: Uuid, page: u32, per_page: u32) -> String {
        format!("projects:user:{}:p{}:pp{}", user_id, page, per_page)
    }

    /// Project count cache key (for a user)
    pub fn project_count(user_id: Uuid) -> String {
        format!("projects:user:{}:count", user_id)
    }

    /// Pattern to invalidate all project lists for a user
    pub fn project_list_pattern(user_id: Uuid) -> String {
        format!("projects:user:{}:*", user_id)
    }

    /// Pattern to invalidate all project-related caches
    pub fn project_pattern(project_id: Uuid) -> String {
        format!("*:project:{}*", project_id)
    }

    // =========================================================================
    // Tender keys
    // =========================================================================

    /// Tender cache key
    pub fn tender(tender_id: Uuid) -> String {
        format!("tender:{}", tender_id)
    }

    /// Tender list cache key (for a project)
    pub fn tender_list(project_id: Uuid, page: u32, per_page: u32) -> String {
        format!("tenders:project:{}:p{}:pp{}", project_id, page, per_page)
    }

    /// All tenders for a user cache key
    pub fn tender_list_all(user_id: Uuid, page: u32, per_page: u32) -> String {
        format!("tenders:user:{}:p{}:pp{}", user_id, page, per_page)
    }

    /// Tender count for a project
    pub fn tender_count(project_id: Uuid) -> String {
        format!("tenders:project:{}:count", project_id)
    }

    /// Tender count for all user projects
    pub fn tender_count_all(user_id: Uuid) -> String {
        format!("tenders:user:{}:count", user_id)
    }

    /// Pattern to invalidate tender lists for a project
    pub fn tender_list_pattern(project_id: Uuid) -> String {
        format!("tenders:project:{}:*", project_id)
    }

    /// Pattern to invalidate all tenders for a user
    pub fn tender_user_pattern(user_id: Uuid) -> String {
        format!("tenders:user:{}:*", user_id)
    }

    // =========================================================================
    // Task keys
    // =========================================================================

    /// Task cache key
    pub fn task(task_id: Uuid) -> String {
        format!("task:{}", task_id)
    }

    /// Task list cache key (for a project)
    pub fn task_list(project_id: Uuid, page: u32, per_page: u32) -> String {
        format!("tasks:project:{}:p{}:pp{}", project_id, page, per_page)
    }

    /// All tasks for a user cache key
    pub fn task_list_all(user_id: Uuid, page: u32, per_page: u32) -> String {
        format!("tasks:user:{}:p{}:pp{}", user_id, page, per_page)
    }

    /// Task count for a project
    pub fn task_count(project_id: Uuid) -> String {
        format!("tasks:project:{}:count", project_id)
    }

    /// Task count for all user projects
    pub fn task_count_all(user_id: Uuid) -> String {
        format!("tasks:user:{}:count", user_id)
    }

    /// Pattern to invalidate task lists for a project
    pub fn task_list_pattern(project_id: Uuid) -> String {
        format!("tasks:project:{}:*", project_id)
    }

    /// Pattern to invalidate all tasks for a user
    pub fn task_user_pattern(user_id: Uuid) -> String {
        format!("tasks:user:{}:*", user_id)
    }

    // =========================================================================
    // Document keys
    // =========================================================================

    /// Document cache key
    pub fn document(document_id: Uuid) -> String {
        format!("document:{}", document_id)
    }

    /// Document list cache key
    pub fn document_list(project_id: Uuid) -> String {
        format!("documents:project:{}", project_id)
    }

    // =========================================================================
    // Bid keys
    // =========================================================================

    /// Bid cache key
    pub fn bid(bid_id: Uuid) -> String {
        format!("bid:{}", bid_id)
    }

    /// Bid list cache key
    pub fn bid_list(tender_id: Uuid) -> String {
        format!("bids:tender:{}", tender_id)
    }

    // =========================================================================
    // AI keys
    // =========================================================================

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

    /// Pattern to invalidate all AI caches for a project
    pub fn ai_pattern(project_id: Uuid) -> String {
        format!("ai:*:project:{}*", project_id)
    }

    // =========================================================================
    // Dashboard / Stats keys
    // =========================================================================

    /// Dashboard stats cache key for a user
    pub fn dashboard_stats(user_id: Uuid) -> String {
        format!("dashboard:user:{}", user_id)
    }

    /// Pattern to invalidate dashboard for a user
    pub fn dashboard_pattern(user_id: Uuid) -> String {
        format!("dashboard:user:{}*", user_id)
    }
}

/// Cache TTL constants in seconds
pub mod ttl {
    use std::time::Duration;

    /// Profile data - 5 minutes (changes infrequently)
    pub const PROFILE: Duration = Duration::from_secs(300);
    
    /// List data - 2 minutes (balances freshness vs performance)
    pub const LIST: Duration = Duration::from_secs(120);
    
    /// Count queries - 1 minute (used for pagination)
    pub const COUNT: Duration = Duration::from_secs(60);
    
    /// Individual entity - 5 minutes
    pub const ENTITY: Duration = Duration::from_secs(300);
    
    /// Dashboard stats - 30 seconds (needs to be relatively fresh)
    pub const DASHBOARD: Duration = Duration::from_secs(30);
    
    /// AI responses - 1 hour (expensive to compute, rarely changes)
    pub const AI: Duration = Duration::from_secs(3600);
}
