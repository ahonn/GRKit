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

/// A selective color definition that keeps only red. No published effect uses the feature.
fn keep_red() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/keep-red.json")
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
        hex::encode(Sha256::digest(artwork::grkit_pixels())),
        "5d9423843d61423d1a3b7d8cbbe84a7885b729ab29aa5b92333d3b7d43b25e46"
    );
    assert_eq!(
        hex::encode(Sha256::digest(artwork::version_pixels())),
        "4119d11482f7f7a0636d481beae529aa4d87a4ec8d4afc62c2d8d4c8b295b0c6"
    );
    // The tiles of the module folders keep the rules of the host artwork.
    let mut modules: Vec<Vec<u8>> = Vec::new();
    for entry in std::fs::read_dir(root().join("../modules")).unwrap() {
        let tiles = entry.unwrap().path().join("tiles");
        for tile in std::fs::read_dir(tiles).into_iter().flatten() {
            let png = std::fs::read(tile.unwrap().path()).unwrap();
            let (width, height, rgba) = grkit_core::png::decode_rgba8(&png).unwrap();
            assert_eq!((width, height), (40, 40));
            modules.push(rgba);
        }
    }
    for pixels in [
        artwork::grkit_pixels(),
        artwork::version_pixels(),
        artwork::off_pixels(),
        artwork::render_tile("OFF", "film").unwrap(),
        artwork::render_tile("OFF", "filter").unwrap(),
        artwork::render_tile("OFF", "plain").unwrap(),
        artwork::render_box("Kodak", "PORTRA", "400", true).unwrap(),
        artwork::render_box("FUJIFILM", "SUPERIA", "400", false).unwrap(),
        artwork::render_cine("Kodak", "VISION3", "250D").unwrap(),
    ]
    .into_iter()
    .chain(modules)
    {
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
    let mut definition = read_json(root().join("portra-400-endura-v1/effect.json"));
    definition["name"] = json!("My Portrait");
    definition["curves"][0][100] = json!(0);
    let (compiled, _) =
        effects::decode(&effects::compile_definition(&definition).unwrap()).unwrap();
    let compiled = compiled.to_json();
    assert_eq!(compiled["name"], "My Portrait");
    assert_eq!(compiled["curves"], definition["curves"]);
    assert_eq!(compiled["matrix"], definition["matrix"]);
    assert_eq!(compiled["effect_id"], definition["effect_id"]);
}

