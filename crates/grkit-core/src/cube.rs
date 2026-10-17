//! Adobe / Resolve `.cube` LUTs and the color transforms that `.cube` import needs.
//!
//! RGB triples are display or scene values in sRGB primaries unless a name says otherwise.

use anyhow::{bail, ensure, Context, Result};
use std::str::FromStr;

pub type Matrix = [[f64; 3]; 3];
pub type Rgb = [f64; 3];

pub const MAX_3D_SIZE: usize = 256;
pub const MAX_1D_SIZE: usize = 65536;

/// A parsed LUT. `table` holds 3D entries with red changing fastest, or 1D rows.
#[derive(Clone, Debug)]
pub struct Cube {
    pub title: Option<String>,
    pub three_d: bool,
    pub size: usize,
    pub domain_min: Rgb,
    pub domain_max: Rgb,
    pub table: Vec<[f32; 3]>,
}

fn numbers<const N: usize>(fields: &[&str], line: usize, keyword: &str) -> Result<[f64; N]> {
    ensure!(
        fields.len() == N,
        "Line {line}: {keyword} expects {N} values"
    );
    let mut result = [0.0; N];
    for (slot, field) in result.iter_mut().zip(fields) {
        *slot = field
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite())
            .with_context(|| format!("Line {line}: expected a finite number, found {field:?}"))?;
    }
    Ok(result)
}

impl Cube {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let text = std::str::from_utf8(bytes).context("Expected a UTF-8 .cube file")?;
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        let mut title = None;
        let (mut size_3d, mut size_1d) = (None, None);
        let (mut domain_min, mut domain_max, mut range) = (None, None, None);
        let mut table = Vec::new();
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let raw = raw.trim();
            // A TITLE keeps any '#' inside its quotes; other lines may end with a comment.
            let content = if raw.starts_with("TITLE") {
                raw
            } else {
                raw.split('#').next().unwrap().trim()
            };
            if content.is_empty() {
                continue;
            }
            let fields: Vec<&str> = content.split_whitespace().collect();
            let keyword = fields[0];
            if keyword.parse::<f64>().is_ok() {
                let row = numbers::<3>(&fields, line, "A data row")?;
                table.push(row.map(|v| v as f32));
                continue;
            }
            ensure!(
                table.is_empty(),
                "Line {line}: keyword {keyword} after data rows"
            );
            let duplicate = |set: bool| -> Result<()> {
                ensure!(!set, "Line {line}: duplicate {keyword}");
                Ok(())
            };
            match keyword {
                "TITLE" => {
                    duplicate(title.is_some())?;
                    let value = content["TITLE".len()..].trim();
                    let value = value
                        .strip_prefix('"')
                        .and_then(|v| v.strip_suffix('"'))
                        .with_context(|| format!("Line {line}: expected a quoted TITLE"))?;
                    title = Some(value.to_owned());
                }
                "LUT_3D_SIZE" | "LUT_1D_SIZE" => {
                    let three_d = keyword == "LUT_3D_SIZE";
                    let slot = if three_d { &mut size_3d } else { &mut size_1d };
                    duplicate(slot.is_some())?;
                    let limit = if three_d { MAX_3D_SIZE } else { MAX_1D_SIZE };
                    let size = (fields.len() == 2)
                        .then(|| fields[1].parse::<usize>().ok())
                        .flatten()
                        .filter(|n| (2..=limit).contains(n))
                        .with_context(|| format!("Line {line}: {keyword} must be 2 to {limit}"))?;
                    *slot = Some(size);
                }
                "DOMAIN_MIN" | "DOMAIN_MAX" => {
                    let slot = if keyword == "DOMAIN_MIN" {
                        &mut domain_min
                    } else {
                        &mut domain_max
                    };
                    duplicate(slot.is_some())?;
                    *slot = Some(numbers::<3>(&fields[1..], line, keyword)?);
                }
                "LUT_3D_INPUT_RANGE" | "LUT_1D_INPUT_RANGE" => {
                    duplicate(range.is_some())?;
                    range = Some((keyword, numbers::<2>(&fields[1..], line, keyword)?));
                }
                _ => bail!("Line {line}: unsupported keyword {keyword}"),
            }
        }
        let (three_d, size) = match (size_3d, size_1d) {
            (Some(n), None) => (true, n),
            (None, Some(n)) => (false, n),
            (Some(_), Some(_)) => bail!("A .cube file with both 1D and 3D tables is not supported"),
            (None, None) => bail!("Expected LUT_3D_SIZE or LUT_1D_SIZE"),
        };
        if let Some((keyword, _)) = range {
            ensure!(
                keyword
                    == if three_d {
                        "LUT_3D_INPUT_RANGE"
                    } else {
                        "LUT_1D_INPUT_RANGE"
                    },
                "{keyword} does not match the table dimension"
            );
            ensure!(
                domain_min.is_none() && domain_max.is_none(),
                "Use either DOMAIN_MIN / DOMAIN_MAX or {keyword}"
            );
        }
        let (domain_min, domain_max) = match range {
            Some((_, [low, high])) => ([low; 3], [high; 3]),
            None => (
                domain_min.unwrap_or([0.0; 3]),
                domain_max.unwrap_or([1.0; 3]),
            ),
        };
        ensure!(
            (0..3).all(|c| domain_min[c] < domain_max[c]),
            "Expected each domain minimum below its maximum"
        );
        let expected = if three_d { size.pow(3) } else { size };
        ensure!(
            table.len() == expected,
            "Expected {expected} data rows, found {}",
            table.len()
        );
        ensure!(
            table.iter().flatten().all(|v| v.is_finite()),
            "Data values exceed the supported range"
        );
        Ok(Self {
            title,
            three_d,
            size,
            domain_min,
            domain_max,
            table,
        })
    }

    /// Applies the LUT with domain clamping: trilinear for 3D, per channel for 1D.
    pub fn apply(&self, rgb: Rgb) -> Rgb {
        let last = (self.size - 1) as f64;
        let position = |c: usize| {
            let unit = (rgb[c] - self.domain_min[c]) / (self.domain_max[c] - self.domain_min[c]);
            let x = unit.clamp(0.0, 1.0) * last;
            let low = (x as usize).min(self.size - 2);
            (low, x - low as f64)
        };
        if !self.three_d {
            return std::array::from_fn(|c| {
                let (low, t) = position(c);
                let (a, b) = (self.table[low][c] as f64, self.table[low + 1][c] as f64);
                a + (b - a) * t
            });
        }
        let (r, g, b) = (position(0), position(1), position(2));
        let n = self.size;
        let mut result = [0.0; 3];
        for corner in 0..8 {
            let (dr, dg, db) = (corner & 1, (corner >> 1) & 1, corner >> 2);
            let weight = [(r, dr), (g, dg), (b, db)]
                .iter()
                .map(|((_, t), d)| if *d == 1 { *t } else { 1.0 - t })
                .product::<f64>();
            if weight == 0.0 {
                continue;
            }
            let entry = self.table[(r.0 + dr) + (g.0 + dg) * n + (b.0 + db) * n * n];
            for (slot, value) in result.iter_mut().zip(entry) {
                *slot += weight * value as f64;
            }
        }
        result
    }
}

