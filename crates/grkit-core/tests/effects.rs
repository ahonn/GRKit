use grkit_core::{artwork, effects};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../effects")
}

fn fixture() -> Value {
    let curve: Vec<u16> = (0..256).map(|v| v * 64).collect();
    json!({"effect_id": "custom-v1", "name": "Custom", "flags": 0,
        "matrix": [8192, 0, 0, 0, 8192, 0, 0, 0, 8192], "curves": [&curve, &curve, &curve]})
}

fn read_json(path: impl AsRef<std::path::Path>) -> Value {
    effects::parse_json(&fs::read(path).unwrap()).unwrap()
}

#[test]
fn starter_definitions_match_existing_golden_bytes() {
    let expected = read_json(root().join("golden.json"));
    let mut actual = serde_json::Map::new();
    for entry in fs::read_dir(root()).unwrap() {
        let path = entry.unwrap().path().join("effect.json");
        if !path.is_file() {
            continue;
        }
        let definition = read_json(path);
        let bytes = effects::compile_definition(&definition).unwrap();
        let filename = effects::filename(definition["effect_id"].as_str().unwrap()).unwrap();
        actual.insert(filename, hex::encode(Sha256::digest(&bytes)).into());
        let (_, icon) = effects::decode(&bytes).unwrap();
        assert_eq!(
            hex::encode(Sha256::digest(icon)),
            expected["icons"][definition["effect_id"].as_str().unwrap()]
        );
    }
    assert_eq!(Value::Object(actual), expected["grf"]);
    assert_eq!(
        hex::encode(Sha256::digest(artwork::off_pixels())),
        expected["off"]
    );
}

#[test]
fn host_artwork_matches_golden_pixels_and_preserves_binary_transparency() {
    assert_eq!(
        hex::encode(Sha256::digest(artwork::plugins_pixels())),
        "994c7df1e812485f987680a32ff1e4d02fbd49ff21039a098f7f849de7709e25"
    );
    for pixels in [
        artwork::plugins_pixels(),
        artwork::framing_pixels(),
        artwork::off_pixels(),
        artwork::render_tile("OFF", "film").unwrap(),
        artwork::render_tile("OFF", "filter").unwrap(),
        artwork::render_tile("OFF", "plain").unwrap(),
    ] {
        assert_eq!(pixels.len(), effects::ICON_SIZE);
        assert!(pixels[..160].iter().chain(&pixels[6240..]).all(|v| *v == 0));
        assert!(pixels
            .iter()
            .skip(3)
            .step_by(4)
            .all(|v| *v == 0 || *v == 255));
    }
}

#[test]
fn user_definition_preserves_edited_name_and_creative_curve() {
    let mut definition = read_json(root().join("ektar-100-endura-v3/effect.json"));
    definition["name"] = json!("My Ektar");
    definition["curves"][0][100] = json!(0);
    let (compiled, _) =
        effects::decode(&effects::compile_definition(&definition).unwrap()).unwrap();
    assert_eq!(compiled["name"], "My Ektar");
    assert_eq!(compiled["curves"], definition["curves"]);
    assert_eq!(compiled["matrix"], definition["matrix"]);
    assert_eq!(compiled["effect_id"], definition["effect_id"]);
}

#[test]
fn real_fit_imports_reproduce_editable_definitions() {
    for entry in fs::read_dir(root()).unwrap() {
        let path = entry.unwrap().path().join("effect.json");
        if !path.is_file() {
            continue;
        }
        let definition = read_json(path);
        if definition["attribution"]["origin"] != "derived-fit" {
            continue;
        }
        let fit = read_json(
            root()
                .join("sources")
                .join(definition["attribution"]["source"].as_str().unwrap()),
        );
        assert_eq!(
            effects::import_fit(&fit).unwrap(),
            json!({"matrix": definition["matrix"], "curves": definition["curves"]})
        );
    }
}

#[test]
fn round_trip_preserves_layout_and_nonmonotone_curves() {
    let mut effect = fixture();
    effect["curves"][0][100] = json!(0);
    let icon = vec![0; effects::ICON_SIZE];
    let bytes = effects::encode(&effect, &icon).unwrap();
    assert_eq!(bytes.len(), 8192);
    assert_eq!(&bytes[..8], b"GRFX0001");
    assert_eq!(&bytes[1684..8084], &icon);
    assert_eq!(effects::decode(&bytes).unwrap(), (effect, icon));
}

#[test]
fn invalid_fields_and_types_are_rejected() {
    let edits = [
        ("effect_id", json!("Bad_ID")),
        ("effect_id", json!("a\n")),
        ("name", json!(" Bad")),
        ("name", json!("x".repeat(17))),
        ("name", json!("é")),
        ("flags", json!(true)),
        ("flags", json!(2)),
        ("flags", json!(1.0)),
        ("matrix", json!(vec![0; 8])),
        ("matrix", json!(vec![32768; 9])),
        ("curves", json!(vec![vec![0; 255]; 3])),
    ];
    for (key, value) in edits {
        let mut effect = fixture();
        effect[key] = value;
        assert!(
            effects::encode(&effect, &vec![0; effects::ICON_SIZE]).is_err(),
            "{key}: {}",
            effect[key]
        );
    }
    let mut effect = fixture();
    effect["extra"] = json!(0);
    assert!(effects::encode(&effect, &vec![0; effects::ICON_SIZE]).is_err());
    for icon in [vec![0; 6399], vec![0; 6401], [0, 0, 0, 128].repeat(1600)] {
        assert!(effects::encode(&fixture(), &icon).is_err());
    }
}

