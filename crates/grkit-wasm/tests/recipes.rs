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
    }]);
    let bytes = encode_recipes(&library.to_string()).unwrap();
    let decoded: Value = serde_json::from_str(&decode_recipes(&bytes).unwrap()).unwrap();
    assert_eq!(decoded, library);
}
