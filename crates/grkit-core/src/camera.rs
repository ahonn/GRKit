//! Cameras that a package can target.
//!
//! GR IIIx 1.60 runs the RTOS code of GR III 2.10. The two RTOS images differ only
//! in the model name, version, build time, default year, ModeSet numbers, and one
//! byte of the model selection, and none of these bytes is a GRKit patch site or
//! input. A runtime built for `gr3-2.10` is therefore valid on both cameras. The
//! cameras differ only in the ModeSet file that enables SD scripts.
//!
//! A camera lists each firmware release that it can run with the runtime build
//! for it. GR III 2.20 and GR IIIx 1.70 change only the end screen; their code
//! moves by less than the relocation radius, so the installer relocates the
//! `gr3-2.10` runtime to them (docs/architecture.md, Relocation).
//!
//! Each camera has an HDF model that runs the same firmware and ModeSet file:
//! Ricoh publishes one firmware for both. The RTOS selects the HDF model from a
//! factory adjustment, not from the ModeSet numbers. A package for a camera is
//! therefore also the package for its HDF model.
//!
//! GR IV 1.11 is a different build with its own runtime. GR IV 1.20 moves and
//! changes code beyond the relocation radius, so it has a runtime of its own,
//! `gr4-1.20`, which has not been tested on a camera. GR IV 30th Anniversary
//! Edition cameras have another 1.20 build from the factory, which mixes code
//! of 1.11 and 1.20, so it has the runtime `gr4-1.20-0528`. Each has
//! Image Control, Framing, Guides, Date Stamp, and Grain. The 1.11 runtime has
//! been tested on a GR IV camera, but not on a GR IV HDF camera.
//!
//! A camera has one package. When the camera has several runtime builds, the
//! package holds the installer of each, and the camera selects the installer
//! of its build at each power-on (`runtime::package_runtime_files`).

use anyhow::{bail, Result};

/// A firmware version that a camera runs, and the runtime build for it. Two
/// builds of one version are two releases with the same version.
#[derive(Debug, PartialEq, Eq)]
pub struct Release {
    pub firmware: &'static str,
    pub target: &'static str,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Camera {
    /// The ID that the CLI, the site, and `grkit verify` use.
    pub id: &'static str,
    pub name: &'static str,
    /// The HDF model, which runs the same firmware and ModeSet file.
    pub hdf: &'static str,
    /// The firmware releases that a GRKit runtime supports, oldest first.
    pub releases: &'static [Release],
    /// The ModeSet command file: the major and minor numbers of the RTOS.
    pub modeset: &'static str,
    /// True when the runtime has Image Control, so a package can have effects.
    pub effects: bool,
}

pub const GR3: Camera = Camera {
    id: "gr3",
    name: "GR III",
    hdf: "GR III HDF",
    releases: &[
        Release {
            firmware: "2.10",
            target: "gr3-2.10",
        },
        Release {
            firmware: "2.20",
            target: "gr3-2.10",
        },
    ],
    modeset: "00078350.588",
    effects: true,
};

pub const GR3X: Camera = Camera {
    id: "gr3x",
    name: "GR IIIx",
    hdf: "GR IIIx HDF",
    releases: &[
        Release {
            firmware: "1.60",
            target: "gr3-2.10",
        },
        Release {
            firmware: "1.70",
            target: "gr3-2.10",
        },
    ],
    modeset: "00078490.609",
    effects: true,
};

pub const GR4: Camera = Camera {
    id: "gr4",
    name: "GR IV",
    hdf: "GR IV HDF",
    releases: &[
        Release {
            firmware: "1.11",
            target: "gr4-1.11",
        },
        Release {
            firmware: "1.20",
            target: "gr4-1.20",
        },
        // The factory build of GR IV 30th Anniversary Edition cameras.
        Release {
            firmware: "1.20",
            target: "gr4-1.20-0528",
        },
    ],
    modeset: "00078560.636",
    effects: true,
};

pub const CAMERAS: [&Camera; 3] = [&GR3, &GR3X, &GR4];

impl Camera {
    /// The runtime builds that the camera runs, in release order, each once.
    pub fn targets(&self) -> Vec<&'static str> {
        let mut targets = Vec::new();
        for release in self.releases {
            if !targets.contains(&release.target) {
                targets.push(release.target);
            }
        }
        targets
    }

    /// Whether a runtime build runs on the camera.
    pub fn runs(&self, target: &str) -> bool {
        self.releases.iter().any(|release| release.target == target)
    }

    /// The firmware versions of the releases, oldest first, each once.
    pub fn versions(&self) -> Vec<&'static str> {
        let mut versions = Vec::new();
        for release in self.releases {
            if !versions.contains(&release.firmware) {
                versions.push(release.firmware);
            }
        }
        versions
    }

    /// The firmware versions that run a runtime build, oldest first.
    pub fn firmware_for(&self, target: &str) -> Vec<&'static str> {
        self.releases
            .iter()
            .filter(|release| release.target == target)
            .map(|release| release.firmware)
            .collect()
    }

    /// The camera family: GR IV cameras have their own guide and layout.
    pub fn is_gr4(&self) -> bool {
        self.id == GR4.id
    }
}

/// `web/src/generated/cameras.ts`: the ModeSet file and the firmware releases
/// of each camera, so the site needs no copy of them.
pub fn site_module() -> String {
    use std::fmt::Write;
    let mut out = String::from(
        "// Generated from crates/grkit-core/src/camera.rs. Do not edit: change the\n\
         // cameras there, then run\n\
         // GRKIT_UPDATE_GENERATED=1 cargo test -p grkit-core --test generated.\n\n\
         /** The ModeSet file of each camera, and the firmware releases that a\n \
         * runtime build supports, oldest first. */\n\
         export const CAMERA_RELEASES = {\n",
    );
    for camera in CAMERAS {
        writeln!(out, "  {}: {{", camera.id).unwrap();
        writeln!(out, "    modeset: '{}',", camera.modeset).unwrap();
        out.push_str("    releases: [\n");
        for release in camera.releases {
            writeln!(
                out,
                "      {{ firmware: '{}', target: '{}' }},",
                release.firmware, release.target
            )
            .unwrap();
        }
        out.push_str("    ],\n  },\n");
    }
    out.push_str("} as const;\n");
    out
}

/// The generated files, as paths from the repository root and contents.
pub fn generated() -> Vec<(&'static str, String)> {
    vec![("web/src/generated/cameras.ts", site_module())]
}

pub fn find(id: &str) -> Result<&'static Camera> {
    match CAMERAS.iter().find(|camera| camera.id == id) {
        Some(camera) => Ok(camera),
        None => bail!(
            "Unknown camera {id:?}. Expected one of: {}",
            CAMERAS.map(|camera| camera.id).join(", ")
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_release_names_its_runtime_build() {
        assert_eq!(GR3.targets(), ["gr3-2.10"]);
        assert_eq!(GR3.firmware_for("gr3-2.10"), ["2.10", "2.20"]);
        assert_eq!(GR3X.firmware_for("gr3-2.10"), ["1.60", "1.70"]);
        assert!(GR3X.runs("gr3-2.10") && !GR3X.runs("gr4-1.11"));
        assert_eq!(GR4.targets(), ["gr4-1.11", "gr4-1.20", "gr4-1.20-0528"]);
        assert_eq!(GR4.firmware_for("gr4-1.20"), ["1.20"]);
        assert_eq!(GR4.firmware_for("gr4-1.20-0528"), ["1.20"]);
        assert_eq!(GR4.versions(), ["1.11", "1.20"]);
    }
}
