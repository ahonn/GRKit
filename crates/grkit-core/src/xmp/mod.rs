//! Lightroom / Camera Raw `.xmp` presets as display-referred 3D LUTs.
//!
//! The preset is a pure `Rgb -> Rgb` on display sRGB in 0..1. Exposure, grain, and
//! incremental white-balance hints are returned as data and are not baked into the LUT.
//! By default, [`prepare`] composes a Standard→reference tone [`bridge`] before the preset.

mod bridge;
mod parse;
pub mod table;
mod tone_data;
mod transform;
pub(crate) mod xml;

use anyhow::{ensure, Result};

use crate::cube::Cube;

pub use bridge::{apply as apply_bridge, WARNING as BRIDGE_WARNING};
pub use parse::{looks_like, parse, Look, Preset, Setting};
pub use table::{decode_big_table, LookTable, RgbTable};
pub use transform::{
    apply, blacks_curve, contrast2012, highlights_curve, hints, hue_weights, sample_cube,
    sample_cube_with, shadows_curve, warnings, whites_curve, Hints, Pchip, TableNotes, Transform,
    WhiteBalance, HUE_BANDS, LUT_SIZE,
};

/// A preset turned into a LUT for `fit::import`, with the settings that were skipped or only
/// approximated, and the hints that stay outside the color transform.
#[derive(Clone, Debug)]
pub struct Prepared {
    pub cube: Cube,
    pub warnings: Vec<String>,
    pub hints: Hints,
}

/// Parses an XMP preset and samples its color transform into a 33³ LUT, with the base bridge on.
pub fn prepare(bytes: &[u8]) -> Result<Prepared> {
    prepare_with(bytes, true)
}

/// [`prepare`], with an explicit choice of the Standard→reference tone bridge.
pub fn prepare_with(bytes: &[u8], base_bridge: bool) -> Result<Prepared> {
    let preset = parse(bytes)?;
    let transform = Transform::new(&preset)?;
    let mut warnings = warnings(&preset, &transform);
    if base_bridge {
        warnings.insert(0, bridge::WARNING.to_owned());
    }
    let hints = hints(&preset);
    let cube = sample_cube_with(&transform, preset.name.clone(), base_bridge)?;
    ensure!(
        cube.three_d && cube.size == LUT_SIZE,
        "Internal error: preset LUT size"
    );
    Ok(Prepared {
        cube,
        warnings,
        hints,
    })
}
