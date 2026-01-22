//! Hiring domain types
//!
//! Types for marketplace hiring flow: hire requests, external subcontractors, contracts, messaging.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ============================================================================
// Hire Request Status
// ============================================================================

/// Hire request status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HireRequestStatus {
    Draft,
    Pending,
    Sent,
    Viewed,
    Interested,
    Negotiating,
    ContractSent,
    ContractSigned,
    Hired,
    Declined,
    Cancelled,
    Expired,
}

impl std::fmt::Display for HireRequestStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HireRequestStatus::Draft => write!(f, "draft"),
            HireRequestStatus::Pending => write!(f, "pending"),
            HireRequestStatus::Sent => write!(f, "sent"),
            HireRequestStatus::Viewed => write!(f, "viewed"),
            HireRequestStatus::Interested => write!(f, "interested"),
            HireRequestStatus::Negotiating => write!(f, "negotiating"),
            HireRequestStatus::ContractSent => write!(f, "contract_sent"),
            HireRequestStatus::ContractSigned => write!(f, "contract_signed"),
            HireRequestStatus::Hired => write!(f, "hired"),
            HireRequestStatus::Declined => write!(f, "declined"),
            HireRequestStatus::Cancelled => write!(f, "cancelled"),
            HireRequestStatus::Expired => write!(f, "expired"),
        }
    }
}

/// Rate type for hire requests
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RateType {
    Fixed,
    Hourly,
    Daily,
    Weekly,
    PerUnit,
    Negotiable,
}

impl std::fmt::Display for RateType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RateType::Fixed => write!(f, "fixed"),
            RateType::Hourly => write!(f, "hourly"),
            RateType::Daily => write!(f, "daily"),
            RateType::Weekly => write!(f, "weekly"),
            RateType::PerUnit => write!(f, "per_unit"),
            RateType::Negotiable => write!(f, "negotiable"),
        }
    }
}

// ============================================================================
// External Subcontractors
// ============================================================================

/// External subcontractor response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalSubcontractorResponse {
    pub id: Uuid,
    pub added_by: Uuid,
    pub company_name: String,
    pub contact_name: Option<String>,
    pub contact_email: Option<String>,
    pub contact_phone: Option<String>,
    pub trade: String,
    pub secondary_trades: Vec<String>,
    pub location: Option<String>,
    pub address: Option<String>,
    pub license_number: Option<String>,
    pub insurance_info: Option<String>,
    pub notes: Option<String>,
    pub rating: f64,
    pub projects_together: i32,
    pub is_preferred: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create external subcontractor request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateExternalSubcontractorInput {
    pub company_name: String,
    pub contact_name: Option<String>,
    pub contact_email: Option<String>,
    pub contact_phone: Option<String>,
    pub trade: String,
    pub secondary_trades: Option<Vec<String>>,
    pub location: Option<String>,
    pub address: Option<String>,
    pub license_number: Option<String>,
    pub insurance_info: Option<String>,
    pub notes: Option<String>,
    pub is_preferred: Option<bool>,
}

/// Update external subcontractor request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateExternalSubcontractorInput {
    pub company_name: Option<String>,
    pub contact_name: Option<String>,
    pub contact_email: Option<String>,
    pub contact_phone: Option<String>,
    pub trade: Option<String>,
    pub secondary_trades: Option<Vec<String>>,
    pub location: Option<String>,
    pub address: Option<String>,
    pub license_number: Option<String>,
    pub insurance_info: Option<String>,
    pub notes: Option<String>,
    pub rating: Option<f64>,
    pub is_preferred: Option<bool>,
}

/// External subcontractor filter query
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExternalSubcontractorQuery {
    pub trade: Option<String>,
    pub is_preferred: Option<bool>,
    pub search: Option<String>,
}

// ============================================================================
// Hire Requests
// ============================================================================

/// Subcontractor info in hire request (either platform or external)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HireRequestSubcontractor {
    pub id: Uuid,
    pub is_external: bool,
    pub company_name: String,
    pub contact_name: Option<String>,
    pub contact_email: Option<String>,
    pub contact_phone: Option<String>,
    pub trade: String,
    pub location: Option<String>,
    pub rating: Option<f64>,
    pub verified: bool,
}

