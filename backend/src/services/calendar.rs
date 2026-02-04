use std::collections::HashSet;
use std::sync::Arc;

use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use tracing::{info, warn};

use crate::db::models::{CreateSyncedCalendarEvent, DiscordIntegration};
use crate::db::{DiscordIntegrationRepository, SyncedCalendarRepository, UserRepository};
use crate::error::AppResult;
use crate::services::discord::ScheduledEvent;
use crate::services::twitch::ScheduleSegment;
use crate::AppState;

/// Calendar sync manager:
///
/// - Finds all Discord integrations with calendar sync enabled.
/// - For each integration, fetches the Twitch schedule for the owner (using the
///   user's stored access token; attempts a refresh if necessary).
/// - Upserts synced calendar rows in the DB for schedule segments, and creates/updates
///   matching Discord scheduled events (and stores the `discord_event_id`).
/// - Removes synced events (and corresponding Discord events) that no longer exist in the
///   Twitch schedule.
pub struct CalendarSyncManager;

impl CalendarSyncManager {
    /// Synchronize calendar events for all integrations that have calendar sync enabled.
    pub async fn sync_all(state: &Arc<AppState>) -> AppResult<()> {
        info!("Starting periodic calendar synchronization for integrations");

        let integrations =
            match DiscordIntegrationRepository::find_with_calendar_sync(&state.db).await {
                Ok(v) => v,
                Err(e) => {
                    warn!(
                        "Failed to list integrations with calendar sync enabled: {:?}",
                        e
                    );
                    return Ok(());
                }
            };

        for integration in integrations {
            match Self::sync_for_integration(state, &integration).await {
                Ok(_) => info!("Synced calendar for integration {}", integration.id),
                Err(e) => warn!(
                    "Failed to sync calendar for integration {}: {:?}",
                    integration.id, e
                ),
            }
        }

        Ok(())
    }

    // Helper to delete a Discord scheduled event and log failures
    async fn delete_discord_event(state: &Arc<AppState>, guild_id: &str, event_id: &str) {
        let discord_guard = state.discord.read().await;
        if let Some(discord) = discord_guard.as_ref() {
            if let Err(e) = discord.delete_scheduled_event(guild_id, event_id).await {
                warn!(
                    "Failed to delete Discord scheduled event {} in guild {}: {:?}",
                    event_id, guild_id, e
                );
            }
        } else {
            warn!(
                "Discord service not available; skipping deletion of event {}",
                event_id
            );
        }
    }

    // Helper to delete a synced calendar DB row and log failures
    async fn delete_db_row(state: &Arc<AppState>, id: &str) {
        if let Err(e) = SyncedCalendarRepository::delete(&state.db, id).await {
            warn!("Failed to delete synced calendar event {}: {:?}", id, e);
        }
    }

