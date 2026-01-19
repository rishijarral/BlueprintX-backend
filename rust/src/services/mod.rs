//! Service layer modules for external integrations.
//!
//! Contains clients for Redis caching and AI service communication.

pub mod ai_client;
pub mod cache;

pub use ai_client::AiClient;
pub use cache::RedisCache;
