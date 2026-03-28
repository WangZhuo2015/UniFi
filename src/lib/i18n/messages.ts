import { enUS } from './locales/en-US';
import { zhCN } from './locales/zh-CN';
import type { Locale, TranslationMessages } from './types';

export const DEFAULT_LOCALE: Locale = 'zh-CN';
export const FALLBACK_LOCALE: Locale = 'en-US';
export const AVAILABLE_LOCALES = ['zh-CN', 'en-US'] as const satisfies readonly Locale[];

export const messages: Record<Locale, TranslationMessages> = {
  'zh-CN': zhCN,
  'en-US': enUS,
};
