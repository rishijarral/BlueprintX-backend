//! JWKS cache for Supabase JWT verification

use anyhow::{Context, Result};
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use parking_lot::RwLock;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::Claims;

/// JWKS response structure
#[derive(Debug, Deserialize)]
struct JwksResponse {
    keys: Vec<Jwk>,
}

/// Individual JWK key
#[derive(Debug, Clone, Deserialize)]
struct Jwk {
    kid: String,
    kty: String,
    #[serde(default)]
    #[allow(dead_code)]
    alg: Option<String>,
    n: String,
    e: String,
}

/// Cached key with expiration
#[derive(Clone)]
struct CachedKey {
    key: DecodingKey,
    cached_at: Instant,
}

/// JWKS cache for validating Supabase JWTs
#[derive(Clone)]
pub struct JwksCache {
    inner: Arc<RwLock<JwksCacheInner>>,
    jwks_url: String,
    issuer: String,
    audience: String,
    ttl: Duration,
}

struct JwksCacheInner {
    keys: HashMap<String, CachedKey>,
    last_fetch: Option<Instant>,
}

impl JwksCache {
    pub fn new(jwks_url: String, issuer: String, audience: String, ttl_seconds: u64) -> Self {
        Self {
            inner: Arc::new(RwLock::new(JwksCacheInner {
                keys: HashMap::new(),
                last_fetch: None,
            })),
            jwks_url,
            issuer,
            audience,
            ttl: Duration::from_secs(ttl_seconds),
        }
    }

    /// Verify a JWT token and return the claims
    pub async fn verify_token(&self, token: &str) -> Result<Claims> {
        // Decode header to get kid
        let header = decode_header(token).context("Invalid JWT header")?;
        let kid = header.kid.context("JWT missing kid header")?;

        // Try to get cached key
        let decoding_key = self.get_or_fetch_key(&kid).await?;

        // Set up validation
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[&self.issuer]);
        validation.set_audience(&[&self.audience]);
        validation.validate_exp = true;
        validation.validate_nbf = true;

        // Decode and validate
        let token_data =
            decode::<Claims>(token, &decoding_key, &validation).context("JWT validation failed")?;

        Ok(token_data.claims)
    }

    async fn get_or_fetch_key(&self, kid: &str) -> Result<DecodingKey> {
        // Check cache first
        {
            let cache = self.inner.read();
            if let Some(cached) = cache.keys.get(kid) {
                if cached.cached_at.elapsed() < self.ttl {
                    return Ok(cached.key.clone());
                }
            }
        }

        // Fetch fresh keys
        self.refresh_keys().await?;

        // Try again
        let cache = self.inner.read();
        cache
            .keys
            .get(kid)
            .map(|c| c.key.clone())
            .context("Key not found in JWKS")
    }

    async fn refresh_keys(&self) -> Result<()> {
        // Check if we recently fetched
        {
            let cache = self.inner.read();
            if let Some(last) = cache.last_fetch {
                // Don't refetch more than once per second
                if last.elapsed() < Duration::from_secs(1) {
                    return Ok(());
                }
            }
        }

        tracing::debug!("Fetching JWKS from {}", self.jwks_url);

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .context("Failed to create HTTP client")?;

        let response = client
            .get(&self.jwks_url)
            .send()
            .await
            .context("Failed to fetch JWKS")?;

        if !response.status().is_success() {
            anyhow::bail!("JWKS fetch failed with status: {}", response.status());
        }

        let jwks: JwksResponse = response.json().await.context("Failed to parse JWKS")?;

        let mut cache = self.inner.write();
        cache.last_fetch = Some(Instant::now());

        for jwk in jwks.keys {
            if jwk.kty != "RSA" {
                continue;
            }

            match DecodingKey::from_rsa_components(&jwk.n, &jwk.e) {
                Ok(key) => {
                    cache.keys.insert(
                        jwk.kid.clone(),
                        CachedKey {
                            key,
                            cached_at: Instant::now(),
                        },
                    );
                    tracing::debug!("Cached JWKS key: {}", jwk.kid);
                }
                Err(e) => {
                    tracing::warn!("Failed to parse JWK {}: {}", jwk.kid, e);
                }
            }
        }

        tracing::info!("JWKS cache refreshed with {} keys", cache.keys.len());
        Ok(())
    }

    /// Pre-warm the cache by fetching keys
    pub async fn warm_cache(&self) -> Result<()> {
        self.refresh_keys().await
    }
}
