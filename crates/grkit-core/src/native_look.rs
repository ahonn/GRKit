//! Native looks (GRF feature 32): the MultiAxial set of a look that a camera firmware
//! defines, for another camera with the same stages.
//!
//! The MultiAxial set holds its own hue bounds and bank selectors (12 banks; bank 0 is a
//! bank like the others), all 21 control fields, and five matrices for each bank, one for
//! each level. A level matrix has a row sum of the level's gain. The file keeps the six
//! off-diagonal coefficients of level 0 (row r: columns r + 1 and r + 2, mod 3) and, for
//! levels 1 to 4, a small correction to `(c * gain + 512) >> 10` of each level-0
//! coefficient `c`. The diagonal is the gain minus the row's off-diagonals. INSTALL
//! expands the set with the same rule, so the registers equal those of the source look.
//! With highlight correction, the set does not change, as in the source firmware.
//!
//! A native look gives its default Fn adjustments with feature 64 (`effects::FN_DEFAULTS`):
//! the camera's own stages, such as its tone generator, make the look from them for each
//! picture.
//!
//! Layout: from `abi::GRF_HUE`: 16 u16 bounds, 20 u8 selectors, 21 u16 control fields, 5 u16 gains, then zeros up to
//! `effects::SIZE`. From `effects::SIZE`: 12 x 6 i16 level-0 coefficients, 4 x 12 x 6 i8
//! corrections of levels 1 to 4, then zeros or the Fn defaults of feature 64. The file has
//! `effects::SIZE_WITH_LEVELS` bytes.
use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};

use crate::abi::GRF_HUE;
use crate::cube::{self, Rgb};
use crate::effects::SIZE;
use crate::hue;

pub const BANKS: usize = 12;
pub const LEVELS: usize = 5;
pub const BOUNDS: usize = GRF_HUE;
pub const SELECTORS: usize = BOUNDS + 16 * 2;
pub const CONTROL: usize = SELECTORS + 20;
pub const GAINS: usize = CONTROL + 21 * 2;
/// The first byte after the head of the set; zeros follow up to `effects::SIZE`.
pub const HEAD_END: usize = GAINS + LEVELS * 2;
pub const MATRICES: usize = SIZE;
pub const CORRECTIONS: usize = MATRICES + BANKS * 6 * 2;
/// The first byte after the set.
pub const END: usize = CORRECTIONS + (LEVELS - 1) * BANKS * 6;
const _: () = assert!(HEAD_END <= SIZE && END <= crate::effects::SIZE_WITH_LEVELS);
use crate::effects::FN_MAX;
/// The saturation of GR IV at a shown value (-4..=4): a row-major matrix on gamma-encoded
/// RGB, after the tone curves. It is RGB x diag(1, f, f) x YCbCr with the BT.601
/// matrices in 8-bit fractions, f = 1.1 to the power of the value rounded to three
/// decimals, and the result in 9-bit fractions. GR IV skips it at 0. The GR III runtime
/// applies it for a native look (runtime/picture.c).
pub fn saturation_matrix(value: i8) -> [f64; 9] {
    const YCBCR: [i64; 9] = [77, 150, 29, -43, -85, 128, 128, -107, -21];
    const RGB: [i64; 9] = [256, 0, 359, 256, -88, -183, 256, 454, 0];
    const FACTORS: [i64; 9] = [683, 751, 826, 909, 1000, 1100, 1210, 1331, 1464];
    let value = value.clamp(-FN_MAX, FN_MAX);
    if value == 0 {
        return crate::fit::IDENTITY;
    }
    let factor = FACTORS[(value + FN_MAX) as usize];
    std::array::from_fn(|k| {
        let (i, j) = (k / 3, k % 3);
        let sum = RGB[i * 3] * YCBCR[j] * 1000
            + (RGB[i * 3 + 1] * YCBCR[3 + j] + RGB[i * 3 + 2] * YCBCR[6 + j]) * factor;
        // 512 / (256 x 256 x 1000), rounded half away from zero.
        let q9 = (sum.abs() + 64_000) / 128_000 * sum.signum();
        q9 as f64 / 512.0
    })
}

/// Control fields 14..17 are 9-bit slopes; the others hold 12 bits.
const SLOPE_FIELDS: std::ops::Range<usize> = 14..18;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeLook {
    /// Four non-increasing bounds for each Cb/Cr quadrant, 0..=2048.
    pub bounds: [u16; 16],
    /// The bank (0..12) of each of the five segments of each quadrant.
    pub selectors: [u8; 20],
    pub control: [u16; 21],
    /// The row sum of each level, 1024 = 1.
    pub gains: [u16; LEVELS],
    /// The off-diagonal coefficients of level 0 of each bank, Q10.
    pub matrices: [[i16; 6]; BANKS],
    /// The corrections of levels 1 to 4.
    pub corrections: [[[i8; 6]; BANKS]; LEVELS - 1],
}

