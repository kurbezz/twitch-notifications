use std::sync::Arc;

use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::post,
    Router,
};
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;

use crate::db::{NotificationSettingsRepository, UserRepository};
use crate::error::{AppError, AppResult};
use crate::services::notifications::{
    CategoryChangeData, NotificationContent, NotificationService, RewardRedemptionData,
    StreamOfflineData, StreamOnlineData, TitleChangeData,
};
use crate::AppState;
use chrono::{Duration, Utc};

type HmacSha256 = Hmac<Sha256>;

const TWITCH_MESSAGE_ID_HEADER: &str = "twitch-eventsub-message-id";
const TWITCH_MESSAGE_TIMESTAMP_HEADER: &str = "twitch-eventsub-message-timestamp";
const TWITCH_MESSAGE_SIGNATURE_HEADER: &str = "twitch-eventsub-message-signature";
const TWITCH_MESSAGE_TYPE_HEADER: &str = "twitch-eventsub-message-type";

// Message types
const MESSAGE_TYPE_VERIFICATION: &str = "webhook_callback_verification";
const MESSAGE_TYPE_NOTIFICATION: &str = "notification";
const MESSAGE_TYPE_REVOCATION: &str = "revocation";

// Subscription types
const SUB_TYPE_STREAM_ONLINE: &str = "stream.online";
const SUB_TYPE_STREAM_OFFLINE: &str = "stream.offline";
const SUB_TYPE_CHANNEL_UPDATE: &str = "channel.update";
const SUB_TYPE_CHANNEL_POINTS_REDEMPTION: &str =
    "channel.channel_points_custom_reward_redemption.add";

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/twitch", post(handle_twitch_webhook))
}

// ============================================================================
// EventSub Payload Types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct EventSubPayload {
    pub subscription: EventSubSubscription,
    pub challenge: Option<String>,
    pub event: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct EventSubSubscription {
    pub id: String,
    #[serde(rename = "type")]
    pub subscription_type: String,
    pub status: String,
}

// Event types

#[derive(Debug, Deserialize)]
pub struct StreamOnlineEvent {
    pub broadcaster_user_id: String,
    pub broadcaster_user_name: String,
}

#[derive(Debug, Deserialize)]
pub struct StreamOfflineEvent {
    pub broadcaster_user_id: String,
    pub broadcaster_user_name: String,
}

#[derive(Debug, Deserialize)]
pub struct ChannelUpdateEvent {
    pub broadcaster_user_id: String,
    pub broadcaster_user_name: String,
    pub title: String,
    pub category_id: String,
    pub category_name: String,
}

#[derive(Debug, Deserialize)]
pub struct ChannelPointsRedemptionEvent {
    pub broadcaster_user_id: String,
    pub broadcaster_user_name: String,
    pub user_name: String,
    pub user_input: Option<String>,
    pub reward: RewardInfo,
}

#[derive(Debug, Deserialize)]
pub struct RewardInfo {
    pub title: String,
    pub cost: i32,
}

// ============================================================================
// Stream State Cache (for detecting changes)
// ============================================================================

use std::collections::HashMap;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct StreamState {
    pub is_live: bool,
    pub title: String,
    pub category_id: String,
}

lazy_static::lazy_static! {
    static ref STREAM_STATE_CACHE: RwLock<HashMap<String, StreamState>> = RwLock::new(HashMap::new());
}

// ============================================================================
// Webhook Handler
// ============================================================================

