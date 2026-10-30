import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import type { ImportedEffect } from '../catalog';
import { hasNativeLooks, importsForCamera, missingNativeLooks, NATIVE_LOOKS, nativeProblem, takesNativeLooks } from './native';

const effect = (id: string, kind: 'lut' | 'native') => ({ id, imported: { kind } }) as ImportedEffect;

describe('importsForCamera', () => {
  const imported = [effect('lut-a', 'lut'), effect('gr4-cinema-yellow', 'native'), effect('gr4-cinema-green', 'native')];

  it('keeps every import on GR III and GR IIIx', () => {
    expect(importsForCamera(imported, 'gr3')).toEqual(imported);
    expect(importsForCamera(imported, 'gr3x')).toEqual(imported);
    expect(takesNativeLooks('gr3x')).toBe(true);
  });

  it('leaves the native looks out on GR IV and GR IV Monochrome', () => {
    for (const camera of ['gr4', 'gr4m']) {
      expect(takesNativeLooks(camera)).toBe(false);
      expect(importsForCamera(imported, camera).map((item) => item.id)).toEqual(['lut-a']);
    }
  });

  it('tells which native looks the page lacks', () => {
    expect(NATIVE_LOOKS.map((look) => look.id)).toEqual(['gr4-cinema-yellow', 'gr4-cinema-green']);
    expect(missingNativeLooks(imported.slice(0, 1)).map((look) => look.id)).toEqual(
      NATIVE_LOOKS.map((look) => look.id),
    );
    expect(hasNativeLooks(imported.slice(0, 2))).toBe(false);
    expect(hasNativeLooks(imported)).toBe(true);
    expect(missingNativeLooks(imported.slice(2)).map((look) => look.id)).toEqual(['gr4-cinema-yellow']);
  });
});

describe('nativeProblem', () => {
  it('maps the grkit-core messages', () => {
    const problem = (message: string) => nativeProblem(new Error(message));
    expect(problem('This is not a supported GR IV firmware file. Supported versions: 1.11, 1.20')).toBe(
      'unsupported',
    );
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
