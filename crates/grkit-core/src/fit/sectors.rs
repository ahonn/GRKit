//! Hue sectors of the camera's MultiAxial stage: the sector rounds and the joint refinement.

use crate::cube::{self, delta_e2000, Matrix, Rgb};
use crate::hue;

use super::solve::{minimize, minimize_with, STEP};
use super::*;

/// The weight of the step between neighbouring sectors at their boundary: their matrices must
/// agree on the colors there, so smooth hue gradients show no edge.
const SEAM: f64 = 32.0;

/// The largest sector coefficient magnitude, within the stored i8 in 1/128.
const SECTOR_LIMIT: f64 = 0.98;
/// The gain on the validation colors that hue sectors must bring, in CIEDE2000 and as a part.
pub(super) const SECTOR_GAIN: f64 = 0.02;
pub(super) const VALIDATION_COLORS: usize = 1500;
/// The start of the sector weights. On 14 color LUTs, a start of 0.25 gave 5% less error on
/// photos than 0.98: from a high weight, the rounds stop in a worse minimum.
const SECTOR_WEIGHT_START: f64 = 0.25;
/// The weight of a bound (matrix limit, sector limit, weight range) in the stages with one
/// CIEDE2000 residual per sample, per root of the sample count.
const BOUND_WEIGHT: f64 = 100.0;
/// The largest change of the sector weight over the first segment of the curve. With
/// highlight correction the installer doubles the curve's slopes (`hue::HueSectors::
/// slopes_double`); this keeps the first slope within its 9-bit field after rounding.
const WEIGHT_STEP: f64 = 0.62;
/// The weight of the post matrix row sums and the CSP limit in the joint refinement.
const EXACT_WEIGHT: f64 = 1000.0;
/// Samples and iterations of each joint refinement pass.
const JOINT_SAMPLES: usize = 3000;
const JOINT_ITERATIONS: usize = 15;
/// Iterations of the sector solve in each round of `hue_sectors`.
const SECTOR_ITERATIONS: usize = 25;

/// The chroma of the seam colors, as Cb/Cr distance over Y in the stage's YCbCr. Ratios of 2
/// to 3 reach the edge of the gamut, where saturated blues and purples showed steps of up to
/// 24 dE between neighbouring hues when the seams held only at 0.3 and 0.6.
const SEAM_CHROMA: [f64; 5] = [0.3, 0.6, 1.2, 2.0, 3.0];

/// Colors on each sector boundary at the `SEAM_CHROMA` levels that have no negative channel,
/// with the sectors on either side. With levels, the luminances cover each level's range of
/// `hue::LEVEL_CURVE`.
fn seam_colors(levels: usize) -> Vec<(usize, usize, Rgb)> {
    let stage: Matrix = hue::STAGE.map(|r| r.map(|v| v / 256.0));
    let inverse = cube::inverse(&stage).expect("the stage matrix is invertible");
    // Above 0.75 the colors are brighter than diffuse white; saturated blues and purples at
    // +1 to +2 EV showed steps of up to 16 dE between neighbouring hues without them.
    let luminances: &[f64] = if levels > 1 {
        &[0.03, 0.06, 0.1, 0.15, 0.22, 0.33, 0.5, 0.75, 1.0, 1.5, 2.0]
    } else {
        &[0.05, 0.18, 0.5]
    };
    let mut out = Vec::new();
    for k in 0..hue::SECTORS {
        let (cb, cr) = hue::direction(8192.0 * k as f64 / hue::SECTORS as f64);
        for &y in luminances {
            for c in SEAM_CHROMA {
                let color = cube::mul(&inverse, [y, c * y * cb, c * y * cr]);
                if color.iter().all(|v| *v >= 0.0) {
                    out.push(((k + hue::SECTORS - 1) % hue::SECTORS, k, color));
                }
            }
        }
    }
    out
}

/// Coefficients of one level of sectors.
const LEVEL_COEFFICIENTS: usize = hue::SECTORS * 6;

/// The levels of a sector vector: one, or `hue::LEVELS`.
fn level_count(sectors: &[f64]) -> usize {
    sectors.len() / LEVEL_COEFFICIENTS
}

