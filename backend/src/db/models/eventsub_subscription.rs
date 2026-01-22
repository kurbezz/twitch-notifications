use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct EventSubSubscription {
    pub id: String,
    pub twitch_subscription_id: String,
    pub user_id: String,
    pub subscription_type: String,
    pub status: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEventSubSubscription {
    pub twitch_subscription_id: String,
    pub subscription_type: String,
    pub status: String,
}
