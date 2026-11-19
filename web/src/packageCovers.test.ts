import { describe, expect, it } from 'vitest';
import { colorsFromLook, NEUTRAL_COLORS } from './coverPalette';
import { coverWithoutColors, grayCover, isFilmCover, shownCover } from './packageCovers';

describe('grayCover', () => {
  it('strips colors so a site file stays classic gray, Cinema included', () => {
    const gold = { template: 'box' as const, brand: 'Kodak', name: 'GOLD', number: '200', band: true };
    const cinema = { template: 'cine' as const, brand: 'RICOH', name: 'CINEMA', number: 'YELLOW' };
    expect(grayCover(gold)).toEqual(gold);
    expect(grayCover({ ...cinema, colors: colorsFromLook('cinemaYellow') })).toEqual(cinema);
    expect(isFilmCover(undefined)).toBe(false);
    expect(coverWithoutColors({ ...gold, colors: NEUTRAL_COLORS })).toEqual(gold);
  });
});

describe('shownCover', () => {
  const portra = { template: 'box' as const, brand: 'Kodak', name: 'PORTRA', number: '400', band: false };
  const kodak = { ...portra, colors: colorsFromLook('kodak') };
  const cinema = { template: 'cine' as const, brand: 'RICOH', name: 'CINEMA', number: 'YELLOW', colors: colorsFromLook('cinemaYellow') };

  it('draws site and native looks gray, also when the page cover has colors', () => {
    expect(shownCover(portra, 'site')!.colors).toEqual(NEUTRAL_COLORS);
    expect(shownCover(cinema, 'site')!.colors).toEqual(NEUTRAL_COLORS);
  });

  it('draws an import with its own colors, and one without colors gray', () => {
    expect(shownCover(kodak, 'own')).toEqual(kodak);
    expect(shownCover(portra, 'own')!.colors).toEqual(NEUTRAL_COLORS);
  });

  it('draws a custom cover, and nothing without cover fields', () => {
    expect(shownCover(portra, 'site', kodak)).toEqual(kodak);
    expect(shownCover(undefined, 'own')).toBeUndefined();
  });
});
