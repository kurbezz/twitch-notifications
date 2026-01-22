use crate::db::models::{CreateDiscordIntegration, DiscordIntegration, UpdateDiscordIntegration};
use crate::error::{AppError, AppResult};
use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct DiscordIntegrationRepository;

impl DiscordIntegrationRepository {
    /// Create a new Discord integration
    pub async fn create(
        pool: &SqlitePool,
        user_id: &str,
        integration: CreateDiscordIntegration,
    ) -> AppResult<DiscordIntegration> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().naive_utc();

        sqlx::query_as!(
            DiscordIntegration,
            r#"
            INSERT INTO discord_integrations (
                id, user_id, discord_guild_id, discord_channel_id,
                discord_guild_name, discord_channel_name, discord_webhook_url,
                notify_stream_online, notify_stream_offline,
                notify_title_change, notify_category_change, notify_reward_redemption,
                calendar_sync_enabled, created_at, updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING
                id as "id!: String",
                user_id as "user_id!: String",
                discord_guild_id as "discord_guild_id!: String",
                discord_channel_id as "discord_channel_id!: String",
                discord_guild_name as "discord_guild_name?: String",
                discord_channel_name as "discord_channel_name?: String",
                discord_webhook_url as "discord_webhook_url?: String",
                is_enabled as "is_enabled!: bool",
                notify_stream_online as "notify_stream_online!: bool",
                notify_stream_offline as "notify_stream_offline!: bool",
                notify_title_change as "notify_title_change!: bool",
                notify_category_change as "notify_category_change!: bool",
                notify_reward_redemption as "notify_reward_redemption!: bool",
                calendar_sync_enabled as "calendar_sync_enabled!: bool",
                created_at as "created_at!: chrono::NaiveDateTime",
                updated_at as "updated_at!: chrono::NaiveDateTime"
            "#,
            id,
            user_id,
            integration.discord_guild_id,
            integration.discord_channel_id,
            integration.discord_guild_name,
            integration.discord_channel_name,
            integration.discord_webhook_url,
            true,
            false,
            true,
            true,
            false,
            false,
            now,
            now
        )
        .fetch_one(pool)
        .await
        .map_err(AppError::Database)
    }

