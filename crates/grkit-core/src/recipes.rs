//! The recipe library that the site writes to the card: `GRKIT/RECIPES.BIN`.
//!
//! A recipe is a named set of picture settings that the camera applies in one
//! step (`runtime/recipes.h`). The camera holds each recipe as
//! `RECIPE_WORDS` words, and a library record holds the same words, so the
//! camera applies a library recipe as it applies a recipe that it saved.
//! This module gives the portable model of a recipe, its words, and the file.
//!
//! The file is `HEADER_SIZE` bytes, then `RECORD_SIZE` bytes for each recipe
//! (`RCP_FIELDS` and `RCP_RECORD_FIELDS` in `abi`). INSTALL reads it into the
//! core and skips each record that the checks of `runtime/recipe_library.h`
//! reject.

use crate::abi::{
    RCP_BYTES, RCP_COUNT, RCP_CRC, RCP_MAGIC, RCP_RECORD_FAMILY, RCP_RECORD_NAME, RCP_RECORD_WORDS,
    RECIPE_WORDS,
};
use crate::native::{self, Family as NativeFamily};
use anyhow::{bail, ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::fmt::Write;

/// The magic of the file.
pub const MAGIC: &[u8; 8] = b"GRKR0001";
/// The bytes before the first record.
pub const HEADER_SIZE: usize = 32;
/// The bytes of each record.
pub const RECORD_SIZE: usize = 64;
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

    fn from_code(code: u8) -> Result<Family> {
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
    /// adjustments as stored bytes 0 to 8 (4 is neutral).
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
pub const MODULE_SETTING_VALUES: u8 = 4;
/// The bits of one module setting in word 4: bit 0 holds it, then its value.
const MODULE_SETTING_BITS: u32 = 3;

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
    fn values(&self) -> [Option<u8>; 3] {
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

// The words of a recipe (runtime/recipes.h and runtime/native_look.h).
const USED: u32 = 0x0800_0000;
const NUMBER_SHIFT: u32 = 23;
const HIDDEN_SHIFT: u32 = 28;
const LOOK_KEPT: u32 = 0;
const LOOK_OFF: u32 = 1;
const LOOK_EFFECT: u32 = 2;
const GRAIN_HELD: u32 = 4;
const GRAIN_LEVEL_SHIFT: u32 = 3;
const GRAIN_SIZE_SHIFT: u32 = 6;
const CODE_SHIFT: u32 = 8;
const CODE_MASK: u32 = 0x7fff;
const TONE_SHIFT: u32 = 5;
const INTENSITY_SHIFT: u32 = 9;
const FILTER_SHIFT: u32 = 11;
const MODE_SHIFT: u32 = 14;
const BA_SHIFT: u32 = 19;
const GM_SHIFT: u32 = 24;
const CORRECTION_SHIFT: u32 = 10;
const NATIVE_GRAIN_SHIFT: u32 = 22;
const NATIVE_GRAIN_SIZE_SHIFT: u32 = 24;
const SOLARIZATION_SHIFT: u32 = 26;
/// The word of the module settings (`MODULE_SETTINGS`).
const MODULE_WORD: usize = 4;
/// The bit of `MODULE_WORD` of a recipe that is off (`RECIPE_OFF`).
const OFF_SHIFT: u32 = 28;
/// Each word stays below 2^29, as the state file keeps it.
pub const WORD_LIMIT: u32 = 1 << 29;

// The exposure settings in word 4, after the module settings, each 0 when
// the recipe does not hold it (runtime/native_look.h): the compensation in
// sixths of a stop + 31, the third stop of the ISO limit from ISO 100 + 1,
// and the stored metering and ND filter + 1.
const COMPENSATION_SHIFT: u32 = 9;
const COMPENSATION_ZERO: i32 = 31;
const ISO_SHIFT: u32 = 15;
const METERING_SHIFT: u32 = 21;
const ND_SHIFT: u32 = 24;
const _: () =
    assert!(MODULE_SETTING_BITS as usize * MODULE_SETTINGS.len() == COMPENSATION_SHIFT as usize);

/// The base-9 digits of nine values, the first lowest.
fn digits(values: &[u8; 9], digit: impl Fn(u8) -> u32) -> u32 {
    values.iter().rev().fold(0, |sum, &v| sum * 9 + digit(v))
}

fn undigits(mut word: u32, value: impl Fn(u32) -> u8) -> [u8; 9] {
    let mut values = [0; 9];
    for slot in &mut values {
        *slot = value(word % 9);
        word /= 9;
    }
    values
}

/// A level as a digit: (stored - 4) mod 9, so that 0 is neutral.
fn level_digit(stored: u8) -> u32 {
    (u32::from(stored) + 5) % 9
}

fn level_of_digit(digit: u32) -> u8 {
    ((digit + 4) % 9) as u8
}

impl Exposure {
    fn bits(&self) -> u32 {
        let compensation = self.compensation.map_or(0, |thirds| {
            (2 * i32::from(thirds) + COMPENSATION_ZERO) as u32
        });
        let iso = self.iso_limit.map_or(0, |iso| {
            native::ISO_STEPS
                .iter()
                .position(|&step| step == iso)
                .unwrap() as u32
                + 1
        });
        let metering = self.metering.map_or(0, |m| m as u32 + 1);
        let nd = self.nd.map_or(0, |nd| nd as u32 + 1);
        compensation << COMPENSATION_SHIFT
            | iso << ISO_SHIFT
            | metering << METERING_SHIFT
            | nd << ND_SHIFT
    }

    fn of_bits(word: u32) -> Result<Exposure> {
        let compensation = (word >> COMPENSATION_SHIFT & 63) as i32;
        let iso = (word >> ISO_SHIFT & 63) as usize;
        let metering = word >> METERING_SHIFT & 7;
        let nd = word >> ND_SHIFT & 3;
        ensure!(
            compensation == 0 || (compensation - COMPENSATION_ZERO) % 2 == 0,
            "A library recipe has its compensation in thirds of a stop"
        );
        Ok(Exposure {
            compensation: (compensation != 0)
                .then(|| ((compensation - COMPENSATION_ZERO) / 2) as i8),
            iso_limit: match iso {
                0 => None,
                n => Some(*native::ISO_STEPS.get(n - 1).context("Unknown ISO limit")?),
            },
            metering: match metering {
                0 => None,
                1 => Some(Metering::Multi),
                2 => Some(Metering::Center),
                3 => Some(Metering::Spot),
                4 => Some(Metering::Highlight),
                _ => bail!("Unknown metering {metering}"),
            },
            nd: match nd {
                0 => None,
                1 => Some(Nd::Auto),
                2 => Some(Nd::On),
                _ => Some(Nd::Off),
            },
        })
    }

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

    /// The words of the recipe, as the camera holds them.
    pub fn words(&self) -> Result<[u32; RECIPE_WORDS]> {
        self.check()?;
        ensure!(self.number <= 15, "The number of a recipe is at most 15");
        let mut w = [
            USED | u32::from(self.number) << NUMBER_SHIFT | u32::from(self.hidden) << HIDDEN_SHIFT,
            0,
            0,
            0,
            0,
        ];
        match &self.look {
            Look::Kept => w[0] |= LOOK_KEPT,
            Look::Off => w[0] |= LOOK_OFF,
            Look::Effect { code, adjustments } => {
                w[0] |= LOOK_EFFECT | u32::from(*code) << CODE_SHIFT;
                w[1] = digits(adjustments, u32::from);
            }
            Look::Native(look) => {
                w[0] |= LOOK_OFF;
                w[1] = digits(&look.levels, level_digit);
                w[2] = (look.image_type as u32 + 1)
                    | u32::from(look.tone) << TONE_SHIFT
                    | u32::from(look.intensity) << INTENSITY_SHIFT
                    | u32::from(look.filter) << FILTER_SHIFT;
                w[3] = u32::from(look.grain_strength) << NATIVE_GRAIN_SHIFT
                    | u32::from(look.grain_size) << NATIVE_GRAIN_SIZE_SHIFT
                    | u32::from(look.solarization) << SOLARIZATION_SHIFT;
            }
        }
        if let Some(grain) = self.grain {
            w[0] |= GRAIN_HELD
                | u32::from(grain.level) << GRAIN_LEVEL_SHIFT
                | u32::from(grain.size) << GRAIN_SIZE_SHIFT;
        }
        if let Some(balance) = self.white_balance {
            w[2] |= (balance.mode as u32 + 1) << MODE_SHIFT
                | u32::from(balance.shift_ba) << BA_SHIFT
                | u32::from(balance.shift_gm) << GM_SHIFT;
            w[3] |= (balance.kelvin - native::TEMPERATURE_MIN) / native::TEMPERATURE_STEP;
        }
        for (index, value) in self.corrections.iter().enumerate() {
            if let Some(value) = value {
                w[3] |= (u32::from(*value) + 1) << (CORRECTION_SHIFT + 3 * index as u32);
            }
        }
        w[MODULE_WORD] |= self.exposure.bits() | u32::from(!self.enabled) << OFF_SHIFT;
        if let Some(glow) = self.glow {
            for (index, value) in glow.values().iter().enumerate() {
                if let Some(value) = value {
                    w[MODULE_WORD] |=
                        (1 | u32::from(*value) << 1) << (MODULE_SETTING_BITS * index as u32);
                }
            }
        }
        debug_assert!(w.iter().all(|&word| word < WORD_LIMIT));
        Ok(w)
    }

    /// The recipe of the words of a record.
    pub fn from_words(name: String, family: Family, w: [u32; RECIPE_WORDS]) -> Result<Recipe> {
        ensure!(
            w[0] & USED != 0 && w.iter().all(|&word| word < WORD_LIMIT),
            "Not the words of a recipe"
        );
        let image_type = w[2] & 31;
        let look = match w[0] & 3 {
            LOOK_EFFECT => Look::Effect {
                code: (w[0] >> CODE_SHIFT & CODE_MASK) as u16,
                adjustments: undigits(w[1], |digit| digit as u8),
            },
            LOOK_OFF if image_type != 0 => Look::Native(NativeLook {
                image_type: image_type as usize - 1,
                levels: undigits(w[1], level_of_digit),
                tone: (w[2] >> TONE_SHIFT & 15) as u8,
                intensity: (w[2] >> INTENSITY_SHIFT & 3) as u8,
                filter: (w[2] >> FILTER_SHIFT & 7) as u8,
                grain_strength: (w[3] >> NATIVE_GRAIN_SHIFT & 3) as u8,
                grain_size: (w[3] >> NATIVE_GRAIN_SIZE_SHIFT & 3) as u8,
                solarization: (w[3] >> SOLARIZATION_SHIFT & 7) as u8,
            }),
            LOOK_OFF => Look::Off,
            LOOK_KEPT => Look::Kept,
            other => bail!("Unknown look {other}"),
        };
        let mode = w[2] >> MODE_SHIFT & 31;
        let recipe = Recipe {
            name,
            family,
            look,
            grain: (w[0] & GRAIN_HELD != 0).then(|| Grain {
                level: (w[0] >> GRAIN_LEVEL_SHIFT & 7) as u8,
                size: (w[0] >> GRAIN_SIZE_SHIFT & 3) as u8,
            }),
            white_balance: (mode != 0).then(|| WhiteBalance {
                mode: mode as usize - 1,
                shift_ba: (w[2] >> BA_SHIFT & 31) as u8,
                shift_gm: (w[2] >> GM_SHIFT & 31) as u8,
                kelvin: native::TEMPERATURE_MIN + (w[3] & 1023) * native::TEMPERATURE_STEP,
            }),
            corrections: std::array::from_fn(|index| {
                let value = w[3] >> (CORRECTION_SHIFT + 3 * index as u32) & 7;
                (value != 0).then(|| value as u8 - 1)
            }),
            glow: glow_of_word(w[MODULE_WORD])?,
            exposure: Exposure::of_bits(w[MODULE_WORD])?,
            enabled: w[MODULE_WORD] >> OFF_SHIFT & 1 == 0,
            hidden: w[0] >> HIDDEN_SHIFT & 1 != 0,
            number: (w[0] >> NUMBER_SHIFT & 15) as u8,
        };
        recipe.check()?;
        ensure!(
            recipe.words()? == w,
            "The words of {} hold bits that no field has",
            recipe.name
        );
        Ok(recipe)
    }
}

/// The Glow of word 4. Bloom and Halation are held together, and the color
/// only with them.
fn glow_of_word(word: u32) -> Result<Option<Glow>> {
    let held: [Option<u8>; 3] = std::array::from_fn(|index| {
        let bits = word >> (MODULE_SETTING_BITS * index as u32);
        (bits & 1 != 0).then_some((bits >> 1 & 3) as u8)
    });
    match held {
        [None, None, None] => Ok(None),
        [Some(bloom), Some(halation), halation_color] => Ok(Some(Glow {
            bloom,
            halation,
            halation_color,
        })),
        _ => bail!("A recipe holds Bloom and Halation together"),
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

/// A recipe that the site offers: its ID, the effect that it uses, and the
/// recipe for each camera line that can hold it. Other recipes come from the
/// camera.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    pub id: &'static str,
    /// The ID of the effect of the look, which the package must hold.
    pub effect: Option<&'static str>,
    /// The person who made a community recipe, and the post where they
    /// shared it, or `None` for a recipe of GRKit.
    pub author: Option<&'static str>,
    pub source: Option<&'static str>,
    pub recipes: Vec<Recipe>,
}

/// The recipes that the site offers.
pub fn presets() -> Vec<Preset> {
    // CineStill 800T is Vision3 500T without its anti-halation layer: the
    // film look and a red halation close around the lights, without the
    // wide glow of Bloom.
    let cinestill = |family| Recipe {
        name: "CineStill 800T".into(),
        family,
        look: Look::Effect {
            code: effect_code("vision3-500t-supra-v1"),
            adjustments: [4; 9],
        },
        grain: Some(Grain { level: 2, size: 0 }),
        glow: Some(Glow {
            bloom: 0,
            halation: 2,
            halation_color: Some(0),
        }),
        white_balance: None,
        corrections: [None; 4],
        exposure: Exposure::default(),
        enabled: true,
        hidden: false,
        number: 0,
    };
    // A community recipe of Negative Film with its nine levels as the camera
    // shows them, -4 to +4, the Auto white balance with its shifts (stored
    // 14 is none), and the highlight and shadow corrections.
    let negative =
        |name: &str, levels: [i8; 9], shift_gm: i8, shift_ba: i8, corrections: [u8; 2]| Recipe {
            name: name.into(),
            family: Family::Gr3,
            look: Look::Native(NativeLook {
                image_type: NEGATIVE_FILM,
                levels: levels.map(|level| (level + 4) as u8),
                tone: 0,
                intensity: 0,
                filter: 0,
                grain_strength: 0,
                grain_size: 0,
                solarization: 0,
            }),
            grain: None,
            glow: None,
            white_balance: Some(WhiteBalance {
                mode: 0,
                shift_ba: (shift_ba + 14) as u8,
                shift_gm: (shift_gm + 14) as u8,
                kelvin: 5500,
            }),
            corrections: [Some(corrections[0]), Some(corrections[1]), None, None],
            exposure: Exposure::default(),
            enabled: true,
            hidden: false,
            number: 0,
        };
    vec![
        // Highlight correction On, shadow correction Off; AWB G2 A14.
        Preset {
            id: "bennet-800",
            effect: None,
            author: Some("Bennet"),
            source: Some("https://www.reddit.com/r/ricohGR/comments/1k27hmk/"),
            recipes: vec![negative(
                "Bennet 800",
                [3, -1, 0, 2, -2, -4, 0, 0, -1],
                -2,
                14,
                [1, 4],
            )],
        },
        // Highlight correction Auto, shadow correction Mid; AWB A6.
        Preset {
            id: "reggie-negative",
            effect: None,
            author: Some("Reggie Ballesteros"),
            source: Some("https://www.reddit.com/r/ricohGR/comments/180a8nt/"),
            recipes: vec![negative(
                "Reggie Negative",
                [2, 0, 0, 3, -4, -1, 1, 0, 0],
                0,
                6,
                [0, 2],
            )],
        },
        Preset {
            id: "cinestill-800t",
            effect: Some("vision3-500t-supra-v1"),
            author: None,
            source: None,
            recipes: vec![cinestill(Family::Gr3), cinestill(Family::Gr4)],
        },
    ]
}

/// The portable index of Negative Film (`native::TYPES`).
const NEGATIVE_FILM: usize = 6;

/// `web/src/generated/recipes.ts`: the recipes that the site offers, and the
/// camera names of the native types in each site language.
fn site_module() -> String {
    use crate::language::Language;
    let mut out = String::from(
        "// Generated from crates/grkit-core/src/recipes.rs. Do not edit: change the\n\
         // tables there, then run\n\
         // GRKIT_UPDATE_GENERATED=1 cargo test -p grkit-core --test generated.\n\n\
         import type { Preset } from '../recipes';\n\n\
         /** The recipes that the site offers. */\n\
         export const PRESETS: readonly Preset[] = ",
    );
    out.push_str(&serde_json::to_string_pretty(&presets()).unwrap());
    // The names of the stored values that a recipe shows: the nine levels of
    // a native look in portable order, the white balance modes of GR III and
    // GR IV, and the values of each correction setting.
    let levels: Vec<&str> = native::TYPES[0].families[0]
        .map_or(&[][..], |(_, keys)| keys)
        .to_vec();
    let modes: Vec<[Option<&str>; 2]> = native::WHITE_BALANCE
        .iter()
        .map(|mode| mode.labels)
        .collect();
    let corrections: Vec<Vec<&str>> = native::CORRECTIONS
        .iter()
        .map(|correction| correction.labels.labels.to_vec())
        .collect();
    writeln!(
        out,
        ";\n\n/** The levels of a native look in portable order (`native::TYPES`). */\n\
         export const LEVEL_KEYS: readonly string[] = {};\n\n\
         /** The labels of each white balance mode in portable order: GR III, then GR IV, or null. */\n\
         export const WHITE_BALANCE_MODES: readonly (readonly (string | null)[])[] = {};\n\n\
         /** The labels of the stored values of each correction setting, in portable order. */\n\
         export const CORRECTION_VALUES: readonly (readonly string[])[] = {};",
        serde_json::to_string(&levels).unwrap(),
        serde_json::to_string(&modes).unwrap(),
        serde_json::to_string(&corrections).unwrap(),
    )
    .unwrap();
    out.push_str(
        "\n/** The camera names of the types in each site language, by portable index. */\n",
    );
    out.push_str("export const TYPE_NAMES: Readonly<Record<string, readonly string[]>> = {\n");
    for language in Language::ALL {
        let names = serde_json::to_string(crate::menu_text::native_type_names(language)).unwrap();
        writeln!(out, "  '{}': {names},", language.code()).unwrap();
    }
    out.push_str("};\n");
    out
}

/// The generated files, as paths from the repository root and contents.
pub fn generated() -> Vec<(&'static str, String)> {
    vec![("web/src/generated/recipes.ts", site_module())]
}

/// Checks the size of a library: at most `LIBRARY_MAX` recipes, and at most
/// `ON_MAX` of them on.
pub fn check_library(recipes: &[Recipe]) -> Result<()> {
    ensure!(
        recipes.len() <= LIBRARY_MAX,
        "A library has at most {LIBRARY_MAX} recipes"
    );
    let on = recipes.iter().filter(|recipe| recipe.enabled).count();
    ensure!(on <= ON_MAX, "At most {ON_MAX} recipes are on, not {on}");
    Ok(())
}

/// The recipes in the order of the camera: those that are on, then those
/// that are off, each in their order.
pub fn camera_order(recipes: &[Recipe]) -> Vec<&Recipe> {
    let (on, off): (Vec<_>, Vec<_>) = recipes.iter().partition(|recipe| recipe.enabled);
    on.into_iter().chain(off).collect()
}

/// The bytes of `RECIPES.BIN` for a library.
pub fn encode(recipes: &[Recipe]) -> Result<Vec<u8>> {
    check_library(recipes)?;
    let mut bytes = vec![0; HEADER_SIZE + RECORD_SIZE * recipes.len()];
    bytes[RCP_MAGIC..RCP_MAGIC + 8].copy_from_slice(MAGIC);
    let size = bytes.len() as u32;
    put(&mut bytes, RCP_BYTES, size);
    put(&mut bytes, RCP_COUNT, recipes.len() as u32);
    for (index, recipe) in recipes.iter().enumerate() {
        // A record of a package holds a name: INSTALL compares recipes by it.
        ensure!(
            !recipe.name.is_empty(),
            "Recipe {} of a package needs a name",
            index + 1
        );
        let words = recipe
            .words()
            .with_context(|| format!("Recipe {}: {}", index + 1, recipe.name))?;
        let record = HEADER_SIZE + RECORD_SIZE * index;
        for (n, unit) in recipe.name.encode_utf16().enumerate() {
            bytes[record + RCP_RECORD_NAME + 2 * n..][..2].copy_from_slice(&unit.to_le_bytes());
        }
        for (n, word) in words.iter().enumerate() {
            put(&mut bytes, record + RCP_RECORD_WORDS + 4 * n, *word);
        }
        bytes[record + RCP_RECORD_FAMILY] = recipe.family.code();
    }
    let crc = checksum(&bytes);
    put(&mut bytes, RCP_CRC, crc);
    Ok(bytes)
}

/// The recipes of `RECIPES.BIN`. A damaged file is an error.
pub fn decode(bytes: &[u8]) -> Result<Vec<Recipe>> {
    ensure!(
        bytes.len() >= HEADER_SIZE && &bytes[RCP_MAGIC..RCP_MAGIC + 8] == MAGIC,
        "Not a recipe library"
    );
    let count = get(bytes, RCP_COUNT) as usize;
    ensure!(
        count <= LIBRARY_MAX
            && get(bytes, RCP_BYTES) as usize == bytes.len()
            && bytes.len() == HEADER_SIZE + RECORD_SIZE * count,
        "The recipe library has a wrong size"
    );
    ensure!(
        get(bytes, RCP_CRC) == checksum(bytes),
        "The recipe library is damaged"
    );
    (0..count)
        .map(|index| {
            let record = &bytes[HEADER_SIZE + RECORD_SIZE * index..][..RECORD_SIZE];
            let units: Vec<u16> = record[RCP_RECORD_NAME..RCP_RECORD_NAME + 2 * NAME_UNITS]
                .chunks_exact(2)
                .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
                .take_while(|&unit| unit != 0)
                .collect();
            let name = String::from_utf16(&units).context("A recipe name is not UTF-16")?;
            let words = std::array::from_fn(|n| get(record, RCP_RECORD_WORDS + 4 * n));
            let family = Family::from_code(record[RCP_RECORD_FAMILY])?;
            Recipe::from_words(name, family, words).with_context(|| format!("Recipe {}", index + 1))
        })
        .collect()
}

/// The recipes of the recipe file (`GRKIT/RECIPES.TXT`) in its text: key
/// and value lines (`runtime::recipe_key`), for a camera of `family`. A
/// recipe that the checks of the family reject stays out.
pub fn read_recipe_file(text: &str, family: Family) -> Vec<Recipe> {
    use crate::abi::{RECIPE_NAME_WORDS, RECIPE_SLOTS};
    use crate::runtime::recipe_key;
    let values = file_values(text);
    let word = |index: usize| values.get(&recipe_key(index)).copied().unwrap_or(0);
    let names = RECIPE_WORDS * RECIPE_SLOTS;
    let mut out = Vec::new();
    for slot in 0..RECIPE_SLOTS {
        let w: [u32; RECIPE_WORDS] = std::array::from_fn(|k| word(slot * RECIPE_WORDS + k));
        // A word that is not a recipe word ends the recipes.
        if w[0] & USED == 0 || w[0] >= WORD_LIMIT {
            break;
        }
        let mut name = String::new();
        for n in 0..crate::abi::RECIPE_NAME_CHARS {
            let c = word(names + slot * RECIPE_NAME_WORDS + n / 4) >> (7 * (n % 4)) & 0x7f;
            if !(0x20..=0x7e).contains(&c) {
                break;
            }
            name.push(c as u8 as char);
        }
        if let Ok(recipe) = Recipe::from_words(name.trim().to_string(), family, w) {
            out.push(recipe);
        }
    }
    out
}

/// The text of the recipe file for `recipes`, in the order of the camera
/// (`camera_order`), with the import hash of the file text `previous`, so that
/// a package that INSTALL imported does not import again.
pub fn write_recipe_file(recipes: &[Recipe], previous: &str) -> Result<String> {
    use crate::abi::{RECIPE_NAME_WORDS, RECIPE_SLOTS};
    use crate::runtime::{recipe_key, RECIPE_IMPORTED_KEY};
    check_library(recipes)?;
    let names = RECIPE_WORDS * RECIPE_SLOTS;
    let mut text = String::new();
    let mut put = |key: u32, value: u32| {
        if value != 0 {
            text.push_str(&format!("{key}\r\n{value}\r\n"));
        }
    };
    for (slot, recipe) in camera_order(recipes).into_iter().enumerate() {
        let words = recipe
            .words()
            .with_context(|| format!("Recipe {}: {}", slot + 1, recipe.name))?;
        for (k, &value) in words.iter().enumerate() {
            put(recipe_key(slot * RECIPE_WORDS + k), value);
        }
        let mut name = [0u32; RECIPE_NAME_WORDS];
        for (n, byte) in recipe.name.bytes().enumerate() {
            name[n / 4] |= u32::from(byte & 0x7f) << (7 * (n % 4));
        }
        for (k, &value) in name.iter().enumerate() {
            put(recipe_key(names + slot * RECIPE_NAME_WORDS + k), value);
        }
    }
    let imported = file_values(previous)
        .get(&RECIPE_IMPORTED_KEY)
        .copied()
        .unwrap_or(0);
    put(RECIPE_IMPORTED_KEY, imported);
    Ok(text)
}

/// The key and value pairs of a state file text.
fn file_values(text: &str) -> std::collections::BTreeMap<u32, u32> {
    let mut values = std::collections::BTreeMap::new();
    let mut lines = text.split_whitespace();
    while let (Some(key), Some(value)) = (lines.next(), lines.next()) {
        if let (Ok(key), Ok(value)) = (key.parse::<u32>(), value.parse::<u32>()) {
            values.insert(key, value);
        }
    }
    values
}

/// The CRC-32 of the file, which reads the CRC field as zero.
pub fn checksum(bytes: &[u8]) -> u32 {
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(&bytes[..RCP_CRC]);
    hasher.update(&[0; 4]);
    hasher.update(&bytes[RCP_CRC + 4..]);
    hasher.finalize()
}

fn put(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

fn get(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index(label: &str) -> usize {
        native::TYPES.iter().position(|t| t.label == label).unwrap()
    }

    pub(crate) fn sample() -> Vec<Recipe> {
        vec![
            Recipe {
                name: "Portra Days".into(),
                family: Family::Gr3,
                look: Look::Effect {
                    code: 0x1234,
                    adjustments: [4, 5, 3, 4, 4, 4, 4, 8, 0],
                },
                grain: Some(Grain { level: 2, size: 0 }),
                glow: Some(Glow {
                    bloom: 1,
                    halation: 3,
                    halation_color: Some(2),
                }),
                white_balance: Some(WhiteBalance {
                    mode: 4,
                    shift_ba: 17,
                    shift_gm: 12,
                    kelvin: 5500,
                }),
                corrections: [Some(2), None, Some(1), Some(0)],
                exposure: Exposure {
                    compensation: Some(-2),
                    iso_limit: Some(3200),
                    metering: Some(Metering::Spot),
                    nd: Some(Nd::Auto),
                },
                enabled: true,
                hidden: false,
                number: 0,
            },
            Recipe {
                name: "Nega 2".into(),
                family: Family::Gr4,
                look: Look::Native(NativeLook {
                    image_type: index("NegativeFilm"),
                    levels: [6, 3, 4, 2, 4, 4, 5, 4, 7],
                    tone: 0,
                    intensity: 0,
                    filter: 0,
                    grain_strength: 2,
                    grain_size: 1,
                    solarization: 0,
                }),
                grain: None,
                glow: None,
                white_balance: Some(WhiteBalance {
                    mode: 14,
                    shift_ba: 14,
                    shift_gm: 14,
                    kelvin: 3200,
                }),
                corrections: [None; 4],
                exposure: Exposure::default(),
                enabled: true,
                hidden: false,
                number: 0,
            },
            Recipe {
                name: "Solar".into(),
                family: Family::Gr4Monochrome,
                look: Look::Native(NativeLook {
                    image_type: index("MonochromeSolarization"),
                    levels: [4, 4, 5, 4, 4, 4, 3, 4, 6],
                    tone: 1,
                    intensity: 0,
                    filter: 0,
                    grain_strength: 0,
                    grain_size: 0,
                    solarization: 2,
                }),
                grain: Some(Grain { level: 4, size: 3 }),
                glow: Some(Glow {
                    bloom: 0,
                    halation: 2,
                    halation_color: None,
                }),
                white_balance: None,
                corrections: [Some(0), Some(4), Some(0), Some(5)],
                exposure: Exposure::default(),
                enabled: true,
                hidden: false,
                number: 0,
            },
        ]
    }

    #[test]
    fn a_library_round_trips() {
        let recipes = sample();
        let bytes = encode(&recipes).unwrap();
        assert_eq!(bytes.len(), HEADER_SIZE + 3 * RECORD_SIZE);
        assert_eq!(decode(&bytes).unwrap(), recipes);
        // Each word stays below 2^29, as the camera checks.
        for recipe in &recipes {
            assert!(recipe.words().unwrap().iter().all(|&w| w < WORD_LIMIT));
        }
    }

    #[test]
    fn the_words_follow_the_camera_layout() {
        let words = sample()[1].words().unwrap();
        // Film Look Off, the native type + 1, ColorTemp1 + 1, no shift.
        assert_eq!(words[0], USED | LOOK_OFF);
        assert_eq!(words[2] & 31, index("NegativeFilm") as u32 + 1);
        assert_eq!(words[2] >> MODE_SHIFT & 31, 15);
        assert_eq!(words[3] & 1023, 70);
        assert_eq!(words[3] >> NATIVE_GRAIN_SHIFT & 3, 2);
        // Saturation 6 is digit 2, the first and lowest.
        assert_eq!(words[1] % 9, 2);
        let effect = sample()[0].words().unwrap();
        assert_eq!(effect[0] & 3, LOOK_EFFECT);
        assert_eq!(effect[0] >> CODE_SHIFT & CODE_MASK, 0x1234);
        assert_eq!(effect[0] >> GRAIN_LEVEL_SHIFT & 7, 2);
        assert_eq!(effect[3] >> CORRECTION_SHIFT & 7, 3);
        assert_eq!(effect[3] >> (CORRECTION_SHIFT + 3) & 7, 0);
        // Bloom 1, Halation 3, color 2: held bit, then the value.
        assert_eq!(effect[MODULE_WORD] & 0x1ff, 0b101_111_011);
        // -2/3 stop is -4 sixths + 31, ISO 3200 the 16th third stop, Spot 2 + 1,
        // ND Auto 0 + 1.
        assert_eq!(effect[MODULE_WORD] >> COMPENSATION_SHIFT & 63, 27);
        assert_eq!(effect[MODULE_WORD] >> ISO_SHIFT & 63, 16);
        assert_eq!(effect[MODULE_WORD] >> METERING_SHIFT & 7, 3);
        assert_eq!(effect[MODULE_WORD] >> ND_SHIFT & 3, 1);
        assert_eq!(words[MODULE_WORD], 0);
        // Without a color, the camera keeps its own.
        assert_eq!(sample()[2].words().unwrap()[MODULE_WORD], 0b101_001);
    }

    #[test]
    fn exposure_values_that_the_camera_does_not_have_are_rejected() {
        let mut recipes = sample();
        recipes[0].exposure.compensation = Some(16);
        assert!(recipes[0].check().is_err(), "more than 5 stops");
        let mut recipes = sample();
        recipes[0].exposure.iso_limit = Some(204_800);
        assert!(recipes[0].check().is_err(), "GR III stops at ISO 102400");
        recipes[0].exposure.iso_limit = Some(3000);
        assert!(recipes[0].check().is_err(), "not a third stop");
        recipes[1].exposure.iso_limit = Some(204_800);
        assert!(recipes[1].check().is_ok());
        // A library recipe holds its compensation in thirds.
        assert!(Exposure::of_bits(30 << COMPENSATION_SHIFT).is_err());
    }

    #[test]
    fn the_recipe_file_round_trips_and_keeps_the_import() {
        let mut recipes = sample()
            .into_iter()
            .filter(|recipe| recipe.family == Family::Gr3)
            .collect::<Vec<_>>();
        let mut camera = recipes[0].clone();
        camera.name = String::new();
        camera.number = 2;
        camera.hidden = true;
        recipes.push(camera);
        let previous = format!(
            "{}\r\n123\r\n991000000\r\n5\r\n",
            crate::runtime::RECIPE_IMPORTED_KEY
        );
        let text = write_recipe_file(&recipes, &previous).unwrap();
        assert!(text.contains(&format!(
            "{}\r\n123\r\n",
            crate::runtime::RECIPE_IMPORTED_KEY
        )));
        assert_eq!(read_recipe_file(&text, Family::Gr3), recipes);
        // Recipe 1 has the automatic name and number 2, and is hidden.
        let w0 = recipes[1].words().unwrap()[0];
        assert_eq!(w0 >> NUMBER_SHIFT & 15, 2);
        assert_eq!(w0 >> HIDDEN_SHIFT & 1, 1);
        assert!(text.contains(&format!("991000001\r\n{w0}\r\n")));
        // A package record needs a name.
        assert!(encode(&recipes).is_err());
        assert!(encode(&recipes[..1]).is_ok());
    }

    #[test]
    fn glow_holds_bloom_and_halation_together() {
        assert!(glow_of_word(0b011).is_err());
        assert!(glow_of_word(0b111_000_000).is_err());
        let mut recipes = sample();
        recipes[2].glow = Some(Glow {
            bloom: 0,
            halation: 2,
            halation_color: Some(1),
        });
        assert!(
            recipes[2].check().is_err(),
            "Monochrome has no Halation color"
        );
        recipes[0].glow = Some(Glow {
            bloom: 4,
            halation: 0,
            halation_color: None,
        });
        assert!(recipes[0].check().is_err());
    }

    #[test]
    fn values_that_the_camera_does_not_have_are_rejected() {
        let mut recipes = sample();
        recipes[0].family = Family::Gr4Monochrome;
        assert!(
            recipes[0].check().is_err(),
            "Monochrome has no white balance"
        );
        let mut recipes = sample();
        let Look::Native(look) = &mut recipes[1].look else {
            unreachable!()
        };
        look.image_type = index("CinemaYellow");
        recipes[1].family = Family::Gr3;
        assert!(recipes[1].check().is_err(), "GR III has no Cinema");
        let mut recipes = sample();
        recipes[2].corrections[0] = Some(3);
        assert!(
            recipes[2].check().is_err(),
            "highlight correction has 3 values"
        );
        let mut recipes = sample();
        recipes[0].name = "A name that is too long".into();
        assert!(recipes[0].check().is_err());
        // An empty name is the automatic name of the camera.
        recipes[0].name.clear();
        assert!(recipes[0].check().is_ok());
        recipes[0].name = "負片".into();
        assert!(
            recipes[0].check().is_err(),
            "the English font draws only ASCII"
        );
        assert!(encode(&vec![sample()[0].clone(); LIBRARY_MAX + 1]).is_err());
    }

    /// A library holds `LIBRARY_MAX` recipes, at most `ON_MAX` of them on. The
    /// recipe file has those that are on first; a recipe that is off keeps
    /// its off bit through the file and the package library.
    #[test]
    fn recipes_that_are_off_follow_those_that_are_on() {
        let mut recipes = vec![sample()[0].clone(); LIBRARY_MAX];
        for (index, recipe) in recipes.iter_mut().enumerate() {
            recipe.name = format!("R{index}");
            recipe.enabled = index >= LIBRARY_MAX - ON_MAX;
        }
        assert!(encode(&recipes).is_ok());
        let text = write_recipe_file(&recipes, "").unwrap();
        let read = read_recipe_file(&text, Family::Gr3);
        assert_eq!(read.len(), LIBRARY_MAX);
        assert!(read[..ON_MAX].iter().all(|recipe| recipe.enabled));
        assert!(read[ON_MAX..].iter().all(|recipe| !recipe.enabled));
        assert_eq!(read[0].name, format!("R{}", LIBRARY_MAX - ON_MAX));
        assert_eq!(read[ON_MAX].name, "R0");
        assert_eq!(decode(&encode(&recipes).unwrap()).unwrap(), recipes);
        // The off bit is bit 28 of word 4, past the exposure settings.
        assert_eq!(recipes[0].words().unwrap()[MODULE_WORD] >> OFF_SHIFT, 1);
        recipes[0].enabled = true;
        let error = encode(&recipes).unwrap_err();
        assert!(format!("{error:#}").contains("At most 40"), "{error:#}");
        assert!(write_recipe_file(&recipes, "").is_err());
    }

    #[test]
    fn the_effect_code_is_that_of_the_installer() {
        // FNV-1a of "portra-400" in the low 15 bits.
        let hash = "portra-400".bytes().fold(0x811c_9dc5u32, |h, b| {
            (h ^ u32::from(b)).wrapping_mul(0x0100_0193)
        });
        assert_eq!(u32::from(effect_code("portra-400")), hash & 0x7fff);
    }

    /// Each preset holds a recipe that the camera can hold, for distinct
    /// camera lines, with the code of a built-in effect. A community recipe
    /// names its author and its post.
    #[test]
    fn the_presets_are_recipes_of_the_camera() {
        assert_eq!(native::TYPES[NEGATIVE_FILM].label, "NegativeFilm");
        for preset in presets() {
            assert!(encode(&preset.recipes).is_ok(), "{}", preset.id);
            let families: std::collections::BTreeSet<_> = preset
                .recipes
                .iter()
                .map(|recipe| recipe.family.code())
                .collect();
            assert_eq!(families.len(), preset.recipes.len(), "{}", preset.id);
            assert_eq!(
                preset.author.is_some(),
                preset.source.is_some(),
                "{}",
                preset.id
            );
            assert!(
                preset.source.is_none_or(|url| url.starts_with("https://")),
                "{}",
                preset.id
            );
            for recipe in &preset.recipes {
                match (&recipe.look, preset.effect) {
                    (Look::Effect { code, .. }, Some(effect)) => {
                        assert_eq!(*code, effect_code(effect));
                        let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                            .join("../../effects")
                            .join(effect)
                            .join("effect.json");
                        assert!(source.exists(), "{} needs the effect {effect}", preset.id);
                    }
                    (Look::Effect { .. }, None) => panic!("{} names no effect", preset.id),
                    (_, effect) => assert!(effect.is_none(), "{}", preset.id),
                }
            }
        }
    }

    #[test]
    fn a_damaged_file_is_rejected() {
        let mut bytes = encode(&sample()).unwrap();
        bytes[HEADER_SIZE + 5] ^= 1;
        assert!(decode(&bytes).is_err());
        let bytes = encode(&sample()).unwrap();
        assert!(decode(&bytes[..bytes.len() - 1]).is_err());
    }
}
