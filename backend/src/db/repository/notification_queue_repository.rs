use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::db::models::{CreateNotificationTask, NotificationTask};
use crate::error::{AppError, AppResult};

/// Repository for the persistent notification retry queue.
///
/// Implementation notes:
/// - Claiming uses an atomic single-statement UPDATE with a subselect:
///   `UPDATE ... WHERE id = (SELECT id FROM ... LIMIT 1) RETURNING ...`
///   This avoids a long-lived transaction and reduces contention on SQLite.
/// - Queries filter out expired tasks (where `expires_at` IS NOT NULL AND <= CURRENT_TIMESTAMP).
pub struct NotificationQueueRepository;

impl NotificationQueueRepository {
    /// Create a new queued notification task.
    ///
    /// `task.max_attempts` and `task.next_attempt_at` may be omitted and will be
    /// defaulted by repository logic. `expires_at` may be provided to limit how
    /// long the worker should attempt retries (useful for time-sensitive notifications).
    pub async fn create(
        pool: &SqlitePool,
        task: CreateNotificationTask,
    ) -> AppResult<NotificationTask> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().naive_utc();
        let next_attempt_at = task.next_attempt_at.unwrap_or(now);
        let max_attempts = task.max_attempts.unwrap_or(5);

        let row = sqlx::query_as::<_, NotificationTask>(
            r#"
            INSERT INTO notification_queue (
                id,
                notification_log_id,
                user_id,
                notification_type,
                content_json,
                message,
                destination_type,
                destination_id,
                webhook_url,
                attempts,
                max_attempts,
                next_attempt_at,
                expires_at,
                last_error,
                status,
                created_at,
                updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING
                id,
                notification_log_id,
                user_id,
                notification_type,
                content_json,
                message,
                destination_type,
                destination_id,
                webhook_url,
                attempts,
                max_attempts,
                next_attempt_at,
                expires_at,
                last_error,
                status,
                created_at,
                updated_at
        "#,
        )
        .bind(id)
        .bind(task.notification_log_id)
        .bind(task.user_id)
        .bind(task.notification_type)
        .bind(task.content_json)
        .bind(task.message)
        .bind(task.destination_type)
        .bind(task.destination_id)
        .bind(task.webhook_url)
        .bind(0i32) // attempts
        .bind(max_attempts)
        .bind(next_attempt_at)
        .bind(task.expires_at)
        .bind::<Option<String>>(None) // last_error
        .bind("pending")
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(row)
    }

    /// Claim up to `limit` due (non-expired) tasks and return them.
    ///
    /// This implementation atomically claims a single task per statement by
    /// using an `UPDATE ... WHERE id = (SELECT id ... LIMIT 1) RETURNING ...`
    /// pattern in a loop. It avoids holding a long transaction so other writers
    /// are not blocked.
    pub async fn fetch_and_claim_due(
        pool: &SqlitePool,
        limit: i64,
    ) -> AppResult<Vec<NotificationTask>> {
        let mut tasks: Vec<NotificationTask> = Vec::new();
        if limit <= 0 {
            return Ok(tasks);
        }

        for _ in 0..(limit as usize) {
            let now = Utc::now().naive_utc();

            let opt = sqlx::query_as::<_, NotificationTask>(
                r#"
                UPDATE notification_queue
                SET status = 'processing', updated_at = ?
                WHERE id = (
                    SELECT id FROM notification_queue
                    WHERE status = 'pending'
                      AND next_attempt_at <= CURRENT_TIMESTAMP
                      AND (expires_at IS NULL OR expires_at > CURRENT_TIMESTAMP)
                    ORDER BY next_attempt_at ASC
                    LIMIT 1
                )
                RETURNING
                    id,
                    notification_log_id,
                    user_id,
                    notification_type,
                    content_json,
                    message,
                    destination_type,
                    destination_id,
                    webhook_url,
                    attempts,
                    max_attempts,
                    next_attempt_at,
                    expires_at,
                    last_error,
                    status,
                    created_at,
                    updated_at
                "#,
            )
            .bind(now)
            .fetch_optional(pool)
            .await
            .map_err(AppError::Database)?;

            if let Some(task) = opt {
                tasks.push(task);
            } else {
                break;
            }
        }

        Ok(tasks)
    }

