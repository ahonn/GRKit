//! Cameras that a package can target.
//!
//! GR IIIx 1.60 runs the RTOS code of GR III 2.10. The two RTOS images differ only
//! in the model name, version, build time, default year, ModeSet numbers, and one
//! byte of the model selection, and none of these bytes is a GRKit patch site or
//! input. A runtime built for `gr3-2.10` is therefore valid on both cameras. The
//! cameras differ only in the ModeSet file that enables SD scripts.

use anyhow::{bail, Result};

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
}

pub const GR3: Camera = Camera {
    id: "gr3",
    name: "GR III",
    firmware: "2.10",
    target: "gr3-2.10",
    modeset: "00078350.588",
};

pub const GR3X: Camera = Camera {
    id: "gr3x",
    name: "GR IIIx",
    firmware: "1.60",
    target: "gr3-2.10",
    modeset: "00078490.609",
};

pub const CAMERAS: [&Camera; 2] = [&GR3, &GR3X];

pub fn find(id: &str) -> Result<&'static Camera> {
    match CAMERAS.iter().find(|camera| camera.id == id) {
        Some(camera) => Ok(camera),
        None => bail!(
            "Unknown camera {id:?}. Expected one of: {}",
            CAMERAS.map(|camera| camera.id).join(", ")
        ),
    }
}
