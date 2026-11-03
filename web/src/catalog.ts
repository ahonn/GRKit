import { CATALOG_SCHEMA } from './generated/site';
import type { Input } from './import/names';

export { MAX_EFFECTS } from './generated/site';

/** The fields of a film cover icon, as `effect.json` gives them (`effects/README.md`, Icons). */
export interface Cover {
  template: 'box' | 'cine';
  brand: string;
  name: string;
  /** Missing when the cover shows a large name instead of a number. */
  number?: string;
  /** A light band at the bottom of a film box. */
  band?: boolean;
}

/** The fields that the page shows for each effect. */
interface EffectFields {
  id: string;
  name: string;
  /** The ADJ icon: 40 x 40 RGBA pixels, base64-encoded. */
  icon: string;
  /** The cover fields of the icon. Catalogs of releases before 1.0.1 do not have them. */
  cover?: Cover;
  monochrome: boolean;
}

/** One effect of `effects/<target>/catalog.json`, which `grkit effects publish` writes. */
export interface CatalogEffect extends EffectFields {
  look: string;
  origin: string;
  license: string;
  /** The `.GRF` filename, the same in the site and in the package. */
  file: string;
  sha256: string;
  /** The preview table path, relative to the catalog. */
  preview: string;
  imported?: undefined;
  card?: undefined;
}

/**
 * An effect that this page made in the browser: a fit of a LUT, or a native look of a firmware
 * file that the user chose. It has no site files.
 */
export interface ImportedEffect<I extends Imported = Imported> extends EffectFields {
  cover: Cover;
  /** The key of the preview table in the preview cache (`provideTable`). */
  preview: string;
  imported: I;
  card?: undefined;
}

/**
 * An effect file on the connected card that the catalog and the imports do not have, such as an
 * import from an earlier visit. The page shows it without a preview.
 */
export interface CardEffect extends EffectFields {
  imported?: undefined;
  card: {
    /** The path of the file from the card root. */
    path: string;
    bytes: Uint8Array;
  };
}

/** An effect that a package can hold. */
export type PackageEffect = Effect | CardEffect;

/** `imported` tells the two kinds apart. */
export type Effect = CatalogEffect | ImportedEffect;

export const GRADES = ['A', 'B', 'C', 'D'] as const;
/** How closely an imported effect follows its LUT, from A (nearly the same) to D. */
export type Grade = (typeof GRADES)[number];

/** The data of an imported effect that the package and the preview use. */
interface ImportData {
  /** The compiled `.GRF` effect, which the package takes as it is. */
  blob: Uint8Array;
  /** The RAW preview table, in the format of the catalog tables. */
  table: Uint8Array;
}

/** What `importLut` returns for one LUT, besides the effect fields. */
export interface LutImport extends ImportData {
  kind: 'lut';
  grade: Grade;
  /** The input encoding that the fit used: the user's choice or the guess. */
  input: Input;
  /** Conditions to check before using the effect, in English, as grkit-wasm gives them. */
  warnings: readonly string[];
  /** The LUT file name. */
  source: string;
}

/**
 * A look that GR IV has built in, which `nativeLooks` reads from the firmware file of the user.
 * Its data follows the Standard of GR III, so only GR III and GR IIIx packages take it.
 */
export interface NativeImport extends ImportData {
  kind: 'native';
  /** The GR IV version of the firmware file, such as `1.20`. */
  version: string;
  /** The firmware file name. */
  source: string;
}

/** `kind` tells the two sources apart. */
export type Imported = LutImport | NativeImport;

export interface Catalog {
  target: string;
  lutSize: number;
  baseline: string;
  /**
   * The Standard base gamma of the target's firmware, over which the page fits imported LUTs.
   * Catalogs before 1.1.0 do not have it, and the page then offers no import.
   */
  baseGamma?: Uint16Array;
  effects: readonly CatalogEffect[];
}

/** The `base_gamma` array of a catalog: 256 integers from 0 to 16384. */
export function readBaseGamma(value: unknown): Uint16Array | undefined {
  if (
    !Array.isArray(value) ||
    value.length !== 256 ||
    !value.every((item) => Number.isInteger(item) && item >= 0 && item <= 16384)
  ) {
    return undefined;
  }
  return Uint16Array.from(value as number[]);
}


export function siteUrl(path: string): string {
  return new URL(path, document.baseURI).href;
}

export function effectUrl(target: string, path: string): string {
  return siteUrl(`effects/${target}/${path}`);
}

/** The preview table of `effect`: a catalog URL, or the cache key of an imported table. */
export function previewTable(target: string, effect: Effect): string {
  return effect.imported ? effect.preview : effectUrl(target, effect.preview);
}

async function fetchCatalog(target: string): Promise<Catalog> {
  const response = await fetch(effectUrl(target, 'catalog.json'), { cache: 'no-cache' });
  if (!response.ok) {
    throw new Error(`Catalog request failed: ${response.status}`);
  }
  const data = await response.json();
  if (
    data?.schema !== CATALOG_SCHEMA ||
    data.target !== target ||
    !Array.isArray(data.effects) ||
    data.preview?.input !== 'linear-srgb16' ||
    data.preview.shaper !== 'srgb' ||
    data.preview.order !== 'rgb' ||
    !Number.isInteger(data.preview.size) ||
    data.preview.size < 2 ||
    data.preview.size > 256 ||
    typeof data.preview.baseline !== 'string'
  ) {
    throw new Error('Unexpected catalog format');
  }
  return {
    target,
    lutSize: data.preview.size,
    baseline: data.preview.baseline,
    baseGamma: readBaseGamma(data.base_gamma),
    effects: data.effects,
  };
}

const catalogs = new Map<string, Promise<Catalog>>();

/**
 * One cached request for each target, so `use()` receives the same promise on each render.
 * A failed request leaves the cache, so the next call requests the catalog again.
 */
export function loadCatalog(target: string): Promise<Catalog> {
  const cached = catalogs.get(target);
  if (cached) {
    return cached;
  }
  const catalog = fetchCatalog(target);
  catalogs.set(target, catalog);
  catalog.catch(() => {
    if (catalogs.get(target) === catalog) {
      catalogs.delete(target);
    }
  });
  return catalog;
}