/// Hire request response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HireRequestResponse {
    pub id: Uuid,
    pub project_id: Uuid,
    pub project_name: String,
    pub tender_id: Option<Uuid>,
    pub gc_id: Uuid,
    pub gc_company_name: String,
    pub subcontractor: HireRequestSubcontractor,
    pub status: String,
    pub trade: String,
    pub title: String,
    pub message: Option<String>,
    pub scope_description: Option<String>,
    pub proposed_amount: Option<f64>,
    pub rate_type: Option<String>,
    pub unit_description: Option<String>,
    pub estimated_hours: Option<i32>,
    pub estimated_start_date: Option<DateTime<Utc>>,
    pub estimated_end_date: Option<DateTime<Utc>>,
    pub response_deadline: Option<DateTime<Utc>>,
    pub sub_response: Option<String>,
    pub sub_counter_amount: Option<f64>,
    pub unread_messages: i32,
    pub contract_id: Option<Uuid>,
    pub viewed_at: Option<DateTime<Utc>>,
    pub responded_at: Option<DateTime<Utc>>,
    pub hired_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create hire request input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateHireRequestInput {
    pub project_id: Uuid,
    pub tender_id: Option<Uuid>,
    pub subcontractor_id: Option<Uuid>,
    pub external_sub_id: Option<Uuid>,
    pub trade: String,
    pub title: String,
    pub message: Option<String>,
    pub scope_description: Option<String>,
    pub proposed_amount: Option<f64>,
    pub rate_type: Option<String>,
    pub unit_description: Option<String>,
    pub estimated_hours: Option<i32>,
    pub estimated_start_date: Option<DateTime<Utc>>,
    pub estimated_end_date: Option<DateTime<Utc>>,
    pub response_deadline: Option<DateTime<Utc>>,
    pub send_immediately: Option<bool>,
}

/// Update hire request input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateHireRequestInput {
    pub title: Option<String>,
    pub message: Option<String>,
    pub scope_description: Option<String>,
    pub proposed_amount: Option<f64>,
    pub rate_type: Option<String>,
    pub unit_description: Option<String>,
    pub estimated_hours: Option<i32>,
    pub estimated_start_date: Option<DateTime<Utc>>,
    pub estimated_end_date: Option<DateTime<Utc>>,
    pub response_deadline: Option<DateTime<Utc>>,
}

/// Hire request status transition input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HireRequestStatusInput {
    pub status: String,
    pub response: Option<String>,
    pub counter_amount: Option<f64>,
}

/// Hire request filter query
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HireRequestQuery {
    pub project_id: Option<Uuid>,
    pub status: Option<String>,
    pub trade: Option<String>,
    pub as_gc: Option<bool>,
    pub as_sub: Option<bool>,
}

// ============================================================================
// Contracts
// ============================================================================

/// Contract status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContractStatus {
    Draft,
    PendingGc,
    PendingSub,
    GcSigned,
    FullySigned,
    Active,
    Completed,
    Terminated,
    Disputed,
}

impl std::fmt::Display for ContractStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContractStatus::Draft => write!(f, "draft"),
            ContractStatus::PendingGc => write!(f, "pending_gc"),
            ContractStatus::PendingSub => write!(f, "pending_sub"),
            ContractStatus::GcSigned => write!(f, "gc_signed"),
            ContractStatus::FullySigned => write!(f, "fully_signed"),
            ContractStatus::Active => write!(f, "active"),
            ContractStatus::Completed => write!(f, "completed"),
            ContractStatus::Terminated => write!(f, "terminated"),
            ContractStatus::Disputed => write!(f, "disputed"),
        }
    }
}

/// Payment milestone in contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentMilestone {
    pub name: String,
    pub amount: f64,
    pub due_upon: String,
    pub is_paid: bool,
    pub paid_at: Option<DateTime<Utc>>,
}

/// Contract section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractSection {
    pub key: String,
    pub title: String,
    pub content: String,
    pub editable: bool,
}

/// Contract template response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractTemplateResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub template_type: String,
    pub content: String,
    pub sections: Vec<ContractSection>,
    pub variables: Vec<TemplateVariable>,
    pub is_system: bool,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

/// Template variable definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateVariable {
    pub key: String,
    pub label: String,
    pub r#type: String,
    pub options: Option<Vec<String>>,
    pub required: Option<bool>,
    pub default: Option<String>,
}

/// Contract response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractResponse {
    pub id: Uuid,
    pub hire_request_id: Uuid,
    pub project_id: Uuid,
    pub project_name: String,
    pub template_id: Option<Uuid>,
    pub template_name: Option<String>,
    pub contract_number: Option<String>,
    pub title: String,
    pub content: String,
    pub sections: Vec<ContractSection>,
    pub terms_summary: Option<String>,
    pub amount: f64,
    pub payment_schedule: Vec<PaymentMilestone>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub gc_signed: bool,
    pub gc_signed_at: Option<DateTime<Utc>>,
    pub sub_signed: bool,
    pub sub_signed_at: Option<DateTime<Utc>>,
    pub status: String,
    pub pdf_path: Option<String>,
    pub notes: Option<String>,
    pub subcontractor: HireRequestSubcontractor,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create contract input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateContractInput {
    pub hire_request_id: Uuid,
    pub template_id: Option<Uuid>,
    pub title: String,
    pub content: Option<String>,
    pub sections: Option<Vec<ContractSection>>,
    pub terms_summary: Option<String>,
    pub amount: f64,
    pub payment_schedule: Option<Vec<PaymentMilestone>>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub notes: Option<String>,
    pub variables: Option<serde_json::Value>,
}

