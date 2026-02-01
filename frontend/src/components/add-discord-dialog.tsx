import { useState } from 'react';
import { useForm } from '@/lib/useForm';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { discordApi, authApi } from '@/lib/api';
import type { DiscordChannel } from '@/lib/api';

import { Button } from '@/components/ui/button';
import { alert as showAlert } from '@/lib/dialog';
import { Loader2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';

type Props = {
  open: boolean;
  onClose: () => void;
  /**
   * Optional ownerId — when provided, the integration will be created on behalf of that user.
   */
  ownerId?: string;
};

export default function AddDiscordDialog({ open, onClose, ownerId }: Props) {
  const form = useForm<{
    guildId: string;
    guildName: string;
    channelId: string;
    webhookUrl: string;
  }>({
    defaultValues: { guildId: '', guildName: '', channelId: '', webhookUrl: '' },
  });
  const setValue = form.setValue?.bind(form) ?? (() => {});
  const getValues =
    form.getValues?.bind(form) ??
    (() => ({}) as { guildId: string; guildName: string; channelId: string; webhookUrl: string });
  const reset = form.reset?.bind(form) ?? (() => {});
  const guildId = getValues().guildId ?? '';
  const guildName = getValues().guildName ?? '';
  const channelId = getValues().channelId ?? '';
  const webhookUrl = getValues().webhookUrl ?? '';
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  const {
    data: sharedGuilds,
    isLoading: isLoadingGuilds,
    isError: guildsError,
  } = useQuery({
    queryKey: ['discord-shared-guilds'],
    queryFn: discordApi.listSharedGuilds,
    enabled: open,
  });

  const { data: channels, isLoading: isLoadingChannels } = useQuery<DiscordChannel[], unknown>({
    queryKey: ['discord-channels', guildId],
    queryFn: () => (guildId ? discordApi.listChannels(guildId) : Promise.resolve([])),
    enabled: !!guildId,
  });

  // manual channel lookup removed; channel must be selected from the server's channels

  const linkDiscord = async () => {
    try {
      const resp = await authApi.getDiscordAuthUrl();
      window.location.href = resp.url;
    } catch {
      await showAlert(t('discord.add_dialog.failed_servers'));
    }
  };

  const handleSubmit = async (e?: React.FormEvent) => {
    e?.preventDefault();
    // clear previous error for fresh attempt
    setErrorMessage(null);
    if (!guildId.trim() || !channelId.trim()) return;

    setIsSubmitting(true);
    try {
      const selectedChannelName = channels?.find((c: DiscordChannel) => c.id === channelId)?.name;

      await discordApi.create(
        {
          discord_guild_id: guildId.trim(),
          discord_channel_id: channelId.trim(),
          discord_guild_name: guildName.trim() || undefined,
          discord_channel_name: selectedChannelName?.trim() || undefined,
          discord_webhook_url: webhookUrl.trim() || undefined,
        },
        ownerId,
      );

      // Refresh global and owner-scoped lists
      queryClient.invalidateQueries({ queryKey: ['discord-integrations'] });
      if (ownerId) queryClient.invalidateQueries({ queryKey: ['discord-integrations', ownerId] });

      onClose();
      reset({ guildId: '', guildName: '', channelId: '', webhookUrl: '' });
      // clear any previous error on success
      setErrorMessage(null);
    } catch (err: unknown) {
      // Prefer a detailed message from the API response (server returns { error: { code, message, details } })
      console.error('Failed to create discord integration:', err);

      let msg = t('discord.add_dialog.create_error');

      if (err && typeof err === 'object' && err !== null) {
        const e = err as Record<string, unknown>;

        // Top-level message if present
        const topMessage = typeof e.message === 'string' ? e.message : undefined;

        // Check nested structure: error -> details -> message
        let detailsMessage: string | undefined;

        if (e.error && typeof e.error === 'object' && e.error !== null) {
          const errObj = e.error as Record<string, unknown>;

          if (errObj.details && typeof errObj.details === 'object' && errObj.details !== null) {
            const detailsObj = errObj.details as Record<string, unknown>;
            if (typeof detailsObj.message === 'string') {
              detailsMessage = detailsObj.message;
            }
          }

          if (!detailsMessage && typeof errObj.message === 'string') {
            detailsMessage = errObj.message;
          }
        }

        msg = detailsMessage ?? topMessage ?? msg;
      }

      setErrorMessage(msg);
    } finally {
      setIsSubmitting(false);
    }
  };

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="w-full max-w-md rounded-lg bg-background p-6 shadow-lg">
        <h2 className="text-lg font-semibold mb-4">{t('discord.add_dialog.title')}</h2>
        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label className="text-sm font-medium">{t('discord.add_dialog.server_label')}</label>

            {isLoadingGuilds ? (
              <div className="mt-1 text-sm text-muted-foreground">
                {t('discord.add_dialog.loading_servers')}
              </div>
            ) : guildsError ? (
              <div className="mt-2 space-y-2 text-sm text-muted-foreground">
                <div>{t('discord.add_dialog.failed_servers')}</div>
                <div>
                  <button
                    type="button"
                    onClick={linkDiscord}
                    className="mt-1 text-sm underline text-primary"
                  >
                    {t('discord.add_dialog.link_discord')}
                  </button>
                </div>
              </div>
            ) : sharedGuilds && sharedGuilds.length > 0 ? (
              <select
                value={guildId}
                onChange={(e) => {
                  const selected = sharedGuilds.find((g) => g.id === e.target.value);
                  setValue('guildId', e.target.value);
                  setValue('guildName', selected?.name || '');
                  // reset channel selection when guild changes
                  setValue('channelId', '');
                }}
                className="mt-1 w-full rounded-md border bg-background px-3 py-2 text-sm"
                required
              >
                <option value="">{t('discord.add_dialog.select_server')}</option>
                {sharedGuilds.map((g) => (
                  <option key={g.id} value={g.id}>
                    {g.name}
                  </option>
                ))}
              </select>
            ) : (
              <div className="mt-2 text-sm text-muted-foreground">
                {t('discord.add_dialog.failed_servers')}
              </div>
            )}
          </div>

          <div>
            <label className="text-sm font-medium">{t('discord.add_dialog.channel_label')}</label>

            <select
              value={channelId}
              onChange={(e) => setValue('channelId', e.target.value)}
              className="mt-1 w-full rounded-md border bg-background px-3 py-2 text-sm"
              required
              disabled={!guildId || isLoadingChannels || !(channels && channels.length > 0)}
            >
              {!guildId ? (
                <option value="">{t('discord.add_dialog.select_channel_first')}</option>
              ) : isLoadingChannels ? (
                <option value="">{t('discord.add_dialog.loading_channels')}</option>
              ) : channels && channels.length > 0 ? (
                <>
                  <option value="">{t('discord.add_dialog.select_server')}</option>
                  {channels.map((c) => (
                    <option key={c.id} value={c.id}>
                      {c.name}
                    </option>
                  ))}
                </>
              ) : (
                <option value="">{t('discord.add_dialog.no_text_channels')}</option>
              )}
            </select>
          </div>

          {/* Channel name is derived from selected channel; no manual input */}

          <div>
            <label className="text-sm font-medium">{t('discord.add_dialog.webhook_label')}</label>
            <input
              type="url"
              value={webhookUrl}
              onChange={(e) => setValue('webhookUrl', e.target.value)}
              placeholder={t('discord.add_dialog.webhook_example')}
              className="mt-1 w-full rounded-md border bg-background px-3 py-2 text-sm"
            />
            <p className="mt-1 text-xs text-muted-foreground">
              {t('discord.add_dialog.webhook_hint')}
            </p>
          </div>

          {errorMessage ? <div className="text-sm text-red-500">{errorMessage}</div> : null}
          <div className="flex justify-end gap-2">
            <Button
              type="button"
              variant="ghost"
              onClick={() => {
                // Clear local error when cancelling
                setErrorMessage(null);
                onClose();
              }}
            >
              {t('discord.add_dialog.cancel')}
            </Button>
            <Button type="submit" disabled={isSubmitting || !guildId.trim() || !channelId.trim()}>
              {isSubmitting ? <Loader2 className="h-4 w-4 animate-spin mr-2" /> : null}
              {t('discord.add_dialog.add')}
            </Button>
          </div>
        </form>
      </div>
    </div>
  );
}
