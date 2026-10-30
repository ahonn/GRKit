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

use std::fmt::Write;

/// A camera family: the builds of one camera line, which share the names,
/// the labels, and the stored values of the settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    Gr3,
    Gr4,
}

impl Family {
    pub const ALL: [Family; 2] = [Family::Gr3, Family::Gr4];

    /// The family prefix of the target names (`gr3-2.10`, `gr4-1.20`).
    pub fn key(self) -> &'static str {
        match self {
            Family::Gr3 => "gr3",
            Family::Gr4 => "gr4",
        }
    }

    /// The family of a target name.
    pub fn of_target(target: &str) -> Option<Family> {
        Family::ALL
            .into_iter()
            .find(|family| target.split('-').next() == Some(family.key()))
    }

    fn index(self) -> usize {
        self as usize
    }
}

/// The labels of a setting, in the order of their stored values: the label
/// of stored value `first + n` is `labels[n]`. An empty label marks a stored
/// value without a label.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Labels {
    pub first: u32,
    pub labels: &'static [&'static str],
}

impl Labels {
    /// Each label and its stored value.
    pub fn values(&self) -> impl Iterator<Item = (&'static str, u32)> + '_ {
        self.labels
            .iter()
            .enumerate()
            .filter(|(_, label)| !label.is_empty())
            .map(|(n, &label)| (label, self.first + n as u32))
    }
}

const fn labels(first: u32, labels: &'static [&'static str]) -> Labels {
    Labels { first, labels }
}

/// The nine levels of a parameter, M4 to P4. OFF (stored 4) is neutral.
pub const LEVELS: Labels = labels(0, &["M4", "M3", "M2", "M1", "OFF", "P1", "P2", "P3", "P4"]);
/// The A-B shift of a white balance mode, B14 to A14. NORMAL (stored 14) is
/// no shift.
pub const SHIFT_BA: Labels = labels(
    0,
    &[
        "B14", "B13", "B12", "B11", "B10", "B9", "B8", "B7", "B6", "B5", "B4", "B3", "B2", "B1",
        "NORMAL", "A1", "A2", "A3", "A4", "A5", "A6", "A7", "A8", "A9", "A10", "A11", "A12", "A13",
        "A14",
    ],
);
/// The G-M shift, G14 to M14. NORMAL (stored 14) is no shift.
pub const SHIFT_GM: Labels = labels(
    0,
    &[
        "G14", "G13", "G12", "G11", "G10", "G9", "G8", "G7", "G6", "G5", "G4", "G3", "G2", "G1",
        "NORMAL", "M1", "M2", "M3", "M4", "M5", "M6", "M7", "M8", "M9", "M10", "M11", "M12", "M13",
        "M14",
    ],
);
/// The color temperature in kelvin: the setting takes a number.
pub const TEMPERATURE_MIN: u32 = 2500;
pub const TEMPERATURE_MAX: u32 = 10000;
pub const TEMPERATURE_STEP: u32 = 10;

/// A parameter of an Image Control type.
pub struct Parameter {
    /// The name after the type in the setting name.
    pub key: &'static str,
    /// The labels in each family, or `None` where the family has no such
    /// parameter.
    pub labels: [Option<Labels>; 2],
}

const fn both(labels: Labels) -> [Option<Labels>; 2] {
    [Some(labels), Some(labels)]
}