/// The register range of a Q10 coefficient.
fn register(v: i32) -> bool {
    (-2048..=2047).contains(&v)
}

impl NativeLook {
    /// The 3x3 Q10 matrix (row-major) of a bank at a level, as INSTALL writes it.
    pub fn level_matrix(&self, bank: usize, level: usize) -> [i32; 9] {
        let gain = i32::from(self.gains[level]);
        let mut m = [0; 9];
        for r in 0..3 {
            let mut sum = 0;
            for j in 0..2 {
                let base = (i32::from(self.matrices[bank][r * 2 + j]) * gain + 512) >> 10;
                let fix = if level == 0 {
                    0
                } else {
                    i32::from(self.corrections[level - 1][bank][r * 2 + j])
                };
                m[r * 3 + (r + 1 + j) % 3] = base + fix;
                sum += base + fix;
            }
            m[r * 4] = gain - sum;
        }
        m
    }

    /// The set of five level matrices for each bank, whose rows sum to the gains.
    pub fn from_levels(
        bounds: [u16; 16],
        selectors: [u8; 20],
        control: [u16; 21],
        levels: &[[[i32; 9]; LEVELS]; BANKS],
    ) -> Result<Self> {
        let row_sum = |m: &[i32; 9], r: usize| m[r * 3] + m[r * 3 + 1] + m[r * 3 + 2];
        let mut gains = [0u16; LEVELS];
        for (level, gain) in gains.iter_mut().enumerate() {
            let sum = row_sum(&levels[0][level], 0);
            ensure!(
                (0..=2047).contains(&sum)
                    && levels
                        .iter()
                        .all(|bank| (0..3).all(|r| row_sum(&bank[level], r) == sum)),
                "Each row of level {level} must have the same sum"
            );
            *gain = sum as u16;
        }
        let off = |m: &[i32; 9], r: usize, j: usize| m[r * 3 + (r + 1 + j) % 3];
        let mut matrices = [[0i16; 6]; BANKS];
        for (bank, slot) in matrices.iter_mut().enumerate() {
            for r in 0..3 {
                for j in 0..2 {
                    slot[r * 2 + j] = i16::try_from(off(&levels[bank][0], r, j))
                        .context("A level-0 coefficient leaves the i16 range")?;
                }
            }
        }
        let mut look = Self {
            bounds,
            selectors,
            control,
            gains,
            matrices,
            corrections: [[[0; 6]; BANKS]; LEVELS - 1],
        };
        // Without corrections, `level_matrix` gives the rounded scale of level 0.
        let base = look.clone();
        for (l, corrections) in look.corrections.iter_mut().enumerate() {
            for (bank, (slot, source)) in corrections.iter_mut().zip(levels).enumerate() {
                let rounded = base.level_matrix(bank, l + 1);
                for (k, fix) in slot.iter_mut().enumerate() {
                    let (r, j) = (k / 2, k % 2);
                    *fix = i8::try_from(off(&source[l + 1], r, j) - off(&rounded, r, j))
                        .context("A level coefficient is too far from its level-0 value")?;
                }
            }
        }
        look.check()?;
        ensure!(
            (0..BANKS)
                .all(|bank| (0..LEVELS).all(|l| look.level_matrix(bank, l) == levels[bank][l])),
            "The level matrices do not round-trip"
        );
        Ok(look)
    }

    pub fn check(&self) -> Result<()> {
        ensure!(
            self.bounds
                .chunks_exact(4)
                .all(|q| q.windows(2).all(|p| p[0] >= p[1]) && q[0] <= 2048),
            "Expected four non-increasing hue bounds of 0..=2048 for each quadrant"
        );
        ensure!(
            self.selectors.iter().all(|s| usize::from(*s) < BANKS),
            "Expected bank selectors of 0..12"
        );
        ensure!(
            self.control.iter().enumerate().all(|(k, v)| {
                *v <= if SLOPE_FIELDS.contains(&k) {
                    0x1ff
                } else {
                    0xfff
                }
            }),
            "A MultiAxial control field leaves its register range"
        );
        ensure!(
            self.gains.iter().all(|g| *g <= 2047),
            "Expected level gains of 0..=2047"
        );
        ensure!(
            (0..BANKS).all(|bank| (0..LEVELS)
                .all(|level| self.level_matrix(bank, level).iter().all(|v| register(*v)))),
            "A level matrix leaves the 12-bit register range"
        );
        Ok(())
    }

