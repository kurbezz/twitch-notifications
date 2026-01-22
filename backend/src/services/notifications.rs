use std::sync::Arc;

use crate::db::{
    CreateNotificationLog, DiscordIntegration, DiscordIntegrationRepository,
    NotificationLogRepository, NotificationSettings, NotificationSettingsRepository,
    TelegramIntegration, TelegramIntegrationRepository, UserRepository,
};
use crate::error::AppResult;
use crate::services::discord::DiscordService;
use crate::services::telegram::TelegramService;
use crate::AppState;

use async_trait::async_trait;
use sqlx::SqlitePool;
use tokio::sync::RwLock;

/// Types of notifications that can be sent
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationType {
    StreamOnline,
    StreamOffline,
    TitleChange,
    CategoryChange,
    RewardRedemption,
}

impl NotificationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            NotificationType::StreamOnline => "stream_online",
            NotificationType::StreamOffline => "stream_offline",
            NotificationType::TitleChange => "title_change",
            NotificationType::CategoryChange => "category_change",
            NotificationType::RewardRedemption => "reward_redemption",
        }
    }
}

/// Data for stream online notifications
#[derive(Debug, Clone)]
pub struct StreamOnlineData {
    pub streamer_name: String,
    pub streamer_avatar: Option<String>,
    pub title: String,
    pub category: String,
    pub thumbnail_url: Option<String>,
}

/// Data for stream offline notifications
#[derive(Debug, Clone)]
pub struct StreamOfflineData {
    pub streamer_name: String,
}

/// Data for title change notifications
#[derive(Debug, Clone)]
pub struct TitleChangeData {
    pub streamer_name: String,
    pub new_title: String,
}

/// Data for category change notifications
#[derive(Debug, Clone)]
pub struct CategoryChangeData {
    pub streamer_name: String,
    pub new_category: String,
}

/// Data for reward redemption notifications
#[derive(Debug, Clone)]
pub struct RewardRedemptionData {
    pub redeemer_name: String,
    pub reward_name: String,
    pub reward_cost: i32,
    pub user_input: Option<String>,
    pub broadcaster_name: String,
}

