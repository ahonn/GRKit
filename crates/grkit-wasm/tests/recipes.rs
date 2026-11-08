use grkit_wasm::{decode_recipes, encode_recipes};
use serde_json::{json, Value};

/// A recipe library that the site encodes for a package comes back from its
/// bytes, in the JSON form of the site (`web/src/recipes.ts`), with a recipe
/// that is off.
#[test]
fn a_recipe_library_comes_back_from_its_bytes() {
    let library = json!([{
        "name": "Street",
        "family": "gr3",
        "look": { "kind": "off" },
        "grain": { "level": 2, "size": 1 },
        "glow": null,
        "whiteBalance": null,
        "corrections": [null, null, null, null],
        "exposure": { "compensation": null, "isoLimit": null, "metering": null, "nd": null },
        "enabled": false,
        "hidden": true,
        "number": 3,
        "id": 0,
    }]);
    let bytes = encode_recipes(&library.to_string(), r#"["keep","keep","keep"]"#).unwrap();
    let decoded: Value = serde_json::from_str(&decode_recipes(&bytes).unwrap()).unwrap();
    assert_eq!(
        decoded,
        json!({ "recipes": library, "dial": ["keep", "keep", "keep"] })
    );
}

/// A library carries what INSTALL sets U1 to U3 to, in the JSON form of the
/// site: a step that names a record that is off follows.
#[test]
fn a_recipe_library_carries_the_mode_dial() {
    let recipe = |name: &str, enabled: bool| {
        json!({
            "name": name,
            "family": "gr3",
            "look": { "kind": "off" },
            "grain": null,
            "glow": null,
            "whiteBalance": null,
            "corrections": [null, null, null, null],
            "exposure": { "compensation": null, "isoLimit": null, "metering": null, "nd": null },
            "enabled": enabled,
            "hidden": false,
            "number": 0,
            "id": 0,
        })
    };
    let library = json!([recipe("A", true), recipe("B", false)]);
    let bytes =
        encode_recipes(&library.to_string(), r#"[{"record":0},{"record":1},"off"]"#).unwrap();
    let decoded: Value = serde_json::from_str(&decode_recipes(&bytes).unwrap()).unwrap();
    assert_eq!(decoded["dial"], json!([{ "record": 0 }, "follow", "off"]));
}

/// A recipe file keeps the ID that the site generated for each recipe, and
/// a recipe without one comes back without one: INSTALL generates it.
#[test]
fn a_recipe_file_keeps_the_ids_of_its_recipes() {
    use grkit_wasm::{read_recipe_file, write_recipe_file};
    let recipe = |name: &str, id: u32| {
        json!({
            "name": name,
            "family": "gr3",
            "look": { "kind": "off" },
            "grain": { "level": 2, "size": 1 },
            "glow": null,
            "whiteBalance": null,
            "corrections": [null, null, null, null],
            "exposure": { "compensation": null, "isoLimit": null, "metering": null, "nd": null },
            "enabled": true,
            "hidden": false,
            "number": 0,
            "id": id,
        })
    };
    let library = json!([recipe("A", 123_456), recipe("B", 0)]);
    let text = write_recipe_file(&library.to_string(), "gr3", "", &[0, 0, 0]).unwrap();
    let read: Value = serde_json::from_str(&read_recipe_file(&text, "gr3").unwrap()).unwrap();
    assert_eq!(read, library);
}

/// The site writes what each user mode of the mode dial does, and reads it
/// back: 0 follows, `RECIPE_DIAL_OFF` (2^29 - 1, `DIAL_OFF` of
/// `web/src/recipes.ts`) turns GRKit off, `RECIPE_DIAL_SAVED` (2^29 - 2,
/// `DIAL_SAVED`) applies the settings that the camera saved, and a recipe ID
/// applies the recipe. A user mode whose recipe the write leaves out follows.
#[test]
fn a_recipe_file_gives_the_user_modes_of_the_mode_dial() {
    use grkit_wasm::{read_recipe_dial, write_recipe_file};
    const OFF: u32 = (1 << 29) - 1;
    const SAVED: u32 = (1 << 29) - 2;
    assert_eq!(OFF, grkit_core::runtime::RECIPE_DIAL_OFF);
    assert_eq!(SAVED, grkit_core::runtime::RECIPE_DIAL_SAVED);
    assert_eq!(SAVED - 1, grkit_core::runtime::RECIPE_ID_MAX);
    let library = json!([{
        "name": "A",
        "family": "gr3",
        "look": { "kind": "off" },
        "grain": null,
        "glow": null,
        "whiteBalance": null,
        "corrections": [null, null, null, null],
        "exposure": { "compensation": null, "isoLimit": null, "metering": null, "nd": null },
        "enabled": true,
        "hidden": false,
        "number": 0,
        "id": 77,
    }]);
    let text = write_recipe_file(&library.to_string(), "gr3", "", &[OFF, 77, 78]).unwrap();
    assert_eq!(read_recipe_dial(&text), [OFF, 77, 0]);
    let text = write_recipe_file(&library.to_string(), "gr3", &text, &[SAVED, 77, OFF]).unwrap();
    assert_eq!(read_recipe_dial(&text), [SAVED, 77, OFF]);
    assert_eq!(read_recipe_dial(""), [0, 0, 0]);
}
