import { browser } from '$app/environment';
import { writable } from 'svelte/store';
import { AVAILABLE_LOCALES, DEFAULT_LOCALE, FALLBACK_LOCALE, messages } from './messages';
import type { Locale, TranslationKey, TranslationVars } from './types';

const STORAGE_KEY = 'unifi.locale';

export function normalizeLocale(input: string | null | undefined): Locale {
  if (!input) {
    return DEFAULT_LOCALE;
  }

  const normalized = input.toLowerCase();
  if (normalized.startsWith('zh')) {
    return 'zh-CN';
  }

  return 'en-US';
}

function persistLocale(next: Locale) {
  if (!browser) {
    return;
  }

  window.localStorage.setItem(STORAGE_KEY, next);
  document.documentElement.lang = next;
}

const initialLocale: Locale = browser
  ? normalizeLocale(window.localStorage.getItem(STORAGE_KEY) ?? window.navigator.language)
  : DEFAULT_LOCALE;

export const locale = writable<Locale>(initialLocale);

if (browser) {
  locale.subscribe((value) => {
    persistLocale(value);
  });
}

export function setLocale(next: Locale) {
  locale.set(next);
}

export function getAvailableLocales() {
  return [...AVAILABLE_LOCALES];
}

export function t(currentLocale: Locale, key: TranslationKey, vars?: TranslationVars) {
  const catalog = messages[currentLocale] ?? messages[FALLBACK_LOCALE];
  const fallbackCatalog = messages[FALLBACK_LOCALE];
  let text = catalog[key] ?? fallbackCatalog[key];

  if (!vars) {
    return text;
  }

  return text.replace(/\{(\w+)\}/g, (_, token) => String(vars[token] ?? `{${token}}`));
}

export function translateSignalQuality(
  currentLocale: Locale,
  quality: 'excellent' | 'good' | 'fair' | 'weak' | 'poor'
) {
  const keyMap = {
    excellent: 'signalExcellent',
    good: 'signalGood',
    fair: 'signalFair',
    weak: 'signalWeak',
    poor: 'signalPoor',
  } satisfies Record<typeof quality, TranslationKey>;

  return t(currentLocale, keyMap[quality]);
}
