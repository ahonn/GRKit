import { LIBRARY_MAX, ON_MAX } from '../recipes';

/** What the import dialog does with a recipe built from a preset. */
export type PresetRecipePlan = 'none' | 'full' | 'add' | 'addOff';

/**
 * Whether to add a recipe from a preset, add it as off, or skip because the library is full.
 * `wantsRecipe` is the checkbox; `hasRecipe` is true when the hints build a recipe.
 */
export function presetRecipePlan(
  wantsRecipe: boolean,
  hasRecipe: boolean,
  librarySize: number,
  onCount: number,
): PresetRecipePlan {
  if (!wantsRecipe || !hasRecipe) return 'none';
  if (librarySize >= LIBRARY_MAX) return 'full';
  if (onCount >= ON_MAX) return 'addOff';
  return 'add';
}
