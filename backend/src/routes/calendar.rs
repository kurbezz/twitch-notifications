use std::sync::Arc;

use axum::{extract::State, routing::{get, post}, Json, Router};
use serde_json::json;

use crate::AppState;
use crate::db::DiscordIntegrationRepository;
use crate::error::AppResult;
use crate::routes::auth::AuthUser;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/sync", post(sync_now))
        .route("/status", get(get_status))
}

/// Trigger a manual calendar sync for all integrations that have calendar sync enabled.
async fn sync_now(State(state): State<Arc<AppState>>, AuthUser(_user): AuthUser) -> AppResult<Json<serde_json::Value>> {
    // Find integrations with calendar sync enabled
    let integrations = DiscordIntegrationRepository::find_with_calendar_sync(&state.db).await?;

    let mut synced = 0usize;

    for integration in integrations.into_iter() {
        match crate::services::calendar::CalendarSyncManager::sync_for_integration(&state, &integration).await {
            Ok(_) => synced += 1,
            Err(e) => tracing::warn!("Failed to sync calendar for integration {}: {:?}", integration.id, e),
        }
    }

    Ok(Json(json!({
        "synced": synced,
        "message": format!("Synced {} integrations", synced)
    })))
}

/// Return a simple status for calendar sync: whether any integrations have it enabled,
/// last sync timestamp across synced events, and total synced events count.
async fn get_status(State(state): State<Arc<AppState>>, AuthUser(_user): AuthUser) -> AppResult<Json<serde_json::Value>> {
    let enabled = {
        let list = DiscordIntegrationRepository::find_with_calendar_sync(&state.db).await?;
        !list.is_empty()
    };

    // Query DB for max(last_synced_at) and count of events
    let row = sqlx::query!(
        r#"SELECT MAX(last_synced_at) as last_synced_at, COUNT(*) as events_count FROM synced_calendar_events"#
    )
    .fetch_one(&state.db)
    .await
    .map_err(crate::error::AppError::Database)?;

    let last_sync = row
        .last_synced_at
        .map(|dt| chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(dt, chrono::Utc).to_rfc3339());

    let events_count = row.events_count as i64;

    Ok(Json(json!({
        "enabled": enabled,
        "last_sync": last_sync,
        "events_count": events_count
    })))
}
