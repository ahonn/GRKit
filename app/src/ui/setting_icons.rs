//! The icons of the settings of a recipe on its page
//! (`pages::recipe_text::Icon`): the camera's own icon of each setting,
//! redrawn as vectors on its 40 x 40 grid in one color, so that they follow
//! the theme. They use only filled paths (knockouts by `evenodd`), strokes,
//! clip paths and fill opacity: the app's SVG drawing has no masks, and a
//! gradient's `currentColor` does not follow the theme.

use nsh_core::element::ImageSrc;
use nsh_core::prelude::*;

/// The setting `icon` (one of the constants here), `size` points wide, in
/// the text color.
pub fn setting_icon(icon: &str, size: f32) -> AnyElement {
    let markup = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="40" viewBox="0 0 40 40" fill="currentColor">{icon}</svg>"#
    );
    svg(ImageSrc::svg_markup(&markup))
        .size(px(size))
        .flex_shrink(0.0)
        .text_color(move || nsh_base::theme().foreground)
        .into_element()
}

/// Three disks: Saturation.
pub const SATURATION: &str = r#"<path fill-rule="evenodd" d="M12.4 12.2a7.6 7.6 0 1 0 15.2 0a7.6 7.6 0 1 0 -15.2 0ZM3.6 27.4a7.6 7.6 0 1 0 15.2 0a7.6 7.6 0 1 0 -15.2 0ZM21.2 27.4a7.6 7.6 0 1 0 15.2 0a7.6 7.6 0 1 0 -15.2 0Z"/>"#;

/// A ring cut in six: Hue.
pub const HUE: &str = r#"<path fill-rule="evenodd" d="M36.45 18.7A16.5 16.5 0 0 0 29.35 6.41L25.98 12.24A9.8 9.8 0 0 1 29.71 18.7ZM27.1 5.11A16.5 16.5 0 0 0 12.9 5.11L16.27 10.94A9.8 9.8 0 0 1 23.73 10.94ZM10.65 6.41A16.5 16.5 0 0 0 3.55 18.7L10.29 18.7A9.8 9.8 0 0 1 14.02 12.24ZM3.55 21.3A16.5 16.5 0 0 0 10.65 33.59L14.02 27.76A9.8 9.8 0 0 1 10.29 21.3ZM12.9 34.89A16.5 16.5 0 0 0 27.1 34.89L23.73 29.06A9.8 9.8 0 0 1 16.27 29.06ZM29.35 33.59A16.5 16.5 0 0 0 36.45 21.3L29.71 21.3A9.8 9.8 0 0 1 25.98 27.76Z"/>"#;

/// A disk with a plus over a disk with a minus: High/Low Key.
pub const KEY: &str = r#"<clipPath id="k"><path clip-rule="evenodd" d="M0 0h40v40h-40ZM11.1 14.5a13.9 13.9 0 1 0 27.8 0a13.9 13.9 0 1 0 -27.8 0Z"/></clipPath><path fill-rule="evenodd" d="M2.5 27.5a9.5 9.5 0 1 0 19 0a9.5 9.5 0 1 0 -19 0ZM6.5 26.5h10v3.4h-10Z" clip-path="url(#k)"/><path fill-rule="evenodd" d="M13.5 14.5a11.5 11.5 0 1 0 23 0a11.5 11.5 0 1 0 -23 0ZM23.3 8.3L26.7 8.3L26.7 12.8L31.2 12.8L31.2 16.2L26.7 16.2L26.7 20.7L23.3 20.7L23.3 16.2L18.8 16.2L18.8 12.8L23.3 12.8Z"/>"#;

/// A ring with its right half filled: Contrast.
pub const CONTRAST: &str = r#"<path fill-rule="evenodd" d="M3 20a17 17 0 1 0 34 0a17 17 0 1 0 -34 0ZM20 5.8A14.2 14.2 0 0 0 20 34.2Z"/>"#;

/// Contrast with a triangle up: Contrast (Highlight).
pub const CONTRAST_HIGHLIGHT: &str = r#"<path fill-rule="evenodd" d="M1 24.5a13.5 13.5 0 1 0 27 0a13.5 13.5 0 1 0 -27 0ZM14.5 13.6A10.9 10.9 0 0 0 14.5 35.4ZM31.25 3L38 14L24.5 14Z"/>"#;

