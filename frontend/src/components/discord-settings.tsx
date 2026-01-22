import { useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { authApi } from '@/lib/api';
import { Button } from '@/components/ui/button';
import { Loader2 } from 'lucide-react';
import { useAuth } from '@/hooks/useAuth';
import { alert as showAlert, confirm as showConfirm } from '@/lib/dialog';
import { useTranslation } from 'react-i18next';

/**
 * DiscordSettings
 *
 * Displays Discord connection status for the current user and provides
 * buttons to start the OAuth linking flow or unlink the account.
 *
 * Notes:
 * - Linking is performed by redirecting the browser to the backend `/api/auth/discord/link`
 *   endpoint (which starts the OAuth flow). The backend will handle the callback and
 *   redirect back to the frontend afterward.
 * - After unlinking we refresh the user and invalidate the Discord integrations list.
 */
export default function DiscordSettings({ redirectTo = '/settings' }: { redirectTo?: string }) {
  const { user, refreshUser } = useAuth();
  const { t } = useTranslation();
  const queryClient = useQueryClient();

  const [isUnlinking, setIsUnlinking] = useState(false);
  const [isLinking, setIsLinking] = useState(false);

  const discordUserId = user?.discord_user_id;
  const discordUsername = user?.discord_username;
  const discordAvatar = user?.discord_avatar_url;

  const handleLink = async () => {
    if (isLinking) return;
    setIsLinking(true);
    try {
      // Request the server to generate a signed Discord OAuth URL for this user (authenticated request)
      const { url } = await authApi.getDiscordAuthUrl(redirectTo);
      if (!url) {
        throw new Error(t('telegram.integrations_block.link_error', { service: 'Discord' }));
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
      setIsLinking(false);
    }
  };

  const handleUnlink = async () => {
    if (!(await showConfirm(t('telegram.integrations_block.discord_disconnect_confirm')))) return;

    setIsUnlinking(true);
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
      setIsUnlinking(false);
    }
  };

  return (
    <section className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-xl font-semibold">{t('integrations_page.tabs.discord')}</h2>
          <p className="text-sm text-muted-foreground">
            {t('telegram.integrations_block.subtitle')}
          </p>
        </div>

        {/* Removed update button per request */}
      </div>

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
                {discordUsername
                  ? t('telegram.integrations_block.connected_as', { username: discordUsername })
                  : t('telegram.integrations_block.connected_id', { id: discordUserId })}
              </div>
              <div className="text-sm text-muted-foreground">
                {t('telegram.integrations_block.discord_id', { id: discordUserId })}
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
          <div className="mb-3 text-sm text-muted-foreground">
            {t('telegram.integrations_block.connect_discord_prompt')}
          </div>
          <div>
            <Button onClick={() => handleLink()} disabled={isLinking}>
              {isLinking ? <Loader2 className="h-4 w-4 mr-2 animate-spin" /> : null}
              {t('telegram.integrations_block.connect_discord')}
            </Button>
          </div>
        </div>
      )}
    </section>
  );
}
