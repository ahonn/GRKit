import type { DocGroup, DocSlug } from './pages';
import type { DocScreen } from './screens';

/**
 * Text with inline marks: `code`, **bold**, {{KEY}} for a camera button, and [label](target).
 * A target is `doc:<slug>` or `doc:<slug>#<id>`, `site:install`, `site:fastboot`, `support`, or an https address.
 */
export type Inline = string;

export type DocBlock =
  | { kind: 'p'; text: Inline }
  | { kind: 'h2'; id: string; text: string }
  | { kind: 'steps'; items: readonly Inline[] }
  | { kind: 'list'; items: readonly Inline[] }
  | { kind: 'note'; tone: 'plain' | 'warning'; title: string; text: Inline }
  | { kind: 'facts'; items: readonly (readonly [label: string, value: Inline])[] }
  | { kind: 'table'; head: readonly string[]; rows: readonly (readonly Inline[])[] }
  | { kind: 'files'; items: readonly (readonly [path: string, about: string])[] }
  | { kind: 'screen'; screens: readonly DocScreen[] }
  | { kind: 'troubleshooter' }
  | { kind: 'framing' }
  | { kind: 'cameras' };

export interface DocText {
  title: string;
  /** The first line of the page, also its search result text. */
  lead: string;
  /** Other words that a reader may search for this page. */
  keywords: string;
  blocks: readonly DocBlock[];
}

/** The answers and results of the troubleshooter (`Troubleshooter.tsx` holds its paths). */
export type TroubleNode = 'start' | 'installed' | 'card' | 'adj' | 'firmware' | 'scripts' | 'package' | 'files' | 'stops';

export interface TroubleText {
  title: string;
  text?: string;
  steps?: readonly string[];
  /** The answers of a question, in the order of its paths. */
  answers?: readonly string[];
  alt?: string;
}

export interface DocsContent {
  home: {
    title: string;
    lead: string;
    trust: string;
    need: string;
    tasks: readonly { slug: DocSlug; label: string; hint: string }[];
    rescue: string;
    all: string;
  };
  ui: {
    docs: string;
    searchLabel: string;
    searchHome: string;
    searchShort: string;
    searchButton: string;
    noResults: string;
    sendReport: string;
    browse: string;
    previous: string;
    next: string;
    stuck: string;
    report: string;
    photo: string;
    camera: string;
    firmware: string;
  };
  groups: Readonly<Record<DocGroup, string>>;
  trouble: {
    title: string;
    question: (number: number) => string;
    result: string;
    back: string;
    stillBroken: string;
    allRecovery: string;
    cardFolders: string;
    nodes: Readonly<Record<TroubleNode, TroubleText>>;
  };
  framing: {
    label: string;
    ratio: string;
    mask: string;
    masks: readonly [dark: string, black: string, lines: string];
    alt: string;
  };
  pages: Readonly<Record<DocSlug, DocText>>;
}
