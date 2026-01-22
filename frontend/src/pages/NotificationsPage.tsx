import { useQuery } from '@tanstack/react-query';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { notificationsApi } from '@/lib/api';
import {
  cn,
  formatRelativeTime,
  getNotificationTypeLabel,
  getDestinationTypeLabel,
} from '@/lib/utils';
import { Button } from '@/components/ui/button';
import {
  Bell,
  CheckCircle,
  ChevronLeft,
  ChevronRight,
  Filter,
  RefreshCw,
  XCircle,
  AlertCircle,
  Play,
  Edit,
  Gamepad2,
  Gift,
  StopCircle,
  MessageCircle,
  Zap,
} from 'lucide-react';

// Notification type icon mapping
function getNotificationIcon(type: string) {
  const icons: Record<string, React.ReactNode> = {
    stream_online: <Play className="h-4 w-4" />,
    stream_offline: <StopCircle className="h-4 w-4" />,
    title_change: <Edit className="h-4 w-4" />,
    category_change: <Gamepad2 className="h-4 w-4" />,
    reward_redemption: <Gift className="h-4 w-4" />,
  };
  return icons[type] || <Bell className="h-4 w-4" />;
}

// Destination icon mapping
function getDestinationIcon(type: string) {
  const icons: Record<string, React.ReactNode> = {
    telegram: <MessageCircle className="h-4 w-4" />,
    discord: <Zap className="h-4 w-4" />,
  };
  return icons[type] || <Bell className="h-4 w-4" />;
}

// Status badge component
function StatusBadge({ status }: { status: string }) {
  const { t } = useTranslation();

  const statusConfig: Record<string, { icon: React.ReactNode; className: string; label: string }> =
    {
      sent: {
        icon: <CheckCircle className="h-3 w-3" />,
        className: 'bg-green-500/10 text-green-500 border-green-500/20',
        label: t('status.sent'),
      },
      failed: {
        icon: <XCircle className="h-3 w-3" />,
        className: 'bg-red-500/10 text-red-500 border-red-500/20',
        label: t('status.failed'),
      },
      pending: {
        icon: <AlertCircle className="h-3 w-3" />,
        className: 'bg-yellow-500/10 text-yellow-500 border-yellow-500/20',
        label: t('status.pending'),
      },
    };

  const config = statusConfig[status] || statusConfig.pending;

  return (
    <span
      className={cn(
        'inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-xs font-medium',
        config.className,
      )}
    >
      {config.icon}
      {config.label}
    </span>
  );
}

// Filter button component
function FilterButton({
  active,
  onClick,
  children,
}: {
  active: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      onClick={onClick}
      className={cn(
        'rounded-full px-3 py-1.5 text-sm font-medium transition-colors',
        active
          ? 'bg-primary text-primary-foreground'
          : 'bg-muted text-muted-foreground hover:bg-muted/80',
      )}
    >
      {children}
    </button>
  );
}

