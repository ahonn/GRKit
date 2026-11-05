use grkit_core::imports::{decode, encode, Import, MAGIC, MAX_DETAILS};
use grkit_core::testing;

fn import(id: &str, details: &str) -> Import {
    Import {
        details: details.into(),
        effect: testing::effect(id),
    }
}

#[test]
fn a_library_keeps_each_import_in_order() {
    let imports = vec![
        import(
            "lut-warm",
            r#"{"cover":{"template":"box","brand":"LUT","name":"WARM"}}"#,
        ),
        import("lut-cold", "{}"),
    ];
    let bytes = encode(&imports).unwrap();
    assert!(bytes.starts_with(MAGIC));
    assert_eq!(decode(&bytes).unwrap(), imports);
    assert_eq!(decode(&encode(&[]).unwrap()).unwrap(), []);
}

#[test]
fn a_library_rejects_bad_details_and_effects() {
    // Details that are not a JSON object, or that are too large.
    assert!(encode(&[import("lut-a", "[]")]).is_err());
    assert!(encode(&[import("lut-a", "not json")]).is_err());
    let large = format!(r#"{{"text":"{}"}}"#, "x".repeat(MAX_DETAILS));
    assert!(encode(&[import("lut-a", &large)]).is_err());
    // An effect that is not complete, and an ID twice.
    let mut broken = import("lut-a", "{}");
    broken.effect.truncate(100);
    assert!(encode(&[broken]).is_err());
    assert!(encode(&[import("lut-a", "{}"), import("lut-a", "{}")]).is_err());
}

#[test]
fn decode_rejects_a_damaged_file() {
    let bytes = encode(&[import("lut-a", "{}"), import("lut-b", "{}")]).unwrap();
    assert!(decode(b"GRKIMP00").is_err());
    assert!(decode(&bytes[..bytes.len() - 1]).is_err());
    let mut longer = bytes.clone();
    longer.push(0);
    assert!(decode(&longer).is_err());
    // A count larger than the imports that follow.
    let mut count = bytes.clone();
    count[8] = 3;
    assert!(decode(&count).is_err());
}
