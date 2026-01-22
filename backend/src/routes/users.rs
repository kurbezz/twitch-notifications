use std::sync::Arc;

use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::db::{User, UserRepository};
use crate::error::AppResult;
use crate::routes::auth::AuthUser;
use crate::AppState;

/// Router for user-related endpoints (searching users)
pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/", get(search_users))
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    /// Query string to search for (login or display name)
    pub q: Option<String>,
    /// Maximum number of results to return
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: String,
    pub twitch_id: String,
    pub twitch_login: String,
    pub twitch_display_name: String,
    pub twitch_profile_image_url: Option<String>,
    pub telegram_user_id: Option<String>,
    pub telegram_username: Option<String>,
    pub telegram_photo_url: Option<String>,
}

impl From<User> for UserResponse {
    fn from(u: User) -> Self {
        Self {
            id: u.id,
            twitch_id: u.twitch_id,
            twitch_login: u.twitch_login,
            twitch_display_name: u.twitch_display_name,
            // Database currently stores profile URL as non-null text; map into Option for API
            twitch_profile_image_url: Some(u.twitch_profile_image_url),
            telegram_user_id: u.telegram_user_id,
            telegram_username: u.telegram_username,
            telegram_photo_url: u.telegram_photo_url,
        }
    }
}

/// Search users by twitch login or display name.
/// Requires authentication. Returns an empty array for empty/too-short queries.
async fn search_users(
    State(state): State<Arc<AppState>>,
    AuthUser(_user): AuthUser,
    Query(query): Query<SearchQuery>,
) -> AppResult<Json<Vec<UserResponse>>> {
    let q = query.q.unwrap_or_default().trim().to_string();

    // Avoid performing searches for very short queries
    if q.len() < 2 {
        return Ok(Json(Vec::new()));
    }

    let limit = query.limit.unwrap_or(10).min(50) as i64;

    let users = UserRepository::search(&state.db, &q, limit).await?;
    let res: Vec<UserResponse> = users.into_iter().map(Into::into).collect();

    Ok(Json(res))
}
