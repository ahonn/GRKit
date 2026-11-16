//! Preset color transform on display-referred sRGB.
//!
//! The stages follow the order of the raw converter that the presets are made for, and their
//! curves and strengths are fitted to its renders of synthetic charts (`tone_data`, and the
//! constants below). The transform starts from a rendered picture, so it is an approximation:
//! the converter applies some stages to scene values before its own tone curve.
//!
//! 1. The profile: its `LookTable` in linear ProPhoto RGB, then its point curves, both scaled
//!    by the Look amount
//! 2. Basic tone: Highlights, Shadows, Whites, Blacks, then `Contrast2012`
//! 3. Parametric curve, then the point curves (master on the largest and smallest channel,
//!    then red, green, blue), in ProPhoto primaries with the sRGB encoding
//! 4. `ConvertToGrayscale` with the gray mixers, or else HSL; then `Saturation`, `Vibrance`
//! 5. The profile's `RGBTable` with its amount
//! 6. `ColorGrade*` / `SplitToning*`

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use super::parse::{Preset, Setting};
use super::table::{
    apply_amount, decode_big_table, lookup_look, lookup_rgb_3d, BigTable, Gamma, Gamut,
    LookEncoding, LookTable, Primaries, RgbTable,
};
use super::tone_data;
use crate::color::{inverse, mul, product, srgb_decode, srgb_encode, Matrix, Rgb};
use crate::cube::Cube;

pub const LUT_SIZE: usize = 33;
const CURVE_SAMPLES: usize = 256;

/// The scene value under the default rendering, at 33 even steps of the sRGB-encoded display
/// value: the tone curve of the raw converter, inverted. Fitted to its render of a gray ramp.
#[allow(clippy::approx_constant)]
#[rustfmt::skip]
const SCENE_VALUES: [f64; 33] = [
    0.00000, 0.00500, 0.00965, 0.01508, 0.02076, 0.02677,
    0.03343, 0.04044, 0.04824, 0.05663, 0.06593, 0.07559,
    0.08637, 0.09747, 0.10927, 0.12218, 0.13587, 0.15049,
    0.16624, 0.18364, 0.20461, 0.22696, 0.25074, 0.28267,
    0.31774, 0.35476, 0.41090, 0.47273, 0.55617, 0.66837,
    0.84216, 1.13507, 2.03647,
];

/// Camera calibration: for each slider (hue and saturation of the red, green, and blue
/// primary) at -100, -50, +50, and +100, what the slider adds to each row of the identity
/// matrix. A row holds the weight of red minus green and of blue minus green, so a gray stays
/// a gray. The matrix acts on scene values (`SCENE_VALUES`) in ProPhoto primaries, where the
/// raw converter turns the primaries. Fitted to renders of a chart.
#[allow(clippy::approx_constant)]
#[rustfmt::skip]
const CALIBRATION: [[[[f64; 2]; 3]; 4]; 6] = [
    [
        [[-0.0303, 0.0015], [-0.2885, -0.0120], [0.2944, -0.3335]],
        [[-0.0273, 0.0038], [-0.1450, -0.0004], [0.1391, -0.1668]],
        [[-0.0196, 0.0176], [0.1467, 0.0197], [-0.1550, 0.1599]],
        [[-0.0224, 0.0227], [0.3057, 0.0202], [-0.2776, 0.3051]],
    ],
    [
        [[-0.0155, 0.0124], [0.3988, 0.0070], [0.3960, -0.4014]],
        [[-0.0184, 0.0121], [0.1970, 0.0111], [0.1931, -0.2007]],
        [[-0.0273, 0.0084], [-0.1781, 0.0039], [-0.1940, 0.1893]],
        [[-0.0345, 0.0073], [-0.3316, -0.0122], [-0.3283, 0.3534]],
    ],
    [
        [[-0.3213, 0.0153], [0.0102, 0.0100], [-0.0030, 0.3232]],
        [[-0.1695, 0.0128], [0.0006, 0.0127], [-0.0064, 0.1679]],
        [[0.1290, 0.0112], [0.0027, 0.0053], [0.0014, -0.1720]],
        [[0.2861, 0.0160], [0.0134, -0.0023], [0.0128, -0.3448]],
    ],
    [
        [[-0.4083, 0.0022], [0.0104, -0.0041], [0.0076, -0.4118]],
        [[-0.2122, 0.0073], [0.0044, 0.0039], [0.0001, -0.2062]],
        [[0.1630, 0.0100], [-0.0043, 0.0163], [-0.0108, 0.1996]],
        [[0.3420, 0.0041], [0.0020, 0.0183], [-0.0127, 0.3781]],
    ],
    [
        [[0.2969, -0.2858], [0.0306, 0.2835], [0.0097, -0.0114]],
        [[0.1374, -0.1308], [0.0067, 0.1530], [0.0048, -0.0078]],
        [[-0.1879, 0.1454], [0.0031, -0.1470], [-0.0142, -0.0118]],
        [[-0.3524, 0.2789], [0.0129, -0.2988], [-0.0146, -0.0212]],
    ],
    [
        [[-0.4154, 0.4100], [0.0055, 0.4051], [-0.0034, 0.0025]],
        [[-0.2171, 0.2085], [0.0030, 0.2060], [-0.0052, -0.0017]],
        [[0.1748, -0.1760], [-0.0079, -0.1793], [-0.0034, -0.0140]],
        [[0.3763, -0.3426], [-0.0160, -0.3375], [-0.0012, -0.0192]],
    ],
];
const CALIBRATION_KEYS: [&str; 6] = [
    "RedHue",
    "RedSaturation",
    "GreenHue",
    "GreenSaturation",
    "BlueHue",
    "BlueSaturation",
];

/// The factor of the distance of a color from its gray, by the Saturation slider.
const SATURATION_FACTOR: [(f64, f64); 5] = [
    (-100.0, 0.0),
    (-50.0, 0.51),
    (0.0, 1.0),
    (50.0, 1.49),
    (100.0, 1.95),
];
/// Vibrance, fitted to renders of a hue chart. The hue and saturation are those of the color in
/// linear ProPhoto RGB, and the hue tables hold a factor every 30 degrees.
///
/// More vibrance multiplies the saturation and keeps the value: the base-2 logarithm of the
/// factor is the strength, times the hue factor (small for reds and oranges, which protects
/// skin), times (1 - saturation)^power.
const VIBRANCE_MORE: [(f64, f64); 3] = [(0.0, 0.0), (50.0, 0.44), (100.0, 1.062)];
const VIBRANCE_MORE_POWER: f64 = 0.597;
const VIBRANCE_MORE_HUE: [f64; 12] = [
    0.598, 0.641, 1.205, 1.124, 1.126, 1.216, 1.036, 1.066, 1.023, 1.047, 1.126, 0.792,
];
/// Less vibrance removes this part of the saturation, times the hue factor, times
/// (1 - saturation)^power. The color moves mostly toward the gray of its luminance and partly
/// toward the gray of its value.
const VIBRANCE_LESS: [(f64, f64); 3] = [(-100.0, 0.877), (-50.0, 0.525), (0.0, 0.0)];
const VIBRANCE_LESS_POWER: f64 = 0.205;
const VIBRANCE_LESS_HUE: [f64; 12] = [
    1.001, 0.995, 0.844, 0.951, 0.993, 1.019, 1.01, 0.979, 1.114, 1.1, 0.931, 1.062,
];
const VIBRANCE_LESS_TO_LUMINANCE: f64 = 0.666;

/// Color grading: how much each region acts along the encoded channel value, at 11 even steps
/// from 0 to 1, and the strength of each slider at 100. All fitted to renders of a gray ramp.
/// Shadows and highlights reach further into the tones as Blending grows: their profiles at
/// Blending 0, 50, and 100.
const GRADE_SHADOWS: [[f64; 11]; 3] = [
    [0.85, 0.85, 0.70, 0.37, 0.07, 0.01, 0.0, 0.0, 0.0, 0.0, 0.0],
    [
        0.85, 0.85, 0.75, 0.47, 0.21, 0.12, 0.07, 0.04, 0.02, 0.0, 0.0,
    ],
    [
        1.0, 0.98, 0.90, 0.77, 0.60, 0.43, 0.27, 0.14, 0.06, 0.015, 0.0,
    ],
];
const GRADE_HIGHLIGHTS: [[f64; 11]; 3] = [
    [0.0, 0.0, 0.0, 0.0, 0.0, 0.02, 0.10, 0.35, 0.49, 0.36, 0.0],
    [
        0.0, 0.02, 0.06, 0.08, 0.18, 0.20, 0.35, 0.50, 0.55, 0.36, 0.0,
    ],
    [
        0.02, 0.08, 0.25, 0.50, 0.75, 0.93, 1.0, 0.93, 0.75, 0.44, 0.0,
    ],
];
const GRADE_MIDTONES: [f64; 11] = [
    0.0, 0.07, 0.29, 0.65, 0.94, 0.97, 0.72, 0.39, 0.14, 0.03, 0.0,
];
const GRADE_GLOBAL: [f64; 11] = [
    1.0, 1.0, 0.97, 0.92, 0.86, 0.77, 0.65, 0.52, 0.37, 0.19, 0.0,
];
const GRADE_SHADOW_TINT: f64 = 2.4;
const GRADE_SHADOW_FLOOR: f64 = 0.186;
const GRADE_MIDTONE_TINT: f64 = 0.88;
const GRADE_HIGHLIGHT_TINT: f64 = 1.0;
/// The largest gain of the highlight tint, as a logarithm.
const GRADE_HIGHLIGHT_CAP: f64 = 0.34;
const GRADE_GLOBAL_TINT: f64 = 1.12;
const GRADE_SHADOW_DARKEN: f64 = 0.98;
const GRADE_SHADOW_LIFT: f64 = 0.14;
const GRADE_MIDTONE_LUM: f64 = 0.376;
const GRADE_HIGHLIGHT_LUM: f64 = 0.30;
const GRADE_GLOBAL_LUM: f64 = 0.48;
/// Balance acts as more Blending for one region and less for the other: at 100, this much of
/// the whole Blending range.
const GRADE_BALANCE_SHIFT: f64 = 0.6;

