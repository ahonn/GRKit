import { loadWasm } from './package';

/**
 * The recipe library that the site writes to the card as `GRKIT/RECIPES.BIN`
 * (`grkit_core::recipes`). The camera lists these recipes in ADJ before the recipes that it saves.
 * The types are the JSON form of the Rust model; the wasm build checks and encodes them.
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

export interface WhiteBalance {
  mode: number;
  /** Stored values 0 to 28; 14 is no shift. */
  shiftBa: number;
  shiftGm: number;
  kelvin: number;
}

export type Metering = 'multi' | 'center' | 'spot' | 'highlight';
export type Nd = 'off' | 'on' | 'auto';

export interface Hints {
  /** Exposure compensation in thirds of a stop. */
  exposure: number | null;
  isoLimit: number | null;
  metering: Metering | null;
  nd: Nd | null;
}

export interface Recipe {
  name: string;
  family: Family;
  look: Look;
  grain: Grain | null;
  whiteBalance: WhiteBalance | null;
  /** The stored value of each correction setting, or null for Unset. */
  corrections: (number | null)[];
  hints: Hints;
}

/** A recipe of the site, with the credit that stays on the site. */
export interface LibraryRecipe {
  recipe: Recipe;
  author: string;
  source: string;
  /**
   * The values that an import or a copy to another camera could not keep, which the person checks
   * (`recipeText.Flag`). A save in the editor clears them.
   */
  flags?: { field: string; text: string }[];
}

export interface Choice {
  value: number;
  label: string;
}

export interface ModelParameter {
  key: string;
  choices: Choice[];
}

export interface ModelType {
  index: number;
  label: string;
  /** The camera name of the type in the site language. */
  name: string;
  parameters: ModelParameter[];
  grain: boolean;
  grainSizes: boolean;
}

export interface ModelMode {
  index: number;
  label: string;
  shifts: boolean;
  temperature: boolean;
}

/** The values that a recipe of a family can hold (`recipes::Model`). */
export interface Model {
  types: ModelType[];
  modes: ModelMode[];
  corrections: ModelParameter[];
}

/** The most recipes of a library, and the most characters of a name. */
export const LIBRARY_MAX = 24;
export const NAME_MAX = 16;
/** The nine levels, M4 to P4: stored 0 to 8, and 4 is neutral. */
export const NEUTRAL = 4;
/** No white balance shift. */
export const NO_SHIFT = 14;
/** The ISO limits that the form offers. */
export const ISO_LIMITS = [
  200, 400, 800, 1600, 3200, 6400, 12800, 25600, 51200, 102400, 204800,
] as const;

/** The camera line of a camera entry of the site (`cameras.ts`). */
export function familyOf(cameraKey: string): Family {
  if (cameraKey === 'gr4m') return 'gr4Monochrome';
  return cameraKey.startsWith('gr4') ? 'gr4' : 'gr3';
}

/** A name that the camera draws in each menu language: 1 to 16 printable ASCII characters. */
export function nameFits(name: string): boolean {
  return name.length >= 1 && name.length <= NAME_MAX && /^[\x20-\x7e]+$/.test(name);
}

/** The model of a family with the camera names of the types in a site language. */
export function modelFor(models: Readonly<Record<Family, Model>>, names: readonly string[] | undefined, family: Family): Model {
  const model = models[family];
  return names ? { ...model, types: model.types.map((type) => ({ ...type, name: names[type.index] ?? type.name })) } : model;
}

const models = new Map<string, Promise<Model>>();

/**
 * The model of a family with the type names of a site language (`generated/recipes.ts`), loaded
 * once: the tables load with the Recipes section.
 */
export function loadModel(family: Family, language: string): Promise<Model> {
  const key = `${family}/${language}`;
  let model = models.get(key);
  if (!model) {
    model = import('./generated/recipes').then(({ RECIPE_MODELS, TYPE_NAMES }) =>
      modelFor(RECIPE_MODELS, TYPE_NAMES[language], family),
    );
    model.catch(() => models.delete(key));
    models.set(key, model);
  }
  return model;
}

/** The camera names of the types in each site language, loaded once. */
let typeNames: Promise<readonly (readonly string[])[]> | undefined;
export function loadTypeNames(): Promise<readonly (readonly string[])[]> {
  typeNames ??= import('./generated/recipes').then(({ TYPE_NAMES }) => Object.values(TYPE_NAMES));
  typeNames.catch(() => (typeNames = undefined));
  return typeNames;
}

