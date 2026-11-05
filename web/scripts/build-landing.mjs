// Builds the images of the landing page in public/landing/ from the source photos, which stay
// outside the repository:
//   node scripts/build-landing.mjs PHOTOS EFFECTS [GRAIN]
// PHOTOS holds the JPEGs of SOURCES under their `file` names. EFFECTS is the site effects folder
// of the preview target (catalog.json and preview/). The film looks use the preview tables, as
// the effects section does. Date Imprint and Glow run the module code on the full-size photo,
// built for this computer like `grkit module test` (it needs cargo and a C compiler). Grain comes
// from the camera's own grain stage, so GRAIN holds pictures that GR III firmware 2.10 recorded of
// the tram photo at ISO 100, with Film Look Tri-X 400 and each Grain strength (GRAIN_FILES).
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import jpeg from 'jpeg-js';
import { LANDING_LOOKS } from '../src/landing-data.ts';
import { applyLinearLut } from '../src/preview/lut.ts';
import { linearSample } from './linear-sample.mjs';

const ROOT = fileURLToPath(new URL('../../', import.meta.url));
const OUTPUT = fileURLToPath(new URL('../public/landing/', import.meta.url));
const QUALITY = 80;

/** The source photos. The page credits each one; src/landing-data.ts holds the credits. */
const SOURCES = {
  kitchen: {
    file: 'Sakiko_buys_a_delicious_soup_at_Kitchen_Grizzly._(53311001084).jpg',
    sha256: 'e8ae3af72569a1f89c327f6c90cb7bb846a5482592d72cf19e19f0c2ecfd81ae',
  },
  tram: {
    file: 'Tramway_(53731862585).jpg',
    sha256: 'dff8652af5e3210cc305401fa0b9e7eaf61137fafabcbdb4612d4fd75593743e',
  },
  street: {
    file: 'People_dining_outside,_Rue_des_Marronniers,_Lyon,_France_julesvernex2.jpg',
    sha256: '5da52ffc6910829caea7497cceaaf4dba90a9eabf588e8b25c52b736ccb722be',
  },
};
/** The recorded Grain pictures in GRAIN, Weak to Extra Strong, 6000 x 4000. */
const GRAIN_FILES = ['grain-1-weak.jpg', 'grain-2-medium.jpg', 'grain-3-strong.jpg', 'grain-4-extra.jpg'];

const [photos, effects, grain] = process.argv.slice(2);
if (!photos || !effects) throw new Error('Usage: node scripts/build-landing.mjs PHOTOS EFFECTS [GRAIN]');
const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');
function photo(key) {
  const { file, sha256: expected } = SOURCES[key];
  const bytes = readFileSync(join(photos, file));
  if (sha256(bytes) !== expected) throw new Error(`${file} is not the expected photo`);
  return bytes;
}
const encode = (width, height, rgba) => jpeg.encode({ data: rgba, width, height }, QUALITY).data;
mkdirSync(join(OUTPUT, 'looks'), { recursive: true });

// 1. The film looks: the Standard development and each look of the hero.
const catalog = JSON.parse(readFileSync(join(effects, 'catalog.json'), 'utf8'));
const { image, meanError } = linearSample(photo('kitchen'), readFileSync(join(effects, 'preview/standard.lut')));
const tables = [['Standard', 'standard', 'preview/standard.lut']];
// The looks of the hero (landing-data.ts), by their catalog names.
for (const { name, image } of LANDING_LOOKS) {
  const effect = catalog.effects.find((item) => item.name === name);
  if (!effect) throw new Error(`The catalog has no ${name}`);
  tables.push([name, image, effect.preview]);
}
for (const [, slug, path] of tables) {
  const table = readFileSync(join(effects, path));
  const size = Math.round(Math.cbrt(table.length / 3));
  writeFileSync(join(OUTPUT, 'looks', `${slug}.jpg`), encode(image.width, image.height, applyLinearLut(image.data, table, size)));
}
console.log(`Looks: ${image.width} x ${image.height}, baseline difference ${meanError.toFixed(2)} of 255.`);

