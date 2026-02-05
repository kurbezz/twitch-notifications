-- Store the last sent Telegram message id per integration so we can delete it
-- when sending a new notification (replace-in-place behavior).
ALTER TABLE telegram_integrations ADD COLUMN last_telegram_message_id INTEGER;
