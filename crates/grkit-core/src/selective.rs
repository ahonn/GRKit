//! Selective color: keep the chroma of one hue range and remove it elsewhere.
//!
//! The runtime applies it to the CbCr planes of a recorded picture, after the effect's own
//! rendering. This module holds the integer parameters that a GRF stores, their conversion
//! from degrees, and the same weight table and pair arithmetic as `runtime/picture.c`, so a
//! site preview matches the camera bit for bit.
//!
//! The hue is the angle of (Cb - 128, Cr - 128). Each side of the target hue has its own
//! inner and outer half widths: inside the inner one a pair keeps all of its chroma, outside
//! the outer one none, and between them the weight falls linearly in tan(angle offset). The
//! "higher" side is toward larger angles (from red, toward orange and skin). A chroma gate
//! fades out pairs whose chroma along the target is low.

use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};

/// Bytes of the GRF extension that hold the parameters, from offset 106.
pub const SIZE: usize = 14;
/// Weight table cells per axis: one cell for each 2 x 2 block of (Cb, Cr) values.
pub const TABLE_SIDE: usize = 128;
/// The largest projection of a CbCr vector: 128 * sqrt(2).
const ALONG: usize = 182;
/// Q12 bounds on |direction|^2: about 0.95 to 1.03 of 4096^2.
const DIRECTION_SQUARED: std::ops::RangeInclusive<i64> = 15_000_000..=18_000_000;
/// Q8 tangent bound, about 76 degrees.
const MAX_TAN: u16 = 1024;
/// The largest chroma along the target.
const MAX_CHROMA: u8 = 181;
const MAX_WIDTH_DEGREES: f64 = 75.0;

/// Kernel parameters, as the GRF and the runtime record store them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Selective {
    /// Q12 cosine and sine of the target hue.
    pub direction: [i16; 2],
    /// Q8 tangents of the inner and outer half widths toward smaller angles.
    pub lower: [u16; 2],
    /// Q8 tangents of the inner and outer half widths toward larger angles.
    pub higher: [u16; 2],
    /// No chroma kept at or below `chroma[0]`, all of it from `chroma[1]`.
    pub chroma: [u8; 2],
}

fn pair<T: TryFrom<i64>>(value: &Value, what: &str) -> Result<[T; 2]> {
    let values = value
        .as_array()
        .filter(|v| v.len() == 2)
        .with_context(|| format!("Expected two {what} values"))?;
    let item = |v: &Value| -> Result<T> {
        v.as_i64()
            .and_then(|n| T::try_from(n).ok())
            .with_context(|| format!("Expected two {what} values"))
    };
    Ok([item(&values[0])?, item(&values[1])?])
}

fn degrees(value: &Value, what: &str) -> Result<[f64; 2]> {
    let values = value
        .as_array()
        .filter(|v| v.len() == 2)
        .with_context(|| format!("Expected inner and outer {what} half widths in degrees"))?;
    let read = |v: &Value| v.as_f64().filter(|d| d.is_finite());
    match (read(&values[0]), read(&values[1])) {
        (Some(inner), Some(outer)) => Ok([inner, outer]),
        _ => anyhow::bail!("Expected inner and outer {what} half widths in degrees"),
    }
}

impl Selective {
    /// Checks the bounds that the runtime's GRF parser also checks.
    pub fn validate(&self) -> Result<()> {
        let [c, s] = self.direction.map(i64::from);
        ensure!(
            DIRECTION_SQUARED.contains(&(c * c + s * s)),
            "Expected a Q12 unit direction for selective color"
        );
        for side in [self.lower, self.higher] {
            ensure!(
                side[0] < side[1] && side[1] <= MAX_TAN,
                "Expected selective color tangents with inner < outer <= {MAX_TAN}"
            );
        }
        ensure!(
            self.chroma[0] < self.chroma[1] && self.chroma[1] <= MAX_CHROMA,
            "Expected a selective color chroma gate with c0 < c1 <= {MAX_CHROMA}"
        );
        Ok(())
    }