/// Contrast with a triangle down: Contrast (Shadow).
pub const CONTRAST_SHADOW: &str = r#"<path fill-rule="evenodd" d="M1 15.5a13.5 13.5 0 1 0 27 0a13.5 13.5 0 1 0 -27 0ZM14.5 4.6A10.9 10.9 0 0 0 14.5 26.4ZM24.5 26L38 26L31.25 37Z"/>"#;

/// A disk with an S cut out of it: Sharpness.
pub const SHARPNESS: &str = r#"<path fill-rule="evenodd" d="M3 20a17 17 0 1 0 34 0a17 17 0 1 0 -34 0ZM26.15 9.41A7.8 8.1 0 1 0 20 22.5A2.8 3.1 0 1 1 17.79 27.51L13.85 30.59A7.8 8.1 0 1 0 20 17.5A2.8 3.1 0 1 1 22.21 12.49Z"/>"#;

/// An oval ring round a faint oval: Shading.
pub const SHADING: &str = r#"<path fill-rule="evenodd" d="M8 20a12 17.5 0 1 0 24 0a12 17.5 0 1 0 -24 0ZM11 20a9 14.5 0 1 0 18 0a9 14.5 0 1 0 -18 0Z"/><path fill-rule="evenodd" d="M13.2 20a6.8 12.2 0 1 0 13.6 0a6.8 12.2 0 1 0 -13.6 0Z" fill-opacity="0.45"/>"#;

/// A sphere, light in its center: Clarity.
pub const CLARITY: &str = r#"<path fill-rule="evenodd" d="M12 20a8 8 0 1 0 16 0a8 8 0 1 0 -16 0Z"/><path fill-rule="evenodd" d="M9 20a11 11 0 1 0 22 0a11 11 0 1 0 -22 0ZM12 20a8 8 0 1 0 16 0a8 8 0 1 0 -16 0Z" fill-opacity="0.85"/><path fill-rule="evenodd" d="M6.5 20a13.5 13.5 0 1 0 27 0a13.5 13.5 0 1 0 -27 0ZM9 20a11 11 0 1 0 22 0a11 11 0 1 0 -22 0Z" fill-opacity="0.68"/><path fill-rule="evenodd" d="M4.5 20a15.5 15.5 0 1 0 31 0a15.5 15.5 0 1 0 -31 0ZM6.5 20a13.5 13.5 0 1 0 27 0a13.5 13.5 0 1 0 -27 0Z" fill-opacity="0.52"/><path fill-rule="evenodd" d="M3 20a17 17 0 1 0 34 0a17 17 0 1 0 -34 0ZM4.5 20a15.5 15.5 0 1 0 31 0a15.5 15.5 0 1 0 -31 0Z" fill-opacity="0.38"/>"#;

