import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it, vi } from 'vitest';
import { LIBRARY } from './card';
import { changedOnCard, coverLibraryAdds, type CardSession } from './cardSession';
import type { CatalogEffect, Cover, ImportedEffect, LutImport, NativeImport, RestoredImport } from './catalog';
import { colorsFromLook, NEUTRAL_COLORS } from './coverPalette';
import {
  cardCustomCovers,
  customCoverOf,
  loadCustomCovers,
  mergeCustomCovers,
  saveCustomCovers,
  siteCoverFields,
} from './customCovers';
import { encodeLibrary, readLibrary } from './import/library';
import { coverIcon } from './import/names';
import { NATIVE_LOOKS } from './import/native';
import {
  catalogEffectBytes,
  coverPixels,
  effectLabel,
  libraryCovers,
  loadWasm,
  withPackagedCover,
} from './package';
import { shownCover, type CoverChoice, type CustomCovers } from './packageCovers';

// What the page does with the film covers of a card, with real effect files: grkit-wasm draws the
// icons and relabels the files as on the page. Each case connects a card that an earlier version
// or another browser wrote, checks what the page marks as changed, writes what a write puts there,
// and connects again: a second connection must find nothing left to change.

vi.mock('./transfer', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./transfer')>()),
  // The page downloads grkit-wasm. The checks read the build of `pnpm wasm`.
  fetchFile: async (url: string) => {
    if (url.includes('grkit_wasm_bg')) return readFileSync(resolve(import.meta.dirname, 'wasm/grkit_wasm_bg.wasm'));
    throw new Error(`No site file in the checks: ${url}`);
  },
}));

/** An effect file that `importLut` made from an identity LUT. The cases give it other IDs. */
const FIXTURE = new Uint8Array(readFileSync(resolve(import.meta.dirname, 'test/fixtures/effect.grf')));

const CRC_TABLE = Uint32Array.from({ length: 256 }, (_, n) => {
  let c = n;
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  return c >>> 0;
});

/** The CRC32 of an effect file, with its CRC field read as zero (`effects::checksum`). */
function checksum(bytes: Uint8Array): number {
  let crc = 0xffffffff;
  bytes.forEach((byte, i) => {
    const value = i >= 12 && i < 16 ? 0 : byte;
    crc = CRC_TABLE[(crc ^ value) & 0xff]! ^ (crc >>> 8);
  });
  return (crc ^ 0xffffffff) >>> 0;
}

/** An effect file with the ID `id`, the name `name`, and the icon of `cover`. */
async function effectFile(id: string, name: string, cover: Cover): Promise<Uint8Array> {
  const bytes = FIXTURE.slice();
  bytes.fill(0, 16, 48);
  bytes.set(new TextEncoder().encode(id), 16);
  new DataView(bytes.buffer).setUint32(12, checksum(bytes), true);
  const wasm = await loadWasm();
  return wasm.relabelEffect(bytes, name, wasm.iconPixels(JSON.stringify(cover)));
}

const icon = async (bytes: Uint8Array) => (await effectLabel(bytes))!.icon;
const same = (a: Uint8Array, b: Uint8Array) => a.length === b.length && a.every((byte, i) => byte === b[i]);

async function sha256(bytes: Uint8Array): Promise<string> {
  const digest = await crypto.subtle.digest('SHA-256', bytes as Uint8Array<ArrayBuffer>);
  return Array.from(new Uint8Array(digest), (b) => b.toString(16).padStart(2, '0')).join('');
}

const PORTRA: Cover = { template: 'box', brand: 'Kodak', name: 'PORTRA', number: '400', band: false };
const TRIX: Cover = { template: 'box', brand: 'Kodak', name: 'TRI-X', number: '400', band: true };

/** A site film look, with its site file: the gray cover, as the site publishes it. */
async function siteLook(id: string, name: string, cover: Cover): Promise<{ effect: CatalogEffect; site: Uint8Array }> {
  const site = await effectFile(id, name, cover);
  return {
    site,
    effect: {
      id,
      name,
      icon: '',
      cover,
      monochrome: false,
      look: '',
      origin: '',
      license: '',
      file: `${id.toUpperCase().slice(0, 8)}.GRF`,
      sha256: await sha256(site),
      preview: '',
    },
  };
}

