//! The native picture settings that camera recipes hold: the Image Control
//! type of the main set and its parameters, the white balance, and the
//! correction and noise reduction settings of still pictures.
//!
//! Recipes write them through the RTOS shell command `UserData Set <name>
//! <label>` and read them from the settings record (`runtime/recipes.h`).
//! A GR IV Monochrome type has no label, so the type setter takes its stored
//! value.
//! This module is the one source of the portable model: each type,
//! parameter, mode, and setting has a portable index here, and the camera and
//! the site name it by that index. Each camera family gives the names and
//! the labels. `grkit-build` checks each family against the firmware of its
//! builds (`grkit_build::userdata`) and writes the field offsets of each
//! build.
mod header;
mod tables;
mod types;

pub use header::header;
pub use tables::*;
pub use types::{ImageType, TYPES};

#[cfg(test)]
mod tests {
    use super::*;
    use types::NINE;

    #[test]
    fn each_type_names_known_parameters_with_labels() {
        for family in Family::ALL {
            for (index, image_type) in TYPES.iter().enumerate() {
                let Some((_, keys)) = image_type.families[family.index()] else {
                    continue;
                };
                for key in keys {
                    let parameter = parameter(key);
                    assert!(
                        parameter_labels(family, parameter).is_some(),
                        "{family:?} {} {key}",
                        image_type.label
                    );
                    assert!(parameter_name(family, index, parameter).is_some());
                }
                // A type has at most one tone parameter.
                let tones = keys
                    .iter()
                    .filter(|key| key.starts_with("ColorTone"))
                    .count();
                assert!(tones <= 1, "{}", image_type.label);
            }
        }
        assert_eq!(
            &PARAMETERS[..LEVEL_COUNT]
                .iter()
                .map(|p| p.key)
                .collect::<Vec<_>>(),
            NINE
        );
    }

    #[test]
    fn the_monochrome_types_come_last() {
        for (index, image_type) in TYPES.iter().enumerate() {
            assert_eq!(
                image_type.monochrome,
                index >= FIRST_MONOCHROME,
                "{}",
                image_type.label
            );
            if image_type.monochrome {
                assert!(image_type.families[Family::Gr3.index()].is_none());
            }
            for (key, labels) in image_type.overrides {
                let parameter = parameter(key);
                let family = parameter_labels(Family::Gr4, parameter).unwrap();
                // The camera tables hold the labels of the parameter, so an
                // override names no other label.
                for (label, value) in labels.values() {
                    assert!(
                        family.values().any(|other| other == (label, value)),
                        "{key}"
                    );
                }
            }
        }
        let hdr = TYPES
            .iter()
            .position(|t| t.label == "MonochromeHdrCuisine")
            .unwrap();
        let tone = parameter("ColorToneHdrIntense");
        assert_eq!(
            type_parameter_labels(Family::Gr4, hdr, tone)
                .unwrap()
                .values()
                .collect::<Vec<_>>(),
            [("OFF", 0), ("Sepia", 2)]
        );
    }

    #[test]
    fn names_follow_the_firmware_patterns() {
        let negative = TYPES
            .iter()
            .position(|t| t.label == "NegativeFilm")
            .unwrap();
        let saturation = parameter("Saturation");
        assert_eq!(
            parameter_name(Family::Gr3, negative, saturation).unwrap(),
            "AfterReleaseStillImageControlNo0ParamNegativeFilmSaturation"
        );
        assert_eq!(
            parameter_name(Family::Gr4, negative, saturation).unwrap(),
            "StillImageControlNo0ParamNegativeFilmSaturation"
        );
        let monotone = TYPES.iter().position(|t| t.label == "MonotoneGR").unwrap();
        assert!(parameter_name(Family::Gr3, monotone, saturation).is_none());
        assert_eq!(
            shift_names(Family::Gr4, 0).unwrap()[0],
            "StillWhiteBalanceAutoStandardAdjustBA"
        );
        assert!(shift_names(Family::Gr3, 2).is_none());
        assert!(shift_names(Family::Gr3, 15).is_none());
        assert_eq!(Family::of_target("gr4-1.20-0528"), Some(Family::Gr4));
    }
}
