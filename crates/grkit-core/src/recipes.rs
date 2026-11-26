//! The recipe library that the site writes to the card: `GRKIT/RECIPES.BIN`.
//!
//! A recipe is a named set of picture settings that the camera applies in one
//! step (`runtime/recipes.h`). The camera holds each recipe as
//! `RECIPE_WORDS` words, and a library record holds the same words, so the
//! camera applies a library recipe as it applies a recipe that it saved.
//! This module gives the portable model of a recipe, its words, and the file.
//!
//! The file is `HEADER_SIZE` bytes, then `RECORD_SIZE` bytes for each recipe
//! (`RCP_FIELDS` and `RCP_RECORD_FIELDS` in `abi`). INSTALL reads it into the
//! core and skips each record that the checks of `runtime/install_recipes.h`
//! reject.

mod file;
mod from_preset;
pub mod layout;
mod library;
mod model;
mod presets;
mod site;
#[cfg(test)]
mod tests;
mod words;

pub use file::{read_recipe_file, write_recipe_file};
pub use from_preset::from_preset;
pub use library::{check_library, checksum, decode, encode, HEADER_SIZE, MAGIC, RECORD_SIZE};
pub use model::*;
pub use presets::{presets, Preset};
pub use site::generated;
pub use words::WORD_LIMIT;

/// The name words of a recipe (`abi::RECIPE_NAME_WORDS`): four 7-bit ASCII
/// characters in each word, the first in the low bits, at most
/// `abi::RECIPE_NAME_CHARS`.
pub fn name_words(name: &str) -> [u32; crate::abi::RECIPE_NAME_WORDS] {
    let mut words = [0; crate::abi::RECIPE_NAME_WORDS];
    for (i, c) in name.bytes().take(crate::abi::RECIPE_NAME_CHARS).enumerate() {
        words[i / 4] |= u32::from(c & 0x7f) << (7 * (i % 4));
    }
    words
}

/// The name of `name_words`: the characters up to the first one that is not
/// printable ASCII, such as the 0 after a name. Empty for the automatic name.
pub fn name_of_words(words: &[u32]) -> String {
    (0..words.len() * 4)
        .map(|i| (words[i / 4] >> (7 * (i % 4)) & 0x7f) as u8)
        .take_while(|c| (0x20..=0x7e).contains(c))
        .map(char::from)
        .collect()
}
