//! A recipe from the exposure, grain, and white-balance hints of an XMP preset.

use super::{effect_code, Exposure, Family, Grain, Look, Recipe, WhiteBalance};
use crate::native::{
    self, COLOR_TEMPERATURE_MODE, EXPOSURE_THIRDS_MAX, TEMPERATURE_MAX, TEMPERATURE_MIN,
    TEMPERATURE_STEP,
};
use crate::xmp::{Hints, WhiteBalance as PresetWhiteBalance};

/// Provisional: Lightroom IncrementalTemperature ±100 maps to this many A-B steps.
const WB_SHIFT_STEPS: i8 = 7;
/// Provisional: Lightroom Custom Tint ±150 maps to ±`WB_SHIFT_STEPS` on the G-M axis.
const CUSTOM_TINT_RANGE: f64 = 150.0;
/// Provisional: Lightroom IncrementalTemperature / IncrementalTint ±100.
const INCREMENTAL_RANGE: f64 = 100.0;
/// Provisional: GrainAmount steps of 25 map to GRKit grain levels 1 to 4.
const GRAIN_AMOUNT_STEP: f64 = 25.0;
/// Provisional: GrainSize below this is small (size 1).
const GRAIN_SIZE_SMALL: f64 = 20.0;
/// Provisional: GrainSize below this is medium (size 2); else large (size 3).
const GRAIN_SIZE_MEDIUM: f64 = 50.0;
/// Provisional: default GrainSize when the preset omits it.
const GRAIN_SIZE_DEFAULT: f64 = 25.0;
/// Provisional: kelvin when a non-temperature mode still stores a value.
const DEFAULT_KELVIN: u32 = 5500;

/// A recipe that carries the exposure, grain, and white balance of a preset for the imported
/// film look `effect_id`. Returns `None` when the hints give none of those three.
pub fn from_preset(hints: &Hints, family: Family, name: &str, effect_id: &str) -> Option<Recipe> {
    let exposure = exposure(hints);
    let grain = grain(hints);
    let white_balance = white_balance(hints, family);
    if exposure.compensation.is_none() && grain.is_none() && white_balance.is_none() {
        return None;
    }
    Some(Recipe {
        name: name.into(),
        family,
        look: Look::Effect {
            code: effect_code(effect_id),
            adjustments: [0; 9],
        },
        grain,
        glow: None,
        grain_style: None,
        border: None,
        white_balance,
        corrections: [None; 4],
        exposure,
        enabled: true,
        hidden: false,
        number: 0,
        id: 0,
    })
}

fn exposure(hints: &Hints) -> Exposure {
    let Some(stops) = hints.exposure else {
        return Exposure::default();
    };
    // Provisional: one stop is three thirds on the camera.
    let thirds = (stops * 3.0)
        .round()
        .clamp(-(EXPOSURE_THIRDS_MAX as f64), EXPOSURE_THIRDS_MAX as f64) as i8;
    Exposure {
        compensation: (thirds != 0).then_some(thirds),
        ..Exposure::default()
    }
}

fn grain(hints: &Hints) -> Option<Grain> {
    let amount = hints.grain_amount?;
    if amount == 0.0 {
        return None;
    }
    let level = ((amount / GRAIN_AMOUNT_STEP).ceil() as u8).clamp(1, 4);
    let size_value = hints.grain_size.unwrap_or(GRAIN_SIZE_DEFAULT);
    let size = if size_value < GRAIN_SIZE_SMALL {
        1
    } else if size_value < GRAIN_SIZE_MEDIUM {
        2
    } else {
        3
    };
    Some(Grain { level, size })
}

