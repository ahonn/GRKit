// The Cloudflare Pages worker (`_worker.js`). `_routes.json` sends it only the root page, so it
// costs nothing for the other requests. scripts/prerender.mjs bundles it into the build.
import { LANGUAGE_PATHS, visitorLanguage } from './languagePage';

/** The headers of `_headers` for every path. Pages does not add them to worker responses. */
declare const __SITE_HEADERS__: Readonly<Record<string, string>>;

interface Env {
  ASSETS: { fetch(request: Request): Promise<Response> };
}

/** The request properties that Cloudflare adds. */
type EdgeRequest = Request & { cf?: { country?: string } };

export default {
  async fetch(request: EdgeRequest, env: Env): Promise<Response> {
    const url = new URL(request.url);
    if (url.pathname === '/' && (request.method === 'GET' || request.method === 'HEAD')) {
      const language = visitorLanguage({
        cookie: request.headers.get('Cookie'),
        acceptLanguage: request.headers.get('Accept-Language'),
        country: request.cf?.country ?? null,
      });
      if (language !== 'en') {
        url.pathname = LANGUAGE_PATHS[language];
        return new Response(null, {
          status: 302,
          headers: { Location: url.href, 'Cache-Control': 'private, no-store', Vary: 'Accept-Language, Cookie' },
        });
      }
    }
    const asset = await env.ASSETS.fetch(request);
    const response = new Response(asset.body, asset);
    for (const [name, value] of Object.entries(__SITE_HEADERS__)) {
      if (!response.headers.has(name)) response.headers.set(name, value);
    }
    response.headers.append('Vary', 'Accept-Language, Cookie');
    return response;
  },
};
