import { getApiUrl } from './utils';

// API base URL resolved at runtime (runtime override -> build-time env -> window.location)
const API_BASE_URL = getApiUrl();

// ============================================================================
// Types
// ============================================================================

export interface User {
  id: string;
  twitch_id: string;
  twitch_login: string;
  twitch_display_name: string;
  twitch_profile_image_url: string | null;
  // Optional Telegram fields (may be absent for users who haven't linked Telegram yet)
  telegram_user_id?: string | null;
  telegram_username?: string | null;
  telegram_photo_url?: string | null;
  // Preferred language (optional)
  lang?: string;
}

export interface UserSettings {
  id: string;
  user_id: string;
  stream_online_message: string;
  stream_offline_message: string;
  stream_title_change_message: string;
  stream_category_change_message: string;
  reward_redemption_message: string;
  notify_stream_online: boolean;
  notify_stream_offline: boolean;
  notify_title_change: boolean;
  notify_category_change: boolean;
  notify_reward_redemption: boolean;
  created_at: string;
  updated_at: string;
}

export interface TelegramIntegration {
  id: string;
  user_id: string;
  telegram_chat_id: string;
  telegram_chat_title: string | null;
  telegram_chat_type: string;
  is_enabled: boolean;
  notify_stream_online: boolean;
  notify_stream_offline: boolean;
  notify_title_change: boolean;
  notify_category_change: boolean;
  notify_reward_redemption: boolean;
  created_at: string;
  updated_at: string;
}

export interface TelegramBotInfo {
  username: string;
  id: string;
}

export interface DiscordIntegration {
  id: string;
  user_id: string;
  discord_guild_id: string;
  discord_channel_id: string;
  discord_guild_name: string | null;
  discord_channel_name: string | null;
  discord_webhook_url: string | null;
  is_enabled: boolean;
  notify_stream_online: boolean;
  notify_stream_offline: boolean;
  notify_title_change: boolean;
  notify_category_change: boolean;
  notify_reward_redemption: boolean;
  calendar_sync_enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface DiscordGuild {
  id: string;
  name: string;
  icon: string | null;
}

export interface DiscordChannel {
  id: string;
  name: string | null;
  type: number;
}

export interface DiscordInvite {
  invite_url: string;
  permissions?: number;
  scopes?: string[];
}

export interface TrackedReward {
  id: string;
  user_id: string;
  reward_id: string;
  reward_title: string;
  reward_cost: number;
  is_tracked: boolean;
  chat_response_enabled: boolean;
  chat_response_message: string | null;
  created_at: string;
  updated_at: string;
}

export interface TwitchReward {
  id: string;
  title: string;
  cost: number;
  is_enabled: boolean;
  is_paused: boolean;
  prompt: string | null;
  background_color: string | null;
}

export interface Notification {
  id: string;
  user_id: string;
  notification_type: string;
  destination_type: string;
  destination_id: string;
  content: string;
  status: string;
  error_message: string | null;
  created_at: string;
}

export interface NotificationStats {
  total_sent: number;
  total_failed: number;
  by_type: Record<string, number>;
  by_destination: Record<string, number>;
}

export interface PaginatedResponse<T> {
  items: T[];
  total: number;
  page: number;
  per_page: number;
  total_pages: number;
}

export interface PlaceholderInfo {
  name: string;
  description: string;
  example: string;
}

export interface MessagesInfo {
  stream_online_message: string;
  stream_offline_message: string;
  stream_title_change_message: string;
  stream_category_change_message: string;
  reward_redemption_message: string;
  placeholders: {
    stream: PlaceholderInfo[];
    reward: PlaceholderInfo[];
  };
}

export interface OutgoingShare {
  grantee_user_id: string;
  grantee_twitch_login: string;
  grantee_display_name: string;
  can_manage: boolean;
  created_at: string;
  updated_at: string;
}

export interface IncomingShare {
  owner_user_id: string;
  owner_twitch_login: string;
  owner_display_name: string;
  can_manage: boolean;
  created_at: string;
  updated_at: string;
}

/* Notification flags moved to per-integration APIs (Telegram/Discord).
   Use the per-integration APIs to query and update which notifications are
   enabled for each integration (e.g. `telegramApi.update` / `discordApi.update`).
*/

export interface ApiError {
  error: string;
  message: string;
  status: number;
}

// ============================================================================
// API Client
// ============================================================================

class ApiClient {
  private baseUrl: string;
  private token: string | null = null;