/// The eight HSL / gray-mixer bands: name and center hue in degrees. The hue is that of the
/// color in linear ProPhoto RGB, where the bands of the raw converter lie; the centers are
/// fitted to its renders of a hue chart.
pub const HUE_BANDS: [(&str, f64); 8] = [
    ("Red", 3.4),
    ("Orange", 30.2),
    ("Yellow", 54.5),
    ("Green", 95.0),
    ("Aqua", 158.2),
    ("Blue", 232.6),
    ("Purple", 272.4),
    ("Magenta", 329.1),
];

/// The hue shift of each band in degrees at +100 and at -100.
const BAND_HUE_SHIFT: [(f64, f64); 8] = [
    (28.2, -27.2),
    (32.2, -31.6),
    (38.5, -31.7),
    (55.4, -36.6),
    (64.7, -58.6),
    (40.2, -62.9),
    (50.0, -39.9),
    (31.2, -52.7),
];
/// More saturation of a band at +100: the factor 2^(this * (1 - saturation)^power).
const BAND_SATURATION_MORE: (f64, f64) = (1.065, 0.8);
/// Luminance of a band at 100, as a base-2 logarithm of the value: lighter grows with the
/// saturation and with the room below white, darker with the saturation and the value.
/// Each tuple is the strength, the power of the saturation, and the power of the other term.
const BAND_LIGHTER: (f64, f64, f64) = (1.281, 0.498, 0.455);
const BAND_DARKER: (f64, f64, f64) = (4.36, 0.842, 0.382);

/// The `crs:WhiteBalance` name of a preset. An unknown name is `AsShot`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum WhiteBalance {
    #[default]
    AsShot,
    Auto,
    Daylight,
    Cloudy,
    Shade,
    Tungsten,
    Fluorescent,
    Flash,
    /// Absolute kelvin and tint of a Custom white balance.
    #[serde(rename_all = "camelCase")]
    Custom {
        temperature: f64,
        tint: f64,
    },
}

/// Settings that stay outside the baked LUT.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hints {
    pub exposure: Option<f64>,
    pub grain_amount: Option<f64>,
    pub grain_size: Option<f64>,
    pub grain_frequency: Option<f64>,
    pub incremental_temperature: Option<f64>,
    pub incremental_tint: Option<f64>,
    pub white_balance: WhiteBalance,
}

impl Hints {
    /// True when no exposure, grain, white-balance mode, or shift is set.
    pub fn is_empty(&self) -> bool {
        self.exposure.is_none()
            && self.grain_amount.is_none()
            && self.grain_size.is_none()
            && self.grain_frequency.is_none()
            && self.incremental_temperature.is_none()
            && self.incremental_tint.is_none()
            && matches!(self.white_balance, WhiteBalance::AsShot)
    }
}

/// Prepared transform: tables decoded, matrices built, curves sampled once.
pub struct Transform {
    look_table: Option<(LookTable, f64)>,
    rgb_table: Option<(RgbTable, f64, Matrix, Matrix)>,
    /// Look amount used to scale Look tables and Look point curves.
    look_amount: f64,
    /// Contrast + Highlights/Shadows/Whites/Blacks, sampled once in `new`.
    /// Highlights and Shadows: a curve of the luminance, as display grays.
    local_tone: [f64; CURVE_SAMPLES],
    basic_tone: [f64; CURVE_SAMPLES],
    grayscale: bool,
    gray_mixers: [f64; 8],
    hue_adj: [f64; 8],
    sat_adj: [f64; 8],
    lum_adj: [f64; 8],
    saturation: f64,
    vibrance: f64,
    /// The matrix of the camera calibration sliders, or none when all are 0.
    calibration: Option<Matrix>,
    /// The gain curves of the color grade (`ColorGrade::curves`).
    grade_curves: Option<[[f64; CURVE_SAMPLES]; 3]>,
    parametric: Option<[f64; CURVE_SAMPLES]>,
    master_curve: Option<Pchip>,
    channel_curves: [Option<Pchip>; 3],
    look_master_curve: Option<Pchip>,
    look_channel_curves: [Option<Pchip>; 3],
    to_prophoto: Matrix,
    from_prophoto: Matrix,
    /// Warning material gathered while building (tables / Look), for `warnings`.
    pub(crate) table_notes: TableNotes,
}

/// Table / Look issues discovered while building the transform (not recomputed later).
#[derive(Clone, Debug, Default)]
pub struct TableNotes {
    pub missing_table_payload: bool,
    pub rgb_unsupported: Option<String>,
    pub missing_named_profile: Option<String>,
    pub skipped_look_settings: bool,
}

/// The hue, saturation, and luminance of one color grading region.
#[derive(Clone, Copy, Debug, Default)]
struct Region {
    hue: f64,
    saturation: f64,
    luminance: f64,
}

#[derive(Clone, Debug, Default)]
struct ColorGrade {
    shadows: Region,
    midtones: Region,
    highlights: Region,
    global: Region,
    balance: f64,
    /// Blending from 0 to 1. A file with split toning only has none, and renders as 1.
    blending: f64,
}

#[derive(Clone, Debug)]
struct Parametric {
    shadows: f64,
    darks: f64,
    lights: f64,
    highlights: f64,
    shadow_split: f64,
    midtone_split: f64,
    highlight_split: f64,
}

impl Transform {
    pub fn new(preset: &Preset) -> Result<Self> {
        let to_prophoto = display_srgb_to_prophoto();
        let from_prophoto = inverse(&to_prophoto).unwrap();

        let look_amount = preset.look.as_ref().and_then(|l| l.amount).unwrap_or(1.0);
        let (look_table, rgb_table, mut table_notes) = resolve_tables(preset, look_amount)?;

        let grayscale = preset.bool_flag("ConvertToGrayscale")
            || preset
                .look
                .as_ref()
                .is_some_and(|l| l.bool_flag("ConvertToGrayscale"));

        let mut gray_mixers = [0.0; 8];
        for (i, (name, _)) in HUE_BANDS.iter().enumerate() {
            gray_mixers[i] = preset.number(&format!("GrayMixer{name}")).unwrap_or(0.0);
        }

        let mut hue_adj = [0.0; 8];
        let mut sat_adj = [0.0; 8];
        let mut lum_adj = [0.0; 8];
        for (i, (name, _)) in HUE_BANDS.iter().enumerate() {
            hue_adj[i] = preset
                .number(&format!("HueAdjustment{name}"))
                .unwrap_or(0.0);
            sat_adj[i] = preset
                .number(&format!("SaturationAdjustment{name}"))
                .unwrap_or(0.0);
            lum_adj[i] = preset
                .number(&format!("LuminanceAdjustment{name}"))
                .unwrap_or(0.0);
        }

        let contrast = preset.number("Contrast2012").unwrap_or(0.0);
        let highlights = preset.number("Highlights2012").unwrap_or(0.0);
        let shadows = preset.number("Shadows2012").unwrap_or(0.0);
        let whites = preset.number("Whites2012").unwrap_or(0.0);
        let blacks = preset.number("Blacks2012").unwrap_or(0.0);
        let mut local_tone = identity_curve();
        if highlights != 0.0 {
            compose(&mut local_tone, |x| highlights_curve(x, highlights));
        }
        if shadows != 0.0 {
            compose(&mut local_tone, |x| shadows_curve(x, shadows));
        }
        let mut basic_tone = identity_curve();
        if whites != 0.0 {
            compose(&mut basic_tone, |x| whites_curve(x, whites));
        }
        if blacks != 0.0 {
            compose(&mut basic_tone, |x| blacks_curve(x, blacks));
        }
        // Contrast comes after Whites and Blacks.
        if contrast != 0.0 {
            compose(&mut basic_tone, |x| contrast2012(x, contrast));
        }

        let parametric = Parametric::from_preset(preset).map(|p| {
            let mut samples = identity_curve();
            compose(&mut samples, |x| p.eval(x));
            // The sum of the regions can turn back at extreme amounts: keep the curve rising.
            for i in 1..samples.len() {
                samples[i] = samples[i].max(samples[i - 1]);
            }
            samples
        });

        let master_curve = point_curve("ToneCurvePV2012", preset.seq("ToneCurvePV2012"))?;
        let channel_curves = [
            point_curve("ToneCurvePV2012Red", preset.seq("ToneCurvePV2012Red"))?,
            point_curve("ToneCurvePV2012Green", preset.seq("ToneCurvePV2012Green"))?,
            point_curve("ToneCurvePV2012Blue", preset.seq("ToneCurvePV2012Blue"))?,
        ];
        let look_master_curve = match &preset.look {
            Some(l) => point_curve("ToneCurvePV2012", l.seq("ToneCurvePV2012"))?,
            None => None,
        };
        let look_channel_curves = match &preset.look {
            Some(l) => [
                point_curve("ToneCurvePV2012Red", l.seq("ToneCurvePV2012Red"))?,
                point_curve("ToneCurvePV2012Green", l.seq("ToneCurvePV2012Green"))?,
                point_curve("ToneCurvePV2012Blue", l.seq("ToneCurvePV2012Blue"))?,
            ],
            None => [None, None, None],
        };

        table_notes.missing_named_profile = missing_named_look_profile(
            preset,
            look_table.is_some(),
            rgb_table.is_some(),
            look_master_curve.is_some() || look_channel_curves.iter().any(|c| c.is_some()),
        );
        table_notes.skipped_look_settings = look_has_unread_color_settings(preset);

        Ok(Self {
            look_table,
            rgb_table,
            look_amount,
            local_tone,
            basic_tone,
            grayscale,
            gray_mixers,
            hue_adj,
            sat_adj,
            lum_adj,
            saturation: preset.number("Saturation").unwrap_or(0.0),
            vibrance: preset.number("Vibrance").unwrap_or(0.0),
            calibration: calibration(preset),
            grade_curves: ColorGrade::from_preset(preset).curves(),
            parametric,
            master_curve,
            channel_curves,
            look_master_curve,
            look_channel_curves,
            to_prophoto,
            from_prophoto,
            table_notes,
        })
    }