async fn handle_twitch_webhook(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<(StatusCode, String), AppError> {
    // Extract headers
    let message_id = get_header(&headers, TWITCH_MESSAGE_ID_HEADER)?;
    let timestamp = get_header(&headers, TWITCH_MESSAGE_TIMESTAMP_HEADER)?;
    let signature = get_header(&headers, TWITCH_MESSAGE_SIGNATURE_HEADER)?;
    let message_type = get_header(&headers, TWITCH_MESSAGE_TYPE_HEADER)?;

    // Verify signature
    verify_signature(&state, &message_id, &timestamp, &body, &signature)?;

    // Parse the payload
    let payload: EventSubPayload = serde_json::from_slice(&body)
        .map_err(|e| AppError::BadRequest(format!("Invalid payload: {}", e)))?;

    tracing::info!(
        "Received EventSub webhook: message_type={}, subscription_type={}, subscription_id={}",
        message_type,
        payload.subscription.subscription_type,
        payload.subscription.id
    );

    // Handle based on message type
    match message_type.as_str() {
        MESSAGE_TYPE_VERIFICATION => {
            // Respond with the challenge for webhook verification
            let challenge = payload
                .challenge
                .ok_or_else(|| AppError::BadRequest("Missing challenge".to_string()))?;

            tracing::info!(
                "Webhook verification challenge received: subscription_id={}, subscription_type={}, challenge={}",
                payload.subscription.id,
                payload.subscription.subscription_type,
                challenge
            );

            // Try to update subscription status in DB if it exists
            if let Ok(Some(db_sub)) =
                crate::db::EventSubSubscriptionRepository::find_by_twitch_subscription_id(
                    &state.db,
                    &payload.subscription.id,
                )
                .await
            {
                tracing::info!(
                    "Found subscription in DB for verification: id={}, current_status={}",
                    db_sub.id,
                    db_sub.status
                );

                // Update status to enabled after successful verification response
                // Note: Twitch will send another message with status='enabled' after verification,
                // but we can optimistically update it here since we're responding correctly
                if let Ok(updated) = crate::db::EventSubSubscriptionRepository::update_status(
                    &state.db,
                    &payload.subscription.id,
                    "enabled",
                )
                .await
                {
                    tracing::info!(
                        "Updated subscription status to 'enabled' after successful verification: subscription_id={}, type={}",
                        updated.twitch_subscription_id,
                        updated.subscription_type
                    );
                } else {
                    tracing::warn!(
                        "Failed to update subscription status in DB after verification: subscription_id={}",
                        payload.subscription.id
                    );
                }
            } else {
                tracing::warn!(
                    "Webhook verification received for subscription {} but not found in DB. This may indicate a sync issue.",
                    payload.subscription.id
                );
            }

            tracing::info!(
                "Responding to webhook verification challenge for subscription: {} (type: {})",
                payload.subscription.id,
                payload.subscription.subscription_type
            );

            Ok((StatusCode::OK, challenge))
        }
        MESSAGE_TYPE_NOTIFICATION => {
            // Handle the notification
            handle_notification(&state, &payload).await?;
            Ok((StatusCode::OK, "OK".to_string()))
        }
        MESSAGE_TYPE_REVOCATION => {
            // Handle subscription revocation
            tracing::warn!(
                "Subscription revoked: id={}, type={}, reason={}",
                payload.subscription.id,
                payload.subscription.subscription_type,
                payload.subscription.status
            );
            Ok((StatusCode::OK, "OK".to_string()))
        }
        _ => {
            tracing::warn!("Unknown message type: {}", message_type);
            Ok((StatusCode::OK, "OK".to_string()))
        }
    }
}

// ============================================================================
// Signature Verification
// ============================================================================

fn verify_signature(
    state: &Arc<AppState>,
    message_id: &str,
    timestamp: &str,
    body: &[u8],
    signature: &str,
) -> AppResult<()> {
    // Get the secret from config
    let secret = &state.config.jwt.secret; // Using JWT secret as webhook secret for simplicity

    // Build the message to sign: message_id + timestamp + body
    let mut message = Vec::new();
    message.extend_from_slice(message_id.as_bytes());
    message.extend_from_slice(timestamp.as_bytes());
    message.extend_from_slice(body);

    // Create HMAC
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|_| AppError::Internal(anyhow::anyhow!("Failed to create HMAC")))?;

    mac.update(&message);

    // Parse the signature (format: sha256=<hex>)
    let expected_sig = if let Some(hex_sig) = signature.strip_prefix("sha256=") {
        hex::decode(hex_sig)
            .map_err(|_| AppError::BadRequest("Invalid signature format".to_string()))?
    } else {
        return Err(AppError::BadRequest("Invalid signature format".to_string()));
    };

    // Verify
    mac.verify_slice(&expected_sig)
        .map_err(|_| AppError::Unauthorized)?;

    // Check timestamp is not too old (within 10 minutes)
    if let Ok(msg_time) = chrono::DateTime::parse_from_rfc3339(timestamp) {
        let now = chrono::Utc::now();
        let diff = now.signed_duration_since(msg_time);
        if diff.num_minutes().abs() > 10 {
            return Err(AppError::BadRequest("Message too old".to_string()));
        }
    }

    Ok(())
}

