import { useCallback, useId, useState, type FormEvent, type KeyboardEvent } from 'react';
import { CAMERAS } from '../cameras';
import { parseInline, targetUrl } from '../docs/inline';
import { DOC_GROUPS, DOC_PAGES, docSlug, type DocSlug, type DocsSitePage } from '../docs/pages';
import { DOC_SCREENS, screenModel, screenUrl, type DocScreen } from '../docs/screens';
import { searchDocs, type DocResult } from '../docs/search';
import type { DocBlock, DocsContent, Inline, TroubleNode } from '../docs/types';
import { loadFonts } from '../fonts';
import { FRAMING_RATIOS } from '../generated/framing';
import { applyLanguage, STRINGS, type Language } from '../i18n';
import { landingImage } from '../landing';
import { pagePath } from '../languagePage';
import { Footer, Masthead, SUPPORT_FORM } from './SiteChrome';

const docPath = (language: Language, slug: DocSlug) => pagePath(language, `docs/${slug}`);

function Text({ text, language }: { text: Inline; language: Language }) {
  return parseInline(text).map((part, index) => {
    switch (part.kind) {
      case 'code':
        return <code key={index}>{part.text}</code>;
      case 'bold':
        return <strong key={index}>{part.text}</strong>;
      case 'key':
        return <kbd key={index}>{part.text}</kbd>;
      case 'link':
        return (
          <a key={index} href={targetUrl(part.target, language, SUPPORT_FORM)}>
            {part.text}
          </a>
        );
      default:
        return <span key={index}>{part.text}</span>;
    }
  });
}

/** The search of the docs, with its result list. `/` moves the focus to it. */
function DocsSearch({ content, language, large }: { content: DocsContent; language: Language; large?: boolean }) {
  const [query, setQuery] = useState('');
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState(0);
  const listId = useId();
  const t = content.ui;
  const results: DocResult[] = searchDocs(content, query);
  const showList = open && query.trim() !== '';
  const focusOnSlash = useCallback((input: HTMLInputElement | null) => {
    if (!input) return;
    const onKey = (event: globalThis.KeyboardEvent) => {
      const { target } = event;
      if (event.key !== '/' || (target instanceof Element && target.closest('input, select, textarea'))) return;
      event.preventDefault();
      input.focus();
    };
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  }, []);

  function onKeyDown(event: KeyboardEvent<HTMLInputElement>) {
    if (event.key === 'Escape') {
      setQuery('');
      setActive(0);
    } else if (results.length > 0 && event.key === 'ArrowDown') {
      event.preventDefault();
      setActive((active + 1) % results.length);
    } else if (results.length > 0 && event.key === 'ArrowUp') {
      event.preventDefault();
      setActive((active - 1 + results.length) % results.length);
    }
  }

  function onSubmit(event: FormEvent) {
    event.preventDefault();
    const result = results[active];
    if (result) location.assign(docPath(language, result.slug));
  }

  return (
    <div
      className={large ? 'docs-search large' : 'docs-search'}
      onBlur={(event) => {
        if (!event.currentTarget.contains(event.relatedTarget)) setOpen(false);
      }}
    >
      <form role="search" onSubmit={onSubmit}>
        <input
          ref={focusOnSlash}
          type="search"
          role="combobox"
          aria-expanded={showList}
          aria-controls={listId}
          aria-autocomplete="list"
          aria-activedescendant={showList && results.length > 0 ? `${listId}-${active}` : undefined}
          aria-label={t.searchLabel}
          placeholder={large ? t.searchHome : t.searchShort}
          autoComplete="off"
          value={query}
          onChange={(event) => {
            setQuery(event.target.value);
            setActive(0);
            setOpen(true);
          }}
          onFocus={() => setOpen(true)}
          onKeyDown={onKeyDown}
        />
        {large && <button type="submit">{t.searchButton}</button>}
      </form>
      {showList && (
        <ul id={listId} role="listbox" aria-label={t.searchLabel} className="docs-results">
          {results.map((result, index) => (
            <li key={result.slug} id={`${listId}-${index}`} role="option" aria-selected={index === active}>
              <a href={docPath(language, result.slug)} tabIndex={-1}>
                <span>{result.title}</span>
                <small>{result.group}</small>
              </a>
            </li>
          ))}
          {results.length === 0 && (
            <li role="presentation" className="docs-no-results">
              {t.noResults} <a href={`${docPath(language, 'grk-not-showing')}#report`}>{t.sendReport}</a>
            </li>
          )}
        </ul>
      )}
    </div>
  );
}