    /// Maps display sRGB 0..1 through the prepared preset.
    pub fn apply(&self, display: Rgb) -> Rgb {
        let mut rgb = display.map(|v| v.clamp(0.0, 1.0));

        // 0. Camera calibration turns the primaries of the scene, before all other stages.
        if let Some(matrix) = &self.calibration {
            let linear = mul(&self.to_prophoto, rgb.map(srgb_decode));
            let turned = mul(matrix, linear.map(scene_of)).map(display_of);
            rgb = mul(&self.from_prophoto, turned).map(|v| srgb_encode(v.clamp(0.0, 1.0)));
        }

        // 1. The profile: its look table in linear ProPhoto, then its point curves.
        if self.look_table.is_some() {
            let mut linear = mul(&self.to_prophoto, rgb.map(srgb_decode));
            linear = self.apply_look_table(linear);
            rgb = mul(&self.from_prophoto, linear).map(srgb_encode);
        }
        rgb = self.apply_point_curves(
            rgb,
            &self.look_master_curve,
            &self.look_channel_curves,
            self.look_amount,
        );

        // 2. Basic tone.
        if !curve_is_identity(&self.local_tone) {
            rgb = luminance_gain(rgb, |x| eval_samples(&self.local_tone, x));
        }
        if !curve_is_identity(&self.basic_tone) {
            rgb = self.wide_tone(rgb, |x| eval_samples(&self.basic_tone, x));
        }

        // 3. Parametric curve, then the point curves.
        if let Some(p) = &self.parametric {
            rgb = self.wide_tone(rgb, |x| eval_samples(p, x));
        }
        rgb = self.apply_point_curves(rgb, &self.master_curve, &self.channel_curves, 1.0);

        // 4. Grayscale or HSL, then Saturation and Vibrance.
        if self.grayscale {
            rgb = self.apply_grayscale(rgb);
        } else {
            rgb = self.apply_hsl(rgb);
        }
        rgb = self.apply_saturation_vibrance(rgb);

        // 5. The profile's RGB table (output-referred).
        if let Some((table, amount, prophoto_to_table, table_to_prophoto)) = &self.rgb_table {
            rgb = apply_rgb_table(
                table,
                *amount,
                prophoto_to_table,
                table_to_prophoto,
                &self.to_prophoto,
                &self.from_prophoto,
                rgb,
            );
        }

        // 6. Color grading.
        rgb = self.apply_color_grade(rgb);

        rgb.map(|v| v.clamp(0.0, 1.0))
    }

    /// The master curve on the largest and smallest channel, then the channel curves, each
    /// moved toward the identity by `amount`.
    fn apply_point_curves(
        &self,
        rgb: Rgb,
        master: &Option<Pchip>,
        channels: &[Option<Pchip>; 3],
        amount: f64,
    ) -> Rgb {
        if master.is_none() && channels.iter().all(Option::is_none) {
            return rgb;
        }
        self.in_wide(rgb, |mut wide| {
            if let Some(c) = master {
                wide = rgb_tone(wide, |x| x + amount * (c.eval(x) - x));
            }
            for (value, curve) in wide.iter_mut().zip(channels) {
                if let Some(c) = curve {
                    let x = value.clamp(0.0, 1.0);
                    *value = x + amount * (c.eval(x) - x);
                }
            }
            wide
        })
    }

    /// Runs `f` on the color in ProPhoto primaries with the sRGB encoding, where the raw
    /// converter applies its tone sliders and curves.
    fn in_wide(&self, rgb: Rgb, f: impl FnOnce(Rgb) -> Rgb) -> Rgb {
        let encode = |v: f64| srgb_encode(v.clamp(0.0, 1.0));
        let wide = mul(&self.to_prophoto, rgb.map(srgb_decode)).map(encode);
        mul(&self.from_prophoto, f(wide).map(srgb_decode)).map(encode)
    }

    /// A tone curve on the largest and smallest channel (`rgb_tone`) in that space: a
    /// saturated color keeps more of its saturation than with the curve on its sRGB channels.
    fn wide_tone(&self, rgb: Rgb, curve: impl Fn(f64) -> f64) -> Rgb {
        self.in_wide(rgb, |wide| rgb_tone(wide, curve))
    }

    fn apply_look_table(&self, rgb: Rgb) -> Rgb {
        let Some((table, amount)) = &self.look_table else {
            return rgb;
        };
        if *amount == 0.0 {
            return rgb;
        }
        let hsv = rgb_to_hsv(rgb);
        let mut index_hsv = hsv;
        if table.encoding == LookEncoding::Srgb {
            index_hsv[2] = srgb_encode(index_hsv[2].clamp(0.0, 1.0));
        }
        let delta = lookup_look(table, index_hsv);
        let sat_scale = 1.0 + amount * (delta[1] as f64 - 1.0);
        let mut out = [
            (hsv[0] + amount * delta[0] as f64 / 360.0).rem_euclid(1.0),
            (hsv[1] * sat_scale).clamp(0.0, 1.0),
            hsv[2] * (1.0 + amount * (delta[2] as f64 - 1.0)),
        ];
        if table.encoding == LookEncoding::Srgb {
            let encoded_v = srgb_encode(hsv[2].clamp(0.0, 1.0));
            let mapped_v = encoded_v * (1.0 + amount * (delta[2] as f64 - 1.0));
            out[2] = srgb_decode(mapped_v.clamp(0.0, 1.0));
        }
        hsv_to_rgb(out)
    }

    fn apply_grayscale(&self, rgb: Rgb) -> Rgb {
        let hsv = rgb_to_hsv(mul(&self.to_prophoto, rgb.map(srgb_decode)).map(|v| v.max(0.0)));
        let w = hue_weights(hsv[0] * 360.0);
        // Each mixer scales how much that hue band contributes to the gray value.
        let boost: f64 = (0..8).map(|i| w[i] * self.gray_mixers[i] / 100.0).sum();
        let y = (gray_of(rgb) * (1.0 + hue_strength(hsv[1]) * boost)).clamp(0.0, 1.0);
        [y, y, y]
    }

    /// The HSL bands change the hue, saturation, and value of the color in linear ProPhoto
    /// RGB. No band changes a gray: each change needs saturation.
    fn apply_hsl(&self, rgb: Rgb) -> Rgb {
        if self
            .hue_adj
            .iter()
            .chain(&self.sat_adj)
            .chain(&self.lum_adj)
            .all(|v| *v == 0.0)
        {
            return rgb;
        }
        let linear = mul(&self.to_prophoto, rgb.map(srgb_decode)).map(|v| v.max(0.0));
        let mut hsv = rgb_to_hsv(linear);
        let (saturation, value) = (hsv[1], hsv[2].min(1.0));
        let w = hue_weights(hsv[0] * 360.0);
        let (mut turn, mut less, mut more, mut lift) = (0.0, 1.0, 0.0, 0.0);
        for i in 0..8 {
            let part = |amount: f64| (amount / 100.0).clamp(-1.0, 1.0);
            let hue = part(self.hue_adj[i]);
            let (up, down) = BAND_HUE_SHIFT[i];
            turn += w[i] * if hue > 0.0 { hue * up } else { -hue * down };
            // Less saturation is linear in the factor and reaches gray at -100.
            let sat = part(self.sat_adj[i]);
            if sat < 0.0 {
                less *= 1.0 + w[i] * sat;
            } else {
                more += w[i] * sat;
            }
            // Luminance is not linear in the slider: half the slider darkens by about a
            // third and lightens by more than half.
            let lum = part(self.lum_adj[i]);
            lift += w[i]
                * if lum > 0.0 {
                    let (strength, by_saturation, by_room) = BAND_LIGHTER;
                    lum.powf(0.79)
                        * strength
                        * saturation.powf(by_saturation)
                        * (1.0 - value).powf(by_room)
                } else {
                    let (strength, by_saturation, by_value) = BAND_DARKER;
                    -(-lum).powf(1.6)
                        * strength
                        * saturation.powf(by_saturation)
                        * value.powf(by_value)
                };
        }
        let (strength, power) = BAND_SATURATION_MORE;
        hsv[0] = (hsv[0] + turn / 360.0).rem_euclid(1.0);
        hsv[1] = (saturation * less * (more * strength * (1.0 - saturation).powf(power)).exp2())
            .clamp(0.0, 1.0);
        hsv[2] *= lift.exp2();
        mul(&self.from_prophoto, hsv_to_rgb(hsv)).map(|v| srgb_encode(v.clamp(0.0, 1.0)))
    }