    /// From an effect definition: `{"hue": degrees, "lower": [inner, outer],
    /// "higher": [inner, outer], "chroma": [c0, c1]}`.
    pub fn from_definition(value: &Value) -> Result<Self> {
        let object = value
            .as_object()
            .filter(|o| {
                o.len() == 4
                    && ["hue", "lower", "higher", "chroma"]
                        .iter()
                        .all(|k| o.contains_key(*k))
            })
            .context("Expected selective_color with hue, lower, higher, and chroma")?;
        let hue = object["hue"]
            .as_f64()
            .filter(|h| h.is_finite() && (0.0..360.0).contains(h))
            .context("Expected a selective color hue from 0 to 360 degrees")?;
        let side = |key: &str| -> Result<[u16; 2]> {
            let [inner, outer] = degrees(&object[key], key)?;
            ensure!(
                0.0 <= inner && inner < outer && outer <= MAX_WIDTH_DEGREES,
                "Expected 0 <= inner < outer <= {MAX_WIDTH_DEGREES} degrees for {key}"
            );
            let tan = |d: f64| (d.to_radians().tan() * 256.0).round() as u16;
            Ok([tan(inner), tan(outer)])
        };
        let radians = hue.to_radians();
        let result = Self {
            direction: [
                (radians.cos() * 4096.0).round() as i16,
                (radians.sin() * 4096.0).round() as i16,
            ],
            lower: side("lower")?,
            higher: side("higher")?,
            chroma: pair::<u8>(&object["chroma"], "chroma gate")?,
        };
        result.validate()?;
        Ok(result)
    }

    /// The decoded form: the integers that the GRF holds.
    pub fn to_json(&self) -> Value {
        json!({"direction_q12": self.direction, "lower_tan_q8": self.lower,
               "higher_tan_q8": self.higher, "chroma": self.chroma})
    }

    pub fn from_json(value: &Value) -> Result<Self> {
        let keys = ["direction_q12", "lower_tan_q8", "higher_tan_q8", "chroma"];
        ensure!(
            value
                .as_object()
                .is_some_and(|o| o.len() == 4 && keys.iter().all(|k| o.contains_key(*k))),
            "Expected selective_color with direction_q12, lower_tan_q8, higher_tan_q8, and chroma"
        );
        let result = Self {
            direction: pair::<i16>(&value["direction_q12"], "direction")?,
            lower: pair::<u16>(&value["lower_tan_q8"], "lower tangent")?,
            higher: pair::<u16>(&value["higher_tan_q8"], "higher tangent")?,
            chroma: pair::<u8>(&value["chroma"], "chroma gate")?,
        };
        result.validate()?;
        Ok(result)
    }

    /// Little-endian GRF bytes: i16 cos, i16 sin, u16 lower inner and outer, u16 higher
    /// inner and outer, u8 c0, u8 c1.
    pub fn to_bytes(&self) -> [u8; SIZE] {
        let mut bytes = [0; SIZE];
        let words = [
            self.direction[0] as u16,
            self.direction[1] as u16,
            self.lower[0],
            self.lower[1],
            self.higher[0],
            self.higher[1],
        ];
        for (index, word) in words.iter().enumerate() {
            bytes[index * 2..index * 2 + 2].copy_from_slice(&word.to_le_bytes());
        }
        bytes[12] = self.chroma[0];
        bytes[13] = self.chroma[1];
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        ensure!(bytes.len() == SIZE, "Expected {SIZE} selective color bytes");
        let word = |i: usize| u16::from_le_bytes([bytes[i * 2], bytes[i * 2 + 1]]);
        let result = Self {
            direction: [word(0) as i16, word(1) as i16],
            lower: [word(2), word(3)],
            higher: [word(4), word(5)],
            chroma: [bytes[12], bytes[13]],
        };
        result.validate()?;
        Ok(result)
    }

