import type { Language } from '../i18n';
import type { DocsContent } from './types';

/** The text of the docs in `language`. Each language is its own chunk. */
export function loadDocsContent(language: Language): Promise<DocsContent> {
  switch (language) {
    case 'en':
      return import('./content/en').then((module) => module.en);
    case 'zh-Hans':
      return import('./content/zh-Hans').then((module) => module.zhHans);
    case 'zh-Hant':
      return import('./content/zh-Hant').then((module) => module.zhHant);
    case 'ja':
      return import('./content/ja').then((module) => module.ja);
    case 'ko':
      return import('./content/ko').then((module) => module.ko);
  }
}
