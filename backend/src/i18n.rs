/*
Simple i18n helper for the backend.

This module provides:
- A tiny embedded translations store for RU/EN (compile-time embedded JSON).
- A simple `tr` function to lookup translations by key + optional params.
- A `t` convenience wrapper using the default language (DEFAULT_LANG).

Usage:
    use crate::i18n;
    let msg = i18n::t("validation.owner_telegram_not_linked");
    let msg_with = i18n::tr(None, "messages.stream_online_default", Some(&[("streamer", "Ninja"), ("title", "Let's go!"), ("game", "Fortnite"), ("url", "https://twitch.tv/ninja")]));

Notes:
- Placeholders in translation strings use single-brace format: `{name}`.
- Default language is `ru`. If a key is missing for the requested language,
  the fallback language will be used.
*/

use std::collections::HashMap;
use std::sync::OnceLock;

pub const DEFAULT_LANG: &str = "ru";

static TRANSLATIONS: OnceLock<HashMap<String, HashMap<String, String>>> = OnceLock::new();

const RU_JSON: &str = r#"
{
  "validation.owner_telegram_not_linked": "У владельца не привязан аккаунт Telegram. Пожалуйста, свяжите Telegram в разделе «Настройки» перед добавлением интеграции.",
  "error.refresh_telegram_photo.download_failed": "Не удалось скачать фото по ссылке и TELEGRAM_BOT_TOKEN не настроен. Попробуйте перепривязать Telegram или настройте BOT_TOKEN на сервере.",
  "error.refresh_telegram_photo.not_found": "Не удалось получить фото из Telegram (ни по ссылке, ни через Bot API).",
  "error.refresh_telegram_photo.service_unavailable": "Не удалось обновить фото из-за внутренней ошибки. Проверьте логи сервера или попробуйте позже.",
  "validation.chat_id.group_invalid": "Неверный Chat ID для группы. Ожидается отрицательное число (например, -123456789).",
  "validation.chat_id.supergroup_invalid": "Неверный Chat ID для супергруппы/канала. Ожидается формат -100<цифры> (например, -1001234567890).",
  "validation.telegram_bot_not_configured": "Telegram bot не настроен на сервере, невозможно проверить права администратора. Пожалуйста, настройте бота.",
  "validation.must_be_admin": "Вы должны быть администратором в этом чате, чтобы добавить интеграцию",
  "validation.admin_check_failed": "Не удалось проверить права администратора. Убедитесь, что бот добавлен в чат и повторите попытку.",
  "errors.no_share_manage": "У вас нет прав управлять интеграциями для этого пользователя",
  "not_found.user": "Пользователь не найден",
  "bad_request.no_discord_linked": "У пользователя не привязан Discord",
  "service_unavailable.discord_service_unavailable": "Сервис Discord недоступен",
  "errors.insufficient_permissions": "Пользователь должен быть владельцем сервера или иметь права «Управление сервером»/«Администратор»",
  "messages.stream_online_default": "🔴 {streamer} начал стрим!\n\n{title}\n🎮 {game}\n\n{url}",
  "messages.stream_offline_default": "⚫ {streamer} завершил стрим",
  "messages.stream_title_change_default": "📝 {streamer} изменил название стрима:\n\n{title}",
  "messages.stream_category_change_default": "🎮 {streamer} сменил категорию на: {game}",
  "messages.reward_redemption_default": "🎁 {user} активировал награду \"{reward}\"!",
  "messages.test_notification_title": "🧪 Тестовое уведомление",
  "messages.test_notification_body": "Это тестовое уведомление от Уведомлений Twitch.\n\nЕсли вы видите это сообщение, ваша интеграция работает корректно! ✅",
  "test_notification.success": "Тестовое уведомление отправлено успешно",
  "test_notification.failure": "Не удалось отправить тестовое уведомление: {err}",
  "not_found.integration": "Интеграция не найдена",
  "integration.deleted": "Интеграция успешно удалена",
  "integration.delete_error": "Не удалось удалить интеграцию",
  "integration.create_error": "Не удалось создать интеграцию",
  "integration.update_error": "Не удалось обновить интеграцию",
  "auth.logged_out": "Вы вышли из системы",
  "telegram.already_linked": "Telegram уже привязан",
  "telegram.linked": "Telegram успешно подключен",
  "telegram.unlinked": "Telegram успешно отключён",
  "discord.unlinked": "Discord успешно отключён",
  "auth.token_refreshed": "Токен успешно обновлён",
  "error.unsupported_language": "Неподдерживаемый язык: {lang}",
  "app.name": "Уведомления Twitch"
}
"#;

