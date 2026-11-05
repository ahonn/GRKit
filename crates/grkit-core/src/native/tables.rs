//! The families, the parameters and their labels, the white balance modes,
//! and the correction and exposure settings of the portable model.

use super::types::TYPES;

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

    pub(super) fn index(self) -> usize {
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

pub(super) const fn labels(first: u32, labels: &'static [&'static str]) -> Labels {
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

/// The portable index of ColorTemp1, the mode that uses the color temperature.
pub const COLOR_TEMPERATURE_MODE: usize = 14;

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

/// The exposure settings of still pictures that a recipe sets. Each one is a
/// single field that all shooting modes share.
///
/// - Exposure compensation is a float in stops (`EXPOSURE_SETTING`). The
///   exposure step (`EXPOSURE_STEP_SETTING`) is 1/3 or 1/2.
/// - The Auto ISO limits are APEX Sv floats: ISO 100 is 5, and each third
///   stop adds 1/3. The ISO step (`ISO_STEP_SETTING`) is 1/3 or 1.
/// - Metering and the ND filter are label settings.
pub const EXPOSURE_SETTING: &str = "CommonXv";
pub const EXPOSURE_STEP_SETTING: &str = "CommonExposureStep";
pub const ISO_UPPER_SETTING: &str = "StillSvAutoUpper";
pub const ISO_LOWER_SETTING: &str = "StillSvAutoLower";
pub const ISO_STEP_SETTING: &str = "CommonSvStep";
/// The largest exposure compensation, in thirds of a stop (5 stops).
pub const EXPOSURE_THIRDS_MAX: u32 = 15;
/// The nominal ISO of each third stop from ISO 100: index n is Sv 5 + n/3.
pub const ISO_STEPS: [u32; 34] = [
    100, 125, 160, 200, 250, 320, 400, 500, 640, 800, 1000, 1250, 1600, 2000, 2500, 3200, 4000,
    5000, 6400, 8000, 10000, 12800, 16000, 20000, 25600, 32000, 40000, 51200, 64000, 80000, 102400,
    128000, 160000, 204800,
];
/// The Sv of ISO 100.
pub const ISO_SV_BASE: u32 = 5;
pub const METERING: Correction = Correction {
    name: "StillAeMode",
    labels: labels(0, &["Multi", "Center", "Spot", "Highlight"]),
};
pub const ND_FILTER: Correction = Correction {
    name: "StillNdFilter",
    labels: labels(0, &["Auto", "On", "Off"]),
};

impl Family {
    /// The highest ISO of the family, the largest Auto ISO upper limit.
    pub fn iso_max(self) -> u32 {
        match self {
            Family::Gr3 => 102_400,
            Family::Gr4 => 204_800,
        }
    }
}

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