    /// Find Discord integration by id
    pub async fn find_by_id(pool: &SqlitePool, id: &str) -> AppResult<Option<DiscordIntegration>> {
        sqlx::query_as!(
            DiscordIntegration,
            r#"
                SELECT
                    id as "id!: String",
                    user_id as "user_id!: String",
                    discord_guild_id as "discord_guild_id!: String",
                    discord_channel_id as "discord_channel_id!: String",
                    discord_guild_name as "discord_guild_name?: String",
                    discord_channel_name as "discord_channel_name?: String",
                    discord_webhook_url as "discord_webhook_url?: String",
                    is_enabled as "is_enabled!: bool",
                    notify_stream_online as "notify_stream_online!: bool",
                    notify_stream_offline as "notify_stream_offline!: bool",
                    notify_title_change as "notify_title_change!: bool",
                    notify_category_change as "notify_category_change!: bool",
                    notify_reward_redemption as "notify_reward_redemption!: bool",
                    calendar_sync_enabled as "calendar_sync_enabled!: bool",
                    created_at as "created_at!: chrono::NaiveDateTime",
                    updated_at as "updated_at!: chrono::NaiveDateTime"
                FROM discord_integrations
                WHERE id = ?
                "#,
            id
        )
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)
    }

    /// Find all Discord integrations for a user
    pub async fn find_by_user_id(
        pool: &SqlitePool,
        user_id: &str,
    ) -> AppResult<Vec<DiscordIntegration>> {
        sqlx::query_as!(
            DiscordIntegration,
            r#"
            SELECT
                id as "id!: String",
                user_id as "user_id!: String",
                discord_guild_id as "discord_guild_id!: String",
                discord_channel_id as "discord_channel_id!: String",
                discord_guild_name as "discord_guild_name?: String",
                discord_channel_name as "discord_channel_name?: String",
                discord_webhook_url as "discord_webhook_url?: String",
                is_enabled as "is_enabled!: bool",
                notify_stream_online as "notify_stream_online!: bool",
                notify_stream_offline as "notify_stream_offline!: bool",
                notify_title_change as "notify_title_change!: bool",
                notify_category_change as "notify_category_change!: bool",
                notify_reward_redemption as "notify_reward_redemption!: bool",
                calendar_sync_enabled as "calendar_sync_enabled!: bool",
                created_at as "created_at!: chrono::NaiveDateTime",
                updated_at as "updated_at!: chrono::NaiveDateTime"
            FROM discord_integrations
            WHERE user_id = ?
            ORDER BY created_at DESC
            "#,
            user_id
        )
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)
    }

    /// Find Discord integrations by channel id
    pub async fn find_by_channel_id(
        pool: &SqlitePool,
        channel_id: &str,
    ) -> AppResult<Vec<DiscordIntegration>> {
        sqlx::query_as!(
            DiscordIntegration,
            r#"
            SELECT
                id as "id!: String",
                user_id as "user_id!: String",
                discord_guild_id as "discord_guild_id!: String",
                discord_channel_id as "discord_channel_id!: String",
                discord_guild_name as "discord_guild_name?: String",
                discord_channel_name as "discord_channel_name?: String",
                discord_webhook_url as "discord_webhook_url?: String",
                is_enabled as "is_enabled!: bool",
                notify_stream_online as "notify_stream_online!: bool",
                notify_stream_offline as "notify_stream_offline!: bool",
                notify_title_change as "notify_title_change!: bool",
                notify_category_change as "notify_category_change!: bool",
                notify_reward_redemption as "notify_reward_redemption!: bool",
                calendar_sync_enabled as "calendar_sync_enabled!: bool",
                created_at as "created_at!: chrono::NaiveDateTime",
                updated_at as "updated_at!: chrono::NaiveDateTime"
            FROM discord_integrations
            WHERE discord_channel_id = ?
            "#,
            channel_id
        )
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)
    }

    /// Update Discord integration
    pub async fn update(
        pool: &SqlitePool,
        id: &str,
        update: UpdateDiscordIntegration,
    ) -> AppResult<DiscordIntegration> {
        let current = Self::find_by_id(pool, id)
            .await?
            .ok_or_else(|| AppError::NotFound("Discord integration not found".to_string()))?;

        let discord_channel_id = update
            .discord_channel_id
            .unwrap_or(current.discord_channel_id);
        let discord_channel_name = update.discord_channel_name.or(current.discord_channel_name);
        let discord_webhook_url = update.discord_webhook_url.or(current.discord_webhook_url);
        let is_enabled = update.is_enabled.unwrap_or(current.is_enabled);
        let notify_stream_online = update
            .notify_stream_online
            .unwrap_or(current.notify_stream_online);
        let notify_stream_offline = update
            .notify_stream_offline
            .unwrap_or(current.notify_stream_offline);
        let notify_title_change = update
            .notify_title_change
            .unwrap_or(current.notify_title_change);
        let notify_category_change = update
            .notify_category_change
            .unwrap_or(current.notify_category_change);
        let notify_reward_redemption = update
            .notify_reward_redemption
            .unwrap_or(current.notify_reward_redemption);
        let calendar_sync_enabled = update
            .calendar_sync_enabled
            .unwrap_or(current.calendar_sync_enabled);
        let now = Utc::now().naive_utc();

        sqlx::query_as!(
            DiscordIntegration,
            r#"
            UPDATE discord_integrations
            SET discord_channel_id = ?,
                discord_channel_name = ?,
                discord_webhook_url = ?,
                is_enabled = ?,
                notify_stream_online = ?,
                notify_stream_offline = ?,
                notify_title_change = ?,
                notify_category_change = ?,
                notify_reward_redemption = ?,
                calendar_sync_enabled = ?,
                updated_at = ?
            WHERE id = ?
            RETURNING
                id as "id!: String",
                user_id as "user_id!: String",
                discord_guild_id as "discord_guild_id!: String",
                discord_channel_id as "discord_channel_id!: String",
                discord_guild_name as "discord_guild_name?: String",
                discord_channel_name as "discord_channel_name?: String",
                discord_webhook_url as "discord_webhook_url?: String",
                is_enabled as "is_enabled!: bool",
                notify_stream_online as "notify_stream_online!: bool",
                notify_stream_offline as "notify_stream_offline!: bool",
                notify_title_change as "notify_title_change!: bool",
                notify_category_change as "notify_category_change!: bool",
                notify_reward_redemption as "notify_reward_redemption!: bool",
                calendar_sync_enabled as "calendar_sync_enabled!: bool",
                created_at as "created_at!: chrono::NaiveDateTime",
                updated_at as "updated_at!: chrono::NaiveDateTime"
            "#,
            discord_channel_id,
            discord_channel_name,
            discord_webhook_url,
            is_enabled,
            notify_stream_online,
            notify_stream_offline,
            notify_title_change,
            notify_category_change,
            notify_reward_redemption,
            calendar_sync_enabled,
            now,
            id
        )
        .fetch_one(pool)
        .await
        .map_err(AppError::Database)
    }

    /// Delete Discord integration
    pub async fn delete(pool: &SqlitePool, id: &str) -> AppResult<()> {
        sqlx::query!("DELETE FROM discord_integrations WHERE id = ?", id)
            .execute(pool)
            .await
            .map_err(AppError::Database)?;

        Ok(())
    }

    /// Find all Discord integrations with calendar sync enabled
    pub async fn find_with_calendar_sync(pool: &SqlitePool) -> AppResult<Vec<DiscordIntegration>> {
        sqlx::query_as!(
            DiscordIntegration,
            r#"
            SELECT
                id as "id!: String",
                user_id as "user_id!: String",
                discord_guild_id as "discord_guild_id!: String",
                discord_channel_id as "discord_channel_id!: String",
                discord_guild_name as "discord_guild_name?: String",
                discord_channel_name as "discord_channel_name?: String",
                discord_webhook_url as "discord_webhook_url?: String",
                is_enabled as "is_enabled!: bool",
                notify_stream_online as "notify_stream_online!: bool",
                notify_stream_offline as "notify_stream_offline!: bool",
                notify_title_change as "notify_title_change!: bool",
                notify_category_change as "notify_category_change!: bool",
                notify_reward_redemption as "notify_reward_redemption!: bool",
                calendar_sync_enabled as "calendar_sync_enabled!: bool",
                created_at as "created_at!: chrono::NaiveDateTime",
                updated_at as "updated_at!: chrono::NaiveDateTime"
            FROM discord_integrations
            WHERE calendar_sync_enabled = ?
            "#,
            true
        )
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)
    }
}
