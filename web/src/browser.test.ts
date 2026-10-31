import { describe, expect, it } from 'vitest';
import { isAppBrowser } from './browser';

const IPHONE = 'Mozilla/5.0 (iPhone; CPU iPhone OS 18_5 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko)';
const ANDROID = 'Mozilla/5.0 (Linux; Android 14; 23127PN0CC Build/UKQ1.230804.001';

describe('isAppBrowser', () => {
  it('finds the browsers of chat and social apps', () => {
    for (const agent of [
      `${IPHONE} Mobile/15E148 MicroMessenger/8.0.50(0x1800323c) NetType/WIFI Language/zh_CN`,
      `${ANDROID}; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/126.0 Mobile Safari/537.36 XWEB/1260117 MMWEBSDK/20240501 MicroMessenger/8.0.50.2701(0x28003233) WeChat/arm64 Weixin NetType/WIFI`,
      `${IPHONE} Mobile/15E148 QQ/9.0.75.610 V1_IPH_SQ_9.0.75_1_APP_A Pixel/1179 MiniAppEnable SimpleUISwitch/0`,
      `${IPHONE} Mobile/15E148 xhsdiscover/8.40 (iPhone16,1;iOS 18.5;zh_CN)`,
      `${ANDROID}; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/120 Mobile Safari/537.36 aweme_310000 JsSdk/1.0`,
      `${IPHONE} Mobile/15E148 Weibo (iPhone15,2__weibo__14.6.0__iphone__os18.5)`,
      `${IPHONE} Mobile/15E148 [FBAN/FBIOS;FBAV/470.0.0.40.108]`,
      `${IPHONE} Mobile/15E148 Instagram 345.0.0.0 (iPhone15,2; iOS 18_5; en_US)`,
    ]) {
      expect(isAppBrowser(agent), agent).toBe(true);
    }
  });

  it('finds an Android WebView and an iPhone app browser without a name', () => {
    expect(isAppBrowser(`${ANDROID}; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/126.0 Mobile Safari/537.36`)).toBe(true);
    expect(isAppBrowser(`${IPHONE} Mobile/15E148`)).toBe(true);
  });

  it('leaves out the browsers', () => {
    for (const agent of [
      `${IPHONE} Version/18.5 Mobile/15E148 Safari/604.1`,
      `${IPHONE} CriOS/138.0 Mobile/15E148 Safari/604.1`,
      `${ANDROID}) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0 Mobile Safari/537.36`,
      `${ANDROID}) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/109.0 MQQBrowser/15.4 Mobile Safari/537.36`,
      `${ANDROID}) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/109.0 Quark/7.4.0 Mobile Safari/537.36`,
      'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0 Safari/537.36 Edg/138.0',
      'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.5 Safari/605.1.15',
    ]) {
      expect(isAppBrowser(agent), agent).toBe(false);
    }
  });
});