const EN_JSON: &str = r#"
{
  "validation.owner_telegram_not_linked": "Owner has no linked Telegram account. Please link Telegram in Settings before adding an integration.",
  "error.refresh_telegram_photo.download_failed": "Failed to download photo from URL and TELEGRAM_BOT_TOKEN is not configured. Try re-linking Telegram or configure BOT_TOKEN on the server.",
  "error.refresh_telegram_photo.not_found": "Failed to obtain a photo from Telegram (neither by URL nor via the Bot API).",
  "error.refresh_telegram_photo.service_unavailable": "Failed to update photo due to an internal error. Check server logs or try again later.",
  "validation.chat_id.group_invalid": "Invalid chat ID for a group. Expected a negative number (e.g., -123456789).",
  "validation.chat_id.supergroup_invalid": "Invalid chat ID for a supergroup/channel. Expected format -100<digits> (e.g., -1001234567890).",
  "validation.telegram_bot_not_configured": "Telegram bot is not configured on the server; cannot check admin permissions. Please configure the bot.",
  "validation.must_be_admin": "You must be an administrator in this chat to add an integration",
  "validation.admin_check_failed": "Failed to verify admin permissions. Ensure the bot is added to the chat and try again.",
  "errors.no_share_manage": "You do not have permission to manage integrations for this user",
  "not_found.user": "User not found",
  "bad_request.no_discord_linked": "User has no linked Discord",
  "service_unavailable.discord_service_unavailable": "Discord service is unavailable",
  "errors.insufficient_permissions": "User must be server owner or have Manage Server / Administrator permissions",
  "messages.stream_online_default": "🔴 {streamer} started streaming!\n\n{title}\n🎮 {game}\n\n{url}",
  "messages.stream_offline_default": "⚫ {streamer} ended the stream",
  "messages.stream_title_change_default": "📝 {streamer} changed stream title:\n\n{title}",
  "messages.stream_category_change_default": "🎮 {streamer} changed category to: {game}",
  "messages.reward_redemption_default": "🎁 {user} redeemed reward \"{reward}\"!",
  "messages.test_notification_title": "🧪 Test Notification",
  "messages.test_notification_body": "This is a test notification from Twitch Notifications.\n\nIf you can see this message, your integration is working correctly! ✅",
  "test_notification.success": "Test notification sent successfully",
  "test_notification.failure": "Failed to send test notification: {err}",
  "not_found.integration": "Integration not found",
  "integration.deleted": "Integration deleted successfully",
  "integration.delete_error": "Failed to delete integration",
  "integration.create_error": "Failed to create integration",
  "integration.update_error": "Failed to update integration",
  "auth.logged_out": "Logged out",
  "telegram.already_linked": "Telegram already linked",
  "telegram.linked": "Telegram linked",
  "telegram.unlinked": "Telegram unlinked",
  "discord.unlinked": "Discord unlinked",
  "auth.token_refreshed": "Token refreshed successfully",
  "error.unsupported_language": "Unsupported language: {lang}",
  "app.name": "Twitch Notifications"
}
"#;

