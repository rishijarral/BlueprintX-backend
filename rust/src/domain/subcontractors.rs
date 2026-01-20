//! Subcontractor domain types
//!
//! Marketplace/directory of subcontractors.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Recent project for subcontractor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentProject {
    pub name: String,
    pub value: i64,        // In cents
    pub completed: String, // Date string
}

/// Subcontractor entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subcontractor {
    pub id: Uuid,
    pub name: String,
    pub trade: String,
    pub rating: f64, // 0-5 scale
    pub review_count: i32,
    pub location: String,
    pub description: Option<String>,
    pub contact_email: Option<String>,
    pub contact_phone: Option<String>,
    pub projects_completed: i32,
    pub average_bid_value: Option<i64>,
    pub response_time: Option<String>,
    pub verified: bool,
    pub specialties: Vec<String>,
    pub recent_projects: Vec<RecentProject>,
    pub created_at: DateTime<Utc>,
}

/// Query params for listing subcontractors
#[derive(Debug, Clone, Deserialize, Default)]
pub struct SubcontractorQuery {
    #[serde(default)]
    pub trade: Option<String>,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub verified_only: Option<bool>,
    #[serde(default)]
    pub min_rating: Option<f64>,
}

/// Response DTO for subcontractor
#[derive(Debug, Clone, Serialize)]
pub struct SubcontractorResponse {
    pub id: Uuid,
    pub name: String,
    pub trade: String,
    pub rating: f64,
    pub review_count: i32,
    pub location: String,
    pub description: Option<String>,
    pub contact_email: Option<String>,
    pub contact_phone: Option<String>,
    pub projects_completed: i32,
    pub average_bid_value: Option<i64>,
    pub response_time: Option<String>,
    pub verified: bool,
    pub specialties: Vec<String>,
    pub recent_projects: Vec<RecentProject>,
    pub created_at: DateTime<Utc>,
}

impl From<Subcontractor> for SubcontractorResponse {
    fn from(s: Subcontractor) -> Self {
        Self {
            id: s.id,
            name: s.name,
            trade: s.trade,
            rating: s.rating,
            review_count: s.review_count,
            location: s.location,
            description: s.description,
            contact_email: s.contact_email,
            contact_phone: s.contact_phone,
            projects_completed: s.projects_completed,
            average_bid_value: s.average_bid_value,
            response_time: s.response_time,
            verified: s.verified,
            specialties: s.specialties,
            recent_projects: s.recent_projects,
            created_at: s.created_at,
        }
    }
}
