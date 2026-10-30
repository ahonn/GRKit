mod common;

use common::*;
use grkit_core::cube::{self, Cube, Encoding, LabConverter};
use grkit_core::fit::{self, Import, Options, Params, IDENTITY};
use grkit_core::{effects, hue};
use serde_json::{json, Value};
use std::sync::LazyLock;

/// Smooth curves that reach white at scene-linear 1, where sRGB and linear LUTs clip.
fn known(post: [f64; 9]) -> Params {
    let white = model().base_gamma(1.0) / 64.0;
    let curve = |a: f64, toe: f64| -> [f64; 256] {
        std::array::from_fn(|i| {
            let s = (1.0 - (-a * i as f64 / white).exp()) / (1.0 - (-a).exp());
            (toe + (16383.0 - toe) * s).min(16383.0).round()
        })
    };
    Params {
        native: None,
        matrix: [0.9, 0.12, -0.02, 0.03, 0.92, 0.05, -0.01, 0.06, 0.95],
        curves: [curve(3.0, 160.0), curve(2.8, 96.0), curve(3.3, 240.0)],
        post,
        sectors: None,
    }
}

const POST: [f64; 9] = [1.08, -0.05, -0.03, -0.04, 1.06, -0.02, -0.02, -0.06, 1.08];

fn synthetic_cube(input: Encoding, size: usize, params: &Params) -> Cube {
    let model = model();
    parse(&cube_text(size, "", |x| {
        model.render(params, model.decode(input, x))
    }))
    .unwrap()
}

fn fit_cube(input: Encoding, size: usize, params: &Params) -> fit::Fit {
    let (model, cube) = (model(), synthetic_cube(input, size, params));
    let target = fit::cube_target(&model, &cube, input, 0.0);
    fit::fit(&synthetic_base(), &target, &Options::default()).unwrap()
}

/// The chart error of the known parameters against their own LUT: the interpolation floor.
fn floor(input: Encoding, size: usize, params: &Params) -> (f64, f64) {
    let (model, lab) = (model(), cube::LabConverter::default());
    let cube = synthetic_cube(input, size, params);
    let target = fit::cube_target(&model, &cube, input, 0.0);
    let errors = fit::colorchecker()
        .map(|p| cube::delta_e2000(lab.lab(model.render(params, p)), lab.lab(target(p))));
    (
        errors.iter().sum::<f64>() / 24.0,
        errors.iter().copied().fold(0.0, f64::max),
    )
}

fn round_trip(input: Encoding, size: usize, params: &Params) -> fit::Fit {
    let started = std::time::Instant::now();
    let result = fit_cube(input, size, params);
    let elapsed = started.elapsed();
    let (floor_mean, floor_max) = floor(input, size, params);
    let fitted = result.params();
    let distance = |a: &[f64; 9], b: &[f64; 9]| {
        a.iter()
            .zip(b)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0, f64::max)
    };
    let (matrix, post) = (
        distance(&fitted.matrix, &params.matrix),
        distance(&fitted.post, &params.post),
    );
    // Compare the curves from 5% to white. Below 5%, a cell of a 65-point linear LUT spans
    // dozens of curve entries, and the fit follows the LUT's chord. Above white, sRGB and
    // linear LUTs clip.
    let index = |linear: f64| (model().base_gamma(linear) / 64.0) as usize;
    let (black, white) = (index(0.05), index(1.0));
    let curve = (0..3)
        .flat_map(|c| (black..white).map(move |i| (c, i)))
        .map(|(c, i)| (fitted.curves[c][i] - params.curves[c][i]).abs())
        .fold(0.0, f64::max);
    eprintln!(
        "{} {size}: mean {:.3} max {:.3} (floor {floor_mean:.3} / {floor_max:.3}) standard {:.3} csp {} | matrix {matrix:.4} post {post:.4} curve {curve:.0} | {elapsed:.1?}",
        input.name(),
        result.mean_de,
        result.max_de,
        result.standard_mean_de,
        result.csp_max,
    );
    assert!(
        result.mean_de < 0.5 && result.max_de < floor_max + 1.0,
        "{result:?}"
    );
    // The random samples follow natural scenes and hold few saturated colors, which fix the
    // post matrix less tightly than the colors do. The camera's chroma suppression also
    // removes the chroma near black and white, where the samples then fix neither matrix.
    // The color error above stays small.
    assert!(matrix < 0.025 && post < 0.04 && curve < 100.0);
    assert!(result.standard_mean_de > result.mean_de);
    assert!(result.csp_max <= fit::CSP_LIMIT);
    result
}

