//! Firmware builds that GRKit can build a runtime for.
//!
//! A target pins one official firmware release and holds the addresses that the
//! loader and the installer use on it. The RTOS of each target loads at
//! `RTOS_BASE`, and its startup code copies the data section from `data_source`
//! to `data_start` and clears the BSS up to `heap_min`.
//!
//! Each family has a module here with its hooks, which joins the hooks that the
//! families share (`common`) and its own, and each target a module with its
//! data and the sites of the hooks in its build (`Target::module`).
//! `grkit port` drafts the module of a new build.

use anyhow::{bail, Result};

use crate::components::{HookSpec, Layout};

pub use grkit_core::runtime::HEADER_REFERENCE;

pub mod common;
pub mod gr3;
pub mod gr3_210;
pub mod gr4;
pub mod gr4_111;
pub mod gr4_120;
pub mod gr4_120_0528;

pub use gr3_210::{IMAGE_CONTROL as GR3_IMAGE_CONTROL, TARGET as GR3_210};
pub use gr4_111::{IMAGE_CONTROL as GR4_IMAGE_CONTROL, TARGET as GR4_111};
pub use gr4_120::{IMAGE_CONTROL as GR4_120_IMAGE_CONTROL, TARGET as GR4_120};
pub use gr4_120_0528::TARGET as GR4_120_0528;

pub const RTOS_BASE: u32 = 0x53000000;
/// The RTOS word that names the firmware version and its build, such as
/// `01 14 0a 0b` (little-endian 0x0b0a1401) for GR IV 1.20.
pub const RTOS_IDENTITY: u32 = 0x53002038;

pub const TARGETS: [&Target; 4] = [&GR3_210, &GR4_111, &GR4_120, &GR4_120_0528];

/// The zip that holds the firmware file of a target.
#[derive(Debug, PartialEq, Eq)]
pub struct Download {
    pub zip: &'static str,
    pub zip_sha256: &'static str,
    /// The firmware file in the zip, which `grkit build` takes.
    pub file: &'static str,
    pub source: Source,
}

/// Where the release gets the zip of a target.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Source {
    /// The Ricoh download page. The private backup holds the same zip.
    Ricoh,
    /// Only the private backup. Ricoh does not publish the build, so `file` is
    /// the RTOS image that the dump tool copied from a camera, and the SHA-256
    /// values of the target are those of that image.
    Dump,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Target {
    pub name: &'static str,
    /// The camera line of the build. The name starts with its name and `-`.
    pub family: &'static Family,
    /// The model and version, for messages.
    pub firmware: &'static str,
    pub firmware_sha256: &'static str,
    pub decoded_sha256: &'static str,
    /// The official download that holds the firmware file. The release builds
    /// the runtime from it.
    pub download: Download,
    /// The `RTOS_IDENTITY` words of the builds that the runtime is for. When a
    /// package has a runtime for each build of a camera, an installer installs
    /// at once on a build that it names, and the others try it only when no
    /// installer names the build (`runtime::runtime_files_for`). A camera with
    /// one runtime needs none.
    pub identities: &'static [u32],
    pub data_start: u32,
    pub data_source: u32,
    pub data_size: u32,
    /// The script interpreter's table of integer arrays and its array count.
    pub array_table: u32,
    pub array_count: u32,
    /// Script arrays lie in `heap_min..heap_end`. `heap_min` is the BSS end.
    pub heap_min: u32,
    pub heap_end: u32,
    /// The English entry of the language table. The installer waits until it
    /// holds its value from the firmware image.
    pub english_slot: u32,
    /// Where the base of icon pixel offsets is: a global that holds it, or with
    /// `pixel_base_indirect`, a global that points to the icon drawer whose first
    /// word holds it.
    pub pixel_base_slot: u32,
    pub pixel_base_indirect: bool,
    /// The high byte of the native ImageTone values that photos record
    /// besides the GR III values, if any: 0x81 on GR IV (0x8100 and up).
    pub native_tone_page: Option<u32>,
    /// The pointer to the current model record, and the offset of the byte
    /// of the record that `SystemConfig_GetImageSensorColorType` returns: 1
    /// on GR IV Monochrome. The installer leaves out color-only module
    /// settings rows on a monochrome camera. None for a camera line without
    /// a monochrome model.
    pub model_record: Option<(u32, u32)>,
    /// The runtime components and their hooks.
    pub layout: &'static Layout,
    /// Native Image Control data, if the family has processing hooks.
    pub image_control: Option<&'static ImageControl>,
    /// The installer data that no firmware code
    /// loads directly: (address, the address that code loads). The address
    /// keeps its offset from the loaded one.
    pub relocation_bases: &'static [(u32, u32)],
}

/// The builds of one camera line, such as each GR IV build. They share the
/// runtime code, the hooks, and the header `runtime/targets/<family>.h`.
/// Builds of a family also have the same header definitions (tests), so a
/// change for one build fails until it reaches each build.
#[derive(Debug, PartialEq, Eq)]
pub struct Family {
    pub name: &'static str,
    /// The hooks of each build, in installation order. A build gives the site
    /// of each hook (`Layout::sites`).
    pub hooks: &'static [HookSpec],
    /// The Image Control processing source, if the family has processing hooks.
    pub processing_source: Option<&'static str>,
}

/// The native Image Control data that the installer checks and copies.
#[derive(Debug, PartialEq, Eq)]
pub struct ImageControl {
    /// The three type tables. Entry 11 of the first one is the Standard record.
    pub tables: [u32; 3],
    /// The bytes of each table that the installer compares.
    pub table_bytes: u32,
    /// The Standard record and the coefficient pointer at its +4.
    pub record: u32,
    pub coefficients: u32,
    /// The four pointers that the firmware writes at record +0x44 after startup.
    pub record_tail: [u32; 4],
    /// The Standard base gamma that the effect fit assumes: 256 u16 entries.
    /// The firmware computes each build's base from 15 ROM curves, the tone
    /// setting, and a gain. This is the curve for tone 0 and the lowest gain.
    pub base_gamma: u32,
    /// The word that holds the settings domain, the offset of the native Image
    /// Control type in the settings payload, and the number of native types.
    pub settings_domain: u32,
    pub settings_type: u32,
    pub type_count: u32,
}

impl Target {
    /// The runtime header of the build in `runtime/` (`runtime/firmware.h`
    /// selects it).
    pub fn header(&self) -> String {
        format!("targets/{}.h", self.name)
    }

    /// The module of the target in `target/`: the name with `_` for `-` and
    /// without `.`, such as `gr4_120`.
    pub fn module(&self) -> String {
        self.name.replace('-', "_").replace('.', "")
    }

    /// The bytes of a native icon descriptor before the pixels, from the icon
    /// slot of the family: 8 on GR III, 12 on GR IV (width and a valid flag,
    /// height, pixel offset).
    pub fn icon_descriptor_bytes(&self) -> Result<usize> {
        Ok(grkit_core::runtime::icon_stride(self.name)? - grkit_core::effects::ICON_SIZE)
    }
}

pub fn find(name: &str) -> Result<&'static Target> {
    match TARGETS.iter().find(|target| target.name == name) {
        Some(target) => Ok(target),
        None => bail!(
            "Unknown target {name:?}. Expected one of: {}",
            TARGETS.map(|target| target.name).join(", ")
        ),
    }
}
