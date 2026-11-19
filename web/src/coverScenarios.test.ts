import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it, vi } from 'vitest';
import { LIBRARY } from './card';
import { changedOnCard, coverLibraryAdds, schemeOnCard, type CardSession } from './cardSession';
import type { CatalogEffect, Cover, ImportedEffect, LutImport, NativeImport, RestoredImport } from './catalog';
import { colorsFromLook } from './coverPalette';
import {
  cardCustomizations,
  customCoverOf,
  loadCustomizations,
  loadCoverScheme,
  mergeCustomizations,
  migrateCoverScheme,
  migratedScheme,
  restoredCustomizations,
  saveCustomizations,
  siteCoverFields,
  withoutCustomCovers,
} from './customCovers';
import { encodeLibrary, readLibrary } from './import/library';
import { coverIcon } from './import/names';
import { NATIVE_LOOKS } from './import/native';
import { relabelNative } from './import/edit';
import {
  catalogEffectBytes,
  coverPixels,
  effectLabel,
  importBytes,
  libraryCovers,
  libraryImports,
  loadWasm,
} from './package';
import { shownCover, type CoverScheme, type Customization, type Customizations } from './packageCovers';

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

const PORTRA: Cover = {
  template: 'box',
  brand: 'Kodak',
  name: 'PORTRA',
  number: '400',
  band: false,
};
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
  const blob = await effectFile(
    id,
    name,
    cover ?? coverIcon({ template: 'box', look: 'gray', brand: 'X', name: 'X', number: '', band: false }),
  );
  const imported: LutImport | RestoredImport = cover
    ? {
        kind: 'lut',
        blob,
        table: new Uint8Array(),
        grade: 'A',
        input: 'standard',
        warnings: [],
        source: `${name}.cube`,
        hints: '',
      }
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
    imported: {
      kind: 'native',
      blob,
      table: new Uint8Array(),
      version: '1.20',
      source: 'fw.zip',
      tone: new Uint16Array(),
    },
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
  covers: Customizations;
  scheme?: CoverScheme;
}

/**
 * The card after a write of `page`, as `applyCard` makes it: each changed site look and import
 * from the package, and each changed native look that only the card has from its own file.
 */
async function write(session: CardSession, page: Page): Promise<CardSession> {
  const catalog = page.catalog.map((item) => item.effect);
  const changed = await changedOnCard(session, catalog, page.imported, page.covers, page.scheme);
  const files = new Map(session.contents);
  for (const id of changed) {
    const site = page.catalog.find((item) => item.effect.id === id);
    const own = page.imported.find((effect) => effect.id === id);
    if (site) files.set(id, await catalogEffectBytes(site.effect, site.site, page.covers, page.scheme));
    else if (own) files.set(id, await importBytes(own, page.scheme ?? 'gray'));
  }
  const adds = await coverLibraryAdds(session, changed, catalog, page.imported, [], page.scheme);
  for (const add of adds) {
    const id = [...session.effects].find(([, paths]) => paths.some((path) => path.endsWith(add.path)))![0];
    files.set(id, add.bytes);
  }
  return { ...session, contents: files };
}

/** Connects, writes, and connects again. Returns what each connection marks as changed. */
async function connectWriteConnect(session: CardSession, page: Page) {
  const catalog = page.catalog.map((item) => item.effect);
  const before = await changedOnCard(session, catalog, page.imported, page.covers, page.scheme);
  const written = await write(session, page);
  const after = await changedOnCard(written, catalog, page.imported, page.covers, page.scheme);
  return { before: [...before].sort(), after: [...after], written };
}

