import { type ClassValue, clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';
import i18n from '@/i18n';

/**
 * Merge class names with Tailwind CSS classes
 * Uses clsx for conditional classes and tailwind-merge to handle conflicts
 */
export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

/**
 * Format a date string to a human-readable format (locale-aware)
 */
export function formatDate(date: string | Date): string {
  const d = typeof date === 'string' ? new Date(date) : date;
  const locale = i18n.language || 'ru';
  return d.toLocaleDateString(locale, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  });
}

/**
 * Format a date string to a relative time (e.g., "2 hours ago")
 * Uses Intl.RelativeTimeFormat where available and falls back to a localized
 * "just now" string for very recent events.
 */
export function formatRelativeTime(date: string | Date): string {
  const d = typeof date === 'string' ? new Date(date) : date;
  const now = new Date();
  const diffInSeconds = Math.floor((now.getTime() - d.getTime()) / 1000);

  if (diffInSeconds < 60) {
    // Use a translation key when available, otherwise fallback to a sensible default.
    return i18n.t('relative.just_now');
  }

  const locale = i18n.language || 'ru';
  const rtf = new Intl.RelativeTimeFormat(locale, { numeric: 'auto' });

  const diffInMinutes = Math.floor(diffInSeconds / 60);
  if (diffInMinutes < 60) {
    return rtf.format(-diffInMinutes, 'minute');
  }

  const diffInHours = Math.floor(diffInMinutes / 60);
  if (diffInHours < 24) {
    return rtf.format(-diffInHours, 'hour');
  }

  const diffInDays = Math.floor(diffInHours / 24);
  if (diffInDays < 7) {
    return rtf.format(-diffInDays, 'day');
  }

  return formatDate(d);
}

/**
 * Russian pluralization helper
 */
export function pluralize(n: number, one: string, few: string, many: string): string {
  const mod10 = n % 10;
  const mod100 = n % 100;

  if (mod10 === 1 && mod100 !== 11) {
    return one;
  }
  if (mod10 >= 2 && mod10 <= 4 && (mod100 < 10 || mod100 >= 20)) {
    return few;
  }
  return many;
}

/**
 * Format a number with locale-aware separators
 */
export function formatNumber(num: number): string {
  const locale = i18n.language || 'ru';
  return num.toLocaleString(locale);
}

/**
 * Truncate a string to a maximum length with ellipsis
 */
export function truncate(str: string, maxLength: number): string {
  if (str.length <= maxLength) return str;
  return str.slice(0, maxLength - 3) + '...';
}

/**
 * Generate a Twitch channel URL
 */
export function getTwitchChannelUrl(login: string): string {
  return `https://twitch.tv/${login}`;
}

/**
 * Generate a Twitch profile image URL with specified size
 */
export function getTwitchProfileImageUrl(
  url: string | null | undefined,
  size: number = 300,
): string {
  if (!url) {
    return `https://static-cdn.jtvnw.net/user-default-pictures-uv/ebe4cd89-b4f4-4bc5-a00f-a7d6ae61fe00-profile_image-${size}x${size}.png`;
  }
  // Replace {width}x{height} or existing dimensions with desired size
  return url.replace(/\d+x\d+/, `${size}x${size}`);
}

/**
 * Get the notification type label (localized)
 */
export function getNotificationTypeLabel(type: string): string {
  const key = `notifications_labels.${type}`;
  const val = i18n.t(key);
  // If translation is not found, i18n.t returns the key - in that case fallback to the raw type
  return val !== key ? val : type;
}

/**
 * Get the notification type icon name
 */
export function getNotificationTypeIcon(type: string): string {
  const icons: Record<string, string> = {
    stream_online: 'play-circle',
    stream_offline: 'stop-circle',
    title_change: 'edit',
    category_change: 'gamepad-2',
    reward_redemption: 'gift',
  };
  return icons[type] || 'bell';
}

/**
 * Get the destination type label (localized)
 */
export function getDestinationTypeLabel(type: string): string {
  // Prefer translation keys used for filter labels where available
  const key = `notifications.filters.${type}`;
  const val = i18n.t(key);
  if (val !== key) return val;

  // Fallback for chat destination (keep compatibility with previous label)
  if (type === 'chat') {
    return i18n.t('notifications.destinations.chat');
  }

  return type;
}

/**
 * Debounce a function
 */
export function debounce<T extends (...args: unknown[]) => unknown>(
  func: T,
  wait: number,
): (...args: Parameters<T>) => void {
  let timeout: ReturnType<typeof setTimeout> | null = null;

  return function executedFunction(...args: Parameters<T>) {
    const later = () => {
      timeout = null;
      func(...args);
    };

    if (timeout !== null) {
      clearTimeout(timeout);
    }
    timeout = setTimeout(later, wait);
  };
}

/**
 * Copy text to clipboard
 */
export async function copyToClipboard(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    // Fallback for older browsers
    const textArea = document.createElement('textarea');
    textArea.value = text;
    textArea.style.position = 'fixed';
    textArea.style.left = '-999999px';
    textArea.style.top = '-999999px';
    document.body.appendChild(textArea);
    textArea.focus();
    textArea.select();

    try {
      document.execCommand('copy');
      return true;
    } catch {
      return false;
    } finally {
      textArea.remove();
    }
  }
}

/**
 * Check if we're running in development mode
 */
export function isDev(): boolean {
  const meta = import.meta as unknown as { env?: { DEV?: boolean } };
  return meta.env?.DEV ?? false;
}

/**
 * Get the API base URL.
 *
 * Resolution order:
 * 1. Optional runtime override injected into the page (e.g. `window.__RUNTIME_CONFIG__.API_URL` or `window.__API_URL__`)
 * 2. Build-time `import.meta.env.VITE_API_URL`
 * 3. Fallback to `window.location.origin` (so the frontend can be deployed/configured at runtime without rebuild)
 */
export function getApiUrl(): string {
  // Support optional runtime overrides (useful for container/image deployments)
  const win = window as unknown as {
    __RUNTIME_CONFIG__?: { API_URL?: string };
    __API_URL__?: string;
  };
  const runtime = win.__RUNTIME_CONFIG__?.API_URL ?? win.__API_URL__;
  if (typeof runtime === 'string' && runtime.length > 0) {
    return runtime.replace(/\/+$/, ''); // trim trailing slash
  }

  // Build-time environment (Vite)
  const meta = import.meta as unknown as { env?: { VITE_API_URL?: string } };
  if (meta.env?.VITE_API_URL && meta.env.VITE_API_URL.length > 0) {
    return meta.env.VITE_API_URL.replace(/\/+$/, '');
  }

  // Fallback: derive from current location (protocol + host + optional port)
  // Use origin when available; construct it otherwise for broader compatibility.
  const origin =
    (window.location && (window.location as Location).origin) ||
    `${window.location.protocol}//${window.location.hostname}${window.location.port ? `:${window.location.port}` : ''}`;
  return origin.replace(/\/+$/, '');
}

/**
 * Sleep for a specified number of milliseconds
 */
export function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Generate a random string
 */
export function randomString(length: number): string {
  const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
  let result = '';
  for (let i = 0; i < length; i++) {
    result += chars.charAt(Math.floor(Math.random() * chars.length));
  }
  return result;
}
