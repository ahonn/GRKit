import { describe, expect, it } from 'vitest';
import { INBOX } from './card';
import { changedImports, extraCardEffects, packageEffects, type CardSession } from './cardSession';
import type { CardEffect, ImportedEffect, NativeImport } from './catalog';

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