describe('site film looks on a card', () => {
  it('leaves the gray site covers alone', async () => {
    const portra = await siteLook('portra-400-v1', 'Portra 400', PORTRA);
    const result = await connectWriteConnect(card({ [portra.effect.id]: portra.site }), {
      catalog: [portra],
      imported: [],
      covers: new Map(),
    });
    expect(result).toMatchObject({ before: [], after: [] });
  });

  it('writes a custom name and cover, and the site file after a reset', async () => {
    const portra = await siteLook('portra-400-v1', 'Portra 400', PORTRA);
    const custom = coverIcon({
      template: 'stack',
      look: 'velvia',
      brand: 'Mine',
      name: 'PORTRA',
      number: '',
      band: false,
    });
    const page = {
      catalog: [portra],
      imported: [],
      covers: new Map([[portra.effect.id, { name: 'Warm', cover: custom }]]),
    };
    const result = await connectWriteConnect(card({ [portra.effect.id]: portra.site }), page);
    expect(result.before).toEqual(['portra-400-v1']);
    expect(result.after).toEqual([]);
    const written = (await effectLabel(result.written.contents!.get('portra-400-v1')!))!;
    expect([written.name, written.icon]).toEqual(['Warm', await coverPixels(custom)]);

    const reset = await connectWriteConnect(result.written, { ...page, covers: new Map() });
    expect(reset.before).toEqual(['portra-400-v1']);
    expect(reset.after).toEqual([]);
    expect(same(reset.written.contents!.get('portra-400-v1')!, portra.site)).toBe(true);
  });

  it('checks the site file before it changes the icon', async () => {
    const portra = await siteLook('portra-400-v1', 'Portra 400', PORTRA);
    const damaged = portra.site.slice();
    damaged[200] = damaged[200]! ^ 1;
    const custom = coverIcon({
      template: 'box',
      look: 'kodak',
      brand: 'Kodak',
      name: 'PORTRA',
      number: '',
      band: false,
    });
    await expect(
      catalogEffectBytes(portra.effect, damaged, new Map([[portra.effect.id, { cover: custom }]])),
    ).rejects.toThrow('Effect checksum mismatch: portra-400-v1');
    await expect(catalogEffectBytes(portra.effect, damaged, new Map())).rejects.toThrow('checksum');
  });
});

