import { useState } from 'react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { telegramApi, type TelegramIntegration } from '@/lib/api';
import { Button } from '@/components/ui/button';
import Toggle from '@/components/toggle';
import {
  MessageCircle,
  TestTube2,
  Trash2,
  Loader2,
  Settings2,
  CheckCircle,
  XCircle,
} from 'lucide-react';
import { alert as showAlert, confirm as showConfirm } from '@/lib/dialog';
import { useTranslation } from 'react-i18next';

type Props = {
  integration: TelegramIntegration;
  /**
   * If false, the card is rendered read-only (no edit/delete/test actions).
   */
  canManage?: boolean;
  /**
   * Optional ownerId used to invalidate owner-scoped cache keys when provided.
   */
  ownerId?: string;
};

export default function TelegramCard({ integration, canManage = true, ownerId }: Props) {
  const queryClient = useQueryClient();
  const [isDeleting, setIsDeleting] = useState(false);
  const [isTesting, setIsTesting] = useState(false);
  const { t } = useTranslation();

  const chatTypeLabel = (type?: string) => {
    if (type === 'private') return t('telegram.add_dialog.chat_type.private');
    if (type === 'group') return t('telegram.add_dialog.chat_type.group');
    if (type === 'supergroup' || type === 'channel')
      return t('telegram.add_dialog.chat_type.supergroup');
    return type ?? '';
  };

  const updateMutation = useMutation({
    mutationFn: (data: Parameters<typeof telegramApi.update>[1]) =>
      telegramApi.update(integration.id, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['telegram-integrations'] });
      if (ownerId) {
        queryClient.invalidateQueries({ queryKey: ['telegram-integrations', ownerId] });
      }
    },
  });

  type TelegramUpdatePayload = Parameters<typeof telegramApi.update>[1];

  const handleToggle = (field: keyof TelegramUpdatePayload, value: boolean) => {
    if (!canManage) return;
    updateMutation.mutate({ [field]: value } as TelegramUpdatePayload);
  };

  const handleDelete = async () => {
    if (!canManage) return;
    if (!(await showConfirm(t('integrations_page.card.delete_confirm')))) return;
    setIsDeleting(true);
    try {
      await telegramApi.delete(integration.id);
      queryClient.invalidateQueries({ queryKey: ['telegram-integrations'] });
      if (ownerId) {
        queryClient.invalidateQueries({ queryKey: ['telegram-integrations', ownerId] });
      }
    } catch {
      await showAlert(t('integrations_page.card.delete_error'));
    } finally {
      setIsDeleting(false);
    }
  };

  const handleTest = async () => {
    if (!canManage) return;
    setIsTesting(true);
    try {
      const result = await telegramApi.test(integration.id);
      if (result.success) {
        await showAlert(t('integrations_page.card.test_success'));
      } else {
        await showAlert(`${t('integrations_page.card.test_failed')}: ${result.message}`);
      }
    } catch {
      await showAlert(t('integrations_page.card.test_failed'));
    } finally {
      setIsTesting(false);
    }
  };

  return (
    <div className="rounded-lg border bg-card p-6">
      <div className="flex items-start justify-between">
        <div className="flex items-center gap-3">
          <div className="flex h-10 w-10 items-center justify-center rounded-full bg-blue-500/10">
            <MessageCircle className="h-5 w-5 text-blue-500" />
          </div>
          <div>
            <h3 className="font-semibold">
              {integration.telegram_chat_title ||
                t('telegram.add_dialog.name_example', { chatId: integration.telegram_chat_id })}
            </h3>
            <p className="text-sm text-muted-foreground">
              ID: {integration.telegram_chat_id} • {chatTypeLabel(integration.telegram_chat_type)}
            </p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          {canManage ? (
            <>
              <Button
                variant="ghost"
                size="icon"
                onClick={handleTest}
                disabled={isTesting}
                title={t('integrations_page.card.test_button_title')}
              >
                {isTesting ? (
                  <Loader2 className="h-4 w-4 animate-spin" />
                ) : (
                  <TestTube2 className="h-4 w-4" />
                )}
              </Button>
              <Button
                variant="ghost"
                size="icon"
                onClick={handleDelete}
                disabled={isDeleting}
                className="text-destructive hover:text-destructive"
                title={t('integrations_page.card.delete_button_title')}
              >
                {isDeleting ? (
                  <Loader2 className="h-4 w-4 animate-spin" />
                ) : (
                  <Trash2 className="h-4 w-4" />
                )}
              </Button>
            </>
          ) : null}
        </div>
      </div>

      {/* Status */}
      <div className="mt-4 flex items-center gap-2">
        {integration.is_enabled ? (
          <span className="flex items-center gap-1 text-sm text-green-500">
            <CheckCircle className="h-4 w-4" />
            {t('integrations_page.card.status_active')}
          </span>
        ) : (
          <span className="flex items-center gap-1 text-sm text-muted-foreground">
            <XCircle className="h-4 w-4" />
            {t('integrations_page.card.status_disabled')}
          </span>
        )}
      </div>

      {/* Settings */}
      <div className="mt-4 space-y-3 border-t pt-4">
        <div className="flex items-center gap-2 text-sm font-medium text-muted-foreground">
          <Settings2 className="h-4 w-4" />
          {t('integrations_page.card.settings_header')}
        </div>
        <div className="grid gap-2 sm:grid-cols-2">
          <Toggle
            label={t('integrations_page.card.toggle_stream_online')}
            checked={integration.notify_stream_online}
            onChange={(v) => handleToggle('notify_stream_online', v)}
            disabled={updateMutation.isPending || !canManage}
          />
          <Toggle
            label={t('integrations_page.card.toggle_stream_offline')}
            checked={integration.notify_stream_offline}
            onChange={(v) => handleToggle('notify_stream_offline', v)}
            disabled={updateMutation.isPending || !canManage}
          />
          <Toggle
            label={t('integrations_page.card.toggle_title_change')}
            checked={integration.notify_title_change}
            onChange={(v) => handleToggle('notify_title_change', v)}
            disabled={updateMutation.isPending || !canManage}
          />
          <Toggle
            label={t('integrations_page.card.toggle_category_change')}
            checked={integration.notify_category_change}
            onChange={(v) => handleToggle('notify_category_change', v)}
            disabled={updateMutation.isPending || !canManage}
          />
          <Toggle
            label={t('integrations_page.card.toggle_reward_redemption')}
            checked={integration.notify_reward_redemption}
            onChange={(v) => handleToggle('notify_reward_redemption', v)}
            disabled={updateMutation.isPending || !canManage}
          />
        </div>

        <div className="pt-2">
          <Toggle
            label={t('integrations_page.card.enabled_label')}
            checked={integration.is_enabled}
            onChange={(v) => handleToggle('is_enabled', v)}
            disabled={updateMutation.isPending || !canManage}
          />
        </div>
      </div>
    </div>
  );
}
