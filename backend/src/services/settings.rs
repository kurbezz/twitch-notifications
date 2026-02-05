use std::sync::Arc;

use crate::db::{
    DiscordIntegrationRepository, NotificationSettings, NotificationSettingsRepository,
    TelegramIntegrationRepository, UpdateNotificationSettings,
};
use crate::error::{AppError, AppResult};
use crate::AppState;

pub struct SettingsService;

impl SettingsService {
    /// Validate a notification message
    pub fn validate_message(message: &str, message_type: &str) -> AppResult<()> {
        if message.is_empty() {
            return Err(AppError::Validation(format!(
                "{} message cannot be empty",
                message_type
            )));
        }

        if message.len() > 4096 {
            return Err(AppError::Validation(format!(
                "{} message cannot exceed 4096 characters",
                message_type
            )));
        }

        Ok(())
    }

    /// Get aggregated notification flags from integrations
    pub async fn get_aggregated_notify_flags(
        state: &Arc<AppState>,
        user_id: &str,
    ) -> AppResult<(bool, bool, bool, bool)> {
        let telegrams = TelegramIntegrationRepository::find_by_user_id(&state.db, user_id).await?;
        let discords = DiscordIntegrationRepository::find_by_user_id(&state.db, user_id).await?;

        let notify_stream_online = telegrams.iter().any(|i| i.notify_stream_online)
            || discords.iter().any(|i| i.notify_stream_online);
        let notify_stream_offline = telegrams.iter().any(|i| i.notify_stream_offline)
            || discords.iter().any(|i| i.notify_stream_offline);
        let notify_title_change = telegrams.iter().any(|i| i.notify_title_change)
            || discords.iter().any(|i| i.notify_title_change);
        let notify_category_change = telegrams.iter().any(|i| i.notify_category_change)
            || discords.iter().any(|i| i.notify_category_change);

        Ok((
            notify_stream_online,
            notify_stream_offline,
            notify_title_change,
            notify_category_change,
        ))
    }

    /// Update notification messages
    pub async fn update_messages(
        state: &Arc<AppState>,
        user_id: &str,
        stream_online_message: Option<String>,
        stream_offline_message: Option<String>,
        stream_title_change_message: Option<String>,
        stream_category_change_message: Option<String>,
        reward_redemption_message: Option<String>,
    ) -> AppResult<crate::db::NotificationSettings> {
        // Validate messages
        if let Some(ref msg) = stream_online_message {
            Self::validate_message(msg, "stream_online")?;
        }
        if let Some(ref msg) = stream_offline_message {
            Self::validate_message(msg, "stream_offline")?;
        }
        if let Some(ref msg) = stream_title_change_message {
            Self::validate_message(msg, "stream_title_change")?;
        }
        if let Some(ref msg) = stream_category_change_message {
            Self::validate_message(msg, "stream_category")?;
        }
        if let Some(ref msg) = reward_redemption_message {
            Self::validate_message(msg, "reward_redemption")?;
        }

        let update = UpdateNotificationSettings {
            stream_online_message,
            stream_offline_message,
            stream_title_change_message,
            stream_category_change_message,
            reward_redemption_message,
            notify_reward_redemption: None,
        };

        NotificationSettingsRepository::update(&state.db, user_id, update).await
    }

    /// Reset settings to defaults
    pub async fn reset_to_defaults(
        state: &Arc<AppState>,
        user_id: &str,
    ) -> AppResult<crate::db::NotificationSettings> {
        let defaults = NotificationSettings::default();

        let update = UpdateNotificationSettings {
            stream_online_message: Some(defaults.stream_online_message.clone()),
            stream_offline_message: Some(defaults.stream_offline_message.clone()),
            stream_title_change_message: Some(defaults.stream_title_change_message.clone()),
            stream_category_change_message: Some(defaults.stream_category_change_message.clone()),
            reward_redemption_message: Some(defaults.reward_redemption_message.clone()),
            notify_reward_redemption: Some(defaults.notify_reward_redemption),
        };

        NotificationSettingsRepository::update(&state.db, user_id, update).await
    }

    /// Update notify_reward_redemption setting
    pub async fn update_notify_reward_redemption(
        state: &Arc<AppState>,
        user_id: &str,
        notify_reward_redemption: Option<bool>,
    ) -> AppResult<crate::db::NotificationSettings> {
        let update = UpdateNotificationSettings {
            stream_online_message: None,
            stream_offline_message: None,
            stream_title_change_message: None,
            stream_category_change_message: None,
            reward_redemption_message: None,
            notify_reward_redemption,
        };

        NotificationSettingsRepository::update(&state.db, user_id, update).await
    }

    /// Get notification settings
    pub async fn get_settings(
        state: &Arc<AppState>,
        user_id: &str,
    ) -> AppResult<crate::db::NotificationSettings> {
        NotificationSettingsRepository::get_or_create(&state.db, user_id).await
    }
}
