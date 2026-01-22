import { useQuery } from '@tanstack/react-query';
import { Link } from 'react-router-dom';
import { useAuth } from '@/hooks/useAuth';
import { telegramApi, discordApi, notificationsApi } from '@/lib/api';
import { cn, getTwitchProfileImageUrl, getTwitchChannelUrl, formatRelativeTime } from '@/lib/utils';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import {
  Bell,
  CheckCircle,
  ExternalLink,
  MessageCircle,
  Plus,
  XCircle,
  Zap,
  Activity,
  TrendingUp,
} from 'lucide-react';

// Stats card component
function StatCard({
  title,
  value,
  icon,
  description,
  trend,
}: {
  title: string;
  value: string | number;
  icon: React.ReactNode;
  description?: string;
  trend?: { value: number; positive: boolean };
}) {
  return (
    <div className="rounded-lg border bg-card p-6 shadow-sm">
      <div className="flex items-center justify-between">
        <div className="text-muted-foreground">{icon}</div>
        {trend && (
          <div
            className={cn(
              'flex items-center gap-1 text-sm',
              trend.positive ? 'text-green-500' : 'text-red-500',
            )}
          >
            <TrendingUp className={cn('h-4 w-4', !trend.positive && 'rotate-180')} />
            {trend.value}%
          </div>
        )}
      </div>
      <div className="mt-4">
        <p className="text-3xl font-bold">{value}</p>
        <p className="text-sm text-muted-foreground">{title}</p>
        {description && <p className="mt-1 text-xs text-muted-foreground">{description}</p>}
      </div>
    </div>
  );
}

// Integration status component
function IntegrationStatus({
  type,
  count,
  enabled,
}: {
  type: 'telegram' | 'discord';
  count: number;
  enabled: number;
}) {
  const { t } = useTranslation();
  const icons = {
    telegram: <MessageCircle className="h-5 w-5" />,
    discord: <Zap className="h-5 w-5" />,
  };

  const colors = {
    telegram: 'bg-blue-500/10 text-blue-500',
    discord: 'bg-indigo-500/10 text-indigo-500',
  };

  const labels = {
    telegram: t('integrations_page.tabs.telegram'),
    discord: t('integrations_page.tabs.discord'),
  };

  return (
    <div className="flex items-center justify-between rounded-lg border p-4">
      <div className="flex items-center gap-3">
        <div className={cn('rounded-lg p-2', colors[type])}>{icons[type]}</div>
        <div>
          <p className="font-medium">{labels[type]}</p>
          <p className="text-sm text-muted-foreground">
            {t('dashboard.integrations.count', { count })}
          </p>
        </div>
      </div>
      <div className="flex items-center gap-2">
        {enabled > 0 ? (
          <span className="flex items-center gap-1 text-sm text-green-500">
            <CheckCircle className="h-4 w-4" />
            {t('dashboard.integrations.enabled', { count: enabled })}
          </span>
        ) : (
          <span className="flex items-center gap-1 text-sm text-muted-foreground">
            <XCircle className="h-4 w-4" />
            {t('integrations_page.card.status_disabled')}
          </span>
        )}
        <Link to="/integrations">
          <Button variant="ghost" size="sm">
            {t('dashboard.integrations.configure')}
          </Button>
        </Link>
      </div>
    </div>
  );
}

// Recent notification component
function RecentNotification({
  type,
  destination,
  message,
  status,
  createdAt,
}: {
  type: string;
  destination: string;
  message: string;
  status: string;
  createdAt: string;
}) {
  const { t } = useTranslation();

  const typeLabels: Record<string, string> = {
    stream_online: t('notifications.filters.stream_online'),
    stream_offline: t('notifications.filters.stream_offline'),
    title_change: t('notifications.filters.title_change'),
    category_change: t('notifications.filters.category_change'),
    reward_redemption: t('notifications.filters.reward_redemption'),
  };

  const destinationLabels: Record<string, string> = {
    telegram: t('notifications.filters.telegram'),
    discord: t('notifications.filters.discord'),
    chat: t('dashboard.chat'),
  };

  return (
    <div className="flex items-start gap-4 rounded-lg border p-4">
      <div
        className={cn(
          'mt-1 h-2 w-2 rounded-full',
          status === 'sent' ? 'bg-green-500' : 'bg-red-500',
        )}
      />
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <span className="font-medium">{typeLabels[type] || type}</span>
          <span className="text-xs text-muted-foreground">{t('ui.arrow')}</span>
          <span className="text-sm text-muted-foreground">
            {destinationLabels[destination] || destination}
          </span>
        </div>
        <p className="mt-1 text-sm text-muted-foreground truncate">{message}</p>
        <p className="mt-1 text-xs text-muted-foreground">{formatRelativeTime(createdAt)}</p>
      </div>
    </div>
  );
}

