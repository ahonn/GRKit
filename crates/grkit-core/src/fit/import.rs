//! `.cube` and LUT image import: the fit target, the input guess, and effect.json.

use anyhow::{ensure, Result};
use serde::Serialize;
use serde_json::Value;

use crate::cube::{delta_e2000, Cube, Encoding, LabConverter, Layout, Rgb};
use crate::xmp::{self, Hints as PresetHints};
use crate::{effects, packaging};

use super::*;

pub const MAX_EXPOSURE: f64 = 8.0;

/// The fit target of a LUT: scene-linear RGB, exposed by `exposure` EV, encoded, then looked up.
pub fn cube_target<'a>(
    model: &'a Model,
    cube: &'a Cube,
    input: Encoding,
    exposure: f64,
) -> impl Fn(Rgb) -> Rgb + 'a {
    let gain = 2f64.powf(exposure);
    move |linear| {
        cube.apply(model.encode(input, linear.map(|v| v * gain)))
            .map(|v| v.clamp(0.0, 1.0))
    }
}

/// What `import` writes besides the fitted coefficients.
#[derive(Clone, Debug)]
pub struct Import<'a> {
    pub effect_id: &'a str,
    pub name: &'a str,
    /// The `icon` of effect.json: a label tile or a film cover (`effects::icon_pixels`).
    pub icon: &'a Value,
    /// The `.cube` or HALD PNG file name, recorded in the attribution.
    pub source: &'a str,
    /// The input encoding of the LUT, or `None` for the one that its data suggests
    /// (`guess_input`).
    pub input: Option<Encoding>,
    pub exposure: f64,
    pub options: Options,
}

#[derive(Serialize)]
struct Errors {
    mean_de: f64,
    max_de: f64,
    standard_mean_de: f64,
    csp_max: i32,
    holdout_mean_de: f64,
    holdout_p95_de: f64,
    grade: &'static str,
}

#[derive(Serialize)]
struct Attribution<'a> {
    origin: &'a str,
    source: &'a str,
    source_sha256: String,
    input: &'a str,
    exposure: f64,
    fit: Errors,
}

fn attribution_origin(preset: bool) -> &'static str {
    if preset {
        "xmp-preset"
    } else {
        "cube-fit"
    }
}

/// Field order follows the existing effect.json files.
#[derive(Serialize)]
struct Definition<'a> {
    schema: &'a str,
    effect_id: &'a str,
    name: &'a str,
    flags: u32,
    matrix: [i16; 9],
    icon: &'a Value,
    attribution: Attribution<'a>,
    curves: [&'a [u16]; 3],
    #[serde(skip_serializing_if = "Option::is_none")]
    post_matrix: Option<[i16; 9]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hue_sectors: Option<serde_json::Value>,
}

/// Pretty effect.json text that `effects::compile_definition` accepts.
fn definition_text(
    fit: &Fit,
    request: &Import,
    input: Encoding,
    sha256: String,
    preset: bool,
) -> Result<String> {
    let round = |v: f64| (v * 1000.0).round() / 1000.0;
    let definition = Definition {
        schema: effects::SCHEMA,
        effect_id: request.effect_id,
        name: request.name,
        flags: if fit.monochrome {
            effects::MONOCHROME
        } else {
            0
        },
        matrix: fit.matrix,
        icon: request.icon,
        attribution: Attribution {
            origin: attribution_origin(preset),
            source: request.source,
            source_sha256: sha256,
            input: input.name(),
            exposure: request.exposure,
            fit: Errors {
                mean_de: round(fit.mean_de),
                max_de: round(fit.max_de),
                standard_mean_de: round(fit.standard_mean_de),
                csp_max: fit.csp_max,
                holdout_mean_de: round(fit.holdout_mean_de),
                holdout_p95_de: round(fit.holdout_p95_de),
                grade: Grade::of(fit).letter(),
            },
        },
        curves: [&fit.curves[0], &fit.curves[1], &fit.curves[2]],
        post_matrix: fit.post_matrix,
        hue_sectors: fit.sectors.map(|s| s.to_json()),
    };
    let mut text = serde_json::to_string_pretty(&definition)?;
    text.push('\n');
    effects::compile_definition(&effects::parse_json(text.as_bytes())?)?;
    Ok(text)
}

