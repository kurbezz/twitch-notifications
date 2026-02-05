use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use super::ChatType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramIntegration {
    pub id: String,
    pub user_id: String,
    pub telegram_chat_id: String,
    pub telegram_chat_title: Option<String>,
    pub telegram_chat_type: Option<ChatType>,
    pub is_enabled: bool,

    // Per-integration notification settings
    pub notify_stream_online: bool,
    pub notify_stream_offline: bool,
    pub notify_title_change: bool,
    pub notify_category_change: bool,
    pub notify_reward_redemption: bool,

    /// Last Telegram message id sent to this chat; used to delete the previous message when sending a new one.
    pub last_telegram_message_id: Option<i32>,

    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTelegramIntegration {
    pub telegram_chat_id: String,
    pub telegram_chat_title: Option<String>,
    pub telegram_chat_type: Option<ChatType>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateTelegramIntegration {
    pub telegram_chat_title: Option<String>,
    pub is_enabled: Option<bool>,
    pub notify_stream_online: Option<bool>,
    pub notify_stream_offline: Option<bool>,
    pub notify_title_change: Option<bool>,
    pub notify_category_change: Option<bool>,
    pub notify_reward_redemption: Option<bool>,
}
