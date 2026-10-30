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
    RCP_BYTES, RCP_COUNT, RCP_CRC, RCP_MAGIC, RCP_RECORD_FAMILY, RCP_RECORD_HINTS, RCP_RECORD_NAME,
    RCP_RECORD_WORDS, RECIPE_WORDS,
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
/// The most recipes of a library.
pub const LIBRARY_MAX: usize = 24;
/// The most UTF-16 units of a name.
pub const NAME_UNITS: usize = 16;

/// The GRKit Grain levels: Off to Extra Strong, and the sizes: Auto, then
/// small to large (`CORE.GRAIN`, `CORE.GRAIN_SIZE`).
pub const GRAIN_LEVELS: u8 = 5;
pub const GRAIN_SIZES: u8 = 4;
/// The exposure compensation of a hint, in thirds of a stop.
pub const EXPOSURE_THIRDS_MAX: i8 = 15;
/// The largest ISO limit of a hint.
pub const ISO_MAX: u32 = 204_800;

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

/// The metering of an exposure hint.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Metering {
    Multi,
    Center,
    Spot,
    Highlight,
}

/// The ND filter of an exposure hint.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Nd {
    Off,
    On,
    Auto,
}

/// Exposure settings that the camera shows after apply and does not write.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hints {
    /// Exposure compensation in thirds of a stop.
    pub exposure: Option<i8>,
    /// The highest ISO of Auto ISO.
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
    pub white_balance: Option<WhiteBalance>,
    /// The stored value of each correction setting
    /// (`native::CORRECTIONS`), or `None` for Unset.
    pub corrections: [Option<u8>; 4],
    #[serde(default)]
    pub hints: Hints,
}

// The words of a recipe (runtime/recipes.h and runtime/native_look.h).
const USED: u32 = 0x0800_0000;
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
/// Each word stays below 2^29, as the state file keeps it.
pub const WORD_LIMIT: u32 = 1 << 29;

// The hint word.
const EXPOSURE_BITS: u32 = 0x3f;
const ISO_SHIFT: u32 = 6;
const ISO_BITS: u32 = 0x3_ffff;
const METERING_SHIFT: u32 = 24;
const ND_SHIFT: u32 = 27;

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

impl Hints {
    pub fn word(&self) -> u32 {
        let exposure = self
            .exposure
            .map_or(0, |thirds| (i32::from(thirds) + 16) as u32);
        let iso = self.iso_limit.unwrap_or(0);
        let metering = self.metering.map_or(0, |m| m as u32 + 1);
        let nd = self.nd.map_or(0, |nd| nd as u32 + 1);
        exposure | iso << ISO_SHIFT | metering << METERING_SHIFT | nd << ND_SHIFT
    }