/// How closely an effect follows its LUT, from the held-out colors.
///
/// The bounds make the grades agree with the pixel errors of 807 LUTs on 56 natural pictures,
/// where A is a mean dE2000 up to 1.5 and a 95th percentile up to 3, B up to 3 and 6, and C a
/// mean up to 5. 715 grades agree, 84 are too strict, and 8 are too good. The earlier bounds,
/// from two pictures, made 197 grades too strict.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Grade {
    A,
    B,
    C,
    D,
}

impl Grade {
    /// (mean, 95th percentile) bounds of the held-out errors for A, B, and C.
    const BOUNDS: [(f64, f64); 3] = [(1.2, 3.75), (2.0, 8.0), (4.5, f64::INFINITY)];

    pub fn of(fit: &Fit) -> Self {
        let within =
            |(mean, p95): (f64, f64)| fit.holdout_mean_de <= mean && fit.holdout_p95_de <= p95;
        match Self::BOUNDS.iter().position(|b| within(*b)) {
            Some(0) => Self::A,
            Some(1) => Self::B,
            Some(_) => Self::C,
            None => Self::D,
        }
    }

    pub fn letter(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
            Self::D => "D",
        }
    }

    pub fn summary(self) -> &'static str {
        match self {
            Self::A => "nearly the same as the LUT",
            Self::B => "the same look with small differences",
            Self::C => "a similar look with visible color differences",
            Self::D => "a different look",
        }
    }
}

/// Local flaws of an effect that the held-out mean hides, each beyond what its LUT does.
#[derive(Clone, Copy, Debug, Default)]
pub struct Flaws {
    /// The largest fall of L* along an exposure ramp of a saturated color, from -2 EV up to
    /// `SENSOR_HEADROOM`.
    pub fall: f64,
    /// The largest lag of L* behind the LUT along such a ramp, from 0 EV on: how much less the
    /// effect brightens a color than the LUT does. A lagging color stays a flat, saturated
    /// patch where the LUT fades it toward white.
    pub lag: f64,
    /// The largest color step between neighbouring hues of a saturated hue sweep, in CIEDE2000.
    pub step: f64,
}

/// The flaws that `import` reports. Of 53 test LUTs, mostly hard ones, 1 fit falls, 6 lag, and
/// 11 step by more than these.
pub const FALL_WARNING: f64 = 5.0;
pub const LAG_WARNING: f64 = 20.0;
pub const STEP_WARNING: f64 = 8.0;

/// The primary and secondary colors at full saturation.
const PRIMARIES: [Rgb; 6] = [
    [1.0, 0.0, 0.0],
    [1.0, 1.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, 1.0, 1.0],
    [0.0, 0.0, 1.0],
    [1.0, 0.0, 1.0],
];

/// The fully saturated color of hue `turn` (0..1) on the edge of the RGB cube: the path through
/// `PRIMARIES`.
fn edge_color(turn: f64) -> Rgb {
    let at = turn.rem_euclid(1.0) * 6.0;
    let k = (at as usize).min(5);
    let (a, b) = (PRIMARIES[k], PRIMARIES[(k + 1) % 6]);
    let f = at - k as f64;
    std::array::from_fn(|c| a[c] + f * (b[c] - a[c]))
}

