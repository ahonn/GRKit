import type { CatalogEffect, Cover } from './catalog';
import { lookFromColors, resolveColors } from './coverPalette';
import { readCover, type LibraryDetails } from './import/library';
import { coverIcon, validName, type CoverFields } from './import/names';
import {
  filmBoxCover,
  grayLookCover,
  individualCover,
  isFilmCover,
  sameCover,
  schemeCover,
  COVER_SCHEMES,
  type CoverScheme,
  type Customization,
  type Customizations,
} from './packageCovers';

// The names and covers that the user gave site film looks. This browser keeps them. A write bakes
// them into the effect files, and each package keeps them in the import library (`kind: 'cover'`),
// so another browser gets them back from the card, also after a newer site file of the look. A
// site film look without a custom cover has the cover of the scheme (`CoverScheme`): this browser
// keeps the scheme, and a card or a ZIP tells it by its effect files (`schemeOnCard`).

const CUSTOM_COVERS_KEY = 'grkit.customCovers';
const COVER_SCHEME_KEY = 'grkit.coverScheme';
/** More than the catalog has, so a damaged value cannot grow without bound. */
const MAX_SAVED = 256;

/** A custom cover has its colors: without them it would be the gray cover. */
const readCustom = (value: unknown): Cover | undefined => {
  const cover = readCover(value);
  return cover?.colors ? cover : undefined;
};

/**
 * `value` as a customization, or undefined. Pages before custom names kept the cover itself as
 * the value, which still reads as a customization with that cover.
 */
function readCustomization(value: unknown): Customization | undefined {
  if (typeof value !== 'object' || value === null) return undefined;
  if ('template' in value) {
    const cover = readCustom(value);
    return cover && { cover };
  }
  const { name, cover: stored } = value as { name?: unknown; cover?: unknown };
  const cover = readCustom(stored);
  const read: Customization = {
    ...(typeof name === 'string' && validName(name) && { name }),
    ...(cover && { cover }),
  };
  return read.name !== undefined || read.cover ? read : undefined;
}

/** The customizations that this browser keeps, or none. */
export function loadCustomizations(storage: Pick<Storage, 'getItem'> | null): Customizations {
  const customs = new Map<string, Customization>();
  try {
    const value: unknown = JSON.parse(storage?.getItem(CUSTOM_COVERS_KEY) ?? 'null');
    if (typeof value !== 'object' || value === null || Array.isArray(value)) return customs;
    for (const [id, item] of Object.entries(value).slice(0, MAX_SAVED)) {
      const custom = readCustomization(item);
      if (custom) customs.set(id, custom);
    }
  } catch {
    // An unreadable value is the same as none.
  }
  return customs;
}

export function saveCustomizations(storage: Pick<Storage, 'setItem'> | null, customs: Customizations): void {
  try {
    storage?.setItem(CUSTOM_COVERS_KEY, JSON.stringify(Object.fromEntries(customs)));
  } catch {
    // Without storage, the customizations last until the page closes. A write still keeps them.
  }
}

/** The cover scheme that this browser keeps, or null before the first one. */
function storedScheme(storage: Pick<Storage, 'getItem'> | null): CoverScheme | null {
  try {
    const value = storage?.getItem(COVER_SCHEME_KEY);
    return COVER_SCHEMES.find((scheme) => scheme === value) ?? null;
  } catch {
    return null;
  }
}

/**
 * The cover scheme of a new visitor: film box colors for the color looks, gray for B&W. A card
 * that the page reads tells the scheme of a person who installed GRKit before.
 */
export const DEFAULT_SCHEME: CoverScheme = 'colorFilmBox';

/** The cover scheme that this browser keeps, or `DEFAULT_SCHEME` without one. */
export function loadCoverScheme(storage: Pick<Storage, 'getItem'> | null): CoverScheme {
  return storedScheme(storage) ?? DEFAULT_SCHEME;
}

