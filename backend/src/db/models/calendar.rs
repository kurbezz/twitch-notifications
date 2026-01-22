use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// ============================================================================
// Calendar Event Models (synced from Twitch schedule)
// ============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct SyncedCalendarEvent {
    pub id: String,
    pub user_id: String,
    pub twitch_segment_id: String,
    pub discord_integration_id: Option<String>,
    pub discord_event_id: Option<String>,
    pub title: String,
    pub start_time: NaiveDateTime,
    pub end_time: Option<NaiveDateTime>,
    pub category_name: Option<String>,
    pub is_recurring: bool,
    pub last_synced_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSyncedCalendarEvent {
    pub twitch_segment_id: String,
    pub discord_integration_id: Option<String>,
    pub title: String,
    pub start_time: NaiveDateTime,
    pub end_time: Option<NaiveDateTime>,
    pub category_name: Option<String>,
    pub is_recurring: bool,
}