/// Measures `Flaws` of `params` against `target` on colors at the edge of the sRGB gamut, where
/// the hue sectors meet and where the samples of the fit are few: exposure ramps and hue sweeps
/// at the edge of the Lab gamut and through the primary colors.
pub fn flaws(model: &Model, params: &Params, target: &dyn Fn(Rgb) -> Rgb) -> Flaws {
    let converter = LabConverter::default();
    let lab = |rgb: Rgb| converter.lab(rgb);
    let color = |l: f64, hue: f64, share: f64| {
        let chroma = share * sectors::largest_chroma(l, hue);
        cube::lab_d50_to_srgb([l, chroma * hue.cos(), chroma * hue.sin()]).map(|v| v.max(0.0))
    };
    let mut flaws = Flaws::default();
    let mut ramps = Vec::new();
    for k in 0..72 {
        let hue = std::f64::consts::TAU * f64::from(k) / 72.0;
        for l in [25.0, 40.0, 55.0, 70.0, 85.0] {
            for share in [0.5, 0.9, 1.0] {
                ramps.push(color(l, hue, share));
            }
        }
    }
    for primary in PRIMARIES {
        for level in [0.2, 0.5] {
            ramps.push(primary.map(|v| v * level));
        }
    }
    // Steps of a quarter stop from -2 EV; step 8 is 0 EV.
    for base in ramps {
        let (mut peak, mut fall) = ([f64::MIN; 2], [0.0f64; 2]);
        let mut gap_at_zero = None;
        for step in 0..=24 {
            let linear = base.map(|v| v * 2f64.powf(-2.0 + 0.25 * f64::from(step)));
            if linear.iter().any(|v| *v > SENSOR_HEADROOM) {
                break;
            }
            let pair = [lab(model.render(params, linear))[0], lab(target(linear))[0]];
            for i in 0..2 {
                peak[i] = peak[i].max(pair[i]);
                fall[i] = fall[i].max(peak[i] - pair[i]);
            }
            let gap = pair[1] - pair[0];
            if step >= 8 {
                let zero = *gap_at_zero.get_or_insert(gap);
                flaws.lag = flaws.lag.max(gap - zero);
            }
        }
        flaws.fall = flaws.fall.max(fall[0] - fall[1]);
    }
    let mut sweeps: Vec<Box<dyn Fn(f64) -> Rgb>> = Vec::new();
    for l in [30.0, 45.0, 60.0, 75.0, 90.0] {
        for share in [0.5, 0.95] {
            sweeps.push(Box::new(move |turn| {
                color(l, std::f64::consts::TAU * turn, share)
            }));
        }
    }
    for level in [0.3, 0.6, 1.0] {
        sweeps.push(Box::new(move |turn| edge_color(turn).map(|v| v * level)));
    }
    for sweep in &sweeps {
        for ev in [-1, 0, 1, 2] {
            let mut previous: Option<[[f64; 3]; 2]> = None;
            for k in 0..=720 {
                let linear = sweep(f64::from(k) / 720.0).map(|v| v * 2f64.powi(ev));
                if linear.iter().any(|v| *v > SENSOR_HEADROOM) {
                    previous = None;
                    continue;
                }
                let pair = [lab(model.render(params, linear)), lab(target(linear))];
                if let Some(before) = previous {
                    let step = delta_e2000(pair[0], before[0]) - delta_e2000(pair[1], before[1]);
                    flaws.step = flaws.step.max(step);
                }
                previous = Some(pair);
            }
        }
    }
    flaws
}

/// A warning for a grade of C or D, for a caller that does not describe the grade otherwise.
pub fn grade_warning(fit: &Fit) -> Option<String> {
    let grade = Grade::of(fit);
    (grade >= Grade::C).then(|| {
        format!(
            "Grade {}: {}. The look needs changes that one color matrix and three tone \
             curves cannot make.",
            grade.letter(),
            grade.summary()
        )
    })
}

/// The result of `import`.
#[derive(Clone, Debug)]
pub struct Imported {
    pub fit: Fit,
    /// The effect.json text.
    pub text: String,
    /// The input encoding that the fit used: the requested one, or the guess.
    pub input: Encoding,
    /// Conditions that the user should check before using the effect. The grade is in `fit`
    /// (`Grade::of`, `grade_warning`).
    pub warnings: Vec<String>,
    /// When the source was an XMP preset: exposure, grain, and white-balance hints that the
    /// color LUT does not bake in.
    pub hints: Option<PresetHints>,
}

/// The inputs that `guess_input` chooses from. Linear is left out: a look LUT does not take
/// scene-linear values.
const GUESSES: [Encoding; 6] = [
    Encoding::Standard,
    Encoding::Srgb,
    Encoding::Vlog,
    Encoding::Flog,
    Encoding::Flog2,
    Encoding::Flog2C,
];
/// How many times closer to Standard a log input must keep the LUT than the best of Standard
/// and sRGB, for `guess_input` to choose it. On 32 LUTs of known input, 1.15 to 1.5 gave the
/// same choices.
const LOG_MARGIN: f64 = 1.3;

