import { loadWasm } from './package';

/**
 * The recipes of the site (`grkit_core::recipes`): the presets that it offers, and the recipes of
 * the camera that it reads from the card or from a GRKit.zip. The site does not change the
 * settings of a recipe; the camera makes and edits recipes. The types are the JSON form of the
 * Rust model; the wasm build checks and encodes them.
 */

/** The camera line of a recipe. A recipe applies only on its own line. */
export type Family = 'gr3' | 'gr4' | 'gr4Monochrome';

export interface NativeLook {
  imageType: number;
  levels: number[];
  tone: number;
  intensity: number;
  filter: number;
  grainStrength: number;
  grainSize: number;
  solarization: number;
}

export type Look =
  | { kind: 'kept' }
  | { kind: 'off' }
  | { kind: 'effect'; code: number; adjustments: number[] }
  | ({ kind: 'native' } & NativeLook);

export interface Grain {
  /** 0 Off to 4 Extra Strong. */
  level: number;
  /** 0 Auto, then 1 to 3 small to large. */
  size: number;
}

/** Glow: Bloom and Halation 0 Off to 3 Strong, and the Halation color 0 Red, 1 Orange, 2 Gold,
 * 3 White, or null to keep the camera's. GR IV Monochrome has no color. */
export interface Glow {
  bloom: number;
  halation: number;
  halationColor: number | null;
}

export interface WhiteBalance {
  mode: number;
  /** Stored values 0 to 28; 14 is no shift. */
  shiftBa: number;
  shiftGm: number;
  kelvin: number;
}

export type Metering = 'multi' | 'center' | 'spot' | 'highlight';
export type Nd = 'off' | 'on' | 'auto';

/** The exposure settings that apply sets, each null to keep the camera setting. */
export interface Exposure {
  /** Exposure compensation in thirds of a stop. */
  compensation: number | null;
  /** The upper limit of Auto ISO. */
  isoLimit: number | null;
  metering: Metering | null;
  nd: Nd | null;
}

export interface Recipe {
  name: string;
  family: Family;
  look: Look;
  grain: Grain | null;
  glow: Glow | null;
  whiteBalance: WhiteBalance | null;
  /** The stored value of each correction setting, or null for Unset. */
  corrections: (number | null)[];
  exposure: Exposure;
  /**
   * The recipe is on: the camera lists it in MENU and, unless it is hidden, in ADJ. A recipe that
   * is off stays on the card for the site. At most `ON_MAX` recipes are on.
   */
  enabled: boolean;
  /** The recipe stays out of the rows of ADJ (the Options of the module on the camera). */
  hidden: boolean;
  /** The number after the automatic name of a recipe without a name, 0 for none. */
  number: number;
  /**
   * The ID of the recipe on the card, or 0 for a recipe that the card does not hold yet. The card
   * keeps the ID with the recipe, so the user modes of the mode dial keep their recipes.
   */
  id: number;
}

/**
 * The most recipes of a library and of the card, the most of them that are on
 * (`grkit_core::recipes`), and the most characters of a name.
 */
export const LIBRARY_MAX = 64;
export const ON_MAX = 40;
const NAME_MAX = 16;

/** A recipe that the site offers (`grkit_core::recipes::Preset`). */
export interface Preset {
  id: string;
  /** The ID of the effect of the look, which the package must hold. */
  effect: string | null;
  /** The person who made a community recipe and the post where they shared it, or null. */
  author: string | null;
  source: string | null;
  /** The recipe for each camera line that can hold it. */
  recipes: Recipe[];
}

/** The camera line of a camera entry of the site (`cameras.ts`). */
export function familyOf(cameraKey: string): Family {
  if (cameraKey === 'gr4m') return 'gr4Monochrome';
  return cameraKey.startsWith('gr4') ? 'gr4' : 'gr3';
}

/** The bytes of `RECIPES.BIN` for a library. */
export async function encodeLibrary(recipes: readonly Recipe[]): Promise<Uint8Array> {
  const wasm = await loadWasm();
  return wasm.encodeRecipes(JSON.stringify(recipes));
}

