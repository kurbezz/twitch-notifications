use std::collections::HashMap;
use std::sync::Arc;

use axum::http::HeaderMap;
use chrono::{Duration, Utc};
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;
use tokio::sync::RwLock;

use crate::db::{NotificationSettingsRepository, UserRepository};
use crate::error::{AppError, AppResult};
use crate::services::notifications::{
    CategoryChangeData, NotificationContent, NotificationService, RewardRedemptionData,
    StreamOfflineData, StreamOnlineData, TitleChangeData,
};
use crate::AppState;

type HmacSha256 = Hmac<Sha256>;

const TWITCH_MESSAGE_ID_HEADER: &str = "twitch-eventsub-message-id";
const TWITCH_MESSAGE_TIMESTAMP_HEADER: &str = "twitch-eventsub-message-timestamp";
const TWITCH_MESSAGE_SIGNATURE_HEADER: &str = "twitch-eventsub-message-signature";
const TWITCH_MESSAGE_TYPE_HEADER: &str = "twitch-eventsub-message-type";

const SUB_TYPE_STREAM_ONLINE: &str = "stream.online";
const SUB_TYPE_STREAM_OFFLINE: &str = "stream.offline";
const SUB_TYPE_CHANNEL_UPDATE: &str = "channel.update";
const SUB_TYPE_CHANNEL_POINTS_REDEMPTION: &str =
    "channel.channel_points_custom_reward_redemption.add";

#[derive(Debug, Clone)]
pub struct StreamState {
    pub is_live: bool,
    pub cached_at: chrono::DateTime<Utc>,
}

lazy_static::lazy_static! {
    static ref STREAM_STATE_CACHE: RwLock<HashMap<String, StreamState>> = RwLock::new(HashMap::new());
    // Separate cache for tracking title/category changes (not part of stream status)
    static ref CHANNEL_INFO_CACHE: RwLock<HashMap<String, (String, String)>> = RwLock::new(HashMap::new());
}

#[derive(Debug, Deserialize)]
pub struct EventSubSubscription {
    pub id: String,
    #[serde(rename = "type")]
    pub subscription_type: String,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct EventSubPayload {
    pub subscription: EventSubSubscription,
    pub challenge: Option<String>,
    pub event: Option<serde_json::Value>,
}

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

pub struct WebhookService;

impl WebhookService {
    /// Extract required headers from request
    pub fn extract_headers(headers: &HeaderMap) -> AppResult<(String, String, String, String)> {
        let message_id = Self::get_header(headers, TWITCH_MESSAGE_ID_HEADER)?;
        let timestamp = Self::get_header(headers, TWITCH_MESSAGE_TIMESTAMP_HEADER)?;
        let signature = Self::get_header(headers, TWITCH_MESSAGE_SIGNATURE_HEADER)?;
        let message_type = Self::get_header(headers, TWITCH_MESSAGE_TYPE_HEADER)?;
        Ok((message_id, timestamp, signature, message_type))
    }

