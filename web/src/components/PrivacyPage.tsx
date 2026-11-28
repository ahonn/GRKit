import { useState } from 'react';
import { loadFonts } from '../fonts';
import { applyLanguage, STRINGS, type Language } from '../i18n';
import { PRIVACY_CONTACT, PRIVACY_INTRO, PRIVACY_PARTS, PRIVACY_UPDATED } from '../privacy';
import { Footer, Masthead } from './SiteChrome';

/**
 * The privacy policy of the phone app and the site (`/app/privacy/` under each language page).
 * The title and the lead are in the page language; the policy is in English.
 */
export function PrivacyPage({ initialLanguage }: { initialLanguage: Language }) {
  const [language, setLanguage] = useState(initialLanguage);
  const t = STRINGS[language];

  function chooseLanguage(next: Language) {
    loadFonts(next);
    applyLanguage(next, 'app/privacy');
    setLanguage(next);
  }

  return (
    <div className="page">
      <Masthead strings={t} language={language} page="app/privacy" onLanguage={chooseLanguage} />
      <main>
        <section className="section" aria-labelledby="privacy-label">
          <h1 id="privacy-label">{t.privacyPage.title}</h1>
          <div className="section-body">
            {language !== 'en' && <p className="resources-lead">{t.privacyPage.lead}</p>}
            <div className="privacy-text" lang="en">
              <p className="privacy-date">Last updated: {PRIVACY_UPDATED}</p>
              <p>{PRIVACY_INTRO}</p>
            </div>
          </div>
        </section>
        {PRIVACY_PARTS.map((part) => (
          <section key={part.id} className="section" aria-labelledby={part.id} lang="en">
            <h2 id={part.id}>{part.title}</h2>
            <div className="section-body privacy-text">
              {part.text.map((paragraph) => (
                <p key={paragraph}>{paragraph}</p>
              ))}
              {part.points.length > 0 && (
                <ul className="privacy-points">
                  {part.points.map((point) => (
                    <li key={point.name}>
                      <strong>{point.name}</strong> {point.text}
                    </li>
                  ))}
                </ul>
              )}
            </div>
          </section>
        ))}
        <section className="section" aria-labelledby="privacy-contact" lang="en">
          <h2 id="privacy-contact">Contact</h2>
          <div className="section-body privacy-text">
            <p>
              <a href={`mailto:${PRIVACY_CONTACT}`}>{PRIVACY_CONTACT}</a>
            </p>
          </div>
        </section>
      </main>
      <Footer strings={t} language={language} />
    </div>
  );
}