describe('the cover scheme', () => {
  const box = (cover: Cover) => ({
    ...cover,
    colors: colorsFromLook(cover === PORTRA ? 'portra' : 'trix'),
  });
  const mine = coverIcon({
    template: 'stack',
    look: 'velvia',
    brand: 'Mine',
    name: 'TRI-X',
    number: '',
    band: false,
  });

  it('changes both ways the covers without an individual cover, and keeps names and individual covers', async () => {
    const portra = await siteLook('portra-400-v1', 'Portra 400', PORTRA);
    const trix = await siteLook('tri-x-400-v1', 'Tri-X 400', TRIX);
    const catalog = [portra.effect, trix.effect];
    const covers = new Map([
      [portra.effect.id, { name: 'Warm' }],
      [trix.effect.id, { cover: mine }],
    ]);
    let session = card({ [portra.effect.id]: portra.site, [trix.effect.id]: trix.site });
    for (const [scheme, portraCover] of [
      ['filmBox', box(PORTRA)],
      ['gray', PORTRA],
    ] as const) {
      const result = await connectWriteConnect(session, {
        catalog: [portra, trix],
        imported: [],
        covers,
        scheme,
      });
      expect(result.after).toEqual([]);
      session = result.written;
      const label = (await effectLabel(session.contents!.get(portra.effect.id)!))!;
      expect([label.name, label.icon]).toEqual(['Warm', await coverPixels(portraCover)]);
      expect(await icon(session.contents!.get(trix.effect.id)!)).toEqual(await coverPixels(mine));
      // The card tells the scheme by the look without an individual cover.
      expect(await schemeOnCard(session, catalog, new Set([trix.effect.id]), scheme)).toBe(scheme);
    }
    // A look that is not on the card yet follows the scheme at the next write.
    const added = await catalogEffectBytes(trix.effect, trix.site, new Map(), 'filmBox');
    expect(await icon(added)).toEqual(await coverPixels(box(TRIX)));
  });

  it('switches every cover of a browser that kept film box covers as custom covers', async () => {
    const portra = await siteLook('portra-400-v1', 'Portra 400', PORTRA);
    const trix = await siteLook('tri-x-400-v1', 'Tri-X 400', TRIX);
    const covers = new Map([
      [portra.effect.id, { cover: box(PORTRA) }],
      [trix.effect.id, { cover: box(TRIX) }],
    ]);
    let session = card({ [portra.effect.id]: portra.site, [trix.effect.id]: trix.site });
    expect(await schemeOnCard(session, [portra.effect, trix.effect], new Set(), 'filmBox')).toBe('gray');
    for (const [scheme, changed] of [
      ['gray', []],
      ['filmBox', [portra.effect.id, trix.effect.id].sort()],
      ['gray', [portra.effect.id, trix.effect.id].sort()],
    ] as const) {
      const result = await connectWriteConnect(session, {
        catalog: [portra, trix],
        imported: [],
        covers,
        scheme,
      });
      expect(result.before).toEqual(changed);
      expect(result.after).toEqual([]);
      session = result.written;
    }
  });

  it('comes from the color and the B&W looks of a card without an individual cover', async () => {
    const portra = await siteLook('portra-400-v1', 'Portra 400', PORTRA);
    const trix = await siteLook('tri-x-400-v1', 'Tri-X 400', TRIX);
    const mono = { ...trix.effect, monochrome: true };
    const catalog = [portra.effect, mono];
    const boxed = await catalogEffectBytes(portra.effect, portra.site, new Map(), 'filmBox');
    // An icon that an earlier drawing of the film box made, a little different, still counts.
    const drawn = await effectFile(trix.effect.id, 'Tri-X 400', { ...box(TRIX), number: '401' });
    expect(await icon(drawn)).not.toEqual(await coverPixels(box(TRIX)));
    const scheme = (files: Record<string, Uint8Array>, browser: CoverScheme = 'gray', individual: string[] = []) =>
      schemeOnCard(card(files), catalog, new Set(individual), browser);
    expect(await scheme({ [portra.effect.id]: boxed, [trix.effect.id]: drawn })).toBe('filmBox');
    expect(await scheme({ [portra.effect.id]: boxed, [trix.effect.id]: trix.site })).toBe('colorFilmBox');
    expect(await scheme({ [portra.effect.id]: portra.site, [trix.effect.id]: drawn })).toBe('gray');
    // A group without a counted look keeps what the browser gives it.
    expect(await scheme({ [portra.effect.id]: boxed }, 'filmBox')).toBe('filmBox');
    expect(await scheme({ [portra.effect.id]: boxed, [trix.effect.id]: drawn }, 'gray', [trix.effect.id])).toBe(
      'colorFilmBox',
    );
    expect(await scheme({}, 'colorFilmBox')).toBe('colorFilmBox');
  });

  it('gives film box covers only to color looks with colorFilmBox', async () => {
    const portra = await siteLook('portra-400-v1', 'Portra 400', PORTRA);
    const trix = await siteLook('tri-x-400-v1', 'Tri-X 400', TRIX);
    const mono = { ...trix, effect: { ...trix.effect, monochrome: true } };
    const session = card({ [portra.effect.id]: portra.site, [trix.effect.id]: trix.site });
    const result = await connectWriteConnect(session, {
      catalog: [portra, mono],
      imported: [],
      covers: new Map(),
      scheme: 'colorFilmBox',
    });
    expect([result.before, result.after]).toEqual([[portra.effect.id], []]);
    expect(await icon(result.written.contents!.get(portra.effect.id)!)).toEqual(await coverPixels(box(PORTRA)));
  });

  it('gives the Cinema looks their gray or Cinema cover, and keeps an edited name and cover', async () => {
    const yellow = await nativeImport(0);
    const base = NATIVE_LOOKS[0]!.cover;
    for (const [scheme, cover] of [
      ['gray', { ...base, colors: undefined }],
      ['filmBox', base],
    ] as const) {
      const result = await connectWriteConnect(card({ [yellow.id]: yellow.imported.blob }), {
        catalog: [],
        imported: [yellow],
        covers: new Map(),
        scheme,
      });
      expect(result.after).toEqual([]);
      expect(await icon(result.written.contents!.get(yellow.id)!)).toEqual(await coverPixels(cover));
      expect(await schemeOnCard(result.written, NATIVE_LOOKS, new Set(), scheme)).toBe(scheme);
    }
    // An edited Cinema look: the card file and its library entry have the name and the cover.
    const own = coverIcon({
      template: 'cine',
      look: 'velvia',
      brand: 'RICOH',
      name: 'CINEMA',
      number: 'MINE',
      band: false,
    });
    const edited = await relabelNative(yellow, base, 'My Cinema', own);
    const written = await importBytes(edited, 'gray');
    expect(await effectLabel(written)).toMatchObject({
      name: 'My Cinema',
      icon: await coverPixels(own),
    });
    const [entry] = await readLibrary((await encodeLibrary(await libraryImports([edited], 'gray')))!);
    expect(entry!.details).toMatchObject({ kind: 'native', cover: own });
    expect((await effectLabel(entry!.effect))!.name).toBe('My Cinema');
  });

  it('comes from a restored ZIP, which wins for the looks that it holds', () => {
    const browser = new Map([
      ['portra-400-v1', { name: 'Mine', cover: mine }],
      ['tri-x-400-v1', { cover: mine }],
      ['ektar-100-v1', { name: 'Kept' }],
    ]);
    const zip = new Map([['portra-400-v1', { name: 'Warm' }]]);
    const held = new Set(['portra-400-v1', 'tri-x-400-v1']);
    expect([...restoredCustomizations(browser, zip, held)]).toEqual([
      ['ektar-100-v1', { name: 'Kept' }],
      ['portra-400-v1', { name: 'Warm' }],
    ]);
  });

  it('clears the custom covers and keeps the names', async () => {
    const portra = await siteLook('portra-400-v1', 'Portra 400', PORTRA);
    const customs = new Map([
      [portra.effect.id, { name: 'Warm', cover: mine }],
      ['lut-a', { cover: mine }],
    ]);
    expect([...withoutCustomCovers([portra.effect], customs)]).toEqual([
      [portra.effect.id, { name: 'Warm' }],
      ['lut-a', { cover: mine }],
    ]);
  });

  it('moves the covers of the earlier film box switch to the scheme', async () => {
    const portra = await siteLook('portra-400-v1', 'Portra 400', PORTRA);
    const trix = await siteLook('tri-x-400-v1', 'Tri-X 400', TRIX);
    const catalog = [portra.effect, trix.effect];
    const boxed = new Map<string, Customization>([
      [portra.effect.id, { name: 'Warm', cover: box(PORTRA) }],
      [trix.effect.id, { cover: box(TRIX) }],
    ]);
    expect(migratedScheme(catalog, boxed)).toEqual({
      scheme: 'filmBox',
      customs: new Map([[portra.effect.id, { name: 'Warm' }]]),
    });
    // One cover of the person's own: gray, and only that cover stays.
    const mixed = new Map([...boxed, [trix.effect.id, { cover: mine }]]);
    expect(migratedScheme(catalog, mixed)).toEqual({
      scheme: 'gray',
      customs: new Map([
        [portra.effect.id, { name: 'Warm' }],
        [trix.effect.id, { cover: mine }],
      ]),
    });
  });

  it('gives a new visitor film box colors for the color looks, and keeps a chosen scheme', async () => {
    const values = new Map<string, string>();
    const storage = {
      getItem: (key: string) => values.get(key) ?? null,
      setItem: (key: string, value: string) => void values.set(key, value),
    };
    await migrateCoverScheme(storage, async () => []);
    expect(loadCoverScheme(storage)).toBe('colorFilmBox');
    values.set('grkit.coverScheme', 'gray');
    await migrateCoverScheme(storage, async () => []);
    expect(loadCoverScheme(storage)).toBe('gray');
  });
});

