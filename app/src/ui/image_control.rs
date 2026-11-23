//! The Image Control icons as the camera draws them: a dark tile with a thin
//! gray border and the type's short name in every UI language, on the
//! camera's 40-unit grid.

use nsh_core::element::ImageSrc;
use nsh_core::prelude::*;

use super::image_control_glyphs::LINES;

const TILE: &str = "#353536";
const BORDER: &str = "#808080";
const INK: &str = "#ffffff";
const DISC_INK: &str = "#000000";
const CINEMA_PERFORATION: &str = "#9b9b9b";
const CURVE_DIAGONAL: &str = "#808080";
const PERFORATION: f32 = 2.6;

const NEGATIVE: usize = 6;
const POSITIVE: usize = 7;
const BLEACH: usize = 8;
const RETRO: usize = 9;
const CROSS: usize = 11;
const CINEMA_YELLOW: usize = 12;
const CINEMA_GREEN: usize = 13;
const SOLARIZATION: usize = 20;

/// The perforations of a type, and their color.
fn perforations(image_type: usize) -> (Vec<(f32, f32)>, &'static str) {
    let steps = [0.0_f32, 1.0, 2.0, 3.0, 4.0].map(|step| 3.0 + step * 7.25);
    match image_type {
        NEGATIVE | POSITIVE | BLEACH | CROSS => (
            steps.iter().flat_map(|&x| [(x, 4.5), (x, 32.75)]).collect(),
            INK,
        ),
        CINEMA_YELLOW | CINEMA_GREEN => (
            steps.iter().flat_map(|&y| [(3.0, y), (34.4, y)]).collect(),
            CINEMA_PERFORATION,
        ),
        _ => (Vec::new(), INK),
    }
}

/// The icon of a type, in the order of `recipes.type.*`.
fn icon_svg(image_type: usize) -> String {
    let mut svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 40 40" width="40" height="40"><rect x="1.5" y="1.5" width="37" height="37" fill="{TILE}" stroke="{BORDER}" stroke-width="1"/>"#
    );
    let (holes, hole_color) = perforations(image_type);
    for (x, y) in holes {
        svg.push_str(&format!(
            r#"<rect x="{x}" y="{y}" width="{PERFORATION}" height="{PERFORATION}" fill="{hole_color}"/>"#
        ));
    }
    match image_type {
        RETRO => svg.push_str(&format!(
            r#"<defs><radialGradient id="disc" cx="20" cy="20" r="17" gradientUnits="userSpaceOnUse"><stop offset="0.78" stop-color="{INK}"/><stop offset="1" stop-color="{TILE}"/></radialGradient></defs><rect x="3" y="3" width="34" height="34" rx="17" fill="url(#disc)"/>"#
        )),
        CROSS => {
            for path in [
                "M7 14.5H12C17 14.5 19 25.5 25 25.5H30.5",
                "M7 25.5H12C14.4 25.5 15.8 23.2 16.9 21.1",
                "M19.4 17.9C20.9 15.9 22.6 14.5 25 14.5H30.5",
            ] {
                svg.push_str(&format!(
                    r#"<path d="{path}" fill="none" stroke="{INK}" stroke-width="2.2"/>"#
                ));
            }
            for head in ["M30 11.3L34.2 14.5L30 17.7Z", "M30 22.3L34.2 25.5L30 28.7Z"] {
                svg.push_str(&format!(r#"<path d="{head}" fill="{INK}"/>"#));
            }
        }
        SOLARIZATION => svg.push_str(&format!(
            r#"<path d="M5 35L37 3" fill="none" stroke="{CURVE_DIAGONAL}" stroke-width="1"/><path d="M4 34.5C6.5 27 9.5 13.5 14 13.5C19.5 13.5 19.5 27.5 25 27.5C31 27.5 33 12 36.5 3.5" fill="none" stroke="{INK}" stroke-width="2"/><path d="M2 2H4V35H38V37H2Z" fill="{INK}"/>"#
        )),
        _ => {}
    }
    let ink = if image_type == RETRO { DISC_INK } else { INK };
    for (path, weight) in LINES.get(image_type).copied().unwrap_or_default() {
        svg.push_str(&format!(
            r#"<path d="{path}" fill="{ink}" stroke="{ink}" stroke-width="{weight}" stroke-linejoin="round"/>"#
        ));
    }
    svg.push_str("</svg>");
    svg
}

/// The camera's icon of an Image Control type, `size` points square.
pub fn image_control_tile(image_type: usize, size: f32, faded: bool) -> AnyElement {
    image(ImageSrc::svg_markup(&icon_svg(image_type)))
        .w(px(size))
        .h(px(size))
        .object_fit(ResizeMode::Contain)
        .opacity(if faded { 0.45 } else { 1.0 })
        .into_element()
}
