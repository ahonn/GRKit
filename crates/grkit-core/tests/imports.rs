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

#[test]
fn upsert_merges_the_cover_and_keeps_the_other_details() {
    use grkit_core::{cover::Cover, effects, imports};
    let cover = |name: &str| Cover::for_name(name);
    let effect = |id: &str, cover: &Cover| {
        let (effect, _) = effects::decode(&testing::effect(id)).unwrap();
        effect.encode(&cover.pixels().unwrap()).unwrap()
    };
    let gold = cover("Kodak Gold 200");
    let mut library = vec![import("lut-a", r#"{"grade":"A","cover":{"label":"X"}}"#)];
    imports::upsert(&mut library, effect("lut-a", &gold), &gold, false).unwrap();
    imports::upsert(&mut library, effect("portra-v1", &gold), &gold, true).unwrap();
    imports::upsert(&mut library, effect("lut-b", &gold), &gold, false).unwrap();
    let details = |id: &str| -> serde_json::Value {
        serde_json::from_str(&imports::find(&library, id).unwrap().details).unwrap()
    };
    assert_eq!(details("lut-a")["grade"], "A");
    assert_eq!(details("portra-v1")["kind"], "cover");
    assert_eq!(details("lut-b").as_object().unwrap().len(), 1);
    assert_eq!(
        imports::find(&library, "lut-a").unwrap().cover(),
        Some(gold.clone())
    );
    // A cover that does not draw the icon of the effect.
    let other = cover("Fuji Velvia 100");
    assert!(imports::upsert(&mut library, effect("lut-a", &gold), &other, false).is_err());
    assert_eq!(library.len(), 3);
    assert_eq!(
        imports::remove(&mut library, "portra-v1")
            .unwrap()
            .id()
            .unwrap(),
        "portra-v1"
    );
    assert!(imports::remove(&mut library, "portra-v1").is_none());
    assert_eq!(
        decode(&imports::library_file(&library).unwrap().unwrap()).unwrap(),
        library
    );
    assert_eq!(imports::library_file(&[]).unwrap(), None);
    assert_eq!(import("lut-c", r#"{"cover":{"label":"x"}}"#).cover(), None);
}
