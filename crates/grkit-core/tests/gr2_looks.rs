//! Positive Film from a local GR II firmware file. The digests stand for the camera's records,
//! so the tests hold no firmware data.
use anyhow::{Context, Result};
use grkit_core::gr2_looks::{detect, for_preview, Image, EFFECT_ID, GR4_EFFECT_ID};
use sha2::{Digest, Sha256};

fn firmware() -> Result<Vec<u8>> {
    let path = std::env::var("GRKIT_TEST_FIRMWARE_GR2").context("Set GRKIT_TEST_FIRMWARE_GR2")?;
    Ok(std::fs::read(path)?)
}

/// The official zip gives the GR III effect: the look matrix on GR III Standard, the curve of
/// Positive's tone after Standard's, and Positive's hue-sector set after Standard's at full
/// weight on every level. The effect file keeps it. The GR IV effect differs only in the look matrix.
#[test]
#[ignore = "Requires local GRKIT_TEST_FIRMWARE_GR2 (rg2_v003.zip or rg2_v003.frm)"]
fn gr2_positive_film_becomes_an_effect() -> Result<()> {
    let bytes = firmware()?;
    assert!(detect(&bytes));
    let image = Image::read(&bytes)?;
    let effect = image.effect(EFFECT_ID, &grkit_core::fit::STANDARD_LOOK)?;
    assert_eq!(effect.id, EFFECT_ID);
    assert_eq!(
        effect.matrix,
        [7837, 2488, -2133, 502, 7995, -305, -121, 2507, 5806]
    );
    let [red, green, blue] = &effect.curves;
    assert!(red == green && green == blue);
    assert!(red.windows(2).all(|w| w[0] <= w[1]) && red[0] == 0 && red[255] == 16383);
    // Deeper shadows and a lower shoulder than Standard, the same white.
    let linear = |i: usize| (i * 16383 / 255) as u16;
    assert!(red[32] < linear(32) && red[192] < linear(192));
    let mut hash = Sha256::new();
    for v in red {
        hash.update(v.to_le_bytes());
    }
    assert_eq!(
        hex::encode(hash.finalize()),
        "aec9ccbcbd2d7f9cb25886ad8cf2e276af8666796320ce5e41a76e914488579a"
    );
    let native = effect.native_look.clone().unwrap();
    assert_eq!(native.gains, [1024; 5]);
    for bank in 0..12 {
        let first = native.level_matrix(bank, 0);
        assert!((1..5).all(|level| native.level_matrix(bank, level) == first));
    }
    let mut hash = Sha256::new();
    for bank in 0..12 {
        for v in native.level_matrix(bank, 0) {
            hash.update(v.to_le_bytes());
        }
    }
    assert_eq!(
        hex::encode(hash.finalize()),
        "ed6d7ced3de9f7fc8ea09ab88da797799b857380a98484963031368839999fca"
    );
    // The GR IV effect is the same look on GR IV Standard: back on GR III Standard, it is the
    // GR III effect but for rounding.
    let gr4 = image.effect(GR4_EFFECT_ID, &grkit_core::fit::GR4_STANDARD_LOOK)?;
    assert_ne!(gr4.matrix, effect.matrix);
    assert_eq!(
        (&gr4.curves, &gr4.native_look),
        (&effect.curves, &effect.native_look)
    );
    let back = for_preview(gr4)?;
    assert!(back
        .matrix
        .iter()
        .zip(effect.matrix)
        .all(|(a, b)| (a - b).abs() <= 2));
    let icon = vec![0; grkit_core::effects::ICON_SIZE];
    let (decoded, _) = grkit_core::effects::decode(&effect.encode(&icon)?)?;
    assert_eq!(decoded, effect);
    Ok(())
}