    /// Writes the set into an effect file of `effects::SIZE_WITH_LEVELS` bytes.
    pub fn write(&self, blob: &mut [u8]) {
        let mut put = |at: usize, bytes: &[u8]| blob[at..at + bytes.len()].copy_from_slice(bytes);
        for (k, v) in self.bounds.iter().enumerate() {
            put(BOUNDS + k * 2, &v.to_le_bytes());
        }
        put(SELECTORS, &self.selectors);
        for (k, v) in self.control.iter().enumerate() {
            put(CONTROL + k * 2, &v.to_le_bytes());
        }
        for (k, v) in self.gains.iter().enumerate() {
            put(GAINS + k * 2, &v.to_le_bytes());
        }
        for (k, v) in self.matrices.iter().flatten().enumerate() {
            put(MATRICES + k * 2, &v.to_le_bytes());
        }
        let corrections: Vec<u8> = self
            .corrections
            .iter()
            .flatten()
            .flatten()
            .map(|v| *v as u8)
            .collect();
        put(CORRECTIONS, &corrections);
    }

    /// Reads the native look of an effect file.
    pub fn read(blob: &[u8]) -> Result<Self> {
        ensure!(blob.len() >= END, "Expected a native look");
        let u16_at = |at: usize| u16::from_le_bytes([blob[at], blob[at + 1]]);
        let look = Self {
            bounds: std::array::from_fn(|k| u16_at(BOUNDS + k * 2)),
            selectors: std::array::from_fn(|k| blob[SELECTORS + k]),
            control: std::array::from_fn(|k| u16_at(CONTROL + k * 2)),
            gains: std::array::from_fn(|k| u16_at(GAINS + k * 2)),
            matrices: std::array::from_fn(|b| {
                std::array::from_fn(|k| u16_at(MATRICES + (b * 6 + k) * 2) as i16)
            }),
            corrections: std::array::from_fn(|l| {
                std::array::from_fn(|b| {
                    std::array::from_fn(|k| blob[CORRECTIONS + (l * BANKS + b) * 6 + k] as i8)
                })
            }),
        };
        look.check()?;
        Ok(look)
    }

    pub fn to_json(&self) -> Value {
        json!({
            "bounds": self.bounds,
            "selectors": self.selectors,
            "control": self.control.to_vec(),
            "gains": self.gains,
            "matrices": self.matrices.to_vec(),
            "corrections": self.corrections.iter().map(|l| l.to_vec()).collect::<Vec<_>>(),
        })
    }

    pub fn from_json(value: &Value) -> Result<Self> {
        let message = "Expected native_look with 16 bounds, 20 selectors, 21 control fields, 5 gains, 12 x 6 matrices, and 4 x 12 x 6 corrections";
        let keys = [
            "bounds",
            "selectors",
            "control",
            "gains",
            "matrices",
            "corrections",
        ];
        let object = value.as_object().context(message)?;
        ensure!(
            object.len() == keys.len() && keys.iter().all(|k| object.contains_key(*k)),
            "{message}"
        );
        let ints = |value: &Value, n: usize, min: i64, max: i64| -> Result<Vec<i64>> {
            let items = value.as_array().filter(|a| a.len() == n).context(message)?;
            items
                .iter()
                .map(|v| {
                    v.as_i64()
                        .filter(|v| (min..=max).contains(v))
                        .context(message)
                })
                .collect()
        };
        let bounds = ints(&value["bounds"], 16, 0, 0xffff)?;
        let selectors = ints(&value["selectors"], 20, 0, 0xff)?;
        let control = ints(&value["control"], 21, 0, 0xffff)?;
        let gains = ints(&value["gains"], LEVELS, 0, 0xffff)?;
        let banks = value["matrices"]
            .as_array()
            .filter(|a| a.len() == BANKS)
            .context(message)?;
        let mut matrices = [[0i16; 6]; BANKS];
        for (slot, bank) in matrices.iter_mut().zip(banks) {
            for (v, n) in slot.iter_mut().zip(ints(bank, 6, -32768, 32767)?) {
                *v = n as i16;
            }
        }
        let levels = value["corrections"]
            .as_array()
            .filter(|a| a.len() == LEVELS - 1)
            .context(message)?;
        let mut corrections = [[[0i8; 6]; BANKS]; LEVELS - 1];
        for (slot, level) in corrections.iter_mut().zip(levels) {
            let banks = level
                .as_array()
                .filter(|a| a.len() == BANKS)
                .context(message)?;
            for (bank_slot, bank) in slot.iter_mut().zip(banks) {
                for (v, n) in bank_slot.iter_mut().zip(ints(bank, 6, -128, 127)?) {
                    *v = n as i8;
                }
            }
        }
        let look = Self {
            bounds: std::array::from_fn(|k| bounds[k] as u16),
            selectors: std::array::from_fn(|k| selectors[k] as u8),
            control: std::array::from_fn(|k| control[k] as u16),
            gains: std::array::from_fn(|k| gains[k] as u16),
            matrices,
            corrections,
        };
        look.check()?;
        Ok(look)
    }

