/** The groups of the docs, in the order of the docs home page. */
export const DOC_GROUPS = ['start', 'fix', 'cameras', 'features'] as const;
export type DocGroup = (typeof DOC_GROUPS)[number];

/** The pages of the docs in reading order. The edge worker imports this list, so it has no text. */
export const DOC_PAGES = [
  { slug: 'install', group: 'start' },
  { slug: 'update', group: 'start' },
  { slug: 'grk-not-showing', group: 'fix' },
  { slug: 'recovery', group: 'fix' },
  { slug: 'faq', group: 'fix' },
  { slug: 'cameras', group: 'cameras' },
  { slug: 'fast-start', group: 'cameras' },
  { slug: 'firmware-dump', group: 'cameras' },
  { slug: 'film-look', group: 'features' },
  { slug: 'composition', group: 'features' },
  { slug: 'effects', group: 'features' },
  { slug: 'recipes', group: 'features' },
  { slug: 'mode-dial', group: 'features' },
  { slug: 'raw', group: 'features' },
  { slug: 'settings', group: 'features' },
] as const satisfies readonly { slug: string; group: DocGroup }[];

export type DocSlug = (typeof DOC_PAGES)[number]['slug'];

/** The site page of the docs home, and of each doc. */
export type DocsSitePage = 'docs' | `docs/${DocSlug}`;

export const DOCS_SITE_PAGES: readonly DocsSitePage[] = ['docs', ...DOC_PAGES.map((page) => `docs/${page.slug}` as const)];

/** The doc of a site page, or null for the docs home. */
export function docSlug(page: DocsSitePage): DocSlug | null {
  return page === 'docs' ? null : (page.slice('docs/'.length) as DocSlug);
}