#[test]
fn synthetic_base_gamma_matches_the_firmware_shape() {
    let base = synthetic_base();
    assert_eq!((base[0], base[255]), (0, 16384));
    assert!(base.windows(2).all(|p| p[0] <= p[1]));
    let gray = model().base_gamma(0.18) / 16384.0;
    assert!((gray - 0.46).abs() < 0.01, "{gray}");
}

#[test]
fn srgb_round_trip_recovers_the_model() {
    let result = round_trip(Encoding::Srgb, 33, &known(POST));
    assert!(result.post_matrix.is_some());
}

/// The fit errors without the curves, for test logs.
fn brief(fit: &fit::Fit) -> String {
    format!(
        "mean {:.3} max {:.3} standard {:.3} csp {} mono {} matrix {:?} post {:?}",
        fit.mean_de,
        fit.max_de,
        fit.standard_mean_de,
        fit.csp_max,
        fit.monochrome,
        fit.matrix,
        fit.post_matrix
    )
}

#[test]
fn standard_lut_fits_its_target() {
    // Standard clips saturated colors, so the LUT cannot hold the known parameters there.
    // The fit is checked against the target, not against the parameters.
    let result = fit_cube(Encoding::Standard, 33, &known(POST));
    let (_, floor_max) = floor(Encoding::Standard, 33, &known(POST));
    eprintln!("standard 33: {} floor max {floor_max:.3}", brief(&result));
    assert!(result.mean_de < 1.0 && result.max_de < floor_max + 3.0);
    assert!(result.csp_max <= fit::CSP_LIMIT);
}

#[test]
fn flog_round_trip_recovers_the_model() {
    round_trip(Encoding::Flog, 65, &known(POST));
}

#[test]
fn flog2_round_trip_recovers_the_model() {
    round_trip(Encoding::Flog2, 65, &known(POST));
}

#[test]
fn flog2c_round_trip_recovers_the_model() {
    round_trip(Encoding::Flog2C, 65, &known(POST));
}

#[test]
fn standard_encoding_inverts_and_makes_an_identity_lut_neutral() {
    let model = model();
    for linear in [[0.18; 3], [0.05, 0.2, 0.6], [0.5, 0.3, 0.1]] {
        let back = model.decode(Encoding::Standard, model.encode(Encoding::Standard, linear));
        assert!(
            back.iter().zip(linear).all(|(a, b)| (a - b).abs() < 1e-3),
            "{back:?}"
        );
    }
    // An identity LUT on a Standard image is Standard itself.
    let cube = parse(&cube_text(33, "", |x| x)).unwrap();
    let target = fit::cube_target(&model, &cube, Encoding::Standard, 0.0);
    let result = fit::fit(&synthetic_base(), &target, &Options::default()).unwrap();
    eprintln!("standard identity: {}", brief(&result));
    assert!(result.mean_de < 0.5 && result.standard_mean_de < 0.5);
}

#[test]
fn linear_round_trip_recovers_the_model() {
    round_trip(Encoding::Linear, 65, &known(POST));
}

#[test]
fn vlog_round_trip_recovers_the_model() {
    round_trip(Encoding::Vlog, 65, &known(POST));
}

#[test]
fn post_matrix_stays_within_the_csp_limit() {
    assert_eq!(fit::csp_max(&[8192, 0, 0, 0, 8192, 0, 0, 0, 8192]), 150);
    // Cb x P reaches 301 / 256 in the blue column, beyond the limit of 240.
    let saturated = [1.9, -0.45, -0.45, -0.45, 1.9, -0.45, -0.45, -0.45, 1.9];
    let post = fit::constrain(&saturated).unwrap();
    let csp = fit::csp_max(&post);
    assert!((200..=fit::CSP_LIMIT).contains(&csp), "{csp}");
    assert!(post
        .chunks(3)
        .all(|row| row.iter().map(|v| *v as i32).sum::<i32>() == 8192));
    // The fit of that look keeps the limit too, if it keeps a post matrix at all.
    let result = fit_cube(Encoding::Vlog, 17, &known(saturated));
    eprintln!("saturated: {}", brief(&result));
    assert!(result.csp_max <= fit::CSP_LIMIT);
    assert_eq!(
        result.csp_max,
        fit::csp_max(
            &result
                .post_matrix
                .unwrap_or([8192, 0, 0, 0, 8192, 0, 0, 0, 8192])
        )
    );
}

