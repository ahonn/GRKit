// Writes dist/THIRD_PARTY_LICENSES.txt after `vite build`.
//
// The page bundles JavaScript packages and fonts, which `build.license` in
// vite.config.ts lists in dist/.vite/licenses.json. The WebAssembly module
// contains Rust crates, which Vite cannot see, so this script reads them from
// `cargo metadata`. MIT, Apache-2.0, and OFL-1.1 require their notices in each
// copy that the site distributes.
import { execFileSync } from 'node:child_process';
import { readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
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
  for (const name of Object.keys(manifest.dependencies ?? {})) {
    if (found.has(name)) continue;
    const directory = join(web, 'node_modules', name);
    const own = JSON.parse(readFileSync(join(directory, 'package.json'), 'utf8'));
    const texts = licenseTexts(directory);
    if (texts.length === 0) throw new Error(`${name} has no license file`);
    found.set(name, { name, version: own.version, identifier: own.license, text: texts.join('\n\n') });
  }
  return [...found.values()].sort((a, b) => a.name.localeCompare(b.name));
}

/** The crates that grkit-wasm links for the browser. Procedural macros run only in the compiler. */
function rustCrates() {
  const output = execFileSync(
    'cargo',
    ['metadata', '--format-version', '1', '--locked', '--filter-platform', 'wasm32-unknown-unknown'],
    { cwd: join(web, '..'), maxBuffer: 64 * 1024 * 1024 },
  );
  const metadata = JSON.parse(output.toString());
  const packages = new Map(metadata.packages.map((entry) => [entry.id, entry]));
  const nodes = new Map(metadata.resolve.nodes.map((node) => [node.id, node]));
  const workspace = new Set(metadata.workspace_members);
  const root = metadata.packages.find((entry) => entry.name === 'grkit-wasm' && workspace.has(entry.id));
  const seen = new Set();
  const pending = [root.id];
  while (pending.length > 0) {
    const id = pending.pop();
    if (seen.has(id)) continue;
    seen.add(id);
    for (const dependency of nodes.get(id).deps) {
      const normal = dependency.dep_kinds.some((kind) => kind.kind === null);
      const macro = packages.get(dependency.pkg).targets.some((target) => target.kind.includes('proc-macro'));
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
      return { name: entry.name, version: entry.version, identifier: entry.license, text: texts.join('\n\n') };
    });
}

function section(title, entries) {
  const parts = entries.map(
    (entry) => `--- ${entry.name} ${entry.version} (${entry.identifier ?? 'see text'}) ---\n\n${entry.text.trim()}\n`,
  );
  return `=== ${title} ===\n\n${parts.join('\n')}`;
}

const javascript = javascriptPackages();
const rust = rustCrates();
const text = [
  'GRKit site: third-party licenses\n',
  'The GRKit site contains the third-party software and fonts below.',
  'The GRKit code is licensed under the MIT License. See LICENSE in https://github.com/ahonn/GRKit.\n',
  section('JavaScript packages and fonts', javascript),
  section('Rust crates in the WebAssembly module', rust),
].join('\n');
writeFileSync(join(dist, 'THIRD_PARTY_LICENSES.txt'), text);
// The site serves dist/ as it is. The intermediate list is not a site file.
rmSync(join(dist, '.vite'), { recursive: true, force: true });
console.log(`THIRD_PARTY_LICENSES.txt: ${javascript.length} JavaScript packages and fonts, ${rust.length} Rust crates`);
