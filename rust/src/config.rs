use anyhow::{Context, Result};
use std::env;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Environment {
    Dev,
    Staging,
    Prod,
}

impl Environment {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "prod" | "production" => Self::Prod,
            "staging" => Self::Staging,
            _ => Self::Dev,
        }
    }

    #[allow(dead_code)]
    pub fn is_dev(&self) -> bool {
        matches!(self, Self::Dev)
    }

    #[allow(dead_code)]
    pub fn is_prod(&self) -> bool {
        matches!(self, Self::Prod)
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Settings {
    pub env: Environment,
    pub server_addr: String,

    // Database
    pub database_url: String,
    pub database_max_connections: u32,

    // Redis
    pub redis_url: String,
    pub redis_cache_ttl_seconds: u64,

    // CORS
    pub cors_allow_origins: Vec<String>,

    // Supabase Auth
    pub supabase_jwt_jwks_url: String,
    pub supabase_jwt_issuer: String,
    pub supabase_jwt_audience: String,
    pub jwks_cache_ttl_seconds: u64,

    // AI Service
    pub ai_service_url: String,
    pub ai_service_token: String,
    pub ai_service_timeout_seconds: u64,

    // Supabase API (for auth proxy)
    pub supabase_url: String,
    pub supabase_anon_key: String,
    pub supabase_service_role_key: String,
}

impl Settings {
    pub fn from_env() -> Result<Self> {
        let env = Environment::from_str(&env::var("ENV").unwrap_or_else(|_| "dev".to_string()));
        let server_addr = env::var("SERVER_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());

        // Database
        let database_url = env::var("DATABASE_URL").context("DATABASE_URL must be set")?;
        let database_max_connections = env::var("DATABASE_MAX_CONNECTIONS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(10);

        // Redis
        let redis_url =
            env::var("REDIS_URL").unwrap_or_else(|_| "redis://redis:6379/0".to_string());
        let redis_cache_ttl_seconds = env::var("REDIS_CACHE_TTL_SECONDS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(3600); // 1 hour default

        // CORS
        let cors_allow_origins = env::var("CORS_ALLOW_ORIGINS")
            .unwrap_or_else(|_| "http://localhost:3000".to_string())
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        // Supabase Auth
        let supabase_jwt_jwks_url =
            env::var("SUPABASE_JWT_JWKS_URL").context("SUPABASE_JWT_JWKS_URL must be set")?;
        let supabase_jwt_issuer =
            env::var("SUPABASE_JWT_ISSUER").context("SUPABASE_JWT_ISSUER must be set")?;
        let supabase_jwt_audience =
            env::var("SUPABASE_JWT_AUDIENCE").unwrap_or_else(|_| "authenticated".to_string());
        let jwks_cache_ttl_seconds = env::var("JWKS_CACHE_TTL_SECONDS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1800); // 30 minutes default

        // AI Service
        let ai_service_url =
            env::var("AI_SERVICE_URL").unwrap_or_else(|_| "http://ai-service:8000".to_string());
        let ai_service_token =
            env::var("AI_SERVICE_TOKEN").context("AI_SERVICE_TOKEN must be set")?;
        let ai_service_timeout_seconds = env::var("AI_SERVICE_TIMEOUT_SECONDS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(120); // 2 minutes default for LLM calls

        // Supabase API (for auth proxy)
        let supabase_url = env::var("SUPABASE_URL").context("SUPABASE_URL must be set")?;
        let supabase_anon_key =
            env::var("SUPABASE_ANON_KEY").context("SUPABASE_ANON_KEY must be set")?;
        let supabase_service_role_key = env::var("SUPABASE_SERVICE_ROLE_KEY")
            .context("SUPABASE_SERVICE_ROLE_KEY must be set")?;

        Ok(Settings {
            env,
            server_addr,
            database_url,
            database_max_connections,
            redis_url,
            redis_cache_ttl_seconds,
            cors_allow_origins,
            supabase_jwt_jwks_url,
            supabase_jwt_issuer,
            supabase_jwt_audience,
            jwks_cache_ttl_seconds,
            ai_service_url,
            ai_service_token,
            ai_service_timeout_seconds,
            supabase_url,
            supabase_anon_key,
            supabase_service_role_key,
        })
    }
}
