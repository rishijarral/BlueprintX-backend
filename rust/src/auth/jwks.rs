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

/// Individual JWK key (supports both RSA and EC)
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct Jwk {
    kid: String,
    kty: String,
    #[serde(default)]
    alg: Option<String>,
    // RSA fields
    #[serde(default)]
    n: Option<String>,
    #[serde(default)]
    e: Option<String>,
    // EC fields
    #[serde(default)]
    crv: Option<String>,
    #[serde(default)]
    x: Option<String>,
    #[serde(default)]
    y: Option<String>,
}

/// Cached key with expiration
#[derive(Clone)]
struct CachedKey {
    key: DecodingKey,
    algorithm: Algorithm,
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
    /// Shared HTTP client (avoids creating new clients on each refresh)
    http_client: reqwest::Client,
}

struct JwksCacheInner {
    keys: HashMap<String, CachedKey>,
    last_fetch: Option<Instant>,
}

impl JwksCache {
    pub fn new(
        jwks_url: String,
        issuer: String,
        audience: String,
        ttl_seconds: u64,
        http_client: reqwest::Client,
    ) -> Self {
        Self {
            inner: Arc::new(RwLock::new(JwksCacheInner {
                keys: HashMap::new(),
                last_fetch: None,
            })),
            jwks_url,
            issuer,
            audience,
            ttl: Duration::from_secs(ttl_seconds),
            http_client,
        }
    }

    /// Verify a JWT token and return the claims
    pub async fn verify_token(&self, token: &str) -> Result<Claims> {
        // Decode header to get kid
        let header = decode_header(token).context("Invalid JWT header")?;
        let kid = header.kid.context("JWT missing kid header")?;

        // Try to get cached key
        let (decoding_key, algorithm) = self.get_or_fetch_key(&kid).await?;

        // Set up validation with the correct algorithm
        let mut validation = Validation::new(algorithm);
        validation.set_issuer(&[&self.issuer]);
        validation.set_audience(&[&self.audience]);
        validation.validate_exp = true;
        validation.validate_nbf = true;

        // Decode and validate
        let token_data =
            decode::<Claims>(token, &decoding_key, &validation).context("JWT validation failed")?;

        Ok(token_data.claims)
    }

    async fn get_or_fetch_key(&self, kid: &str) -> Result<(DecodingKey, Algorithm)> {
        // Check cache first
        {
            let cache = self.inner.read();
            if let Some(cached) = cache.keys.get(kid) {
                if cached.cached_at.elapsed() < self.ttl {
                    return Ok((cached.key.clone(), cached.algorithm));
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
            .map(|c| (c.key.clone(), c.algorithm))
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

        // Use shared HTTP client instead of creating a new one
        let response = self
            .http_client
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
            let result = match jwk.kty.as_str() {
                "EC" => {
                    // EC key (ES256, ES384, ES512)
                    let x = match &jwk.x {
                        Some(x) => x,
                        None => continue,
                    };
                    let y = match &jwk.y {
                        Some(y) => y,
                        None => continue,
                    };
                    let algorithm = match jwk.alg.as_deref() {
                        Some("ES384") => Algorithm::ES384,
                        _ => Algorithm::ES256, // Default to ES256
                    };
                    DecodingKey::from_ec_components(x, y).map(|key| (key, algorithm))
                }
                "RSA" => {
                    // RSA key (RS256, RS384, RS512)
                    let n = match &jwk.n {
                        Some(n) => n,
                        None => continue,
                    };
                    let e = match &jwk.e {
                        Some(e) => e,
                        None => continue,
                    };
                    let algorithm = match jwk.alg.as_deref() {
                        Some("RS384") => Algorithm::RS384,
                        Some("RS512") => Algorithm::RS512,
                        _ => Algorithm::RS256, // Default to RS256
                    };
                    DecodingKey::from_rsa_components(n, e).map(|key| (key, algorithm))
                }
                _ => {
                    tracing::debug!("Skipping unsupported key type: {}", jwk.kty);
                    continue;
                }
            };

            match result {
                Ok((key, algorithm)) => {
                    cache.keys.insert(
                        jwk.kid.clone(),
                        CachedKey {
                            key,
                            algorithm,
                            cached_at: Instant::now(),
                        },
                    );
                    tracing::debug!("Cached JWKS key: {} ({})", jwk.kid, jwk.kty);
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
