use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::db::models::{CreateSyncedCalendarEvent, SyncedCalendarEvent};
use crate::error::{AppError, AppResult};

/// Repository for managing synced calendar events (`synced_calendar_events` table).
pub struct SyncedCalendarRepository;

impl SyncedCalendarRepository {
    /// Create or update a synced calendar event identified by (twitch_segment_id, discord_integration_id).
    ///
    /// This will insert a new row if one doesn't exist or update fields on conflict.
    pub async fn upsert_by_twitch_segment_and_integration(
        pool: &SqlitePool,
        user_id: &str,
        create: CreateSyncedCalendarEvent,
    ) -> AppResult<SyncedCalendarEvent> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().naive_utc();

        // Use INSERT ... ON CONFLICT(...) DO UPDATE to perform upsert and return the row.
        let record = sqlx::query_as!(
            SyncedCalendarEvent,
            r#"
            INSERT INTO synced_calendar_events (
                id,
                user_id,
                twitch_segment_id,
                discord_integration_id,
                discord_event_id,
                title,
                start_time,
                end_time,
                category_name,
                is_recurring,
                last_synced_at,
                created_at,
                updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(twitch_segment_id, discord_integration_id) DO UPDATE SET
                title = excluded.title,
                start_time = excluded.start_time,
                end_time = excluded.end_time,
                category_name = excluded.category_name,
                is_recurring = excluded.is_recurring,
                last_synced_at = excluded.last_synced_at,
                updated_at = excluded.updated_at
            RETURNING
                id as "id!: String",
                user_id as "user_id!: String",
                twitch_segment_id as "twitch_segment_id!: String",
                discord_integration_id as "discord_integration_id?: String",
                discord_event_id as "discord_event_id?: String",
                title as "title!: String",
                start_time as "start_time!: chrono::NaiveDateTime",
                end_time as "end_time?: chrono::NaiveDateTime",
                category_name as "category_name?: String",
                is_recurring as "is_recurring!: bool",
                last_synced_at as "last_synced_at!: chrono::NaiveDateTime",
                created_at as "created_at!: chrono::NaiveDateTime",
                updated_at as "updated_at!: chrono::NaiveDateTime"
            "#,
            id,
            user_id,
            create.twitch_segment_id,
            create.discord_integration_id,
            None::<String>, // discord_event_id initially unknown when creating from Twitch schedule
            create.title,
            create.start_time,
            create.end_time,
            create.category_name,
            create.is_recurring,
            now,
            now,
            now
        )
        .fetch_one(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(record)
    }

    /// Find all synced calendar events for a given Discord integration.
    pub async fn find_by_integration(
        pool: &SqlitePool,
        integration_id: &str,
    ) -> AppResult<Vec<SyncedCalendarEvent>> {
        let rows = sqlx::query_as!(
            SyncedCalendarEvent,
            r#"
            SELECT
                id as "id!: String",
                user_id as "user_id!: String",
                twitch_segment_id as "twitch_segment_id!: String",
                discord_integration_id as "discord_integration_id?: String",
                discord_event_id as "discord_event_id?: String",
                title as "title!: String",
                start_time as "start_time!: chrono::NaiveDateTime",
                end_time as "end_time?: chrono::NaiveDateTime",
                category_name as "category_name?: String",
                is_recurring as "is_recurring!: bool",
                last_synced_at as "last_synced_at!: chrono::NaiveDateTime",
                created_at as "created_at!: chrono::NaiveDateTime",
                updated_at as "updated_at!: chrono::NaiveDateTime"
            FROM synced_calendar_events
            WHERE discord_integration_id = ?
            "#,
            integration_id
        )
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(rows)
    }

    /// Find a single synced calendar event by twitch segment id and discord integration id.
    pub async fn find_by_twitch_segment_and_integration(
        pool: &SqlitePool,
        twitch_segment_id: &str,
        integration_id: &str,
    ) -> AppResult<Option<SyncedCalendarEvent>> {
        let row = sqlx::query_as!(
            SyncedCalendarEvent,
            r#"
            SELECT
                id as "id!: String",
                user_id as "user_id!: String",
                twitch_segment_id as "twitch_segment_id!: String",
                discord_integration_id as "discord_integration_id?: String",
                discord_event_id as "discord_event_id?: String",
                title as "title!: String",
                start_time as "start_time!: chrono::NaiveDateTime",
                end_time as "end_time?: chrono::NaiveDateTime",
                category_name as "category_name?: String",
                is_recurring as "is_recurring!: bool",
                last_synced_at as "last_synced_at!: chrono::NaiveDateTime",
                created_at as "created_at!: chrono::NaiveDateTime",
                updated_at as "updated_at!: chrono::NaiveDateTime"
            FROM synced_calendar_events
            WHERE twitch_segment_id = ? AND discord_integration_id = ?
            "#,
            twitch_segment_id,
            integration_id
        )
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(row)
    }

    // `find_by_user_id` removed - unused.

    /// Update the stored `discord_event_id` for a synced event and mark it as synced now.
    pub async fn update_discord_event_id(
        pool: &SqlitePool,
        id: &str,
        discord_event_id: Option<&str>,
    ) -> AppResult<()> {
        let now = Utc::now().naive_utc();
        let discord_event_id_owned = discord_event_id.map(|s| s.to_string());

        sqlx::query!(
            r#"
            UPDATE synced_calendar_events
            SET discord_event_id = ?,
                last_synced_at = ?,
                updated_at = ?
            WHERE id = ?
            "#,
            discord_event_id_owned,
            now,
            now,
            id
        )
        .execute(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(())
    }

    /// Delete a synced calendar event by its primary id.
    pub async fn delete(pool: &SqlitePool, id: &str) -> AppResult<()> {
        sqlx::query!("DELETE FROM synced_calendar_events WHERE id = ?", id)
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
        Ok(())
    }
}