    /// The bank (0..12) that the hardware picks at a joined hue coordinate (`hue::coordinate`).
    pub fn bank_at(&self, coordinate: f64) -> usize {
        // The hardware quadrant of each quarter of the joined coordinate.
        const QUADRANT_AT: [usize; 4] = [0, 3, 2, 1];
        let quarter = ((coordinate / 2048.0) as usize).min(3);
        let q = QUADRANT_AT[quarter];
        let x = (coordinate - 2048.0 * quarter as f64).floor();
        let n = self.bounds[q * 4..q * 4 + 4]
            .iter()
            .position(|b| x >= f64::from(*b))
            .unwrap_or(4);
        usize::from(self.selectors[5 * q + n])
    }

    /// The stage for previews: level 0 and the weight curve of control fields 10..20.
    pub fn stage(&self) -> Stage {
        let unit = |m: [i32; 9]| m.map(|v| f64::from(v) / 1024.0);
        let signed = |v: u16| {
            f64::from(if v & 0x100 != 0 {
                v as i32 - 0x200
            } else {
                v as i32
            })
        };
        Stage {
            look: self.clone(),
            matrices: std::array::from_fn(|bank| unit(self.level_matrix(bank, 0))),
            starts: std::array::from_fn(|k| f64::from(self.control[10 + k])),
            slopes: std::array::from_fn(|k| signed(self.control[14 + k])),
            breaks: [
                0.0,
                f64::from(self.control[18]),
                f64::from(self.control[19]),
                f64::from(self.control[20]),
            ],
        }
    }
}

/// A native look's MultiAxial stage for previews. It applies level 0: the levels of a
/// native look's control fields have no measured position on GR III.
#[derive(Clone, Debug, PartialEq)]
pub struct Stage {
    look: NativeLook,
    matrices: [[f64; 9]; BANKS],
    starts: [f64; 4],
    slopes: [f64; 4],
    breaks: [f64; 4],
}

impl Stage {
    fn weight(&self, y: f64) -> f64 {
        let x = (hue::WEIGHT_SCALE * y).clamp(0.0, 4095.0);
        let k = (0..4).rev().find(|&k| x >= self.breaks[k]).unwrap_or(0);
        ((self.starts[k] + self.slopes[k] * (x - self.breaks[k]) / 32.0) / 1024.0).clamp(0.0, 1.0)
    }

