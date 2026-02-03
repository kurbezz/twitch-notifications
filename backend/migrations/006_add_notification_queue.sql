-- 006_add_notification_queue.sql
-- Create a persistent notification retry queue with an expiration (TTL) so
-- time-sensitive notifications aren't retried after they become irrelevant.
--
-- The table stores:
--  - `content_json` -- serialized payload of the specific notification variant
--  - `message`      -- already-rendered message (so retries re-send the same content)
--  - scheduling + attempts metadata (attempts, max_attempts, next_attempt_at)
--  - optional `expires_at` -- stop retrying after this time (set per-task or defaulted)
--  - `notification_log_id` links to the original notification_history row so the worker
--    can update the log when a retry eventually succeeds or the task is DLQ'd.
--
-- The migration also creates helpful indexes and a trigger that defaults
-- `expires_at` to `created_at + 1 hour` when not provided (this gives a safe default TTL).
--
-- Notes about retry policy:
--  - The application should set a short expires_at for time-sensitive events
--    (e.g., stream_online). If expires_at is past, the worker will not attempt the task.
--  - The worker should increment `attempts` and, when `attempts >= max_attempts`,
--    move the task to 'dead' (DLQ) and update the referenced notification log accordingly.
--
-- This migration is intended to be a single combined migration for queue + TTL support.

CREATE TABLE IF NOT EXISTS notification_queue (
    id TEXT PRIMARY KEY,
    notification_log_id TEXT REFERENCES notification_history(id) ON DELETE SET NULL,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    -- Notification payload + rendered message
    notification_type TEXT NOT NULL,   -- e.g. 'stream_online', 'title_change', etc.
    content_json TEXT NOT NULL,        -- JSON-serialized variant payload
    message TEXT NOT NULL,             -- rendered message (template expanded)

    -- Destination info
    destination_type TEXT NOT NULL,    -- 'telegram', 'discord', ...
    destination_id TEXT NOT NULL,      -- chat_id or channel_id
    webhook_url TEXT,                  -- optional webhook URL (Discord webhook)

    -- Retry metadata
    attempts INTEGER NOT NULL DEFAULT 0,
    max_attempts INTEGER NOT NULL DEFAULT 5,
    next_attempt_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,

    -- Expiration (TTL) - if set and <= CURRENT_TIMESTAMP the worker should treat the task as expired
    expires_at DATETIME,

    -- Last observed error and state
    last_error TEXT,
    status TEXT NOT NULL DEFAULT 'pending', -- 'pending', 'processing', 'succeeded', 'dead'

    -- Timestamps
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Indexes to make the worker and queries efficient
CREATE INDEX IF NOT EXISTS idx_notification_queue_next_attempt_at ON notification_queue(next_attempt_at);
CREATE INDEX IF NOT EXISTS idx_notification_queue_status ON notification_queue(status);
CREATE INDEX IF NOT EXISTS idx_notification_queue_user_id ON notification_queue(user_id);
CREATE INDEX IF NOT EXISTS idx_notification_queue_expires_at ON notification_queue(expires_at);

-- If expires_at is not provided, default it to created_at + 5 minutes.
-- This prevents old queued notifications from being retried forever.
CREATE TRIGGER IF NOT EXISTS notification_queue_set_expires_at_after_insert
AFTER INSERT ON notification_queue
FOR EACH ROW
WHEN NEW.expires_at IS NULL
BEGIN
  UPDATE notification_queue
  SET expires_at = datetime(NEW.created_at, '+5 minutes')
  WHERE id = NEW.id;
END;
