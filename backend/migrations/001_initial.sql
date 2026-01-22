-- Initial database migration for Twitch Notifications Service
-- Simplified schema: streamer configures only for their own channel

-- Users table - stores Twitch authenticated users (streamers)
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    twitch_id TEXT UNIQUE NOT NULL,
    twitch_login TEXT NOT NULL,
    twitch_display_name TEXT NOT NULL,
    twitch_email TEXT NOT NULL,
    twitch_profile_image_url TEXT NOT NULL,
    twitch_access_token TEXT NOT NULL,
    twitch_refresh_token TEXT NOT NULL,
    twitch_token_expires_at DATETIME NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_users_twitch_id ON users(twitch_id);
CREATE INDEX IF NOT EXISTS idx_users_twitch_login ON users(twitch_login);

-- User settings table - notification templates and preferences
CREATE TABLE IF NOT EXISTS user_settings (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,

    -- Stream notification messages (supports placeholders like {streamer}, {title}, {game}, {url})
    stream_online_message TEXT NOT NULL,
    stream_offline_message TEXT NOT NULL,
    stream_title_change_message TEXT NOT NULL,
    stream_category_change_message TEXT NOT NULL,

    -- Channel point reward redemption message
    reward_redemption_message TEXT NOT NULL,

    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_user_settings_user_id ON user_settings(user_id);

-- Telegram integration
CREATE TABLE IF NOT EXISTS telegram_integrations (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    telegram_chat_id TEXT NOT NULL,
    telegram_chat_title TEXT,
    telegram_chat_type TEXT NOT NULL DEFAULT 'private', -- 'private', 'group', 'supergroup', 'channel'
    is_enabled BOOLEAN NOT NULL DEFAULT TRUE,

    -- Per-integration notification settings
    notify_stream_online BOOLEAN NOT NULL DEFAULT TRUE,
    notify_stream_offline BOOLEAN NOT NULL DEFAULT FALSE,
    notify_title_change BOOLEAN NOT NULL DEFAULT TRUE,
    notify_category_change BOOLEAN NOT NULL DEFAULT TRUE,
    notify_reward_redemption BOOLEAN NOT NULL DEFAULT FALSE,

    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,

    UNIQUE(user_id, telegram_chat_id)
);

CREATE INDEX IF NOT EXISTS idx_telegram_integrations_user_id ON telegram_integrations(user_id);
CREATE INDEX IF NOT EXISTS idx_telegram_integrations_chat_id ON telegram_integrations(telegram_chat_id);

-- Discord integration
CREATE TABLE IF NOT EXISTS discord_integrations (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    discord_guild_id TEXT NOT NULL,
    discord_channel_id TEXT NOT NULL,
    discord_guild_name TEXT,
    discord_channel_name TEXT,
    discord_webhook_url TEXT,
    is_enabled BOOLEAN NOT NULL DEFAULT TRUE,

    -- Per-integration notification settings
    notify_stream_online BOOLEAN NOT NULL DEFAULT TRUE,
    notify_stream_offline BOOLEAN NOT NULL DEFAULT FALSE,
    notify_title_change BOOLEAN NOT NULL DEFAULT TRUE,
    notify_category_change BOOLEAN NOT NULL DEFAULT TRUE,
    notify_reward_redemption BOOLEAN NOT NULL DEFAULT FALSE,
    calendar_sync_enabled BOOLEAN NOT NULL DEFAULT FALSE,

    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,

    UNIQUE(user_id, discord_guild_id, discord_channel_id)
);

CREATE INDEX IF NOT EXISTS idx_discord_integrations_user_id ON discord_integrations(user_id);
CREATE INDEX IF NOT EXISTS idx_discord_integrations_guild_id ON discord_integrations(discord_guild_id);

-- EventSub subscriptions - tracks active Twitch EventSub subscriptions for the streamer's channel
CREATE TABLE IF NOT EXISTS eventsub_subscriptions (
    id TEXT PRIMARY KEY,
    twitch_subscription_id TEXT UNIQUE NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    subscription_type TEXT NOT NULL, -- 'stream.online', 'stream.offline', 'channel.update', 'channel.channel_points_custom_reward_redemption.add'
    status TEXT NOT NULL DEFAULT 'pending', -- 'pending', 'enabled', 'webhook_callback_verification_failed', 'revoked'
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_eventsub_subscriptions_user_id ON eventsub_subscriptions(user_id);
CREATE INDEX IF NOT EXISTS idx_eventsub_subscriptions_type ON eventsub_subscriptions(subscription_type);

-- Notification history - for tracking sent notifications
CREATE TABLE IF NOT EXISTS notification_history (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    notification_type TEXT NOT NULL, -- 'stream_online', 'stream_offline', 'title_change', 'category_change', 'reward_redemption'
    destination_type TEXT NOT NULL, -- 'telegram', 'discord', 'chat'
    destination_id TEXT NOT NULL, -- chat_id or channel_id
    content TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'sent', -- 'sent', 'failed', 'pending'
    error_message TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_notification_history_user_id ON notification_history(user_id);
CREATE INDEX IF NOT EXISTS idx_notification_history_created_at ON notification_history(created_at);
CREATE INDEX IF NOT EXISTS idx_notification_history_notification_type ON notification_history(notification_type);

-- Calendar events synced from Twitch schedule to Discord
CREATE TABLE IF NOT EXISTS synced_calendar_events (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    twitch_segment_id TEXT NOT NULL,
    discord_integration_id TEXT REFERENCES discord_integrations(id) ON DELETE CASCADE,
    discord_event_id TEXT,

    title TEXT NOT NULL,
    start_time DATETIME NOT NULL,
    end_time DATETIME,
    category_name TEXT,
    is_recurring BOOLEAN NOT NULL DEFAULT FALSE,

    last_synced_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,

    UNIQUE(twitch_segment_id, discord_integration_id)
);

CREATE INDEX IF NOT EXISTS idx_synced_calendar_events_user_id ON synced_calendar_events(user_id);

-- Session tokens for web auth
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT UNIQUE NOT NULL,
    expires_at DATETIME NOT NULL,
    user_agent TEXT,
    ip_address TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_used_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_token_hash ON sessions(token_hash);
CREATE INDEX IF NOT EXISTS idx_sessions_expires_at ON sessions(expires_at);

-- Stream state cache - to detect changes (for the streamer's own channel)
CREATE TABLE IF NOT EXISTS stream_state_cache (
    user_id TEXT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    is_live BOOLEAN NOT NULL DEFAULT FALSE,
    title TEXT,
    category_id TEXT,
    category_name TEXT,
    started_at DATETIME,
    viewer_count INTEGER,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Settings shares table - allows users to share their settings with other users
CREATE TABLE IF NOT EXISTS settings_shares (
    id TEXT PRIMARY KEY,
    owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    grantee_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    can_manage BOOLEAN NOT NULL DEFAULT FALSE,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (owner_user_id, grantee_user_id)
);

CREATE INDEX IF NOT EXISTS idx_settings_shares_owner_user_id ON settings_shares(owner_user_id);
CREATE INDEX IF NOT EXISTS idx_settings_shares_grantee_user_id ON settings_shares(grantee_user_id);