    /// Mark a task as succeeded. Returns the updated task row.
    pub async fn mark_succeeded(pool: &SqlitePool, id: &str) -> AppResult<NotificationTask> {
        let now = Utc::now().naive_utc();
        let row = sqlx::query_as::<_, NotificationTask>(
            r#"
            UPDATE notification_queue
            SET status = 'succeeded', updated_at = ?
            WHERE id = ?
            RETURNING
                id,
                notification_log_id,
                user_id,
                notification_type,
                content_json,
                message,
                destination_type,
                destination_id,
                webhook_url,
                attempts,
                max_attempts,
                next_attempt_at,
                expires_at,
                last_error,
                status,
                created_at,
                updated_at
            "#,
        )
        .bind(now)
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(row)
    }

    /// Increment attempts, set `next_attempt_at` and `last_error`. If the
    /// new attempt count reaches `max_attempts`, the task will be moved to 'dead'.
    ///
    /// Returns the updated task row.
    pub async fn register_attempt_and_schedule(
        pool: &SqlitePool,
        id: &str,
        next_attempt_at: chrono::NaiveDateTime,
        last_error: Option<String>,
    ) -> AppResult<NotificationTask> {
        let now = Utc::now().naive_utc();
        let row = sqlx::query_as::<_, NotificationTask>(
            r#"
                UPDATE notification_queue
                SET
                    attempts = attempts + 1,
                    next_attempt_at = ?,
                    last_error = ?,
                    status = CASE WHEN attempts + 1 >= max_attempts THEN 'dead' ELSE 'pending' END,
                    updated_at = ?
                WHERE id = ?
                RETURNING
                    id,
                    notification_log_id,
                    user_id,
                    notification_type,
                    content_json,
                    message,
                    destination_type,
                    destination_id,
                    webhook_url,
                    attempts,
                    max_attempts,
                    next_attempt_at,
                    expires_at,
                    last_error,
                    status,
                    created_at,
                    updated_at
                "#,
        )
        .bind(next_attempt_at)
        .bind(last_error)
        .bind(now)
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(row)
    }

    /// Mark the task as dead (moved to DLQ) and set the last error.
    pub async fn mark_dead(
        pool: &SqlitePool,
        id: &str,
        last_error: Option<String>,
    ) -> AppResult<NotificationTask> {
        let now = Utc::now().naive_utc();
        let row = sqlx::query_as::<_, NotificationTask>(
            r#"
                UPDATE notification_queue
                SET status = 'dead', last_error = ?, updated_at = ?
                WHERE id = ?
                RETURNING
                    id,
                    notification_log_id,
                    user_id,
                    notification_type,
                    content_json,
                    message,
                    destination_type,
                    destination_id,
                    webhook_url,
                    attempts,
                    max_attempts,
                    next_attempt_at,
                    expires_at,
                    last_error,
                    status,
                    created_at,
                    updated_at
                "#,
        )
        .bind(last_error)
        .bind(now)
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(row)
    }

    /// Fetch a task by id.
    #[allow(dead_code)]
    pub async fn find_by_id(pool: &SqlitePool, id: &str) -> AppResult<NotificationTask> {
        let row = sqlx::query_as::<_, NotificationTask>(
            r#"
                SELECT
                    id,
                    notification_log_id,
                    user_id,
                    notification_type,
                    content_json,
                    message,
                    destination_type,
                    destination_id,
                    webhook_url,
                    attempts,
                    max_attempts,
                    next_attempt_at,
                    last_error,
                    status,
                    created_at,
                    updated_at,
                    expires_at
                FROM notification_queue
                WHERE id = ?
                "#,
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(row)
    }
}