    // Process a Twitch schedule segment: upsert DB row and ensure Discord scheduled event is created/updated
    async fn process_segment(
        state: &Arc<AppState>,
        integration: &DiscordIntegration,
        segment: &ScheduleSegment,
        twitch_login: &str,
    ) {
        // Parse start/end times
        let start_time: NaiveDateTime = match parse_rfc3339_to_naive(&segment.start_time) {
            Some(dt) => dt,
            None => {
                warn!(
                    "Failed to parse segment start_time '{}'; skipping segment {}",
                    segment.start_time, segment.id
                );
                return;
            }
        };

        let end_time = segment
            .end_time
            .as_ref()
            .and_then(|s| parse_rfc3339_to_naive(s));

        let create = CreateSyncedCalendarEvent {
            twitch_segment_id: segment.id.clone(),
            discord_integration_id: Some(integration.id.clone()),
            title: segment.title.clone(),
            start_time,
            end_time,
            category_name: segment.category.as_ref().map(|c| c.name.clone()),
            is_recurring: segment.is_recurring,
        };

        // Upsert DB row
        let record = match SyncedCalendarRepository::upsert_by_twitch_segment_and_integration(
            &state.db,
            &integration.user_id,
            create,
        )
        .await
        {
            Ok(r) => r,
            Err(e) => {
                warn!(
                    "Failed to upsert synced calendar event for segment {}: {:?}",
                    segment.id, e
                );
                return;
            }
        };

        // Check if event is in the past - Discord doesn't allow scheduling events in the past
        let now = Utc::now().naive_utc();
        if start_time < now {
            warn!(
                "Segment {} starts in the past ({}), skipping Discord event creation",
                segment.id, start_time
            );
            return;
        }

        // Prepare Discord ScheduledEvent payload
        // For EXTERNAL events (entity_type: 3), Discord requires:
        // - entity_metadata with a location
        // - channel_id must be None (EXTERNAL events cannot have a channel)
        let location = format!("https://twitch.tv/{}", twitch_login);
        let scheduled_event = ScheduledEvent {
            id: None,
            guild_id: integration.discord_guild_id.clone(),
            channel_id: None, // EXTERNAL events (entity_type: 3) cannot have channel_id
            name: record.title.clone(),
            description: record.category_name.clone(),
            scheduled_start_time: segment.start_time.clone(),
            scheduled_end_time: segment.end_time.clone(),
            privacy_level: 2, // GUILD_ONLY
            entity_type: 3,   // EXTERNAL
            entity_metadata: Some(crate::services::discord::EntityMetadata {
                location: Some(location),
            }),
        };

        // Ensure we have a Discord service available
        let discord_guard = state.discord.read().await;
        let discord = discord_guard.as_ref();
        if discord.is_none() {
            warn!(
                "Discord service not available; skipping Discord event sync for segment {}",
                segment.id
            );
            // We still keep the DB row; it will be synced later when the service is available.
            return;
        }
        let discord = discord.unwrap();

        // If an event already exists on Discord, update it; otherwise create a new one
        if let Some(existing_discord_id) = record.discord_event_id.as_ref() {
            match discord
                .update_scheduled_event(
                    &integration.discord_guild_id,
                    existing_discord_id,
                    scheduled_event,
                )
                .await
            {
                Ok(_) => {
                    // Touch last_synced_at by updating discord_event_id with same id
                    if let Err(e) = SyncedCalendarRepository::update_discord_event_id(
                        &state.db,
                        &record.id,
                        Some(existing_discord_id),
                    )
                    .await
                    {
                        warn!(
                            "Failed to update last_synced_at for event {}: {:?}",
                            record.id, e
                        );
                    }
                }
                Err(e) => {
                    warn!(
                        "Failed to update Discord scheduled event {}: {:?}",
                        existing_discord_id, e
                    );
                }
            }
        } else {
            match discord.create_scheduled_event(scheduled_event).await {
                Ok(created) => {
                    if let Some(created_id) = created.id {
                        if let Err(e) = SyncedCalendarRepository::update_discord_event_id(
                            &state.db,
                            &record.id,
                            Some(&created_id),
                        )
                        .await
                        {
                            warn!(
                                "Failed to store Discord event id for {}: {:?}",
                                record.id, e
                            );
                        }
                    } else {
                        warn!("Discord returned a created scheduled event without an id for segment {}", segment.id);
                    }
                }
                Err(e) => {
                    warn!(
                        "Failed to create Discord scheduled event for segment {}: {:?}",
                        segment.id, e
                    );
                }
            }
        }
    }

