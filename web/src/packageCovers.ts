import type { Cover, CatalogEffect, PackageEffect } from './catalog';
import { NEUTRAL_COLORS, resolveColors, type CoverColors } from './coverPalette';

/** Film cover layouts that the packager can recolor. */
export function isFilmCover(cover: Cover | undefined): cover is Cover {
  return !!cover && (cover.template === 'box' || cover.template === 'cine' || cover.template === 'stack');
}

/**
 * The covers that the user gave site film looks, by effect ID (`customCovers.ts`). Each one has
 * its `colors`, and it wins over the colorful switch.
 */
export type CustomCovers = ReadonlyMap<string, Cover>;

/** How a write draws the film covers: the colorful switch and the custom covers. */
export interface CoverChoice {
  colorful: boolean;
  custom: CustomCovers;
}

export const NO_CUSTOM_COVERS: CustomCovers = new Map();

/**
 * Cover JSON to bake into a GRF for the colorful switch. Colorful on: stored or packaging
 * colors. Colorful off: no `colors` (classic gray), including Cinema looks whose web cover
 * stores yellow/green — the unlock GRF was gray, and a gray write keeps that.
 */
export function packageCover(cover: Cover, colorful: boolean): Cover {
  if (!isFilmCover(cover)) return cover;
  if (!colorful) {
    const { colors: _colors, ...rest } = cover;
    return rest;
  }
  return { ...cover, colors: resolveColors(cover) };
}

/**
 * The cover that a write bakes into an effect file, or null when the file keeps its own icon.
 * A custom cover always wins. Otherwise only the colorful switch recolors a film cover; with the
 * switch off the file stays as the site or the import made it.
 */
export function bakedCover(own: Cover | undefined, colorful: boolean, custom?: Cover): Cover | null {
  if (custom) return custom;
  if (!isFilmCover(own) || !colorful) return null;
  return packageCover(own, true);
}

/**
 * Whose colors the effect file has: `own` for a LUT import, whose file has the colors of its
 * cover; `switch` for a site film look and a native look, whose file has the gray cover and
 * takes colors only from the colorful switch.
 */
export type CoverSource = 'own' | 'switch';

/**
 * The cover that the camera shows for an effect after a write, with its colors, so the page draws
 * the same. A custom cover wins; an import cover with colors keeps them; else the colorful switch
 * gives packaging colors or gray. Undefined without film cover fields: the page then draws the
 * icon pixels of the file, which a write keeps.
 */
export function shownCover(
  own: Cover | undefined,
  source: CoverSource,
  colorful: boolean,
  custom?: Cover,
): Cover | undefined {
  if (custom) return custom;
  if (!isFilmCover(own)) return undefined;
  if (source === 'own' && own.colors) return own;
  return { ...own, colors: colorful ? resolveColors(own) : NEUTRAL_COLORS };
}

/**
 * True when a film cover must be relabeled for a colorful package. Cinema unlocks bake gray
 * icons while the page cover has colors — those need a rewrite when colorful is on.
 */
export function needsCoverRelabel(cover: Cover | undefined, colorful: boolean): boolean {
  return bakedCover(cover, colorful) !== null;
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
