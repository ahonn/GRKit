use grkit_core::{cube, effects, fit, site};
use grkit_wasm::{
    effect_preview, guess_input, import_lut, relabel_effect, ImportRequest, MAX_LUT_SIZE,
};
use serde_json::{json, Value};
use std::sync::LazyLock;

/// The analytic stand-in for the firmware table from the core import tests.
fn synthetic_base() -> [u16; 256] {
    let scale = 1.2 * 0.18 / fit::MID_INDEX;
    let white = 255.0 * scale;
    std::array::from_fn(|i| {
        let x = i as f64 * scale;
        let shoulder = x * (1.0 + x / (white * white)) / (1.0 + x);
        (16384.0 * cube::srgb_encode(shoulder)).round() as u16
    })
}

/// A small warm `.cube` LUT: red up, blue down, a little less saturation.
fn warm_cube() -> Vec<u8> {
    let n = 9;
    let mut text = format!("TITLE \"Warm\"\nLUT_3D_SIZE {n}\n");
    for b in 0..n {
        for g in 0..n {
            for r in 0..n {
                let [r, g, b] = [r, g, b].map(|v| v as f64 / (n - 1) as f64);
                let (r, b) = ((r * 1.04).min(1.0), b * 0.94);
                let luma = 0.3 * r + 0.6 * g + 0.1 * b;
                let [r, g, b] = [r, g, b].map(|v| luma + (v - luma) * 0.9);
                text.push_str(&format!("{r:.6} {g:.6} {b:.6}\n"));
            }
        }
    }
    text.into_bytes()
}

static COVER: LazyLock<Value> = LazyLock::new(
    || json!({"template": "box", "brand": "Kodak", "name": "WARM", "number": "400", "band": true}),
);

fn request<'a>(id: &'a str, name: &'a str) -> ImportRequest<'a> {
    ImportRequest {
        effect_id: id,
        name,
        icon: &COVER,
        source: "warm.cube",
        input: "standard",
    }
}

#[test]
fn imports_a_cube_as_a_package_effect_with_a_preview() {
    let imported =
        import_lut(&warm_cube(), &synthetic_base(), &request("warm-v1", "Warm")).unwrap();
    let (effect, icon) = effects::decode(&imported.blob).unwrap();
    assert_eq!(effect.id, "warm-v1");
    assert_eq!(effect.name, "Warm");
    assert_eq!(icon.len(), 40 * 40 * 4);
    assert_eq!(imported.icon, icon);
    let definition = effects::parse_json(imported.definition.as_bytes()).unwrap();
    assert_eq!(definition["attribution"]["source"], "warm.cube");
    // The cover replaces the label tile of `fit::import`, as the catalog effects use covers.
    assert_eq!(definition["icon"], *COVER);
    assert_eq!(icon, effects::icon_pixels(&COVER).unwrap());
    assert_eq!(definition["attribution"]["fit"]["grade"], imported.grade);
    assert_eq!(
        effects::compile_definition(&definition).unwrap(),
        imported.blob
    );
    assert_eq!(
        imported.preview,
        site::linear_preview_lut(&synthetic_base(), &effect).unwrap()
    );
    assert!(!imported.monochrome);
    assert!(["A", "B"].contains(&imported.grade), "{}", imported.grade);
}

#[test]
fn invalid_requests_fail_with_their_cause() {
    let base = synthetic_base();
    let cube = warm_cube();
    let error = |result: anyhow::Result<grkit_wasm::Imported>| {
        format!("{:#}", result.err().expect("expected an error"))
    };
    assert!(
        error(import_lut(&cube, &base[..255], &request("warm-v1", "Warm")))
            .contains("256 base gamma values")
    );
    assert!(
        error(import_lut(&cube, &[0; 256], &request("warm-v1", "Warm")))
            .contains("Unexpected Standard base gamma")
    );
    assert!(error(import_lut(&cube, &base, &request("Warm", "Warm"))).contains("effect ID"));
    assert!(error(import_lut(
        &cube,
        &base,
        &request("warm-v1", "A name over 16 chars")
    ))
    .contains("name"));
    let wide = json!({"template": "box", "brand": "Kodak", "name": "WARM",
        "number": "123456789012", "band": true});
    let mut cover = request("warm-v1", "Warm");
    cover.icon = &wide;
    assert!(error(import_lut(&cube, &base, &cover)).contains("does not fit"));
    let mut log = request("warm-v1", "Warm");
    log.input = "slog3";
    assert!(import_lut(&cube, &base, &log).is_err());
    let large = vec![0; MAX_LUT_SIZE + 1];
    assert!(error(import_lut(&large, &base, &request("warm-v1", "Warm"))).contains("larger"));
    assert!(import_lut(b"not a LUT", &base, &request("warm-v1", "Warm")).is_err());
}