  constructor(baseUrl: string) {
    this.baseUrl = baseUrl;

    // Load stored token from localStorage if present and not expired (guarded for SSR)
    try {
      if (typeof window !== 'undefined') {
        const stored = localStorage.getItem('auth.token');
        const expiresAtStr = localStorage.getItem('auth.expires_at');
        if (stored && expiresAtStr) {
          const expiresAt = Number(expiresAtStr);
          // Only load token if it hasn't expired yet
          if (Number.isFinite(expiresAt) && expiresAt > Math.floor(Date.now() / 1000)) {
            this.token = stored;
          } else {
            // Token is expired, clear it
            localStorage.removeItem('auth.token');
            localStorage.removeItem('auth.expires_at');
          }
        } else if (stored && !expiresAtStr) {
          // Token exists but no expiry, load it (fallback for legacy tokens)
          this.token = stored;
        }
      }
    } catch {
      // ignore storage errors (private mode / restricted environments)
    }
  }

  /**
   * Set the current auth token used for Authorization header.
   * If `persist` is true this will store the token in localStorage so it
   * survives page reloads. Passing `null` clears the token.
   *
   * The optional `expiresAt` value (unix seconds) will also be persisted
   * under `auth.expires_at` so the app can detect expiration client-side.
   */
  public setToken(token: string | null, persist: boolean = true, expiresAt?: number | null): void {
    this.token = token;
    if (persist && typeof window !== 'undefined') {
      try {
        if (token) {
          // Use consistent storage keys: `auth.token` and `auth.expires_at`
          localStorage.setItem('auth.token', token);
          if (expiresAt != null) {
            localStorage.setItem('auth.expires_at', String(expiresAt));
          } else {
            localStorage.removeItem('auth.expires_at');
          }
        } else {
          localStorage.removeItem('auth.token');
          localStorage.removeItem('auth.expires_at');
        }
      } catch {
        // ignore storage errors
      }
    }
  }

  public getToken(): string | null {
    return this.token;
  }

  public clearToken(persist: boolean = true): void {
    this.setToken(null, persist);
  }

  private async request<T>(path: string, options: RequestInit = {}): Promise<T> {
    const url = `${this.baseUrl}${path}`;

    // Build headers using the Headers API so we can safely inspect/set values
    const headers = new Headers(options.headers || {});
    if (!headers.has('Content-Type')) {
      headers.set('Content-Type', 'application/json');
    }

    // If token is set and Authorization header wasn't explicitly provided, add it
    if (this.token && !headers.has('Authorization')) {
      headers.set('Authorization', `Bearer ${this.token}`);
    }

    const response = await fetch(url, {
      ...options,
      headers,
      credentials: 'include', // Include cookies for session management
    });

    if (response.status === 401) {
      // Do not force a navigation here to avoid reload/redirect loops.
      // Instead, throw a structured ApiError and let callers decide how to handle it.
      const unauthorizedError: ApiError = {
        error: 'unauthorized',
        message: 'Unauthorized',
        status: 401,
      };
      throw unauthorizedError;
    }

    if (!response.ok) {
      let errorData: ApiError;
      try {
        errorData = await response.json();
      } catch {
        errorData = {
          error: 'unknown_error',
          message: response.statusText || 'An error occurred',
          status: response.status,
        };
      }
      throw errorData;
    }

    // Handle empty responses
    const text = await response.text();
    if (!text) {
      return undefined as T;
    }

    return JSON.parse(text) as T;
  }

  async get<T>(path: string): Promise<T> {
    return this.request<T>(path, { method: 'GET' });
  }

