//! The Image Control types of the main set in portable order.

use super::tables::{labels, Labels, Prefix};

/// An Image Control type of the main set.
pub struct ImageType {
    /// The label of the type for `StillImageControlNo0`, and the name of its
    /// parameters.
    pub label: &'static str,
    /// In each family: the prefix of its parameter names and its parameters
    /// in portable order, or `None` where the family has no such type.
    pub families: [Option<(Prefix, &'static [&'static str])>; 2],
    /// A type of GR IV Monochrome. `StillImageControlNo0` has no label for
    /// it: the type setter takes its stored value, which `grkit-build` finds
    /// in the firmware. A build can lack a Monochrome type that a later
    /// build adds.
    pub monochrome: bool,
    /// The labels of a parameter of this type that differ from those of the
    /// parameter, by key.
    pub overrides: &'static [(&'static str, Labels)],
}

pub(super) const NINE: &[&str] = &[
    "Saturation",
    "ColorHue",
    "ImageKey",
    "Contrast",
    "ContrastHighLight",
    "ContrastShadow",
    "Sharpness",
    "Shading",
    "ClarityControl",
];

/// The parameters of most GR IV Monochrome types.
const MONOCHROME: &[&str] = &[
    "ImageKey",
    "Contrast",
    "ContrastHighLight",
    "ContrastShadow",
    "Sharpness",
    "Shading",
    "ClarityControl",
    "ColorToneMonotone",
    "Particle",
    "ParticleSize",
    "ParticleStrength",
];