export function NotificationsPage() {
  const { t } = useTranslation();
  const [page, setPage] = useState(1);
  const [filters, setFilters] = useState<{
    notification_type?: string;
    destination_type?: string;
    status?: string;
  }>({});
  const perPage = 20;

  // Fetch notifications
  const {
    data: notificationsData,
    isLoading,
    isFetching,
    refetch,
  } = useQuery({
    queryKey: ['notifications', page, filters],
    queryFn: () =>
      notificationsApi.list({
        page,
        per_page: perPage,
        ...filters,
      }),
  });

  // Fetch stats
  const { data: stats } = useQuery({
    queryKey: ['notification-stats'],
    queryFn: notificationsApi.getStats,
  });

  const notifications = notificationsData?.items || [];
  const totalPages = notificationsData?.total_pages || 0;
  const total = notificationsData?.total || 0;

  const handleFilterChange = (key: string, value: string | undefined) => {
    setPage(1);
    setFilters((prev) => ({
      ...prev,
      [key]: value === prev[key as keyof typeof prev] ? undefined : value,
    }));
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4">
        <div>
          <h1 className="text-2xl font-bold">{t('notifications.title')}</h1>
          <p className="text-muted-foreground">{t('notifications.subtitle')}</p>
        </div>
        <Button variant="outline" onClick={() => refetch()} disabled={isFetching} className="gap-2">
          <RefreshCw className={cn('h-4 w-4', isFetching && 'animate-spin')} />
          {t('notifications.refresh')}
        </Button>
      </div>

      {/* Stats cards */}
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <div className="rounded-lg border bg-card p-4">
          <div className="flex items-center gap-2 text-muted-foreground">
            <Bell className="h-4 w-4" />
            <span className="text-sm">{t('notifications.stats.all')}</span>
          </div>
          <p className="mt-2 text-2xl font-bold">
            {(stats?.total_sent || 0) + (stats?.total_failed || 0)}
          </p>
        </div>
        <div className="rounded-lg border bg-card p-4">
          <div className="flex items-center gap-2 text-green-500">
            <CheckCircle className="h-4 w-4" />
            <span className="text-sm">{t('notifications.stats.successful')}</span>
          </div>
          <p className="mt-2 text-2xl font-bold text-green-500">{stats?.total_sent || 0}</p>
        </div>
        <div className="rounded-lg border bg-card p-4">
          <div className="flex items-center gap-2 text-red-500">
            <XCircle className="h-4 w-4" />
            <span className="text-sm">{t('notifications.stats.errors')}</span>
          </div>
          <p className="mt-2 text-2xl font-bold text-red-500">{stats?.total_failed || 0}</p>
        </div>
        <div className="rounded-lg border bg-card p-4">
          <div className="flex items-center gap-2 text-muted-foreground">
            <Filter className="h-4 w-4" />
            <span className="text-sm">{t('notifications.stats.filtered')}</span>
          </div>
          <p className="mt-2 text-2xl font-bold">{total}</p>
        </div>
      </div>

      {/* Filters */}
      <div className="space-y-3">
        <div className="flex items-center gap-2 text-sm text-muted-foreground">
          <Filter className="h-4 w-4" />
          {t('notifications.filters_all_title')}
        </div>
        <div className="flex flex-wrap gap-2">
          <FilterButton
            active={!filters.notification_type && !filters.destination_type && !filters.status}
            onClick={() => setFilters({})}
          >
            {t('notifications.filters.all')}
          </FilterButton>
          <span className="text-muted-foreground">{t('ui.pipe')}</span>
          <FilterButton
            active={filters.notification_type === 'stream_online'}
            onClick={() => handleFilterChange('notification_type', 'stream_online')}
          >
            {t('notifications.filters.stream_online')}
          </FilterButton>
          <FilterButton
            active={filters.notification_type === 'stream_offline'}
            onClick={() => handleFilterChange('notification_type', 'stream_offline')}
          >
            {t('notifications.filters.stream_offline')}
          </FilterButton>
          <FilterButton
            active={filters.notification_type === 'title_change'}
            onClick={() => handleFilterChange('notification_type', 'title_change')}
          >
            {t('notifications.filters.title_change')}
          </FilterButton>
          <FilterButton
            active={filters.notification_type === 'category_change'}
            onClick={() => handleFilterChange('notification_type', 'category_change')}
          >
            {t('notifications.filters.category_change')}
          </FilterButton>
          <FilterButton
            active={filters.notification_type === 'reward_redemption'}
            onClick={() => handleFilterChange('notification_type', 'reward_redemption')}
          >
            {t('notifications.filters.reward_redemption')}
          </FilterButton>
          <span className="text-muted-foreground">{t('ui.pipe')}</span>
          <FilterButton
            active={filters.destination_type === 'telegram'}
            onClick={() => handleFilterChange('destination_type', 'telegram')}
          >
            {t('notifications.filters.telegram')}
          </FilterButton>
          <FilterButton
            active={filters.destination_type === 'discord'}
            onClick={() => handleFilterChange('destination_type', 'discord')}
          >
            {t('notifications.filters.discord')}
          </FilterButton>
          <span className="text-muted-foreground">{t('ui.pipe')}</span>
          <FilterButton
            active={filters.status === 'sent'}
            onClick={() => handleFilterChange('status', 'sent')}
          >
            {t('notifications.filters.sent')}
          </FilterButton>
          <FilterButton
            active={filters.status === 'failed'}
            onClick={() => handleFilterChange('status', 'failed')}
          >
            {t('notifications.filters.failed')}
          </FilterButton>
        </div>
      </div>

      {/* Notifications list */}
      <div className="rounded-lg border">
        {isLoading ? (
          <div className="p-8 text-center">
            <RefreshCw className="mx-auto h-8 w-8 animate-spin text-muted-foreground" />
            <p className="mt-2 text-muted-foreground">{t('notifications.loading')}</p>
          </div>
        ) : notifications.length === 0 ? (
          <div className="p-8 text-center">
            <Bell className="mx-auto h-12 w-12 text-muted-foreground/50" />
            <h3 className="mt-4 font-semibold">{t('notifications.not_found')}</h3>
            <p className="mt-2 text-sm text-muted-foreground">
              {Object.keys(filters).length > 0
                ? t('notifications.try_filters')
                : t('notifications.will_appear')}
            </p>
          </div>
        ) : (
          <div className="divide-y">
            {notifications.map((notification) => (
              <div
                key={notification.id}
                className="flex items-start gap-4 p-4 hover:bg-muted/50 transition-colors"
              >
                {/* Type icon */}
                <div
                  className={cn(
                    'mt-1 flex h-10 w-10 items-center justify-center rounded-lg',
                    notification.status === 'sent'
                      ? 'bg-primary/10 text-primary'
                      : 'bg-red-500/10 text-red-500',
                  )}
                >
                  {getNotificationIcon(notification.notification_type)}
                </div>

                {/* Content */}
                <div className="flex-1 min-w-0">
                  <div className="flex flex-wrap items-center gap-2">
                    <span className="font-medium">
                      {getNotificationTypeLabel(notification.notification_type)}
                    </span>
                    <span className="text-muted-foreground">{t('ui.arrow')}</span>
                    <span className="inline-flex items-center gap-1 text-sm text-muted-foreground">
                      {getDestinationIcon(notification.destination_type)}
                      {getDestinationTypeLabel(notification.destination_type)}
                    </span>
                    <StatusBadge status={notification.status} />
                  </div>
                  <p className="mt-1 text-sm text-muted-foreground line-clamp-2">
                    {notification.content}
                  </p>
                  {notification.error_message && (
                    <p className="mt-1 text-sm text-red-500">
                      {t('notifications.error_prefix')}: {notification.error_message}
                    </p>
                  )}
                  <p className="mt-1 text-xs text-muted-foreground">
                    {formatRelativeTime(notification.created_at)}
                  </p>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Pagination */}
      {totalPages > 1 && (
        <div className="flex items-center justify-between">
          <p className="text-sm text-muted-foreground">
            {t('notifications.showing', {
              from: (page - 1) * perPage + 1,
              to: Math.min(page * perPage, total),
              total,
            })}
          </p>
          <div className="flex items-center gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={() => setPage(page - 1)}
              disabled={page <= 1}
            >
              <ChevronLeft className="h-4 w-4" />
              {t('notifications.pagination.prev')}
            </Button>
            <span className="px-2 text-sm">
              {t('notifications.pagination.page_of', { page, total: totalPages })}
            </span>
            <Button
              variant="outline"
              size="sm"
              onClick={() => setPage(page + 1)}
              disabled={page >= totalPages}
            >
              {t('notifications.pagination.next')}
              <ChevronRight className="h-4 w-4" />
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}
