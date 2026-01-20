//! Profile domain types
//!
//! User profile information stored in the profiles table.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::auth::UserType;

/// User profile entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: Uuid,
    pub email: String,
    pub user_type: UserType,
    pub company_name: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub phone: Option<String>,
    pub title: Option<String>,
    pub bio: Option<String>,
    pub location: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request DTO for updating a profile
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateProfileRequest {
    #[serde(default)]
    pub first_name: Option<String>,
    #[serde(default)]
    pub last_name: Option<String>,
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(default)]
    pub company_name: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub bio: Option<String>,
    #[serde(default)]
    pub location: Option<String>,
}

/// Response DTO for profile
#[derive(Debug, Clone, Serialize)]
pub struct ProfileResponse {
    pub id: Uuid,
    pub email: String,
    pub user_type: UserType,
    pub company_name: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub phone: Option<String>,
    pub title: Option<String>,
    pub bio: Option<String>,
    pub location: Option<String>,
    pub updated_at: DateTime<Utc>,
}

impl From<Profile> for ProfileResponse {
    fn from(p: Profile) -> Self {
        Self {
            id: p.id,
            email: p.email,
            user_type: p.user_type,
            company_name: p.company_name,
            first_name: p.first_name,
            last_name: p.last_name,
            phone: p.phone,
            title: p.title,
            bio: p.bio,
            location: p.location,
            updated_at: p.updated_at,
        }
    }
}