/** The recipes of the bytes of a `RECIPES.BIN`. Throws for a damaged library. */
export async function decodeLibrary(bytes: Uint8Array): Promise<Recipe[]> {
  const wasm = await loadWasm();
  return JSON.parse(wasm.decodeRecipes(bytes)) as Recipe[];
}

/** The recipes of the recipe file of the camera (`RECIPE_FILE`) for a camera line. */
export async function readRecipeFile(text: string, family: Family): Promise<Recipe[]> {
  const wasm = await loadWasm();
  try {
    return JSON.parse(wasm.readRecipeFile(text, family)) as Recipe[];
  } catch {
    return [];
  }
}

/**
 * The recipe file for `recipes` of the camera line `family`, which replaces `previous`: its text,
 * and the recipes in their order with the ID that the file gives each. It keeps the import record
 * and the user modes of the mode dial of `previous`, and each recipe of it that `readRecipeFile`
 * leaves out, so a write never deletes a recipe that the site cannot show.
 */
export async function writeRecipeFile(
  recipes: readonly Recipe[],
  family: Family,
  previous: string,
): Promise<{ text: string; recipes: Recipe[] }> {
  const wasm = await loadWasm();
  return JSON.parse(wasm.writeRecipeFile(JSON.stringify(recipes), family, previous)) as {
    text: string;
    recipes: Recipe[];
  };
}

/** The automatic name of a recipe: its look and its number, or "Recipe". */
export function automaticName(base: string, number: number): string {
  const suffix = number ? ` ${number}` : '';
  const ascii = [...base].filter((c) => c >= ' ' && c <= '~').join('').trim();
  const text = (ascii || 'Recipe').slice(0, NAME_MAX - suffix.length).trim();
  return `${text}${suffix}`;
}

const LIBRARY_KEY = (family: Family) => `grkit.recipes.${family}`;

/** The library of a family that this browser keeps, in the order of the camera, or an empty one. */
export function loadLibrary(storage: Pick<Storage, 'getItem'> | null, family: Family): Recipe[] {
  try {
    const value: unknown = JSON.parse(storage?.getItem(LIBRARY_KEY(family)) ?? '[]');
    if (!Array.isArray(value)) return [];
    const recipes = (value as Recipe[])
      .filter((recipe) => typeof recipe?.name === 'string' && recipe.family === family)
      .slice(0, LIBRARY_MAX);
    return cameraOrder(recipes);
  } catch {
    return [];
  }
}

export function saveLibrary(storage: Pick<Storage, 'setItem'> | null, family: Family, library: readonly Recipe[]): void {
  try {
    storage?.setItem(LIBRARY_KEY(family), JSON.stringify(library));
  } catch {
    // A full or blocked storage keeps the library for this visit only.
  }
}

/** JSON with sorted object keys, so that the order of the keys does not count. */
function canonical(value: unknown): string {
  if (Array.isArray(value)) return `[${value.map(canonical).join(',')}]`;
  if (value && typeof value === 'object') {
    const entries = Object.entries(value as Record<string, unknown>)
      .filter(([, item]) => item !== undefined)
      .sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0));
    return `{${entries.map(([key, item]) => `${JSON.stringify(key)}:${canonical(item)}`).join(',')}}`;
  }
  return JSON.stringify(value);
}

/** The count of recipes that are on. */
export function onCount(library: readonly Recipe[]): number {
  return library.filter((recipe) => recipe.enabled).length;
}

/**
 * The library in the order of the camera: the recipes that are on, at most `ON_MAX`, then those
 * that are off, each in their order. A recipe that is on past `ON_MAX` turns off, as INSTALL does.
 */
export function cameraOrder(library: readonly Recipe[]): Recipe[] {
  let on = 0;
  const recipes = library.map((recipe) => {
    if (!recipe.enabled) return recipe;
    on += 1;
    return on <= ON_MAX ? recipe : { ...recipe, enabled: false };
  });
  return [...recipes.filter((recipe) => recipe.enabled), ...recipes.filter((recipe) => !recipe.enabled)];
}

/**
 * The library of each camera line in a package library (`decodeLibrary`), in the order of the
 * camera. A record holds its camera line.
 */
