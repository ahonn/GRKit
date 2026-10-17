use grkit_core::cube::{self, Cube, Encoding};
use grkit_core::effects;
use grkit_core::fit::{self, Import, Model, Options, Params, IDENTITY};
use std::fmt::Write;

fn cube_text(size: usize, header: &str, map: impl Fn([f64; 3]) -> [f64; 3]) -> String {
    let mut text = format!("# synthetic\nTITLE \"Synthetic #1\"\nLUT_3D_SIZE {size}\n{header}\n");
    let last = (size - 1) as f64;
    for b in 0..size {
        for g in 0..size {
            for r in 0..size {
                let out = map([r as f64 / last, g as f64 / last, b as f64 / last]);
                writeln!(text, "{:.6} {:.6} {:.6}", out[0], out[1], out[2]).unwrap();
            }
        }
    }
    text
}

fn parse(text: &str) -> anyhow::Result<Cube> {
    Cube::parse(text.as_bytes())
}

fn error(text: &str) -> String {
    format!("{:#}", parse(text).unwrap_err())
}

#[test]
fn parses_3d_tables_with_comments_and_trilinear_lookup() {
    let text = cube_text(2, "# corner order: red fastest", |rgb| rgb.map(|v| v * 0.5));
    let cube = parse(&format!("\u{feff}{text}\n# trailing comment\n")).unwrap();
    assert!(cube.three_d);
    assert_eq!(
        (cube.size, cube.title.as_deref()),
        (2, Some("Synthetic #1"))
    );
    assert_eq!(cube.table[1], [0.5, 0.0, 0.0]);
    assert_eq!(cube.table[2], [0.0, 0.5, 0.0]);
    assert_eq!(cube.table[4], [0.0, 0.0, 0.5]);
    let out = cube.apply([0.25, 0.5, 1.5]);
    for (actual, expected) in out.iter().zip([0.125, 0.25, 0.5]) {
        assert!((actual - expected).abs() < 1e-7, "{out:?}");
    }
}

#[test]
fn parses_1d_tables_and_domains() {
    let text =
        "LUT_1D_SIZE 3\nDOMAIN_MIN 0 0 -1\nDOMAIN_MAX 2 2 1\n0 0 0\n0.2 0.4 0.6 # middle\n1 1 1\n";
    let cube = parse(text).unwrap();
    assert!(!cube.three_d);
    let out = cube.apply([1.0, 0.5, 0.0]);
    for (actual, expected) in out.iter().zip([0.2, 0.2, 0.6]) {
        assert!((actual - expected).abs() < 1e-6, "{out:?}");
    }
    assert_eq!(cube.apply([3.0, -1.0, 9.0]), [1.0, 0.0, 1.0]);
    let ranged = parse("LUT_1D_SIZE 2\nLUT_1D_INPUT_RANGE 0 4\n0 0 0\n1 1 1\n").unwrap();
    assert!((ranged.apply([1.0; 3])[0] - 0.25).abs() < 1e-9);
    let shaped = parse(&cube_text(
        2,
        "DOMAIN_MIN -1 -1 -1\nDOMAIN_MAX 1 1 1",
        |v| v,
    ))
    .unwrap();
    assert!((shaped.apply([0.0; 3])[1] - 0.5).abs() < 1e-9);
}

