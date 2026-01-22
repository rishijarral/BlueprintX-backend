//! Service layer modules for external integrations.
//!
//! Contains clients for Redis caching, AI service communication, and notification services.

pub mod ai_client;
pub mod cache;
pub mod notifications;

pub use ai_client::AiClient;
pub use cache::RedisCache;
#[allow(unused_imports)]
pub use notifications::*;