/** Each question and its next node, in the order of the answers in the text. */
const TROUBLE_PATHS: Partial<Record<TroubleNode, readonly TroubleNode[]>> = {
  start: ['adj', 'installed', 'firmware', 'stops'],
  installed: ['card', 'scripts'],
  card: ['package', 'files'],
};
const TROUBLE_SCREENS: Partial<Record<TroubleNode, DocScreen>> = {
  start: 'adj-grk',
  installed: 'modeset',
  adj: 'adj-grk',
  firmware: 'modeset',
  scripts: 'modeset',
};
const TROUBLE_TREE = new Set<TroubleNode>(['card', 'package', 'files']);

function Troubleshooter({ content, language }: { content: DocsContent; language: Language }) {
  const [trail, setTrail] = useState<readonly TroubleNode[]>(['start']);
  const t = content.trouble;
  const id = trail[trail.length - 1]!;
  const node = t.nodes[id];
  const paths = TROUBLE_PATHS[id];
  const screen = TROUBLE_SCREENS[id];
  const tree = TROUBLE_TREE.has(id);
  return (
    <section className="docs-check" aria-live="polite" aria-label={t.title}>
      <div className="docs-check-head">
        {t.title}
        <span>{paths ? t.question(trail.length) : t.result}</span>
      </div>
      <div className={screen || tree ? 'docs-check-body' : 'docs-check-body solo'}>
        <div>
          <p className="docs-question">{node.title}</p>
          {node.text && <p className="docs-hint">{node.text}</p>}
          {node.steps && (
            <ol className="docs-fix">
              {node.steps.map((step) => (
                <li key={step}>{step}</li>
              ))}
            </ol>
          )}
          {paths && (
            <div className="docs-answers">
              {node.answers?.map((answer, index) => (
                <button key={answer} type="button" onClick={() => setTrail([...trail, paths[index]!])}>
                  {answer}
                </button>
              ))}
            </div>
          )}
          {(!paths || trail.length > 1) && (
            <div className="docs-check-actions">
              {!paths &&
                (id === 'stops' ? (
                  <a className="solid" href={docPath(language, 'recovery')}>
                    {t.allRecovery}
                  </a>
                ) : (
                  <a className="solid" href={SUPPORT_FORM}>
                    {t.stillBroken}
                  </a>
                ))}
              {trail.length > 1 && (
                <button type="button" onClick={() => setTrail(trail.slice(0, -1))}>
                  {t.back}
                </button>
              )}
            </div>
          )}
        </div>
        {screen && <img src={screenUrl(screen, language)} alt={node.alt ?? ''} width={720} height={480} />}
        {tree && (
          <pre className="docs-tree" aria-label={t.cardFolders}>
            {'SD card\n└ SCRIPT\n  └ GRKIT\n    └ INSTALL.ERR'}
          </pre>
        )}
      </div>
    </section>
  );
}

const MASK_STYLES = [
  { boxShadow: '0 0 0 2000px rgb(0 0 0 / 0.5)' },
  { boxShadow: '0 0 0 2000px #000' },
  { outline: '2px solid #fff', outlineOffset: '-2px' },
] as const;

function FramingPreview({ content }: { content: DocsContent }) {
  const [ratioIndex, setRatioIndex] = useState(0);
  const [mask, setMask] = useState(0);
  const t = content.framing;
  const ratio = FRAMING_RATIOS[ratioIndex]!;
  const wide = ratio.ratio >= 1.5;
  const area = {
    width: `${wide ? 100 : (ratio.ratio / 1.5) * 100}%`,
    height: `${wide ? (1.5 / ratio.ratio) * 100 : 100}%`,
    ...MASK_STYLES[mask],
  };
  return (
    <section aria-label={t.label}>
      <div className="docs-frame">
        <img src={landingImage('looks/portra-400.jpg')} alt={t.alt} width={1200} height={800} />
        <div className="docs-frame-area" style={area} />
        <span className="docs-frame-tag">{ratio.label}</span>
      </div>
      <div className="docs-frame-controls">
        <div>
          <span>{t.ratio}</span>
          <div className="docs-chips">
            {FRAMING_RATIOS.map((item, index) => (
              <button key={item.label} type="button" aria-pressed={index === ratioIndex} onClick={() => setRatioIndex(index)}>
                {item.name}
              </button>
            ))}
          </div>
        </div>
        <div>
          <span>{t.mask}</span>
          <div className="docs-chips">
            {t.masks.map((item, index) => (
              <button key={item} type="button" aria-pressed={index === mask} onClick={() => setMask(index)}>
                {item}
              </button>
            ))}
          </div>
        </div>
      </div>
      <p className="docs-credit">
        {content.ui.photo}: <a href="https://www.flickr.com/photos/mujitra/53311001084/">MIKI Yoshihito</a>,{' '}
        <a href="https://creativecommons.org/licenses/by/2.0/">CC BY 2.0</a>
      </p>
    </section>
  );
}

