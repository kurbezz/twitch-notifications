import { useState, useEffect } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import MessageEditor from '@/components/message-editor';
import { settingsApi, MessagesInfo, authApi, User, ApiError } from '@/lib/api';
import { useTranslation } from 'react-i18next';
import { useAuth } from '@/hooks/useAuth';
import { setLanguage } from '@/i18n';

import { Loader2, Edit } from 'lucide-react';

export default function UserSettingsBlock({
  ownerId,
  canManage = true,
}: {
  ownerId?: string;
  canManage?: boolean;
}) {
  const queryClient = useQueryClient();

  const messagesKey = ownerId ? ['settings', 'user', ownerId, 'messages'] : ['messages'];

  const { data: messagesData, isLoading: messagesLoading } = useQuery<MessagesInfo | null>({
    queryKey: messagesKey,
    queryFn: () => (ownerId ? settingsApi.getMessagesForUser(ownerId) : settingsApi.getMessages()),
    retry: false,
  });

  const updateMessagesMutation = useMutation({
    mutationFn: (payload: Partial<MessagesInfo>) =>
      ownerId
        ? settingsApi.updateMessagesForUser(ownerId, payload)
        : settingsApi.updateMessages(payload),
    onMutate: () => {
      // clear previous save error when a new mutation starts
      setSaveError(null);
    },
    onError: (err: unknown) => {
      // Extract a friendly message from the error object safely.
      let message = t('user_settings.saving_error');
      if (err instanceof Error) {
        message = err.message;
      } else if (typeof err === 'object' && err != null && 'message' in err) {
        const m = (err as { message?: unknown }).message;
        if (typeof m === 'string') message = m;
      }
      setSaveError(message);
    },
    onSuccess: () => {
      setSaveError(null);
      queryClient.invalidateQueries({ queryKey: ['settings'] });
      queryClient.invalidateQueries({ queryKey: messagesKey });
    },
  });

  // Reset-to-defaults functionality removed.

  // Owner-level notification flag queries are not needed in this component anymore;
  // notification toggles are managed per-integration on the /integrations page.

  // Owner-level setting updates for notification flags (e.g. notify_reward_redemption)
  // are handled per-integration now. The mutation to toggle that owner setting has
  // been removed from this component.

  // Reward toggle handler removed; per-integration toggles live on /integrations.

  const [localMessages, setLocalMessages] = useState<Partial<MessagesInfo>>({});
  const [saveError, setSaveError] = useState<string | null>(null);

  const { t } = useTranslation();
  const { user } = useAuth();
  const updateLangMutation = useMutation<User, ApiError, string>({
    mutationFn: (lang: string) => authApi.updateMe({ lang }),
  });
  const handleLanguageChange = (lang: string) => {
    updateLangMutation.mutate(lang, {
      onSuccess: () => {
        setLanguage(lang);
        queryClient.invalidateQueries({ queryKey: ['auth', 'me'] });
      },
    });
  };

  useEffect(() => {
    if (messagesData) {
      setLocalMessages({
        stream_online_message: messagesData.stream_online_message,
        stream_offline_message: messagesData.stream_offline_message,
        stream_title_change_message: messagesData.stream_title_change_message,
        stream_category_change_message: messagesData.stream_category_change_message,
        reward_redemption_message: messagesData.reward_redemption_message,
      });
    }
  }, [messagesData]);

  const handleMessageChange = (field: string, value: string) => {
    // Clear any previous save error when user begins editing again
    setSaveError(null);
    setLocalMessages((prev) => ({ ...prev, [field]: value }));
  };

  // Accept optional value (from MessageEditor) to avoid relying on local state being updated synchronously.
  const handleMessageSave = (field: string, value?: string) => {
    const val = value !== undefined ? value : localMessages[field as keyof typeof localMessages];
    if (val !== undefined) {
      updateMessagesMutation.mutate({ [field]: val } as Partial<MessagesInfo>);
    }
  };

  if (messagesLoading) {
    return (
      <div className="flex items-center justify-center min-h-[200px]">
        <Loader2 className="h-8 w-8 animate-spin text-twitch" />
      </div>
    );
  }

  return (
    <section className="space-y-6">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Edit className="h-5 w-5 text-twitch" />
          <h2 className="text-xl font-semibold">{t('user_settings.templates')}</h2>
        </div>

        {!ownerId && (
          <div className="flex flex-col items-end">
            <label className="text-sm font-medium">{t('user_settings.language')}</label>
            <div className="mt-1">
              <select
                value={user?.lang ?? 'ru'}
                onChange={(e) => handleLanguageChange(e.target.value)}
                disabled={!canManage || updateLangMutation.isPending}
                className="rounded-md border bg-background px-3 py-2 text-sm"
              >
                <option value="ru">{t('user_settings.language_ru')}</option>
                <option value="en">{t('user_settings.language_en')}</option>
              </select>
            </div>
          </div>
        )}
      </div>

      <div className="space-y-4">
        <MessageEditor
          label={t('message_editor.stream_online')}
          description={t('message_editor.stream_online_desc')}
          value={localMessages.stream_online_message || messagesData?.stream_online_message || ''}
          placeholders={messagesData?.placeholders?.stream || []}
          onChange={(value) => handleMessageChange('stream_online_message', value)}
          onSave={(value) => handleMessageSave('stream_online_message', value)}
          isSaving={updateMessagesMutation.isPending}
          canEdit={!!canManage}
        />

        <MessageEditor
          label={t('message_editor.stream_offline')}
          description={t('message_editor.stream_offline_desc')}
          value={localMessages.stream_offline_message || messagesData?.stream_offline_message || ''}
          placeholders={messagesData?.placeholders?.stream || []}
          onChange={(value) => handleMessageChange('stream_offline_message', value)}
          onSave={(value) => handleMessageSave('stream_offline_message', value)}
          isSaving={updateMessagesMutation.isPending}
          canEdit={!!canManage}
        />

        <MessageEditor
          label={t('message_editor.title_change')}
          description={t('message_editor.title_change_desc')}
          value={
            localMessages.stream_title_change_message ||
            messagesData?.stream_title_change_message ||
            ''
          }
          placeholders={messagesData?.placeholders?.stream || []}
          onChange={(value) => handleMessageChange('stream_title_change_message', value)}
          onSave={(value) => handleMessageSave('stream_title_change_message', value)}
          isSaving={updateMessagesMutation.isPending}
          canEdit={!!canManage}
        />

        <MessageEditor
          label={t('message_editor.category_change')}
          description={t('message_editor.category_change_desc')}
          value={
            localMessages.stream_category_change_message ||
            messagesData?.stream_category_change_message ||
            ''
          }
          placeholders={messagesData?.placeholders?.stream || []}
          onChange={(value) => handleMessageChange('stream_category_change_message', value)}
          onSave={(value) => handleMessageSave('stream_category_change_message', value)}
          isSaving={updateMessagesMutation.isPending}
          canEdit={!!canManage}
        />

        <MessageEditor
          label={t('message_editor.reward_activation')}
          description={t('message_editor.reward_activation_desc')}
          value={
            localMessages.reward_redemption_message || messagesData?.reward_redemption_message || ''
          }
          placeholders={messagesData?.placeholders?.reward || []}
          onChange={(value) => handleMessageChange('reward_redemption_message', value)}
          onSave={(value) => handleMessageSave('reward_redemption_message', value)}
          isSaving={updateMessagesMutation.isPending}
          canEdit={!!canManage}
        />
        {saveError && (
          <div className="mt-2 rounded-md bg-red-50 p-3 text-sm text-red-700">{saveError}</div>
        )}
      </div>
    </section>
  );
}