fn get_header(headers: &HeaderMap, name: &str) -> AppResult<String> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::BadRequest(format!("Missing header: {}", name)))
}

// ============================================================================
// Notification Handlers
// ============================================================================

async fn handle_notification(state: &Arc<AppState>, payload: &EventSubPayload) -> AppResult<()> {
    // Update subscription status to 'enabled' if we receive a notification
    // (this confirms the subscription is active)
    if let Ok(Some(db_sub)) =
        crate::db::EventSubSubscriptionRepository::find_by_twitch_subscription_id(
            &state.db,
            &payload.subscription.id,
        )
        .await
    {
        if db_sub.status != "enabled" {
            tracing::info!(
                "Received notification for subscription with status '{}', updating to 'enabled': subscription_id={}, type={}",
                db_sub.status,
                payload.subscription.id,
                payload.subscription.subscription_type
            );
            let _ = crate::db::EventSubSubscriptionRepository::update_status(
                &state.db,
                &payload.subscription.id,
                "enabled",
            )
            .await;
        }
    }

    let event = payload
        .event
        .as_ref()
        .ok_or_else(|| AppError::BadRequest("Missing event data".to_string()))?;

    match payload.subscription.subscription_type.as_str() {
        SUB_TYPE_STREAM_ONLINE => {
            let event: StreamOnlineEvent = serde_json::from_value(event.clone())
                .map_err(|e| AppError::BadRequest(format!("Invalid event data: {}", e)))?;

            handle_stream_online(state, event).await?;
        }
        SUB_TYPE_STREAM_OFFLINE => {
            let event: StreamOfflineEvent = serde_json::from_value(event.clone())
                .map_err(|e| AppError::BadRequest(format!("Invalid event data: {}", e)))?;

            handle_stream_offline(state, event).await?;
        }
        SUB_TYPE_CHANNEL_UPDATE => {
            let event: ChannelUpdateEvent = serde_json::from_value(event.clone())
                .map_err(|e| AppError::BadRequest(format!("Invalid event data: {}", e)))?;

            handle_channel_update(state, event).await?;
        }
        SUB_TYPE_CHANNEL_POINTS_REDEMPTION => {
            let event: ChannelPointsRedemptionEvent = serde_json::from_value(event.clone())
                .map_err(|e| AppError::BadRequest(format!("Invalid event data: {}", e)))?;

            handle_channel_points_redemption(state, event).await?;
        }
        _ => {
            tracing::debug!(
                "Unhandled subscription type: {}",
                payload.subscription.subscription_type
            );
        }
    }

    Ok(())
}

