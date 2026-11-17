/**
 * Catalog film-look groups for the install picker. Unknown IDs fall back to Color vs B&W from
 * `monochrome` on the effect.
 */
export type EffectGroup = 'negative' | 'slide' | 'cinema' | 'instant' | 'mono';

/** Section order in the All / Color filters. */
export const EFFECT_GROUP_ORDER = [
  'negative',
  'slide',
  'cinema',
  'instant',
  'mono',
] as const satisfies readonly EffectGroup[];

export const EFFECT_GROUPS: Readonly<Record<string, EffectGroup>> = {
  'aerocolor-iv-v1': 'negative',
  'c200-premier-v1': 'negative',
  'ektar-100-edge-v1': 'negative',
  'fuji-160c-v1': 'negative',
  'gold-200-premier-v1': 'negative',
  'portra-160-vc-v1': 'negative',
  'portra-400-endura-v1': 'negative',
  'portra-800-push2-v1': 'negative',
  'pro-400h-crystal-v1': 'negative',
  'ultramax-400-ultra-v1': 'negative',
  'vista-200-v1': 'negative',
  'xtra-400-crystal-v1': 'negative',

  'astia-100f-v1': 'slide',
  'ektachrome-e100-v1': 'slide',
  'kodachrome-64-v1': 'slide',
  'precisa-100-v1': 'slide',
  'provia-100f-v1': 'slide',
  'velvia-100-v1': 'slide',

  'eterna-500t-3523xd-v1': 'cinema',
  'vision3-250d-2383-v1': 'cinema',
  'vision3-500t-supra-v1': 'cinema',

  'instax-v1': 'instant',

  'delta-100-v1': 'mono',
  'double-x-5222-mono-v1': 'mono',
  'hp5-plus-400-mono-v1': 'mono',
  'neopan-acros-100-v1': 'mono',
  'red-filter-mono-v1': 'mono',
  't-max-400-v1': 'mono',
  'trix-400-mono-v1': 'mono',
  'xp2-super-v1': 'mono',
};

export type EffectFilter = 'all' | 'color' | 'mono' | 'yours';

export function groupOf(id: string, monochrome: boolean): EffectGroup {
  return EFFECT_GROUPS[id] ?? (monochrome ? 'mono' : 'negative');
}