/// How far a LUT read with `input` is from the camera's Standard rendering: the RMS of the
/// neutral luma from -6 to +3 EV, plus the mean CIEDE2000 / 100 of the ColorChecker at -1, 0,
/// and +1 EV. A look changes a normal rendering moderately, so with the input that it expects,
/// a LUT stays close. With another input it loses or doubles the contrast of a log curve.
pub fn input_distance(model: &Model, cube: &Cube, input: Encoding) -> f64 {
    let target = cube_target(model, cube, input, 0.0);
    let luma = |c: Rgb| LUMA[0] * c[0] + LUMA[1] * c[1] + LUMA[2] * c[2];
    let grays: Vec<f64> = (0..=36)
        .map(|i| 0.18 * 2f64.powf(-6.0 + f64::from(i) * 0.25))
        .collect();
    let tone = (grays
        .iter()
        .map(|g| (luma(target([*g; 3])) - luma(model.standard([*g; 3]))).powi(2))
        .sum::<f64>()
        / grays.len() as f64)
        .sqrt();
    let converter = LabConverter::default();
    let chart = colorchecker();
    let mut color = 0.0;
    for ev in [-1, 0, 1] {
        for patch in chart {
            let c = patch.map(|v| v * 2f64.powi(ev));
            color += delta_e2000(converter.lab(target(c)), converter.lab(model.standard(c)));
        }
    }
    tone + color / (3.0 * chart.len() as f64) / 100.0
}

/// Whether the LUT data cannot tell `a` from `b`: two display inputs, or F-Log2 and F-Log2 C,
/// which share their curve.
fn similar(a: Encoding, b: Encoding) -> bool {
    a == b
        || (a.is_display() && b.is_display())
        || matches!(
            (a, b),
            (Encoding::Flog2, Encoding::Flog2C) | (Encoding::Flog2C, Encoding::Flog2)
        )
}

/// The distance of each input that `guess_input` chooses from.
fn guess_distances(model: &Model, cube: &Cube) -> Vec<(Encoding, f64)> {
    GUESSES
        .iter()
        .map(|input| (*input, input_distance(model, cube, *input)))
        .collect()
}

/// The guess from `guess_distances`: a log input only when it is `LOG_MARGIN` times closer
/// than the closest display input.
fn choose(distances: &[(Encoding, f64)]) -> Encoding {
    let best = |log: bool| {
        distances
            .iter()
            .filter(|(input, _)| log != input.is_display())
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .copied()
            .expect("GUESSES has log and display inputs")
    };
    let (log, display) = (best(true), best(false));
    if display.1 >= LOG_MARGIN * log.1 {
        log.0
    } else {
        Encoding::Standard
    }
}

/// The input that a LUT most likely expects, from its data alone (`input_distance`). A log
/// input wins only when it is `LOG_MARGIN` times closer than Standard and sRGB; otherwise the
/// choice is Standard, as the data does not tell Standard from sRGB reliably.
pub fn guess_input(base_gamma: &[u16; 256], cube: &Cube) -> Encoding {
    choose(&guess_distances(&Model::new(base_gamma), cube))
}

/// The progress of `import` when its fit starts and when it ends, in proportion to the time of a
/// color LUT: before the fit, it reads the LUT and guesses its input; after it, it checks flaws.
const IMPORT_FIT: (f64, f64) = (0.01, 0.99);

/// Fits a `.cube` file, HALD PNG, or Lightroom / Camera Raw `.xmp` preset over `base_gamma`
/// and returns the fit and its effect.json.
pub fn import(bytes: &[u8], base_gamma: &[u16; 256], request: &Import) -> Result<Imported> {
    import_with_progress(bytes, base_gamma, request, &|_| {})
}

