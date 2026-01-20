//! User settings domain types
//!
//! User preferences and notification settings.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Notification settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NotificationSettings {
    #[serde(default = "default_true")]
    pub email_notifications: bool,
    #[serde(default = "default_true")]
    pub push_notifications: bool,
    #[serde(default = "default_true")]
    pub bid_updates: bool,
    #[serde(default = "default_true")]
    pub rfi_alerts: bool,
    #[serde(default = "default_true")]
    pub task_reminders: bool,
    #[serde(default)]
    pub weekly_reports: bool,
}

fn default_true() -> bool {
    true
}

/// User settings entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSettings {
    pub user_id: Uuid,
    pub notification_settings: NotificationSettings,
    pub updated_at: DateTime<Utc>,
}

/// Request DTO for updating user settings
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateUserSettingsRequest {
    pub notification_settings: NotificationSettings,
}

/// Response DTO for user settings
#[derive(Debug, Clone, Serialize)]
pub struct UserSettingsResponse {
    pub user_id: Uuid,
    pub notification_settings: NotificationSettings,
    pub updated_at: DateTime<Utc>,
}

impl From<UserSettings> for UserSettingsResponse {
    fn from(s: UserSettings) -> Self {
        Self {
            user_id: s.user_id,
            notification_settings: s.notification_settings,
            updated_at: s.updated_at,
        }
    }
}
