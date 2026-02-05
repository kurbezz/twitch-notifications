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

        sqlx::query_as!(
            EventSubSubscription,
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
            RETURNING
                id as "id!: String",
                twitch_subscription_id as "twitch_subscription_id!: String",
                user_id as "user_id!: String",
                subscription_type as "subscription_type!: String",
                status as "status!: String",
                created_at as "created_at!: chrono::NaiveDateTime",
                updated_at as "updated_at!: chrono::NaiveDateTime"
            "#,
            id,
            twitch_subscription_id,
            user_id,
            subscription_type,
            status,
            now,
            now
        )
        .fetch_one(pool)
        .await
        .map_err(AppError::Database)
    }

    /// List all subscriptions for a user
    pub async fn find_by_user_id(
        pool: &SqlitePool,
        user_id: &str,
    ) -> AppResult<Vec<EventSubSubscription>> {
        sqlx::query_as!(
            EventSubSubscription,
            r#"
            SELECT
                id as "id!: String",
                twitch_subscription_id as "twitch_subscription_id!: String",
                user_id as "user_id!: String",
                subscription_type as "subscription_type!: String",
                status as "status!: String",
                created_at as "created_at!: chrono::NaiveDateTime",
                updated_at as "updated_at!: chrono::NaiveDateTime"
            FROM eventsub_subscriptions
            WHERE user_id = ?
            "#,
            user_id
        )
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)
    }

    /// Find a subscription by Twitch subscription id
    pub async fn find_by_twitch_subscription_id(
        pool: &SqlitePool,
        twitch_subscription_id: &str,
    ) -> AppResult<Option<EventSubSubscription>> {
        sqlx::query_as!(
            EventSubSubscription,
            r#"
            SELECT
                id as "id!: String",
                twitch_subscription_id as "twitch_subscription_id!: String",
                user_id as "user_id!: String",
                subscription_type as "subscription_type!: String",
                status as "status!: String",
                created_at as "created_at!: chrono::NaiveDateTime",
                updated_at as "updated_at!: chrono::NaiveDateTime"
            FROM eventsub_subscriptions
            WHERE twitch_subscription_id = ?
            "#,
            twitch_subscription_id
        )
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)
    }

    /// Update subscription status
    pub async fn update_status(
        pool: &SqlitePool,
        twitch_subscription_id: &str,
        new_status: &str,
    ) -> AppResult<EventSubSubscription> {
        let now = Utc::now().naive_utc();

        sqlx::query_as!(
            EventSubSubscription,
            r#"
            UPDATE eventsub_subscriptions
            SET status = ?, updated_at = ?
            WHERE twitch_subscription_id = ?
            RETURNING
                id as "id!: String",
                twitch_subscription_id as "twitch_subscription_id!: String",
                user_id as "user_id!: String",
                subscription_type as "subscription_type!: String",
                status as "status!: String",
                created_at as "created_at!: chrono::NaiveDateTime",
                updated_at as "updated_at!: chrono::NaiveDateTime"
            "#,
            new_status,
            now,
            twitch_subscription_id
        )
        .fetch_one(pool)
        .await
        .map_err(AppError::Database)
    }

    /// Delete a subscription
    pub async fn delete(pool: &SqlitePool, id: &str) -> AppResult<()> {
        sqlx::query!("DELETE FROM eventsub_subscriptions WHERE id = ?", id)
            .execute(pool)
            .await
            .map_err(AppError::Database)?;

        Ok(())
    }
}
