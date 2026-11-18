import type { Cover } from './catalog';
import { lookFromColors, NEUTRAL_COLORS, resolveColors } from './coverPalette';
import { readCover, type LibraryDetails } from './import/library';
import { coverIcon, type CoverFields } from './import/names';
import { isFilmCover, packageCover, type CustomCovers } from './packageCovers';

// The custom covers of site film looks. This browser keeps them, and each package keeps the ones
// of its effects in the import library (`kind: 'cover'`), so another browser gets them back from
// the card. A site film look without a custom cover follows the colorful switch.

const CUSTOM_COVERS_KEY = 'grkit.customCovers';
/** More than the catalog has, so a damaged value cannot grow without bound. */
const MAX_SAVED = 256;

/** A custom cover has its colors: without them it would follow the colorful switch. */
const readCustom = (value: unknown): Cover | undefined => {
  const cover = readCover(value);
  return cover?.colors ? cover : undefined;
};

/** The custom covers that this browser keeps, or none. */
export function loadCustomCovers(storage: Pick<Storage, 'getItem'> | null): CustomCovers {
  const covers = new Map<string, Cover>();
  try {
    const value: unknown = JSON.parse(storage?.getItem(CUSTOM_COVERS_KEY) ?? 'null');
    if (typeof value !== 'object' || value === null || Array.isArray(value)) return covers;
    for (const [id, item] of Object.entries(value).slice(0, MAX_SAVED)) {
      const cover = readCustom(item);
      if (cover) covers.set(id, cover);
    }
  } catch {
    // An unreadable value is the same as none.
  }
  return covers;
}

export function saveCustomCovers(storage: Pick<Storage, 'setItem'> | null, covers: CustomCovers): void {
  try {
    storage?.setItem(CUSTOM_COVERS_KEY, JSON.stringify(Object.fromEntries(covers)));
  } catch {
    // Without storage, the covers last until the page closes. A package still keeps them.
  }
}

/** `covers` with `cover` for `id`, or without a custom cover for `id` when `cover` is null. */
export function withCustomCover(covers: CustomCovers, id: string, cover: Cover | null): CustomCovers {
  const next = new Map(covers);
  if (cover) next.set(id, cover);
  else next.delete(id);
  return next;
}

/** The cover that a site film look gets without a custom cover, with the colors of the camera. */
function defaultCover(cover: Cover, colorful: boolean): Cover {
  const packed = packageCover(cover, colorful);
  return { ...packed, colors: packed.colors ?? NEUTRAL_COLORS };
}

/**
 * The fields that the cover dialog opens with for a site film look: its custom cover, else the
 * cover that the camera shows under the colorful switch. Null for a look without a film cover.
 */
export function siteCoverFields(own: Cover | undefined, colorful: boolean, custom?: Cover): CoverFields | null {
  if (!isFilmCover(own)) return null;
  const shown = custom ?? defaultCover(own, colorful);
  return {
    template: shown.template,
    look: lookFromColors(resolveColors(shown)) ?? 'gray',
    brand: shown.brand,
    name: shown.name,
    number: shown.number ?? '',
    band: shown.band ?? false,
  };
}

const sameCover = (a: Cover, b: Cover) =>
  a.template === b.template &&
  a.brand === b.brand &&
  a.name === b.name &&
  (a.number ?? '') === (b.number ?? '') &&
  (a.template !== 'box' || (a.band ?? false) === (b.band ?? false)) &&
  JSON.stringify(resolveColors(a)) === JSON.stringify(resolveColors(b));

/**
 * The custom cover that the dialog fields make, or null when they make the default cover: the
 * look then keeps following the colorful switch.
 */
export function customCoverOf(own: Cover, colorful: boolean, fields: CoverFields): Cover | null {
  const cover = coverIcon(fields);
  return sameCover(cover, defaultCover(own, colorful)) ? null : cover;
}

/** One entry of a card's import library, with the ID of its effect. */
export interface LibraryItem {
  id: string;
  details: LibraryDetails;
}

/**
 * The custom covers of a card's import library that this browser can use: entries of site film
 * looks whose effect file on the card still has that cover. A later write that put the default
 * cover back can leave an old entry, which then does not count. `cardIcon` gives the icon of the
 * effect file on the card, and `pixels` the icon of a cover.
 */
export async function cardCustomCovers(
  items: readonly LibraryItem[],
  siteIds: ReadonlySet<string>,
  cardIcon: (id: string) => Promise<Uint8Array | undefined>,
  pixels: (cover: Cover) => Promise<Uint8Array>,
): Promise<Map<string, Cover>> {
  const covers = new Map<string, Cover>();
  for (const { id, details } of items) {
    if (details.kind !== 'cover' || !siteIds.has(id)) continue;
    const cover = readCustom(details.cover);
    const icon = cover && (await cardIcon(id));
    if (!cover || !icon) continue;
    const expected = await pixels(cover);
    if (icon.length === expected.length && icon.every((byte, i) => byte === expected[i])) covers.set(id, cover);
  }
  return covers;
}

/**
 * The custom covers of this browser with those of a card added. A cover of this browser stays:
 * it can be newer than the card, and a write then puts it on the card.
 */
export function mergeCustomCovers(browser: CustomCovers, card: CustomCovers): CustomCovers {
  const next = new Map(card);
  for (const [id, cover] of browser) next.set(id, cover);
  return next;
}
