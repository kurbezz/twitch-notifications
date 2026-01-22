-- 003_add_discord_fields.sql
-- Add Discord-related fields to the `users` table so a linked Discord user
-- can be stored on the user's profile.
--
-- Notes:
--  - All new columns are nullable to avoid touching existing rows.
--  - A UNIQUE index on `discord_user_id` prevents multiple accounts linking the same Discord ID.
--  - An index on `discord_username` is added to speed up lookups (optional).

-- Add nullable columns to store Discord info on the user profile
ALTER TABLE users ADD COLUMN discord_user_id TEXT;
ALTER TABLE users ADD COLUMN discord_username TEXT;
ALTER TABLE users ADD COLUMN discord_avatar_url TEXT;

-- Indexes
CREATE UNIQUE INDEX IF NOT EXISTS idx_users_discord_user_id ON users(discord_user_id);
CREATE INDEX IF NOT EXISTS idx_users_discord_username ON users(discord_username);
