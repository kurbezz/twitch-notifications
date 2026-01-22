import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { settingsApi, MessagesInfo } from '@/lib/api';

import MessageEditor from '@/components/message-editor';
import { Loader2, MessageSquare } from 'lucide-react';

// ImportMeta type is declared elsewhere; avoid redeclaring it with `any` here.

export function BotSettingsPage({ showNotice = true }: { showNotice?: boolean }) {
  const queryClient = useQueryClient();

  // Fetch settings (user-level)
  const { isLoading: settingsLoading } = useQuery({
    queryKey: ['settings'],
    queryFn: settingsApi.getSettings,
  });

  // Fetch message templates and placeholders
  const { data: messagesData, isLoading: messagesLoading } = useQuery({
    queryKey: ['messages'],
    queryFn: settingsApi.getMessages,
  });

  // Discord invite fetch removed (invite button removed from UI)

  // Mutations

  const updateMessagesMutation = useMutation({
    mutationFn: settingsApi.updateMessages,
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
      queryClient.invalidateQueries({ queryKey: ['messages'] });
    },
  });

  // Local state for editing the reward message
  const [localMessages, setLocalMessages] = useState<Partial<MessagesInfo>>({});
  const [saveError, setSaveError] = useState<string | null>(null);
  const { t } = useTranslation();

  useEffect(() => {
    if (messagesData) {
      setLocalMessages({
        reward_redemption_message: messagesData.reward_redemption_message,
      });
    }
  }, [messagesData]);

  // Auto-save disabled; messages are saved manually via the editor

  const handleMessageChange = (value: string) => {
    // Clear any previous save error when user begins editing again
    setSaveError(null);
    setLocalMessages((prev) => ({ ...prev, reward_redemption_message: value }));
  };

  // Accept an optional value directly from the editor to avoid relying on
  // React state updates being flushed synchronously when saving.
  const handleMessageSave = (value?: string) => {
    const val = value !== undefined ? value : localMessages.reward_redemption_message;
    if (val !== undefined) {
      updateMessagesMutation.mutate({ reward_redemption_message: val });
    }
  };

  if (settingsLoading || messagesLoading) {
    return (
      <div className="flex items-center justify-center min-h-[400px]">
        <Loader2 className="h-8 w-8 animate-spin text-twitch" />
      </div>
    );
  }

  const rewardPlaceholders = messagesData?.placeholders?.reward || [];
  // Auto-save disabled; template changes are saved manually

  return (
    <div className="space-y-8">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <MessageSquare className="h-6 w-6 text-twitch" />
          <div>
            <h1 className="text-2xl font-bold">{t('bot_settings.title')}</h1>
            <p className="text-muted-foreground">{t('bot_settings.subtitle')}</p>
          </div>
        </div>
      </div>

      {/* Chat bot settings */}
      <section className="space-y-4">
        <div className="grid gap-4 md:grid-cols-1">
          {showNotice ? (
            <div className="rounded-md border border-muted-foreground/10 bg-muted p-3 text-sm text-muted-foreground">
              {t('bot_settings.notice')}
            </div>
          ) : null}
        </div>

        <MessageEditor
          label={t('bot_settings.reward.title')}
          description={t('bot_settings.reward.description')}
          value={
            localMessages.reward_redemption_message || messagesData?.reward_redemption_message || ''
          }
          placeholders={rewardPlaceholders}
          onChange={(value) => handleMessageChange(value)}
          onSave={(value) => handleMessageSave(value)}
          isSaving={updateMessagesMutation.isPending}
        />
        {saveError && (
          <div className="mt-2 rounded-md bg-red-50 p-3 text-sm text-red-700">{saveError}</div>
        )}
      </section>
    </div>
  );
}

export default BotSettingsPage;
