//! Deterministic 40 × 40 RGBA menu tiles.
//!
//! Tiles follow the camera's own menu icons: a dark square with a gray frame
//! and anti-aliased text. Alpha is binary; text edges blend in RGB.

use ab_glyph::{point, Font, FontRef, PxScale, Rect, ScaleFont};
use anyhow::{bail, ensure, Result};

/// Inter Medium 4.1, SIL Open Font License 1.1 (`assets/Inter-LICENSE.txt`).
const FONT: &[u8] = include_bytes!("../assets/Inter-Medium.ttf");
/// Effect icon labels keep the character set of the original bitmap tiles.
const LABEL_CHARACTERS: &[u8] = b"EFGHMNOV01245";
/// Native text tiles, such as "Std", use about 15-pixel capitals.
const LABEL_SIZE: f32 = 21.0;
/// Text stays inside the frame with a small margin.
const LABEL_WIDTH: f32 = 30.0;

fn dot(rgba: &mut [u8], x: usize, y: usize, shade: u8) {
    let offset = (y * 40 + x) * 4;
    rgba[offset..offset + 4].copy_from_slice(&[shade, shade, shade, 255]);
}

fn background() -> Vec<u8> {
    let mut rgba = vec![0; 40 * 40 * 4];
    for y in 3..37 {
        for x in 3..37 {
            dot(
                &mut rgba,
                x,
                y,
                if x == 3 || x == 36 || y == 3 || y == 36 {
                    128
                } else {
                    51
                },
            );
        }
    }
    rgba
}

/// Blend `ink` over an opaque tile pixel by glyph coverage.
fn blend(rgba: &mut [u8], x: i32, y: i32, ink: u8, coverage: f32) {
    if !(0..40).contains(&x) || !(0..40).contains(&y) {
        return;
    }
    let offset = (y as usize * 40 + x as usize) * 4;
    if rgba[offset + 3] != 255 {
        return;
    }
    let coverage = coverage.clamp(0.0, 1.0);
    for channel in &mut rgba[offset..offset + 3] {
        let old = f32::from(*channel);
        *channel = (old + (f32::from(ink) - old) * coverage).round() as u8;
    }
}

/// Draw `label` centered horizontally, with its capitals centered on `middle`.
/// The size shrinks from `size` until the label fits within `width` pixels.
fn text(rgba: &mut [u8], label: &str, size: f32, width: f32, middle: f32, ink: u8) {
    let font = FontRef::try_from_slice(FONT).expect("embedded font is valid");
    let mut scale = size;
    loop {
        let scaled = font.as_scaled(PxScale::from(scale));
        let mut caret = 0.0;
        let mut previous = None;
        let mut glyphs = Vec::new();
        for character in label.chars() {
            let id = scaled.glyph_id(character);
            if let Some(previous) = previous {
                caret += scaled.kern(previous, id);
            }
            glyphs.push(id.with_scale_and_position(scale, point(caret, 0.0)));
            caret += scaled.h_advance(id);
            previous = Some(id);
        }
        let outlines: Vec<_> = glyphs
            .into_iter()
            .filter_map(|glyph| font.outline_glyph(glyph))
            .collect();
        let bounds = outlines
            .iter()
            .map(|outline| outline.px_bounds())
            .reduce(|a, b| Rect {
                min: point(a.min.x.min(b.min.x), a.min.y.min(b.min.y)),
                max: point(a.max.x.max(b.max.x), a.max.y.max(b.max.y)),
            });
        let Some(bounds) = bounds else { return };
        if bounds.width() > width && scale > 6.0 {
            scale -= 0.5;
            continue;
        }
        let capital = font
            .outline_glyph(scaled.glyph_id('H').with_scale(scale))
            .map_or(scale * 0.73, |outline| outline.px_bounds().height());
        // Whole-pixel offsets keep vertical and horizontal stems sharp.
        let dx = (20.0 - (bounds.min.x + bounds.max.x) / 2.0).round() as i32;
        let dy = (middle + capital / 2.0).round() as i32;
        for outline in outlines {
            let origin = outline.px_bounds().min;
            outline.draw(|x, y, coverage| {
                blend(
                    rgba,
                    origin.x as i32 + x as i32 + dx,
                    origin.y as i32 + y as i32 + dy,
                    ink,
                    coverage,
                )
            });
        }
        return;
    }
}

pub fn render_tile(label: &str, shape: &str) -> Result<Vec<u8>> {
    ensure!(
        (1..=4).contains(&label.len()) && label.bytes().all(|c| LABEL_CHARACTERS.contains(&c)),
        "Expected one to four supported label characters"
    );
    let mut rgba = background();
    let ink = match shape {
        "filter" => {
            // A white disc of radius 13.5, anti-aliased with 4 x 4 samples.
            for y in 5..35 {
                for x in 5..35 {
                    let inside = (0..16)
                        .filter(|sample| {
                            let sx = x as f32 + (sample % 4) as f32 / 4.0 + 0.125 - 20.0;
                            let sy = y as f32 + (sample / 4) as f32 / 4.0 + 0.125 - 20.0;
                            sx * sx + sy * sy <= 13.5 * 13.5
                        })
                        .count();
                    blend(&mut rgba, x, y, 255, inside as f32 / 16.0);
                }
            }
            30
        }
        "film" => {
            for top in [6, 31] {
                for left in [6, 12, 18, 24, 30] {
                    for y in top..top + 3 {
                        for x in left..left + 3 {
                            dot(&mut rgba, x, y, 230);
                        }
                    }
                }
            }
            255
        }
        "plain" => 255,
        _ => bail!("Expected film, filter, or plain tile shape"),
    };
    // Text inside the disc keeps clear of its curved edge.
    let (size, width) = if shape == "filter" {
        (17.0, 19.0)
    } else {
        (LABEL_SIZE, LABEL_WIDTH)
    };
    text(&mut rgba, label, size, width, 20.0, ink);
    Ok(rgba)
}

pub fn off_pixels() -> Vec<u8> {
    render_tile("OFF", "plain").expect("constant tile is valid")
}

/// A panoramic frame inside the same tile used by the other module entries.
pub fn framing_pixels() -> Vec<u8> {
    let mut rgba = background();
    for y in 12..28 {
        for x in 7..33 {
            let edge = x == 7 || x == 32 || y == 12 || y == 27;
            dot(&mut rgba, x, y, if edge { 255 } else { 20 });
        }
    }
    rgba
}

/// A frame with the proportions of `width` x `height`, centered above its label.
fn ratio_pixels(label: &str, width: usize, height: usize) -> Vec<u8> {
    let mut rgba = background();
    let (left, top) = ((40 - width) / 2, 6 + (19 - height) / 2);
    for y in top..top + height {
        for x in left..left + width {
            let edge = x == left || x == left + width - 1 || y == top || y == top + height - 1;
            dot(&mut rgba, x, y, if edge { 255 } else { 20 });
        }
    }
    text(&mut rgba, label, 11.0, 32.0, 30.5, 255);
    rgba
}

/// Framing artwork in runtime order: the module tile, then the XPan (65:24),
/// 16:9, and 4:3 ratios.
pub fn framing_tiles() -> Vec<Vec<u8>> {
    vec![
        framing_pixels(),
        ratio_pixels("XPan", 30, 11),
        ratio_pixels("16:9", 30, 17),
        ratio_pixels("4:3", 24, 18),
    ]
}

pub fn plugins_pixels() -> Vec<u8> {
    let mut rgba = background();
    text(&mut rgba, "GRK", LABEL_SIZE, LABEL_WIDTH, 20.0, 255);
    rgba
}