/// Initialize translations map (lazy).
fn build_translations() -> HashMap<String, HashMap<String, String>> {
    let mut out: HashMap<String, HashMap<String, String>> = HashMap::new();

    // Parse RU
    let ru_map: HashMap<String, String> = serde_json::from_str(RU_JSON).unwrap_or_else(|e| {
        panic!("failed to parse RU_JSON in i18n module: {}", e);
    });
    out.insert("ru".to_string(), ru_map);

    // Parse EN
    let en_map: HashMap<String, String> = serde_json::from_str(EN_JSON).unwrap_or_else(|e| {
        panic!("failed to parse EN_JSON in i18n module: {}", e);
    });
    out.insert("en".to_string(), en_map);

    out
}

/// Returns the global translations map (lang -> (key -> message)).
fn translations() -> &'static HashMap<String, HashMap<String, String>> {
    TRANSLATIONS.get_or_init(build_translations)
}

/// Normalize a language tag into a short, lowercase code (e.g. "en-US" -> "en").
///
/// This is useful when accepting language values from external sources (browser
/// `navigator.language`, query params, etc.) and wanting to convert them to
/// the canonical short form used by our translations keys.
pub fn normalize_language(lang: &str) -> String {
    lang.split('-').next().unwrap_or(lang).to_lowercase()
}

/// Returns true if the given language code is supported by the backend i18n
/// translations (e.g. "ru", "en").
pub fn is_supported_language(lang: &str) -> bool {
    translations().contains_key(lang)
}

/// Translate a key using an explicit language (or default if None).
///
/// - `lang`: optional language code (`"ru"`, `"en"`, ...). If None, DEFAULT_LANG is used.
/// - `key`: translation key (flat string, e.g. "validation.owner_telegram_not_linked").
/// - `params`: optional slice of (name, value) for placeholder replacement. Replacements use single-brace placeholders `{name}`.
///
/// Returns the translated and parameter-substituted string. If no translation is found,
/// returns a sensible fallback (default language value or the key itself).
pub fn tr(lang: Option<&str>, key: &str, params: Option<&[(&str, &str)]>) -> String {
    let map = translations();

    let desired = lang.unwrap_or(DEFAULT_LANG);

    // Try requested language
    let val = map
        .get(desired)
        .and_then(|m| m.get(key))
        .cloned()
        // Fallback to default language
        .or_else(|| map.get(DEFAULT_LANG).and_then(|m| m.get(key)).cloned())
        // If still missing, return the key itself (useful in logs)
        .unwrap_or_else(|| key.to_string());

    if let Some(params) = params {
        let mut s = val;
        for (k, v) in params {
            s = s.replace(&format!("{{{}}}", k), v);
        }
        s
    } else {
        val
    }
}

/// Convenience wrapper: translate using default language (DEFAULT_LANG).
pub fn t(key: &str) -> String {
    tr(None, key, None)
}

/// Convenience wrapper with params (default language).
pub fn t_with(key: &str, params: &[(&str, &str)]) -> String {
    tr(None, key, Some(params))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tr_basic() {
        let s = tr(Some("ru"), "validation.owner_telegram_not_linked", None);
        assert!(s.contains("Telegram"));
    }

    #[test]
    fn test_t_with_params() {
        let s = t_with(
            "messages.stream_online_default",
            &[
                ("streamer", "User"),
                ("title", "Hello"),
                ("game", "Chess"),
                ("url", "http://x"),
            ],
        );
        assert!(s.contains("User"));
        assert!(s.contains("Hello"));
    }

    #[test]
    fn test_fallback_to_default() {
        // Unknown language falls back to default (ru)
        let s = tr(Some("fr"), "validation.owner_telegram_not_linked", None);
        assert!(s.contains("Telegram"));
    }

    #[test]
    fn missing_key_returns_key() {
        let k = "non.existent.key";
        let s = t(k);
        assert_eq!(s, k.to_string());
    }

    #[test]
    fn test_is_supported_language() {
        assert!(is_supported_language("ru"));
        assert!(is_supported_language("en"));
        assert!(!is_supported_language("fr"));
    }

    #[test]
    fn test_normalize_language() {
        assert_eq!(normalize_language("en-US"), "en");
        assert_eq!(normalize_language("ru"), "ru");
        assert_eq!(normalize_language("EN-us"), "en");
    }
}
