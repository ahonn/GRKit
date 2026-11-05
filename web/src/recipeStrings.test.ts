import { describe, expect, it } from 'vitest';
import { PRESETS } from './generated/recipes';
import { RECIPE_STRINGS } from './recipeStrings';

describe('recipe strings', () => {
  const english = RECIPE_STRINGS.en;
  it('has each text in each language, and the text of each preset', () => {
    for (const strings of Object.values(RECIPE_STRINGS)) {
      expect(Object.keys(strings).sort()).toEqual(Object.keys(english).sort());
      expect(Object.keys(strings.presetText).sort()).toEqual(PRESETS.map((preset) => preset.id).sort());
      expect(strings.grainLevels).toHaveLength(5);
      expect(strings.glowLevels).toHaveLength(4);
      expect(strings.halationColors).toHaveLength(4);
    }
  });
});
