import { Routes, Route, Navigate } from 'react-router-dom';
// QueryClientProvider is applied in `main.tsx`; do not duplicate it here.
import { Toaster } from '@/components/ui/toaster';
import { AuthProvider, useAuth } from '@/hooks/useAuth';

// Pages
import { HomePage } from '@/pages/HomePage';
import { LoginPage } from '@/pages/LoginPage';
import { DashboardPage } from '@/pages/DashboardPage';
import { SettingsPage } from '@/pages/SettingsPage';
import { OwnerSettingsPage } from '@/pages/OwnerSettingsPage';
import { IntegrationsPage } from '@/pages/IntegrationsPage';
import { NotificationsPage } from '@/pages/NotificationsPage';
import { AuthCallbackPage } from '@/pages/AuthCallbackPage';
import TelegramCallbackPage from '@/pages/TelegramCallbackPage';

// Layout
import { Layout } from '@/components/Layout';

// QueryClient is created and provided in `main.tsx`

// Protected route wrapper
function ProtectedRoute({ children }: { children: React.ReactNode }) {
  const { user, isLoading } = useAuth();

  if (isLoading) {
    return (
      <div className="flex h-screen items-center justify-center">
        <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-twitch"></div>
      </div>
    );
  }

  if (!user) {
    return <Navigate to="/login" replace />;
  }

  return <>{children}</>;
}

// Public route wrapper (redirects to dashboard if already logged in)
function PublicRoute({ children }: { children: React.ReactNode }) {
  const { user, isLoading } = useAuth();

  if (isLoading) {
    return (
      <div className="flex h-screen items-center justify-center">
        <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-twitch"></div>
      </div>
    );
  }

  if (user) {
    return <Navigate to="/dashboard" replace />;
  }

  return <>{children}</>;
}

function AppRoutes() {
  return (
    <Routes>
      {/* Public routes */}
      <Route
        path="/"
        element={
          <PublicRoute>
            <HomePage />
          </PublicRoute>
        }
      />
      <Route
        path="/login"
        element={
          <PublicRoute>
            <LoginPage />
          </PublicRoute>
        }
      />
      <Route path="/auth/callback" element={<AuthCallbackPage />} />

      {/* Protected routes */}
      <Route
        path="/dashboard"
        element={
          <ProtectedRoute>
            <Layout>
              <DashboardPage />
            </Layout>
          </ProtectedRoute>
        }
      />
      <Route
        path="/settings"
        element={
          <ProtectedRoute>
            <Layout>
              <SettingsPage />
            </Layout>
          </ProtectedRoute>
        }
      />
      <Route
        path="/settings/:userId"
        element={
          <ProtectedRoute>
            <Layout>
              <OwnerSettingsPage />
            </Layout>
          </ProtectedRoute>
        }
      />
      <Route
        path="/integrations"
        element={
          <ProtectedRoute>
            <Layout>
              <IntegrationsPage />
            </Layout>
          </ProtectedRoute>
        }
      />
      <Route
        path="/integrations/telegram"
        element={
          <ProtectedRoute>
            <Navigate to="/integrations?tab=telegram" replace />
          </ProtectedRoute>
        }
      />
      <Route
        path="/integrations/telegram/callback"
        element={
          <ProtectedRoute>
            <Layout>
              <TelegramCallbackPage />
            </Layout>
          </ProtectedRoute>
        }
      />
      <Route
        path="/integrations/discord"
        element={
          <ProtectedRoute>
            <Navigate to="/integrations?tab=discord" replace />
          </ProtectedRoute>
        }
      />
      <Route
        path="/notifications"
        element={
          <ProtectedRoute>
            <Layout>
              <NotificationsPage />
            </Layout>
          </ProtectedRoute>
        }
      />

      {/* Catch all - redirect to home */}
      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  );
}

function App() {
  return (
    <AuthProvider>
      <div className="min-h-screen bg-background text-foreground">
        <AppRoutes />
        <Toaster />
      </div>
    </AuthProvider>
  );
}

export default App;
