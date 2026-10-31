/**
 * App browsers by user agent: WeChat and WeCom, QQ, Weibo, Xiaohongshu, Douyin and other
 * ByteDance apps, DingTalk, Feishu and Lark, Alipay, and Facebook, Instagram, Threads, LINE.
 * MQQBrowser alone is QQ Browser, a full browser; the QQ app adds `QQ/`.
 */
const APP_BROWSER = new RegExp(
  [
    'MicroMessenger',
    'wxwork',
    '\\sQQ/',
    'Weibo',
    'xhsdiscover',
    'aweme',
    'BytedanceWebview',
    'DingTalk',
    'Lark/',
    'Feishu',
    'AlipayClient',
    'FBAN',
    'FBAV',
    'Instagram',
    'Barcelona',
    '\\sLine/',
  ].join('|'),
  'i',
);

/**
 * Whether the page runs in the browser of another app, such as WeChat. Such a browser often
 * saves no file that the page makes, so GRKit.zip does not arrive. Besides the apps by name, an
 * Android WebView marks itself with `wv`, and an iPhone or iPad app browser leaves out `Safari/`,
 * which Safari and the other iOS browsers give.
 */
export function isAppBrowser(userAgent: string): boolean {
  if (APP_BROWSER.test(userAgent)) return true;
  if (/Android/.test(userAgent) && /;\s?wv\)/.test(userAgent)) return true;
  return /iPhone|iPad|iPod/.test(userAgent) && !/Safari\//.test(userAgent);
}

/** Whether the page shows the hint to open it in a browser instead of an app. */
export const suggestOwnBrowser = () => typeof navigator !== 'undefined' && isAppBrowser(navigator.userAgent);
