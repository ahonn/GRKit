//! Fit of a target look, such as a `.cube` LUT, to the camera model of GRKit effects.
//!
//! The model and the fitting approach are ported from gr3-film-mods (MIT, see
//! `GR3_FILM_MODS_LICENSE.txt`): `cube_fit.py`, `joint_fit.py`, and `lab_fit.py`, FINDINGS
//! §11-§13 and §22l-§22m. For scene-linear RGB `L`:
//!
//!   code_c = Curve_c(BaseGamma(K (M L)_c)) / 16384,   display = clip(P code, 0, 1)
//!
//! `M` is the effect matrix (rows sum to 1), `Curve_c` the effect curves, and `P` the optional
//! post matrix on gamma-encoded RGB. The display output is compared with sRGB-encoded targets.
//! `BaseGamma` is the still-path Standard base gamma that the caller reads from the user's
//! firmware: 256 entries of 14-bit output over a linear index domain.

mod import;
mod model;
mod sectors;
mod solve;

pub use import::*;
pub use model::*;

use anyhow::{ensure, Context, Result};

use crate::cube::{self, delta_e2000, LabConverter, Rgb};
use crate::hue;

use cube::rows;
use model::LUMA;
use sectors::{SectorCoefficients, SECTOR_GAIN, VALIDATION_COLORS};
use solve::{fit_curve, minimize};

/// The scene-linear level, as a multiple of diffuse white, beyond which the lightness bound of
/// the fit and `flaws` do not look: the Standard base gamma reaches 98% of its top there, and
/// the sensor clips about there.
pub(super) const SENSOR_HEADROOM: f64 = 3.0;

const GRAY_WEIGHT: f64 = 150.0;
const CHART_WEIGHT: f64 = 20.0;
const SKIN_WEIGHT: f64 = 60.0;
/// Weight of the post-matrix row sums relative to the root of the total sample weight.
const NEUTRAL_WEIGHT: f64 = 300.0;
/// Weight of a CSP coefficient beyond the limit, in the units of `NEUTRAL_WEIGHT`.
const CSP_WEIGHT: f64 = 300.0;
/// The largest effect matrix coefficient that the fit uses. Of 329 third-party LUTs, 99% fit
/// with coefficients up to 1.5. A larger coefficient amplifies sensor noise in the colors, and
/// the camera's matrix stage is not known to handle one.
pub const MATRIX_LIMIT: f64 = 2.0;
/// Pulls the post matrix toward the identity, relative to the root of the total sample weight,
/// in the Lab units of the post fit. It only fixes directions that the samples barely
/// determine: a damping of 1.0, or a damping of the effect matrix, made strong looks worse.
const POST_DAMPING: f64 = 0.1;
/// A target is toned monochrome when its color at each output level varies by at most this.
pub const TONED_TOLERANCE: f64 = 0.02;
/// A target is monochrome when no channel differs from another by more than this.
pub const MONOCHROME_TOLERANCE: f64 = 0.01;

/// Fit controls. `Default` is the production fit; tests may use fewer samples or rounds.
#[derive(Clone, Debug)]
pub struct Options {
    pub post: bool,
    pub random_samples: usize,
    /// Alternations of curves and matrix before the post matrix.
    pub rounds: usize,
    /// Alternations of curves, matrix, and post matrix after the first post fit.
    pub joint_rounds: usize,
    /// Fit hue sectors for the camera's MultiAxial stage (`hue`), for color effects. The fit
    /// keeps them only if they improve colors that it does not see.
    pub sectors: bool,
    /// Fit separate sector matrices for the stage's luminance levels (`hue::LEVELS`).
    pub levels: bool,
    /// Alternations of sectors, weight curve, matrix, and curves with the post matrix.
    pub sector_rounds: usize,
    /// Samples for the sector stage: a stride through the colored samples.
    pub sector_samples: usize,
    /// Passes of the final joint refinement of the matrix, the post matrix, and the sectors,
    /// for color effects with a post matrix.
    pub joint_passes: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            post: true,
            random_samples: 6000,
            rounds: 8,
            joint_rounds: 6,
            sectors: true,
            levels: true,
            sector_rounds: 5,
            sector_samples: 3000,
            joint_passes: 2,
        }
    }
}

