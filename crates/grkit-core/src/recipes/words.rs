//! The words of a recipe as the camera holds them (`layout`).

use super::layout::*;
use super::{Exposure, Family, Glow, Grain, Look, Metering, NativeLook, Nd, Recipe, WhiteBalance};
use crate::abi::RECIPE_WORDS;
use crate::native;
use anyhow::{bail, ensure, Context, Result};

/// Each word stays below 2^29, as the state file keeps it.
pub const WORD_LIMIT: u32 = 1 << VALUE_BITS;

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
        let compensation = self
            .compensation
            .map_or(0, |thirds| (2 * i32::from(thirds) + EV_ZERO as i32) as u32);
        let iso = self.iso_limit.map_or(0, |iso| {
            native::ISO_STEPS
                .iter()
                .position(|&step| step == iso)
                .unwrap() as u32
                + 1
        });
        let metering = self.metering.map_or(0, |m| m as u32 + 1);
        let nd = self.nd.map_or(0, |nd| nd as u32 + 1);
        compensation << EV_SHIFT | iso << ISO_SHIFT | metering << METERING_SHIFT | nd << ND_SHIFT
    }

    pub(super) fn of_bits(word: u32) -> Result<Exposure> {
        let compensation = (word >> EV_SHIFT & EV_MASK) as i32;
        let iso = (word >> ISO_SHIFT & ISO_MASK) as usize;
        let metering = word >> METERING_SHIFT & METERING_MASK;
        let nd = word >> ND_SHIFT & ND_MASK;
        ensure!(
            compensation == 0 || (compensation - EV_ZERO as i32) % 2 == 0,
            "A library recipe has its compensation in thirds of a stop"
        );
        Ok(Exposure {
            compensation: (compensation != 0).then(|| ((compensation - EV_ZERO as i32) / 2) as i8),
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
}

impl Recipe {
    /// The words of the recipe, as the camera holds them.
    pub fn words(&self) -> Result<[u32; RECIPE_WORDS]> {
        self.check()?;
        ensure!(
            u32::from(self.number) <= NUMBER_MAX,
            "The number of a recipe is at most {NUMBER_MAX}"
        );
        let mut w = [
            USED | u32::from(self.number) << NUMBER_SHIFT | if self.hidden { HIDDEN } else { 0 },
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
                w[FN_WORD] = digits(adjustments, u32::from);
            }
            Look::Native(look) => {
                w[0] |= LOOK_OFF;
                w[FN_WORD] = digits(&look.levels, level_digit);
                w[NATIVE_WORD] = (look.image_type as u32 + 1) << TYPE_SHIFT
                    | u32::from(look.tone) << TONE_SHIFT
                    | u32::from(look.intensity) << INTENSITY_SHIFT
                    | u32::from(look.filter) << FILTER_SHIFT;
                w[EXTRA_WORD] = u32::from(look.grain_strength) << NATIVE_GRAIN_SHIFT
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
            w[NATIVE_WORD] |= (balance.mode as u32 + 1) << MODE_SHIFT
                | u32::from(balance.shift_ba) << BA_SHIFT
                | u32::from(balance.shift_gm) << GM_SHIFT;
            w[EXTRA_WORD] |= (balance.kelvin - native::TEMPERATURE_MIN) / native::TEMPERATURE_STEP;
        }
        for (index, value) in self.corrections.iter().enumerate() {
            if let Some(value) = value {
                w[EXTRA_WORD] |=
                    (u32::from(*value) + 1) << (CORRECTION_SHIFT + CORRECTION_BITS * index as u32);
            }
        }
        w[MODULE_WORD] |= self.exposure.bits() | if self.enabled { 0 } else { OFF };
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
        let image_type = w[NATIVE_WORD] >> TYPE_SHIFT & TYPE_MASK;
        let look = match w[0] & LOOK_MASK {
            LOOK_EFFECT => Look::Effect {
                code: (w[0] >> CODE_SHIFT & CODE_MASK) as u16,
                adjustments: undigits(w[FN_WORD], |digit| digit as u8),
            },
            LOOK_OFF if image_type != 0 => Look::Native(NativeLook {
                image_type: image_type as usize - 1,
                levels: undigits(w[FN_WORD], level_of_digit),
                tone: (w[NATIVE_WORD] >> TONE_SHIFT & TONE_MASK) as u8,
                intensity: (w[NATIVE_WORD] >> INTENSITY_SHIFT & INTENSITY_MASK) as u8,
                filter: (w[NATIVE_WORD] >> FILTER_SHIFT & FILTER_MASK) as u8,
                grain_strength: (w[EXTRA_WORD] >> NATIVE_GRAIN_SHIFT & NATIVE_GRAIN_MASK) as u8,
                grain_size: (w[EXTRA_WORD] >> NATIVE_GRAIN_SIZE_SHIFT & NATIVE_GRAIN_SIZE_MASK)
                    as u8,
                solarization: (w[EXTRA_WORD] >> SOLARIZATION_SHIFT & SOLARIZATION_MASK) as u8,
            }),
            LOOK_OFF => Look::Off,
            LOOK_KEPT => Look::Kept,
            other => bail!("Unknown look {other}"),
        };
        let mode = w[NATIVE_WORD] >> MODE_SHIFT & MODE_MASK;
        let recipe = Recipe {
            name,
            family,
            look,
            grain: (w[0] & GRAIN_HELD != 0).then(|| Grain {
                level: (w[0] >> GRAIN_LEVEL_SHIFT & GRAIN_LEVEL_MASK) as u8,
                size: (w[0] >> GRAIN_SIZE_SHIFT & GRAIN_SIZE_MASK) as u8,
            }),
            white_balance: (mode != 0).then(|| WhiteBalance {
                mode: mode as usize - 1,
                shift_ba: (w[NATIVE_WORD] >> BA_SHIFT & SHIFT_MASK) as u8,
                shift_gm: (w[NATIVE_WORD] >> GM_SHIFT & SHIFT_MASK) as u8,
                kelvin: native::TEMPERATURE_MIN
                    + (w[EXTRA_WORD] & TEMPERATURE_MASK) * native::TEMPERATURE_STEP,
            }),
            corrections: std::array::from_fn(|index| {
                let value = w[EXTRA_WORD] >> (CORRECTION_SHIFT + CORRECTION_BITS * index as u32)
                    & CORRECTION_MASK;
                (value != 0).then(|| value as u8 - 1)
            }),
            glow: glow_of_word(w[MODULE_WORD])?,
            exposure: Exposure::of_bits(w[MODULE_WORD])?,
            enabled: w[MODULE_WORD] & OFF == 0,
            hidden: w[0] & HIDDEN != 0,
            number: (w[0] >> NUMBER_SHIFT & NUMBER_MAX) as u8,
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
pub(super) fn glow_of_word(word: u32) -> Result<Option<Glow>> {
    let held: [Option<u8>; 3] = std::array::from_fn(|index| {
        let bits = word >> (MODULE_SETTING_BITS * index as u32);
        (bits & 1 != 0).then_some((bits >> 1 & MODULE_VALUE_MASK) as u8)
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
