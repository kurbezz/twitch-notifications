use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{AppError, AppResult};

#[derive(Clone)]
pub struct DiscordService {
    client: reqwest::Client,
    bot_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordEmbed {
    pub title: Option<String>,
    pub description: Option<String>,
    pub url: Option<String>,
    pub color: Option<u32>,
    pub timestamp: Option<String>,
    pub footer: Option<EmbedFooter>,
    pub image: Option<EmbedImage>,
    pub thumbnail: Option<EmbedThumbnail>,
    pub author: Option<EmbedAuthor>,
    pub fields: Option<Vec<EmbedField>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedFooter {
    pub text: String,
    pub icon_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedImage {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedThumbnail {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedAuthor {
    pub name: String,
    pub url: Option<String>,
    pub icon_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedField {
    pub name: String,
    pub value: String,
    #[serde(default)]
    pub inline: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookMessage {
    pub content: Option<String>,
    pub username: Option<String>,
    pub avatar_url: Option<String>,
    pub embeds: Option<Vec<DiscordEmbed>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordMessage {
    pub content: Option<String>,
    pub embeds: Option<Vec<DiscordEmbed>>,
    pub tts: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordGuild {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordChannel {
    pub id: String,
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub channel_type: u8,
    pub guild_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordRole {
    pub id: String,
    #[serde(deserialize_with = "deserialize_permissions")]
    pub permissions: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordGuildInfo {
    pub id: String,
    pub name: Option<String>,
    pub owner_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordMember {
    pub roles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledEvent {
    pub id: Option<String>,
    pub guild_id: String,
    pub channel_id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub scheduled_start_time: String,
    pub scheduled_end_time: Option<String>,
    pub privacy_level: u8, // 2 = GUILD_ONLY
    pub entity_type: u8,   // 1 = STAGE_INSTANCE, 2 = VOICE, 3 = EXTERNAL
    pub entity_metadata: Option<EntityMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityMetadata {
    pub location: Option<String>,
}

impl DiscordService {
    pub async fn new(bot_token: String) -> AppResult<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| AppError::Discord(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self { client, bot_token })
    }

    fn api_url(&self, endpoint: &str) -> String {
        format!("https://discord.com/api/v10{}", endpoint)
    }

    fn auth_header(&self) -> String {
        format!("Bot {}", self.bot_token)
    }

    /// Parse retry_after from a Discord rate limit error response
    fn parse_retry_after(error_text: &str) -> Option<f64> {
        if let Ok(json) = serde_json::from_str::<Value>(error_text) {
            if let Some(retry_after) = json.get("retry_after") {
                return retry_after.as_f64();
            }
        }
        None
    }

    /// Handle rate limiting by waiting for the retry_after duration
    async fn handle_rate_limit(error_text: &str) -> AppResult<()> {
        if let Some(retry_after) = Self::parse_retry_after(error_text) {
            let wait_seconds = (retry_after.ceil() as u64) + 1; // Add 1 second buffer
            tracing::warn!(
                "Discord rate limit hit, waiting {} seconds before retry",
                wait_seconds
            );
            tokio::time::sleep(tokio::time::Duration::from_secs(wait_seconds)).await;
            Ok(())
        } else {
            Err(AppError::Discord(format!(
                "Rate limited but could not parse retry_after: {}",
                error_text
            )))
        }
    }

    /// Send a message to a channel
    pub async fn send_message(&self, channel_id: &str, message: DiscordMessage) -> AppResult<()> {
        let url = self.api_url(&format!("/channels/{}/messages", channel_id));

        let response = self
            .client
            .post(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .json(&message)
            .send()
            .await
            .map_err(|e| AppError::Discord(format!("Failed to send message: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::Discord(format!(
                "Discord API error ({}): {}",
                status, error_text
            )));
        }

        Ok(())
    }

    /// Send a message via webhook
    pub async fn send_webhook_message(
        &self,
        webhook_url: &str,
        message: WebhookMessage,
    ) -> AppResult<()> {
        let response = self
            .client
            .post(webhook_url)
            .header("Content-Type", "application/json")
            .json(&message)
            .send()
            .await
            .map_err(|e| AppError::Discord(format!("Failed to send webhook message: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::Discord(format!(
                "Discord webhook error ({}): {}",
                status, error_text
            )));
        }

        Ok(())
    }

    /// Get guilds the bot is a member of
    pub async fn get_guilds(&self) -> AppResult<Vec<DiscordGuild>> {
        let url = self.api_url("/users/@me/guilds");

        let response = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| AppError::Discord(format!("Failed to get guilds: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::Discord(format!(
                "Discord API error ({}): {}",
                status, error_text
            )));
        }

        response
            .json()
            .await
            .map_err(|e| AppError::Discord(format!("Failed to parse guilds response: {}", e)))
    }

    /// Get channels in a guild
    pub async fn get_guild_channels(&self, guild_id: &str) -> AppResult<Vec<DiscordChannel>> {
        let url = self.api_url(&format!("/guilds/{}/channels", guild_id));

        let response = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| AppError::Discord(format!("Failed to get channels: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::Discord(format!(
                "Discord API error ({}): {}",
                status, error_text
            )));
        }

        response
            .json()
            .await
            .map_err(|e| AppError::Discord(format!("Failed to parse channels response: {}", e)))
    }

    /// Get a single Discord channel by ID
    pub async fn get_channel(&self, channel_id: &str) -> AppResult<DiscordChannel> {
        let url = self.api_url(&format!("/channels/{}", channel_id));

        let response = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| AppError::Discord(format!("Failed to get channel: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::Discord(format!(
                "Discord API error ({}): {}",
                status, error_text
            )));
        }

        response
            .json()
            .await
            .map_err(|e| AppError::Discord(format!("Failed to parse channel response: {}", e)))
    }

    /// Check whether a specific user is a member of a guild
    pub async fn is_user_in_guild(&self, guild_id: &str, user_id: &str) -> AppResult<bool> {
        let url = self.api_url(&format!("/guilds/{}/members/{}", guild_id, user_id));

        let response = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| AppError::Discord(format!("Failed to check guild membership: {}", e)))?;

        if response.status().is_success() {
            Ok(true)
        } else if response.status() == reqwest::StatusCode::NOT_FOUND {
            Ok(false)
        } else {
            let error_text = response.text().await.unwrap_or_default();
            tracing::warn!("Discord membership check failed: body={}", error_text);
            Err(AppError::Discord(format!(
                "Discord API error: {}",
                error_text
            )))
        }
    }

    // ------------------------------------------------------------------------
    // Additional helpers for guild/member/role queries and permission checks.
    // ------------------------------------------------------------------------

    /// Fetch basic guild information (we only need owner_id here)
    pub async fn get_guild(&self, guild_id: &str) -> AppResult<DiscordGuildInfo> {
        let url = self.api_url(&format!("/guilds/{}", guild_id));

        let response = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| AppError::Discord(format!("Failed to get guild info: {}", e)))?;

        if response.status().is_success() {
            response
                .json()
                .await
                .map_err(|e| AppError::Discord(format!("Failed to parse guild response: {}", e)))
        } else if response.status() == reqwest::StatusCode::NOT_FOUND {
            Err(AppError::NotFound("Guild not found".to_string()))
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            Err(AppError::Discord(format!(
                "Discord API error ({}): {}",
                status, error_text
            )))
        }
    }

    /// Fetch roles for a guild (role id + permissions)
    pub async fn get_guild_roles(&self, guild_id: &str) -> AppResult<Vec<DiscordRole>> {
        let url = self.api_url(&format!("/guilds/{}/roles", guild_id));

        let response = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| AppError::Discord(format!("Failed to get guild roles: {}", e)))?;

        if response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            match serde_json::from_str::<Vec<DiscordRole>>(&body) {
                Ok(r) => Ok(r),
                Err(e) => {
                    tracing::warn!(
                        "Failed to parse roles response for guild {}: {} -- body: {}",
                        guild_id,
                        e,
                        body
                    );
                    Err(AppError::Discord(format!(
                        "Failed to parse roles response: {}",
                        e
                    )))
                }
            }
        } else {
            let error_text = response.text().await.unwrap_or_default();
            tracing::warn!("Discord get_guild_roles failed: body={}", error_text);
            Err(AppError::Discord(format!(
                "Discord API error: {}",
                error_text
            )))
        }
    }

    /// Fetch a guild member (includes member roles)
    pub async fn get_guild_member(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> AppResult<DiscordMember> {
        let url = self.api_url(&format!("/guilds/{}/members/{}", guild_id, user_id));

        let response = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| AppError::Discord(format!("Failed to get guild member: {}", e)))?;

        if response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            match serde_json::from_str::<DiscordMember>(&body) {
                Ok(m) => Ok(m),
                Err(e) => {
                    tracing::warn!("Failed to parse guild member response for guild {} user {}: {} -- body: {}", guild_id, user_id, e, body);
                    Err(AppError::Discord(format!(
                        "Failed to parse member response: {}",
                        e
                    )))
                }
            }
        } else if response.status() == reqwest::StatusCode::NOT_FOUND {
            Err(AppError::NotFound("Member not found".to_string()))
        } else {
            let error_text = response.text().await.unwrap_or_default();
            tracing::warn!("Discord get_guild_member failed: body={}", error_text);
            Err(AppError::Discord(format!(
                "Discord API error: {}",
                error_text
            )))
        }
    }

    /// Determine whether a given user in a guild has either ADMINISTRATOR or MANAGE_GUILD permissions.
    /// Returns Ok(true) if they do, Ok(false) if they don't or are not a member, or Err on other failures.
    pub async fn user_has_manage_permissions(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> AppResult<bool> {
        // Try to fetch member; if not found or inaccessible, treat as no permissions.
        let member = match self.get_guild_member(guild_id, user_id).await {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(
                    "Failed to fetch guild member {} for guild {}: {:?}",
                    user_id,
                    guild_id,
                    e
                );
                return Ok(false);
            }
        };

        // Fetch roles for guild
        let roles = match self.get_guild_roles(guild_id).await {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("Failed to fetch roles for guild {}: {:?}", guild_id, e);
                return Ok(false);
            }
        };

        // Compute accumulated permissions from member roles
        let mut perms: u64 = 0;
        for rid in member.roles.iter() {
            if let Some(role) = roles.iter().find(|r| r.id == *rid) {
                perms |= role.permissions;
            }
        }

        // If owner, allow
        match self.get_guild(guild_id).await {
            Ok(guild_info) => {
                if let Some(owner_id) = guild_info.owner_id {
                    if owner_id == user_id {
                        return Ok(true);
                    }
                }
            }
            Err(e) => tracing::warn!("Failed to fetch guild info for {}: {:?}", guild_id, e),
        }

        // Permission bits: ADMINISTRATOR (1 << 3), MANAGE_GUILD (1 << 5)
        const PERM_ADMINISTRATOR: u64 = 1 << 3; // 8
        const PERM_MANAGE_GUILD: u64 = 1 << 5; // 32

        Ok((perms & PERM_ADMINISTRATOR) != 0 || (perms & PERM_MANAGE_GUILD) != 0)
    }

    /// Create a scheduled event in a guild
    /// Retries once on rate limit (429) errors
    pub async fn create_scheduled_event(&self, event: ScheduledEvent) -> AppResult<ScheduledEvent> {
        let url = self.api_url(&format!("/guilds/{}/scheduled-events", event.guild_id));

        let response = self
            .client
            .post(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .json(&event)
            .send()
            .await
            .map_err(|e| AppError::Discord(format!("Failed to create scheduled event: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();

            // Handle rate limiting (429) with retry
            if status.as_u16() == 429 {
                Self::handle_rate_limit(&error_text).await?;
                // Retry once after waiting
                let retry_response = self
                    .client
                    .post(&url)
                    .header("Authorization", self.auth_header())
                    .header("Content-Type", "application/json")
                    .json(&event)
                    .send()
                    .await
                    .map_err(|e| {
                        AppError::Discord(format!("Failed to retry create scheduled event: {}", e))
                    })?;

                if !retry_response.status().is_success() {
                    let retry_status = retry_response.status();
                    let retry_error_text = retry_response.text().await.unwrap_or_default();
                    return Err(AppError::Discord(format!(
                        "Discord API error ({}): {}",
                        retry_status, retry_error_text
                    )));
                }

                return retry_response.json().await.map_err(|e| {
                    AppError::Discord(format!("Failed to parse event response: {}", e))
                });
            }

            return Err(AppError::Discord(format!(
                "Discord API error ({}): {}",
                status, error_text
            )));
        }

        response
            .json()
            .await
            .map_err(|e| AppError::Discord(format!("Failed to parse event response: {}", e)))
    }

    /// Update a scheduled event
    /// Retries once on rate limit (429) errors
    pub async fn update_scheduled_event(
        &self,
        guild_id: &str,
        event_id: &str,
        event: ScheduledEvent,
    ) -> AppResult<ScheduledEvent> {
        let url = self.api_url(&format!(
            "/guilds/{}/scheduled-events/{}",
            guild_id, event_id
        ));

        let response = self
            .client
            .patch(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .json(&event)
            .send()
            .await
            .map_err(|e| AppError::Discord(format!("Failed to update scheduled event: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();

            // Handle rate limiting (429) with retry
            if status.as_u16() == 429 {
                Self::handle_rate_limit(&error_text).await?;
                // Retry once after waiting
                let retry_response = self
                    .client
                    .patch(&url)
                    .header("Authorization", self.auth_header())
                    .header("Content-Type", "application/json")
                    .json(&event)
                    .send()
                    .await
                    .map_err(|e| {
                        AppError::Discord(format!("Failed to retry update scheduled event: {}", e))
                    })?;

                if !retry_response.status().is_success() {
                    let retry_status = retry_response.status();
                    let retry_error_text = retry_response.text().await.unwrap_or_default();
                    return Err(AppError::Discord(format!(
                        "Discord API error ({}): {}",
                        retry_status, retry_error_text
                    )));
                }

                return retry_response.json().await.map_err(|e| {
                    AppError::Discord(format!("Failed to parse event response: {}", e))
                });
            }

            return Err(AppError::Discord(format!(
                "Discord API error ({}): {}",
                status, error_text
            )));
        }

        response
            .json()
            .await
            .map_err(|e| AppError::Discord(format!("Failed to parse event response: {}", e)))
    }

    /// Delete a scheduled event
    /// Retries once on rate limit (429) errors
    pub async fn delete_scheduled_event(&self, guild_id: &str, event_id: &str) -> AppResult<()> {
        let url = self.api_url(&format!(
            "/guilds/{}/scheduled-events/{}",
            guild_id, event_id
        ));

        let response = self
            .client
            .delete(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| AppError::Discord(format!("Failed to delete scheduled event: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();

            // Handle rate limiting (429) with retry
            if status.as_u16() == 429 {
                Self::handle_rate_limit(&error_text).await?;
                // Retry once after waiting
                let retry_response = self
                    .client
                    .delete(&url)
                    .header("Authorization", self.auth_header())
                    .send()
                    .await
                    .map_err(|e| {
                        AppError::Discord(format!("Failed to retry delete scheduled event: {}", e))
                    })?;

                if !retry_response.status().is_success() {
                    let retry_status = retry_response.status();
                    let retry_error_text = retry_response.text().await.unwrap_or_default();
                    return Err(AppError::Discord(format!(
                        "Discord API error ({}): {}",
                        retry_status, retry_error_text
                    )));
                }

                return Ok(());
            }

            return Err(AppError::Discord(format!(
                "Discord API error ({}): {}",
                status, error_text
            )));
        }

        Ok(())
    }

    /// Get scheduled events for a guild
    pub async fn get_scheduled_events(&self, guild_id: &str) -> AppResult<Vec<ScheduledEvent>> {
        let url = self.api_url(&format!("/guilds/{}/scheduled-events", guild_id));

        let response = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| AppError::Discord(format!("Failed to get scheduled events: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::Discord(format!(
                "Discord API error ({}): {}",
                status, error_text
            )));
        }

        response
            .json()
            .await
            .map_err(|e| AppError::Discord(format!("Failed to parse events response: {}", e)))
    }
}

fn deserialize_permissions<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    struct PermVisitor;

    impl<'de> serde::de::Visitor<'de> for PermVisitor {
        type Value = u64;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a number or string containing a u64 permissions bitfield")
        }

        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(v)
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            v.parse::<u64>()
                .map_err(|e| E::custom(format!("invalid permissions string: {}", e)))
        }

        fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            self.visit_str(&v)
        }
    }

    deserializer.deserialize_any(PermVisitor)
}

impl DiscordEmbed {
    pub fn new() -> Self {
        Self {
            title: None,
            description: None,
            url: None,
            color: None,
            timestamp: None,
            footer: None,
            image: None,
            thumbnail: None,
            author: None,
            fields: None,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    pub fn color(mut self, color: u32) -> Self {
        self.color = Some(color);
        self
    }

    pub fn timestamp(mut self, timestamp: impl Into<String>) -> Self {
        self.timestamp = Some(timestamp.into());
        self
    }

    pub fn footer(mut self, text: impl Into<String>, icon_url: Option<String>) -> Self {
        self.footer = Some(EmbedFooter {
            text: text.into(),
            icon_url,
        });
        self
    }
}

//
// OAuth helpers for Discord
// These functions implement a minimal OAuth token exchange and user fetch
// used by the account linking flow. They intentionally keep dependencies
// small and surface Discord API errors through AppError::Discord.
//

#[derive(Debug, Deserialize)]
pub struct DiscordTokenResponse {
    pub access_token: String,
}

/// Exchange an authorization code for an access token at Discord's OAuth2 token endpoint.
pub async fn exchange_code_for_token(
    client_id: &str,
    client_secret: &str,
    code: &str,
    redirect_uri: &str,
) -> AppResult<DiscordTokenResponse> {
    let client = reqwest::Client::new();
    let response = client
        .post("https://discord.com/api/oauth2/token")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
        ])
        .send()
        .await
        .map_err(|e| AppError::Discord(format!("Failed to exchange code for token: {}", e)))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| AppError::Discord(format!("Failed to read token response body: {}", e)))?;

    if !status.is_success() {
        return Err(AppError::Discord(format!(
            "Discord token exchange failed ({}): {}",
            status, body
        )));
    }

    let token_resp = serde_json::from_str::<DiscordTokenResponse>(&body)
        .map_err(|e| AppError::Discord(format!("Failed to parse token response: {}", e)))?;
    Ok(token_resp)
}

#[derive(Debug, Deserialize)]
pub struct DiscordUser {
    pub id: String,
    pub username: String,
    pub discriminator: String,
    pub avatar: Option<String>,
}

/// Fetch the Discord user object using a Bearer access token
pub async fn get_discord_user(access_token: &str) -> AppResult<DiscordUser> {
    let client = reqwest::Client::new();
    let response = client
        .get("https://discord.com/api/users/@me")
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| AppError::Discord(format!("Failed to fetch Discord user: {}", e)))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| AppError::Discord(format!("Failed to read user response body: {}", e)))?;