/// The parameters in portable order. The first nine are the levels of the
/// Fn adjustments, in the order of the GRKit record (RECORD_PARAMETERS).
pub const PARAMETERS: [Parameter; 21] = [
    Parameter {
        key: "Saturation",
        labels: both(LEVELS),
    },
    Parameter {
        key: "ColorHue",
        labels: both(LEVELS),
    },
    Parameter {
        key: "ImageKey",
        labels: both(LEVELS),
    },
    Parameter {
        key: "Contrast",
        labels: both(LEVELS),
    },
    Parameter {
        key: "ContrastHighLight",
        labels: both(LEVELS),
    },
    Parameter {
        key: "ContrastShadow",
        labels: both(LEVELS),
    },
    Parameter {
        key: "Sharpness",
        labels: both(LEVELS),
    },
    Parameter {
        key: "Shading",
        labels: both(LEVELS),
    },
    Parameter {
        key: "ClarityControl",
        labels: both(LEVELS),
    },
    Parameter {
        key: "ColorToneMonotone",
        labels: both(labels(
            0,
            &["OFF", "Sepia", "Red", "Green", "Blue", "Purple"],
        )),
    },
    Parameter {
        key: "ColorToneBleechBypass",
        labels: both(labels(1, &["Normal", "Cool", "Warm"])),
    },
    Parameter {
        key: "ColorToneRetro",
        labels: both(LEVELS),
    },
    Parameter {
        key: "ColorToneHdrIntense",
        labels: both(labels(0, &["OFF", "BW", "Sepia"])),
    },
    Parameter {
        key: "HdrIntense",
        labels: both(labels(1, &["Low", "Mid", "High"])),
    },
    Parameter {
        key: "ColorToneCrossProcess",
        labels: both(labels(0, &["Basic", "Magenta", "Yellow"])),
    },
    Parameter {
        key: "ColorToneCinema",
        labels: [None, Some(labels(0, &["Low", "Mid", "High"]))],
    },
    Parameter {
        key: "FilterEffectCustom",
        labels: both(labels(
            0,
            &["OFF", "Custom1", "Custom2", "Custom3", "Custom4"],
        )),
    },
    // GR III: one grain parameter, its levels from High (1) to Low (3).
    Parameter {
        key: "Particle",
        labels: [
            Some(labels(0, &["Off", "High", "Mid", "Low"])),
            Some(labels(0, &["Off", "On"])),
        ],
    },
    Parameter {
        key: "ParticleSize",
        labels: [None, Some(labels(0, &["Small", "Medium", "Large"]))],
    },
    Parameter {
        key: "ParticleStrength",
        labels: [None, Some(labels(0, &["Low", "Mid", "High"]))],
    },
    // GR IV Monochrome: the kind of Solarized Effect.
    Parameter {
        key: "SolarizationType",
        labels: [None, Some(labels(0, &["TYPE1", "TYPE2", "TYPE3"]))],
    },
];

/// The number of the levels: the first parameters.
pub const LEVEL_COUNT: usize = 9;

/// The portable index of the first GR IV Monochrome type.
pub const FIRST_MONOCHROME: usize = 14;

/// The parameter index of a key.
pub fn parameter(key: &str) -> usize {
    PARAMETERS
        .iter()
        .position(|parameter| parameter.key == key)
        .unwrap_or_else(|| panic!("no parameter {key}"))
}

/// The prefix of the setting names of a parameter. Most types use the names
/// of the main set; two GR III types use names of a later firmware block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Prefix {
    Main,
    AfterRelease,
    Fwup,
}

impl Prefix {
    pub const ALL: [Prefix; 3] = [Prefix::Main, Prefix::AfterRelease, Prefix::Fwup];

    /// The setting name before the type: `<prefix><type><parameter>`.
    pub fn text(self) -> &'static str {
        match self {
            Prefix::Main => "StillImageControlNo0Param",
            Prefix::AfterRelease => "AfterReleaseStillImageControlNo0Param",
            Prefix::Fwup => "StillFwupImageControlNo0Param",
        }
    }
}

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

