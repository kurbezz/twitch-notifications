use std::sync::Arc;

use axum::{
    extract::{Path, State},
    routing::{delete, get, put},
    Json, Router,
};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::db::{
    DiscordIntegrationRepository, NotificationSettings, NotificationSettingsRepository,
    SettingsShareRepository, TelegramIntegrationRepository, UpdateNotificationSettings,
    UserRepository,
};
use crate::error::{AppError, AppResult};
use crate::routes::auth::AuthUser;
use crate::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(get_settings).put(update_settings))
        .route("/messages", get(get_messages).put(update_messages))
        .route("/reset", put(reset_to_defaults).post(reset_to_defaults))
        // Shared settings endpoints
        .route("/shared", get(list_shared).post(create_share))
        .route("/shared/incoming", get(list_incoming_shared))
        .route(
            "/shared/:grantee_id",
            delete(revoke_share).put(update_share),
        )
        // Access another user's settings (requires an active share)
        // NOTE: specific sub-routes must come before the generic "/:user_id" route
        .route(
            "/:user_id/messages",
            get(get_messages_for_user).put(update_messages_for_user),
        )
        .route("/:user_id/reset", put(reset_to_defaults_for_user))
        .route(
            "/:user_id",
            get(get_settings_for_user).put(update_settings_for_user),
        )
}

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Serialize)]
pub struct MessagesResponse {
    pub stream_online_message: String,
    pub stream_offline_message: String,
    pub stream_title_change_message: String,
    pub stream_category_change_message: String,
    pub reward_redemption_message: String,
    pub placeholders: PlaceholdersInfo,
}

#[derive(Debug, Serialize)]
pub struct PlaceholdersInfo {
    pub stream: Vec<PlaceholderInfo>,
    pub reward: Vec<PlaceholderInfo>,
}