async function lutImport(id: string, name: string, cover: Cover | undefined): Promise<ImportedEffect> {
  const blob = await effectFile(id, name, cover ?? coverIcon({ template: 'box', look: 'gray', brand: 'X', name: 'X', number: '', band: false }));
  const imported: LutImport | RestoredImport = cover
    ? { kind: 'lut', blob, table: new Uint8Array(), grade: 'A', input: 'standard', warnings: [], source: `${name}.cube`, hints: '' }
    : { kind: 'restored', blob, table: new Uint8Array(), source: 'GRKit.zip' };
  return { id, name, icon: '', cover, monochrome: false, preview: '', imported } as ImportedEffect;
}

/** A native look as an unlock makes it: the effect file has the gray cover, the page the colors. */
async function nativeImport(index: 0 | 1): Promise<ImportedEffect<NativeImport>> {
  const look = NATIVE_LOOKS[index]!;
  const { colors: _, ...gray } = look.cover;
  const blob = await effectFile(look.id, look.name, gray);
  return {
    id: look.id,
    name: look.name,
    icon: '',
    cover: look.cover,
    monochrome: false,
    preview: '',
    imported: { kind: 'native', blob, table: new Uint8Array(), version: '1.20', source: 'fw.zip' },
  };
}

/** A card with the effect files `files` in its library, by effect ID. */
function card(files: Record<string, Uint8Array>): CardSession {
  const ids = Object.keys(files);
  return {
    effects: new Map(ids.map((id, index) => [id, [`${LIBRARY}/EFFECT${index}.GRF`]])),
    contents: new Map(Object.entries(files)),
  } as unknown as CardSession;
}

interface Page {
  catalog: readonly { effect: CatalogEffect; site: Uint8Array }[];
  imported: readonly ImportedEffect[];
  covers: CoverChoice;
}

const choice = (colorful: boolean, custom: CustomCovers = new Map()): CoverChoice => ({ colorful, custom });

/**
 * The card after a write of `page`, as `applyCard` makes it: each changed site look and import
 * from the package, and each changed native look that only the card has from its own file.
 */
async function write(session: CardSession, page: Page): Promise<CardSession> {
  const catalog = page.catalog.map((item) => item.effect);
  const changed = await changedOnCard(session, catalog, page.imported, page.covers);
  const files = new Map(session.contents);
  for (const id of changed) {
    const site = page.catalog.find((item) => item.effect.id === id);
    const own = page.imported.find((effect) => effect.id === id);
    if (site) files.set(id, (await catalogEffectBytes(site.effect, site.site, page.covers)).bytes);
    else if (own) {
      const cover = own.cover ?? NATIVE_LOOKS.find((look) => look.id === id)?.cover;
      files.set(id, (await withPackagedCover(own.imported.blob, own.name, cover, page.covers.colorful)).bytes);
    }
  }
  const adds = await coverLibraryAdds(session, changed, catalog, page.imported, page.covers.colorful, []);
  for (const add of adds) {
    const id = [...session.effects].find(([, paths]) => paths.some((path) => path.endsWith(add.path)))![0];
    files.set(id, add.bytes);
  }
  return { ...session, contents: files };
}

/** Connects, writes, and connects again. Returns what each connection marks as changed. */
async function connectWriteConnect(session: CardSession, page: Page) {
  const catalog = page.catalog.map((item) => item.effect);
  const before = await changedOnCard(session, catalog, page.imported, page.covers);
  const written = await write(session, page);
  const after = await changedOnCard(written, catalog, page.imported, page.covers);
  return { before: [...before].sort(), after: [...after], written };
}

