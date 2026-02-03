use chrono::{Duration, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration as StdDuration;
use tokio::sync::RwLock;

use crate::config::Config;
use crate::error::{AppError, AppResult};

const TWITCH_AUTH_URL: &str = "https://id.twitch.tv/oauth2";
const TWITCH_API_URL: &str = "https://api.twitch.tv/helix";

#[derive(Debug, Clone)]
pub struct TwitchService {
    client: Client,
    client_id: String,
    client_secret: String,
    redirect_uri: String,
    webhook_url: String,
    app_access_token: Arc<RwLock<Option<AppAccessToken>>>,
}

#[derive(Debug, Clone)]
pub struct AppAccessToken {
    pub token: String,
    pub expires_at: chrono::DateTime<Utc>,
}

// ============================================================================
// OAuth Types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
    pub scope: Vec<String>,
    pub token_type: String,
}

#[derive(Debug, Deserialize)]
pub struct AppAccessTokenResponse {
    pub access_token: String,
    pub expires_in: i64,
}

// ============================================================================
// User Types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct TwitchUsersResponse {
    pub data: Vec<TwitchUser>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TwitchUser {
    pub id: String,
    pub login: String,
    pub display_name: String,
    pub email: Option<String>,
    pub profile_image_url: Option<String>,
    pub broadcaster_type: Option<String>,
    pub description: Option<String>,
    pub created_at: Option<String>,
}

// ============================================================================
// Stream Types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct StreamsResponse {
    pub data: Vec<Stream>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Stream {
    pub id: String,
    pub user_id: String,
    pub user_login: String,
    pub user_name: String,
    pub game_id: String,
    pub game_name: String,
    #[serde(rename = "type")]
    pub stream_type: String,
    pub title: String,
    pub viewer_count: i32,
    pub started_at: String,
    pub language: String,
    pub thumbnail_url: String,
    pub is_mature: bool,
}

// ============================================================================
// Channel Types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ChannelInfoResponse {
    pub data: Vec<ChannelInfo>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChannelInfo {
    pub broadcaster_id: String,
    pub broadcaster_login: String,
    pub broadcaster_name: String,
    pub broadcaster_language: String,
    pub game_id: String,
    pub game_name: String,
    pub title: String,
    pub delay: i32,
    pub tags: Vec<String>,
}

// ============================================================================
// EventSub Types
// ============================================================================

