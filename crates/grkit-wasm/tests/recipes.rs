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

/// A write gives the text of the recipe file and the recipes with the IDs
/// that the file gives them, so the site keeps the IDs that the card holds.
#[test]
fn a_recipe_file_write_gives_the_ids_of_its_recipes() {
    use grkit_wasm::{read_recipe_file, write_recipe_file};
    let recipe = |name: &str| {
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
        })
    };
    let library = json!([recipe("A"), recipe("B")]);
    let file: Value =
        serde_json::from_str(&write_recipe_file(&library.to_string(), "gr3", "").unwrap()).unwrap();
    let ids: Vec<u64> = file["recipes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|recipe| recipe["id"].as_u64().unwrap())
        .collect();
    assert_eq!(ids, [1, 2]);
    let read: Value =
        serde_json::from_str(&read_recipe_file(file["text"].as_str().unwrap(), "gr3").unwrap())
            .unwrap();
    assert_eq!(read, file["recipes"]);
}
