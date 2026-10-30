use grkit_core::{effects, site};
use grkit_wasm::native_looks;

fn base() -> [u16; 256] {
    // The analytic stand-in of the import tests: any rising table works for the checks.
    std::array::from_fn(|i| ((i as f64 / 255.0).powf(0.45) * 16000.0) as u16)
}

#[test]
fn other_files_fail_with_the_supported_versions() {
    let error = native_looks(b"PK not a zip", &base())
        .err()
        .unwrap()
        .to_string();
    assert!(error.contains("ZIP"), "{error}");
    let error = native_looks(&[0; 64], &base()).err().unwrap().to_string();
    assert!(error.contains("1.11, 1.20"), "{error}");
}

/// The page gets every look with compiled effects, icons, and previews that follow the
/// default tone steps.
#[test]
#[ignore = "Requires local GRKIT_TEST_FIRMWARE_GR4"]
fn a_gr4_file_gives_its_looks() {
    let file = std::fs::read(std::env::var("GRKIT_TEST_FIRMWARE_GR4").unwrap()).unwrap();
    let looks = native_looks(&file, &base()).unwrap();
    assert_eq!(
        looks.iter().map(|l| l.id.as_str()).collect::<Vec<_>>(),
        grkit_core::gr4_looks::LOOKS
            .iter()
            .filter(|look| look.offered)
            .map(|look| look.effect_id)
            .collect::<Vec<_>>()
    );
    for look in &looks {
        assert_eq!(look.version, "1.11");
        let (effect, icon) = effects::decode(&look.blob).unwrap();
        assert_eq!(icon, look.icon);
        assert_eq!(look.preview.len(), site::LUT_SIZE.pow(3) * 3);
        // Default tone steps change the preview from the one without them.
        let plain = site::linear_preview_lut(&base(), &effect).unwrap();
        let steps = effect.fn_defaults.is_some_and(|d| d[2..6] != [0; 4]);
        assert_eq!(plain != look.preview, steps, "{}", look.id);
        assert_eq!(effect.native_look.is_some(), !effect.monochrome());
    }
}