#[test]
fn identity_lut_fits_a_neutral_effect() {
    let cube = parse(&cube_text(17, "", |x| x)).unwrap();
    let options = Options {
        post: false,
        ..Options::default()
    };
    let model = model();
    let target = fit::cube_target(&model, &cube, Encoding::Srgb, 0.0);
    let result = fit::fit(&synthetic_base(), &target, &options).unwrap();
    eprintln!(
        "identity: mean {:.3} max {:.3} matrix {:?}",
        result.mean_de, result.max_de, result.matrix
    );
    assert!(result.mean_de < 1.0, "{result:?}");
    assert!(result.post_matrix.is_none());
    // The synthetic base gamma differs from sRGB near white, so the fit mixes up to about
    // 5% of the channels to follow it. The last digits differ between build profiles.
    let params = result.params();
    assert!(params
        .matrix
        .iter()
        .zip(IDENTITY)
        .all(|(a, b)| (a - b).abs() < 0.06));
}

#[test]
fn import_writes_a_compilable_definition() {
    let text = cube_text(9, "", |x| x.map(|v| v * 0.9 + 0.05));
    let icon = json!({"label": "FG", "shape": "filter"});
    let mut request = Import {
        effect_id: "faded-v1",
        name: "Faded",
        icon: &icon,
        source: "faded.cube",
        input: Some(Encoding::Srgb),
        exposure: -0.5,
        options: Options {
            random_samples: 300,
            rounds: 2,
            joint_rounds: 1,
            post: true,
            sectors: false,
            ..Options::default()
        },
    };
    let imported = fit::import(text.as_bytes(), &synthetic_base(), &request).unwrap();
    let (result, json) = (&imported.fit, &imported.text);
    assert!(imported.warnings.is_empty(), "{:?}", imported.warnings);
    // The file name does not decide the input: a log name gives no warning for a look over a
    // JPEG.
    let named = Import {
        source: "FLog2C_to_Faded_VLog.cube",
        ..request.clone()
    };
    let warnings = fit::import(text.as_bytes(), &synthetic_base(), &named)
        .unwrap()
        .warnings;
    assert!(
        !warnings.iter().any(|w| w.contains("suggests")),
        "{warnings:?}"
    );
    // A LUT made for V-Log warns when it is read as Standard, and not as V-Log.
    let model = model();
    let vlog = cube_text(17, "", |x| model.standard(model.decode(Encoding::Vlog, x)));
    let read_as = |input| Import {
        input: Some(input),
        ..request.clone()
    };
    let loud = fit::import(
        vlog.as_bytes(),
        &synthetic_base(),
        &read_as(Encoding::Standard),
    )
    .unwrap()
    .warnings;
    assert!(
        loud.iter().any(|w| w.contains("data suggests vlog input")),
        "{loud:?}"
    );
    let quiet = fit::import(vlog.as_bytes(), &synthetic_base(), &read_as(Encoding::Vlog))
        .unwrap()
        .warnings;
    assert!(!quiet.iter().any(|w| w.contains("suggests")), "{quiet:?}");
    // A guessed log input asks the user to check it, as another camera's log reads alike.
    let guessed = fit::import(
        vlog.as_bytes(),
        &synthetic_base(),
        &Import {
            input: None,
            ..request.clone()
        },
    )
    .unwrap();
    assert_eq!(guessed.input, Encoding::Vlog);
    assert!(
        guessed
            .warnings
            .iter()
            .any(|w| w.contains("suggests vlog input") && w.contains("S-Log3")),
        "{:?}",
        guessed.warnings
    );
    assert!(!result.monochrome);
    let definition = effects::parse_json(json.as_bytes()).unwrap();
    effects::compile_definition(&definition).unwrap();
    let keys: Vec<&str> = json
        .lines()
        .filter_map(|l| l.strip_prefix("  \"")?.split('"').next())
        .collect();
    let mut expected = vec![
        "schema",
        "effect_id",
        "name",
        "flags",
        "matrix",
        "icon",
        "attribution",
        "curves",
    ];
    if result.post_matrix.is_some() {
        expected.push("post_matrix");
    }
    assert_eq!(keys, expected);
    let attribution = &definition["attribution"];
    assert_eq!(attribution["origin"], "cube-fit");
    assert_eq!(attribution["source"], "faded.cube");
    assert_eq!(attribution["input"], "srgb");
    assert_eq!(attribution["exposure"], -0.5);
    assert_eq!(
        attribution["source_sha256"],
        grkit_core::packaging::sha256(text.as_bytes())
    );
    assert!(attribution["fit"]["mean_de"].as_f64().unwrap() < 2.0);
    assert_eq!(attribution["fit"]["csp_max"], result.csp_max);
    assert_eq!(attribution["fit"]["grade"], fit::Grade::of(result).letter());
    assert!(attribution["fit"]["holdout_mean_de"].as_f64().unwrap() > 0.0);
    assert!(definition.get("glow").is_none());

    // Without an input, the import uses the guess: Standard for a look over a JPEG.
    let guessed = fit::import(
        text.as_bytes(),
        &synthetic_base(),
        &Import {
            input: None,
            ..request.clone()
        },
    )
    .unwrap();
    assert_eq!(guessed.input, Encoding::Standard);
    let bad = json!({"label": "ab", "shape": "filter"});
    request.icon = &bad;
    let failure = format!(
        "{:#}",
        fit::import(text.as_bytes(), &synthetic_base(), &request).unwrap_err()
    );
    assert!(failure.contains("label"), "{failure}");
    request.icon = &icon;
    request.exposure = f64::NAN;
    assert!(fit::import(text.as_bytes(), &synthetic_base(), &request).is_err());
}