/// `import`, which reports its progress to `progress`.
pub fn import_with_progress(
    bytes: &[u8],
    base_gamma: &[u16; 256],
    request: &Import,
    progress: Progress,
) -> Result<Imported> {
    ensure!(
        request.exposure.is_finite() && request.exposure.abs() <= MAX_EXPOSURE,
        "Expected an exposure from -{MAX_EXPOSURE} to {MAX_EXPOSURE} EV"
    );
    // Reject an invalid ID, name, or icon before the fit, which takes seconds.
    effects::check_identity(request.effect_id, request.name)?;
    effects::icon_pixels(request.icon)?;
    let (cube, preset_warnings, hints) = if xmp::looks_like(bytes) {
        let prepared = xmp::prepare_with(bytes, request.options.base_bridge)?;
        (prepared.cube, prepared.warnings, Some(prepared.hints))
    } else {
        (Cube::read(bytes)?, Vec::new(), None)
    };
    let from_preset = hints.is_some();
    let model = Model::new(base_gamma);
    // The data, not the file name, tells the input. An XMP preset is a look on top of the
    // camera's Standard rendering unless the caller names another encoding.
    let distances = guess_distances(&model, &cube);
    let guess = choose(&distances);
    let input = request.input.unwrap_or(if from_preset {
        Encoding::Standard
    } else {
        guess
    });
    let target = cube_target(&model, &cube, input, request.exposure);
    progress(IMPORT_FIT.0);
    let fit = fit_with_progress(
        base_gamma,
        &target,
        &request.options,
        &span(progress, IMPORT_FIT.0, IMPORT_FIT.1),
    )?;
    let text = definition_text(&fit, request, input, packaging::sha256(bytes), from_preset)?;
    let mut warnings = preset_warnings;
    let limit = (MATRIX_LIMIT * 8192.0) as i32 - 64;
    if fit.matrix.iter().any(|v| (*v as i32).abs() >= limit) {
        warnings.push(format!(
            "The fit reached the matrix limit of {MATRIX_LIMIT}. The effect mixes the color \
             channels strongly and can show color noise at high ISO."
        ));
    }
    if cube.image_layout.is_some()
        && crate::lut_image::noise(&cube.table, cube.size)
            > crate::lut_image::noise_limit(cube.size)
    {
        warnings.push(
            "The LUT image is noisy, as from grain or sharpening. Export it again with grain, \
             sharpening, clarity, texture, dehaze, vignette, and noise reduction off."
                .to_owned(),
        );
    }
    if let Some(layout) = cube.image_layout.filter(|l| *l != Layout::Hald) {
        warnings.push(format!(
            "Read the image as a LUT in {}, not as a HALD image",
            layout.name()
        ));
    }
    let distance = |input: Encoding| {
        distances
            .iter()
            .find(|(guessed, _)| *guessed == input)
            .map_or_else(|| input_distance(&model, &cube, input), |(_, d)| *d)
    };
    // A guess that keeps the LUT closer to Standard than the chosen input does. On 820 LUTs of
    // known input, this warns for 92% of wrong inputs and 8% of right ones; requiring the guess
    // to be `LOG_MARGIN` times closer missed a quarter of the wrong inputs. The data does not
    // tell Standard from sRGB, or F-Log2 from F-Log2 C, so these do not warn about each other.
    // Preset imports always use Standard unless the caller overrides, so they skip this guess.
    if !from_preset && !similar(guess, input) && distance(input) > distance(guess) {
        warnings.push(format!(
            "The LUT data suggests {} input, but the import used {}",
            guess.name(),
            input.name()
        ));
    }
    // A guessed log input. The data cannot tell a supported log from another camera's log,
    // which the import then reads as the nearest supported one.
    if !from_preset && request.input.is_none() && !guess.is_display() {
        let near: Vec<&str> = distances
            .iter()
            .filter(|(other, d)| {
                !other.is_display() && !similar(*other, guess) && *d < LOG_MARGIN * distance(guess)
            })
            .map(|(other, _)| other.name())
            .collect();
        let also = if near.is_empty() {
            String::new()
        } else {
            format!(", and it fits {} about as well", near.join(" or "))
        };
        warnings.push(format!(
            "The LUT data suggests {} input{also}. Choose the input if you know it. A LUT for \
             another camera log, such as S-Log3, LogC, C-Log, or N-Log, imports with wrong colors.",
            guess.name()
        ));
    }
    let flaws = flaws(&model, &fit.params(), &target);
    if flaws.fall > FALL_WARNING {
        warnings.push(format!(
            "Some saturated colors get darker as their exposure grows, by up to {:.0} L*, where \
             the LUT keeps them brightening",
            flaws.fall
        ));
    }
    if flaws.lag > LAG_WARNING {
        warnings.push(format!(
            "Some saturated colors stay darker and more saturated than in the LUT as their \
             exposure grows, by up to {:.0} L*",
            flaws.lag
        ));
    }
    if flaws.step > STEP_WARNING {
        warnings.push(format!(
            "Some saturated colors change abruptly between neighbouring hues, by up to {:.0} \
             dE2000 more than in the LUT",
            flaws.step
        ));
    }
    if cube.video_input || cube.video_output {
        warnings.push(
            "The LUT uses video levels. GRKit converts 64-940 of 1023, an assumption that \
             Resolve does not document."
                .to_owned(),
        );
    }
    progress(1.0);
    Ok(Imported {
        fit,
        text,
        input,
        warnings,
        hints,
    })
}
