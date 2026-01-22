import { useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import {
  telegramApi,
  discordApi,
  type TelegramIntegration,
  type DiscordIntegration,
} from '@/lib/api';
import TelegramCard from '@/components/telegram-card';
import DiscordCard from '@/components/discord-card';
import AddTelegramDialog from '@/components/add-telegram-dialog';
import AddDiscordDialog from '@/components/add-discord-dialog';
import { Button } from '@/components/ui/button';
import { useTranslation } from 'react-i18next';
import { MessageCircle, Zap, Plus, Loader2 } from 'lucide-react';
import { useAuth } from '@/hooks/useAuth';
import { Link } from 'react-router-dom';

/**
 * UserIntegrationsBlock
 *
 * Reusable block that lists Telegram and Discord integrations for either the
 * current user (ownerId undefined) or a specific owner (ownerId provided).
 *
 * Props:
 * - ownerId?: string  — when provided, operations are executed on behalf of that owner.
 * - canManage?: boolean — whether current user has manage rights for the owner.
 */
export default function UserIntegrationsBlock({
  ownerId,
  canManage = true,
}: {
  ownerId?: string;
  canManage?: boolean;
}) {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  const telegramKey = ownerId ? ['telegram-integrations', ownerId] : ['telegram-integrations'];
  const discordKey = ownerId ? ['discord-integrations', ownerId] : ['discord-integrations'];
  const { user } = useAuth();
  // Only allow adding Telegram integrations on the client when managing the current
  // user and they've linked their Telegram account. For owner pages (ownerId provided)
  // we can't reliably know the owner's linked status here, so allow the button and
  // rely on the backend validation for that case.
  const canAddTelegram = ownerId ? true : !!user?.telegram_user_id;

  const { data: telegramIntegrations, isFetching: fetchingTelegram } = useQuery<
    TelegramIntegration[]
  >({
    queryKey: telegramKey,
    queryFn: () => telegramApi.list(ownerId),
    retry: false,
  });

  const { data: discordIntegrations, isFetching: fetchingDiscord } = useQuery<DiscordIntegration[]>(
    {
      queryKey: discordKey,
      queryFn: () => discordApi.list(ownerId),
      retry: false,
    },
  );

  const [showAddTelegram, setShowAddTelegram] = useState(false);
  const [showAddDiscord, setShowAddDiscord] = useState(false);

  const refreshAll = () => {
    queryClient.invalidateQueries({ queryKey: telegramKey });
    queryClient.invalidateQueries({ queryKey: discordKey });
    // Also refresh global lists
    queryClient.invalidateQueries({ queryKey: ['telegram-integrations'] });
    queryClient.invalidateQueries({ queryKey: ['discord-integrations'] });
  };

  return (
    <section className="space-y-6">
      {/* Telegram */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <MessageCircle className="h-5 w-5 text-blue-500" />
          <h3 className="text-lg font-semibold">{t('integrations_page.tabs.telegram')}</h3>
        </div>

        {canManage ? (
          <div>
            <Button onClick={() => setShowAddTelegram(true)} disabled={!canAddTelegram}>
              <Plus className="h-4 w-4 mr-2" />
              {t('integrations_page.telegram.add')}
            </Button>
            {!ownerId && !user?.telegram_user_id ? (
              <p className="mt-1 text-xs text-muted-foreground">
                {t('integrations_page.telegram.add_prompt')}{' '}
                <Link to="/settings">{t('user_settings.language')}</Link>
              </p>
            ) : null}
          </div>
        ) : null}
      </div>

      {fetchingTelegram ? (
        <div className="py-6 flex items-center justify-center">
          <Loader2 className="h-6 w-6 animate-spin text-twitch" />
        </div>
      ) : telegramIntegrations && telegramIntegrations.length > 0 ? (
        <div className="grid gap-4 md:grid-cols-2">
          {telegramIntegrations.map((t) => (
            <TelegramCard key={t.id} integration={t} canManage={canManage} ownerId={ownerId} />
          ))}
        </div>
      ) : (
        <div className="rounded-lg border border-dashed p-8 text-center">
          <MessageCircle className="mx-auto h-12 w-12 text-muted-foreground/50" />
          <h4 className="mt-4 text-lg font-semibold">
            {t('integrations_page.telegram.no_integrations')}
          </h4>
          <p className="mt-2 text-sm text-muted-foreground">
            {canManage
              ? t('integrations_page.telegram.add_prompt')
              : t('integrations_page.telegram.no_integrations')}
          </p>
        </div>
      )}

      <AddTelegramDialog
        open={showAddTelegram}
        onClose={() => {
          setShowAddTelegram(false);
          refreshAll();
        }}
        ownerId={ownerId}
      />

      {/* Discord */}
      <div className="flex items-center justify-between mt-6">
        <div className="flex items-center gap-2">
          <Zap className="h-5 w-5 text-indigo-500" />
          <h3 className="text-lg font-semibold">{t('integrations_page.tabs.discord')}</h3>
        </div>

        {canManage ? (
          <div>
            <Button onClick={() => setShowAddDiscord(true)}>
              <Plus className="h-4 w-4 mr-2" />
              {t('integrations_page.discord.add')}
            </Button>
          </div>
        ) : null}
      </div>

      {fetchingDiscord ? (
        <div className="py-6 flex items-center justify-center">
          <Loader2 className="h-6 w-6 animate-spin text-twitch" />
        </div>
      ) : discordIntegrations && discordIntegrations.length > 0 ? (
        <div className="grid gap-4 md:grid-cols-2">
          {discordIntegrations.map((d) => (
            <DiscordCard key={d.id} integration={d} canManage={canManage} ownerId={ownerId} />
          ))}
        </div>
      ) : (
        <div className="rounded-lg border border-dashed p-8 text-center">
          <Zap className="mx-auto h-12 w-12 text-muted-foreground/50" />
          <h4 className="mt-4 text-lg font-semibold">
            {t('integrations_page.discord.no_integrations')}
          </h4>
          <p className="mt-2 text-sm text-muted-foreground">
            {canManage
              ? t('integrations_page.discord.add_prompt')
              : t('integrations_page.discord.no_integrations')}
          </p>
        </div>
      )}

      <AddDiscordDialog
        open={showAddDiscord}
        onClose={() => {
          setShowAddDiscord(false);
          refreshAll();
        }}
        ownerId={ownerId}
      />
    </section>
  );
}