#[derive(Debug, Serialize)]
pub struct CreateEventSubRequest {
    #[serde(rename = "type")]
    pub subscription_type: String,
    pub version: String,
    pub condition: serde_json::Value,
    pub transport: EventSubTransport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSubTransport {
    pub method: String,
    pub callback: String,
    pub secret: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct EventSubResponse {
    pub data: Vec<EventSubSubscription>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EventSubSubscription {
    pub id: String,
    pub status: String,
    #[serde(rename = "type")]
    pub subscription_type: String,
    pub version: String,
    pub condition: serde_json::Value,
    pub created_at: String,
    pub transport: EventSubTransport,
    pub cost: i32,
}

#[derive(Debug, Deserialize)]
pub struct ListEventSubResponse {
    pub data: Vec<EventSubSubscription>,
}

// ============================================================================
// Channel Points Types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct CustomRewardsResponse {
    pub data: Vec<CustomReward>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CustomReward {
    pub id: String,
    pub broadcaster_id: String,
    pub broadcaster_login: String,
    pub broadcaster_name: String,
    pub title: String,
    pub prompt: Option<String>,
    pub cost: i32,
    pub is_enabled: bool,
    pub is_paused: bool,
    pub is_in_stock: bool,
    pub background_color: Option<String>,
    pub image: Option<RewardImage>,
    pub default_image: Option<RewardImage>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RewardImage {
    pub url_1x: String,
    pub url_2x: String,
    pub url_4x: String,
}

// ============================================================================
// Schedule Types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ScheduleResponse {
    pub data: ScheduleData,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScheduleData {
    pub segments: Vec<ScheduleSegment>,
    pub broadcaster_id: String,
    pub broadcaster_name: String,
    pub broadcaster_login: String,
    pub vacation: Option<Vacation>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScheduleSegment {
    pub id: String,
    pub start_time: String,
    pub end_time: Option<String>,
    pub title: String,
    pub canceled_until: Option<String>,
    pub category: Option<ScheduleCategory>,
    pub is_recurring: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScheduleCategory {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Vacation {
    pub start_time: String,
    pub end_time: String,
}

// ============================================================================
// Chat Types
// ============================================================================

#[derive(Debug, Serialize)]
pub struct SendChatMessageRequest {
    pub broadcaster_id: String,
    pub sender_id: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct SendChatMessageResponse {
    pub data: Vec<ChatMessageResult>,
}

#[derive(Debug, Deserialize)]
pub struct ChatMessageResult {
    pub message_id: String,
    pub is_sent: bool,
    pub drop_reason: Option<DropReason>,
}

#[derive(Debug, Deserialize)]
pub struct DropReason {
    pub code: String,
    pub message: String,
}

impl TwitchService {
    pub async fn new(config: &Config) -> AppResult<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| AppError::Internal(e.into()))?;

        let service = Self {
            client,
            client_id: config.twitch.client_id.clone(),
            client_secret: config.twitch.client_secret.clone(),
            redirect_uri: config.twitch.redirect_uri.clone(),
            webhook_url: config.server.webhook_url.clone(),
            app_access_token: Arc::new(RwLock::new(None)),
        };

        // Get app access token for EventSub subscriptions
        service.refresh_app_access_token().await?;

        // Spawn background refresher task to proactively refresh the app token
        // shortly before it expires. Uses exponential backoff on failures.
        {
            let refresher = service.clone();
            tokio::spawn(async move {
                refresher.start_app_access_token_refresher().await;
            });
        }

        Ok(service)
    }

    // ========================================================================
    // OAuth Methods
    // ========================================================================

    /// Generate the OAuth authorization URL
    pub fn get_auth_url(&self, state: &str, scopes: &[&str]) -> String {
        let scope = scopes.join(" ");
        format!(
            "{}/authorize?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}",
            TWITCH_AUTH_URL,
            self.client_id,
            urlencoding::encode(&self.redirect_uri),
            urlencoding::encode(&scope),
            urlencoding::encode(state)
        )
    }

    /// Exchange authorization code for tokens
    pub async fn exchange_code(&self, code: &str) -> AppResult<TokenResponse> {
        let response = self
            .send_with_backoff(|| {
                self.client
                    .post(format!("{}/token", TWITCH_AUTH_URL))
                    .form(&[
                        ("client_id", self.client_id.as_str()),
                        ("client_secret", self.client_secret.as_str()),
                        ("code", code),
                        ("grant_type", "authorization_code"),
                        ("redirect_uri", self.redirect_uri.as_str()),
                    ])
            })
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::TwitchApi(format!(
                "Failed to exchange code: {}",
                error_text
            )));
        }

        response
            .json::<TokenResponse>()
            .await
            .map_err(|e| AppError::TwitchApi(format!("Failed to parse token response: {}", e)))
    }

    /// Refresh user access token
    pub async fn refresh_token(&self, refresh_token: &str) -> AppResult<TokenResponse> {
        let response = self
            .send_with_backoff(|| {
                self.client
                    .post(format!("{}/token", TWITCH_AUTH_URL))
                    .form(&[
                        ("client_id", self.client_id.as_str()),
                        ("client_secret", self.client_secret.as_str()),
                        ("refresh_token", refresh_token),
                        ("grant_type", "refresh_token"),
                    ])
            })
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::TwitchApi(format!(
                "Failed to refresh token: {}",
                error_text
            )));
        }

        response
            .json::<TokenResponse>()
            .await
            .map_err(|e| AppError::TwitchApi(format!("Failed to parse token response: {}", e)))
    }

    /// Get app access token (for EventSub and other app-level operations)
    ///
    /// This will fetch a new app token from Twitch and store it (including
    /// its expiry) into the internal lock. The method takes `&self` and uses
    /// interior mutability so it can be called concurrently.
    pub async fn refresh_app_access_token(&self) -> AppResult<()> {
        let response = self
            .send_with_backoff(|| {
                self.client
                    .post(format!("{}/token", TWITCH_AUTH_URL))
                    .form(&[
                        ("client_id", self.client_id.as_str()),
                        ("client_secret", self.client_secret.as_str()),
                        ("grant_type", "client_credentials"),
                    ])
            })
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::TwitchApi(format!(
                "Failed to get app access token: {}",
                error_text
            )));
        }

        let token_response: AppAccessTokenResponse = response
            .json()
            .await
            .map_err(|e| AppError::TwitchApi(format!("Failed to parse token response: {}", e)))?;

        let expires_at = Utc::now() + Duration::seconds(token_response.expires_in);
        let token = AppAccessToken {
            token: token_response.access_token,
            expires_at,
        };

        let mut guard = self.app_access_token.write().await;
        *guard = Some(token);

        tracing::info!(
            "Refreshed Twitch app access token; expires at {}",
            expires_at
        );

        Ok(())
    }

