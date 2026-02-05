use std::sync::Arc;

use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::db::UserRepository;
use crate::error::{AppError, AppResult};
use crate::services::discord::{exchange_code_for_token, get_discord_user};
use crate::services::subscriptions::SubscriptionManager;
use crate::services::telegram::verify_telegram_login_payload;
use crate::services::twitch::TwitchService;
use crate::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthState {
    pub csrf_token: String,
    pub redirect_to: Option<String>,
    pub lang: Option<String>,
    pub iat: usize,
    pub exp: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordOAuthState {
    pub csrf_token: String,
    pub user_id: String,
    pub redirect_to: Option<String>,
    pub iat: usize,
    pub exp: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
}

pub struct AuthService;

impl AuthService {
    /// Generate OAuth state JWT for Twitch login
    pub fn generate_oauth_state(
        state: &Arc<AppState>,
        redirect_to: Option<String>,
        lang: Option<String>,
    ) -> AppResult<String> {
        let csrf_token = Self::generate_random_string(32);
        let now = Utc::now();
        let iat = now.timestamp() as usize;
        let exp = (now + Duration::minutes(10)).timestamp() as usize;

        let state_claims = OAuthState {
            csrf_token,
            redirect_to,
            lang,
            iat,
            exp,
        };

        let state_jwt = encode(
            &Header::default(),
            &state_claims,
            &EncodingKey::from_secret(state.config.jwt.secret.as_bytes()),
        )?;

        Ok(state_jwt)
    }

    /// Decode and validate OAuth state JWT
    pub fn decode_oauth_state(
        state: &Arc<AppState>,
        state_encoded: &str,
    ) -> AppResult<OAuthState> {
        let token_data = decode::<OAuthState>(
            state_encoded,
            &DecodingKey::from_secret(state.config.jwt.secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|e| {
            tracing::error!("Failed to decode OAuth state: {:?}", e);
            e
        })?;
        Ok(token_data.claims)
    }

    /// Generate Discord OAuth state JWT
    pub fn generate_discord_oauth_state(
        state: &Arc<AppState>,
        user_id: String,
        redirect_to: Option<String>,
    ) -> AppResult<String> {
        let csrf_token = Self::generate_random_string(32);
        let now = Utc::now();
        let iat = now.timestamp() as usize;
        let exp = (now + Duration::minutes(10)).timestamp() as usize;

        let state_claims = DiscordOAuthState {
            csrf_token,
            user_id,
            redirect_to,
            iat,
            exp,
        };

        let state_jwt = encode(
            &Header::default(),
            &state_claims,
            &EncodingKey::from_secret(state.config.jwt.secret.as_bytes()),
        )?;

        Ok(state_jwt)
    }

    /// Decode and validate Discord OAuth state JWT
    pub fn decode_discord_oauth_state(
        state: &Arc<AppState>,
        state_encoded: &str,
    ) -> AppResult<DiscordOAuthState> {
        let token_data = decode::<DiscordOAuthState>(
            state_encoded,
            &DecodingKey::from_secret(state.config.jwt.secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|e| {
            tracing::error!("Failed to decode Discord OAuth state: {:?}", e);
            e
        })?;
        Ok(token_data.claims)
    }

    /// Create a signed JWT for a user id
    pub fn create_jwt(state: &Arc<AppState>, user_id: &str) -> AppResult<String> {
        let now = Utc::now();
        let exp = now + Duration::hours(state.config.jwt.expiration_hours);
        let claims = Claims {
            sub: user_id.to_string(),
            iat: now.timestamp() as usize,
            exp: exp.timestamp() as usize,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(state.config.jwt.secret.as_bytes()),
        )?;
        Ok(token)
    }

    /// Decode and validate a JWT, returning the claims
    pub fn decode_jwt(state: &Arc<AppState>, token: &str) -> AppResult<Claims> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(state.config.jwt.secret.as_bytes()),
            &Validation::default(),
        )?;
        Ok(token_data.claims)
    }

    /// Get user from JWT token
    pub async fn get_user_from_token(
        state: &Arc<AppState>,
        token: &str,
    ) -> AppResult<crate::db::User> {
        let claims = Self::decode_jwt(state, token)?;
        let user = UserRepository::find_by_id(&state.db, &claims.sub)
            .await?
            .ok_or(AppError::Unauthorized)?;
        Ok(user)
    }

    /// Handle Twitch OAuth callback
    pub async fn handle_twitch_callback(
        state: &Arc<AppState>,
        code: String,
        oauth_state: OAuthState,
    ) -> AppResult<(String, String)> {
        // Exchange code for tokens
        let token_response = state.twitch.exchange_code(&code).await?;

        // Get user info from Twitch
        let twitch_user = state.twitch.get_user(&token_response.access_token).await?;

        // Calculate token expiry
        let token_expires_at = TwitchService::calculate_token_expiry(token_response.expires_in);

        // Create or update user
        let user = UserRepository::upsert_by_twitch_id(
            &state.db,
            &twitch_user.id,
            &twitch_user.login,
            &twitch_user.display_name,
            twitch_user.email.as_deref().unwrap_or(""),
            twitch_user.profile_image_url.as_deref().unwrap_or(""),
            &token_response.access_token,
            &token_response.refresh_token,
            token_expires_at.naive_utc(),
            oauth_state.lang.as_deref(),
        )
        .await?;

        // Spawn background task to synchronize EventSub subscriptions
        {
            let state = state.clone();
            let user = user.clone();
            tokio::spawn(async move {
                match SubscriptionManager::sync_for_user(&state, &user).await {
                    Ok(_) => tracing::info!("Synced EventSub subscriptions for user {}", user.id),
                    Err(e) => tracing::warn!(
                        "Failed to sync EventSub subscriptions for user {}: {:?}",
                        user.id,
                        e
                    ),
                }
            });
        }

        // Create JWT token
        let token = Self::create_jwt(state, &user.id)?;

        // Build redirect URL with token
        let frontend_base = state.config.server.frontend_url.trim_end_matches('/');
        let callback_url = format!("{}/auth/callback", frontend_base);
        let token_enc: String = url::form_urlencoded::byte_serialize(token.as_bytes()).collect();
        let expires_at = (Utc::now() + Duration::hours(state.config.jwt.expiration_hours)).timestamp();

        let raw_redirect = oauth_state.redirect_to.as_deref().unwrap_or("/dashboard");
        let safe_redirect = if Self::is_safe_redirect(raw_redirect, frontend_base) {
            raw_redirect.to_string()
        } else {
            tracing::warn!("Rejected unsafe redirect_to value: {}", raw_redirect);
            "/dashboard".to_string()
        };

        let redirect_with_fragment = format!(
            "{}#access_token={}&token_type=Bearer&expires_at={}&redirect_to={}",
            callback_url,
            token_enc,
            expires_at,
            urlencoding::encode(&safe_redirect)
        );

        Ok((redirect_with_fragment, user.id))
    }

    /// Handle Discord OAuth callback
    pub async fn handle_discord_callback(
        state: &Arc<AppState>,
        code: String,
        discord_state: DiscordOAuthState,
    ) -> AppResult<String> {
        let client_id = state
            .config
            .discord
            .client_id
            .as_ref()
            .ok_or_else(|| AppError::ServiceUnavailable("Discord OAuth not configured".to_string()))?;
        let client_secret = state
            .config
            .discord
            .client_secret
            .as_ref()
            .ok_or_else(|| AppError::ServiceUnavailable("Discord OAuth not configured".to_string()))?;

        // Exchange code for token
        let callback_url = format!(
            "{}/api/auth/discord/callback",
            state.config.server.webhook_url.trim_end_matches('/')
        );
        let token_resp = exchange_code_for_token(client_id, client_secret, &code, &callback_url).await?;

        // Fetch user info
        let discord_user = get_discord_user(&token_resp.access_token).await?;

        // Compose friendly username and avatar url
        let username = format!("{}#{}", discord_user.username, discord_user.discriminator);
        let avatar_url = if let Some(avatar) = discord_user.avatar {
            format!(
                "https://cdn.discordapp.com/avatars/{}/{}.png",
                discord_user.id, avatar
            )
        } else {
            String::new()
        };

        // Persist Discord identity
        UserRepository::set_discord_info(
            &state.db,
            &discord_state.user_id,
            &discord_user.id,
            &username,
            &avatar_url,
        )
        .await?;

        // Build redirect URL
        let redirect_url = Self::compose_redirect_url(
            &state.config.server.frontend_url,
            discord_state.redirect_to.as_deref(),
        );

        Ok(redirect_url)
    }

    /// Handle Telegram login linking
    pub async fn handle_telegram_link(
        state: &Arc<AppState>,
        user_id: String,
        telegram_payload: std::collections::HashMap<String, String>,
    ) -> AppResult<()> {
        let bot_token = state
            .config
            .telegram
            .bot_token
            .as_ref()
            .ok_or_else(|| AppError::ServiceUnavailable("Telegram bot not configured".to_string()))?;

        // Verify signature
        let info = verify_telegram_login_payload(&telegram_payload, bot_token)?;

        // Check if already linked
        let user = UserRepository::find_by_id(&state.db, &user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        if let Some(existing_tg_id) = &user.telegram_user_id {
            if existing_tg_id == &info.id {
                return Ok(()); // Already linked
            }
        }

        // Download and store photo
        let stored_photo_url = match Self::download_and_store_telegram_photo(
            state,
            &info.id,
            info.photo_url.as_deref(),
        )
        .await
        {
            Ok(url_opt) => url_opt,
            Err(e) => {
                tracing::warn!("Failed to download Telegram photo for {}: {:?}", &info.id, e);
                None
            }
        };

        // Update user
        UserRepository::set_telegram_info(
            &state.db,
            &user_id,
            &info.id,
            info.username.as_deref(),
            stored_photo_url.as_deref(),
        )
        .await?;

        Ok(())
    }

    /// Download and store Telegram profile photo
    pub async fn download_and_store_telegram_photo(
        state: &Arc<AppState>,
        telegram_user_id: &str,
        photo_url: Option<&str>,
    ) -> AppResult<Option<String>> {
        let client = reqwest::Client::new();
        let mut bytes_opt = None;
        let mut ext = String::from("jpg");

        // Try direct GET first
        if let Some(url) = photo_url {
            match client.get(url).send().await {
                Ok(resp) => {
                    if resp.status().is_success() {
                        let content_type = resp
                            .headers()
                            .get(reqwest::header::CONTENT_TYPE)
                            .and_then(|v| v.to_str().ok())
                            .unwrap_or("");
                        let mime = content_type.split(';').next().unwrap_or("");
                        if mime.starts_with("image/") {
                            let subtype = mime.split('/').nth(1).unwrap_or("jpeg");
                            ext = match subtype {
                                "jpeg" | "jpg" => "jpg".to_string(),
                                "png" => "png".to_string(),
                                "webp" => "webp".to_string(),
                                _ => "jpg".to_string(),
                            };
                            bytes_opt = Some(resp.bytes().await?);
                        }
                    }
                }
                Err(_) => {}
            }
        }

        // Try Bot API fallback
        if bytes_opt.is_none() {
            if let Some(bot_token) = state.config.telegram.bot_token.as_ref() {
                let photos_url = format!(
                    "https://api.telegram.org/bot{}/getUserProfilePhotos?user_id={}&limit=1",
                    bot_token, telegram_user_id
                );
                if let Ok(photos_resp) = client.get(&photos_url).send().await {
                    if photos_resp.status().is_success() {
                        let photos_json: serde_json::Value = photos_resp.json().await?;
                        if photos_json.get("ok").and_then(|v| v.as_bool()) == Some(true) {
                            if let Some(photos_arr) = photos_json["result"]["photos"].as_array() {
                                if !photos_arr.is_empty() {
                                    if let Some(sizes) = photos_arr[0].as_array() {
                                        if let Some(best) = sizes.last() {
                                            if let Some(file_id) = best["file_id"].as_str() {
                                                let get_file_url = format!(
                                                    "https://api.telegram.org/bot{}/getFile?file_id={}",
                                                    bot_token, file_id
                                                );
                                                if let Ok(file_resp) = client.get(&get_file_url).send().await {
                                                    if file_resp.status().is_success() {
                                                        let file_json: serde_json::Value = file_resp.json().await?;
                                                        if file_json.get("ok").and_then(|v| v.as_bool()) == Some(true) {
                                                            if let Some(file_path_raw) = file_json["result"]["file_path"].as_str() {
                                                                let file_url = format!(
                                                                    "https://api.telegram.org/file/bot{}/{}",
                                                                    bot_token, file_path_raw
                                                                );
                                                                if let Ok(final_resp) = client.get(&file_url).send().await {
                                                                    if final_resp.status().is_success() {
                                                                        let b = final_resp.bytes().await?;
                                                                        bytes_opt = Some(b);
                                                                        if let Some(found_ext) = file_path_raw.split('.').next_back() {
                                                                            ext = found_ext.to_string();
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let bytes = match bytes_opt {
            Some(b) => b,
            None => return Ok(None),
        };

        let dir = std::path::Path::new("data/telegram_photos");
        tokio::fs::create_dir_all(dir).await.map_err(|e| {
            AppError::Internal(anyhow::anyhow!("Failed to create photo directory: {}", e))
        })?;

        let filename = format!("{}.{}", telegram_user_id, ext);
        let path = dir.join(&filename);

        tokio::fs::write(&path, &bytes)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to write photo to disk: {}", e)))?;

        let base = state.config.server.webhook_url.trim_end_matches('/');
        let public_url = format!("{}/api/auth/telegram/photo/{}", base, telegram_user_id);

        Ok(Some(public_url))
    }

    /// Refresh Twitch access token
    pub async fn refresh_twitch_token(
        state: &Arc<AppState>,
        user_id: String,
    ) -> AppResult<chrono::DateTime<chrono::Utc>> {
        let user = UserRepository::find_by_id(&state.db, &user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        let token_response = state.twitch.refresh_token(&user.twitch_refresh_token).await?;
        let token_expires_at = TwitchService::calculate_token_expiry(token_response.expires_in);

        UserRepository::update_tokens(
            &state.db,
            &user_id,
            &token_response.access_token,
            &token_response.refresh_token,
            token_expires_at.naive_utc(),
        )
        .await?;

        // Spawn background task to sync subscriptions
        {
            let state = state.clone();
            let user = user.clone();
            tokio::spawn(async move {
                match SubscriptionManager::sync_for_user(&state, &user).await {
                    Ok(_) => tracing::info!("Synced EventSub subscriptions for user {}", user.id),
                    Err(e) => tracing::warn!(
                        "Failed to sync EventSub subscriptions for user {}: {:?}",
                        user.id,
                        e
                    ),
                }
            });
        }

        Ok(token_expires_at)
    }

    /// Unlink Telegram from user
    pub async fn unlink_telegram(state: &Arc<AppState>, user_id: String) -> AppResult<()> {
        let user = UserRepository::find_by_id(&state.db, &user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        // Remove integrations
        if let Some(telegram_id) = user.telegram_user_id.clone() {
            let integrations =
                crate::db::TelegramIntegrationRepository::find_by_chat_id(&state.db, &telegram_id).await?;
            for integration in integrations {
                if integration.user_id == user_id {
                    crate::db::TelegramIntegrationRepository::delete(&state.db, &integration.id).await?;
                }
            }
        }

        // Clear Telegram fields
        UserRepository::clear_telegram_info(&state.db, &user_id).await?;

        // Remove cached photos
        if let Some(telegram_id) = user.telegram_user_id {
            let dir = std::path::Path::new("data/telegram_photos");
            let exts = ["jpg", "jpeg", "png", "webp"];
            for ext in &exts {
                let p = dir.join(format!("{}.{}", telegram_id, ext));
                if tokio::fs::metadata(&p).await.is_ok() {
                    if let Err(e) = tokio::fs::remove_file(&p).await {
                        tracing::warn!("Failed to remove cached Telegram photo {}: {:?}", p.display(), e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Unlink Discord from user
    pub async fn unlink_discord(state: &Arc<AppState>, user_id: String) -> AppResult<()> {
        let user = UserRepository::find_by_id(&state.db, &user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        // Remove integrations
        if let Some(discord_id) = user.discord_user_id.clone() {
            let integrations =
                crate::db::DiscordIntegrationRepository::find_by_channel_id(&state.db, &discord_id).await?;
            for integration in integrations {
                if integration.user_id == user_id {
                    crate::db::DiscordIntegrationRepository::delete(&state.db, &integration.id).await?;
                }
            }
        }

        // Clear Discord fields
        UserRepository::clear_discord_info(&state.db, &user_id).await?;

        Ok(())
    }

    /// Generate random string
    pub fn generate_random_string(length: usize) -> String {
        use rand::Rng;
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        let mut rng = rand::thread_rng();
        (0..length)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }

    /// Check if redirect URL is safe
    fn is_safe_redirect(redirect: &str, frontend_base: &str) -> bool {
        if redirect.starts_with('/') && !redirect.starts_with("//") {
            return true;
        }
        if let Ok(u) = Url::parse(redirect) {
            if let Ok(front) = Url::parse(frontend_base) {
                return u.origin() == front.origin();
            }
        }
        false
    }

    /// Compose redirect URL for OAuth callback
    fn compose_redirect_url(frontend_base: &str, redirect_to: Option<&str>) -> String {
        let frontend = frontend_base.trim_end_matches('/');

        match redirect_to {
            Some(r) if !r.is_empty() => {
                if r.starts_with("http://") || r.starts_with("https://") {
                    r.to_string()
                } else if r.starts_with('/') {
                    format!("{}{}", frontend, r)
                } else {
                    format!("{}/{}", frontend, r)
                }
            }
            _ => format!("{}/auth/callback", frontend),
        }
    }
}
