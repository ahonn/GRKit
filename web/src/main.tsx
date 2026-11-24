import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { InstallPage } from './App';
import { canWriteCards } from './card';
import { loadLastCard } from './cardStore';
import { loadCatalog } from './catalog';
import { migrateCoverScheme } from './customCovers';
import { PREVIEW_TARGET } from './generated/site';
import { loadImports } from './import/store';
import { loadFonts } from './fonts';
import { docPageText } from './docs/meta';
import { loadDocsContent } from './docs/load';
import type { DocsSitePage } from './docs/pages';
import { showLanguage, showPageText, takeLegacyLanguage } from './i18n';
import { installPath, isInstallAnchor, pagePath, pathPage } from './languagePage';
import { DocsPage } from './components/DocsPage';
import { FastbootPage } from './components/FastbootPage';
import { HomePage } from './components/HomePage';
import { ResourcesPage } from './components/ResourcesPage';
// The site serves its fonts itself, so a page view sends no request to a font host.
// loadFonts adds the Chinese, Japanese, or Korean fonts when a page in that language needs them.
import '@fontsource/ibm-plex-sans/400.css';
import '@fontsource/ibm-plex-sans/500.css';
import '@fontsource/ibm-plex-sans/600.css';
import '@fontsource/ibm-plex-sans-condensed/500.css';
import '@fontsource/ibm-plex-sans-condensed/700.css';
import '@fontsource/ibm-plex-mono/400.css';
import '@fontsource/ibm-plex-mono/500.css';
import './styles.css';

const { language, page } = pathPage(location.pathname);
// A visitor who chose a language before each language had its own page goes to that page once.
const legacy = takeLegacyLanguage();
const chosen = legacy !== null && location.pathname === '/' ? legacy : language;
// The installer was part of the home page before it had its own page, so a saved or shared link
// to one of its sections goes to that section of the installer.
const target =
  page === 'home' && isInstallAnchor(location.hash)
    ? installPath(chosen)
    : chosen !== language
      ? pagePath(chosen, page)
      : null;

function browserStorage(): Storage | null {
  try {
    return localStorage;
  } catch {
    return null;
  }
}

/** The result of `task`, or `fallback` when the store is slow, so it does not hold the page back. */
const soon = <T,>(task: Promise<T>, fallback: T) =>
  Promise.race([task, new Promise<T>((resolve) => setTimeout(() => resolve(fallback), 800))]);

if (target !== null) {
  location.replace(`${target}${location.search}${location.hash}`);
} else {
  if (page !== 'docs' && !page.startsWith('docs/')) showLanguage(language, page);
  loadFonts(language);
  // The build fills the root with the page of this language for search engines. The first render
  // replaces it.
  const root = createRoot(document.getElementById('root')!);
  if (page === 'docs' || page.startsWith('docs/')) {
    const docsPage = page as DocsSitePage;
    const content = await loadDocsContent(language);
    showPageText(language, docPageText(language, docsPage, content));
    root.render(
      <StrictMode>
        <DocsPage language={language} page={docsPage} content={content} />
      </StrictMode>,
    );
  } else if (page === 'fastboot') {
    // The fast-boot firmware page needs no imports and no card.
    root.render(
      <StrictMode>
        <FastbootPage initialLanguage={language} />
      </StrictMode>,
    );
  } else if (page === 'resources') {
    root.render(
      <StrictMode>
        <ResourcesPage initialLanguage={language} />
      </StrictMode>,
    );
  } else if (page === 'home') {
    root.render(
      <StrictMode>
        <HomePage initialLanguage={language} />
      </StrictMode>,
    );
  } else {
    // The effects imported and the card connected on an earlier visit. A browser of the film box
    // colors switch before the cover scheme moves its custom covers to a scheme once.
    const [imports, lastCard] = await Promise.all([
      soon(loadImports(), []),
      canWriteCards() ? soon(loadLastCard(), null) : null,
      soon(migrateCoverScheme(browserStorage(), async () => (await loadCatalog(PREVIEW_TARGET)).effects), undefined),
    ]);
    root.render(
      <StrictMode>
        <InstallPage initialLanguage={language} initialImports={imports} initialCard={lastCard} />
      </StrictMode>,
    );
  }
}
