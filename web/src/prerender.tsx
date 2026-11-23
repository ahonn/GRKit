// The entry that scripts/prerender.mjs builds for Node. It renders the first screen of each page
// of each language, so the HTML of the page holds its text before the script runs.
import { renderToString } from 'react-dom/server';
import { InstallPage } from './App';
import { FastbootPage } from './components/FastbootPage';
import { HomePage } from './components/HomePage';
import { DocsPage } from './components/DocsPage';
import { ResourcesPage } from './components/ResourcesPage';
import { en } from './docs/content/en';
import { ja } from './docs/content/ja';
import { ko } from './docs/content/ko';
import { zhHans } from './docs/content/zh-Hans';
import { zhHant } from './docs/content/zh-Hant';
import { docPageText } from './docs/meta';
import { DOCS_SITE_PAGES, type DocsSitePage } from './docs/pages';
import type { DocsContent } from './docs/types';
import { pageText as sitePageText, type Language } from './i18n';
import type { Page } from './languagePage';

export { LANGUAGES, STRINGS } from './i18n';
export { LANGUAGE_PATHS } from './languagePage';
export { DOCS_SITE_PAGES };

const DOCS: Record<Language, DocsContent> = { en, 'zh-Hans': zhHans, 'zh-Hant': zhHant, ja, ko };

const isDocs = (page: Page): page is DocsSitePage => page === 'docs' || page.startsWith('docs/');

/** The title, the description, and the address of `page`, with the text of a doc. */
export function pageText(language: Language, page: Page) {
  return isDocs(page) ? docPageText(language, page, DOCS[language]) : sitePageText(language, page);
}

export const VERSION = __GRKIT_VERSION__;

/** The markup of `page` in `language`, as a browser without a card or storage shows it first. */
export function render(language: Language, page: Page): string {
  if (isDocs(page)) return renderToString(<DocsPage language={language} page={page} content={DOCS[language]} />);
  switch (page) {
    case 'home':
      return renderToString(<HomePage initialLanguage={language} />);
    case 'install':
      return renderToString(<InstallPage initialLanguage={language} />);
    case 'fastboot':
      return renderToString(<FastbootPage initialLanguage={language} />);
    case 'resources':
      return renderToString(<ResourcesPage initialLanguage={language} />);
  }
}