describe('imports on a card', () => {
  it('keeps the cover that the file of an import has', async () => {
    const covers = [
      PORTRA,
      coverIcon({
        template: 'box',
        look: 'gray',
        brand: 'Kodak',
        name: 'PORTRA',
        number: '400',
        band: false,
      }),
      coverIcon({
        template: 'box',
        look: 'kodak',
        brand: 'Kodak',
        name: 'PORTRA',
        number: '400',
        band: false,
      }),
      undefined,
    ];
    for (const cover of covers) {
      const effect = await lutImport('lut-portra-1a2b', 'My Portra', cover);
      const result = await connectWriteConnect(card({ [effect.id]: effect.imported.blob }), {
        catalog: [],
        imported: [effect],
        covers: new Map(),
      });
      expect(result).toMatchObject({ before: [], after: [] });
    }
  });

  it('updates an import that the user renamed on another visit', async () => {
    const before = await lutImport('lut-portra-1a2b', 'My Portra', PORTRA);
    const renamed = await lutImport('lut-portra-1a2b', 'Portra Warm', PORTRA);
    const result = await connectWriteConnect(card({ [before.id]: before.imported.blob }), {
      catalog: [],
      imported: [renamed],
      covers: new Map(),
    });
    expect(result.before).toEqual(['lut-portra-1a2b']);
    expect(result.after).toEqual([]);
    expect((await effectLabel(result.written.contents!.get(before.id)!))!.name).toBe('Portra Warm');
  });
});