function CameraTable({ content }: { content: DocsContent }) {
  return (
    <div className="docs-table">
      <table>
        <thead>
          <tr>
            <th scope="col">{content.ui.camera}</th>
            <th scope="col">{content.ui.firmware}</th>
          </tr>
        </thead>
        <tbody>
          {CAMERAS.filter((camera) => camera.support).map((camera) => (
            <tr key={camera.key}>
              <td>{camera.name}</td>
              <td>{camera.support!.firmware}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function Block({ block, content, language }: { block: DocBlock; content: DocsContent; language: Language }) {
  switch (block.kind) {
    case 'p':
      return (
        <p>
          <Text text={block.text} language={language} />
        </p>
      );
    case 'h2':
      return <h2 id={block.id}>{block.text}</h2>;
    case 'steps':
      return (
        <ol className="docs-steps">
          {block.items.map((item) => (
            <li key={item}>
              <span>
                <Text text={item} language={language} />
              </span>
            </li>
          ))}
        </ol>
      );
    case 'list':
      return (
        <ul className="docs-list">
          {block.items.map((item) => (
            <li key={item}>
              <Text text={item} language={language} />
            </li>
          ))}
        </ul>
      );
    case 'note':
      return (
        <aside className={`docs-note ${block.tone}`}>
          <strong>{block.title}</strong>
          <span>
            <Text text={block.text} language={language} />
          </span>
        </aside>
      );
    case 'facts':
      return (
        <dl className="docs-facts">
          {block.items.map(([label, value]) => (
            <div key={label}>
              <dt>{label}</dt>
              <dd>
                <Text text={value} language={language} />
              </dd>
            </div>
          ))}
        </dl>
      );
    case 'table':
      return (
        <div className="docs-table">
          <table>
            <thead>
              <tr>
                {block.head.map((cell) => (
                  <th key={cell} scope="col">
                    {cell}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {block.rows.map((row) => (
                <tr key={row[0]}>
                  {row.map((cell, index) => (
                    <td key={index}>
                      <Text text={cell} language={language} />
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      );
    case 'files':
      return (
        <ul className="docs-files">
          {block.items.map(([path, about]) => (
            <li key={path}>
              <code>{path}</code>
              <span>{about}</span>
            </li>
          ))}
        </ul>
      );
    case 'screen':
      return (
        <div className={block.screens.length > 1 ? 'docs-screens compare' : 'docs-screens'}>
          {block.screens.map((screen) => (
            <figure key={screen}>
              <img src={screenUrl(screen, language)} alt={DOC_SCREENS[screen].alt[language]} width={720} height={480} loading="lazy" />
              {block.screens.length > 1 && screenModel(screen) && <figcaption>{screenModel(screen)}</figcaption>}
            </figure>
          ))}
        </div>
      );
    case 'troubleshooter':
      return <Troubleshooter content={content} language={language} />;
    case 'framing':
      return <FramingPreview content={content} />;
    case 'cameras':
      return <CameraTable content={content} />;
  }
}

function DocsHome({ content, language }: { content: DocsContent; language: Language }) {
  const t = content.home;
  return (
    <main>
      <section className="docs-hero">
        <div>
          <h1>{t.title}</h1>
          <p className="docs-lead">{t.lead}</p>
          <DocsSearch content={content} language={language} large />
          <p className="docs-trust">{t.trust}</p>
        </div>
        <div>
          <nav className="docs-tasks" aria-label={t.need}>
            <div>{t.need}</div>
            {t.tasks.map((task) => (
              <a key={task.slug} href={docPath(language, task.slug)}>
                {task.label}
                <small>{task.hint}</small>
              </a>
            ))}
          </nav>
          <a className="docs-rescue" href={docPath(language, 'recovery')}>
            <span>{t.rescue}</span>
            <span aria-hidden="true">›</span>
          </a>
        </div>
      </section>
      <h2 className="docs-index-title">{t.all}</h2>
      {DOC_GROUPS.map((group) => (
        <section key={group} className="docs-index-row">
          <h3>{content.groups[group]}</h3>
          <ul>
            {DOC_PAGES.filter((page) => page.group === group).map((page) => (
              <li key={page.slug}>
                <a href={docPath(language, page.slug)}>{content.pages[page.slug].title}</a>
              </li>
            ))}
          </ul>
        </section>
      ))}
    </main>
  );
}

function DocsNav({ content, language, slug }: { content: DocsContent; language: Language; slug: DocSlug }) {
  const [open, setOpen] = useState(false);
  return (
    <div className="docs-side">
      <button type="button" className="docs-side-toggle" aria-expanded={open} onClick={() => setOpen(!open)}>
        {content.ui.browse}
        <span aria-hidden="true">▾</span>
      </button>
      <nav className={open ? 'docs-side-body' : 'docs-side-body closed'} aria-label={content.ui.docs}>
        {DOC_GROUPS.map((name) => {
          const pages = DOC_PAGES.filter((page) => page.group === name);
          return (
            <div key={name}>
              <h2>{content.groups[name]}</h2>
              <ul>
                {pages.map((page) => (
                  <li key={page.slug}>
                    <a href={docPath(language, page.slug)} aria-current={page.slug === slug ? 'page' : undefined}>
                      {content.pages[page.slug].title}
                    </a>
                  </li>
                ))}
              </ul>
            </div>
          );
        })}
      </nav>
    </div>
  );
}

function DocArticle({ content, language, slug }: { content: DocsContent; language: Language; slug: DocSlug }) {
  const index = DOC_PAGES.findIndex((page) => page.slug === slug);
  const { group } = DOC_PAGES[index]!;
  const page = content.pages[slug];
  const previous = DOC_PAGES[index - 1];
  const next = DOC_PAGES[index + 1];
  return (
    <div className="docs-layout">
      <DocsNav content={content} language={language} slug={slug} />
      <main className="docs-article">
        <p className="docs-crumb">
          <a href={pagePath(language, 'docs')}>{content.ui.docs}</a> / {content.groups[group]}
        </p>
        <h1>{page.title}</h1>
        <p className="docs-lead">{page.lead}</p>
        {page.blocks.map((block, blockIndex) => (
          <Block key={blockIndex} block={block} content={content} language={language} />
        ))}
        <nav className="docs-pager" aria-label={`${content.ui.previous} / ${content.ui.next}`}>
          {previous ? (
            <a href={docPath(language, previous.slug)}>
              <small>{content.ui.previous}</small>
              {content.pages[previous.slug].title}
            </a>
          ) : (
            <span />
          )}
          {next ? (
            <a className="next" href={docPath(language, next.slug)}>
              <small>{content.ui.next}</small>
              {content.pages[next.slug].title}
            </a>
          ) : (
            <span />
          )}
        </nav>
        <p className="docs-help">
          {content.ui.stuck} <a href={SUPPORT_FORM}>{content.ui.report}</a>
        </p>
      </main>
    </div>
  );
}

/** The docs home and each doc (`/docs/` under each language page). */
export function DocsPage({ language, page, content }: { language: Language; page: DocsSitePage; content: DocsContent }) {
  const t = STRINGS[language];
  const slug = docSlug(page);

  // Each language has its own text chunk, so another language loads as its own page.
  function chooseLanguage(next: Language) {
    loadFonts(next);
    applyLanguage(next, page);
    location.reload();
  }

  return (
    <div className="page docs-page">
      <Masthead
        strings={t}
        language={language}
        page={page}
        onLanguage={chooseLanguage}
        search={slug ? <DocsSearch content={content} language={language} /> : undefined}
      />
      {slug ? <DocArticle content={content} language={language} slug={slug} /> : <DocsHome content={content} language={language} />}
      <Footer strings={t} language={language} />
    </div>
  );
}