    fn from_word(word: u32) -> Result<Hints> {
        let exposure = word & EXPOSURE_BITS;
        let iso = word >> ISO_SHIFT & ISO_BITS;
        let metering = word >> METERING_SHIFT & 7;
        let nd = word >> ND_SHIFT & 3;
        ensure!(word >> 29 == 0, "Unknown hint bits in {word:#x}");
        let hints = Hints {
            exposure: (exposure != 0).then(|| exposure as i8 - 16),
            iso_limit: (iso != 0).then_some(iso),
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
                1 => Some(Nd::Off),
                2 => Some(Nd::On),
                _ => Some(Nd::Auto),
            },
        };
        hints.check()?;
        Ok(hints)
    }

    fn check(&self) -> Result<()> {
        if let Some(thirds) = self.exposure {
            ensure!(
                thirds.abs() <= EXPOSURE_THIRDS_MAX,
                "Exposure compensation {thirds}/3 is out of range"
            );
        }
        if let Some(iso) = self.iso_limit {
            ensure!((100..=ISO_MAX).contains(&iso), "ISO {iso} is out of range");
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
        self.hints.check()
    }

    /// The words of the recipe, as the camera holds them.
    pub fn words(&self) -> Result<[u32; RECIPE_WORDS]> {
        self.check()?;
        let mut w = [USED, 0, 0, 0];
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
        debug_assert!(w.iter().all(|&word| word < WORD_LIMIT));
        Ok(w)
    }

    /// The recipe of the words of a record.
    pub fn from_words(
        name: String,
        family: Family,
        w: [u32; RECIPE_WORDS],
        hints: Hints,
    ) -> Result<Recipe> {
        ensure!(
            w[0] & USED != 0 && w.iter().all(|&word| word < WORD_LIMIT),
            "Not the words of a recipe"
        );
        ensure!(w[0] >> 23 & 15 == 0, "A library recipe has no number");
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
            hints,
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

/// A name of 1 to `NAME_UNITS` printable ASCII characters. The camera draws
/// a name with the font of its menu language, and only ASCII is in each one:
/// the English font has no other characters. The file holds UTF-16 units.
pub fn check_name(name: &str) -> Result<()> {
    ensure!(
        (1..=NAME_UNITS).contains(&name.len()),
        "A recipe name has 1 to {NAME_UNITS} characters"
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

/// The values of a stored setting: each stored value and its label.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Choice {
    pub value: u32,
    pub label: &'static str,
}

/// A parameter of a type in the model of a family.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelParameter {
    pub key: &'static str,
    pub choices: Vec<Choice>,
}

/// A type in the model of a family.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelType {
    /// The portable index (`NativeLook::image_type`).
    pub index: usize,
    pub label: &'static str,
    /// The camera name in the requested language.
    pub name: &'static str,
    pub parameters: Vec<ModelParameter>,
    /// The type has native grain, and its sizes.
    pub grain: bool,
    pub grain_sizes: bool,
}

/// A white balance mode in the model of a family.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelMode {
    pub index: usize,
    pub label: &'static str,
    pub shifts: bool,
    pub temperature: bool,
}

/// The values that a recipe of a family can hold, for the form of the site.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Model {
    pub types: Vec<ModelType>,
    pub modes: Vec<ModelMode>,
    /// The values of each correction setting (`native::CORRECTIONS`).
    pub corrections: Vec<ModelParameter>,
}

/// The model of a family, with the camera names of the types in `language`.
pub fn model(family: Family, language: crate::language::Language) -> Model {
    let native_family = family.native();
    let names = crate::menu_text::native_type_names(language);
    let choices = |labels: native::Labels| -> Vec<Choice> {
        labels
            .values()
            .map(|(label, value)| Choice { value, label })
            .collect()
    };
    let types = native::TYPES
        .iter()
        .enumerate()
        .filter(|(_, image_type)| image_type.monochrome == family.monochrome())
        .filter_map(|(index, image_type)| {
            let (_, keys) = image_type.families[native_family as usize]?;
            Some(ModelType {
                index,
                label: image_type.label,
                name: names[index],
                parameters: keys
                    .iter()
                    .filter(|key| !key.starts_with("Particle"))
                    .map(|&key| ModelParameter {
                        key,
                        choices: choices(
                            native::type_parameter_labels(
                                native_family,
                                index,
                                native::parameter(key),
                            )
                            .unwrap(),
                        ),
                    })
                    .collect(),
                grain: keys.contains(&"Particle"),
                grain_sizes: keys.contains(&"ParticleSize"),
            })
        })
        .collect();
    let modes = if family.monochrome() {
        Vec::new()
    } else {
        native::WHITE_BALANCE
            .iter()
            .enumerate()
            .filter_map(|(index, mode)| {
                Some(ModelMode {
                    index,
                    label: mode.labels[native_family as usize]?,
                    shifts: mode.shifts,
                    temperature: index == native::COLOR_TEMPERATURE_MODE,
                })
            })
            .collect()
    };
    Model {
        types,
        modes,
        corrections: native::CORRECTIONS
            .iter()
            .map(|correction| ModelParameter {
                key: correction.name,
                choices: choices(correction.labels),
            })
            .collect(),
    }
}

