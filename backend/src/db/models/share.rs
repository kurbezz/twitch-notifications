use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// ============================================================================
// Settings Share Models
// ============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct SettingsShare {
    pub id: String,
    pub owner_user_id: String,
    pub grantee_user_id: String,
    pub can_manage: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
