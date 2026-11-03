//! The camera around an effect: the Standard base gamma, the look and post matrices, and the
//! chroma suppression of each picture.

use crate::cube::{self, rows, Encoding, Matrix, Rgb};
use crate::hue;

/// Assumption, not a measurement: 18% gray lands at base-gamma index 7.8 (Standard sRGB ~0.46).
pub const MID_INDEX: f64 = 7.8;
pub const K: f64 = MID_INDEX / 0.18;
/// The GR III Standard look matrix: the `standard` input encoding and the Standard comparison.
pub const STANDARD_LOOK: [i16; 9] = [8116, 743, -667, 15, 8598, -421, -244, 542, 7894];
pub const IDENTITY: [f64; 9] = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];

/// X-Rite ColorChecker Classic, CIE L*a*b* D50 reference values for charts made after
/// November 2014, in patch order (dark skin to black).
const COLORCHECKER_LAB: [[f64; 3]; 24] = [
    [37.54, 14.37, 14.92],
    [64.66, 19.27, 17.50],
    [49.32, -3.82, -22.54],
    [43.46, -12.74, 22.72],
    [54.94, 9.61, -24.79],
    [70.48, -32.26, -0.37],
    [62.73, 35.83, 56.50],
    [39.43, 10.75, -45.17],
    [50.57, 48.64, 16.67],
    [30.10, 22.54, -20.87],
    [71.77, -24.13, 58.19],
    [71.51, 18.24, 67.37],
    [28.37, 15.42, -49.80],
    [54.38, -39.72, 32.27],
    [42.43, 51.05, 28.62],
    [81.80, 2.67, 80.41],
    [50.63, 51.28, -14.12],
    [49.57, -29.71, -28.32],
    [95.19, -1.03, 2.93],
    [81.29, -0.57, 0.44],
    [66.89, -0.75, -0.06],
    [50.76, -0.13, 0.14],
    [35.63, -0.46, -0.48],
    [20.64, 0.07, -0.46],
];
/// BT.601 RGB to YCbCr in 1/256 units: the YCbCr (CSP) matrix that the runtime multiplies by `P`.
pub const BT601: [[i32; 3]; 3] = [[77, 150, 29], [-43, -85, 128], [128, -107, -21]];
/// The CSP registers hold -256..255. The limit leaves room for the hue rotation.
pub const CSP_LIMIT: i32 = 240;
/// The camera's chroma suppression (CSY) after the YCbCr matrix: Cb and Cr times a gain that
/// the luminance of the pixel selects, four straight lines of the 10-bit luminance from these
/// borders, where 512 is the gain 1. GR III 2.10 and GR IV 1.11 write these values for each
/// picture, with and without an effect. The gain falls to 0 toward black and from luminance
/// 768 toward white.
const CSY_OFFSET: [f64; 4] = [0.0, 512.0, 512.0, 432.0];
const CSY_SLOPE: [f64; 4] = [73.0, 0.0, -7.0, -14.0];
const CSY_BORDER: [f64; 4] = [0.0, 56.0, 768.0, 856.0];
/// Rec. 709 luma weights, the start of a monochrome matrix fit.
pub(super) const LUMA: [f64; 3] = [0.2126, 0.7152, 0.0722];

/// Scene-linear sRGB of the ColorChecker 24 patches (Bradford D50 to D65), clipped at 0.
pub fn colorchecker() -> [Rgb; 24] {
    COLORCHECKER_LAB.map(|lab| cube::lab_d50_to_srgb(lab).map(|v| v.max(0.0)))
}

/// Linear interpolation over 256 entries with `np.interp` clipping.
fn lookup(table: impl Fn(usize) -> f64, x: f64) -> f64 {
    let x = x.clamp(0.0, 255.0);
    let low = (x as usize).min(254);
    let t = x - low as f64;
    table(low) * (1.0 - t) + table(low + 1) * t
}

/// Unquantized effect parameters: matrix and post matrix row-major, curves 0..16383.
#[derive(Clone, Debug, PartialEq)]
pub struct Params {
    pub matrix: [f64; 9],
    pub curves: [[f64; 256]; 3],
    pub post: [f64; 9],
    /// The MultiAxial stage after the matrix, if the effect has hue sectors.
    pub sectors: Option<hue::Stage>,
    /// The MultiAxial stage of a native look (GRF feature 32), which excludes `sectors`.
    pub native: Option<crate::native_look::Stage>,
}

/// The camera around an effect: the Standard base gamma of the user's firmware.
#[derive(Clone, Debug)]
pub struct Model {
    base: [f64; 256],
}

impl Model {
    pub fn new(base_gamma: &[u16; 256]) -> Self {
        Self {
            base: base_gamma.map(f64::from),
        }
    }

    pub fn base_gamma(&self, linear: f64) -> f64 {
        lookup(|i| self.base[i], K * linear)
    }

    /// The scene-linear value whose base gamma is `level`, within the table's range.
    fn base_gamma_inverse(&self, level: f64) -> f64 {
        let level = level.clamp(self.base[0], self.base[255]);
        let low = (0..255)
            .find(|&i| self.base[i + 1] >= level && self.base[i + 1] > self.base[i])
            .unwrap_or(254);
        let (a, b) = (self.base[low], self.base[low + 1]);
        let t = if b > a { (level - a) / (b - a) } else { 0.0 };
        (low as f64 + t.clamp(0.0, 1.0)) / K
    }