/// A quantized fit and its CIEDE2000 errors on the ColorChecker at 0 EV.
#[derive(Clone, Debug)]
pub struct Fit {
    pub matrix: [i16; 9],
    pub curves: [[u16; 256]; 3],
    pub post_matrix: Option<[i16; 9]>,
    pub mean_de: f64,
    pub max_de: f64,
    pub standard_mean_de: f64,
    /// The largest CSP coefficient magnitude, 1/256 units, at hue 0.
    pub csp_max: i32,
    /// The target is gray, so the fit has equal matrix rows, one curve, and no post matrix.
    pub monochrome: bool,
    /// The target is a toned monochrome, such as sepia: equal matrix rows, three curves, and
    /// no post matrix. It is not a MONOCHROME effect, because the curves differ.
    pub toned: bool,
    /// Mean and 95th percentile CIEDE2000 on colors that the fit does not see (`holdout`).
    pub holdout_mean_de: f64,
    pub holdout_p95_de: f64,
    /// The MultiAxial stage, for a color effect whose fit it improves.
    pub sectors: Option<hue::HueSectors>,
}

impl Fit {
    pub fn params(&self) -> Params {
        let unit = |m: [i16; 9]| m.map(|v| v as f64 / 8192.0);
        Params {
            native: None,
            matrix: unit(self.matrix),
            curves: self.curves.map(|c| c.map(f64::from)),
            post: self.post_matrix.map_or(IDENTITY, unit),
            sectors: self.sectors.map(|s| s.stage()),
        }
    }
}

/// Colors that the fit does not see, for an error that the fit cannot overfit: CIE L*a*b*
/// with lightness 10 to 100, any hue, and chroma up to 70 biased toward low values as in
/// natural scenes, within sRGB, at -1 to +1 EV.
pub fn holdout() -> Vec<Rgb> {
    natural_colors(2, HOLDOUT_COLORS)
}

/// Natural colors as `holdout` draws them, from the generator seed `seed`.
fn natural_colors(seed: u64, count: usize) -> Vec<Rgb> {
    let mut rng = Random(seed);
    let mut colors = Vec::with_capacity(count);
    while colors.len() < count {
        let l = 10.0 + 90.0 * rng.next();
        let chroma = 70.0 * rng.next().powi(2);
        let hue = std::f64::consts::TAU * rng.next();
        let exposure = 2f64.powf(2.0 * rng.next() - 1.0);
        let linear = cube::lab_d50_to_srgb([l, chroma * hue.cos(), chroma * hue.sin()]);
        if linear.iter().all(|v| (0.0..=1.0).contains(v)) {
            colors.push(linear.map(|v| v * exposure));
        }
    }
    colors
}

const HOLDOUT_COLORS: usize = 3000;

/// SplitMix64, so the random samples are deterministic on every host.
struct Random(u64);