    /// Synchronize calendar events for a single Discord integration.
    pub async fn sync_for_integration(
        state: &Arc<AppState>,
        integration: &DiscordIntegration,
    ) -> AppResult<()> {
        info!(
            "Syncing calendar for integration {} (guild {})",
            integration.id, integration.discord_guild_id
        );

        // Load user
        let user = match UserRepository::find_by_id(&state.db, &integration.user_id).await? {
            Some(u) => u,
            None => {
                warn!(
                    "Owner user {} for integration {} not found; skipping",
                    integration.user_id, integration.id
                );
                return Ok(());
            }
        };

        // Try to fetch schedule using the current access token; if it fails (e.g. token expired)
        // attempt to refresh the token and retry once.
        let schedule_opt = match state
            .twitch
            .get_schedule(&user.twitch_access_token, &user.twitch_id)
            .await
        {
            Ok(s) => s,
            Err(err) => {
                warn!(
                    "Failed to fetch schedule for user {}: {}. Attempting token refresh.",
                    user.id, err
                );

                match state.twitch.refresh_token(&user.twitch_refresh_token).await {
                    Ok(token_resp) => {
                        // Update stored tokens for the user
                        let expires_at = Utc::now() + Duration::seconds(token_resp.expires_in);
                        if let Err(e) = UserRepository::update_tokens(
                            &state.db,
                            &user.id,
                            &token_resp.access_token,
                            &token_resp.refresh_token,
                            expires_at.naive_utc(),
                        )
                        .await
                        {
                            warn!(
                                "Failed to persist refreshed Twitch tokens for user {}: {:?}",
                                user.id, e
                            );
                            return Ok(());
                        }

                        // Retry schedule fetch with refreshed access token
                        match state
                            .twitch
                            .get_schedule(&token_resp.access_token, &user.twitch_id)
                            .await
                        {
                            Ok(s2) => s2,
                            Err(e2) => {
                                warn!(
                                    "Failed to fetch schedule for user {} after token refresh: {:?}",
                                    user.id, e2
                                );
                                return Ok(());
                            }
                        }
                    }
                    Err(e) => {
                        warn!(
                            "Failed to refresh Twitch token for user {}: {:?}",
                            user.id, e
                        );
                        return Ok(());
                    }
                }
            }
        };

        // If there's no schedule (Twitch returned 404 / no schedule), clean up any existing synced events.
        if schedule_opt.is_none() {
            info!(
                "No schedule for user {}; cleaning up existing synced events for integration {}",
                user.id, integration.id
            );

            let existing =
                SyncedCalendarRepository::find_by_integration(&state.db, &integration.id).await?;
            for ev in existing {
                if let Some(discord_event_id) = ev.discord_event_id.as_ref() {
                    Self::delete_discord_event(
                        state,
                        &integration.discord_guild_id,
                        discord_event_id,
                    )
                    .await;
                }

                Self::delete_db_row(state, &ev.id).await;
            }

            return Ok(());
        }

        let schedule = schedule_opt.unwrap();

        // Build a set of segment ids that are currently present so we can remove stale DB rows later.
        let mut segment_ids: HashSet<String> = HashSet::new();

        for segment in schedule.segments.into_iter() {
            segment_ids.insert(segment.id.clone());

            // If the segment has been canceled (Twitch provides `canceled_until`), treat it as removed.
            if segment.canceled_until.is_some() {
                if let Ok(Some(existing)) =
                    SyncedCalendarRepository::find_by_twitch_segment_and_integration(
                        &state.db,
                        &segment.id,
                        &integration.id,
                    )
                    .await
                {
                    if let Some(discord_event_id) = existing.discord_event_id.as_ref() {
                        Self::delete_discord_event(
                            state,
                            &integration.discord_guild_id,
                            discord_event_id,
                        )
                        .await;
                    }

                    Self::delete_db_row(state, &existing.id).await;
                }
                // Skip further processing for canceled segment
                continue;
            }

            // Process segment: upsert DB row and ensure Discord scheduled event is created/updated
            Self::process_segment(state, integration, &segment, &user.twitch_login).await;

            // Add a small delay between requests to avoid hitting Discord rate limits
            // Discord allows 5 requests per 5 seconds for scheduled events endpoints
            tokio::time::sleep(tokio::time::Duration::from_millis(1200)).await;
        } // end for segments

        // Cleanup: remove DB rows (and their Discord events) for segments that no longer exist
        let existing_rows =
            SyncedCalendarRepository::find_by_integration(&state.db, &integration.id).await?;
        for row in existing_rows.into_iter() {
            if !segment_ids.contains(&row.twitch_segment_id) {
                if let Some(discord_event_id) = row.discord_event_id.as_ref() {
                    Self::delete_discord_event(
                        state,
                        &integration.discord_guild_id,
                        discord_event_id,
                    )
                    .await;
                }

                Self::delete_db_row(state, &row.id).await;
            }
        }

        Ok(())
    }
}

/// Parse an RFC3339 datetime string into a UTC NaiveDateTime.
///
/// Returns None if parsing fails.
fn parse_rfc3339_to_naive(s: &str) -> Option<NaiveDateTime> {
    match DateTime::parse_from_rfc3339(s) {
        Ok(dt) => Some(dt.with_timezone(&Utc).naive_utc()),
        Err(_) => None,
    }
}
