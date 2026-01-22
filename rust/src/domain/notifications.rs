//! Notification domain types
//!
//! In-app notification system for real-time user alerts.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Notification type enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NotificationType {
    // Bid-related
    BidReceived,
    BidAwarded,
    BidRejected,
    BidShortlisted,
    BidWithdrawn,

    // Hire request related
    HireRequestReceived,
    HireRequestAccepted,
    HireRequestDeclined,
    HireRequestExpired,

    // Contract related
    ContractSent,
    ContractSigned,
    ContractFullySigned,

    // Review related
    ReviewReceived,
    ReviewResponseReceived,

    // Profile/verification
    ProfileVerified,
    ProfileRejected,
    ProfileViewed,

    // Message related
    NewMessage,

    // Tender related
    TenderPublished,
    TenderClosingSoon,
    TenderClosed,

    // System
    System,
}

impl std::fmt::Display for NotificationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = serde_json::to_string(self).unwrap_or_default();
        write!(f, "{}", s.trim_matches('"'))
    }
}

impl From<String> for NotificationType {
    fn from(s: String) -> Self {
        serde_json::from_str(&format!("\"{}\"", s)).unwrap_or(NotificationType::System)
    }
}

/// Notification entity
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Notification {
    pub id: Uuid,
    pub user_id: Uuid,
    #[sqlx(rename = "type")]
    pub notification_type: String,
    pub title: String,
    pub message: Option<String>,
    pub data: sqlx::types::Json<serde_json::Value>,
    pub is_read: bool,
    pub read_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Request to create a notification
#[derive(Debug, Clone, Deserialize)]
pub struct CreateNotificationRequest {
    pub user_id: Uuid,
    pub notification_type: NotificationType,
    pub title: String,
    pub message: Option<String>,
    pub data: Option<serde_json::Value>,
}

/// Query params for listing notifications
#[derive(Debug, Clone, Deserialize, Default)]
pub struct NotificationQuery {
    #[serde(default)]
    pub unread_only: Option<bool>,
    #[serde(default)]
    pub notification_type: Option<String>,
}

/// Response DTO for notification
#[derive(Debug, Clone, Serialize)]
pub struct NotificationResponse {
    pub id: Uuid,
    pub notification_type: String,
    pub title: String,
    pub message: Option<String>,
    pub data: serde_json::Value,
    pub is_read: bool,
    pub read_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl From<Notification> for NotificationResponse {
    fn from(n: Notification) -> Self {
        Self {
            id: n.id,
            notification_type: n.notification_type,
            title: n.title,
            message: n.message,
            data: n.data.0,
            is_read: n.is_read,
            read_at: n.read_at,
            created_at: n.created_at,
        }
    }
}

/// Unread count response
#[derive(Debug, Clone, Serialize)]
pub struct UnreadCountResponse {
    pub count: i64,
}

/// Mark notifications as read request
#[derive(Debug, Clone, Deserialize)]
pub struct MarkReadRequest {
    #[serde(default)]
    pub notification_ids: Option<Vec<Uuid>>,
}
