/**
 * i18n initialization for the frontend
 *
 * - Loads built-in JSON resources (English / Russian)
 * - Detects initial language from localStorage or navigator
 * - Persists language selection to localStorage when changed
 */

import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';

import en from './locales_en.json';
import ru from './locales_ru.json';

const resources = {
  en: { translation: en },
  ru: { translation: ru },
} as const;

function getInitialLanguage(): string {
  try {
    const stored = (localStorage.getItem('i18nextLng') ?? localStorage.getItem('lang')) || '';
    if (stored) return stored.split('-')[0];
  } catch {
    // ignore (e.g. SSR or private mode)
  }

  if (typeof navigator !== 'undefined') {
    const nav = navigator.language || (navigator.languages && navigator.languages[0]);
    if (nav) return nav.split('-')[0];
  }

  // Default to Russian since the app's original content is in Russian
  return 'ru';
}

i18n.use(initReactI18next).init({
  resources,
  lng: getInitialLanguage(),
  fallbackLng: 'ru',
  supportedLngs: ['ru', 'en'],
  interpolation: {
    escapeValue: false, // React already escapes
  },
  react: {
    useSuspense: false, // keep rendering predictable without Suspense boundary
  },
});

/**
 * Persist language selection to localStorage so the user's choice is remembered.
 */
i18n.on('languageChanged', (lng) => {
  try {
    localStorage.setItem('i18nextLng', lng);
  } catch {
    // ignore errors (e.g. private browsing)
  }
});

export const availableLanguages = ['ru', 'en'] as const;
export const setLanguage = (lng: string) => i18n.changeLanguage(lng);

export default i18n;
