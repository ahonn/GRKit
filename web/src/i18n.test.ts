import { describe, expect, it } from 'vitest';
import { matchLanguage, STRINGS } from './i18n';

describe('matchLanguage', () => {
  it('maps Chinese tags by script and region', () => {
    expect(matchLanguage(['zh-CN'])).toBe('zh-Hans');
    expect(matchLanguage(['zh'])).toBe('zh-Hans');
    expect(matchLanguage(['zh-Hant-TW'])).toBe('zh-Hant');
    expect(matchLanguage(['zh-HK'])).toBe('zh-Hant');
    expect(matchLanguage(['zh-TW'])).toBe('zh-Hant');
  });

  it('uses the first supported tag and falls back to English', () => {
    expect(matchLanguage(['ja-JP', 'zh-TW', 'en'])).toBe('zh-Hant');
    expect(matchLanguage(['fr', 'en-GB'])).toBe('en');
    expect(matchLanguage(['ja'])).toBe('en');
    expect(matchLanguage([])).toBe('en');
  });
});

describe('STRINGS', () => {
  it('gives each language the same install steps and effect looks', () => {
    const shape = (language: keyof typeof STRINGS) => ({
      steps: STRINGS[language].steps.map((parts) => parts.filter((p) => p.startsWith('`'))),
      looks: Object.keys(STRINGS[language].looks).sort(),
    });
    expect(shape('zh-Hans')).toEqual(shape('en'));
    expect(shape('zh-Hant')).toEqual(shape('en'));
  });
});
