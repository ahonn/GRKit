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
pub mod layout;
mod library;
mod model;
mod presets;
mod site;
#[cfg(test)]
mod tests;
mod words;

pub use file::{read_recipe_file, write_recipe_file, RecipeFile};
pub use library::{check_library, checksum, decode, encode, HEADER_SIZE, MAGIC, RECORD_SIZE};
pub use model::*;
pub use presets::{presets, Preset};
pub use site::generated;
pub use words::WORD_LIMIT;
