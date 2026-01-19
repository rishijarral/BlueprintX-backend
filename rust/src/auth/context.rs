use super::Claims;
use uuid::Uuid;

/// Authenticated user context extracted from JWT
/// This is attached to request extensions after successful auth
#[derive(Debug, Clone)]
pub struct AuthContext {
    /// User ID (from JWT sub claim)
    pub user_id: Uuid,

    /// User email if available
    pub email: Option<String>,

    /// User role if specified
    pub role: Option<String>,

    /// Token issuer
    pub issuer: String,

    /// Token audience
    pub audience: String,
}

impl AuthContext {
    pub fn from_claims(claims: &Claims) -> Result<Self, &'static str> {
        let user_id = Uuid::parse_str(&claims.sub).map_err(|_| "Invalid user ID in token")?;

        Ok(Self {
            user_id,
            email: claims.email.clone(),
            role: claims.role.clone(),
            issuer: claims.iss.clone(),
            audience: claims.aud.clone(),
        })
    }
}
