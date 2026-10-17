//! Portable effect formats and SD package rules. All platform I/O lives outside this crate.

pub mod artwork;
pub mod cube;
pub mod effects;
pub mod fit;
pub mod modules;
pub mod packaging;
pub mod runtime;
pub mod selective;
pub mod site;

pub type Files = std::collections::BTreeMap<String, Vec<u8>>;
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