/// The matrix of sector `k` at stage luminance `y` in a sector vector.
/// It sums the level matrices by their shares, the same shares that decide the Jacobian
/// groups. `hue::Stage::matrix` blends the same two levels; the two differ only in rounding,
/// and the fit keeps this form so that its results stay reproducible.
fn sector_matrix(sectors: &[f64], k: usize, y: f64) -> [f64; 9] {
    let mut m = [0.0; 9];
    for (l, share) in hue::level_shares(level_count(sectors), y) {
        let start = l * LEVEL_COEFFICIENTS + k * 6;
        let level = hue::matrix(&sectors[start..start + 6]);
        for (slot, v) in m.iter_mut().zip(level) {
            *slot += share * v;
        }
    }
    m
}

/// The Jacobian group (level * SECTORS + sector) of each coefficient's column, for the rows
/// of colors in `colors` (stage inputs): the rows that each group moves.
fn sector_groups(levels: usize, colors: impl Iterator<Item = (usize, Rgb)>) -> Vec<Vec<usize>> {
    let mut groups = vec![Vec::new(); levels * hue::SECTORS];
    for (row, v) in colors {
        if let Some(k) = hue::sector(v) {
            for (l, _) in hue::level_shares(levels, hue::luminance(v)) {
                groups[l * hue::SECTORS + k].push(row);
            }
        }
    }
    groups
}

/// The groups that each seam constrains.
fn seam_groups(levels: usize, seams: &[(usize, usize, Rgb)]) -> Vec<Vec<usize>> {
    seams
        .iter()
        .map(|(a, b, color)| {
            hue::level_shares(levels, hue::luminance(*color))
                .into_iter()
                .flat_map(|(l, _)| [l * hue::SECTORS + a, l * hue::SECTORS + b])
                .collect()
        })
        .collect()
}

/// The largest CIE L*a*b* chroma within sRGB at lightness `l` and hue angle `hue`, by
/// bisection.
pub(super) fn largest_chroma(l: f64, hue: f64) -> f64 {
    let (cos, sin) = (hue.cos(), hue.sin());
    let (mut low, mut high) = (0.0, 150.0);
    for _ in 0..30 {
        let middle = (low + high) / 2.0;
        let rgb = cube::lab_d50_to_srgb([l, middle * cos, middle * sin]);
        if rgb.iter().all(|v| (0.0..=1.0).contains(v)) {
            low = middle;
        } else {
            high = middle;
        }
    }
    low
}

/// The ramps start from colors at these lightnesses, `RAMP_HUES` hues, and these shares of the
/// largest chroma within sRGB at each lightness and hue, and from the primary and secondary
/// colors at `RAMP_PRIMARY_LEVELS`.
const RAMP_LIGHTNESS: [f64; 5] = [25.0, 40.0, 55.0, 70.0, 85.0];
const RAMP_HUES: usize = 72;
const RAMP_SHARES: [f64; 3] = [0.35, 0.7, 1.0];
const RAMP_PRIMARY_LEVELS: [f64; 2] = [0.2, 0.5];
/// Steps of a ramp from `RAMP_LOW` EV, `RAMP_STEP` stops apart: -2 to +4 EV.
const RAMP_STEPS: usize = 13;
const RAMP_LOW: f64 = -2.0;
const RAMP_STEP: f64 = 0.5;
/// The weight of a fall of 1 L* along a ramp, in the units of a sample's CIEDE2000 residual.
const RAMP_WEIGHT: f64 = 4.0;
/// The weight of a lag of 1 L* behind the target along a ramp, beyond `LAG_TOLERANCE`, within
/// `SENSOR_HEADROOM`. A weight of 1 left the lag of the built-in looks at 11 L*, and 2 halved
/// it to 9 at the same error on photos.
const LAG_WEIGHT: f64 = 2.0;
const LAG_TOLERANCE: f64 = 1.0;

