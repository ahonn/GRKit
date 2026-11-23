import { existsSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { LANGUAGES } from '../i18n';
import { pagePath, pathPage } from '../languagePage';
import { en } from './content/en';
import { ja } from './content/ja';
import { ko } from './content/ko';
import { zhHans } from './content/zh-Hans';
import { zhHant } from './content/zh-Hant';
import { parseInline } from './inline';
import { DOC_PAGES, type DocSlug } from './pages';
import { DOC_SCREENS } from './screens';
import { searchDocs } from './search';
import type { DocBlock, DocsContent } from './types';

const CONTENT: Record<(typeof LANGUAGES)[number]['code'], DocsContent> = { en, 'zh-Hans': zhHans, 'zh-Hant': zhHant, ja, ko };

const inlineTexts = (block: DocBlock): readonly string[] => {
  switch (block.kind) {
    case 'p':
    case 'note':
      return [block.text];
    case 'steps':
    case 'list':
      return block.items;
    case 'facts':
      return block.items.map(([, value]) => value);
    case 'table':
      return block.rows.flat();
    default:
      return [];
  }
};

/** What a translation must keep: the blocks, and the links, keys, and code of each. */
const shape = (content: DocsContent) =>
  DOC_PAGES.map(({ slug }) =>
    content.pages[slug].blocks.map((block) => ({
      kind: block.kind,
      id: block.kind === 'h2' ? block.id : undefined,
      files: block.kind === 'files' ? block.items.map(([path]) => path) : undefined,
      screens: block.kind === 'screen' ? block.screens : undefined,
      marks: inlineTexts(block).map((text) =>
        parseInline(text)
          .filter((part) => part.kind === 'link' || part.kind === 'code' || part.kind === 'key')
          .map((part) => (part.kind === 'link' ? `link:${part.target}` : `${part.kind}:${part.text}`))
          .sort(),
      ),
    })),
  );

describe('docs', () => {
  it('keeps the blocks, links, keys, and code in each language', () => {
    for (const content of Object.values(CONTENT)) expect(shape(content)).toEqual(shape(en));
  });

  it('gives each troubleshooter question the same answers in each language', () => {
    for (const content of Object.values(CONTENT)) {
      for (const [id, node] of Object.entries(en.trouble.nodes)) {
        const text = content.trouble.nodes[id as keyof typeof en.trouble.nodes];
        expect(text.answers?.length).toBe(node.answers?.length);
        expect(text.steps?.length).toBe(node.steps?.length);
      }
    }
  });

  it('links only to pages and sections that exist', () => {
    for (const { slug } of DOC_PAGES) {
      for (const block of en.pages[slug].blocks) {
        for (const text of inlineTexts(block)) {
          for (const part of parseInline(text)) {
            if (part.kind !== 'link' || !part.target.startsWith('doc:')) continue;
            const [target, id] = part.target.slice(4).split('#') as [DocSlug, string | undefined];
            expect(DOC_PAGES.map((page) => page.slug)).toContain(target);
            if (id) expect(en.pages[target].blocks.some((item) => item.kind === 'h2' && item.id === id)).toBe(true);
          }
        }
      }
    }
  });

  it('has a capture of each camera screen in each language', () => {
    for (const screen of Object.keys(DOC_SCREENS)) {
      if (screen === 'modeset') continue;
      for (const { code } of LANGUAGES) {
        expect(existsSync(new URL(`../../public/docs-screens/${screen}.${code}.jpg`, import.meta.url)), `${screen} ${code}`).toBe(true);
      }
    }
  });

  it('routes each doc in each language', () => {
    for (const { code } of LANGUAGES) {
      expect(pathPage(pagePath(code, 'docs'))).toEqual({ language: code, page: 'docs' });
      for (const { slug } of DOC_PAGES) {
        expect(pathPage(pagePath(code, `docs/${slug}`))).toEqual({ language: code, page: `docs/${slug}` });
      }
    }
  });

  it('finds a page by title, keyword, and error number', () => {
    expect(searchDocs(en, 'framing')[0]?.slug).toBe('composition');
    expect(searchDocs(en, 'bloom')[0]?.slug).toBe('effects');
    expect(searchDocs(en, '47')[0]?.slug).toBe('grk-not-showing');
    expect(searchDocs(en, 'zzzz')).toEqual([]);
  });
});
