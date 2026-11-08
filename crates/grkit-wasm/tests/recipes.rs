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
    let bytes = encode_recipes(&library.to_string()).unwrap();
    let decoded: Value = serde_json::from_str(&decode_recipes(&bytes).unwrap()).unwrap();
    assert_eq!(decoded, library);
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
    let text = write_recipe_file(&library.to_string(), "gr3", "").unwrap();
    let read: Value = serde_json::from_str(&read_recipe_file(&text, "gr3").unwrap()).unwrap();
    assert_eq!(read, library);
}
