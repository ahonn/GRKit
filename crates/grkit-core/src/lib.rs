//! Portable effect formats and SD package rules. All platform I/O lives outside this crate.

pub mod abi;
pub mod artwork;
pub mod camera;
pub mod capabilities;
pub mod card;
pub mod color;
pub mod cube;
pub mod effects;
pub mod fit;
pub mod grm;
pub mod guide;
pub mod hue;
pub mod language;
pub mod lut_image;
pub mod menu_text;
pub mod modules;
pub mod native;
pub mod packaging;
pub mod png;
pub mod recipes;
pub mod runtime;
pub mod selective;
pub mod site;
#[doc(hidden)]
pub mod testing;
mod tiff;

pub type Files = std::collections::BTreeMap<String, Vec<u8>>;
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
