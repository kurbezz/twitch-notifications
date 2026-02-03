import { useState, type ComponentType } from 'react';
import { useForm, useWatch } from '@/lib/useForm';
import type { AnyFieldApi } from '@tanstack/react-form';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { telegramApi, TelegramBotInfo } from '@/lib/api';
import { Button } from '@/components/ui/button';
import { Loader2 } from 'lucide-react';
import { useAuth } from '@/hooks/useAuth';
import { alert as showAlert } from '@/lib/dialog';
import { useTranslation } from 'react-i18next';

type Props = {
  open: boolean;
  onClose: () => void;
  /**
   * Optional ownerId — when provided, the integration will be created on behalf of that user.
   */
  ownerId?: string;
};

export default function AddTelegramDialog({ open, onClose, ownerId }: Props) {
  const [isSubmitting, setIsSubmitting] = useState(false);
  const queryClient = useQueryClient();
  const { user } = useAuth();
  const { t } = useTranslation();
  const isTelegramLinked = !!user?.telegram_user_id;

  const { data: botInfo, isLoading: botLoading } = useQuery<TelegramBotInfo | undefined>({
    queryKey: ['telegram-bot-info'],
    queryFn: telegramApi.getBotInfo,
    retry: false,
  });

  const form = useForm<{ chatId: string; chatTitle: string; chatType: string }>({
    defaultValues: { chatId: '', chatTitle: '', chatType: 'private' },
    onSubmit: async (values) => {
      // Chat ID is required only for non-private chats — for private chats we'll use the linked telegram_user_id
      if (values.chatType !== 'private' && !(values.chatId ?? '').trim()) return;
      if (!isTelegramLinked) {
        await showAlert(t('telegram.add_dialog.private_chat_require_link'));
        return;
      }

      setIsSubmitting(true);
      try {
        const payloadChatId =
          values.chatType === 'private'
            ? (user?.telegram_user_id ?? (values.chatId ?? '').trim())
            : (values.chatId ?? '').trim();
        await telegramApi.create(
          {
            telegram_chat_id: payloadChatId,
            telegram_chat_title: (values.chatTitle ?? '').trim() || undefined,
            telegram_chat_type: values.chatType ?? 'private',
          },
          ownerId,
        );

        // Refresh global and owner-scoped lists
        queryClient.invalidateQueries({ queryKey: ['telegram-integrations'] });
        if (ownerId) {
          queryClient.invalidateQueries({ queryKey: ['telegram-integrations', ownerId] });
        }

        onClose();
        reset({ chatId: '', chatTitle: '', chatType: 'private' });
      } catch (err: unknown) {
        let message = t('telegram.add_dialog.create_error');
        if (typeof err === 'string') {
          message = err;
        } else if (err && typeof err === 'object') {
          const e = err as Record<string, unknown>;
          if (typeof e['message'] === 'string') {
            message = e['message'] as string;
          } else if (typeof e['error'] === 'string') {
            message = e['error'] as string;
          }
        }
        await showAlert(message);
      } finally {
        setIsSubmitting(false);
      }
    },
  });
  const { setValue, reset } = form;
  const chatId = useWatch(form, 'chatId') ?? '';
  const chatTitle = useWatch(form, 'chatTitle') ?? '';
  const chatType = useWatch(form, 'chatType') ?? 'private';
  const Field = (form.Field ?? (() => null)) as ComponentType<{
    name?: string;
    children?: (field: AnyFieldApi) => unknown;
    validators?: {
      onChange?: (args: { value: unknown }) => string | undefined;
    };
  }>;

  if (!open) return null;

  // displayChatId and isChatIdRequired are inlined where used to avoid unused variable warnings

  // Client-side validation for chat id based on type
  const chatIdToValidate = chatType === 'private' ? (user?.telegram_user_id ?? '') : chatId.trim();
  const chatIdError = (() => {
    if (chatType === 'group') {
      if (
        !/^[0-9-]+$/.test(chatIdToValidate) ||
        !/^-?[0-9]+$/.test(chatIdToValidate) ||
        !chatIdToValidate.startsWith('-')
      ) {
        return t('telegram.add_dialog.group_id_error');
      }
    }
    if (chatType === 'supergroup' || chatType === 'channel') {
      if (!/^-100[0-9]+$/.test(chatIdToValidate)) {
        return t('telegram.add_dialog.supergroup_id_error');
      }
    }
    if (chatType !== 'private' && !chatIdToValidate) {
      return t('telegram.add_dialog.chat_id_required');
    }
    return '';
  })();

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="w-full max-w-md rounded-lg bg-background p-6 shadow-lg">
        <h2 className="text-lg font-semibold mb-4">{t('telegram.add_dialog.title')}</h2>
        <form
          onSubmit={(e) => {
            e.preventDefault();
            form.handleSubmit?.();
          }}
          className="space-y-4"
        >
          <div>
            <div className="mt-2 text-sm text-muted-foreground">
              {botLoading ? (
                <>{t('telegram.add_dialog.bot_loading')}</>
              ) : botInfo?.username ? (
                <>
                  {t('telegram.add_dialog.bot_instructions', {
                    bot: '@' + botInfo.username.replace(/^@/, ''),
                  })}
                </>
              ) : (
                <>{t('telegram.integrations_block.bot_not_configured')}</>
              )}
            </div>
            {!isTelegramLinked ? (
              <div className="mt-2 space-y-2 text-sm text-muted-foreground">
                <div>{t('telegram.add_dialog.private_chat_require_link')}</div>
                <div>
                  <Button asChild variant="outline">
                    <a href="/settings">{t('telegram.add_dialog.link_settings')}</a>
                  </Button>
                </div>
              </div>
            ) : null}
          </div>
          <div>
            <label className="text-sm font-medium">
              {t('telegram.add_dialog.name_placeholder')}
            </label>
            {Field ? (
              <Field name="chatTitle">
                {(field: AnyFieldApi) => (
                  <input
                    id={field.name}
                    name={field.name}
                    type="text"
                    value={field.state?.value ?? ''}
                    onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
                      field.handleChange?.(e.target.value)
                    }
                    placeholder={t('telegram.add_dialog.name_example')}
                    className="mt-1 w-full rounded-md border bg-background px-3 py-2 text-sm"
                  />
                )}
              </Field>
            ) : (
              <input
                type="text"
                value={chatTitle}
                onChange={(e) => setValue('chatTitle', e.target.value)}
                placeholder={t('telegram.add_dialog.name_example')}
                className="mt-1 w-full rounded-md border bg-background px-3 py-2 text-sm"
              />
            )}
          </div>
          <div>
            <label className="text-sm font-medium">
              {t('telegram.add_dialog.chat_type_label')}
            </label>
            {Field ? (
              <Field name="chatType">
                {(field: AnyFieldApi) => (
                  <select
                    id={field.name}
                    name={field.name}
                    value={field.state?.value ?? 'private'}
                    onChange={(e: React.ChangeEvent<HTMLSelectElement>) =>
                      field.handleChange?.(e.target.value)
                    }
                    className="mt-1 w-full rounded-md border bg-background px-3 py-2 text-sm"
                  >
                    <option value="private">{t('telegram.add_dialog.chat_type.private')}</option>
                    <option value="group">{t('telegram.add_dialog.chat_type.group')}</option>
                    <option value="supergroup">
                      {t('telegram.add_dialog.chat_type.supergroup')}
                    </option>
                  </select>
                )}
              </Field>
            ) : (
              <select
                value={chatType}
                onChange={(e) => setValue('chatType', e.target.value)}
                className="mt-1 w-full rounded-md border bg-background px-3 py-2 text-sm"
              >
                <option value="private">{t('telegram.add_dialog.chat_type.private')}</option>
                <option value="group">{t('telegram.add_dialog.chat_type.group')}</option>
                <option value="supergroup">{t('telegram.add_dialog.chat_type.supergroup')}</option>
              </select>
            )}
          </div>
          <div>
            <label className="text-sm font-medium">
              {t('telegram.add_dialog.chat_id_label')} {chatType !== 'private' ? '*' : ''}
            </label>
            {Field ? (
              <Field
                name="chatId"
                validators={{
                  onChange: ({ value }: { value: unknown }) => {
                    const formValues = form.getValues?.() as unknown;
                    const type =
                      (formValues as { chatType?: string } | undefined)?.chatType ?? 'private';
                    const val =
                      type === 'private'
                        ? (user?.telegram_user_id ?? '')
                        : String(value ?? '').trim();
                    if (type !== 'private' && !val) {
                      return t('telegram.add_dialog.chat_id_required');
                    }
                    if (type === 'group') {
                      if (
                        !/^[0-9-]+$/.test(val) ||
                        !/^-?[0-9]+$/.test(val) ||
                        !val.startsWith('-')
                      ) {
                        return t('telegram.add_dialog.group_id_error');
                      }
                    }
                    if (type === 'supergroup' || type === 'channel') {
                      if (!/^-100[0-9]+$/.test(val)) {
                        return t('telegram.add_dialog.supergroup_id_error');
                      }
                    }
                    return undefined;
                  },
                }}
              >
                {(field: AnyFieldApi) => (
                  <div>
                    <input
                      id={field.name}
                      name={field.name}
                      type="text"
                      value={
                        chatType === 'private'
                          ? (user?.telegram_user_id ?? '')
                          : (field.state?.value ?? '')
                      }
                      onChange={(e: React.ChangeEvent<HTMLInputElement>) => {
                        if (chatType !== 'private') field.handleChange?.(e.target.value);
                      }}
                      onBlur={field.handleBlur}
                      placeholder={t('telegram.add_dialog.chat_id_example')}
                      className="mt-1 w-full rounded-md border bg-background px-3 py-2 text-sm"
                      required={isTelegramLinked && chatType !== 'private'}
                      disabled={chatType === 'private' || !isTelegramLinked}
                    />
                    {/* validation / helper text */}
                    {!field.state.meta.isValid &&
                    field.state.meta.errors &&
                    field.state.meta.errors.length > 0 ? (
                      <div className="mt-1 text-sm text-red-500" role="alert">
                        {field.state.meta.errors.join(', ')}
                      </div>
                    ) : chatType === 'private' ? (
                      user?.telegram_user_id ? (
                        <p className="mt-1 text-xs">
                          {t('telegram.add_dialog.private_chat_notice', {
                            id: user.telegram_user_id,
                          })}
                        </p>
                      ) : (
                        <p className="mt-1 text-xs">
                          {t('telegram.add_dialog.private_chat_require_link')}
                        </p>
                      )
                    ) : botLoading ? (
                      <p className="mt-1 text-xs">{t('telegram.add_dialog.bot_loading')}</p>
                    ) : botInfo?.username ? (
                      <p className="mt-1 text-xs">
                        {t('telegram.add_dialog.bot_instructions', {
                          bot: '@' + botInfo.username.replace(/^@/, ''),
                        })}
                      </p>
                    ) : (
                      <p className="mt-1 text-xs">
                        {t('telegram.add_dialog.bot_instructions', { bot: '' })}
                      </p>
                    )}
                  </div>
                )}
              </Field>
            ) : (
              <div>
                <input
                  type="text"
                  value={chatType === 'private' ? (user?.telegram_user_id ?? '') : chatId}
                  onChange={(e) => {
                    if (chatType !== 'private') setValue('chatId', e.target.value);
                  }}
                  placeholder={t('telegram.add_dialog.chat_id_example')}
                  className="mt-1 w-full rounded-md border bg-background px-3 py-2 text-sm"
                  required={isTelegramLinked && chatType !== 'private'}
                  disabled={chatType === 'private' || !isTelegramLinked}
                />
                {/* fallback validation for non-Field mode */}
                {(() => {
                  const val =
                    chatType === 'private' ? (user?.telegram_user_id ?? '') : chatId.trim();
                  if (chatType === 'group') {
                    if (!/^[0-9-]+$/.test(val) || !/^-?[0-9]+$/.test(val) || !val.startsWith('-')) {
                      return (
                        <span className="text-sm text-red-500">
                          {t('telegram.add_dialog.group_id_error')}
                        </span>
                      );
                    }
                  }
                  if (chatType === 'supergroup' || chatType === 'channel') {
                    if (!/^-100[0-9]+$/.test(val)) {
                      return (
                        <span className="text-sm text-red-500">
                          {t('telegram.add_dialog.supergroup_id_error')}
                        </span>
                      );
                    }
                  }
                  if (chatType !== 'private' && !val) {
                    return (
                      <span className="text-sm text-red-500">
                        {t('telegram.add_dialog.chat_id_required')}
                      </span>
                    );
                  }
                  return chatType === 'private' ? (
                    user?.telegram_user_id ? (
                      <p className="mt-1 text-xs">
                        {t('telegram.add_dialog.private_chat_notice', {
                          id: user.telegram_user_id,
                        })}
                      </p>
                    ) : (
                      <p className="mt-1 text-xs">
                        {t('telegram.add_dialog.private_chat_require_link')}
                      </p>
                    )
                  ) : botLoading ? (
                    <p className="mt-1 text-xs">{t('telegram.add_dialog.bot_loading')}</p>
                  ) : botInfo?.username ? (
                    <p className="mt-1 text-xs">
                      {t('telegram.add_dialog.bot_instructions', {
                        bot: '@' + botInfo.username.replace(/^@/, ''),
                      })}
                    </p>
                  ) : (
                    <p className="mt-1 text-xs">
                      {t('telegram.add_dialog.bot_instructions', { bot: '' })}
                    </p>
                  );
                })()}
              </div>
            )}
            {(chatType === 'group' || chatType === 'supergroup' || chatType === 'channel') &&
            !chatIdError ? (
              <p className="mt-1 text-xs text-muted-foreground">
                {t('telegram.add_dialog.admin_requirement')}
              </p>
            ) : null}
          </div>
          <div className="flex justify-end gap-2">
            <Button type="button" variant="ghost" onClick={onClose}>
              {t('telegram.add_dialog.cancel')}
            </Button>
            <Button type="submit" disabled={isSubmitting || !isTelegramLinked || !!chatIdError}>
              {isSubmitting ? <Loader2 className="h-4 w-4 animate-spin mr-2" /> : null}
              {t('telegram.add_dialog.add')}
            </Button>
          </div>
        </form>
      </div>
    </div>
  );
}