export function packageLibraries(recipes: readonly Recipe[]): Partial<Record<Family, Recipe[]>> {
  const out: Partial<Record<Family, Recipe[]>> = {};
  for (const family of new Set(recipes.map((recipe) => recipe.family))) {
    out[family] = cameraOrder(recipes.filter((recipe) => recipe.family === family));
  }
  return out;
}

/**
 * Whether the person chooses between the recipes of a card and the library of the site: the
 * library has recipes, and they differ. Else the page takes the recipes of the card.
 */
export function cardNeedsChoice(library: readonly Recipe[], card: readonly Recipe[]): boolean {
  return library.length > 0 && !sameRecipes(library, card);
}

/** The settings of a recipe, without its place on the camera: on or off, ADJ, its number, and its ID. */
const settings = (recipe: Recipe) => canonical({ ...recipe, enabled: true, hidden: false, number: 0, id: 0 });

/** Whether the library holds a recipe with the settings and the name of `recipe`. */
export function holdsRecipe(library: readonly Recipe[], recipe: Recipe): boolean {
  const key = settings(recipe);
  return library.some((item) => settings(item) === key);
}

/** The preset that `recipe` is, by its settings and its name, or undefined. */
export function presetOf(presets: readonly Preset[], recipe: Recipe): Preset | undefined {
  const key = settings(recipe);
  return presets.find((preset) => preset.recipes.some((item) => settings(item) === key));
}

/**
 * The recipes of the presets for the camera line `family` that the library does not hold. The site
 * lists them after the recipes of the library, off, until the person turns one on.
 */
export function presetRows(
  presets: readonly Preset[],
  family: Family,
  library: readonly Recipe[],
): { preset: Preset; recipe: Recipe }[] {
  return presets.flatMap((preset) => {
    const recipe = preset.recipes.find((item) => item.family === family);
    return recipe && !holdsRecipe(library, recipe) ? [{ preset, recipe: { ...recipe, enabled: false } }] : [];
  });
}

/**
 * The library with a preset recipe after the others, on while fewer than `ON_MAX` are on, and
 * shown in ADJ. The camera chooses the recipes that ADJ shows.
 */
export function withPreset(library: readonly Recipe[], recipe: Recipe): Recipe[] {
  return cameraOrder([...library, { ...recipe, enabled: onCount(library) < ON_MAX, hidden: false, number: 0, id: 0 }]);
}

/** Whether two libraries hold the same recipes in the same order. */
export function sameRecipes(a: readonly Recipe[], b: readonly Recipe[]): boolean {
  return a.length === b.length && canonical(a) === canonical(b);
}

/**
 * The code of an effect in the records and in recipes: the low 15 bits of the FNV-1a hash of its
 * ID (`grkit_core::recipes::effect_code`).
 */
export function effectCode(id: string): number {
  let hash = 0x811c9dc5;
  for (const byte of new TextEncoder().encode(id)) {
    hash ^= byte;
    hash = Math.imul(hash, 0x01000193) >>> 0;
  }
  return hash & 0x7fff;
}

/**
 * The warnings of a library for a package: the recipes whose Grain has no effect without the
 * Grain module, those whose Glow the camera skips without the Glow module, and the effect recipes
 * whose effect the package does not have. `codes` holds the recipe code of each effect of the
 * package.
 */
export function libraryWarnings(
  library: readonly Recipe[],
  modules: { grain: boolean; glow: boolean },
  codes: ReadonlySet<number>,
): { grain: string[]; glow: string[]; missing: string[] } {
  return {
    grain: modules.grain
      ? []
      : library.filter((recipe) => recipe.grain && recipe.grain.level > 0).map((recipe) => recipe.name),
    // Glow Off changes nothing without the module, so only a Glow that is on is skipped.
    glow: modules.glow
      ? []
      : library
          .filter((recipe) => recipe.glow && (recipe.glow.bloom > 0 || recipe.glow.halation > 0))
          .map((recipe) => recipe.name),
    missing: library
      .filter((recipe) => recipe.look.kind === 'effect' && !codes.has(recipe.look.code))
      .map((recipe) => recipe.name),
  };
}
