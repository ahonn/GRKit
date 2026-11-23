import { STRINGS, type Language } from '../i18n';
import { pagePath } from '../languagePage';
import { docSlug, type DocsSitePage } from './pages';
import type { DocsContent } from './types';

/** The title, the description, and the address of the docs home or of a doc. */
export function docPageText(language: Language, page: DocsSitePage, content: DocsContent) {
  const slug = docSlug(page);
  const path = pagePath(language, page);
  if (!slug) return { title: `${STRINGS[language].docsLink} · GRKit`, description: content.home.lead, path };
  const doc = content.pages[slug];
  return { title: `${doc.title} · GRKit ${STRINGS[language].docsLink}`, description: doc.lead, path };
}