/// Exposure ramps for a bound on the lightness: as the exposure of a color grows, the effect's
/// L* must not fall. The hue sectors act on the stage's luminance through the weight curve and
/// the levels, so a sector can darken or saturate a color as it gets brighter. The samples
/// hardly constrain this above diffuse white or for saturated colors: on 44 LUTs, 24 fits fell
/// by more than 3 L* along an exposure ramp of a natural color, where their LUT gets brighter.
/// With this bound 2 of 49 did. The ramps cover the edge of the gamut, where the falls were
/// largest; random natural colors left gaps there that the fit used, and 36 hues left gaps
/// that a fit fell by 15 L* in.
///
/// Within `SENSOR_HEADROOM`, the effect must also brighten each step about as much as the
/// target does. Without this, a saturated color stayed a flat patch where the LUT fades it
/// toward white: the built-in looks lagged their LUTs by up to 30 L*.
pub(super) struct Ramps {
    points: Vec<Rgb>,
    /// The L* of the target at each point, and whether the point is within the sensor range.
    target: Vec<(f64, bool)>,
}

impl Ramps {
    pub(super) fn new(target: &dyn Fn(Rgb) -> Rgb, converter: &LabConverter) -> Self {
        let mut points = Vec::new();
        for k in 0..RAMP_HUES {
            let hue = std::f64::consts::TAU * k as f64 / RAMP_HUES as f64;
            let (cos, sin) = (hue.cos(), hue.sin());
            for l in RAMP_LIGHTNESS {
                let largest = largest_chroma(l, hue);
                for share in RAMP_SHARES {
                    let chroma = share * largest;
                    let rgb = cube::lab_d50_to_srgb([l, chroma * cos, chroma * sin]);
                    points.extend((0..RAMP_STEPS).map(|step| {
                        let exposure = 2f64.powf(RAMP_LOW + RAMP_STEP * step as f64);
                        rgb.map(|v| v * exposure)
                    }));
                }
            }
        }
        for primary in [
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 1.0],
            [1.0, 0.0, 1.0],
        ] {
            for level in RAMP_PRIMARY_LEVELS {
                points.extend((0..RAMP_STEPS).map(|step| {
                    let exposure = level * 2f64.powf(RAMP_LOW + RAMP_STEP * step as f64);
                    primary.map(|v: f64| v * exposure)
                }));
            }
        }
        let target = points
            .iter()
            .map(|p| {
                (
                    converter.lab(target(*p).map(|v| v.clamp(0.0, 1.0)))[0],
                    p.iter().all(|v| *v <= super::SENSOR_HEADROOM),
                )
            })
            .collect();
        Self { points, target }
    }

    /// The residual rows: one for each pair of neighbouring steps.
    pub(super) fn rows(&self) -> usize {
        self.points.len() / RAMP_STEPS * (RAMP_STEPS - 1)
    }

    /// The L* of each point.
    pub(super) fn lightness(&self, problem: &Problem, params: &Params) -> Vec<f64> {
        self.points
            .iter()
            .map(|l| problem.converter.lab(problem.model.render(params, *l))[0])
            .collect()
    }

    /// The fall of L* between the two steps of `row`.
    fn pair(&self, l: &[f64], row: usize) -> f64 {
        let i = row / (RAMP_STEPS - 1) * RAMP_STEPS + row % (RAMP_STEPS - 1);
        let fall = RAMP_WEIGHT * (l[i] - l[i + 1]).max(0.0);
        let (from, to) = (self.target[i], self.target[i + 1]);
        if !to.1 {
            return fall;
        }
        let lag = (to.0 - from.0) - (l[i + 1] - l[i]);
        fall + LAG_WEIGHT * (lag - LAG_TOLERANCE).max(0.0)
    }

    pub(super) fn residuals(&self, l: &[f64], out: &mut Vec<f64>) {
        out.extend((0..self.rows()).map(|row| self.pair(l, row)));
    }

    /// The pair rows that point `i` enters.
    fn rows_of(i: usize) -> impl Iterator<Item = usize> {
        let k = i % RAMP_STEPS;
        let base = i / RAMP_STEPS * (RAMP_STEPS - 1);
        (k > 0)
            .then(|| base + k - 1)
            .into_iter()
            .chain((k + 1 < RAMP_STEPS).then_some(base + k))
    }

    /// The points of each Jacobian group (level * SECTORS + sector) under `matrix`.
    fn groups(&self, levels: usize, matrix: &[f64; 9]) -> Vec<Vec<usize>> {
        let m = rows(matrix);
        sector_groups(
            levels,
            self.points
                .iter()
                .enumerate()
                .map(|(i, l)| (i, cube::mul(&m, *l))),
        )
    }

    /// The Jacobian entries of a sector coefficient that moves only `points`: recomputes their
    /// L* under `moved`, from the L* `base` and the residuals `r` of the ramp rows. `column`
    /// holds the ramp rows.
    fn column(
        &self,
        problem: &Problem,
        moved: &Params,
        base: &[f64],
        points: &[usize],
        r: &[f64],
        column: &mut [f64],
    ) {
        if points.is_empty() {
            return;
        }
        let mut l = base.to_vec();
        for &i in points {
            l[i] = problem
                .converter
                .lab(problem.model.render(moved, self.points[i]))[0];
        }
        for &i in points {
            for row in Self::rows_of(i) {
                column[row] = (self.pair(&l, row) - r[row]) / STEP;
            }
        }
    }
}

