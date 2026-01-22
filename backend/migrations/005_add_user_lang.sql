-- Migration 002
-- Add 'lang' column to 'users' table to store user's preferred language.
-- Default value is 'ru' (Russian) for backward compatibility.

-- Add the column with a default. SQLite will use the default for new rows.
ALTER TABLE users ADD COLUMN lang TEXT DEFAULT 'ru';

-- Make sure existing rows have a value (in case the DB didn't populate defaults for older rows).
UPDATE users SET lang = 'ru' WHERE lang IS NULL;

-- Optional index to speed up queries that filter by language (e.g., for localized notifications).
CREATE INDEX IF NOT EXISTS idx_users_lang ON users(lang);
