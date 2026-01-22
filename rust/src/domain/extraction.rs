//! Extraction domain types
//!
//! Types for AI-extracted data: materials, rooms, milestones, trade scopes.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ============================================================================
// Extracted Materials
// ============================================================================

/// Extracted material from blueprints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedMaterialResponse {
    pub id: Uuid,
    pub project_id: Uuid,
    pub document_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub quantity: Option<f64>,
    pub unit: Option<String>,
    pub unit_cost: Option<f64>,
    pub total_cost: Option<f64>,
    pub location: Option<String>,
    pub room: Option<String>,
    pub specification: Option<String>,
    pub trade_category: Option<String>,
    pub csi_division: Option<String>,
    pub source_page: Option<i32>,
    pub confidence: f64,
    pub is_verified: bool,
    pub verified_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create/update material request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialInput {
    pub name: String,
    pub description: Option<String>,
    pub quantity: Option<f64>,
    pub unit: Option<String>,
    pub unit_cost: Option<f64>,
    pub location: Option<String>,
    pub room: Option<String>,
    pub specification: Option<String>,
    pub trade_category: Option<String>,
    pub csi_division: Option<String>,
    pub source_page: Option<i32>,
}

/// Material filter query
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MaterialQuery {
    pub trade_category: Option<String>,
    pub room: Option<String>,
    pub is_verified: Option<bool>,
    pub search: Option<String>,
}

// ============================================================================
// Extracted Rooms
// ============================================================================

/// Room finishes (stored as JSON)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RoomFinishes {
    pub floor: Option<String>,
    pub walls: Option<String>,
    pub ceiling: Option<String>,
    pub base: Option<String>,
    pub paint_color: Option<String>,
}

/// Extracted room/space from blueprints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedRoomResponse {
    pub id: Uuid,
    pub project_id: Uuid,
    pub document_id: Option<Uuid>,
    pub room_name: String,
    pub room_number: Option<String>,
    pub room_type: Option<String>,
    pub floor: Option<String>,
    pub area_sqft: Option<f64>,
    pub ceiling_height: Option<f64>,
    pub perimeter_ft: Option<f64>,
    pub finishes: RoomFinishes,
    pub fixtures: Vec<String>,
    pub notes: Option<String>,
    pub source_page: Option<i32>,
    pub confidence: f64,
    pub is_verified: bool,
    pub verified_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create/update room request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomInput {
    pub room_name: String,
    pub room_number: Option<String>,
    pub room_type: Option<String>,
    pub floor: Option<String>,
    pub area_sqft: Option<f64>,
    pub ceiling_height: Option<f64>,
    pub perimeter_ft: Option<f64>,
    pub finishes: Option<RoomFinishes>,
    pub fixtures: Option<Vec<String>>,
    pub notes: Option<String>,
    pub source_page: Option<i32>,
}

/// Room filter query
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RoomQuery {
    pub floor: Option<String>,
    pub room_type: Option<String>,
    pub is_verified: Option<bool>,
    pub search: Option<String>,
}

// ============================================================================
// Project Milestones
// ============================================================================

/// Milestone status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MilestoneStatus {
    Pending,
    InProgress,
    Completed,
    Delayed,
    Cancelled,
}

impl std::fmt::Display for MilestoneStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MilestoneStatus::Pending => write!(f, "pending"),
            MilestoneStatus::InProgress => write!(f, "in_progress"),
            MilestoneStatus::Completed => write!(f, "completed"),
            MilestoneStatus::Delayed => write!(f, "delayed"),
            MilestoneStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Project milestone response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MilestoneResponse {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub phase: Option<String>,
    pub phase_order: i32,
    pub estimated_duration_days: Option<i32>,
    pub estimated_start_date: Option<DateTime<Utc>>,
    pub estimated_end_date: Option<DateTime<Utc>>,
    pub actual_start_date: Option<DateTime<Utc>>,
    pub actual_end_date: Option<DateTime<Utc>>,
    pub dependencies: Vec<String>,
    pub trades_involved: Vec<String>,
    pub deliverables: Vec<String>,
    pub status: String,
    pub progress: f64,
    pub is_ai_generated: bool,
    pub is_verified: bool,
    pub verified_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create/update milestone request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MilestoneInput {
    pub name: String,
    pub description: Option<String>,
    pub phase: Option<String>,
    pub phase_order: Option<i32>,
    pub estimated_duration_days: Option<i32>,
    pub estimated_start_date: Option<DateTime<Utc>>,
    pub estimated_end_date: Option<DateTime<Utc>>,
    pub dependencies: Option<Vec<String>>,
    pub trades_involved: Option<Vec<String>>,
    pub deliverables: Option<Vec<String>>,
    pub status: Option<String>,
    pub progress: Option<f64>,
}

/// Milestone filter query
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MilestoneQuery {
    pub phase: Option<String>,
    pub status: Option<String>,
    pub is_verified: Option<bool>,
}

// ============================================================================
// Extracted Trade Scopes
// ============================================================================

/// Trade scope item (inclusion/exclusion)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeItem {
    pub item: String,
    pub details: Option<String>,
}

/// Extracted trade scope response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeScopeResponse {
    pub id: Uuid,
    pub project_id: Uuid,
    pub document_id: Option<Uuid>,
    pub trade: String,
    pub trade_display_name: Option<String>,
    pub csi_division: Option<String>,
    pub inclusions: Vec<ScopeItem>,
    pub exclusions: Vec<ScopeItem>,
    pub required_sheets: Vec<String>,
    pub spec_sections: Vec<String>,
    pub rfi_needed: Vec<String>,
    pub assumptions: Vec<String>,
    pub estimated_value: Option<f64>,
    pub confidence: f64,
    pub is_verified: bool,
    pub verified_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create/update trade scope request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeScopeInput {
    pub trade: String,
    pub trade_display_name: Option<String>,
    pub csi_division: Option<String>,
    pub inclusions: Option<Vec<ScopeItem>>,
    pub exclusions: Option<Vec<ScopeItem>>,
    pub required_sheets: Option<Vec<String>>,
    pub spec_sections: Option<Vec<String>>,
    pub rfi_needed: Option<Vec<String>>,
    pub assumptions: Option<Vec<String>>,
    pub estimated_value: Option<f64>,
}

/// Trade scope filter query
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TradeScopeQuery {
    pub trade: Option<String>,
    pub is_verified: Option<bool>,
}

// ============================================================================
// Extraction Summary
// ============================================================================

/// Summary of all extracted data for a project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionSummary {
    pub project_id: Uuid,
    pub materials_count: i64,
    pub rooms_count: i64,
    pub milestones_count: i64,
    pub trade_scopes_count: i64,
    pub verified_materials: i64,
    pub verified_rooms: i64,
    pub verified_milestones: i64,
    pub verified_trade_scopes: i64,
    pub last_extraction_at: Option<DateTime<Utc>>,
    pub processing_job_id: Option<Uuid>,
    pub processing_status: Option<String>,
}

/// Verify item request (for materials, rooms, milestones, trade scopes)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyItemRequest {
    pub is_verified: bool,
}

/// Bulk verify request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkVerifyRequest {
    pub ids: Vec<Uuid>,
    pub is_verified: bool,
}
