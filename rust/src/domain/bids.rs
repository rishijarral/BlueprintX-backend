use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Bid status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BidStatus {
    Draft,
    Submitted,
    UnderReview,
    Shortlisted,
    Awarded,
    Rejected,
    Withdrawn,
}

impl Default for BidStatus {
    fn default() -> Self {
        Self::Draft
    }
}

/// Bid entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bid {
    pub id: Uuid,
    pub tender_id: Uuid,
    pub bidder_id: Uuid,
    pub company_name: String,
    pub contact_name: Option<String>,
    pub contact_email: Option<String>,
    pub contact_phone: Option<String>,
    pub bid_amount: i64, // cents
    pub status: BidStatus,
    pub notes: Option<String>,
    pub submitted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request DTO for creating a bid
#[derive(Debug, Clone, Deserialize)]
pub struct CreateBidRequest {
    pub company_name: String,
    #[serde(default)]
    pub contact_name: Option<String>,
    #[serde(default)]
    pub contact_email: Option<String>,
    #[serde(default)]
    pub contact_phone: Option<String>,
    pub bid_amount: i64,
    #[serde(default)]
    pub notes: Option<String>,
}

/// Request DTO for updating a bid
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateBidRequest {
    #[serde(default)]
    pub company_name: Option<String>,
    #[serde(default)]
    pub contact_name: Option<String>,
    #[serde(default)]
    pub contact_email: Option<String>,
    #[serde(default)]
    pub contact_phone: Option<String>,
    #[serde(default)]
    pub bid_amount: Option<i64>,
    #[serde(default)]
    pub status: Option<BidStatus>,
    #[serde(default)]
    pub notes: Option<String>,
}

/// Response DTO for bid
#[derive(Debug, Clone, Serialize)]
pub struct BidResponse {
    pub id: Uuid,
    pub tender_id: Uuid,
    pub company_name: String,
    pub contact_name: Option<String>,
    pub contact_email: Option<String>,
    pub contact_phone: Option<String>,
    pub bid_amount: i64,
    pub status: BidStatus,
    pub notes: Option<String>,
    pub submitted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Bid> for BidResponse {
    fn from(b: Bid) -> Self {
        Self {
            id: b.id,
            tender_id: b.tender_id,
            company_name: b.company_name,
            contact_name: b.contact_name,
            contact_email: b.contact_email,
            contact_phone: b.contact_phone,
            bid_amount: b.bid_amount,
            status: b.status,
            notes: b.notes,
            submitted_at: b.submitted_at,
            created_at: b.created_at,
            updated_at: b.updated_at,
        }
    }
}

/// Bid summary for leveling
#[derive(Debug, Clone, Serialize)]
pub struct BidSummary {
    pub id: Uuid,
    pub company_name: String,
    pub bid_amount: i64,
    pub status: BidStatus,
    pub submitted_at: Option<DateTime<Utc>>,
}

impl From<Bid> for BidSummary {
    fn from(b: Bid) -> Self {
        Self {
            id: b.id,
            company_name: b.company_name,
            bid_amount: b.bid_amount,
            status: b.status,
            submitted_at: b.submitted_at,
        }
    }
}
