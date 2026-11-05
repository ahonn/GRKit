//! The portable model of a recipe and the checks of its values against the
//! camera tables of its family.

use super::layout::CODE_MASK;
use crate::native::{self, Family as NativeFamily};
use anyhow::{bail, ensure, Context, Result};
use serde::{Deserialize, Serialize};

/// The most recipes of a library and of a camera (`abi::RECIPE_SLOTS`).
pub const LIBRARY_MAX: usize = crate::abi::RECIPE_SLOTS;
/// The most recipes that are on (`abi::RECIPE_ON_MAX`): the camera lists only
/// them, and keeps the others for the site.
pub const ON_MAX: usize = crate::abi::RECIPE_ON_MAX;
/// The most UTF-16 units of a name.
pub const NAME_UNITS: usize = 16;

/// The GRKit Grain levels: Off to Extra Strong, and the sizes: Auto, then
/// small to large (`CORE.GRAIN`, `CORE.GRAIN_SIZE`).
pub const GRAIN_LEVELS: u8 = 5;
pub const GRAIN_SIZES: u8 = 4;
/// The largest exposure compensation, in thirds of a stop.
pub const EXPOSURE_THIRDS_MAX: i8 = native::EXPOSURE_THIRDS_MAX as i8;

/// The camera line of a recipe. A recipe applies only on its own line.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Family {
    /// GR III and GR IIIx.
    Gr3,
    /// GR IV and GR IV HDF.
    Gr4,
    /// GR IV Monochrome.
    Gr4Monochrome,
}

impl Family {
    /// The value of the family byte of a record.
    pub fn code(self) -> u8 {
        match self {
            Family::Gr3 => 1,
            Family::Gr4 => 2,
            Family::Gr4Monochrome => 3,
        }
    }

    pub(super) fn from_code(code: u8) -> Result<Family> {
        Ok(match code {
            1 => Family::Gr3,
            2 => Family::Gr4,
            3 => Family::Gr4Monochrome,
            _ => bail!("Unknown recipe family {code}"),
        })
    }

    /// The family of the native names and labels.
    pub fn native(self) -> NativeFamily {
        match self {
            Family::Gr3 => NativeFamily::Gr3,
            Family::Gr4 | Family::Gr4Monochrome => NativeFamily::Gr4,
        }
    }

    pub fn monochrome(self) -> bool {
        self == Family::Gr4Monochrome
    }
}

/// The look of a recipe.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Look {
    /// The look stays as it is.
    Kept,
    /// Film Look Off, and the native type stays as it is.
    Off,
    /// A GRKit effect, by the low 15 bits of its record code, with its Fn
    /// adjustments as the record stores them (`abi::RECORD_PARAMETERS`): each
    /// 0 to 8, the offset from the Fn default of the effect mod 9, so 0
    /// keeps the default.
    #[serde(rename_all = "camelCase")]
    Effect { code: u16, adjustments: [u8; 9] },
    /// Film Look Off and a native Image Control type.
    Native(NativeLook),
}

/// A native Image Control type and its parameters, as stored values of the
/// camera (`grkit_core::native`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeLook {
    /// The portable index of the type (`native::TYPES`).
    pub image_type: usize,
    /// The stored value of each level (`native::LEVELS`) in portable order,
    /// 4 for a level that the type does not have.
    pub levels: [u8; 9],
    /// The stored value of the one tone parameter of the type, else 0.
    pub tone: u8,
    /// The stored HDR intensity, else 0.
    pub intensity: u8,
    /// The stored filter of a B&W type, else 0.
    pub filter: u8,
    /// The native grain: strength 0 (Off) to 3 (High), size 0 (Small) to 2
    /// (Large). GR III has one size, 0.
    pub grain_strength: u8,
    pub grain_size: u8,
    /// The stored Solarized Effect type, else 0.
    pub solarization: u8,
}

/// The GRKit Grain of a recipe.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Grain {
    /// 0 Off to 4 Extra Strong.
    pub level: u8,
    /// 0 Auto, then 1 to 3 small to large.
    pub size: u8,
}

/// The module settings that a recipe holds, in the order of their bits in
/// word 4: Glow (`modules/glow`). Each has at most `MODULE_SETTING_VALUES`
/// values. The camera finds each in the settings table by its state file ID,
/// so a recipe holds a setting only while its module is loaded.
pub const MODULE_SETTINGS: [(&str, &str); 3] = [
    ("GLOW", "BLOOM"),
    ("GLOW", "HALATION"),
    ("GLOW", "HALATION_COLOR"),
];
/// Whether each of `MODULE_SETTINGS` is color-only (`color_only` in the
/// manifest of its module): GR IV Monochrome does not hold it.
pub const MODULE_SETTINGS_COLOR_ONLY: [bool; 3] = [false, false, true];
pub const MODULE_SETTING_VALUES: u8 = 4;

