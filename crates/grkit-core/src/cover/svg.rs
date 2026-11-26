//! The SVG of a cover, as the site draws it (`web/src/components/EffectIcon.tsx`).

use super::*;
use crate::cover_font::{ADVANCE, KERNING, OUTLINE};
use std::fmt::Write;

/// The size, letter spacing, and width of a cover text (`styles.css`, `EffectIcon.tsx`).
pub(super) struct Style {
    pub(super) size: f32,
    spacing: f32,
    width: f32,
}

const BRAND: Style = Style {
    size: 8.0,
    spacing: 0.0,
    width: 34.0,
};
const NAME: Style = Style {
    size: 6.6,
    spacing: 0.2,
    width: 38.0,
};
const NUMBER: Style = Style {
    size: 14.0,
    spacing: -0.3,
    width: 36.0,
};
pub(super) const LARGE_NAME: Style = Style {
    size: 12.0,
    spacing: -0.2,
    width: 38.0,
};
/// The smallest size of a large name, and the capital height as a share of the size.
const MIN_LARGE_SIZE: f32 = 5.5;
const CAP_HEIGHT: f32 = 0.7;

/// The index of a character in the tables of `cover_font`.
fn glyph(c: u8) -> usize {
    usize::from(c.clamp(b' ', b'~') - b' ')
}

/// The kerning between two characters, in thousandths of the font size.
fn kerning(left: u8, right: u8) -> i32 {
    KERNING
        .binary_search_by_key(&(left, right), |&(a, b, _)| (a, b))
        .map_or(0, |at| i32::from(KERNING[at].2))
}

/// The width of `text` as a browser measures it: the advances, the kerning, and the spacing
/// after each character.
fn text_width(text: &str, size: f32, spacing: f32) -> f32 {
    let bytes = text.as_bytes();
    let advances: i32 = bytes.iter().map(|&c| i32::from(ADVANCE[glyph(c)])).sum();
    let kerned: i32 = bytes.windows(2).map(|p| kerning(p[0], p[1])).sum();
    (advances + kerned) as f32 * size / 1000.0 + spacing * text.len() as f32
}

/// A number without trailing zeros.
fn number(value: f32) -> String {
    let text = format!("{value:.3}");
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}

pub(super) fn hex(rgb: Rgb) -> String {
    format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2])
}

fn escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            c => escaped.push(c),
        }
    }
    escaped
}

fn rect(svg: &mut String, y: f32, height: f32, fill: Rgb) {
    let _ = write!(
        svg,
        r#"<rect x="0" y="{}" width="40" height="{}" fill="{}"/>"#,
        number(y),
        number(height),
        hex(fill)
    );
}

