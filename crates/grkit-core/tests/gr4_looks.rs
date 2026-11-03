//! Cinema (Yellow) and Cinema (Green) from local GR IV firmware files. The digests stand for
//! the camera's tables, so the tests hold no firmware data.
use anyhow::{Context, Result};
use grkit_core::gr4_looks::{mono_matrix, Image, Kind, LOOKS};
use grkit_core::native_tone::ToneSteps;
use sha2::{Digest, Sha256};
use std::io::Write;

fn firmware(variable: &str) -> Result<Vec<u8>> {
    let path = std::env::var(variable).with_context(|| format!("Set {variable}"))?;
    Ok(std::fs::read(path)?)
}

fn digest(curves: &[[u16; 256]]) -> String {
    let mut hash = Sha256::new();
    for value in curves.iter().flatten() {
        hash.update(value.to_le_bytes());
    }
    hex::encode(hash.finalize())
}

/// The tone tables and curve sets of GR IV 1.11 give the live-view tables of the camera:
/// Standard with neutral steps, and Cinema (Yellow), type 18, with its steps.
#[test]
#[ignore = "Requires local GRKIT_TEST_FIRMWARE_GR4"]
fn gr4_native_tone_matches_the_camera() -> Result<()> {
    let image = Image::read(&firmware("GRKIT_TEST_FIRMWARE_GR4")?)?;
    assert_eq!(image.source.version, "1.11");
    let tables = image.tone_tables()?;
    let steps = |key, contrast, shadow, highlight| ToneSteps {
        key,
        contrast,
        shadow,
        highlight,
        input_scale: 0x2158,
    };
    assert_eq!(
        digest(&[tables.base_curve(&steps(0, 0, 0, 0))]),
        "a7551a5811f906b587c2dd8e0495a4943c0a1a0a734fdbba83bde4ce34f01748"
    );
    assert!(!image.curve_set(11)?.enabled);
    let yellow = image.curve_set(18)?;
    assert!(yellow.enabled && yellow.weight == 1);
    assert_eq!(
        digest(&yellow.apply(&tables.base_curve(&steps(-1, 3, 2, 0)))),
        "f472ad7dc0424ee9a1328575adc6a9aefe46701f96e08fd9de2b90f003df406a"
    );
    assert!(image.curve_set(19)?.enabled);
    Ok(())
}

/// Both looks become effects with their defaults, every level of their MultiAxial sets,
/// and their curve sets. Standard's look matrix converts to the target's Standard.
#[test]
#[ignore = "Requires local GRKIT_TEST_FIRMWARE_GR4"]
fn gr4_cinema_looks_become_effects() -> Result<()> {
    let image = Image::read(&firmware("GRKIT_TEST_FIRMWARE_GR4")?)?;
    let standard = grkit_core::fit::STANDARD_LOOK;
    let icon = vec![0; grkit_core::effects::ICON_SIZE];
    for look in LOOKS.iter().filter(|l| matches!(l.kind, Kind::Color)) {
        let effect = image.effect(
            look.image_type,
            look.defaults,
            &standard,
            "cinema",
            "Cinema",
        )?;
        let native = effect.native_look.clone().unwrap();
        assert_eq!(effect.fn_defaults, Some(look.defaults));
        assert_eq!(native.gains, [1024, 960, 896, 832, 768]);
        assert_eq!(effect.curves, image.curve_set(look.image_type)?.mixed());
        let (decoded, _) = grkit_core::effects::decode(&effect.encode(&icon)?)?;
        assert_eq!(decoded, effect);
    }
    let effect = image.effect(11, [0; 9], &standard, "std", "Std")?;
    for (got, want) in effect.matrix.iter().zip(standard) {
        assert!(
            (i32::from(*got) - i32::from(want)).abs() <= 1,
            "{got} vs {want}"
        );
    }
    // Each definition compiles, as the site and the CLI compile it.
    for definition in image.definitions()? {
        grkit_core::effects::compile_definition(&definition)?;
    }
    Ok(())
}

/// GR IV 1.11 holds every look: Cinema, and the monochrome looks of GR IV Monochrome. Their
/// gray is the luma of GR III Standard, and High Contrast takes the curve set of
/// Hi-Contrast B&W. Only the offered looks become effects and definitions.
#[test]
#[ignore = "Requires local GRKIT_TEST_FIRMWARE_GR4"]
fn gr4_111_holds_the_monochrome_looks() -> Result<()> {
    let image = Image::read(&firmware("GRKIT_TEST_FIRMWARE_GR4")?)?;
    let effects = image.all_effects()?;
    let ids: Vec<&str> = effects.iter().map(|(look, _)| look.effect_id).collect();
    assert_eq!(ids, LOOKS.map(|look| look.effect_id));
    let offered: Vec<&str> = image
        .effects()?
        .iter()
        .map(|(look, _)| look.effect_id)
        .collect();
    assert_eq!(offered, ["gr4-cinema-yellow", "gr4-cinema-green"]);
    let definitions: Vec<_> = image
        .definitions()?
        .into_iter()
        .map(|d| d["effect_id"].clone())
        .collect();
    assert_eq!(definitions, offered);
    let matrix = mono_matrix();
    assert_eq!(matrix[..3].iter().map(|v| i32::from(*v)).sum::<i32>(), 8192);
    for (look, effect) in &effects {
        if matches!(look.kind, Kind::Monochrome(_)) {
            assert!(
                effect.monochrome() && effect.matrix == matrix,
                "{}",
                look.effect_id
            );
            assert_eq!(effect.fn_defaults.is_some(), look.defaults != [0; 9]);
        }
    }
    let by_id = |id: &str| {
        &effects
            .iter()
            .find(|(look, _)| look.effect_id == id)
            .unwrap()
            .1
    };
    assert_eq!(
        by_id("gr4-mono-high-contrast").curves[0],
        image.curve_set(16)?.mixed()[1]
    );
    assert_eq!(
        by_id("gr4-mono-soft").fn_defaults,
        Some([0, 0, 1, -2, 0, 0, -1, 0, 0])
    );
    Ok(())
}

/// GR IV 1.20, read from its official zip, holds the same looks as 1.11.
#[test]
#[ignore = "Requires local GRKIT_TEST_FIRMWARE_GR4 and GRKIT_TEST_FIRMWARE_GR4_120"]
fn gr4_120_holds_the_looks_of_111() -> Result<()> {
    let old = Image::read(&firmware("GRKIT_TEST_FIRMWARE_GR4")?)?;
    let file = firmware("GRKIT_TEST_FIRMWARE_GR4_120")?;
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    zip.start_file("fwdc248b.bin", zip::write::SimpleFileOptions::default())?;
    zip.write_all(&file)?;
    let new = Image::read(&zip.finish()?.into_inner())?;
    assert_eq!(new.source.version, "1.20");
    assert_eq!(new.tone_tables()?, old.tone_tables()?);
    let strip = |mut definitions: Vec<serde_json::Value>| {
        for d in &mut definitions {
            d["attribution"] = serde_json::Value::Null;
        }
        definitions
    };
    let new_definitions = strip(new.definitions()?);
    for definition in strip(old.definitions()?) {
        assert!(
            new_definitions.contains(&definition),
            "{}",
            definition["effect_id"]
        );
    }
    let ids: Vec<&str> = new
        .all_effects()?
        .iter()
        .map(|(look, _)| look.effect_id)
        .collect();
    assert_eq!(ids, LOOKS.map(|look| look.effect_id));
    for definition in new.definitions()? {
        grkit_core::effects::compile_definition(&definition)?;
    }
    Ok(())
}
