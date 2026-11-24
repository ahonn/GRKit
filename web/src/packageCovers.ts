import type { Cover, CatalogEffect, PackageEffect } from './catalog';
import { colorsFromLook, NEUTRAL_COLORS, resolveColors, type CoverColors } from './coverPalette';
import { GR2_POSITIVE_SINCE } from './release';
import { groupOf } from './effectGroups';

/** Film cover layouts that a custom cover can replace. */
export function isFilmCover(cover: Cover | undefined): cover is Cover {
  return !!cover && (cover.template === 'box' || cover.template === 'cine' || cover.template === 'stack');
}

/**
 * What the user changed of a site film look (`customCovers.ts`): its name, its cover, or both.
 * A custom cover has its `colors`. A write bakes both into the effect file; the effect stays.
 */
export interface Customization {
  name?: string;
  cover?: Cover;
}

/** The customizations of site film looks, by effect ID. A look without one is as the site has it. */
export type Customizations = ReadonlyMap<string, Customization>;

export const NO_CUSTOMIZATIONS: Customizations = new Map();

/**
 * The cover that the site file of a film look has: without `colors`, the classic gray. This is
 * also true of Cinema looks, whose page cover stores yellow or green.
 */
export function grayCover(cover: Cover): Cover {
  if (!isFilmCover(cover)) return cover;
  const { colors: _colors, ...rest } = cover;
  return rest;
}

/**
 * The default color scheme of the covers of built-in looks (site film looks and native looks)
 * without an individual cover: `gray`, the cover of the site file; `colorFilmBox`, the colors of
 * the film box for color looks and gray for B&W looks; or `filmBox`, the colors of the film box of
 * each look.
 */
export type CoverScheme = 'gray' | 'colorFilmBox' | 'filmBox';

export const COVER_SCHEMES: readonly CoverScheme[] = ['gray', 'colorFilmBox', 'filmBox'];

/** Whether the look `effect` is a B&W look, as the B&W filter of the page groups it. */
export const isMonoLook = (effect: { id: string; monochrome?: boolean }) =>
  groupOf(effect.id, effect.monochrome ?? false) === 'mono';

/** Whether the camera draws `a` and `b` the same. */
export const sameCover = (a: Cover, b: Cover) =>
  a.template === b.template &&
  a.brand === b.brand &&
  a.name === b.name &&
  (a.number ?? '') === (b.number ?? '') &&
  (a.template !== 'box' || (a.band ?? false) === (b.band ?? false)) &&
  JSON.stringify(resolveColors(a)) === JSON.stringify(resolveColors(b));

/** The gray cover of the built-in look with the cover `base`, with its colors. */
export function grayLookCover(base: Cover): Cover {
  return { ...grayCover(base), colors: NEUTRAL_COLORS };
}

/** The film box cover of the built-in look with the cover `base`, with its colors. */
export function filmBoxCover(base: Cover): Cover {
  return { ...grayCover(base), colors: resolveColors(base) };
}

/**
 * The cover that `scheme` gives a built-in look (`base`, its site or native cover; `mono` for a
 * B&W look) without an individual cover, or undefined when that is the gray cover of its file:
 * always for `gray`, for a B&W look with `colorFilmBox`, and for a look whose film box is gray.
 */
export function schemeCover(base: Cover | undefined, scheme: CoverScheme, mono = false): Cover | undefined {
  if (scheme === 'gray' || (scheme === 'colorFilmBox' && mono) || !isFilmCover(base)) return undefined;
  const box = filmBoxCover(base);
  return sameCover(box, grayLookCover(base)) ? undefined : box;
}

/**
 * `cover` when it is an individual cover of the built-in look with the cover `base`: a cover
 * other than its gray and its film box cover. Those two follow the scheme.
 */
export function individualCover(base: Cover | undefined, cover: Cover | undefined): Cover | undefined {
  if (!cover || !isFilmCover(base)) return undefined;
  return sameCover(cover, grayLookCover(base)) || sameCover(cover, filmBoxCover(base)) ? undefined : cover;
}

/**
 * The cover that a write gives the site film look `effect`: its individual cover, else the cover
 * of `scheme`. Undefined leaves the gray cover of the site file.
 */