    /// Saturation moves a color away from, or toward, the gray of its luminance in linear
    /// light. Vibrance follows, with a strength that depends on the hue and the saturation.
    fn apply_saturation_vibrance(&self, rgb: Rgb) -> Rgb {
        let mut linear = rgb.map(|v| srgb_decode(v.clamp(0.0, 1.0)));
        let luminance = |c: Rgb| 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
        let toward_gray = |c: Rgb, factor: f64| {
            let gray = luminance(c);
            c.map(|v| (gray + factor * (v - gray)).clamp(0.0, 1.0))
        };
        if self.saturation != 0.0 {
            linear = toward_gray(linear, interpolate(&SATURATION_FACTOR, self.saturation));
        }
        if self.vibrance == 0.0 {
            return if self.saturation == 0.0 {
                rgb
            } else {
                linear.map(srgb_encode)
            };
        }
        let wide = mul(&self.to_prophoto, linear).map(|v| v.max(0.0));
        let hsv = rgb_to_hsv(wide);
        let by_hue = |table: &[f64; 12]| {
            let position = hsv[0] * 12.0;
            let low = (position as usize) % 12;
            let t = position - position.floor();
            table[low] * (1.0 - t) + table[(low + 1) % 12] * t
        };
        let room = 1.0 - hsv[1].min(1.0);
        let wide = if self.vibrance > 0.0 {
            let strength = interpolate(&VIBRANCE_MORE, self.vibrance)
                * by_hue(&VIBRANCE_MORE_HUE)
                * room.powf(VIBRANCE_MORE_POWER);
            hsv_to_rgb([hsv[0], (hsv[1] * strength.exp2()).min(1.0), hsv[2]])
        } else {
            let less = interpolate(&VIBRANCE_LESS, self.vibrance)
                * by_hue(&VIBRANCE_LESS_HUE)
                * room.powf(VIBRANCE_LESS_POWER);
            let factor = (1.0 - less).clamp(0.0, 1.0);
            let of_luminance = mul(&self.to_prophoto, toward_gray(linear, factor));
            let of_value = hsv_to_rgb([hsv[0], hsv[1] * factor, hsv[2]]);
            std::array::from_fn(|c| {
                VIBRANCE_LESS_TO_LUMINANCE * of_luminance[c]
                    + (1.0 - VIBRANCE_LESS_TO_LUMINANCE) * of_value[c]
            })
        };
        mul(&self.from_prophoto, wide).map(|v| srgb_encode(v.clamp(0.0, 1.0)))
    }

    /// Color grading is one gain curve for each channel of linear ProPhoto RGB: the gain of a
    /// channel depends on the value of that channel only.
    fn apply_color_grade(&self, rgb: Rgb) -> Rgb {
        let Some(curves) = &self.grade_curves else {
            return rgb;
        };
        let linear = mul(&self.to_prophoto, rgb.map(srgb_decode));
        let graded: Rgb = std::array::from_fn(|c| {
            let value = linear[c].clamp(0.0, 1.0);
            value * eval_samples(&curves[c], srgb_encode(value))
        });
        mul(&self.from_prophoto, graded).map(|v| srgb_encode(v.clamp(0.0, 1.0)))
    }
}

impl ColorGrade {
    /// Split toning is the older record of the same grade: a file holds each region one time,
    /// so a `ColorGrade` value wins over the `SplitToning` value of that region.
    fn from_preset(preset: &Preset) -> Self {
        let number = |keys: &[&str]| keys.iter().find_map(|k| preset.number(k)).unwrap_or(0.0);
        let region = |name: &str, split: Option<&str>| Region {
            hue: number(&[
                &format!("ColorGrade{name}Hue"),
                &split.map_or(String::new(), |s| format!("SplitToning{s}Hue")),
            ]),
            saturation: number(&[
                &format!("ColorGrade{name}Sat"),
                &split.map_or(String::new(), |s| format!("SplitToning{s}Saturation")),
            ]),
            luminance: number(&[&format!("ColorGrade{name}Lum")]),
        };
        Self {
            shadows: region("Shadow", Some("Shadow")),
            midtones: region("Midtone", None),
            highlights: region("Highlight", Some("Highlight")),
            global: region("Global", None),
            balance: number(&["ColorGradeBalance", "SplitToningBalance"]),
            blending: preset.number("ColorGradeBlending").unwrap_or(100.0) / 100.0,
        }
    }

    /// The gain of each channel of linear ProPhoto RGB over its sRGB-encoded value, or none
    /// for a grade that changes nothing.
    fn curves(&self) -> Option<[[f64; CURVE_SAMPLES]; 3]> {
        let regions = [self.shadows, self.midtones, self.highlights, self.global];
        if regions
            .iter()
            .all(|r| r.saturation == 0.0 && r.luminance == 0.0)
        {
            return None;
        }
        // A positive Balance gives the tones to the highlights. The midtones have one profile.
        let shift = GRADE_BALANCE_SHIFT * self.balance / 100.0;
        let (for_shadows, for_highlights) = (self.blending - shift, self.blending + shift);
        let tint = |region: Region| -> Rgb {
            hsv_to_rgb([region.hue.rem_euclid(360.0) / 360.0, 1.0, 1.0])
        };
        let luminance = |c: Rgb| 0.288 * c[0] + 0.712 * c[1];
        Some(std::array::from_fn(|channel| {
            std::array::from_fn(|i| {
                let x = i as f64 / (CURVE_SAMPLES - 1) as f64;
                let (shadows, midtones, highlights, global) = (
                    blended_profile(&GRADE_SHADOWS, for_shadows, x),
                    profile(&GRADE_MIDTONES, x),
                    blended_profile(&GRADE_HIGHLIGHTS, for_highlights, x),
                    profile(&GRADE_GLOBAL, x),
                );
                // Luminance first: one gain for all channels.
                let mut gain = 1.0;
                let lum = self.shadows.luminance / 100.0;
                if lum < 0.0 {
                    gain *= 1.0 + GRADE_SHADOW_DARKEN * lum * shadows.powi(3);
                } else if lum > 0.0 && x > 0.0 {
                    // A lift adds to the encoded value.
                    let lifted = (x + GRADE_SHADOW_LIFT * lum * shadows * shadows).min(1.0);
                    gain *= srgb_decode(lifted) / srgb_decode(x);
                }
                let log_gain = GRADE_MIDTONE_LUM * self.midtones.luminance / 100.0 * midtones
                    + GRADE_HIGHLIGHT_LUM * self.highlights.luminance / 100.0
                        * smoothstep(0.4, 0.8, x)
                    + GRADE_GLOBAL_LUM * self.global.luminance / 100.0 * global;
                gain *= log_gain.exp();
                // The tint of the shadows is linear, the other regions are gains of the
                // logarithm. Shadows and highlights keep the luminance of the tint until a
                // channel reaches the floor or the cap; from there the other channels move.
                let color = tint(self.shadows);
                let strength = GRADE_SHADOW_TINT * self.shadows.saturation / 100.0;
                if strength > 0.0 {
                    let lowest = color[0].min(color[1]).min(color[2]);
                    let neutral =
                        luminance(color).min(lowest + (1.0 - GRADE_SHADOW_FLOOR) / strength);
                    gain *= 1.0 + shadows * strength * (color[channel] - neutral);
                }
                let color = tint(self.midtones);
                let mut log_gain = GRADE_MIDTONE_TINT * self.midtones.saturation / 100.0
                    * midtones
                    * (color[channel] - 0.5);
                let color = tint(self.highlights);
                let strength = GRADE_HIGHLIGHT_TINT * self.highlights.saturation / 100.0;
                if strength > 0.0 {
                    let highest = color[0].max(color[1]).max(color[2]);
                    let neutral = luminance(color).max(highest - GRADE_HIGHLIGHT_CAP / strength);
                    log_gain += strength * highlights * (color[channel] - neutral);
                }
                let color = tint(self.global);
                log_gain += GRADE_GLOBAL_TINT * self.global.saturation / 100.0
                    * global
                    * (color[channel] - 0.5);
                gain * log_gain.exp()
            })
        }))
    }
}