/// The step between two sectors' matrices on a seam color, relative to its brightness.
fn seam_step(sectors: &[f64], a: usize, b: usize, color: &Rgb, weight: f64) -> [f64; 3] {
    let y = hue::luminance(*color);
    let left = cube::mul(&rows(&sector_matrix(sectors, a, y)), *color);
    let right = cube::mul(&rows(&sector_matrix(sectors, b, y)), *color);
    let mean = color.iter().sum::<f64>() / 3.0;
    std::array::from_fn(|k| weight * (left[k] - right[k]) / mean)
}

fn stage_of(sectors: &[f64], weight: &[f64]) -> hue::Stage {
    hue::Stage {
        levels: sectors
            .chunks_exact(LEVEL_COEFFICIENTS)
            .map(|level| std::array::from_fn(|k| hue::matrix(&level[k * 6..k * 6 + 6])))
            .collect(),
        curve: hue::Curve::through(std::array::from_fn(|k| weight[k].clamp(0.0, 1.0))),
    }
}

/// Sector coefficients (one level, or `hue::LEVELS`) and the four weights, before rounding.
#[derive(Clone, Debug)]
pub(super) struct SectorCoefficients {
    pub(super) coefficients: Vec<f64>,
    pub(super) weight: Vec<f64>,
}

impl SectorCoefficients {
    /// The coefficients and weights as an effect stores them.
    pub(super) fn quantize(&self) -> hue::HueSectors {
        quantize_sectors(&self.coefficients, &self.weight)
    }
}

impl Problem {
    /// One CIEDE2000 residual for sample `i`, the measure of `score`: Lab residuals favor
    /// saturated colors and can make the score worse.
    fn sample_error(&self, params: &Params, i: usize) -> f64 {
        delta_e2000(
            self.converter
                .lab(self.model.render(params, self.linear[i])),
            self.labs[i],
        )
    }

    /// The rows of `subset` that each Jacobian group (level * SECTORS + sector) moves, with
    /// the sectors that the matrix of `offsets` puts the samples in.
    fn sector_members(&self, levels: usize, subset: &[usize], offsets: &[f64]) -> Vec<Vec<usize>> {
        let matrix = rows(&matrix_from(offsets));
        sector_groups(
            levels,
            subset
                .iter()
                .enumerate()
                .map(|(row, &i)| (row, cube::mul(&matrix, self.linear[i]))),
        )
    }

