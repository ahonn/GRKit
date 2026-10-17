/** One effect of `effects/<target>/catalog.json`, which `grkit effects publish` writes. */
export interface Effect {
  id: string;
  name: string;
  label: string;
  shape: 'film' | 'filter' | 'plain';
  monochrome: boolean;
  look: string;
  origin: string;
  license: string;
  /** The `.GRF` filename, the same in the site and in the package. */
  file: string;
  sha256: string;
  /** The preview table path, relative to the catalog. */
  preview: string;
}

export interface Catalog {
  target: string;
  lutSize: number;
  effects: readonly Effect[];
}

const SCHEMA = 'grkit-site-effects-v1';
/** The effects that one package can hold (`runtime::MAX_COUNT`). */
export const MAX_EFFECTS = 20;

export function siteUrl(path: string): string {
  return new URL(path, document.baseURI).href;
}

export function effectUrl(target: string, path: string): string {
  return siteUrl(`effects/${target}/${path}`);
}

async function fetchCatalog(target: string): Promise<Catalog> {
  const response = await fetch(effectUrl(target, 'catalog.json'), { cache: 'no-cache' });
  if (!response.ok) {
    throw new Error(`Catalog request failed: ${response.status}`);
  }
  const data = await response.json();
  if (data?.schema !== SCHEMA || data.target !== target || !Array.isArray(data.effects)) {
    throw new Error('Unexpected catalog format');
  }
  return { target, lutSize: data.preview.size, effects: data.effects };
}

const catalogs = new Map<string, Promise<Catalog>>();

/** One cached request for each target, so `use()` receives the same promise on each render. */
export function loadCatalog(target: string): Promise<Catalog> {
  let catalog = catalogs.get(target);
  if (!catalog) {
    catalog = fetchCatalog(target);
    catalogs.set(target, catalog);
  }
  return catalog;
}