impl Parametric {
    fn from_preset(preset: &Preset) -> Option<Self> {
        let shadows = preset.number("ParametricShadows").unwrap_or(0.0);
        let darks = preset.number("ParametricDarks").unwrap_or(0.0);
        let lights = preset.number("ParametricLights").unwrap_or(0.0);
        let highlights = preset.number("ParametricHighlights").unwrap_or(0.0);
        if shadows == 0.0 && darks == 0.0 && lights == 0.0 && highlights == 0.0 {
            return None;
        }
        Some(Self {
            shadows,
            darks,
            lights,
            highlights,
            shadow_split: preset.number("ParametricShadowSplit").unwrap_or(25.0) / 100.0,
            midtone_split: preset.number("ParametricMidtoneSplit").unwrap_or(50.0) / 100.0,
            highlight_split: preset.number("ParametricHighlightSplit").unwrap_or(75.0) / 100.0,
        })
    }

    /// The curve at a display value: the measured response of each region (`tone_data`, at the
    /// default splits 25, 50, 75) added up. Other splits move the regions: the value goes to
    /// the place it has under the default splits, and the result comes back from there.
    fn eval(&self, x: f64) -> f64 {
        let s = self.shadow_split.clamp(0.01, 0.97);
        let m = self.midtone_split.clamp(s + 0.01, 0.98);
        let h = self.highlight_split.clamp(m + 0.01, 0.99);
        let (own, default) = ([0.0, s, m, h, 1.0], [0.0, 0.25, 0.5, 0.75, 1.0]);
        let warp = |v: f64, from: &[f64; 5], to: &[f64; 5]| {
            let v = v.clamp(0.0, 1.0);
            let i = (0..4).rfind(|&i| from[i] <= v).unwrap_or(0);
            to[i] + (v - from[i]) / (from[i + 1] - from[i]) * (to[i + 1] - to[i])
        };
        let at = warp(x, &own, &default);
        let regions = [
            (&tone_data::PARAMETRIC_SHADOWS, self.shadows),
            (&tone_data::PARAMETRIC_DARKS, self.darks),
            (&tone_data::PARAMETRIC_LIGHTS, self.lights),
            (&tone_data::PARAMETRIC_HIGHLIGHTS, self.highlights),
        ];
        let moved: f64 = regions
            .iter()
            .map(|(slider, amount)| slider.eval(*amount, at) - at)
            .sum();
        warp(at + moved, &default, &own)
    }
}

/// Samples the preset into a 33³ identity-domain Cube, with the base bridge on.
pub fn sample_cube(preset: &Preset) -> Result<Cube> {
    let transform = Transform::new(preset)?;
    sample_cube_with(&transform, preset.name.clone(), true)
}

/// Samples an already-built transform (so `prepare` does not build it twice).
/// When `base_bridge` is set, each sample runs through [`super::bridge::apply`] first.
pub fn sample_cube_with(
    transform: &Transform,
    title: Option<String>,
    base_bridge: bool,
) -> Result<Cube> {
    let last = (LUT_SIZE - 1) as f64;
    let mut table = Vec::with_capacity(LUT_SIZE.pow(3));
    for b in 0..LUT_SIZE {
        for g in 0..LUT_SIZE {
            for r in 0..LUT_SIZE {
                let mut rgb = [r as f64 / last, g as f64 / last, b as f64 / last];
                if base_bridge {
                    rgb = super::bridge::apply(rgb);
                }
                table.push(transform.apply(rgb).map(|v| v as f32));
            }
        }
    }
    Ok(Cube {
        title,
        three_d: true,
        size: LUT_SIZE,
        domain_min: [0.0; 3],
        domain_max: [1.0; 3],
        table,
        shaper: None,
        video_input: false,
        video_output: false,
        image_layout: None,
    })
}

/// Convenience: parse-free apply for tests that already hold a preset.
pub fn apply(preset: &Preset, display: Rgb) -> Result<Rgb> {
    Ok(Transform::new(preset)?.apply(display))
}

pub fn hints(preset: &Preset) -> Hints {
    let num = |key: &str| preset.number(key).and_then(|v| (v != 0.0).then_some(v));
    Hints {
        exposure: num("Exposure2012"),
        grain_amount: num("GrainAmount"),
        grain_size: num("GrainSize"),
        grain_frequency: num("GrainFrequency"),
        incremental_temperature: num("IncrementalTemperature"),
        incremental_tint: num("IncrementalTint"),
        white_balance: white_balance(preset),
    }
}

fn white_balance(preset: &Preset) -> WhiteBalance {
    let name = preset
        .text("WhiteBalance")
        .unwrap_or("As Shot")
        .to_ascii_lowercase();
    match name.as_str() {
        "as shot" => WhiteBalance::AsShot,
        "auto" => WhiteBalance::Auto,
        "daylight" => WhiteBalance::Daylight,
        "cloudy" => WhiteBalance::Cloudy,
        "shade" => WhiteBalance::Shade,
        "tungsten" => WhiteBalance::Tungsten,
        "fluorescent" => WhiteBalance::Fluorescent,
        "flash" => WhiteBalance::Flash,
        "custom" => WhiteBalance::Custom {
            temperature: preset.number("Temperature").unwrap_or(5500.0),
            tint: preset.number("Tint").unwrap_or(0.0),
        },
        _ => WhiteBalance::AsShot,
    }
}

// --- Warnings --------------------------------------------------------------

/// Lightroom defaults for skipped settings. Only a non-default value is reported.
/// Everything else in the skipped list defaults to 0 or off.
const SKIP_DEFAULTS: &[(&str, &str)] = &[
    ("Sharpness", "40"),
    ("SharpenRadius", "1.0"),
    ("SharpenDetail", "25"),
    ("SharpenEdgeMasking", "0"),
    ("LuminanceSmoothing", "0"),
    ("ColorNoiseReduction", "25"),
    ("ColorNoiseReductionDetail", "50"),
    ("ColorNoiseReductionSmoothness", "50"),
];

/// One short plain sentence per group: approximated, skipped, then table / Look notes.
pub fn warnings(preset: &Preset, transform: &Transform) -> Vec<String> {
    let mut out = Vec::new();

    let approx = [
        ("Highlights2012", "Highlights"),
        ("Shadows2012", "Shadows"),
        ("Whites2012", "Whites"),
        ("Blacks2012", "Blacks"),
    ]
    .into_iter()
    .filter(|(key, _)| preset.number(key).is_some_and(|v| v != 0.0))
    .map(|(_, name)| name)
    .collect::<Vec<_>>();
    if !approx.is_empty() {
        out.push(format!(
            "Approximated {} with a fixed global curve.",
            join_names(&approx)
        ));
    }

    let mut skipped = Vec::new();
    let skip_zero = [
        ("Clarity", "Clarity"),
        ("Clarity2012", "Clarity"),
        ("Texture", "Texture"),
        ("Dehaze", "Dehaze"),
        ("PostCropVignetteAmount", "vignette"),
        ("VignetteAmount", "vignette"),
        ("LensProfileEnable", "lens corrections"),
        ("AutoLateralCA", "lens corrections"),
        ("CropTop", "crop"),
        ("CropLeft", "crop"),
        ("CropBottom", "crop"),
        ("CropRight", "crop"),
        ("CropAngle", "crop"),
        ("ShadowTint", "camera calibration"),
    ];
    for (key, name) in skip_zero {
        if is_non_default(preset, key) && !skipped.contains(&name) {
            skipped.push(name);
        }
    }
    for (key, name) in [
        ("Sharpness", "sharpening"),
        ("SharpenRadius", "sharpening"),
        ("SharpenDetail", "sharpening"),
        ("SharpenEdgeMasking", "sharpening"),
        ("LuminanceSmoothing", "noise reduction"),
        ("ColorNoiseReduction", "noise reduction"),
        ("ColorNoiseReductionDetail", "noise reduction"),
        ("ColorNoiseReductionSmoothness", "noise reduction"),
    ] {
        if is_non_default(preset, key) && !skipped.contains(&name) {
            skipped.push(name);
        }
    }

    if preset
        .text("CameraProfile")
        .is_some_and(|p| !p.is_empty() && !is_neutral_profile(p))
    {
        skipped.push("camera profile");
    }

    if has_local_adjustments(preset) {
        skipped.push("local adjustments and masks");
    }

    if !skipped.is_empty() {
        out.push(format!("Skipped {}.", join_names(&skipped)));
    }

    // White balance stays outside the LUT; the recipe of a later milestone can hold it.
    let hints = hints(preset);
    let wb_set = !matches!(hints.white_balance, WhiteBalance::AsShot)
        || hints.incremental_temperature.is_some()
        || hints.incremental_tint.is_some();
    if wb_set {
        out.push("The film look does not hold the white balance.".into());
    }

    let notes = &transform.table_notes;
    if notes.missing_table_payload {
        out.push("Skipped the profile table: the preset does not contain it.".into());
    }
    if let Some(reason) = &notes.rgb_unsupported {
        out.push(format!("Skipped RGBTable: {reason}."));
    }
    if let Some(name) = &notes.missing_named_profile {
        out.push(format!(
            "Skipped the profile \"{name}\": the preset does not contain it."
        ));
    }
    if notes.skipped_look_settings {
        out.push("Skipped some settings of the profile.".into());
    }

    out
}

