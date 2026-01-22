import { useAuth } from '@/hooks/useAuth';
import { Button } from '@/components/ui/button';
import { Bell, Calendar, Gift, MessageSquare, Zap } from 'lucide-react';
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

export function HomePage() {
  const { login, isLoading } = useAuth();
  const { t } = useTranslation();

  return (
    <div className="min-h-screen bg-background">
      {/* Header */}
      <header className="border-b">
        <div className="container flex h-16 items-center justify-between px-4">
          <div className="flex items-center gap-2">
            <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-twitch">
              <MessageSquare className="h-5 w-5 text-white" />
            </div>
            <span className="font-semibold">{t('app.name')}</span>
          </div>
          <Button onClick={login} variant="twitch" disabled={isLoading}>
            <TwitchIcon className="mr-2 h-4 w-4" />
            {t('home.hero.login')}
          </Button>
        </div>
      </header>

      {/* Hero Section */}
      <section className="container px-4 py-24 text-center">
        <div className="mx-auto max-w-3xl space-y-6">
          <div className="inline-flex items-center rounded-full border px-3 py-1 text-sm">
            <Zap className="mr-2 h-4 w-4 text-twitch" />
            {t('home.tagline')}
          </div>
          <h1 className="text-4xl font-bold tracking-tight sm:text-5xl md:text-6xl">
            {t('home.hero.title_prefix')}{' '}
            <span className="text-gradient">{t('home.hero.title_strong')}</span>
          </h1>
          <p className="mx-auto max-w-xl text-lg text-muted-foreground">
            {t('home.hero.subtitle')}
          </p>
          <div className="flex flex-col sm:flex-row items-center justify-center gap-4">
            <Button onClick={login} size="lg" variant="twitch" disabled={isLoading}>
              <TwitchIcon className="mr-2 h-5 w-5" />
              {t('home.hero.start_now')}
            </Button>
            <Button variant="outline" size="lg" asChild>
              <a href="#features">{t('home.hero.learn_more')}</a>
            </Button>
          </div>
        </div>
      </section>

      {/* Features Section */}
      <section id="features" className="border-t bg-muted/50 py-24">
        <div className="container px-4">
          <div className="text-center mb-16">
            <h2 className="text-3xl font-bold tracking-tight">{t('home.features.title')}</h2>
            <p className="mt-4 text-lg text-muted-foreground">{t('home.features.subtitle')}</p>
          </div>

          <div className="grid gap-8 md:grid-cols-2 lg:grid-cols-3">
            {/* Feature 1 */}
            <div className="rounded-xl border bg-card p-6 shadow-sm card-hover">
              <div className="flex h-12 w-12 items-center justify-center rounded-lg bg-twitch/10">
                <Bell className="h-6 w-6 text-twitch" />
              </div>
              <h3 className="mt-4 font-semibold">
                {t('home.features.stream_notifications.title')}
              </h3>
              <p className="mt-2 text-sm text-muted-foreground">
                {t('home.features.stream_notifications.description')}
              </p>
            </div>

            {/* Feature 2 */}
            <div className="rounded-xl border bg-card p-6 shadow-sm card-hover">
              <div className="flex h-12 w-12 items-center justify-center rounded-lg bg-blue-500/10">
                <MessageSquare className="h-6 w-6 text-blue-500" />
              </div>
              <h3 className="mt-4 font-semibold">{t('home.features.telegram.title')}</h3>
              <p className="mt-2 text-sm text-muted-foreground">
                {t('home.features.telegram.description')}
              </p>
            </div>

            {/* Feature 3 */}
            <div className="rounded-xl border bg-card p-6 shadow-sm card-hover">
              <div className="flex h-12 w-12 items-center justify-center rounded-lg bg-indigo-500/10">
                <Zap className="h-6 w-6 text-indigo-500" />
              </div>
              <h3 className="mt-4 font-semibold">{t('home.features.discord.title')}</h3>
              <p className="mt-2 text-sm text-muted-foreground">
                {t('home.features.discord.description')}
              </p>
            </div>

            {/* Feature 4 */}
            <div className="rounded-xl border bg-card p-6 shadow-sm card-hover">
              <div className="flex h-12 w-12 items-center justify-center rounded-lg bg-green-500/10">
                <Gift className="h-6 w-6 text-green-500" />
              </div>
              <h3 className="mt-4 font-semibold">{t('home.features.rewards.title')}</h3>
              <p className="mt-2 text-sm text-muted-foreground">
                {t('home.features.rewards.description')}
              </p>
            </div>

            {/* Feature 5 */}
            <div className="rounded-xl border bg-card p-6 shadow-sm card-hover">
              <div className="flex h-12 w-12 items-center justify-center rounded-lg bg-orange-500/10">
                <Calendar className="h-6 w-6 text-orange-500" />
              </div>
              <h3 className="mt-4 font-semibold">{t('home.features.calendar.title')}</h3>
              <p className="mt-2 text-sm text-muted-foreground">
                {t('home.features.calendar.description')}
              </p>
            </div>

            {/* Feature 6 */}
            <div className="rounded-xl border bg-card p-6 shadow-sm card-hover">
              <div className="flex h-12 w-12 items-center justify-center rounded-lg bg-purple-500/10">
                <MessageSquare className="h-6 w-6 text-purple-500" />
              </div>
              <h3 className="mt-4 font-semibold">{t('home.features.custom_messages.title')}</h3>
              <p className="mt-2 text-sm text-muted-foreground">
                {t('home.features.custom_messages.description')}
              </p>
            </div>
          </div>
        </div>
      </section>

      {/* CTA Section */}
      <section className="border-t py-24">
        <div className="container px-4 text-center">
          <h2 className="text-3xl font-bold tracking-tight">{t('home.cta.ready')}</h2>
          <p className="mt-4 text-lg text-muted-foreground">{t('home.cta.subtitle')}</p>
          <Button onClick={login} size="lg" variant="twitch" className="mt-8" disabled={isLoading}>
            <TwitchIcon className="mr-2 h-5 w-5" />
            {t('home.hero.login')}
          </Button>
        </div>
      </section>

      {/* Footer */}
      <footer className="border-t py-8">
        <div className="container px-4 flex flex-col sm:flex-row items-center justify-between gap-4 text-sm text-muted-foreground">
          <p>{t('layout.footer.copyright', { year: new Date().getFullYear() })}</p>
          <div className="flex items-center gap-4">
            <a
              href="https://github.com"
              target="_blank"
              rel="noopener noreferrer"
              className="hover:text-foreground transition-colors"
            >
              {t('layout.footer.github')}
            </a>
            <a
              href="https://twitch.tv"
              target="_blank"
              rel="noopener noreferrer"
              className="hover:text-foreground transition-colors"
            >
              {t('layout.footer.twitch')}
            </a>
          </div>
        </div>
      </footer>
    </div>
  );
}
