use crate::db::models::{CreateEventSubSubscription, EventSubSubscription};
use crate::error::{AppError, AppResult};
use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct EventSubSubscriptionRepository;

impl EventSubSubscriptionRepository {
    /// Create a new EventSub subscription
    pub async fn create(
        pool: &SqlitePool,
        user_id: &str,
        subscription: CreateEventSubSubscription,
    ) -> AppResult<EventSubSubscription> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().naive_utc();

        // Take ownership of fields we need to reuse after moving `subscription`
        let twitch_subscription_id = subscription.twitch_subscription_id;
        let subscription_type = subscription.subscription_type;
        let status = subscription.status;

        sqlx::query(
            r#"
            INSERT INTO eventsub_subscriptions (
                id,
                twitch_subscription_id,
                user_id,
                subscription_type,
                status,
                created_at,
                updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(id.clone())
        .bind(&twitch_subscription_id)
        .bind(user_id)
        .bind(subscription_type)
        .bind(status)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;

        Self::find_by_twitch_subscription_id(pool, &twitch_subscription_id)
            .await?
            .ok_or_else(|| AppError::NotFound("EventSub subscription not found".to_string()))
    }

    // find_by_user_id_and_type removed - unused

    /// List all subscriptions for a user
    pub async fn find_by_user_id(
        pool: &SqlitePool,
        user_id: &str,
    ) -> AppResult<Vec<EventSubSubscription>> {
        sqlx::query_as::<_, EventSubSubscription>(
            r#"
            SELECT
                id,
                twitch_subscription_id,
                user_id,
                subscription_type,
                status,
                created_at,
                updated_at
            FROM eventsub_subscriptions
            WHERE user_id = ?
            "#,
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)
    }

    /// Find a subscription by Twitch subscription id
    pub async fn find_by_twitch_subscription_id(
        pool: &SqlitePool,
        twitch_subscription_id: &str,
    ) -> AppResult<Option<EventSubSubscription>> {
        sqlx::query_as::<_, EventSubSubscription>(
            r#"
            SELECT
                id,
                twitch_subscription_id,
                user_id,
                subscription_type,
                status,
                created_at,
                updated_at
            FROM eventsub_subscriptions
            WHERE twitch_subscription_id = ?
            "#,
        )
        .bind(twitch_subscription_id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)
    }

    // update_status removed - unused

    /// Delete a subscription
    pub async fn delete(pool: &SqlitePool, id: &str) -> AppResult<()> {
        sqlx::query!("DELETE FROM eventsub_subscriptions WHERE id = ?", id)
            .execute(pool)
            .await
            .map_err(AppError::Database)?;

        Ok(())
    }

    // delete_by_twitch_subscription_id removed - unused
}