/// The declared encoding of a LUT's input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Encoding {
    /// sRGB-encoded display RGB of the scene, clipped to 0..1.
    Srgb,
    /// Scene-linear sRGB-primaries RGB.
    Linear,
    /// Panasonic V-Log over V-Gamut primaries.
    Vlog,
}

impl FromStr for Encoding {
    type Err = anyhow::Error;
    fn from_str(name: &str) -> Result<Self> {
        Ok(match name {
            "srgb" => Self::Srgb,
            "linear" => Self::Linear,
            "vlog" => Self::Vlog,
            _ => bail!("Expected input encoding srgb, linear, or vlog"),
        })
    }
}

impl Encoding {
    pub fn name(self) -> &'static str {
        match self {
            Self::Srgb => "srgb",
            Self::Linear => "linear",
            Self::Vlog => "vlog",
        }
    }

    /// Scene-linear sRGB-primaries RGB to LUT input, as gr3-film-mods `cube_fit.load_target`.
    pub fn encode(self, linear: Rgb) -> Rgb {
        match self {
            Self::Srgb => linear.map(|v| srgb_encode(v.clamp(0.0, 1.0))),
            Self::Linear => linear,
            Self::Vlog => mul(&srgb_to_vgamut(), linear).map(|v| vlog_encode(v.max(0.0))),
        }
    }

    /// The inverse of `encode` within its unclipped range.
    pub fn decode(self, encoded: Rgb) -> Rgb {
        match self {
            Self::Srgb => encoded.map(srgb_decode),
            Self::Linear => encoded,
            Self::Vlog => mul(&vgamut_to_srgb(), encoded.map(vlog_decode)),
        }
    }
}

pub fn srgb_encode(v: f64) -> f64 {
    if v <= 0.0031308 {
        12.92 * v
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    }
}

