use std::sync::Arc;

use crate::db::UserRepository;
use crate::error::AppError;
use crate::services::auth::AuthService;
use crate::services::twitch::TwitchService;
use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Redirect},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

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
// Handlers
// ============================================================================

/// Initiate Twitch OAuth login
async fn login(
    State(state): State<Arc<AppState>>,
    Query(query): Query<LoginQuery>,
) -> Result<impl IntoResponse, AppError> {
    let redirect_to = query.redirect_to.filter(|r| !r.is_empty());
    let lang = query
        .lang
        .filter(|l| !l.is_empty())
        .map(|l| crate::i18n::normalize_language(&l))
        .filter(|l| crate::i18n::is_supported_language(l.as_str()));

    let state_jwt = AuthService::generate_oauth_state(&state, redirect_to, lang)?;
    let scopes = TwitchService::get_required_scopes();
    let auth_url = state.twitch.get_auth_url(&state_jwt, &scopes);

    Ok(Redirect::to(&auth_url))
}

/// Handle Twitch OAuth callback
async fn callback(
    State(state): State<Arc<AppState>>,
    Query(query): Query<CallbackQuery>,
) -> Result<impl IntoResponse, AppError> {
    if let Some(error) = query.error {
        let description = query.error_description.unwrap_or_default();
        tracing::error!("OAuth error: {} - {}", error, description);
        return Err(AppError::BadRequest(format!(
            "OAuth error: {}",
            description
        )));
    }

    let code = query.code.ok_or_else(|| {
        tracing::error!("OAuth callback missing authorization code");
        AppError::BadRequest("Missing authorization code".to_string())
    })?;

    let state_encoded = query.state.ok_or_else(|| {
        tracing::error!("OAuth callback missing state parameter");
        AppError::BadRequest("Missing state parameter".to_string())
    })?;

    let oauth_state = AuthService::decode_oauth_state(&state, &state_encoded)?;
    let (redirect_url, user_id) =
        AuthService::handle_twitch_callback(&state, code, oauth_state).await?;

    tracing::info!("OAuth authentication successful for user: {}", user_id);
    Ok(Redirect::to(&redirect_url))
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
async fn telegram_link(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Json(request): Json<TelegramLoginRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Check if already linked
    if let Some(existing_tg_id) = &user.telegram_user_id {
        if existing_tg_id == &request.id {
            return Ok(Json(
                serde_json::json!({ "message": crate::i18n::t("telegram.already_linked") }),
            ));
        }
    }

    // Build payload map
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

    AuthService::handle_telegram_link(&state, user.id, payload).await?;

    Ok(Json(
        serde_json::json!({ "message": crate::i18n::t("telegram.linked") }),
    ))
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
async fn refresh_telegram_photo(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    let current = UserRepository::find_by_id(&state.db, &user.id)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    let tg_id = current
        .telegram_user_id
        .clone()
        .ok_or_else(|| AppError::BadRequest("Telegram not linked".to_string()))?;

    match AuthService::download_and_store_telegram_photo(
        &state,
        &tg_id,
        current.telegram_photo_url.as_deref(),
    )
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

/// Unlink Telegram from the current user's profile
async fn telegram_unlink(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    AuthService::unlink_telegram(&state, user.id).await?;
    Ok(Json(
        serde_json::json!({ "message": crate::i18n::t("telegram.unlinked") }),
    ))
}

async fn discord_link(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Query(query): Query<LoginQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let client_id =
        state.config.discord.client_id.as_ref().ok_or_else(|| {
            AppError::ServiceUnavailable("Discord OAuth not configured".to_string())
        })?;

    let redirect_to = query.redirect_to.filter(|r| !r.is_empty());
    let state_jwt = AuthService::generate_discord_oauth_state(&state, user.id, redirect_to)?;

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

    Ok(Json(serde_json::json!({ "url": auth_url })))
}

async fn discord_callback(
    State(state): State<Arc<AppState>>,
    Query(query): Query<CallbackQuery>,
) -> Result<impl IntoResponse, AppError> {
    if let Some(error) = query.error {
        let description = query.error_description.unwrap_or_default();
        tracing::error!("OAuth error: {} - {}", error, description);
        return Err(AppError::BadRequest(format!(
            "OAuth error: {}",
            description
        )));
    }

    let code = query.code.ok_or_else(|| {
        tracing::error!("OAuth callback missing authorization code");
        AppError::BadRequest("Missing authorization code".to_string())
    })?;

    let state_encoded = query.state.ok_or_else(|| {
        tracing::error!("OAuth callback missing state parameter");
        AppError::BadRequest("Missing state parameter".to_string())
    })?;

    let discord_state = AuthService::decode_discord_oauth_state(&state, &state_encoded)?;
    let redirect_url = AuthService::handle_discord_callback(&state, code, discord_state).await?;

    Ok(Redirect::to(&redirect_url))
}

async fn discord_unlink(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    AuthService::unlink_discord(&state, user.id).await?;
    Ok(Json(
        serde_json::json!({ "message": crate::i18n::t("discord.unlinked") }),
    ))
}

/// Refresh Twitch access token
async fn refresh_token(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    let expires_at = AuthService::refresh_twitch_token(&state, user.id).await?;
    Ok(Json(serde_json::json!({
        "message": crate::i18n::t("auth.token_refreshed"),
        "expires_at": expires_at.to_rfc3339()
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
    AuthService::get_user_from_token(state, token).await
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
