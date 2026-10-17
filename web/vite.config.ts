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

export default defineConfig({
  plugins: [react(), siteData(process.env.GRKIT_SITE_DATA ?? '../dist/site-data')],
  build: { target: 'es2023' },
  test: { environment: 'node' },
});