pub fn srgb_decode(v: f64) -> f64 {
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

// Panasonic V-Log / V-Gamut Reference Manual (rev. 1.0, 2014): reflection-domain curve,
// full-range normalized code values.
const VLOG_CUT1: f64 = 0.01;
const VLOG_CUT2: f64 = 0.181;
const VLOG_B: f64 = 0.00873;
const VLOG_C: f64 = 0.241514;
const VLOG_D: f64 = 0.598206;

pub fn vlog_encode(v: f64) -> f64 {
    if v < VLOG_CUT1 {
        5.6 * v + 0.125
    } else {
        VLOG_C * (v + VLOG_B).log10() + VLOG_D
    }
}

pub fn vlog_decode(v: f64) -> f64 {
    if v < VLOG_CUT2 {
        (v - 0.125) / 5.6
    } else {
        10f64.powf((v - VLOG_D) / VLOG_C) - VLOG_B
    }
}

const D65: [f64; 2] = [0.3127, 0.3290];
const D50: [f64; 2] = [0.3457, 0.3585];
const SRGB_PRIMARIES: [[f64; 2]; 3] = [[0.64, 0.33], [0.30, 0.60], [0.15, 0.06]];
const VGAMUT_PRIMARIES: [[f64; 2]; 3] = [[0.730, 0.280], [0.165, 0.840], [0.100, -0.030]];

pub fn mul(m: &Matrix, v: Rgb) -> Rgb {
    std::array::from_fn(|r| m[r][0] * v[0] + m[r][1] * v[1] + m[r][2] * v[2])
}

pub fn product(a: &Matrix, b: &Matrix) -> Matrix {
    std::array::from_fn(|r| std::array::from_fn(|c| (0..3).map(|k| a[r][k] * b[k][c]).sum()))
}

pub fn inverse(m: &Matrix) -> Option<Matrix> {
    let cofactor = |r: usize, c: usize| {
        let (r1, r2, c1, c2) = ((r + 1) % 3, (r + 2) % 3, (c + 1) % 3, (c + 2) % 3);
        m[r1][c1] * m[r2][c2] - m[r1][c2] * m[r2][c1]
    };
    let determinant: f64 = (0..3).map(|c| m[0][c] * cofactor(0, c)).sum();
    (determinant.abs() > 1e-12)
        .then(|| std::array::from_fn(|r| std::array::from_fn(|c| cofactor(c, r) / determinant)))
}

fn white(xy: [f64; 2]) -> Rgb {
    [xy[0] / xy[1], 1.0, (1.0 - xy[0] - xy[1]) / xy[1]]
}

/// RGB to XYZ for chromaticity primaries and a white point (SMPTE RP 177).
fn rgb_to_xyz(primaries: &[[f64; 2]; 3], whitepoint: [f64; 2]) -> Matrix {
    let columns: Matrix = std::array::from_fn(|r| std::array::from_fn(|c| white(primaries[c])[r]));
    let scale = mul(&inverse(&columns).unwrap(), white(whitepoint));
    std::array::from_fn(|r| std::array::from_fn(|c| columns[r][c] * scale[c]))
}

pub fn srgb_to_xyz() -> Matrix {
    rgb_to_xyz(&SRGB_PRIMARIES, D65)
}

/// Both spaces use D65, so the conversion needs no chromatic adaptation.
pub fn srgb_to_vgamut() -> Matrix {
    product(
        &inverse(&rgb_to_xyz(&VGAMUT_PRIMARIES, D65)).unwrap(),
        &srgb_to_xyz(),
    )
}

pub fn vgamut_to_srgb() -> Matrix {
    inverse(&srgb_to_vgamut()).unwrap()
}

/// CIE L*a*b* D50 to linear sRGB with Bradford adaptation to D65.
pub fn lab_d50_to_srgb(lab: [f64; 3]) -> Rgb {
    const BRADFORD: Matrix = [
        [0.8951, 0.2664, -0.1614],
        [-0.7502, 1.7135, 0.0367],
        [0.0389, -0.0685, 1.0296],
    ];
    let finv = |t: f64| {
        if t > 6.0 / 29.0 {
            t.powi(3)
        } else {
            3.0 * (6.0f64 / 29.0).powi(2) * (t - 4.0 / 29.0)
        }
    };
    let fy = (lab[0] + 16.0) / 116.0;
    let source = white(D50);
    let xyz = [
        source[0] * finv(fy + lab[1] / 500.0),
        finv(fy),
        source[2] * finv(fy - lab[2] / 200.0),
    ];
    let (from, to) = (mul(&BRADFORD, source), mul(&BRADFORD, white(D65)));
    let gain: Matrix = std::array::from_fn(|r| {
        std::array::from_fn(|c| if r == c { to[r] / from[r] } else { 0.0 })
    });
    let adapt = product(&inverse(&BRADFORD).unwrap(), &product(&gain, &BRADFORD));
    mul(&inverse(&srgb_to_xyz()).unwrap(), mul(&adapt, xyz))
}

/// CIE L*a*b* (D65) of sRGB-encoded display RGB, clipped to 0..1.
pub struct LabConverter(Matrix);

impl Default for LabConverter {
    fn default() -> Self {
        Self(srgb_to_xyz())
    }
}

impl LabConverter {
    pub fn lab(&self, display: Rgb) -> [f64; 3] {
        let xyz = mul(&self.0, display.map(|v| srgb_decode(v.clamp(0.0, 1.0))));
        let reference = white(D65);
        let f = |t: f64| {
            if t > (6.0f64 / 29.0).powi(3) {
                t.cbrt()
            } else {
                t / (3.0 * (6.0f64 / 29.0).powi(2)) + 4.0 / 29.0
            }
        };
        let [fx, fy, fz] = std::array::from_fn(|c| f(xyz[c] / reference[c]));
        [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
    }
}

/// CIEDE2000 with unit weights (Sharma, Wu, and Dalal 2005).
pub fn delta_e2000(a: [f64; 3], b: [f64; 3]) -> f64 {
    use std::f64::consts::PI;
    let c1 = a[1].hypot(a[2]);
    let c2 = b[1].hypot(b[2]);
    let mean_c7 = ((c1 + c2) / 2.0).powi(7);
    let g = 0.5 * (1.0 - (mean_c7 / (mean_c7 + 25f64.powi(7))).sqrt());
    let (a1, a2) = (a[1] * (1.0 + g), b[1] * (1.0 + g));
    let (c1, c2) = (a1.hypot(a[2]), a2.hypot(b[2]));
    let hue = |y: f64, x: f64| {
        if x == 0.0 && y == 0.0 {
            0.0
        } else {
            y.atan2(x).rem_euclid(2.0 * PI)
        }
    };
    let (h1, h2) = (hue(a[2], a1), hue(b[2], a2));
    let delta_l = b[0] - a[0];
    let delta_c = c2 - c1;
    let mut delta_h = h2 - h1;
    if c1 * c2 == 0.0 {
        delta_h = 0.0;
    } else if delta_h > PI {
        delta_h -= 2.0 * PI;
    } else if delta_h < -PI {
        delta_h += 2.0 * PI;
    }
    let delta_hh = 2.0 * (c1 * c2).sqrt() * (delta_h / 2.0).sin();
    let mean_l = (a[0] + b[0]) / 2.0;
    let mean_c = (c1 + c2) / 2.0;
    let mean_h = if c1 * c2 == 0.0 {
        h1 + h2
    } else if (h1 - h2).abs() <= PI {
        (h1 + h2) / 2.0
    } else if h1 + h2 < 2.0 * PI {
        (h1 + h2 + 2.0 * PI) / 2.0
    } else {
        (h1 + h2 - 2.0 * PI) / 2.0
    };
    let t = 1.0 - 0.17 * (mean_h - PI / 6.0).cos()
        + 0.24 * (2.0 * mean_h).cos()
        + 0.32 * (3.0 * mean_h + PI / 30.0).cos()
        - 0.20 * (4.0 * mean_h - 63f64.to_radians()).cos();
    let delta_theta = 30f64.to_radians() * (-((mean_h.to_degrees() - 275.0) / 25.0).powi(2)).exp();
    let mean_c7 = mean_c.powi(7);
    let rc = 2.0 * (mean_c7 / (mean_c7 + 25f64.powi(7))).sqrt();
    let sl = 1.0 + 0.015 * (mean_l - 50.0).powi(2) / (20.0 + (mean_l - 50.0).powi(2)).sqrt();
    let sc = 1.0 + 0.045 * mean_c;
    let sh = 1.0 + 0.015 * mean_c * t;
    let rt = -(2.0 * delta_theta).sin() * rc;
    ((delta_l / sl).powi(2)
        + (delta_c / sc).powi(2)
        + (delta_hh / sh).powi(2)
        + rt * (delta_c / sc) * (delta_hh / sh))
        .sqrt()
}
