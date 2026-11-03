//! The private runtime layout and the production loader templates. The layout
//! values are those of GR III 2.10 unless a function takes the target.
//!
//! `state` has the state files of the resident loader, `loader` the loader
//! script, and `files` the runtime files of a package. This module exports
//! their items.

use anyhow::{ensure, Result};
use serde_json::{json, Value};

pub use crate::abi::{
    HEADER_REFERENCE, HOST_ACTIVE, HOST_CAPABILITIES, HOST_COUNT, HOST_FRAME_RATIO,
    HOST_HEADER_SIZE as HEADER_SIZE, HOST_RECORDS, HOST_STATE_CHANGES, PICTURE_SCRATCH,
    RECORD_CODE, RECORD_PARAMETERS, RECORD_SIZE, RECORD_TONING, TAIL_SIZE,
};
pub use crate::abi::{
    HOST_ICON_COUNT, ICONS_PER_CHUNK, ICON_CHUNK_SLOTS, MAX_COUNT, MODULES_MAX, MODULE_ARRAY_WORDS,
    MODULE_SETTINGS_MAX, MODULE_TILES_MAX,
};

mod files;
mod loader;
mod state;

pub use files::*;
pub use loader::*;
pub use state::*;

/// The icon slot of GR III: an 8-byte native descriptor, then 40 x 40 RGBA.
pub const ICON_STRIDE: usize = 6408;
/// The icon slot of each camera family. GR IV descriptors have 12 bytes.
/// The family header in `runtime/targets` defines the same value.
const FAMILY_ICON_STRIDES: [(&str, usize); 2] = [("gr3", ICON_STRIDE), ("gr4", 6412)];

/// The icon slot size of a runtime target that a camera release runs. The
/// slot belongs to the family, the target name before its first `-`.
pub fn icon_stride(target: &str) -> Result<usize> {
    let family = target.split_once('-').map(|(family, _)| family);
    match FAMILY_ICON_STRIDES
        .iter()
        .find(|(name, _)| Some(*name) == family)
    {
        Some(&(_, stride)) if crate::camera::CAMERAS.iter().any(|c| c.runs(target)) => Ok(stride),
        _ => anyhow::bail!("Unknown runtime target {target:?}"),
    }
}
/// The most values of a module setting.
pub const MODULE_SETTING_VALUES: usize = 65536;
/// The most values of a row of a native settings page (`SETTING_VALUES` in
/// runtime/core.h): one for each module that the Modules page lists.
pub const NATIVE_SETTING_VALUES: usize = 8;
/// The most settings rows of a module page (`GRM_SETTING_ROWS`). A page has 0,
/// or 2 to this many rows. The native settings page shows one row more
/// (`SETTING_ROWS` in `runtime/core.h`): in MENU, the Enable row of the
/// module comes first.
pub const NATIVE_SETTING_ROWS: usize = 5;
/// The words of the tail array: the picture scratch, the menu text, and the
/// live-view labels (`abi::TAIL_SIZE`).
pub const TAIL_WORDS: usize = crate::abi::TAIL_SIZE / 4;
// One script array holds the header, every record, the scratch, and the text: at most 65536 words.
const _: () = assert!(HEADER_SIZE + MAX_COUNT * RECORD_SIZE <= 65536 * 4);
const _: () = assert!(TAIL_SIZE <= 65536 * 4);
const _: () = assert!(PICTURE_SCRATCH.is_multiple_of(4) && TAIL_SIZE.is_multiple_of(4));
/// One script array holds one icon chunk: 40 slots of at most 6412 bytes stay
/// below 65536 words.
pub const MAX_CHUNKS: usize =
    (MAX_COUNT + HOST_ICON_COUNT + MODULE_TILES_MAX).div_ceil(ICONS_PER_CHUNK);
const _: () = assert!(MAX_CHUNKS <= ICON_CHUNK_SLOTS);
const _: () = assert!(ICONS_PER_CHUNK * 6412 <= 65536 * 4);

pub fn layout(count: usize) -> Result<Value> {
    ensure!(count <= MAX_COUNT, "Expected 0..{MAX_COUNT} effects");
    let icon_words: Vec<_> = (0..count + HOST_ICON_COUNT)
        .step_by(ICONS_PER_CHUNK)
        .map(|start| ICONS_PER_CHUNK.min(count + HOST_ICON_COUNT - start) * ICON_STRIDE / 4)
        .collect();
    Ok(
        json!({"words": (HEADER_SIZE + count * RECORD_SIZE) / 4, "tail_words": TAIL_WORDS, "icon_words": icon_words}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The installer, the module SDK, and the menu agree on the values of a
    /// native settings row.
    #[test]
    fn native_setting_rows_have_one_value_limit() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtime");
        for (file, name) in [
            ("module_link.h", "GRM_SETTING_ROW_VALUES"),
            ("sdk/grkit.h", "GRKIT_SETTING_ROW_VALUES"),
            ("core.h", "SETTING_VALUES"),
        ] {
            let text = std::fs::read_to_string(root.join(file)).unwrap();
            let value = text
                .lines()
                .find_map(|line| line.strip_prefix(&format!("#define {name} ")))
                .and_then(|value| value.trim_end_matches('U').parse::<usize>().ok());
            assert_eq!(value, Some(NATIVE_SETTING_VALUES), "{file}");
        }
        for (file, name, rows) in [
            ("module_link.h", "GRM_SETTING_ROWS", NATIVE_SETTING_ROWS),
            ("sdk/grkit.h", "GRKIT_SETTING_ROWS", NATIVE_SETTING_ROWS),
            ("core.h", "SETTING_ROWS", NATIVE_SETTING_ROWS + 1),
        ] {
            let text = std::fs::read_to_string(root.join(file)).unwrap();
            let value = text
                .lines()
                .find_map(|line| line.strip_prefix(&format!("#define {name} ")))
                .and_then(|value| value.trim_end_matches('U').parse::<usize>().ok());
            assert_eq!(value, Some(rows), "{file}");
        }
    }
}
