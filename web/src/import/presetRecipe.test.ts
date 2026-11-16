import { describe, expect, it } from 'vitest';
import { LIBRARY_MAX, ON_MAX } from '../recipes';
import { presetRecipePlan } from './presetRecipe';

describe('presetRecipePlan', () => {
  it('skips without the checkbox or without a recipe', () => {
    expect(presetRecipePlan(false, true, 0, 0)).toBe('none');
    expect(presetRecipePlan(true, false, 0, 0)).toBe('none');
  });

  it('reports a full library', () => {
    expect(presetRecipePlan(true, true, LIBRARY_MAX, 0)).toBe('full');
  });

  it('adds as off when ON_MAX recipes are on', () => {
    expect(presetRecipePlan(true, true, 10, ON_MAX)).toBe('addOff');
  });

  it('adds as on when there is room', () => {
    expect(presetRecipePlan(true, true, 0, 0)).toBe('add');
    expect(presetRecipePlan(true, true, LIBRARY_MAX - 1, ON_MAX - 1)).toBe('add');
  });
});