#[test]
fn rejects_malformed_tables() {
    let rows = "0 0 0\n1 0 0\n0 1 0\n1 1 0\n0 0 1\n1 0 1\n0 1 1\n";
    for (text, message) in [
        (
            format!("LUT_3D_SIZE 2\n{rows}"),
            "Expected 8 data rows, found 7",
        ),
        (
            format!("LUT_3D_SIZE 2\n{rows}1 1\n"),
            "Line 9: A data row expects 3 values",
        ),
        (
            format!("LUT_3D_SIZE 2\n{rows}1 1 1e999\n"),
            "Line 9: expected a finite number",
        ),
        (
            format!("LUT_3D_SIZE 2\n{rows}1 1 NaN\n"),
            "Line 9: expected a finite number",
        ),
        (
            format!("LUT_3D_SIZE 2\n{rows}1 1 x\n"),
            "Line 9: expected a finite number",
        ),
        (
            "LUT_3D_SIZE 1\n0 0 0\n".into(),
            "LUT_3D_SIZE must be 2 to 256",
        ),
        ("LUT_3D_SIZE 257\n".into(), "LUT_3D_SIZE must be 2 to 256"),
        (
            "LUT_3D_SIZE 2\nLUT_3D_SIZE 2\n".into(),
            "duplicate LUT_3D_SIZE",
        ),
        ("LUT_3D_SIZE 2\nLUT_1D_SIZE 2\n".into(), "both 1D and 3D"),
        ("0 0 0\n".into(), "Expected LUT_3D_SIZE or LUT_1D_SIZE"),
        (
            "LUT_1D_SIZE 2\n0 0 0\nDOMAIN_MIN 0 0 0\n1 1 1\n".into(),
            "after data rows",
        ),
        (
            "LUT_1D_SIZE 2\nDOMAIN_MAX 0 1 1\n0 0 0\n1 1 1\n".into(),
            "domain minimum below",
        ),
        (
            "LUT_1D_SIZE 2\nDOMAIN_MIN 0 0\n".into(),
            "DOMAIN_MIN expects 3 values",
        ),
        (
            "LUT_1D_SIZE 2\nLUT_3D_INPUT_RANGE 0 1\n".into(),
            "does not match",
        ),
        (
            "LUT_1D_SIZE 2\nLUT_IN_VIDEO_RANGE\n".into(),
            "unsupported keyword LUT_IN_VIDEO_RANGE",
        ),
        (
            "TITLE unquoted\nLUT_1D_SIZE 2\n".into(),
            "expected a quoted TITLE",
        ),
    ] {
        let actual = error(&text);
        assert!(actual.contains(message), "{actual:?} lacks {message:?}");
    }
    assert!(Cube::parse(b"LUT_1D_SIZE 2\n\xff\n").is_err());
}

#[test]
fn color_transforms_match_published_values() {
    for v in [0.0, 0.002, 0.18, 0.5, 1.0] {
        assert!((cube::srgb_decode(cube::srgb_encode(v)) - v).abs() < 1e-12);
    }
    // V-Log: 0 % reflectance at code 128 / 1023 full range, 18 % at 433 / 1023.
    assert!((cube::vlog_encode(0.0) - 0.125).abs() < 1e-12);
    assert!((cube::vlog_encode(0.18) * 1023.0 - 433.0).abs() < 0.5);
    for v in [0.0, 0.005, 0.18, 4.0] {
        assert!((cube::vlog_decode(cube::vlog_encode(v)) - v).abs() < 1e-9);
    }
    let gray = cube::mul(&cube::srgb_to_vgamut(), [0.18; 3]);
    assert!(gray.iter().all(|v| (v - 0.18).abs() < 1e-9));
    let red = cube::mul(&cube::srgb_to_vgamut(), [1.0, 0.0, 0.0]);
    assert!(red[0] < 1.0 && red[1] > 0.0 && red[2] > 0.0);
    // Sharma, Wu, and Dalal (2005) test pairs 1, 7, and 17.
    for (a, b, expected) in [
        ([50.0, 2.6772, -79.7751], [50.0, 0.0, -82.7485], 2.0425),
        ([50.0, 0.0, 0.0], [50.0, -1.0, 2.0], 2.3669),
        ([50.0, 2.5, 0.0], [73.0, 25.0, -18.0], 27.1492),
    ] {
        assert!((cube::delta_e2000(a, b) - expected).abs() < 1e-4);
    }
    let chart = fit::colorchecker();
    let white = cube::LabConverter::default().lab(chart[18].map(cube::srgb_encode));
    assert!((white[0] - 95.19).abs() < 0.5 && white[1].abs() < 1.5 && white[2].abs() < 3.5);
}

