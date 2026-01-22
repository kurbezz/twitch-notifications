use serde::Serialize;
use teloxide::prelude::*;
use teloxide::types::{ChatId, ParseMode};

use crate::db::NotificationSettings;
use crate::error::{AppError, AppResult};
use crate::services::notifications::{IntegrationContext, NotificationContent, Notifier};
use async_trait::async_trait;

#[derive(Clone)]
pub struct TelegramService {
    bot: Bot,
}

#[derive(Debug, Clone, Serialize)]
pub struct TelegramMessage {
    pub chat_id: String,
    pub text: String,
    pub parse_mode: Option<String>,
    pub disable_web_page_preview: bool,
    pub disable_notification: bool,
}

impl Default for TelegramMessage {
    fn default() -> Self {
        Self {
            chat_id: String::new(),
            text: String::new(),
            parse_mode: Some("HTML".to_string()),
            disable_web_page_preview: false,
            disable_notification: false,
        }
    }
}

impl TelegramService {
    pub async fn new(token: String) -> AppResult<Self> {
        let bot = Bot::new(token);

        // Verify the bot token by getting bot info
        match bot.get_me().await {
            Ok(me) => {
                tracing::info!("Telegram bot initialized: @{}", me.username());
                Ok(Self { bot })
            }
            Err(e) => {
                tracing::error!("Failed to initialize Telegram bot: {}", e);
                Err(AppError::Telegram(format!(
                    "Failed to initialize bot: {}",
                    e
                )))
            }
        }
    }

    pub async fn send_message(&self, message: TelegramMessage) -> AppResult<i32> {
        let chat_id: i64 = message
            .chat_id
            .parse()
            .map_err(|_| AppError::Telegram("Invalid chat_id".to_string()))?;

        let mut request = self
            .bot
            .send_message(ChatId(chat_id), &message.text)
            .disable_web_page_preview(message.disable_web_page_preview)
            .disable_notification(message.disable_notification);

        if message.parse_mode.as_deref() == Some("HTML") {
            request = request.parse_mode(ParseMode::Html);
        } else if message.parse_mode.as_deref() == Some("Markdown") {
            request = request.parse_mode(ParseMode::MarkdownV2);
        }

        match request.await {
            Ok(sent_message) => {
                tracing::debug!(
                    "Telegram message sent to {}: message_id={}",
                    message.chat_id,
                    sent_message.id
                );
                Ok(sent_message.id.0)
            }
            Err(e) => {
                tracing::error!("Failed to send Telegram message: {}", e);
                Err(AppError::Telegram(format!("Failed to send message: {}", e)))
            }
        }
    }

    pub fn get_bot(&self) -> &Bot {
        &self.bot
    }

    /// Check whether a given user id is an administrator in the given chat.
    /// Returns Ok(true) if the user is an administrator or the creator, Ok(false) if not,
    /// or an AppError if the check could not be completed (e.g. bot not a member).
    pub async fn is_user_admin(&self, chat_id: &str, user_id: &str) -> AppResult<bool> {
        let chat_id: i64 = chat_id
            .parse()
            .map_err(|_| AppError::Telegram("Invalid chat_id".to_string()))?;
        // Telegram user IDs are represented as unsigned in teloxide types; parse as u64 to match
        let user_id: u64 = user_id
            .parse()
            .map_err(|_| AppError::Telegram("Invalid user_id".to_string()))?;

        // Attempt to fetch the list of chat administrators using the bot.
        // Note: this requires the bot to be a member of the chat; if it's not,
        // the call will fail and we return an error so the caller can surface
        // an actionable message (e.g. ask to add the bot to the chat).
        match self.bot.get_chat_administrators(ChatId(chat_id)).await {
            Ok(admins) => {
                for admin in admins.into_iter() {
                    // admin.user.id is a UserId wrapper; compare by .0
                    if admin.user.id.0 == user_id {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            Err(e) => {
                tracing::warn!("Failed to fetch chat administrators for {}: {}", chat_id, e);
                Err(AppError::Telegram(format!(
                    "Failed to fetch chat administrators: {}",
                    e
                )))
            }
        }
    }
}

// Notifier implementation moved to bottom of file.

/// Data returned when Telegram login has been verified.
#[derive(Debug, Clone)]
pub struct TelegramLoginInfo {
    pub id: String,
    pub username: Option<String>,
    pub photo_url: Option<String>,
}

/// Verify Telegram Login Widget payload according to Telegram's docs:
/// https://core.telegram.org/widgets/login
///
/// Returns `TelegramLoginInfo` when the `hash` is valid and the payload is recent.
pub fn verify_telegram_login_payload(
    payload: &std::collections::HashMap<String, String>,
    bot_token: &str,
) -> AppResult<TelegramLoginInfo> {
    use hmac::{Hmac, Mac};
    use sha2::{Digest, Sha256};

    // Extract hash
    let received_hash = payload.get("hash").ok_or_else(|| {
        AppError::BadRequest("Missing hash in Telegram login payload".to_string())
    })?;

    // Build data_check_string by sorting keys (excluding hash) and joining key=value with \n
    let mut items: Vec<(String, String)> = payload
        .iter()
        .filter(|(k, _)| k.as_str() != "hash")
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    items.sort_by(|a, b| a.0.cmp(&b.0));

    let data_check_string = items
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<String>>()
        .join("\n");

    // Secret key is sha256(bot_token)
    let secret_key = Sha256::digest(bot_token.as_bytes());

    // HMAC-SHA256
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(&secret_key)
        .map_err(|e| AppError::Telegram(format!("Failed to initialize HMAC: {}", e)))?;
    mac.update(data_check_string.as_bytes());
    let computed = mac.finalize().into_bytes();
    let computed_hex = hex::encode(computed);

    if computed_hex != *received_hash {
        tracing::warn!(
            "Telegram login payload hash mismatch (expected {}, got {})",
            computed_hex,
            received_hash
        );
        return Err(AppError::BadRequest(
            "Invalid Telegram login data".to_string(),
        ));
    }

    // Parse auth_date
    let auth_date_str = payload.get("auth_date").ok_or_else(|| {
        AppError::BadRequest("Missing auth_date in Telegram login payload".to_string())
    })?;
    let auth_date = auth_date_str.parse::<i64>().map_err(|_| {
        AppError::BadRequest("Invalid auth_date in Telegram login payload".to_string())
    })?;

    // Reject stale auths (older than 24 hours)
    let now = chrono::Utc::now().timestamp();
    if now - auth_date > 24 * 60 * 60 {
        return Err(AppError::BadRequest(
            "Telegram login data is too old".to_string(),
        ));
    }

    // Ensure id exists
    let id = payload
        .get("id")
        .cloned()
        .ok_or_else(|| AppError::BadRequest("Missing id in Telegram login payload".to_string()))?;

    // Build returned struct
    let info = TelegramLoginInfo {
        id,
        username: payload.get("username").cloned(),
        photo_url: payload.get("photo_url").cloned(),
    };

    Ok(info)
}

#[async_trait]
impl Notifier for TelegramService {
    async fn send_notification<'a>(
        &self,
        ctx: &IntegrationContext,
        _content: NotificationContent<'a>,
        _settings: &NotificationSettings,
        _stream_url: Option<String>,
        message: String,
    ) -> AppResult<()> {
        // The message is rendered by NotificationService and passed in here —
        // send it verbatim to Telegram.
        self.send_message(TelegramMessage {
            chat_id: ctx.destination_id.clone(),
            text: message,
            ..Default::default()
        })
        .await
        .map(|_| ())
    }
}
