// The language pages of the site. The edge worker (edge.ts) and the page share this module, so
// it must not touch the DOM at import time.
import type { Language } from './i18n';

/** The address of each language page. English is the root page, which other visitors also get. */
export const LANGUAGE_PATHS: Readonly<Record<Language, string>> = {
  en: '/',
  'zh-Hans': '/zh-hans/',
  'zh-Hant': '/zh-hant/',
  ja: '/ja/',
  ko: '/ko/',
};

/** The cookie that keeps the language that the visitor chose. The edge worker reads it. */
export const LANGUAGE_COOKIE = 'grkit_language';

const CODES = Object.keys(LANGUAGE_PATHS) as Language[];

export function isLanguage(value: unknown): value is Language {
  return CODES.some((code) => code === value);
}

/** The language of the page at `pathname`. Any other path is English. */
export function pathLanguage(pathname: string): Language {
  const path = pathname.toLowerCase().replace(/\/?$/, '/');
  return CODES.find((code) => LANGUAGE_PATHS[code] === path) ?? 'en';
}

/** Countries where Traditional Chinese is the usual script. */
const TRADITIONAL = new Set(['TW', 'HK', 'MO']);

/**
 * Maps language tags, most preferred first, to a supported language. Traditional script wins
 * for TW, HK, and MO. A bare `zh` takes the script of `country` when the caller knows it.
 */
export function matchLanguage(tags: readonly string[], country?: string | null): Language {
  for (const tag of tags) {
    const lower = tag.toLowerCase();
    if (lower === 'zh' || lower.startsWith('zh-')) {
      if (/hant|-(tw|hk|mo)\b/.test(lower)) return 'zh-Hant';
      if (/hans|-(cn|sg|my)\b/.test(lower)) return 'zh-Hans';
      return country && TRADITIONAL.has(country) ? 'zh-Hant' : 'zh-Hans';
    }
    if (lower === 'ja' || lower.startsWith('ja-')) return 'ja';
    if (lower === 'ko' || lower.startsWith('ko-')) return 'ko';
    if (lower === 'en' || lower.startsWith('en-')) return 'en';
  }
  return 'en';
}

/** The tags of an Accept-Language header, most preferred first, without `*` and q=0 entries. */
export function acceptedLanguages(header: string): string[] {
  return header
    .split(',')
    .map((part, index) => {
      const [tag = '', ...params] = part.split(';').map((item) => item.trim());
      const q = params.find((param) => param.startsWith('q='));
      const quality = q === undefined ? 1 : Number(q.slice(2));
      return { tag, quality: Number.isFinite(quality) ? quality : 0, index };
    })
    .filter((item) => item.tag !== '' && item.tag !== '*' && item.quality > 0)
    .sort((a, b) => b.quality - a.quality || a.index - b.index)
    .map((item) => item.tag);
}

/** The value of the cookie `name` in a Cookie header or `document.cookie`. */
export function cookieValue(cookies: string, name: string): string | null {
  for (const part of cookies.split(';')) {
    const [key, ...value] = part.trim().split('=');
    if (key === name) return value.join('=');
  }
  return null;
}

/** The language of the countries where most visitors read it. */
const COUNTRY_LANGUAGES: Readonly<Record<string, Language>> = {
  CN: 'zh-Hans',
  TW: 'zh-Hant',
  HK: 'zh-Hant',
  MO: 'zh-Hant',
  JP: 'ja',
  KR: 'ko',
};

export interface Visitor {
  /** The Cookie header. */
  cookie: string | null;
  /** The Accept-Language header. */
  acceptLanguage: string | null;
  /** The country of the visitor's IP address, as an ISO 3166-1 alpha-2 code. */
  country: string | null;
}

/**
 * The language page for a visitor of the root page. A language that the visitor chose comes
 * first, then the browser languages. The country of the IP address decides only when the
 * browser sends no language, and it picks the script of a bare `zh`.
 */
export function visitorLanguage({ cookie, acceptLanguage, country }: Visitor): Language {
  const chosen = cookie === null ? null : cookieValue(cookie, LANGUAGE_COOKIE);
  if (isLanguage(chosen)) return chosen;
  const tags = acceptedLanguages(acceptLanguage ?? '');
  if (tags.length > 0) return matchLanguage(tags, country);
  return (country && COUNTRY_LANGUAGES[country]) || 'en';
}