  async post<T>(path: string, body?: unknown): Promise<T> {
    return this.request<T>(path, {
      method: 'POST',
      body: body ? JSON.stringify(body) : undefined,
    });
  }

  async put<T>(path: string, body?: unknown): Promise<T> {
    return this.request<T>(path, {
      method: 'PUT',
      body: body ? JSON.stringify(body) : undefined,
    });
  }

  async patch<T>(path: string, body?: unknown): Promise<T> {
    return this.request<T>(path, {
      method: 'PATCH',
      body: body ? JSON.stringify(body) : undefined,
    });
  }

  async delete<T>(path: string): Promise<T> {
    return this.request<T>(path, { method: 'DELETE' });
  }
}

const client = new ApiClient(API_BASE_URL);

// Helpers to manage auth token & expiry
export function setAuthToken(token: string | null, expiresAt?: number | null): void {
  // Persist token + expiry using client's single source of truth.
  // `client.setToken` will update both token and expiry in localStorage.
  client.setToken(token, true, expiresAt);
}

export function clearAuthToken(): void {
  // Client handles clearing both token and expiry values.
  client.clearToken(true);
}

export function getAuthToken(): string | null {
  return client.getToken();
}

export function getAuthTokenExpiresAt(): number | null {
  try {
    if (typeof window === 'undefined') return null;
    const v = localStorage.getItem('auth.expires_at');
    if (!v) return null;
    const n = Number(v);
    return Number.isFinite(n) ? n : null;
  } catch {
    return null;
  }
}

// ============================================================================
// Auth API
// ============================================================================

export const authApi = {
  getMe: (): Promise<User> => client.get('/api/auth/me'),
  updateMe: (payload: { lang?: string }): Promise<User> => client.put('/api/auth/me', payload),

  logout: async (): Promise<void> => {
    try {
      await client.post('/api/auth/logout');
    } finally {
      // Ensure local token is cleared even if logout endpoint doesn't exist or fails.
      client.clearToken();
    }
  },

  getLoginUrl: (redirectTo?: string, lang?: string): string => {
    const params = new URLSearchParams();
    if (redirectTo) params.set('redirect_to', redirectTo);
    if (lang) params.set('lang', lang);
    const query = params.toString();
    // Use getApiUrl() directly here so this URL reflects any runtime overrides
    return `${getApiUrl()}/api/auth/login${query ? `?${query}` : ''}`;
  },

  // Unlink Telegram from the current authenticated user
  unlinkTelegram: (): Promise<void> => client.post('/api/auth/telegram/unlink'),

  // Refresh cached Telegram profile photo (best-effort)
  refreshTelegramPhoto: (): Promise<{ photo_url?: string }> =>
    client.post('/api/auth/telegram/photo/refresh'),

  // Helpers for Discord account linking
  // Ask the backend (authenticated) to create a signed URL for Discord OAuth.
  // The backend returns the URL in JSON; the frontend should call this via XHR (so
  // the Authorization header is included) and then redirect the browser to the returned URL.
  getDiscordAuthUrl: (redirectTo?: string): Promise<{ url: string }> => {
    const params = redirectTo ? `?redirect_to=${encodeURIComponent(redirectTo)}` : '';
    return client.get(`/api/auth/discord/link${params}`);
  },

  // Unlink Discord from the current authenticated user
  unlinkDiscord: (): Promise<void> => client.post('/api/auth/discord/unlink'),
};

// ============================================================================
// Settings API
// ============================================================================

export const settingsApi = {
  // Own settings
  getSettings: (): Promise<UserSettings> => client.get('/api/settings'),

  updateSettings: (
    data: Partial<Omit<UserSettings, 'id' | 'user_id' | 'created_at' | 'updated_at'>>,
  ): Promise<UserSettings> => client.put('/api/settings', data),

  // Own messages
  getMessages: (): Promise<MessagesInfo> => client.get('/api/settings/messages'),

  updateMessages: (data: {
    stream_online_message?: string;
    stream_offline_message?: string;
    stream_title_change_message?: string;
    stream_category_change_message?: string;
    reward_redemption_message?: string;
  }): Promise<MessagesInfo> => client.put('/api/settings/messages', data),

  // Notification flags are now managed per-integration (see `telegramApi` / `discordApi`)

  // Reset own settings
  resetToDefaults: (): Promise<UserSettings> => client.post('/api/settings/reset'),

  // ----------------------
  // Shares API
  // ----------------------

  // List outgoing shares (who I've shared with)
  listOutgoingShares: (): Promise<OutgoingShare[]> => client.get('/api/settings/shared'),

  // List incoming shares (who shared with me)
  listIncomingShares: (): Promise<IncomingShare[]> => client.get('/api/settings/shared/incoming'),

  // Create a new share (grant access by twitch_login)
  createShare: (data: { twitch_login: string; can_manage?: boolean }): Promise<OutgoingShare> =>
    client.post('/api/settings/shared', data),

  // Update an existing share (toggle can_manage)
  updateShare: (granteeId: string, data: { can_manage: boolean }): Promise<OutgoingShare> =>
    client.put(`/api/settings/shared/${granteeId}`, data),

  // Revoke a share
  deleteShare: (granteeId: string): Promise<void> =>
    client.delete(`/api/settings/shared/${granteeId}`),

  // ----------------------
  // Per-user settings (accessing another user's settings when shared)
  // ----------------------

  getSettingsForUser: (userId: string): Promise<UserSettings> =>
    client.get(`/api/settings/${userId}`),

  updateSettingsForUser: (
    userId: string,
    data: Partial<Omit<UserSettings, 'id' | 'user_id' | 'created_at' | 'updated_at'>>,
  ): Promise<UserSettings> => client.put(`/api/settings/${userId}`, data),

  getMessagesForUser: (userId: string): Promise<MessagesInfo> =>
    client.get(`/api/settings/${userId}/messages`),

  updateMessagesForUser: (
    userId: string,
    data: {
      stream_online_message?: string;
      stream_offline_message?: string;
      stream_title_change_message?: string;
      stream_category_change_message?: string;
      reward_redemption_message?: string;
    },
  ): Promise<MessagesInfo> => client.put(`/api/settings/${userId}/messages`, data),

  // Per-user notification flags are managed per-integration now.
  // To change flags for a user's integrations, fetch that user's integrations
  // and update them via the integration-specific endpoints (e.g. `telegramApi.update`, `discordApi.update`).

  resetToDefaultsForUser: (userId: string): Promise<UserSettings> =>
    client.put(`/api/settings/${userId}/reset`),
};

// Users API (search)
export const usersApi = {
  search: (q: string, limit: number = 10): Promise<User[]> =>
    client.get(`/api/users?q=${encodeURIComponent(q)}&limit=${limit}`),
};

// ============================================================================
// Telegram Integrations API
// ============================================================================

export const telegramApi = {
  list: (userId?: string): Promise<TelegramIntegration[]> =>
    client.get(
      `/api/integrations/telegram${userId ? `?user_id=${encodeURIComponent(userId)}` : ''}`,
    ),

  get: (id: string): Promise<TelegramIntegration> => client.get(`/api/integrations/telegram/${id}`),

  create: (
    data: {
      telegram_chat_id: string;
      telegram_chat_title?: string;
      telegram_chat_type?: string;
    },
    userId?: string,
  ): Promise<TelegramIntegration> =>
    client.post(
      `/api/integrations/telegram${userId ? `?user_id=${encodeURIComponent(userId)}` : ''}`,
      data,
    ),

  update: (
    id: string,
    data: Partial<{
      is_enabled: boolean;
      notify_stream_online: boolean;
      notify_stream_offline: boolean;
      notify_title_change: boolean;
      notify_category_change: boolean;
      notify_reward_redemption: boolean;
    }>,
  ): Promise<TelegramIntegration> => client.put(`/api/integrations/telegram/${id}`, data),

  delete: (id: string): Promise<void> => client.delete(`/api/integrations/telegram/${id}`),

  test: (id: string): Promise<{ success: boolean; message: string }> =>
    client.post(`/api/integrations/telegram/${id}/test`),

  // Get basic info about the configured Telegram bot (username & id). Requires the bot to be configured.
  getBotInfo: (): Promise<TelegramBotInfo> => client.get('/api/integrations/telegram/bot'),

  // Link a Telegram user (data returned from the Telegram Login Widget) to the current authenticated user.
  link: (data: {
    id: string;
    first_name?: string;
    last_name?: string;
    username?: string;
    photo_url?: string;
    auth_date: number;
    hash: string;
  }): Promise<void> => client.post('/api/auth/telegram/link', data),
};

// ============================================================================
// Discord Integrations API
// ============================================================================

export const discordApi = {
  list: (userId?: string): Promise<DiscordIntegration[]> =>
    client.get(
      `/api/integrations/discord${userId ? `?user_id=${encodeURIComponent(userId)}` : ''}`,
    ),

  get: (id: string): Promise<DiscordIntegration> => client.get(`/api/integrations/discord/${id}`),

  create: (
    data: {
      discord_guild_id: string;
      discord_channel_id: string;
      discord_guild_name?: string;
      discord_channel_name?: string;
      discord_webhook_url?: string;
    },
    userId?: string,
  ): Promise<DiscordIntegration> =>
    client.post(
      `/api/integrations/discord${userId ? `?user_id=${encodeURIComponent(userId)}` : ''}`,
      data,
    ),

  update: (
    id: string,
    data: Partial<{
      discord_channel_id: string;
      discord_channel_name: string;
      discord_webhook_url: string;
      is_enabled: boolean;
      notify_stream_online: boolean;
      notify_stream_offline: boolean;
      notify_title_change: boolean;
      notify_category_change: boolean;
      notify_reward_redemption: boolean;
      calendar_sync_enabled: boolean;
    }>,
  ): Promise<DiscordIntegration> => client.put(`/api/integrations/discord/${id}`, data),

  delete: (id: string): Promise<void> => client.delete(`/api/integrations/discord/${id}`),

  test: (id: string): Promise<{ success: boolean; message: string }> =>
    client.post(`/api/integrations/discord/${id}/test`),

  getInvite: (): Promise<DiscordInvite> => client.get('/api/integrations/discord/invite'),

  listGuilds: (): Promise<DiscordGuild[]> => client.get('/api/integrations/discord/guilds'),

  // List guilds that are common between the bot and the authenticated user
  listSharedGuilds: (): Promise<DiscordGuild[]> =>
    client.get('/api/integrations/discord/guilds/shared'),

  listChannels: (guildId: string): Promise<DiscordChannel[]> =>
    client.get(`/api/integrations/discord/guilds/${guildId}/channels`),

  getChannel: (channelId: string): Promise<DiscordChannel> =>
    client.get(`/api/integrations/discord/channels/${channelId}`),
};

// ============================================================================
// Rewards API
// ============================================================================

export const rewardsApi = {
  listTwitchRewards: (): Promise<TwitchReward[]> => client.get('/api/rewards/twitch'),

  listTracked: (): Promise<TrackedReward[]> => client.get('/api/rewards'),

  get: (id: string): Promise<TrackedReward> => client.get(`/api/rewards/${id}`),

  track: (data: {
    reward_id: string;
    reward_title: string;
    reward_cost: number;
    chat_response_enabled?: boolean;
    chat_response_message?: string;
  }): Promise<TrackedReward> => client.post('/api/rewards', data),

  update: (
    id: string,
    data: Partial<{
      is_tracked: boolean;
      chat_response_enabled: boolean;
      chat_response_message: string;
    }>,
  ): Promise<TrackedReward> => client.put(`/api/rewards/${id}`, data),

  delete: (id: string): Promise<void> => client.delete(`/api/rewards/${id}`),
};

// ============================================================================
// Notifications API
// ============================================================================

export const notificationsApi = {
  list: async (params?: {
    page?: number;
    per_page?: number;
    notification_type?: string;
    destination_type?: string;
    status?: string;
  }): Promise<PaginatedResponse<Notification>> => {
    const searchParams = new URLSearchParams();
    if (params?.page) searchParams.set('page', params.page.toString());
    if (params?.per_page) searchParams.set('per_page', params.per_page.toString());
    if (params?.notification_type) searchParams.set('notification_type', params.notification_type);
    if (params?.destination_type) searchParams.set('destination_type', params.destination_type);
    if (params?.status) searchParams.set('status', params.status);

    const query = searchParams.toString();
    const raw: unknown = await client.get(`/api/notifications${query ? `?${query}` : ''}`);

    const asString = (v: unknown, fallback = ''): string =>
      typeof v === 'string' ? v : typeof v === 'number' ? String(v) : fallback;

    // Normalize a single notification object coming from either old or new API
    const normalize = (n: unknown): Notification => {
      const obj = n && typeof n === 'object' ? (n as Record<string, unknown>) : {};
      const error_message =
        typeof obj['error_message'] === 'string' ? (obj['error_message'] as string) : null;
      return {
        id: asString(obj['id']),
        user_id: asString(obj['user_id'] ?? obj['userId']),
        notification_type: asString(obj['notification_type']),
        destination_type: asString(obj['destination_type']),
        destination_id: asString(obj['destination_id']),
        // Support both `content` (new) and `message` (old)
        content: asString(obj['content'] ?? obj['message'], ''),
        status: asString(obj['status']),
        error_message,
        created_at: asString(obj['created_at']),
      };
    };

    // New paginated shape: { items, total, page, per_page, total_pages }
    if (
      typeof raw === 'object' &&
      raw !== null &&
      Array.isArray((raw as Record<string, unknown>)['items'])
    ) {
      const items = ((raw as Record<string, unknown>)['items'] as unknown[]).map(normalize);
      return {
        items,
        total: Number((raw as Record<string, unknown>)['total'] ?? items.length),
        page: Number((raw as Record<string, unknown>)['page'] ?? 1),
        per_page: Number((raw as Record<string, unknown>)['per_page'] ?? items.length),
        total_pages: Number((raw as Record<string, unknown>)['total_pages'] ?? 1),
      };
    }

    // Old paginated shape: { notifications: [...], pagination: { page, per_page, total, total_pages } }
    if (
      typeof raw === 'object' &&
      raw !== null &&
      Array.isArray((raw as Record<string, unknown>)['notifications']) &&
      (raw as Record<string, unknown>)['pagination']
    ) {
      const items = ((raw as Record<string, unknown>)['notifications'] as unknown[]).map(normalize);
      const p = (raw as Record<string, unknown>)['pagination'] as Record<string, unknown>;
      return {
        items,
        total: Number(p['total'] ?? items.length),
        page: Number(p['page'] ?? 1),
        per_page: Number(p['per_page'] ?? items.length),
        total_pages: Number(p['total_pages'] ?? 1),
      };
    }

    // Raw array of notifications (no pagination)
    if (Array.isArray(raw)) {
      const items = (raw as unknown[]).map(normalize);
      return { items, total: items.length, page: 1, per_page: items.length, total_pages: 1 };
    }

    // Fallback empty response
    return { items: [], total: 0, page: 1, per_page: params?.per_page ?? 20, total_pages: 0 };
  },

  getStats: (): Promise<NotificationStats> => client.get('/api/notifications/stats'),
};

// ============================================================================
// Calendar API
// ============================================================================

export const calendarApi = {
  sync: (): Promise<{ synced: number; message: string }> => client.post('/api/calendar/sync'),

  getStatus: (): Promise<{
    enabled: boolean;
    last_sync: string | null;
    events_count: number;
  }> => client.get('/api/calendar/status'),
};

// ============================================================================
// Export
// ============================================================================

export { client as api };
export default client;
