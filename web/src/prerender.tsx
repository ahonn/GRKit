// The entry that scripts/prerender.mjs builds for Node. It renders the first screen of each
// language page, so the HTML of the page holds its text before the script runs.
import { renderToString } from 'react-dom/server';
import { App } from './App';
import { FastbootPage } from './components/FastbootPage';
import type { Language } from './i18n';

export { LANGUAGES, pageText, STRINGS } from './i18n';
export { fastbootPath, LANGUAGE_PATHS } from './languagePage';

export const VERSION = __GRKIT_VERSION__;

/** The markup of the page in `language`, as a browser without a card or storage shows it first. */
export function render(language: Language): string {
  return renderToString(<App initialLanguage={language} />);
}

/** The markup of the fast-boot firmware page in `language`. */
export function renderFastboot(language: Language): string {
  return renderToString(<FastbootPage initialLanguage={language} />);
}