export function saveCoverScheme(storage: Pick<Storage, 'setItem'> | null, scheme: CoverScheme): void {
  try {
    storage?.setItem(COVER_SCHEME_KEY, scheme);
  } catch {
    // Without storage, the scheme lasts until the page closes. A write still keeps it on the card.
  }
}

/**
 * `customs` with `custom` for `id`. A customization without a name and a cover, which leaves the
 * look as the site has it, is removed.
 */
export function withCustomization(customs: Customizations, id: string, custom: Customization): Customizations {
  const next = new Map(customs);
  const kept: Customization = {
    ...(custom.name !== undefined && { name: custom.name }),
    ...(custom.cover && { cover: custom.cover }),
  };
  if (kept.name !== undefined || kept.cover) next.set(id, kept);
  else next.delete(id);
  return next;
}

/** The customization that the dialog makes for `effect`: a name equal to its own is none. */
export function customizationOf(effect: CatalogEffect, name: string, cover: Cover | null): Customization {
  return {
    ...(name !== effect.name && { name }),
    ...(cover && { cover }),
  };
}

/** The catalog with the custom name of each site look that has one. */
export function withCustomNames<T extends { id: string; name: string }>(
  effects: readonly T[],
  customs: Customizations,
): T[] {
  return effects.map((effect) => {
    const name = customs.get(effect.id)?.name;
    return name === undefined ? effect : { ...effect, name };
  });
}

/**
 * `customs` without the covers that are no individual covers of their site film look
 * (`individualCover`): the gray and the film box cover follow the scheme. The names stay.
 */
export function withIndividualCovers(catalog: readonly CatalogEffect[], customs: Customizations): Customizations {
  let next = customs;
  for (const { id, cover: base } of catalog) {
    const own = customs.get(id);
    if (own?.cover && !individualCover(base, own.cover))
      next = withCustomization(next, id, { ...own, cover: undefined });
  }
  return next;
}

/**
 * The scheme and the customizations of this browser after it read the catalog. The film box colors
 * switch before the scheme saved the film box cover of each site film look as its custom cover, so
 * a browser without a scheme gets `filmBox` when each site film look with a custom cover has
 * exactly its film box cover, else `gray`. Covers that are no individual covers go either way.
 */
export function migratedScheme(
  catalog: readonly CatalogEffect[],
  customs: Customizations,
): { scheme: CoverScheme; customs: Customizations } {
  const covered = catalog.filter((effect) => isFilmCover(effect.cover) && customs.get(effect.id)?.cover);
  const boxed =
    covered.length > 0 &&
    covered.every(
      (effect) =>
        schemeCover(effect.cover, 'filmBox') !== undefined &&
        sameCover(customs.get(effect.id)!.cover!, filmBoxCover(effect.cover!)),
    );
  return { scheme: boxed ? 'filmBox' : 'gray', customs: withIndividualCovers(catalog, customs) };
}

/**
 * Brings the state of this browser to the rule of individual covers at the start of a visit
 * (`migratedScheme`): a browser without a scheme gets one, and covers that follow the scheme go.
 * It needs the catalog only when this browser has custom covers. A failure leaves the state for
 * the next visit.
 */
export async function migrateCoverScheme(
  storage: Pick<Storage, 'getItem' | 'setItem'> | null,
  catalog: () => Promise<readonly CatalogEffect[]>,
): Promise<void> {
  if (!storage) return;
  const stored = storedScheme(storage);
  const customs = loadCustomizations(storage);
  if (![...customs.values()].some((custom) => custom.cover)) {
    if (stored === null) saveCoverScheme(storage, DEFAULT_SCHEME);
    return;
  }
  try {
    const migrated = migratedScheme(await catalog(), customs);
    saveCustomizations(storage, migrated.customs);
    if (stored === null) saveCoverScheme(storage, migrated.scheme);
  } catch (error) {
    console.warn(error);
  }
}