async fn handle_stream_online(state: &Arc<AppState>, event: StreamOnlineEvent) -> AppResult<()> {
    tracing::info!(
        "Stream online event received: broadcaster={} (twitch_id={})",
        event.broadcaster_user_name,
        event.broadcaster_user_id
    );

    // Update cache
    {
        let mut cache = STREAM_STATE_CACHE.write().await;
        cache.insert(
            event.broadcaster_user_id.clone(),
            StreamState {
                is_live: true,
                title: String::new(),
                category_id: String::new(),
            },
        );
    }

    // Find user by Twitch ID
    let user =
        match UserRepository::find_by_twitch_id(&state.db, &event.broadcaster_user_id).await? {
            Some(u) => {
                tracing::info!(
                    "User found for broadcaster {}: user_id={}, twitch_login={}",
                    event.broadcaster_user_id,
                    u.id,
                    u.twitch_login
                );
                u
            }
            None => {
                tracing::warn!(
                    "No user found for broadcaster: {} (twitch_id={}). Event will be ignored.",
                    event.broadcaster_user_name,
                    event.broadcaster_user_id
                );
                return Ok(());
            }
        };

    // Get stream info from Twitch API for additional details
    let stream = state
        .twitch
        .get_stream(&user.twitch_access_token, &event.broadcaster_user_id)
        .await?;

    let (title, category, thumbnail) = if let Some(s) = stream {
        tracing::debug!(
            "Stream details retrieved: title='{}', category='{}'",
            s.title,
            s.game_name
        );
        (s.title, s.game_name, Some(s.thumbnail_url))
    } else {
        tracing::warn!(
            "Could not retrieve stream details for broadcaster {}, using defaults",
            event.broadcaster_user_id
        );
        ("Stream started!".to_string(), "Unknown".to_string(), None)
    };

    // Send notifications
    let notification_service = NotificationService::new(state);

    let broadcaster_name = event.broadcaster_user_name.clone();
    let data = StreamOnlineData {
        streamer_name: broadcaster_name.clone(),
        streamer_avatar: if user.twitch_profile_image_url.is_empty() {
            None
        } else {
            Some(user.twitch_profile_image_url.clone())
        },
        title,
        category,
        thumbnail_url: thumbnail,
    };

    tracing::info!(
        "Attempting to send stream online notifications for user {} (broadcaster: {})",
        user.id,
        broadcaster_name
    );

    let results = notification_service
        .send_notification(&user.id, NotificationContent::StreamOnline(&data))
        .await?;

    let successful = results.iter().filter(|r| r.success).count();
    let failed = results.len() - successful;

    tracing::info!(
        "Stream online notification results for user {}: total={}, successful={}, failed={}",
        user.id,
        results.len(),
        successful,
        failed
    );

    if results.is_empty() {
        tracing::warn!(
            "No notifications were sent for user {} (broadcaster: {}). Check integration settings.",
            user.id,
            broadcaster_name
        );
    } else {
        for result in &results {
            if result.success {
                tracing::info!(
                    "Notification sent successfully: destination_type={}, destination_id={}",
                    result.destination_type,
                    result.destination_id
                );
            } else {
                tracing::warn!(
                    "Notification failed: destination_type={}, destination_id={}, error={:?}",
                    result.destination_type,
                    result.destination_id,
                    result.error
                );
            }
        }
    }

    Ok(())
}

async fn handle_stream_offline(state: &Arc<AppState>, event: StreamOfflineEvent) -> AppResult<()> {
    tracing::info!(
        "Stream offline: {} ({})",
        event.broadcaster_user_name,
        event.broadcaster_user_id
    );

    // Update cache
    {
        let mut cache = STREAM_STATE_CACHE.write().await;
        cache.remove(&event.broadcaster_user_id);
    }

    // Find user by Twitch ID
    let user =
        match UserRepository::find_by_twitch_id(&state.db, &event.broadcaster_user_id).await? {
            Some(u) => u,
            None => {
                tracing::debug!(
                    "No user found for broadcaster: {}",
                    event.broadcaster_user_id
                );
                return Ok(());
            }
        };

    // Send offline notifications (if the user has integrations enabled)
    let notification_service = NotificationService::new(state);

    let data = StreamOfflineData {
        streamer_name: event.broadcaster_user_name,
    };

    let results = notification_service
        .send_notification(&user.id, NotificationContent::StreamOffline(&data))
        .await?;

    tracing::info!(
        "Sent {} stream offline notifications for user {}",
        results.len(),
        user.id
    );

    Ok(())
}