/// `web/src/generated/recipes.ts`: the model of each family with the English
/// type names, and the camera names of the types in each site language.
fn site_module() -> String {
    use crate::language::Language;
    let mut out = String::from(
        "// Generated from crates/grkit-core/src/recipes.rs. Do not edit: change the\n\
         // tables there, then run\n\
         // GRKIT_UPDATE_GENERATED=1 cargo test -p grkit-core --test generated.\n\n\
         import type { Family, Model } from '../recipes';\n\n\
         /** The values that a recipe of each camera line can hold, with the English type names. */\n\
         export const RECIPE_MODELS: Readonly<Record<Family, Model>> = ",
    );
    out.push_str("{\n");
    for (key, family) in [
        ("gr3", Family::Gr3),
        ("gr4", Family::Gr4),
        ("gr4Monochrome", Family::Gr4Monochrome),
    ] {
        let model = serde_json::to_string(&model(family, Language::English)).unwrap();
        writeln!(out, "  {key}: {model},").unwrap();
    }
    out.push_str(
        "};\n\n/** The camera names of the types in each site language, by portable index. */\n",
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

/// The bytes of `RECIPES.BIN` for a library.
pub fn encode(recipes: &[Recipe]) -> Result<Vec<u8>> {
    ensure!(
        recipes.len() <= LIBRARY_MAX,
        "A library has at most {LIBRARY_MAX} recipes"
    );
    let mut bytes = vec![0; HEADER_SIZE + RECORD_SIZE * recipes.len()];
    bytes[RCP_MAGIC..RCP_MAGIC + 8].copy_from_slice(MAGIC);
    let size = bytes.len() as u32;
    put(&mut bytes, RCP_BYTES, size);
    put(&mut bytes, RCP_COUNT, recipes.len() as u32);
    for (index, recipe) in recipes.iter().enumerate() {
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
        put(&mut bytes, record + RCP_RECORD_HINTS, recipe.hints.word());
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
            let hints = Hints::from_word(get(record, RCP_RECORD_HINTS))?;
            Recipe::from_words(name, family, words, hints)
                .with_context(|| format!("Recipe {}", index + 1))
        })
        .collect()
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
                white_balance: Some(WhiteBalance {
                    mode: 4,
                    shift_ba: 17,
                    shift_gm: 12,
                    kelvin: 5500,
                }),
                corrections: [Some(2), None, Some(1), Some(0)],
                hints: Hints {
                    exposure: Some(-2),
                    iso_limit: Some(3200),
                    metering: Some(Metering::Spot),
                    nd: Some(Nd::Auto),
                },
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
                white_balance: Some(WhiteBalance {
                    mode: 14,
                    shift_ba: 14,
                    shift_gm: 14,
                    kelvin: 3200,
                }),
                corrections: [None; 4],
                hints: Hints::default(),
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
                white_balance: None,
                corrections: [Some(0), Some(4), Some(0), Some(5)],
                hints: Hints::default(),
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
        recipes[0].name.clear();
        assert!(recipes[0].check().is_err());
        recipes[0].name = "負片".into();
        assert!(
            recipes[0].check().is_err(),
            "the English font draws only ASCII"
        );
        assert!(encode(&vec![sample()[0].clone(); LIBRARY_MAX + 1]).is_err());
    }

    #[test]
    fn the_effect_code_is_that_of_the_installer() {
        // FNV-1a of "portra-400" in the low 15 bits.
        let hash = "portra-400".bytes().fold(0x811c_9dc5u32, |h, b| {
            (h ^ u32::from(b)).wrapping_mul(0x0100_0193)
        });
        assert_eq!(u32::from(effect_code("portra-400")), hash & 0x7fff);
    }

    #[test]
    fn the_model_of_each_family_holds_its_types() {
        use crate::language::Language;
        let gr3 = model(Family::Gr3, Language::English);
        assert_eq!(gr3.types.len(), 12);
        assert!(gr3.modes.iter().any(|mode| mode.label == "Multi"));
        let mono = model(Family::Gr4Monochrome, Language::Japanese);
        assert_eq!(mono.types.len(), 7);
        assert!(mono.modes.is_empty());
        assert_eq!(mono.types[0].name, "スタンダード");
        let gr4 = model(Family::Gr4, Language::English);
        assert_eq!(gr4.types.len(), 14);
        assert!(gr4
            .types
            .iter()
            .all(|t| !t.parameters.iter().any(|p| p.key.starts_with("Particle"))));
        assert!(gr4
            .modes
            .iter()
            .any(|mode| mode.temperature && mode.label == "ColorTemp1"));
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
