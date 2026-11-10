// The entry that scripts/prerender.mjs builds for Node. It renders the first screen of each page
// of each language, so the HTML of the page holds its text before the script runs.
import { renderToString } from 'react-dom/server';
import { InstallPage } from './App';
import { FastbootPage } from './components/FastbootPage';
import { HomePage } from './components/HomePage';
import type { Language } from './i18n';
import type { Page } from './languagePage';

export { LANGUAGES, pageText, STRINGS } from './i18n';
export { LANGUAGE_PATHS } from './languagePage';

export const VERSION = __GRKIT_VERSION__;

/** The markup of `page` in `language`, as a browser without a card or storage shows it first. */
export function render(language: Language, page: Page): string {
  switch (page) {
    case 'home':
      return renderToString(<HomePage initialLanguage={language} />);
    case 'install':
      return renderToString(<InstallPage initialLanguage={language} />);
    case 'fastboot':
      return renderToString(<FastbootPage initialLanguage={language} />);
  }
}