fn is_non_default(preset: &Preset, key: &str) -> bool {
    let Some(value) = preset.text(key) else {
        return false;
    };
    if let Some((_, default)) = SKIP_DEFAULTS.iter().find(|(k, _)| *k == key) {
        return !values_equal(value, default);
    }
    // Default 0 / false / off.
    match key {
        "LensProfileEnable" | "AutoLateralCA" => {
            value != "0" && !value.eq_ignore_ascii_case("false")
        }
        _ => {
            preset.number(key).is_some_and(|n| n != 0.0)
                || (preset.number(key).is_none()
                    && value != "0"
                    && !value.eq_ignore_ascii_case("false"))
        }
    }
}

fn values_equal(a: &str, b: &str) -> bool {
    if let (Ok(x), Ok(y)) = (a.parse::<f64>(), b.parse::<f64>()) {
        (x - y).abs() < 1e-9
    } else {
        a == b
    }
}

fn has_local_adjustments(preset: &Preset) -> bool {
    const KEYS: &[&str] = &[
        "MaskGroupBasedCorrections",
        "GradientBasedCorrections",
        "CircularGradientBasedCorrections",
        "PaintBasedCorrections",
        "RetouchInfo",
        "RetouchAreas",
    ];
    KEYS.iter().any(|k| match preset.settings.get(*k) {
        Some(Setting::Text(v)) => !v.is_empty() && v != "0",
        Some(Setting::Seq(v)) => !v.is_empty(),
        Some(Setting::Map(v)) => !v.is_empty(),
        None => false,
    })
}

fn is_neutral_profile(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.contains("adobe standard")
        || n == "embedded"
        || n.contains("camera standard")
        || n == "default"
}

fn join_names(names: &[&str]) -> String {
    match names {
        [] => String::new(),
        [a] => (*a).to_owned(),
        [a, b] => format!("{a} and {b}"),
        _ => {
            let (last, rest) = names.split_last().unwrap();
            format!("{} and {last}", rest.join(", "))
        }
    }
}

// --- Curves ----------------------------------------------------------------

/// The measured response of each basic tone slider on a display value (`tone_data`).
pub fn contrast2012(x: f64, amount: f64) -> f64 {
    tone_data::CONTRAST.eval(amount, x)
}

/// Highlights and Shadows adapt to each part of the picture; this is the median change of the
/// luminance of a photograph, which `luminance_gain` applies.
pub fn highlights_curve(x: f64, amount: f64) -> f64 {
    tone_data::HIGHLIGHTS.eval(amount, x)
}

pub fn shadows_curve(x: f64, amount: f64) -> f64 {
    tone_data::SHADOWS.eval(amount, x)
}

pub fn whites_curve(x: f64, amount: f64) -> f64 {
    tone_data::WHITES.eval(amount, x)
}

pub fn blacks_curve(x: f64, amount: f64) -> f64 {
    tone_data::BLACKS.eval(amount, x)
}

/// Monotone cubic Hermite (Fritsch–Carlson / PCHIP). Exact at knots, no overshoot, C1.
#[derive(Clone, Debug)]
pub struct Pchip {
    xs: Vec<f64>,
    ys: Vec<f64>,
    slopes: Vec<f64>,
}

impl Pchip {
    pub fn new(points: &[(f64, f64)]) -> Option<Self> {
        if points.len() < 2 {
            return None;
        }
        let mut pts = points.to_vec();
        pts.sort_by(|a, b| a.0.total_cmp(&b.0));
        pts.dedup_by(|a, b| (a.0 - b.0).abs() < 1e-15);
        if pts.len() < 2 {
            return None;
        }
        let n = pts.len();
        let xs: Vec<f64> = pts.iter().map(|p| p.0).collect();
        let ys: Vec<f64> = pts.iter().map(|p| p.1).collect();
        let mut delta = vec![0.0; n - 1];
        for i in 0..n - 1 {
            delta[i] = (ys[i + 1] - ys[i]) / (xs[i + 1] - xs[i]);
        }
        let mut slopes = vec![0.0; n];
        slopes[0] = delta[0];
        slopes[n - 1] = delta[n - 2];
        for i in 1..n - 1 {
            if delta[i - 1] == 0.0 || delta[i] == 0.0 || delta[i - 1].signum() != delta[i].signum()
            {
                slopes[i] = 0.0;
            } else {
                let w1 = 2.0 * (xs[i + 1] - xs[i]) + (xs[i] - xs[i - 1]);
                let w2 = (xs[i + 1] - xs[i]) + 2.0 * (xs[i] - xs[i - 1]);
                slopes[i] = (w1 + w2) / (w1 / delta[i - 1] + w2 / delta[i]);
            }
        }
        // Fritsch–Carlson limiter.
        for i in 0..n - 1 {
            if delta[i] == 0.0 {
                slopes[i] = 0.0;
                slopes[i + 1] = 0.0;
            } else {
                let a = slopes[i] / delta[i];
                let b = slopes[i + 1] / delta[i];
                let s = a * a + b * b;
                if s > 9.0 {
                    let t = 3.0 / s.sqrt();
                    slopes[i] = t * a * delta[i];
                    slopes[i + 1] = t * b * delta[i];
                }
            }
        }
        Some(Self { xs, ys, slopes })
    }

    pub fn eval(&self, x: f64) -> f64 {
        if x <= self.xs[0] {
            return self.ys[0];
        }
        if x >= self.xs[self.xs.len() - 1] {
            return self.ys[self.ys.len() - 1];
        }
        let mut i = 0;
        while i + 1 < self.xs.len() && x > self.xs[i + 1] {
            i += 1;
        }
        let h = self.xs[i + 1] - self.xs[i];
        let t = (x - self.xs[i]) / h;
        let t2 = t * t;
        let t3 = t2 * t;
        let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
        let h10 = t3 - 2.0 * t2 + t;
        let h01 = -2.0 * t3 + 3.0 * t2;
        let h11 = t3 - t2;
        h00 * self.ys[i]
            + h10 * h * self.slopes[i]
            + h01 * self.ys[i + 1]
            + h11 * h * self.slopes[i + 1]
    }
}

fn point_curve(name: &str, seq: Option<&[String]>) -> Result<Option<Pchip>> {
    let Some(points) = seq else {
        return Ok(None);
    };
    if points.is_empty() {
        return Ok(None);
    }
    let mut curve = Vec::with_capacity(points.len());
    for p in points {
        let mut parts = p.split(',');
        let x_text = parts
            .next()
            .map(str::trim)
            .ok_or_else(|| anyhow::anyhow!("Invalid {name} curve point"))?;
        let y_text = parts
            .next()
            .map(str::trim)
            .ok_or_else(|| anyhow::anyhow!("Invalid {name} curve point"))?;
        let x: f64 = x_text
            .parse()
            .ok()
            .filter(|v: &f64| v.is_finite())
            .ok_or_else(|| anyhow::anyhow!("Invalid {name} curve point"))?;
        let y: f64 = y_text
            .parse()
            .ok()
            .filter(|v: &f64| v.is_finite())
            .ok_or_else(|| anyhow::anyhow!("Invalid {name} curve point"))?;
        curve.push((x / 255.0, y / 255.0));
    }
    Ok(Pchip::new(&curve))
}

// --- Color spaces / RGB table ----------------------------------------------

fn clamp_amount(amount: f64, min: f64, max: f64) -> f64 {
    // Plain factor; clamp to the table's own range when present, else to 0..2.
    let (lo, hi) = if min.is_finite() && max.is_finite() {
        (min, max.max(min))
    } else {
        (0.0, 2.0)
    };
    amount.clamp(lo, hi)
}

type ResolvedLookTable = Option<(LookTable, f64)>;
type ResolvedRgbTable = Option<(RgbTable, f64, Matrix, Matrix)>;

/// Resolves LookTable / RGBTable from the Look first, then the top level.
fn resolve_tables(
    preset: &Preset,
    look_amount: f64,
) -> Result<(ResolvedLookTable, ResolvedRgbTable, TableNotes)> {
    let mut notes = TableNotes::default();
    let look = preset.look.as_ref();

    let look_fp = look
        .and_then(|l| l.text("LookTable"))
        .or_else(|| preset.text("LookTable"))
        .map(str::trim);
    let rgb_fp = look
        .and_then(|l| l.text("RGBTable"))
        .or_else(|| preset.text("RGBTable"))
        .map(str::trim);

    let mut look_table = None;
    if let Some(fp) = look_fp {
        match preset.tables.get(fp) {
            None => notes.missing_table_payload = true,
            Some(encoded) => match decode_big_table(encoded)? {
                BigTable::Look(table) => {
                    let amount = clamp_amount(look_amount, table.min_amount, table.max_amount);
                    look_table = Some((table, amount));
                }
                BigTable::Rgb(_) => bail!("LookTable fingerprint points at an RGB table"),
            },
        }
    }

    let mut rgb_table = None;
    if let Some(fp) = rgb_fp {
        match preset.tables.get(fp) {
            None => notes.missing_table_payload = true,
            Some(encoded) => match decode_big_table(encoded)? {
                BigTable::Rgb(table) => match rgb_space_matrices(&table) {
                    Ok((prophoto_to_table, table_to_prophoto)) => {
                        let rgb_amount = look
                            .and_then(|l| l.number("RGBTableAmount"))
                            .or_else(|| preset.number("RGBTableAmount"))
                            .unwrap_or(1.0);
                        let amount = clamp_amount(
                            look_amount * rgb_amount,
                            table.min_amount,
                            table.max_amount,
                        );
                        rgb_table = Some((table, amount, prophoto_to_table, table_to_prophoto));
                    }
                    Err(reason) => notes.rgb_unsupported = Some(reason),
                },
                BigTable::Look(_) => bail!("RGBTable fingerprint points at a look table"),
            },
        }
    }

    Ok((look_table, rgb_table, notes))
}

