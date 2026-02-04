use sqlx::SqlitePool;
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
        let rows = sqlx::query!(
            r#"
            SELECT
                s.id as "share_id!: String",
                s.owner_user_id as "owner_user_id!: String",
                s.grantee_user_id as "grantee_user_id!: String",
                s.can_manage as "can_manage!: i64",
                s.created_at as "share_created_at!: chrono::NaiveDateTime",
                s.updated_at as "share_updated_at!: chrono::NaiveDateTime",
                u.twitch_login as "grantee_login!: String",
                u.twitch_display_name as "grantee_display_name!: String"
            FROM settings_shares s
            JOIN users u ON u.id = s.grantee_user_id
            WHERE s.owner_user_id = ?
            ORDER BY s.created_at DESC
            "#,
            owner_user_id
        )
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;

        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let share = SettingsShare {
                id: r.share_id,
                owner_user_id: r.owner_user_id,
                grantee_user_id: r.grantee_user_id,
                can_manage: r.can_manage != 0,
                created_at: r.share_created_at,
                updated_at: r.share_updated_at,
            };

            out.push((share, r.grantee_login, r.grantee_display_name));
        }

        Ok(out)
    }

    pub async fn list_with_owner_info(
        pool: &SqlitePool,
        grantee_user_id: &str,
    ) -> AppResult<Vec<(SettingsShare, String, String)>> {
        let rows = sqlx::query!(
            r#"
            SELECT
                s.id as "share_id!: String",
                s.owner_user_id as "owner_user_id!: String",
                s.grantee_user_id as "grantee_user_id!: String",
                s.can_manage as "can_manage!: i64",
                s.created_at as "share_created_at!: chrono::NaiveDateTime",
                s.updated_at as "share_updated_at!: chrono::NaiveDateTime",
                u.twitch_login as "owner_login!: String",
                u.twitch_display_name as "owner_display_name!: String"
            FROM settings_shares s
            JOIN users u ON u.id = s.owner_user_id
            WHERE s.grantee_user_id = ?
            ORDER BY s.created_at DESC
            "#,
            grantee_user_id
        )
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;

        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let share = SettingsShare {
                id: r.share_id,
                owner_user_id: r.owner_user_id,
                grantee_user_id: r.grantee_user_id,
                can_manage: r.can_manage != 0,
                created_at: r.share_created_at,
                updated_at: r.share_updated_at,
            };

            out.push((share, r.owner_login, r.owner_display_name));
        }

        Ok(out)
    }
}