    if !status.is_success() {
        return Err(AppError::Discord(format!(
            "Discord API error ({}): {}",
            status, body
        )));
    }

    let user = serde_json::from_str::<DiscordUser>(&body)
        .map_err(|e| AppError::Discord(format!("Failed to parse user response: {}", e)))?;
    Ok(user)
}

impl DiscordEmbed {
    pub fn image(mut self, url: impl Into<String>) -> Self {
        self.image = Some(EmbedImage { url: url.into() });
        self
    }

    pub fn thumbnail(mut self, url: impl Into<String>) -> Self {
        self.thumbnail = Some(EmbedThumbnail { url: url.into() });
        self
    }

    pub fn author(
        mut self,
        name: impl Into<String>,
        url: Option<String>,
        icon_url: Option<String>,
    ) -> Self {
        self.author = Some(EmbedAuthor {
            name: name.into(),
            url,
            icon_url,
        });
        self
    }

    pub fn field(
        mut self,
        name: impl Into<String>,
        value: impl Into<String>,
        inline: bool,
    ) -> Self {
        let field = EmbedField {
            name: name.into(),
            value: value.into(),
            inline,
        };
        match &mut self.fields {
            Some(fields) => fields.push(field),
            None => self.fields = Some(vec![field]),
        }
        self
    }
}

