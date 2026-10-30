import { describe, expect, it } from 'vitest';
import { RECIPE_MODELS } from './generated/recipes';
import { copyRecipe } from './recipeCopy';
import { nativeLook, NEUTRAL, type Family, type Look, type Recipe } from './recipes';

const model = (family: Family) => RECIPE_MODELS[family];
const type = (family: Family, label: string) => model(family).types.find((item) => item.label === label)!;
const mode = (family: Family, label: string) => model(family).modes.find((item) => item.label === label)!.index;

function recipe(family: Family, look: Look, whiteBalance: string | null): Recipe {
  return {
    name: 'Copy me',
    family,
    look,
    grain: { level: 2, size: 1 },
    whiteBalance: whiteBalance ? { mode: mode(family, whiteBalance), shiftBa: 17, shiftGm: 12, kelvin: 5500 } : null,
    corrections: [1, null, 0, 3],
    hints: { exposure: -1, isoLimit: 3200, metering: 'spot', nd: null },
  };
}
const copy = (value: Recipe, to: Family) => copyRecipe(value, model(value.family), to, model(to));
const fields = (flags: { field: string }[]) => flags.map((flag) => flag.field).sort();

describe('copyRecipe', () => {
  it('GR III to GR IV: the same type, Multi-pattern Auto to AWB, each other value kept', () => {
    const look = { ...nativeLook(type('gr3', 'PositiveFilm')), levels: [6, 3, 4, 4, 5, 4, 4, 2, 4] } as Look;
    const result = copy(recipe('gr3', look, 'Multi'), 'gr4');
    expect(result.flags).toEqual([]);
    expect(result.recipe.family).toBe('gr4');
    expect(result.recipe.look).toMatchObject({ imageType: type('gr4', 'PositiveFilm').index, levels: [6, 3, 4, 4, 5, 4, 4, 2, 4] });
    expect(result.recipe.whiteBalance).toMatchObject({ mode: mode('gr4', 'AutoStandard'), shiftBa: 17, shiftGm: 12 });
    expect(result.recipe.corrections).toEqual([1, null, 0, 3]);
    expect(result.recipe.grain).toEqual({ level: 2, size: 1 });
    expect(result.recipe.hints.isoLimit).toBe(3200);
  });

  it('GR IV to GR III: no Cinema, no AWB (Warm), and one native grain size', () => {
    const cinema = copy(recipe('gr4', nativeLook(type('gr4', 'CinemaYellow')), 'AutoWarm'), 'gr3');
    expect(fields(cinema.flags)).toEqual(['look', 'whiteBalance']);
    expect(cinema.recipe.look).toMatchObject({ imageType: type('gr3', 'Standard').index });
    expect(cinema.recipe.whiteBalance).toBeNull();
    const negative = {
      ...nativeLook(type('gr4', 'NegativeFilm')),
      grainStrength: 2,
      grainSize: 2,
    } as Look;
    const result = copy(recipe('gr4', negative, 'AutoStandard'), 'gr3');
    expect(fields(result.flags)).toEqual(['nativeGrainSize']);
    expect(result.recipe.look).toMatchObject({ grainStrength: 2, grainSize: 0 });
    expect(result.recipe.whiteBalance).toMatchObject({ mode: mode('gr3', 'Auto') });
  });

  it('to GR IV Monochrome: no Film Look, no color type, and no white balance', () => {
    const effect: Look = { kind: 'effect', code: 7, adjustments: Array(9).fill(NEUTRAL) };
    for (const from of ['gr3', 'gr4'] as const) {
      const result = copy(recipe(from, effect, 'Daylight'), 'gr4Monochrome');
      expect(fields(result.flags)).toEqual(['look', 'whiteBalance']);
      expect(result.recipe.look).toMatchObject({ kind: 'native', imageType: type('gr4Monochrome', 'MonochromeStandard').index });
      expect(result.recipe.whiteBalance).toBeNull();
      const monotone = copy(recipe(from, nativeLook(type(from, 'MonotoneGR')), null), 'gr4Monochrome');
      expect(fields(monotone.flags)).toEqual(['look']);
    }
  });

  it('from GR IV Monochrome: no Monochrome type on a color camera', () => {
    for (const to of ['gr3', 'gr4'] as const) {
      const result = copy(recipe('gr4Monochrome', nativeLook(type('gr4Monochrome', 'MonochromeSoft')), null), to);
      expect(fields(result.flags)).toEqual(['look']);
      expect(result.recipe.look).toMatchObject({ imageType: type(to, 'Standard').index });
    }
  });

  it('keeps an effect recipe between the color cameras', () => {
    const effect: Look = { kind: 'effect', code: 7, adjustments: [5, 4, 4, 4, 4, 4, 4, 4, 4] };
    const result = copy(recipe('gr4', effect, 'Shade'), 'gr3');
    expect(result.flags).toEqual([]);
    expect(result.recipe.look).toEqual(effect);
  });
});
