import { describe, expect, it } from 'vitest';
import { RECIPE_MODELS, TYPE_NAMES } from './generated/recipes';
import { LANGUAGES } from './i18n';
import { exposureOf, parseRecipeText, recipeText, shiftsOf } from './recipeText';
import { modelFor, NEUTRAL, sameRecipes, type Family, type Recipe } from './recipes';

/*
 * The fixtures follow the formats of the community recipes of the survey: labels in English,
 * Chinese, and Japanese, a value after a colon or a space, and the white balance shifts as
 * "A:3 G:1", "G3B3", or "+B5". The recipes themselves are made up for the tests.
 */

const model = (family: Family) => modelFor(RECIPE_MODELS, TYPE_NAMES.en, family);
const names = Object.values(TYPE_NAMES);
const parse = (text: string, family: Family) => parseRecipeText(text, family, model(family), [], names);
const typeIndex = (label: string) =>
  Object.values(RECIPE_MODELS)
    .flatMap((item) => item.types)
    .find((type) => type.label === label)!.index;
/** The stored levels of -4..+4 values, in portable order. */
const levels = (...values: number[]) => values.map((value) => value + NEUTRAL);

describe('parseRecipeText', () => {
  it('reads an English recipe with a WB compensation line and a range of exposure', () => {
    const { recipe, flags } = parse(
      `Portra Days
Image Control: Positive Film
Saturation: -1
Hue: +1
High/Low Key: 0
Contrast: -1
Contrast (Highlight): -2
Contrast (Shadow): +1
Sharpness: 0
Shading: -2
Clarity: 0
White Balance: Daylight
WB Compensation: A:3 G:1
Highlight Correction: On
Shadow Correction: Low
Peripheral Illumination Correction: Off
High-ISO NR: Auto
Exposure Compensation: +1/3 to +2/3
ISO: Auto, up to 6400`,
      'gr3',
    );
    expect(recipe.name).toBe('Portra Days');
    expect(recipe.look).toMatchObject({
      kind: 'native',
      imageType: typeIndex('PositiveFilm'),
      levels: levels(-1, 1, 0, -1, -2, 1, 0, -2, 0),
    });
    expect(recipe.whiteBalance).toMatchObject({ mode: 4, shiftBa: 17, shiftGm: 13 });
    expect(recipe.corrections).toEqual([1, 1, 0, 0]);
    expect(recipe.hints).toEqual({ exposure: 1, isoLimit: 6400, metering: null, nd: null });
    // The range keeps its first value and asks to check it.
    expect(flags).toEqual([{ field: 'exposure', text: '+1/3 to +2/3' }]);
  });

  it('reads a Chinese recipe with grain and G3B3 on the white balance line', () => {
    const { recipe, flags } = parse(
      `负片街拍
图像控制：负片
饱和度：+2
色相：-1
对比度：+1
锐度：-1
暗角：-3
清晰度：+1
颗粒：中，大小：大
白平衡：日光 G3B3
高光补偿：开
阴影补偿：低
周边光量补偿：关
高ISO降噪：关
曝光补偿：-0.7
测光：点测光`,
      'gr4',
    );
    expect(flags).toEqual([]);
    expect(recipe.name).toBe('负片街拍');
    expect(recipe.look).toMatchObject({
      kind: 'native',
      imageType: typeIndex('NegativeFilm'),
      levels: levels(2, -1, 0, 1, 0, 0, -1, -3, 1),
      grainStrength: 2,
      grainSize: 2,
    });
    expect(recipe.whiteBalance).toMatchObject({ mode: 4, shiftBa: 11, shiftGm: 11 });
    expect(recipe.corrections).toEqual([1, 1, 0, 5]);
    expect(recipe.hints).toMatchObject({ exposure: -2, metering: 'spot' });
  });

  it('reads a Japanese recipe with labels and values after a space', () => {
    const { recipe, flags } = parse(
      `ネガ調スナップ
イメージコントロール：ネガフィルム調
彩度 +1
コントラスト -1
シャープネス ±0
ホワイトバランス：マルチパターンオート +B5
ハイライト補正：オート
露出補正：+2/3`,
      'gr3',
    );
    expect(flags).toEqual([]);
    expect(recipe.look).toMatchObject({ imageType: typeIndex('NegativeFilm'), levels: levels(1, 0, 0, -1, 0, 0, 0, 0, 0) });
    expect(recipe.whiteBalance).toMatchObject({ mode: 1, shiftBa: 9, shiftGm: 14 });
    expect(recipe.corrections[0]).toBe(0);
    expect(recipe.hints.exposure).toBe(2);
  });

  it('flags each value that the camera line does not have or that it cannot read', () => {
    const { recipe, flags } = parse(
      `Not on GR III
Image Control: Cinema (Yellow)
Grain Effect: High
Saturation: +7
Lens Flare: Strong
White Balance: Candlelight`,
      'gr3',
    );
    // The look stays Standard, which has no grain.
    expect(recipe.look).toMatchObject({ imageType: typeIndex('Standard'), grainStrength: 0 });
    expect(recipe.whiteBalance).toBeNull();
    expect(flags.map((flag) => flag.field).sort()).toEqual(
      ['Saturation', 'look', 'line', 'nativeGrain', 'whiteBalance'].sort(),
    );
  });

  it('keeps the white balance out of a GR IV Monochrome recipe', () => {
    const { recipe, flags } = parse(
      `Grainy Night
Image Control: Grainy
Contrast: +2
White Balance: Daylight`,
      'gr4Monochrome',
    );
    expect(recipe.look).toMatchObject({ imageType: typeIndex('MonochromeGrainy') });
    expect(recipe.whiteBalance).toBeNull();
    expect(flags).toEqual([{ field: 'whiteBalance', text: 'Daylight' }]);
  });

  it('reads the white balance shifts in each order and with one axis', () => {
    expect(shiftsOf('A:7 G:3')).toEqual({ ba: 21, gm: 11 });
    expect(shiftsOf('G3B3')).toEqual({ ba: 11, gm: 11 });
    expect(shiftsOf('+B5')).toEqual({ ba: 9 });
    expect(shiftsOf('M:6 B:11')).toEqual({ ba: 3, gm: 20 });
    expect(shiftsOf('+3', 'gm')).toEqual({ gm: 17 });
    expect(shiftsOf('A15')).toBeNull();
  });

  it('reads exposure compensation in thirds', () => {
    expect(exposureOf('+2/3')).toEqual({ thirds: 2, range: false });
    expect(exposureOf('-1 1/3')).toEqual({ thirds: -4, range: false });
    expect(exposureOf('-0.7')).toEqual({ thirds: -2, range: false });
    expect(exposureOf('+1EV')).toEqual({ thirds: 3, range: false });
    expect(exposureOf('+1/3 to +2/3')).toEqual({ thirds: 1, range: true });
  });
});

