//! The GRKit properties of the XMP packet of a photo (`runtime/photo_xmp.h`).
//!
//! A JPEG (APP1) and a DNG (tag 700 of IFD0) get one packet. Each setting
//! that changes the picture has a property with the English text of its
//! value, so that exiftool and other XMP readers show it. `grkit:State` holds
//! the state file ID and the number of each setting: a RAW development reads
//! only it, so a later change of a name or a text keeps old photos readable.
//! The look name is also a `dc:subject` keyword, which Lightroom shows.
//!
//! The property names are a public format: add properties, but do not
//! rename them.

use crate::capabilities;
use crate::runtime::state_key_id;

/// The namespace of the GRKit properties, with the prefix `grkit`.
pub const NAMESPACE: &str = "https://grkit.app/ns/1.0/";

/// Each setting that changes the picture, as `MODULE.KEY`, and its property.
/// `CORE.ASPECT` and `CORE.CROP` name only a crop: in their frame-only modes
/// the packet leaves them out (`runtime/photo_xmp.h`).
pub const SETTINGS: &[(&str, &str)] = &[
    ("CORE.GRAIN", "Grain"),
    ("CORE.GRAIN_SIZE", "GrainSize"),
    ("GLOW.BLOOM", "Bloom"),
    ("GLOW.HALATION", "Halation"),
    ("GLOW.HALATION_COLOR", "HalationColor"),
    ("CORE.BORDER", "Border"),
    ("CORE.BORDER_COLOR", "BorderColor"),
    ("CORE.ASPECT", "Framing"),
    ("CORE.CROP", "Crop"),
    ("DATE.STYLE", "DateImprint"),
    ("DATE.LOOK", "DateLook"),
    ("DATE.POSITION", "DatePosition"),
];

/// The core handle of a `CORE.KEY` setting, or 0 for a module setting.
pub fn core_handle(setting: &str) -> u32 {
    setting
        .strip_prefix("CORE.")
        .and_then(|key| capabilities::CAPABILITIES.iter().position(|c| c.key == key))
        .map_or(0, |i| capabilities::HANDLE_BASE + i as u32)
}

/// The C initializer of the settings: `{state ID, core handle, "Property"}`.
pub fn c_settings() -> String {
    SETTINGS
        .iter()
        .map(|(setting, property)| {
            format!(
                "{{{}U,{:#x}U,\"{}\"}}",
                state_key_id(setting),
                core_handle(setting),
                property
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn properties_are_xml_names_and_unique() {
        let mut seen = std::collections::BTreeSet::new();
        for (setting, property) in SETTINGS {
            assert!(
                property.starts_with(|c: char| c.is_ascii_uppercase())
                    && property.chars().all(|c| c.is_ascii_alphanumeric()),
                "{property}"
            );
            assert!(seen.insert(*property), "{property}");
            assert!(seen.insert(*setting), "{setting}");
        }
    }

    #[test]
    fn each_setting_exists() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../modules");
        for (setting, _) in SETTINGS {
            let (module, key) = setting.split_once('.').unwrap();
            if module == "CORE" {
                assert_ne!(core_handle(setting), 0, "{setting}");
                continue;
            }
            let manifest =
                std::fs::read_to_string(root.join(module.to_ascii_lowercase()).join("module.toml"))
                    .unwrap();
            assert!(
                manifest.contains(&format!("id = \"{module}\""))
                    && manifest.contains(&format!("key = \"{key}\"")),
                "{setting}"
            );
        }
    }
}
