import { describe, expect, it } from 'vitest';
import { colorsFromLook, NEUTRAL_COLORS, resolveColors } from './coverPalette';
import {
  coverWithoutColors,
  isFilmCover,
  needsCoverRelabel,
  packageCover,
  shownCover,
} from './packageCovers';

describe('packageCover', () => {
  const gold = { template: 'box' as const, brand: 'Kodak', name: 'GOLD', number: '200', band: true };
  const cinema = {
    template: 'cine' as const,
    brand: 'RICOH',
    name: 'CINEMA',
    number: 'YELLOW',
    colors: colorsFromLook('cinemaYellow'),
  };

  it('adds packaging colors when colorful is on and the cover has none', () => {
    expect(packageCover(gold, true)).toEqual({ ...gold, colors: resolveColors(gold) });
    expect(packageCover(gold, true).colors).toEqual(colorsFromLook('gold'));
  });

  it('bakes stored cinema colors when colorful is on', () => {
    expect(packageCover(cinema, true).colors).toEqual(colorsFromLook('cinemaYellow'));
    expect(needsCoverRelabel(cinema, true)).toBe(true);
  });

  it('strips colors when colorful is off so unlock GRFs stay classic gray', () => {
    expect(packageCover(gold, false)).toEqual(gold);
    expect(packageCover(cinema, false)).toEqual({
      template: 'cine',
      brand: 'RICOH',
      name: 'CINEMA',
      number: 'YELLOW',
    });
    expect(needsCoverRelabel(cinema, false)).toBe(false);
    expect(needsCoverRelabel(gold, false)).toBe(false);
  });

  it('flags film covers for relabel only when colorful is on', () => {
    expect(needsCoverRelabel(gold, true)).toBe(true);
    expect(needsCoverRelabel({ ...gold, colors: NEUTRAL_COLORS }, true)).toBe(true);
    expect(isFilmCover(undefined)).toBe(false);
  });

  it('strips colors for equality checks', () => {
    expect(coverWithoutColors({ ...gold, colors: NEUTRAL_COLORS })).toEqual(gold);
  });
});

describe('shownCover', () => {
  const portra = { template: 'box' as const, brand: 'Kodak', name: 'PORTRA', number: '400', band: false };
  const kodak = { ...portra, colors: colorsFromLook('kodak') };
  const cinema = { template: 'cine' as const, brand: 'RICOH', name: 'CINEMA', number: 'YELLOW', colors: colorsFromLook('cinemaYellow') };

  it('draws site and native looks as the switch writes them, also when the page cover has colors', () => {
    expect(shownCover(portra, 'switch', false)!.colors).toEqual(NEUTRAL_COLORS);
    expect(shownCover(portra, 'switch', true)!.colors).toEqual(colorsFromLook('portra'));
    // A Cinema unlock file is gray: the shelf shows gray until the switch is on.
    expect(shownCover(cinema, 'switch', false)!.colors).toEqual(NEUTRAL_COLORS);
    expect(shownCover(cinema, 'switch', true)!.colors).toEqual(colorsFromLook('cinemaYellow'));
  });

  it('draws an import with its own colors, and one without colors as the switch writes it', () => {
    expect(shownCover(kodak, 'own', false)).toEqual(kodak);
    expect(shownCover(kodak, 'own', true)).toEqual(kodak);
    expect(shownCover(portra, 'own', false)!.colors).toEqual(NEUTRAL_COLORS);
    expect(shownCover(portra, 'own', true)!.colors).toEqual(colorsFromLook('portra'));
  });

  it('draws a custom cover in both switch states, and nothing without cover fields', () => {
    expect(shownCover(portra, 'switch', false, kodak)).toEqual(kodak);
    expect(shownCover(portra, 'switch', true, kodak)).toEqual(kodak);
    expect(shownCover(undefined, 'own', true)).toBeUndefined();
  });
});
