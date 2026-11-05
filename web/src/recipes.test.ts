import { describe, expect, it } from 'vitest';
import { PRESETS } from './generated/recipes';
import { recipeChange, type CardSession } from './cardSession';
import {
  effectCode,
  familyOf,
  libraryWarnings,
  namedRecipes,
  LIBRARY_MAX,
  loadLibrary,
  cameraOrder,
  cardNeedsChoice,
  holdsRecipe,
  presetOf,
  presetRows,
  withPreset,
  ON_MAX,
  packageLibraries,
  packageRecipes,
  sameRecipes,
  saveLibrary,
  type Recipe,
} from './recipes';

const recipe = (name: string, change: Partial<Recipe> = {}): Recipe => ({
  name,
  family: 'gr3',
  look: { kind: 'off' },
  grain: null,
  glow: null,
  whiteBalance: null,
  corrections: [null, null, null, null],
  exposure: { compensation: null, isoLimit: null, metering: null, nd: null },
  enabled: true,
  hidden: false,
  number: 0,
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

  it('compares libraries by value, in order', () => {
    const a = recipe('A');
    // The JSON of the card has another key order.
    const { exposure, ...rest } = a;
    const fromCard = JSON.parse(JSON.stringify({ exposure, ...rest })) as Recipe;
    expect(sameRecipes([a], [fromCard])).toBe(true);
    expect(JSON.stringify(fromCard)).not.toBe(JSON.stringify(a));
    expect(sameRecipes([a, recipe('B')], [recipe('B'), a])).toBe(false);
  });

  it('keeps the library of each camera line in the browser', () => {
    const storage = memoryStorage();
    const library = [recipe('A')];
    saveLibrary(storage, 'gr3', library);
    expect(loadLibrary(storage, 'gr3')).toEqual(library);
    expect(loadLibrary(storage, 'gr4')).toEqual([]);
    storage.setItem('grkit.recipes.gr4', '{broken');
    expect(loadLibrary(storage, 'gr4')).toEqual([]);
    saveLibrary(storage, 'gr3', Array(LIBRARY_MAX + 3).fill(library[0]));
    expect(loadLibrary(storage, 'gr3')).toHaveLength(LIBRARY_MAX);
  });

  it('names the recipes with the automatic name of the camera for a package', () => {
    const named = namedRecipes(
      [recipe('', { number: 2 }), recipe('Mine'), recipe('', { number: 0 })],
      (item) => (item.number === 2 ? 'Portra 400' : '负片'),
    );
    expect(named.map((item) => item.name)).toEqual(['Portra 400 2', 'Mine', 'Recipe']);
  });

  it('keeps the recipes that are on first, at most ON_MAX', () => {
    const item = (name: string, enabled: boolean) => recipe(name, { enabled });
    const order = cameraOrder([item('A', false), item('B', true), item('C', false), item('D', true)]);
    expect(order.map((entry) => entry.name)).toEqual(['B', 'D', 'A', 'C']);
    // Past ON_MAX, a recipe that is on turns off and moves after the others that are on.
    const many = Array.from({ length: ON_MAX + 2 }, (_, index) => item(`R${index}`, true));
    const capped = cameraOrder(many);
    expect(capped.filter((entry) => entry.enabled)).toHaveLength(ON_MAX);
    expect(capped.at(-1)).toMatchObject({ name: `R${ON_MAX + 1}`, enabled: false });
  });

  it('restores the whole library of each camera line from a package', () => {
    const off = recipe('Off', { family: 'gr4', enabled: false });
    const color = recipe('Color', { family: 'gr4' });
    const mono = recipe('Mono', { family: 'gr4Monochrome' });
    expect(packageLibraries([off, color, mono])).toEqual({ gr4: [color, off], gr4Monochrome: [mono] });
  });

  it('adds a preset after the recipes that are on, on while there is room', () => {
    const preset = recipe('Preset', { grain: { level: 2, size: 0 } });
    const off = recipe('Off', { enabled: false });
    const added = withPreset([recipe('A'), off], { ...preset, hidden: true, number: 3 });
    expect(added.map((item) => item.name)).toEqual(['A', 'Preset', 'Off']);
    expect(added[1]).toEqual(preset);
    // The library holds it whatever its place on the camera.
    expect(holdsRecipe([{ ...preset, enabled: false, number: 2 }], preset)).toBe(true);
    expect(holdsRecipe([recipe('Preset')], preset)).toBe(false);
    const full = Array.from({ length: ON_MAX }, (_, index) => recipe(`R${index}`));
    expect(withPreset(full, preset).at(-1)).toEqual({ ...preset, enabled: false });
  });

  it('lists the presets that the library does not hold, off, and finds the preset of a recipe', () => {
    const preset = { id: 'p', effect: null, author: null, source: null, recipes: [recipe('P', { grain: { level: 1, size: 0 } })] };
    const rows = presetRows([preset], 'gr3', []);
    expect(rows).toEqual([{ preset, recipe: { ...preset.recipes[0], enabled: false } }]);
    expect(presetRows([preset], 'gr4', [])).toEqual([]);
    // Once the library holds it, on or off, the preset is a recipe of the library.
    const library = withPreset([], preset.recipes[0]!);
    expect(presetRows([preset], 'gr3', library)).toEqual([]);
    expect(presetOf([preset], { ...library[0]!, enabled: false })).toBe(preset);
    expect(presetOf([preset], recipe('Other'))).toBeUndefined();
  });

  it('has a recipe of a built-in effect for each camera line of each preset', () => {
    for (const preset of PRESETS) {
      expect(new Set(preset.recipes.map((item) => item.family)).size).toBe(preset.recipes.length);
      for (const item of preset.recipes) {
        if (item.look.kind === 'effect') expect(item.look.code).toBe(effectCode(preset.effect!));
      }
    }
  });

  it('asks before the recipes of a card replace other recipes of the site', () => {
    const card = [recipe('A'), recipe('B')];
    expect(cardNeedsChoice([], card)).toBe(false);
    expect(cardNeedsChoice([recipe('A'), recipe('B')], card)).toBe(false);
    expect(cardNeedsChoice([recipe('A')], card)).toBe(true);
    expect(cardNeedsChoice([recipe('A')], [])).toBe(true);
  });

  it('reads a stored library of an earlier version of the site', () => {
    const items = new Map<string, string>();
    const { enabled: _, ...old } = recipe('Old');
    // Each recipe was in { recipe, author, source } and had no on or off.
    items.set('grkit.recipes.gr3', JSON.stringify([{ recipe: old, author: '', source: '' }]));
    const storage = { getItem: (key: string) => items.get(key) ?? null };
    expect(loadLibrary(storage, 'gr3')).toEqual([recipe('Old')]);
  });

  it('names the recipes without a name for a package', () => {
    expect(packageRecipes([recipe('', { number: 2 })]).map((item) => item.name)).toEqual(['Recipe 2']);
  });

  it('warns of Grain and Glow without their modules and of a missing effect', () => {
    const library = [
      recipe('Grain', { grain: { level: 2, size: 0 } }),
      recipe('No grain', { grain: { level: 0, size: 0 } }),
      recipe('Glow', { glow: { bloom: 0, halation: 2, halationColor: 1 } }),
      recipe('Glow off', { glow: { bloom: 0, halation: 0, halationColor: null } }),
      recipe('Effect', { look: { kind: 'effect', code: 7, adjustments: Array(9).fill(4) } }),
    ];
    expect(libraryWarnings(library, { grain: false, glow: false }, new Set())).toEqual({
      grain: ['Grain'],
      glow: ['Glow'],
      missing: ['Effect'],
    });
    expect(libraryWarnings(library, { grain: true, glow: true }, new Set([7]))).toEqual({
      grain: [],
      glow: [],
      missing: [],
    });
  });

  it('plans the change of the recipe file on the card', () => {
    const session = (recipes: Recipe[] | null) =>
      ({ state: { recipeText: recipes ? 'text' : null }, recipes }) as unknown as CardSession;
    expect(recipeChange(session(null), null)).toBe('same');
    expect(recipeChange(session([recipe('A')]), null)).toBe('same');
    expect(recipeChange(session(null), [])).toBe('same');
    expect(recipeChange(session([recipe('A')]), [])).toBe('write');
    expect(recipeChange(session([recipe('A')]), [recipe('A', { hidden: true })])).toBe('write');
    expect(recipeChange(session([recipe('A')]), [recipe('A')])).toBe('same');
    expect(recipeChange(session([recipe('A')]), [recipe('B')])).toBe('write');
    expect(recipeChange(session(null), [recipe('B')])).toBe('write');
  });
});