/// The types in portable order. The GR IV Monochrome types come last, from
/// `FIRST_MONOCHROME`. Pictorialism, a type of the GR IV 1.20 firmware that
/// no released Monochrome firmware names, is left out.
pub const TYPES: [ImageType; 21] = [
    ImageType {
        label: "Standard",
        families: [Some((Prefix::Main, NINE)), Some((Prefix::Main, NINE))],
        monochrome: false,
        overrides: &[],
    },
    ImageType {
        label: "Vivid",
        families: [Some((Prefix::Main, NINE)), Some((Prefix::Main, NINE))],
        monochrome: false,
        overrides: &[],
    },
    ImageType {
        label: "MonotoneGR",
        families: [
            Some((
                Prefix::Main,
                &[
                    "ImageKey",
                    "Contrast",
                    "ContrastHighLight",
                    "ContrastShadow",
                    "Sharpness",
                    "Shading",
                    "ClarityControl",
                    "ColorToneMonotone",
                    "FilterEffectCustom",
                    "Particle",
                ],
            )),
            Some((
                Prefix::Main,
                &[
                    "ImageKey",
                    "Contrast",
                    "ContrastHighLight",
                    "ContrastShadow",
                    "Sharpness",
                    "Shading",
                    "ClarityControl",
                    "ColorToneMonotone",
                    "FilterEffectCustom",
                    "Particle",
                    "ParticleSize",
                    "ParticleStrength",
                ],
            )),
        ],
        monochrome: false,
        overrides: &[],
    },
    ImageType {
        label: "SoftMonotone",
        families: [
            Some((
                Prefix::Main,
                &[
                    "ImageKey",
                    "Contrast",
                    "ContrastHighLight",
                    "ContrastShadow",
                    "Sharpness",
                    "Shading",
                    "ColorToneMonotone",
                    "FilterEffectCustom",
                    "Particle",
                ],
            )),
            Some((
                Prefix::Main,
                &[
                    "ImageKey",
                    "Contrast",
                    "ContrastHighLight",
                    "ContrastShadow",
                    "Sharpness",
                    "Shading",
                    "ColorToneMonotone",
                    "FilterEffectCustom",
                    "Particle",
                    "ParticleSize",
                    "ParticleStrength",
                ],
            )),
        ],
        monochrome: false,
        overrides: &[],
    },
    ImageType {
        label: "HardMonotone",
        families: [
            Some((
                Prefix::Main,
                &[
                    "ImageKey",
                    "Contrast",
                    "ContrastHighLight",
                    "ContrastShadow",
                    "Sharpness",
                    "Shading",
                    "ClarityControl",
                    "ColorToneMonotone",
                    "Particle",
                ],
            )),
            Some((
                Prefix::Main,
                &[
                    "ImageKey",
                    "Contrast",
                    "ContrastHighLight",
                    "ContrastShadow",
                    "Sharpness",
                    "Shading",
                    "ClarityControl",
                    "ColorToneMonotone",
                    "Particle",
                    "ParticleSize",
                    "ParticleStrength",
                ],
            )),
        ],
        monochrome: false,
        overrides: &[],
    },
    ImageType {
        label: "HighContrastBW",
        families: [
            Some((
                Prefix::Main,
                &[
                    "ImageKey",
                    "Contrast",
                    "ContrastHighLight",
                    "ContrastShadow",
                    "Sharpness",
                    "Shading",
                    "ClarityControl",
                    "ColorToneMonotone",
                    "FilterEffectCustom",
                    "Particle",
                ],
            )),
            Some((
                Prefix::Main,
                &[
                    "ImageKey",
                    "Contrast",
                    "ContrastHighLight",
                    "ContrastShadow",
                    "Sharpness",
                    "Shading",
                    "ClarityControl",
                    "ColorToneMonotone",
                    "FilterEffectCustom",
                    "Particle",
                    "ParticleSize",
                    "ParticleStrength",
                ],
            )),
        ],
        monochrome: false,
        overrides: &[],
    },
    ImageType {
        label: "NegativeFilm",
        families: [
            Some((
                Prefix::AfterRelease,
                &[
                    "Saturation",
                    "ColorHue",
                    "ImageKey",
                    "Contrast",
                    "ContrastHighLight",
                    "ContrastShadow",
                    "Sharpness",
                    "Shading",
                    "ClarityControl",
                    "Particle",
                ],
            )),
            Some((
                Prefix::Main,
                &[
                    "Saturation",
                    "ColorHue",
                    "ImageKey",
                    "Contrast",
                    "ContrastHighLight",
                    "ContrastShadow",
                    "Sharpness",
                    "Shading",
                    "ClarityControl",
                    "Particle",
                    "ParticleSize",
                    "ParticleStrength",
                ],
            )),
        ],
        monochrome: false,
        overrides: &[],
    },
    ImageType {
        label: "PositiveFilm",
        families: [
            Some((Prefix::Main, NINE)),
            Some((
                Prefix::Main,
                &[
                    "Saturation",
                    "ColorHue",
                    "ImageKey",
                    "Contrast",
                    "ContrastHighLight",
                    "ContrastShadow",
                    "Sharpness",
                    "Shading",
                    "ClarityControl",
                    "Particle",
                    "ParticleSize",
                    "ParticleStrength",
                ],
            )),
        ],
        monochrome: false,
        overrides: &[],
    },
    ImageType {
        label: "BleechBypass_GR",
        families: [
            Some((
                Prefix::Main,
                &[
                    "Saturation",
                    "ColorHue",
                    "ImageKey",
                    "Contrast",
                    "ContrastHighLight",
                    "ContrastShadow",
                    "Sharpness",
                    "Shading",
                    "ClarityControl",
                    "ColorToneBleechBypass",
                ],
            )),
            Some((
                Prefix::Main,
                &[
                    "Saturation",
                    "ColorHue",
                    "ImageKey",
                    "Contrast",
                    "ContrastHighLight",
                    "ContrastShadow",
                    "Sharpness",
                    "Shading",
                    "ClarityControl",
                    "ColorToneBleechBypass",
                ],
            )),
        ],
        monochrome: false,
        overrides: &[],
    },
    ImageType {
        label: "Retro",
        families: [
            Some((
                Prefix::Main,
                &[
                    "Saturation",
                    "ColorHue",
                    "ImageKey",
                    "Contrast",
                    "ContrastHighLight",
                    "ContrastShadow",
                    "Sharpness",
                    "Shading",
                    "ClarityControl",
                    "ColorToneRetro",
                ],
            )),
            Some((
                Prefix::Main,
                &[
                    "Saturation",
                    "ColorHue",
                    "ImageKey",
                    "Contrast",
                    "ContrastHighLight",
                    "ContrastShadow",
                    "Sharpness",
                    "Shading",
                    "ClarityControl",
                    "ColorToneRetro",
                ],
            )),
        ],
        monochrome: false,
        overrides: &[],
    },
    ImageType {
        label: "HdrCuisine",
        families: [
            Some((
                Prefix::Main,
                &[
                    "Saturation",
                    "ColorHue",
                    "ColorToneHdrIntense",
                    "HdrIntense",
                ],
            )),
            Some((
                Prefix::Main,
                &[
                    "Saturation",
                    "ColorHue",
                    "ColorToneHdrIntense",
                    "HdrIntense",
                ],
            )),
        ],
        monochrome: false,
        overrides: &[],
    },
    ImageType {
        label: "CrossProcess",
        families: [
            Some((
                Prefix::Fwup,
                &[
                    "Saturation",
                    "ColorHue",
                    "ImageKey",
                    "Contrast",
                    "ContrastHighLight",
                    "ContrastShadow",
                    "Sharpness",
                    "Shading",
                    "ClarityControl",
                    "ColorToneCrossProcess",
                ],
            )),
            Some((
                Prefix::Main,
                &[
                    "Saturation",
                    "ColorHue",
                    "ImageKey",
                    "Contrast",
                    "ContrastHighLight",
                    "ContrastShadow",
                    "Sharpness",
                    "Shading",
                    "ClarityControl",
                    "ColorToneCrossProcess",
                ],
            )),
        ],
        monochrome: false,
        overrides: &[],
    },
    ImageType {
        label: "CinemaYellow",
        families: [
            None,
            Some((
                Prefix::Main,
                &[
                    "Saturation",
                    "ColorHue",
                    "ImageKey",
                    "Contrast",
                    "ContrastHighLight",
                    "ContrastShadow",
                    "Sharpness",
                    "Shading",
                    "ClarityControl",
                    "ColorToneCinema",
                    "Particle",
                    "ParticleSize",
                    "ParticleStrength",
                ],
            )),
        ],
        monochrome: false,
        overrides: &[],
    },
    ImageType {
        label: "CinemaGreen",
        families: [
            None,
            Some((
                Prefix::Main,
                &[
                    "Saturation",
                    "ColorHue",
                    "ImageKey",
                    "Contrast",
                    "ContrastHighLight",
                    "ContrastShadow",
                    "Sharpness",
                    "Shading",
                    "ClarityControl",
                    "ColorToneCinema",
                    "Particle",
                    "ParticleSize",
                    "ParticleStrength",
                ],
            )),
        ],
        monochrome: false,
        overrides: &[],
    },
    ImageType {
        label: "MonochromeStandard",
        families: [None, Some((Prefix::AfterRelease, MONOCHROME))],
        monochrome: true,
        overrides: &[],
    },
    ImageType {
        label: "MonochromeSoft",
        families: [
            None,
            Some((
                Prefix::AfterRelease,
                &[
                    "ImageKey",
                    "Contrast",
                    "ContrastHighLight",
                    "ContrastShadow",
                    "Sharpness",
                    "Shading",
                    "ColorToneMonotone",
                    "Particle",
                    "ParticleSize",
                    "ParticleStrength",
                ],
            )),
        ],
        monochrome: true,
        overrides: &[],
    },
    ImageType {
        label: "MonochromeHighContrast",
        families: [None, Some((Prefix::AfterRelease, MONOCHROME))],
        monochrome: true,
        overrides: &[],
    },
    ImageType {
        label: "MonochromeGrainy",
        families: [None, Some((Prefix::AfterRelease, MONOCHROME))],
        monochrome: true,
        overrides: &[],
    },
    ImageType {
        label: "MonochromeSolid",
        families: [None, Some((Prefix::AfterRelease, MONOCHROME))],
        monochrome: true,
        overrides: &[],
    },
    ImageType {
        label: "MonochromeHdrCuisine",
        families: [
            None,
            Some((Prefix::AfterRelease, &["ColorToneHdrIntense", "HdrIntense"])),
        ],
        monochrome: true,
        overrides: &[("ColorToneHdrIntense", labels(0, &["OFF", "", "Sepia"]))],
    },
    ImageType {
        label: "MonochromeSolarization",
        families: [
            None,
            Some((
                Prefix::AfterRelease,
                &[
                    "ImageKey",
                    "Sharpness",
                    "ClarityControl",
                    "ColorToneMonotone",
                    "Particle",
                    "ParticleSize",
                    "ParticleStrength",
                    "SolarizationType",
                ],
            )),
        ],
        monochrome: true,
        overrides: &[],
    },
];