    /// The hue sectors: alternates the sector matrices, the weight curve, the matrix, and the
    /// curves with the post matrix, all on Lab residuals of the colored samples with equal
    /// weights, and keeps the best result by `score`. None if no round beats `start`, a fit
    /// without sectors.
    pub(super) fn hue_sectors(
        &self,
        start: &Candidate,
        options: &Options,
        progress: Progress,
    ) -> Option<Candidate> {
        // The sectors keep gray, so the gray ramp does not inform them.
        let colored: Vec<usize> = (0..self.linear.len())
            .filter(|&i| {
                let l = self.linear[i];
                !(l[0] == l[1] && l[1] == l[2])
            })
            .collect();
        let stride = colored.len().div_ceil(options.sector_samples.max(1)).max(1);
        let subset: Vec<usize> = colored.into_iter().step_by(stride).collect();
        let scale = (subset.len() as f64).sqrt();
        let levels = if options.levels { hue::LEVELS } else { 1 };
        let seams = seam_colors(levels);
        let seam_groups = seam_groups(levels, &seams);
        let residuals = |params: &Params, out: &mut Vec<f64>| {
            out.extend(subset.iter().map(|&i| self.sample_error(params, i)));
        };
        let seam =
            |x: &[f64], a: usize, b: usize, color: &Rgb| seam_step(x, a, b, color, SEAM * scale);
        let limit = |v: f64| BOUND_WEIGHT * scale * excess(v, SECTOR_LIMIT);
        let params = |offsets: &[f64],
                      curves: &[[f64; 256]; 3],
                      post: &[f64; 9],
                      stage: hue::Stage| Params {
            native: None,
            matrix: matrix_from(offsets),
            curves: *curves,
            post: *post,
            sectors: Some(stage),
        };
        let ramps = &self.ramps;
        let with_ramps = |params: &Params, out: &mut Vec<f64>| {
            ramps.residuals(&ramps.lightness(self, params), out);
        };
        let start_score = self.score(&start.offsets, &start.curves, &start.post, None);
        let (mut offsets, mut curves, mut post) = (start.offsets.clone(), start.curves, start.post);
        let (mut sectors, mut weight) = (
            vec![0.0; levels * LEVEL_COEFFICIENTS],
            vec![SECTOR_WEIGHT_START; 4],
        );
        let mut best: Option<(f64, Candidate)> = None;
        let rounds = options.sector_rounds as f64;
        for round in 0..options.sector_rounds {
            // The sector solve takes most of a round: each of its iterations reports a step.
            let iterations = std::cell::Cell::new(0usize);
            // The stage applies one sector's matrix to a color, blended from at most two
            // levels, so a coefficient of sector k and level l moves only the samples in k
            // with a share of l, the seams beside k at those levels, and its own limit. The
            // other entries of its Jacobian column are zero, and the forward differences skip
            // them.
            let members = self.sector_members(levels, &subset, &offsets);
            let ramp_groups = ramps.groups(levels, &matrix_from(&offsets));
            let seam_rows = subset.len();
            let limit_rows = seam_rows + 3 * seams.len();
            let ramp_rows = limit_rows + sectors.len();
            sectors = minimize_with(
                &sectors,
                SECTOR_ITERATIONS,
                &|x, out| {
                    out.clear();
                    residuals(&params(&offsets, &curves, &post, stage_of(x, &weight)), out);
                    for (a, b, color) in &seams {
                        out.extend(seam(x, *a, *b, color));
                    }
                    out.extend(x.iter().map(|v| limit(*v)));
                    with_ramps(&params(&offsets, &curves, &post, stage_of(x, &weight)), out);
                },
                &|x, r, columns| {
                    iterations.set(iterations.get() + 1);
                    let solved = iterations.get() as f64 / SECTOR_ITERATIONS as f64;
                    progress((round as f64 + 0.8 * solved) / rounds);
                    let lightness = ramps.lightness(
                        self,
                        &params(&offsets, &curves, &post, stage_of(x, &weight)),
                    );
                    for (j, column) in columns.iter_mut().enumerate() {
                        let group = j / 6;
                        let mut y = x.to_vec();
                        y[j] += STEP;
                        column.clear();
                        column.resize(r.len(), 0.0);
                        let moved = params(&offsets, &curves, &post, stage_of(&y, &weight));
                        for &row in &members[group] {
                            column[row] = (self.sample_error(&moved, subset[row]) - r[row]) / STEP;
                        }
                        for (s, (a, b, color)) in seams.iter().enumerate() {
                            if seam_groups[s].contains(&group) {
                                for (c, value) in seam(&y, *a, *b, color).into_iter().enumerate() {
                                    let row = seam_rows + 3 * s + c;
                                    column[row] = (value - r[row]) / STEP;
                                }
                            }
                        }
                        let row = limit_rows + j;
                        column[row] = (limit(y[j]) - r[row]) / STEP;
                        ramps.column(
                            self,
                            &moved,
                            &lightness,
                            &ramp_groups[group],
                            &r[ramp_rows..],
                            &mut column[ramp_rows..],
                        );
                    }
                },
            );
            weight = minimize(&weight, 20, &|x, out| {
                out.clear();
                let p = params(&offsets, &curves, &post, stage_of(&sectors, x));
                residuals(&p, out);
                out.push(BOUND_WEIGHT * scale * excess(x[1] - x[0], WEIGHT_STEP));
                with_ramps(&p, out);
            })
            .into_iter()
            .map(|v| v.clamp(0.0, 1.0))
            .collect();
            offsets = minimize(&offsets, 20, &|x, out| {
                out.clear();
                let p = params(x, &curves, &post, stage_of(&sectors, &weight));
                residuals(&p, out);
                out.extend(
                    matrix_from(x)
                        .iter()
                        .map(|v| BOUND_WEIGHT * scale * excess(*v, MATRIX_LIMIT)),
                );
                with_ramps(&p, out);
            });
            let stage = stage_of(&sectors, &weight);
            let before = self.score(&offsets, &curves, &post, Some(&stage));
            let new_curves = self.curves(&matrix_from(&offsets), &post, Some(&stage));
            let new_post = if options.post {
                self.post(&matrix_from(&offsets), &new_curves, &post, Some(&stage))
            } else {
                post
            };
            if self.score(&offsets, &new_curves, &new_post, Some(&stage)) < before {
                (curves, post) = (new_curves, new_post);
            }
            let score = self.score(&offsets, &curves, &post, Some(&stage));
            if best.as_ref().is_none_or(|(b, _)| score < *b) {
                let sectors = SectorCoefficients {
                    coefficients: sectors.clone(),
                    weight: weight.clone(),
                };
                best = Some((
                    score,
                    Candidate {
                        offsets: offsets.clone(),
                        curves,
                        post,
                        sectors: Some(sectors),
                    },
                ));
            }
            progress((round + 1) as f64 / rounds);
        }
        let (score, found) = best?;
        if score >= start_score {
            return None;
        }
        Some(found)
    }

