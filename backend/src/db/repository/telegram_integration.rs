use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::db::models::*;
use crate::db::UserRepository;
use crate::error::{AppError, AppResult};

// Intermediate structure for reading from DB (with String for chat_type)
#[derive(sqlx::FromRow)]
struct RowTelegramIntegration {
    id: String,
    user_id: String,
    telegram_chat_id: String,
    telegram_chat_title: Option<String>,
    telegram_chat_type: Option<String>,
    is_enabled: bool,
    notify_stream_online: bool,
    notify_stream_offline: bool,
    notify_title_change: bool,
    notify_category_change: bool,
    notify_reward_redemption: bool,
    created_at: chrono::NaiveDateTime,
    updated_at: chrono::NaiveDateTime,
}

impl From<RowTelegramIntegration> for TelegramIntegration {
    fn from(row: RowTelegramIntegration) -> Self {
        TelegramIntegration {
            id: row.id,
            user_id: row.user_id,
            telegram_chat_id: row.telegram_chat_id,
            telegram_chat_title: row.telegram_chat_title,
            telegram_chat_type: row
                .telegram_chat_type
                .as_deref()
                .and_then(ChatType::from_str),
            is_enabled: row.is_enabled,
            notify_stream_online: row.notify_stream_online,
            notify_stream_offline: row.notify_stream_offline,
            notify_title_change: row.notify_title_change,
            notify_category_change: row.notify_category_change,
            notify_reward_redemption: row.notify_reward_redemption,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

// ============================================================================
// Telegram Integration Repository
// ============================================================================

pub struct TelegramIntegrationRepository;

impl TelegramIntegrationRepository {
    pub async fn create(
        pool: &SqlitePool,
        user_id: &str,
        integration: CreateTelegramIntegration,
    ) -> AppResult<TelegramIntegration> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().naive_utc();

        // Ensure the owner has linked a Telegram account before creating an integration.
        let owner = UserRepository::find_by_id(pool, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;
        if owner.telegram_user_id.is_none() {
            return Err(AppError::Validation(crate::i18n::t(
                "validation.owner_telegram_not_linked",
            )));
        }

        let chat_type_str = integration
            .telegram_chat_type
            .map(|ct| ct.as_str().to_string());

        let row = sqlx::query_as!(
            RowTelegramIntegration,
            r#"
            INSERT INTO telegram_integrations (
                id, user_id, telegram_chat_id, telegram_chat_title, telegram_chat_type,
                is_enabled, notify_stream_online, notify_stream_offline,
                notify_title_change, notify_category_change, notify_reward_redemption,
                created_at, updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING
                id as "id!: String",
                user_id as "user_id!: String",
                telegram_chat_id as "telegram_chat_id!: String",
                telegram_chat_title as "telegram_chat_title?: String",
                telegram_chat_type as "telegram_chat_type?: String",
                is_enabled as "is_enabled!: bool",
                notify_stream_online as "notify_stream_online!: bool",
                notify_stream_offline as "notify_stream_offline!: bool",
                notify_title_change as "notify_title_change!: bool",
                notify_category_change as "notify_category_change!: bool",
                notify_reward_redemption as "notify_reward_redemption!: bool",
                created_at as "created_at!: chrono::NaiveDateTime",
                updated_at as "updated_at!: chrono::NaiveDateTime"
            "#,
            id,
            user_id,
            integration.telegram_chat_id,
            integration.telegram_chat_title,
            chat_type_str,
            true,
            true,
            false,
            true,
            true,
            false,
            now,
            now
        )
        .fetch_one(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(row.into())
    }

    pub async fn find_by_id(pool: &SqlitePool, id: &str) -> AppResult<Option<TelegramIntegration>> {
        let row = sqlx::query_as!(
            RowTelegramIntegration,
            r#"
            SELECT
                id as "id!: String",
                user_id as "user_id!: String",
                telegram_chat_id as "telegram_chat_id!: String",
                telegram_chat_title as "telegram_chat_title?: String",
                telegram_chat_type as "telegram_chat_type?: String",
                is_enabled as "is_enabled!: bool",
                notify_stream_online as "notify_stream_online!: bool",
                notify_stream_offline as "notify_stream_offline!: bool",
                notify_title_change as "notify_title_change!: bool",
                notify_category_change as "notify_category_change!: bool",
                notify_reward_redemption as "notify_reward_redemption!: bool",
                created_at as "created_at!: chrono::NaiveDateTime",
                updated_at as "updated_at!: chrono::NaiveDateTime"
            FROM telegram_integrations
            WHERE id = ?
            "#,
            id
        )
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(row.map(Into::into))
    }

    pub async fn find_by_user_id(
        pool: &SqlitePool,
        user_id: &str,
    ) -> AppResult<Vec<TelegramIntegration>> {
        let rows = sqlx::query_as!(
            RowTelegramIntegration,
            r#"
            SELECT
                id as "id!: String",
                user_id as "user_id!: String",
                telegram_chat_id as "telegram_chat_id!: String",
                telegram_chat_title as "telegram_chat_title?: String",
                telegram_chat_type as "telegram_chat_type?: String",
                is_enabled as "is_enabled!: bool",
                notify_stream_online as "notify_stream_online!: bool",
                notify_stream_offline as "notify_stream_offline!: bool",
                notify_title_change as "notify_title_change!: bool",
                notify_category_change as "notify_category_change!: bool",
                notify_reward_redemption as "notify_reward_redemption!: bool",
                created_at as "created_at!: chrono::NaiveDateTime",
                updated_at as "updated_at!: chrono::NaiveDateTime"
            FROM telegram_integrations
            WHERE user_id = ?
            ORDER BY created_at DESC
            "#,
            user_id
        )
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn find_by_chat_id(
        pool: &SqlitePool,
        chat_id: &str,
    ) -> AppResult<Vec<TelegramIntegration>> {
        let rows = sqlx::query_as!(
            RowTelegramIntegration,
            r#"
            SELECT
                id as "id!: String",
                user_id as "user_id!: String",
                telegram_chat_id as "telegram_chat_id!: String",
                telegram_chat_title as "telegram_chat_title?: String",
                telegram_chat_type as "telegram_chat_type?: String",
                is_enabled as "is_enabled!: bool",
                notify_stream_online as "notify_stream_online!: bool",
                notify_stream_offline as "notify_stream_offline!: bool",
                notify_title_change as "notify_title_change!: bool",
                notify_category_change as "notify_category_change!: bool",
                notify_reward_redemption as "notify_reward_redemption!: bool",
                created_at as "created_at!: chrono::NaiveDateTime",
                updated_at as "updated_at!: chrono::NaiveDateTime"
            FROM telegram_integrations
            WHERE telegram_chat_id = ?
            "#,
            chat_id
        )
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn exists(pool: &SqlitePool, chat_id: &str, user_id: &str) -> AppResult<bool> {
        let row = sqlx::query!(
            r#"
            SELECT 1 as "exists!: i64"
            FROM telegram_integrations
            WHERE telegram_chat_id = ? AND user_id = ?
            LIMIT 1
            "#,
            chat_id,
            user_id
        )
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(row.is_some())
    }

    pub async fn update(
        pool: &SqlitePool,
        id: &str,
        update: UpdateTelegramIntegration,
    ) -> AppResult<TelegramIntegration> {
        let current = Self::find_by_id(pool, id)
            .await?
            .ok_or_else(|| AppError::NotFound("Telegram integration not found".to_string()))?;

        // Bind update values into local variables to avoid creating temporaries
        // that would be borrowed across the query call.
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
        let now = Utc::now().naive_utc();

        let row = sqlx::query_as!(
            RowTelegramIntegration,
            r#"
            UPDATE telegram_integrations
            SET is_enabled = ?,
                notify_stream_online = ?,
                notify_stream_offline = ?,
                notify_title_change = ?,
                notify_category_change = ?,
                notify_reward_redemption = ?,
                updated_at = ?
            WHERE id = ?
            RETURNING
                id as "id!: String",
                user_id as "user_id!: String",
                telegram_chat_id as "telegram_chat_id!: String",
                telegram_chat_title as "telegram_chat_title?: String",
                telegram_chat_type as "telegram_chat_type?: String",
                is_enabled as "is_enabled!: bool",
                notify_stream_online as "notify_stream_online!: bool",
                notify_stream_offline as "notify_stream_offline!: bool",
                notify_title_change as "notify_title_change!: bool",
                notify_category_change as "notify_category_change!: bool",
                notify_reward_redemption as "notify_reward_redemption!: bool",
                created_at as "created_at!: chrono::NaiveDateTime",
                updated_at as "updated_at!: chrono::NaiveDateTime"
            "#,
            is_enabled,
            notify_stream_online,
            notify_stream_offline,
            notify_title_change,
            notify_category_change,
            notify_reward_redemption,
            now,
            id
        )
        .fetch_one(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(row.into())
    }

    pub async fn delete(pool: &SqlitePool, id: &str) -> AppResult<()> {
        sqlx::query!("DELETE FROM telegram_integrations WHERE id = ?", id)
            .execute(pool)
            .await
            .map_err(AppError::Database)?;

        Ok(())
    }

    pub async fn find_enabled_for_user(
        pool: &SqlitePool,
        user_id: &str,
    ) -> AppResult<Vec<TelegramIntegration>> {
        let rows = sqlx::query_as!(
            RowTelegramIntegration,
            r#"
            SELECT
                id as "id!: String",
                user_id as "user_id!: String",
                telegram_chat_id as "telegram_chat_id!: String",
                telegram_chat_title as "telegram_chat_title?: String",
                telegram_chat_type as "telegram_chat_type?: String",
                is_enabled as "is_enabled!: bool",
                notify_stream_online as "notify_stream_online!: bool",
                notify_stream_offline as "notify_stream_offline!: bool",
                notify_title_change as "notify_title_change!: bool",
                notify_category_change as "notify_category_change!: bool",
                notify_reward_redemption as "notify_reward_redemption!: bool",
                created_at as "created_at!: chrono::NaiveDateTime",
                updated_at as "updated_at!: chrono::NaiveDateTime"
            FROM telegram_integrations
            WHERE user_id = ? AND is_enabled = ?
            "#,
            user_id,
            true
        )
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(rows.into_iter().map(Into::into).collect())
    }
}
