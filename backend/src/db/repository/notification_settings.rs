use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::db::models::*;
use crate::error::{AppError, AppResult};

// ============================================================================
// Notification Settings Repository
// ============================================================================

pub struct NotificationSettingsRepository;

impl NotificationSettingsRepository {
    pub async fn create(pool: &SqlitePool, user_id: &str) -> AppResult<NotificationSettings> {
        let id = Uuid::new_v4().to_string();
        let defaults = NotificationSettings::default();
        let now = Utc::now().naive_utc();

        let settings = sqlx::query_as!(
            NotificationSettings,
            r#"
            INSERT INTO user_settings (
                id, user_id,
                stream_online_message, stream_offline_message,
                stream_title_change_message,
                stream_category_change_message, reward_redemption_message, notify_reward_redemption,
                created_at, updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING
                id as "id!: String",
                user_id as "user_id!: String",
                stream_online_message as "stream_online_message!: String",
                stream_offline_message as "stream_offline_message!: String",
                stream_title_change_message as "stream_title_change_message!: String",
                stream_category_change_message as "stream_category_change_message!: String",
                reward_redemption_message as "reward_redemption_message!: String",
                notify_reward_redemption as "notify_reward_redemption!: bool",
                created_at as "created_at!: chrono::NaiveDateTime",
                updated_at as "updated_at!: chrono::NaiveDateTime"
            "#,
            id,
            user_id,
            defaults.stream_online_message,
            defaults.stream_offline_message,
            defaults.stream_title_change_message,
            defaults.stream_category_change_message,
            defaults.reward_redemption_message,
            defaults.notify_reward_redemption,
            now,
            now
        )
        .fetch_one(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(settings)
    }

    pub async fn find_by_user_id(
        pool: &SqlitePool,
        user_id: &str,
    ) -> AppResult<Option<NotificationSettings>> {
        sqlx::query_as!(
            NotificationSettings,
            r#"
            SELECT
                id as "id!: String",
                user_id as "user_id!: String",
                stream_online_message as "stream_online_message!: String",
                stream_offline_message as "stream_offline_message!: String",
                stream_title_change_message as "stream_title_change_message!: String",
                stream_category_change_message as "stream_category_change_message!: String",
                reward_redemption_message as "reward_redemption_message!: String",
                notify_reward_redemption as "notify_reward_redemption!: bool",
                created_at as "created_at!: chrono::NaiveDateTime",
                updated_at as "updated_at!: chrono::NaiveDateTime"
            FROM user_settings
            WHERE user_id = ?
            "#,
            user_id
        )
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)
    }

    pub async fn get_or_create(
        pool: &SqlitePool,
        user_id: &str,
    ) -> AppResult<NotificationSettings> {
        if let Some(settings) = Self::find_by_user_id(pool, user_id).await? {
            // If any message templates are empty (e.g. legacy DB with empty defaults),
            // persist default templates and return updated settings.
            let needs_patch = settings.stream_online_message.trim().is_empty()
                || settings.stream_offline_message.trim().is_empty()
                || settings.stream_title_change_message.trim().is_empty()
                || settings.stream_category_change_message.trim().is_empty()
                || settings.reward_redemption_message.trim().is_empty();

            if needs_patch {
                let defaults = NotificationSettings::default();
                // Update only empty columns (avoid overwriting non-empty values)
                let now = Utc::now().naive_utc();
                let settings = sqlx::query_as!(
                    NotificationSettings,
                    r#"
                    UPDATE user_settings
                    SET
                        stream_online_message = CASE WHEN stream_online_message = '' THEN ? ELSE stream_online_message END,
                        stream_offline_message = CASE WHEN stream_offline_message = '' THEN ? ELSE stream_offline_message END,
                        stream_title_change_message = CASE WHEN stream_title_change_message = '' THEN ? ELSE stream_title_change_message END,
                        stream_category_change_message = CASE WHEN stream_category_change_message = '' THEN ? ELSE stream_category_change_message END,
                        reward_redemption_message = CASE WHEN reward_redemption_message = '' THEN ? ELSE reward_redemption_message END,
                        updated_at = ?
                    WHERE user_id = ?
                    RETURNING
                        id as "id!: String",
                        user_id as "user_id!: String",
                        stream_online_message as "stream_online_message!: String",
                        stream_offline_message as "stream_offline_message!: String",
                        stream_title_change_message as "stream_title_change_message!: String",
                        stream_category_change_message as "stream_category_change_message!: String",
                        reward_redemption_message as "reward_redemption_message!: String",
                        notify_reward_redemption as "notify_reward_redemption!: bool",
                        created_at as "created_at!: chrono::NaiveDateTime",
                        updated_at as "updated_at!: chrono::NaiveDateTime"
                    "#,
                    defaults.stream_online_message,
                    defaults.stream_offline_message,
                    defaults.stream_title_change_message,
                    defaults.stream_category_change_message,
                    defaults.reward_redemption_message,
                    now,
                    user_id
                )
                .fetch_one(pool)
                .await
                .map_err(AppError::Database)?;

                return Ok(settings);
            }

            Ok(settings)
        } else {
            Self::create(pool, user_id).await
        }
    }

    pub async fn update(
        pool: &SqlitePool,
        user_id: &str,
        update: UpdateNotificationSettings,
    ) -> AppResult<NotificationSettings> {
        let current = Self::get_or_create(pool, user_id).await?;

        let stream_online_message = update
            .stream_online_message
            .unwrap_or(current.stream_online_message);
        let stream_offline_message = update
            .stream_offline_message
            .unwrap_or(current.stream_offline_message);
        let stream_title_change_message = update
            .stream_title_change_message
            .unwrap_or(current.stream_title_change_message);
        let stream_category_change_message = update
            .stream_category_change_message
            .unwrap_or(current.stream_category_change_message);
        let reward_redemption_message = update
            .reward_redemption_message
            .unwrap_or(current.reward_redemption_message);
        let notify_reward_redemption = update
            .notify_reward_redemption
            .unwrap_or(current.notify_reward_redemption);

        let now = Utc::now().naive_utc();
        sqlx::query_as!(
            NotificationSettings,
            r#"
            UPDATE user_settings
            SET stream_online_message = ?,
                stream_offline_message = ?,
                stream_title_change_message = ?,
                stream_category_change_message = ?,
                reward_redemption_message = ?,
                notify_reward_redemption = ?,
                updated_at = ?
            WHERE user_id = ?
            RETURNING
                id as "id!: String",
                user_id as "user_id!: String",
                stream_online_message as "stream_online_message!: String",
                stream_offline_message as "stream_offline_message!: String",
                stream_title_change_message as "stream_title_change_message!: String",
                stream_category_change_message as "stream_category_change_message!: String",
                reward_redemption_message as "reward_redemption_message!: String",
                notify_reward_redemption as "notify_reward_redemption!: bool",
                created_at as "created_at!: chrono::NaiveDateTime",
                updated_at as "updated_at!: chrono::NaiveDateTime"
            "#,
            stream_online_message,
            stream_offline_message,
            stream_title_change_message,
            stream_category_change_message,
            reward_redemption_message,
            notify_reward_redemption,
            now,
            user_id
        )
        .fetch_one(pool)
        .await
        .map_err(AppError::Database)
    }
}
