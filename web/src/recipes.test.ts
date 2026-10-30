import { describe, expect, it } from 'vitest';
import { recipeChange, type CardSession } from './cardSession';
import {
  effectCode,
  familyOf,
  fromCard,
  libraryWarnings,
  LIBRARY_MAX,
  loadLibrary,
  nameFits,
  sameRecipes,
  saveLibrary,
  type LibraryRecipe,
  type Recipe,
} from './recipes';

const recipe = (name: string, change: Partial<Recipe> = {}): Recipe => ({
  name,
  family: 'gr3',
  look: { kind: 'off' },
  grain: null,
  whiteBalance: null,
  corrections: [null, null, null, null],
  hints: { exposure: null, isoLimit: null, metering: null, nd: null },
  ...change,
});

function memoryStorage(): Storage {
  const items = new Map<string, string>();
  return {
    get length() {
      return items.size;
    },
    clear: () => items.clear(),
    getItem: (key) => items.get(key) ?? null,
    key: (index) => [...items.keys()][index] ?? null,
    removeItem: (key) => void items.delete(key),
    setItem: (key, value) => void items.set(key, value),
  };
}

describe('recipes', () => {
  it('gives the effect code of the installer', () => {
    // grkit_core::recipes::effect_code of the same ID.
    expect(effectCode('portra-400-endura-v1')).toBe(16270);
  });

  it('maps each camera entry to its camera line', () => {
    expect(familyOf('gr3')).toBe('gr3');
    expect(familyOf('gr3x')).toBe('gr3');
    expect(familyOf('gr4')).toBe('gr4');
    expect(familyOf('gr4m')).toBe('gr4Monochrome');
  });

  it('accepts names that each camera font draws', () => {
    expect(nameFits('Street 400')).toBe(true);
    expect(nameFits('')).toBe(false);
    expect(nameFits('A name that is too long')).toBe(false);
    expect(nameFits('街拍')).toBe(false);
  });

  it('compares libraries by value, in order', () => {
    const a = recipe('A');
    // The JSON of the card has another key order.
    const { hints, ...rest } = a;
    const fromCard = JSON.parse(JSON.stringify({ hints, ...rest })) as Recipe;
    expect(sameRecipes([a], [fromCard])).toBe(true);
    expect(JSON.stringify(fromCard)).not.toBe(JSON.stringify(a));
    expect(sameRecipes([a, recipe('B')], [recipe('B'), a])).toBe(false);
  });

  it('keeps the library of each camera line in the browser', () => {
    const storage = memoryStorage();
    const library: LibraryRecipe[] = [{ recipe: recipe('A'), author: 'me', source: '' }];
    saveLibrary(storage, 'gr3', library);
    expect(loadLibrary(storage, 'gr3')).toEqual(library);
    expect(loadLibrary(storage, 'gr4')).toEqual([]);
    storage.setItem('grkit.recipes.gr4', '{broken');
    expect(loadLibrary(storage, 'gr4')).toEqual([]);
    saveLibrary(storage, 'gr3', Array(LIBRARY_MAX + 3).fill(library[0]));
    expect(loadLibrary(storage, 'gr3')).toHaveLength(LIBRARY_MAX);
  });

  it('keeps the credit of a recipe of the card by its name', () => {
    const kept: LibraryRecipe[] = [{ recipe: recipe('A'), author: 'me', source: 'https://example.com' }];
    expect(fromCard([recipe('A'), recipe('B')], kept)).toEqual([
      { recipe: recipe('A'), author: 'me', source: 'https://example.com' },
      { recipe: recipe('B'), author: '', source: '' },
    ]);
  });

  it('warns of Grain without the Grain module and of a missing effect', () => {
    const library = [
      recipe('Grain', { grain: { level: 2, size: 0 } }),
      recipe('No grain', { grain: { level: 0, size: 0 } }),
      recipe('Effect', { look: { kind: 'effect', code: 7, adjustments: Array(9).fill(4) } }),
    ];
    expect(libraryWarnings(library, false, new Map())).toEqual({ grain: ['Grain'], missing: ['Effect'] });
    expect(libraryWarnings(library, true, new Map([[7, 'seven']]))).toEqual({ grain: [], missing: [] });
  });

  it('plans the change of the library on the card', () => {
    const session = (recipes: Recipe[] | null) =>
      ({ state: { recipes: recipes ? new Uint8Array(1) : null }, recipes }) as unknown as CardSession;
    expect(recipeChange(session(null), null)).toBe('same');
    expect(recipeChange(session([recipe('A')]), null)).toBe('same');
    expect(recipeChange(session(null), [])).toBe('same');
    expect(recipeChange(session([recipe('A')]), [])).toBe('remove');
    expect(recipeChange(session([recipe('A')]), [recipe('A')])).toBe('same');
    expect(recipeChange(session([recipe('A')]), [recipe('B')])).toBe('write');
    expect(recipeChange(session(null), [recipe('B')])).toBe('write');
  });
});