/// A disk with specks cut out of it: the grain of Image Control.
pub const NATIVE_GRAIN: &str = r#"<path fill-rule="evenodd" d="M3 20a17 17 0 1 0 34 0a17 17 0 1 0 -34 0ZM7.85 24.83a0.98 0.98 0 1 0 1.95 0a0.98 0.98 0 1 0 -1.95 0ZM20.35 15.7a0.8 0.8 0 1 0 1.6 0a0.8 0.8 0 1 0 -1.6 0ZM30.69 13.27a0.78 0.78 0 1 0 1.57 0a0.78 0.78 0 1 0 -1.57 0ZM6.92 13.87a0.85 0.85 0 1 0 1.7 0a0.85 0.85 0 1 0 -1.7 0ZM14.72 21.53a1.2 1.2 0 1 0 2.39 0a1.2 1.2 0 1 0 -2.39 0ZM16.46 14.05a1.23 1.23 0 1 0 2.45 0a1.23 1.23 0 1 0 -2.45 0ZM25.06 11.81a1.31 1.31 0 1 0 2.61 0a1.31 1.31 0 1 0 -2.61 0ZM19.65 32a1.15 1.15 0 1 0 2.3 0a1.15 1.15 0 1 0 -2.3 0ZM16.55 28.23a0.83 0.83 0 1 0 1.67 0a0.83 0.83 0 1 0 -1.67 0ZM27.25 22.34a1.22 1.22 0 1 0 2.44 0a1.22 1.22 0 1 0 -2.44 0ZM12.68 26.25a1.36 1.36 0 1 0 2.73 0a1.36 1.36 0 1 0 -2.73 0ZM21.39 18.6a1.17 1.17 0 1 0 2.33 0a1.17 1.17 0 1 0 -2.33 0ZM18.01 9.38a0.77 0.77 0 1 0 1.53 0a0.77 0.77 0 1 0 -1.53 0ZM11.08 16.51a0.84 0.84 0 1 0 1.68 0a0.84 0.84 0 1 0 -1.68 0ZM5.22 18.37a1.36 1.36 0 1 0 2.72 0a1.36 1.36 0 1 0 -2.72 0ZM8.78 11.42a0.86 0.86 0 1 0 1.71 0a0.86 0.86 0 1 0 -1.71 0ZM18.61 22.85a0.91 0.91 0 1 0 1.83 0a0.91 0.91 0 1 0 -1.83 0ZM15.74 7.31a1.02 1.02 0 1 0 2.05 0a1.02 1.02 0 1 0 -2.05 0ZM22.29 8.75a1.36 1.36 0 1 0 2.72 0a1.36 1.36 0 1 0 -2.72 0ZM14.14 12.47a0.82 0.82 0 1 0 1.64 0a0.82 0.82 0 1 0 -1.64 0ZM30.19 26.28a1.43 1.43 0 1 0 2.87 0a1.43 1.43 0 1 0 -2.87 0ZM10.16 29.97a0.98 0.98 0 1 0 1.96 0a0.98 0.98 0 1 0 -1.96 0ZM29.51 19.34a1.38 1.38 0 1 0 2.76 0a1.38 1.38 0 1 0 -2.76 0ZM23.6 15.21a1.44 1.44 0 1 0 2.87 0a1.44 1.44 0 1 0 -2.87 0ZM10.74 22.77a0.96 0.96 0 1 0 1.91 0a0.96 0.96 0 1 0 -1.91 0ZM26.06 25.13a0.86 0.86 0 1 0 1.72 0a0.86 0.86 0 1 0 -1.72 0ZM19.21 6.06a1.19 1.19 0 1 0 2.37 0a1.19 1.19 0 1 0 -2.37 0ZM9.67 13.98a0.79 0.79 0 1 0 1.57 0a0.79 0.79 0 1 0 -1.57 0ZM22.7 27.48a1.37 1.37 0 1 0 2.75 0a1.37 1.37 0 1 0 -2.75 0ZM17.76 18.92a1.11 1.11 0 1 0 2.22 0a1.11 1.11 0 1 0 -2.22 0ZM11.53 10.71a1.06 1.06 0 1 0 2.13 0a1.06 1.06 0 1 0 -2.13 0ZM14.96 16.57a0.95 0.95 0 1 0 1.91 0a0.95 0.95 0 1 0 -1.91 0ZM15.78 33.66a0.75 0.75 0 1 0 1.5 0a0.75 0.75 0 1 0 -1.5 0ZM26.32 30.33a1.31 1.31 0 1 0 2.62 0a1.31 1.31 0 1 0 -2.62 0ZM31.44 22.5a0.86 0.86 0 1 0 1.71 0a0.86 0.86 0 1 0 -1.71 0ZM20.89 24.5a0.76 0.76 0 1 0 1.52 0a0.76 0.76 0 1 0 -1.52 0ZM32.85 16.41a0.79 0.79 0 1 0 1.57 0a0.79 0.79 0 1 0 -1.57 0ZM19.44 12.49a1.3 1.3 0 1 0 2.6 0a1.3 1.3 0 1 0 -2.6 0ZM27.68 15.1a1.06 1.06 0 1 0 2.11 0a1.06 1.06 0 1 0 -2.11 0ZM12.74 19.35a0.92 0.92 0 1 0 1.84 0a0.92 0.92 0 1 0 -1.84 0ZM26.62 18.03a1.12 1.12 0 1 0 2.24 0a1.12 1.12 0 1 0 -2.24 0ZM22.42 22.45a0.76 0.76 0 1 0 1.52 0a0.76 0.76 0 1 0 -1.52 0ZM14.53 31.58a0.76 0.76 0 1 0 1.52 0a0.76 0.76 0 1 0 -1.52 0ZM24.22 20.47a0.95 0.95 0 1 0 1.9 0a0.95 0.95 0 1 0 -1.9 0ZM28.79 9.93a0.83 0.83 0 1 0 1.65 0a0.83 0.83 0 1 0 -1.65 0ZM7.71 28.55a0.86 0.86 0 1 0 1.72 0a0.86 0.86 0 1 0 -1.72 0Z"/>"#;

