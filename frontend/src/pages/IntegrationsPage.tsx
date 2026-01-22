import { useEffect, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { telegramApi, discordApi } from '@/lib/api';
import type { TelegramIntegration, DiscordIntegration } from '@/lib/api';
import { cn } from '@/lib/utils';
import { Button } from '@/components/ui/button';
import BotSettingsPage from '@/pages/BotSettingsPage';
import TelegramSettings from '@/components/telegram-settings';
import { useAuth } from '@/hooks/useAuth';
import { useSearchParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';

// Extracted components
import TelegramCard from '@/components/telegram-card';
import DiscordCard from '@/components/discord-card';
import AddTelegramDialog from '@/components/add-telegram-dialog';
import AddDiscordDialog from '@/components/add-discord-dialog';

import { MessageCircle, Plus, Loader2, Zap, MessageSquare } from 'lucide-react';

interface IntegrationsPageProps {
  defaultTab?: 'telegram' | 'discord' | 'bot';
}

// Tab button component
function TabButton({
  active,
  onClick,
  children,
  icon,
  count,
}: {
  active: boolean;
  onClick: () => void;
  children: React.ReactNode;
  icon: React.ReactNode;
  count?: number;
}) {
  return (
    <button
      onClick={onClick}
      className={cn(
        'flex items-center gap-2 px-4 py-2 rounded-lg font-medium transition-colors',
        active
          ? 'bg-primary text-primary-foreground'
          : 'text-muted-foreground hover:bg-muted hover:text-foreground',
      )}
    >
      {icon}
      {children}
      {count !== undefined && count > 0 && (
        <span
          className={cn(
            'ml-1 text-xs rounded-full px-2 py-0.5',
            active ? 'bg-primary-foreground/20' : 'bg-muted-foreground/20',
          )}
        >
          {count}
        </span>
      )}
    </button>
  );
}

// Toggle component

// Telegram card component

// Discord card component

// Add Telegram Dialog

// Add Discord Dialog

export function IntegrationsPage({ defaultTab = 'telegram' }: IntegrationsPageProps) {
  const { t } = useTranslation();
  const { user } = useAuth();
  const canAddTelegram = !!user?.telegram_user_id;
  const [searchParams, setSearchParams] = useSearchParams();
  const paramTab = searchParams.get('tab');
  const activeTab =
    paramTab === 'telegram' || paramTab === 'discord' || paramTab === 'bot' ? paramTab : defaultTab;
  const setActiveTab = (tTab: 'telegram' | 'discord' | 'bot') => {
    const params = new URLSearchParams(searchParams);
    params.set('tab', tTab);
    setSearchParams(params);
  };

  // Ensure the `tab` query param is always present so tab selection persists
  // even when it matches the default tab.
  useEffect(() => {
    const current = searchParams.get('tab');
    if (current !== 'telegram' && current !== 'discord' && current !== 'bot') {
      const params = new URLSearchParams(searchParams);
      params.set('tab', activeTab);
      // Use replace to avoid polluting history on initial load.
      setSearchParams(params, { replace: true });
    }
  }, [searchParams, activeTab, setSearchParams]);

  const [showAddTelegram, setShowAddTelegram] = useState(false);
  const [showAddDiscord, setShowAddDiscord] = useState(false);

  // Fetch integrations
  const { data: telegramIntegrations = [], isLoading: loadingTelegram } = useQuery<
    TelegramIntegration[]
  >({
    queryKey: ['telegram-integrations'],
    queryFn: () => telegramApi.list(),
  });

  const { data: discordIntegrations = [], isLoading: loadingDiscord } = useQuery<
    DiscordIntegration[]
  >({
    queryKey: ['discord-integrations'],
    queryFn: () => discordApi.list(),
  });

  const { data: inviteData, isLoading: inviteLoading } = useQuery({
    queryKey: ['discord-invite'],
    queryFn: discordApi.getInvite,
    retry: false,
  });

  const isLoading = loadingTelegram || loadingDiscord;

  return (
    <div className="space-y-6">
      {/* Header */}
      <div>
        <h1 className="text-2xl font-bold">{t('integrations_page.title')}</h1>
        <p className="text-muted-foreground">{t('integrations_page.subtitle')}</p>
      </div>

      {/* Tabs */}
      <div className="flex items-center gap-2">
        <TabButton
          active={activeTab === 'telegram'}
          onClick={() => setActiveTab('telegram')}
          icon={<MessageCircle className="h-4 w-4" />}
          count={telegramIntegrations?.length}
        >
          {t('integrations_page.tabs.telegram')}
        </TabButton>
        <TabButton
          active={activeTab === 'discord'}
          onClick={() => setActiveTab('discord')}
          icon={<Zap className="h-4 w-4" />}
          count={discordIntegrations?.length}
        >
          {t('integrations_page.tabs.discord')}
        </TabButton>
        <TabButton
          active={activeTab === 'bot'}
          onClick={() => setActiveTab('bot')}
          icon={<MessageSquare className="h-4 w-4" />}
        >
          {t('integrations_page.tabs.bot')}
        </TabButton>
      </div>

      {/* Content */}
      {isLoading ? (
        <div className="flex items-center justify-center min-h-[200px]">
          <Loader2 className="h-8 w-8 animate-spin text-twitch" />
        </div>
      ) : (
        <>
          {/* Telegram Tab */}
          {activeTab === 'telegram' && (
            <div className="space-y-4">
              {!canAddTelegram && (
                <div className="mb-4">
                  <TelegramSettings redirectTo="/integrations?tab=telegram" />
                </div>
              )}
              <div className="flex justify-end">
                {canAddTelegram && telegramIntegrations?.length > 0 && (
                  <Button onClick={() => setShowAddTelegram(true)}>
                    <Plus className="h-4 w-4 mr-2" />
                    {t('integrations_page.telegram.add')}
                  </Button>
                )}
              </div>

              {telegramIntegrations?.length > 0 ? (
                <div className="grid gap-4 md:grid-cols-2">
                  {telegramIntegrations.map((integration) => (
                    <TelegramCard key={integration.id} integration={integration} />
                  ))}
                </div>
              ) : canAddTelegram ? (
                <div className="rounded-lg border border-dashed p-8 text-center">
                  <MessageCircle className="mx-auto h-12 w-12 text-muted-foreground/50" />
                  <h3 className="mt-4 text-lg font-semibold">
                    {t('integrations_page.telegram.no_integrations')}
                  </h3>
                  <p className="mt-2 text-sm text-muted-foreground">
                    {t('integrations_page.telegram.add_prompt')}
                  </p>
                  <Button className="mt-4" onClick={() => setShowAddTelegram(true)}>
                    <Plus className="h-4 w-4 mr-2" />
                    {t('integrations_page.telegram.add_button')}
                  </Button>
                </div>
              ) : null}
            </div>
          )}

          {/* Discord Tab */}
          {activeTab === 'discord' && (
            <div className="space-y-4">
              <div className="flex justify-end items-center gap-2">
                {inviteData?.invite_url ? (
                  <Button asChild>
                    <a href={inviteData.invite_url} target="_blank" rel="noopener noreferrer">
                      {t('integrations_page.discord.invite_bot')}
                    </a>
                  </Button>
                ) : inviteLoading ? (
                  <Button variant="ghost" disabled>
                    <Loader2 className="h-4 w-4 animate-spin mr-2" />
                    {t('integrations_page.discord.loading')}
                  </Button>
                ) : (
                  <Button variant="ghost" disabled>
                    {t('integrations_page.discord.bot_not_configured')}
                  </Button>
                )}
                {discordIntegrations?.length > 0 && (
                  <Button onClick={() => setShowAddDiscord(true)}>
                    <Plus className="h-4 w-4 mr-2" />
                    {t('integrations_page.discord.add')}
                  </Button>
                )}
              </div>

              {discordIntegrations?.length > 0 ? (
                <div className="grid gap-4 md:grid-cols-2">
                  {discordIntegrations.map((integration) => (
                    <DiscordCard key={integration.id} integration={integration} />
                  ))}
                </div>
              ) : (
                <div className="rounded-lg border border-dashed p-8 text-center">
                  <Zap className="mx-auto h-12 w-12 text-muted-foreground/50" />
                  <h3 className="mt-4 text-lg font-semibold">
                    {t('integrations_page.discord.no_integrations')}
                  </h3>
                  <p className="mt-2 text-sm text-muted-foreground">
                    {t('integrations_page.discord.add_prompt')}
                  </p>
                  <Button className="mt-4" onClick={() => setShowAddDiscord(true)}>
                    <Plus className="h-4 w-4 mr-2" />
                    {t('integrations_page.discord.add')}
                  </Button>
                </div>
              )}
            </div>
          )}

          {/* Bot Tab */}
          {activeTab === 'bot' && (
            <div className="space-y-4">
              <BotSettingsPage showNotice={false} />
            </div>
          )}
        </>
      )}

      {/* Dialogs */}
      {canAddTelegram && (
        <AddTelegramDialog open={showAddTelegram} onClose={() => setShowAddTelegram(false)} />
      )}
      <AddDiscordDialog open={showAddDiscord} onClose={() => setShowAddDiscord(false)} />
    </div>
  );
}