const NINE: &[&str] = &[
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

/// A white balance mode.
pub struct WhiteBalance {
    /// The label of the mode for `StillWhiteBalance` in each family, which
    /// also names its shifts (`StillWhiteBalance<label>AdjustBA`), or `None`.
    pub labels: [Option<&'static str>; 2],
    /// The mode has the A-B and G-M shifts.
    pub shifts: bool,
}

const fn mode(gr3: Option<&'static str>, gr4: Option<&'static str>, shifts: bool) -> WhiteBalance {
    WhiteBalance {
        labels: [gr3, gr4],
        shifts,
    }
}

/// The white balance modes in portable order. The camera measures Manual
/// and the Custom modes, so a recipe holds only the mode.
pub const WHITE_BALANCE: [WhiteBalance; 18] = [
    mode(Some("Auto"), Some("AutoStandard"), true),
    mode(Some("Multi"), None, true),
    mode(None, Some("AutoWarm"), true),
    mode(None, Some("AutoWhite"), true),
    mode(Some("Daylight"), Some("Daylight"), true),
    mode(Some("Shade"), Some("Shade"), true),
    mode(Some("Cloudiness"), Some("Cloudiness"), true),
    mode(Some("Flu_D"), Some("Flu_D"), true),
    mode(Some("Flu_N"), Some("Flu_N"), true),
    mode(Some("Flu_W"), Some("Flu_W"), true),
    mode(Some("Flu_L"), Some("Flu_L"), true),
    mode(Some("Tungsten"), Some("Tungsten"), true),
    mode(Some("Cte"), Some("Cte"), true),
    mode(Some("Manual1"), Some("Manual1"), true),
    mode(Some("ColorTemp1"), Some("ColorTemp1"), true),
    mode(Some("Custom1"), Some("Custom1"), false),
    mode(Some("Custom2"), Some("Custom2"), false),
    mode(Some("Custom3"), Some("Custom3"), false),
];

/// The setting of the color temperature of ColorTemp1, in kelvin.
pub const COLOR_TEMPERATURE: &str = "StillWhiteBalanceColorTemp";

/// A correction or noise reduction setting of still pictures.
pub struct Correction {
    pub name: &'static str,
    pub labels: Labels,
}

/// The correction and noise reduction settings in portable order.
pub const CORRECTIONS: [Correction; 4] = [
    Correction {
        name: "StillHighlightCorrect",
        labels: labels(0, &["Auto", "On", "Off"]),
    },
    Correction {
        name: "StillShadowCorrect",
        labels: labels(0, &["Auto", "Low", "Mid", "High", "Off"]),
    },
    Correction {
        name: "StillLensShadingCorrection",
        labels: labels(0, &["Off", "On"]),
    },
    Correction {
        name: "StillHighIsoNR",
        labels: labels(0, &["Auto", "Low", "Mid", "High", "Custom", "Off"]),
    },
];

/// The settings of the type and of the active set.
pub const TYPE_SETTING: &str = "StillImageControlNo0";
pub const ACTIVE_SET_SETTING: &str = "StillActiveImageControlSet";
/// The label of the main set for `StillActiveImageControlSet`.
pub const MAIN_SET: &str = "No0";
pub const WHITE_BALANCE_SETTING: &str = "StillWhiteBalance";

/// The setting name of parameter `parameter` of type `image_type` in a
/// family, or `None` where the type has no such parameter.
pub fn parameter_name(family: Family, image_type: usize, parameter: usize) -> Option<String> {
    let (prefix, keys) = TYPES[image_type].families[family.index()]?;
    let key = PARAMETERS[parameter].key;
    keys.contains(&key)
        .then(|| format!("{}{}{key}", prefix.text(), TYPES[image_type].label))
}

/// The labels of a parameter in a family.
pub fn parameter_labels(family: Family, parameter: usize) -> Option<Labels> {
    PARAMETERS[parameter].labels[family.index()]
}

/// The labels of a parameter of a type in a family: an override of the
/// type, or those of the parameter. The camera tables of runtime/native.h
/// hold those of the parameter, which include each override.
pub fn type_parameter_labels(
    family: Family,
    image_type: usize,
    parameter: usize,
) -> Option<Labels> {
    let key = PARAMETERS[parameter].key;
    TYPES[image_type]
        .overrides
        .iter()
        .find(|(name, _)| *name == key)
        .map(|(_, labels)| *labels)
        .or_else(|| parameter_labels(family, parameter))
}

/// The setting names of the shifts of a white balance mode in a family.
pub fn shift_names(family: Family, mode: usize) -> Option<[String; 2]> {
    let label = WHITE_BALANCE[mode].labels[family.index()]?;
    WHITE_BALANCE[mode].shifts.then(|| {
        [
            format!("{WHITE_BALANCE_SETTING}{label}AdjustBA"),
            format!("{WHITE_BALANCE_SETTING}{label}AdjustGM"),
        ]
    })
}

/// The labels of a type in a family, in portable order, with `None` for a
/// type that the family does not have.
pub fn type_labels(family: Family) -> Vec<Option<&'static str>> {
    TYPES
        .iter()
        .map(|image_type| image_type.families[family.index()].map(|_| image_type.label))
        .collect()
}

/// The labels of the white balance modes in a family, in portable order.
pub fn mode_labels(family: Family) -> Vec<Option<&'static str>> {
    WHITE_BALANCE
        .iter()
        .map(|mode| mode.labels[family.index()])
        .collect()
}