/// WB: the White Balance mode.
pub const WHITE_BALANCE: &str = r#"<path fill="none" stroke="currentColor" stroke-width="3.1" d="M3.5 11L8.12 29L12.75 15.5L17.38 29L22 11M24.5 11V29H31.23A4.77 4.77 0 0 0 31.23 19.46H24.5M24.5 19.46H30.83M24.5 11H30.63A4.23 4.23 0 0 1 30.63 19.46" stroke-linejoin="miter" stroke-miterlimit="2.2"/>"#;

/// WB over a slider: the shifts A/B and G/M.
pub const WB_SHIFT: &str = r#"<path fill="none" stroke="currentColor" stroke-width="2.6" d="M6 5L9.75 19L13.5 8.5L17.25 19L21 5M23 5V19H28.79A3.71 3.71 0 0 0 28.79 11.58H23M23 11.58H28.39M23 5H28.19A3.29 3.29 0 0 1 28.19 11.58" stroke-linejoin="miter" stroke-miterlimit="2.2"/><path fill="none" stroke="currentColor" stroke-width="2.4" d="M5 31H16.5M23.5 31H35"/><path fill="none" stroke="currentColor" stroke-width="2.4" d="M16.5 26.5h7v9h-7Z" stroke-linejoin="miter" stroke-miterlimit="2.2"/>"#;

/// A cube lit from above: Highlight Correction.
pub const HIGHLIGHT: &str = r#"<path fill-rule="evenodd" d="M20 4L34 11L20 18L6 11Z"/><path fill-rule="evenodd" d="M6 13.2L18.9 19.7L18.9 35L6 28.5Z" fill-opacity="0.72"/><path fill-rule="evenodd" d="M21.1 19.7L34 13.2L34 28.5L21.1 35ZM26 25.8L29.5 24L29.5 30.3L26 32.1Z" fill-opacity="0.42"/>"#;

/// A person in front of their shadow: Shadow Correction.
pub const SHADOW: &str = r#"<clipPath id="s"><path clip-rule="evenodd" d="M0 0h40v40h-40ZM6.2 11.6a8.8 8.8 0 1 0 17.6 0a8.8 8.8 0 1 0 -17.6 0Z"/></clipPath><clipPath id="t"><path clip-rule="evenodd" d="M0 0h40v40h-40ZM1.8 36C1.8 23.8 9 19.3 15 19.3C21 19.3 28.2 23.8 28.2 36Z"/></clipPath><g clip-path="url(#s)"><g clip-path="url(#t)"><path fill-rule="evenodd" d="M18.4 9.6a6.6 6.6 0 1 0 13.2 0a6.6 6.6 0 1 0 -13.2 0ZM14 34C14 24 19 19.5 25 19.5C31 19.5 36 24 36 34Z" fill-opacity="0.42"/></g></g><path fill-rule="evenodd" d="M8.4 11.6a6.6 6.6 0 1 0 13.2 0a6.6 6.6 0 1 0 -13.2 0Z"/><path fill-rule="evenodd" d="M4 36C4 26 9 21.5 15 21.5C21 21.5 26 26 26 36Z"/>"#;

/// A square dark in its center: Peripheral Illumination Correction.
pub const PERIPHERAL: &str = r#"<path fill-rule="evenodd" d="M9 5H31A4 4 0 0 1 35 9V31A4 4 0 0 1 31 35H9A4 4 0 0 1 5 31V9A4 4 0 0 1 9 5ZM6.8 20a13.2 13.2 0 1 0 26.4 0a13.2 13.2 0 1 0 -26.4 0Z"/><path fill-rule="evenodd" d="M6.8 20a13.2 13.2 0 1 0 26.4 0a13.2 13.2 0 1 0 -26.4 0ZM9.4 20a10.6 10.6 0 1 0 21.2 0a10.6 10.6 0 1 0 -21.2 0Z" fill-opacity="0.62"/><path fill-rule="evenodd" d="M9.4 20a10.6 10.6 0 1 0 21.2 0a10.6 10.6 0 1 0 -21.2 0ZM12 20a8 8 0 1 0 16 0a8 8 0 1 0 -16 0Z" fill-opacity="0.32"/><path fill-rule="evenodd" d="M12 20a8 8 0 1 0 16 0a8 8 0 1 0 -16 0ZM14.6 20a5.4 5.4 0 1 0 10.8 0a5.4 5.4 0 1 0 -10.8 0Z" fill-opacity="0.12"/>"#;

