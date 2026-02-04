use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::db::{
    CreateNotificationLog,
    CreateNotificationTask,
    DiscordIntegration,
    DiscordIntegrationRepository,
    NotificationLogRepository,
    // Queue/retry types
    NotificationQueueRepository,
    NotificationSettings,
    NotificationSettingsRepository,
    NotificationTask,
    TelegramIntegration,
    TelegramIntegrationRepository,
    UserRepository,
};
use crate::error::AppResult;
use crate::services::discord::DiscordService;
use crate::services::telegram::TelegramService;
use crate::AppState;

use async_trait::async_trait;
use chrono::Utc;
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamOnlineData {
    pub streamer_name: String,
    pub streamer_avatar: Option<String>,
    pub title: String,
    pub category: String,
    pub thumbnail_url: Option<String>,
}

/// Data for stream offline notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamOfflineData {
    pub streamer_name: String,
}

/// Data for title change notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TitleChangeData {
    pub streamer_name: String,
    pub new_title: String,
}

/// Data for category change notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryChangeData {
    pub streamer_name: String,
    pub new_category: String,
}

/// Data for reward redemption notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Serialize the notification-specific payload so the background worker can
/// reconstruct the original `NotificationContent` and re-send it.
fn serialize_notification_content<'a>(content: NotificationContent<'a>) -> (String, String) {
    match content {
        NotificationContent::StreamOnline(data) => (
            "stream_online".to_string(),
            serde_json::to_string(&data).unwrap_or_else(|_| "{}".to_string()),
        ),
        NotificationContent::StreamOffline(data) => (
            "stream_offline".to_string(),
            serde_json::to_string(&data).unwrap_or_else(|_| "{}".to_string()),
        ),
        NotificationContent::TitleChange(data) => (
            "title_change".to_string(),
            serde_json::to_string(&data).unwrap_or_else(|_| "{}".to_string()),
        ),
        NotificationContent::CategoryChange(data) => (
            "category_change".to_string(),
            serde_json::to_string(&data).unwrap_or_else(|_| "{}".to_string()),
        ),
        NotificationContent::RewardRedemption(data) => (
            "reward_redemption".to_string(),
            serde_json::to_string(&data).unwrap_or_else(|_| "{}".to_string()),
        ),
    }
}

/// Heuristics to decide whether an error is likely transient and should be retried.
/// This inspects common HTTP API messages and network error strings.
fn is_retryable_error(err: Option<&str>, destination_type: &str) -> bool {
    let e = match err {
        Some(v) => v.to_lowercase(),
        None => return false,
    };

    // Common transient indicators
    if e.contains("too many requests")
        || e.contains("429")
        || e.contains("timeout")
        || e.contains("timed out")
        || e.contains("temporarily unavailable")
        || e.contains("service unavailable")
        || e.contains("bad gateway")
        || e.contains("connection reset")
        || e.contains("failed to send")
    {
        return true;
    }

    // Try to parse numeric status codes in known message shapes like:
    // "Discord API error (502): ..." or "Discord webhook error (503): ..."
    if destination_type == "discord"
        && (e.contains("discord api error (") || e.contains("discord webhook error ("))
    {
        if let Some(open) = e.find('(') {
            if let Some(close_rel) = e[open + 1..].find(')') {
                let code_str = &e[open + 1..open + 1 + close_rel];
                if let Ok(code) = code_str.parse::<u16>() {
                    return code == 429 || code >= 500;
                }
            }
        }
    }

    // Default conservative behavior: do not retry
    false
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
    state: Arc<AppState>,
    telegram: Arc<RwLock<Option<TelegramService>>>,
    discord: Arc<RwLock<Option<DiscordService>>>,
}

