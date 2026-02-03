use chrono::Utc;

use sqlx::Row;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::db::models::*;
use crate::error::{AppError, AppResult};

// ============================================================================
// User Repository
// ============================================================================

pub struct UserRepository;

impl UserRepository {
    pub async fn find_by_id(pool: &SqlitePool, id: &str) -> AppResult<Option<User>> {
        let row = sqlx::query(
            r#"
            SELECT
                id, twitch_id, twitch_login, twitch_display_name,
                twitch_email, twitch_profile_image_url,
                twitch_access_token, twitch_refresh_token, twitch_token_expires_at,
                telegram_user_id, telegram_username, telegram_photo_url,
                discord_user_id, discord_username, discord_avatar_url,
                lang,
                created_at, updated_at
            FROM users
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(row.map(|r| User {
            id: r.get("id"),
            twitch_id: r.get("twitch_id"),
            twitch_login: r.get("twitch_login"),
            twitch_display_name: r.get("twitch_display_name"),
            twitch_email: r.get("twitch_email"),
            twitch_profile_image_url: r.get("twitch_profile_image_url"),
            twitch_access_token: r.get("twitch_access_token"),
            twitch_refresh_token: r.get("twitch_refresh_token"),
            twitch_token_expires_at: r.get("twitch_token_expires_at"),
            telegram_user_id: r.get("telegram_user_id"),
            telegram_username: r.get("telegram_username"),
            telegram_photo_url: r.get("telegram_photo_url"),
            discord_user_id: r.get("discord_user_id"),
            discord_username: r.get("discord_username"),
            discord_avatar_url: r.get("discord_avatar_url"),
            lang: r.get("lang"),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        }))
    }

    pub async fn find_by_twitch_id(pool: &SqlitePool, twitch_id: &str) -> AppResult<Option<User>> {
        let row = sqlx::query(
            r#"
            SELECT
                id, twitch_id, twitch_login, twitch_display_name,
                twitch_email, twitch_profile_image_url,
                twitch_access_token, twitch_refresh_token, twitch_token_expires_at,
                telegram_user_id, telegram_username, telegram_photo_url,
                discord_user_id, discord_username, discord_avatar_url,
                lang,
                created_at, updated_at
            FROM users
            WHERE twitch_id = ?
            "#,
        )
        .bind(twitch_id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(row.map(|r| User {
            id: r.get("id"),
            twitch_id: r.get("twitch_id"),
            twitch_login: r.get("twitch_login"),
            twitch_display_name: r.get("twitch_display_name"),
            twitch_email: r.get("twitch_email"),
            twitch_profile_image_url: r.get("twitch_profile_image_url"),
            twitch_access_token: r.get("twitch_access_token"),
            twitch_refresh_token: r.get("twitch_refresh_token"),
            twitch_token_expires_at: r.get("twitch_token_expires_at"),
            telegram_user_id: r.get("telegram_user_id"),
            telegram_username: r.get("telegram_username"),
            telegram_photo_url: r.get("telegram_photo_url"),
            discord_user_id: r.get("discord_user_id"),
            discord_username: r.get("discord_username"),
            discord_avatar_url: r.get("discord_avatar_url"),
            lang: r.get("lang"),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        }))
    }

    pub async fn find_by_login(pool: &SqlitePool, login: &str) -> AppResult<Option<User>> {
        let row = sqlx::query(
            r#"
            SELECT
                id, twitch_id, twitch_login, twitch_display_name,
                twitch_email, twitch_profile_image_url,
                twitch_access_token, twitch_refresh_token, twitch_token_expires_at,
                telegram_user_id, telegram_username, telegram_photo_url,
                discord_user_id, discord_username, discord_avatar_url,
                lang,
                created_at, updated_at
            FROM users
            WHERE twitch_login = ?
            "#,
        )
        .bind(login)
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(row.map(|r| User {
            id: r.get("id"),
            twitch_id: r.get("twitch_id"),
            twitch_login: r.get("twitch_login"),
            twitch_display_name: r.get("twitch_display_name"),
            twitch_email: r.get("twitch_email"),
            twitch_profile_image_url: r.get("twitch_profile_image_url"),
            twitch_access_token: r.get("twitch_access_token"),
            twitch_refresh_token: r.get("twitch_refresh_token"),
            twitch_token_expires_at: r.get("twitch_token_expires_at"),
            telegram_user_id: r.get("telegram_user_id"),
            telegram_username: r.get("telegram_username"),
            telegram_photo_url: r.get("telegram_photo_url"),
            discord_user_id: r.get("discord_user_id"),
            discord_username: r.get("discord_username"),
            discord_avatar_url: r.get("discord_avatar_url"),
            lang: r.get("lang"),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        }))
    }

    /// Search users by twitch login or display name (case-insensitive).
    /// Returns up to `limit` results ordered by twitch_login.
    pub async fn search(pool: &SqlitePool, query: &str, limit: i64) -> AppResult<Vec<User>> {
        if query.trim().is_empty() {
            return Ok(Vec::new());
        }

        let pattern = format!("%{}%", query.to_lowercase());

        let rows = sqlx::query(
            r#"
            SELECT
                id, twitch_id, twitch_login, twitch_display_name,
                twitch_email, twitch_profile_image_url,
                twitch_access_token, twitch_refresh_token, twitch_token_expires_at,
                telegram_user_id, telegram_username, telegram_photo_url,
                discord_user_id, discord_username, discord_avatar_url,
                created_at, updated_at
            FROM users
            WHERE LOWER(twitch_login) LIKE ? OR LOWER(twitch_display_name) LIKE ?
            ORDER BY twitch_login ASC
            LIMIT ?
            "#,
        )
        .bind(&pattern)
        .bind(&pattern)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(rows
            .into_iter()
            .map(|r| User {
                id: r.get("id"),
                twitch_id: r.get("twitch_id"),
                twitch_login: r.get("twitch_login"),
                twitch_display_name: r.get("twitch_display_name"),
                twitch_email: r.get("twitch_email"),
                twitch_profile_image_url: r.get("twitch_profile_image_url"),
                twitch_access_token: r.get("twitch_access_token"),
                twitch_refresh_token: r.get("twitch_refresh_token"),
                twitch_token_expires_at: r.get("twitch_token_expires_at"),
                telegram_user_id: r.get("telegram_user_id"),
                telegram_username: r.get("telegram_username"),
                telegram_photo_url: r.get("telegram_photo_url"),
                discord_user_id: r.get("discord_user_id"),
                discord_username: r.get("discord_username"),
                discord_avatar_url: r.get("discord_avatar_url"),
                lang: r.get("lang"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
            })
            .collect())
    }

    pub async fn list_all(pool: &SqlitePool) -> AppResult<Vec<User>> {
        let rows = sqlx::query(
            r#"
            SELECT
                id, twitch_id, twitch_login, twitch_display_name,
                twitch_email, twitch_profile_image_url,
                twitch_access_token, twitch_refresh_token, twitch_token_expires_at,
                telegram_user_id, telegram_username, telegram_photo_url,
                discord_user_id, discord_username, discord_avatar_url,
                lang,
                created_at, updated_at
            FROM users
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(rows
            .into_iter()
            .map(|r| User {
                id: r.get("id"),
                twitch_id: r.get("twitch_id"),
                twitch_login: r.get("twitch_login"),
                twitch_display_name: r.get("twitch_display_name"),
                twitch_email: r.get("twitch_email"),
                twitch_profile_image_url: r.get("twitch_profile_image_url"),
                twitch_access_token: r.get("twitch_access_token"),
                twitch_refresh_token: r.get("twitch_refresh_token"),
                twitch_token_expires_at: r.get("twitch_token_expires_at"),
                telegram_user_id: r.get("telegram_user_id"),
                telegram_username: r.get("telegram_username"),
                telegram_photo_url: r.get("telegram_photo_url"),
                discord_user_id: r.get("discord_user_id"),
                discord_username: r.get("discord_username"),
                discord_avatar_url: r.get("discord_avatar_url"),
                lang: r.get("lang"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
            })
            .collect())
    }

    pub async fn update_tokens(
        pool: &SqlitePool,
        user_id: &str,
        access_token: &str,
        refresh_token: &str,
        token_expires_at: chrono::NaiveDateTime,
    ) -> AppResult<()> {
        let now = Utc::now();
        sqlx::query(
            r#"
            UPDATE users
            SET
                twitch_access_token = ?,
                twitch_refresh_token = ?,
                twitch_token_expires_at = ?,
                updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(access_token)
        .bind(refresh_token)
        .bind(token_expires_at)
        .bind(now)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_by_twitch_id(
        pool: &SqlitePool,
        twitch_id: &str,
        twitch_login: &str,
        twitch_display_name: &str,
        twitch_email: &str,
        twitch_profile_image_url: &str,
        twitch_access_token: &str,
        twitch_refresh_token: &str,
        twitch_token_expires_at: chrono::NaiveDateTime,
        lang: Option<&str>,
    ) -> AppResult<User> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        // Check if user exists
        let existing_user = Self::find_by_twitch_id(pool, twitch_id).await?;

        let result_user = if let Some(user) = existing_user {
            // Update existing user
            Self::update_tokens(
                pool,
                &user.id,
                twitch_access_token,
                twitch_refresh_token,
                twitch_token_expires_at,
            )
            .await?;

            sqlx::query(
                r#"
                UPDATE users
                SET
                    twitch_login = ?,
                    twitch_display_name = ?,
                    twitch_email = ?,
                    twitch_profile_image_url = ?,
                    updated_at = ?
                WHERE id = ?
                RETURNING
                    id, twitch_id, twitch_login, twitch_display_name,
                    twitch_email, twitch_profile_image_url,
                    twitch_access_token, twitch_refresh_token, twitch_token_expires_at,
                    telegram_user_id, telegram_username, telegram_photo_url,
                    discord_user_id, discord_username, discord_avatar_url,
                    lang,
                    created_at, updated_at
                "#,
            )
            .bind(twitch_login)
            .bind(twitch_display_name)
            .bind(twitch_email)
            .bind(twitch_profile_image_url)
            .bind(now)
            .bind(&user.id)
            .fetch_one(pool)
            .await
            .map_err(AppError::Database)?
        } else {
            // Create new user
            sqlx::query(
                r#"
                INSERT INTO users (
                    id, twitch_id, twitch_login, twitch_display_name,
                    twitch_email, twitch_profile_image_url,
                    twitch_access_token, twitch_refresh_token, twitch_token_expires_at,
                    telegram_user_id, telegram_username, telegram_photo_url,
                    discord_user_id, discord_username, discord_avatar_url,
                    lang,
                    created_at, updated_at
                )
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                RETURNING
                    id, twitch_id, twitch_login, twitch_display_name,
                    twitch_email, twitch_profile_image_url,
                    twitch_access_token, twitch_refresh_token, twitch_token_expires_at,
                    telegram_user_id, telegram_username, telegram_photo_url,
                    discord_user_id, discord_username, discord_avatar_url,
                    lang,
                    created_at, updated_at
                "#,
            )
            .bind(&id)
            .bind(twitch_id)
            .bind(twitch_login)
            .bind(twitch_display_name)
            .bind(twitch_email)
            .bind(twitch_profile_image_url)
            .bind(twitch_access_token)
            .bind(twitch_refresh_token)
            .bind(twitch_token_expires_at)
            .bind(None::<String>)
            .bind(None::<String>)
            .bind(None::<String>)
            .bind(None::<String>)
            .bind(None::<String>)
            .bind(None::<String>)
            .bind(lang)
            .bind(now)
            .bind(now)
            .fetch_one(pool)
            .await
            .map_err(AppError::Database)?
        };

        Ok(User {
            id: result_user.get("id"),
            twitch_id: result_user.get("twitch_id"),
            twitch_login: result_user.get("twitch_login"),
            twitch_display_name: result_user.get("twitch_display_name"),
            twitch_email: result_user.get("twitch_email"),
            twitch_profile_image_url: result_user.get("twitch_profile_image_url"),
            twitch_access_token: result_user.get("twitch_access_token"),
            twitch_refresh_token: result_user.get("twitch_refresh_token"),
            twitch_token_expires_at: result_user.get("twitch_token_expires_at"),
            telegram_user_id: result_user.get("telegram_user_id"),
            telegram_username: result_user.get("telegram_username"),
            telegram_photo_url: result_user.get("telegram_photo_url"),
            discord_user_id: result_user.get("discord_user_id"),
            discord_username: result_user.get("discord_username"),
            discord_avatar_url: result_user.get("discord_avatar_url"),
            lang: result_user.get("lang"),
            created_at: result_user.get("created_at"),
            updated_at: result_user.get("updated_at"),
        })
    }

    pub async fn set_telegram_info(
        pool: &SqlitePool,
        user_id: &str,
        telegram_user_id: &str,
        telegram_username: Option<&str>,
        telegram_photo_url: Option<&str>,
    ) -> AppResult<()> {
        let now = Utc::now();
        sqlx::query(
            r#"
            UPDATE users
            SET
                telegram_user_id = ?,
                telegram_username = ?,
                telegram_photo_url = ?,
                updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(telegram_user_id)
        .bind(telegram_username)
        .bind(telegram_photo_url)
        .bind(now)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
        Ok(())
    }

    pub async fn set_lang(pool: &SqlitePool, user_id: &str, lang: Option<&str>) -> AppResult<()> {
        let now = Utc::now();
        sqlx::query(
            r#"
            UPDATE users
            SET
                lang = ?,
                updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(lang)
        .bind(now)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
        Ok(())
    }

    pub async fn clear_telegram_info(pool: &SqlitePool, user_id: &str) -> AppResult<()> {
        let now = Utc::now();
        sqlx::query(
            r#"
            UPDATE users
            SET
                telegram_user_id = NULL,
                telegram_username = NULL,
                telegram_photo_url = NULL,
                updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(now)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(())
    }

    pub async fn set_discord_info(
        pool: &SqlitePool,
        user_id: &str,
        discord_user_id: &str,
        discord_username: &str,
        discord_avatar_url: &str,
    ) -> AppResult<()> {
        let now = Utc::now();
        sqlx::query(
            r#"
            UPDATE users
            SET
                discord_user_id = ?,
                discord_username = ?,
                discord_avatar_url = ?,
                updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(discord_user_id)
        .bind(discord_username)
        .bind(discord_avatar_url)
        .bind(now)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(())
    }

    pub async fn clear_discord_info(pool: &SqlitePool, user_id: &str) -> AppResult<()> {
        let now = Utc::now();
        sqlx::query(
            r#"
            UPDATE users
            SET
                discord_user_id = NULL,
                discord_username = NULL,
                discord_avatar_url = NULL,
                updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(now)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(())
    }
}
