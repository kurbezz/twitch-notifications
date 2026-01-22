//! Initialization helpers for the application:
//! - database connection + migrations
//! - optional integrations (Telegram / Discord)
//! - background worker spawn helpers
//!
//! This module centralizes bits that used to live in `main.rs`.

use std::{path::Path, sync::Arc};

use anyhow::Result;

use crate::config::Config;

/// Redact potentially sensitive information from a database URL before logging.
///
/// Attempts to parse the URL and remove userinfo (username:password) components.
/// Falls back to removing everything before '@' or returning "(redacted)".
pub fn redact_db_url(db_url: &str) -> String {
    if let Ok(url) = url::Url::parse(db_url) {
        let scheme = url.scheme();
        let host = url.host_str().unwrap_or("");
        let port_part = url.port().map(|p| format!(":{}", p)).unwrap_or_default();
        let path = url.path();
        format!("{}://{}{}{}", scheme, host, port_part, path)
    } else {
        if let Some(at_pos) = db_url.find('@') {
            let without_creds = &db_url[at_pos + 1..];
            return format!("(redacted){}", without_creds);
        }
        "(redacted)".to_string()
    }
}

/// Initialize SQLite database connection and run migrations.
///
/// Creates the parent directory for the database file (if applicable),
/// opens a connection pool using `create_if_missing(true)` and runs migrations.
pub async fn init_db(config: &Config) -> Result<sqlx::SqlitePool> {
    let db_url = &config.database.url;
    tracing::info!("Connecting to database: {}", redact_db_url(db_url));

    if let Some(parent) = Path::new(db_url.strip_prefix("sqlite://").unwrap_or(db_url)).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).ok();
        }
    }

    let connect_options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(db_url.strip_prefix("sqlite://").unwrap_or(db_url))
        .create_if_missing(true);

    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(config.database.max_connections)
        .connect_with(connect_options)
        .await?;

    tracing::info!("Running database migrations");
    // Keep the same path as before (relative to crate root)
    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}

/// Initialize optional integrations (Telegram, Discord) and store them into `AppState`.
///
/// Any errors are logged; failures to initialize an optional integration do not stop
/// the application from starting.
pub async fn initialize_optional_integrations(state: &Arc<crate::AppState>) {
    // Telegram
    if let Some(ref token) = state.config.telegram.bot_token {
        tracing::info!("Initializing Telegram bot");
        match crate::services::telegram::TelegramService::new(token.clone()).await {
            Ok(telegram) => {
                *state.telegram.write().await = Some(telegram);
                tracing::info!("Telegram bot initialized successfully");
            }
            Err(e) => {
                tracing::warn!("Failed to initialize Telegram bot: {}", e);
            }
        }
    }

    // Discord
    if let Some(ref token) = state.config.discord.bot_token {
        tracing::info!("Initializing Discord bot");
        match crate::services::discord::DiscordService::new(token.clone()).await {
            Ok(discord) => {
                *state.discord.write().await = Some(discord);
                tracing::info!("Discord bot initialized successfully");
            }
            Err(e) => {
                tracing::warn!("Failed to initialize Discord bot: {}", e);
            }
        }
    }
}

/// Spawn background workers:
/// - periodic EventSub synchronization for all users
/// - periodic calendar synchronization for integrations
///
/// These are spawned as `tokio::spawn` tasks and will run for the lifetime
/// of the process.
pub fn spawn_background_workers(state: Arc<crate::AppState>) {
    // EventSub sync worker
    {
        let state = state.clone();
        tokio::spawn(async move {
            loop {
                tracing::info!("Starting periodic EventSub synchronization for all users");

                match crate::db::UserRepository::list_all(&state.db).await {
                    Ok(users) => {
                        for user in users {
                            if let Err(e) =
                                crate::services::subscriptions::SubscriptionManager::sync_for_user(
                                    &state, &user,
                                )
                                .await
                            {
                                tracing::warn!(
                                    "Failed to sync EventSub for user {}: {:?}",
                                    user.id,
                                    e
                                );
                            } else {
                                tracing::info!("Synced EventSub for user {}", user.id);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to list users for EventSub synchronization: {:?}",
                            e
                        );
                    }
                }

                // Sleep for 1 hour between sync cycles
                tokio::time::sleep(std::time::Duration::from_secs(60 * 60)).await;
            }
        });
    }

    // Calendar sync worker
    {
        let state = state.clone();
        tokio::spawn(async move {
            loop {
                tracing::info!("Starting periodic calendar synchronization for integrations");

                if let Err(e) =
                    crate::services::calendar::CalendarSyncManager::sync_all(&state).await
                {
                    tracing::warn!("Calendar sync failed: {:?}", e);
                }

                // Sleep for 1 hour between sync cycles
                tokio::time::sleep(std::time::Duration::from_secs(60 * 60)).await;
            }
        });
    }
}
