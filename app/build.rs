//! Writes `locales/` as the translations JSON nsh's generated roots compile
//! in, read with the same reader (`nsh_i18n`), so the roots nsh does not
//! generate (`cargo run`, `cargo test`) show the same text:
//! `grkit_app::translations`.
//!
//! Also sets `cfg(demo)` when debug assertions or the `demo` feature are on,
//! or when a development variable is set for the build: it compiles the demo
//! camera and the development hooks (`src/dev`), which a release build leaves
//! out. nsh builds with the release profile, so `GRKIT_DEMO=1 nsh run` and
//! `GRKIT_CAMERA_URL=… nsh run` need the variables.

use std::{env, fs, path::PathBuf};

/// The variables that start the demo camera or point the app at a
/// development camera (`src/dev`, `camera_api::transport`).
const DEVELOPMENT_VARIABLES: [&str; 3] = ["GRKIT_DEMO", "GRKIT_CAMERA_URL", "GRKIT_DEMO_SCREEN"];

fn main() {
    let app_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    println!("cargo::rerun-if-changed=locales");
    let translations = nsh_i18n::read_translations(&app_dir).unwrap_or_else(|e| panic!("{e}"));
    let out = PathBuf::from(env::var("OUT_DIR").unwrap()).join("translations.json");
    fs::write(out, nsh_i18n::translations_json(&translations)).unwrap();

    println!("cargo::rustc-check-cfg=cfg(demo)");
    for name in DEVELOPMENT_VARIABLES {
        println!("cargo::rerun-if-env-changed={name}");
    }
    let development = DEVELOPMENT_VARIABLES
        .iter()
        .any(|name| env::var_os(name).is_some_and(|value| !value.is_empty()));
    if env::var_os("CARGO_CFG_DEBUG_ASSERTIONS").is_some()
        || env::var_os("CARGO_FEATURE_DEMO").is_some()
        || development
    {
        println!("cargo::rustc-cfg=demo");
    }
}