/// NR over steps: High-ISO Noise Reduction.
pub const NOISE: &str = r#"<path fill="none" stroke="currentColor" stroke-width="2.8" d="M8 19V5L17.5 19V5M22.5 19V5H28.36A3.64 3.64 0 0 1 28.36 12.28H22.5M27.36 12.28L32 19" stroke-linejoin="miter" stroke-miterlimit="2.2"/><path fill-rule="evenodd" d="M6 31h7v4h-7ZM16.5 27.5h7v7.5h-7ZM27 24h7v11h-7Z"/>"#;

/// Plus and minus across a square: Exposure Compensation.
pub const EXPOSURE: &str = r#"<path fill="none" stroke="currentColor" stroke-width="2.6" d="M8.8 5.3H31.2A3.5 3.5 0 0 1 34.7 8.8V31.2A3.5 3.5 0 0 1 31.2 34.7H8.8A3.5 3.5 0 0 1 5.3 31.2V8.8A3.5 3.5 0 0 1 8.8 5.3Z"/><path fill-rule="evenodd" d="M34.7 5.3V31.2A3.5 3.5 0 0 1 31.2 34.7H5.3ZM22.5 25.5h9v3.2h-9Z"/><path fill-rule="evenodd" d="M11.4 8L14.6 8L14.6 11.4L18 11.4L18 14.6L14.6 14.6L14.6 18L11.4 18L11.4 14.6L8 14.6L8 11.4L11.4 11.4Z"/>"#;

/// ISO AUTO: ISO Upper Limit.
pub const ISO: &str = r#"<path fill="none" stroke="currentColor" stroke-width="3" d="M9 5V20M21.9 6.88A4.5 3.75 0 1 0 18 12.5A4.5 3.75 0 1 1 14.1 18.12M25.5 12.5a4.75 7.5 0 1 0 9.5 0a4.75 7.5 0 1 0 -9.5 0Z" stroke-linejoin="miter" stroke-miterlimit="2.2"/><path fill="none" stroke="currentColor" stroke-width="2.2" d="M4.5 35.5L8 25.5L11.5 35.5M5.9 32.1H10.1M13.5 25.5V32.5A3 3 0 0 0 19.5 32.5V25.5M21.5 25.5H28M24.75 25.5V35.5M30 30.5a3.25 5 0 1 0 6.5 0a3.25 5 0 1 0 -6.5 0Z" stroke-linejoin="miter" stroke-miterlimit="2.2"/>"#;

/// A frame round ( o ): Metering.
pub const METERING: &str = r#"<path fill="none" stroke="currentColor" stroke-width="2.6" d="M7.5 9H32.5A3.5 3.5 0 0 1 36 12.5V27.5A3.5 3.5 0 0 1 32.5 31H7.5A3.5 3.5 0 0 1 4 27.5V12.5A3.5 3.5 0 0 1 7.5 9Z"/><path fill="none" stroke="currentColor" stroke-width="2.8" d="M15.4 20a4.6 4.6 0 1 0 9.2 0a4.6 4.6 0 1 0 -9.2 0Z"/><path fill="none" stroke="currentColor" stroke-width="2.6" d="M11.96 13.25A10.5 10.5 0 0 0 11.96 26.75M28.04 13.25A10.5 10.5 0 0 1 28.04 26.75" stroke-linecap="round"/>"#;

/// ND: the ND filter.
pub const ND: &str = r#"<path fill="none" stroke="currentColor" stroke-width="3.1" d="M6.5 29V11L18 29V11M23 11V29H26.85A7.65 9 0 0 0 26.85 11Z" stroke-linejoin="miter" stroke-miterlimit="2.2"/>"#;
