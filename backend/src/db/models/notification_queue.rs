use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Represents a queued notification retry task.
///
/// Each record corresponds to an attempted notification delivery that should be
/// retried by a background worker using exponential backoff. The entry stores
/// the serialized notification-specific payload (`content_json`) and the
/// rendered `message` so retransmits are consistent even if user settings
/// change later.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct NotificationTask {
    /// Primary key (UUID)
    pub id: String,

    /// Optional reference to the original notification log entry.
    pub notification_log_id: Option<String>,

    /// Owning user id (references `users.id`)
    pub user_id: String,

    /// Notification type (e.g. 'stream_online', 'title_change', ...)
    pub notification_type: String,

    /// JSON-serialized payload for the specific notification variant.
    /// Contents are deserialized by the worker to re-create `NotificationContent`.
    pub content_json: String,

    /// The rendered message (template expanded at the time the task was created).
    /// Used for Telegram messages and for the textual content of Discord messages.
    pub message: String,

    /// Destination type ('telegram', 'discord', ...)
    pub destination_type: String,

    /// Destination id (chat_id for Telegram, channel_id for Discord)
    pub destination_id: String,

    /// Optional webhook URL (used for Discord webhook-based integrations).
    pub webhook_url: Option<String>,

    /// Number of attempts already made.
    pub attempts: i32,

    /// Maximum attempts permitted before moving the task to DLQ.
    pub max_attempts: i32,

    /// Timestamp when the task becomes eligible for the next retry.
    pub next_attempt_at: NaiveDateTime,

    /// Last error message observed when an attempt failed (if any).
    pub last_error: Option<String>,

    /// Task status: 'pending', 'processing', 'succeeded', 'dead' (DLQ)
    pub status: String,

    /// Creation timestamp
    pub created_at: NaiveDateTime,

    /// Last update timestamp
    pub updated_at: NaiveDateTime,

    /// Optional expiration timestamp (TTL). If present and <= now, the worker should treat the task as expired.
    pub expires_at: Option<NaiveDateTime>,
}

/// Data required to create a new queued notification task.
///
/// The background worker expects `content_json` to contain enough information
/// to reconstruct the original `NotificationContent` so an embed/message can be
/// recreated on retries. `max_attempts`, `next_attempt_at`, and `expires_at` are optional
/// when creating tasks and will be defaulted by repository logic when omitted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateNotificationTask {
    pub notification_log_id: Option<String>,
    pub user_id: String,
    pub notification_type: String,
    pub content_json: String,
    pub message: String,
    pub destination_type: String,
    pub destination_id: String,
    pub webhook_url: Option<String>,

    /// Optional override for maximum attempts; repository can default this.
    pub max_attempts: Option<i32>,

    /// Optional explicit schedule for the first attempt; defaults to now or a configured backoff.
    pub next_attempt_at: Option<NaiveDateTime>,

    /// Optional explicit expiration time (if not set, DB trigger or application default will apply).
    pub expires_at: Option<NaiveDateTime>,
}
