// Generated from crates/grkit-core/src/modules.rs. Do not edit: change the
// list there, then run
// GRKIT_UPDATE_GENERATED=1 cargo test -p grkit-core --test generated.

/** The official modules in menu order. `needsEffects`: the module shows only with effects. */
export const OFFICIAL_MODULES = [
  { key: 'recipes', id: 'RECIPES', needsEffects: false },
  { key: 'filmLook', id: 'FILMLOOK', needsEffects: true },
  { key: 'framing', id: 'FRAMING', needsEffects: false },
  { key: 'crop', id: 'CROP', needsEffects: false },
  { key: 'guides', id: 'GUIDES', needsEffects: false },
  { key: 'dateStamp', id: 'DATE', needsEffects: false },
  { key: 'glow', id: 'GLOW', needsEffects: false },
  { key: 'border', id: 'BORDER', needsEffects: false },
  { key: 'grain', id: 'GRAIN', needsEffects: false },
] as const;

export type ModuleKey = (typeof OFFICIAL_MODULES)[number]['key'];

/** The official modules that each camera can install, in menu order. */
export const CAMERA_MODULES = {
  gr3: ['recipes', 'filmLook', 'framing', 'crop', 'guides', 'dateStamp', 'glow', 'border', 'grain'],
  gr3x: ['recipes', 'filmLook', 'framing', 'crop', 'guides', 'dateStamp', 'glow', 'border', 'grain'],
  gr4: ['recipes', 'filmLook', 'framing', 'crop', 'guides', 'dateStamp', 'glow', 'border', 'grain'],
} as const satisfies Readonly<Record<string, readonly ModuleKey[]>>;