export function effectiveCover(
  effect: { id: string; cover?: Cover; monochrome?: boolean },
  customs: Customizations,
  scheme: CoverScheme,
): Cover | undefined {
  return (
    individualCover(effect.cover, customs.get(effect.id)?.cover) ??
    schemeCover(effect.cover, scheme, isMonoLook(effect))
  );
}

/** A cover of a native look. The icon that grkit-wasm draws has the same fields. */
const cinema = (number: string, look: 'cinemaYellow' | 'cinemaGreen'): Cover => ({
  template: 'cine',
  brand: 'RICOH',
  name: 'CINEMA',
  number,
  colors: colorsFromLook(look),
});

/** The camera whose firmware file holds a native look. */
export type NativeFirmware = 'gr4' | 'gr2';

const GR3_CAMERAS = ['gr3', 'gr3x'] as const;

const gr2Positive = (id: string, cameras: readonly string[]) => ({
  id,
  name: 'GR II Positive',
  firmware: 'gr2' as const,
  cameras,
  since: GR2_POSITIVE_SINCE,
  cover: {
    template: 'cine' as const,
    brand: 'RICOH',
    name: 'GR II',
    number: 'POSITIVE',
    colors: colorsFromLook('gr2Positive'),
  },
});

/**
 * The native looks that `nativeLooks` reads (the offered looks of grkit-core
 * `gr4_looks::LOOKS` and `gr2_looks::TARGETS`), in its order for each firmware. `cameras` are the
 * camera entries (`Camera.key`) that take the look. GR IV has the Cinema looks built in, and GR II
 * Positive has an effect for GR III's Standard and one for GR IV's.
 */
export const NATIVE_LOOKS: readonly {
  id: string;
  name: string;
  firmware: NativeFirmware;
  cameras: readonly string[];
  /** The first release whose packager reads the look. The page hides it before that release. */
  since?: string;
  cover: Cover;
}[] = [
  { id: 'gr4-cinema-yellow', name: 'Cinema Yellow', firmware: 'gr4', cameras: GR3_CAMERAS, cover: cinema('YELLOW', 'cinemaYellow') },
  { id: 'gr4-cinema-green', name: 'Cinema Green', firmware: 'gr4', cameras: GR3_CAMERAS, cover: cinema('GREEN', 'cinemaGreen') },
  gr2Positive('gr2-positive-film', GR3_CAMERAS),
  gr2Positive('gr2-positive-film-gr4', ['gr4']),
];

/** The native look `id`, or undefined for another effect. */
export const nativeLook = (id: string) => NATIVE_LOOKS.find((look) => look.id === id);

/**
 * The cover that a write gives a native look (`id`, a color look), whose import records
 * `recorded`: an individual cover, else the cover of `scheme`. Undefined leaves the gray cover of its file, and
 * is also the answer for another effect.
 */
export function nativeCover(id: string, recorded: Cover | undefined, scheme: CoverScheme): Cover | undefined {
  const base = nativeLook(id)?.cover;
  return individualCover(base, recorded) ?? schemeCover(base, scheme);
}

/**
 * Whose colors the effect file has: `own` for a LUT import, whose file has the colors of its
 * cover; `site` for a site film look and a native look, whose file has the gray cover.
 */
export type CoverSource = 'own' | 'site';

/**
 * The cover that the camera shows for an effect after a write, with its colors, so the page draws
 * the same. A custom cover wins; an import cover with colors keeps them; else gray. Undefined
 * without film cover fields: the page then draws the icon pixels of the file, which a write keeps.
 */
export function shownCover(own: Cover | undefined, source: CoverSource, custom?: Cover): Cover | undefined {
  if (custom) return custom;
  if (!isFilmCover(own)) return undefined;
  if (source === 'own' && own.colors) return own;
  return { ...own, colors: NEUTRAL_COLORS };
}

/** Whether `effect` is a site catalog effect (has a published file and checksum). */
export function isCatalogEffect(effect: PackageEffect): effect is CatalogEffect {
  return !effect.imported && !effect.card && 'file' in effect && 'sha256' in effect;
}

/** Strip optional colors for tests / equality of wire covers without RGB. */
export function coverWithoutColors(cover: Cover): Omit<Cover, 'colors'> & { colors?: CoverColors } {
  const { colors: _colors, ...rest } = cover;
  return rest;
}