fn white_balance(hints: &Hints, family: Family) -> Option<WhiteBalance> {
    if family.monochrome() {
        return None;
    }
    let mode = match &hints.white_balance {
        PresetWhiteBalance::AsShot => {
            if hints.incremental_temperature.is_none() && hints.incremental_tint.is_none() {
                return None;
            }
            // As Shot with a shift: Auto of the family (GR IV: AutoStandard).
            mode_named(family, "Auto")?
        }
        PresetWhiteBalance::Auto => mode_named(family, "Auto")?,
        PresetWhiteBalance::Daylight => mode_named(family, "Daylight")?,
        PresetWhiteBalance::Cloudy => mode_named(family, "Cloudiness")?,
        PresetWhiteBalance::Shade => mode_named(family, "Shade")?,
        PresetWhiteBalance::Tungsten => mode_named(family, "Tungsten")?,
        // First fluorescent mode (Flu_D); the camera has Flu_D/N/W/L.
        PresetWhiteBalance::Fluorescent => mode_named(family, "Flu_D")?,
        // The camera has no Flash white balance.
        PresetWhiteBalance::Flash => return None,
        PresetWhiteBalance::Custom { .. } => COLOR_TEMPERATURE_MODE,
    };
    if native::WHITE_BALANCE
        .get(mode)
        .is_none_or(|m| m.labels[family.native() as usize].is_none())
    {
        return None;
    }
    let (shift_ba, shift_gm, kelvin) = match &hints.white_balance {
        PresetWhiteBalance::Custom { temperature, tint } => (
            shift_steps(hints.incremental_temperature, INCREMENTAL_RANGE),
            shift_steps(Some(*tint), CUSTOM_TINT_RANGE),
            clamp_kelvin(*temperature),
        ),
        _ => (
            shift_steps(hints.incremental_temperature, INCREMENTAL_RANGE),
            shift_steps(hints.incremental_tint, INCREMENTAL_RANGE),
            DEFAULT_KELVIN,
        ),
    };
    Some(WhiteBalance {
        mode,
        shift_ba,
        shift_gm,
        kelvin,
    })
}

/// The portable index of a white-balance label of the family, or of the GR III label when the
/// GR IV name differs (Auto → AutoStandard).
fn mode_named(family: Family, key: &str) -> Option<usize> {
    let index = family.native() as usize;
    // Auto: first mode whose label starts with "Auto" (Auto / AutoStandard), not AutoWarm.
    if key == "Auto" {
        return native::WHITE_BALANCE.iter().position(|mode| {
            mode.labels[index].is_some_and(|label| label == "Auto" || label == "AutoStandard")
        });
    }
    native::WHITE_BALANCE
        .iter()
        .position(|mode| mode.labels[index] == Some(key))
}

/// Stored shift 0..=28 with 14 for none.
///
/// SHIFT_BA runs B14 (0) through NORMAL (14) to A14 (28): warmer (amber) is above 14.
/// SHIFT_GM runs G14 (0) through NORMAL (14) to M14 (28): magenta is above 14.
/// presets.rs states A-B with A positive and G-M with M positive the same way.
fn shift_steps(value: Option<f64>, range: f64) -> u8 {
    let steps = value.map_or(0.0, |v| {
        ((v / range) * f64::from(WB_SHIFT_STEPS))
            .round()
            .clamp(-f64::from(WB_SHIFT_STEPS), f64::from(WB_SHIFT_STEPS))
    });
    (14 + steps as i8) as u8
}