impl Default for DiscordEmbed {
    fn default() -> Self {
        Self::new()
    }
}

// Color constants for embeds
pub mod colors {
    pub const TWITCH_PURPLE: u32 = 0x9146FF;
    pub const SUCCESS: u32 = 0x57F287;
    pub const INFO: u32 = 0x5865F2;
}

/// Helper function to create a stream online notification embed
pub fn create_stream_online_embed(
    streamer_name: &str,
    streamer_avatar: Option<&str>,
    title: &str,
    game: &str,
    thumbnail_url: Option<&str>,
    stream_url: &str,
) -> DiscordEmbed {
    let mut embed = DiscordEmbed::new()
        .title(format!("🔴 {} is now live!", streamer_name))
        .description(title)
        .url(stream_url)
        .color(colors::TWITCH_PURPLE)
        .field("Game", game, true)
        .timestamp(chrono::Utc::now().to_rfc3339());

    if let Some(avatar) = streamer_avatar {
        embed = embed.author(
            streamer_name,
            Some(stream_url.to_string()),
            Some(avatar.to_string()),
        );
    }

    if let Some(thumbnail) = thumbnail_url {
        let thumbnail_sized = thumbnail
            .replace("{width}", "440")
            .replace("{height}", "248");
        embed = embed.image(thumbnail_sized);
    }

    embed
}

