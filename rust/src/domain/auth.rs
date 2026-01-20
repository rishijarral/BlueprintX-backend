//! Authentication domain types
//!
//! These types are used for authentication requests and responses,
//! acting as a proxy to Supabase Auth.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// User type enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UserType {
    Gc,  // General Contractor
    Sub, // Subcontractor
}

impl Default for UserType {
    fn default() -> Self {
        Self::Gc
    }
}

/// Sign up request
#[derive(Debug, Clone, Deserialize)]
pub struct SignUpRequest {
    pub email: String,
    pub password: String,
    #[serde(default)]
    pub user_type: UserType,
    #[serde(default)]
    pub company_name: Option<String>,
}

/// Sign in request
#[derive(Debug, Clone, Deserialize)]
pub struct SignInRequest {
    pub email: String,
    pub password: String,
}

/// Token refresh request
#[derive(Debug, Clone, Deserialize)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

/// User info from Supabase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub email: Option<String>,
    #[serde(default)]
    pub user_type: UserType,
    pub created_at: Option<DateTime<Utc>>,
}

/// Auth response with tokens (for signin or auto-confirmed signup)
#[derive(Debug, Clone, Serialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
    pub user: User,
}

/// Signup response when email confirmation is required
#[derive(Debug, Clone, Serialize)]
pub struct SignupPendingResponse {
    pub user_id: String,
    pub email: String,
    pub confirmation_required: bool,
    pub message: String,
}

/// Session response
#[derive(Debug, Clone, Serialize)]
pub struct SessionResponse {
    pub user: User,
    pub access_token: String,
    pub expires_at: i64,
}

// Supabase Auth API response types

/// Response when signup returns tokens (email confirmation disabled or auto-confirmed)
#[derive(Debug, Clone, Deserialize)]
pub struct SupabaseAuthResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub expires_at: Option<i64>,
    pub refresh_token: String,
    pub user: SupabaseUser,
}

/// Response when signup requires email confirmation (new Supabase format 2025+)
/// Returns just the user object without tokens
#[derive(Debug, Clone, Deserialize)]
pub struct SupabaseSignupResponse {
    pub id: String,
    pub email: Option<String>,
    pub created_at: Option<String>,
    pub confirmation_sent_at: Option<String>,
    pub user_metadata: Option<serde_json::Value>,
    pub app_metadata: Option<serde_json::Value>,
    pub identities: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SupabaseUser {
    pub id: String,
    pub email: Option<String>,
    pub created_at: Option<String>,
    pub user_metadata: Option<serde_json::Value>,
    pub app_metadata: Option<serde_json::Value>,
}

/// New Supabase error format (2025+)
#[derive(Debug, Clone, Deserialize)]
pub struct SupabaseErrorResponse {
    // New format fields
    pub code: Option<i32>,
    pub error_code: Option<String>,
    pub msg: Option<String>,
    // Legacy format fields
    pub error: Option<String>,
    pub error_description: Option<String>,
    pub message: Option<String>,
}

impl SupabaseErrorResponse {
    pub fn get_message(&self) -> String {
        self.message
            .clone()
            .or_else(|| self.msg.clone())
            .or_else(|| self.error_description.clone())
            .or_else(|| self.error.clone())
            .unwrap_or_else(|| "Unknown authentication error".to_string())
    }
}

impl From<SupabaseUser> for User {
    fn from(su: SupabaseUser) -> Self {
        let user_type = su
            .user_metadata
            .as_ref()
            .and_then(|m| m.get("user_type"))
            .and_then(|v| v.as_str())
            .map(|s| match s {
                "sub" => UserType::Sub,
                _ => UserType::Gc,
            })
            .unwrap_or_default();

        Self {
            id: su.id,
            email: su.email,
            user_type,
            created_at: su.created_at.and_then(|s| s.parse().ok()),
        }
    }
}

impl From<SupabaseSignupResponse> for SignupPendingResponse {
    fn from(sr: SupabaseSignupResponse) -> Self {
        Self {
            user_id: sr.id,
            email: sr.email.unwrap_or_default(),
            confirmation_required: sr.confirmation_sent_at.is_some(),
            message: "Please check your email to confirm your account.".to_string(),
        }
    }
}
