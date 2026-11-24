import type { Language } from '../i18n';
import { fastbootPath, installPath, pagePath } from '../languagePage';
import type { DocSlug } from './pages';

export type InlinePart =
  | { kind: 'text'; text: string }
  | { kind: 'code'; text: string }
  | { kind: 'bold'; text: string }
  | { kind: 'key'; text: string }
  | { kind: 'link'; text: string; target: string };

const MARKS = /`([^`]+)`|\*\*([^*]+)\*\*|\{\{([^}]+)\}\}|\[([^\]]+)\]\(([^)\s]+)\)/g;

/** Splits text with inline marks into its parts. */
export function parseInline(text: string): InlinePart[] {
  const parts: InlinePart[] = [];
  let at = 0;
  for (const match of text.matchAll(MARKS)) {
    if (match.index > at) parts.push({ kind: 'text', text: text.slice(at, match.index) });
    const [whole, code, bold, key, label, target] = match;
    if (code !== undefined) parts.push({ kind: 'code', text: code });
    else if (bold !== undefined) parts.push({ kind: 'bold', text: bold });
    else if (key !== undefined) parts.push({ kind: 'key', text: key });
    else parts.push({ kind: 'link', text: label!, target: target! });
    at = match.index + whole.length;
  }
  if (at < text.length) parts.push({ kind: 'text', text: text.slice(at) });
  return parts;
}

/** The text of inline marks as a reader sees it, for the search index. */
export function plainInline(text: string): string {
  return parseInline(text)
    .map((part) => part.text)
    .join('');
}

/** The address of a link target in `language`. */
export function targetUrl(target: string, language: Language, support: string): string {
  if (target === 'support') return support;
  if (target === 'site:install') return installPath(language);
  if (target === 'site:fastboot') return fastbootPath(language);
  if (target.startsWith('doc:')) {
    const [slug, id] = target.slice(4).split('#');
    return `${pagePath(language, `docs/${slug as DocSlug}`)}${id ? `#${id}` : ''}`;
  }
  return target;
}
