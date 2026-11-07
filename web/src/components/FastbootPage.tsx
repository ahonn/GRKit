import { useState } from 'react';
import { loadFonts } from '../fonts';
import { applyLanguage, LANGUAGES, STRINGS, type Language } from '../i18n';
import { LANGUAGE_PATHS } from '../languagePage';
import { guideUrl } from '../links';
import { FastbootSection } from './FastbootSection';

/**
 * The page of the fast-boot firmware (`/fastboot/` under each language page): what it changes,
 * how to make, install, and remove it, and a way back to the language page.
 */
export function FastbootPage({ initialLanguage }: { initialLanguage: Language }) {
  const [language, setLanguage] = useState(initialLanguage);
  const t = STRINGS[language];

  function chooseLanguage(next: Language) {
    loadFonts(next);
    applyLanguage(next, true);
    setLanguage(next);
  }

  return (
    <div className="page">
      <header className="masthead">
        <div className="masthead-top">
          <div className="masthead-title">
            <h1>
              <a href={LANGUAGE_PATHS[language]} className="masthead-home">
                GRK<span className="dot">i</span>t
              </a>
            </h1>
          </div>
          <div className="languages">
            <nav className="masthead-nav masthead-back" aria-label="GRKit">
              <a href={LANGUAGE_PATHS[language]}>{t.fastboot.back}</a>
            </nav>
            <span className="language-select">
              <select
                className="button"
                aria-label={t.language}
                value={language}
                onChange={(event) => chooseLanguage(event.target.value as Language)}
              >
                {LANGUAGES.map((item) => (
                  <option key={item.code} value={item.code} lang={item.code}>
                    {item.label}
                  </option>
                ))}
              </select>
            </span>
          </div>
        </div>
      </header>
      <main>
        <FastbootSection strings={t} terms={guideUrl('TERMS', language)} />
      </main>
      <footer className="footer">
        <div className="footer-links">
          <ul>
            <li>
              <a href={guideUrl('RECOVERY', language)}>{t.recovery}</a>
            </li>
          </ul>
          <ul>
            <li>
              <a href={guideUrl('TERMS', language)}>{t.terms}</a>
            </li>
          </ul>
        </div>
        <div className="footer-meta">
          <p>{t.trademarks}</p>
        </div>
      </footer>
    </div>
  );
}
