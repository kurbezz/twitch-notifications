use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DiscordIntegration {
    pub id: String,
    pub user_id: String,
    pub discord_guild_id: String,
    pub discord_channel_id: String,
    pub discord_guild_name: Option<String>,
    pub discord_channel_name: Option<String>,
    pub discord_webhook_url: Option<String>,
    pub is_enabled: bool,

    // Per-integration notification settings
    pub notify_stream_online: bool,
    pub notify_stream_offline: bool,
    pub notify_title_change: bool,
    pub notify_category_change: bool,
    pub notify_reward_redemption: bool,

    // Calendar sync to Discord events
    pub calendar_sync_enabled: bool,

    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDiscordIntegration {
    pub discord_guild_id: String,
    pub discord_channel_id: String,
    pub discord_guild_name: Option<String>,
    pub discord_channel_name: Option<String>,
    pub discord_webhook_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateDiscordIntegration {
    pub discord_channel_id: Option<String>,
    pub discord_channel_name: Option<String>,
    pub discord_webhook_url: Option<String>,
    pub is_enabled: Option<bool>,
    pub notify_stream_online: Option<bool>,
    pub notify_stream_offline: Option<bool>,
    pub notify_title_change: Option<bool>,
    pub notify_category_change: Option<bool>,
    pub notify_reward_redemption: Option<bool>,
    pub calendar_sync_enabled: Option<bool>,
}