/// A short fit without hue sectors; `hue_sectors_follow_a_hue_selective_look` covers them.
fn quick() -> Options {
    Options {
        random_samples: 1500,
        rounds: 4,
        joint_rounds: 2,
        post: true,
        sectors: false,
        ..Options::default()
    }
}

static IMPORT_ICON: LazyLock<Value> = LazyLock::new(|| json!({"label": "IM", "shape": "film"}));

fn import_request(input: Encoding) -> Import<'static> {
    Import {
        effect_id: "imported-v1",
        name: "Imported",
        icon: &IMPORT_ICON,
        source: "imported.cube",
        input: Some(input),
        exposure: 0.0,
        options: quick(),
    }
}

#[test]
fn gray_luts_import_as_monochrome_effects() {
    // A red-filter black and white look with a lifted black.
    let text = cube_text(17, "", |[r, g, b]| {
        [0.08 + 0.9 * (0.6 * r + 0.35 * g + 0.05 * b); 3]
    });
    let imported = fit::import(
        text.as_bytes(),
        &synthetic_base(),
        &import_request(Encoding::Standard),
    )
    .unwrap();
    let fit = &imported.fit;
    eprintln!("mono: {}", brief(fit));
    assert!(fit.monochrome && fit.post_matrix.is_none());
    // The LUT mixes Standard-encoded values, which a linear matrix approximates.
    assert!(fit.mean_de < 3.0);
    let definition = effects::parse_json(imported.text.as_bytes()).unwrap();
    assert_eq!(definition["flags"], effects::MONOCHROME);
    effects::compile_definition(&definition).unwrap();
    // The red weight leads, as the LUT does.
    assert!(fit.matrix[0] > fit.matrix[1] && fit.matrix[1] > fit.matrix[2]);
}

