import { plainInline } from './inline';
import { DOC_PAGES, type DocSlug } from './pages';
import type { DocBlock, DocsContent } from './types';

export interface DocResult {
  slug: DocSlug;
  title: string;
  group: string;
}

function blockText(block: DocBlock): string {
  switch (block.kind) {
    case 'p':
    case 'note':
      return plainInline(block.text);
    case 'h2':
      return block.text;
    case 'steps':
    case 'list':
      return block.items.map(plainInline).join(' ');
    case 'facts':
      return block.items.map(([label, value]) => `${label} ${plainInline(value)}`).join(' ');
    case 'table':
      return [...block.head, ...block.rows.flat().map(plainInline)].join(' ');
    case 'files':
      return block.items.flat().join(' ');
    default:
      return '';
  }
}

/** The pages that hold each word of `query`, best first: title, then keywords, then the text. */
export function searchDocs(content: DocsContent, query: string, limit = 6): DocResult[] {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean);
  if (words.length === 0) return [];
  const found: { result: DocResult; score: number; order: number }[] = [];
  DOC_PAGES.forEach(({ slug, group }, order) => {
    const page = content.pages[slug];
    const title = page.title.toLowerCase();
    const keywords = `${content.groups[group]} ${page.keywords}`.toLowerCase();
    const body = `${page.lead} ${page.blocks.map(blockText).join(' ')}`.toLowerCase();
    let score = 0;
    for (const word of words) {
      const points = title.startsWith(word) ? 8 : title.includes(word) ? 6 : keywords.includes(word) ? 4 : body.includes(word) ? 1 : 0;
      if (points === 0) return;
      score += points;
    }
    found.push({ result: { slug, title: page.title, group: content.groups[group] }, score, order });
  });
  return found
    .sort((a, b) => b.score - a.score || a.order - b.order)
    .slice(0, limit)
    .map((item) => item.result);
}
