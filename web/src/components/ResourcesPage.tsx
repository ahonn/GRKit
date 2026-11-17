import { useState } from 'react';
import { loadFonts } from '../fonts';
import { applyLanguage, STRINGS, type Language } from '../i18n';
import { RESOURCE_LINKS, type ResourceId } from '../resources';
import { Footer, Masthead } from './SiteChrome';

/**
 * The page of videos and articles about GRKit (`/resources/` under each language page). The
 * titles stay as the publishers wrote them; the notes under them are in the page language.
 */
export function ResourcesPage({ initialLanguage }: { initialLanguage: Language }) {
  const [language, setLanguage] = useState(initialLanguage);
  const t = STRINGS[language];
  const page = t.resources;

  function chooseLanguage(next: Language) {
    loadFonts(next);
    applyLanguage(next, 'resources');
    setLanguage(next);
  }

  const videos = RESOURCE_LINKS.filter((item) => item.kind === 'video');
  const articles = RESOURCE_LINKS.filter((item) => item.kind === 'article');

  return (
    <div className="page">
      <Masthead strings={t} language={language} page="resources" onLanguage={chooseLanguage} />
      <main>
        <section className="section" aria-labelledby="resources-label">
          <h1 id="resources-label">{page.title}</h1>
          <div className="section-body">
            <p className="resources-lead">{page.lead}</p>
          </div>
        </section>
        <ResourceGroup id="resources-videos" title={page.videos} items={videos} notes={page.notes} />
        <ResourceGroup id="resources-articles" title={page.articles} items={articles} notes={page.notes} />
      </main>
      <Footer strings={t} language={language} />
    </div>
  );
}

function ResourceGroup({
  id,
  title,
  items,
  notes,
}: {
  id: string;
  title: string;
  items: readonly (typeof RESOURCE_LINKS)[number][];
  notes: Readonly<Record<ResourceId, string>>;
}) {
  return (
    <section className="section" aria-labelledby={id}>
      <h2 id={id}>{title}</h2>
      <div className="section-body">
        <ul className="resource-list">
          {items.map((item) => (
            <li key={item.id} className="resource-item">
              <a href={item.href} target="_blank" rel="noreferrer">
                {item.title}
              </a>
              <p className="resource-meta">
                <span className="resource-source">{item.source}</span>
                <span aria-hidden="true"> · </span>
                <span>{notes[item.id]}</span>
              </p>
            </li>
          ))}
        </ul>
      </div>
    </section>
  );
}
