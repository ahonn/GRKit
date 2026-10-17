use grkit_core::{cube, effects, fit, packaging, site};
use serde_json::{json, Value};
use std::{fs, path::PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../effects")
}

fn definition(id: &str) -> Value {
    effects::parse_json(&fs::read(root().join(id).join("effect.json")).unwrap()).unwrap()
}

/// The analytic stand-in for the firmware table from the import tests.
fn synthetic_base() -> [u16; 256] {
    let scale = 1.2 * 0.18 / fit::MID_INDEX;
    let white = 255.0 * scale;
    std::array::from_fn(|i| {
        let x = i as f64 * scale;
        let shoulder = x * (1.0 + x / (white * white)) / (1.0 + x);
        (16384.0 * cube::srgb_encode(shoulder)).round() as u16
    })
}

/// A selective color definition that keeps only red. No published effect uses the feature.
fn keep_red() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/keep-red.json");
    effects::parse_json(&fs::read(path).unwrap()).unwrap()
}

fn decoded(definition: &Value) -> Value {
    effects::decode(&effects::compile_definition(definition).unwrap())
        .unwrap()
        .0
}

fn entry(table: &[u8], r: usize, g: usize, b: usize) -> [u8; 3] {
    let index = ((b * site::LUT_SIZE + g) * site::LUT_SIZE + r) * 3;
    table[index..index + 3].try_into().unwrap()
}

#[test]
fn standard_preview_returns_each_input() {
    let curve: Vec<u16> = (0..256).map(|v| v * 64).collect();
    let standard = json!({"effect_id": "standard-v1", "name": "Standard", "flags": 0,
        "matrix": fit::STANDARD_LOOK, "curves": [&curve, &curve, &curve]});
    let table = site::preview_lut(&synthetic_base(), &standard).unwrap();
    assert_eq!(table.len(), site::LUT_SIZE.pow(3) * 3);
    let level = |i: usize| (i as f64 * 255.0 / (site::LUT_SIZE - 1) as f64).round() as i32;
    for b in 0..site::LUT_SIZE {
        for g in 0..site::LUT_SIZE {
            for r in 0..site::LUT_SIZE {
                let actual = entry(&table, r, g, b);
                for (value, expected) in actual.iter().zip([level(r), level(g), level(b)]) {
                    assert!(
                        (*value as i32 - expected).abs() <= 1,
                        "{r} {g} {b}: {actual:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn effect_previews_follow_the_effect() {
    let base = synthetic_base();
    let mono = site::preview_lut(&base, &decoded(&definition("red-filter-mono-v1"))).unwrap();
    assert!(mono
        .chunks_exact(3)
        .all(|rgb| rgb[0] == rgb[1] && rgb[1] == rgb[2]));
    let ektar = site::preview_lut(&base, &decoded(&definition("ektar-100-edge-v1"))).unwrap();
    let gray = |table: &[u8], i| entry(table, i, i, i);
    // Brighter input stays brighter, and the film look differs from Standard.
    for i in 1..site::LUT_SIZE {
        assert!(gray(&ektar, i)[1] >= gray(&ektar, i - 1)[1]);
    }
    let level = |i: usize| (i * 255 / (site::LUT_SIZE - 1)) as i32;
    let red = entry(&ektar, 28, 6, 4);
    let shift = (red[0] as i32 - level(28)).abs() + (red[1] as i32 - level(6)).abs();
    assert!(shift > 8, "{red:?}");
}

#[test]
fn published_effects_match_their_definitions() {
    let ids = ["ektar-100-edge-v1", "red-filter-mono-v1"];
    let definitions: Vec<Value> = ids.iter().map(|id| definition(id)).collect();
    let files = site::publish_effects(&definitions, &synthetic_base(), "gr3-2.10").unwrap();
    let catalog = effects::parse_json(&files["catalog.json"]).unwrap();
    assert_eq!(catalog["schema"], site::CATALOG_SCHEMA);
    assert_eq!(catalog["target"], "gr3-2.10");
    assert_eq!(catalog["preview"]["size"], site::LUT_SIZE);
    let entries = catalog["effects"].as_array().unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(files.len(), 1 + 2 * entries.len());
    for (entry, definition) in entries.iter().zip(&definitions) {
        let id = entry["id"].as_str().unwrap();
        let blob = &files[entry["file"].as_str().unwrap()];
        assert_eq!(entry["file"], effects::filename(id).unwrap());
        assert_eq!(blob, &effects::compile_definition(definition).unwrap());
        assert_eq!(entry["sha256"], packaging::sha256(blob));
        assert_eq!(entry["name"], definition["name"]);
        assert_eq!(entry["label"], definition["icon"]["label"]);
        assert_eq!(entry["look"], definition["attribution"]["target"]);
        assert_eq!(
            files[entry["preview"].as_str().unwrap()].len(),
            site::LUT_SIZE.pow(3) * 3
        );
    }
    assert_eq!(entries[0]["monochrome"], false);
    assert_eq!(entries[1]["monochrome"], true);
    assert_eq!(entries[1]["shape"], "filter");

    let duplicate = [definitions[0].clone(), definitions[0].clone()];
    let error = site::publish_effects(&duplicate, &synthetic_base(), "gr3-2.10").unwrap_err();
    assert!(error.to_string().contains("Duplicate effect ID"), "{error}");
    assert!(site::publish_effects(&[], &synthetic_base(), "gr3-2.10").is_err());
}

#[test]
fn selective_color_previews_keep_red_and_gray_other_hues() {
    let base = synthetic_base();
    let accent = site::preview_lut(&base, &decoded(&keep_red())).unwrap();
    let mut standard = decoded(&keep_red());
    standard.as_object_mut().unwrap().remove("selective_color");
    let plain = site::preview_lut(&base, &standard).unwrap();
    let last = site::LUT_SIZE - 1;
    // Saturated red keeps its color; green and blue turn neutral.
    let red = entry(&accent, last, 0, 0);
    assert!(red[0] as i32 - red[1] as i32 > 100, "{red:?}");
    for rgb in [entry(&accent, 0, last, 0), entry(&accent, 0, 0, last)] {
        assert!(
            rgb.iter().max().unwrap() - rgb.iter().min().unwrap() <= 2,
            "{rgb:?}"
        );
    }
    // The gray ramp has no chroma to keep or remove. The 8-bit YCbCr round trip,
    // as in a camera JPEG, can move a channel by one level.
    for i in 0..site::LUT_SIZE {
        let (a, p) = (entry(&accent, i, i, i), entry(&plain, i, i, i));
        assert!(
            a.iter().zip(p).all(|(a, p)| a.abs_diff(p) <= 1),
            "{a:?} {p:?}"
        );
    }
}