#[test]
fn guesses_the_input_from_the_lut_data() {
    let base = synthetic_base();
    assert_eq!(guess_input(&warm_cube(), &base).unwrap(), "standard");
    // A LUT that shows V-Log input as the Standard rendering asks for V-Log, whatever its name.
    let model = fit::Model::new(&base);
    let n = 17;
    let mut text = format!("TITLE \"Look\"\nLUT_3D_SIZE {n}\n");
    for b in 0..n {
        for g in 0..n {
            for r in 0..n {
                let code = [r, g, b].map(|v| v as f64 / (n - 1) as f64);
                let [r, g, b] = model.standard(model.decode(cube::Encoding::Vlog, code));
                text.push_str(&format!("{r:.6} {g:.6} {b:.6}\n"));
            }
        }
    }
    assert_eq!(guess_input(text.as_bytes(), &base).unwrap(), "vlog");
    assert!(guess_input(b"not a LUT", &base).is_err());
}

#[test]
fn the_preview_of_an_effect_file_is_the_preview_of_its_import() {
    let base = synthetic_base();
    let imported = import_lut(&warm_cube(), &base, &request("lut-warm", "Warm")).unwrap();
    assert_eq!(
        effect_preview(&imported.blob, &base, &[]).unwrap(),
        imported.preview
    );
    assert!(effect_preview(b"GRFX0001 not an effect", &base, &[]).is_err());
    assert!(effect_preview(&imported.blob, &base[..255], &[]).is_err());
    // Tone curves are 512 values: the neutral curve, then the look curve.
    assert!(effect_preview(&imported.blob, &base, &[0; 511]).is_err());
}

#[test]
fn a_relabeled_effect_keeps_its_id_and_its_look() {
    let imported =
        import_lut(&warm_cube(), &synthetic_base(), &request("warm-v1", "Warm")).unwrap();
    let (before, icon) = effects::decode(&imported.blob).unwrap();
    // A new name keeps the ID, the icon, and every value of the look.
    let renamed = relabel_effect(&imported.blob, "Warm Day", None).unwrap();
    let (after, same_icon) = effects::decode(&renamed).unwrap();
    assert_eq!(
        (after.id.as_str(), after.name.as_str()),
        ("warm-v1", "Warm Day")
    );
    assert_eq!(same_icon, icon);
    assert_eq!(after.to_json()["name"], "Warm Day");
    let mut look = before.to_json();
    look["name"] = json!("Warm Day");
    assert_eq!(after.to_json(), look);
    // Back to the old name, the file is the same again.
    assert_eq!(
        relabel_effect(&renamed, "Warm", None).unwrap(),
        imported.blob
    );
    // A new icon replaces the old one.
    let cover = json!({"template": "box", "brand": "Kodak", "name": "DAY", "band": false});
    let pixels = effects::icon_pixels(&cover).unwrap();
    let (_, new_icon) =
        effects::decode(&relabel_effect(&imported.blob, "Warm", Some(&pixels)).unwrap()).unwrap();
    assert_eq!(new_icon, pixels);
    assert_ne!(new_icon, icon);
    // A name that the camera cannot show, an icon of another size, and a file that is not an
    // effect fail.
    for name in ["", " Warm", "Warm 1234567890123", "Wärm"] {
        assert!(
            relabel_effect(&imported.blob, name, None).is_err(),
            "{name:?}"
        );
    }
    assert!(relabel_effect(&imported.blob, "Warm", Some(&[0; 16])).is_err());
    assert!(relabel_effect(b"GRFX0001", "Warm", None).is_err());
}
