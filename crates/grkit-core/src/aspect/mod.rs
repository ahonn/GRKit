//! The capability CORE.ASPECT (`runtime/aspect.h`): the Framing ratios, the
//! one source of each ratio for its consumers, such as the Framing module,
//! Crop, Border, Guides, and the RAW development.
//!
//! - `ratios`: the catalog, each value with its names, label, width : height,
//!   geometry, and families, and the order of the rows.
//! - `geometry`: the stock sizes of each family, the size of a ratio in a
//!   size class, the classification of a photo size, the preview insets, and
//!   the live-view boxes.
//! - `header`: the runtime header `runtime/framing_ratios.h`.
//!
//! The site lists the ratios from `site::generated`. The Framing module keeps
//! its own rows and tiles, and tests check them, and the docs, against the
//! catalog.
//!
//! A new ratio takes the next value, because the camera saves the value and a
//! value keeps its meaning across releases. It also needs a label at the end
//! of `artwork::OVERLAY_LABELS`, a tile, which `examples/module-icons.rs`
//! draws from the catalog, and its rows in the Framing module.

pub mod geometry;
pub mod header;
pub mod ratios;

pub use ratios::{label_index, names, rows, Geometry, Ratio, Rounding, RATIOS};

/// The generated runtime header.
pub fn generated() -> Vec<(&'static str, String)> {
    vec![(header::PATH, header::header())]
}