impl Random {
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        ((z ^ (z >> 31)) >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// The largest chroma (CIE L*a*b* D50) of the random samples.
const SAMPLE_CHROMA: f64 = 90.0;

/// The samples: a gray ramp and the chart over -2..+2 EV, as in cube_fit.py, and random scene
/// colors.
///
/// The random colors follow natural scenes: any lightness and hue, with chroma biased toward low
/// values, within sRGB, at -2 to +2 EV. Most pixels of a photo have low or medium chroma.
/// Random RGB channels, as cube_fit.py draws them, are mostly highly saturated colors. A fit to
/// them traded medium chroma for high chroma: on 39 photos, the film looks lost 11 to 18% of the
/// chroma of their LUT. With these samples they lose 5 to 12%. A 3 x 3 matrix scales all
/// chroma alike, so it cannot follow a LUT that adds more chroma to muted colors than to vivid ones.
fn samples(random: usize) -> (Vec<Rgb>, Vec<f64>) {
    let mut linear = Vec::new();
    let mut weights = Vec::new();
    for i in 0..600 {
        let v = 0.18 * 2f64.powf(-7.0 + 11.5 * i as f64 / 599.0);
        linear.push([v; 3]);
        weights.push(GRAY_WEIGHT);
    }
    let chart = colorchecker();
    for ev in -2..=2 {
        for (patch, rgb) in chart.iter().enumerate() {
            linear.push(rgb.map(|v| v * 2f64.powi(ev)));
            weights.push(if patch < 2 { SKIN_WEIGHT } else { CHART_WEIGHT });
        }
    }
    let mut rng = Random(1);
    while weights.len() < 600 + 5 * chart.len() + random {
        let l = 10.0 + 90.0 * rng.next();
        let chroma = SAMPLE_CHROMA * rng.next().powi(2);
        let hue = std::f64::consts::TAU * rng.next();
        let exposure = 2f64.powf(4.0 * rng.next() - 2.0);
        let rgb = cube::lab_d50_to_srgb([l, chroma * hue.cos(), chroma * hue.sin()]);
        if rgb.iter().all(|v| (0.0..=1.0).contains(v)) {
            linear.push(rgb.map(|v| v * exposure));
            weights.push(1.0);
        }
    }
    (linear, weights)
}

/// A matrix whose rows sum to 1 from its six off-diagonal coefficients, or from the green and
/// blue weights of one row that all three rows share.
fn matrix_from(offsets: &[f64]) -> [f64; 9] {
    if let [green, blue] = *offsets {
        let row = [1.0 - green - blue, green, blue];
        return std::array::from_fn(|n| row[n % 3]);
    }
    let mut m = [0.0; 9];
    for r in 0..3 {
        let others = [(r + 1) % 3, (r + 2) % 3];
        for (k, c) in others.into_iter().enumerate() {
            m[r * 3 + c] = offsets[r * 2 + k];
        }
        m[r * 4] = 1.0 - offsets[r * 2] - offsets[r * 2 + 1];
    }
    m
}

/// The shape of the fitted effect.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Shape {
    Color,
    /// Equal matrix rows and three curves.
    Toned,
    /// Equal matrix rows and one curve.
    Monochrome,
}

/// Whether each output level of the target has one color: true for gray and toned looks.
fn toned(targets: &[Rgb]) -> bool {
    const BINS: usize = 64;
    let level = |t: &Rgb| t.iter().sum::<f64>() / 3.0;
    let bin = |t: &Rgb| ((level(t) * BINS as f64) as usize).min(BINS - 1);
    let mut sums = vec![([0.0; 3], 0usize); BINS];
    for t in targets {
        let (sum, count) = &mut sums[bin(t)];
        for c in 0..3 {
            sum[c] += t[c] - level(t);
        }
        *count += 1;
    }
    targets.iter().all(|t| {
        let (sum, count) = sums[bin(t)];
        (0..3).all(|c| (t[c] - level(t) - sum[c] / count as f64).abs() <= TONED_TOLERANCE)
    })
}

/// How far `v` exceeds `limit` in magnitude, the argument of each bound penalty.
fn excess(v: f64, limit: f64) -> f64 {
    (v.abs() - limit).max(0.0)
}

/// The penalties of a post matrix `p`: its row sums, which keep neutrals neutral, its CSP
/// coefficients beyond `CSP_LIMIT`, and a pull toward the identity. Directions that the
/// samples do not determine, such as the mixing of equal channels, would otherwise drift and
/// amplify noise in the colors.
fn post_penalties(p: &[f64], sums: f64, csp: f64, damping: f64, out: &mut Vec<f64>) {
    out.extend((0..3).map(|r| sums * (p[r * 3..r * 3 + 3].iter().sum::<f64>() - 1.0)));
    let limit = CSP_LIMIT as f64 / 256.0;
    let coefficients = cube::product(&bt601(), &rows(p.try_into().unwrap()));
    out.extend(
        coefficients
            .iter()
            .flatten()
            .map(|v| csp * excess(*v, limit)),
    );
    out.extend(p.iter().zip(IDENTITY).map(|(v, i)| damping * (v - i)));
}

/// The unquantized coefficients of one stage of the fit.
#[derive(Clone, Debug)]
struct Candidate {
    offsets: Vec<f64>,
    curves: [[f64; 256]; 3],
    post: [f64; 9],
    sectors: Option<SectorCoefficients>,
}

impl Candidate {
    fn plain(offsets: Vec<f64>, curves: [[f64; 256]; 3], post: [f64; 9]) -> Self {
        Self {
            offsets,
            curves,
            post,
            sectors: None,
        }
    }

