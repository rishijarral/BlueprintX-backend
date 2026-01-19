use serde::{Deserialize, Serialize};

/// JWT claims structure for Supabase tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: String,

    /// Audience
    pub aud: String,

    /// Issuer
    pub iss: String,

    /// Issued at (Unix timestamp)
    pub iat: i64,

    /// Expiration (Unix timestamp)
    pub exp: i64,

    /// Not before (Unix timestamp) - optional
    #[serde(default)]
    pub nbf: Option<i64>,

    /// User email - optional
    #[serde(default)]
    pub email: Option<String>,

    /// User role - optional
    #[serde(default)]
    pub role: Option<String>,

    /// App metadata from Supabase - optional
    #[serde(default)]
    pub app_metadata: Option<AppMetadata>,

    /// User metadata from Supabase - optional
    #[serde(default)]
    pub user_metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppMetadata {
    #[serde(default)]
    pub provider: Option<String>,

    #[serde(default)]
    pub providers: Option<Vec<String>>,
}