    /// The display RGB of the camera's Standard look, without an effect, with the chroma
    /// suppression that each picture of the camera has.
    pub fn standard(&self, linear: Rgb) -> Rgb {
        display(
            cube::mul(&rows(&STANDARD_LOOK.map(|v| v as f64 / 8192.0)), linear)
                .map(|v| self.base_gamma(v) / 16384.0),
        )
    }

    /// Scene-linear sRGB-primaries RGB to the input of a LUT in `encoding`.
    pub fn encode(&self, encoding: Encoding, linear: Rgb) -> Rgb {
        let log =
            |to: Matrix, curve: fn(f64) -> f64| cube::mul(&to, linear).map(|v| curve(v.max(0.0)));
        match encoding {
            Encoding::Standard => self.standard(linear),
            Encoding::Srgb => linear.map(|v| cube::srgb_encode(v.clamp(0.0, 1.0))),
            Encoding::Linear => linear,
            Encoding::Vlog => log(cube::srgb_to_vgamut(), cube::vlog_encode),
            Encoding::Flog => log(cube::srgb_to_fgamut(), cube::flog_encode),
            Encoding::Flog2 => log(cube::srgb_to_fgamut(), cube::flog2_encode),
            Encoding::Flog2C => log(cube::srgb_to_fgamut_c(), cube::flog2_encode),
        }
    }

    /// The inverse of `encode` within its unclipped range.
    pub fn decode(&self, encoding: Encoding, encoded: Rgb) -> Rgb {
        let log = |from: Matrix, curve: fn(f64) -> f64| cube::mul(&from, encoded.map(curve));
        match encoding {
            Encoding::Standard => {
                let look = cube::inverse(&rows(&STANDARD_LOOK.map(|v| v as f64 / 8192.0))).unwrap();
                let codes = undisplay(encoded).map(|v| v.clamp(0.0, 1.0));
                cube::mul(&look, codes.map(|v| self.base_gamma_inverse(v * 16384.0)))
            }
            Encoding::Srgb => encoded.map(cube::srgb_decode),
            Encoding::Linear => encoded,
            Encoding::Vlog => log(cube::vgamut_to_srgb(), cube::vlog_decode),
            Encoding::Flog => log(cube::fgamut_to_srgb(), cube::flog_decode),
            Encoding::Flog2 => log(cube::fgamut_to_srgb(), cube::flog2_decode),
            Encoding::Flog2C => log(cube::fgamut_c_to_srgb(), cube::flog2_decode),
        }
    }

    /// Gamma-encoded RGB after the curves and before the post matrix.
    pub fn codes(&self, params: &Params, linear: Rgb) -> Rgb {
        let mixed = self.mixed(params, linear);
        std::array::from_fn(|c| {
            lookup(|i| params.curves[c][i], self.base_gamma(mixed[c]) / 64.0) / 16384.0
        })
    }

    /// Linear RGB after the matrix and the hue sectors, the input of the base gamma.
    pub fn mixed(&self, params: &Params, linear: Rgb) -> Rgb {
        let mixed = cube::mul(&rows(&params.matrix), linear);
        match (&params.sectors, &params.native) {
            (Some(stage), _) => stage.apply(mixed),
            (None, Some(stage)) => stage.apply(mixed),
            (None, None) => mixed,
        }
    }

    /// The display RGB that the camera renders for scene-linear `linear`.
    pub fn render(&self, params: &Params, linear: Rgb) -> Rgb {
        display(cube::mul(&rows(&params.post), self.codes(params, linear)))
    }
}

/// The CSY gain (1.0 = no change) at a luminance of 0..1.
pub fn chroma_gain(luminance: f64) -> f64 {
    let y = (luminance * 1023.0).clamp(0.0, 1023.0);
    let area = (1..4).rev().find(|&a| y >= CSY_BORDER[a]).unwrap_or(0);
    (CSY_OFFSET[area] + CSY_SLOPE[area] * (y - CSY_BORDER[area]) / 8.0).clamp(0.0, 1023.0) / 512.0
}

/// The camera's output for RGB codes after the post matrix: the chroma suppression, then the
/// clip of each color. The runtime puts the post matrix into the YCbCr matrix, so the
/// luminance of CSY is the BT.601 luminance of these codes.
pub fn display(codes: Rgb) -> Rgb {
    let ycc = cube::mul(&bt601(), codes);
    let gain = chroma_gain(ycc[0]);
    let out = if gain == 1.0 {
        codes
    } else {
        cube::mul(&rgb_from_bt601(), [ycc[0], ycc[1] * gain, ycc[2] * gain])
    };
    out.map(|v| v.clamp(0.0, 1.0))
}

/// The smallest CSY gain that `undisplay` divides by, so that a target near black or white
/// does not ask the curves for an unbounded chroma.
const CSY_FLOOR: f64 = 0.25;

/// The codes before the chroma suppression for a display color, the inverse of `display`
/// within its unclipped range and with the gain at least `CSY_FLOOR`.
pub fn undisplay(display: Rgb) -> Rgb {
    let ycc = cube::mul(&bt601(), display);
    let gain = chroma_gain(ycc[0]).max(CSY_FLOOR);
    if gain == 1.0 {
        return display;
    }
    cube::mul(&rgb_from_bt601(), [ycc[0], ycc[1] / gain, ycc[2] / gain])
}

pub(super) fn bt601() -> Matrix {
    BT601.map(|r| r.map(|v| v as f64 / 256.0))
}

fn rgb_from_bt601() -> Matrix {
    static INVERSE: std::sync::OnceLock<Matrix> = std::sync::OnceLock::new();
    *INVERSE.get_or_init(|| cube::inverse(&bt601()).unwrap())
}