/// The Glow of a recipe: Bloom and Halation, each 0 Off to 3 Strong, and the
/// color of Halation, 0 Red, 1 Orange, 2 Gold, 3 White. A recipe without a
/// color keeps the color of the camera. GR IV Monochrome has no color.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Glow {
    pub bloom: u8,
    pub halation: u8,
    #[serde(default)]
    pub halation_color: Option<u8>,
}

impl Glow {
    /// The values of `MODULE_SETTINGS`, `None` for each that is not held.
    pub(super) fn values(&self) -> [Option<u8>; 3] {
        [Some(self.bloom), Some(self.halation), self.halation_color]
    }
}

/// The white balance of a recipe.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhiteBalance {
    /// The portable index of the mode (`native::WHITE_BALANCE`).
    pub mode: usize,
    /// The A-B and G-M shifts: stored values 0 to 28, 14 is no shift.
    pub shift_ba: u8,
    pub shift_gm: u8,
    /// The color temperature of ColorTemp1, in kelvin.
    pub kelvin: u32,
}

/// The metering of a recipe, in the order of the stored values.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Metering {
    Multi,
    Center,
    Spot,
    Highlight,
}

/// The ND filter of a recipe, in the order of the stored values.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Nd {
    Auto,
    On,
    Off,
}

/// The exposure settings of a recipe, which apply sets: each `None` keeps the
/// camera setting. The camera rounds the compensation to its exposure step,
/// and the ISO limit to its ISO step and up to its lower limit.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Exposure {
    /// Exposure compensation in thirds of a stop.
    pub compensation: Option<i8>,
    /// The upper limit of Auto ISO: a value of `native::ISO_STEPS`.
    pub iso_limit: Option<u32>,
    pub metering: Option<Metering>,
    pub nd: Option<Nd>,
}

/// A recipe of the library.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recipe {
    pub name: String,
    pub family: Family,
    pub look: Look,
    pub grain: Option<Grain>,
    #[serde(default)]
    pub glow: Option<Glow>,
    pub white_balance: Option<WhiteBalance>,
    /// The stored value of each correction setting
    /// (`native::CORRECTIONS`), or `None` for Unset.
    pub corrections: [Option<u8>; 4],
    #[serde(default)]
    pub exposure: Exposure,
    /// The recipe is on: the camera lists it in MENU, and ADJ shows it
    /// unless it is hidden. A recipe that is off stays on the card for the
    /// site. At most `ON_MAX` recipes are on.
    #[serde(default = "on")]
    pub enabled: bool,
    /// The recipe stays out of the rows of ADJ (the Options of the module).
    #[serde(default)]
    pub hidden: bool,
    /// The number after the automatic name of a recipe without a name: 0
    /// for none, 2 for the second recipe of a look, at most 15.
    #[serde(default)]
    pub number: u8,
}

fn on() -> bool {
    true
}

impl Exposure {
    fn check(&self, family: Family) -> Result<()> {
        if let Some(thirds) = self.compensation {
            ensure!(
                thirds.abs() <= EXPOSURE_THIRDS_MAX,
                "Exposure compensation {thirds}/3 is out of range"
            );
        }
        if let Some(iso) = self.iso_limit {
            ensure!(
                native::ISO_STEPS.contains(&iso) && iso <= family.native().iso_max(),
                "The camera has no ISO limit {iso}"
            );
        }
        Ok(())
    }
}

