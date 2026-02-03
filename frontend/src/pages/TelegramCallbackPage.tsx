import { useEffect, useRef, useState } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import { telegramApi } from '@/lib/api';
import { useQueryClient } from '@tanstack/react-query';
import { Loader2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { useAuth } from '@/hooks/useAuth';
import { useTranslation } from 'react-i18next';

/**
 * TelegramCallbackPage
 *
 * This page handles redirects from the Telegram Login Widget. Telegram will
 * redirect the browser to this page with query parameters (id, username, auth_date, hash, ...).
 *
 * The page:
 *  - Parses the query parameters
 *  - Sends them to the backend via `telegramApi.link(...)` to verify the payload and create an integration
 *  - Invalidates the telegram integrations query so the UI refreshes
 *  - Redirects the user back to `/integrations` on success
 *
 * Note: This page is intended to be protected (user must be logged in). Ensure you mount it
 * under a protected route so unauthenticated redirects to login happen automatically.
 */
export default function TelegramCallbackPage() {
  const location = useLocation();
  const navigate = useNavigate();
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const { refreshUser } = useAuth();

  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<boolean>(false);
  const [isLinking, setIsLinking] = useState(false);
  const linkInProgressRef = useRef(false);

  useEffect(() => {
    const params = new URLSearchParams(location.search);

    const id = params.get('id');
    const hash = params.get('hash');
    const authDate = params.get('auth_date');
    const redirectTo = params.get('redirect_to') || '/integrations';

    if (!id || !hash || !authDate) {
      setError(t('telegram_callback.insufficient_data'));
      setLoading(false);
      return;
    }

    const payload = {
      id,
      first_name: params.get('first_name') || undefined,
      last_name: params.get('last_name') || undefined,
      username: params.get('username') || undefined,
      photo_url: params.get('photo_url') || undefined,
      auth_date: Number(authDate),
      hash,
    };

    const sessionKey = `telegram.link:${payload.id}:${payload.auth_date}:${payload.hash}`;
    let interval: ReturnType<typeof setInterval> | null = null;
    let cancelled = false;

    // Helper that polls the sessionKey until it's 'done' or removed
    async function waitForDoneAndRedirect() {
      setLoading(true);
      interval = setInterval(async () => {
        try {
          const v = sessionStorage.getItem(sessionKey);
          if (v === 'done') {
            if (interval) {
              clearInterval(interval);
              interval = null;
            }
            setSuccess(true);
            setLoading(false);
            try {
              queryClient.invalidateQueries({ queryKey: ['telegram-integrations'] });
              await refreshUser();
            } catch {
              // ignore
            }
            if (!cancelled) navigate(redirectTo, { replace: true });
          } else if (!v) {
            if (interval) {
              clearInterval(interval);
              interval = null;
            }
            setLoading(false);
          }
        } catch {
          if (interval) {
            clearInterval(interval);
            interval = null;
          }
          setLoading(false);
        }
      }, 400);
    }

    (async () => {
      try {
        const state = sessionStorage.getItem(sessionKey);
        if (state === 'done') {
          // Already succeeded elsewhere - refresh and redirect immediately
          setSuccess(true);
          setLoading(false);
          try {
            queryClient.invalidateQueries({ queryKey: ['telegram-integrations'] });
            await refreshUser();
          } catch {
            // ignore
          }
          if (!cancelled) navigate(redirectTo, { replace: true });
          return;
        }

        if (state === 'inprogress') {
          // Another tab/instance is handling it — wait for it to finish
          await waitForDoneAndRedirect();
          return;
        }

        // No one is handling this payload — attempt to link now
        if (linkInProgressRef.current) return;
        linkInProgressRef.current = true;
        setIsLinking(true);
        setLoading(true);

        try {
          try {
            sessionStorage.setItem(sessionKey, 'inprogress');
          } catch {
            // ignore storage failures (private mode, etc)
          }

          await telegramApi.link(payload);

          try {
            sessionStorage.setItem(sessionKey, 'done');
          } catch {
            // ignore storage errors
          }

          queryClient.invalidateQueries({ queryKey: ['telegram-integrations'] });
          setSuccess(true);

          try {
            await refreshUser();
          } catch {
            // ignore refresh errors
          }

          if (!cancelled) navigate(redirectTo, { replace: true });
        } catch (err: unknown) {
          try {
            sessionStorage.removeItem(sessionKey);
          } catch {
            // ignore
          }

          let message = t('telegram_callback.link_error_generic');

          if (typeof err === 'string') {
            message = err;
          } else if (err && typeof err === 'object') {
            const e = err as Record<string, unknown>;

            if (e.error && typeof e.error === 'object') {
              const terr = e.error as Record<string, unknown>;
              const code = (terr['code'] as string | undefined) ?? undefined;
              const details = (terr['details'] as Record<string, unknown> | undefined) ?? undefined;

              if (code === 'RATE_LIMITED') {
                const retry =
                  details && typeof details['retry_after_seconds'] === 'number'
                    ? (details['retry_after_seconds'] as number)
                    : null;
                if (retry && retry > 0) {
                  message = t('telegram_callback.rate_limited_with_retry', { seconds: retry });
                } else {
                  message = t('telegram_callback.rate_limited');
                }
              } else if (typeof terr['message'] === 'string') {
                message = terr['message'] as string;
              } else if (typeof terr['error'] === 'string') {
                message = terr['error'] as string;
              } else {
                try {
                  message = JSON.stringify(terr);
                } catch {
                  // keep default message
                }
              }
            } else if (typeof e.message === 'string') {
              message = e.message;
            } else if (typeof e.error === 'string') {
              message = e.error;
            } else if (typeof e.msg === 'string') {
              message = e.msg;
            } else {
              try {
                message = JSON.stringify(e);
              } catch {
                // keep default message if serialization fails
              }
            }
          }

          setError(message);
        } finally {
          linkInProgressRef.current = false;
          setIsLinking(false);
          setLoading(false);
        }
      } catch {
        setError(t('telegram_callback.processing_error'));
        setLoading(false);
      }
    })();

    return () => {
      cancelled = true;
      if (interval) clearInterval(interval);
    };
  }, [location.search, navigate, queryClient, refreshUser, t]);

  return (
    <div className="flex items-center justify-center min-h-[60vh] p-6">
      <div className="max-w-lg w-full rounded-lg border bg-background p-8 text-center shadow">
        {loading ? (
          <div className="flex flex-col items-center gap-4">
            <Loader2 className="h-8 w-8 animate-spin text-twitch" />
            <p className="text-sm text-muted-foreground">{t('telegram_callback.processing')}</p>
          </div>
        ) : error ? (
          <div className="space-y-4">
            <h2 className="text-lg font-semibold text-destructive">
              {t('telegram_callback.failed_connect')}
            </h2>
            <p className="text-sm text-muted-foreground break-words">{error}</p>
            <div className="flex items-center justify-center gap-2">
              <Button onClick={() => navigate('/integrations')}>
                {t('telegram_callback.return_to_integrations')}
              </Button>
              <Button variant="ghost" onClick={() => window.location.reload()} disabled={isLinking}>
                {isLinking ? <Loader2 className="h-4 w-4 mr-2 animate-spin" /> : null}
                {t('telegram_callback.retry')}
              </Button>
            </div>
          </div>
        ) : success ? (
          <div className="space-y-4">
            <h2 className="text-lg font-semibold text-foreground">
              {t('telegram_callback.success')}
            </h2>
            <p className="text-sm text-muted-foreground">{t('telegram_callback.redirecting')}</p>
            <div className="flex items-center justify-center">
              <Button onClick={() => navigate('/integrations')}>
                {t('telegram_callback.return_to_integrations')}
              </Button>
            </div>
          </div>
        ) : (
          <div>
            <p className="text-sm text-muted-foreground">{t('telegram_callback.ready')}</p>
          </div>
        )}
      </div>
    </div>
  );
}
