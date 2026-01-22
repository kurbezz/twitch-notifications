import { Link, useLocation, useNavigate } from 'react-router-dom';
import { useAuth } from '@/hooks/useAuth';
import { cn, getTwitchProfileImageUrl } from '@/lib/utils';
import { Bell, Home, LogOut, Menu, MessageSquare, Settings, X, Zap } from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { setLanguage } from '@/i18n';

interface LayoutProps {
  children: React.ReactNode;
}

interface NavItem {
  label: string;
  href: string;
  icon: React.ReactNode;
}

// nav items are defined inside the component to allow translation via useTranslation()

export function Layout({ children }: LayoutProps) {
  const { t, i18n } = useTranslation();
  const { user, logout } = useAuth();
  const location = useLocation();
  const navigate = useNavigate();
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);

  const navItems: NavItem[] = [
    {
      label: t('layout.nav.home'),
      href: '/dashboard',
      icon: <Home className="h-5 w-5" />,
    },
    {
      label: t('layout.nav.integrations'),
      href: '/integrations',
      icon: <Zap className="h-5 w-5" />,
    },
    {
      label: t('layout.nav.settings'),
      href: '/settings',
      icon: <Settings className="h-5 w-5" />,
    },
    {
      label: t('layout.nav.history'),
      href: '/notifications',
      icon: <Bell className="h-5 w-5" />,
    },
  ];

  const handleLogout = async () => {
    await logout();
    navigate('/');
  };

  return (
    <div className="min-h-screen bg-background">
      {/* Top navigation bar */}
      <header className="sticky top-0 z-50 border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
        <div className="container flex h-16 items-center justify-between px-4">
          {/* Logo */}
          <Link to="/dashboard" className="flex items-center gap-2">
            <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-twitch">
              <MessageSquare className="h-5 w-5 text-white" />
            </div>
            <span className="hidden font-semibold sm:inline-block">{t('app.name')}</span>
          </Link>

          {/* Desktop navigation */}
          <nav className="hidden md:flex items-center gap-1">
            {navItems.map((item) => (
              <Link
                key={item.href}
                to={item.href}
                className={cn(
                  'flex items-center gap-2 rounded-lg px-3 py-2 text-sm font-medium transition-colors',
                  location.pathname === item.href
                    ? 'bg-accent text-accent-foreground'
                    : 'text-muted-foreground hover:bg-accent hover:text-accent-foreground',
                )}
              >
                {item.icon}
                {item.label}
              </Link>
            ))}
          </nav>

          {/* User menu */}
          <div className="flex items-center gap-4">
            {/* Language selector */}
            <div className="hidden sm:flex items-center gap-2">
              <button
                onClick={() => setLanguage('ru')}
                className={cn(
                  'rounded px-2 py-1 text-xs font-medium',
                  i18n.language?.startsWith('ru')
                    ? 'bg-accent text-accent-foreground'
                    : 'text-muted-foreground hover:bg-muted',
                )}
                aria-label={t('user_settings.language_ru')}
              >
                RU
              </button>
              <button
                onClick={() => setLanguage('en')}
                className={cn(
                  'rounded px-2 py-1 text-xs font-medium',
                  i18n.language?.startsWith('en')
                    ? 'bg-accent text-accent-foreground'
                    : 'text-muted-foreground hover:bg-muted',
                )}
                aria-label={t('user_settings.language_en')}
              >
                EN
              </button>
            </div>

            {user && (
              <div className="hidden md:flex items-center gap-3">
                <Link
                  to="/settings"
                  className="flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground transition-colors"
                >
                  <img
                    src={getTwitchProfileImageUrl(user.twitch_profile_image_url, 70)}
                    alt={user.twitch_display_name}
                    className="h-8 w-8 rounded-full ring-2 ring-twitch/20"
                  />
                  <span className="font-medium">{user.twitch_display_name}</span>
                </Link>
                <button
                  onClick={handleLogout}
                  className="flex items-center gap-2 rounded-lg px-3 py-2 text-sm font-medium text-muted-foreground hover:bg-accent hover:text-accent-foreground transition-colors"
                >
                  <LogOut className="h-4 w-4" />
                  <span className="hidden lg:inline">{t('layout.logout')}</span>
                </button>
              </div>
            )}

            {/* Mobile menu button */}
            <button
              onClick={() => setMobileMenuOpen(!mobileMenuOpen)}
              className="flex md:hidden items-center justify-center rounded-lg p-2 text-muted-foreground hover:bg-accent hover:text-accent-foreground"
            >
              {mobileMenuOpen ? <X className="h-6 w-6" /> : <Menu className="h-6 w-6" />}
            </button>
          </div>
        </div>

        {/* Mobile navigation */}
        {mobileMenuOpen && (
          <div className="border-t md:hidden">
            <nav className="container flex flex-col gap-1 p-4">
              {navItems.map((item) => (
                <Link
                  key={item.href}
                  to={item.href}
                  onClick={() => setMobileMenuOpen(false)}
                  className={cn(
                    'flex items-center gap-3 rounded-lg px-3 py-3 text-sm font-medium transition-colors',
                    location.pathname === item.href
                      ? 'bg-accent text-accent-foreground'
                      : 'text-muted-foreground hover:bg-accent hover:text-accent-foreground',
                  )}
                >
                  {item.icon}
                  {item.label}
                </Link>
              ))}

              <hr className="my-2" />

              {user && (
                <>
                  <Link
                    to="/settings"
                    onClick={() => setMobileMenuOpen(false)}
                    className="flex items-center gap-3 rounded-lg px-3 py-3 text-sm font-medium text-muted-foreground hover:bg-accent hover:text-accent-foreground"
                  >
                    <img
                      src={getTwitchProfileImageUrl(user.twitch_profile_image_url, 70)}
                      alt={user.twitch_display_name}
                      className="h-8 w-8 rounded-full"
                    />
                    <span>{user.twitch_display_name}</span>
                  </Link>
                  <button
                    onClick={handleLogout}
                    className="flex items-center gap-3 rounded-lg px-3 py-3 text-sm font-medium text-destructive hover:bg-destructive/10"
                  >
                    <LogOut className="h-5 w-5" />
                    {t('layout.logout')}
                  </button>
                </>
              )}
            </nav>
          </div>
        )}
      </header>

      {/* Main content */}
      <main className="container py-6 px-4">{children}</main>

      {/* Footer */}
      <footer className="border-t py-6 mt-auto">
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
