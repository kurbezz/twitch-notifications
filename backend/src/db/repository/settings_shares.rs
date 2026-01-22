use chrono::NaiveDateTime;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::db::models::SettingsShare;
use crate::error::{AppError, AppResult};

// ============================================================================
// Settings Share Repository
// ============================================================================

pub struct SettingsShareRepository;

impl SettingsShareRepository {
    /// Create a new settings share (owner grants access to grantee).
    pub async fn create(
        pool: &SqlitePool,
        owner_user_id: &str,
        grantee_user_id: &str,
        can_manage: bool,
    ) -> AppResult<SettingsShare> {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().naive_utc();

        sqlx::query_as!(
            SettingsShare,
            r#"
            INSERT INTO settings_shares (
                id, owner_user_id, grantee_user_id, can_manage, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?)
            RETURNING
                id as "id!: String",
                owner_user_id as "owner_user_id!: String",
                grantee_user_id as "grantee_user_id!: String",
                can_manage as "can_manage!: bool",
                created_at as "created_at!: chrono::NaiveDateTime",
                updated_at as "updated_at!: chrono::NaiveDateTime"
            "#,
            id,
            owner_user_id,
            grantee_user_id,
            can_manage,
            now,
            now
        )
        .fetch_one(pool)
        .await
        .map_err(AppError::Database)
    }

    /// Find a single share by owner and grantee.
    pub async fn find_by_owner_and_grantee(
        pool: &SqlitePool,
        owner_user_id: &str,
        grantee_user_id: &str,
    ) -> AppResult<Option<SettingsShare>> {
        sqlx::query_as!(
            SettingsShare,
            r#"
            SELECT
                id as "id!: String",
                owner_user_id as "owner_user_id!: String",
                grantee_user_id as "grantee_user_id!: String",
                can_manage as "can_manage!: bool",
                created_at as "created_at!: chrono::NaiveDateTime",
                updated_at as "updated_at!: chrono::NaiveDateTime"
            FROM settings_shares
            WHERE owner_user_id = ? AND grantee_user_id = ?
            LIMIT 1
            "#,
            owner_user_id,
            grantee_user_id
        )
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)
    }

    /// Update the `can_manage` flag for an existing share.
    pub async fn update_can_manage(
        pool: &SqlitePool,
        owner_user_id: &str,
        grantee_user_id: &str,
        can_manage: bool,
    ) -> AppResult<SettingsShare> {
        let now = chrono::Utc::now().naive_utc();

        sqlx::query_as!(
            SettingsShare,
            r#"
            UPDATE settings_shares
            SET can_manage = ?, updated_at = ?
            WHERE owner_user_id = ? AND grantee_user_id = ?
            RETURNING
                id as "id!: String",
                owner_user_id as "owner_user_id!: String",
                grantee_user_id as "grantee_user_id!: String",
                can_manage as "can_manage!: bool",
                created_at as "created_at!: chrono::NaiveDateTime",
                updated_at as "updated_at!: chrono::NaiveDateTime"
            "#,
            can_manage,
            now,
            owner_user_id,
            grantee_user_id
        )
        .fetch_one(pool)
        .await
        .map_err(AppError::Database)
    }

    /// Delete a share (revoke access).
    pub async fn delete(
        pool: &SqlitePool,
        owner_user_id: &str,
        grantee_user_id: &str,
    ) -> AppResult<()> {
        sqlx::query!(
            "DELETE FROM settings_shares WHERE owner_user_id = ? AND grantee_user_id = ?",
            owner_user_id,
            grantee_user_id
        )
        .execute(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(())
    }

    /// List shares for an owner along with grantee user info.
    /// Returns tuples (SettingsShare, grantee_twitch_login, grantee_display_name).
    pub async fn list_with_grantee_info(
        pool: &SqlitePool,
        owner_user_id: &str,
    ) -> AppResult<Vec<(SettingsShare, String, String)>> {
        // We intentionally select explicit columns so they map cleanly into a `SettingsShare`
        // and the additional grantee fields.
        let rows = sqlx::query(
            r#"
            SELECT
                s.id as share_id,
                s.owner_user_id as owner_user_id,
                s.grantee_user_id as grantee_user_id,
                s.can_manage as can_manage,
                s.created_at as share_created_at,
                s.updated_at as share_updated_at,
                u.twitch_login as grantee_login,
                u.twitch_display_name as grantee_display_name
            FROM settings_shares s
            JOIN users u ON u.id = s.grantee_user_id
            WHERE s.owner_user_id = ?
            ORDER BY s.created_at DESC
            "#,
        )
        .bind(owner_user_id)
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;

        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            // Map query row into SettingsShare by reading columns from the row
            let id: String = r.try_get("share_id").map_err(AppError::Database)?;
            let owner_user_id_col: String =
                r.try_get("owner_user_id").map_err(AppError::Database)?;
            let grantee_user_id: String =
                r.try_get("grantee_user_id").map_err(AppError::Database)?;
            // SQLite represents booleans as integers (0/1)
            let can_manage_i: i64 = r.try_get("can_manage").map_err(AppError::Database)?;
            let created_at: NaiveDateTime =
                r.try_get("share_created_at").map_err(AppError::Database)?;
            let updated_at: NaiveDateTime =
                r.try_get("share_updated_at").map_err(AppError::Database)?;
            let login: String = r.try_get("grantee_login").map_err(AppError::Database)?;
            let display: String = r
                .try_get("grantee_display_name")
                .map_err(AppError::Database)?;

            let share = SettingsShare {
                id,
                owner_user_id: owner_user_id_col,
                grantee_user_id,
                can_manage: can_manage_i != 0,
                created_at,
                updated_at,
            };

            out.push((share, login, display));
        }

        Ok(out)
    }

    pub async fn list_with_owner_info(
        pool: &SqlitePool,
        grantee_user_id: &str,
    ) -> AppResult<Vec<(SettingsShare, String, String)>> {
        // Use a dynamic query and manual row extraction similar to list_with_grantee_info
        let rows = sqlx::query(
            r#"
            SELECT
                s.id as share_id,
                s.owner_user_id as owner_user_id,
                s.grantee_user_id as grantee_user_id,
                s.can_manage as can_manage,
                s.created_at as share_created_at,
                s.updated_at as share_updated_at,
                u.twitch_login as owner_login,
                u.twitch_display_name as owner_display_name
            FROM settings_shares s
            JOIN users u ON u.id = s.owner_user_id
            WHERE s.grantee_user_id = ?
            ORDER BY s.created_at DESC
            "#,
        )
        .bind(grantee_user_id)
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;

        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let id: String = r.try_get("share_id").map_err(AppError::Database)?;
            let owner_user_id_col: String =
                r.try_get("owner_user_id").map_err(AppError::Database)?;
            let grantee_user_id_col: String =
                r.try_get("grantee_user_id").map_err(AppError::Database)?;
            let can_manage_i: i64 = r.try_get("can_manage").map_err(AppError::Database)?;
            let created_at: NaiveDateTime =
                r.try_get("share_created_at").map_err(AppError::Database)?;
            let updated_at: NaiveDateTime =
                r.try_get("share_updated_at").map_err(AppError::Database)?;
            let login: String = r.try_get("owner_login").map_err(AppError::Database)?;
            let display: String = r
                .try_get("owner_display_name")
                .map_err(AppError::Database)?;

            let share = SettingsShare {
                id,
                owner_user_id: owner_user_id_col,
                grantee_user_id: grantee_user_id_col,
                can_manage: can_manage_i != 0,
                created_at,
                updated_at,
            };

            out.push((share, login, display));
        }

        Ok(out)
    }
}
