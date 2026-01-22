//! Admin domain types
//!
//! Types for admin panel operations including verification and audit logging.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Admin action types for audit logging
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AdminAction {
    VerifySubcontractor,
    RejectSubcontractor,
    GrantAdmin,
    RevokeAdmin,
    SuspendUser,
    UnsuspendUser,
    DeleteContent,
    UpdateSystemSetting,
    ViewSensitiveData,
}

impl std::fmt::Display for AdminAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = serde_json::to_string(self).unwrap_or_default();
        write!(f, "{}", s.trim_matches('"'))
    }
}

/// Target types for audit logging
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditTargetType {
    Subcontractor,
    Profile,
    Tender,
    Bid,
    Contract,
    Review,
    Project,
    SystemSetting,
}

impl std::fmt::Display for AuditTargetType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = serde_json::to_string(self).unwrap_or_default();
        write!(f, "{}", s.trim_matches('"'))
    }
}

/// Admin audit log entry
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AdminAuditLog {
    pub id: Uuid,
    pub admin_id: Uuid,
    pub action: String,
    pub target_type: String,
    pub target_id: Option<Uuid>,
    pub details: sqlx::types::Json<serde_json::Value>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Response DTO for audit log
#[derive(Debug, Clone, Serialize)]
pub struct AdminAuditLogResponse {
    pub id: Uuid,
    pub admin_id: Uuid,
    pub admin_name: Option<String>,
    pub action: String,
    pub target_type: String,
    pub target_id: Option<Uuid>,
    pub details: serde_json::Value,
    pub ip_address: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Pending verification item
#[derive(Debug, Clone, Serialize)]
pub struct PendingVerification {
    pub id: Uuid,
    pub profile_id: Option<Uuid>,
    pub name: String,
    pub trade: String,
    pub location: Option<String>,
    pub contact_email: Option<String>,
    pub headline: Option<String>,
    pub company_description: Option<String>,
    pub year_established: Option<i32>,
    pub employee_count: Option<String>,
    pub certifications: Vec<serde_json::Value>,
    pub insurance: Option<serde_json::Value>,
    pub license_info: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub profile_email: Option<String>,
    pub profile_name: Option<String>,
}

/// Request to approve verification
#[derive(Debug, Clone, Deserialize)]
pub struct ApproveVerificationRequest {
    #[serde(default)]
    pub notes: Option<String>,
}

/// Request to reject verification
#[derive(Debug, Clone, Deserialize)]
pub struct RejectVerificationRequest {
    pub reason: String,
    #[serde(default)]
    pub notes: Option<String>,
}

/// Admin dashboard stats
#[derive(Debug, Clone, Serialize)]
pub struct AdminDashboardStats {
    pub pending_verifications: i64,
    pub total_subcontractors: i64,
    pub verified_subcontractors: i64,
    pub total_tenders: i64,
    pub open_tenders: i64,
    pub total_bids: i64,
    pub total_contracts: i64,
    pub active_contracts: i64,
    pub total_users: i64,
    pub gc_users: i64,
    pub sub_users: i64,
    pub recent_signups_7d: i64,
    pub recent_verifications_7d: i64,
}

/// Query params for audit log
#[derive(Debug, Clone, Deserialize, Default)]
pub struct AuditLogQuery {
    #[serde(default)]
    pub admin_id: Option<Uuid>,
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub target_type: Option<String>,
    #[serde(default)]
    pub target_id: Option<Uuid>,
    #[serde(default)]
    pub from_date: Option<DateTime<Utc>>,
    #[serde(default)]
    pub to_date: Option<DateTime<Utc>>,
}

/// Query params for pending verifications
#[derive(Debug, Clone, Deserialize, Default)]
pub struct VerificationQuery {
    #[serde(default)]
    pub trade: Option<String>,
    #[serde(default)]
    pub sort_by: Option<String>, // created_at, name
    #[serde(default)]
    pub sort_order: Option<String>,
}
