-- Migration: Add user-level notify_reward_redemption to user_settings
-- Purpose: store whether the user wants notifications for reward redemptions at the user level.
-- This is separate from per-integration flags.

ALTER TABLE user_settings
ADD COLUMN notify_reward_redemption BOOLEAN NOT NULL DEFAULT FALSE;

-- Defensive: ensure existing rows have a concrete value (older SQLite versions / migration runners)
UPDATE user_settings
SET notify_reward_redemption = FALSE
WHERE notify_reward_redemption IS NULL;
