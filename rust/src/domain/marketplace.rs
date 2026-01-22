//! Marketplace domain types
//!
//! Enhanced types for the subcontractor marketplace, portfolio, and saved searches.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use super::subcontractors::RecentProject;

/// Verification status for subcontractors
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    #[default]
    Pending,
    Verified,
    Rejected,
}

impl From<String> for VerificationStatus {
    fn from(s: String) -> Self {
        match s.as_str() {
            "verified" => Self::Verified,
            "rejected" => Self::Rejected,
            _ => Self::Pending,
        }
    }
}

impl std::fmt::Display for VerificationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Verified => write!(f, "verified"),
            Self::Rejected => write!(f, "rejected"),
        }
    }
}

/// Availability status for subcontractors
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum AvailabilityStatus {
    #[default]
    Available,
    Busy,
    NotTakingWork,
}

impl From<String> for AvailabilityStatus {
    fn from(s: String) -> Self {
        match s.as_str() {
            "busy" => Self::Busy,
            "not_taking_work" => Self::NotTakingWork,
            _ => Self::Available,
        }
    }
}

impl std::fmt::Display for AvailabilityStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Available => write!(f, "available"),
            Self::Busy => write!(f, "busy"),
            Self::NotTakingWork => write!(f, "not_taking_work"),
        }
    }
}

/// Certification info
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Certification {
    pub name: String,
    pub issuer: Option<String>,
    pub issue_date: Option<String>,
    pub expiry_date: Option<String>,
    pub credential_id: Option<String>,
    pub verified: bool,
}

/// Insurance info
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InsuranceInfo {
    pub general_liability: Option<i64>, // Coverage amount in cents
    pub workers_comp: Option<i64>,
    pub auto_liability: Option<i64>,
    pub expiry_date: Option<String>,
    pub carrier: Option<String>,
    pub verified: bool,
}

/// License info
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LicenseInfo {
    pub number: Option<String>,
    pub state: Option<String>,
    pub license_type: Option<String>,
    pub expiry_date: Option<String>,
    pub verified: bool,
}

/// Enhanced subcontractor profile for marketplace
#[derive(Debug, Clone, Serialize)]
pub struct SubcontractorProfile {
    pub id: Uuid,
    pub profile_id: Option<Uuid>,
    pub name: String,
    pub trade: String,
    pub secondary_trades: Vec<String>,
    pub headline: Option<String>,
    pub company_description: Option<String>,
    pub rating: f64,
    pub review_count: i32,
    pub location: Option<String>,
    pub contact_email: Option<String>,
    pub contact_phone: Option<String>,
    pub website: Option<String>,
    pub projects_completed: i32,
    pub average_bid_value: Option<i64>,
    pub response_time: Option<String>,
    pub response_time_hours: Option<i32>,
    pub verified: bool,
    pub verification_status: String,
    pub specialties: Vec<String>,
    pub service_areas: Vec<String>,
    pub certifications: Vec<Certification>,
    pub insurance: Option<InsuranceInfo>,
    pub license_info: Option<LicenseInfo>,
    pub year_established: Option<i32>,
    pub employee_count: Option<String>,
    pub min_project_value: Option<i64>,
    pub max_project_value: Option<i64>,
    pub availability_status: String,
    pub recent_projects: Vec<RecentProject>,
    pub portfolio_count: i32,
    pub created_at: DateTime<Utc>,
}

/// Query params for marketplace search
#[derive(Debug, Clone, Deserialize, Default)]
pub struct MarketplaceSubcontractorQuery {
    #[serde(default)]
    pub search: Option<String>,
    #[serde(default)]
    pub trade: Option<String>,
    #[serde(default)]
    pub trades: Option<Vec<String>>,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub verified_only: Option<bool>,
    #[serde(default)]
    pub min_rating: Option<f64>,
    #[serde(default)]
    pub availability: Option<String>,
    #[serde(default)]
    pub min_project_value: Option<i64>,
    #[serde(default)]
    pub max_project_value: Option<i64>,
    #[serde(default)]
    pub has_insurance: Option<bool>,
    #[serde(default)]
    pub sort_by: Option<String>, // rating, reviews, response_time, newest
    #[serde(default)]
    pub sort_order: Option<String>, // asc, desc
}

