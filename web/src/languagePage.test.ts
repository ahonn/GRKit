import { afterEach, describe, expect, it, vi } from 'vitest';
import edge from './edge';
import {
  acceptedLanguages,
  cookieValue,
  fastbootPath,
  isFastbootPath,
  pathLanguage,
  visitorLanguage,
  type Visitor,
} from './languagePage';

const visitor = (fields: Partial<Visitor>): Visitor => ({ cookie: null, acceptLanguage: null, country: null, ...fields });

describe('pathLanguage', () => {
  it('reads the language page of an address', () => {
    expect(pathLanguage('/')).toBe('en');
    expect(pathLanguage('/zh-hant/')).toBe('zh-Hant');
    expect(pathLanguage('/zh-Hans')).toBe('zh-Hans');
    expect(pathLanguage('/ja/')).toBe('ja');
    expect(pathLanguage('/ko')).toBe('ko');
    expect(pathLanguage('/faq')).toBe('en');
  });

  it('reads the language of a fast-boot firmware page', () => {
    expect(pathLanguage('/fastboot/')).toBe('en');
    expect(pathLanguage('/zh-hans/fastboot')).toBe('zh-Hans');
    expect(pathLanguage('/ko/fastboot/')).toBe('ko');
    expect(isFastbootPath('/ja/fastboot')).toBe(true);
    expect(isFastbootPath('/ja/')).toBe(false);
    expect(isFastbootPath('/faq/fastboot/')).toBe(false);
    expect(fastbootPath('zh-Hant')).toBe('/zh-hant/fastboot/');
    expect(fastbootPath('en')).toBe('/fastboot/');
  });
});

describe('acceptedLanguages', () => {
  it('orders the tags by quality and drops the wildcard and q=0', () => {
    expect(acceptedLanguages('en-US,en;q=0.9,ja;q=0.95,*;q=0.5,ko;q=0')).toEqual(['en-US', 'ja', 'en']);
    expect(acceptedLanguages('')).toEqual([]);
    expect(acceptedLanguages('*')).toEqual([]);
  });
});

describe('cookieValue', () => {
  it('finds one cookie among others', () => {
    expect(cookieValue('a=1; grkit_language=ja; b=2', 'grkit_language')).toBe('ja');
    expect(cookieValue('a=1', 'grkit_language')).toBeNull();
  });
});

describe('visitorLanguage', () => {
  it('keeps the language that the visitor chose', () => {
    const chosen = visitor({ cookie: 'grkit_language=en', acceptLanguage: 'zh-TW', country: 'TW' });
    expect(visitorLanguage(chosen)).toBe('en');
    expect(visitorLanguage(visitor({ cookie: 'grkit_language=ko', acceptLanguage: 'ja' }))).toBe('ko');
  });

  it('ignores a cookie with an unknown language', () => {
    expect(visitorLanguage(visitor({ cookie: 'grkit_language=fr', acceptLanguage: 'ja' }))).toBe('ja');
  });

  it('follows the browser language before the country', () => {
    expect(visitorLanguage(visitor({ acceptLanguage: 'en-US,en;q=0.9', country: 'TW' }))).toBe('en');
    expect(visitorLanguage(visitor({ acceptLanguage: 'ja-JP', country: 'US' }))).toBe('ja');
    expect(visitorLanguage(visitor({ acceptLanguage: 'de-DE,de;q=0.9', country: 'JP' }))).toBe('en');
  });

  it('takes the script of a bare zh from the country', () => {
    expect(visitorLanguage(visitor({ acceptLanguage: 'zh', country: 'HK' }))).toBe('zh-Hant');
    expect(visitorLanguage(visitor({ acceptLanguage: 'zh', country: 'CN' }))).toBe('zh-Hans');
    expect(visitorLanguage(visitor({ acceptLanguage: 'zh-CN', country: 'TW' }))).toBe('zh-Hans');
  });

  it('uses the country when the browser sends no language', () => {
    expect(visitorLanguage(visitor({ country: 'KR' }))).toBe('ko');
    expect(visitorLanguage(visitor({ country: 'TW' }))).toBe('zh-Hant');
    expect(visitorLanguage(visitor({ country: 'US' }))).toBe('en');
    expect(visitorLanguage(visitor({}))).toBe('en');
  });
});

describe('edge worker', () => {
  const page = '<!doctype html><title>GRKit</title>';
  const assets = { fetch: vi.fn(async () => new Response(page, { headers: { 'Content-Type': 'text/html' } })) };
  const request = (path: string, headers: Record<string, string>, country?: string) =>
    Object.assign(new Request(`https://grkit.app${path}`, { headers }), { cf: country ? { country } : undefined });

  afterEach(() => {
    vi.unstubAllGlobals();
    assets.fetch.mockClear();
  });

  it('sends a visitor of the root page to the page of their language', async () => {
    const response = await edge.fetch(request('/?ref=x', { 'Accept-Language': 'zh-TW' }), { ASSETS: assets });
    expect(response.status).toBe(302);
    expect(response.headers.get('Location')).toBe('https://grkit.app/zh-hant/?ref=x');
    expect(assets.fetch).not.toHaveBeenCalled();
  });

  it('serves the English page with the site headers', async () => {
    vi.stubGlobal('__SITE_HEADERS__', { 'Content-Security-Policy': "default-src 'self'" });
    const response = await edge.fetch(request('/', {}, 'US'), { ASSETS: assets });
    expect(response.status).toBe(200);
    expect(await response.text()).toBe(page);
    expect(response.headers.get('Content-Security-Policy')).toBe("default-src 'self'");
    expect(response.headers.get('Vary')).toBe('Accept-Language, Cookie');
  });

  it('leaves a language page as it is', async () => {
    vi.stubGlobal('__SITE_HEADERS__', {});
    const response = await edge.fetch(request('/ja/', { 'Accept-Language': 'ko' }), { ASSETS: assets });
    expect(response.status).toBe(200);
    expect(assets.fetch).toHaveBeenCalledOnce();
  });
});
