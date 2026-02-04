import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { settingsApi, MessagesInfo } from '@/lib/api';

import MessageEditor from '@/components/message-editor';
import ToggleSwitch from '@/components/toggle-switch';
import { Loader2 } from 'lucide-react';

export default function UserBotSettingsBlock({
  ownerId,
  canManage = true,
}: {
  ownerId?: string;
  canManage?: boolean;
}) {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  // Fetch settings (user-level)
  const settingsKey = ownerId ? ['settings', 'user', ownerId] : ['settings'];
  const { data: settings, isLoading: settingsLoading } = useQuery({
    queryKey: settingsKey,
    queryFn: () =>
      ownerId ? settingsApi.getSettingsForUser(ownerId) : settingsApi.getSettings(),
    retry: false,
  });

  // Fetch message templates and placeholders
  const messagesKey = ownerId ? ['settings', 'user', ownerId, 'messages'] : ['messages'];
  const { data: messagesData, isLoading: messagesLoading } = useQuery({
    queryKey: messagesKey,
    queryFn: () =>
      ownerId ? settingsApi.getMessagesForUser(ownerId) : settingsApi.getMessages(),
    retry: false,
  });

  // Local state for editing the reward message
  const [localMessages, setLocalMessages] = useState<Partial<MessagesInfo>>({});
  const [saveError, setSaveError] = useState<string | null>(null);

  // Mutations

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
      queryClient.invalidateQueries({ queryKey: settingsKey });
      queryClient.invalidateQueries({ queryKey: messagesKey });
    },
  });

  const updateSettingsMutation = useMutation({
    mutationFn: (data: { notify_reward_redemption?: boolean }) =>
      ownerId
        ? settingsApi.updateSettingsForUser(ownerId, data)
        : settingsApi.updateSettings(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: settingsKey });
    },
  });

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
    <div className="space-y-4">
      <div className="grid gap-4 md:grid-cols-1">
        <div className="rounded-md border border-muted-foreground/10 bg-muted p-3 text-sm text-muted-foreground">
          {t('bot_settings.notice')}
        </div>
      </div>

      <ToggleSwitch
        label={t('bot_settings.reward_notification_toggle.label')}
        description={t('bot_settings.reward_notification_toggle.description')}
        checked={settings?.notify_reward_redemption ?? false}
        onChange={(checked) => {
          updateSettingsMutation.mutate({ notify_reward_redemption: checked });
        }}
        disabled={updateSettingsMutation.isPending || !canManage}
      />

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
        canEdit={canManage}
      />
      {saveError && (
        <div className="mt-2 rounded-md bg-red-50 p-3 text-sm text-red-700">{saveError}</div>
      )}
    </div>
  );
}
