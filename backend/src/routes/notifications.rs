use std::sync::Arc;

use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::db::NotificationLogRepository;
use crate::error::AppResult;
use crate::routes::auth::AuthUser;
use crate::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_notifications))
        .route("/stats", get(get_notification_stats))
}

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ListNotificationsQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub notification_type: Option<String>,
    pub destination_type: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct NotificationsListResponse {
    pub items: Vec<NotificationResponse>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
}

#[derive(Debug, Serialize)]
pub struct NotificationResponse {
    pub id: String,
    pub user_id: String,
    pub notification_type: String,
    pub destination_type: String,
    pub destination_id: String,
    pub content: String,
    pub status: String,
    pub error_message: Option<String>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Serialize)]
pub struct NotificationStatsResponse {
    pub total_sent: i64,
    pub total_failed: i64,
    pub by_type: std::collections::HashMap<String, i64>,
    pub by_destination: std::collections::HashMap<String, i64>,
}

// ============================================================================
// Handlers
// ============================================================================

/// List notification history for the current user
async fn list_notifications(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Query(query): Query<ListNotificationsQuery>,
) -> AppResult<Json<NotificationsListResponse>> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * per_page;

    // Get notifications (apply optional filters)
    let notifications = NotificationLogRepository::find_by_user_id_with_filters(
        &state.db,
        &user.id,
        Some(per_page),
        Some(offset),
        query.notification_type.as_deref(),
        query.destination_type.as_deref(),
        query.status.as_deref(),
    )
    .await?;

    // Get total count after applying filters
    let total = NotificationLogRepository::count_by_user_id_with_filters(
        &state.db,
        &user.id,
        query.notification_type.as_deref(),
        query.destination_type.as_deref(),
        query.status.as_deref(),
    )
    .await?;

    let total_pages = (total as f64 / per_page as f64).ceil() as i64;

    let notification_responses: Vec<NotificationResponse> = notifications
        .into_iter()
        .map(|n| NotificationResponse {
            id: n.id,
            user_id: n.user_id,
            notification_type: n.notification_type,
            destination_type: n.destination_type,
            destination_id: n.destination_id,
            content: n.content,
            status: n.status,
            error_message: n.error_message,
            created_at: n.created_at,
        })
        .collect();

    Ok(Json(NotificationsListResponse {
        items: notification_responses,
        total,
        page,
        per_page,
        total_pages,
    }))
}

/// Get notification statistics for the current user
async fn get_notification_stats(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
) -> AppResult<Json<NotificationStatsResponse>> {
    // Run aggregation queries in parallel
    let db = state.db.clone();
    let user_id = user.id.clone();

    let (total_sent, total_failed, type_counts, dest_counts) = tokio::try_join!(
        {
            let db = db.clone();
            let user_id = user_id.clone();
            async move {
                NotificationLogRepository::count_by_user_id_and_status(&db, &user_id, "sent").await
            }
        },
        {
            let db = db.clone();
            let user_id = user_id.clone();
            async move {
                NotificationLogRepository::count_by_user_id_and_status(&db, &user_id, "failed")
                    .await
            }
        },
        {
            let db = db.clone();
            let user_id = user_id.clone();
            async move { NotificationLogRepository::counts_by_notification_type(&db, &user_id).await }
        },
        {
            let db = db.clone();
            let user_id = user_id.clone();
            async move { NotificationLogRepository::counts_by_destination_type(&db, &user_id).await }
        }
    )?;

    Ok(Json(NotificationStatsResponse {
        total_sent,
        total_failed,
        by_type: type_counts,
        by_destination: dest_counts,
    }))
}
