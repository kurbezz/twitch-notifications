use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::db::{
    CreateDiscordIntegration, DiscordIntegration, TelegramIntegration, UpdateDiscordIntegration,
    UpdateTelegramIntegration, UserRepository,
};
use crate::error::{AppError, AppErrorWithDetails, AppResult};
use crate::routes::auth::AuthUser;
use crate::services::integrations::IntegrationService;
use crate::AppState;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::User;
    use chrono::Utc;

    fn make_user_with_discord(id: &str, discord_id: Option<&str>) -> User {
        let now = Utc::now().naive_utc();
        User {
            id: id.to_string(),
            twitch_id: "t1".to_string(),
            twitch_login: "login".to_string(),
            twitch_display_name: "display".to_string(),
            twitch_email: "e@example.com".to_string(),
            twitch_profile_image_url: "".to_string(),
            twitch_access_token: "a".to_string(),
            twitch_refresh_token: "r".to_string(),
            twitch_token_expires_at: now,
            telegram_user_id: None,
            telegram_username: None,
            telegram_photo_url: None,
            discord_user_id: discord_id.map(|s| s.to_string()),
            discord_username: discord_id.map(|s| format!("user#1234-{}", s)),
            discord_avatar_url: discord_id.map(|s| format!("https://cdn/{}.png", s)),
            lang: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn selects_owner_discord_when_owner_is_auth() {
        let owner = make_user_with_discord("owner", Some("owner_disc"));
        // auth_user has same id (owner creating for themselves)
        let auth_user = owner.clone();

        let res = select_discord_account_to_check(&owner.id, &auth_user, &owner);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), "owner_disc".to_string());
    }

    #[test]
    fn selects_auth_discord_when_creating_on_behalf() {
        let owner = make_user_with_discord("owner", None);
        let auth_user = make_user_with_discord("grantee", Some("grantee_disc"));

        let res = select_discord_account_to_check(&owner.id, &auth_user, &owner);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), "grantee_disc".to_string());
    }

    #[test]
    fn returns_error_when_selected_account_not_linked() {
        let owner = make_user_with_discord("owner", None);
        // auth_user also missing discord
        let auth_user = make_user_with_discord("owner", None);

        let res = select_discord_account_to_check(&owner.id, &auth_user, &owner);
        match res {
            Err(AppError::BadRequest(msg)) => {
                // message should be localized key translation (non-empty)
                assert!(!msg.is_empty());
            }
            other => panic!("expected BadRequest, got: {:?}", other),
        }
    }
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        // Telegram routes
        .route("/telegram", get(list_telegram_integrations))
        .route("/telegram", post(create_telegram_integration))
        .route("/telegram/:id", get(get_telegram_integration))
        .route("/telegram/:id", put(update_telegram_integration))
        .route("/telegram/:id", delete(delete_telegram_integration))
        .route("/telegram/:id/test", post(test_telegram_integration))
        .route("/telegram/bot", get(get_telegram_bot_info))
        // Discord routes
        .route("/discord", get(list_discord_integrations))
        .route("/discord", post(create_discord_integration))
        .route("/discord/:id", get(get_discord_integration))
        .route("/discord/:id", put(update_discord_integration))
        .route("/discord/:id", delete(delete_discord_integration))
        .route("/discord/:id/test", post(test_discord_integration))
        // Discord bot info
        .route("/discord/guilds", get(list_discord_guilds))
        .route("/discord/guilds/shared", get(list_shared_discord_guilds))
        .route("/discord/invite", get(get_discord_invite))
        .route("/discord/channels/:channel_id", get(get_discord_channel))
        .route(
            "/discord/guilds/:guild_id/channels",
            get(list_discord_channels),
        )
}

// ============================================================================
// Request/Response Types
// ============================================================================

// Telegram