impl NotificationService {
    pub fn new(state: &Arc<AppState>) -> Self {
        Self {
            pool: state.db.clone(),
            state: state.clone(),
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

        // Determine notification type once (used for logging / queueing)
        let ntype = match content {
            NotificationContent::StreamOnline(_) => NotificationType::StreamOnline,
            NotificationContent::StreamOffline(_) => NotificationType::StreamOffline,
            NotificationContent::TitleChange(_) => NotificationType::TitleChange,
            NotificationContent::CategoryChange(_) => NotificationType::CategoryChange,
            NotificationContent::RewardRedemption(_) => NotificationType::RewardRedemption,
        };

        // Telegram integrations
        let telegram_integrations =
            TelegramIntegrationRepository::find_enabled_for_user(&self.pool, user_id).await?;

        tracing::info!(
            "Checking Telegram integrations for user {}: found {} enabled integration(s)",
            user_id,
            telegram_integrations.len()
        );

        for integration in telegram_integrations {
            let should_send = match content {
                NotificationContent::StreamOnline(_) => {
                    let enabled = integration.notify_stream_online;
                    tracing::debug!(
                        "Telegram integration {} (chat_id={}): notify_stream_online={}",
                        integration.id,
                        integration.telegram_chat_id,
                        enabled
                    );
                    enabled
                }
                NotificationContent::StreamOffline(_) => integration.notify_stream_offline,
                NotificationContent::TitleChange(_) => integration.notify_title_change,
                NotificationContent::CategoryChange(_) => integration.notify_category_change,
                NotificationContent::RewardRedemption(_) => {
                    // Only send reward notifications if both the integration and the user-level setting allow it.
                    let integration_enabled = integration.notify_reward_redemption;
                    let user_setting_enabled = settings.notify_reward_redemption;
                    tracing::debug!(
                        "Telegram integration {} (chat_id={}): notify_reward_redemption={}, user_setting_notify_reward_redemption={}",
                        integration.id,
                        integration.telegram_chat_id,
                        integration_enabled,
                        user_setting_enabled
                    );
                    integration_enabled && user_setting_enabled
                }
            };

            if should_send {
                tracing::info!(
                    "Sending notification via Telegram integration {} (chat_id={})",
                    integration.id,
                    integration.telegram_chat_id
                );
                let res = self
                    .send_telegram_notification(
                        &integration,
                        &settings,
                        content,
                        stream_url.as_deref(),
                        &message,
                    )
                    .await;

                // Determine whether this error should be retried.
                let should_retry =
                    !res.success && is_retryable_error(res.error.as_deref(), "telegram");

                // Create a log entry. If we plan to retry, mark log as 'pending'.
                let log = self
                    .log_notification(
                        user_id,
                        ntype,
                        &res,
                        &message,
                        if should_retry { Some("pending") } else { None },
                    )
                    .await?;

                if should_retry {
                    let ctx = IntegrationContext {
                        destination_id: integration.telegram_chat_id.clone(),
                        webhook_url: None,
                    };
                    // Enqueue for retries
                    self.enqueue_retry(&log, "telegram", &ctx, content, &message)
                        .await?;
                }

                results.push(res);
            } else {
                let reason = match content {
                    NotificationContent::RewardRedemption(_) => {
                        if !integration.notify_reward_redemption {
                            "integration notify_reward_redemption disabled"
                        } else if !settings.notify_reward_redemption {
                            "user-level notify_reward_redemption setting disabled"
                        } else {
                            "notification type not enabled"
                        }
                    }
                    _ => "notification type not enabled for this integration",
                };
                tracing::debug!(
                    "Skipping Telegram integration {} (chat_id={}): {}",
                    integration.id,
                    integration.telegram_chat_id,
                    reason
                );
            }
        }

        // Discord integrations
        let discord_integrations =
            DiscordIntegrationRepository::find_by_user_id(&self.pool, user_id).await?;

        tracing::info!(
            "Checking Discord integrations for user {}: found {} integration(s)",
            user_id,
            discord_integrations.len()
        );

        for integration in discord_integrations {
            let should_send = match content {
                NotificationContent::StreamOnline(_) => {
                    let enabled = integration.notify_stream_online;
                    tracing::debug!(
                        "Discord integration {} (channel_id={}): is_enabled={}, notify_stream_online={}",
                        integration.id,
                        integration.discord_channel_id,
                        integration.is_enabled,
                        enabled
                    );
                    enabled
                }
                NotificationContent::StreamOffline(_) => integration.notify_stream_offline,
                NotificationContent::TitleChange(_) => integration.notify_title_change,
                NotificationContent::CategoryChange(_) => integration.notify_category_change,
                NotificationContent::RewardRedemption(_) => {
                    // Require both the integration flag and the user-level flag for reward notifications.
                    let integration_enabled = integration.notify_reward_redemption;
                    let user_setting_enabled = settings.notify_reward_redemption;
                    tracing::debug!(
                        "Discord integration {} (channel_id={}): notify_reward_redemption={}, user_setting_notify_reward_redemption={}",
                        integration.id,
                        integration.discord_channel_id,
                        integration_enabled,
                        user_setting_enabled
                    );
                    integration_enabled && user_setting_enabled
                }
            };

            if !integration.is_enabled {
                tracing::debug!(
                    "Skipping Discord integration {} (channel_id={}): integration is disabled",
                    integration.id,
                    integration.discord_channel_id
                );
                continue;
            }

            if should_send {
                tracing::info!(
                    "Sending notification via Discord integration {} (channel_id={})",
                    integration.id,
                    integration.discord_channel_id
                );
                let res = self
                    .send_discord_notification(
                        &integration,
                        &settings,
                        content,
                        stream_url.as_deref(),
                        &message,
                    )
                    .await;

                let should_retry =
                    !res.success && is_retryable_error(res.error.as_deref(), "discord");

                let log = self
                    .log_notification(
                        user_id,
                        ntype,
                        &res,
                        &message,
                        if should_retry { Some("pending") } else { None },
                    )
                    .await?;

                if should_retry {
                    let ctx = IntegrationContext {
                        destination_id: integration.discord_channel_id.clone(),
                        webhook_url: integration.discord_webhook_url.clone(),
                    };
                    self.enqueue_retry(&log, "discord", &ctx, content, &message)
                        .await?;
                }

                results.push(res);
            } else {
                let reason = match content {
                    NotificationContent::RewardRedemption(_) => {
                        if !integration.notify_reward_redemption {
                            "integration notify_reward_redemption disabled"
                        } else if !settings.notify_reward_redemption {
                            "user-level notify_reward_redemption setting disabled"
                        } else {
                            "notification type not enabled"
                        }
                    }
                    _ => "notification type not enabled for this integration",
                };
                tracing::debug!(
                    "Skipping Discord integration {} (channel_id={}): {}",
                    integration.id,
                    integration.discord_channel_id,
                    reason
                );
            }
        }

        if results.is_empty() {
            let user_setting_info = match content {
                NotificationContent::RewardRedemption(_) => {
                    format!(", user_setting_notify_reward_redemption={}", settings.notify_reward_redemption)
                }
                _ => String::new(),
            };
            tracing::warn!(
                "No notifications were sent for user {} (notification_type={}{}). Possible reasons: \
                1) No integrations configured, \
                2) All integrations disabled, \
                3) Notification type disabled for all integrations{}",
                user_id,
                ntype.as_str(),
                user_setting_info,
                if matches!(content, NotificationContent::RewardRedemption(_)) {
                    ", 4) User-level notify_reward_redemption setting disabled"
                } else {
                    ""
                }
            );
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
        status_override: Option<&str>,
    ) -> AppResult<crate::db::NotificationLog> {
        let status = if result.success {
            "sent".to_string()
        } else if let Some(ov) = status_override {
            ov.to_string()
        } else {
            "failed".to_string()
        };

        let log = CreateNotificationLog {
            user_id: user_id.to_string(),
            notification_type: notification_type.as_str().to_string(),
            destination_type: result.destination_type.clone(),
            destination_id: result.destination_id.clone(),
            content: message.to_string(),
            status,
            error_message: result.error.clone(),
        };

        let created = NotificationLogRepository::create(&self.pool, log).await?;
        Ok(created)
    }

    /// Enqueue a failed notification for background retry processing.
    async fn enqueue_retry<'a>(
        &self,
        log: &crate::db::NotificationLog,
        destination_type: &str,
        ctx: &IntegrationContext,
        content: NotificationContent<'a>,
        message: &str,
    ) -> AppResult<()> {
        // Serialize specific payload & choose initial schedule based on config
        let (notification_type, content_json) = serialize_notification_content(content);

        let cfg = &self.state.config.notification_retry;
        let initial_secs = cfg.initial_backoff_seconds as i64;
        let next_attempt_at = Utc::now().naive_utc() + chrono::Duration::seconds(initial_secs);

        // Determine expiration/TTL based on notification type to avoid retrying stale events
        let expires_in_secs = match notification_type.as_str() {
            "stream_online" => cfg.stream_online_ttl_seconds,
            "title_change" => cfg.title_change_ttl_seconds,
            "category_change" => cfg.category_change_ttl_seconds,
            "reward_redemption" => cfg.reward_redemption_ttl_seconds,
            _ => cfg.default_ttl_seconds,
        } as i64;

        let expires_at = Utc::now().naive_utc() + chrono::Duration::seconds(expires_in_secs);

        let task = CreateNotificationTask {
            notification_log_id: Some(log.id.clone()),
            user_id: log.user_id.clone(),
            notification_type,
            content_json,
            message: message.to_string(),
            destination_type: destination_type.to_string(),
            destination_id: ctx.destination_id.clone(),
            webhook_url: ctx.webhook_url.clone(),
            max_attempts: Some(cfg.max_attempts as i32),
            next_attempt_at: Some(next_attempt_at),
            expires_at: Some(expires_at),
        };

        NotificationQueueRepository::create(&self.pool, task).await?;
        tracing::info!(
            "Enqueued notification retry: log={}, dest={}, next_attempt_at={}, expires_at={}",
            log.id,
            ctx.destination_id,
            next_attempt_at,
            expires_at
        );
        Ok(())
    }

    /// Process a single queued notification task: attempt delivery, schedule retries,
    /// and move to DLQ when necessary.
    ///
    /// The method:
    ///  - skips/expiries tasks past `expires_at`,
    ///  - attempts delivery via Telegram/Discord services,
    ///  - on success marks the queue entry as `succeeded` and the notification log as `sent`,
    ///  - on transient failure computes exponential backoff, increments attempts and reschedules,
    ///  - on permanent failure or when max attempts are exhausted, moves the task to `dead` (DLQ)
    ///    and marks the notification log as `failed`.
    pub async fn process_queued_task(&self, task: NotificationTask) -> AppResult<()> {
        let now = Utc::now().naive_utc();

        // If expired, move to DLQ and update log as 'expired'
        if let Some(exp) = task.expires_at {
            if exp <= now {
                tracing::info!(
                    "Notification task {} expired (expires_at={} now={}), moving to DLQ",
                    task.id,
                    exp,
                    now
                );
                let _ = NotificationQueueRepository::mark_dead(
                    &self.pool,
                    &task.id,
                    Some("expired".to_string()),
                )
                .await;
                if let Some(ref log_id) = task.notification_log_id {
                    let _ = NotificationLogRepository::update_status(
                        &self.pool,
                        log_id,
                        "expired",
                        Some("Notification expired"),
                    )
                    .await;
                }
                return Ok(());
            }
        }

        // Load notification settings and user; failures to load are treated as transient
        let settings =
            match NotificationSettingsRepository::get_or_create(&self.pool, &task.user_id).await {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!(
                        "Failed to load notification settings for user {}: {:?}",
                        task.user_id,
                        e
                    );
                    let cfg = &self.state.config.notification_retry;
                    let next = now + chrono::Duration::seconds(cfg.initial_backoff_seconds as i64);
                    let _ = NotificationQueueRepository::register_attempt_and_schedule(
                        &self.pool,
                        &task.id,
                        next,
                        Some(format!("Failed to load settings: {}", e)),
                    )
                    .await;
                    return Ok(());
                }
            };

        let user = match UserRepository::find_by_id(&self.pool, &task.user_id).await {
            Ok(Some(u)) => u,
            Ok(None) => {
                tracing::warn!(
                    "User {} for notification task {} not found; moving to DLQ",
                    task.user_id,
                    task.id
                );
                let _ = NotificationQueueRepository::mark_dead(
                    &self.pool,
                    &task.id,
                    Some("user not found".to_string()),
                )
                .await;
                if let Some(ref log_id) = task.notification_log_id {
                    let _ = NotificationLogRepository::update_status(
                        &self.pool,
                        log_id,
                        "failed",
                        Some("User not found"),
                    )
                    .await;
                }
                return Ok(());
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to fetch user {} for notification task {}: {:?}",
                    task.user_id,
                    task.id,
                    e
                );
                let cfg = &self.state.config.notification_retry;
                let next = now + chrono::Duration::seconds(cfg.initial_backoff_seconds as i64);
                let _ = NotificationQueueRepository::register_attempt_and_schedule(
                    &self.pool,
                    &task.id,
                    next,
                    Some(format!("Failed to fetch user: {}", e)),
                )
                .await;
                return Ok(());
            }
        };