fn c_string(text: &str) -> String {
    format!("\"{text}\"")
}

fn c_strings<'a>(texts: impl Iterator<Item = Option<&'a str>>) -> String {
    texts
        .map(|text| text.map(c_string).unwrap_or_else(|| "0".into()))
        .collect::<Vec<_>>()
        .join(",")
}

/// The label table of a label set: a label for each stored value from 0 to
/// the last one, 0 for a value without a label.
fn label_table(labels: &Labels) -> String {
    let last = labels.first + labels.labels.len() as u32;
    c_strings((0..last).map(|value| {
        value
            .checked_sub(labels.first)
            .and_then(|n| labels.labels.get(n as usize).copied())
            .filter(|label| !label.is_empty())
    }))
}

/// The contents of `runtime/native.h`.
pub fn header() -> String {
    let mut out = String::new();
    out.push_str(
        "/* Generated from crates/grkit-core/src/native.rs. Do not edit: change the\n   \
         tables there, then run GRKIT_UPDATE_GENERATED=1 cargo test -p grkit-core\n   \
         --test generated.\n\n   \
         The native picture settings of camera recipes (runtime/recipes.h): the\n   \
         portable index of each Image Control type, parameter, white balance mode,\n   \
         and correction setting, and the names and labels of each family for\n   \
         UserData Set. A label table gives the label of each stored value, 0 for a\n   \
         value without one. runtime/targets/<target>-native.h gives the stored\n   \
         values and the field offsets of a build. */\n",
    );
    out.push_str("#ifndef GRKIT_NATIVE_H\n#define GRKIT_NATIVE_H\n\n");
    writeln!(out, "#define NATIVE_TYPES {}U", TYPES.len()).unwrap();
    writeln!(out, "#define NATIVE_FIRST_MONOCHROME {FIRST_MONOCHROME}U").unwrap();
    writeln!(out, "#define NATIVE_PARAMETERS {}U", PARAMETERS.len()).unwrap();
    writeln!(out, "#define NATIVE_LEVELS {LEVEL_COUNT}U").unwrap();
    writeln!(out, "#define NATIVE_MODES {}U", WHITE_BALANCE.len()).unwrap();
    writeln!(out, "#define NATIVE_CORRECTIONS {}U", CORRECTIONS.len()).unwrap();
    writeln!(out, "#define NATIVE_TEMPERATURE_MIN {TEMPERATURE_MIN}U").unwrap();
    writeln!(out, "#define NATIVE_TEMPERATURE_MAX {TEMPERATURE_MAX}U").unwrap();
    writeln!(out, "#define NATIVE_TEMPERATURE_STEP {TEMPERATURE_STEP}U").unwrap();
    for (index, parameter) in PARAMETERS.iter().enumerate() {
        writeln!(
            out,
            "#define NATIVE_PARAMETER_{} {index}U",
            parameter.key.to_ascii_uppercase()
        )
        .unwrap();
    }
    out.push('\n');
    writeln!(
        out,
        "#define NATIVE_TYPE_SETTING {}",
        c_string(TYPE_SETTING)
    )
    .unwrap();
    writeln!(
        out,
        "#define NATIVE_ACTIVE_SET_SETTING {}",
        c_string(ACTIVE_SET_SETTING)
    )
    .unwrap();
    writeln!(out, "#define NATIVE_MAIN_SET {}", c_string(MAIN_SET)).unwrap();
    writeln!(
        out,
        "#define NATIVE_WHITE_BALANCE_SETTING {}",
        c_string(WHITE_BALANCE_SETTING)
    )
    .unwrap();
    writeln!(
        out,
        "#define NATIVE_COLOR_TEMPERATURE_SETTING {}",
        c_string(COLOR_TEMPERATURE)
    )
    .unwrap();
    writeln!(
        out,
        "#define NATIVE_PREFIXES {}",
        c_strings(Prefix::ALL.iter().map(|prefix| Some(prefix.text())))
    )
    .unwrap();
    writeln!(
        out,
        "#define NATIVE_PARAMETER_KEYS {}",
        c_strings(PARAMETERS.iter().map(|parameter| Some(parameter.key)))
    )
    .unwrap();
    writeln!(
        out,
        "#define NATIVE_CORRECTION_NAMES {}",
        c_strings(CORRECTIONS.iter().map(|correction| Some(correction.name)))
    )
    .unwrap();
    writeln!(
        out,
        "#define NATIVE_SHIFT_BA_LABELS {}",
        label_table(&SHIFT_BA)
    )
    .unwrap();
    writeln!(
        out,
        "#define NATIVE_SHIFT_GM_LABELS {}",
        label_table(&SHIFT_GM)
    )
    .unwrap();
    for (index, correction) in CORRECTIONS.iter().enumerate() {
        writeln!(
            out,
            "#define NATIVE_CORRECTION_{index}_LABELS {}",
            label_table(&correction.labels)
        )
        .unwrap();
    }
    out.push('\n');
    for family in Family::ALL {
        let key = family.key().to_ascii_uppercase();
        writeln!(out, "/* {}. */", family.key()).unwrap();
        writeln!(
            out,
            "#define NATIVE_{key}_TYPE_LABELS {}",
            c_strings(type_labels(family).into_iter())
        )
        .unwrap();
        writeln!(
            out,
            "#define NATIVE_{key}_MODE_LABELS {}",
            c_strings(mode_labels(family).into_iter())
        )
        .unwrap();
        writeln!(
            out,
            "#define NATIVE_{key}_MODE_SHIFTS {}",
            WHITE_BALANCE
                .iter()
                .map(
                    |mode| u32::from(mode.labels[family.index()].is_some() && mode.shifts)
                        .to_string()
                )
                .collect::<Vec<_>>()
                .join(",")
        )
        .unwrap();
        // The prefix of the parameter names of each type, an index of
        // NATIVE_PREFIXES, 0 for a type that the family does not have.
        writeln!(
            out,
            "#define NATIVE_{key}_TYPE_PREFIXES {}",
            TYPES
                .iter()
                .map(|image_type| {
                    image_type.families[family.index()]
                        .map(|(prefix, _)| Prefix::ALL.iter().position(|&p| p == prefix).unwrap())
                        .unwrap_or(0)
                        .to_string()
                })
                .collect::<Vec<_>>()
                .join(",")
        )
        .unwrap();
        // The labels of each parameter in one list: those of parameter p from
        // NATIVE_<FAMILY>_LABEL_STARTS[p], NATIVE_<FAMILY>_LABEL_COUNTS[p]
        // entries, one for each stored value from 0.
        let (mut flat, mut starts, mut counts) = (Vec::new(), Vec::new(), Vec::new());
        for parameter in PARAMETERS.iter() {
            starts.push(flat.len().to_string());
            let entries: Vec<Option<&str>> = match parameter.labels[family.index()] {
                Some(labels) => {
                    let last = labels.first + labels.labels.len() as u32;
                    (0..last)
                        .map(|value| {
                            value
                                .checked_sub(labels.first)
                                .and_then(|n| labels.labels.get(n as usize).copied())
                        })
                        .collect()
                }
                None => Vec::new(),
            };
            counts.push(entries.len().to_string());
            flat.extend(entries);
        }
        writeln!(
            out,
            "#define NATIVE_{key}_LABELS {}",
            c_strings(flat.into_iter())
        )
        .unwrap();
        writeln!(
            out,
            "#define NATIVE_{key}_LABEL_STARTS {}",
            starts.join(",")
        )
        .unwrap();
        writeln!(
            out,
            "#define NATIVE_{key}_LABEL_COUNTS {}",
            counts.join(",")
        )
        .unwrap();
    }
    out.push_str("\n#endif\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

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
