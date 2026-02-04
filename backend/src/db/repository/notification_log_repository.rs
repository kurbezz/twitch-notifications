use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::db::models::*;
use crate::error::{AppError, AppResult};

// ============================================================================
// Notification Log Repository
// ============================================================================

pub struct NotificationLogRepository;

impl NotificationLogRepository {
    pub async fn create(
        pool: &SqlitePool,
        log: CreateNotificationLog,
    ) -> AppResult<NotificationLog> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().naive_utc();

        sqlx::query_as!(
            NotificationLog,
            r#"
            INSERT INTO notification_history (
                id, user_id, notification_type, destination_type,
                destination_id, content, status, error_message, created_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING
                id as "id!: String",
                user_id as "user_id!: String",
                notification_type as "notification_type!: String",
                destination_type as "destination_type!: String",
                destination_id as "destination_id!: String",
                content as "content!: String",
                status as "status!: String",
                error_message as "error_message?: String",
                created_at as "created_at!: chrono::NaiveDateTime"
            "#,
            id,
            log.user_id,
            log.notification_type,
            log.destination_type,
            log.destination_id,
            log.content,
            log.status,
            log.error_message,
            now
        )
        .fetch_one(pool)
        .await
        .map_err(AppError::Database)
    }

    // `find_by_id` removed - unused.

    /// Find notifications for a user with optional filters and pagination.
    pub async fn find_by_user_id_with_filters(
        pool: &SqlitePool,
        user_id: &str,
        limit: Option<i64>,
        offset: Option<i64>,
        notification_type: Option<&str>,
        destination_type: Option<&str>,
        status: Option<&str>,
    ) -> AppResult<Vec<NotificationLog>> {
        let limit_val = limit.unwrap_or(100);
        let offset_val = offset.unwrap_or(0);

        sqlx::query_as!(
            NotificationLog,
            r#"
            SELECT
                id as "id!: String",
                user_id as "user_id!: String",
                notification_type as "notification_type!: String",
                destination_type as "destination_type!: String",
                destination_id as "destination_id!: String",
                content as "content!: String",
                status as "status!: String",
                error_message as "error_message?: String",
                created_at as "created_at!: chrono::NaiveDateTime"
            FROM notification_history
            WHERE user_id = ?
            AND (? IS NULL OR notification_type = ?)
            AND (? IS NULL OR destination_type = ?)
            AND (? IS NULL OR status = ?)
            ORDER BY created_at DESC
            LIMIT ?
            OFFSET ?
            "#,
            user_id,
            notification_type,
            notification_type,
            destination_type,
            destination_type,
            status,
            status,
            limit_val,
            offset_val,
        )
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)
    }

    /// Count notifications for a user with optional filters.
    pub async fn count_by_user_id_with_filters(
        pool: &SqlitePool,
        user_id: &str,
        notification_type: Option<&str>,
        destination_type: Option<&str>,
        status: Option<&str>,
    ) -> AppResult<i64> {
        let count: i32 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM notification_history WHERE user_id = ? AND (? IS NULL OR notification_type = ?) AND (? IS NULL OR destination_type = ?) AND (? IS NULL OR status = ?)",
            user_id,
            notification_type,
            notification_type,
            destination_type,
            destination_type,
            status,
            status
        )
        .fetch_one(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(count as i64)
    }

    /// Count notifications for a user with a specific status (e.g., 'sent', 'failed')
    pub async fn count_by_user_id_and_status(
        pool: &SqlitePool,
        user_id: &str,
        status: &str,
    ) -> AppResult<i64> {
        let count: i32 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM notification_history WHERE user_id = ? AND status = ?",
            user_id,
            status
        )
        .fetch_one(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(count as i64)
    }

    /// Counts grouped by notification_type
    pub async fn counts_by_notification_type(
        pool: &SqlitePool,
        user_id: &str,
    ) -> AppResult<std::collections::HashMap<String, i64>> {
        let rows = sqlx::query!(
            r#"
            SELECT notification_type, COUNT(*) as "count!: i64"
            FROM notification_history
            WHERE user_id = ?
            GROUP BY notification_type
            "#,
            user_id
        )
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;

        let mut map = std::collections::HashMap::new();
        for row in rows {
            map.insert(row.notification_type, row.count);
        }

        Ok(map)
    }

    /// Counts grouped by destination_type
    pub async fn counts_by_destination_type(
        pool: &SqlitePool,
        user_id: &str,
    ) -> AppResult<std::collections::HashMap<String, i64>> {
        let rows = sqlx::query!(
            r#"
            SELECT destination_type, COUNT(*) as "count!: i64"
            FROM notification_history
            WHERE user_id = ?
            GROUP BY destination_type
            "#,
            user_id
        )
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;

        let mut map = std::collections::HashMap::new();
        for row in rows {
            map.insert(row.destination_type, row.count);
        }

        Ok(map)
    }

    /// Update the status and optional error message for an existing notification log entry.
    /// Returns the updated `NotificationLog`.
    pub async fn update_status(
        pool: &SqlitePool,
        id: &str,
        status: &str,
        error_message: Option<&str>,
    ) -> AppResult<NotificationLog> {
        // Convert to an owned Option<String> to satisfy the query macro type expectations.
        let last_error = error_message.map(|s| s.to_string());

        let updated = sqlx::query_as!(
            NotificationLog,
            r#"
            UPDATE notification_history
            SET status = ?, error_message = ?
            WHERE id = ?
            RETURNING
                id as "id!: String",
                user_id as "user_id!: String",
                notification_type as "notification_type!: String",
                destination_type as "destination_type!: String",
                destination_id as "destination_id!: String",
                content as "content!: String",
                status as "status!: String",
                error_message as "error_message?: String",
                created_at as "created_at!: chrono::NaiveDateTime"
            "#,
            status,
            last_error,
            id
        )
        .fetch_one(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(updated)
    }
}