/// A text centered on x = 20 at `size`, as the outlines of the site's font, so that it does
/// not depend on the fonts of the device. A text wider than the style width compresses to it,
/// as `fit` of the site does. A character outside the font's table draws as a text.
fn text(svg: &mut String, content: &str, y: f32, style: &Style, size: f32, fill: Rgb) {
    let length = text_width(content, size, style.spacing);
    let squeeze = if length > style.width {
        style.width / length
    } else {
        1.0
    };
    if !content.bytes().all(|c| (b' '..=b'~').contains(&c)) {
        let _ = write!(
            svg,
            r#"<text x="20" y="{}" font-size="{}" fill="{}""#,
            number(y),
            number(size),
            hex(fill)
        );
        if style.spacing != 0.0 {
            let _ = write!(svg, r#" letter-spacing="{}""#, number(style.spacing));
        }
        if squeeze < 1.0 {
            let _ = write!(
                svg,
                r#" textLength="{}" lengthAdjust="spacingAndGlyphs""#,
                number(style.width)
            );
        }
        let _ = write!(svg, ">{}</text>", escape(content));
        return;
    }
    let unit = size / 1000.0;
    let mut pen = 20.0 - length * squeeze / 2.0;
    let mut path = String::new();
    let bytes = content.as_bytes();
    for (at, &c) in bytes.iter().enumerate() {
        let mut across = true;
        for token in OUTLINE[glyph(c)].split_ascii_whitespace() {
            match token.parse::<f32>() {
                Ok(value) if across => {
                    let _ = write!(path, "{} ", number(pen + value * unit * squeeze));
                    across = false;
                }
                Ok(value) => {
                    let _ = write!(path, "{} ", number(y - value * unit));
                    across = true;
                }
                Err(_) => path.push_str(token),
            }
        }
        let kern = bytes.get(at + 1).map_or(0, |&next| kerning(c, next));
        pen += ((i32::from(ADVANCE[glyph(c)]) + kern) as f32 * unit + style.spacing) * squeeze;
    }
    let _ = write!(
        svg,
        r#"<path aria-label="{}" fill="{}" d="{}"/>"#,
        escape(content),
        hex(fill),
        path.trim_end()
    );
}

/// The size of a large name: smaller until it fits, down to `MIN_LARGE_SIZE`.
pub(super) fn large_size(name: &str) -> f32 {
    let style = &LARGE_NAME;
    let length = text_width(name, style.size, style.spacing);
    if length > style.width {
        (style.size * style.width / length).max(MIN_LARGE_SIZE)
    } else {
        style.size
    }
}

/// A large name: smaller until it fits, down to `MIN_LARGE_SIZE`, with its capitals centered
/// on `center`, as `fitLarge` of the site does.
fn large_name(svg: &mut String, name: &str, center: f32, fill: Rgb) {
    let style = &LARGE_NAME;
    let size = large_size(name);
    text(
        svg,
        name,
        center + size * CAP_HEIGHT / 2.0,
        style,
        size,
        fill,
    );
}

pub(super) fn film_svg(film: &Film) -> String {
    let colors = film.colors();
    let mut svg = String::from(concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 40 40" width="40" height="40""#,
        r#" font-family="'IBM Plex Sans Condensed', sans-serif" font-weight="700""#,
        r#" text-anchor="middle">"#
    ));
    rect(&mut svg, 3.0, 34.0, colors.body);
    // The rows of the brand, the name, and the number, and the center of a large name.
    let (brand_y, name_y, number_y, center) = if film.layout == Layout::Stack {
        let footer = 28.0;
        rect(&mut svg, footer, 37.0 - footer, colors.band);
        (34.8, 13.0, 24.5, (3.0 + footer) / 2.0)
    } else {
        let header_end = 12.0;
        rect(&mut svg, 3.0, header_end - 3.0, colors.band);
        let (number_y, body_end) = match film.layout {
            Layout::Box2 => {
                rect(&mut svg, 34.0, 3.0, colors.accent);
                (32.0, 34.0)
            }
            Layout::Cine => {
                for y in [32.0, 34.0, 36.0] {
                    rect(&mut svg, y, 1.0, colors.accent);
                }
                (30.2, 32.0)
            }
            _ => (33.0, 37.0),
        };
        (10.4, 19.0, number_y, (header_end + body_end) / 2.0)
    };
    text(
        &mut svg,
        &film.brand,
        brand_y,
        &BRAND,
        BRAND.size,
        colors.brand,
    );
    match &film.number {
        Some(value) => {
            text(&mut svg, &film.name, name_y, &NAME, NAME.size, colors.text);
            text(&mut svg, value, number_y, &NUMBER, NUMBER.size, colors.text);
        }
        None => large_name(&mut svg, &film.name, center, colors.text),
    }
    svg.push_str("</svg>");
    svg
}

/// 40 x 40 RGBA pixels as rectangles: one for each run of equal opaque pixels in a row.
pub(super) fn pixel_svg(rgba: &[u8]) -> String {
    let mut svg = String::from(concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 40 40" width="40" height="40""#,
        r#" shape-rendering="crispEdges">"#
    ));
    let pixel = |x: usize, y: usize| {
        let at = (y * 40 + x) * 4;
        rgba.get(at..at + 4)
            .filter(|p| p[3] != 0)
            .map(|p| [p[0], p[1], p[2]])
    };
    for y in 0..40 {
        let mut x = 0;
        while x < 40 {
            let Some(color) = pixel(x, y) else {
                x += 1;
                continue;
            };
            let start = x;
            while x < 40 && pixel(x, y) == Some(color) {
                x += 1;
            }
            let _ = write!(
                svg,
                r#"<rect x="{start}" y="{y}" width="{}" height="1" fill="{}"/>"#,
                x - start,
                hex(color)
            );
        }
    }
    svg.push_str("</svg>");
    svg
}