#[derive(Debug, Deserialize)]
pub struct CreateTelegramRequest {
    pub telegram_chat_id: String,
    pub telegram_chat_title: Option<String>,
    pub telegram_chat_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTelegramRequest {
    pub telegram_chat_title: Option<String>,
    pub is_enabled: Option<bool>,
    pub notify_stream_online: Option<bool>,
    pub notify_stream_offline: Option<bool>,
    pub notify_title_change: Option<bool>,
    pub notify_category_change: Option<bool>,
    pub notify_reward_redemption: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct TelegramIntegrationResponse {
    pub id: String,
    pub telegram_chat_id: String,
    pub telegram_chat_title: Option<String>,
    pub telegram_chat_type: Option<String>,
    pub is_enabled: bool,
    pub notify_stream_online: bool,
    pub notify_stream_offline: bool,
    pub notify_title_change: bool,
    pub notify_category_change: bool,
    pub notify_reward_redemption: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl From<TelegramIntegration> for TelegramIntegrationResponse {
    fn from(integration: TelegramIntegration) -> Self {
        Self {
            id: integration.id,
            telegram_chat_id: integration.telegram_chat_id,
            telegram_chat_title: integration.telegram_chat_title,
            telegram_chat_type: integration.telegram_chat_type,
            is_enabled: integration.is_enabled,
            notify_stream_online: integration.notify_stream_online,
            notify_stream_offline: integration.notify_stream_offline,
            notify_title_change: integration.notify_title_change,
            notify_category_change: integration.notify_category_change,
            notify_reward_redemption: integration.notify_reward_redemption,
            created_at: integration.created_at,
            updated_at: integration.updated_at,
        }
    }
}

// Discord

#[derive(Debug, Deserialize)]
pub struct CreateDiscordRequest {
    pub discord_guild_id: String,
    pub discord_guild_name: Option<String>,
    pub discord_channel_id: String,
    pub discord_channel_name: Option<String>,
    pub discord_webhook_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateDiscordRequest {
    pub discord_channel_id: Option<String>,
    pub discord_channel_name: Option<String>,
    pub discord_webhook_url: Option<String>,
    pub is_enabled: Option<bool>,
    pub notify_stream_online: Option<bool>,
    pub notify_stream_offline: Option<bool>,
    pub notify_title_change: Option<bool>,
    pub notify_category_change: Option<bool>,
    pub notify_reward_redemption: Option<bool>,
    pub calendar_sync_enabled: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct DiscordIntegrationResponse {
    pub id: String,
    pub discord_guild_id: String,
    pub discord_guild_name: Option<String>,
    pub discord_channel_id: String,
    pub discord_channel_name: Option<String>,
    pub discord_webhook_url: Option<String>,
    pub is_enabled: bool,
    pub notify_stream_online: bool,
    pub notify_stream_offline: bool,
    pub notify_title_change: bool,
    pub notify_category_change: bool,
    pub notify_reward_redemption: bool,
    pub calendar_sync_enabled: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl From<DiscordIntegration> for DiscordIntegrationResponse {
    fn from(integration: DiscordIntegration) -> Self {
        Self {
            id: integration.id,
            discord_guild_id: integration.discord_guild_id,
            discord_guild_name: integration.discord_guild_name,
            discord_channel_id: integration.discord_channel_id,
            discord_channel_name: integration.discord_channel_name,
            discord_webhook_url: integration.discord_webhook_url,
            is_enabled: integration.is_enabled,
            notify_stream_online: integration.notify_stream_online,
            notify_stream_offline: integration.notify_stream_offline,
            notify_title_change: integration.notify_title_change,
            notify_category_change: integration.notify_category_change,
            notify_reward_redemption: integration.notify_reward_redemption,
            calendar_sync_enabled: integration.calendar_sync_enabled,
            created_at: integration.created_at,
            updated_at: integration.updated_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct DiscordGuildResponse {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DiscordChannelResponse {
    pub id: String,
    pub name: String,
    pub channel_type: u8,
}

#[derive(Debug, Serialize)]
pub struct DiscordInviteResponse {
    pub invite_url: String,
    pub permissions: u64,
    pub scopes: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct TestNotificationResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct OwnerQuery {
    pub user_id: Option<String>,
}

// ============================================================================
// Telegram Handlers
// ============================================================================

/// List all Telegram integrations for the current user (or for another user if shared)
async fn list_telegram_integrations(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Query(query): Query<OwnerQuery>,
) -> AppResult<Json<Vec<TelegramIntegrationResponse>>> {
    let owner_id = query.user_id.clone().unwrap_or_else(|| user.id.clone());

    if !IntegrationService::check_access(&state, &owner_id, &user.id, false).await? {
        tracing::warn!(
            "Access denied: user {} attempted to list telegram integrations of owner {} without share",
            user.id,
            owner_id
        );
        return Err(AppError::Forbidden);
    }

    let integrations = IntegrationService::list_telegram_integrations(&state, &owner_id).await?;
    let response: Vec<TelegramIntegrationResponse> =
        integrations.into_iter().map(Into::into).collect();

    Ok(Json(response))
}

/// Create a new Telegram integration (optionally on behalf of an owner via ?user_id=...)
async fn create_telegram_integration(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Query(query): Query<OwnerQuery>,
    Json(request): Json<CreateTelegramRequest>,
) -> AppResult<Json<TelegramIntegrationResponse>> {
    let owner_id = query.user_id.clone().unwrap_or_else(|| user.id.clone());

    if !IntegrationService::check_access(&state, &owner_id, &user.id, true).await? {
        tracing::warn!(
            "Access denied: user {} attempted to create telegram integration for owner {} without manage rights",
            user.id,
            owner_id
        );
        return Err(AppError::Forbidden);
    }

    let owner = UserRepository::find_by_id(&state.db, &owner_id)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    let chat_type = request
        .telegram_chat_type
        .clone()
        .or(Some("private".to_string()));
    let chat_id = IntegrationService::determine_telegram_chat_id(
        &owner_id,
        &user,
        &owner,
        chat_type.as_deref(),
        &request.telegram_chat_id,
    )?;

    IntegrationService::validate_telegram_chat_id(&chat_id, chat_type.as_deref())?;

    // Check admin permissions for non-private chats
    if let Some("group") | Some("supergroup") | Some("channel") = chat_type.as_deref() {
        let owner_tg_id = owner.telegram_user_id.clone().ok_or_else(|| {
            AppError::Validation(crate::i18n::t("validation.owner_telegram_not_linked"))
        })?;
        let is_admin =
            IntegrationService::check_telegram_admin(&state, &chat_id, &owner_tg_id).await?;
        if !is_admin {
            return Err(AppError::Validation(crate::i18n::t(
                "validation.must_be_admin",
            )));
        }
    }

    let created = IntegrationService::create_telegram_integration(
        &state,
        &owner_id,
        chat_id,
        request.telegram_chat_title,
        chat_type,
    )
    .await?;

    Ok(Json(created.into()))
}

/// Get a specific Telegram integration
async fn get_telegram_integration(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Path(id): Path<String>,
) -> AppResult<Json<TelegramIntegrationResponse>> {
    let integration = IntegrationService::get_telegram_integration(&state, &id)
        .await?
        .ok_or_else(|| AppError::NotFound(crate::i18n::t("not_found.integration")))?;

    if !IntegrationService::check_access(&state, &integration.user_id, &user.id, false).await? {
        tracing::warn!(
            "Access denied: user {} attempted to view telegram integration {} owned by {} without share",
            user.id,
            id,
            integration.user_id
        );
        return Err(AppError::Forbidden);
    }

    Ok(Json(integration.into()))
}

/// Update a Telegram integration
async fn update_telegram_integration(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Path(id): Path<String>,
    Json(request): Json<UpdateTelegramRequest>,
) -> AppResult<Json<TelegramIntegrationResponse>> {
    let existing = IntegrationService::get_telegram_integration(&state, &id)
        .await?
        .ok_or_else(|| AppError::NotFound(crate::i18n::t("not_found.integration")))?;

    if !IntegrationService::check_access(&state, &existing.user_id, &user.id, true).await? {
        tracing::warn!(
            "Access denied: user {} attempted to update telegram integration {} owned by {} without manage rights",
            user.id,
            id,
            existing.user_id
        );
        return Err(AppError::Forbidden);
    }

    let update = UpdateTelegramIntegration {
        telegram_chat_title: request.telegram_chat_title,
        is_enabled: request.is_enabled,
        notify_stream_online: request.notify_stream_online,
        notify_stream_offline: request.notify_stream_offline,
        notify_title_change: request.notify_title_change,
        notify_category_change: request.notify_category_change,
        notify_reward_redemption: request.notify_reward_redemption,
    };

    let updated = IntegrationService::update_telegram_integration(&state, &id, update).await?;

    Ok(Json(updated.into()))
}

/// Delete a Telegram integration
async fn delete_telegram_integration(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let existing = IntegrationService::get_telegram_integration(&state, &id)
        .await?
        .ok_or_else(|| AppError::NotFound(crate::i18n::t("not_found.integration")))?;

    if !IntegrationService::check_access(&state, &existing.user_id, &user.id, true).await? {
        tracing::warn!(
            "Access denied: user {} attempted to delete telegram integration {} owned by {} without manage rights",
            user.id,
            id,
            existing.user_id
        );
        return Err(AppError::Forbidden);
    }

    IntegrationService::delete_telegram_integration(&state, &id).await?;

    Ok(Json(serde_json::json!({
        "message": crate::i18n::t("integration.deleted")
    })))
}

/// Send a test notification to a Telegram integration
async fn test_telegram_integration(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Path(id): Path<String>,
) -> AppResult<Json<TestNotificationResponse>> {
    let integration = IntegrationService::get_telegram_integration(&state, &id)
        .await?
        .ok_or_else(|| AppError::NotFound(crate::i18n::t("not_found.integration")))?;

    if !IntegrationService::check_access(&state, &integration.user_id, &user.id, true).await? {
        tracing::warn!(
            "Access denied: user {} attempted to test telegram integration {} owned by {} without manage rights",
            user.id,
            id,
            integration.user_id
        );
        return Err(AppError::Forbidden);
    }

    let telegram_guard = state.telegram.read().await;
    let telegram = telegram_guard.as_ref().ok_or_else(|| {
        AppError::ServiceUnavailable("Telegram service not available".to_string())
    })?;

    let owner = UserRepository::find_by_id(&state.db, &integration.user_id).await?;
    let owner_lang = owner.as_ref().and_then(|o| o.lang.as_deref());
    let title = crate::i18n::tr(owner_lang, "messages.test_notification_title", None);
    let body = crate::i18n::tr(owner_lang, "messages.test_notification_body", None);
    let message = format!("<b>{}</b>\n\n{}", title, body);

    match telegram
        .send_message(crate::services::telegram::TelegramMessage {
            chat_id: integration.telegram_chat_id.clone(),
            text: message,
            ..Default::default()
        })
        .await
    {
        Ok(_) => Ok(Json(TestNotificationResponse {
            success: true,
            message: crate::i18n::tr(owner_lang, "test_notification.success", None),
        })),
        Err(e) => {
            let err_msg = e.to_string();
            Ok(Json(TestNotificationResponse {
                success: false,
                message: crate::i18n::tr(
                    owner_lang,
                    "test_notification.failure",
                    Some(&[("err", &err_msg)]),
                ),
            }))
        }
    }
}

// ============================================================================
// Discord Handlers
// ============================================================================

/// List all Discord integrations for the current user (or for another user if shared)
async fn list_discord_integrations(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Query(query): Query<OwnerQuery>,
) -> AppResult<Json<Vec<DiscordIntegrationResponse>>> {
    let owner_id = query.user_id.clone().unwrap_or_else(|| user.id.clone());

    if !IntegrationService::check_access(&state, &owner_id, &user.id, false).await? {
        tracing::warn!(
            "Access denied: user {} attempted to list discord integrations of owner {} without share",
            user.id,
            owner_id
        );
        return Err(AppError::Forbidden);
    }

    let integrations = IntegrationService::list_discord_integrations(&state, &owner_id).await?;
    let response: Vec<DiscordIntegrationResponse> =
        integrations.into_iter().map(Into::into).collect();

    Ok(Json(response))
}

/// Create a new Discord integration (optionally on behalf of an owner via ?user_id=...)
async fn create_discord_integration(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Query(query): Query<OwnerQuery>,
    Json(request): Json<CreateDiscordRequest>,
) -> Result<Json<DiscordIntegrationResponse>, AppErrorWithDetails> {
    let owner_id = query.user_id.clone().unwrap_or_else(|| user.id.clone());

    if !IntegrationService::check_access(&state, &owner_id, &user.id, true).await? {
        tracing::warn!(
            "Отказ в доступе: пользователь {} попытался создать Discord-интеграцию от имени {} без прав на управление",
            user.id,
            owner_id
        );
        return Err(AppError::Forbidden.with_details(serde_json::json!({
            "reason": "no_share_manage",
            "message": crate::i18n::t("errors.no_share_manage")
        })));
    }

    let owner_user = UserRepository::find_by_id(&state.db, &owner_id)
        .await?
        .ok_or_else(|| AppError::NotFound(crate::i18n::t("not_found.user")))?;

    let discord_account_to_check =
        IntegrationService::select_discord_account_to_check(&owner_id, &user, &owner_user)?;

    let has_manage = IntegrationService::check_discord_manage_permissions(
        &state,
        &request.discord_guild_id,
        &discord_account_to_check,
    )
    .await?;

    if !has_manage {
        tracing::warn!(
            "Отказ в доступе: пользователь {} попытался создать интеграцию для сервера {} без прав администратора/управления",
            owner_id,
            request.discord_guild_id
        );
        return Err(AppError::Forbidden.with_details(serde_json::json!({
            "reason": "insufficient_permissions",
            "message": crate::i18n::t("errors.insufficient_permissions")
        })));
    }

    let integration = CreateDiscordIntegration {
        discord_guild_id: request.discord_guild_id,
        discord_guild_name: request.discord_guild_name,
        discord_channel_id: request.discord_channel_id,
        discord_channel_name: request.discord_channel_name,
        discord_webhook_url: request.discord_webhook_url,
    };

    let created =
        IntegrationService::create_discord_integration(&state, &owner_id, integration).await?;

    Ok(Json(created.into()))
}

/// Get a specific Discord integration
async fn get_discord_integration(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Path(id): Path<String>,
) -> AppResult<Json<DiscordIntegrationResponse>> {
    let integration = IntegrationService::get_discord_integration(&state, &id)
        .await?
        .ok_or_else(|| AppError::NotFound(crate::i18n::t("not_found.integration")))?;

    if !IntegrationService::check_access(&state, &integration.user_id, &user.id, false).await? {
        tracing::warn!(
            "Access denied: user {} attempted to view discord integration {} owned by {} without share",
            user.id,
            id,
            integration.user_id
        );
        return Err(AppError::Forbidden);
    }

    Ok(Json(integration.into()))
}

/// Update a Discord integration
async fn update_discord_integration(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Path(id): Path<String>,
    Json(request): Json<UpdateDiscordRequest>,
) -> AppResult<Json<DiscordIntegrationResponse>> {
    let existing = IntegrationService::get_discord_integration(&state, &id)
        .await?
        .ok_or_else(|| AppError::NotFound(crate::i18n::t("not_found.integration")))?;

    if !IntegrationService::check_access(&state, &existing.user_id, &user.id, true).await? {
        tracing::warn!(
            "Access denied: user {} attempted to update discord integration {} owned by {} without manage rights",
            user.id,
            id,
            existing.user_id
        );
        return Err(AppError::Forbidden);
    }

    let update = UpdateDiscordIntegration {
        discord_channel_id: request.discord_channel_id,
        discord_channel_name: request.discord_channel_name,
        discord_webhook_url: request.discord_webhook_url,
        is_enabled: request.is_enabled,
        notify_stream_online: request.notify_stream_online,
        notify_stream_offline: request.notify_stream_offline,
        notify_title_change: request.notify_title_change,
        notify_category_change: request.notify_category_change,
        notify_reward_redemption: request.notify_reward_redemption,
        calendar_sync_enabled: request.calendar_sync_enabled,
    };

    let updated = IntegrationService::update_discord_integration(&state, &id, update).await?;

    // Trigger calendar sync if enabled
    let enabled_now = request.calendar_sync_enabled.unwrap_or(false);
    if enabled_now && !existing.calendar_sync_enabled {
        let state_clone = state.clone();
        let updated_clone = updated.clone();
        tokio::spawn(async move {
            if let Err(e) = crate::services::calendar::CalendarSyncManager::sync_for_integration(
                &state_clone,
                &updated_clone,
            )
            .await
            {
                tracing::warn!(
                    "Immediate calendar sync for integration {} failed: {:?}",
                    updated_clone.id,
                    e
                );
            }
        });
    }

    Ok(Json(updated.into()))
}

/// Delete a Discord integration
async fn delete_discord_integration(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let existing = IntegrationService::get_discord_integration(&state, &id)
        .await?
        .ok_or_else(|| AppError::NotFound(crate::i18n::t("not_found.integration")))?;

    if !IntegrationService::check_access(&state, &existing.user_id, &user.id, true).await? {
        tracing::warn!(
            "Access denied: user {} attempted to delete discord integration {} owned by {} without manage rights",
            user.id,
            id,
            existing.user_id
        );
        return Err(AppError::Forbidden);
    }

    IntegrationService::delete_discord_integration(&state, &id).await?;

    Ok(Json(serde_json::json!({
        "message": crate::i18n::t("integration.deleted")
    })))
}

/// Send a test notification to a Discord integration
async fn test_discord_integration(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Path(id): Path<String>,
) -> AppResult<Json<TestNotificationResponse>> {
    let integration = IntegrationService::get_discord_integration(&state, &id)
        .await?
        .ok_or_else(|| AppError::NotFound(crate::i18n::t("not_found.integration")))?;

    if !IntegrationService::check_access(&state, &integration.user_id, &user.id, true).await? {
        tracing::warn!(
            "Access denied: user {} attempted to test discord integration {} owned by {} without manage rights",
            user.id,
            id,
            integration.user_id
        );
        return Err(AppError::Forbidden);
    }

    let discord_guard = state.discord.read().await;
    let discord = discord_guard
        .as_ref()
        .ok_or_else(|| AppError::ServiceUnavailable("Discord service not available".to_string()))?;

    use crate::services::discord::{colors, DiscordEmbed, DiscordMessage, WebhookMessage};

    let owner = UserRepository::find_by_id(&state.db, &integration.user_id).await?;
    let owner_lang = owner.as_ref().and_then(|o| o.lang.as_deref());
    let title = crate::i18n::tr(owner_lang, "messages.test_notification_title", None);
    let body = crate::i18n::tr(owner_lang, "messages.test_notification_body", None);
    let embed = DiscordEmbed::new()
        .title(&title)
        .description(&body)
        .color(colors::SUCCESS)
        .timestamp(chrono::Utc::now().to_rfc3339());

    let result = if let Some(ref webhook_url) = integration.discord_webhook_url {
        let message = WebhookMessage {
            content: None,
            username: Some(crate::i18n::t("app.name")),
            avatar_url: None,
            embeds: Some(vec![embed]),
        };
        discord.send_webhook_message(webhook_url, message).await
    } else {
        let message = DiscordMessage {
            content: None,
            embeds: Some(vec![embed]),
            tts: None,
        };
        discord
            .send_message(&integration.discord_channel_id, message)
            .await
    };

    match result {
        Ok(_) => Ok(Json(TestNotificationResponse {
            success: true,
            message: crate::i18n::tr(owner_lang, "test_notification.success", None),
        })),
        Err(e) => {
            let err_msg = e.to_string();
            Ok(Json(TestNotificationResponse {
                success: false,
                message: crate::i18n::tr(
                    owner_lang,
                    "test_notification.failure",
                    Some(&[("err", &err_msg)]),
                ),
            }))
        }
    }
}

/// List Discord guilds the bot is a member of
async fn list_discord_guilds(
    State(state): State<Arc<AppState>>,
    AuthUser(_user): AuthUser,
) -> AppResult<Json<Vec<DiscordGuildResponse>>> {
    let discord_guard = state.discord.read().await;
    let discord = discord_guard
        .as_ref()
        .ok_or_else(|| AppError::ServiceUnavailable("Discord service not available".to_string()))?;

    let guilds = discord.get_guilds().await?;

    let response: Vec<DiscordGuildResponse> = guilds
        .into_iter()
        .map(|g| DiscordGuildResponse {
            id: g.id,
            name: g.name,
            icon: g.icon,
        })
        .collect();

    Ok(Json(response))
}

/// List Discord guilds that are common between the bot and the authenticated user
async fn list_shared_discord_guilds(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
) -> AppResult<Json<Vec<DiscordGuildResponse>>> {
    let discord_guard = state.discord.read().await;
    let discord = discord_guard
        .as_ref()
        .ok_or_else(|| AppError::ServiceUnavailable("Discord service not available".to_string()))?;

    // Ensure the user has a linked Discord account
    let discord_user_id = user
        .discord_user_id
        .clone()
        .ok_or_else(|| AppError::BadRequest("Discord not linked".to_string()))?;

    let guilds = discord.get_guilds().await?;

    let mut response: Vec<DiscordGuildResponse> = Vec::new();

    for g in guilds.into_iter() {
        match discord.is_user_in_guild(&g.id, &discord_user_id).await {
            Ok(true) => response.push(DiscordGuildResponse {
                id: g.id,
                name: g.name,
                icon: g.icon,
            }),
            Ok(false) => continue,
            Err(e) => {
                tracing::warn!("Failed to check membership for guild {}: {:?}", g.id, e);
                continue;
            }
        }
    }

    Ok(Json(response))
}

#[derive(Debug, Serialize)]
pub struct TelegramBotInfoResponse {
    pub username: String,
    pub id: String,
}

/// Get basic info for the configured Telegram bot (username and id)
async fn get_telegram_bot_info(
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<TelegramBotInfoResponse>> {
    // Take a clone of the optional Telegram service so we don't hold the lock during the async call.
    let guard = state.telegram.read().await;
    let tg_opt = guard.clone();
    drop(guard);

    let tg = tg_opt
        .as_ref()
        .ok_or_else(|| AppError::ServiceUnavailable("Telegram bot not configured".to_string()))?;

    use teloxide::prelude::Requester;

    let me = tg
        .get_bot()
        .get_me()
        .await
        .map_err(|e| AppError::Telegram(format!("Failed to fetch Telegram bot info: {}", e)))?;

    Ok(Json(TelegramBotInfoResponse {
        username: me.username().to_string(),
        id: me.id.to_string(),
    }))
}

/// Get invite URL for the Discord bot (for adding the bot to a server)
async fn get_discord_invite(
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<DiscordInviteResponse>> {
    let client_id = state.config.discord.client_id.as_ref().ok_or_else(|| {
        AppError::ServiceUnavailable("Discord client ID not configured".to_string())
    })?;

    // Required permissions:
    // - VIEW_CHANNEL (1024)
    // - SEND_MESSAGES (2048)
    // - EMBED_LINKS (16384)
    // - MANAGE_EVENTS (524288) (needed for calendar sync to Discord Events)
    const PERMISSIONS: u64 = 1024 + 2048 + 16384 + 524288; // 543744

    let invite_url = format!(
        "https://discord.com/oauth2/authorize?client_id={}&permissions={}&scope=bot%20applications.commands",
        client_id, PERMISSIONS
    );

    Ok(Json(DiscordInviteResponse {
        invite_url,
        permissions: PERMISSIONS,
        scopes: vec!["bot".to_string(), "applications.commands".to_string()],
    }))
}

/// List channels in a Discord guild
async fn list_discord_channels(
    State(state): State<Arc<AppState>>,
    AuthUser(_user): AuthUser,
    Path(guild_id): Path<String>,
) -> AppResult<Json<Vec<DiscordChannelResponse>>> {
    let discord_guard = state.discord.read().await;
    let discord = discord_guard
        .as_ref()
        .ok_or_else(|| AppError::ServiceUnavailable("Discord service not available".to_string()))?;

    let channels = discord.get_guild_channels(&guild_id).await?;

    // Filter to only text channels (type 0) and news channels (type 5)
    let response: Vec<DiscordChannelResponse> = channels
        .into_iter()
        .filter(|c| c.channel_type == 0 || c.channel_type == 5)
        .filter_map(|c| {
            c.name.map(|name| DiscordChannelResponse {
                id: c.id,
                name,
                channel_type: c.channel_type,
            })
        })
        .collect();

    Ok(Json(response))
}

/// Get a Discord channel by ID
async fn get_discord_channel(
    State(state): State<Arc<AppState>>,
    AuthUser(_user): AuthUser,
    Path(channel_id): Path<String>,
) -> AppResult<Json<DiscordChannelResponse>> {
    let discord_guard = state.discord.read().await;
    let discord = discord_guard
        .as_ref()
        .ok_or_else(|| AppError::ServiceUnavailable("Discord service not available".to_string()))?;

    let channel = discord.get_channel(&channel_id).await?;

    // Ensure it's a text channel (type 0) or news channel (type 5) and has a name
    if channel.channel_type != 0 && channel.channel_type != 5 {
        return Err(AppError::BadRequest(
            "Channel is not a text or news channel".to_string(),
        ));
    }

    let name = channel
        .name
        .clone()
        .ok_or_else(|| AppError::BadRequest("Channel has no name".to_string()))?;

    Ok(Json(DiscordChannelResponse {
        id: channel.id,
        name,
        channel_type: channel.channel_type,
    }))
}
