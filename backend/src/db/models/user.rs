use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub twitch_id: String,
    pub twitch_login: String,
    pub twitch_display_name: String,
    pub twitch_email: String,
    pub twitch_profile_image_url: String,
    pub twitch_access_token: String,
    pub twitch_refresh_token: String,
    pub twitch_token_expires_at: NaiveDateTime,

    // Telegram integration fields stored on the user's profile when they link their Telegram account
    pub telegram_user_id: Option<String>,
    pub telegram_username: Option<String>,
    pub telegram_photo_url: Option<String>,

    // Discord integration fields stored on the user's profile when they link their Discord account
    pub discord_user_id: Option<String>,
    pub discord_username: Option<String>,
    pub discord_avatar_url: Option<String>,
    pub lang: Option<String>,

    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