#[test]
fn unrepresentable_luts_fail_clearly_or_fit_loosely() {
    let base = synthetic_base();
    let negative = cube_text(9, "", |x| x.map(|v| 1.0 - v));
    let message = format!(
        "{:#}",
        fit::import(negative.as_bytes(), &base, &import_request(Encoding::Srgb)).unwrap_err()
    );
    assert!(message.contains("inverts brightness"), "{message}");
    // Infrared false color: the channels trade places, beyond what one matrix plus curves fit.
    let infrared = cube_text(17, "", |[r, g, b]| {
        let swap = [b, r, g];
        let gray = (r + g + b) / 3.0;
        swap.map(|v| (4.0 * (v - gray) + gray).clamp(0.0, 1.0))
    });
    let imported =
        fit::import(infrared.as_bytes(), &base, &import_request(Encoding::Srgb)).unwrap();
    eprintln!("infrared: {} {:?}", brief(&imported.fit), imported.warnings);
    let limit = (fit::MATRIX_LIMIT * 8192.0) as i16 + 2;
    assert!(imported.fit.matrix.iter().all(|v| v.abs() <= limit));
    // The matrix and the post matrix follow part of the swap, so the fit is loose.
    let grade = fit::Grade::of(&imported.fit);
    assert!(matches!(grade, fit::Grade::C | fit::Grade::D), "{grade:?}");
    let warning = fit::grade_warning(&imported.fit).unwrap();
    assert!(
        warning.starts_with("Grade C") || warning.starts_with("Grade D"),
        "{warning}"
    );
}

#[test]
fn toned_luts_share_one_matrix_row_and_no_post_matrix() {
    // Sepia over a red-filter black and white look.
    let text = cube_text(17, "", |[r, g, b]| {
        let y = 0.05 + 0.9 * (0.6 * r + 0.35 * g + 0.05 * b);
        [y.powf(0.9), y, y.powf(1.15)]
    });
    let imported = fit::import(
        text.as_bytes(),
        &synthetic_base(),
        &import_request(Encoding::Standard),
    )
    .unwrap();
    let fit = &imported.fit;
    eprintln!("toned: {}", brief(fit));
    assert!(fit.toned && !fit.monochrome && fit.post_matrix.is_none());
    assert!(fit.matrix[..3] == fit.matrix[3..6] && fit.matrix[..3] == fit.matrix[6..]);
    assert!(fit.curves[0] != fit.curves[2]);
    assert!(fit.mean_de < 3.0);
    let definition = effects::parse_json(imported.text.as_bytes()).unwrap();
    assert_eq!(definition["flags"], 0);
    effects::compile_definition(&definition).unwrap();
}

#[test]
fn noisy_lut_images_warn() {
    // A level 8 identity, as editors export one, with grain of about 2 / 255. A coarser table
    // is allowed more curvature (`lut_image::noise_limit`).
    let mut state = 7u32;
    let mut grain = || {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (state >> 24) as i32 % 5 - 2
    };
    let data: Vec<u8> = (0..512 * 512)
        .flat_map(|i: i32| [i % 64, i / 64 % 64, i / 4096].map(|v| (v * 255 + 31) / 63))
        .map(|v| (v + grain()).clamp(0, 255) as u8)
        .collect();
    let warnings = fit::import(
        &png(512, 512, RGB, 8, &data),
        &synthetic_base(),
        &import_request(Encoding::Standard),
    )
    .unwrap()
    .warnings;
    assert!(warnings.iter().any(|w| w.contains("noisy")), "{warnings:?}");
}