    /// The parameters with the sectors as the camera rounds them.
    fn quantized(&self) -> Params {
        Params {
            native: None,
            matrix: matrix_from(&self.offsets),
            curves: self.curves,
            post: self.post,
            sectors: self.sectors.as_ref().map(|s| s.quantize().stage()),
        }
    }
}

struct Problem {
    model: Model,
    linear: Vec<Rgb>,
    roots: Vec<f64>,
    targets: Vec<Rgb>,
    labs: Vec<[f64; 3]>,
    converter: LabConverter,
    neutral: f64,
    /// The root of the total sample weight, the scale of the damping terms.
    scale: f64,
    shape: Shape,
    /// The exposure ramps of the lightness bound in the sector stages.
    ramps: sectors::Ramps,
}

impl Problem {
    fn curves(
        &self,
        matrix: &[f64; 9],
        post: &[f64; 9],
        sectors: Option<&hue::Stage>,
    ) -> [[f64; 256]; 3] {
        // joint_fit.py: fit the curves to the target pulled back through the chroma
        // suppression and the post matrix.
        let inverse = cube::inverse(&rows(post)).unwrap_or(rows(&IDENTITY));
        let pulled: Vec<Rgb> = self
            .targets
            .iter()
            .map(|t| cube::mul(&inverse, undisplay(*t)).map(|v| v.clamp(0.0, 1.0)))
            .collect();
        let mixed: Vec<Rgb> = self
            .linear
            .iter()
            .map(|l| {
                let mixed = cube::mul(&rows(matrix), *l);
                sectors
                    .map_or(mixed, |s| s.apply(mixed))
                    .map(|v| self.model.base_gamma(v))
            })
            .collect();
        if self.shape == Shape::Monochrome {
            // The rows are equal, so each channel has the same level. Fit the mean target.
            let curve = fit_curve((0..self.linear.len()).map(|i| {
                let gray = pulled[i].iter().sum::<f64>() / 3.0;
                (mixed[i][0], gray, self.roots[i].powi(2))
            }));
            return [curve; 3];
        }
        std::array::from_fn(|c| {
            fit_curve(
                (0..self.linear.len()).map(|i| (mixed[i][c], pulled[i][c], self.roots[i].powi(2))),
            )
        })
    }

    fn matrix(&self, offsets: &[f64], curves: &[[f64; 256]; 3], post: &[f64; 9]) -> Vec<f64> {
        minimize(offsets, 30, &|p, out| {
            let params = Params {
                native: None,
                matrix: matrix_from(p),
                curves: *curves,
                post: *post,
                sectors: None,
            };
            out.clear();
            for ((l, t), w) in self.linear.iter().zip(&self.targets).zip(&self.roots) {
                let d = self.model.render(&params, *l);
                out.extend((0..3).map(|c| w * (d[c] - t[c])));
            }
            // A look that the model cannot follow, such as infrared false color, drives the
            // coefficients without bound. Keep them within the limit.
            out.extend(
                params
                    .matrix
                    .iter()
                    .map(|v| self.neutral * excess(*v, MATRIX_LIMIT)),
            );
        })
    }

    /// The mean CIEDE2000 of the samples, one measure for the stages, whose own objectives
    /// differ (RGB for the curves and the matrix, Lab for the post matrix). Each sample counts
    /// once: the fit weights would make the gray ramp decide.
    fn score(
        &self,
        offsets: &[f64],
        curves: &[[f64; 256]; 3],
        post: &[f64; 9],
        sectors: Option<&hue::Stage>,
    ) -> f64 {
        let params = Params {
            native: None,
            matrix: matrix_from(offsets),
            curves: *curves,
            post: *post,
            sectors: sectors.cloned(),
        };
        let sum: f64 = self
            .linear
            .iter()
            .zip(&self.labs)
            .map(|(l, lab)| delta_e2000(self.converter.lab(self.model.render(&params, *l)), *lab))
            .sum();
        sum / self.linear.len() as f64
    }

