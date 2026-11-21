import type { Language } from './i18n';
import { LANGUAGE_PATHS } from './languagePage';

export const REPOSITORY = 'https://github.com/ahonn/grkit';

/** A guide in `resources/` in the page language, or in English. */
export function guideUrl(name: 'RECOVERY' | 'DUMP' | 'TERMS', language: Language): string {
  return `${REPOSITORY}/blob/main/resources/${name}${language === 'en' ? '' : `.${language}`}.md`;
}

/** Where a GR Snap link sits on the site, which the link reports to the GR Snap site. */
export type GrSnapPlacement = 'android-bar' | 'footer' | 'card-done';

/**
 * GR Snap, the RICOH GR app by the author of GRKit, on its site in the page language. Its site has
 * the language paths of this site.
 */
export function grSnapUrl(language: Language, placement: GrSnapPlacement): string {
  return `https://grsnap.app${LANGUAGE_PATHS[language]}?utm_source=grkit&utm_medium=${placement}`;
}
