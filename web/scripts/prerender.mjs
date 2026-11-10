// Writes the language pages, the sitemap, the 404 page, and the edge worker after `vite build`.
//
// Search engines and link previews read the HTML that the server sends. The page renders in the
// browser, so this script renders the first screen of each page of each language into its own
// file: the home page (dist/index.html for English, dist/<language>/index.html), the installer
// (install/index.html under each), and the fast-boot firmware page (fastboot/index.html under
// each). Each page names its other languages with hreflang links. The edge worker sends a visitor
// of the root page to the home page of their language.
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { build } from 'vite';

const SITE = 'https://grkit.app';
const web = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const dist = join(web, 'dist');
// Inside web/, so the Node build resolves react and the other packages from web/node_modules.
const server = join(web, 'node_modules', '.cache', 'grkit-prerender');

/** The `content` or text of an HTML attribute or element. */
const escape = (text) =>
  text.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;').replaceAll('"', '&quot;');

/** Replaces the one match of `pattern` in `html`, and fails when the template has none. */
function replaceOne(html, pattern, replacement) {
  if (!pattern.test(html)) throw new Error(`dist/index.html has no match for ${pattern}`);
  return html.replace(pattern, replacement);
}

/** The headers of the `/*` rule in `_headers`, which the worker adds to the root page. */
function siteHeaders(file) {
  const headers = {};
  let all = false;
  for (const line of readFileSync(file, 'utf8').split('\n')) {
    if (line.trim() === '' || line.trimStart().startsWith('#')) continue;
    if (!/^\s/.test(line)) {
      all = line.trim() === '/*';
      continue;
    }
    if (!all) continue;
    const colon = line.indexOf(':');
    headers[line.slice(0, colon).trim()] = line.slice(colon + 1).trim();
  }
  if (!headers['Content-Security-Policy']) throw new Error('_headers has no Content-Security-Policy for /*');
  return headers;
}

async function renderer() {
  await build({
    root: web,
    logLevel: 'warn',
    build: { ssr: 'src/prerender.tsx', outDir: server, emptyOutDir: true, copyPublicDir: false, license: false },
  });
  // The page renders as for a browser that it knows nothing about. Node has its own navigator
  // and storage, which would show the hints for an unknown browser. Without a browser, the
  // catalog requests never answer, so the effect lists render their fallback.
  for (const name of ['navigator', 'localStorage', 'sessionStorage']) {
    Object.defineProperty(globalThis, name, { value: undefined, configurable: true });
  }
  globalThis.fetch = () => new Promise(() => {});
  return import(pathToFileURL(join(server, 'prerender.js')).href);
}

async function edgeWorker(headers) {
  await build({
    root: web,
    configFile: false,
    logLevel: 'warn',
    define: { __SITE_HEADERS__: JSON.stringify(headers) },
    build: {
      outDir: dist,
      emptyOutDir: false,
      copyPublicDir: false,
      minify: false,
      target: 'es2022',
      lib: { entry: 'src/edge.ts', formats: ['es'], fileName: () => '_worker.js' },
    },
  });
  // Only the root page runs the worker. Pages serves the other files directly.
  writeFileSync(join(dist, '_routes.json'), `${JSON.stringify({ version: 1, include: ['/'], exclude: [] }, null, 2)}\n`);
}

const template = readFileSync(join(dist, 'index.html'), 'utf8');
const { render, LANGUAGES, LANGUAGE_PATHS, STRINGS, VERSION, pageText } = await renderer();
const codes = LANGUAGES.map((language) => language.code);
const pageUrl = (code, page = 'home') => `${SITE}${pageText(code, page).path}`;

/** The pages of each language. Search engines index the home page and the installer. */
const PAGES = [
  { page: 'home', index: true },
  { page: 'install', index: true },
  { page: 'fastboot', index: false },
];

function structuredData(code) {
  const strings = STRINGS[code];
  const data = {
    '@context': 'https://schema.org',
    '@graph': [
      { '@type': 'WebSite', '@id': `${SITE}/#website`, name: 'GRKit', url: `${SITE}/`, inLanguage: codes },
      {
        '@type': 'SoftwareApplication',
        name: 'GRKit',
        url: pageUrl(code),
        description: strings.description,
        inLanguage: code,
        applicationCategory: 'MultimediaApplication',
        operatingSystem: 'RICOH GR III, GR IIIx, GR IV',
        softwareVersion: VERSION,
        image: `${SITE}/og.jpg`,
        isAccessibleForFree: true,
        offers: { '@type': 'Offer', price: '0', priceCurrency: 'USD' },
        sameAs: ['https://github.com/ahonn/grkit'],
      },
    ],
  };
  // A JSON-LD block ends at `</script>`, so `<` never appears as itself.
  return JSON.stringify(data).replaceAll('<', '\\u003c');
}

