#![allow(unused_imports)]

//! Database models split into separate files.
//! This module re-exports individual model modules so existing imports like
//! `use crate::db::models::*;` continue to work.

pub mod calendar;
pub mod discord_integration;
pub mod eventsub_subscription;
pub mod notification_history;
pub mod notification_queue;
pub mod notification_settings;
pub mod share;
pub mod telegram_integration;
pub mod user;

// Re-export all types at the `crate::db::models` namespace for backward compatibility.
pub use self::calendar::*;
pub use self::discord_integration::*;
pub use self::eventsub_subscription::*;
pub use self::notification_history::*;
pub use self::notification_queue::*;
pub use self::notification_settings::*;
pub use self::share::*;
pub use self::telegram_integration::*;
pub use self::user::*;
