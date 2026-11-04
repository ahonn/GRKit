// Builds the default sample from a camera JPEG, with its scene-linear pixels estimated by
// linear-sample.mjs. The full JPEG stays outside the repository.
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { gzipSync } from 'node:zlib';
import { encodeLinear } from '../src/preview/linear.ts';
import { linearSample } from './linear-sample.mjs';

const SOURCE = {
  url: 'https://upload.wikimedia.org/wikipedia/commons/7/73/Sakiko_buys_a_delicious_soup_at_Kitchen_Grizzly._%2853311001084%29.jpg',
  page: 'https://www.flickr.com/photos/mujitra/53311001084/',
  sha256: 'e8ae3af72569a1f89c327f6c90cb7bb846a5482592d72cf19e19f0c2ecfd81ae',
};
const [input, tablePath] = process.argv.slice(2);
if (!input || !tablePath) {
  throw new Error(
    'Usage: node scripts/build-sample.mjs /path/to/Sakiko_buys_a_delicious_soup_at_Kitchen_Grizzly._(53311001084).jpg /path/to/effects/gr3-2.10/preview/standard.lut',
  );
}
const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');
const source = await readFile(input);
if (sha256(source) !== SOURCE.sha256) throw new Error('Expected the Kitchen Grizzly JPEG');
const table = await readFile(tablePath);
const { image, meanError } = linearSample(source, table);

const bytes = gzipSync(encodeLinear(image), { level: 9 });
const output = new URL('../public/samples/', import.meta.url);
await mkdir(output, { recursive: true });
await writeFile(new URL('kitchen-grizzly.grlinear', output), bytes);
await writeFile(
  new URL('kitchen-grizzly.json', output),
  JSON.stringify(
    {
      source: SOURCE.url,
      sourcePage: SOURCE.page,
      sourceSha256: SOURCE.sha256,
      title: 'Sakiko buys a delicious soup at Kitchen Grizzly',
      author: 'MIKI Yoshihito',
      license: 'CC-BY-2.0',
      licenseUrl: 'https://creativecommons.org/licenses/by/2.0/',
      camera: 'RICOH GR IIIx',
      decoder: 'jpeg-js 0.4.4',
      settings: {
        input: 'sRGB JPEG',
        outputColor: 'linear sRGB, estimated by inverting preview/standard.lut',
        standardTableSha256: sha256(table),
        bits: 16,
        resize: 'area resampling in linear light',
      },
      width: image.width,
      height: image.height,
      sha256: sha256(bytes),
    },
    null,
    2,
  ) + '\n',
);
console.log(
  `Sample: ${image.width} x ${image.height}, ${bytes.length} bytes. ` +
    `Mean difference between the baseline and the JPEG: ${meanError.toFixed(2)} of 255.`,
);