/** The HTML of `page` in `code`, with its title, description, address, and other languages. */
function htmlPage(code, { page, index }) {
  const text = pageText(code, page);
  const title = escape(text.title);
  const description = escape(text.description);
  let html = template;
  html = replaceOne(html, /<html lang="[^"]*">/, `<html lang="${code}">`);
  html = replaceOne(html, /<title>[^<]*<\/title>/, `<title>${title}</title>`);
  html = replaceOne(html, /(<meta\s+name="description"\s+content=")[^"]*(")/, `$1${description}$2`);
  html = replaceOne(html, /(<meta property="og:url" content=")[^"]*(")/, `$1${pageUrl(code, page)}$2`);
  html = replaceOne(html, /(<meta property="og:title" content=")[^"]*(")/, `$1${title}$2`);
  html = replaceOne(html, /(<meta property="og:description" content=")[^"]*(")/, `$1${description}$2`);
  const head = [
    ...(index ? [] : ['<meta name="robots" content="noindex" />']),
    `<link rel="canonical" href="${pageUrl(code, page)}" />`,
    ...codes.map((other) => `<link rel="alternate" hreflang="${other}" href="${pageUrl(other, page)}" />`),
    ...(index ? [`<link rel="alternate" hreflang="x-default" href="${pageUrl('en', page)}" />`] : []),
    ...(page === 'home' ? [`<script type="application/ld+json">${structuredData(code)}</script>`] : []),
  ].join('\n    ');
  html = replaceOne(html, /<\/head>/, `  ${head}\n  </head>`);
  return replaceOne(html, /<div id="root"><\/div>/, `<div id="root">${render(code, page)}</div>`);
}

/** The value of an HTML attribute as the browser reads it. */
const unescape = (text) =>
  text.replaceAll('&quot;', '"').replaceAll('&#x27;', "'").replaceAll('&lt;', '<').replaceAll('&gt;', '>').replaceAll('&amp;', '&');

/**
 * Allows the style attributes of the rendered pages in the Content-Security-Policy of `_headers`.
 * The page itself sets styles through the DOM, which the policy allows, but a style attribute
 * in the HTML needs its hash. Without them, the compare slider and the frames lose their layout
 * until the script runs.
 */
function allowStyleAttributes(file, pages) {
  const styles = new Set();
  for (const html of pages) {
    for (const match of html.matchAll(/\sstyle="([^"]*)"/g)) styles.add(unescape(match[1]));
  }
  if (styles.size === 0) return;
  const hashes = [...styles]
    .sort()
    .map((style) => `'sha256-${createHash('sha256').update(style).digest('base64')}'`)
    .join(' ');
  const headers = readFileSync(file, 'utf8');
  const policy = /(Content-Security-Policy:[^\n]*style-src 'self')/;
  if (!policy.test(headers)) throw new Error("_headers has no style-src 'self' in its Content-Security-Policy");
  writeFileSync(file, headers.replace(policy, `$1 'unsafe-hashes' ${hashes}`));
}

const pages = [];
for (const kind of PAGES) {
  for (const code of codes) {
    const file = join(dist, `.${pageText(code, kind.page).path}`, 'index.html');
    mkdirSync(dirname(file), { recursive: true });
    const html = htmlPage(code, kind);
    writeFileSync(file, html);
    pages.push(html);
  }
}
allowStyleAttributes(join(dist, '_headers'), pages);

// Pages serves 404.html with status 404 for a path without a file. Without it, Pages serves the
// root page for every path, and search engines see each wrong address as a copy of the site.
let notFound = template;
notFound = replaceOne(notFound, /<title>[^<]*<\/title>/, '<title>Page not found · GRKit</title>');
notFound = replaceOne(notFound, /<meta\s+name="description"[^>]*>/, '<meta name="robots" content="noindex" />');
notFound = notFound.replace(/\s*<script\b[^>]*\bsrc="[^"]*"[^>]*><\/script>/g, '');
notFound = notFound.replace(/\s*<link rel="modulepreload"[^>]*>/g, '');
notFound = replaceOne(
  notFound,
  /<div id="root"><\/div>/,
  `<main class="not-found"><h1>GRKit</h1><p>Page not found.</p><p>${codes
    .map((code) => `<a href="${LANGUAGE_PATHS[code]}" lang="${code}">${escape(LANGUAGES.find((item) => item.code === code).label)}</a>`)
    .join(' · ')}</p></main>`,
);
writeFileSync(join(dist, '404.html'), notFound);

const urls = PAGES.filter((kind) => kind.index)
  .flatMap(({ page }) =>
    codes.map(
      (code) =>
        `  <url>\n    <loc>${pageUrl(code, page)}</loc>\n${codes
          .map((other) => `    <xhtml:link rel="alternate" hreflang="${other}" href="${pageUrl(other, page)}"/>`)
          .join('\n')}\n    <xhtml:link rel="alternate" hreflang="x-default" href="${pageUrl('en', page)}"/>\n  </url>`,
    ),
  )
  .join('\n');
writeFileSync(
  join(dist, 'sitemap.xml'),
  `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml">\n${urls}\n</urlset>\n`,
);

await edgeWorker(siteHeaders(join(dist, '_headers')));
rmSync(server, { recursive: true, force: true });