    /// Get a valid app access token, refreshing it if it would expire soon.
    ///
    /// This method will return the current token if it's valid for at least
    /// `REFRESH_MARGIN_SECS` seconds, otherwise it triggers a refresh and
    /// returns the new token. This ensures callers get a token that is not
    /// about to expire immediately.
    pub async fn get_valid_app_access_token(&self) -> AppResult<String> {
        const REFRESH_MARGIN_SECS: i64 = 60; // refresh 60 seconds before expiry

        // Fast path: check under a read lock
        {
            let guard = self.app_access_token.read().await;
            if let Some(ref t) = *guard {
                if t.expires_at - Duration::seconds(REFRESH_MARGIN_SECS) > Utc::now() {
                    return Ok(t.token.clone());
                }
            }
        }

        // Otherwise, trigger a refresh and read again
        self.refresh_app_access_token().await?;

        let guard = self.app_access_token.read().await;
        if let Some(ref t) = *guard {
            Ok(t.token.clone())
        } else {
            Err(AppError::TwitchApi(
                "No app access token available".to_string(),
            ))
        }
    }

    /// Validate a token
    pub async fn validate_token(&self, access_token: &str) -> AppResult<bool> {
        let response = self
            .send_with_backoff(|| {
                self.client
                    .get(format!("{}/validate", TWITCH_AUTH_URL))
                    .header("Authorization", format!("OAuth {}", access_token))
            })
            .await?;

        Ok(response.status().is_success())
    }

