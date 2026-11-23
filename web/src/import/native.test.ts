import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import type { ImportedEffect } from '../catalog';
import { NATIVE_LOOK_IDS } from './names';
import { hasNativeLooks, importsForCamera, missingNativeLooks, NATIVE_LOOKS, nativeLooksFor, nativeProblem } from './native';

const effect = (id: string, kind: 'lut' | 'native') => ({ id, imported: { kind } }) as ImportedEffect;

describe('importsForCamera', () => {
  const imported = [
    effect('lut-a', 'lut'),
    effect('gr4-cinema-yellow', 'native'),
    effect('gr4-cinema-green', 'native'),
    effect('gr2-positive-film', 'native'),
    effect('gr2-positive-film-gr4', 'native'),
  ];
  const ids = (items: readonly { id: string }[]) => items.map((item) => item.id);

  it('keeps the GR III native looks on GR III and GR IIIx', () => {
    for (const camera of ['gr3', 'gr3x']) {
      expect(ids(importsForCamera(imported, camera))).toEqual(ids(imported.slice(0, 4)));
      expect(ids(nativeLooksFor(camera))).toEqual(['gr4-cinema-yellow', 'gr4-cinema-green', 'gr2-positive-film']);
    }
  });

  it('keeps only the GR IV Positive on GR IV, and no native look on GR IV Monochrome', () => {
    expect(ids(nativeLooksFor('gr4'))).toEqual(['gr2-positive-film-gr4']);
    expect(ids(importsForCamera(imported, 'gr4'))).toEqual(['lut-a', 'gr2-positive-film-gr4']);
    expect(nativeLooksFor('gr4m')).toEqual([]);
    expect(ids(importsForCamera(imported, 'gr4m'))).toEqual(['lut-a']);
  });

  it('tells which native looks the page lacks', () => {
    expect(NATIVE_LOOKS.map((look) => [look.id, look.firmware])).toEqual([
      ['gr4-cinema-yellow', 'gr4'],
      ['gr4-cinema-green', 'gr4'],
      ['gr2-positive-film', 'gr2'],
      ['gr2-positive-film-gr4', 'gr2'],
    ]);
    expect(NATIVE_LOOKS.map((look) => look.id)).toEqual(NATIVE_LOOK_IDS);
    expect(missingNativeLooks(imported.slice(0, 1)).map((look) => look.id)).toEqual(
      NATIVE_LOOKS.map((look) => look.id),
    );
    expect(hasNativeLooks(imported.slice(0, 4))).toBe(false);
    expect(hasNativeLooks(imported)).toBe(true);
    expect(ids(missingNativeLooks(imported.slice(2, 3), nativeLooksFor('gr3')))).toEqual([
      'gr4-cinema-yellow',
      'gr2-positive-film',
    ]);
    expect(ids(missingNativeLooks(imported.slice(0, 1), nativeLooksFor('gr4')))).toEqual(['gr2-positive-film-gr4']);
    // A Cinema look already on the card is not offered again as an unlock tile.
    expect(
      ids(missingNativeLooks([{ id: 'lut-a' }, { id: 'gr4-cinema-yellow' }], nativeLooksFor('gr3'))),
    ).toEqual(['gr4-cinema-green', 'gr2-positive-film']);
  });
});

describe('nativeProblem', () => {
  it('maps the grkit-core messages', () => {
    const problem = (message: string) => nativeProblem(new Error(message));
    expect(problem('This is not a supported GR IV firmware file. Supported versions: 1.11, 1.20')).toBe(
      'unsupported',
    );
    expect(problem('This is not a supported GR II firmware file. Supported versions: 3.00')).toBe('unsupported');
    expect(problem('The ZIP file has no fwdc248b.bin')).toBe('notFirmware');
    expect(problem('Invalid ZIP file: invalid Zip archive')).toBe('notFirmware');
    expect(problem('The file is too large')).toBe('tooLarge');
    expect(problem('fwdc248b.bin is too large')).toBe('tooLarge');
    expect(problem('The firmware read timed out')).toBe('failed');
    expect(nativeProblem('unknown')).toBe('failed');
  });
});

/** grkit-wasm, which the page builds before its checks (`pnpm wasm`). */
async function loadWasm() {
  const wasm = await import('../wasm/grkit_wasm.js');
  await wasm.default({
    module_or_path: readFileSync(resolve(import.meta.dirname, '../wasm/grkit_wasm_bg.wasm')),
  });
  return wasm;
}

// The analytic stand-in of the grkit-wasm tests: any rising table works for these checks.
const baseGamma = Uint16Array.from({ length: 256 }, (_, i) => Math.floor((i / 255) ** 0.45 * 16000));

describe('nativeLooks errors', () => {
  it('give the problems that the dialog explains', async () => {
    const wasm = await loadWasm();
    const problem = (bytes: Uint8Array) => {
      try {
        wasm.nativeLooks(bytes, baseGamma);
      } catch (error) {
        return nativeProblem(error);
      }
      throw new Error('nativeLooks read the file');
    };
    expect(problem(new Uint8Array(64))).toBe('unsupported');
    expect(problem(new TextEncoder().encode('PK not a zip'))).toBe('notFirmware');
  });
});

// A check with an official GR IV ZIP, which the repository does not hold. Set GRKIT_TEST_GR4_ZIP to
// its path to run it.
const zip = process.env.GRKIT_TEST_GR4_ZIP;

describe.skipIf(!zip)('nativeLooks with an official GR IV ZIP', () => {
  it('reads Cinema Yellow and Cinema Green', async () => {
    const wasm = await loadWasm();
    const read = wasm.nativeLooks(readFileSync(zip!), baseGamma).map((look) => {
      try {
        return { id: look.id, name: look.name, version: look.version, blob: look.blob, preview: look.preview };
      } finally {
        look.free();
      }
    });
    expect(read.map((look) => look.id)).toEqual(NATIVE_LOOKS.map((look) => look.id));
    expect(read.map((look) => look.name)).toEqual(NATIVE_LOOKS.map((look) => look.name));
    for (const look of read) {
      expect(['1.11', '1.20']).toContain(look.version);
      expect(new TextDecoder().decode(look.blob.subarray(0, 8))).toBe('GRFX0001');
      expect(look.preview.length).toBe(33 ** 3 * 3);
    }
  });
});