describe('site film looks on a card', () => {
  it('leaves gray site covers of an earlier version alone while color covers are off', async () => {
    const portra = await siteLook('portra-400-v1', 'Portra 400', PORTRA);
    const result = await connectWriteConnect(card({ [portra.effect.id]: portra.site }), {
      catalog: [portra],
      imported: [],
      covers: choice(false),
    });
    expect(result).toMatchObject({ before: [], after: [] });
  });

  it('recolors gray site covers once when color covers are on, and back to gray when off', async () => {
    const portra = await siteLook('portra-400-v1', 'Portra 400', PORTRA);
    const trix = await siteLook('tri-x-400-v1', 'Tri-X 400', TRIX);
    const page = { catalog: [portra, trix], imported: [], covers: choice(true) };
    const colored = await connectWriteConnect(card({ [portra.effect.id]: portra.site, [trix.effect.id]: trix.site }), page);
    expect(colored.before).toEqual(['portra-400-v1', 'tri-x-400-v1']);
    expect(colored.after).toEqual([]);
    expect(await icon(colored.written.contents!.get('portra-400-v1')!)).toEqual(
      await coverPixels({ ...PORTRA, colors: colorsFromLook('portra') }),
    );

    const gray = await connectWriteConnect(colored.written, { ...page, covers: choice(false) });
    expect(gray.before).toEqual(['portra-400-v1', 'tri-x-400-v1']);
    expect(gray.after).toEqual([]);
    expect(same(gray.written.contents!.get('portra-400-v1')!, portra.site)).toBe(true);
  });

  it('writes a custom cover over both switch states, and the default cover after a reset', async () => {
    const portra = await siteLook('portra-400-v1', 'Portra 400', PORTRA);
    const custom = coverIcon({ template: 'stack', look: 'velvia', brand: 'Mine', name: 'PORTRA', number: '', band: false });
    const covers = new Map([[portra.effect.id, custom]]);
    for (const colorful of [false, true]) {
      const result = await connectWriteConnect(card({ [portra.effect.id]: portra.site }), {
        catalog: [portra],
        imported: [],
        covers: choice(colorful, covers),
      });
      expect(result.before).toEqual(['portra-400-v1']);
      expect(result.after).toEqual([]);
      expect(await icon(result.written.contents!.get('portra-400-v1')!)).toEqual(await coverPixels(custom));
      // The switch does not change a custom cover on the card.
      const page = { catalog: [portra], imported: [], covers: choice(!colorful, covers) };
      expect([...(await changedOnCard(result.written, [portra.effect], [], page.covers))]).toEqual([]);
    }

    const withCustom = (await connectWriteConnect(card({ [portra.effect.id]: portra.site }), {
      catalog: [portra],
      imported: [],
      covers: choice(false, covers),
    })).written;
    const reset = await connectWriteConnect(withCustom, { catalog: [portra], imported: [], covers: choice(false) });
    expect(reset.before).toEqual(['portra-400-v1']);
    expect(reset.after).toEqual([]);
    expect(same(reset.written.contents!.get('portra-400-v1')!, portra.site)).toBe(true);
  });

  it('checks the site file before it changes the icon', async () => {
    const portra = await siteLook('portra-400-v1', 'Portra 400', PORTRA);
    const damaged = portra.site.slice();
    damaged[200] = damaged[200]! ^ 1;
    const custom = coverIcon({ template: 'box', look: 'kodak', brand: 'Kodak', name: 'PORTRA', number: '', band: false });
    await expect(
      catalogEffectBytes(portra.effect, damaged, choice(true, new Map([[portra.effect.id, custom]]))),
    ).rejects.toThrow('Effect checksum mismatch: portra-400-v1');
    await expect(catalogEffectBytes(portra.effect, damaged, choice(false))).rejects.toThrow('checksum');
  });
});

