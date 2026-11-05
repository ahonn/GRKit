// Writes dist/THIRD_PARTY_LICENSES.txt after `vite build`.
//
// The page bundles JavaScript packages and fonts, which `build.license` in
// vite.config.ts lists in dist/.vite/licenses.json. The WebAssembly module
// contains Rust crates, which Vite cannot see, so this script reads them from
// `cargo metadata`. MIT, Apache-2.0, and OFL-1.1 require their notices in each
// copy that the site distributes.
import { execFileSync } from 'node:child_process';
import { createRequire } from 'node:module';
import { readdirSync, readFileSync, realpathSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const web = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const dist = join(web, 'dist');
const viteLicenses = join(dist, '.vite', 'licenses.json');
const LICENSE_FILE = /^(licen[cs]e|copying|notice)/i;

function licenseTexts(directory) {
  return readdirSync(directory)
    .filter((name) => LICENSE_FILE.test(name))
    .sort()
    .map((name) => readFileSync(join(directory, name), 'utf8').trim());
}

/** The packages that the page bundles. A dependency without bundled code is added from its directory. */
function javascriptPackages() {
  const listed = JSON.parse(readFileSync(viteLicenses, 'utf8'));
  const found = new Map(listed.map((entry) => [entry.name, entry]));
  const manifest = JSON.parse(readFileSync(join(web, 'package.json'), 'utf8'));
  const directories = Object.keys(manifest.dependencies ?? {}).map((name) =>
    join(web, 'node_modules', name),
  );
  // Vite's main bundle license report omits dependencies bundled only in a worker.
  const rawRequire = createRequire(
    realpathSync(join(web, 'node_modules/@colorhythm/libraw-wasm/package.json')),
  );
  directories.push(dirname(dirname(rawRequire.resolve('typed-cstruct'))));
  for (const directory of directories) {
    const own = JSON.parse(readFileSync(join(directory, 'package.json'), 'utf8'));
    const name = own.name;
    if (found.has(name)) continue;
    const texts = licenseTexts(directory);
    if (texts.length === 0) throw new Error(`${name} has no license file`);
    found.set(name, {
      name,
      version: own.version,
      identifier: own.license,
      text: texts.join('\n\n'),
    });
  }
  return [...found.values()].sort((a, b) => a.name.localeCompare(b.name));
}

/** The crates that grkit-wasm links for the browser. Procedural macros run only in the compiler. */
function rustCrates() {
  const output = execFileSync(
    'cargo',
    [
      'metadata',
      '--format-version',
      '1',
      '--locked',
      '--filter-platform',
      'wasm32-unknown-unknown',
    ],
    { cwd: join(web, '..'), maxBuffer: 64 * 1024 * 1024 },
  );
  const metadata = JSON.parse(output.toString());
  const packages = new Map(metadata.packages.map((entry) => [entry.id, entry]));
  const nodes = new Map(metadata.resolve.nodes.map((node) => [node.id, node]));
  const workspace = new Set(metadata.workspace_members);
  const root = metadata.packages.find(
    (entry) => entry.name === 'grkit-wasm' && workspace.has(entry.id),
  );
  const seen = new Set();
  const pending = [root.id];
  while (pending.length > 0) {
    const id = pending.pop();
    if (seen.has(id)) continue;
    seen.add(id);
    for (const dependency of nodes.get(id).deps) {
      const normal = dependency.dep_kinds.some((kind) => kind.kind === null);
      const macro = packages
        .get(dependency.pkg)
        .targets.some((target) => target.kind.includes('proc-macro'));
      if (normal && !macro) pending.push(dependency.pkg);
    }
  }
  return [...seen]
    .filter((id) => !workspace.has(id))
    .map((id) => packages.get(id))
    .sort((a, b) => a.name.localeCompare(b.name) || a.version.localeCompare(b.version))
    .map((entry) => {
      const texts = licenseTexts(dirname(entry.manifest_path));
      if (texts.length === 0) throw new Error(`${entry.name} ${entry.version} has no license file`);
      return {
        name: entry.name,
        version: entry.version,
        identifier: entry.license,
        text: texts.join('\n\n'),
      };
    });
}

function section(title, entries) {
  const parts = entries.map(
    (entry) =>
      `--- ${entry.name} ${entry.version} (${entry.identifier ?? 'see text'}) ---\n\n${entry.text.trim()}\n`,
  );
  return `=== ${title} ===\n\n${parts.join('\n')}`;
}

const javascript = javascriptPackages();
const rust = rustCrates();
const text = [
  'GRKit site: third-party licenses\n',
  'The GRKit site contains the third-party software and fonts below.',
  'The GRKit code is licensed under the MIT License. See LICENSE in https://github.com/ahonn/grkit.\n',
  section('JavaScript packages and fonts', javascript),
  section('Rust crates in the WebAssembly module', rust),
  '=== LibRaw in the RAW decoder WebAssembly module ===\n',
  'LibRaw 0.22.1 is distributed under CDDL-1.0. The wrapper is MIT licensed.',
  'Source and build instructions: https://github.com/colorhythm/libraw-wasm/tree/9b0f59a5e0f6d3d12a8f6fa594015ef4f30a6e95',
  ...['COPYRIGHT', 'LICENSE.CDDL'].map((file) =>
    readFileSync(join(web, 'node_modules/@colorhythm/libraw-wasm/LibRaw', file), 'utf8'),
  ),
  '=== Default sample photo ===\n',
  'Sakiko buys a delicious soup at Kitchen Grizzly by MIKI Yoshihito, taken with a RICOH GR IIIx. CC BY 2.0.',
  'Source: https://www.flickr.com/photos/mujitra/53311001084/',
  'License: https://creativecommons.org/licenses/by/2.0/',
  'The JPEG was converted to estimated linear sRGB, resized, and gzip compressed.',
  'The preview shows it with the film effects of GRKit.',
  'See samples/kitchen-grizzly.json for the source hash, processing settings, and output hash.\n',
  '=== Landing page photos ===\n',
  'The landing page shows these photos, resized and cropped, with the film effects and the modules of GRKit.',
  'scripts/build-landing.mjs makes the images from them.\n',
  'Sakiko buys a delicious soup at Kitchen Grizzly by MIKI Yoshihito, taken with a RICOH GR IIIx. CC BY 2.0.',
  'Source: https://www.flickr.com/photos/mujitra/53311001084/',
  'License: https://creativecommons.org/licenses/by/2.0/\n',
  'Tramway by MIKI Yoshihito, taken with a RICOH GR IIIx. CC BY 2.0.',
  'Source: https://www.flickr.com/photos/mujitra/53731862585/',
  'License: https://creativecommons.org/licenses/by/2.0/\n',
  'People dining outside, Rue des Marronniers, Lyon, France by Jules Verne Times Two, taken with a RICOH GR III. CC BY-SA 4.0.',
  'Source: https://commons.wikimedia.org/wiki/File:People_dining_outside,_Rue_des_Marronniers,_Lyon,_France_julesvernex2.jpg',
  'License: https://creativecommons.org/licenses/by-sa/4.0/',
  'The Glow images made from this photo (landing/glow-off.jpg, landing/bloom.jpg, landing/halation.jpg) are licensed under CC BY-SA 4.0.',
].join('\n');
writeFileSync(join(dist, 'THIRD_PARTY_LICENSES.txt'), text);
// The site serves dist/ as it is. The intermediate list is not a site file.
rmSync(join(dist, '.vite'), { recursive: true, force: true });
console.log(
  `THIRD_PARTY_LICENSES.txt: ${javascript.length} JavaScript packages and fonts, ${rust.length} Rust crates`,
);
