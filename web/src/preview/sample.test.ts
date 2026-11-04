import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { gunzipSync } from 'node:zlib';
import { expect, it } from 'vitest';
import { decodeLinear } from './linear';

it('ships the documented sample as valid linear pixels with its source attribution', () => {
  const bytes = readFileSync(new URL('../../public/samples/kitchen-grizzly.grlinear', import.meta.url));
  const manifest = JSON.parse(
    readFileSync(new URL('../../public/samples/kitchen-grizzly.json', import.meta.url), 'utf8'),
  );
  expect(createHash('sha256').update(bytes).digest('hex')).toBe(manifest.sha256);
  expect(manifest.license).toBe('CC-BY-2.0');
  expect(manifest.author).toBe('MIKI Yoshihito');
  const unpacked = gunzipSync(bytes);
  const image = decodeLinear(
    unpacked.buffer.slice(unpacked.byteOffset, unpacked.byteOffset + unpacked.byteLength),
  );
  expect([image.width, image.height]).toEqual([manifest.width, manifest.height]);
  expect(image.data.some((value) => value > 0)).toBe(true);
});
