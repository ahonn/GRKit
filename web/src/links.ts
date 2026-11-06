import type { Language } from './i18n';

export const REPOSITORY = 'https://github.com/ahonn/grkit';

/** A guide in `resources/` in the page language, or in English. */
export function guideUrl(name: 'RECOVERY' | 'DUMP' | 'TERMS', language: Language): string {
  return `${REPOSITORY}/blob/main/resources/${name}${language === 'en' ? '' : `.${language}`}.md`;
}