#[test]
fn monochrome_requires_identical_rows_and_channels() {
    let mut effect = fixture();
    effect["flags"] = json!(effects::MONOCHROME);
    let icon = vec![0; effects::ICON_SIZE];
    assert!(effects::encode(&effect, &icon).is_err());
    effect["matrix"] = json!([4096, 3686, 410, 4096, 3686, 410, 4096, 3686, 410]);
    assert_eq!(
        effects::decode(&effects::encode(&effect, &icon).unwrap())
            .unwrap()
            .0,
        effect
    );
    effect["curves"][1][3] = json!(193);
    assert!(effects::encode(&effect, &icon).is_err());
}

#[test]
fn corruption_reserved_fields_and_string_padding_are_rejected() {
    let bytes = effects::encode(&fixture(), &vec![0; effects::ICON_SIZE]).unwrap();
    for offset in [0, 8, 12, 16, 48, 80, 128, 148, 1684, 8084] {
        let mut changed = bytes.clone();
        changed[offset] ^= 1;
        assert!(effects::decode(&changed).is_err());
    }
    for offset in [84, 146, 8084, 47, 79] {
        let mut changed = bytes.clone();
        changed[offset] = 1;
        let checksum = effects::checksum(&changed);
        changed[12..16].copy_from_slice(&checksum.to_le_bytes());
        assert!(effects::decode(&changed).is_err(), "offset {offset}");
    }
    assert!(effects::decode(&bytes[..8191]).is_err());
    assert!(effects::decode(&[bytes.as_slice(), &[0]].concat()).is_err());
    for length in 0..16 {
        assert!(effects::decode(&vec![0; length]).is_err());
    }
}

#[test]
fn default_flags_and_filename_contract() {
    let mut effect = fixture();
    effect.as_object_mut().unwrap().remove("flags");
    assert_eq!(
        effects::decode(&effects::encode(&effect, &vec![0; effects::ICON_SIZE]).unwrap())
            .unwrap()
            .0["flags"],
        0
    );
    let filename = effects::filename("custom-v1").unwrap();
    assert_eq!(filename.len(), 12);
    assert!(effects::valid_filename(&filename));
    assert!(effects::filename("../bad").is_err());
    for name in ["../a.GRF", "aaaaaaaaa.GRF", "a.GRF\n", ".GRF", "é.GRF"] {
        assert!(!effects::valid_filename(name));
    }
}

#[test]
fn duplicate_json_keys_are_rejected_recursively() {
    for input in [
        r#"{"flags":0,"flags":1}"#,
        r#"{"nested":[{"x":1,"x":2}]}"#,
        r#"{"x":1,"\u0078":2}"#,
        "{} {}",
    ] {
        assert!(effects::parse_json(input.as_bytes()).is_err());
    }
    let input = br#"{"values":[true,false,null,-1,18446744073709551615,1.5,"abc"]}"#;
    assert_eq!(
        effects::parse_json(input).unwrap(),
        serde_json::from_slice::<Value>(input).unwrap()
    );
}

#[test]
fn knots_validate_and_round_ties_even() {
    assert_eq!(
        effects::interpolate_curve(&json!([[0, 0], [255, 16320]])).unwrap(),
        (0..256).map(|i| i * 64).collect::<Vec<_>>()
    );
    let curve = effects::interpolate_curve(&json!([[0, 0], [4, 2], [255, 2]])).unwrap();
    assert_eq!(&curve[..5], &[0, 0, 1, 2, 2]);
    for points in [
        json!([[1, 0], [255, 1]]),
        json!([[0, 0], [0, 1], [255, 2]]),
        json!([[0, 0], [255, 16384]]),
        json!([[0, 0], [true, 1], [255, 3]]),
    ] {
        assert!(effects::interpolate_curve(&points).is_err());
    }
}

#[test]
fn fit_quantization_rounds_ties_even_and_rejects_extra_stages() {
    let mut fit = read_json(root().join("sources/ektar_b2n3.json"));
    fit["matrix"][0] = json!([1, -1.5 / 8192.0, 0.5 / 8192.0]);
    let converted = effects::import_fit(&fit).unwrap();
    assert_eq!(
        &converted["matrix"].as_array().unwrap()[..3],
        &[json!(8194), json!(-2), json!(0)]
    );
    fit["post_matrix"] = json!([]);
    assert!(effects::import_fit(&fit).is_err());
}

#[test]
fn compiler_rejects_ambiguous_definitions_and_invalid_artwork() {
    let mut definition = read_json(root().join("mono-orange-v1/effect.json"));
    definition["curves"] = json!([]);
    definition["curve_knots"] = json!(vec![[[0, 0], [255, 16320]]; 3]);
    assert!(effects::compile_definition(&definition).is_err());
    for (label, shape) in [
        ("", "film"),
        ("X", "film"),
        ("XXXXX", "plain"),
        ("MO", "other"),
        ("é", "film"),
    ] {
        assert!(artwork::render_tile(label, shape).is_err());
    }
}

#[test]
fn pinned_source_inputs_match_their_manifest() {
    let directory = root().join("sources");
    let manifest = read_json(directory.join("manifest.json"));
    assert_eq!(manifest["schema"], "grkit-effect-sources-v1");
    let files = manifest["files"].as_object().unwrap();
    assert!(!files.is_empty());
    for (name, item) in files {
        assert!(!name.contains('/'), "{name}");
        let bytes = fs::read(directory.join(name)).unwrap();
        assert_eq!(
            item["sha256"],
            json!(hex::encode(Sha256::digest(&bytes))),
            "{name}"
        );
    }
}
