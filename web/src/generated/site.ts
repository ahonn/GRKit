// Generated from crates/grkit-core/src/site.rs. Do not edit: change the
// constants there, then run
// GRKIT_UPDATE_GENERATED=1 cargo test -p grkit-core --test generated.

/** The target whose effects and previews the site shows. */
export const PREVIEW_TARGET = 'gr3-2.10';

/** The effects that one package can hold. */
export const MAX_EFFECTS = 40;

/** The schema of the effect catalog of each target. */
export const CATALOG_SCHEMA = 'grkit-site-effects-v1';

/** The longest effect name and the longest effect ID (effects.rs). */
export const EFFECT_NAME_MAX = 16;
export const EFFECT_ID_MAX = 31;

/** An effect ID and the name of its GRF file in a package (effects::filename). */
export const EFFECT_FILENAME_SAMPLE = { id: 'velvia-100-v1', filename: '8E5B19FF.GRF' } as const;