describe('imports on a card', () => {
  it('recolors a LUT import of a version without colors once, and keeps it after the write', async () => {
    // Imports before color covers kept their cover without colors and a gray icon.
    const old = await lutImport('lut-portra-1a2b', 'My Portra', PORTRA);
    const off = await connectWriteConnect(card({ [old.id]: old.imported.blob }), {
      catalog: [],
      imported: [old],
      covers: choice(false),
    });
    expect(off).toMatchObject({ before: [], after: [] });

    const on = await connectWriteConnect(card({ [old.id]: old.imported.blob }), {
      catalog: [],
      imported: [old],
      covers: choice(true),
    });
    expect(on.before).toEqual(['lut-portra-1a2b']);
    expect(on.after).toEqual([]);
    expect(await icon(on.written.contents!.get(old.id)!)).toEqual(
      await coverPixels({ ...PORTRA, colors: colorsFromLook('portra') }),
    );
  });

  it('keeps the colors that an import chose under both switch states, gray included', async () => {
    for (const look of ['gray', 'kodak'] as const) {
      const cover = coverIcon({ template: 'box', look, brand: 'Kodak', name: 'PORTRA', number: '400', band: false });
      const effect = await lutImport('lut-portra-1a2b', 'My Portra', cover);
      for (const colorful of [false, true]) {
        const result = await connectWriteConnect(card({ [effect.id]: effect.imported.blob }), {
          catalog: [],
          imported: [effect],
          covers: choice(colorful),
        });
        expect(result).toMatchObject({ before: [], after: [] });
      }
    }
    expect(coverIcon({ template: 'box', look: 'gray', brand: 'K', name: 'P', number: '', band: false }).colors).toEqual(
      NEUTRAL_COLORS,
    );
  });

  it('updates an import that the user renamed on another visit', async () => {
    const before = await lutImport('lut-portra-1a2b', 'My Portra', PORTRA);
    const renamed = await lutImport('lut-portra-1a2b', 'Portra Warm', PORTRA);
    const result = await connectWriteConnect(card({ [before.id]: before.imported.blob }), {
      catalog: [],
      imported: [renamed],
      covers: choice(false),
    });
    expect(result.before).toEqual(['lut-portra-1a2b']);
    expect(result.after).toEqual([]);
    expect((await effectLabel(result.written.contents!.get(before.id)!))!.name).toBe('Portra Warm');
  });

  it('leaves a restored import without cover fields as its file has it', async () => {
    const restored = await lutImport('lut-vision-9f00', 'Vision', undefined);
    for (const colorful of [false, true]) {
      const result = await connectWriteConnect(card({ [restored.id]: restored.imported.blob }), {
        catalog: [],
        imported: [restored],
        covers: choice(colorful),
      });
      expect(result).toMatchObject({ before: [], after: [] });
    }
  });

  it('recolors an unlocked Cinema look once, and back to gray when color covers are off', async () => {
    const yellow = await nativeImport(0);
    const page = { catalog: [], imported: [yellow], covers: choice(true) };
    const on = await connectWriteConnect(card({ [yellow.id]: yellow.imported.blob }), page);
    expect(on.before).toEqual(['gr4-cinema-yellow']);
    expect(on.after).toEqual([]);
    const off = await connectWriteConnect(on.written, { ...page, covers: choice(false) });
    expect(off.before).toEqual(['gr4-cinema-yellow']);
    expect(off.after).toEqual([]);
    expect(same(off.written.contents!.get(yellow.id)!, yellow.imported.blob)).toBe(true);
  });

  it('relabels a Cinema look that only the card has, both ways', async () => {
    // A browser without the unlock: the card file is the only copy of the look.
    const green = await nativeImport(1);
    const session = card({ [green.id]: green.imported.blob });
    const on = await connectWriteConnect(session, { catalog: [], imported: [], covers: choice(true) });
    expect(on.before).toEqual(['gr4-cinema-green']);
    expect(on.after).toEqual([]);
    expect(await icon(on.written.contents!.get(green.id)!)).toEqual(await coverPixels(green.cover));
    const off = await connectWriteConnect(on.written, { catalog: [], imported: [], covers: choice(false) });
    expect(off.before).toEqual(['gr4-cinema-green']);
    expect(off.after).toEqual([]);
    expect(same(off.written.contents!.get(green.id)!, green.imported.blob)).toBe(true);
  });
});

