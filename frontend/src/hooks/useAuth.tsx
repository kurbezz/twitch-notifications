import { createContext, useContext, useCallback, useMemo, ReactNode, useEffect } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { authApi, getAuthToken, getAuthTokenExpiresAt, clearAuthToken, ApiError } from '@/lib/api';
import i18n, { setLanguage } from '@/i18n';

export interface User {
  id: string;
  twitch_id: string;
  twitch_login: string;
  twitch_display_name: string;
  twitch_profile_image_url: string | null;
  // Preferred language (optional)
  lang?: string;
  // Optional Telegram fields (may be absent for users who haven't linked Telegram yet)
  telegram_user_id?: string | null;
  telegram_username?: string | null;
  telegram_photo_url?: string | null;
  // Optional Discord fields (may be absent for users who haven't linked Discord yet)
  discord_user_id?: string | null;
  discord_username?: string | null;
  discord_avatar_url?: string | null;
}

interface AuthContextType {
  user: User | null;
  isLoading: boolean;
  isAuthenticated: boolean;
  login: () => void;
  logout: () => Promise<void>;
  refreshUser: () => Promise<void>;
}

const AuthContext = createContext<AuthContextType | undefined>(undefined);

interface AuthProviderProps {
  children: ReactNode;
}

export function AuthProvider({ children }: AuthProviderProps) {
  const queryClient = useQueryClient();

  // Fetcher that tries token-based auth first, then falls back to cookie-based auth.
  const fetchUserWithFallback = useCallback(async (): Promise<User | null> => {
    try {
      const token = getAuthToken();
      const expiresAt = getAuthTokenExpiresAt();

      // Try token-based auth first if we have a presumably valid token.
      if (token && (!expiresAt || expiresAt > Math.floor(Date.now() / 1000))) {
        try {
          return await authApi.getMe();
        } catch (err) {
          const apiErr = err as ApiError | undefined;
          if (apiErr?.status === 401) {
            // Token rejected — clear it and fall back to cookie-based attempt below.
            clearAuthToken();
          } else {
            // Non-auth error — rethrow so React Query can decide whether to retry.
            throw err;
          }
        }
      }

      // Try cookie-based auth as a fallback (may return 200 with user or 401/not found).
      try {
        return await authApi.getMe();
      } catch {
        // Not authenticated via cookie either — treat as unauthenticated.
        return null;
      }
    } catch {
      // Unexpected failure — treat as unauthenticated.
      return null;
    }
  }, []);

  const { data, isLoading, refetch } = useQuery<User | null, ApiError>({
    queryKey: ['auth', 'me'],
    queryFn: fetchUserWithFallback,
    staleTime: 1000 * 60 * 5, // 5 minutes
    retry: 1,
    refetchOnWindowFocus: false,
  });

  const user = data ?? null;

  useEffect(() => {
    if (user?.lang) {
      try {
        setLanguage(user.lang);
      } catch {
        // ignore errors (e.g. invalid language)
      }
    }
  }, [user?.lang]);

  // Redirect to Twitch OAuth login
  const login = useCallback(() => {
    const currentPath = window.location.pathname;
    const origin = window.location.origin;
    const redirectTo =
      currentPath !== '/login' && currentPath !== '/'
        ? `${origin}${currentPath}`
        : `${origin}/dashboard`;
    // Determine UI language and include it in OAuth login redirect so backend can set it for new users
    const lang = i18n && i18n.language ? i18n.language.split('-')[0] : undefined;
    // Use authApi helper so runtime API base overrides are respected
    window.location.href = authApi.getLoginUrl(redirectTo, lang);
  }, []);

  // Logout and clear user state + cache
  const logout = useCallback(async () => {
    try {
      await authApi.logout();
    } catch (error) {
      console.error('Logout error:', error);
    } finally {
      // Ensure any stored token is cleared client-side and reset cached user
      clearAuthToken();
      queryClient.setQueryData(['auth', 'me'], null);
      window.location.href = '/';
    }
  }, [queryClient]);

  // Refresh user data - throws on authentication failure
  const refreshUser = useCallback(async () => {
    const result = await refetch();
    if (!result.data) {
      // If user is not authenticated after refetch, propagate failure so callers
      // can perform auth-related flows (e.g. redirect to login).
      throw new Error('Not authenticated');
    }
  }, [refetch]);

  const value: AuthContextType = useMemo(
    () => ({
      user,
      isLoading,
      isAuthenticated: !!user,
      login,
      logout,
      refreshUser,
    }),
    [user, isLoading, login, logout, refreshUser],
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth(): AuthContextType {
  const context = useContext(AuthContext);
  if (context === undefined) {
    throw new Error('useAuth must be used within an AuthProvider');
  }
  return context;
}

// Hook for checking if user has specific permissions
export function useRequireAuth(): AuthContextType & { requireAuth: () => boolean } {
  const auth = useAuth();
  const { isAuthenticated, isLoading, login } = auth;

  const requireAuth = useCallback(() => {
    if (!isAuthenticated && !isLoading) {
      login();
      return false;
    }
    return true;
  }, [isAuthenticated, isLoading, login]);

  return { ...auth, requireAuth };
}

// Hook for getting user's Twitch profile image with fallback
export function useUserAvatar(size: 70 | 150 | 300 = 150): string | null {
  const { user } = useAuth();

  if (!user?.twitch_profile_image_url) {
    return null;
  }

  // Twitch profile images can be resized by modifying the URL
  // Default size is 300x300, we can request different sizes
  return user.twitch_profile_image_url.replace(/\d+x\d+/, `${size}x${size}`);
}