/** The camera lines and their names. */
export const FAMILIES: readonly { family: Family; name: string }[] = [
  { family: 'gr3', name: 'GR III / GR IIIx' },
  { family: 'gr4', name: 'GR IV' },
  { family: 'gr4Monochrome', name: 'GR IV Monochrome' },
];

/** Why the camera cannot hold a recipe, or null. */
export async function recipeProblem(recipe: Recipe): Promise<string | null> {
  const wasm = await loadWasm();
  return wasm.recipeProblem(JSON.stringify(recipe)) ?? null;
}

/** The bytes of `RECIPES.BIN` for a library. */
export async function encodeLibrary(recipes: readonly Recipe[]): Promise<Uint8Array> {
  const wasm = await loadWasm();
  return wasm.encodeRecipes(JSON.stringify(recipes));
}

/** The recipes of a `RECIPES.BIN`, or null for a damaged file. */
export async function decodeLibrary(bytes: Uint8Array): Promise<Recipe[] | null> {
  const wasm = await loadWasm();
  try {
    return JSON.parse(wasm.decodeRecipes(bytes)) as Recipe[];
  } catch {
    return null;
  }
}


/** A new recipe of a family: the first native type with neutral values. */
export function newRecipe(family: Family, model: Model, name: string): Recipe {
  return {
    name,
    family,
    look: nativeLook(model.types[0]!),
    grain: null,
    whiteBalance: null,
    corrections: [null, null, null, null],
    hints: { exposure: null, isoLimit: null, metering: null, nd: null },
  };
}

/** The neutral look of a native type: each level 4, each other value its first choice. */
export function nativeLook(type: ModelType): Look {
  const first = (key: string) => type.parameters.find((p) => p.key === key)?.choices[0]?.value ?? 0;
  const tone = type.parameters.find((p) => p.key.startsWith('ColorTone'));
  return {
    kind: 'native',
    imageType: type.index,
    levels: Array(9).fill(NEUTRAL),
    tone: tone ? (tone.choices[0]?.value ?? 0) : 0,
    intensity: first('HdrIntense'),
    filter: first('FilterEffectCustom'),
    grainStrength: 0,
    grainSize: 0,
    solarization: first('SolarizationType'),
  };
}

const LIBRARY_KEY = (family: Family) => `grkit.recipes.${family}`;

/** The library of a family that this browser keeps, or an empty one. */
export function loadLibrary(storage: Pick<Storage, 'getItem'> | null, family: Family): LibraryRecipe[] {
  try {
    const value: unknown = JSON.parse(storage?.getItem(LIBRARY_KEY(family)) ?? '[]');
    if (!Array.isArray(value)) return [];
    return value
      .filter(
        (item): item is LibraryRecipe =>
          typeof item === 'object' && item !== null && typeof item.recipe?.name === 'string',
      )
      .filter((item) => item.recipe.family === family)
      .slice(0, LIBRARY_MAX);
  } catch {
    return [];
  }
}

export function saveLibrary(
  storage: Pick<Storage, 'setItem'> | null,
  family: Family,
  library: readonly LibraryRecipe[],
): void {
  try {
    storage?.setItem(LIBRARY_KEY(family), JSON.stringify(library));
  } catch {
    // A full or blocked storage keeps the library for this visit only.
  }
}

/**
 * The library of a card: its recipes, with the credit of a kept recipe of the same name. The card
 * holds no credit.
 */
export function fromCard(recipes: readonly Recipe[], kept: readonly LibraryRecipe[]): LibraryRecipe[] {
  return recipes.map((recipe) => {
    const same = kept.find((item) => item.recipe.name === recipe.name);
    return { recipe, author: same?.author ?? '', source: same?.source ?? '' };
  });
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
 * Grain module, and the effect recipes whose effect the package does not have. `codes` maps the
 * code of each effect of the package to its ID.
 */
export function libraryWarnings(
  library: readonly Recipe[],
  grainModule: boolean,
  codes: ReadonlyMap<number, string>,
): { grain: string[]; missing: string[] } {
  return {
    grain: grainModule
      ? []
      : library.filter((recipe) => recipe.grain && recipe.grain.level > 0).map((recipe) => recipe.name),
    missing: library
      .filter((recipe) => recipe.look.kind === 'effect' && !codes.has(recipe.look.code))
      .map((recipe) => recipe.name),
  };
}