    /// Refines the matrix offsets, the post matrix, and the sector coefficients and weights
    /// together on one CIEDE2000 residual per sample, the measure of `score`, and solves the
    /// curves again after each pass. Keeps the best pass by `score`.
    pub(super) fn joint(
        &self,
        start: &Candidate,
        options: &Options,
        progress: Progress,
    ) -> Candidate {
        let no = start.offsets.len();
        let ns = start.sectors.as_ref().map_or(0, |s| s.coefficients.len());
        let stride = self.linear.len().div_ceil(JOINT_SAMPLES).max(1);
        let subset: Vec<usize> = (0..self.linear.len()).step_by(stride).collect();
        let scale = (subset.len() as f64).sqrt();
        let levels = start
            .sectors
            .as_ref()
            .map_or(1, |s| level_count(&s.coefficients));
        let seams = seam_colors(levels);
        // x: matrix offsets, post matrix, sector coefficients, sector weights.
        let mut x: Vec<f64> = start.offsets.clone();
        x.extend_from_slice(&start.post);
        if let Some(s) = &start.sectors {
            x.extend_from_slice(&s.coefficients);
            x.extend_from_slice(&s.weight);
        }
        let post_of = |x: &[f64]| -> [f64; 9] { x[no..no + 9].try_into().unwrap() };
        let unpack = |x: &[f64], curves: &[[f64; 256]; 3]| Params {
            native: None,
            matrix: matrix_from(&x[..no]),
            curves: *curves,
            post: post_of(x),
            sectors: (ns > 0).then(|| stage_of(&x[no + 9..no + 9 + ns], &x[no + 9 + ns..])),
        };
        let score = |x: &[f64], curves: &[[f64; 256]; 3]| {
            self.score(
                &x[..no],
                curves,
                &post_of(x),
                unpack(x, curves).sectors.as_ref(),
            )
        };
        let ramps = &self.ramps;
        let nr = ramps.rows();
        let mut curves = start.curves;
        let mut best = (score(&x, &curves), x.clone(), curves);
        let passes = options.joint_passes as f64;
        for index in 0..options.joint_passes {
            let fixed = curves;
            let iterations = std::cell::Cell::new(0usize);
            // The bounds of `matrix`, `post`, and `hue_sectors`, for one residual per sample.
            let penalties = |x: &[f64], out: &mut Vec<f64>| {
                out.extend(
                    matrix_from(&x[..no])
                        .iter()
                        .map(|v| BOUND_WEIGHT * scale * excess(*v, MATRIX_LIMIT)),
                );
                post_penalties(
                    &x[no..no + 9],
                    EXACT_WEIGHT * scale,
                    EXACT_WEIGHT * scale,
                    POST_DAMPING * scale,
                    out,
                );
                if ns > 0 {
                    let s = &x[no + 9..no + 9 + ns];
                    for (a, b, color) in &seams {
                        out.extend(seam_step(s, *a, *b, color, SEAM * scale));
                    }
                    out.extend(
                        s.iter()
                            .map(|v| BOUND_WEIGHT * scale * excess(*v, SECTOR_LIMIT)),
                    );
                    let weight = &x[no + 9 + ns..];
                    out.extend(
                        weight
                            .iter()
                            .map(|v| BOUND_WEIGHT * scale * ((v - 1.0).max(0.0) + (-v).max(0.0))),
                    );
                    out.push(BOUND_WEIGHT * scale * excess(weight[1] - weight[0], WEIGHT_STEP));
                }
            };
            let residuals = |x: &[f64], out: &mut Vec<f64>| {
                out.clear();
                let params = unpack(x, &fixed);
                out.extend(subset.iter().map(|&i| self.sample_error(&params, i)));
                ramps.residuals(&ramps.lightness(self, &params), out);
                penalties(x, out);
            };
            x = minimize_with(&x, JOINT_ITERATIONS, &residuals, &|x, r, columns| {
                iterations.set(iterations.get() + 1);
                let solved = iterations.get() as f64 / JOINT_ITERATIONS as f64;
                progress((index as f64 + solved) / passes);
                let n = subset.len();
                // A sector coefficient moves only the samples in its sector, as in
                // `hue_sectors`; the matrix decides the sector of each sample.
                let members = if ns > 0 {
                    self.sector_members(levels, &subset, &x[..no])
                } else {
                    Vec::new()
                };
                let ramp_groups = if ns > 0 {
                    ramps.groups(levels, &matrix_from(&x[..no]))
                } else {
                    Vec::new()
                };
                let lightness = ramps.lightness(self, &unpack(x, &fixed));
                let mut tail = Vec::new();
                for (j, column) in columns.iter_mut().enumerate() {
                    let mut y = x.to_vec();
                    y[j] += STEP;
                    let moved = unpack(&y, &fixed);
                    column.clear();
                    column.resize(r.len(), 0.0);
                    if (no + 9..no + 9 + ns).contains(&j) {
                        for &row in &members[(j - no - 9) / 6] {
                            column[row] = (self.sample_error(&moved, subset[row]) - r[row]) / STEP;
                        }
                        let group = &ramp_groups[(j - no - 9) / 6];
                        ramps.column(self, &moved, &lightness, group, &r[n..], &mut column[n..]);
                    } else {
                        for row in 0..n {
                            column[row] = (self.sample_error(&moved, subset[row]) - r[row]) / STEP;
                        }
                        let moved_lightness = ramps.lightness(self, &moved);
                        for row in 0..nr {
                            column[n + row] =
                                (ramps.pair(&moved_lightness, row) - r[n + row]) / STEP;
                        }
                    }
                    tail.clear();
                    penalties(&y, &mut tail);
                    for (k, v) in tail.iter().enumerate() {
                        column[n + nr + k] = (v - r[n + nr + k]) / STEP;
                    }
                }
            });
            let params = unpack(&x, &fixed);
            let solved = self.curves(&params.matrix, &params.post, params.sectors.as_ref());
            let (with_solved, with_fixed) = (score(&x, &solved), score(&x, &fixed));
            let pass = if with_fixed < with_solved {
                (with_fixed, fixed)
            } else {
                (with_solved, solved)
            };
            curves = pass.1;
            if pass.0 < best.0 {
                best = (pass.0, x.clone(), curves);
            }
            progress((index + 1) as f64 / passes);
        }
        let (_, x, curves) = best;
        Candidate {
            offsets: x[..no].to_vec(),
            curves,
            post: post_of(&x),
            sectors: (ns > 0).then(|| SectorCoefficients {
                coefficients: x[no + 9..no + 9 + ns].to_vec(),
                weight: x[no + 9 + ns..].iter().map(|v| v.clamp(0.0, 1.0)).collect(),
            }),
        }
    }
}