describe('recipeText', () => {
  const recipes: Recipe[] = [
    {
      name: 'Street Nega',
      family: 'gr3',
      look: {
        kind: 'native',
        imageType: typeIndex('NegativeFilm'),
        levels: levels(2, -1, 0, 1, 0, 0, -1, -3, 1),
        tone: 0,
        intensity: 0,
        filter: 0,
        grainStrength: 3,
        grainSize: 0,
        solarization: 0,
      },
      grain: { level: 2, size: 1 },
      whiteBalance: { mode: 4, shiftBa: 17, shiftGm: 12, kelvin: 5500 },
      corrections: [1, null, 0, 5],
      hints: { exposure: -4, isoLimit: 3200, metering: 'spot', nd: 'auto' },
    },
    {
      name: 'Cine Green',
      family: 'gr4',
      look: {
        kind: 'native',
        imageType: typeIndex('CinemaGreen'),
        levels: levels(0, 0, 1, 0, 0, 0, 0, 0, 0),
        tone: 2,
        intensity: 0,
        filter: 0,
        grainStrength: 1,
        grainSize: 2,
        solarization: 0,
      },
      grain: null,
      whiteBalance: { mode: 14, shiftBa: 14, shiftGm: 14, kelvin: 4300 },
      corrections: [null, null, null, null],
      hints: { exposure: null, isoLimit: null, metering: null, nd: null },
    },
    {
      name: 'Solar Sepia',
      family: 'gr4Monochrome',
      look: {
        kind: 'native',
        imageType: typeIndex('MonochromeSolarization'),
        levels: levels(0, 0, -2, 0, 0, 0, 1, 0, 2),
        tone: 1,
        intensity: 0,
        filter: 0,
        grainStrength: 0,
        grainSize: 0,
        solarization: 2,
      },
      grain: null,
      whiteBalance: null,
      corrections: [0, 4, null, 2],
      hints: { exposure: 5, isoLimit: null, metering: 'highlight', nd: null },
    },
  ];

  for (const { code } of LANGUAGES) {
    it(`exports a recipe that the import reads back (${code})`, () => {
      for (const recipe of recipes) {
        const text = recipeText(
          { recipe, author: 'Someone', source: 'https://example.com/recipe' },
          modelFor(RECIPE_MODELS, TYPE_NAMES[code], recipe.family),
          code,
        );
        const parsed = parse(text, recipe.family);
        expect(parsed.flags, text).toEqual([]);
        expect(sameRecipes([parsed.recipe], [recipe]), text).toBe(true);
        expect(parsed.author).toBe('Someone');
        expect(parsed.source).toBe('https://example.com/recipe');
      }
    });
  }
});
