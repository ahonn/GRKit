import { describe, expect, it } from 'vitest';
import type { ImportedEffect, RestoredImport } from '../catalog';
import { nativeTone, readDetails, withDetails } from './library';

const cover = { template: 'box', brand: 'Kodak', name: 'VISION', number: '5247', band: false } as const;

const restored = (): ImportedEffect<RestoredImport> => ({
  id: 'lut-vision',
  name: 'Vision',
  icon: '',
  cover: undefined,
  monochrome: false,
  preview: 'restored/lut-vision/1',
  imported: { kind: 'restored', blob: new Uint8Array([1]), table: new Uint8Array([2]), source: 'GRKit.zip' },
});

describe('import library details', () => {
  it('keeps only the values that the page can use', () => {
    expect(readDetails('not json')).toEqual({});
    expect(readDetails('[]')).toEqual({});
    expect(
      readDetails(
        JSON.stringify({
          cover,
          grade: 'B',
          input: 'flog2',
          warnings: ['Check the shadows.'],
          source: 'Vision.cube',
          extra: 1,
        }),
      ),
    ).toEqual({ cover, grade: 'B', input: 'flog2', warnings: ['Check the shadows.'], source: 'Vision.cube' });
    expect(
      readDetails(
        JSON.stringify({ cover: { ...cover, brand: 'x'.repeat(13) }, grade: 'E', input: 'slog3', warnings: [1] }),
      ),
    ).toEqual({});
  });

  it('makes a LUT import again from full details, and keeps a cover from partial ones', () => {
    const full = withDetails(restored(), {
      cover,
      grade: 'A',
      input: 'standard',
      warnings: [],
      source: 'Vision.cube',
    });
    expect(full.cover).toEqual(cover);
    expect(full.imported).toEqual({
      kind: 'lut',
      blob: new Uint8Array([1]),
      table: new Uint8Array([2]),
      grade: 'A',
      input: 'standard',
      warnings: [],
      source: 'Vision.cube',
    });
    const partial = withDetails(restored(), { cover });
    expect(partial.cover).toEqual(cover);
    expect(partial.imported.kind).toBe('restored');
    expect(withDetails(restored(), undefined)).toEqual(restored());
  });

  it('makes a native look again from its details with tone curves', () => {
    const tone = Uint16Array.from({ length: 512 }, (_, i) => i * 100);
    // The JSON details keep the curves as base64 of their little-endian bytes.
    const bytes = new Uint8Array(tone.buffer);
    const text = btoa(String.fromCharCode(...bytes));
    const cine = { template: 'cine', brand: 'RICOH', name: 'CINEMA', number: 'YELLOW' } as const;
    const details = readDetails(
      JSON.stringify({ kind: 'native', cover: cine, version: '1.20', source: 'gr4_v120.zip', tone: text }),
    );
    expect(details).toEqual({ kind: 'native', cover: cine, version: '1.20', source: 'gr4_v120.zip', tone });
    expect(nativeTone(details)).toEqual(tone);
    const look = withDetails({ ...restored(), id: 'gr4-cinema-yellow' }, details);
    expect(look.cover).toEqual(cine);
    expect(look.imported).toEqual({
      kind: 'native',
      blob: new Uint8Array([1]),
      table: new Uint8Array([2]),
      version: '1.20',
      source: 'gr4_v120.zip',
      tone,
    });
    // Without its version or with curves of another size, a native look does not come back.
    expect(nativeTone({ ...details, version: undefined })).toBeUndefined();
    expect(readDetails(JSON.stringify({ kind: 'native', tone: btoa('abc') })).tone).toBeUndefined();
  });
});