        let stream_url = Some(format!("https://twitch.tv/{}", user.twitch_login));
        let ctx = IntegrationContext {
            destination_id: task.destination_id.clone(),
            webhook_url: task.webhook_url.clone(),
        };

        // Attempt sending via the appropriate service.
        let send_result: Result<(), crate::error::AppError> = match task.destination_type.as_str() {
            "telegram" => {
                let telegram_opt = self.telegram.read().await.clone();
                let telegram = match telegram_opt {
                    Some(t) => t,
                    None => {
                        // Service not initialized -> transient; schedule retry
                        let cfg = &self.state.config.notification_retry;
                        let next =
                            now + chrono::Duration::seconds(cfg.initial_backoff_seconds as i64);
                        let _ = NotificationQueueRepository::register_attempt_and_schedule(
                            &self.pool,
                            &task.id,
                            next,
                            Some("Telegram service not initialized".to_string()),
                        )
                        .await;
                        return Ok(());
                    }
                };

                match task.notification_type.as_str() {
                    "stream_online" => {
                        let data: StreamOnlineData = serde_json::from_str(&task.content_json)
                            .map_err(|e| crate::error::AppError::Internal(anyhow::anyhow!(e)))?;
                        telegram
                            .send_notification(
                                &ctx,
                                NotificationContent::StreamOnline(&data),
                                &settings,
                                stream_url,
                                task.message.clone(),
                            )
                            .await
                    }
                    "stream_offline" => {
                        let data: StreamOfflineData = serde_json::from_str(&task.content_json)
                            .map_err(|e| crate::error::AppError::Internal(anyhow::anyhow!(e)))?;
                        telegram
                            .send_notification(
                                &ctx,
                                NotificationContent::StreamOffline(&data),
                                &settings,
                                stream_url,
                                task.message.clone(),
                            )
                            .await
                    }
                    "title_change" => {
                        let data: TitleChangeData = serde_json::from_str(&task.content_json)
                            .map_err(|e| crate::error::AppError::Internal(anyhow::anyhow!(e)))?;
                        telegram
                            .send_notification(
                                &ctx,
                                NotificationContent::TitleChange(&data),
                                &settings,
                                stream_url,
                                task.message.clone(),
                            )
                            .await
                    }
                    "category_change" => {
                        let data: CategoryChangeData = serde_json::from_str(&task.content_json)
                            .map_err(|e| crate::error::AppError::Internal(anyhow::anyhow!(e)))?;
                        telegram
                            .send_notification(
                                &ctx,
                                NotificationContent::CategoryChange(&data),
                                &settings,
                                stream_url,
                                task.message.clone(),
                            )
                            .await
                    }
                    "reward_redemption" => {
                        let data: RewardRedemptionData = serde_json::from_str(&task.content_json)
                            .map_err(|e| {
                            crate::error::AppError::Internal(anyhow::anyhow!(e))
                        })?;
                        telegram
                            .send_notification(
                                &ctx,
                                NotificationContent::RewardRedemption(&data),
                                &settings,
                                stream_url,
                                task.message.clone(),
                            )
                            .await
                    }
                    _ => Err(crate::error::AppError::BadRequest(
                        "Unknown notification type".to_string(),
                    )),
                }
            }
            "discord" => {
                let discord_opt = self.discord.read().await.clone();
                let discord = match discord_opt {
                    Some(d) => d,
                    None => {
                        // Service not initialized -> transient; schedule retry
                        let cfg = &self.state.config.notification_retry;
                        let next =
                            now + chrono::Duration::seconds(cfg.initial_backoff_seconds as i64);
                        let _ = NotificationQueueRepository::register_attempt_and_schedule(
                            &self.pool,
                            &task.id,
                            next,
                            Some("Discord service not initialized".to_string()),
                        )
                        .await;
                        return Ok(());
                    }
                };

                match task.notification_type.as_str() {
                    "stream_online" => {
                        let data: StreamOnlineData = serde_json::from_str(&task.content_json)
                            .map_err(|e| crate::error::AppError::Internal(anyhow::anyhow!(e)))?;
                        discord
                            .send_notification(
                                &ctx,
                                NotificationContent::StreamOnline(&data),
                                &settings,
                                stream_url,
                                task.message.clone(),
                            )
                            .await
                    }
                    "stream_offline" => {
                        let data: StreamOfflineData = serde_json::from_str(&task.content_json)
                            .map_err(|e| crate::error::AppError::Internal(anyhow::anyhow!(e)))?;
                        discord
                            .send_notification(
                                &ctx,
                                NotificationContent::StreamOffline(&data),
                                &settings,
                                stream_url,
                                task.message.clone(),
                            )
                            .await
                    }
                    "title_change" => {
                        let data: TitleChangeData = serde_json::from_str(&task.content_json)
                            .map_err(|e| crate::error::AppError::Internal(anyhow::anyhow!(e)))?;
                        discord
                            .send_notification(
                                &ctx,
                                NotificationContent::TitleChange(&data),
                                &settings,
                                stream_url,
                                task.message.clone(),
                            )
                            .await
                    }
                    "category_change" => {
                        let data: CategoryChangeData = serde_json::from_str(&task.content_json)
                            .map_err(|e| crate::error::AppError::Internal(anyhow::anyhow!(e)))?;
                        discord
                            .send_notification(
                                &ctx,
                                NotificationContent::CategoryChange(&data),
                                &settings,
                                stream_url,
                                task.message.clone(),
                            )
                            .await
                    }
                    "reward_redemption" => {
                        let data: RewardRedemptionData = serde_json::from_str(&task.content_json)
                            .map_err(|e| {
                            crate::error::AppError::Internal(anyhow::anyhow!(e))
                        })?;
                        discord
                            .send_notification(
                                &ctx,
                                NotificationContent::RewardRedemption(&data),
                                &settings,
                                stream_url,
                                task.message.clone(),
                            )
                            .await
                    }
                    _ => Err(crate::error::AppError::BadRequest(
                        "Unknown notification type".to_string(),
                    )),
                }
            }
            _ => {
                // Unknown destination type -> move to DLQ and update the log.
                let msg = format!("Unknown destination type: {}", task.destination_type);
                let _ =
                    NotificationQueueRepository::mark_dead(&self.pool, &task.id, Some(msg.clone()))
                        .await;
                if let Some(ref log_id) = task.notification_log_id {
                    let _ = NotificationLogRepository::update_status(
                        &self.pool,
                        log_id,
                        "failed",
                        Some(&msg),
                    )
                    .await;
                }
                return Ok(());
            }
        };

