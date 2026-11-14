//! The tiles of the Framing module are those that `examples/module-icons.rs`
//! draws from the catalog of `grkit_core::aspect`.

#[path = "../examples/module-icons.rs"]
#[allow(dead_code)]
mod icons;

use grkit_core::{aspect::RATIOS, png};
use std::path::PathBuf;

#[test]
fn framing_tiles_are_those_of_the_generator() {
    let tiles = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../modules/framing/tiles");
    for ratio in &RATIOS[1..] {
        let (width, height) = ratio.aspect;
        let pixels = icons::ratio_tile(f64::from(width) / f64::from(height), ratio.label).unwrap();
        let file = std::fs::read(tiles.join(ratio.tile)).unwrap();
        let expected = png::encode_rgba8(40, 40, &pixels).unwrap();
        assert_eq!(
            png::decode_rgba8(&file).unwrap().2,
            pixels,
            "{}",
            ratio.tile
        );
        assert_eq!(
            file, expected,
            "{} is the encoding of the generator",
            ratio.tile
        );
    }
}
