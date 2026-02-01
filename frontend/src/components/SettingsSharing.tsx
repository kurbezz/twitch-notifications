import { useEffect, useState } from 'react';
import { useForm as tanUseForm } from '@tanstack/react-form';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { settingsApi, usersApi } from '@/lib/api';
import type { OutgoingShare, IncomingShare, User } from '@/lib/api';
import { alert as showAlert, confirm as showConfirm } from '@/lib/dialog';

import { Button } from '@/components/ui/button';
import { Plus, Trash2, ExternalLink, Loader2 } from 'lucide-react';
import { Link } from 'react-router-dom';
import { useTranslation } from 'react-i18next';

function getErrorMessage(err: unknown, fallback = 'An error occurred') {
  if (err && typeof err === 'object' && 'message' in err) {
    const msg = (err as Record<string, unknown>)['message'];
    if (typeof msg === 'string') return msg;
  }
  return fallback;
}

/**
 * UI to manage sharing of your settings with other users (outgoing),
 * and to view settings that others shared with you (incoming).
 *
 * - Outgoing: give access by Twitch login, list grantees, toggle can_manage, revoke.
 * - Incoming: list owners who shared with you and open their settings in a modal
 *   (editable if owner granted can_manage).
 */

export default function SettingsSharing() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();

  const [activeTab, setActiveTab] = useState<'outgoing' | 'incoming'>('outgoing');

  // Form state for creating a share (search by nickname + select)
  const { setValue, getValues, reset } = tanUseForm<{ searchTerm: string; canManage: boolean }>({
    defaultValues: { searchTerm: '', canManage: false },
  });
  const [selectedUser, setSelectedUser] = useState<User | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);

  // Debounce the search input (reads from form state)
  const [debouncedSearch, setDebouncedSearch] = useState('');
  useEffect(() => {
    const id = setTimeout(() => setDebouncedSearch(getValues().searchTerm ?? ''), 300);
    return () => clearTimeout(id);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [getValues().searchTerm]);

  // Query for user search results (enabled when debounced search >= 2 chars)
  const { data: userSearchResults, isLoading: userSearchLoading } = useQuery<User[]>({
    queryKey: ['users', 'search', debouncedSearch],
    queryFn: () => usersApi.search(debouncedSearch, 10),
    enabled: debouncedSearch.trim().length >= 2,
    staleTime: 60_000,
    refetchOnWindowFocus: false,
  });

  // Owner editor moved to a dedicated page: /settings/:userId (no modal state here)

  // Queries
  const {
    data: outgoing,
    isLoading: outgoingLoading,
    isFetching: outgoingFetching,
  } = useQuery<OutgoingShare[]>({
    queryKey: ['settings', 'shares', 'outgoing'],
    queryFn: settingsApi.listOutgoingShares,
  });

  const {
    data: incoming,
    isLoading: incomingLoading,
    isFetching: incomingFetching,
  } = useQuery<IncomingShare[]>({
    queryKey: ['settings', 'shares', 'incoming'],
    queryFn: settingsApi.listIncomingShares,
  });

  // Create share
  const createMutation = useMutation({
    mutationFn: (payload: { twitch_login: string; can_manage?: boolean }) =>
      settingsApi.createShare(payload),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['settings', 'shares', 'outgoing'] });
      queryClient.invalidateQueries({ queryKey: ['settings', 'shares', 'incoming'] });
      reset();
      setSelectedUser(null);
    },
    onError: (err: unknown) => {
      const msg = getErrorMessage(err, t('settings_sharing.create_error'));
      showAlert(msg);
    },
  });

  // Update share (toggle can_manage)
  const updateMutation = useMutation({
    mutationFn: ({ granteeId, can_manage }: { granteeId: string; can_manage: boolean }) =>
      settingsApi.updateShare(granteeId, { can_manage }),
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: ['settings', 'shares', 'outgoing'] }),
    onError: (err: unknown) => {
      showAlert(getErrorMessage(err, t('settings_sharing.update_error')));
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (granteeId: string) => settingsApi.deleteShare(granteeId),
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: ['settings', 'shares', 'outgoing'] }),
    onError: (err: unknown) => {
      showAlert(getErrorMessage(err, t('settings_sharing.delete_error')));
    },
  });

  // Owner messages handled on the dedicated owner settings page (no modal here)

  // Owner integrations listing and management moved to the Owner Settings page (see /settings/:userId).

  // Owner integration actions moved to the Owner Settings page (no helpers remain here).

  // Update owner messages
  // Message updates for other users are handled on the dedicated page (UserSettingsBlock).

  // Initialize local messages when ownerMessages loads
  // Local message initialization moved to UserSettingsBlock.

  const handleCreateShare = async (e?: React.FormEvent) => {
    e?.preventDefault();
    const values = getValues();
    const login = selectedUser ? selectedUser.twitch_login : (values.searchTerm ?? '').trim();
    if (!login) {
      await showAlert(t('settings_sharing.enter_login'));
      return;
    }
    setIsSubmitting(true);
    try {
      await createMutation.mutateAsync({ twitch_login: login, can_manage: canManage });
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleToggleCanManage = async (granteeId: string, current: boolean) => {
    updateMutation.mutate({ granteeId, can_manage: !current });
  };

  const handleDeleteShare = async (granteeId: string) => {
    if (!(await showConfirm(t('settings_sharing.delete_confirm')))) return;
    deleteMutation.mutate(granteeId);
  };

  // Owner modal removed; navigating to dedicated owner settings page instead.

  // Owner message editing is handled on the dedicated owner settings page (/settings/:userId)

  // Owner message saving handled on the dedicated owner settings page

  return (
    <section className="rounded-lg border bg-card p-6 space-y-4">
      <div className="flex items-start justify-between">
        <div>
          <h2 className="text-xl font-semibold">{t('settings_sharing.title')}</h2>
          <p className="text-sm text-muted-foreground">{t('settings_sharing.description')}</p>
        </div>
      </div>

      {/* Tabs */}
      <div className="flex gap-2">
        <button
          className={`px-3 py-1 rounded ${activeTab === 'outgoing' ? 'bg-twitch text-white' : 'bg-muted/30'}`}
          onClick={() => setActiveTab('outgoing')}
        >
          {t('settings_sharing.tabs.outgoing')}
        </button>
        <button
          className={`px-3 py-1 rounded ${activeTab === 'incoming' ? 'bg-twitch text-white' : 'bg-muted/30'}`}
          onClick={() => setActiveTab('incoming')}
        >
          {t('settings_sharing.tabs.incoming')}
        </button>
      </div>

      {activeTab === 'outgoing' ? (
        <div className="space-y-4">
          {/* Grant form */}
          <form onSubmit={handleCreateShare} className="flex gap-2 items-start">
            <div className="relative w-full">
              <input
                type="text"
                value={getValues().searchTerm ?? ''}
                onChange={(e) => {
                  setValue('searchTerm', e.target.value);
                  setSelectedUser(null);
                }}
                placeholder={t('settings_sharing.placeholder')}
                className="rounded-md border bg-background px-3 py-2 text-sm w-full"
              />

              {selectedUser && (
                <div className="mt-2 flex items-center gap-2">
                  <div className="inline-flex items-center gap-3 rounded-full bg-muted/20 px-3 py-1">
                    <div className="font-medium text-sm">{selectedUser.twitch_login}</div>
                    <div className="text-xs text-muted-foreground">
                      {selectedUser.twitch_display_name}
                    </div>
                  </div>
                  <button
                    type="button"
                    onClick={() => {
                      setSelectedUser(null);
                      setValue('searchTerm', '');
                    }}
                    className="text-sm text-red-500 hover:underline"
                  >
                    {t('settings_sharing.clear')}
                  </button>
                </div>
              )}

              {debouncedSearch.trim().length >= 2 && !selectedUser && (
                <div className="absolute left-0 right-0 mt-1 z-10 bg-card border rounded-md max-h-60 overflow-auto">
                  {userSearchLoading ? (
                    <div className="py-2 flex items-center justify-center">
                      <Loader2 className="h-4 w-4 animate-spin text-twitch" />
                    </div>
                  ) : userSearchResults && userSearchResults.length > 0 ? (
                    userSearchResults.map((u) => (
                      <button
                        key={u.id}
                        type="button"
                        onClick={() => {
                          setSelectedUser(u);
                          setValue('searchTerm', u.twitch_login);
                        }}
                        className="w-full text-left px-3 py-2 hover:bg-muted/20"
                      >
                        <div className="font-medium">{u.twitch_login}</div>
                        <div className="text-sm text-muted-foreground">{u.twitch_display_name}</div>
                      </button>
                    ))
                  ) : (
                    <div className="py-2 px-3 text-sm text-muted-foreground">
                      {t('settings_sharing.search_none')}
                    </div>
                  )}
                </div>
              )}
            </div>

            <label className="flex items-center gap-2 mt-1">
              <input
                type="checkbox"
                checked={getValues().canManage ?? false}
                onChange={(e) => setValue('canManage', e.target.checked)}
                className="h-4 w-4"
              />
              <span className="text-sm">{t('settings_sharing.can_manage')}</span>
            </label>
            <Button type="submit" disabled={isSubmitting}>
              {isSubmitting ? (
                <Loader2 className="h-4 w-4 animate-spin mr-2" />
              ) : (
                <Plus className="h-4 w-4 mr-2" />
              )}
              {t('settings_sharing.add')}
            </Button>
          </form>

          {/* Outgoing list */}
          <div className="rounded-md border p-4 space-y-2">
            {outgoingLoading || outgoingFetching ? (
              <div className="py-4 flex items-center justify-center">
                <Loader2 className="h-5 w-5 animate-spin text-twitch" />
              </div>
            ) : outgoing && outgoing.length > 0 ? (
              outgoing.map((s) => (
                <div key={s.grantee_user_id} className="flex items-center justify-between gap-4">
                  <div>
                    <div className="font-medium">{s.grantee_twitch_login}</div>
                    <div className="text-sm text-muted-foreground">{s.grantee_display_name}</div>
                  </div>
                  <div className="flex items-center gap-2">
                    <label className="flex items-center gap-2 text-sm">
                      <input
                        type="checkbox"
                        checked={s.can_manage}
                        onChange={() => handleToggleCanManage(s.grantee_user_id, s.can_manage)}
                        className="h-4 w-4"
                      />
                      <span>{t('settings_sharing.can_manage')}</span>
                    </label>
                    <Button variant="ghost" onClick={() => handleDeleteShare(s.grantee_user_id)}>
                      <Trash2 className="h-4 w-4" />
                    </Button>
                  </div>
                </div>
              ))
            ) : (
              <div className="py-4 text-sm text-muted-foreground">{t('settings_sharing.none')}</div>
            )}
          </div>
        </div>
      ) : (
        // Incoming
        <div className="space-y-4">
          {incomingLoading || incomingFetching ? (
            <div className="py-4 flex items-center justify-center">
              <Loader2 className="h-5 w-5 animate-spin text-twitch" />
            </div>
          ) : incoming && incoming.length > 0 ? (
            incoming.map((s) => (
              <div key={s.owner_user_id} className="flex items-center justify-between gap-4">
                <div>
                  <div className="font-medium">{s.owner_twitch_login}</div>
                  <div className="text-sm text-muted-foreground">{s.owner_display_name}</div>
                </div>
                <div className="flex items-center gap-2">
                  <div className="text-sm">
                    {s.can_manage
                      ? t('settings_sharing.full_access')
                      : t('settings_sharing.view_only')}
                  </div>
                  <Button variant="ghost" asChild>
                    <Link to={`/settings/${s.owner_user_id}`}>
                      <ExternalLink className="h-4 w-4 mr-2" />
                      {t('settings_sharing.view')}
                    </Link>
                  </Button>
                </div>
              </div>
            ))
          ) : (
            <div className="py-4 text-sm text-muted-foreground">
              {t('settings_sharing.none_incoming')}
            </div>
          )}
        </div>
      )}

      {/* Owner settings and integrations are now available on a dedicated page. Use the "Open" button in the incoming list to navigate to the owner's settings page. */}
    </section>
  );
}