/**
 * The fields that the cover dialog opens with for a built-in look with the cover `base` (a site
 * film look, or a native look; `mono` for a B&W look): its individual cover (`recorded`), else the
 * cover of `scheme`. Null for a look without a film cover.
 */
export function siteCoverFields(
  base: Cover | undefined,
  recorded: Cover | undefined,
  scheme: CoverScheme,
  mono = false,
): CoverFields | null {
  if (!isFilmCover(base)) return null;
  const shown = individualCover(base, recorded) ?? schemeCover(base, scheme, mono) ?? grayLookCover(base);
  return {
    template: shown.template,
    look: lookFromColors(resolveColors(shown)) ?? 'gray',
    brand: shown.brand,
    name: shown.name,
    number: shown.number ?? '',
    band: shown.band ?? false,
  };
}

/**
 * The individual cover that the dialog fields make for the built-in look with the cover `base`,
 * or null when they make its gray or its film box cover: the look then follows the scheme.
 */
export function customCoverOf(base: Cover, fields: CoverFields): Cover | null {
  return individualCover(base, coverIcon(fields)) ?? null;
}

/** One entry of a card's import library, with the ID of its effect. */
export interface LibraryItem {
  id: string;
  details: LibraryDetails;
}

/**
 * The customizations of a card's import library that this browser can use: entries of site film
 * looks of `catalog` (`kind: 'cover'`) with a name or an individual cover (`individualCover`) that
 * the effect file on the card still has. A later write that put the site name or the cover of the
 * scheme back can leave an old entry, whose field then does not count. `cardLabel` gives the name
 * and the icon of the effect file on the card, and `pixels` the icon of a cover.
 */
export async function cardCustomizations(
  items: readonly LibraryItem[],
  catalog: readonly CatalogEffect[],
  cardLabel: (id: string) => Promise<{ name: string; icon: Uint8Array } | undefined>,
  pixels: (cover: Cover) => Promise<Uint8Array>,
): Promise<Map<string, Customization>> {
  const customs = new Map<string, Customization>();
  for (const { id, details } of items) {
    const site = catalog.find((effect) => effect.id === id);
    if (details.kind !== 'cover' || !site) continue;
    const label = await cardLabel(id);
    if (!label) continue;
    const cover = individualCover(site.cover, readCustom(details.cover));
    const expected = cover && (await pixels(cover));
    const custom: Customization = {
      ...(details.name !== undefined && details.name === label.name && { name: details.name }),
      ...(cover &&
        expected &&
        label.icon.length === expected.length &&
        label.icon.every((byte, i) => byte === expected[i]) && { cover }),
    };
    if (custom.name !== undefined || custom.cover) customs.set(id, custom);
  }
  return customs;
}

/**
 * The customizations of this browser with those of a card added. What this browser has stays,
 * field by field: it can be newer than the card, and a write then puts it on the card.
 */
export function mergeCustomizations(browser: Customizations, card: Customizations): Customizations {
  const next = new Map(card);
  for (const [id, custom] of browser) next.set(id, { ...card.get(id), ...custom });
  return next;
}

/**
 * The customizations after a restore from a ZIP (`zip`, from `cardCustomizations`). For each site
 * film look that the ZIP holds (`held`), the ZIP wins: its name and cover, or none without an
 * entry. Other looks keep what this browser has.
 */
export function restoredCustomizations(
  browser: Customizations,
  zip: Customizations,
  held: ReadonlySet<string>,
): Customizations {
  const next = new Map([...browser].filter(([id]) => !held.has(id)));
  for (const [id, custom] of zip) if (held.has(id)) next.set(id, custom);
  return next;
}

/** `customs` without the custom covers of the site film looks of `catalog`. The names stay. */
export function withoutCustomCovers(catalog: readonly CatalogEffect[], customs: Customizations): Customizations {
  let next = customs;
  for (const { id } of catalog) {
    const own = customs.get(id);
    if (own?.cover) next = withCustomization(next, id, { ...own, cover: undefined });
  }
  return next;
}
