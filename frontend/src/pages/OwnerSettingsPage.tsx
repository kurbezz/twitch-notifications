import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { useParams, Navigate, Link } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { settingsApi } from '@/lib/api';
import { useAuth } from '@/hooks/useAuth';
import { Loader2, MessageSquare } from 'lucide-react';
import UserSettingsBlock from '@/components/user-settings-block';
import UserIntegrationsBlock from '@/components/user-integrations-block';
import UserBotSettingsBlock from '@/components/user-bot-settings-block';
import { Button } from '@/components/ui/button';

/**
 * Page for viewing/editing another user's settings & integrations.
 *
 * - URL: /settings/:userId
 * - Determines whether the current user has access and whether they have `can_manage`.
 * - If no access -> show an informative message (403-like).
 * - If access: show settings block (message templates) and integrations block (Telegram/Discord).
 */
export function OwnerSettingsPage() {
  const { t } = useTranslation();
  const { user } = useAuth();
  const params = useParams();
  const ownerId = params.userId;

  // Hooks must be called unconditionally at the top of the component
  // Fetch incoming shares (owners who shared with me). We'll derive access/can_manage for this owner.
  const {
    data: incomingShares,
    isLoading: sharesLoading,
    error: sharesError,
  } = useQuery({
    queryKey: ['settings', 'shares', 'incoming'],
    queryFn: settingsApi.listIncomingShares,
    retry: false,
  });

  const incomingForOwner = useMemo(
    () => incomingShares?.find((s) => s.owner_user_id === ownerId) ?? null,
    [incomingShares, ownerId],
  );

  const canManage = !!incomingForOwner?.can_manage;
  const ownerLogin = incomingForOwner?.owner_twitch_login ?? ownerId;
  const ownerDisplay = incomingForOwner?.owner_display_name ?? '';

  // Redirect to own settings page when trying to open your own settings via this route
  if (!ownerId) return <Navigate to="/settings" replace />;
  if (user && ownerId === user.id) return <Navigate to="/settings" replace />;

  if (sharesLoading) {
    return (
      <div className="flex items-center justify-center min-h-[300px]">
        <Loader2 className="h-8 w-8 animate-spin text-twitch" />
      </div>
    );
  }

  // If there is no incoming share for this owner, show an access error.
  if (!incomingForOwner) {
    // If there was an error fetching incoming shares, show a general message
    if (sharesError) {
      return (
        <div className="max-w-3xl mx-auto py-12">
          <div className="rounded-lg border border-red-200 bg-red-50 p-6">
            <h2 className="text-lg font-semibold text-red-800">
              {t('owner_settings.access_error')}
            </h2>
            <p className="mt-2 text-sm text-red-700">{t('owner_settings.access_error_desc')}</p>
            <div className="mt-4">
              <Button asChild>
                <Link to="/settings">{t('owner_settings.return_to_my_settings')}</Link>
              </Button>
            </div>
          </div>
        </div>
      );
    }

    return (
      <div className="max-w-3xl mx-auto py-12">
        <div className="rounded-lg border border-yellow-200 bg-yellow-50 p-6">
          <h2 className="text-lg font-semibold">{t('owner_settings.access_denied')}</h2>
          <p className="mt-2 text-sm text-muted-foreground">
            {t('owner_settings.no_access_desc', { owner: ownerLogin })}
          </p>
          <div className="mt-4 flex gap-2">
            <Button asChild>
              <Link to="/settings">{t('owner_settings.my_settings')}</Link>
            </Button>
            <Button asChild variant="outline">
              <Link to="/settings/shared">{t('owner_settings.manage_accesses')}</Link>
            </Button>
          </div>
        </div>
      </div>
    );
  }

  // Render the owner settings + integrations; pass `canManage` so child components enable/disable editing
  return (
    <div className="space-y-8 max-w-4xl mx-auto py-8">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">
            {t('owner_settings.settings_for', { owner: ownerLogin })}
          </h1>
          {ownerDisplay ? <p className="text-sm text-muted-foreground">{ownerDisplay}</p> : null}
          <p className="text-sm text-muted-foreground mt-2">
            {canManage ? t('owner_settings.manage_notice') : t('owner_settings.view_notice')}
          </p>
        </div>

        <div className="flex items-center gap-2">
          <Button variant="outline" asChild>
            <Link to="/settings">{t('owner_settings.my_settings')}</Link>
          </Button>
        </div>
      </div>

      {/* Settings block (message templates etc.) */}
      <UserSettingsBlock ownerId={ownerId} canManage={canManage} />

      {/* Chat bot settings block */}
      <section className="space-y-6">
        <div className="flex items-center gap-2">
          <MessageSquare className="h-5 w-5 text-twitch" />
          <h2 className="text-xl font-semibold">{t('bot_settings.title')}</h2>
        </div>
        <UserBotSettingsBlock ownerId={ownerId} canManage={canManage} />
      </section>

      {/* Integrations block (Telegram / Discord) */}
      <UserIntegrationsBlock ownerId={ownerId} canManage={canManage} />
    </div>
  );
}

export default OwnerSettingsPage;