impl Recipe {
    /// Check each value against the camera tables of the family. The camera
    /// checks the same values and shows a recipe that fails as unavailable.
    pub fn check(&self) -> Result<()> {
        check_name(&self.name)?;
        let family = self.family.native();
        match &self.look {
            Look::Effect { code, adjustments } => {
                ensure!(
                    *code != 0 && u32::from(*code) <= CODE_MASK,
                    "Effect code {code} is out of range"
                );
                ensure!(
                    adjustments.iter().all(|&v| v <= 8),
                    "An Fn adjustment is out of range"
                );
            }
            Look::Native(look) => check_native(self.family, look)?,
            Look::Kept | Look::Off => {}
        }
        if let Some(grain) = self.grain {
            ensure!(
                grain.level < GRAIN_LEVELS && grain.size < GRAIN_SIZES,
                "Grain is out of range"
            );
        }
        if let Some(glow) = self.glow {
            ensure!(
                glow.values()
                    .iter()
                    .flatten()
                    .all(|&v| v < MODULE_SETTING_VALUES),
                "Glow is out of range"
            );
            ensure!(
                glow.halation_color.is_none() || !self.family.monochrome(),
                "GR IV Monochrome has no Halation color"
            );
        }
        if let Some(balance) = self.white_balance {
            ensure!(
                !self.family.monochrome(),
                "GR IV Monochrome has no white balance"
            );
            let mode = native::WHITE_BALANCE
                .get(balance.mode)
                .context("Unknown white balance mode")?;
            ensure!(
                mode.labels[family as usize].is_some(),
                "The camera has no white balance mode {}",
                balance.mode
            );
            let shifts = native::SHIFT_BA.labels.len() as u8;
            ensure!(
                balance.shift_ba < shifts && balance.shift_gm < shifts,
                "A white balance shift is out of range"
            );
            ensure!(
                (native::TEMPERATURE_MIN..=native::TEMPERATURE_MAX).contains(&balance.kelvin)
                    && (balance.kelvin - native::TEMPERATURE_MIN)
                        .is_multiple_of(native::TEMPERATURE_STEP),
                "Color temperature {} K is out of range",
                balance.kelvin
            );
        }
        for (index, value) in self.corrections.iter().enumerate() {
            if let Some(value) = value {
                let labels = &native::CORRECTIONS[index].labels;
                ensure!(
                    labels
                        .values()
                        .any(|(_, stored)| stored == u32::from(*value)),
                    "{} has no value {value}",
                    native::CORRECTIONS[index].name
                );
            }
        }
        self.exposure.check(self.family)
    }
}

/// A name of at most `NAME_UNITS` printable ASCII characters, or none for
/// the automatic name. The camera draws a name with the font of its menu
/// language, and only ASCII is in each one: the English font has no other
/// characters.
pub fn check_name(name: &str) -> Result<()> {
    ensure!(
        name.len() <= NAME_UNITS,
        "A recipe name has at most {NAME_UNITS} characters"
    );
    ensure!(
        name.bytes().all(|byte| (b' '..=b'~').contains(&byte)),
        "A recipe name has only the letters, digits, and symbols of ASCII"
    );
    Ok(())
}

fn check_native(family: Family, look: &NativeLook) -> Result<()> {
    let native_family = family.native();
    let image_type = native::TYPES
        .get(look.image_type)
        .context("Unknown Image Control type")?;
    ensure!(
        image_type.monochrome == family.monochrome(),
        "{} is not a type of this camera",
        image_type.label
    );
    let (_, keys) = image_type.families[native_family as usize]
        .with_context(|| format!("{} is not a type of this camera", image_type.label))?;
    let has = |key: &str| keys.contains(&key);
    let fits = |key: &str, value: u8| -> bool {
        let parameter = native::parameter(key);
        native::type_parameter_labels(native_family, look.image_type, parameter).is_some_and(
            |labels| {
                labels
                    .values()
                    .any(|(_, stored)| stored == u32::from(value))
            },
        )
    };
    for (index, &value) in look.levels.iter().enumerate() {
        let key = native::PARAMETERS[index].key;
        if has(key) {
            ensure!(fits(key, value), "{key} has no value {value}");
        } else {
            ensure!(value == 4, "{} has no {key}", image_type.label);
        }
    }
    let tone = keys
        .iter()
        .find(|key| key.starts_with("ColorTone"))
        .copied();
    match tone {
        Some(key) => ensure!(fits(key, look.tone), "{key} has no value {}", look.tone),
        None => ensure!(look.tone == 0, "{} has no tone", image_type.label),
    }
    let optional = |key: &str, value: u8| -> Result<()> {
        if has(key) {
            ensure!(fits(key, value), "{key} has no value {value}");
        } else {
            ensure!(value == 0, "{} has no {key}", image_type.label);
        }
        Ok(())
    };
    optional("HdrIntense", look.intensity)?;
    optional("FilterEffectCustom", look.filter)?;
    optional("SolarizationType", look.solarization)?;
    // The native grain in portable values (runtime/native_look.h).
    if look.grain_strength == 0 {
        ensure!(look.grain_size == 0, "Grain Off has no size");
    } else {
        ensure!(
            has("Particle") && look.grain_strength <= 3,
            "{} has no native grain {}",
            image_type.label,
            look.grain_strength
        );
        if has("ParticleSize") {
            ensure!(look.grain_size <= 2, "Native grain size is out of range");
        } else {
            ensure!(look.grain_size == 0, "The native grain has one size");
        }
    }
    Ok(())
}

/// The code of an effect in the records and in recipes: the low 15 bits of
/// the FNV-1a hash of its ID (`effect_code` in runtime/install_effects.h).
pub fn effect_code(id: &str) -> u16 {
    let hash = id.bytes().fold(0x811c_9dc5u32, |hash, byte| {
        (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193)
    });
    (hash & CODE_MASK) as u16
}