// 2. Date Imprint and Glow, with the module code.
const work = mkdtempSync(join(tmpdir(), 'grkit-landing-'));
try {
  const decoded = (key) => jpeg.decode(photo(key), { useTArray: true, formatAsRGBA: false, maxMemoryUsageInMB: 1024 });
  /** RGB pixels of a part of a picture, kept from `left`, `top`, `width` x `height`. */
  function part(picture, left, top, width, height) {
    const data = new Uint8Array(width * height * 3);
    for (let y = 0; y < height; y++) {
      const from = ((top + y) * picture.width + left) * 3;
      data.set(picture.data.subarray(from, from + width * 3), y * width * 3);
    }
    return { width, height, data };
  }
  /** RGBA pixels of a part of an RGB picture, reduced by `factor` in each direction (area mean). */
  function crop(picture, left, top, width, height, factor) {
    const w = Math.floor(width / factor);
    const h = Math.floor(height / factor);
    const rgba = new Uint8Array(w * h * 4);
    for (let y = 0; y < h; y++)
      for (let x = 0; x < w; x++) {
        const sum = [0, 0, 0];
        for (let dy = 0; dy < factor; dy++)
          for (let dx = 0; dx < factor; dx++) {
            const at = ((top + y * factor + dy) * picture.width + left + x * factor + dx) * 3;
            for (let c = 0; c < 3; c++) sum[c] += picture.data[at + c];
          }
        const to = (y * w + x) * 4;
        for (let c = 0; c < 3; c++) rgba[to + c] = Math.round(sum[c] / (factor * factor));
        rgba[to + 3] = 255;
      }
    return { width: w, height: h, rgba };
  }
  /** The renderer of a module: its sources, its generated descriptor, the harness, and landing-render.c. */
  function renderer(module, defines) {
    const objects = join(work, module);
    execFileSync('cargo', ['run', '-q', '-p', 'grkit-cli', '--', 'module', 'test', '--dir', join(ROOT, 'modules', module), '--objects', objects], {
      cwd: ROOT,
      stdio: ['ignore', 'ignore', 'inherit'],
    });
    const manifest = readFileSync(join(ROOT, 'modules', module, 'module.toml'), 'utf8');
    const sources = [...(/^sources = \[(.*)\]/m.exec(manifest)?.[1] ?? '').matchAll(/"([^"]+)"/g)].map((m) => join(ROOT, 'modules', module, m[1]));
    const sdk = join(ROOT, 'runtime', 'sdk');
    const executable = join(work, `render-${module}`);
    execFileSync('cc', [
      '-std=c11', '-O2', '-Wall', '-Wno-unused-function', '-DGRKIT_TEST_PLANE_BYTES=(64U<<20)', ...defines,
      '-include', join(sdk, 'grkit_test.h'), '-I', sdk, '-I', objects,
      join(objects, 'grkit_module.c'), ...sources, fileURLToPath(new URL('landing-render.c', import.meta.url)), join(sdk, 'grkit_test.c'),
      '-o', executable,
    ], { stdio: ['ignore', 'ignore', 'inherit'] });
    return (picture, args, env = {}) => {
      const input = join(work, 'in.rgb');
      const output = join(work, 'out.rgb');
      writeFileSync(input, picture.data);
      execFileSync(executable, [input, String(picture.width), String(picture.height), output, ...args.map(String)], {
        env: { ...process.env, ...env },
        stdio: ['ignore', 'ignore', 'inherit'],
      });
      return { width: picture.width, height: picture.height, data: readFileSync(output) };
    };
  }
  const save = (name, { width, height, rgba }) => writeFileSync(join(OUTPUT, name), encode(width, height, rgba));

  // The date of the tram photo (EXIF 2024-05-06 16:30), 'YY M D in each digit look, then the
  // bottom right quarter of the photo at half size.
  const date = renderer('date', ['-DRENDER_DATE']);
  const tram = decoded('tram');
  ['classic', 'gr1s', 'gr2'].forEach((look, value) => {
    const out = date(tram, [1, value], { RENDER_TIME: '2024 5 6 16 30' });
    save(`date-${look}.jpg`, crop(out, 4400, 2800, 1600, 1200, 2));
  });

  // Glow on the street at night (EXIF 7.2 EV at ISO 100), cut to 3:2 as the camera records it:
  // Off, then Bloom and Halation at the Medium strength, the lamps at one third.
  const glow = renderer('glow', []);
  const full = decoded('street');
  const street = part(full, (full.width - 5916) >> 1, (full.height - 3944) >> 1, 5916, 3944);
  const lamps = (picture) => crop(picture, 1740, 720, 2160, 1620, 3);
  save('glow-off.jpg', lamps(glow(street, [0, 0, 7.2])));
  save('bloom.jpg', lamps(glow(street, [2, 0, 7.2])));
  save('halation.jpg', lamps(glow(street, [0, 2, 7.2])));
  console.log('Date Imprint and Glow: done.');

  // Grain: the light grey footbridge girder, at full size. A smaller size averages the grain
  // away, and the grain shows most on light mid tones.
  if (grain) {
    GRAIN_FILES.forEach((file, index) => {
      const picture = jpeg.decode(readFileSync(join(grain, file)), { useTArray: true, formatAsRGBA: false, maxMemoryUsageInMB: 1024 });
      if (picture.width !== 6000 || picture.height !== 4000) throw new Error(`${file} is not a 6000 x 4000 picture`);
      save(`grain-${index + 1}.jpg`, crop(picture, 1950, 450, 720, 540, 1));
    });
    console.log('Grain: done.');
  }
} finally {
  rmSync(work, { recursive: true, force: true });
}