#[derive(Debug, Serialize)]
pub struct PlaceholderInfo {
    pub name: String,
    pub description: String,
    pub example: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMessagesRequest {
    #[serde(alias = "stream_online_message")]
    pub stream_online_message: Option<String>,
    #[serde(alias = "stream_offline_message")]
    pub stream_offline_message: Option<String>,
    #[serde(alias = "stream_title_change_message")]
    pub stream_title_change_message: Option<String>,
    #[serde(alias = "stream_category_change_message")]
    pub stream_category_change_message: Option<String>,
    pub reward_redemption_message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UserSettingsResponse {
    pub id: String,
    pub user_id: String,
    pub stream_online_message: String,
    pub stream_offline_message: String,
    pub stream_title_change_message: String,
    pub stream_category_change_message: String,
    pub reward_redemption_message: String,
    pub notify_stream_online: bool,
    pub notify_stream_offline: bool,
    pub notify_title_change: bool,
    pub notify_category_change: bool,
    pub notify_reward_redemption: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSettingsRequest {
    #[allow(dead_code)] // Kept for API compatibility, but only notify_reward_redemption is used
    pub notify_stream_online: Option<bool>,
    #[allow(dead_code)] // Kept for API compatibility, but only notify_reward_redemption is used
    pub notify_stream_offline: Option<bool>,
    #[allow(dead_code)] // Kept for API compatibility, but only notify_reward_redemption is used
    pub notify_title_change: Option<bool>,
    #[allow(dead_code)] // Kept for API compatibility, but only notify_reward_redemption is used
    pub notify_category_change: Option<bool>,
    pub notify_reward_redemption: Option<bool>,
}

// ============================================================================
// Handlers
// ============================================================================

/// Get notification message templates
async fn get_messages(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
) -> AppResult<Json<MessagesResponse>> {
    let settings = NotificationSettingsRepository::get_or_create(&state.db, &user.id).await?;

    Ok(Json(MessagesResponse {
        stream_online_message: settings.stream_online_message,
        stream_offline_message: settings.stream_offline_message,
        stream_title_change_message: settings.stream_title_change_message,
        stream_category_change_message: settings.stream_category_change_message,
        reward_redemption_message: settings.reward_redemption_message,
        placeholders: get_placeholders_info(),
    }))
}

/// Update notification message templates
async fn update_messages(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Json(request): Json<UpdateMessagesRequest>,
) -> AppResult<Json<MessagesResponse>> {
    // Normalize placeholders (convert {{...}} -> {...}) and validate messages
    let stream_online_message = request
        .stream_online_message
        .as_ref()
        .map(|m| normalize_placeholders(m));
    let stream_offline_message = request
        .stream_offline_message
        .as_ref()
        .map(|m| normalize_placeholders(m));
    let stream_title_change_message = request
        .stream_title_change_message
        .as_ref()
        .map(|m| normalize_placeholders(m));
    let stream_category_change_message = request
        .stream_category_change_message
        .as_ref()
        .map(|m| normalize_placeholders(m));
    let reward_redemption_message = request
        .reward_redemption_message
        .as_ref()
        .map(|m| normalize_placeholders(m));

    if let Some(ref msg) = stream_online_message {
        validate_message(msg, "stream_online")?;
    }
    if let Some(ref msg) = stream_offline_message {
        validate_message(msg, "stream_offline")?;
    }
    if let Some(ref msg) = stream_title_change_message {
        validate_message(msg, "stream_title_change")?;
    }
    if let Some(ref msg) = stream_category_change_message {
        validate_message(msg, "stream_category")?;
    }
    if let Some(ref msg) = reward_redemption_message {
        validate_message(msg, "reward_redemption")?;
    }

    let update = UpdateNotificationSettings {
        stream_online_message,
        stream_offline_message,
        stream_title_change_message,
        stream_category_change_message,
        reward_redemption_message,
        notify_reward_redemption: None,
    };

    let settings = NotificationSettingsRepository::update(&state.db, &user.id, update).await?;

    Ok(Json(MessagesResponse {
        stream_online_message: settings.stream_online_message,
        stream_offline_message: settings.stream_offline_message,
        stream_title_change_message: settings.stream_title_change_message,
        stream_category_change_message: settings.stream_category_change_message,
        reward_redemption_message: settings.reward_redemption_message,
        placeholders: get_placeholders_info(),
    }))
}

/// Reset settings to defaults
async fn reset_to_defaults(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
) -> AppResult<Json<MessagesResponse>> {
    let defaults = NotificationSettings::default();

    let update = UpdateNotificationSettings {
        stream_online_message: Some(defaults.stream_online_message.clone()),
        stream_offline_message: Some(defaults.stream_offline_message.clone()),
        stream_title_change_message: Some(defaults.stream_title_change_message.clone()),
        stream_category_change_message: Some(defaults.stream_category_change_message.clone()),
        reward_redemption_message: Some(defaults.reward_redemption_message.clone()),
        notify_reward_redemption: Some(defaults.notify_reward_redemption),
    };

    let settings = NotificationSettingsRepository::update(&state.db, &user.id, update).await?;

    Ok(Json(MessagesResponse {
        stream_online_message: settings.stream_online_message,
        stream_offline_message: settings.stream_offline_message,
        stream_title_change_message: settings.stream_title_change_message,
        stream_category_change_message: settings.stream_category_change_message,
        reward_redemption_message: settings.reward_redemption_message,
        placeholders: get_placeholders_info(),
    }))
}

/// Get full user settings (messages + aggregated notify flags)
async fn get_settings(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
) -> AppResult<Json<UserSettingsResponse>> {
    let settings = NotificationSettingsRepository::get_or_create(&state.db, &user.id).await?;

    let telegrams = TelegramIntegrationRepository::find_by_user_id(&state.db, &user.id).await?;
    let discords = DiscordIntegrationRepository::find_by_user_id(&state.db, &user.id).await?;

    let notify_stream_online = telegrams.iter().any(|i| i.notify_stream_online)
        || discords.iter().any(|i| i.notify_stream_online);
    let notify_stream_offline = telegrams.iter().any(|i| i.notify_stream_offline)
        || discords.iter().any(|i| i.notify_stream_offline);
    let notify_title_change = telegrams.iter().any(|i| i.notify_title_change)
        || discords.iter().any(|i| i.notify_title_change);
    let notify_category_change = telegrams.iter().any(|i| i.notify_category_change)
        || discords.iter().any(|i| i.notify_category_change);
    // Use the user-level setting for reward redemptions (persisted in user_settings).
    let notify_reward_redemption = settings.notify_reward_redemption;

    Ok(Json(UserSettingsResponse {
        id: settings.id,
        user_id: settings.user_id,
        stream_online_message: settings.stream_online_message,
        stream_offline_message: settings.stream_offline_message,
        stream_title_change_message: settings.stream_title_change_message,
        stream_category_change_message: settings.stream_category_change_message,
        reward_redemption_message: settings.reward_redemption_message,
        notify_stream_online,
        notify_stream_offline,
        notify_title_change,
        notify_category_change,
        notify_reward_redemption,
        created_at: settings.created_at,
        updated_at: settings.updated_at,
    }))
}

/// Update chat bot settings (notify_reward_redemption) - does NOT affect integrations
async fn update_settings(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Json(request): Json<UpdateSettingsRequest>,
) -> AppResult<Json<UserSettingsResponse>> {
    // Only update chat bot settings - integrations are managed separately
    // notify_reward_redemption controls chat bot notifications only
    let update_ns = UpdateNotificationSettings {
        stream_online_message: None,
        stream_offline_message: None,
        stream_title_change_message: None,
        stream_category_change_message: None,
        reward_redemption_message: None,
        notify_reward_redemption: request.notify_reward_redemption,
    };
    NotificationSettingsRepository::update(&state.db, &user.id, update_ns).await?;

    // Return the updated settings
    get_settings(State(state), AuthUser(user)).await
}

/// Get full settings for another user (requires share)
async fn get_settings_for_user(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Path(owner_id): Path<String>,
) -> AppResult<Json<UserSettingsResponse>> {
    // If requesting own settings, delegate to existing handler
    if owner_id == user.id {
        return get_settings(State(state), AuthUser(user)).await;
    }

    // Must be shared
    let share =
        SettingsShareRepository::find_by_owner_and_grantee(&state.db, &owner_id, &user.id).await?;
    if share.is_none() {
        tracing::warn!(
            "Access denied: user {} attempted to view settings of owner {} without share",
            user.id,
            owner_id
        );
        return Err(AppError::Forbidden);
    }

    let settings = NotificationSettingsRepository::get_or_create(&state.db, &owner_id)
        .await
        .map_err(|e| {
            tracing::error!(
                "Failed to get/create settings for owner {} requested by {}: {:?}",
                owner_id,
                user.id,
                e
            );
            e
        })?;

    let telegrams = TelegramIntegrationRepository::find_by_user_id(&state.db, &owner_id).await?;
    let discords = DiscordIntegrationRepository::find_by_user_id(&state.db, &owner_id).await?;

    let notify_stream_online = telegrams.iter().any(|i| i.notify_stream_online)
        || discords.iter().any(|i| i.notify_stream_online);
    let notify_stream_offline = telegrams.iter().any(|i| i.notify_stream_offline)
        || discords.iter().any(|i| i.notify_stream_offline);
    let notify_title_change = telegrams.iter().any(|i| i.notify_title_change)
        || discords.iter().any(|i| i.notify_title_change);
    let notify_category_change = telegrams.iter().any(|i| i.notify_category_change)
        || discords.iter().any(|i| i.notify_category_change);
    let notify_reward_redemption = settings.notify_reward_redemption;

    Ok(Json(UserSettingsResponse {
        id: settings.id,
        user_id: settings.user_id,
        stream_online_message: settings.stream_online_message,
        stream_offline_message: settings.stream_offline_message,
        stream_title_change_message: settings.stream_title_change_message,
        stream_category_change_message: settings.stream_category_change_message,
        reward_redemption_message: settings.reward_redemption_message,
        notify_stream_online,
        notify_stream_offline,
        notify_title_change,
        notify_category_change,
        notify_reward_redemption,
        created_at: settings.created_at,
        updated_at: settings.updated_at,
    }))
}

/// Update chat bot settings for another user (requires manage rights) - does NOT affect integrations
async fn update_settings_for_user(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Path(owner_id): Path<String>,
    Json(request): Json<UpdateSettingsRequest>,
) -> AppResult<Json<UserSettingsResponse>> {
    if owner_id == user.id {
        return update_settings(State(state), AuthUser(user), Json(request)).await;
    }

    // Ensure shared with manage rights
    let share =
        SettingsShareRepository::find_by_owner_and_grantee(&state.db, &owner_id, &user.id).await?;
    match share {
        Some(s) if s.can_manage => {}
        _ => return Err(AppError::Forbidden),
    }

    // Only update chat bot settings - integrations are managed separately
    // notify_reward_redemption controls chat bot notifications only
    let update_ns = UpdateNotificationSettings {
        stream_online_message: None,
        stream_offline_message: None,
        stream_title_change_message: None,
        stream_category_change_message: None,
        reward_redemption_message: None,
        notify_reward_redemption: request.notify_reward_redemption,
    };
    let settings = NotificationSettingsRepository::update(&state.db, &owner_id, update_ns).await?;

    // Re-fetch integrations to compute current aggregated notify flags after per-integration updates
    let telegrams = TelegramIntegrationRepository::find_by_user_id(&state.db, &owner_id).await?;
    let discords = DiscordIntegrationRepository::find_by_user_id(&state.db, &owner_id).await?;

    let notify_stream_online = telegrams.iter().any(|i| i.notify_stream_online)
        || discords.iter().any(|i| i.notify_stream_online);
    let notify_stream_offline = telegrams.iter().any(|i| i.notify_stream_offline)
        || discords.iter().any(|i| i.notify_stream_offline);
    let notify_title_change = telegrams.iter().any(|i| i.notify_title_change)
        || discords.iter().any(|i| i.notify_title_change);
    let notify_category_change = telegrams.iter().any(|i| i.notify_category_change)
        || discords.iter().any(|i| i.notify_category_change);
    let notify_reward_redemption = settings.notify_reward_redemption;

    Ok(Json(UserSettingsResponse {
        id: settings.id,
        user_id: settings.user_id,
        stream_online_message: settings.stream_online_message,
        stream_offline_message: settings.stream_offline_message,
        stream_title_change_message: settings.stream_title_change_message,
        stream_category_change_message: settings.stream_category_change_message,
        reward_redemption_message: settings.reward_redemption_message,
        notify_stream_online,
        notify_stream_offline,
        notify_title_change,
        notify_category_change,
        notify_reward_redemption,
        created_at: settings.created_at,
        updated_at: settings.updated_at,
    }))
}

/// List users the current user has shared their settings with
async fn list_shared(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
) -> AppResult<Json<Vec<SharedUserResponse>>> {
    let rows = SettingsShareRepository::list_with_grantee_info(&state.db, &user.id).await?;
    let resp: Vec<SharedUserResponse> = rows
        .into_iter()
        .map(
            |(share, grantee_login, grantee_display)| SharedUserResponse {
                grantee_user_id: share.grantee_user_id,
                grantee_login,
                grantee_display_name: grantee_display,
                can_manage: share.can_manage,
                created_at: share.created_at,
                updated_at: share.updated_at,
            },
        )
        .collect();
    Ok(Json(resp))
}

#[derive(Debug, Deserialize)]
pub struct CreateShareRequest {
    pub twitch_login: String,
    pub can_manage: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateShareRequest {
    pub can_manage: bool,
}

#[derive(Debug, Serialize)]
pub struct SharedUserResponse {
    pub grantee_user_id: String,
    pub grantee_login: String,
    pub grantee_display_name: String,
    pub can_manage: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Serialize)]
pub struct IncomingShareResponse {
    pub owner_user_id: String,
    pub owner_twitch_login: String,
    pub owner_display_name: String,
    pub can_manage: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

/// Create a new share (grant access to another user)
async fn create_share(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Json(request): Json<CreateShareRequest>,
) -> AppResult<Json<SharedUserResponse>> {
    // Find user by twitch login
    let grantee = UserRepository::find_by_login(&state.db, &request.twitch_login)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    if grantee.id == user.id {
        return Err(AppError::Validation(
            "Cannot share settings with yourself".to_string(),
        ));
    }

    // Ensure we don't duplicate an existing share
    if SettingsShareRepository::find_by_owner_and_grantee(&state.db, &user.id, &grantee.id)
        .await?
        .is_some()
    {
        return Err(AppError::Conflict("Share already exists".to_string()));
    }

    let can_manage = request.can_manage.unwrap_or(false);

    let share =
        SettingsShareRepository::create(&state.db, &user.id, &grantee.id, can_manage).await?;

    Ok(Json(SharedUserResponse {
        grantee_user_id: share.grantee_user_id,
        grantee_login: request.twitch_login,
        grantee_display_name: grantee.twitch_display_name,
        can_manage: share.can_manage,
        created_at: share.created_at,
        updated_at: share.updated_at,
    }))
}

/// List incoming shares (owners who shared with the current user)
async fn list_incoming_shared(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
) -> AppResult<Json<Vec<IncomingShareResponse>>> {
    let rows = SettingsShareRepository::list_with_owner_info(&state.db, &user.id).await?;

    let resp: Vec<IncomingShareResponse> = rows
        .into_iter()
        .map(
            |(share, owner_login, owner_display)| IncomingShareResponse {
                owner_user_id: share.owner_user_id,
                owner_twitch_login: owner_login,
                owner_display_name: owner_display,
                can_manage: share.can_manage,
                created_at: share.created_at,
                updated_at: share.updated_at,
            },
        )
        .collect();

    Ok(Json(resp))
}

/// Revoke a share (owner revokes access previously granted)
async fn revoke_share(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Path(grantee_id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    SettingsShareRepository::delete(&state.db, &user.id, &grantee_id).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// Update an existing share (toggle `can_manage`)
async fn update_share(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Path(grantee_id): Path<String>,
    Json(request): Json<UpdateShareRequest>,
) -> AppResult<Json<SharedUserResponse>> {
    let updated = SettingsShareRepository::update_can_manage(
        &state.db,
        &user.id,
        &grantee_id,
        request.can_manage,
    )
    .await?;
    // Fetch grantee info
    let grantee = UserRepository::find_by_id(&state.db, &grantee_id)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    Ok(Json(SharedUserResponse {
        grantee_user_id: updated.grantee_user_id,
        grantee_login: grantee.twitch_login,
        grantee_display_name: grantee.twitch_display_name,
        can_manage: updated.can_manage,
        created_at: updated.created_at,
        updated_at: updated.updated_at,
    }))
}

/// Get notification message templates for another user (if shared)
async fn get_messages_for_user(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Path(owner_id): Path<String>,
) -> AppResult<Json<MessagesResponse>> {
    // If requesting own messages, delegate to existing handler
    if owner_id == user.id {
        return get_messages(State(state), AuthUser(user)).await;
    }

    // Must be shared
    let share =
        SettingsShareRepository::find_by_owner_and_grantee(&state.db, &owner_id, &user.id).await?;
    if share.is_none() {
        tracing::warn!(
            "Access denied: user {} attempted to view messages of owner {} without share",
            user.id,
            owner_id
        );
        return Err(AppError::Forbidden);
    }

    let settings = NotificationSettingsRepository::get_or_create(&state.db, &owner_id)
        .await
        .map_err(|e| {
            tracing::error!(
                "Failed to get/create settings for owner {} requested by {}: {:?}",
                owner_id,
                user.id,
                e
            );
            e
        })?;

    Ok(Json(MessagesResponse {
        stream_online_message: settings.stream_online_message,
        stream_offline_message: settings.stream_offline_message,
        stream_title_change_message: settings.stream_title_change_message,
        stream_category_change_message: settings.stream_category_change_message,
        reward_redemption_message: settings.reward_redemption_message,
        placeholders: get_placeholders_info(),
    }))
}

/// Update notification message templates for another user (if shared with manage rights)
async fn update_messages_for_user(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Path(owner_id): Path<String>,
    Json(request): Json<UpdateMessagesRequest>,
) -> AppResult<Json<MessagesResponse>> {
    // If updating own messages, delegate to existing handler
    if owner_id == user.id {
        return update_messages(State(state), AuthUser(user), Json(request)).await;
    }

    // Ensure shared with manage rights
    let share =
        SettingsShareRepository::find_by_owner_and_grantee(&state.db, &owner_id, &user.id).await?;
    match share {
        Some(s) if s.can_manage => { /* allowed */ }
        _ => return Err(AppError::Forbidden),
    }

    // Normalize placeholders (convert {{...}} -> {...}) and validate messages
    let stream_online_message = request
        .stream_online_message
        .as_ref()
        .map(|m| normalize_placeholders(m));
    let stream_offline_message = request
        .stream_offline_message
        .as_ref()
        .map(|m| normalize_placeholders(m));
    let stream_title_change_message = request
        .stream_title_change_message
        .as_ref()
        .map(|m| normalize_placeholders(m));
    let stream_category_change_message = request
        .stream_category_change_message
        .as_ref()
        .map(|m| normalize_placeholders(m));
    let reward_redemption_message = request
        .reward_redemption_message
        .as_ref()
        .map(|m| normalize_placeholders(m));

    if let Some(ref msg) = stream_online_message {
        validate_message(msg, "stream_online")?;
    }
    if let Some(ref msg) = stream_offline_message {
        validate_message(msg, "stream_offline")?;
    }
    if let Some(ref msg) = stream_title_change_message {
        validate_message(msg, "stream_title_change")?;
    }
    if let Some(ref msg) = stream_category_change_message {
        validate_message(msg, "stream_category")?;
    }
    if let Some(ref msg) = reward_redemption_message {
        validate_message(msg, "reward_redemption")?;
    }

    let update = UpdateNotificationSettings {
        stream_online_message,
        stream_offline_message,
        stream_title_change_message,
        stream_category_change_message,
        reward_redemption_message,
        notify_reward_redemption: None,
    };

    let settings = NotificationSettingsRepository::update(&state.db, &owner_id, update).await?;

    Ok(Json(MessagesResponse {
        stream_online_message: settings.stream_online_message,
        stream_offline_message: settings.stream_offline_message,
        stream_title_change_message: settings.stream_title_change_message,
        stream_category_change_message: settings.stream_category_change_message,
        reward_redemption_message: settings.reward_redemption_message,
        placeholders: get_placeholders_info(),
    }))
}

/// Reset settings to defaults for another user (if shared with manage rights)
async fn reset_to_defaults_for_user(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Path(owner_id): Path<String>,
) -> AppResult<Json<MessagesResponse>> {
    if owner_id == user.id {
        return reset_to_defaults(State(state), AuthUser(user)).await;
    }

    // Ensure shared with manage rights
    let share =
        SettingsShareRepository::find_by_owner_and_grantee(&state.db, &owner_id, &user.id).await?;
    match share {
        Some(s) if s.can_manage => {}
        _ => return Err(AppError::Forbidden),
    }

    let defaults = NotificationSettings::default();

    let update = UpdateNotificationSettings {
        stream_online_message: Some(defaults.stream_online_message.clone()),
        stream_offline_message: Some(defaults.stream_offline_message.clone()),
        stream_title_change_message: Some(defaults.stream_title_change_message.clone()),
        stream_category_change_message: Some(defaults.stream_category_change_message.clone()),
        reward_redemption_message: Some(defaults.reward_redemption_message.clone()),
        notify_reward_redemption: Some(defaults.notify_reward_redemption),
    };

    let settings = NotificationSettingsRepository::update(&state.db, &owner_id, update).await?;

    Ok(Json(MessagesResponse {
        stream_online_message: settings.stream_online_message,
        stream_offline_message: settings.stream_offline_message,
        stream_title_change_message: settings.stream_title_change_message,
        stream_category_change_message: settings.stream_category_change_message,
        reward_redemption_message: settings.reward_redemption_message,
        placeholders: get_placeholders_info(),
    }))
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Validate a notification message
fn validate_message(message: &str, message_type: &str) -> AppResult<()> {
    // Check message length
    if message.is_empty() {
        return Err(AppError::Validation(format!(
            "{} message cannot be empty",
            message_type
        )));
    }

    if message.len() > 4096 {
        return Err(AppError::Validation(format!(
            "{} message cannot exceed 4096 characters",
            message_type
        )));
    }

    Ok(())
}

/// Normalize placeholders in a message template.
/// Converts occurrences like `{{streamer}}` into `{streamer}`.
///
/// This allows the UI to show placeholders with double braces (e.g. `{{streamer}}`)
/// while the stored templates and server-side replacements use a single pair (`{streamer}`).
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

fn get_placeholders_info() -> PlaceholdersInfo {
    PlaceholdersInfo {
        stream: vec![
            PlaceholderInfo {
                name: "{streamer}".to_string(),
                description: "Streamer's display name".to_string(),
                example: "xQc".to_string(),
            },
            PlaceholderInfo {
                name: "{title}".to_string(),
                description: "Stream title".to_string(),
                example: "Just Chatting".to_string(),
            },
            PlaceholderInfo {
                name: "{game}".to_string(),
                description: "Game category".to_string(),
                example: "Just Chatting".to_string(),
            },
            PlaceholderInfo {
                name: "{url}".to_string(),
                description: "Stream URL".to_string(),
                example: "https://twitch.tv/xqc".to_string(),
            },
        ],
        reward: vec![
            PlaceholderInfo {
                name: "{user}".to_string(),
                description: "Username who redeemed the reward".to_string(),
                example: "viewer123".to_string(),
            },
            PlaceholderInfo {
                name: "{reward}".to_string(),
                description: "Reward title".to_string(),
                example: "Custom Reward Name".to_string(),
            },
        ],
    }
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
}
