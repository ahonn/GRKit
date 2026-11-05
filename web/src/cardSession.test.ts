import { describe, expect, it, vi } from 'vitest';
import { INBOX, RECIPE_FILE, type CardDirectory, type CardFileHandle } from './card';
import {
  changedImports,
  extraCardEffects,
  openSession,
  packageEffects,
  withFamily,
  type CardSession,
} from './cardSession';
import type { CardEffect, ImportedEffect, NativeImport } from './catalog';
import type { Family } from './recipes';

// The wasm build reads the recipe file. Here each read gives one recipe that names the text and
// the camera line.
vi.mock('./recipes', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./recipes')>()),
  readRecipeFile: async (text: string, family: Family) => [{ name: `${text} ${family}`, family }],
}));

const imported: ImportedEffect<NativeImport> = {
  id: 'gr4-cinema-yellow',
  name: 'Cinema Yellow',
  icon: '',
  cover: { template: 'cine', brand: 'RICOH', name: 'CINEMA', number: 'YELLOW' },
  monochrome: false,
  preview: 'native/gr4-cinema-yellow/1',
  imported: {
    kind: 'native',
    version: '1.20',
    source: 'gr4_v120.zip',
    blob: new Uint8Array([1]),
    table: new Uint8Array([2]),
  },
};

function cardEffect(id: string, folder: string): CardEffect {
  return {
    id,
    name: id,
    icon: '',
    monochrome: false,
    card: { path: `${folder}/12345678.GRF`, bytes: new Uint8Array([3]) },
  };
}

describe.each([INBOX, 'GRKIT/EFFECTS'])('imports after connecting a card with effects in %s', (folder) => {
  it('uses the imported copy once in the list and both package modes', () => {
    const cinema = cardEffect(imported.id, folder);
    const other = cardEffect('lut-other', INBOX);
    const extra = [cinema, other];
    const available = [imported, ...extraCardEffects(extra, [imported])];

    expect(available).toEqual([imported, other]);
    expect(available.filter((effect) => effect.id === cinema.id)).toEqual([imported]);
    expect(packageEffects(available)).toEqual([imported, other]);
    expect(extra).toEqual([cinema, other]);
  });

  it('restores the card copy when the local import is removed', () => {
    const cinema = cardEffect(imported.id, folder);
    const extra = [cinema];

    expect(extraCardEffects(extra, [imported])).toEqual([]);
    expect(extraCardEffects(extra, [])).toEqual([cinema]);
    expect(packageEffects(extraCardEffects(extra, []))).toEqual(folder === INBOX ? [cinema] : []);
  });
});

describe('changedImports', () => {
  it('names the imports whose card file differs from the effect of the page', () => {
    const session = {
      contents: new Map([
        [imported.id, new Uint8Array([1])],
        ['lut-old', new Uint8Array([4, 5])],
      ]),
    } as unknown as CardSession;
    const renamed = { ...imported, id: 'lut-old', imported: { ...imported.imported, blob: new Uint8Array([4, 6]) } };
    const absent = { ...imported, id: 'lut-new' };
    expect([...changedImports(session, [imported, renamed, absent])]).toEqual(['lut-old']);
    // A session read before the page kept card files changes nothing.
    expect(changedImports({} as CardSession, [renamed]).size).toBe(0);
  });
});

const notFound = () => Object.assign(new Error('NotFoundError'), { name: 'NotFoundError' });

/** A card that holds only `files`, by their paths from the root. A write fails. */
function memoryCard(files: Record<string, string>, prefix = ''): CardDirectory {
  const paths = Object.keys(files).filter((path) => path.startsWith(prefix));
  const isFolder = (name: string) => paths.some((path) => path.startsWith(`${prefix}${name}/`));
  const file = (name: string): CardFileHandle => ({
    kind: 'file',
    name,
    getFile: async () => new Blob([files[`${prefix}${name}`]!]),
    createWritable: () => Promise.reject(new Error('read-only')),
  });
  return {
    kind: 'directory',
    name: prefix,
    getDirectoryHandle: async (name) => {
      if (!isFolder(name)) throw notFound();
      return memoryCard(files, `${prefix}${name}/`);
    },
    getFileHandle: async (name) => {
      if (!(`${prefix}${name}` in files)) throw notFound();
      return file(name);
    },
    removeEntry: () => Promise.reject(new Error('read-only')),
    async *entries() {
      for (const name of new Set(paths.map((path) => path.slice(prefix.length).split('/')[0]!))) {
        yield [name, isFolder(name) ? memoryCard(files, `${prefix}${name}/`) : file(name)];
      }
    },
  };
}

describe('openSession', () => {
  it('reads the recipes of the card for a camera line', async () => {
    const session = await openSession(memoryCard({ [RECIPE_FILE]: 'file' }), [], 'gr4');
    expect(session).toMatchObject({ family: 'gr4', recipes: [{ name: 'file gr4', family: 'gr4' }] });
    // One card fits GR IV and GR IV Monochrome: a change of camera reads the file again.
    expect(await withFamily(session, 'gr4Monochrome')).toMatchObject({
      family: 'gr4Monochrome',
      recipes: [{ name: 'file gr4Monochrome' }],
    });
    expect(await withFamily(session, 'gr4')).toBe(session);
  });

  it('has no recipes without a recipe file', async () => {
    const session = await openSession(memoryCard({ 'DCIM/100RICOH/R0000001.JPG': 'jpeg' }), [], 'gr3');
    expect(session.recipes).toBeNull();
    expect((await withFamily(session, 'gr4')).recipes).toBeNull();
  });
});
