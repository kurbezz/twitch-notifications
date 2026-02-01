use std::sync::Arc;

use crate::db::{DiscordIntegrationRepository, TelegramIntegrationRepository, UserRepository};
use crate::error::AppError;
use crate::services::discord::{exchange_code_for_token, get_discord_user};
use crate::services::subscriptions::SubscriptionManager;
use crate::services::telegram::verify_telegram_login_payload;
use crate::services::twitch::TwitchService;
use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Redirect},
    routing::{get, post},
    Json, Router,
};
use chrono::{Duration, Utc};
use http;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use url::Url;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/login", get(login))
        .route("/callback", get(callback))
        .route("/me", get(me).put(update_me))
        .route("/refresh", post(refresh_token))
        .route("/logout", post(logout))
        .route("/telegram/link", post(telegram_link))
        .route("/telegram/unlink", post(telegram_unlink))
        .route("/telegram/photo/:id", get(get_telegram_photo))
        .route("/telegram/photo/refresh", post(refresh_telegram_photo))
        .route("/discord/link", get(discord_link))
        .route("/discord/callback", get(discord_callback))
        .route("/discord/unlink", post(discord_unlink))
}

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct LoginQuery {
    redirect_to: Option<String>,
    lang: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: String,
    pub twitch_id: String,
    pub twitch_login: String,
    pub twitch_display_name: String,
    pub twitch_profile_image_url: Option<String>,

    // Telegram fields (optional)
    pub telegram_user_id: Option<String>,
    pub telegram_username: Option<String>,
    pub telegram_photo_url: Option<String>,

    // Discord fields (optional)
    pub discord_user_id: Option<String>,
    pub discord_username: Option<String>,
    pub discord_avatar_url: Option<String>,

    // Preferred language (optional)
    pub lang: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TelegramLoginRequest {
    pub id: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub username: Option<String>,
    pub photo_url: Option<String>,
    pub auth_date: i64,
    pub hash: String,
}

// ============================================================================
// State for OAuth flow
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OAuthState {
    csrf_token: String,
    redirect_to: Option<String>,
    lang: Option<String>,
    iat: usize,
    exp: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DiscordOAuthState {
    csrf_token: String,
    user_id: String,
    redirect_to: Option<String>,
    iat: usize,
    exp: usize,
}

// ============================================================================
// Handlers
// ============================================================================

/// Initiate Twitch OAuth login
async fn login(
    State(state): State<Arc<AppState>>,
    Query(query): Query<LoginQuery>,
) -> Result<impl IntoResponse, AppError> {
    // Generate CSRF token
    let csrf_token = generate_random_string(32);

    // Build short-lived state claims (10 minutes)
    let now = Utc::now();
    let iat = now.timestamp() as usize;
    let exp = (now + Duration::minutes(10)).timestamp() as usize;

    // Default redirect_to to dashboard if not provided or if it's the current path
    let redirect_to = query.redirect_to.filter(|r| !r.is_empty());

    // Normalize and validate language selection (optional). We accept only supported
    // languages (e.g. 'ru', 'en') and trim any region suffix like 'en-US'.
    let lang = query
        .lang
        .filter(|l| !l.is_empty())
        .map(|l| crate::i18n::normalize_language(&l))
        .filter(|l| crate::i18n::is_supported_language(l.as_str()));

    let state_claims = OAuthState {
        csrf_token: csrf_token.clone(),
        redirect_to,
        lang,
        iat,
        exp,
    };

    // Sign state as a JWT so we don't need to set a CSRF cookie
    let state_jwt = encode(
        &Header::default(),
        &state_claims,
        &EncodingKey::from_secret(state.config.jwt.secret.as_bytes()),
    )?;

    // Get required scopes
    let scopes = TwitchService::get_required_scopes();

    // Generate auth URL with signed state
    let auth_url = state.twitch.get_auth_url(&state_jwt, &scopes);

    Ok(Redirect::to(&auth_url))
}

/// Handle Twitch OAuth callback
async fn callback(
    State(state): State<Arc<AppState>>,
    Query(query): Query<CallbackQuery>,
) -> Result<impl IntoResponse, AppError> {
    // Check for OAuth errors
    if let Some(error) = query.error {
        let description = query.error_description.unwrap_or_default();
        tracing::error!("OAuth error: {} - {}", error, description);
        return Err(AppError::BadRequest(format!(
            "OAuth error: {}",
            description
        )));
    }

    // Get authorization code
    let code = query.code.ok_or_else(|| {
        tracing::error!("OAuth callback missing authorization code");
        AppError::BadRequest("Missing authorization code".to_string())
    })?;

    // Get and validate state (signed JWT)
    let state_encoded = query.state.ok_or_else(|| {
        tracing::error!("OAuth callback missing state parameter");
        AppError::BadRequest("Missing state parameter".to_string())
    })?;

    let token_data = decode::<OAuthState>(
        &state_encoded,
        &DecodingKey::from_secret(state.config.jwt.secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| {
        tracing::error!("Failed to decode OAuth state: {:?}", e);
        e
    })?;
    let oauth_state = token_data.claims;

    tracing::debug!(
        "OAuth callback: code={}, redirect_to={:?}",
        code,
        oauth_state.redirect_to
    );

    // Exchange code for tokens
    let token_response = state.twitch.exchange_code(&code).await?;

    // Get user info from Twitch
    let twitch_user = state.twitch.get_user(&token_response.access_token).await?;

    // Calculate token expiry
    let token_expires_at = TwitchService::calculate_token_expiry(token_response.expires_in);

    // Create or update user (pass optional language so that new users get their browser lang)
    let user = UserRepository::upsert_by_twitch_id(
        &state.db,
        &twitch_user.id,
        &twitch_user.login,
        &twitch_user.display_name,
        &twitch_user.email.clone().unwrap_or_default(),
        &twitch_user.profile_image_url.clone().unwrap_or_default(),
        &token_response.access_token,
        &token_response.refresh_token,
        token_expires_at.naive_utc(),
        oauth_state.lang.as_deref(),
    )
    .await?;

    // Spawn background task to synchronize EventSub subscriptions for the user.
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

    // Create JWT token for the client (Bearer)
    let token = create_jwt(&state, &user.id)?;

    tracing::info!(
        "OAuth authentication successful for user: {} (twitch_id: {})",
        user.id,
        user.twitch_id
    );

    // Always redirect to /auth/callback on the frontend first
    // The token will be in the URL fragment for the callback page to extract and store
    // Then the callback page will redirect to the final destination (redirect_to or dashboard)
    let frontend_base = state.config.server.frontend_url.trim_end_matches('/');
    let callback_url = format!("{}/auth/callback", frontend_base);

    // Put token into URL fragment so frontend can retrieve it securely (fragment not sent to server)
    let token_enc: String = url::form_urlencoded::byte_serialize(token.as_bytes()).collect();
    let expires_at = (Utc::now() + Duration::hours(state.config.jwt.expiration_hours)).timestamp();

    // Include redirect_to in the fragment so AuthCallbackPage knows where to go after token extraction
    // Validate redirect_to to prevent open-redirects. Accept only:
    //  - absolute URLs with the same origin as configured frontend_url
    //  - relative paths starting with a single '/'
    fn is_safe_redirect(redirect: &str, frontend_base: &str) -> bool {
        // allow single-segment relative paths like '/dashboard' but not protocol-relative '//' URLs
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

    let raw_redirect = oauth_state.redirect_to.as_deref().unwrap_or("/dashboard");
    let safe_redirect = if is_safe_redirect(raw_redirect, &frontend_base) {
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

    tracing::debug!(
        "Redirecting to auth/callback at: {} with token expiring at: {}, final redirect: {}",
        callback_url,
        expires_at,
        final_redirect
    );

    Ok(Redirect::to(&redirect_with_fragment))
}

/// Logout - invalidate session
async fn logout(State(_state): State<Arc<AppState>>) -> Result<Json<serde_json::Value>, AppError> {
    // Currently this project uses stateless JWTs for auth. There's no server-side
    // session to clear by default, but exposing a `/logout` endpoint ensures the
    // frontend can call it without 404s and provides a place to implement server-
    // side invalidation (e.g. token blacklist) in the future if needed.
    Ok(Json(
        serde_json::json!({ "message": crate::i18n::t("auth.logged_out") }),
    ))
}

/// Link Telegram login (created via the Telegram Login Widget).
/// This endpoint verifies the Telegram payload using the configured bot token,
/// stores the Telegram user id on the authenticated user's profile, and creates
/// a `telegram_integrations` entry for the user's private chat.
async fn telegram_link(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Json(request): Json<TelegramLoginRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Ensure Telegram bot is configured on the server
    let bot_token =
        state.config.telegram.bot_token.as_ref().ok_or_else(|| {
            AppError::ServiceUnavailable("Telegram bot not configured".to_string())
        })?;

    // Build a payload map matching what Telegram sends so we can verify it
    let mut payload: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    payload.insert("id".to_string(), request.id.clone());
    if let Some(v) = &request.first_name {
        payload.insert("first_name".to_string(), v.clone());
    }
    if let Some(v) = &request.last_name {
        payload.insert("last_name".to_string(), v.clone());
    }
    if let Some(v) = &request.username {
        payload.insert("username".to_string(), v.clone());
    }
    if let Some(v) = &request.photo_url {
        payload.insert("photo_url".to_string(), v.clone());
    }
    payload.insert("auth_date".to_string(), request.auth_date.to_string());
    payload.insert("hash".to_string(), request.hash.clone());

    // Verify signature and parse the payload
    let info = verify_telegram_login_payload(&payload, bot_token)?;

    // Persist Telegram identity on the user's profile (so the app can easily tell if a user has linked Telegram)
    // If the user already has the same telegram_user_id set, treat this request as idempotent and return success.
    if let Some(existing_tg_id) = &user.telegram_user_id {
        if existing_tg_id == &info.id {
            return Ok(Json(
                serde_json::json!({ "message": crate::i18n::t("telegram.already_linked") }),
            ));
        }
    }

    // Try to download and store the photo locally (direct link or Bot API fallback).
    // The helper accepts an optional direct URL and will attempt Bot API fallback if needed.
    let stored_photo_url: Option<String> = match download_and_store_telegram_photo(
        &state,
        &info.id,
        info.photo_url.as_deref(),
    )
    .await
    {
        Ok(url_opt) => url_opt,
        Err(e) => {
            tracing::warn!(
                "Failed to download Telegram photo for {}: {:?}",
                &info.id,
                e
            );
            None
        }
    };

    UserRepository::set_telegram_info(
        &state.db,
        &user.id,
        &info.id,
        info.username.as_deref(),
        stored_photo_url.as_deref(),
    )
    .await?;

    Ok(Json(
        serde_json::json!({ "message": crate::i18n::t("telegram.linked") }),
    ))
}

/// Attempt to download a Telegram profile photo and store it locally.
/// On success returns Some(public_url) which points to `GET /api/auth/telegram/photo/:id`.
async fn download_and_store_telegram_photo(
    state: &Arc<AppState>,
    telegram_user_id: &str,
    photo_url: Option<&str>,
) -> crate::error::AppResult<Option<String>> {
    // Log attempt (if we were given a URL) and prepare HTTP client.
    if let Some(url) = photo_url {
        tracing::info!(
            "Attempting to download Telegram photo (direct): user_id={}, url={}",
            telegram_user_id,
            url
        );
    } else {
        tracing::info!(
            "No direct photo URL provided for {}; will try Bot API fallback",
            telegram_user_id
        );
    }
    let client = reqwest::Client::new();

    // Try direct GET first (only if a direct URL was provided), if it yields image bytes, use them.
    let mut bytes_opt = None;
    let mut ext = String::from("jpg");

    if let Some(url) = photo_url {
        match client.get(url).send().await {
            Ok(resp) => {
                tracing::info!("HTTP GET {} -> {}", url, resp.status());
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
                    } else {
                        tracing::warn!(
                            "Telegram photo has non-image content-type: {}",
                            content_type
                        );
                    }
                } else {
                    tracing::warn!(
                        "Failed to fetch Telegram photo (status: {}) for {}",
                        resp.status(),
                        telegram_user_id
                    );
                }
            }
            Err(e) => {
                tracing::warn!("Failed HTTP GET for Telegram photo URL {}: {:?}", url, e);
            }
        }
    }

    // If direct fetch didn't produce an image, attempt Bot API fallback (if configured).
    if bytes_opt.is_none() {
        if let Some(bot_token) = state.config.telegram.bot_token.as_ref() {
            tracing::info!(
                "Attempting Bot API fallback to fetch profile photo for {}",
                telegram_user_id
            );

            // getUserProfilePhotos
            let photos_url = format!(
                "https://api.telegram.org/bot{}/getUserProfilePhotos?user_id={}&limit=1",
                bot_token, telegram_user_id
            );
            if let Ok(photos_resp) = client.get(&photos_url).send().await {
                tracing::info!(
                    "getUserProfilePhotos {} -> {}",
                    photos_url,
                    photos_resp.status()
                );
                if photos_resp.status().is_success() {
                    let photos_json: serde_json::Value = photos_resp.json().await?;
                    if photos_json.get("ok").and_then(|v| v.as_bool()) == Some(true) {
                        if let Some(photos_arr) = photos_json["result"]["photos"].as_array() {
                            if !photos_arr.is_empty() {
                                if let Some(sizes) = photos_arr[0].as_array() {
                                    if let Some(best) = sizes.last() {
                                        if let Some(file_id) = best["file_id"].as_str() {
                                            // 2) getFile?file_id={file_id}
                                            let get_file_url = format!(
                                                "https://api.telegram.org/bot{}/getFile?file_id={}",
                                                bot_token, file_id
                                            );
                                            if let Ok(file_resp) =
                                                client.get(&get_file_url).send().await
                                            {
                                                tracing::info!(
                                                    "getFile {} -> {}",
                                                    get_file_url,
                                                    file_resp.status()
                                                );
                                                if file_resp.status().is_success() {
                                                    let file_json: serde_json::Value =
                                                        file_resp.json().await?;
                                                    tracing::info!("getFile json: {:?}", file_json);
                                                    if file_json.get("ok").and_then(|v| v.as_bool())
                                                        == Some(true)
                                                    {
                                                        if let Some(file_path_raw) = file_json
                                                            ["result"]["file_path"]
                                                            .as_str()
                                                        {
                                                            // 3) download file via https://api.telegram.org/file/bot<token>/<file_path>
                                                            let file_path =
                                                                file_path_raw.to_string();
                                                            let file_url = format!(
                                                                "https://api.telegram.org/file/bot{}/{}",
                                                                bot_token, file_path
                                                            );
                                                            if let Ok(final_resp) =
                                                                client.get(&file_url).send().await
                                                            {
                                                                tracing::info!(
                                                                    "GET {} -> {}",
                                                                    file_url,
                                                                    final_resp.status()
                                                                );
                                                                if final_resp.status().is_success()
                                                                {
                                                                    let b =
                                                                        final_resp.bytes().await?;
                                                                    bytes_opt = Some(b);
                                                                    if let Some(found_ext) =
                                                                        file_path.split('.').last()
                                                                    {
                                                                        ext = found_ext.to_string();
                                                                    }
                                                                    tracing::info!(
                                                                        "Downloaded Telegram photo via Bot API for {} from {}",
                                                                        telegram_user_id,
                                                                        file_url
                                                                    );
                                                                } else {
                                                                    tracing::warn!(
                                                                        "Failed to download Telegram file from {} -> {}",
                                                                        file_url,
                                                                        final_resp.status()
                                                                    );
                                                                }
                                                            } else {
                                                                tracing::warn!("HTTP request to file URL {} failed", file_url);
                                                            }
                                                        }
                                                    }
                                                } else {
                                                    tracing::warn!(
                                                        "getFile returned non-success: {}",
                                                        file_resp.status()
                                                    );
                                                }
                                            } else {
                                                tracing::warn!(
                                                    "Failed to call getFile for file_id {}",
                                                    file_id
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        tracing::warn!(
                            "getUserProfilePhotos responded with ok=false or no photos: {}",
                            photos_json
                        );
                    }
                } else {
                    tracing::warn!(
                        "getUserProfilePhotos API call failed: {}",
                        photos_resp.status()
                    );
                }
            } else {
                tracing::warn!("Failed to call getUserProfilePhotos via Bot API");
            }
        } else {
            tracing::info!("Telegram bot not configured; skipping Bot API fallback");
        }
    }

    // If after both attempts we still have no bytes, give up.
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

    tracing::info!(
        "Saved Telegram photo for {} -> {}",
        telegram_user_id,
        path.display()
    );

    let base = state.config.server.webhook_url.trim_end_matches('/');
    let public_url = format!("{}/api/auth/telegram/photo/{}", base, telegram_user_id);

    Ok(Some(public_url))
}

/// Serve a previously saved Telegram profile photo (if present).
async fn get_telegram_photo(Path(user_id): Path<String>) -> Result<impl IntoResponse, AppError> {
    let dir = std::path::Path::new("data/telegram_photos");
    let candidates = ["jpg", "jpeg", "png", "webp"];
    for ext in &candidates {
        let p = dir.join(format!("{}.{}", user_id, ext));
        if tokio::fs::metadata(&p).await.is_ok() {
            let data = tokio::fs::read(&p).await.map_err(|e| {
                AppError::Internal(anyhow::anyhow!("Failed to read photo file: {}", e))
            })?;
            let content_type = match *ext {
                "jpg" | "jpeg" => "image/jpeg",
                "png" => "image/png",
                "webp" => "image/webp",
                _ => "application/octet-stream",
            };
            let resp = (
                http::StatusCode::OK,
                [(http::header::CONTENT_TYPE, content_type)],
                data,
            );
            return Ok(resp.into_response());
        }
    }
    Err(AppError::NotFound(
        "Telegram profile photo not found".to_string(),
    ))
}

/// Refresh the current authenticated user's Telegram profile photo (best-effort).
/// Attempts to (re)download the `telegram_photo_url` and cache it locally.
async fn refresh_telegram_photo(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    // Load current user record
    let current = UserRepository::find_by_id(&state.db, &user.id)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    // Ensure Telegram is linked
    let tg_id = current
        .telegram_user_id
        .clone()
        .ok_or_else(|| AppError::BadRequest("Telegram not linked".to_string()))?;

    // Attempt to refresh using either existing photo_url (if any) or Bot API fallback.
    match download_and_store_telegram_photo(&state, &tg_id, current.telegram_photo_url.as_deref())
        .await
    {
        Ok(Some(public_url)) => {
            UserRepository::set_telegram_info(
                &state.db,
                &user.id,
                &tg_id,
                current.telegram_username.as_deref(),
                Some(public_url.as_str()),
            )
            .await?;
            Ok(Json(serde_json::json!({ "photo_url": public_url })))
        }
        Ok(None) => {
            // Informative message depending on whether a bot token exists
            if state.config.telegram.bot_token.is_none() {
                Err(AppError::BadRequest(crate::i18n::t(
                    "error.refresh_telegram_photo.download_failed",
                )))
            } else {
                Err(AppError::NotFound(crate::i18n::t(
                    "error.refresh_telegram_photo.not_found",
                )))
            }
        }
        Err(e) => {
            tracing::warn!("Error refreshing Telegram photo for {}: {:?}", tg_id, e);
            Err(AppError::ServiceUnavailable(crate::i18n::t(
                "error.refresh_telegram_photo.service_unavailable",
            )))
        }
    }
}

/// Get current user info
async fn me(
    State(_state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
) -> Result<Json<UserResponse>, AppError> {
    let profile_image = if user.twitch_profile_image_url.is_empty() {
        None
    } else {
        Some(user.twitch_profile_image_url.clone())
    };

    Ok(Json(UserResponse {
        id: user.id,
        twitch_id: user.twitch_id,
        twitch_login: user.twitch_login,
        twitch_display_name: user.twitch_display_name,
        twitch_profile_image_url: profile_image,
        telegram_user_id: user.telegram_user_id.clone(),
        telegram_username: user.telegram_username.clone(),
        telegram_photo_url: user.telegram_photo_url.clone(),
        discord_user_id: user.discord_user_id.clone(),
        discord_username: user.discord_username.clone(),
        discord_avatar_url: user.discord_avatar_url.clone(),
        lang: user.lang.clone(),
    }))
}

#[derive(Debug, Deserialize)]
pub struct UpdateMeRequest {
    pub lang: Option<String>,
}

/// Update current user's profile (e.g. preferred language)
async fn update_me(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Json(request): Json<UpdateMeRequest>,
) -> Result<Json<UserResponse>, AppError> {
    // Persist language preference (may be None to clear)
    // Normalize and validate requested language before persisting. If an unsupported
    // language is provided, return a BadRequest error with a localized message.
    let lang_to_set: Option<String> = match request.lang {
        Some(lang_str) => {
            let normalized = crate::i18n::normalize_language(&lang_str);
            if !crate::i18n::is_supported_language(&normalized) {
                return Err(AppError::BadRequest(crate::i18n::t_with(
                    "error.unsupported_language",
                    &[("lang", &normalized)],
                )));
            }
            Some(normalized)
        }
        None => None,
    };
    UserRepository::set_lang(&state.db, &user.id, lang_to_set.as_deref()).await?;

    // Reload user
    let updated = UserRepository::find_by_id(&state.db, &user.id)
        .await?
        .ok_or_else(|| AppError::NotFound(crate::i18n::t("not_found.user")))?;

    let profile_image = if updated.twitch_profile_image_url.is_empty() {
        None
    } else {
        Some(updated.twitch_profile_image_url.clone())
    };

    Ok(Json(UserResponse {
        id: updated.id,
        twitch_id: updated.twitch_id,
        twitch_login: updated.twitch_login,
        twitch_display_name: updated.twitch_display_name,
        twitch_profile_image_url: profile_image,
        telegram_user_id: updated.telegram_user_id.clone(),
        telegram_username: updated.telegram_username.clone(),
        telegram_photo_url: updated.telegram_photo_url.clone(),
        discord_user_id: updated.discord_user_id.clone(),
        discord_username: updated.discord_username.clone(),
        discord_avatar_url: updated.discord_avatar_url.clone(),
        lang: updated.lang.clone(),
    }))
}

/// Unlink Telegram from the current user's profile and remove the associated private integration (if present).
async fn telegram_unlink(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    // Load current user record
    let current = UserRepository::find_by_id(&state.db, &user.id)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    // If the user had a Telegram user id recorded, remove any private integration for it (owned by this user).
    if let Some(telegram_id) = current.telegram_user_id.clone() {
        let integrations =
            TelegramIntegrationRepository::find_by_chat_id(&state.db, &telegram_id).await?;
        for integration in integrations {
            if integration.user_id == user.id {
                TelegramIntegrationRepository::delete(&state.db, &integration.id).await?;
            }
        }
    }

    // Clear Telegram fields on the user's profile
    UserRepository::clear_telegram_info(&state.db, &user.id).await?;

    // Remove any cached Telegram profile photos we may have stored when linking.
    // We attempt to delete files by common image extensions and ignore errors.
    if let Some(telegram_id) = current.telegram_user_id.clone() {
        let dir = std::path::Path::new("data/telegram_photos");
        let exts = ["jpg", "jpeg", "png", "webp"];
        for ext in &exts {
            let p = dir.join(format!("{}.{}", telegram_id, ext));
            match tokio::fs::metadata(&p).await {
                Ok(_) => {
                    if let Err(e) = tokio::fs::remove_file(&p).await {
                        tracing::warn!(
                            "Failed to remove cached Telegram photo {}: {:?}",
                            p.display(),
                            e
                        );
                    }
                }
                Err(_) => {
                    // file doesn't exist - nothing to do
                }
            }
        }
    }

    Ok(Json(
        serde_json::json!({ "message": crate::i18n::t("telegram.unlinked") }),
    ))
}

async fn discord_link(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Query(query): Query<LoginQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Ensure Discord OAuth is configured on the server
    let client_id =
        state.config.discord.client_id.as_ref().ok_or_else(|| {
            AppError::ServiceUnavailable("Discord OAuth not configured".to_string())
        })?;

    // Generate CSRF token
    let csrf_token = generate_random_string(32);

    // Build short-lived state claims (10 minutes)
    let now = Utc::now();
    let iat = now.timestamp() as usize;
    let exp = (now + Duration::minutes(10)).timestamp() as usize;

    let redirect_to = query.redirect_to.filter(|r| !r.is_empty());

    let state_claims = DiscordOAuthState {
        csrf_token: csrf_token.clone(),
        user_id: user.id.clone(),
        redirect_to,
        iat,
        exp,
    };

    // Sign state as a JWT so we don't need to set a CSRF cookie
    let state_jwt = encode(
        &Header::default(),
        &state_claims,
        &EncodingKey::from_secret(state.config.jwt.secret.as_bytes()),
    )?;

    // Build backend callback url (Discord must call back to our server)
    let callback_url = format!(
        "{}/api/auth/discord/callback",
        state.config.server.webhook_url.trim_end_matches('/')
    );

    let auth_url = format!(
        "https://discord.com/api/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&scope=identify&state={}",
        urlencoding::encode(client_id),
        urlencoding::encode(&callback_url),
        urlencoding::encode(&state_jwt)
    );

    // Return the auth URL as JSON so the frontend can use XHR/fetch and then redirect.
    Ok(Json(serde_json::json!({ "url": auth_url })))
}

async fn discord_callback(
    State(state): State<Arc<AppState>>,
    Query(query): Query<CallbackQuery>,
) -> Result<impl IntoResponse, AppError> {
    // Check for OAuth errors
    if let Some(error) = query.error {
        let description = query.error_description.unwrap_or_default();
        tracing::error!("OAuth error: {} - {}", error, description);
        return Err(AppError::BadRequest(format!(
            "OAuth error: {}",
            description
        )));
    }

    // Get authorization code
    let code = query.code.ok_or_else(|| {
        tracing::error!("OAuth callback missing authorization code");
        AppError::BadRequest("Missing authorization code".to_string())
    })?;

    // Get and validate state (signed JWT)
    let state_encoded = query.state.ok_or_else(|| {
        tracing::error!("OAuth callback missing state parameter");
        AppError::BadRequest("Missing state parameter".to_string())
    })?;

    let token_data = decode::<DiscordOAuthState>(
        &state_encoded,
        &DecodingKey::from_secret(state.config.jwt.secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| {
        tracing::error!("Failed to decode Discord OAuth state: {:?}", e);
        e
    })?;
    let discord_state = token_data.claims;

    // Ensure Discord OAuth client credentials are configured
    let client_id =
        state.config.discord.client_id.as_ref().ok_or_else(|| {
            AppError::ServiceUnavailable("Discord OAuth not configured".to_string())
        })?;
    let client_secret =
        state.config.discord.client_secret.as_ref().ok_or_else(|| {
            AppError::ServiceUnavailable("Discord OAuth not configured".to_string())
        })?;

    // Exchange code for token
    let callback_url = format!(
        "{}/api/auth/discord/callback",
        state.config.server.webhook_url.trim_end_matches('/')
    );
    let token_resp =
        exchange_code_for_token(client_id, client_secret, &code, &callback_url).await?;

    // Fetch user info using the returned access token
    let discord_user = get_discord_user(&token_resp.access_token).await?;

    // Compose friendly username and avatar url (if available)
    let username = format!("{}#{}", discord_user.username, discord_user.discriminator);
    let avatar_url = if let Some(avatar) = discord_user.avatar {
        format!(
            "https://cdn.discordapp.com/avatars/{}/{}.png",
            discord_user.id, avatar
        )
    } else {
        String::new()
    };

    // Persist Discord identity on the owner's profile
    UserRepository::set_discord_info(
        &state.db,
        &discord_state.user_id,
        &discord_user.id,
        &username,
        &avatar_url,
    )
    .await?;

    // Redirect user back to frontend (respect redirect_to if provided)
    let redirect_url = compose_redirect_url(
        &state.config.server.frontend_url,
        discord_state.redirect_to.as_deref(),
    );
    Ok(Redirect::to(&redirect_url))
}

async fn discord_unlink(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    // Load current user record
    let current = UserRepository::find_by_id(&state.db, &user.id)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    // If the user had a Discord user id recorded, remove any integration representing a personal channel for it (owned by this user).
    if let Some(discord_id) = current.discord_user_id.clone() {
        let integrations =
            DiscordIntegrationRepository::find_by_channel_id(&state.db, &discord_id).await?;
        for integration in integrations {
            if integration.user_id == user.id {
                DiscordIntegrationRepository::delete(&state.db, &integration.id).await?;
            }
        }
    }

    // Clear Discord fields on the user's profile
    UserRepository::clear_discord_info(&state.db, &user.id).await?;

    Ok(Json(
        serde_json::json!({ "message": crate::i18n::t("discord.unlinked") }),
    ))
}

/// Refresh Twitch access token
async fn refresh_token(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    // Refresh the token
    let token_response = state
        .twitch
        .refresh_token(&user.twitch_refresh_token)
        .await?;

    // Calculate new expiry
    let token_expires_at = TwitchService::calculate_token_expiry(token_response.expires_in);

    // Update user tokens
    UserRepository::update_tokens(
        &state.db,
        &user.id,
        &token_response.access_token,
        &token_response.refresh_token,
        token_expires_at.naive_utc(),
    )
    .await?;

    // Spawn background task to synchronize EventSub subscriptions for the user after token refresh
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

    Ok(Json(serde_json::json!({
        "message": crate::i18n::t("auth.token_refreshed"),
        "expires_at": token_expires_at.to_rfc3339()
    })))
}

// ============================================================================
// Helper functions
// ============================================================================

/// Get current user from a bearer token string
pub async fn get_user_from_token(
    state: &Arc<AppState>,
    token: &str,
) -> Result<crate::db::User, AppError> {
    let claims = decode_jwt(state, token)?;
    let user = UserRepository::find_by_id(&state.db, &claims.sub)
        .await?
        .ok_or(AppError::Unauthorized)?;
    Ok(user)
}

/// Generate a random string of specified length
fn generate_random_string(length: usize) -> String {
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

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
}

/// Create a signed JWT for a user id
fn create_jwt(state: &Arc<AppState>, user_id: &str) -> Result<String, AppError> {
    let now = Utc::now();
    let exp = now + Duration::hours(state.config.jwt.expiration_hours);
    let claims = Claims {
        sub: user_id.to_string(),
        iat: now.timestamp() as usize,
        exp: exp.timestamp() as usize,
    };

    let header = Header::default();
    let token = encode(
        &header,
        &claims,
        &EncodingKey::from_secret(state.config.jwt.secret.as_bytes()),
    )?;
    Ok(token)
}

/// Decode and validate a JWT, returning the claims
fn decode_jwt(state: &Arc<AppState>, token: &str) -> Result<Claims, AppError> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(state.config.jwt.secret.as_bytes()),
        &Validation::default(),
    )?;
    Ok(token_data.claims)
}

/// Compose a redirect URL for OAuth callback.
///
/// Rules:
/// - If `redirect_to` is `None` or empty, redirect to `/auth/callback` on the frontend.
/// - If `redirect_to` starts with `http://` or `https://`, treat it as an absolute URL and return it.
/// - Otherwise treat `redirect_to` as a path and join it to the `frontend_base`.
fn compose_redirect_url(frontend_base: &str, redirect_to: Option<&str>) -> String {
    let frontend = frontend_base.trim_end_matches('/');

    match redirect_to {
        Some(r) if !r.is_empty() => {
            if r.starts_with("http://") || r.starts_with("https://") {
                r.to_string()
            } else if r.starts_with('/') {
                // For relative paths, redirect to that path instead of auth/callback
                // This allows customizable post-login redirects
                format!("{}{}", frontend, r)
            } else {
                format!("{}/{}", frontend, r)
            }
        }
        // Default: redirect to auth/callback on the frontend
        // The AuthCallbackPage component will extract the token from the URL fragment
        _ => format!("{}/auth/callback", frontend),
    }
}

// ============================================================================
// Auth Middleware / Extractor
// ============================================================================

use axum::{async_trait, extract::FromRequestParts, http::request::Parts};

/// Extractor for authenticated user
pub struct AuthUser(pub crate::db::User);

#[async_trait]
impl FromRequestParts<Arc<AppState>> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        // Extract Authorization header (Bearer token)
        let auth_header = parts
            .headers
            .get(http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| {
                tracing::debug!("Missing or invalid Authorization header");
                AppError::Unauthorized
            })?;

        if !auth_header.to_ascii_lowercase().starts_with("bearer ") {
            tracing::debug!("Authorization header doesn't start with 'Bearer '");
            return Err(AppError::Unauthorized);
        }

        let token = auth_header[7..].trim();
        if token.is_empty() {
            tracing::debug!("Empty bearer token in Authorization header");
            return Err(AppError::Unauthorized);
        }

        let user = get_user_from_token(state, token).await.map_err(|e| {
            tracing::debug!("Failed to get user from token: {:?}", e);
            e
        })?;

        tracing::debug!("Authenticated user: {}", user.id);
        Ok(AuthUser(user))
    }
}
