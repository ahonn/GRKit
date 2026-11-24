import type { Cover, CatalogEffect, PackageEffect } from './catalog';
import { colorsFromLook, NEUTRAL_COLORS, resolveColors, type CoverColors } from './coverPalette';
import { GR2_POSITIVE_SINCE, GR4_MONO_SINCE } from './release';
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

/** A cover of a look of GR IV Monochrome. */
const monoCover = (name: string, number: string): Cover => ({
  template: 'box',
  brand: 'RICOH',
  name,
  number,
  band: false,
});

/** GR IV has the Cinema looks built in, and on it these converted copies would show wrong colors. */
const CINEMA_CAMERAS: readonly string[] = ['gr3', 'gr3x'];
/** GR IV Monochrome has its own looks built in. */
const MONO_CAMERAS: readonly string[] = ['gr3', 'gr3x', 'gr4'];

const monoLook = (id: string, name: string, line: string, number: string): NativeLookInfo => ({
  id,
  name,
  firmware: 'gr4',
  cover: monoCover(line, number),
  monochrome: true,
  cameras: MONO_CAMERAS,
  since: GR4_MONO_SINCE,
});

const gr2Positive = (id: string, cameras: readonly string[]): NativeLookInfo => ({
  id,
  name: 'GR II Positive',
  firmware: 'gr2',
  monochrome: false,
  cameras,
  since: GR2_POSITIVE_SINCE,
  cover: {
    template: 'cine',
    brand: 'RICOH',
    name: 'GR II',
    number: 'POSITIVE',
    colors: colorsFromLook('gr2Positive'),
  },
});

/** A look that `nativeLooks` reads from a camera firmware file. */
export interface NativeLookInfo {
  id: string;
  name: string;
  firmware: NativeFirmware;
  cover: Cover;
  monochrome: boolean;
  /** The camera entries (`Camera.key`) that can use the look. */
  cameras: readonly string[];
  /** The first release whose packager reads the look. The page hides it before that release. */
  since?: string;
}

/**
 * The native looks that `nativeLooks` reads (grkit-core `gr4_looks::LOOKS` and
 * `gr2_looks::TARGETS`), in its order for each firmware. GR II Positive has an effect for GR III's
 * Standard and one for GR IV's.
 */
export const NATIVE_LOOKS: readonly NativeLookInfo[] = [
  {
    id: 'gr4-cinema-yellow',
    name: 'Cinema Yellow',
    firmware: 'gr4',
    cover: cinema('YELLOW', 'cinemaYellow'),
    monochrome: false,
    cameras: CINEMA_CAMERAS,
  },
  {
    id: 'gr4-cinema-green',
    name: 'Cinema Green',
    firmware: 'gr4',
    cover: cinema('GREEN', 'cinemaGreen'),
    monochrome: false,
    cameras: CINEMA_CAMERAS,
  },
  monoLook('gr4-mono-standard', 'Mono Standard', 'MONO', 'STD'),
  monoLook('gr4-mono-solid', 'Mono Solid', 'MONO', 'SOLID'),
  monoLook('gr4-mono-soft', 'Mono Soft', 'MONO', 'SOFT'),
  monoLook('gr4-mono-high-contrast', 'Mono Hi-Contrast', 'MONO', 'HI'),
  monoLook('gr4-mono-grainy', 'Mono Grainy', 'MONO', 'GRAINY'),
  monoLook('gr4-mono-solarized-1', 'Solarized 1', 'SOLAR', '1'),
  monoLook('gr4-mono-solarized-2', 'Solarized 2', 'SOLAR', '2'),
  monoLook('gr4-mono-solarized-3', 'Solarized 3', 'SOLAR', '3'),
  gr2Positive('gr2-positive-film', CINEMA_CAMERAS),
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
  const look = nativeLook(id);
  const base = look?.cover;
  return individualCover(base, recorded) ?? schemeCover(base, scheme, look?.monochrome ?? false);
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
