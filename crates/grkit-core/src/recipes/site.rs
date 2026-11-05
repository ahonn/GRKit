//! The generated module of the site for recipes.

use super::presets;
use crate::native;
use std::fmt::Write;

/// `web/src/generated/recipes.ts`: the recipes that the site offers, and the
/// camera names of the native types in each site language.
fn site_module() -> String {
    use crate::language::Language;
    let mut out = String::from(
        "// Generated from crates/grkit-core/src/recipes/. Do not edit: change the\n\
         // presets there, then run\n\
         // GRKIT_UPDATE_GENERATED=1 cargo test -p grkit-core --test generated.\n\n\
         import type { Preset } from '../recipes';\n\n\
         /** The recipes that the site offers. */\n\
         export const PRESETS: readonly Preset[] = ",
    );
    out.push_str(&serde_json::to_string_pretty(&presets()).unwrap());
    // The names of the stored values that a recipe shows: the nine levels of
    // a native look in portable order, the white balance modes of GR III and
    // GR IV, and the values of each correction setting.
    let levels: Vec<&str> = native::TYPES[0].families[0]
        .map_or(&[][..], |(_, keys)| keys)
        .to_vec();
    let modes: Vec<[Option<&str>; 2]> = native::WHITE_BALANCE
        .iter()
        .map(|mode| mode.labels)
        .collect();
    let corrections: Vec<Vec<&str>> = native::CORRECTIONS
        .iter()
        .map(|correction| correction.labels.labels.to_vec())
        .collect();
    writeln!(
        out,
        ";\n\n/** The levels of a native look in portable order (`native::TYPES`). */\n\
         export const LEVEL_KEYS: readonly string[] = {};\n\n\
         /** The labels of each white balance mode in portable order: GR III, then GR IV, or null. */\n\
         export const WHITE_BALANCE_MODES: readonly (readonly (string | null)[])[] = {};\n\n\
         /** The labels of the stored values of each correction setting, in portable order. */\n\
         export const CORRECTION_VALUES: readonly (readonly string[])[] = {};",
        serde_json::to_string(&levels).unwrap(),
        serde_json::to_string(&modes).unwrap(),
        serde_json::to_string(&corrections).unwrap(),
    )
    .unwrap();
    out.push_str(
        "\n/** The camera names of the types in each site language, by portable index. */\n",
    );
    out.push_str("export const TYPE_NAMES: Readonly<Record<string, readonly string[]>> = {\n");
    for language in Language::ALL {
        let names = serde_json::to_string(crate::menu_text::native_type_names(language)).unwrap();
        writeln!(out, "  '{}': {names},", language.code()).unwrap();
    }
    out.push_str("};\n");
    out
}

/// The generated files, as paths from the repository root and contents.
pub fn generated() -> Vec<(&'static str, String)> {
    vec![("web/src/generated/recipes.ts", site_module())]
}