/// Rounds sector coefficients and weights as an effect stores them.
fn quantize_sectors(sectors: &[f64], weight: &[f64]) -> hue::HueSectors {
    let level = |coefficients: &[f64]| -> hue::Sectors {
        std::array::from_fn(|k| {
            let mut m = [0i8; 6];
            for (j, v) in m.iter_mut().enumerate() {
                *v = (coefficients[k * 6 + j] * 128.0)
                    .round()
                    .clamp(-127.0, 127.0) as i8;
            }
            // The diagonal must fit the register: the two coefficients of a row sum to -127
            // or more.
            for r in 0..3 {
                if i32::from(m[r * 2]) + i32::from(m[r * 2 + 1]) < -127 {
                    m[r * 2 + 1] = (-127 - i32::from(m[r * 2])) as i8;
                }
            }
            m
        })
    };
    let mut chunks = sectors.chunks_exact(LEVEL_COEFFICIENTS);
    let matrices = level(chunks.next().expect("level 0"));
    let upper: Vec<hue::Sectors> = chunks.map(level).collect();
    let weight = std::array::from_fn(|k| (weight[k] * 255.0).round().clamp(0.0, 255.0) as u8);
    let mut sectors = hue::HueSectors {
        matrices,
        weight,
        levels: (!upper.is_empty()).then(|| upper.try_into().expect("hue::LEVELS levels")),
    };
    // The fit bounds the first weight step by WEIGHT_STEP, a soft bound; this makes the
    // limit exact.
    while !sectors.slopes_double() {
        let [first, second, ..] = sectors.weight;
        sectors.weight[1] = if second > first {
            second - 1
        } else {
            second + 1
        };
    }
    join_last_segment(sectors)
}

