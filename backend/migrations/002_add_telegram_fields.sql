-- 002_add_telegram_fields.sql
-- Add Telegram-related fields to the `users` table so a linked Telegram user
-- can be stored on the user's profile.
--
-- Notes:
--  - All new columns are nullable to avoid touching existing rows.
--  - A UNIQUE index on `telegram_user_id` prevents multiple accounts linking the same Telegram ID.
--  - An index on `telegram_username` is added to speed up lookups (optional).

-- Add nullable columns to store Telegram info on the user profile
ALTER TABLE users ADD COLUMN telegram_user_id TEXT;
ALTER TABLE users ADD COLUMN telegram_username TEXT;
ALTER TABLE users ADD COLUMN telegram_photo_url TEXT;

-- Indexes
CREATE UNIQUE INDEX IF NOT EXISTS idx_users_telegram_user_id ON users(telegram_user_id);
CREATE INDEX IF NOT EXISTS idx_users_telegram_username ON users(telegram_username);
