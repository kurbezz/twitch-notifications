import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { useAuth } from '@/hooks/useAuth';
import { setAuthToken } from '@/lib/api';
import { Loader2, CheckCircle, XCircle } from 'lucide-react';
import { Button } from '@/components/ui/button';

type CallbackStatus = 'loading' | 'success' | 'error';

export function AuthCallbackPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const { refreshUser } = useAuth();
  const [status, setStatus] = useState<CallbackStatus>('loading');
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  useEffect(() => {
    const handleCallback = async () => {
      // Check for error in URL params
      const error = searchParams.get('error');
      const errorDescription = searchParams.get('error_description');

      if (error) {
        setStatus('error');
        setErrorMessage(errorDescription || error);
        return;
      }

      // Try to extract an access token from the URL fragment (returned by backend).
      // Example: /auth/callback#access_token=...&token_type=Bearer&expires_at=...&redirect_to=...
      const hash = window.location.hash.replace(/^#/, '');
      const fragment = new URLSearchParams(hash);
      const accessToken = fragment.get('access_token');
      const expiresAtStr = fragment.get('expires_at');
      const redirectTo = fragment.get('redirect_to');

      if (accessToken) {
        try {
          // The backend percent-encodes the token; decode before storing.
          const decodedToken = decodeURIComponent(accessToken);
          const expiresAt = expiresAtStr ? Number(expiresAtStr) : null;
          // Persist token in API client (and localStorage) so subsequent requests
          // include the Authorization header.
          setAuthToken(decodedToken, expiresAt);
          // Remove token fragment from URL to avoid leaking it in history
          window.history.replaceState(null, '', window.location.pathname + window.location.search);
        } catch (e) {
          // Non-fatal: we'll still attempt a refresh below and surface an error if it fails.
          console.error('Failed to parse access token from fragment', e);
        }
      }

      // Try to refresh the user to verify authentication (will use Authorization header if token present)
      try {
        await refreshUser();
        setStatus('success');

        // Redirect to the final destination after a short delay
        // Use redirect_to from fragment if provided, otherwise go to dashboard
        setTimeout(() => {
          const finalRedirect = redirectTo || '/dashboard';
          navigate(finalRedirect, { replace: true });
        }, 1500);
      } catch {
        setStatus('error');
        setErrorMessage(t('auth_callback.error_desc'));
      }
    };

    handleCallback();
  }, [searchParams, refreshUser, navigate]);

  return (
    <div className="min-h-screen flex items-center justify-center bg-background">
      <div className="w-full max-w-md p-8 text-center">
        {status === 'loading' && (
          <div className="space-y-4">
            <Loader2 className="h-12 w-12 animate-spin text-twitch mx-auto" />
            <h1 className="text-xl font-semibold">{t('auth_callback.logging_in')}</h1>
            <p className="text-muted-foreground">{t('auth_callback.please_wait')}</p>
          </div>
        )}

        {status === 'success' && (
          <div className="space-y-4">
            <div className="mx-auto flex h-16 w-16 items-center justify-center rounded-full bg-green-100 dark:bg-green-900/30">
              <CheckCircle className="h-10 w-10 text-green-500" />
            </div>
            <h1 className="text-xl font-semibold">{t('auth_callback.success')}</h1>
            <p className="text-muted-foreground">{t('auth_callback.redirecting')}</p>
          </div>
        )}

        {status === 'error' && (
          <div className="space-y-4">
            <div className="mx-auto flex h-16 w-16 items-center justify-center rounded-full bg-red-100 dark:bg-red-900/30">
              <XCircle className="h-10 w-10 text-red-500" />
            </div>
            <h1 className="text-xl font-semibold">{t('auth_callback.error')}</h1>
            <p className="text-muted-foreground">{errorMessage || t('auth_callback.error_desc')}</p>
            <div className="flex flex-col gap-2 mt-6">
              <Button onClick={() => navigate('/login', { replace: true })} variant="default">
                {t('auth_callback.try_again')}
              </Button>
              <Button onClick={() => navigate('/', { replace: true })} variant="outline">
                {t('auth_callback.home')}
              </Button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

export default AuthCallbackPage;
