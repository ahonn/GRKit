import type { Cover, CatalogEffect, PackageEffect } from './catalog';
import { resolveColors, type CoverColors } from './coverPalette';

/** Film cover layouts that the packager can recolor. */
export function isFilmCover(cover: Cover | undefined): cover is Cover {
  return !!cover && (cover.template === 'box' || cover.template === 'cine' || cover.template === 'stack');
}

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
 * True when a film cover must be relabeled for a colorful package. Cinema unlocks bake gray
 * icons while the page cover has colors — those need a rewrite when colorful is on.
 */
export function needsCoverRelabel(cover: Cover | undefined, colorful: boolean): boolean {
  return isFilmCover(cover) && colorful;
}

/** @deprecated Use `needsCoverRelabel`. */
export const catalogNeedsCoverRelabel = needsCoverRelabel;

/** Whether `effect` is a site catalog effect (has a published file and checksum). */
export function isCatalogEffect(effect: PackageEffect): effect is CatalogEffect {
  return !effect.imported && !effect.card && 'file' in effect && 'sha256' in effect;
}

/** Strip optional colors for tests / equality of wire covers without RGB. */
export function coverWithoutColors(cover: Cover): Omit<Cover, 'colors'> & { colors?: CoverColors } {
  const { colors: _colors, ...rest } = cover;
  return rest;
}