describe('a card with every kind of film look', () => {
  it('settles after one write for each switch state and custom cover', async () => {
    const portra = await siteLook('portra-400-v1', 'Portra 400', PORTRA);
    const trix = await siteLook('tri-x-400-v1', 'Tri-X 400', TRIX);
    const oldLut = await lutImport('lut-portra-1a2b', 'My Portra', PORTRA);
    const keptLut = await lutImport(
      'lut-gold-77aa',
      'My Gold',
      coverIcon({ template: 'cine', look: 'gold', brand: 'Kodak', name: 'GOLD', number: '200', band: false }),
    );
    const restored = await lutImport('lut-vision-9f00', 'Vision', undefined);
    const yellow = await nativeImport(0);
    const green = await nativeImport(1);
    let session = card({
      [portra.effect.id]: portra.site,
      [trix.effect.id]: trix.site,
      [oldLut.id]: oldLut.imported.blob,
      [keptLut.id]: keptLut.imported.blob,
      [restored.id]: restored.imported.blob,
      [yellow.id]: yellow.imported.blob,
      [green.id]: green.imported.blob,
    });
    const custom = new Map([
      [trix.effect.id, coverIcon({ template: 'box', look: 'ilford', brand: 'Kodak', name: 'TRI-X', number: '', band: true })],
    ]);
    const imported = [oldLut, keptLut, restored, yellow];
    const steps: [CoverChoice, string[]][] = [
      [choice(false), []],
      [choice(true), ['gr4-cinema-green', 'gr4-cinema-yellow', 'lut-portra-1a2b', 'portra-400-v1', 'tri-x-400-v1']],
      [choice(true, custom), ['tri-x-400-v1']],
      [choice(false, custom), ['gr4-cinema-green', 'gr4-cinema-yellow', 'lut-portra-1a2b', 'portra-400-v1']],
      [choice(false), ['tri-x-400-v1']],
    ];
    for (const [covers, expected] of steps) {
      const result = await connectWriteConnect(session, { catalog: [portra, trix], imported, covers });
      expect(result.before).toEqual(expected);
      expect(result.after).toEqual([]);
      session = result.written;
      // The shelf draws what the camera shows: the icon of each file on the card.
      const shelf: [string, Cover | undefined, 'own' | 'switch', Uint8Array?][] = [
        [portra.effect.id, PORTRA, 'switch'],
        [trix.effect.id, TRIX, 'switch'],
        [oldLut.id, oldLut.cover, 'own'],
        [keptLut.id, keptLut.cover, 'own'],
        [restored.id, undefined, 'own', restored.imported.blob],
        [yellow.id, yellow.cover, 'switch'],
        [green.id, NATIVE_LOOKS[1]!.cover, 'switch'],
      ];
      for (const [id, own, source, file] of shelf) {
        const cover = shownCover(own, source, covers.colorful, covers.custom.get(id));
        const drawn = cover ? await coverPixels(cover) : await icon(file!);
        expect(await icon(session.contents!.get(id)!), id).toEqual(drawn);
      }
    }
  });
});

