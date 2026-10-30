import { describe, expect, it } from 'vitest';
import { RECIPE_STRINGS } from './recipeStrings';

describe('recipe strings', () => {
  const english = RECIPE_STRINGS.en;
  it('has each text in each language', () => {
    for (const strings of Object.values(RECIPE_STRINGS)) {
      expect(Object.keys(strings).sort()).toEqual(Object.keys(english).sort());
      expect(Object.keys(strings.parameters).sort()).toEqual(Object.keys(english.parameters).sort());
      expect(Object.keys(strings.modes).sort()).toEqual(Object.keys(english.modes).sort());
      expect(Object.keys(strings.values).sort()).toEqual(Object.keys(english.values).sort());
      expect(Object.keys(strings.fields).sort()).toEqual(Object.keys(english.fields).sort());
      expect(strings.corrections).toHaveLength(4);
      expect(strings.meterings).toHaveLength(4);
      expect(strings.grainLevels).toHaveLength(5);
      expect(strings.grainSizes).toHaveLength(4);
    }
  });
});