/// An analytic stand-in for the firmware table: sRGB over a Reinhard shoulder in the linear
/// index domain, with 18% gray near 0.46 at index 7.8 and white at index 255.
fn synthetic_base() -> [u16; 256] {
    let scale = 1.2 * 0.18 / fit::MID_INDEX;
    let white = 255.0 * scale;
    std::array::from_fn(|i| {
        let x = i as f64 * scale;
        let shoulder = x * (1.0 + x / (white * white)) / (1.0 + x);
        (16384.0 * cube::srgb_encode(shoulder)).round() as u16
    })
}

fn model() -> Model {
    Model::new(&synthetic_base())
}

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
        matrix: [0.9, 0.12, -0.02, 0.03, 0.92, 0.05, -0.01, 0.06, 0.95],
        curves: [curve(3.0, 160.0), curve(2.8, 96.0), curve(3.3, 240.0)],
        post,
    }
}

const POST: [f64; 9] = [1.08, -0.05, -0.03, -0.04, 1.06, -0.02, -0.02, -0.06, 1.08];

fn synthetic_cube(input: Encoding, size: usize, params: &Params) -> Cube {
    let model = model();
    parse(&cube_text(size, "", |x| {
        model.render(params, input.decode(x))
    }))
    .unwrap()
}

fn fit_cube(input: Encoding, size: usize, params: &Params) -> fit::Fit {
    let cube = synthetic_cube(input, size, params);
    let target = fit::cube_target(&cube, input, 0.0);
    fit::fit(&synthetic_base(), &target, &Options::default()).unwrap()
}

/// The chart error of the known parameters against their own LUT: the interpolation floor.
fn floor(input: Encoding, size: usize, params: &Params) -> (f64, f64) {
    let (model, lab) = (model(), cube::LabConverter::default());
    let cube = synthetic_cube(input, size, params);
    let target = fit::cube_target(&cube, input, 0.0);
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
    assert!(matrix < 0.02 && post < 0.02 && curve < 100.0);
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
    let result = fit_cube(Encoding::Vlog, 17, &known(saturated));
    let post = result.post_matrix.unwrap();
    eprintln!(
        "saturated: csp {} mean {:.3} post {post:?}",
        result.csp_max, result.mean_de
    );
    assert!(result.csp_max <= fit::CSP_LIMIT && result.csp_max >= 200);
    assert_eq!(result.csp_max, fit::csp_max(&post));
    assert!(post
        .chunks(3)
        .all(|row| row.iter().map(|v| *v as i32).sum::<i32>() == 8192));
}

#[test]
fn identity_lut_fits_a_neutral_effect() {
    let cube = parse(&cube_text(17, "", |x| x)).unwrap();
    let options = Options {
        post: false,
        ..Options::default()
    };
    let target = fit::cube_target(&cube, Encoding::Srgb, 0.0);
    let result = fit::fit(&synthetic_base(), &target, &options).unwrap();
    eprintln!(
        "identity: mean {:.3} max {:.3} matrix {:?}",
        result.mean_de, result.max_de, result.matrix
    );
    assert!(result.mean_de < 1.0, "{result:?}");
    assert!(result.post_matrix.is_none());
    let params = result.params();
    assert!(params
        .matrix
        .iter()
        .zip(IDENTITY)
        .all(|(a, b)| (a - b).abs() < 0.03));
}

#[test]
fn import_writes_a_compilable_definition() {
    let text = cube_text(9, "", |x| x.map(|v| v * 0.9 + 0.05));
    let mut request = Import {
        effect_id: "faded-v1",
        name: "Faded",
        label: "FG",
        shape: "filter",
        source: "faded.cube",
        input: Encoding::Srgb,
        exposure: -0.5,
        options: Options {
            random_samples: 300,
            rounds: 2,
            joint_rounds: 1,
            post: true,
        },
    };
    let (result, json) = fit::import(text.as_bytes(), &synthetic_base(), &request).unwrap();
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
    assert!(definition.get("glow").is_none());

    request.label = "ab";
    let failure = format!(
        "{:#}",
        fit::import(text.as_bytes(), &synthetic_base(), &request).unwrap_err()
    );
    assert!(failure.contains("label"), "{failure}");
    request.label = "FG";
    request.exposure = f64::NAN;
    assert!(fit::import(text.as_bytes(), &synthetic_base(), &request).is_err());
}
