import type { Flag } from './recipeText';
import { nativeLook, NEUTRAL, type Family, type Model, type Recipe } from './recipes';

/**
 * Copies a recipe to another camera line. A value with a clear match keeps its meaning: a type
 * or a white balance mode of the same name, and the parameters, the corrections, the Grain, and
 * the hints, whose stored values are the same on each line. GR III Auto and Multi-pattern Auto
 * become the AWB of GR IV, and AWB becomes Auto. Each other value takes the default of the
 * target and becomes a flag, so that the person selects it.
 */
export function copyRecipe(recipe: Recipe, from: Model, to: Family, model: Model): { recipe: Recipe; flags: Flag[] } {
  const flags: Flag[] = [];
  const copy: Recipe = structuredClone({ ...recipe, family: to });
  const look = recipe.look;
  const monochrome = to === 'gr4Monochrome';

  if (look.kind === 'effect' && monochrome) {
    // GR IV Monochrome has no Film Look.
    copy.look = nativeLook(model.types[0]!);
    flags.push({ field: 'look', text: '' });
  } else if (look.kind === 'native') {
    const source = from.types.find((type) => type.index === look.imageType);
    const type = model.types.find((candidate) => candidate.label === source?.label);
    if (!type) {
      copy.look = nativeLook(model.types[0]!);
      flags.push({ field: 'look', text: source?.name ?? '' });
    } else if (copy.look.kind === 'native') {
      // The same type: each parameter that the target has keeps its value.
      const slots: Record<string, 'tone' | 'intensity' | 'filter' | 'solarization'> = {
        HdrIntense: 'intensity',
        FilterEffectCustom: 'filter',
        SolarizationType: 'solarization',
      };
      for (const parameter of source!.parameters) {
        if (type.parameters.some((candidate) => candidate.key === parameter.key)) continue;
        flags.push({ field: parameter.key, text: '' });
        const slot = parameter.key.startsWith('ColorTone') ? 'tone' : slots[parameter.key];
        if (slot) copy.look[slot] = 0;
      }
      if (copy.look.grainStrength > 0 && !type.grain) {
        copy.look.grainStrength = 0;
        copy.look.grainSize = 0;
        flags.push({ field: 'nativeGrain', text: '' });
      } else if (copy.look.grainSize > 0 && !type.grainSizes) {
        copy.look.grainSize = 0;
        flags.push({ field: 'nativeGrainSize', text: '' });
      }
      const levels = copy.look.levels;
      const keys = ['Saturation', 'ColorHue', 'ImageKey', 'Contrast', 'ContrastHighLight', 'ContrastShadow', 'Sharpness', 'Shading', 'ClarityControl'];
      keys.forEach((key, index) => {
        if (!type.parameters.some((parameter) => parameter.key === key)) levels[index] = NEUTRAL;
      });
    }
  }

  const balance = recipe.whiteBalance;
  if (balance) {
    if (monochrome) {
      copy.whiteBalance = null;
      flags.push({ field: 'whiteBalance', text: '' });
    } else {
      const label = from.modes.find((mode) => mode.index === balance.mode)?.label;
      const clear: Record<string, string> = { Auto: 'AutoStandard', Multi: 'AutoStandard', AutoStandard: 'Auto' };
      const mode =
        model.modes.find((candidate) => candidate.label === label) ??
        model.modes.find((candidate) => candidate.label === clear[label ?? '']);
      if (!mode) {
        copy.whiteBalance = null;
        flags.push({ field: 'whiteBalance', text: '' });
      } else {
        copy.whiteBalance = { ...balance, mode: mode.index };
        if (!mode.shifts) copy.whiteBalance = { ...copy.whiteBalance, shiftBa: 14, shiftGm: 14 };
      }
    }
  }
  return { recipe: copy, flags };
}
