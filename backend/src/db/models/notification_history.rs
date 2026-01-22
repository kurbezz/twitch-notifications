use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Alias types kept for backward compatibility with existing repository code
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct NotificationLog {
    pub id: String,
    pub user_id: String,
    pub notification_type: String,
    pub destination_type: String,
    pub destination_id: String,
    pub content: String,
    pub status: String,
    pub error_message: Option<String>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateNotificationLog {
    pub user_id: String,
    pub notification_type: String,
    pub destination_type: String,
    pub destination_id: String,
    pub content: String,
    pub status: String,
    pub error_message: Option<String>,
}
