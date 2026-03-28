import type { enUS } from './locales/en-US';

export type Locale = 'zh-CN' | 'en-US';
export type TranslationMessages = {
  [Key in keyof typeof enUS]: string;
};
export type TranslationKey = keyof TranslationMessages;
export type TranslationVars = Record<string, string | number>;