        // Handle send result
        match send_result {
            Ok(_) => {
                // Success -> mark succeeded and update log
                let _ = NotificationQueueRepository::mark_succeeded(&self.pool, &task.id).await;
                if let Some(ref log_id) = task.notification_log_id {
                    let _ =
                        NotificationLogRepository::update_status(&self.pool, log_id, "sent", None)
                            .await;
                }
                tracing::info!("Queued notification {} sent successfully", task.id);
                Ok(())
            }
            Err(e) => {
                let err_str = e.to_string();

                // Permanent errors -> move to DLQ
                if !is_retryable_error(Some(&err_str), &task.destination_type) {
                    let _ = NotificationQueueRepository::mark_dead(
                        &self.pool,
                        &task.id,
                        Some(err_str.clone()),
                    )
                    .await;
                    if let Some(ref log_id) = task.notification_log_id {
                        let _ = NotificationLogRepository::update_status(
                            &self.pool,
                            log_id,
                            "failed",
                            Some(&err_str),
                        )
                        .await;
                    }
                    tracing::warn!("Queued notification {} moved to DLQ: {}", task.id, err_str);
                    return Ok(());
                }

                // Transient error -> schedule retry with exponential backoff
                let cfg = &self.state.config.notification_retry;
                let attempts = task.attempts as u32;

                // Compute delay = min(max_backoff, initial_backoff * 2^attempts)
                let mut delay: u128 = cfg.initial_backoff_seconds as u128;
                for _ in 0..attempts {
                    delay = delay.saturating_mul(2);
                    if delay as u64 >= cfg.max_backoff_seconds {
                        delay = cfg.max_backoff_seconds as u128;
                        break;
                    }
                }
                if delay as u64 > cfg.max_backoff_seconds {
                    delay = cfg.max_backoff_seconds as u128;
                }

                let next = now + chrono::Duration::seconds(delay as i64);

                match NotificationQueueRepository::register_attempt_and_schedule(
                    &self.pool,
                    &task.id,
                    next,
                    Some(err_str.clone()),
                )
                .await
                {
                    Ok(updated_task) => {
                        if updated_task.status == "dead" {
                            if let Some(ref log_id) = task.notification_log_id {
                                let _ = NotificationLogRepository::update_status(
                                    &self.pool,
                                    log_id,
                                    "failed",
                                    Some(&err_str),
                                )
                                .await;
                            }
                            tracing::warn!(
                                "Queued notification {} reached max attempts and moved to DLQ",
                                task.id
                            );
                        } else {
                            if let Some(ref log_id) = task.notification_log_id {
                                let _ = NotificationLogRepository::update_status(
                                    &self.pool,
                                    log_id,
                                    "pending",
                                    Some(&err_str),
                                )
                                .await;
                            }
                            tracing::info!(
                                "Queued notification {} rescheduled after error: {}",
                                task.id,
                                err_str
                            );
                        }
                        Ok(())
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to reschedule queued notification {}: {:?}",
                            task.id,
                            e
                        );
                        Ok(())
                    }
                }
            }
        }
    }
}
