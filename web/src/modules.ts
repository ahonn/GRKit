/**
 * The ADJ modules and the package rules for them. Image Control shows on the
 * camera exactly when the package has effects. The other modules show unless
 * the package hides them (`MODULES.TXT`, through `Package.hideModule`).
 */
export const MODULES = [
  { key: 'imageControl', name: 'Image Control', hide: null },
  { key: 'framing', name: 'Framing', hide: 'framing' },
  { key: 'guides', name: 'Guides', hide: 'guides' },
  { key: 'dateStamp', name: 'Date Stamp', hide: 'date-stamp' },
] as const;

export type ModuleKey = (typeof MODULES)[number]['key'];

export const ALL_MODULES: ReadonlySet<ModuleKey> = new Set(MODULES.map((module) => module.key));

export interface ModulePlan {
  /** The package includes the selected effects only when Image Control is on. */
  includeEffects: boolean;
  /** Names for `Package.hideModule`. */
  hidden: string[];
  /** The modules that the camera menu shows, in menu order. */
  visible: ModuleKey[];
}

export function planModules(enabled: ReadonlySet<ModuleKey>, effectCount: number): ModulePlan {
  const includeEffects = enabled.has('imageControl');
  return {
    includeEffects,
    hidden: MODULES.flatMap((module) => (module.hide && !enabled.has(module.key) ? [module.hide] : [])),
    visible: MODULES.filter((module) =>
      module.key === 'imageControl' ? includeEffects && effectCount > 0 : enabled.has(module.key),
    ).map((module) => module.key),
  };
}