    /// Verify webhook signature
    pub fn verify_signature(
        state: &Arc<AppState>,
        message_id: &str,
        timestamp: &str,
        body: &[u8],
        signature: &str,
    ) -> AppResult<()> {
        let secret = &state.config.jwt.secret;

        let mut message = Vec::new();
        message.extend_from_slice(message_id.as_bytes());
        message.extend_from_slice(timestamp.as_bytes());
        message.extend_from_slice(body);

        let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
            .map_err(|_| AppError::Internal(anyhow::anyhow!("Failed to create HMAC")))?;

        mac.update(&message);

        let expected_sig = if let Some(hex_sig) = signature.strip_prefix("sha256=") {
            hex::decode(hex_sig)
                .map_err(|_| AppError::BadRequest("Invalid signature format".to_string()))?
        } else {
            return Err(AppError::BadRequest("Invalid signature format".to_string()));
        };

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

    /// Handle webhook verification challenge
    pub async fn handle_verification(
        state: &Arc<AppState>,
        payload: &EventSubPayload,
    ) -> AppResult<String> {
        let challenge = payload
            .challenge
            .clone()
            .ok_or_else(|| AppError::BadRequest("Missing challenge".to_string()))?;

        // Try to update subscription status in DB
        if let Ok(Some(_)) =
            crate::db::EventSubSubscriptionRepository::find_by_twitch_subscription_id(
                &state.db,
                &payload.subscription.id,
            )
            .await
        {
            if let Ok(updated) = crate::db::EventSubSubscriptionRepository::update_status(
                &state.db,
                &payload.subscription.id,
                "enabled",
            )
            .await
            {
                tracing::info!(
                    "Updated subscription status to 'enabled' after verification: subscription_id={}, type={}",
                    updated.twitch_subscription_id,
                    updated.subscription_type
                );
            }
        }

        Ok(challenge)
    }

    /// Handle webhook notification
    pub async fn handle_notification(
        state: &Arc<AppState>,
        payload: &EventSubPayload,
    ) -> AppResult<()> {
        // Update subscription status
        if let Ok(Some(db_sub)) =
            crate::db::EventSubSubscriptionRepository::find_by_twitch_subscription_id(
                &state.db,
                &payload.subscription.id,
            )
            .await
        {
            if db_sub.status != "enabled" {
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
                Self::handle_stream_online(state, event).await?;
            }
            SUB_TYPE_STREAM_OFFLINE => {
                let event: StreamOfflineEvent = serde_json::from_value(event.clone())
                    .map_err(|e| AppError::BadRequest(format!("Invalid event data: {}", e)))?;
                Self::handle_stream_offline(state, event).await?;
            }
            SUB_TYPE_CHANNEL_UPDATE => {
                let event: ChannelUpdateEvent = serde_json::from_value(event.clone())
                    .map_err(|e| AppError::BadRequest(format!("Invalid event data: {}", e)))?;
                Self::handle_channel_update(state, event).await?;
            }
            SUB_TYPE_CHANNEL_POINTS_REDEMPTION => {
                let event: ChannelPointsRedemptionEvent = serde_json::from_value(event.clone())
                    .map_err(|e| AppError::BadRequest(format!("Invalid event data: {}", e)))?;
                Self::handle_channel_points_redemption(state, event).await?;
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

    /// Check if stream is online, using cache with 1 minute TTL
    /// If cache is empty or expired, makes API request to check status
    async fn check_stream_status(
        state: &Arc<AppState>,
        broadcaster_id: &str,
        user: &crate::db::User,
    ) -> AppResult<bool> {
        const CACHE_TTL_SECONDS: i64 = 60;

        // Check cache first
        {
            let cache = STREAM_STATE_CACHE.read().await;
            if let Some(stream_state) = cache.get(broadcaster_id) {
                let age = Utc::now() - stream_state.cached_at;
                if age.num_seconds() <= CACHE_TTL_SECONDS {
                    // Cache is valid, return cached status
                    return Ok(stream_state.is_live);
                }
                // Cache expired, fall through to API check
            }
            // No cache entry or expired, fall through to API check
        }

        // Cache is empty or expired, check via API
        let mut access_token = user.twitch_access_token.clone();
        let mut refresh_token = user.twitch_refresh_token.clone();

        // Check if token needs refresh
        let token_expires_at =
            chrono::DateTime::<Utc>::from_naive_utc_and_offset(user.twitch_token_expires_at, Utc);
        if token_expires_at - Duration::seconds(60) <= Utc::now() {
            if let Ok((new_access, new_refresh)) =
                Self::refresh_token_helper(state, &user.id, &refresh_token).await
            {
                access_token = new_access;
                refresh_token = new_refresh;
            }
        }

        // Try to get stream info
        let mut stream_result = state.twitch.get_stream(&access_token, broadcaster_id).await;

        // Retry with refreshed token if unauthorized
        if let Err(AppError::TwitchApi(ref msg)) = stream_result {
            if msg.contains("401") || msg.contains("Unauthorized") {
                if let Ok((new_access, _)) =
                    Self::refresh_token_helper(state, &user.id, &refresh_token).await
                {
                    access_token = new_access;
                    stream_result = state.twitch.get_stream(&access_token, broadcaster_id).await;
                }
            }
        }

        // Update cache based on API result
        let is_live = stream_result.is_ok() && stream_result.as_ref().unwrap().is_some();
        let mut cache = STREAM_STATE_CACHE.write().await;

        if is_live {
            // Stream is online, update cache
            cache.insert(
                broadcaster_id.to_string(),
                StreamState {
                    is_live: true,
                    cached_at: Utc::now(),
                },
            );
        } else {
            // Stream is offline, remove from cache
            cache.remove(broadcaster_id);
        }

        Ok(is_live)
    }

    async fn handle_stream_online(
        state: &Arc<AppState>,
        event: StreamOnlineEvent,
    ) -> AppResult<()> {
        // Update cache
        {
            let mut cache = STREAM_STATE_CACHE.write().await;
            cache.insert(
                event.broadcaster_user_id.clone(),
                StreamState {
                    is_live: true,
                    cached_at: Utc::now(),
                },
            );
        }

        // Find user
        let user =
            match UserRepository::find_by_twitch_id(&state.db, &event.broadcaster_user_id).await? {
                Some(u) => u,
                None => {
                    tracing::warn!(
                        "No user found for broadcaster: {} (twitch_id={})",
                        event.broadcaster_user_name,
                        event.broadcaster_user_id
                    );
                    return Ok(());
                }
            };

        // Get stream info (with token refresh logic)
        let (title, category, thumbnail) =
            Self::get_stream_info(state, &user, &event.broadcaster_user_id).await;

        // Send notifications
        let notification_service = NotificationService::new(state);
        let data = StreamOnlineData {
            streamer_name: event.broadcaster_user_name.clone(),
            streamer_avatar: if user.twitch_profile_image_url.is_empty() {
                None
            } else {
                Some(user.twitch_profile_image_url.clone())
            },
            title,
            category,
            thumbnail_url: thumbnail,
        };

        let results = notification_service
            .send_notification(&user.id, NotificationContent::StreamOnline(&data))
            .await?;

        let successful = results.iter().filter(|r| r.success).count();
        tracing::info!(
            "Stream online notification results for user {}: total={}, successful={}, failed={}",
            user.id,
            results.len(),
            successful,
            results.len() - successful
        );

        Ok(())
    }

    async fn handle_stream_offline(
        state: &Arc<AppState>,
        event: StreamOfflineEvent,
    ) -> AppResult<()> {
        // Update cache
        {
            let mut cache = STREAM_STATE_CACHE.write().await;
            cache.remove(&event.broadcaster_user_id);
        }

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
        let data = StreamOfflineData {
            streamer_name: event.broadcaster_user_name,
        };

        notification_service
            .send_notification(&user.id, NotificationContent::StreamOffline(&data))
            .await?;

        Ok(())
    }

    async fn handle_channel_update(
        state: &Arc<AppState>,
        event: ChannelUpdateEvent,
    ) -> AppResult<()> {
        tracing::info!(
            "Processing channel update: broadcaster={}, title={}, category={}",
            event.broadcaster_user_id,
            event.title,
            event.category_name
        );

        // Check what changed using separate channel info cache
        let (title_changed, category_changed) = {
            let mut cache = CHANNEL_INFO_CACHE.write().await;
            let (prev_title, prev_category_id) = cache
                .get(&event.broadcaster_user_id)
                .cloned()
                .unwrap_or((String::new(), String::new()));

            let title_changed = !prev_title.is_empty() && prev_title != event.title;
            let category_changed =
                !prev_category_id.is_empty() && prev_category_id != event.category_id;

            // Update channel info cache
            cache.insert(
                event.broadcaster_user_id.clone(),
                (event.title.clone(), event.category_id.clone()),
            );

            (title_changed, category_changed)
        };

        if !title_changed && !category_changed {
            tracing::debug!(
                "No changes detected for broadcaster {}",
                event.broadcaster_user_id
            );
            return Ok(());
        }

        // Find user first to check stream status
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

        // Check if stream is online using check_stream_status
        let is_live =
            match Self::check_stream_status(state, &event.broadcaster_user_id, &user).await {
                Ok(live) => live,
                Err(e) => {
                    tracing::warn!(
                        "Failed to check stream status for {}: {}",
                        event.broadcaster_user_id,
                        e
                    );
                    // If check fails, assume offline to be safe
                    false
                }
            };

        if !is_live {
            tracing::info!(
                "Skipping title/category change notifications for {}: stream is offline (title_changed={}, category_changed={})",
                event.broadcaster_user_id,
                title_changed,
                category_changed
            );
            return Ok(());
        }

        let notification_service = NotificationService::new(state);

        if title_changed {
            let data = TitleChangeData {
                streamer_name: event.broadcaster_user_name.clone(),
                new_title: event.title.clone(),
            };
            notification_service
                .send_notification(&user.id, NotificationContent::TitleChange(&data))
                .await?;
        }

        if category_changed {
            let data = CategoryChangeData {
                streamer_name: event.broadcaster_user_name,
                new_category: event.category_name,
            };
            notification_service
                .send_notification(&user.id, NotificationContent::CategoryChange(&data))
                .await?;
        }

        Ok(())
    }

    async fn handle_channel_points_redemption(
        state: &Arc<AppState>,
        event: ChannelPointsRedemptionEvent,
    ) -> AppResult<()> {
        tracing::info!(
            "Processing reward redemption: broadcaster={}, redeemer={}, reward={}",
            event.broadcaster_user_id,
            event.user_name,
            event.reward.title
        );

        // Find user first to check stream status
        let user =
            match UserRepository::find_by_twitch_id(&state.db, &event.broadcaster_user_id).await? {
                Some(u) => u,
                None => {
                    tracing::warn!(
                        "No user found for broadcaster: {}",
                        event.broadcaster_user_id
                    );
                    return Ok(());
                }
            };

        // Check if stream is online using check_stream_status
        let is_live =
            match Self::check_stream_status(state, &event.broadcaster_user_id, &user).await {
                Ok(live) => live,
                Err(e) => {
                    tracing::warn!(
                        "Failed to check stream status for {}: {}",
                        event.broadcaster_user_id,
                        e
                    );
                    // If check fails, assume offline to be safe
                    false
                }
            };

        if !is_live {
            tracing::info!(
                "Skipping reward redemption notification for {}: stream is offline",
                event.broadcaster_user_id
            );
            return Ok(());
        }

        let notification_service = NotificationService::new(state);
        let data = RewardRedemptionData {
            redeemer_name: event.user_name,
            reward_name: event.reward.title,
            reward_cost: event.reward.cost,
            user_input: event.user_input,
            broadcaster_name: event.broadcaster_user_name,
        };

        tracing::info!(
            "Sending reward redemption notification for user {} (broadcaster={})",
            user.id,
            event.broadcaster_user_id
        );

        // Send to integrations
        notification_service
            .send_notification(&user.id, NotificationContent::RewardRedemption(&data))
            .await?;

        // Send chat message if enabled
        let settings = NotificationSettingsRepository::get_or_create(&state.db, &user.id).await?;
        if settings.notify_reward_redemption {
            let message = settings
                .reward_redemption_message
                .replace("{user}", &data.redeemer_name)
                .replace("{reward}", &data.reward_name)
                .replace("{cost}", &data.reward_cost.to_string());

            Self::send_chat_message_with_retry(state, &user, &message).await?;
        }

        Ok(())
    }

    async fn get_stream_info(
        state: &Arc<AppState>,
        user: &crate::db::User,
        broadcaster_id: &str,
    ) -> (String, String, Option<String>) {
        let mut access_token = user.twitch_access_token.clone();
        let mut refresh_token = user.twitch_refresh_token.clone();

        // Check if token needs refresh
        let token_expires_at =
            chrono::DateTime::<Utc>::from_naive_utc_and_offset(user.twitch_token_expires_at, Utc);
        if token_expires_at - Duration::seconds(60) <= Utc::now() {
            if let Ok((new_access, new_refresh)) =
                Self::refresh_token_helper(state, &user.id, &refresh_token).await
            {
                access_token = new_access;
                refresh_token = new_refresh;
            }
        }

        // Try to get stream info
        let mut stream_result = state.twitch.get_stream(&access_token, broadcaster_id).await;

        // Retry with refreshed token if unauthorized
        if let Err(AppError::TwitchApi(ref msg)) = stream_result {
            if msg.contains("401") || msg.contains("Unauthorized") {
                if let Ok((new_access, _)) =
                    Self::refresh_token_helper(state, &user.id, &refresh_token).await
                {
                    access_token = new_access;
                    stream_result = state.twitch.get_stream(&access_token, broadcaster_id).await;
                }
            }
        }

        match stream_result {
            Ok(Some(s)) => (s.title, s.game_name, Some(s.thumbnail_url)),
            _ => ("Stream started!".to_string(), "Unknown".to_string(), None),
        }
    }

    async fn send_chat_message_with_retry(
        state: &Arc<AppState>,
        user: &crate::db::User,
        message: &str,
    ) -> AppResult<()> {
        let mut access_token = user.twitch_access_token.clone();
        let mut refresh_token = user.twitch_refresh_token.clone();

        // Check if token needs refresh
        let token_expires_at =
            chrono::DateTime::<Utc>::from_naive_utc_and_offset(user.twitch_token_expires_at, Utc);
        if token_expires_at - Duration::seconds(60) <= Utc::now() {
            if let Ok((new_access, new_refresh)) =
                Self::refresh_token_helper(state, &user.id, &refresh_token).await
            {
                access_token = new_access;
                refresh_token = new_refresh;
            }
        }

        // Try to send message
        let result = state
            .twitch
            .send_chat_message(&access_token, &user.twitch_id, &user.twitch_id, message)
            .await;

        // Retry with refreshed token if unauthorized
        if let Err(AppError::TwitchApi(ref msg)) = result {
            if msg.contains("401") || msg.contains("Unauthorized") {
                if let Ok((new_access, _)) =
                    Self::refresh_token_helper(state, &user.id, &refresh_token).await
                {
                    let _ = state
                        .twitch
                        .send_chat_message(&new_access, &user.twitch_id, &user.twitch_id, message)
                        .await;
                }
            }
        }

        Ok(())
    }

    async fn refresh_token_helper(
        state: &Arc<AppState>,
        user_id: &str,
        refresh_token: &str,
    ) -> AppResult<(String, String)> {
        let token_response = state.twitch.refresh_token(refresh_token).await?;
        let new_expires_at = crate::services::twitch::TwitchService::calculate_token_expiry(
            token_response.expires_in,
        );

        UserRepository::update_tokens(
            &state.db,
            user_id,
            &token_response.access_token,
            &token_response.refresh_token,
            new_expires_at.naive_utc(),
        )
        .await?;

        Ok((token_response.access_token, token_response.refresh_token))
    }

    fn get_header(headers: &HeaderMap, name: &str) -> AppResult<String> {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .ok_or_else(|| AppError::BadRequest(format!("Missing header: {}", name)))
    }
}