async fn handle_channel_update(state: &Arc<AppState>, event: ChannelUpdateEvent) -> AppResult<()> {
    tracing::info!(
        "Channel update: {} - title='{}', category='{}'",
        event.broadcaster_user_name,
        event.title,
        event.category_name
    );

    // Check what changed
    let (title_changed, category_changed) = {
        let mut cache = STREAM_STATE_CACHE.write().await;

        // Extract previous state values before mutating the cache
        let (prev_is_live, prev_title, prev_category_id) = cache
            .get(&event.broadcaster_user_id)
            .map(|state| {
                (
                    state.is_live,
                    state.title.clone(),
                    state.category_id.clone(),
                )
            })
            .unwrap_or((false, String::new(), String::new()));

        let title_changed = !prev_title.is_empty() && prev_title != event.title;
        let category_changed =
            !prev_category_id.is_empty() && prev_category_id != event.category_id;

        // Now we can mutate the cache
        cache.insert(
            event.broadcaster_user_id.clone(),
            StreamState {
                is_live: prev_is_live,
                title: event.title.clone(),
                category_id: event.category_id.clone(),
            },
        );

        (title_changed, category_changed)
    };

    if !title_changed && !category_changed {
        return Ok(());
    }

    // Find user by Twitch ID
    let user =
        match UserRepository::find_by_twitch_id(&state.db, &event.broadcaster_user_id).await? {
            Some(u) => u,
            None => {
                tracing::debug!(
                    "No user found for broadcaster: {}",
                    event.broadcaster_user_id
                );
                return Ok(());
            }
        };

    let notification_service = NotificationService::new(state);

    // Send title change notification
    if title_changed {
        let data = TitleChangeData {
            streamer_name: event.broadcaster_user_name.clone(),
            new_title: event.title.clone(),
        };

        let results = notification_service
            .send_notification(&user.id, NotificationContent::TitleChange(&data))
            .await?;

        tracing::info!(
            "Sent {} title change notifications for user {}",
            results.len(),
            user.id
        );
    }

    // Send category change notification
    if category_changed {
        let data = CategoryChangeData {
            streamer_name: event.broadcaster_user_name,
            new_category: event.category_name,
        };

        let results = notification_service
            .send_notification(&user.id, NotificationContent::CategoryChange(&data))
            .await?;

        tracing::info!(
            "Sent {} category change notifications for user {}",
            results.len(),
            user.id
        );
    }

    Ok(())
}

