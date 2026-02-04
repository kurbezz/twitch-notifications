use chrono::Utc;

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
        sqlx::query_as!(
            User,
            r#"
            SELECT
                id as "id!: String",
                twitch_id as "twitch_id!: String",
                twitch_login as "twitch_login!: String",
                twitch_display_name as "twitch_display_name!: String",
                twitch_email as "twitch_email!: String",
                twitch_profile_image_url as "twitch_profile_image_url!: String",
                twitch_access_token as "twitch_access_token!: String",
                twitch_refresh_token as "twitch_refresh_token!: String",
                twitch_token_expires_at as "twitch_token_expires_at!: chrono::NaiveDateTime",
                telegram_user_id as "telegram_user_id?: String",
                telegram_username as "telegram_username?: String",
                telegram_photo_url as "telegram_photo_url?: String",
                discord_user_id as "discord_user_id?: String",
                discord_username as "discord_username?: String",
                discord_avatar_url as "discord_avatar_url?: String",
                lang as "lang?: String",
                created_at as "created_at!: chrono::NaiveDateTime",
                updated_at as "updated_at!: chrono::NaiveDateTime"
            FROM users
            WHERE id = ?
            "#,
            id
        )
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)
    }

    pub async fn find_by_twitch_id(pool: &SqlitePool, twitch_id: &str) -> AppResult<Option<User>> {
        sqlx::query_as!(
            User,
            r#"
            SELECT
                id as "id!: String",
                twitch_id as "twitch_id!: String",
                twitch_login as "twitch_login!: String",
                twitch_display_name as "twitch_display_name!: String",
                twitch_email as "twitch_email!: String",
                twitch_profile_image_url as "twitch_profile_image_url!: String",
                twitch_access_token as "twitch_access_token!: String",
                twitch_refresh_token as "twitch_refresh_token!: String",
                twitch_token_expires_at as "twitch_token_expires_at!: chrono::NaiveDateTime",
                telegram_user_id as "telegram_user_id?: String",
                telegram_username as "telegram_username?: String",
                telegram_photo_url as "telegram_photo_url?: String",
                discord_user_id as "discord_user_id?: String",
                discord_username as "discord_username?: String",
                discord_avatar_url as "discord_avatar_url?: String",
                lang as "lang?: String",
                created_at as "created_at!: chrono::NaiveDateTime",
                updated_at as "updated_at!: chrono::NaiveDateTime"
            FROM users
            WHERE twitch_id = ?
            "#,
            twitch_id
        )
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)
    }

    pub async fn find_by_login(pool: &SqlitePool, login: &str) -> AppResult<Option<User>> {
        sqlx::query_as!(
            User,
            r#"
            SELECT
                id as "id!: String",
                twitch_id as "twitch_id!: String",
                twitch_login as "twitch_login!: String",
                twitch_display_name as "twitch_display_name!: String",
                twitch_email as "twitch_email!: String",
                twitch_profile_image_url as "twitch_profile_image_url!: String",
                twitch_access_token as "twitch_access_token!: String",
                twitch_refresh_token as "twitch_refresh_token!: String",
                twitch_token_expires_at as "twitch_token_expires_at!: chrono::NaiveDateTime",
                telegram_user_id as "telegram_user_id?: String",
                telegram_username as "telegram_username?: String",
                telegram_photo_url as "telegram_photo_url?: String",
                discord_user_id as "discord_user_id?: String",
                discord_username as "discord_username?: String",
                discord_avatar_url as "discord_avatar_url?: String",
                lang as "lang?: String",
                created_at as "created_at!: chrono::NaiveDateTime",
                updated_at as "updated_at!: chrono::NaiveDateTime"
            FROM users
            WHERE twitch_login = ?
            "#,
            login
        )
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)
    }

    /// Search users by twitch login or display name (case-insensitive).
    /// Returns up to `limit` results ordered by twitch_login.
    pub async fn search(pool: &SqlitePool, query: &str, limit: i64) -> AppResult<Vec<User>> {
        if query.trim().is_empty() {
            return Ok(Vec::new());
        }

        let pattern = format!("%{}%", query.to_lowercase());

        sqlx::query_as!(
            User,
            r#"
            SELECT
                id as "id!: String",
                twitch_id as "twitch_id!: String",
                twitch_login as "twitch_login!: String",
                twitch_display_name as "twitch_display_name!: String",
                twitch_email as "twitch_email!: String",
                twitch_profile_image_url as "twitch_profile_image_url!: String",
                twitch_access_token as "twitch_access_token!: String",
                twitch_refresh_token as "twitch_refresh_token!: String",
                twitch_token_expires_at as "twitch_token_expires_at!: chrono::NaiveDateTime",
                telegram_user_id as "telegram_user_id?: String",
                telegram_username as "telegram_username?: String",
                telegram_photo_url as "telegram_photo_url?: String",
                discord_user_id as "discord_user_id?: String",
                discord_username as "discord_username?: String",
                discord_avatar_url as "discord_avatar_url?: String",
                lang as "lang?: String",
                created_at as "created_at!: chrono::NaiveDateTime",
                updated_at as "updated_at!: chrono::NaiveDateTime"
            FROM users
            WHERE LOWER(twitch_login) LIKE ? OR LOWER(twitch_display_name) LIKE ?
            ORDER BY twitch_login ASC
            LIMIT ?
            "#,
            pattern,
            pattern,
            limit
        )
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)
    }

    pub async fn list_all(pool: &SqlitePool) -> AppResult<Vec<User>> {
        sqlx::query_as!(
            User,
            r#"
            SELECT
                id as "id!: String",
                twitch_id as "twitch_id!: String",
                twitch_login as "twitch_login!: String",
                twitch_display_name as "twitch_display_name!: String",
                twitch_email as "twitch_email!: String",
                twitch_profile_image_url as "twitch_profile_image_url!: String",
                twitch_access_token as "twitch_access_token!: String",
                twitch_refresh_token as "twitch_refresh_token!: String",
                twitch_token_expires_at as "twitch_token_expires_at!: chrono::NaiveDateTime",
                telegram_user_id as "telegram_user_id?: String",
                telegram_username as "telegram_username?: String",
                telegram_photo_url as "telegram_photo_url?: String",
                discord_user_id as "discord_user_id?: String",
                discord_username as "discord_username?: String",
                discord_avatar_url as "discord_avatar_url?: String",
                lang as "lang?: String",
                created_at as "created_at!: chrono::NaiveDateTime",
                updated_at as "updated_at!: chrono::NaiveDateTime"
            FROM users
            ORDER BY created_at DESC
            "#
        )
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)
    }

    pub async fn update_tokens(
        pool: &SqlitePool,
        user_id: &str,
        access_token: &str,
        refresh_token: &str,
        token_expires_at: chrono::NaiveDateTime,
    ) -> AppResult<()> {
        let now = Utc::now();
        sqlx::query!(
            r#"
            UPDATE users
            SET
                twitch_access_token = ?,
                twitch_refresh_token = ?,
                twitch_token_expires_at = ?,
                updated_at = ?
            WHERE id = ?
            "#,
            access_token,
            refresh_token,
            token_expires_at,
            now,
            user_id
        )
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

            sqlx::query_as!(
                User,
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
                    id as "id!: String",
                    twitch_id as "twitch_id!: String",
                    twitch_login as "twitch_login!: String",
                    twitch_display_name as "twitch_display_name!: String",
                    twitch_email as "twitch_email!: String",
                    twitch_profile_image_url as "twitch_profile_image_url!: String",
                    twitch_access_token as "twitch_access_token!: String",
                    twitch_refresh_token as "twitch_refresh_token!: String",
                    twitch_token_expires_at as "twitch_token_expires_at!: chrono::NaiveDateTime",
                    telegram_user_id as "telegram_user_id?: String",
                    telegram_username as "telegram_username?: String",
                    telegram_photo_url as "telegram_photo_url?: String",
                    discord_user_id as "discord_user_id?: String",
                    discord_username as "discord_username?: String",
                    discord_avatar_url as "discord_avatar_url?: String",
                    lang as "lang?: String",
                    created_at as "created_at!: chrono::NaiveDateTime",
                    updated_at as "updated_at!: chrono::NaiveDateTime"
                "#,
                twitch_login,
                twitch_display_name,
                twitch_email,
                twitch_profile_image_url,
                now,
                user.id
            )
            .fetch_one(pool)
            .await
            .map_err(AppError::Database)?
        } else {
            // Create new user
            sqlx::query_as!(
                User,
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
                    id as "id!: String",
                    twitch_id as "twitch_id!: String",
                    twitch_login as "twitch_login!: String",
                    twitch_display_name as "twitch_display_name!: String",
                    twitch_email as "twitch_email!: String",
                    twitch_profile_image_url as "twitch_profile_image_url!: String",
                    twitch_access_token as "twitch_access_token!: String",
                    twitch_refresh_token as "twitch_refresh_token!: String",
                    twitch_token_expires_at as "twitch_token_expires_at!: chrono::NaiveDateTime",
                    telegram_user_id as "telegram_user_id?: String",
                    telegram_username as "telegram_username?: String",
                    telegram_photo_url as "telegram_photo_url?: String",
                    discord_user_id as "discord_user_id?: String",
                    discord_username as "discord_username?: String",
                    discord_avatar_url as "discord_avatar_url?: String",
                    lang as "lang?: String",
                    created_at as "created_at!: chrono::NaiveDateTime",
                    updated_at as "updated_at!: chrono::NaiveDateTime"
                "#,
                id,
                twitch_id,
                twitch_login,
                twitch_display_name,
                twitch_email,
                twitch_profile_image_url,
                twitch_access_token,
                twitch_refresh_token,
                twitch_token_expires_at,
                None::<String>,
                None::<String>,
                None::<String>,
                None::<String>,
                None::<String>,
                None::<String>,
                lang,
                now,
                now
            )
            .fetch_one(pool)
            .await
            .map_err(AppError::Database)?
        };

        Ok(result_user)
    }

    pub async fn set_telegram_info(
        pool: &SqlitePool,
        user_id: &str,
        telegram_user_id: &str,
        telegram_username: Option<&str>,
        telegram_photo_url: Option<&str>,
    ) -> AppResult<()> {
        let now = Utc::now();
        sqlx::query!(
            r#"
            UPDATE users
            SET
                telegram_user_id = ?,
                telegram_username = ?,
                telegram_photo_url = ?,
                updated_at = ?
            WHERE id = ?
            "#,
            telegram_user_id,
            telegram_username,
            telegram_photo_url,
            now,
            user_id
        )
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
        Ok(())
    }

    pub async fn set_lang(pool: &SqlitePool, user_id: &str, lang: Option<&str>) -> AppResult<()> {
        let now = Utc::now();
        sqlx::query!(
            r#"
            UPDATE users
            SET
                lang = ?,
                updated_at = ?
            WHERE id = ?
            "#,
            lang,
            now,
            user_id
        )
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
        Ok(())
    }

    pub async fn clear_telegram_info(pool: &SqlitePool, user_id: &str) -> AppResult<()> {
        let now = Utc::now();
        sqlx::query!(
            r#"
            UPDATE users
            SET
                telegram_user_id = NULL,
                telegram_username = NULL,
                telegram_photo_url = NULL,
                updated_at = ?
            WHERE id = ?
            "#,
            now,
            user_id
        )
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
        sqlx::query!(
            r#"
            UPDATE users
            SET
                discord_user_id = ?,
                discord_username = ?,
                discord_avatar_url = ?,
                updated_at = ?
            WHERE id = ?
            "#,
            discord_user_id,
            discord_username,
            discord_avatar_url,
            now,
            user_id
        )
        .execute(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(())
    }

    pub async fn clear_discord_info(pool: &SqlitePool, user_id: &str) -> AppResult<()> {
        let now = Utc::now();
        sqlx::query!(
            r#"
            UPDATE users
            SET
                discord_user_id = NULL,
                discord_username = NULL,
                discord_avatar_url = NULL,
                updated_at = ?
            WHERE id = ?
            "#,
            now,
            user_id
        )
        .execute(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use sqlx::sqlite::SqlitePoolOptions;
    // `SqlitePool` is already imported at the module level; avoid unused import here.
    use uuid::Uuid;

    #[tokio::test]
    async fn search_includes_lang() -> anyhow::Result<()> {
        // Use in-memory SQLite pool with a single connection for tests
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await?;

        // Create a minimal users table (including `lang`)
        sqlx::query(
            r#"
            CREATE TABLE users (
                id TEXT PRIMARY KEY,
                twitch_id TEXT NOT NULL,
                twitch_login TEXT NOT NULL,
                twitch_display_name TEXT NOT NULL,
                twitch_email TEXT,
                twitch_profile_image_url TEXT,
                twitch_access_token TEXT,
                twitch_refresh_token TEXT,
                twitch_token_expires_at DATETIME,
                telegram_user_id TEXT,
                telegram_username TEXT,
                telegram_photo_url TEXT,
                discord_user_id TEXT,
                discord_username TEXT,
                discord_avatar_url TEXT,
                lang TEXT DEFAULT 'ru',
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );
            "#,
        )
        .execute(&pool)
        .await?;

        let now = Utc::now().naive_utc();
        let id = Uuid::new_v4().to_string();

        sqlx::query(
            r#"
            INSERT INTO users (
                id, twitch_id, twitch_login, twitch_display_name,
                twitch_email, twitch_profile_image_url,
                twitch_access_token, twitch_refresh_token, twitch_token_expires_at,
                lang, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind("t123")
        .bind("login")
        .bind("display")
        .bind("email")
        .bind("profile")
        .bind("access")
        .bind("refresh")
        .bind(now)
        .bind(Some("en"))
        .bind(now)
        .bind(now)
        .execute(&pool)
        .await?;

        let results = UserRepository::search(&pool, "login", 10)
            .await
            .map_err(|e| anyhow::anyhow!(format!("{:?}", e)))?;

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].lang.as_deref(), Some("en"));

        Ok(())
    }
}