    /// The keep weight of one pair, 0..=256, exactly as the runtime computes it.
    fn weight(&self, factors: &Factors, u: i32, v: i32) -> u32 {
        let [hc, hs] = self.direction.map(i32::from);
        let along = (u * hc + v * hs) >> 12;
        let across = (v * hc - u * hs) >> 12;
        if along <= 0 {
            return 0;
        }
        let a = (along as u32).min(ALONG as u32 - 1);
        let side = usize::from(across > 0);
        let b = across.unsigned_abs();
        let [t0, t1] = if side == 1 { self.higher } else { self.lower }.map(u32::from);
        let (limit, full) = (t1 * a, t0 * a);
        let [c0, c1] = self.chroma.map(u32::from);
        if b << 8 >= limit || a <= c0 {
            return 0;
        }
        let mut w = if b << 8 <= full {
            256
        } else {
            ((limit - (b << 8)) * factors.angle[side][a as usize]) >> 16
        };
        if a < c1 {
            w = (w * (((a - c0) * factors.gate) >> 16)) >> 8;
        }
        w
    }

    /// The weight of each 2 x 2 block of (Cb, Cr) values at its center, `TABLE_SIDE` cells
    /// per axis, Cr rows of Cb columns.
    pub fn table(&self) -> Vec<u8> {
        let factors = Factors::new(self);
        let mut table = Vec::with_capacity(TABLE_SIDE * TABLE_SIDE);
        for j in 0..TABLE_SIDE as i32 {
            for i in 0..TABLE_SIDE as i32 {
                table.push(
                    self.weight(&factors, 2 * i + 1 - 128, 2 * j + 1 - 128)
                        .min(255) as u8,
                );
            }
        }
        table
    }
}

/// Q16 reciprocals that the runtime builds once per picture. Its shift-subtract division
/// gives the same floor quotient as `/`.
struct Factors {
    angle: [[u32; ALONG]; 2],
    gate: u32,
}

impl Factors {
    fn new(s: &Selective) -> Self {
        let mut angle = [[0; ALONG]; 2];
        for (row, [t0, t1]) in angle.iter_mut().zip([s.lower, s.higher]) {
            for (a, slot) in row.iter_mut().enumerate().skip(1) {
                *slot = (256 << 16) / ((u32::from(t1) - u32::from(t0)) * a as u32);
            }
        }
        let gate = (256 << 16) / u32::from(s.chroma[1] - s.chroma[0]);
        Self { angle, gate }
    }
}

/// One pair through the table: the new (Cb, Cr).
pub fn apply(table: &[u8], cb: u8, cr: u8) -> (u8, u8) {
    let mut w = u32::from(table[usize::from(cr >> 1) * TABLE_SIDE + usize::from(cb >> 1)]);
    w += u32::from(w == 255);
    let scale = |c: u8| (128 + (((i32::from(c) - 128) * w as i32 + 128) >> 8)) as u8;
    (scale(cb), scale(cr))
}

/// A display RGB color (0..1) after selective color, as the camera's JPEG planes carry it:
/// BT.601 YCbCr with the YCbCr matrix coefficients (`fit::BT601`, 256 = 1.0), the table,
/// then the JFIF inverse.
pub fn apply_rgb(table: &[u8], rgb: [f64; 3]) -> [f64; 3] {
    let [r, g, b] = rgb.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as i32);
    let row = |m: [i32; 3]| m[0] * r + m[1] * g + m[2] * b;
    let [y, u, v] = crate::fit::BT601.map(row);
    let y = ((y + 128) >> 8).clamp(0, 255) as f64;
    let code = |c: i32| (128 + ((c + 128) >> 8)).clamp(0, 255) as u8;
    let (cb, cr) = apply(table, code(u), code(v));
    let (u, v) = (f64::from(cb) - 128.0, f64::from(cr) - 128.0);
    [
        y + 1.402 * v,
        y - 0.344136 * u - 0.714136 * v,
        y + 1.772 * u,
    ]
    .map(|c| (c / 255.0).clamp(0.0, 1.0))
}