    /// The stage on a linear color after the color matrix.
    pub fn apply(&self, v: Rgb) -> Rgb {
        let Some(coordinate) = hue::coordinate(v) else {
            return v;
        };
        let m = &self.matrices[self.look.bank_at(coordinate)];
        let w = self.weight(hue::luminance(v));
        let mixed = cube::mul(&cube::rows(m), v);
        std::array::from_fn(|c| (v[c] + w * (mixed[c] - v[c])).max(0.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saturation_matrix_follows_the_gr4_rule() {
        assert_eq!(saturation_matrix(0), crate::fit::IDENTITY);
        let rgb = [1.0, 0.0, 1.402, 1.0, -0.344, -0.714, 1.0, 1.772, 0.0];
        let ycbcr = [77, 150, 29, -43, -85, 128, 128, -107, -21].map(|v: i32| f64::from(v) / 256.0);
        for value in [-4i8, -3, -2, -1, 1, 2, 3, 4] {
            let f = (1.1f64.powi(value.into()) * 1000.0).round() / 1000.0;
            let m = saturation_matrix(value);
            for (k, element) in m.iter().enumerate() {
                let (i, j) = (k / 3, k % 3);
                // The 8-bit inverse BT.601 matrix: 359, -88, -183, and 454 over 256.
                let quantized = |v: f64| (v * 256.0).round() / 256.0;
                let expected = 512.0
                    * (quantized(rgb[i * 3]) * ycbcr[j]
                        + f * (quantized(rgb[i * 3 + 1]) * ycbcr[3 + j]
                            + quantized(rgb[i * 3 + 2]) * ycbcr[6 + j]));
                assert_eq!(
                    element * 512.0,
                    expected.round(),
                    "value {value}, element {k}"
                );
            }
            // Each row keeps gray.
            for i in 0..3 {
                assert!((m[i * 3..i * 3 + 3].iter().sum::<f64>() - 1.0).abs() < 3.0 / 512.0);
            }
        }
        assert!(saturation_matrix(-2)[1] > 0.0 && saturation_matrix(2)[1] < 0.0);
    }

    /// A set whose levels scale level 0 by 16 - l over 16, with one rounding off by one.
    fn levels() -> [[[i32; 9]; LEVELS]; BANKS] {
        std::array::from_fn(|bank| {
            std::array::from_fn(|level| {
                let gain = 1024 * (16 - level as i32) / 16;
                let a = 40 + bank as i32 * 7;
                let b = -25 - bank as i32 * 3;
                let mut m = [0; 9];
                for r in 0..3 {
                    let off = [
                        a * gain / 1024 + i32::from(level == 2 && r == 1),
                        b * gain / 1024,
                    ];
                    m[r * 3 + (r + 1) % 3] = off[0];
                    m[r * 3 + (r + 2) % 3] = off[1];
                    m[r * 4] = gain - off[0] - off[1];
                }
                m
            })
        })
    }

    fn look() -> NativeLook {
        let control = [
            32, 328, 328, 328, 293, 800, 1600, 2400, 3200, 32, 1024, 1024, 1024, 1024, 0, 0, 0, 0,
            0, 640, 1280,
        ];
        NativeLook::from_levels(
            [
                1298, 750, 0, 0, 1448, 843, 0, 0, 1298, 750, 0, 0, 1205, 843, 0, 0,
            ],
            [
                0, 1, 2, 2, 2, 3, 4, 5, 5, 5, 6, 7, 8, 8, 8, 9, 10, 11, 11, 11,
            ],
            control,
            &levels(),
        )
        .unwrap()
    }

    #[test]
    fn levels_round_trip_through_corrections() {
        let look = look();
        assert_eq!(look.gains, [1024, 960, 896, 832, 768]);
        for (bank, source) in levels().iter().enumerate() {
            for (level, matrix) in source.iter().enumerate() {
                assert_eq!(look.level_matrix(bank, level), *matrix);
            }
        }
        assert!(look.corrections.iter().flatten().flatten().any(|v| *v != 0));
    }

    #[test]
    fn bytes_and_json_round_trip() {
        let look = look();
        let mut blob = vec![0; crate::effects::SIZE_WITH_LEVELS];
        look.write(&mut blob);
        assert!(blob[HEAD_END..SIZE].iter().all(|v| *v == 0));
        assert!(blob[END..].iter().all(|v| *v == 0));
        assert_eq!(NativeLook::read(&blob).unwrap(), look);
        assert_eq!(NativeLook::from_json(&look.to_json()).unwrap(), look);
    }

    #[test]
    fn checks_ranges() {
        let mut bad = look();
        bad.selectors[3] = 12;
        assert!(bad.check().is_err());
        let mut bad = look();
        bad.bounds[1] = bad.bounds[0] + 1;
        assert!(bad.check().is_err());
        let mut bad = look();
        bad.control[15] = 0x200;
        assert!(bad.check().is_err());
        let mut bad = look();
        bad.matrices[0][0] = 3000;
        assert!(bad.check().is_err());
        // Rows of one level with different sums.
        let mut source = levels();
        source[3][1][0] += 1;
        assert!(
            NativeLook::from_levels(look().bounds, look().selectors, look().control, &source)
                .is_err()
        );
    }

    #[test]
    fn stage_uses_the_selected_bank() {
        let look = look();
        let stage = look.stage();
        // Gray stays.
        assert_eq!(stage.apply([0.2, 0.2, 0.2]), [0.2, 0.2, 0.2]);
        // A red color: +Cr, the first quadrant at a small coordinate, segment 4 (bank 2).
        let red = [0.4, 0.1, 0.12];
        let c = hue::coordinate(red).unwrap();
        let bank = look.bank_at(c);
        let m = look.level_matrix(bank, 0).map(|v| f64::from(v) / 1024.0);
        let expected = cube::mul(&cube::rows(&m), red);
        let got = stage.apply(red);
        for k in 0..3 {
            assert!((got[k] - expected[k].max(0.0)).abs() < 1e-12);
        }
    }
}