fn clamp_kelvin(kelvin: f64) -> u32 {
    let rounded = kelvin.round() as i64;
    let clamped = rounded.clamp(TEMPERATURE_MIN as i64, TEMPERATURE_MAX as i64) as u32;
    let step = TEMPERATURE_STEP;
    let stepped = TEMPERATURE_MIN + ((clamped - TEMPERATURE_MIN + step / 2) / step) * step;
    stepped.clamp(TEMPERATURE_MIN, TEMPERATURE_MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::xmp::WhiteBalance as PresetWhiteBalance;

    fn empty_hints() -> Hints {
        Hints::default()
    }

    #[test]
    fn empty_hints_give_no_recipe() {
        assert!(from_preset(&empty_hints(), Family::Gr3, "Look", "look-v1").is_none());
        assert!(Hints::default().is_empty());
    }

    #[test]
    fn exposure_thirds_and_clamp() {
        let mut hints = empty_hints();
        hints.exposure = Some(0.5);
        let recipe = from_preset(&hints, Family::Gr3, "Look", "look-v1").unwrap();
        assert_eq!(recipe.exposure.compensation, Some(2));

        hints.exposure = Some(0.0);
        assert!(from_preset(&hints, Family::Gr3, "Look", "look-v1").is_none());

        hints.exposure = Some(10.0);
        let recipe = from_preset(&hints, Family::Gr3, "Look", "look-v1").unwrap();
        assert_eq!(
            recipe.exposure.compensation,
            Some(EXPOSURE_THIRDS_MAX as i8)
        );

        hints.exposure = Some(-10.0);
        let recipe = from_preset(&hints, Family::Gr3, "Look", "look-v1").unwrap();
        assert_eq!(
            recipe.exposure.compensation,
            Some(-(EXPOSURE_THIRDS_MAX as i8))
        );
    }

    #[test]
    fn grain_levels_and_sizes() {
        let mut hints = empty_hints();
        hints.grain_amount = Some(25.0);
        hints.grain_size = Some(10.0);
        let recipe = from_preset(&hints, Family::Gr3, "Look", "look-v1").unwrap();
        assert_eq!(recipe.grain, Some(Grain { level: 1, size: 1 }));

        hints.grain_amount = Some(26.0);
        hints.grain_size = Some(25.0);
        assert_eq!(
            from_preset(&hints, Family::Gr3, "Look", "look-v1")
                .unwrap()
                .grain,
            Some(Grain { level: 2, size: 2 })
        );

        hints.grain_amount = Some(76.0);
        hints.grain_size = Some(50.0);
        assert_eq!(
            from_preset(&hints, Family::Gr3, "Look", "look-v1")
                .unwrap()
                .grain,
            Some(Grain { level: 4, size: 3 })
        );

        hints.grain_amount = Some(100.0);
        hints.grain_size = None;
        assert_eq!(
            from_preset(&hints, Family::Gr3, "Look", "look-v1")
                .unwrap()
                .grain,
            Some(Grain { level: 4, size: 2 })
        );
    }

    #[test]
    fn white_balance_modes_per_family() {
        let mut hints = empty_hints();
        hints.white_balance = PresetWhiteBalance::Daylight;
        let gr3 = from_preset(&hints, Family::Gr3, "Look", "look-v1").unwrap();
        assert_eq!(
            gr3.white_balance.unwrap().mode,
            mode_named(Family::Gr3, "Daylight").unwrap()
        );

        let gr4 = from_preset(&hints, Family::Gr4, "Look", "look-v1").unwrap();
        assert_eq!(
            gr4.white_balance.unwrap().mode,
            mode_named(Family::Gr4, "Daylight").unwrap()
        );

        assert!(from_preset(&hints, Family::Gr4Monochrome, "Look", "look-v1").is_none());

        hints.white_balance = PresetWhiteBalance::Cloudy;
        assert_eq!(
            from_preset(&hints, Family::Gr3, "Look", "look-v1")
                .unwrap()
                .white_balance
                .unwrap()
                .mode,
            mode_named(Family::Gr3, "Cloudiness").unwrap()
        );

        hints.white_balance = PresetWhiteBalance::Fluorescent;
        assert_eq!(
            from_preset(&hints, Family::Gr4, "Look", "look-v1")
                .unwrap()
                .white_balance
                .unwrap()
                .mode,
            mode_named(Family::Gr4, "Flu_D").unwrap()
        );

        hints.white_balance = PresetWhiteBalance::Flash;
        assert!(from_preset(&hints, Family::Gr3, "Look", "look-v1").is_none());

        hints.white_balance = PresetWhiteBalance::Auto;
        assert_eq!(
            from_preset(&hints, Family::Gr4, "Look", "look-v1")
                .unwrap()
                .white_balance
                .unwrap()
                .mode,
            0
        );
    }

    #[test]
    fn as_shot_shift_uses_auto_and_none_without_shift() {
        let mut hints = empty_hints();
        hints.white_balance = PresetWhiteBalance::AsShot;
        assert!(from_preset(&hints, Family::Gr3, "Look", "look-v1").is_none());

        hints.incremental_temperature = Some(50.0);
        let recipe = from_preset(&hints, Family::Gr3, "Look", "look-v1").unwrap();
        let wb = recipe.white_balance.unwrap();
        assert_eq!(wb.mode, 0);
        // Warmer toward amber: positive temperature raises shift_ba above 14.
        assert_eq!(wb.shift_ba, 14 + 4);
    }

    #[test]
    fn shift_axes_amber_and_magenta() {
        let mut hints = empty_hints();
        hints.white_balance = PresetWhiteBalance::Daylight;
        hints.incremental_temperature = Some(100.0);
        hints.incremental_tint = Some(100.0);
        let wb = from_preset(&hints, Family::Gr3, "Look", "look-v1")
            .unwrap()
            .white_balance
            .unwrap();
        assert_eq!(wb.shift_ba, 14 + WB_SHIFT_STEPS as u8);
        assert_eq!(wb.shift_gm, 14 + WB_SHIFT_STEPS as u8);

        hints.incremental_temperature = Some(-100.0);
        hints.incremental_tint = Some(-100.0);
        let wb = from_preset(&hints, Family::Gr3, "Look", "look-v1")
            .unwrap()
            .white_balance
            .unwrap();
        assert_eq!(wb.shift_ba, 14 - WB_SHIFT_STEPS as u8);
        assert_eq!(wb.shift_gm, 14 - WB_SHIFT_STEPS as u8);

        hints.incremental_temperature = None;
        hints.incremental_tint = None;
        hints.white_balance = PresetWhiteBalance::Custom {
            temperature: 3200.0,
            tint: 150.0,
        };
        let wb = from_preset(&hints, Family::Gr3, "Look", "look-v1")
            .unwrap()
            .white_balance
            .unwrap();
        assert_eq!(wb.mode, COLOR_TEMPERATURE_MODE);
        assert_eq!(wb.kelvin, 3200);
        assert_eq!(wb.shift_gm, 14 + WB_SHIFT_STEPS as u8);
        assert_eq!(wb.shift_ba, 14);
    }

    #[test]
    fn custom_kelvin_clamps_to_camera_range() {
        let mut hints = empty_hints();
        hints.white_balance = PresetWhiteBalance::Custom {
            temperature: 1000.0,
            tint: 0.0,
        };
        let wb = from_preset(&hints, Family::Gr3, "Look", "look-v1")
            .unwrap()
            .white_balance
            .unwrap();
        assert_eq!(wb.kelvin, TEMPERATURE_MIN);

        hints.white_balance = PresetWhiteBalance::Custom {
            temperature: 20000.0,
            tint: 0.0,
        };
        let wb = from_preset(&hints, Family::Gr3, "Look", "look-v1")
            .unwrap()
            .white_balance
            .unwrap();
        assert_eq!(wb.kelvin, TEMPERATURE_MAX);

        hints.white_balance = PresetWhiteBalance::Custom {
            temperature: 5512.0,
            tint: 0.0,
        };
        let wb = from_preset(&hints, Family::Gr3, "Look", "look-v1")
            .unwrap()
            .white_balance
            .unwrap();
        assert_eq!(wb.kelvin, 5510);
    }

    #[test]
    fn effect_look_without_glow_border_or_corrections() {
        let mut hints = empty_hints();
        hints.exposure = Some(1.0);
        let recipe = from_preset(&hints, Family::Gr3, "Look", "vision3-500t-supra-v1").unwrap();
        assert_eq!(
            recipe.look,
            Look::Effect {
                code: effect_code("vision3-500t-supra-v1"),
                adjustments: [0; 9],
            }
        );
        assert!(recipe.glow.is_none());
        assert!(recipe.border.is_none());
        assert_eq!(recipe.corrections, [None; 4]);
        assert!(recipe.enabled);
        assert!(!recipe.hidden);
        recipe.check().unwrap();
    }
}