export function DashboardPage() {
  const { user } = useAuth();
  const { t } = useTranslation();

  // Fetch integrations
  const { data: telegramIntegrations } = useQuery({
    queryKey: ['telegram-integrations'],
    queryFn: () => telegramApi.list(),
  });

  const { data: discordIntegrations } = useQuery({
    queryKey: ['discord-integrations'],
    queryFn: () => discordApi.list(),
  });

  // Fetch notification stats
  const { data: stats } = useQuery({
    queryKey: ['notification-stats'],
    queryFn: notificationsApi.getStats,
  });

  // Fetch recent notifications
  const { data: recentNotifications } = useQuery({
    queryKey: ['recent-notifications'],
    queryFn: () => notificationsApi.list({ page: 1, per_page: 5 }),
  });

  const telegramList = telegramIntegrations || [];
  const discordList = discordIntegrations || [];
  const telegramEnabled = telegramList.filter((i) => i.is_enabled).length;
  const discordEnabled = discordList.filter((i) => i.is_enabled).length;
  const totalIntegrations = telegramList.length + discordList.length;

  return (
    <div className="space-y-8">
      {/* Welcome header */}
      <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4">
        <div className="flex items-center gap-4">
          {user && (
            <img
              src={getTwitchProfileImageUrl(user.twitch_profile_image_url, 70)}
              alt={user.twitch_display_name}
              className="h-16 w-16 rounded-full ring-4 ring-twitch/20"
            />
          )}
          <div>
            <h1 className="text-2xl font-bold">
              {t('dashboard.greeting', { name: user?.twitch_display_name })}
            </h1>
            <p className="text-muted-foreground">{t('dashboard.overview')}</p>
          </div>
        </div>
        <div className="flex gap-2">
          <a
            href={getTwitchChannelUrl(user?.twitch_login || '')}
            target="_blank"
            rel="noopener noreferrer"
          >
            <Button variant="outline" className="gap-2">
              <ExternalLink className="h-4 w-4" />
              {t('dashboard.twitch_channel')}
            </Button>
          </a>
        </div>
      </div>

      {/* Stats grid */}
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <StatCard
          title={t('dashboard.stats.total_notifications')}
          value={(stats?.total_sent || 0) + (stats?.total_failed || 0)}
          icon={<Bell className="h-5 w-5" />}
          description={t('dashboard.stats.all_time')}
        />
        <StatCard
          title={t('dashboard.stats.successful')}
          value={stats?.total_sent || 0}
          icon={<CheckCircle className="h-5 w-5 text-green-500" />}
        />
        <StatCard
          title={t('dashboard.stats.errors')}
          value={stats?.total_failed || 0}
          icon={<XCircle className="h-5 w-5 text-red-500" />}
        />
        <StatCard
          title={t('dashboard.stats.active_integrations')}
          value={telegramEnabled + discordEnabled}
          icon={<Activity className="h-5 w-5 text-twitch" />}
          description={t('dashboard.stats.configured', { count: totalIntegrations })}
        />
      </div>

      {/* Integrations overview */}
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <h2 className="text-xl font-semibold">{t('dashboard.integrations.title')}</h2>
          <Link to="/integrations">
            <Button variant="ghost" size="sm" className="gap-2">
              <Plus className="h-4 w-4" />
              {t('dashboard.integrations.add')}
            </Button>
          </Link>
        </div>

        <div className="grid gap-4 sm:grid-cols-2">
          <IntegrationStatus
            type="telegram"
            count={telegramList.length}
            enabled={telegramEnabled}
          />
          <IntegrationStatus type="discord" count={discordList.length} enabled={discordEnabled} />
        </div>

        {totalIntegrations === 0 && (
          <div className="rounded-lg border border-dashed p-8 text-center">
            <Zap className="mx-auto h-12 w-12 text-muted-foreground/50" />
            <h3 className="mt-4 font-semibold">{t('dashboard.integrations.no_integrations')}</h3>
            <p className="mt-2 text-sm text-muted-foreground">
              {t('dashboard.integrations.connect_message')}
            </p>
            <Link to="/integrations" className="mt-4 inline-block">
              <Button className="gap-2">
                <Plus className="h-4 w-4" />
                {t('dashboard.integrations.add')}
              </Button>
            </Link>
          </div>
        )}
      </div>

      {/* Recent notifications */}
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <h2 className="text-xl font-semibold">{t('dashboard.recent_notifications.title')}</h2>
          <Link to="/notifications">
            <Button variant="ghost" size="sm">
              {t('dashboard.recent_notifications.show_all')}
            </Button>
          </Link>
        </div>

        {recentNotifications?.items && recentNotifications.items.length > 0 ? (
          <div className="space-y-3">
            {recentNotifications.items.map((notification) => (
              <RecentNotification
                key={notification.id}
                type={notification.notification_type}
                destination={notification.destination_type}
                message={notification.content}
                status={notification.status}
                createdAt={notification.created_at}
              />
            ))}
          </div>
        ) : (
          <div className="rounded-lg border border-dashed p-8 text-center">
            <Bell className="mx-auto h-12 w-12 text-muted-foreground/50" />
            <h3 className="mt-4 font-semibold">
              {t('dashboard.recent_notifications.no_notifications')}
            </h3>
            <p className="mt-2 text-sm text-muted-foreground">
              {t('dashboard.recent_notifications.will_appear')}
            </p>
          </div>
        )}
      </div>
    </div>
  );
}
