/// <reference types="vitest/config" />
import { createReadStream, existsSync, statSync } from 'node:fs';
import { resolve, sep } from 'node:path';
import react from '@vitejs/plugin-react';
import { defineConfig, type Plugin, type PreviewServer, type ViteDevServer } from 'vite';

/** The paths that CI writes into the site next to the web build. */
const DATA_PATH = /^\/(runtime|effects)\//;

/**
 * Serves runtime and effect files from a local site-data directory during `dev` and `preview`.
 * Release builds do not use it: CI copies the web build into the site tree instead.
 */
function siteData(directory: string): Plugin {
  const root = resolve(directory);
  const serve = (server: ViteDevServer | PreviewServer) => {
    if (!existsSync(root)) {
      server.config.logger.warn(`No site data at ${root}. Set GRKIT_SITE_DATA. See web/README.md.`);
      return;
    }
    server.middlewares.use((request, response, next) => {
      const path = decodeURIComponent((request.url ?? '').split('?')[0] ?? '');
      if (!DATA_PATH.test(path)) {
        return next();
      }
      const file = resolve(root, `.${path}`);
      if (!file.startsWith(root + sep) || !existsSync(file) || !statSync(file).isFile()) {
        response.statusCode = 404;
        return response.end();
      }
      response.setHeader(
        'Content-Type',
        file.endsWith('.json') ? 'application/json' : 'application/octet-stream',
      );
      createReadStream(file).pipe(response);
    });
  };
  return { name: 'grkit-site-data', configureServer: serve, configurePreviewServer: serve };
}

/**
 * Removes the WOFF fallback from the Fontsource styles. The site targets ES2023 browsers,
 * which all read WOFF2, so the fallback only doubles the style rules and the font files.
 */
function woff2Only(): Plugin {
  return {
    name: 'grkit-woff2-only',
    enforce: 'pre',
    transform(code, id) {
      if (!id.includes('/@fontsource/') || !id.endsWith('.css')) return;
      return code.replace(/,\s*url\([^)]*\.woff\) format\('woff'\)/g, '');
    },
  };
}

export default defineConfig({
  plugins: [react(), woff2Only(), siteData(process.env.GRKIT_SITE_DATA ?? '../dist/site-data')],
  build: {
    target: 'es2023',
    // The CSP allows fonts only from the site, so fonts stay files and never become data URLs.
    assetsInlineLimit: (file) => (/\.woff2?$/.test(file) ? false : undefined),
    // scripts/third-party-licenses.mjs merges this list into THIRD_PARTY_LICENSES.txt.
    license: { fileName: '.vite/licenses.json' },
  },
  test: { environment: 'node' },
});