/// Update contract input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateContractInput {
    pub title: Option<String>,
    pub content: Option<String>,
    pub sections: Option<Vec<ContractSection>>,
    pub terms_summary: Option<String>,
    pub amount: Option<f64>,
    pub payment_schedule: Option<Vec<PaymentMilestone>>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub notes: Option<String>,
}

/// Sign contract input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignContractInput {
    pub signature: String,
    pub agreed_to_terms: bool,
}

/// Contract filter query
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContractQuery {
    pub project_id: Option<Uuid>,
    pub status: Option<String>,
    pub hire_request_id: Option<Uuid>,
}

// ============================================================================
// Hire Messages
// ============================================================================

/// Message type in hire request negotiation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    Text,
    File,
    CounterOffer,
    ScopeChange,
    ScheduleChange,
    System,
}

impl std::fmt::Display for MessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageType::Text => write!(f, "text"),
            MessageType::File => write!(f, "file"),
            MessageType::CounterOffer => write!(f, "counter_offer"),
            MessageType::ScopeChange => write!(f, "scope_change"),
            MessageType::ScheduleChange => write!(f, "schedule_change"),
            MessageType::System => write!(f, "system"),
        }
    }
}

/// Hire message response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HireMessageResponse {
    pub id: Uuid,
    pub hire_request_id: Uuid,
    pub sender_id: Uuid,
    pub sender_name: String,
    pub sender_type: String,
    pub message: String,
    pub message_type: String,
    pub metadata: serde_json::Value,
    pub is_read: bool,
    pub read_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Send message input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageInput {
    pub message: String,
    pub message_type: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

// ============================================================================
// Project Team
// ============================================================================

/// Team member status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TeamMemberStatus {
    Pending,
    Active,
    OnHold,
    Completed,
    Terminated,
}

impl std::fmt::Display for TeamMemberStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TeamMemberStatus::Pending => write!(f, "pending"),
            TeamMemberStatus::Active => write!(f, "active"),
            TeamMemberStatus::OnHold => write!(f, "on_hold"),
            TeamMemberStatus::Completed => write!(f, "completed"),
            TeamMemberStatus::Terminated => write!(f, "terminated"),
        }
    }
}

/// Project team member response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMemberResponse {
    pub id: Uuid,
    pub project_id: Uuid,
    pub hire_request_id: Option<Uuid>,
    pub contract_id: Option<Uuid>,
    pub subcontractor: HireRequestSubcontractor,
    pub role: Option<String>,
    pub trade: String,
    pub responsibilities: Option<String>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub hourly_rate: Option<f64>,
    pub status: String,
    pub performance_rating: Option<f64>,
    pub notes: Option<String>,
    pub joined_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Add team member input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddTeamMemberInput {
    pub subcontractor_id: Option<Uuid>,
    pub external_sub_id: Option<Uuid>,
    pub hire_request_id: Option<Uuid>,
    pub contract_id: Option<Uuid>,
    pub role: Option<String>,
    pub trade: String,
    pub responsibilities: Option<String>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub hourly_rate: Option<f64>,
    pub notes: Option<String>,
}

/// Update team member input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTeamMemberInput {
    pub role: Option<String>,
    pub responsibilities: Option<String>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub hourly_rate: Option<f64>,
    pub status: Option<String>,
    pub performance_rating: Option<f64>,
    pub notes: Option<String>,
}

// ============================================================================
// Reviews
// ============================================================================

/// Subcontractor review response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubcontractorReviewResponse {
    pub id: Uuid,
    pub subcontractor_id: Option<Uuid>,
    pub external_sub_id: Option<Uuid>,
    pub reviewer_id: Uuid,
    pub reviewer_name: String,
    pub project_id: Option<Uuid>,
    pub project_name: Option<String>,
    pub rating: f64,
    pub quality_rating: Option<f64>,
    pub communication_rating: Option<f64>,
    pub timeliness_rating: Option<f64>,
    pub value_rating: Option<f64>,
    pub title: Option<String>,
    pub comment: Option<String>,
    pub would_hire_again: Option<bool>,
    pub is_verified: bool,
    pub created_at: DateTime<Utc>,
}

/// Create review input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateReviewInput {
    pub subcontractor_id: Option<Uuid>,
    pub external_sub_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
    pub contract_id: Option<Uuid>,
    pub rating: f64,
    pub quality_rating: Option<f64>,
    pub communication_rating: Option<f64>,
    pub timeliness_rating: Option<f64>,
    pub value_rating: Option<f64>,
    pub title: Option<String>,
    pub comment: Option<String>,
    pub would_hire_again: Option<bool>,
}