/// Returns `(prophoto_to_table, table_to_prophoto)` for the table's primaries.
fn rgb_space_matrices(table: &RgbTable) -> Result<(Matrix, Matrix), String> {
    if table.dimensions != 3 {
        return Err("only 3D RGB tables are supported".into());
    }
    if !matches!(table.gamut, Gamut::Clip | Gamut::Extend) {
        return Err(format!("unsupported gamut {:?}", table.gamut));
    }
    let table_to_prophoto = match table.primaries {
        Primaries::Srgb => d65_space_to_prophoto([[0.64, 0.33], [0.30, 0.60], [0.15, 0.06]]),
        Primaries::Adobe => d65_space_to_prophoto([[0.64, 0.33], [0.21, 0.71], [0.15, 0.06]]),
        Primaries::ProPhoto => identity_matrix(),
        Primaries::P3 => d65_space_to_prophoto([[0.680, 0.320], [0.265, 0.690], [0.150, 0.060]]),
        Primaries::Rec2020 => {
            d65_space_to_prophoto([[0.708, 0.292], [0.170, 0.797], [0.131, 0.046]])
        }
    };
    let prophoto_to_table = inverse(&table_to_prophoto).unwrap();
    Ok((prophoto_to_table, table_to_prophoto))
}

fn apply_rgb_table(
    table: &RgbTable,
    amount: f64,
    prophoto_to_table: &Matrix,
    table_to_prophoto: &Matrix,
    display_to_prophoto: &Matrix,
    prophoto_to_display: &Matrix,
    display: Rgb,
) -> Rgb {
    // display sRGB → linear ProPhoto → linear table primaries → encode → lookup → back
    let linear_pp = mul(display_to_prophoto, display.map(srgb_decode));
    let linear_table = mul(prophoto_to_table, linear_pp);
    let encoded = encode_gamma(table.gamma, linear_table);
    let mapped = lookup_rgb_3d(table, encoded.map(|v| v.clamp(0.0, 1.0)));
    let blended = apply_amount(encoded, mapped, amount);
    let linear_out = decode_gamma(table.gamma, blended);
    let linear_pp_out = mul(table_to_prophoto, linear_out);
    mul(prophoto_to_display, linear_pp_out).map(srgb_encode)
}

fn encode_gamma(gamma: Gamma, rgb: Rgb) -> Rgb {
    match gamma {
        Gamma::Linear => rgb,
        Gamma::Srgb => rgb.map(|v| srgb_encode(v.clamp(0.0, 1.0))),
        Gamma::Gamma18 => rgb.map(|v| v.clamp(0.0, 1.0).powf(1.0 / 1.8)),
        Gamma::Gamma22 => rgb.map(|v| v.clamp(0.0, 1.0).powf(1.0 / 2.2)),
        Gamma::Rec2020 => rgb.map(rec2020_encode),
    }
}

fn decode_gamma(gamma: Gamma, rgb: Rgb) -> Rgb {
    match gamma {
        Gamma::Linear => rgb,
        Gamma::Srgb => rgb.map(|v| srgb_decode(v.clamp(0.0, 1.0))),
        Gamma::Gamma18 => rgb.map(|v| v.clamp(0.0, 1.0).powf(1.8)),
        Gamma::Gamma22 => rgb.map(|v| v.clamp(0.0, 1.0).powf(2.2)),
        Gamma::Rec2020 => rgb.map(rec2020_decode),
    }
}

fn rec2020_encode(v: f64) -> f64 {
    let v = v.clamp(0.0, 1.0);
    const A: f64 = 1.099_296_826_809_44;
    const B: f64 = 0.018_053_968_510_807;
    if v < B {
        4.5 * v
    } else {
        A * v.powf(0.45) - (A - 1.0)
    }
}

fn rec2020_decode(v: f64) -> f64 {
    let v = v.clamp(0.0, 1.0);
    const A: f64 = 1.099_296_826_809_44;
    const B: f64 = 0.018_053_968_510_807;
    if v < 4.5 * B {
        v / 4.5
    } else {
        ((v + (A - 1.0)) / A).powf(1.0 / 0.45)
    }
}

