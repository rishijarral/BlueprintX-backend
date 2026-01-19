use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Trade category for tender packages
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TradeCategory {
    GeneralConditions,
    SiteworkExcavation,
    Concrete,
    Masonry,
    Metals,
    WoodPlastics,
    ThermalMoisture,
    DoorsWindows,
    Finishes,
    Specialties,
    Equipment,
    Furnishings,
    SpecialConstruction,
    ConveyingSystems,
    Mechanical,
    Electrical,
    Plumbing,
    Hvac,
    FireProtection,
    Other,
}

impl Default for TradeCategory {
    fn default() -> Self {
        Self::Other
    }
}

/// Tender status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TenderStatus {
    Draft,
    Published,
    Closed,
    Awarded,
    Cancelled,
}

impl Default for TenderStatus {
    fn default() -> Self {
        Self::Draft
    }
}

/// Tender package entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tender {
    pub id: Uuid,
    pub project_id: Uuid,
    pub created_by: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub trade_category: TradeCategory,
    pub scope_of_work: Option<String>,
    pub status: TenderStatus,
    pub bid_due_date: Option<DateTime<Utc>>,
    pub estimated_value: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request DTO for creating a tender
#[derive(Debug, Clone, Deserialize)]
pub struct CreateTenderRequest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub trade_category: TradeCategory,
    #[serde(default)]
    pub scope_of_work: Option<String>,
    #[serde(default)]
    pub bid_due_date: Option<DateTime<Utc>>,
    #[serde(default)]
    pub estimated_value: Option<i64>,
}

/// Request DTO for updating a tender
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateTenderRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub trade_category: Option<TradeCategory>,
    #[serde(default)]
    pub scope_of_work: Option<String>,
    #[serde(default)]
    pub status: Option<TenderStatus>,
    #[serde(default)]
    pub bid_due_date: Option<DateTime<Utc>>,
    #[serde(default)]
    pub estimated_value: Option<i64>,
}

/// Response DTO for tender
#[derive(Debug, Clone, Serialize)]
pub struct TenderResponse {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub trade_category: TradeCategory,
    pub scope_of_work: Option<String>,
    pub status: TenderStatus,
    pub bid_due_date: Option<DateTime<Utc>>,
    pub estimated_value: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Tender> for TenderResponse {
    fn from(t: Tender) -> Self {
        Self {
            id: t.id,
            project_id: t.project_id,
            name: t.name,
            description: t.description,
            trade_category: t.trade_category,
            scope_of_work: t.scope_of_work,
            status: t.status,
            bid_due_date: t.bid_due_date,
            estimated_value: t.estimated_value,
            created_at: t.created_at,
            updated_at: t.updated_at,
        }
    }
}
