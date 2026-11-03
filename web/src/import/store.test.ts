import { describe, expect, it } from 'vitest';
import { storedImport } from './store';

const blob = () => new TextEncoder().encode('GRFX0001 effect');

/** A record as a page before the native looks stored it: a LUT import without `kind`. */
const legacy = () => ({
  id: 'lut-my-look',
  name: 'My Look',
  icon: '',
  cover: { template: 'box', brand: 'LUT', name: 'MY LOOK', band: false },
  monochrome: false,
  preview: 'imported/lut-my-look/1',
  imported: {
    blob: blob(),
    table: new Uint8Array(3),
    grade: 'B',
    input: 'standard',
    warnings: [],
    source: 'My Look.cube',
  },
  savedAt: 1,
});

const native = () => ({
  id: 'gr4-cinema-yellow',
  name: 'Cinema Yellow',
  icon: '',
  cover: { template: 'cine', brand: 'GR IV', name: 'CINEMA', number: 'YELLOW' },
  monochrome: false,
  preview: 'native/gr4-cinema-yellow/1',
  imported: { kind: 'native', blob: blob(), table: new Uint8Array(3), version: '1.20', source: 'gr4_v120.zip' },
  savedAt: 2,
});

describe('storedImport', () => {
  it('reads a record without a kind as a LUT import', () => {
    const item = storedImport(legacy());
    expect(item?.imported.kind).toBe('lut');
    expect(item?.imported).toMatchObject({ grade: 'B', source: 'My Look.cube' });
  });

  it('keeps a native look with its version', () => {
    const item = storedImport(native());
    expect(item?.imported).toMatchObject({ kind: 'native', version: '1.20' });
    expect(item?.savedAt).toBe(2);
  });

  it('drops a native look without a version, an unknown kind, and a blob that is not an effect', () => {
    const noVersion = native();
    delete (noVersion.imported as Partial<typeof noVersion.imported>).version;
    expect(storedImport(noVersion)).toBeNull();
    const unknown = native();
    unknown.imported.kind = 'recipe';
    expect(storedImport(unknown)).toBeNull();
    const notEffect = legacy();
    notEffect.imported.blob = new Uint8Array(8);
    expect(storedImport(notEffect)).toBeNull();
    expect(storedImport(null)).toBeNull();
  });
});