/// Request to update marketplace profile (for subs)
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateMarketplaceProfileRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub headline: Option<String>,
    #[serde(default)]
    pub company_description: Option<String>,
    #[serde(default)]
    pub trade: Option<String>,
    #[serde(default)]
    pub secondary_trades: Option<Vec<String>>,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub contact_email: Option<String>,
    #[serde(default)]
    pub contact_phone: Option<String>,
    #[serde(default)]
    pub website: Option<String>,
    #[serde(default)]
    pub specialties: Option<Vec<String>>,
    #[serde(default)]
    pub service_areas: Option<Vec<String>>,
    #[serde(default)]
    pub certifications: Option<Vec<Certification>>,
    #[serde(default)]
    pub insurance: Option<InsuranceInfo>,
    #[serde(default)]
    pub license_info: Option<LicenseInfo>,
    #[serde(default)]
    pub year_established: Option<i32>,
    #[serde(default)]
    pub employee_count: Option<String>,
    #[serde(default)]
    pub min_project_value: Option<i64>,
    #[serde(default)]
    pub max_project_value: Option<i64>,
    #[serde(default)]
    pub availability_status: Option<String>,
}

/// Portfolio project entity
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PortfolioProject {
    pub id: Uuid,
    pub subcontractor_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub project_type: Option<String>,
    pub trade_category: Option<String>,
    pub location: Option<String>,
    pub completion_date: Option<NaiveDate>,
    pub project_value: Option<i64>,
    pub client_name: Option<String>,
    pub client_testimonial: Option<String>,
    pub images: sqlx::types::Json<Vec<String>>,
    pub is_featured: bool,
    pub display_order: i32,
    pub created_at: DateTime<Utc>,
}

/// Request to create/update portfolio project
#[derive(Debug, Clone, Deserialize)]
pub struct PortfolioProjectRequest {
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub project_type: Option<String>,
    #[serde(default)]
    pub trade_category: Option<String>,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub completion_date: Option<NaiveDate>,
    #[serde(default)]
    pub project_value: Option<i64>,
    #[serde(default)]
    pub client_name: Option<String>,
    #[serde(default)]
    pub client_testimonial: Option<String>,
    #[serde(default)]
    pub images: Option<Vec<String>>,
    #[serde(default)]
    pub is_featured: Option<bool>,
    #[serde(default)]
    pub display_order: Option<i32>,
}

/// Response DTO for portfolio project
#[derive(Debug, Clone, Serialize)]
pub struct PortfolioProjectResponse {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub project_type: Option<String>,
    pub trade_category: Option<String>,
    pub location: Option<String>,
    pub completion_date: Option<NaiveDate>,
    pub project_value: Option<i64>,
    pub client_name: Option<String>,
    pub client_testimonial: Option<String>,
    pub images: Vec<String>,
    pub is_featured: bool,
    pub display_order: i32,
    pub created_at: DateTime<Utc>,
}

impl From<PortfolioProject> for PortfolioProjectResponse {
    fn from(p: PortfolioProject) -> Self {
        Self {
            id: p.id,
            title: p.title,
            description: p.description,
            project_type: p.project_type,
            trade_category: p.trade_category,
            location: p.location,
            completion_date: p.completion_date,
            project_value: p.project_value,
            client_name: p.client_name,
            client_testimonial: p.client_testimonial,
            images: p.images.0,
            is_featured: p.is_featured,
            display_order: p.display_order,
            created_at: p.created_at,
        }
    }
}