/// Helper function to create a stream offline notification embed
pub fn create_stream_offline_embed(streamer_name: &str, stream_url: &str) -> DiscordEmbed {
    DiscordEmbed::new()
        .title(format!("⚫ {} ended the stream", streamer_name))
        .url(stream_url)
        .color(colors::INFO)
        .timestamp(chrono::Utc::now().to_rfc3339())
}

/// Title change embed using the user's rendered template (same text as Telegram).
pub fn create_title_change_embed_with_message(
    streamer_name: &str,
    stream_url: &str,
    message: &str,
) -> DiscordEmbed {
    DiscordEmbed::new()
        .title(format!("📝 {} changed the stream title", streamer_name))
        .description(message)
        .url(stream_url)
        .color(colors::INFO)
        .timestamp(chrono::Utc::now().to_rfc3339())
}

/// Category change embed using the user's rendered template (same text as Telegram).
pub fn create_category_change_embed_with_message(
    streamer_name: &str,
    stream_url: &str,
    message: &str,
) -> DiscordEmbed {
    DiscordEmbed::new()
        .title(format!("🎮 {} changed the category", streamer_name))
        .description(message)
        .url(stream_url)
        .color(colors::INFO)
        .timestamp(chrono::Utc::now().to_rfc3339())
}

/// Helper function to create a reward redemption notification embed
pub fn create_reward_redemption_embed(
    redeemer_name: &str,
    reward_name: &str,
    reward_cost: i32,
    user_input: Option<&str>,
) -> DiscordEmbed {
    let mut embed = DiscordEmbed::new()
        .title(format!("🎁 {} redeemed a reward!", redeemer_name))
        .description(format!("**{}** for {} points", reward_name, reward_cost))
        .color(colors::SUCCESS)
        .timestamp(chrono::Utc::now().to_rfc3339());

    if let Some(input) = user_input {
        if !input.is_empty() {
            embed = embed.field("Message", input, false);
        }
    }

    embed
}