fn identity_matrix() -> Matrix {
    [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
}

/// Display sRGB (D65) → linear ProPhoto RGB (D50) via Bradford.
fn display_srgb_to_prophoto() -> Matrix {
    // Same path as other D65 spaces; uses the shared Bradford helper.
    d65_space_to_prophoto([[0.64, 0.33], [0.30, 0.60], [0.15, 0.06]])
}

/// Linear RGB of a D65 primary set → linear ProPhoto (D50), Bradford chromatic adaptation.
fn d65_space_to_prophoto(primaries: [[f64; 2]; 3]) -> Matrix {
    let adapt = bradford_d65_to_d50();
    let space = rgb_to_xyz(&primaries, [0.3127, 0.3290]);
    let prophoto = rgb_to_xyz(
        &[[0.7347, 0.2653], [0.1596, 0.8404], [0.0366, 0.0001]],
        [0.3457, 0.3585],
    );
    product(&inverse(&prophoto).unwrap(), &product(&adapt, &space))
}

fn bradford_d65_to_d50() -> Matrix {
    let bradford: Matrix = [
        [0.8951, 0.2664, -0.1614],
        [-0.7502, 1.7135, 0.0367],
        [0.0389, -0.0685, 1.0296],
    ];
    let d65 = [0.95047, 1.0, 1.08883];
    let d50 = [0.96422, 1.0, 0.82521];
    let (from, to) = (mul(&bradford, d65), mul(&bradford, d50));
    let gain: Matrix = std::array::from_fn(|r| {
        std::array::from_fn(|c| if r == c { to[r] / from[r] } else { 0.0 })
    });
    product(&inverse(&bradford).unwrap(), &product(&gain, &bradford))
}

fn rgb_to_xyz(primaries: &[[f64; 2]; 3], whitepoint: [f64; 2]) -> Matrix {
    let white = |xy: [f64; 2]| [xy[0] / xy[1], 1.0, (1.0 - xy[0] - xy[1]) / xy[1]];
    let columns: Matrix = std::array::from_fn(|r| std::array::from_fn(|c| white(primaries[c])[r]));
    let scale = mul(&inverse(&columns).unwrap(), white(whitepoint));
    std::array::from_fn(|r| std::array::from_fn(|c| columns[r][c] * scale[c]))
}

fn missing_named_look_profile(
    preset: &Preset,
    has_look_table: bool,
    has_rgb_table: bool,
    has_look_curves: bool,
) -> Option<String> {
    let look = preset.look.as_ref()?;
    let name = look.name.as_deref()?;
    if has_look_table || has_rgb_table || has_look_curves {
        return None;
    }
    Some(name.to_owned())
}

fn look_has_unread_color_settings(preset: &Preset) -> bool {
    let Some(look) = &preset.look else {
        return false;
    };
    const ALLOWED: &[&str] = &[
        "LookTable",
        "RGBTable",
        "RGBTableAmount",
        "Amount",
        "Name",
        "ToneCurvePV2012",
        "ToneCurvePV2012Red",
        "ToneCurvePV2012Green",
        "ToneCurvePV2012Blue",
        "ConvertToGrayscale",
        "ProcessVersion",
        "Version",
        "UUID",
        "Group",
        "Copyright",
        "Parameters",
        "CompatibleVersion",
        "SupportsAmount",
        "SupportsMonochrome",
        "SupportsOutputReferred",
        "CameraProfile",
        "LookName",
        "Cluster",
    ];
    look.settings.iter().any(|(key, setting)| {
        if ALLOWED.contains(&key.as_str()) {
            return false;
        }
        if key.eq_ignore_ascii_case("uuid") || key.ends_with("UUID") {
            return false;
        }
        match setting {
            Setting::Text(v) => {
                if v.is_empty() || v == "0" || v.eq_ignore_ascii_case("false") {
                    return false;
                }
                // A color setting is a number. Text such as `Stubbed="true"` describes the
                // profile and changes no color.
                v.parse::<f64>().is_ok_and(|n| n.is_finite() && n != 0.0)
            }
            Setting::Seq(v) => !v.is_empty(),
            Setting::Map(v) => !v.is_empty(),
        }
    })
}

// --- HSV / RGBTone / helpers -----------------------------------------------

/// Triangle weights that reach zero at neighbouring band centres (partition of unity).
pub fn hue_weights(hue_deg: f64) -> [f64; 8] {
    let mut w = [0.0f64; 8];
    for i in 0..8 {
        let center = HUE_BANDS[i].1;
        let prev = HUE_BANDS[(i + 7) % 8].1;
        let next = HUE_BANDS[(i + 1) % 8].1;
        let d = signed_angle(hue_deg, center);
        let weight = if d >= 0.0 {
            let span = angle_dist(center, next);
            if span <= 1e-12 {
                0.0
            } else {
                (1.0 - d / span).max(0.0)
            }
        } else {
            let span = angle_dist(center, prev);
            if span <= 1e-12 {
                0.0
            } else {
                (1.0 + d / span).max(0.0)
            }
        };
        w[i] = weight;
    }
    w
}

fn rgb_tone(rgb: Rgb, curve: impl Fn(f64) -> f64) -> Rgb {
    let pin = |v: f64| v.clamp(0.0, 1.0);
    let (r, g, b) = (pin(rgb[0]), pin(rgb[1]), pin(rgb[2]));
    let tone = |hi: f64, mid: f64, lo: f64| {
        let rr = curve(hi);
        let bb = curve(lo);
        let gg = if (hi - lo).abs() < 1e-12 {
            curve(mid)
        } else {
            bb + (rr - bb) * (mid - lo) / (hi - lo)
        };
        (rr, gg, bb)
    };
    let (rr, gg, bb) = if r >= g {
        if g > b {
            tone(r, g, b)
        } else if b > r {
            let (bb, rr, gg) = tone(b, r, g);
            (rr, gg, bb)
        } else if b > g {
            let (rr, bb, gg) = tone(r, b, g);
            (rr, gg, bb)
        } else {
            (curve(r), curve(g), curve(g))
        }
    } else if r >= b {
        let (gg, rr, bb) = tone(g, r, b);
        (rr, gg, bb)
    } else if b > g {
        let (bb, gg, rr) = tone(b, g, r);
        (rr, gg, bb)
    } else {
        let (gg, bb, rr) = tone(g, b, r);
        (rr, gg, bb)
    };
    [rr, gg, bb]
}

/// Moves the luminance of a color along a curve of display grays and keeps its chromaticity:
/// the gain of the linear luminance applies to each channel.
fn luminance_gain(rgb: Rgb, curve: impl Fn(f64) -> f64) -> Rgb {
    let linear = rgb.map(|v| srgb_decode(v.clamp(0.0, 1.0)));
    let luminance = 0.2126 * linear[0] + 0.7152 * linear[1] + 0.0722 * linear[2];
    if luminance <= 1e-9 {
        return [curve(0.0); 3];
    }
    let gain = srgb_decode(curve(srgb_encode(luminance))) / luminance;
    linear.map(|v| srgb_encode((v * gain).clamp(0.0, 1.0)))
}

/// Provisional: the saturation from which a hue band changes the brightness in full.
const HUE_FULL_SATURATION: f64 = 0.5;

/// How much a hue band changes the brightness of a color with this saturation. A gray has no
/// hue, so a band must not change it, and a weak color must change little.
fn hue_strength(saturation: f64) -> f64 {
    (saturation / HUE_FULL_SATURATION).clamp(0.0, 1.0)
}

/// The display gray with the luminance of a display color. The weights apply to linear light:
/// on encoded values a saturated blue would turn nearly black.
fn gray_of(rgb: Rgb) -> f64 {
    let linear = rgb.map(|v| srgb_decode(v.clamp(0.0, 1.0)));
    srgb_encode(0.2126 * linear[0] + 0.7152 * linear[1] + 0.0722 * linear[2])
}

fn rgb_to_hsv(rgb: Rgb) -> [f64; 3] {
    let (r, g, b) = (rgb[0], rgb[1], rgb[2]);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let v = max;
    let s = if max <= 1e-12 { 0.0 } else { delta / max };
    let h = if delta <= 1e-12 {
        0.0
    } else if max == r {
        ((g - b) / delta).rem_euclid(6.0) / 6.0
    } else if max == g {
        (((b - r) / delta) + 2.0) / 6.0
    } else {
        (((r - g) / delta) + 4.0) / 6.0
    };
    [h.rem_euclid(1.0), s, v]
}

fn hsv_to_rgb(hsv: [f64; 3]) -> Rgb {
    let (h, s, v) = (
        hsv[0].rem_euclid(1.0),
        hsv[1].clamp(0.0, 1.0),
        hsv[2].max(0.0),
    );
    let c = v * s;
    let x = c * (1.0 - ((h * 6.0).rem_euclid(2.0) - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match (h * 6.0).floor() as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    [r + m, g + m, b + m]
}

/// The matrix of the camera calibration sliders: the identity plus what each slider adds at
/// its amount, between the fitted amounts.
fn calibration(preset: &Preset) -> Option<Matrix> {
    let mut matrix = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let mut any = false;
    for (key, fitted) in CALIBRATION_KEYS.iter().zip(&CALIBRATION) {
        let amount = preset.number(key).unwrap_or(0.0).clamp(-100.0, 100.0);
        if amount == 0.0 {
            continue;
        }
        any = true;
        for (row, target) in matrix.iter_mut().enumerate() {
            let at = |part: usize| {
                let points = [
                    (-100.0, fitted[0][row][part]),
                    (-50.0, fitted[1][row][part]),
                    (0.0, 0.0),
                    (50.0, fitted[2][row][part]),
                    (100.0, fitted[3][row][part]),
                ];
                interpolate(&points, amount)
            };
            let (red, blue) = (at(0), at(1));
            target[0] += red;
            target[1] -= red + blue;
            target[2] += blue;
        }
    }
    any.then_some(matrix)
}

/// The scene value of a linear display value under the default rendering (`SCENE_VALUES`).
fn scene_of(display: f64) -> f64 {
    let position = srgb_encode(display.clamp(0.0, 1.0)) * (SCENE_VALUES.len() - 1) as f64;
    let low = (position as usize).min(SCENE_VALUES.len() - 2);
    SCENE_VALUES[low] + (position - low as f64) * (SCENE_VALUES[low + 1] - SCENE_VALUES[low])
}

/// The linear display value of a scene value: `scene_of` inverted.
fn display_of(scene: f64) -> f64 {
    let last = SCENE_VALUES.len() - 1;
    if scene <= 0.0 {
        return 0.0;
    }
    if scene >= SCENE_VALUES[last] {
        return 1.0;
    }
    let high = SCENE_VALUES.partition_point(|v| *v < scene).clamp(1, last);
    let (below, above) = (SCENE_VALUES[high - 1], SCENE_VALUES[high]);
    srgb_decode(((high - 1) as f64 + (scene - below) / (above - below)) / last as f64)
}

/// A piecewise linear function through `points`, held at its ends.
fn interpolate(points: &[(f64, f64)], x: f64) -> f64 {
    let last = points.len() - 1;
    if x <= points[0].0 {
        return points[0].1;
    }
    if x >= points[last].0 {
        return points[last].1;
    }
    let i = (0..last).rfind(|&i| points[i].0 <= x).unwrap_or(0);
    let t = (x - points[i].0) / (points[i + 1].0 - points[i].0);
    points[i].1 + t * (points[i + 1].1 - points[i].1)
}

/// A profile between its tables for Blending 0, 0.5, and 1. Balance can ask for a little less
/// than 0 or more than 1: the nearest pair of tables continues.
fn blended_profile(tables: &[[f64; 11]; 3], blending: f64, x: f64) -> f64 {
    let blending = blending.clamp(-0.5, 1.5);
    let (low, high, t) = if blending <= 0.5 {
        (0, 1, blending / 0.5)
    } else {
        (1, 2, (blending - 0.5) / 0.5)
    };
    let value = profile(&tables[low], x) * (1.0 - t) + profile(&tables[high], x) * t;
    value.max(0.0)
}

/// A color grading profile at an encoded value.
fn profile(knots: &[f64; 11], x: f64) -> f64 {
    let position = x.clamp(0.0, 1.0) * 10.0;
    let low = (position as usize).min(9);
    knots[low] + (position - low as f64) * (knots[low + 1] - knots[low])
}

fn angle_dist(a: f64, b: f64) -> f64 {
    let d = (a - b).abs() % 360.0;
    d.min(360.0 - d)
}

fn signed_angle(from: f64, center: f64) -> f64 {
    let mut d = (from - center) % 360.0;
    if d > 180.0 {
        d -= 360.0;
    }
    if d < -180.0 {
        d += 360.0;
    }
    d
}

fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn identity_curve() -> [f64; CURVE_SAMPLES] {
    std::array::from_fn(|i| i as f64 / (CURVE_SAMPLES - 1) as f64)
}

fn curve_is_identity(curve: &[f64; CURVE_SAMPLES]) -> bool {
    curve
        .iter()
        .enumerate()
        .all(|(i, v)| (*v - i as f64 / (CURVE_SAMPLES - 1) as f64).abs() < 1e-12)
}

fn compose(curve: &mut [f64; CURVE_SAMPLES], f: impl Fn(f64) -> f64) {
    for slot in curve.iter_mut() {
        *slot = f(*slot);
    }
}

fn eval_samples(curve: &[f64; CURVE_SAMPLES], x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0) * (CURVE_SAMPLES - 1) as f64;
    let i = (x as usize).min(CURVE_SAMPLES - 2);
    let t = x - i as f64;
    curve[i] * (1.0 - t) + curve[i + 1] * t
}