/// Saved search entity
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SavedSearch {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub search_type: String,
    pub filters: sqlx::types::Json<serde_json::Value>,
    pub notify_new_matches: bool,
    pub last_run_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request to create saved search
#[derive(Debug, Clone, Deserialize)]
pub struct CreateSavedSearchRequest {
    pub name: String,
    #[serde(default = "default_search_type")]
    pub search_type: String,
    pub filters: serde_json::Value,
    #[serde(default)]
    pub notify_new_matches: bool,
}

fn default_search_type() -> String {
    "subcontractors".to_string()
}

/// Response DTO for saved search
#[derive(Debug, Clone, Serialize)]
pub struct SavedSearchResponse {
    pub id: Uuid,
    pub name: String,
    pub search_type: String,
    pub filters: serde_json::Value,
    pub notify_new_matches: bool,
    pub last_run_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl From<SavedSearch> for SavedSearchResponse {
    fn from(s: SavedSearch) -> Self {
        Self {
            id: s.id,
            name: s.name,
            search_type: s.search_type,
            filters: s.filters.0,
            notify_new_matches: s.notify_new_matches,
            last_run_at: s.last_run_at,
            created_at: s.created_at,
        }
    }
}

/// Tender visibility enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TenderVisibility {
    #[default]
    Public,
    InvitedOnly,
}

impl From<String> for TenderVisibility {
    fn from(s: String) -> Self {
        match s.as_str() {
            "invited_only" => Self::InvitedOnly,
            _ => Self::Public,
        }
    }
}

impl std::fmt::Display for TenderVisibility {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Public => write!(f, "public"),
            Self::InvitedOnly => write!(f, "invited_only"),
        }
    }
}

/// Enhanced tender for marketplace
#[derive(Debug, Clone, Serialize)]
pub struct MarketplaceTender {
    pub id: Uuid,
    pub project_id: Uuid,
    pub project_name: Option<String>,
    pub gc_company_name: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub trade_category: String,
    pub scope_of_work: Option<String>,
    pub location: Option<String>,
    pub status: String,
    pub visibility: String,
    pub bid_due_date: Option<DateTime<Utc>>,
    pub estimated_value: Option<i64>,
    pub reserve_price: Option<i64>,
    pub requirements: serde_json::Value,
    pub bids_received: i32,
    pub priority: Option<String>,
    pub created_at: DateTime<Utc>,
    // For authenticated sub users
    pub my_bid: Option<MarketplaceBidSummary>,
}

/// Summary of user's bid on a tender
#[derive(Debug, Clone, Serialize)]
pub struct MarketplaceBidSummary {
    pub id: Uuid,
    pub bid_amount: i64,
    pub status: String,
    pub submitted_at: Option<DateTime<Utc>>,
}

/// Query params for marketplace tenders
#[derive(Debug, Clone, Deserialize, Default)]
pub struct MarketplaceTenderQuery {
    #[serde(default)]
    pub search: Option<String>,
    #[serde(default)]
    pub trade: Option<String>,
    #[serde(default)]
    pub trades: Option<Vec<String>>,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub min_value: Option<i64>,
    #[serde(default)]
    pub max_value: Option<i64>,
    #[serde(default)]
    pub due_within_days: Option<i32>,
    #[serde(default)]
    pub sort_by: Option<String>, // due_date, value, newest
    #[serde(default)]
    pub sort_order: Option<String>,
}

/// Enhanced bid request for marketplace
#[derive(Debug, Clone, Deserialize)]
pub struct SubmitBidRequest {
    pub bid_amount: i64,
    #[serde(default)]
    pub breakdown: Option<Vec<BidLineItem>>,
    #[serde(default)]
    pub proposed_timeline_days: Option<i32>,
    #[serde(default)]
    pub proposed_start_date: Option<NaiveDate>,
    #[serde(default)]
    pub cover_letter: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

/// Bid line item for breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BidLineItem {
    pub description: String,
    pub quantity: Option<f64>,
    pub unit: Option<String>,
    pub unit_price: Option<i64>,
    pub total: i64,
}

/// Enhanced bid response for marketplace
#[derive(Debug, Clone, Serialize)]
pub struct MarketplaceBidResponse {
    pub id: Uuid,
    pub tender_id: Uuid,
    pub tender_name: Option<String>,
    pub project_name: Option<String>,
    pub subcontractor_id: Option<Uuid>,
    pub company_name: String,
    pub bid_amount: i64,
    pub breakdown: Vec<BidLineItem>,
    pub proposed_timeline_days: Option<i32>,
    pub proposed_start_date: Option<NaiveDate>,
    pub cover_letter: Option<String>,
    pub status: String,
    pub is_winning_bid: bool,
    pub notes: Option<String>,
    pub submitted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}
