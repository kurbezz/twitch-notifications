import { useAuth } from '@/hooks/useAuth';
import { Button } from '@/components/ui/button';
import { MessageSquare, Bell, Zap, Shield, ArrowRight } from 'lucide-react';
import { useTranslation } from 'react-i18next';

// Twitch icon component
function TwitchIcon({ className }: { className?: string }) {
  return (
    <svg
      className={className}
      viewBox="0 0 24 24"
      fill="currentColor"
      xmlns="http://www.w3.org/2000/svg"
    >
      <path d="M11.571 4.714h1.715v5.143H11.57zm4.715 0H18v5.143h-1.714zM6 0L1.714 4.286v15.428h5.143V24l4.286-4.286h3.428L22.286 12V0zm14.571 11.143l-3.428 3.428h-3.429l-3 3v-3H6.857V1.714h13.714z" />
    </svg>
  );
}

// Features are provided via translations inside the component

export function LoginPage() {
  const { login } = useAuth();
  const { t } = useTranslation();

  const features = [
    {
      icon: <Bell className="h-6 w-6" />,
      title: t('login.features.stream_notifications.title'),
      description: t('login.features.stream_notifications.description'),
    },
    {
      icon: <Zap className="h-6 w-6" />,
      title: t('login.features.rewards.title'),
      description: t('login.features.rewards.description'),
    },
    {
      icon: <MessageSquare className="h-6 w-6" />,
      title: t('login.features.tg_dc.title'),
      description: t('login.features.tg_dc.description'),
    },
    {
      icon: <Shield className="h-6 w-6" />,
      title: t('login.features.security.title'),
      description: t('login.features.security.description'),
    },
  ];

  return (
    <div className="min-h-screen bg-gradient-to-br from-background via-background to-twitch/5">
      {/* Navigation */}
      <header className="absolute top-0 left-0 right-0 z-10">
        <div className="container flex h-16 items-center justify-between px-4">
          <div className="flex items-center gap-2">
            <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-twitch">
              <MessageSquare className="h-5 w-5 text-white" />
            </div>
            <span className="font-semibold">{t('app.name')}</span>
          </div>
        </div>
      </header>

      {/* Main content */}
      <main className="flex min-h-screen flex-col items-center justify-center px-4 pt-16">
        <div className="w-full max-w-md space-y-8 text-center">
          {/* Logo and title */}
          <div className="space-y-4">
            <div className="mx-auto flex h-20 w-20 items-center justify-center rounded-2xl bg-twitch shadow-lg shadow-twitch/25">
              <MessageSquare className="h-10 w-10 text-white" />
            </div>
            <h1 className="text-3xl font-bold tracking-tight">{t('login.title')}</h1>
            <p className="text-muted-foreground">{t('login.subtitle')}</p>
          </div>

          {/* Login button */}
          <div className="space-y-4">
            <Button
              onClick={login}
              variant="twitch"
              size="lg"
              className="w-full gap-2 py-6 text-lg shadow-lg shadow-twitch/25 hover:shadow-xl hover:shadow-twitch/30 transition-all"
            >
              <TwitchIcon className="h-5 w-5" />
              {t('login.login_button')}
              <ArrowRight className="h-4 w-4 ml-2" />
            </Button>

            <p
              className="text-xs text-muted-foreground"
              dangerouslySetInnerHTML={{
                __html: t('login.accept_terms', {
                  terms: `<a href="/terms" class="underline hover:text-foreground">${t('login.terms_text')}</a>`,
                  privacy: `<a href="/privacy" class="underline hover:text-foreground">${t('login.privacy_text')}</a>`,
                }),
              }}
            />
          </div>
        </div>

        {/* Features section */}
        <div className="mt-16 w-full max-w-4xl px-4 pb-16">
          <h2 className="mb-8 text-center text-xl font-semibold text-muted-foreground">
            {t('login.features_title')}
          </h2>
          <div className="grid gap-6 sm:grid-cols-2">
            {features.map((feature, index) => (
              <div
                key={index}
                className="group rounded-xl border bg-card p-6 shadow-sm transition-all hover:shadow-md hover:border-twitch/30"
              >
                <div className="mb-4 flex h-12 w-12 items-center justify-center rounded-lg bg-twitch/10 text-twitch group-hover:bg-twitch group-hover:text-white transition-colors">
                  {feature.icon}
                </div>
                <h3 className="mb-2 font-semibold">{feature.title}</h3>
                <p className="text-sm text-muted-foreground">{feature.description}</p>
              </div>
            ))}
          </div>
        </div>
      </main>

      {/* Footer */}
      <footer className="absolute bottom-0 left-0 right-0 py-4">
        <div className="container text-center text-sm text-muted-foreground">
          <p>{t('login.footer')}</p>
        </div>
      </footer>
    </div>
  );
}

export default LoginPage;
