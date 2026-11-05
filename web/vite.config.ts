/// <reference types="vitest/config" />
import { createReadStream, existsSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { resolve, sep } from 'node:path';
import react from '@vitejs/plugin-react';
import { defineConfig, type Plugin, type PreviewServer, type ViteDevServer } from 'vite';
import { addPlausible, allowPlausible, plausibleOrigin } from './plausible.ts';

/**
 * The GRKit version from the workspace Cargo.toml. The Release workflow builds the site
 * and the runtimes from one commit, so the site shows the version of the runtime that it packages.
 */
function workspaceVersion(): string {
  const manifest = readFileSync(resolve(import.meta.dirname, '../Cargo.toml'), 'utf8');
  const version = /^\[workspace\.package\][^[]*?^version = "([^"]+)"/m.exec(manifest)?.[1];
  if (!version) throw new Error('Cargo.toml has no [workspace.package] version');
  return version;
}

/** The paths that CI writes into the site next to the web build. */
const DATA_PATH = /^\/(runtime|effects|guides|modules)\//;

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

/**
 * The fonts of the first screen. The page renders only after its script runs, so without a
 * preload the browser finds these fonts late, paints a system font first, and the text moves
 * when they arrive.
 */
const FIRST_SCREEN_FONTS =
  /^assets\/ibm-plex-(sans-latin-(400|500|600)|sans-condensed-latin-700|mono-latin-400)-normal-[\w-]+\.woff2$/;

/** Preloads the fonts of the first screen, so they download with the script and the styles. */
function preloadFonts(): Plugin {
  return {
    name: 'grkit-preload-fonts',
    apply: 'build',
    transformIndexHtml: {
      order: 'post',
      handler(_html, context) {
        return Object.keys(context.bundle ?? {})
          .filter((file) => FIRST_SCREEN_FONTS.test(file))
          .sort()
          .map((file) => ({
            tag: 'link',
            attrs: { rel: 'preload', href: `/${file}`, as: 'font', type: 'font/woff2', crossorigin: '' },
            injectTo: 'head' as const,
          }));
      },
    },
  };
}

/**
 * Adds Plausible analytics to the build when PLAUSIBLE_HOST gives the Plausible origin, and
 * extends the Content-Security-Policy of `_headers` for it. The Release workflow sets it.
 */
function plausible(host: string | undefined): Plugin {
  const origin = host ? plausibleOrigin(host) : undefined;
  let outDir = '';
  return {
    name: 'grkit-plausible',
    apply: 'build',
    configResolved(config) {
      // The Node build of scripts/prerender.mjs writes no page and no `_headers`.
      outDir = config.build.ssr ? '' : resolve(config.root, config.build.outDir);
    },
    transformIndexHtml(html) {
      return origin ? addPlausible(html, origin) : html;
    },
    closeBundle() {
      if (!origin || !outDir) return;
      const headers = resolve(outDir, '_headers');
      writeFileSync(headers, allowPlausible(readFileSync(headers, 'utf8'), origin));
    },
  };
}

export default defineConfig({
  plugins: [
    react(),
    woff2Only(),
    preloadFonts(),
    siteData(process.env.GRKIT_SITE_DATA ?? '../dist/site-data'),
    plausible(process.env.PLAUSIBLE_HOST || undefined),
  ],
  define: { __GRKIT_VERSION__: JSON.stringify(workspaceVersion()) },
  // The landing page shows the camera screens of the README from resources/images.
  server: { fs: { allow: ['.', '../resources/images'] } },
  build: {
    target: 'es2023',
    // The CSP allows fonts only from the site, so fonts stay files and never become data URLs.
    assetsInlineLimit: (file) => (/\.woff2?$/.test(file) ? false : undefined),
    // scripts/third-party-licenses.mjs merges this list into THIRD_PARTY_LICENSES.txt.
    license: { fileName: '.vite/licenses.json' },
  },
  test: { environment: 'node' },
});