describe('custom covers in the import library', () => {
  it('come back in another browser from the card, unless the card file has another cover', async () => {
    const portra = await siteLook('portra-400-v1', 'Portra 400', PORTRA);
    const custom = coverIcon({ template: 'box', look: 'velvia', brand: 'Kodak', name: 'PORTRA', number: '', band: true });
    const covers = new Map([[portra.effect.id, custom]]);
    const lut = await lutImport('lut-portra-1a2b', 'My Portra', coverIcon({ template: 'box', look: 'kodak', brand: 'K', name: 'P', number: '', band: false }));
    const baked = (await catalogEffectBytes(portra.effect, portra.site, choice(false, covers))).bytes;
    const library = await encodeLibrary([lut], libraryCovers([portra.effect, lut], [baked, lut.imported.blob], covers));
    const items = await Promise.all(
      (await readLibrary(library!)).map(async (entry) => ({
        id: (await effectLabel(entry.effect))!.id,
        details: entry.details,
      })),
    );
    expect(items.map((item) => [item.id, item.details.kind])).toEqual([
      ['lut-portra-1a2b', undefined],
      ['portra-400-v1', 'cover'],
    ]);
    const siteIds = new Set([portra.effect.id]);
    const cardIcon = (bytes: Uint8Array) => async () => icon(bytes);

    // The card file has the custom cover: another browser takes it.
    const taken = await cardCustomCovers(items, siteIds, cardIcon(baked), coverPixels);
    expect([...taken]).toEqual([[portra.effect.id, custom]]);
    // A later write put the default cover back and left the old entry: it does not count.
    expect((await cardCustomCovers(items, siteIds, cardIcon(portra.site), coverPixels)).size).toBe(0);
    // Without the site look, or without the file on the card, the entry does not count either.
    expect((await cardCustomCovers(items, new Set(), cardIcon(baked), coverPixels)).size).toBe(0);
    expect((await cardCustomCovers(items, siteIds, async () => undefined, coverPixels)).size).toBe(0);

    // The cover of this browser wins over the card's.
    const mine = coverIcon({ template: 'cine', look: 'kodak', brand: 'Kodak', name: 'PORTRA', number: '', band: false });
    expect(mergeCustomCovers(new Map([[portra.effect.id, mine]]), taken).get(portra.effect.id)).toEqual(mine);
    expect(mergeCustomCovers(new Map(), taken).get(portra.effect.id)).toEqual(custom);
  });

  it('holds only custom covers of site looks in the package', () => {
    const blob = new Uint8Array([1]);
    const site = { id: 'portra-400-v1', file: 'P.GRF', sha256: 'x' } as unknown as CatalogEffect;
    const lut = { id: 'lut-a', imported: { kind: 'lut' } } as unknown as ImportedEffect;
    const cover = { ...PORTRA, colors: colorsFromLook('kodak') };
    const custom = new Map([
      ['portra-400-v1', cover],
      ['lut-a', cover],
      ['ektar-100-v1', cover],
    ]);
    expect(libraryCovers([site, lut], [blob, blob], custom)).toEqual([{ cover, effect: blob }]);
  });
});

describe('custom covers in this browser', () => {
  function memoryStorage() {
    const items = new Map<string, string>();
    return {
      getItem: (key: string) => items.get(key) ?? null,
      setItem: (key: string, value: string) => void items.set(key, value),
      items,
    };
  }

  it('are kept with their colors, and an unreadable value is none', () => {
    const storage = memoryStorage();
    const cover = coverIcon({ template: 'box', look: 'kodak', brand: 'Kodak', name: 'PORTRA', number: '400', band: true });
    saveCustomCovers(storage, new Map([['portra-400-v1', cover]]));
    expect([...loadCustomCovers(storage)]).toEqual([['portra-400-v1', cover]]);

    storage.items.set('grkit.customCovers', JSON.stringify({ a: PORTRA, b: { ...cover, brand: '' }, c: cover }));
    expect([...loadCustomCovers(storage).keys()]).toEqual(['c']);
    storage.items.set('grkit.customCovers', '{');
    expect(loadCustomCovers(storage).size).toBe(0);
    expect(loadCustomCovers(null).size).toBe(0);
  });

  it('open the dialog on the cover the camera shows, and the default cover is no custom cover', () => {
    expect(siteCoverFields(PORTRA, false)).toMatchObject({ look: 'gray', brand: 'Kodak', name: 'PORTRA', number: '400' });
    expect(siteCoverFields(PORTRA, true)).toMatchObject({ look: 'portra' });
    const custom = coverIcon({ template: 'stack', look: 'velvia', brand: 'Mine', name: 'PORTRA', number: '', band: false });
    expect(siteCoverFields(PORTRA, false, custom)).toMatchObject({ template: 'stack', look: 'velvia', brand: 'Mine' });
    expect(siteCoverFields(undefined, true)).toBeNull();

    const fields = siteCoverFields(PORTRA, true)!;
    expect(customCoverOf(PORTRA, true, fields)).toBeNull();
    expect(customCoverOf(PORTRA, false, siteCoverFields(PORTRA, false)!)).toBeNull();
    // The colors of the switch-on default, saved with the switch off, stay as a custom cover.
    expect(customCoverOf(PORTRA, false, fields)).toEqual(coverIcon(fields));
  });
});
