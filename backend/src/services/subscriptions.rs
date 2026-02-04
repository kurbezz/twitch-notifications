use std::collections::HashSet;
use std::sync::Arc;

use serde_json::Value;
use tracing::{info, warn};

use crate::db::{CreateEventSubSubscription, EventSubSubscriptionRepository};
use crate::error::AppResult;
use crate::AppState;

// Alias the Twitch service EventSub type to avoid name collisions with DB models
use crate::services::twitch::EventSubSubscription as TwitchEventSub;

pub struct SubscriptionManager;

impl SubscriptionManager {
    /// Ensure EventSub subscriptions for `user` match the required event types.
    ///
    /// - Creates missing subscriptions on Twitch and records them in DB.
    /// - Deletes subscriptions that are no longer required (from Twitch + DB).
    ///
    /// Since notification flags are now per-integration, we subscribe to all event types
    /// and let the integrations filter which events they care about.
    pub async fn sync_for_user(state: &Arc<AppState>, user: &crate::db::User) -> AppResult<()> {
        info!(
            "Syncing EventSub subscriptions for user {} (twitch_id={}, twitch_login={})",
            user.id, user.twitch_id, user.twitch_login
        );

        // All event types that we support
        let required: Vec<String> = vec![
            "stream.online".to_string(),
            "stream.offline".to_string(),
            "channel.update".to_string(),
            "channel.channel_points_custom_reward_redemption.add".to_string(),
        ];

        let required_set: HashSet<String> = required.iter().cloned().collect();

        // Get current DB subscriptions for the user
        let existing_db_subs = EventSubSubscriptionRepository::find_by_user_id(&state.db, &user.id)
            .await
            .unwrap_or_else(|e| {
                warn!(
                    "Failed to load EventSub subscriptions for user {}: {}",
                    user.id, e
                );
                Vec::new()
            });

        info!(
            "Found {} existing EventSub subscription(s) in DB for user {}",
            existing_db_subs.len(),
            user.id
        );

        for sub in &existing_db_subs {
            info!(
                "Existing subscription: type={}, twitch_id={}, status={}",
                sub.subscription_type, sub.twitch_subscription_id, sub.status
            );
        }

        // 1) Remove subscriptions that exist in DB but are not required anymore
        for db_sub in &existing_db_subs {
            if !required_set.contains(&db_sub.subscription_type) {
                // Attempt to delete on Twitch, then remove from DB if successful
                match state
                    .twitch
                    .delete_eventsub_subscription(&db_sub.twitch_subscription_id)
                    .await
                {
                    Ok(_) => {
                        if let Err(e) =
                            EventSubSubscriptionRepository::delete(&state.db, &db_sub.id).await
                        {
                            warn!(
                                "Deleted subscription on Twitch but failed to remove DB row {}: {}",
                                db_sub.id, e
                            );
                        } else {
                            info!(
                                "Removed EventSub {} (twitch_id={}) for user {}",
                                db_sub.subscription_type, db_sub.twitch_subscription_id, user.id
                            );
                        }
                    }
                    Err(e) => {
                        warn!(
                            "Failed to delete EventSub {} (twitch_id={}) on Twitch: {}",
                            db_sub.subscription_type, db_sub.twitch_subscription_id, e
                        );
                        // Do not force-delete DB row here; leave as-is for manual inspection or retry later.
                    }
                }
            }
        }

        // 2) Create missing required subscriptions

        for req in required {
            // Skip if already in DB
            if existing_db_subs.iter().any(|s| s.subscription_type == req) {
                info!(
                    "Subscription {} already exists in DB for user {}, skipping creation",
                    req, user.id
                );
                continue;
            }

            info!(
                "Creating missing EventSub subscription: type={} for user {} (twitch_id={})",
                req, user.id, user.twitch_id
            );

            // Try to create the subscription on Twitch
            let secret = &state.config.jwt.secret;
            let subscribe_result: Result<TwitchEventSub, crate::error::AppError> =
                match req.as_str() {
                    "stream.online" => {
                        state
                            .twitch
                            .subscribe_stream_online(&user.twitch_id, secret)
                            .await
                    }
                    "stream.offline" => {
                        state
                            .twitch
                            .subscribe_stream_offline(&user.twitch_id, secret)
                            .await
                    }
                    "channel.update" => {
                        state
                            .twitch
                            .subscribe_channel_update(&user.twitch_id, secret)
                            .await
                    }
                    "channel.channel_points_custom_reward_redemption.add" => {
                        state
                            .twitch
                            .subscribe_channel_points_redemption(&user.twitch_id, secret)
                            .await
                    }
                    other => {
                        warn!("Unknown subscription type requested: {}", other);
                        continue;
                    }
                };

            match subscribe_result {
                Ok(twitch_sub) => {
                    // Persist to DB
                    match EventSubSubscriptionRepository::create(
                        &state.db,
                        &user.id,
                        CreateEventSubSubscription {
                            twitch_subscription_id: twitch_sub.id.clone(),
                            subscription_type: twitch_sub.subscription_type.clone(),
                            status: twitch_sub.status.clone(),
                        },
                    )
                    .await
                    {
                        Ok(_) => {
                            info!(
                                "Created EventSub {} for user {} (twitch id={})",
                                twitch_sub.subscription_type, user.id, twitch_sub.id
                            );
                        }
                        Err(e) => {
                            warn!(
                                "Created EventSub on Twitch but failed to insert DB row for user {}: {}",
                                user.id, e
                            );
                        }
                    }
                }
                Err(e) => {
                    // Fallback: maybe the subscription already exists on Twitch; list and search
                    warn!(
                        "Failed to create EventSub {} for user {} via API: {}. Attempting to discover existing subscription.",
                        req, user.id, e
                    );

                    match state.twitch.list_eventsub_subscriptions().await {
                        Ok(listing) => {
                            if let Some(found) = listing.into_iter().find(|s| {
                                s.subscription_type == req
                                    && condition_matches_broadcaster(&s.condition, &user.twitch_id)
                            }) {
                                // Insert into DB to reflect reality
                                match EventSubSubscriptionRepository::create(
                                    &state.db,
                                    &user.id,
                                    CreateEventSubSubscription {
                                        twitch_subscription_id: found.id.clone(),
                                        subscription_type: found.subscription_type.clone(),
                                        status: found.status.clone(),
                                    },
                                )
                                .await
                                {
                                    Ok(_) => {
                                        info!(
                                            "Discovered existing EventSub {} for user {} (twitch id={}) and recorded it",
                                            found.subscription_type, user.id, found.id
                                        );
                                    }
                                    Err(err) => {
                                        warn!(
                                            "Discovered existing EventSub {} but failed to persist DB row: {}",
                                            found.id, err
                                        );
                                    }
                                }
                            } else {
                                warn!(
                                    "No matching existing EventSub {} found for broadcaster {}",
                                    req, user.twitch_id
                                );
                            }
                        }
                        Err(list_err) => {
                            warn!(
                                "Failed to list EventSub subscriptions while handling subscription error for user {}: {}",
                                user.id, list_err
                            );
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

/// Check if the EventSub condition object targets the given broadcaster id
fn condition_matches_broadcaster(condition: &Value, broadcaster_id: &str) -> bool {
    // Expect condition to be an object like { \"broadcaster_user_id\": \"123\" }
    if let Some(obj) = condition.as_object() {
        if let Some(val) = obj.get("broadcaster_user_id") {
            if let Some(s) = val.as_str() {
                return s == broadcaster_id;
            }
        }
    }

    // Some EventSub conditions might be nested or encoded differently; be permissive:
    if let Some(s) = condition.as_str() {
        return s == broadcaster_id;
    }

    false
}