/// Moves the last weight so that the curve stays continuous at the last breakpoint.
///
/// The camera reaches the last breakpoint at a stage luminance of about 1.37 and uses the
/// last start weight above it. The 9-bit slope of the segment before it is rounded, which
/// moves its end by up to 0.05 from that weight: a step that colors cross as they get
/// brighter. This picks the last weight nearest to the end of that segment, preferring small
/// changes, and keeps the slopes within the range of highlight correction.
fn join_last_segment(sectors: hue::HueSectors) -> hue::HueSectors {
    let span = f64::from(hue::BORDERS[2] - hue::BORDERS[1]);
    let jump = |s: &hue::HueSectors| {
        let curve = s.stage().curve;
        (curve.starts[3] - curve.starts[2] - curve.slopes[2] * span / 32.0).abs()
    };
    let original = i32::from(sectors.weight[3]);
    let cost = |s: &hue::HueSectors| {
        jump(s) + 0.01 * f64::from((i32::from(s.weight[3]) - original).unsigned_abs())
    };
    let mut best = sectors;
    for weight in (original - 40).max(0)..=(original + 40).min(255) {
        let mut candidate = sectors;
        candidate.weight[3] = weight as u8;
        if candidate.slopes_double() && cost(&candidate) < cost(&best) {
            best = candidate;
        }
    }
    best
}