    /// Lab residuals, the neutral penalty on the row sums, and the CSP penalty.
    fn post(
        &self,
        matrix: &[f64; 9],
        curves: &[[f64; 256]; 3],
        start: &[f64; 9],
        sectors: Option<&hue::Stage>,
    ) -> [f64; 9] {
        let params = Params {
            native: None,
            matrix: *matrix,
            curves: *curves,
            post: IDENTITY,
            sectors: sectors.cloned(),
        };
        let codes: Vec<Rgb> = self
            .linear
            .iter()
            .map(|l| self.model.codes(&params, *l))
            .collect();
        let result = minimize(start, 30, &|p, out| {
            let post = rows(p.try_into().unwrap());
            out.clear();
            for ((code, lab), w) in codes.iter().zip(&self.labs).zip(&self.roots) {
                // The camera suppresses chroma and clips the output as `render` does, so
                // a post matrix that pushes colors out of range must pay for it here.
                let fitted = self.converter.lab(display(cube::mul(&post, *code)));
                out.extend((0..3).map(|c| w * (fitted[c] - lab[c])));
            }
            post_penalties(
                p,
                self.neutral,
                CSP_WEIGHT / NEUTRAL_WEIGHT * self.neutral,
                POST_DAMPING * self.scale,
                out,
            );
        });
        result.try_into().unwrap()
    }
}

const MATRIX_RANGE: &str =
    "The LUT mixes the color channels more than the camera matrix can represent";

/// Rounds a matrix to Q13 with each row summing to 8192.
fn quantize(m: &[f64; 9]) -> Result<[i16; 9]> {
    let mut result = [0; 9];
    for r in 0..3 {
        let mut values: Vec<i64> = m[r * 3..r * 3 + 3]
            .iter()
            .map(|v| (v * 8192.0).round_ties_even() as i64)
            .collect();
        values[r] += 8192 - values.iter().sum::<i64>();
        for (c, value) in values.into_iter().enumerate() {
            result[r * 3 + c] = i16::try_from(value).ok().context(MATRIX_RANGE)?;
        }
    }
    Ok(result)
}

/// Quantizes the shared row of a monochrome or toned matrix, so that the rows stay equal.
fn quantize_monochrome(m: &[f64; 9]) -> Result<[i16; 9]> {
    let mut row = [0i64; 3];
    for (slot, v) in row.iter_mut().zip(&m[..3]) {
        *slot = (v * 8192.0).round_ties_even() as i64;
    }
    // Put the rounding remainder on the largest weight, where it changes the least.
    let largest = (0..3).max_by_key(|&c| row[c]).unwrap();
    row[largest] += 8192 - row.iter().sum::<i64>();
    let mut result = [0; 9];
    for (n, slot) in result.iter_mut().enumerate() {
        *slot = i16::try_from(row[n % 3]).ok().context(MATRIX_RANGE)?;
    }
    Ok(result)
}

/// The largest |BT601 x P| coefficient with the rounding of `picture_post` in the runtime.
pub fn csp_max(post: &[i16; 9]) -> i32 {
    (0..9)
        .map(|n| {
            let (i, j) = (n / 3, n % 3);
            let sum: i32 = (0..3).map(|k| BT601[i][k] * post[k * 3 + j] as i32).sum();
            ((sum + 4096) >> 13).abs()
        })
        .max()
        .unwrap()
}

/// Quantizes `post` and, if its CSP exceeds the limit, moves it toward the identity until it fits.
pub fn constrain(post: &[f64; 9]) -> Result<[i16; 9]> {
    let toward = |s: f64| -> Result<[i16; 9]> {
        quantize(&std::array::from_fn(|n| {
            IDENTITY[n] + s * (post[n] - IDENTITY[n])
        }))
    };
    let fits = |s: f64| toward(s).is_ok_and(|q| csp_max(&q) <= CSP_LIMIT);
    if fits(1.0) {
        return toward(1.0);
    }
    let (mut low, mut high) = (0.0, 1.0);
    for _ in 0..40 {
        let middle = (low + high) / 2.0;
        if fits(middle) {
            low = middle;
        } else {
            high = middle;
        }
    }
    toward(low)
}

/// Receives the progress of a fit or an import: a fraction from 0 to 1 that does not decrease.
pub type Progress<'a> = &'a dyn Fn(f64);

/// The part `from..to` of `progress`, for a stage that reports its own fraction.
fn span<'a>(progress: Progress<'a>, from: f64, to: f64) -> impl Fn(f64) + 'a {
    move |fraction| progress(from + (to - from) * fraction.clamp(0.0, 1.0))
}

