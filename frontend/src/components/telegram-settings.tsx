import { useEffect, useRef, useState } from 'react';
import { useLocation } from 'react-router-dom';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { telegramApi, TelegramBotInfo, authApi } from '@/lib/api';
import { getApiUrl } from '@/lib/utils';
import { useAuth } from '@/hooks/useAuth';
import { Button } from '@/components/ui/button';
import { Loader2 } from 'lucide-react';
import { alert as showAlert, confirm as showConfirm } from '@/lib/dialog';
import { useTranslation } from 'react-i18next';

/**
 * TelegramSettings
 *
 * By default this component behaves as before (used on /integrations and other pages).
 * On the main user settings page (/settings) it renders a combined "Integrations" block
 * containing both Telegram and Discord connection cards so they are presented together.
 *
 * Notes:
 * - The Telegram Login Widget will redirect to `/integrations/telegram/callback`
 *   which should handle linking on the backend and then redirect back to
 *   `/settings` (via the `redirect_to` query parameter).
 * - After unlinking we refresh the user and integration lists.
 */
export default function TelegramSettings({ redirectTo = '/settings' }: { redirectTo?: string }) {
  const { user, refreshUser } = useAuth();
  const { t } = useTranslation();
  const queryClient = useQueryClient();

  const {
    data: botInfo,
    isLoading: loadingBot,
    isError,
  } = useQuery<TelegramBotInfo | undefined>({
    queryKey: ['telegram-bot-info'],
    queryFn: telegramApi.getBotInfo,
    retry: false,
  });

  const widgetRef = useRef<HTMLDivElement | null>(null);
  const blockRef = useRef<HTMLDivElement | null>(null);
  const [isUnlinking, setIsUnlinking] = useState(false);

  // Discord-related local state for the combined block
  const [isDiscordUnlinking, setIsDiscordUnlinking] = useState(false);
  const [isDiscordLinking, setIsDiscordLinking] = useState(false);

  // Detect whether we are on the main /settings page (show combined block there)
  const location = useLocation();
  const pathname =
    location?.pathname ?? (typeof window !== 'undefined' ? window.location.pathname : '');
  const isSettingsPage = pathname.replace(/\/+$/, '') === '/settings';

  // Inject Telegram Login Widget script when bot info is available.
  useEffect(() => {
    if (!botInfo || !widgetRef.current) return;
    const el = widgetRef.current;
    // Clear any previous widget
    el.innerHTML = '';

    const script = document.createElement('script');
    script.src = 'https://telegram.org/js/telegram-widget.js?23';
    const login = botInfo?.username?.replace(/^@/, '') ?? '';
    script.setAttribute('data-telegram-login', login);
    script.setAttribute('data-size', 'large');
    script.setAttribute('data-userpic', 'true');
    script.setAttribute('data-radius', '10');
    const lang =
      typeof navigator !== 'undefined' && navigator.language
        ? navigator.language.split('-')[0]
        : 'ru';
    script.setAttribute('data-lang', lang);

    // Telegram Login Widget redirects the browser with GET + query params; the backend
    // /api/auth/telegram/link endpoint only accepts POST. So send the user to the frontend
    // callback page, which will POST the payload to the backend with the auth token and
    // then redirect to redirectTo.
    const frontendOrigin = typeof window !== 'undefined' ? window.location.origin : getApiUrl();
    const authUrl = `${frontendOrigin}/integrations/telegram/callback?redirect_to=${encodeURIComponent(
      redirectTo,
    )}`;
    script.setAttribute('data-auth-url', authUrl);
    script.async = true;
    el.appendChild(script);

    return () => {
      el.innerHTML = '';
    };
  }, [botInfo, redirectTo, user?.telegram_user_id]);

  // Telegram fields live on the authenticated user profile.
  const telegramUserId = user?.telegram_user_id;
  const telegramUsername = user?.telegram_username;
  const telegramPhoto = user?.telegram_photo_url;

  // Discord fields (for the combined block)
  const discordUserId = user?.discord_user_id;
  const discordUsername = user?.discord_username;
  const discordAvatar = user?.discord_avatar_url;

  const handleUnlink = async () => {
    if (!(await showConfirm(t('telegram.integrations_block.disconnect_confirm')))) return;

    setIsUnlinking(true);
    try {
      await authApi.unlinkTelegram();

      // Refresh local user and integrations lists
      await refreshUser();
      queryClient.invalidateQueries({ queryKey: ['telegram-integrations'] });
      queryClient.invalidateQueries({ queryKey: ['telegram-bot-info'] });
    } catch (err) {
      let message = t('telegram.integrations_block.unlink_error', { service: 'Telegram' });
      if (typeof err === 'string') {
        message = err;
      } else if (err && typeof err === 'object') {
        const e = err as Record<string, unknown>;
        if (typeof e['message'] === 'string') {
          message = e['message'] as string;
        } else if (typeof e['error'] === 'string') {
          message = e['error'] as string;
        }
      }
      await showAlert(message);
    } finally {
      setIsUnlinking(false);
    }
  };

  const handleDiscordLink = async () => {
    if (isDiscordLinking) return;
    setIsDiscordLinking(true);
    try {
      const { url } = await authApi.getDiscordAuthUrl(redirectTo);
      if (!url) {
        throw new Error(t('integrations_page.discord.failed_servers'));
      }
      window.location.href = url;
    } catch (err) {
      let message = t('telegram.integrations_block.link_error', { service: 'Discord' });
      if (typeof err === 'string') {
        message = err;
      } else if (err && typeof err === 'object') {
        const e = err as Record<string, unknown>;
        if (typeof e['message'] === 'string') {
          message = e['message'] as string;
        } else if (typeof e['error'] === 'string') {
          message = e['error'] as string;
        }
      }
      await showAlert(message);
    } finally {
      setIsDiscordLinking(false);
    }
  };

  const handleDiscordUnlink = async () => {
    if (!(await showConfirm(t('telegram.integrations_block.discord_disconnect_confirm')))) return;

    setIsDiscordUnlinking(true);
    try {
      await authApi.unlinkDiscord();

      // Refresh local user and integrations lists
      await refreshUser();
      queryClient.invalidateQueries({ queryKey: ['discord-integrations'] });
    } catch (err) {
      let message = t('telegram.integrations_block.unlink_error', { service: 'Discord' });
      if (typeof err === 'string') {
        message = err;
      } else if (err && typeof err === 'object') {
        const e = err as Record<string, unknown>;
        if (typeof e['message'] === 'string') {
          message = e['message'] as string;
        } else if (typeof e['error'] === 'string') {
          message = e['error'] as string;
        }
      }
      await showAlert(message);
    } finally {
      setIsDiscordUnlinking(false);
    }
  };

  // When rendering the combined view on /settings hide the standalone Discord block
  // (which is also rendered by <DiscordSettings /> on the page) to avoid duplication.
  useEffect(() => {
    if (!isSettingsPage) return;

    const modified: HTMLElement[] = [];
    const timer = setTimeout(() => {
      const discordHeaders = Array.from(document.querySelectorAll('h2')).filter(
        (h) => h.textContent?.trim() === t('integrations_page.tabs.discord'),
      );
      discordHeaders.forEach((h) => {
        const section = h.closest('section') as HTMLElement | null;
        if (section && !blockRef.current?.contains(section)) {
          modified.push(section);
          section.style.display = 'none';
        }
      });
    }, 50);

    return () => {
      clearTimeout(timer);
      modified.forEach((s) => {
        s.style.display = '';
      });
    };
    // We intentionally only run this on initial mount/unmount for the settings page
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isSettingsPage]);

  // If this is the main settings page, present a single combined "Integrations" block.
  if (isSettingsPage) {
    return (
      <section ref={blockRef} className="space-y-4">
        <div className="flex items-center justify-between">
          <div>
            <h2 className="text-xl font-semibold">{t('telegram.integrations_block.title')}</h2>
            <p className="text-sm text-muted-foreground">
              {t('telegram.integrations_block.subtitle')}
            </p>
          </div>
        </div>

        <div className="grid gap-4 md:grid-cols-2">
          {/* Telegram column */}
          <div>
            {telegramUserId ? (
              <div className="rounded-md border p-4">
                <div className="flex items-center gap-4">
                  {telegramPhoto ? (
                    <img
                      src={telegramPhoto}
                      alt={t('telegram.logo_alt')}
                      className="h-10 w-10 rounded-full"
                    />
                  ) : null}

                  <div>
                    <div className="font-medium">
                      {telegramUsername ? (
                        <>
                          {t('telegram.integrations_block.connected_as', {
                            username: `@${telegramUsername}`,
                          })}
                        </>
                      ) : (
                        <>{t('telegram.integrations_block.connected_id', { id: telegramUserId })}</>
                      )}
                    </div>
                    <div className="text-sm text-muted-foreground">
                      {t('telegram.integrations_block.telegram_id', { id: telegramUserId })}
                    </div>
                  </div>

                  <div className="ml-auto">
                    <Button
                      variant="ghost"
                      onClick={handleUnlink}
                      disabled={isUnlinking}
                      className="text-destructive"
                    >
                      {isUnlinking ? <Loader2 className="h-4 w-4 mr-2 animate-spin" /> : null}
                      {t('telegram.integrations_block.disconnect')}
                    </Button>
                  </div>
                </div>
              </div>
            ) : (
              <div className="rounded-md border p-4">
                {loadingBot ? (
                  <div className="text-sm text-muted-foreground">
                    {t('telegram.integrations_block.loading_bot')}
                  </div>
                ) : isError || !botInfo ? (
                  <div className="text-sm text-muted-foreground">
                    {t('telegram.integrations_block.bot_not_configured')}
                  </div>
                ) : (
                  <div>
                    <div className="mb-3 text-sm text-muted-foreground">
                      {t('telegram.integrations_block.connect_prompt')}
                    </div>
                    <div ref={widgetRef}></div>
                  </div>
                )}
              </div>
            )}
          </div>

          {/* Discord column */}
          <div>
            {discordUserId ? (
              <div className="rounded-md border p-4">
                <div className="flex items-center gap-4">
                  {discordAvatar ? (
                    <img
                      src={discordAvatar}
                      alt={t('discord.logo_alt')}
                      className="h-10 w-10 rounded-full"
                    />
                  ) : null}

                  <div>
                    <div className="font-medium">
                      {discordUsername ? (
                        <>
                          {t('telegram.integrations_block.connected_as', {
                            username: discordUsername,
                          })}
                        </>
                      ) : (
                        <>{t('telegram.integrations_block.connected_id', { id: discordUserId })}</>
                      )}
                    </div>
                    <div className="text-sm text-muted-foreground">
                      {t('telegram.integrations_block.discord_id', { id: discordUserId })}
                    </div>
                  </div>

                  <div className="ml-auto">
                    <Button
                      variant="ghost"
                      onClick={handleDiscordUnlink}
                      disabled={isDiscordUnlinking}
                      className="text-destructive"
                    >
                      {isDiscordUnlinking ? (
                        <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                      ) : null}
                      {t('telegram.integrations_block.disconnect')}
                    </Button>
                  </div>
                </div>
              </div>
            ) : (
              <div className="rounded-md border p-4">
                <div className="mb-3 text-sm text-muted-foreground">
                  {t('telegram.integrations_block.connect_discord_prompt')}
                </div>
                <div>
                  <Button onClick={() => handleDiscordLink()} disabled={isDiscordLinking}>
                    {isDiscordLinking ? <Loader2 className="h-4 w-4 mr-2 animate-spin" /> : null}
                    {t('telegram.integrations_block.connect_discord')}
                  </Button>
                </div>
              </div>
            )}
          </div>
        </div>
      </section>
    );
  }

  // Fallback: behave exactly as before when not on /settings (keeps Integrations page behavior unchanged)
  return (
    <section className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-xl font-semibold">{t('telegram.integrations_block.title')}</h2>
          <p className="text-sm text-muted-foreground">
            {t('telegram.integrations_block.connect_prompt')}
          </p>
        </div>
      </div>

      {telegramUserId ? (
        <div className="rounded-md border p-4">
          <div className="flex items-center gap-4">
            {telegramPhoto ? (
              <img
                src={telegramPhoto}
                alt={t('telegram.logo_alt')}
                className="h-10 w-10 rounded-full"
              />
            ) : null}

            <div>
              <div className="font-medium">
                {telegramUsername ? (
                  <>
                    {t('telegram.integrations_block.connected_as', {
                      username: (
                        <a
                          className="text-twitch hover:underline"
                          href={`https://t.me/${telegramUsername}`}
                          target="_blank"
                          rel="noopener noreferrer"
                        >
                          @{telegramUsername}
                        </a>
                      ),
                    })}
                  </>
                ) : (
                  <>{t('telegram.integrations_block.connected_id', { id: telegramUserId })}</>
                )}
              </div>
              <div className="text-sm text-muted-foreground">
                {t('telegram.integrations_block.telegram_id', { id: telegramUserId })}
              </div>
            </div>

            <div className="ml-auto">
              <Button
                variant="ghost"
                onClick={handleUnlink}
                disabled={isUnlinking}
                className="text-destructive"
              >
                {isUnlinking ? <Loader2 className="h-4 w-4 mr-2 animate-spin" /> : null}
                {t('telegram.integrations_block.disconnect')}
              </Button>
            </div>
          </div>
        </div>
      ) : (
        <div className="rounded-md border p-4">
          {loadingBot ? (
            <div className="text-sm text-muted-foreground">
              {t('telegram.integrations_block.loading_bot')}
            </div>
          ) : isError || !botInfo ? (
            <div className="text-sm text-muted-foreground">
              {t('telegram.integrations_block.bot_not_configured')}
            </div>
          ) : (
            <div>
              <div className="mb-3 text-sm text-muted-foreground">
                {t('telegram.integrations_block.connect_prompt')}
              </div>
              <div ref={widgetRef}></div>
            </div>
          )}
        </div>
      )}
    </section>
  );
}