async fn handle_channel_points_redemption(
    state: &Arc<AppState>,
    event: ChannelPointsRedemptionEvent,
) -> AppResult<()> {
    tracing::info!(
        "Channel points redemption: {} redeemed '{}' for {} points on {}",
        event.user_name,
        event.reward.title,
        event.reward.cost,
        event.broadcaster_user_name
    );

    // Find user by Twitch ID
    let user =
        match UserRepository::find_by_twitch_id(&state.db, &event.broadcaster_user_id).await? {
            Some(u) => u,
            None => {
                tracing::debug!(
                    "No user found for broadcaster: {}",
                    event.broadcaster_user_id
                );
                return Ok(());
            }
        };

    let notification_service = NotificationService::new(state);

    let data = RewardRedemptionData {
        redeemer_name: event.user_name,
        reward_name: event.reward.title,
        reward_cost: event.reward.cost,
        user_input: event.user_input,
        broadcaster_name: event.broadcaster_user_name,
    };

    let results = notification_service
        .send_notification(&user.id, NotificationContent::RewardRedemption(&data))
        .await?;

    tracing::info!(
        "Sent {} reward redemption notifications for user {}",
        results.len(),
        user.id
    );

    // Send chat message if enabled
    let settings = NotificationSettingsRepository::get_or_create(&state.db, &user.id).await?;
    if settings.notify_reward_redemption {
        // Normalize placeholders (convert {{...}} -> {...})
        let template = normalize_placeholders(&settings.reward_redemption_message);
        let message = template
            .replace("{user}", &data.redeemer_name)
            .replace("{reward}", &data.reward_name)
            .replace("{cost}", &data.reward_cost.to_string());

        // Check if token is expired or about to expire (within 60 seconds)
        let token_expires_at =
            chrono::DateTime::<Utc>::from_naive_utc_and_offset(user.twitch_token_expires_at, Utc);
        let mut access_token = user.twitch_access_token.clone();
        let mut refresh_token = user.twitch_refresh_token.clone();

        if token_expires_at - Duration::seconds(60) <= Utc::now() {
            // Token expired or about to expire, refresh it
            tracing::debug!(
                "Twitch token expired or about to expire, refreshing for user {}",
                user.id
            );
            let token_response = state.twitch.refresh_token(&refresh_token).await?;
            let new_expires_at = crate::services::twitch::TwitchService::calculate_token_expiry(
                token_response.expires_in,
            );

            // Update stored tokens
            UserRepository::update_tokens(
                &state.db,
                &user.id,
                &token_response.access_token,
                &token_response.refresh_token,
                new_expires_at.naive_utc(),
            )
            .await?;

            access_token = token_response.access_token;
            refresh_token = token_response.refresh_token;
        }

        // Send chat message
        match state
            .twitch
            .send_chat_message(&access_token, &user.twitch_id, &user.twitch_id, &message)
            .await
        {
            Ok(result) => {
                if result.is_sent {
                    tracing::info!(
                        "Sent reward redemption chat message to channel {} for user {}",
                        user.twitch_id,
                        user.id
                    );
                } else {
                    tracing::warn!(
                        "Failed to send reward redemption chat message to channel {} for user {}: {:?}",
                        user.twitch_id,
                        user.id,
                        result.drop_reason
                    );
                }
            }
            Err(err) => {
                // If unauthorized, try refreshing token once more
                if let AppError::TwitchApi(ref msg) = err {
                    if msg.contains("401") || msg.contains("Unauthorized") {
                        tracing::debug!(
                            "Unauthorized when sending chat message, refreshing token for user {}",
                            user.id
                        );
                        let token_response = state.twitch.refresh_token(&refresh_token).await?;
                        let new_expires_at =
                            crate::services::twitch::TwitchService::calculate_token_expiry(
                                token_response.expires_in,
                            );

                        UserRepository::update_tokens(
                            &state.db,
                            &user.id,
                            &token_response.access_token,
                            &token_response.refresh_token,
                            new_expires_at.naive_utc(),
                        )
                        .await?;

                        // Retry once with new token
                        match state
                            .twitch
                            .send_chat_message(
                                &token_response.access_token,
                                &user.twitch_id,
                                &user.twitch_id,
                                &message,
                            )
                            .await
                        {
                            Ok(result) => {
                                if result.is_sent {
                                    tracing::info!(
                                        "Sent reward redemption chat message to channel {} for user {} (after token refresh)",
                                        user.twitch_id,
                                        user.id
                                    );
                                } else {
                                    tracing::warn!(
                                        "Failed to send reward redemption chat message to channel {} for user {} after token refresh: {:?}",
                                        user.twitch_id,
                                        user.id,
                                        result.drop_reason
                                    );
                                }
                            }
                            Err(retry_err) => {
                                tracing::error!(
                                    "Failed to send reward redemption chat message to channel {} for user {} after token refresh: {:?}",
                                    user.twitch_id,
                                    user.id,
                                    retry_err
                                );
                            }
                        }
                    } else {
                        tracing::error!(
                            "Failed to send reward redemption chat message to channel {} for user {}: {:?}",
                            user.twitch_id,
                            user.id,
                            err
                        );
                    }
                } else {
                    tracing::error!(
                        "Failed to send reward redemption chat message to channel {} for user {}: {:?}",
                        user.twitch_id,
                        user.id,
                        err
                    );
                }
            }
        }
    }

    Ok(())
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