#[async_trait::async_trait]
impl crate::services::notifications::Notifier for DiscordService {
    async fn send_notification<'a>(
        &self,
        ctx: &crate::services::notifications::IntegrationContext,
        content: crate::services::notifications::NotificationContent<'a>,
        _settings: &crate::db::NotificationSettings,
        stream_url: Option<String>,
        message: String,
    ) -> crate::error::AppResult<()> {
        let (embed, username, avatar) = match content {
            crate::services::notifications::NotificationContent::StreamOnline(data) => {
                let stream_url_ref = stream_url.as_deref().unwrap_or("");
                let embed = create_stream_online_embed(
                    &data.streamer_name,
                    data.streamer_avatar.as_deref(),
                    &data.title,
                    &data.category,
                    data.thumbnail_url.as_deref(),
                    stream_url_ref,
                );
                (
                    embed,
                    Some(format!("{} is live!", data.streamer_name.clone())),
                    data.streamer_avatar.clone(),
                )
            }
            crate::services::notifications::NotificationContent::StreamOffline(data) => {
                let stream_url_ref = stream_url.as_deref().unwrap_or("");
                let embed = create_stream_offline_embed(&data.streamer_name, stream_url_ref);
                (embed, Some(data.streamer_name.clone()), None)
            }
            crate::services::notifications::NotificationContent::TitleChange(data) => {
                let stream_url_ref = stream_url.as_deref().unwrap_or("");
                let embed = create_title_change_embed_with_message(
                    &data.streamer_name,
                    stream_url_ref,
                    &message,
                );
                (embed, Some(data.streamer_name.clone()), None)
            }
            crate::services::notifications::NotificationContent::CategoryChange(data) => {
                let stream_url_ref = stream_url.as_deref().unwrap_or("");
                let embed = create_category_change_embed_with_message(
                    &data.streamer_name,
                    stream_url_ref,
                    &message,
                );
                (embed, Some(data.streamer_name.clone()), None)
            }
            crate::services::notifications::NotificationContent::RewardRedemption(data) => {
                let embed = create_reward_redemption_embed(
                    &data.redeemer_name,
                    &data.reward_name,
                    data.reward_cost,
                    data.user_input.as_deref(),
                );
                (embed, Some(data.broadcaster_name.clone()), None)
            }
        };

        if let Some(webhook_url) = &ctx.webhook_url {
            let msg = WebhookMessage {
                content: None,
                username,
                avatar_url: avatar,
                embeds: Some(vec![embed]),
            };
            self.send_webhook_message(webhook_url, msg).await
        } else {
            let msg = DiscordMessage {
                content: None,
                embeds: Some(vec![embed]),
                tts: None,
            };
            self.send_message(&ctx.destination_id, msg).await
        }
    }
}
