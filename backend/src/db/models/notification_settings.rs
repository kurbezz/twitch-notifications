use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct NotificationSettings {
    pub id: String,
    pub user_id: String,
    pub stream_online_message: String,
    pub stream_offline_message: String,
    pub stream_title_change_message: String,
    pub stream_category_change_message: String,
    pub reward_redemption_message: String,
    pub notify_reward_redemption: bool,

    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateNotificationSettings {
    pub stream_online_message: Option<String>,
    pub stream_offline_message: Option<String>,
    pub stream_title_change_message: Option<String>,
    pub stream_category_change_message: Option<String>,
    pub reward_redemption_message: Option<String>,
    pub notify_reward_redemption: Option<bool>,
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            id: String::new(),
            user_id: String::new(),
            stream_online_message: crate::i18n::t("messages.stream_online_default"),
            stream_offline_message: crate::i18n::t("messages.stream_offline_default"),
            stream_title_change_message: crate::i18n::t("messages.stream_title_change_default"),
            stream_category_change_message: crate::i18n::t(
                "messages.stream_category_change_default",
            ),
            reward_redemption_message: crate::i18n::t("messages.reward_redemption_default"),
            notify_reward_redemption: false,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        }
    }
}