describe('a card with every kind of film look', () => {
  it('settles after one write for each custom cover', async () => {
    const portra = await siteLook('portra-400-v1', 'Portra 400', PORTRA);
    const trix = await siteLook('tri-x-400-v1', 'Tri-X 400', TRIX);
    const oldLut = await lutImport('lut-portra-1a2b', 'My Portra', PORTRA);
    const keptLut = await lutImport(
      'lut-gold-77aa',
      'My Gold',
      coverIcon({
        template: 'cine',
        look: 'gold',
        brand: 'Kodak',
        name: 'GOLD',
        number: '200',
        band: false,
      }),
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
      [
        trix.effect.id,
        {
          cover: coverIcon({
            template: 'box',
            look: 'ilford',
            brand: 'Kodak',
            name: 'TRI-X',
            number: '',
            band: true,
          }),
        },
      ],
    ]);
    const imported = [oldLut, keptLut, restored, yellow];
    const steps: [Customizations, string[]][] = [
      [new Map(), []],
      [custom, ['tri-x-400-v1']],
      [new Map(), ['tri-x-400-v1']],
    ];
    for (const [covers, expected] of steps) {
      const result = await connectWriteConnect(session, {
        catalog: [portra, trix],
        imported,
        covers,
      });
      expect(result.before).toEqual(expected);
      expect(result.after).toEqual([]);
      session = result.written;
      // The shelf draws what the camera shows: the icon of each file on the card.
      const shelf: [string, Cover | undefined, 'own' | 'site', Uint8Array?][] = [
        [portra.effect.id, PORTRA, 'site'],
        [trix.effect.id, TRIX, 'site'],
        [oldLut.id, oldLut.cover, 'own'],
        [keptLut.id, keptLut.cover, 'own'],
        [restored.id, undefined, 'own', restored.imported.blob],
        [yellow.id, yellow.cover, 'site'],
        [green.id, NATIVE_LOOKS[1]!.cover, 'site'],
      ];
      for (const [id, own, source, file] of shelf) {
        const cover = shownCover(own, source, covers.get(id)?.cover);
        const drawn = cover ? await coverPixels(cover) : await icon(file!);
        expect(await icon(session.contents!.get(id)!), id).toEqual(drawn);
      }
    }
  });
});

describe('customizations in the import library', () => {
  it('come back in another browser from the card, unless the card file has another name or cover', async () => {
    const portra = await siteLook('portra-400-v1', 'Portra 400', PORTRA);
    const custom = coverIcon({
      template: 'box',
      look: 'velvia',
      brand: 'Kodak',
      name: 'PORTRA',
      number: '',
      band: true,
    });
    const covers = new Map([[portra.effect.id, { name: 'Warm Portra', cover: custom }]]);
    const lut = await lutImport(
      'lut-portra-1a2b',
      'My Portra',
      coverIcon({ template: 'box', look: 'kodak', brand: 'K', name: 'P', number: '', band: false }),
    );
    const baked = await catalogEffectBytes(portra.effect, portra.site, covers);
    expect((await effectLabel(baked))!.name).toBe('Warm Portra');
    const library = await encodeLibrary([lut], libraryCovers([portra.effect, lut], [baked, lut.imported.blob], covers));
    const items = await Promise.all(
      (await readLibrary(library!)).map(async (entry) => ({
        id: (await effectLabel(entry.effect))!.id,
        details: entry.details,
      })),
    );
    expect(items.map((item) => [item.id, item.details.kind, item.details.name])).toEqual([
      ['lut-portra-1a2b', undefined, undefined],
      ['portra-400-v1', 'cover', 'Warm Portra'],
    ]);
    const siteIds = [portra.effect];
    const cardLabel = (bytes: Uint8Array) => async () => (await effectLabel(bytes))!;

    // The card file has the custom name and cover: another browser takes them.
    const taken = await cardCustomizations(items, siteIds, cardLabel(baked), coverPixels);
    expect([...taken]).toEqual([[portra.effect.id, { name: 'Warm Portra', cover: custom }]]);
    // A later write put the site name and the default cover back and left the old entry.
    expect((await cardCustomizations(items, siteIds, cardLabel(portra.site), coverPixels)).size).toBe(0);
    // Without the site look, or without the file on the card, the entry does not count either.
    expect((await cardCustomizations(items, [], cardLabel(baked), coverPixels)).size).toBe(0);
    expect((await cardCustomizations(items, siteIds, async () => undefined, coverPixels)).size).toBe(0);

    // What this browser has wins over the card's, field by field.
    const mine = coverIcon({
      template: 'cine',
      look: 'kodak',
      brand: 'Kodak',
      name: 'PORTRA',
      number: '',
      band: false,
    });
    expect(mergeCustomizations(new Map([[portra.effect.id, { cover: mine }]]), taken).get(portra.effect.id)).toEqual({
      name: 'Warm Portra',
      cover: mine,
    });
  });

  it('holds only customizations of site looks in the package', () => {
    const blob = new Uint8Array([1]);
    const site = {
      id: 'portra-400-v1',
      file: 'P.GRF',
      sha256: 'x',
      cover: PORTRA,
    } as unknown as CatalogEffect;
    const lut = { id: 'lut-a', imported: { kind: 'lut' } } as unknown as ImportedEffect;
    const cover = { ...PORTRA, colors: colorsFromLook('kodak') };
    const custom = new Map([
      ['portra-400-v1', { cover }],
      ['lut-a', { cover }],
      ['ektar-100-v1', { cover }],
    ]);
    expect(libraryCovers([site, lut], [blob, blob], custom)).toEqual([{ cover, effect: blob }]);
  });
});

