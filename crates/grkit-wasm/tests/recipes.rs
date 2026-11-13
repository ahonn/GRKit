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
        "border": { "style": 3, "color": 1 },
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
            "border": null,
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

/// The recipe file holds recipes only: a write of the site drops the keys of
/// the user modes of the mode dial that earlier builds kept after the IDs.
#[test]
fn a_recipe_file_drops_the_keys_of_the_mode_dial() {
    use grkit_core::runtime::{RECIPE_ID_KEY_BASE, RECIPE_SLOTS};
    use grkit_wasm::write_recipe_file;
    let library = json!([{
        "name": "A",
        "family": "gr3",
        "look": { "kind": "off" },
        "grain": null,
        "glow": null,
        "border": null,
        "whiteBalance": null,
        "corrections": [null, null, null, null],
        "exposure": { "compensation": null, "isoLimit": null, "metering": null, "nd": null },
        "enabled": true,
        "hidden": false,
        "number": 0,
        "id": 77,
    }])
    .to_string();
    let dial = RECIPE_ID_KEY_BASE + RECIPE_SLOTS as u32;
    let card = write_recipe_file(&library, "gr3", "").unwrap();
    let card = format!("{card}{dial}\r\n77\r\n{}\r\n536870911\r\n", dial + 1);
    let text = write_recipe_file(&library, "gr3", &card).unwrap();
    let keys: Vec<u32> = text
        .split_whitespace()
        .step_by(2)
        .map(|word| word.parse().unwrap())
        .collect();
    assert!(keys.contains(&(RECIPE_ID_KEY_BASE)));
    assert!(keys.iter().all(|&key| !(dial..dial + 3).contains(&key)));
}