#[test]
fn hue_sector_fields_round_trip() {
    use grkit_core::hue::{self, HueSectors};
    // Every stored weight survives the control fields.
    for w in 0..=255u8 {
        let sectors = HueSectors {
            matrices: [[0; 6]; hue::SECTORS],
            weight: [w, w, w / 2, 255],
            levels: None,
        };
        assert_eq!(
            HueSectors::from_bytes(&sectors.to_bytes(), None).unwrap(),
            sectors
        );
        assert_eq!(HueSectors::from_json(&sectors.to_json()).unwrap(), sectors);
    }
    // Highlight correction doubles the slopes, so a weight that falls from 1 to 0 over the
    // first segment leaves the 9-bit field and fails.
    let steep = HueSectors {
        matrices: [[0; 6]; hue::SECTORS],
        weight: [255, 0, 0, 0],
        levels: None,
    };
    assert!(!steep.slopes_double());
    let error = HueSectors::from_json(&steep.to_json())
        .unwrap_err()
        .to_string();
    assert!(error.contains("highlight correction"), "{error}");
    assert!(HueSectors::from_bytes(&steep.to_bytes(), None).is_err());
    // The weight curve passes through its knots.
    let stage = HueSectors {
        matrices: [[0; 6]; hue::SECTORS],
        weight: [0, 51, 204, 255],
        levels: None,
    }
    .stage();
    for (y, w) in hue::KNOTS.iter().zip([0.0, 0.2, 0.8, 1.0]) {
        assert!(
            (stage.curve.weight(*y) - w).abs() < 0.01,
            "{y}: {}",
            stage.curve.weight(*y)
        );
    }
    // The bounds and selectors give 11 equal sectors of the joined coordinate.
    for x in 0..8192i32 {
        assert_eq!(
            hue::bank_at(f64::from(x)),
            x as usize * hue::SECTORS / 8192 + 1,
            "{x}"
        );
    }
    // The coordinate and its direction agree, and gray has none.
    for x in [0.0, 700.0, 2048.0, 3000.0, 5000.0, 8000.0] {
        let (cb, cr) = hue::direction(x);
        let stage: [[f64; 3]; 3] = hue::STAGE.map(|r| r.map(|v| v / 256.0));
        let rgb = cube::mul(&cube::inverse(&stage).unwrap(), [0.3, 0.05 * cb, 0.05 * cr]);
        // The coordinate is circular: 0 and 8192 are the same direction.
        let d = (hue::coordinate(rgb).unwrap() - x).rem_euclid(8192.0);
        assert!(d.min(8192.0 - d) < 1e-6, "{x}");
    }
    assert_eq!(hue::coordinate([0.4; 3]), None);
    // Levels round-trip, and the stage blends them along the level curve: level 0 below
    // the curve, level 3 (and the repeated level 4) above it.
    let mut levels = [[[0i8; 6]; hue::SECTORS]; hue::LEVELS - 1];
    for (l, level) in levels.iter_mut().enumerate() {
        level[2] = [10 * (l as i8 + 1), -5, 0, 0, 0, 0];
    }
    let sectors = HueSectors {
        matrices: [[0; 6]; hue::SECTORS],
        weight: [255; 4],
        levels: Some(levels),
    };
    let bytes = sectors.level_bytes().unwrap();
    assert_eq!(
        HueSectors::from_bytes(&sectors.to_bytes(), Some(&bytes)).unwrap(),
        sectors
    );
    assert_eq!(HueSectors::from_json(&sectors.to_json()).unwrap(), sectors);
    let stage = sectors.stage();
    assert_eq!(stage.matrix(2, 0.01), hue::matrix(&[0.0; 6]));
    let top = hue::matrix(&[30.0 / 128.0, -5.0 / 128.0, 0.0, 0.0, 0.0, 0.0]);
    assert_eq!(stage.matrix(2, 0.9), top);
    let position = hue::level_position(0.2);
    assert!(position > 1.0 && position < 2.0, "{position}");
    let mid = stage.matrix(2, 0.2)[1];
    let (a, b) = (10.0 / 128.0, 20.0 / 128.0);
    assert!(
        (mid - (a + (position - 1.0) * (b - a))).abs() < 1e-12,
        "{mid}"
    );
    let bad = json_value(r#"{"matrices": [[-100, -100, 0, 0, 0, 0]], "weight": [0, 0, 0, 0]}"#);
    assert!(HueSectors::from_json(&bad).is_err());
}

fn json_value(text: &str) -> serde_json::Value {
    let mut value: serde_json::Value = serde_json::from_str(text).unwrap();
    // Pad the matrices to 11 sectors with zeros.
    let matrices = value["matrices"].as_array_mut().unwrap();
    while matrices.len() < 11 {
        matrices.push(serde_json::json!([0, 0, 0, 0, 0, 0]));
    }
    value
}

/// A Standard-input look that turns greens toward cyan and saturates reds: a change by hue
/// that one matrix and three curves cannot make.
fn hue_selective(x: [f64; 3]) -> [f64; 3] {
    let y = 0.299 * x[0] + 0.587 * x[1] + 0.114 * x[2];
    let (cb, cr) = ((x[2] - y) / 1.772, (x[0] - y) / 1.402);
    let (chroma, angle) = (cb.hypot(cr), cr.atan2(cb));
    let near = |center: f64| ((angle - center).cos().max(0.0)).powi(4);
    // Green lies near -2.2 rad in this Cb/Cr plane, red near 1.85 rad.
    let angle = angle + 0.45 * near(-2.2);
    let chroma = chroma * (1.0 + 0.35 * near(1.85));
    let (cb, cr) = (chroma * angle.cos(), chroma * angle.sin());
    let (r, b) = (y + 1.402 * cr, y + 1.772 * cb);
    let g = (y - 0.299 * r - 0.114 * b) / 0.587;
    [r, g, b].map(|v| v.clamp(0.0, 1.0))
}

#[test]
fn hue_sectors_follow_a_hue_selective_look() {
    let cube = parse(&cube_text(33, "", hue_selective)).unwrap();
    let model = model();
    let target = fit::cube_target(&model, &cube, Encoding::Standard, 0.0);
    let options = Options {
        random_samples: 2500,
        sector_rounds: 2,
        sector_samples: 1500,
        ..quick()
    };
    let plain = fit::fit(&synthetic_base(), &target, &options).unwrap();
    let sectored = fit::fit(
        &synthetic_base(),
        &target,
        &Options {
            sectors: true,
            ..options.clone()
        },
    )
    .unwrap();
    eprintln!(
        "hue-selective: holdout {:.3} -> {:.3}, sectors {:?}",
        plain.holdout_mean_de, sectored.holdout_mean_de, sectored.sectors
    );
    let sectors = sectored.sectors.expect("the fit keeps hue sectors");
    assert!(plain.sectors.is_none());
    assert!(sectored.holdout_mean_de < 0.85 * plain.holdout_mean_de);
    // The effect.json, the GRF, and the site parameters carry the same stage.
    let mut request = import_request(Encoding::Standard);
    request.options = Options {
        sectors: true,
        ..options
    };
    let text = cube_text(33, "", hue_selective);
    let imported = fit::import(text.as_bytes(), &synthetic_base(), &request).unwrap();
    let definition = effects::parse_json(imported.text.as_bytes()).unwrap();
    assert!(definition.get("hue_sectors").is_some());
    let blob = effects::compile_definition(&definition).unwrap();
    assert_eq!(blob[84] & 8, 8);
    let (effect, _) = effects::decode(&blob).unwrap();
    let params = grkit_core::site::params(&effect);
    assert_eq!(params, imported.fit.params());
    assert_eq!(params.sectors, Some(imported.fit.sectors.unwrap().stage()));
    let _ = sectors;
}

/// `hue_selective` with a film-like shoulder: colors fade toward white as they get brighter.
fn fading_highlights(x: [f64; 3]) -> [f64; 3] {
    let v = hue_selective(x);
    let y = 0.299 * v[0] + 0.587 * v[1] + 0.114 * v[2];
    let fade = ((y - 0.55) / 0.45).clamp(0.0, 1.0).powi(2);
    v.map(|c| c + fade * (1.0 - c) * 0.8)
}

#[test]
fn hue_sectors_keep_colors_brightening_with_exposure() {
    let cube = parse(&cube_text(33, "", fading_highlights)).unwrap();
    let model = model();
    let target = fit::cube_target(&model, &cube, Encoding::Standard, 0.0);
    let options = Options {
        random_samples: 2500,
        sectors: true,
        sector_rounds: 2,
        sector_samples: 1500,
        ..quick()
    };
    let fit = fit::fit(&synthetic_base(), &target, &options).unwrap();
    let sectors = fit.sectors.expect("the fit keeps hue sectors");
    let params = fit.params();
    let lab = LabConverter::default();
    // Saturated colors of every hue, from -2 to +4 EV: L* may not fall as the exposure grows.
    let mut worst = (0.0f64, 0.0, 0.0);
    for k in 0..36 {
        let hue = std::f64::consts::TAU * k as f64 / 36.0;
        for (l, chroma) in [(30.0, 40.0), (50.0, 50.0), (70.0, 40.0)] {
            let base = cube::lab_d50_to_srgb([l, chroma * hue.cos(), chroma * hue.sin()]);
            if base.iter().any(|v| !(0.0..=1.0).contains(v)) {
                continue;
            }
            let mut peak = f64::MIN;
            for step in 0..=48 {
                let ev = -2.0 + 0.125 * step as f64;
                let lightness = lab.lab(model.render(&params, base.map(|v| v * 2f64.powf(ev))))[0];
                peak = peak.max(lightness);
                if peak - lightness > worst.0 {
                    worst = (peak - lightness, hue.to_degrees(), ev);
                }
            }
        }
    }
    eprintln!(
        "largest L* fall {:.2} at hue {:.0} deg, {:+.2} EV",
        worst.0, worst.1, worst.2
    );
    assert!(worst.0 < 1.0, "L* falls by {:.2} with exposure", worst.0);
    // The weight curve has no step at its last breakpoint, which bright colors cross.
    let curve = sectors.stage().curve;
    let span = f64::from(hue::BORDERS[2] - hue::BORDERS[1]);
    let end = curve.starts[2] + curve.slopes[2] * span / 32.0;
    assert!((curve.starts[3] - end).abs() <= 8.0, "{curve:?}");
}

#[test]
fn flaws_measure_only_what_the_lut_does_not_do() {
    let model = model();
    let params = known([1.1, -0.05, -0.05, -0.05, 1.1, -0.05, -0.05, -0.05, 1.1]);
    let itself = |x: [f64; 3]| model.render(&params, x);
    let flaws = fit::flaws(&model, &params, &itself);
    assert!(
        flaws.fall.abs() < 1e-9 && flaws.lag.abs() < 1e-9 && flaws.step.abs() < 1e-9,
        "{flaws:?}"
    );
    // A target that keeps saturated reds brightening, against an effect that darkens them as
    // the exposure grows.
    let mut darker = params.clone();
    darker.post = [1.0, 0.0, 0.0, -1.5, 1.0, 0.0, -1.5, 0.0, 1.0];
    let flaws = fit::flaws(&model, &darker, &itself);
    assert!(flaws.fall > fit::FALL_WARNING, "{flaws:?}");
    // An effect that saturates every color more than the target: bright saturated colors stay
    // darker than the target as they brighten, so the effect lags behind it.
    let mut vivid = params.clone();
    vivid.post = [1.8, -0.4, -0.4, -0.4, 1.8, -0.4, -0.4, -0.4, 1.8];
    let flaws = fit::flaws(&model, &vivid, &itself);
    assert!(flaws.lag > 5.0, "{flaws:?}");
}

#[test]
fn import_reports_progress_that_rises_to_one() {
    // A warm look, so the fit runs the hue sectors and the joint refinement.
    let text = cube_text(9, "", |x| {
        [x[0] * 0.95 + 0.04, x[1] * 0.9 + 0.02, x[2] * 0.75]
    });
    let icon = json!({"label": "WM", "shape": "filter"});
    let request = Import {
        effect_id: "warm-v1",
        name: "Warm",
        icon: &icon,
        source: "warm.cube",
        input: Some(Encoding::Standard),
        exposure: 0.0,
        options: Options {
            random_samples: 300,
            rounds: 2,
            joint_rounds: 1,
            sector_rounds: 1,
            sector_samples: 200,
            joint_passes: 1,
            ..Options::default()
        },
    };
    let reports = std::cell::RefCell::new(Vec::new());
    fit::import_with_progress(text.as_bytes(), &synthetic_base(), &request, &|fraction| {
        reports.borrow_mut().push(fraction)
    })
    .unwrap();
    let reports = reports.into_inner();
    assert!(
        reports.windows(2).all(|pair| pair[0] <= pair[1]),
        "{reports:?}"
    );
    assert!(
        reports.iter().all(|f| (0.0..=1.0).contains(f)),
        "{reports:?}"
    );
    assert_eq!(reports.last(), Some(&1.0));
    // The hue sectors take most of the time, so they report steps inside their span.
    assert!(
        reports.iter().filter(|f| (0.1..0.7).contains(*f)).count() >= 2,
        "{reports:?}"
    );
}