    /// Revoke a token
    pub async fn revoke_token(&self, access_token: &str) -> AppResult<()> {
        let response = self
            .send_with_backoff(|| {
                self.client
                    .post(format!("{}/revoke", TWITCH_AUTH_URL))
                    .form(&[
                        ("client_id", self.client_id.as_str()),
                        ("token", access_token),
                    ])
            })
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::TwitchApi(format!(
                "Failed to revoke token: {}",
                error_text
            )));
        }

        Ok(())
    }

    // ========================================================================
    // User Methods
    // ========================================================================

    /// Get user info by access token
    pub async fn get_user(&self, access_token: &str) -> AppResult<TwitchUser> {
        let response = self
            .send_with_backoff(|| {
                self.client
                    .get(format!("{}/users", TWITCH_API_URL))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Client-Id", &self.client_id)
            })
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::TwitchApi(format!(
                "Failed to get user: {}",
                error_text
            )));
        }

        let users: TwitchUsersResponse = response
            .json()
            .await
            .map_err(|e| AppError::TwitchApi(format!("Failed to parse users response: {}", e)))?;

        users
            .data
            .into_iter()
            .next()
            .ok_or_else(|| AppError::TwitchApi("No user found".to_string()))
    }

    /// Get users by IDs
    pub async fn get_users_by_ids(
        &self,
        access_token: &str,
        user_ids: &[&str],
    ) -> AppResult<Vec<TwitchUser>> {
        let ids_param = user_ids
            .iter()
            .map(|id| format!("id={}", id))
            .collect::<Vec<_>>()
            .join("&");

        let response = self
            .send_with_backoff(|| {
                self.client
                    .get(format!("{}/users?{}", TWITCH_API_URL, ids_param))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Client-Id", &self.client_id)
            })
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::TwitchApi(format!(
                "Failed to get users: {}",
                error_text
            )));
        }

        let users: TwitchUsersResponse = response
            .json()
            .await
            .map_err(|e| AppError::TwitchApi(format!("Failed to parse users response: {}", e)))?;

        Ok(users.data)
    }

    /// Get users by logins
    pub async fn get_users_by_logins(
        &self,
        access_token: &str,
        logins: &[&str],
    ) -> AppResult<Vec<TwitchUser>> {
        let logins_param = logins
            .iter()
            .map(|login| format!("login={}", login))
            .collect::<Vec<_>>()
            .join("&");

        let response = self
            .send_with_backoff(|| {
                self.client
                    .get(format!("{}/users?{}", TWITCH_API_URL, logins_param))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Client-Id", &self.client_id)
            })
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::TwitchApi(format!(
                "Failed to get users: {}",
                error_text
            )));
        }

        let users: TwitchUsersResponse = response
            .json()
            .await
            .map_err(|e| AppError::TwitchApi(format!("Failed to parse users response: {}", e)))?;

        Ok(users.data)
    }

    // ========================================================================
    // Stream Methods
    // ========================================================================

    /// Get stream info by user ID
    pub async fn get_stream(&self, access_token: &str, user_id: &str) -> AppResult<Option<Stream>> {
        let response = self
            .send_with_backoff(|| {
                self.client
                    .get(format!("{}/streams?user_id={}", TWITCH_API_URL, user_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Client-Id", &self.client_id)
            })
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::TwitchApi(format!(
                "Failed to get stream: {}",
                error_text
            )));
        }

        let streams: StreamsResponse = response
            .json()
            .await
            .map_err(|e| AppError::TwitchApi(format!("Failed to parse streams response: {}", e)))?;

        Ok(streams.data.into_iter().next())
    }

    /// Get streams by user IDs
    pub async fn get_streams(
        &self,
        access_token: &str,
        user_ids: &[&str],
    ) -> AppResult<Vec<Stream>> {
        if user_ids.is_empty() {
            return Ok(vec![]);
        }

        let ids_param = user_ids
            .iter()
            .map(|id| format!("user_id={}", id))
            .collect::<Vec<_>>()
            .join("&");

        let response = self
            .send_with_backoff(|| {
                self.client
                    .get(format!("{}/streams?{}", TWITCH_API_URL, ids_param))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Client-Id", &self.client_id)
            })
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::TwitchApi(format!(
                "Failed to get streams: {}",
                error_text
            )));
        }

        let streams: StreamsResponse = response
            .json()
            .await
            .map_err(|e| AppError::TwitchApi(format!("Failed to parse streams response: {}", e)))?;

        Ok(streams.data)
    }

    // ========================================================================
    // Channel Methods
    // ========================================================================

    /// Get channel info
    pub async fn get_channel_info(
        &self,
        access_token: &str,
        broadcaster_id: &str,
    ) -> AppResult<Option<ChannelInfo>> {
        let response = self
            .send_with_backoff(|| {
                self.client
                    .get(format!(
                        "{}/channels?broadcaster_id={}",
                        TWITCH_API_URL, broadcaster_id
                    ))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Client-Id", &self.client_id)
            })
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::TwitchApi(format!(
                "Failed to get channel info: {}",
                error_text
            )));
        }

        let channels: ChannelInfoResponse = response.json().await.map_err(|e| {
            AppError::TwitchApi(format!("Failed to parse channel info response: {}", e))
        })?;

        Ok(channels.data.into_iter().next())
    }

    // ========================================================================
    // EventSub Methods
    // ========================================================================

    /// Create an EventSub subscription (with retries on transient errors)
    async fn send_with_backoff<F>(&self, make_request: F) -> AppResult<reqwest::Response>
    where
        F: Fn() -> reqwest::RequestBuilder,
    {
        const MAX_RETRIES: usize = 5;
        let mut backoff_secs: u64 = 1;
        let max_backoff_secs: u64 = 60;

        for attempt in 0..MAX_RETRIES {
            match (make_request)().send().await {
                Ok(resp) => {
                    // Retry on 429 (rate limit) or server errors (5xx)
                    if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS
                        || resp.status().is_server_error()
                    {
                        // Respect Retry-After header if present
                        let mut wait_secs = backoff_secs;
                        if let Some(h) = resp.headers().get("retry-after") {
                            if let Ok(s) = h.to_str() {
                                if let Ok(parsed) = s.parse::<u64>() {
                                    wait_secs = parsed;
                                }
                            }
                        }

                        tracing::warn!(
                            "Transient Twitch error (status: {}). Retrying in {}s (attempt {}/{})",
                            resp.status(),
                            wait_secs,
                            attempt + 1,
                            MAX_RETRIES
                        );

                        if attempt + 1 >= MAX_RETRIES {
                            let err_text = resp.text().await.unwrap_or_default();
                            return Err(AppError::TwitchApi(format!(
                                "Failed after {} attempts: {}",
                                attempt + 1,
                                err_text
                            )));
                        }

                        tokio::time::sleep(std::time::Duration::from_secs(wait_secs)).await;
                        backoff_secs = std::cmp::min(backoff_secs * 2, max_backoff_secs);
                        continue;
                    }

                    // Return response even for non-200 (caller will decide how to handle 401/404/etc.)
                    return Ok(resp);
                }
                Err(e) => {
                    // Network-level error -> retryable
                    if attempt + 1 >= MAX_RETRIES {
                        return Err(e.into());
                    }
                    tracing::warn!(
                        "HTTP request failed: {}. Retrying in {}s (attempt {}/{})",
                        e,
                        backoff_secs,
                        attempt + 1,
                        MAX_RETRIES
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(backoff_secs)).await;
                    backoff_secs = std::cmp::min(backoff_secs * 2, max_backoff_secs);
                    continue;
                }
            }
        }

        Err(AppError::TwitchApi(
            "Exceeded Twitch retry attempts".to_string(),
        ))
    }
    /// Helper that runs a request which requires an app access token.
    /// It will attempt the request once, and on 401 will refresh the app token and retry once more.
    /// Transient errors (429/5xx and network errors) are handled by `send_with_backoff`.
    async fn send_app_request_with_token<F>(&self, make_request: F) -> AppResult<reqwest::Response>
    where
        F: Fn(&str) -> reqwest::RequestBuilder,
    {
        let mut refreshed_token = false;
        for _attempt_round in 0..2 {
            // Acquire token (may error)
            let token = self.get_valid_app_access_token().await?;

            match self.send_with_backoff(|| make_request(&token)).await {
                Ok(response) => {
                    // Unauthorized -> refresh once and retry
                    if response.status() == reqwest::StatusCode::UNAUTHORIZED {
                        if !refreshed_token {
                            tracing::warn!(
                                "Unauthorized Twitch app request. Refreshing token and retrying."
                            );
                            self.refresh_app_access_token().await?;
                            refreshed_token = true;
                            continue;
                        } else {
                            let error_text = response.text().await.unwrap_or_default();
                            return Err(AppError::TwitchApi(format!(
                                "Unauthorized Twitch app request: {}",
                                error_text
                            )));
                        }
                    }

                    return Ok(response);
                }
                Err(e) => {
                    // send_with_backoff already handled transient retries; propagate final error
                    return Err(e);
                }
            }
        }

        Err(AppError::TwitchApi(
            "Failed to perform Twitch app request after retries".to_string(),
        ))
    }

    pub async fn create_eventsub_subscription(
        &self,
        subscription_type: &str,
        version: &str,
        condition: serde_json::Value,
        secret: &str,
    ) -> AppResult<EventSubSubscription> {
        let request = CreateEventSubRequest {
            subscription_type: subscription_type.to_string(),
            version: version.to_string(),
            condition,
            transport: EventSubTransport {
                method: "webhook".to_string(),
                callback: format!("{}/webhooks/twitch", self.webhook_url),
                secret: Some(secret.to_string()),
            },
        };

        // Use helper that handles app token acquisition and refresh on 401, while
        // `send_with_backoff` handles transient errors (429/5xx and network errors).
        let response = self
            .send_app_request_with_token(|token| {
                self.client
                    .post(format!("{}/eventsub/subscriptions", TWITCH_API_URL))
                    .header("Authorization", format!("Bearer {}", token))
                    .header("Client-Id", &self.client_id)
                    .json(&request)
            })
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::TwitchApi(format!(
                "Failed to create EventSub subscription: {}",
                error_text
            )));
        }

        let subscription: EventSubResponse = response.json().await.map_err(|e| {
            AppError::TwitchApi(format!("Failed to parse EventSub response: {}", e))
        })?;

        subscription
            .data
            .into_iter()
            .next()
            .ok_or_else(|| AppError::TwitchApi("No subscription created".to_string()))
    }

    /// Delete an EventSub subscription (with retries on transient errors)
    pub async fn delete_eventsub_subscription(&self, subscription_id: &str) -> AppResult<()> {
        let response = self
            .send_app_request_with_token(|token| {
                self.client
                    .delete(format!(
                        "{}/eventsub/subscriptions?id={}",
                        TWITCH_API_URL, subscription_id
                    ))
                    .header("Authorization", format!("Bearer {}", token))
                    .header("Client-Id", &self.client_id)
            })
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::TwitchApi(format!(
                "Failed to delete EventSub subscription: {}",
                error_text
            )));
        }

        Ok(())
    }

    /// List EventSub subscriptions (with retries on transient errors)
    pub async fn list_eventsub_subscriptions(&self) -> AppResult<Vec<EventSubSubscription>> {
        let response = self
            .send_app_request_with_token(|token| {
                self.client
                    .get(format!("{}/eventsub/subscriptions", TWITCH_API_URL))
                    .header("Authorization", format!("Bearer {}", token))
                    .header("Client-Id", &self.client_id)
            })
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::TwitchApi(format!(
                "Failed to list EventSub subscriptions: {}",
                error_text
            )));
        }

        let subscriptions: ListEventSubResponse = response.json().await.map_err(|e| {
            AppError::TwitchApi(format!("Failed to parse EventSub response: {}", e))
        })?;

        Ok(subscriptions.data)
    }

    /// Background task that refreshes the app access token before expiry.
    ///
    /// The refresher now listens for process shutdown (Ctrl-C and SIGTERM on
    /// unix) and will exit promptly when a shutdown is requested.
    async fn start_app_access_token_refresher(self) {
        const REFRESH_MARGIN_SECS: i64 = 60;
        let mut backoff_secs: u64 = 5;

        loop {
            // Compute how long to wait until we should refresh the token
            let wait_duration = {
                let guard = self.app_access_token.read().await;
                if let Some(ref t) = *guard {
                    let refresh_at = t.expires_at - Duration::seconds(REFRESH_MARGIN_SECS);
                    let now = Utc::now();
                    if refresh_at <= now {
                        StdDuration::from_secs(0)
                    } else {
                        let secs = (refresh_at - now).num_seconds();
                        StdDuration::from_secs(if secs > 0 { secs as u64 } else { 0 })
                    }
                } else {
                    // No token yet - try immediately
                    StdDuration::from_secs(0)
                }
            };

            if wait_duration.as_secs() > 0 {
                // Wait for either the computed duration or a shutdown signal.
                let ctrl_c = tokio::signal::ctrl_c();

                #[cfg(unix)]
                {
                    let mut term =
                        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                            .expect("Failed to bind SIGTERM");

                    tokio::select! {
                        _ = ctrl_c => {
                            tracing::info!("App access token refresher received shutdown signal (ctrl_c)");
                            return;
                        }
                        _ = term.recv() => {
                            tracing::info!("App access token refresher received SIGTERM");
                            return;
                        }
                        _ = tokio::time::sleep(wait_duration) => {}
                    }
                }

                #[cfg(not(unix))]
                {
                    tokio::select! {
                        _ = ctrl_c => {
                            tracing::info!("App access token refresher received shutdown signal (ctrl_c)");
                            return;
                        }
                        _ = tokio::time::sleep(wait_duration) => {}
                    }
                }
            }

            // Try refreshing with exponential backoff on failures.
            loop {
                match self.refresh_app_access_token().await {
                    Ok(_) => {
                        backoff_secs = 5;
                        break;
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to refresh app access token: {}. Retrying in {}s",
                            e,
                            backoff_secs
                        );

                        // Wait for either backoff duration or a shutdown signal.
                        let ctrl_c = tokio::signal::ctrl_c();

                        #[cfg(unix)]
                        {
                            let mut term = tokio::signal::unix::signal(
                                tokio::signal::unix::SignalKind::terminate(),
                            )
                            .expect("Failed to bind SIGTERM");

                            tokio::select! {
                                _ = ctrl_c => {
                                    tracing::info!("App access token refresher received shutdown signal (ctrl_c)");
                                    return;
                                }
                                _ = term.recv() => {
                                    tracing::info!("App access token refresher received SIGTERM");
                                    return;
                                }
                                _ = tokio::time::sleep(StdDuration::from_secs(backoff_secs)) => {}
                            }
                        }

                        #[cfg(not(unix))]
                        {
                            tokio::select! {
                                _ = ctrl_c => {
                                    tracing::info!("App access token refresher received shutdown signal (ctrl_c)");
                                    return;
                                }
                                _ = tokio::time::sleep(StdDuration::from_secs(backoff_secs)) => {}
                            }
                        }

                        backoff_secs = std::cmp::min(backoff_secs * 2, 600);
                    }
                }
            }
        }
    }

    /// Subscribe to stream online event
    pub async fn subscribe_stream_online(
        &self,
        broadcaster_id: &str,
        secret: &str,
    ) -> AppResult<EventSubSubscription> {
        self.create_eventsub_subscription(
            "stream.online",
            "1",
            serde_json::json!({
                "broadcaster_user_id": broadcaster_id
            }),
            secret,
        )
        .await
    }

    /// Subscribe to stream offline event
    pub async fn subscribe_stream_offline(
        &self,
        broadcaster_id: &str,
        secret: &str,
    ) -> AppResult<EventSubSubscription> {
        self.create_eventsub_subscription(
            "stream.offline",
            "1",
            serde_json::json!({
                "broadcaster_user_id": broadcaster_id
            }),
            secret,
        )
        .await
    }

    /// Subscribe to channel update event (title/category changes)
    pub async fn subscribe_channel_update(
        &self,
        broadcaster_id: &str,
        secret: &str,
    ) -> AppResult<EventSubSubscription> {
        self.create_eventsub_subscription(
            "channel.update",
            "2",
            serde_json::json!({
                "broadcaster_user_id": broadcaster_id
            }),
            secret,
        )
        .await
    }

    /// Subscribe to channel point reward redemption
    pub async fn subscribe_channel_points_redemption(
        &self,
        broadcaster_id: &str,
        secret: &str,
    ) -> AppResult<EventSubSubscription> {
        self.create_eventsub_subscription(
            "channel.channel_points_custom_reward_redemption.add",
            "1",
            serde_json::json!({
                "broadcaster_user_id": broadcaster_id
            }),
            secret,
        )
        .await
    }

    // ========================================================================
    // Channel Points Methods
    // ========================================================================

    /// Get custom channel point rewards
    pub async fn get_custom_rewards(
        &self,
        access_token: &str,
        broadcaster_id: &str,
    ) -> AppResult<Vec<CustomReward>> {
        let response = self
            .send_with_backoff(|| {
                self.client
                    .get(format!(
                        "{}/channel_points/custom_rewards?broadcaster_id={}",
                        TWITCH_API_URL, broadcaster_id
                    ))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Client-Id", &self.client_id)
            })
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::TwitchApi(format!(
                "Failed to get custom rewards: {}",
                error_text
            )));
        }

        let rewards: CustomRewardsResponse = response
            .json()
            .await
            .map_err(|e| AppError::TwitchApi(format!("Failed to parse rewards response: {}", e)))?;

        Ok(rewards.data)
    }

    // ========================================================================
    // Schedule Methods
    // ========================================================================

    /// Get channel schedule
    pub async fn get_schedule(
        &self,
        access_token: &str,
        broadcaster_id: &str,
    ) -> AppResult<Option<ScheduleData>> {
        let response = self
            .send_with_backoff(|| {
                self.client
                    .get(format!(
                        "{}/schedule?broadcaster_id={}",
                        TWITCH_API_URL, broadcaster_id
                    ))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Client-Id", &self.client_id)
            })
            .await?;

        // 404 is returned when there's no schedule
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::TwitchApi(format!(
                "Failed to get schedule: {}",
                error_text
            )));
        }

        let schedule: ScheduleResponse = response.json().await.map_err(|e| {
            AppError::TwitchApi(format!("Failed to parse schedule response: {}", e))
        })?;

        Ok(Some(schedule.data))
    }

    // ========================================================================
    // Chat Methods
    // ========================================================================

    /// Send a chat message
    pub async fn send_chat_message(
        &self,
        access_token: &str,
        broadcaster_id: &str,
        sender_id: &str,
        message: &str,
    ) -> AppResult<ChatMessageResult> {
        let request = SendChatMessageRequest {
            broadcaster_id: broadcaster_id.to_string(),
            sender_id: sender_id.to_string(),
            message: message.to_string(),
        };

        let response = self
            .send_with_backoff(|| {
                self.client
                    .post(format!("{}/chat/messages", TWITCH_API_URL))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Client-Id", &self.client_id)
                    .json(&request)
            })
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::TwitchApi(format!(
                "Failed to send chat message: {}",
                error_text
            )));
        }

        let result: SendChatMessageResponse = response.json().await.map_err(|e| {
            AppError::TwitchApi(format!("Failed to parse chat message response: {}", e))
        })?;

        result
            .data
            .into_iter()
            .next()
            .ok_or_else(|| AppError::TwitchApi("No message result returned".to_string()))
    }

    // ========================================================================
    // Helper Methods
    // ========================================================================

    /// Calculate token expiry time
    pub fn calculate_token_expiry(expires_in: i64) -> chrono::DateTime<Utc> {
        Utc::now() + Duration::seconds(expires_in)
    }

    /// Get required OAuth scopes for the application
    // NOTE: Removed "channel:read:schedule" — Twitch reports it as an invalid scope.
    pub fn get_required_scopes() -> Vec<&'static str> {
        vec![
            "user:read:email",
            "channel:read:subscriptions",
            "channel:read:redemptions",
            "channel:manage:redemptions",
            "moderator:read:followers",
            "user:read:chat",
            "user:write:chat",
        ]
    }

    pub fn client_id(&self) -> &str {
        &self.client_id
    }
}
