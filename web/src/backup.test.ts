import { describe, expect, it } from 'vitest';
import { BACKUP_SCHEMA, createBackup, decodeImport, readBackup } from './backup';
import type { ImportedEffect } from './catalog';

const bytes = (text = 'GRFX0001 effect') => new TextEncoder().encode(text);

const lut = (): ImportedEffect => ({
  id: 'lut-my-look',
  name: 'My Look',
  icon: 'AAEC/w==',
  cover: { template: 'box', brand: 'LUT', name: 'MY LOOK', band: false },
  monochrome: false,
  preview: 'imported/lut-my-look/1',
  imported: {
    kind: 'lut',
    blob: bytes(),
    table: new Uint8Array([0, 1, 2, 255]),
    grade: 'B',
    input: 'standard',
    warnings: ['Check the shadows.'],
    source: 'My Look.cube',
  },
});

const native = (): ImportedEffect => ({
  id: 'gr4-cinema-yellow',
  name: 'Cinema Yellow',
  icon: '',
  cover: { template: 'cine', brand: 'RICOH', name: 'CINEMA', number: 'YELLOW' },
  monochrome: false,
  preview: 'native/gr4-cinema-yellow/1',
  imported: {
    kind: 'native',
    blob: bytes(),
    table: new Uint8Array([9]),
    version: '1.20',
    source: 'gr4_v120.zip',
  },
});

const file = async (...imports: ImportedEffect[]) => createBackup(imports, {
  language: 'zh-Hant',
  installMode: 'zip',
  selection: ['lut-my-look', 'gr4-cinema-yellow'],
}, 1_700_000_000_000).text();

describe('backup', () => {
  it('round-trips the imports and the settings', async () => {
    const backup = readBackup(await file(lut(), native()));
    expect(backup).not.toBeNull();
    expect(backup?.settings).toEqual({
      language: 'zh-Hant',
      installMode: 'zip',
      selection: ['lut-my-look', 'gr4-cinema-yellow'],
    });
    expect(backup?.imports.map((effect) => effect.id)).toEqual([
      'lut-my-look',
      'gr4-cinema-yellow',
    ]);
    expect(backup?.imports[0]?.imported).toMatchObject({
      kind: 'lut',
      source: 'My Look.cube',
      grade: 'B',
      input: 'standard',
      warnings: ['Check the shadows.'],
    });
    expect(backup?.imports[0]?.imported.blob).toEqual(bytes());
    expect(backup?.imports[0]?.imported.table).toEqual(new Uint8Array([0, 1, 2, 255]));
    expect(backup?.imports[1]?.imported).toMatchObject({ kind: 'native', version: '1.20' });
    // The times keep the order of the page, one after the other.
    expect(backup?.imports[0]?.savedAt).toBe(1_700_000_000_000);
    expect(backup?.imports[1]?.savedAt).toBe(1_700_000_000_001);
  });

  it('rejects a file that is not a backup, and keeps the settings that it can use', () => {
    expect(readBackup('not json')).toBeNull();
    expect(readBackup(JSON.stringify({ schema: 'other/1' }))).toBeNull();
    expect(readBackup(JSON.stringify({ schema: BACKUP_SCHEMA, imports: 'all' }))).toEqual({
      settings: {},
      imports: [],
    });
    expect(
      readBackup(
        JSON.stringify({
          schema: BACKUP_SCHEMA,
          settings: { language: 'fr', installMode: 'usb', selection: [1, 'ok', '', 'x'.repeat(40)] },
          imports: [],
        }),
      ),
    ).toEqual({ settings: { selection: ['ok'] }, imports: [] });
  });

  it('leaves out an import with bytes that the page cannot use', async () => {
    const text = await file(lut());
    const data = JSON.parse(text);
    const good = data.imports[0];
    const badMagic = { ...good, id: 'bad-magic', imported: { ...good.imported, blob: btoa('NOTGRFX0') } };
    const badBase64 = { ...good, id: 'bad-base64', imported: { ...good.imported, blob: '***' } };
    const noTime = { ...good, id: 'no-time', savedAt: undefined };
    const backup = readBackup(
      JSON.stringify({ ...data, imports: [good, badMagic, badBase64, noTime] }),
    );
    expect(backup?.imports.map((effect) => effect.id)).toEqual(['lut-my-look']);
  });

  it('decodes one import and drops a record that is not one', () => {
    expect(decodeImport({ id: 'x' })).toBeNull();
    expect(decodeImport(null)).toBeNull();
  });
});