/// Unified notification content (borrows the specific data)
#[derive(Debug, Clone, Copy)]
pub enum NotificationContent<'a> {
    StreamOnline(&'a StreamOnlineData),
    StreamOffline(&'a StreamOfflineData),
    TitleChange(&'a TitleChangeData),
    CategoryChange(&'a CategoryChangeData),
    RewardRedemption(&'a RewardRedemptionData),
}

/// Abstraction that carries per-integration destination information
#[derive(Debug, Clone)]
pub struct IntegrationContext {
    pub destination_id: String,
    pub webhook_url: Option<String>,
}

/// Normalize placeholders in a message template.
/// Converts occurrences like `{{streamer}}` into `{streamer}`.
fn normalize_placeholders(msg: &str) -> String {
    let mut result = String::with_capacity(msg.len());
    let mut start = 0usize;

    while let Some(open_rel) = msg[start..].find("{{") {
        let open = start + open_rel;
        if let Some(close_rel) = msg[open + 2..].find("}}") {
            let close = open + 2 + close_rel;
            // append text before the opening braces
            result.push_str(&msg[start..open]);
            // take inner content and wrap it with a single pair of braces
            let inner = &msg[open + 2..close];
            result.push('{');
            result.push_str(inner);
            result.push('}');
            start = close + 2;
        } else {
            // no closing braces found; append rest and return
            result.push_str(&msg[start..]);
            return result;
        }
    }

    // append remaining text
    result.push_str(&msg[start..]);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_placeholders_basic() {
        assert_eq!(
            normalize_placeholders("Hello {{streamer}}!"),
            "Hello {streamer}!"
        );
        assert_eq!(
            normalize_placeholders("{{title}} — {{game}}"),
            "{title} — {game}"
        );
        assert_eq!(
            normalize_placeholders("No placeholders here"),
            "No placeholders here"
        );
    }

    #[test]
    fn normalize_placeholders_edgecases() {
        // Triple braces collapse by one level: "Weird {{{streamer}}}" -> "Weird {{streamer}}"
        assert_eq!(
            normalize_placeholders("Weird {{{streamer}}}"),
            "Weird {{streamer}}"
        );
        // Unmatched braces are preserved
        assert_eq!(
            normalize_placeholders("Broken {{streamer"),
            "Broken {{streamer"
        );
    }

    #[test]
    fn render_message_with_double_brace_template() {
        let template = normalize_placeholders("🔴 {{streamer}} is live — {{title}}");
        let rendered = template
            .replace("{streamer}", "xQc")
            .replace("{title}", "Just Chatting")
            .replace("{category}", "Just Chatting")
            .replace("{game}", "Just Chatting")
            .replace("{url}", "");
        assert_eq!(rendered, "🔴 xQc is live — Just Chatting");
    }
}

#[async_trait]
pub trait Notifier: Send + Sync + 'static {
    async fn send_notification<'a>(
        &self,
        ctx: &IntegrationContext,
        content: NotificationContent<'a>,
        settings: &NotificationSettings,
        stream_url: Option<String>,
        message: String,
    ) -> AppResult<()>;
}

/// Result of a notification send attempt
#[derive(Debug)]
pub struct NotificationResult {
    pub destination_type: String,
    pub destination_id: String,
    pub success: bool,
    pub error: Option<String>,
}

/// Service for sending notifications to various channels
pub struct NotificationService {
    pool: SqlitePool,
    telegram: Arc<RwLock<Option<TelegramService>>>,
    discord: Arc<RwLock<Option<DiscordService>>>,
}

impl NotificationService {
    pub fn new(state: &Arc<AppState>) -> Self {
        Self {
            pool: state.db.clone(),
            telegram: state.telegram.clone(),
            discord: state.discord.clone(),
        }
    }

    /// Send the provided notification content to all enabled integrations for the given user and log the results.
    pub async fn send_notification<'a>(
        &self,
        user_id: &str,
        content: NotificationContent<'a>,
    ) -> AppResult<Vec<NotificationResult>> {
        let settings = NotificationSettingsRepository::get_or_create(&self.pool, user_id).await?;
        let user = UserRepository::find_by_id(&self.pool, user_id)
            .await?
            .unwrap();

        // Build a stable stream URL that we can pass to notifiers
        let stream_url = Some(format!("https://twitch.tv/{}", user.twitch_login));

        // Render the message according to user settings and content
        let message = match content {
            NotificationContent::StreamOnline(data) => {
                let template = normalize_placeholders(&settings.stream_online_message);
                template
                    .replace("{streamer}", &data.streamer_name)
                    .replace("{title}", &data.title)
                    .replace("{category}", &data.category)
                    .replace("{game}", &data.category)
                    .replace("{url}", stream_url.as_deref().unwrap_or(""))
            }
            NotificationContent::StreamOffline(data) => {
                let template = normalize_placeholders(&settings.stream_offline_message);
                template.replace("{streamer}", &data.streamer_name)
            }
            NotificationContent::TitleChange(data) => {
                let template = normalize_placeholders(&settings.stream_title_change_message);
                template
                    .replace("{streamer}", &data.streamer_name)
                    .replace("{title}", &data.new_title)
            }
            NotificationContent::CategoryChange(data) => {
                let template = normalize_placeholders(&settings.stream_category_change_message);
                template
                    .replace("{streamer}", &data.streamer_name)
                    .replace("{category}", &data.new_category)
                    .replace("{game}", &data.new_category)
            }
            NotificationContent::RewardRedemption(data) => {
                let template = normalize_placeholders(&settings.reward_redemption_message);
                template
                    .replace("{user}", &data.redeemer_name)
                    .replace("{reward}", &data.reward_name)
                    .replace("{cost}", &data.reward_cost.to_string())
            }
        };

        let mut results: Vec<NotificationResult> = Vec::new();

        // Telegram integrations
        let telegram_integrations =
            TelegramIntegrationRepository::find_enabled_for_user(&self.pool, user_id).await?;

        for integration in telegram_integrations {
            let should_send = match content {
                NotificationContent::StreamOnline(_) => integration.notify_stream_online,
                NotificationContent::StreamOffline(_) => integration.notify_stream_offline,
                NotificationContent::TitleChange(_) => integration.notify_title_change,
                NotificationContent::CategoryChange(_) => integration.notify_category_change,
                NotificationContent::RewardRedemption(_) => {
                    // Only send reward notifications if both the integration and the user-level setting allow it.
                    integration.notify_reward_redemption && settings.notify_reward_redemption
                }
            };

            if should_send {
                let res = self
                    .send_telegram_notification(
                        &integration,
                        &settings,
                        content,
                        stream_url.as_deref(),
                        &message,
                    )
                    .await;
                results.push(res);
            }
        }

        // Discord integrations
        let discord_integrations =
            DiscordIntegrationRepository::find_by_user_id(&self.pool, user_id).await?;

        for integration in discord_integrations {
            let should_send = match content {
                NotificationContent::StreamOnline(_) => integration.notify_stream_online,
                NotificationContent::StreamOffline(_) => integration.notify_stream_offline,
                NotificationContent::TitleChange(_) => integration.notify_title_change,
                NotificationContent::CategoryChange(_) => integration.notify_category_change,
                NotificationContent::RewardRedemption(_) => {
                    // Require both the integration flag and the user-level flag for reward notifications.
                    integration.notify_reward_redemption && settings.notify_reward_redemption
                }
            };

            if integration.is_enabled && should_send {
                let res = self
                    .send_discord_notification(
                        &integration,
                        &settings,
                        content,
                        stream_url.as_deref(),
                        &message,
                    )
                    .await;
                results.push(res);
            }
        }

        // Choose a notification type to be logged
        let ntype = match content {
            NotificationContent::StreamOnline(_) => NotificationType::StreamOnline,
            NotificationContent::StreamOffline(_) => NotificationType::StreamOffline,
            NotificationContent::TitleChange(_) => NotificationType::TitleChange,
            NotificationContent::CategoryChange(_) => NotificationType::CategoryChange,
            NotificationContent::RewardRedemption(_) => NotificationType::RewardRedemption,
        };

        // Log notifications (use the rendered message)
        for result in &results {
            self.log_notification(user_id, ntype, result, &message)
                .await?;
        }

        Ok(results)
    }

    // normalize_placeholders moved to module level

    /// Send a notification via Telegram for any notification content
    async fn send_telegram_notification<'a>(
        &self,
        integration: &TelegramIntegration,
        settings: &NotificationSettings,
        content: NotificationContent<'a>,
        stream_url: Option<&'a str>,
        message: &str,
    ) -> NotificationResult {
        let chat_id = integration.telegram_chat_id.clone();

        // Clone the Option<TelegramService> out of the RwLock guard so we can own it
        let telegram_opt = self.telegram.read().await.clone();
        let telegram = match telegram_opt {
            Some(t) => t,
            None => {
                return NotificationResult {
                    destination_type: "telegram".to_string(),
                    destination_id: chat_id,
                    success: false,
                    error: Some("Telegram service not initialized".to_string()),
                }
            }
        };

        let ctx = IntegrationContext {
            destination_id: chat_id.clone(),
            webhook_url: None,
        };

        // Convert borrowed params into owned types expected by the Notifier trait
        let owned_stream_url = stream_url.map(|s| s.to_string());
        let send_result = telegram
            .send_notification(
                &ctx,
                content,
                settings,
                owned_stream_url,
                message.to_string(),
            )
            .await;

        match send_result {
            Ok(_) => NotificationResult {
                destination_type: "telegram".to_string(),
                destination_id: chat_id,
                success: true,
                error: None,
            },
            Err(e) => NotificationResult {
                destination_type: "telegram".to_string(),
                destination_id: chat_id,
                success: false,
                error: Some(e.to_string()),
            },
        }
    }

    /// Send a notification via Discord for any notification content (uses embeds)
    async fn send_discord_notification<'a>(
        &self,
        integration: &DiscordIntegration,
        settings: &NotificationSettings,
        content: NotificationContent<'a>,
        stream_url: Option<&'a str>,
        message: &str,
    ) -> NotificationResult {
        let channel_id = integration.discord_channel_id.clone();
        let webhook = integration.discord_webhook_url.clone();

        // Clone the Option<DiscordService> out of the RwLock guard so we can own it
        let discord_opt = self.discord.read().await.clone();
        let discord = match discord_opt {
            Some(d) => d,
            None => {
                return NotificationResult {
                    destination_type: "discord".to_string(),
                    destination_id: channel_id,
                    success: false,
                    error: Some("Discord service not initialized".to_string()),
                }
            }
        };

        let ctx = IntegrationContext {
            destination_id: channel_id.clone(),
            webhook_url: webhook.clone(),
        };

        // Convert borrowed params into owned types expected by the Notifier trait
        let owned_stream_url = stream_url.map(|s| s.to_string());
        let send_result = discord
            .send_notification(
                &ctx,
                content,
                settings,
                owned_stream_url,
                message.to_string(),
            )
            .await;

        match send_result {
            Ok(_) => NotificationResult {
                destination_type: "discord".to_string(),
                destination_id: channel_id,
                success: true,
                error: None,
            },
            Err(e) => NotificationResult {
                destination_type: "discord".to_string(),
                destination_id: channel_id,
                success: false,
                error: Some(e.to_string()),
            },
        }
    }

    async fn log_notification(
        &self,
        user_id: &str,
        notification_type: NotificationType,
        result: &NotificationResult,
        message: &str,
    ) -> AppResult<()> {
        let log = CreateNotificationLog {
            user_id: user_id.to_string(),
            notification_type: notification_type.as_str().to_string(),
            destination_type: result.destination_type.clone(),
            destination_id: result.destination_id.clone(),
            content: message.to_string(),
            status: if result.success {
                "sent".to_string()
            } else {
                "failed".to_string()
            },
            error_message: result.error.clone(),
        };

        NotificationLogRepository::create(&self.pool, log).await?;
        Ok(())
    }
}