/// The progress at the end of each stage of `fit`, in proportion to the time of a color LUT:
/// the samples, the matrix and curve rounds, the post matrix rounds, the validation colors, the
/// hue sectors, the joint refinement, and the errors.
const STAGE_ENDS: [f64; 6] = [0.01, 0.02, 0.05, 0.06, 0.75, 0.99];

/// Fits `target`, a map from scene-linear RGB to sRGB-encoded display RGB.
pub fn fit(base_gamma: &[u16; 256], target: &dyn Fn(Rgb) -> Rgb, options: &Options) -> Result<Fit> {
    fit_with_progress(base_gamma, target, options, &|_| {})
}

/// `fit`, which reports its progress to `progress`.
pub fn fit_with_progress(
    base_gamma: &[u16; 256],
    target: &dyn Fn(Rgb) -> Rgb,
    options: &Options,
    progress: Progress,
) -> Result<Fit> {
    let [setup, rounds, posts, validated, sectored, joined] = STAGE_ENDS;
    let (linear, weights) = samples(options.random_samples);
    let targets: Vec<Rgb> = linear
        .iter()
        .map(|l| target(*l).map(|v| v.clamp(0.0, 1.0)))
        .collect();
    ensure!(
        targets.iter().flatten().all(|v| v.is_finite()),
        "The target produced non-finite values"
    );
    let ramp: Vec<f64> = targets
        .iter()
        .zip(&linear)
        .filter(|(_, l)| l[0] == l[1] && l[1] == l[2])
        .map(|(t, _)| t.iter().sum::<f64>() / 3.0)
        .collect();
    // The curves cannot decrease, so they cannot follow a negative.
    ensure!(
        ramp.first() <= ramp.last(),
        "The LUT inverts brightness, which the camera curves cannot represent"
    );
    let monochrome = targets
        .iter()
        .all(|t| t.iter().fold(0.0f64, |m, v| m.max((v - t[0]).abs())) <= MONOCHROME_TOLERANCE);
    let shape = if monochrome {
        Shape::Monochrome
    } else if toned(&targets) {
        Shape::Toned
    } else {
        Shape::Color
    };
    let converter = LabConverter::default();
    let scale = weights.iter().sum::<f64>().sqrt();
    let problem = Problem {
        model: Model::new(base_gamma),
        labs: targets.iter().map(|t| converter.lab(*t)).collect(),
        roots: weights.iter().map(|w| w.sqrt()).collect(),
        neutral: NEUTRAL_WEIGHT * scale,
        scale,
        converter,
        linear,
        targets,
        shape,
        ramps: sectors::Ramps::new(target, &LabConverter::default()),
    };
    progress(setup);
    let shared_rows = shape != Shape::Color;
    let mut offsets = if shared_rows {
        vec![LUMA[1], LUMA[2]]
    } else {
        vec![0.0; 6]
    };
    let mut post = IDENTITY;
    let round_progress = span(progress, setup, rounds);
    for round in 0..options.rounds {
        let curves = problem.curves(&matrix_from(&offsets), &post, None);
        offsets = problem.matrix(&offsets, &curves, &post);
        round_progress((round + 1) as f64 / options.rounds as f64);
    }
    let mut curves = problem.curves(&matrix_from(&offsets), &post, None);
    if options.post && !shared_rows {
        // The stages alternate between different objectives, so a round can make the result
        // worse, and for a look that the model cannot follow the rounds can diverge. Keep the
        // best result of all stages.
        let mut best = (
            problem.score(&offsets, &curves, &post, None),
            Candidate::plain(offsets.clone(), curves, post),
        );
        let mut keep = |offsets: &[f64], curves: &[[f64; 256]; 3], post: &[f64; 9]| {
            let score = problem.score(offsets, curves, post, None);
            if score < best.0 {
                best = (score, Candidate::plain(offsets.to_vec(), *curves, *post));
            }
        };
        post = problem.post(&matrix_from(&offsets), &curves, &post, None);
        keep(&offsets, &curves, &post);
        let post_progress = span(progress, rounds, posts);
        for round in 0..options.joint_rounds {
            curves = problem.curves(&matrix_from(&offsets), &post, None);
            offsets = problem.matrix(&offsets, &curves, &post);
            curves = problem.curves(&matrix_from(&offsets), &post, None);
            post = problem.post(&matrix_from(&offsets), &curves, &post, None);
            keep(&offsets, &curves, &post);
            post_progress((round + 1) as f64 / options.joint_rounds as f64);
        }
        Candidate {
            offsets,
            curves,
            post,
            ..
        } = best.1;
    }
    let mut current = Candidate::plain(offsets, curves, post);
    // Colors that the fit does not see, which a later stage must improve to be kept.
    let validation: Vec<(Rgb, [f64; 3])> = if shape == Shape::Color {
        natural_colors(3, VALIDATION_COLORS)
            .into_iter()
            .map(|l| {
                (
                    l,
                    problem.converter.lab(target(l).map(|v| v.clamp(0.0, 1.0))),
                )
            })
            .collect()
    } else {
        Vec::new()
    };
    let validation_error = |params: &Params| {
        validation
            .iter()
            .map(|(l, lab)| {
                delta_e2000(
                    problem.converter.lab(problem.model.render(params, *l)),
                    *lab,
                )
            })
            .sum::<f64>()
            / validation.len() as f64
    };
    progress(validated);
    // The hue sectors of the camera's MultiAxial stage, kept only if they improve the
    // validation colors by SECTOR_GAIN.
    if options.sectors && shape == Shape::Color {
        let sector_progress = span(progress, validated, sectored);
        if let Some(found) = problem.hue_sectors(&current, options, &sector_progress) {
            let before = validation_error(&current.quantized());
            let after = validation_error(&found.quantized());
            if after < before - SECTOR_GAIN.max(SECTOR_GAIN * before) {
                current = found;
            }
        }
    }
    // The stages above alternate between objectives. A joint refinement on the measure of
    // `score` finds a better result for most looks; keep it only if it improves the
    // validation colors.
    if options.joint_passes > 0 && options.post && shape == Shape::Color {
        let joint = problem.joint(&current, options, &span(progress, sectored, joined));
        if validation_error(&joint.quantized()) < validation_error(&current.quantized()) {
            current = joint;
        }
    }
    progress(joined);
    let Candidate {
        offsets,
        curves,
        post,
        sectors,
    } = current;
    let sectors = sectors.as_ref().map(SectorCoefficients::quantize);
    let identity = quantize(&IDENTITY)?;
    let post_matrix = Some(constrain(&post)?).filter(|p| *p != identity);
    let mut fit = Fit {
        matrix: if shared_rows {
            quantize_monochrome(&matrix_from(&offsets))?
        } else {
            quantize(&matrix_from(&offsets))?
        },
        curves: curves.map(|c| c.map(|v| v.round() as u16)),
        post_matrix,
        mean_de: 0.0,
        max_de: 0.0,
        standard_mean_de: 0.0,
        csp_max: csp_max(&post_matrix.unwrap_or(identity)),
        monochrome,
        toned: shape == Shape::Toned,
        holdout_mean_de: 0.0,
        holdout_p95_de: 0.0,
        sectors,
    };
    let params = fit.params();
    let model = &problem.model;
    let lab = |rgb: Rgb| problem.converter.lab(rgb);
    let chart = colorchecker();
    let (mut sum, mut standard_sum) = (0.0, 0.0);
    for patch in chart {
        let expected = lab(target(patch).map(|v| v.clamp(0.0, 1.0)));
        let de = delta_e2000(lab(model.render(&params, patch)), expected);
        sum += de;
        fit.max_de = fit.max_de.max(de);
        standard_sum += delta_e2000(lab(model.standard(patch)), expected);
    }
    fit.mean_de = sum / chart.len() as f64;
    fit.standard_mean_de = standard_sum / chart.len() as f64;
    let mut errors: Vec<f64> = holdout()
        .into_iter()
        .map(|l| {
            let expected = lab(target(l).map(|v| v.clamp(0.0, 1.0)));
            delta_e2000(lab(model.render(&params, l)), expected)
        })
        .collect();
    errors.sort_by(f64::total_cmp);
    fit.holdout_mean_de = errors.iter().sum::<f64>() / errors.len() as f64;
    fit.holdout_p95_de = errors[errors.len() * 95 / 100];
    progress(1.0);
    Ok(fit)
}