#[test]
fn cube_fits_record_their_pinned_luts() {
    let manifest = read_json(root().join("sources/manifest.json"));
    let luts = manifest["luts"].as_object().unwrap();
    let mut used = Vec::new();
    for entry in fs::read_dir(root()).unwrap() {
        let path = entry.unwrap().path().join("effect.json");
        if !path.is_file() {
            continue;
        }
        let definition = read_json(path);
        let attribution = &definition["attribution"];
        if attribution["origin"] != "cube-fit" {
            continue;
        }
        // A .cube LUT, or a HaldCLUT image, named after the effect.
        let source = attribution["source"].as_str().unwrap();
        let id = definition["effect_id"].as_str().unwrap();
        assert!(
            [".cube", ".png"]
                .iter()
                .any(|extension| source == format!("{id}{extension}")),
            "{source}"
        );
        let lut = &luts[source];
        assert_eq!(lut["sha256"], attribution["source_sha256"], "{source}");
        let generator = &manifest["generators"][lut["generator"].as_str().unwrap()];
        assert_eq!(generator["license"], attribution["license"], "{source}");
        used.push(source.to_owned());
    }
    used.sort();
    assert_eq!(used, luts.keys().cloned().collect::<Vec<_>>());
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
    let (decoded, decoded_icon) = effects::decode(&bytes).unwrap();
    assert_eq!((decoded.to_json(), &decoded_icon), (effect, &icon));
    assert_eq!(decoded.encode(&decoded_icon).unwrap(), bytes);
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
            .0
            .to_json(),
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
    for offset in [106, 108, 146, 8084, 47, 79] {
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
            .0
            .flags,
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
fn compiler_rejects_ambiguous_definitions_and_invalid_artwork() {
    let mut definition = read_json(root().join("red-filter-mono-v1/effect.json"));
    definition["curves"] = json!([]);
    definition["curve_knots"] = json!(vec![[[0, 0], [255, 16320]]; 3]);
    assert!(effects::compile_definition(&definition).is_err());
    for (label, shape) in [
        ("", "film"),
        ("x", "film"),
        ("XXXXX", "plain"),
        ("MO", "other"),
        ("é", "film"),
    ] {
        assert!(artwork::render_tile(label, shape).is_err());
    }
}

#[test]
fn cover_icons_span_the_tile_width_inside_the_row_margin() {
    for pixels in [
        artwork::render_box("Kodak", "KODACHROME", "64", true).unwrap(),
        artwork::render_cine("Kodak", "EXR", "200T").unwrap(),
    ] {
        for y in 0..40 {
            for x in 0..40 {
                let alpha = pixels[(y * 40 + x) * 4 + 3];
                let inside = (3..37).contains(&y);
                assert_eq!(alpha, if inside { 255 } else { 0 }, "({x}, {y})");
            }
        }
    }
    // Only the pixel font has no lowercase, so a wide brand must be capitals.
    assert!(artwork::render_box("Fujicolorpro", "PRO", "400H", true).is_err());
    assert!(artwork::render_box("FUJICOLOR", "PRO", "400H", true).is_ok());
}

#[test]
fn covers_without_a_number_show_a_large_name() {
    // Below the brand band, only the name is light on a box without a band.
    let rows = |pixels: &[u8]| -> Vec<usize> {
        (12..40)
            .filter(|y| (0..40).any(|x| pixels[(y * 40 + x) * 4] > 128))
            .collect()
    };
    let large = artwork::render_box("Kodak", "PORTRA", "", false).unwrap();
    let small = artwork::render_box("Kodak", "PORTRA", "400", false).unwrap();
    assert_ne!(large, small);
    let large_rows = rows(&large);
    // Taller than the 5-pixel font, inside the body, and centered within one row.
    assert!(large_rows.len() >= 8, "{large_rows:?}");
    assert!(
        large_rows.iter().all(|y| (12..37).contains(y)),
        "{large_rows:?}"
    );
    let middle = (large_rows[0] + large_rows[large_rows.len() - 1]) as f32 / 2.0;
    assert!((middle - 24.5).abs() <= 1.5, "{middle}");
    // A long name gets a smaller size and still fills no more than the body.
    for name in [
        "KODACHROME",
        "TEAL ORANGE",
        "MMMMMMMMMMMM",
        "classic chrome"[..12].trim(),
    ] {
        let pixels = artwork::render_box("Kodak", name, "", false).unwrap();
        let used = rows(&pixels);
        assert!(
            !used.is_empty() && used.iter().all(|y| (12..37).contains(y)),
            "{name}: {used:?}"
        );
    }
    // With a number, a name too wide for the pixel font, or with lowercase, gets a smaller
    // Roboto Condensed Black on the name row. A name that fits keeps the pixel font.
    // The condensed font fits every name of 12 characters.
    for name in ["TEAL ORANGE", "VISION 2383", "Portra", "WWWWWWWWWWWW"] {
        assert!(
            artwork::render_box("Kodak", name, "400", true).is_ok(),
            "{name}"
        );
    }
    let error = artwork::render_box("Kodak", "PORTRA", "WWWWWWWWWWWW", true).unwrap_err();
    assert!(format!("{error:#}").contains("does not fit"), "{error:#}");
    // Lowercase works only in Roboto Condensed Black.
    assert!(artwork::render_box("Kodak", "Portra", "", true).is_ok());
}

#[test]
fn compiler_rejects_cover_icons_that_do_not_fit() {
    let definition = read_json(root().join("portra-400-endura-v1/effect.json"));
    let box_icon = definition["icon"].clone();
    for (key, value) in [
        ("name", json!("")),
        ("number", json!("123456789")),
        ("number", json!("é")),
        ("band", json!("yes")),
        ("template", json!("can")),
        ("label", json!("P4")),
    ] {
        let mut changed = definition.clone();
        changed["icon"][key] = value.clone();
        assert!(
            effects::compile_definition(&changed).is_err(),
            "{key} = {value}"
        );
    }
    let mut changed = definition.clone();
    // A cover without a number shows a large name.
    changed["icon"] = json!({"template": "cine", "brand": "Kodak", "name": "VISION3"});
    assert_eq!(
        effects::icon_pixels(&changed["icon"]).unwrap(),
        artwork::render_cine("Kodak", "VISION3", "").unwrap()
    );
    changed["icon"] =
        json!({"template": "box", "brand": "Kodak", "name": "PORTRA", "number": "", "band": true});
    assert!(effects::compile_definition(&changed).is_err());
    changed["icon"] = json!({"template": "cine", "brand": "Kodak", "name": "VISION3", "number": "250D", "band": true});
    assert!(effects::compile_definition(&changed).is_err());
    assert_eq!(
        effects::icon_pixels(&box_icon).unwrap(),
        artwork::render_box("Kodak", "PORTRA", "400", true).unwrap()
    );
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

#[test]
fn selective_color_compiles_to_kernel_integers_and_round_trips() {
    let definition = read_json(keep_red());
    let blob = effects::compile_definition(&definition).unwrap();
    assert_eq!(&blob[84..88], &effects::FEATURE_SELECTIVE.to_le_bytes());
    // cos and sin of 108.6 degrees in Q12, Q8 tangents of 20/35 and 6/14 degrees,
    // then the chroma gate. The runtime test uses the same integers.
    assert_eq!(
        blob[106..120],
        [0xe6, 0xfa, 0x2a, 0x0f, 93, 0, 179, 0, 27, 0, 64, 0, 6, 16]
    );
    assert!(blob[120..128].iter().all(|v| *v == 0));
    let (effect, icon) = effects::decode(&blob).unwrap();
    assert_eq!(
        effect.selective.unwrap().to_json(),
        json!({"direction_q12": [-1306, 3882], "lower_tan_q8": [93, 179],
               "higher_tan_q8": [27, 64], "chroma": [6, 16]})
    );
    assert_eq!(effect.encode(&icon).unwrap(), blob);
}

#[test]
fn selective_color_rejects_invalid_parameters() {
    let base = read_json(keep_red());
    let invalid = [
        json!({"hue": 360, "lower": [20, 35], "higher": [6, 14], "chroma": [6, 16]}),
        json!({"hue": -1, "lower": [20, 35], "higher": [6, 14], "chroma": [6, 16]}),
        json!({"hue": 108.6, "lower": [20, 20], "higher": [6, 14], "chroma": [6, 16]}),
        json!({"hue": 108.6, "lower": [20, 35], "higher": [6, 76], "chroma": [6, 16]}),
        json!({"hue": 108.6, "lower": [-1, 35], "higher": [6, 14], "chroma": [6, 16]}),
        json!({"hue": 108.6, "lower": [20, 20.05], "higher": [6, 14], "chroma": [6, 16]}),
        json!({"hue": 108.6, "lower": [20, 35], "higher": [6, 14], "chroma": [16, 6]}),
        json!({"hue": 108.6, "lower": [20, 35], "higher": [6, 14], "chroma": [6, 182]}),
        json!({"hue": 108.6, "lower": [20, 35], "higher": [6, 14], "chroma": [6.5, 16]}),
        json!({"hue": 108.6, "lower": [20, 35], "higher": [6, 14], "chroma": [6, 16], "x": 1}),
        json!({"hue": 108.6, "lower": [20, 35], "higher": [6, 14]}),
        json!({"direction_q12": [-1306, 3882], "lower_tan_q8": [93, 179],
               "higher_tan_q8": [27, 64], "chroma": [6, 16]}),
    ];
    for value in invalid {
        let mut definition = base.clone();
        definition["selective_color"] = value.clone();
        assert!(effects::compile_definition(&definition).is_err(), "{value}");
    }
    // Selective color needs the chroma that a monochrome effect removes.
    let mut mono = read_json(root().join("red-filter-mono-v1/effect.json"));
    mono["selective_color"] = base["selective_color"].clone();
    let error = effects::compile_definition(&mono).unwrap_err().to_string();
    assert!(error.contains("selective"), "{error}");
}

/// The words of an effect name or an icon text, in capitals. A space or a
/// hyphen separates words.
fn words(text: &str) -> Vec<String> {
    let mut words: Vec<String> = text
        .split([' ', '-'])
        .filter(|word| !word.is_empty())
        .map(str::to_ascii_uppercase)
        .collect();
    words.sort();
    words
}

/// A cover icon shows the words of the effect name. A motion picture film can
/// show its film number, which its target names, in place of the series name:
/// Vision3 500T shows 5219 above 500T.
#[test]
fn published_cover_icons_show_the_words_of_the_effect_name() {
    for entry in fs::read_dir(root()).unwrap() {
        let path = entry.unwrap().path().join("effect.json");
        if !path.is_file() {
            continue;
        }
        let definition = read_json(path);
        let icon = &definition["icon"];
        let name = definition["name"].as_str().unwrap();
        assert!(icon.get("template").is_some(), "{name} has no cover icon");
        // A cover without a number shows the name large, as Instax does.
        let (series, number) = (
            icon["name"].as_str().unwrap(),
            icon["number"].as_str().unwrap_or(""),
        );
        if icon["template"] == "cine" && series.bytes().all(|c| c.is_ascii_digit()) {
            let target = definition["attribution"]["target"].as_str().unwrap();
            assert!(
                target.split(' ').any(|word| word == series),
                "{name}: {target}"
            );
            assert_eq!(name.rsplit(' ').next(), Some(number), "{name}");
        } else {
            assert_eq!(words(&format!("{series} {number}")), words(name), "{name}");
        }
        assert!(
            ["Kodak", "FUJIFILM", "ILFORD"].contains(&icon["brand"].as_str().unwrap()),
            "{name}"
        );
    }
}

#[test]
fn overlay_labels_fit_their_area_and_outline_their_text() {
    let table = artwork::overlay_labels();
    assert!(table.len() <= grkit_core::abi::LABELS_SIZE);
    assert_eq!(table.len() % 4, 0);
    let field = |at: usize, bytes: usize| {
        let mut value = 0usize;
        for (index, byte) in table[at..at + bytes].iter().enumerate() {
            value |= usize::from(*byte) << (8 * index);
        }
        value
    };
    for index in 0..artwork::OVERLAY_LABELS.len() {
        let (width, height, pixels) = artwork::overlay_label_pixels(&table, index);
        assert_eq!(
            (width, height),
            (field(index * 8, 2), field(index * 8 + 2, 2))
        );
        assert_eq!(height, artwork::OVERLAY_LABEL_HEIGHT);
        // A Framing label at the left of the row ends before the centered
        // load cue starts.
        let widest = if index == 0 { 200 } else { 80 };
        assert!(
            (30..=widest).contains(&width),
            "label {index} is {width} wide"
        );
        // White text, its black outline, and transparent pixels.
        for value in [0, 1, 255] {
            assert!(pixels.contains(&value), "label {index} lacks {value}");
        }
        // The outline separates the text from the live view on every side.
        for y in 0..height {
            for x in 0..width {
                if pixels[y * width + x] <= 1 {
                    continue;
                }
                assert!(x > 0 && y > 0 && x + 1 < width && y + 1 < height);
                for (nx, ny) in [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)] {
                    assert_ne!(pixels[ny * width + nx], 0, "label {index} at {x},{y}");
                }
            }
        }
    }
    let cue = field(0, 2);
    let widest = (1..artwork::OVERLAY_LABELS.len())
        .map(|index| field(index * 8, 2))
        .max()
        .unwrap();
    assert!(7 + widest < 360 - cue / 2);
}

/// Label n names the CORE.ASPECT value n (LABEL_RATIO in runtime/sdk/grkit.h):
/// it starts the summary of value n of the Framing module. The Date Imprint
/// sign follows the ratios (LABEL_DATE).
#[test]
fn overlay_labels_follow_the_aspect_values() {
    let aspect = grkit_core::capabilities::CAPABILITIES
        .iter()
        .find(|capability| capability.key == "ASPECT")
        .unwrap();
    assert_eq!(artwork::OVERLAY_LABELS.len(), aspect.values as usize + 1);
    assert_eq!(artwork::OVERLAY_LABELS[aspect.values as usize], "DATE");
    let text = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../modules/framing/text/en.toml"),
    )
    .unwrap();
    // A TOML array of strings is also a JSON array.
    let line = text
        .lines()
        .find_map(|line| line.strip_prefix("summaries = "))
        .unwrap();
    let summaries: Vec<String> = serde_json::from_str(line).unwrap();
    assert_eq!(summaries.len(), aspect.values as usize);
    for (value, summary) in summaries.iter().enumerate().skip(1) {
        assert!(
            summary.starts_with(artwork::OVERLAY_LABELS[value]),
            "label {value}"
        );
    }
}

#[test]
fn a_long_cover_number_takes_a_smaller_size() {
    // YELLOW is too wide at the full size; GREEN keeps it.
    let yellow = artwork::render_cine("RICOH", "CINEMA", "YELLOW").unwrap();
    let green = artwork::render_cine("RICOH", "CINEMA", "GREEN").unwrap();
    // The first and last rows with number ink (white) in the body below the name.
    let rows = |rgba: &[u8]| {
        let ink: Vec<usize> = (21..32)
            .filter(|y| (0..40).any(|x| rgba[(y * 40 + x) * 4] > 200))
            .collect();
        (ink[0], *ink.last().unwrap())
    };
    let (top_y, bottom_y) = rows(&yellow);
    let (top_g, bottom_g) = rows(&green);
    assert!(bottom_y - top_y < bottom_g - top_g);
    assert!(artwork::render_cine("RICOH", "CINEMA", "123456789").is_err());
}
