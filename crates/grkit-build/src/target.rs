//! Firmware builds that GRKit can build a runtime for.
//!
//! A target pins one official firmware release and holds the addresses that the
//! loader and the installer use on it. The RTOS of each target loads at
//! `RTOS_BASE`, and its startup code copies the data section from `data_source`
//! to `data_start` and clears the BSS up to `heap_min`.

use anyhow::{bail, Result};

use crate::components::{Layout, GR3_LAYOUT, GR4_LAYOUT};

pub use grkit_core::runtime::HEADER_REFERENCE;

pub const RTOS_BASE: u32 = 0x53000000;

#[derive(Debug, PartialEq, Eq)]
pub struct Target {
    pub name: &'static str,
    /// The model and version, for messages.
    pub firmware: &'static str,
    pub firmware_sha256: &'static str,
    pub decoded_sha256: &'static str,
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
    /// The runtime components and their hooks. Without a layout, the runtime has
    /// the loader and the installer only.
    pub layout: Option<&'static Layout>,
    /// Native Image Control data, if the layout has processing hooks.
    pub image_control: Option<&'static ImageControl>,
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

pub const GR3_IMAGE_CONTROL: ImageControl = ImageControl {
    tables: [0x55018d94, 0x55018cf0, 0x55018e4c],
    table_bytes: 104,
    record: 0x5501900c,
    coefficients: 0x53dc2754,
    record_tail: [0x53dc27b4, 0x53dc2938, 0x53dc29dc, 0x53dc2444],
    base_gamma: 0x53dcf7f0,
    settings_domain: 0x553458b0,
    settings_type: 0x77c,
    type_count: 26,
};

/// GR IV tables hold 0x22 entries for each of three model variants, which are
/// equal in 1.11. GR IV computes the base gamma with the GR III function from
/// the same curves, so its base gamma is the GR III one.
pub const GR4_IMAGE_CONTROL: ImageControl = ImageControl {
    tables: [0x55053780, 0x55053504, 0x55052cf4],
    table_bytes: 3 * 0x22 * 4,
    record: 0x5501adbc,
    coefficients: 0x54049850,
    record_tail: [0x540498d8, 0x54049a70, 0x54049b28, 0x54049518],
    base_gamma: 0x5407f4b4,
    settings_domain: 0x55379610,
    settings_type: 0x79c,
    type_count: 31,
};

pub const GR3_210: Target = Target {
    name: "gr3-2.10",
    firmware: "GR III 2.10",
    firmware_sha256: "d6a3c9c288d784080e7b283222ecdd8752adfeed6faa36dab44f73115354bfd4",
    decoded_sha256: "53da690797f35ad6dc2c429d8ca1d0de8e333c211ed959816813982cd093b703",
    data_start: 0x55000000,
    data_source: 0x5407b540,
    data_size: 0x3a080,
    array_table: 0x55380f6c,
    array_count: 0x5537fb58,
    heap_min: 0x55804374,
    heap_end: 0x60000000,
    english_slot: 0x55002bb0,
    pixel_base_slot: 0x55018734,
    pixel_base_indirect: false,
    layout: Some(&GR3_LAYOUT),
    image_control: Some(&GR3_IMAGE_CONTROL),
};

/// GR IV and GR IV HDF 1.11. The runtime has the private menu, Image Control,
/// Guides, and Date Stamp so far.
pub const GR4_111: Target = Target {
    name: "gr4-1.11",
    firmware: "GR IV 1.11",
    firmware_sha256: "a2f664dfca034059eb0fd6e18ab08684c326b4a034d85c164dad7e1ec9b5655f",
    decoded_sha256: "c4e597c7c9ca1bc181b30e35139ed90a4fd876ddcca14f12be0e00b6702233ae",
    data_start: 0x55000000,
    data_source: 0x5437b640,
    data_size: 0x57480,
    array_table: 0x553b3ec4,
    array_count: 0x553b2ab0,
    heap_min: 0x558372d4,
    heap_end: 0x60000000,
    english_slot: 0x5500211c,
    // The drawer of the icon manager singleton (0x55082028 + 0x10). GR IV draws
    // icons from a copy of the icon pixels, not from the ICONBIN load address.
    pixel_base_slot: 0x55082038,
    pixel_base_indirect: true,
    layout: Some(&GR4_LAYOUT),
    image_control: Some(&GR4_IMAGE_CONTROL),
};

pub const TARGETS: [&Target; 2] = [&GR3_210, &GR4_111];

pub fn find(name: &str) -> Result<&'static Target> {
    match TARGETS.iter().find(|target| target.name == name) {
        Some(target) => Ok(target),
        None => bail!(
            "Unknown target {name:?}. Expected one of: {}",
            TARGETS.map(|target| target.name).join(", ")
        ),
    }
}
