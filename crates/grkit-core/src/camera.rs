//! Cameras that a package can target.
//!
//! GR IIIx 1.60 runs the RTOS code of GR III 2.10. The two RTOS images differ only
//! in the model name, version, build time, default year, ModeSet numbers, and one
//! byte of the model selection, and none of these bytes is a GRKit patch site or
//! input. A runtime built for `gr3-2.10` is therefore valid on both cameras. The
//! cameras differ only in the ModeSet file that enables SD scripts.
//!
//! GR IV 1.11 is a different build with its own runtime. It has Image Control,
//! Framing (XPan only, cropped from native 16:9), Guides, and Date Stamp, and it
//! has not been tested on a camera.

use anyhow::{bail, Result};

use crate::modules;

#[derive(Debug, PartialEq, Eq)]
pub struct Camera {
    /// The ID that the CLI, the site, and `grkit verify` use.
    pub id: &'static str,
    pub name: &'static str,
    pub firmware: &'static str,
    /// The runtime build that the camera runs.
    pub target: &'static str,
    /// The ModeSet command file: the major and minor numbers of the RTOS.
    pub modeset: &'static str,
    /// True until the runtime has been tested on a camera.
    pub experimental: bool,
    /// True when the runtime has Image Control, so a package can have effects.
    pub effects: bool,
    /// The modules of `modules::NAMES` that the runtime has. The camera menu
    /// never shows the others.
    pub modules: &'static [&'static str],
}

pub const GR3: Camera = Camera {
    id: "gr3",
    name: "GR III",
    firmware: "2.10",
    target: "gr3-2.10",
    modeset: "00078350.588",
    experimental: false,
    effects: true,
    modules: &modules::NAMES,
};

pub const GR3X: Camera = Camera {
    id: "gr3x",
    name: "GR IIIx",
    firmware: "1.60",
    target: "gr3-2.10",
    modeset: "00078490.609",
    experimental: false,
    effects: true,
    modules: &modules::NAMES,
};

/// GR IV and GR IV HDF run the same firmware and ModeSet file.
pub const GR4: Camera = Camera {
    id: "gr4",
    name: "GR IV",
    firmware: "1.11",
    target: "gr4-1.11",
    modeset: "00078560.636",
    experimental: true,
    effects: true,
    modules: &modules::NAMES,
};

pub const CAMERAS: [&Camera; 3] = [&GR3, &GR3X, &GR4];

pub fn find(id: &str) -> Result<&'static Camera> {
    match CAMERAS.iter().find(|camera| camera.id == id) {
        Some(camera) => Ok(camera),
        None => bail!(
            "Unknown camera {id:?}. Expected one of: {}",
            CAMERAS.map(|camera| camera.id).join(", ")
        ),
    }
}
