import '@testing-library/jest-dom';
import './i18n';
import i18next from 'i18next';
import { afterEach } from 'vitest';
import { cleanup } from '@testing-library/react';

// Ensure tests run in a known language
void i18next.changeLanguage('en');

// Cleanup DOM between tests
afterEach(() => {
  cleanup();
});

// Basic matchMedia polyfill for components that rely on it
if (typeof window !== 'undefined' && !window.matchMedia) {
  Object.defineProperty(window, 'matchMedia', {
    writable: true,
    value: (query: string) => ({
      matches: false,
      media: query,
      onchange: null,
      addListener: () => {}, // deprecated
      removeListener: () => {}, // deprecated
      addEventListener: () => {},
      removeEventListener: () => {},
      dispatchEvent: () => false,
    }),
  });
}
