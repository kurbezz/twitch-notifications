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

    // Extract the file path from the database URL
    let db_path = db_url.strip_prefix("sqlite://").unwrap_or(db_url);
    let db_file_path = Path::new(db_path);

    // Create parent directory if it doesn't exist
    if let Some(parent) = db_file_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| {
                anyhow::anyhow!(
                    "Failed to create database directory {}: {}",
                    parent.display(),
                    e
                )
            })?;
            tracing::info!(
                "Database directory created or already exists: {}",
                parent.display()
            );
        }
    }

    let connect_options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(true);

    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(config.database.max_connections)
        .connect_with(connect_options)
        .await?;

    // Log successful database file creation or connection
    if db_file_path.exists() {
        tracing::info!(
            "Successfully connected to database file: {}",
            db_file_path.display()
        );
    } else {
        tracing::info!(
            "Database file created successfully: {}",
            db_file_path.display()
        );
    }

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
/// These are spawned as `tokio::spawn` tasks. The function returns a vector of
/// `JoinHandle<()>`s so callers can await task shutdown. Each worker listens
/// for a shutdown notification via a `tokio::sync::broadcast::Sender<()>`.
pub fn spawn_background_workers(
    state: Arc<crate::AppState>,
    shutdown: tokio::sync::broadcast::Sender<()>,
) -> Vec<tokio::task::JoinHandle<()>> {
    let mut handles = Vec::new();

    // EventSub sync worker
    {
        let mut shutdown_rx = shutdown.subscribe();
        let state = state.clone();
        handles.push(tokio::spawn(async move {
            loop {
                tracing::info!("Starting periodic EventSub synchronization for all users");

                match crate::db::UserRepository::list_all(&state.db).await {
                    Ok(users) => {
                        for user in users {
                            // Check for shutdown between users so we can exit faster.
                            if shutdown_rx.try_recv().is_ok() {
                                tracing::info!("EventSub worker received shutdown signal");
                                return;
                            }

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

                // Sleep for 1 hour between sync cycles or exit early on shutdown.
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        tracing::info!("EventSub worker shutting down");
                        break;
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_secs(60 * 60)) => {}
                }
            }
        }));
    }

    // Calendar sync worker
    {
        let mut shutdown_rx = shutdown.subscribe();
        let state = state.clone();
        handles.push(tokio::spawn(async move {
            loop {
                tracing::info!("Starting periodic calendar synchronization for integrations");

                if let Err(e) =
                    crate::services::calendar::CalendarSyncManager::sync_all(&state).await
                {
                    tracing::warn!("Calendar sync failed: {:?}", e);
                }

                // Sleep for 1 hour between sync cycles or exit early on shutdown.
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        tracing::info!("Calendar sync worker shutting down");
                        break;
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_secs(60 * 60)) => {}
                }
            }
        }));
    }

    // Notification retry worker
    {
        let mut shutdown_rx = shutdown.subscribe();
        let state = state.clone();
        handles.push(tokio::spawn(async move {
            loop {
                tracing::debug!("Polling notification retry queue for due tasks");

                // Exit early if shutdown requested
                if shutdown_rx.try_recv().is_ok() {
                    tracing::info!("Notification retry worker received shutdown signal");
                    break;
                }

                // If retries are disabled, sleep longer and continue.
                if !state.config.notification_retry.enabled {
                    tokio::select! {
                        _ = shutdown_rx.recv() => {
                            tracing::info!("Notification retry worker shutting down");
                            break;
                        }
                        _ = tokio::time::sleep(std::time::Duration::from_secs(60)) => {}
                    }
                    continue;
                }

                let concurrency = state.config.notification_retry.worker_concurrency as i64;

                match crate::db::repository::NotificationQueueRepository::fetch_and_claim_due(
                    &state.db,
                    concurrency,
                )
                .await
                {
                    Ok(tasks) => {
                        if tasks.is_empty() {
                            // Nothing due right now; back off according to configured poll interval.
                            tokio::select! {
                                _ = shutdown_rx.recv() => {
                                    tracing::info!("Notification retry worker shutting down");
                                    break;
                                }
                                _ = tokio::time::sleep(std::time::Duration::from_secs(
                                    state.config.notification_retry.poll_interval_seconds,
                                )) => {}
                            }
                            continue;
                        }

                        // Spawn a task per claimed item (bounded by the number claimed).
                        for task in tasks {
                            if shutdown_rx.try_recv().is_ok() {
                                tracing::info!("Skipping spawning new notification retry tasks due to shutdown");
                                break;
                            }
                            let state = state.clone();
                            tokio::spawn(async move {
                                let svc = crate::services::notifications::NotificationService::new(
                                    &state,
                                );
                                if let Err(e) = svc.process_queued_task(task).await {
                                    tracing::warn!("Notification retry task failed: {:?}", e);
                                }
                            });
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to fetch due notification tasks: {:?}", e);
                    }
                }

                // Wait before next poll or exit early on shutdown.
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        tracing::info!("Notification retry worker shutting down");
                        break;
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_secs(
                        state.config.notification_retry.poll_interval_seconds,
                    )) => {}
                }
            }
        }));
    }

    handles
}
