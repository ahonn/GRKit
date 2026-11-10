import { useState } from 'react';
import { canWriteCards } from '../card';
import { loadFonts } from '../fonts';
import { applyLanguage, STRINGS, type Language } from '../i18n';
import { AppPromo } from './AppPromo';
import { Landing } from './Landing';
import { Footer, Masthead } from './SiteChrome';

/** The home page of a language: what GRKit does, with a way into the installer. */
export function HomePage({ initialLanguage }: { initialLanguage: Language }) {
  const [language, setLanguage] = useState<Language>(initialLanguage);
  const t = STRINGS[language];

  function chooseLanguage(next: Language) {
    loadFonts(next);
    applyLanguage(next, 'home');
    setLanguage(next);
  }

  return (
    <>
      <div className="page">
        <Masthead strings={t} language={language} page="home" onLanguage={chooseLanguage} />
        <main>
          <Landing strings={t} language={language} canWrite={canWriteCards()} />
        </main>
        <Footer strings={t} language={language} />
      </div>
      <AppPromo strings={t} />
    </>
  );
}