describe('customizations in this browser', () => {
  function memoryStorage() {
    const items = new Map<string, string>();
    return {
      getItem: (key: string) => items.get(key) ?? null,
      setItem: (key: string, value: string) => void items.set(key, value),
      items,
    };
  }

  it('are kept, an old cover-only value still reads, and an unreadable value is none', () => {
    const storage = memoryStorage();
    const cover = coverIcon({
      template: 'box',
      look: 'kodak',
      brand: 'Kodak',
      name: 'PORTRA',
      number: '400',
      band: true,
    });
    saveCustomizations(
      storage,
      new Map([
        ['portra-400-v1', { name: 'Warm', cover }],
        ['tri-x-400-v1', { name: 'Mine' }],
      ]),
    );
    expect([...loadCustomizations(storage)]).toEqual([
      ['portra-400-v1', { name: 'Warm', cover }],
      ['tri-x-400-v1', { name: 'Mine' }],
    ]);

    storage.items.set('grkit.customCovers', JSON.stringify({ a: PORTRA, b: { ...cover, brand: '' }, c: cover }));
    expect([...loadCustomizations(storage)]).toEqual([['c', { cover }]]);
    storage.items.set('grkit.customCovers', '{');
    expect(loadCustomizations(storage).size).toBe(0);
    expect(loadCustomizations(null).size).toBe(0);
  });

  it('open the dialog on the cover the camera shows, and the gray and film box covers are no custom covers', () => {
    const gray = siteCoverFields(PORTRA, undefined, 'gray')!;
    expect(gray).toMatchObject({ look: 'gray', brand: 'Kodak', name: 'PORTRA', number: '400' });
    const custom = coverIcon({
      template: 'stack',
      look: 'velvia',
      brand: 'Mine',
      name: 'PORTRA',
      number: '',
      band: false,
    });
    expect(siteCoverFields(PORTRA, custom, 'filmBox')).toMatchObject({
      template: 'stack',
      look: 'velvia',
      brand: 'Mine',
    });
    expect(siteCoverFields(undefined, undefined, 'gray')).toBeNull();

    // The gray and the film box cover are no individual covers; another one is.
    const boxed = siteCoverFields(PORTRA, undefined, 'filmBox')!;
    expect(boxed.look).toBe('portra');
    expect(customCoverOf(PORTRA, gray)).toBeNull();
    expect(customCoverOf(PORTRA, boxed)).toBeNull();
    expect(customCoverOf(PORTRA, { ...gray, look: 'kodak' })).toEqual(coverIcon({ ...gray, look: 'kodak' }));
  });
});
