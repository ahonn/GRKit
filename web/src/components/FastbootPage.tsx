import { useState } from 'react';
import { loadFonts } from '../fonts';
import { applyLanguage, STRINGS, type Language } from '../i18n';
import { guideUrl } from '../links';
import { FastbootSection } from './FastbootSection';
import { Footer, Masthead } from './SiteChrome';

/**
 * The page of the fast-boot firmware (`/fastboot/` under each language page): what it changes,
 * and how to make, install, and remove it.
 */
export function FastbootPage({ initialLanguage }: { initialLanguage: Language }) {
  const [language, setLanguage] = useState(initialLanguage);
  const t = STRINGS[language];

  function chooseLanguage(next: Language) {
    loadFonts(next);
    applyLanguage(next, 'fastboot');
    setLanguage(next);
  }

  return (
    <div className="page">
      <Masthead strings={t} language={language} page="fastboot" onLanguage={chooseLanguage} />
      <main>
        <FastbootSection strings={t} terms={guideUrl('TERMS', language)} />
      </main>
      <Footer strings={t} language={language} />
    </div>
  );
}
