use std::sync::Arc;

use crate::db::{
    CreateDiscordIntegration, CreateTelegramIntegration, DiscordIntegrationRepository,
    SettingsShareRepository, TelegramIntegrationRepository, UpdateDiscordIntegration,
    UpdateTelegramIntegration,
};
use crate::error::{AppError, AppResult};
use crate::AppState;

pub struct IntegrationService;

impl IntegrationService {
    /// Check if user has access to owner's resources (read or manage)
    pub async fn check_access(
        state: &Arc<AppState>,
        owner_id: &str,
        user_id: &str,
        require_manage: bool,
    ) -> AppResult<bool> {
        if owner_id == user_id {
            return Ok(true);
        }

        let share = SettingsShareRepository::find_by_owner_and_grantee(&state.db, owner_id, user_id).await?;

        match share {
            Some(s) if require_manage => Ok(s.can_manage),
            Some(_) => Ok(true), // Read access
            None => Ok(false),
        }
    }

    /// Validate Telegram chat ID format
    pub fn validate_telegram_chat_id(chat_id: &str, chat_type: Option<&str>) -> AppResult<()> {
        let chat_id = chat_id.trim();

        match chat_type {
            Some("group") => {
                if chat_id.len() < 2
                    || !chat_id.starts_with('-')
                    || !chat_id[1..].chars().all(|c| c.is_ascii_digit())
                {
                    return Err(AppError::Validation(crate::i18n::t(
                        "validation.chat_id.group_invalid",
                    )));
                }
            }
            Some("supergroup") | Some("channel") => {
                if chat_id.len() < 5
                    || !chat_id.starts_with("-100")
                    || !chat_id[4..].chars().all(|c| c.is_ascii_digit())
                {
                    return Err(AppError::Validation(crate::i18n::t(
                        "validation.chat_id.supergroup_invalid",
                    )));
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Check if user is admin in Telegram chat
    pub async fn check_telegram_admin(
        state: &Arc<AppState>,
        chat_id: &str,
        telegram_user_id: &str,
    ) -> AppResult<bool> {
        let tg_guard = state.telegram.read().await;
        let telegram_service = tg_guard.as_ref().ok_or_else(|| {
            AppError::Validation(crate::i18n::t("validation.telegram_bot_not_configured"))
        })?;

        telegram_service.is_user_admin(chat_id, telegram_user_id).await
    }

    /// Check if user has manage permissions in Discord guild
    pub async fn check_discord_manage_permissions(
        state: &Arc<AppState>,
        guild_id: &str,
        discord_user_id: &str,
    ) -> AppResult<bool> {
        let discord_guard = state.discord.read().await;
        let discord = discord_guard.as_ref().ok_or_else(|| {
            AppError::ServiceUnavailable(crate::i18n::t(
                "service_unavailable.discord_service_unavailable",
            ))
        })?;

        discord.user_has_manage_permissions(guild_id, discord_user_id).await
    }

    /// Select which Discord account to use for permission checks
    pub fn select_discord_account_to_check(
        owner_id: &str,
        auth_user: &crate::db::User,
        owner_user: &crate::db::User,
    ) -> Result<String, AppError> {
        if owner_id == auth_user.id {
            owner_user
                .discord_user_id
                .clone()
                .ok_or_else(|| AppError::BadRequest(crate::i18n::t("bad_request.no_discord_linked")))
        } else {
            auth_user
                .discord_user_id
                .clone()
                .ok_or_else(|| AppError::BadRequest(crate::i18n::t("bad_request.no_discord_linked")))
        }
    }

    /// Determine effective chat ID for Telegram integration
    pub fn determine_telegram_chat_id(
        owner_id: &str,
        auth_user: &crate::db::User,
        owner_user: &crate::db::User,
        chat_type: Option<&str>,
        provided_chat_id: &str,
    ) -> AppResult<String> {
        if chat_type.as_deref() == Some("private") {
            if owner_id != auth_user.id {
                // Creating for another user - use editor's telegram_user_id
                auth_user.telegram_user_id.clone().ok_or_else(|| {
                    AppError::Validation(crate::i18n::t("validation.owner_telegram_not_linked"))
                })
            } else {
                // Creating for self - use owner's telegram_user_id
                owner_user.telegram_user_id.clone().ok_or_else(|| {
                    AppError::Validation(crate::i18n::t("validation.owner_telegram_not_linked"))
                })
            }
        } else {
            // For non-private chats, validate owner has linked Telegram
            if owner_user.telegram_user_id.is_none() {
                return Err(AppError::Validation(crate::i18n::t(
                    "validation.owner_telegram_not_linked",
                )));
            }
            Ok(provided_chat_id.trim().to_string())
        }
    }

    /// Create Telegram integration
    pub async fn create_telegram_integration(
        state: &Arc<AppState>,
        owner_id: &str,
        chat_id: String,
        chat_title: Option<String>,
        chat_type: Option<String>,
    ) -> AppResult<crate::db::TelegramIntegration> {
        // Check if integration already exists
        let exists = TelegramIntegrationRepository::exists(&state.db, &chat_id, owner_id).await?;
        if exists {
            return Err(AppError::Conflict(
                "Integration already exists for this chat".to_string(),
            ));
        }

        let integration = CreateTelegramIntegration {
            telegram_chat_id: chat_id,
            telegram_chat_title: chat_title,
            telegram_chat_type: chat_type,
        };

        TelegramIntegrationRepository::create(&state.db, owner_id, integration).await
    }

    /// Update Telegram integration
    pub async fn update_telegram_integration(
        state: &Arc<AppState>,
        integration_id: &str,
        update: UpdateTelegramIntegration,
    ) -> AppResult<crate::db::TelegramIntegration> {
        TelegramIntegrationRepository::update(&state.db, integration_id, update).await
    }

    /// Delete Telegram integration
    pub async fn delete_telegram_integration(
        state: &Arc<AppState>,
        integration_id: &str,
    ) -> AppResult<()> {
        TelegramIntegrationRepository::delete(&state.db, integration_id).await
    }

    /// Create Discord integration
    pub async fn create_discord_integration(
        state: &Arc<AppState>,
        owner_id: &str,
        integration: CreateDiscordIntegration,
    ) -> AppResult<crate::db::DiscordIntegration> {
        DiscordIntegrationRepository::create(&state.db, owner_id, integration).await
    }

    /// Update Discord integration
    pub async fn update_discord_integration(
        state: &Arc<AppState>,
        integration_id: &str,
        update: UpdateDiscordIntegration,
    ) -> AppResult<crate::db::DiscordIntegration> {
        DiscordIntegrationRepository::update(&state.db, integration_id, update).await
    }

    /// Delete Discord integration
    pub async fn delete_discord_integration(
        state: &Arc<AppState>,
        integration_id: &str,
    ) -> AppResult<()> {
        DiscordIntegrationRepository::delete(&state.db, integration_id).await
    }

    /// Get Telegram integration by ID
    pub async fn get_telegram_integration(
        state: &Arc<AppState>,
        integration_id: &str,
    ) -> AppResult<Option<crate::db::TelegramIntegration>> {
        TelegramIntegrationRepository::find_by_id(&state.db, integration_id).await
    }

    /// Get Discord integration by ID
    pub async fn get_discord_integration(
        state: &Arc<AppState>,
        integration_id: &str,
    ) -> AppResult<Option<crate::db::DiscordIntegration>> {
        DiscordIntegrationRepository::find_by_id(&state.db, integration_id).await
    }

    /// List Telegram integrations for user
    pub async fn list_telegram_integrations(
        state: &Arc<AppState>,
        user_id: &str,
    ) -> AppResult<Vec<crate::db::TelegramIntegration>> {
        TelegramIntegrationRepository::find_by_user_id(&state.db, user_id).await
    }

    /// List Discord integrations for user
    pub async fn list_discord_integrations(
        state: &Arc<AppState>,
        user_id: &str,
    ) -> AppResult<Vec<crate::db::DiscordIntegration>> {
        DiscordIntegrationRepository::find_by_user_id(&state.db, user_id).await
    }
}
