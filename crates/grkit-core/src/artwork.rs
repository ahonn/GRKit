//! Deterministic 40 × 40 RGBA menu tiles.
//!
//! Tiles follow the camera's own menu icons: a dark square with a gray frame
//! and anti-aliased text. Alpha is binary; text edges blend in RGB.

use ab_glyph::{point, Font, FontRef, PxScale, Rect, ScaleFont};
use anyhow::{bail, ensure, Result};

/// Inter Medium 4.1, SIL Open Font License 1.1 (`assets/Inter-LICENSE.txt`).
const FONT: &[u8] = include_bytes!("../assets/Inter-Medium.ttf");
/// Effect icon labels use capitals and digits, as the native text tiles do.
const LABEL_CHARACTERS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
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

/// Composition guide styles, in runtime order after Off.
#[derive(Clone, Copy)]
enum Guide {
    Thirds,
    Center,
    Golden,
    Diagonals,
}

/// A frame of about 3:2 with its guide lines, centered horizontally. Lines are
/// gray, so they stay distinct from the frame.
fn guide_frame(rgba: &mut [u8], top: usize, guide: Guide) {
    let (width, height) = (28, 18);
    let left = (40 - width) / 2;
    let (right, bottom) = (left + width - 1, top + height - 1);
    for y in top..=bottom {
        for x in left..=right {
            let edge = x == left || x == right || y == top || y == bottom;
            dot(rgba, x, y, if edge { 255 } else { 20 });
        }
    }
    // Symmetric line offsets from the left and top edges.
    let grid = |rgba: &mut [u8], column: usize, row: usize| {
        for x in [left + column, right - column] {
            (top + 1..bottom).for_each(|y| dot(rgba, x, y, 170));
        }
        for y in [top + row, bottom - row] {
            (left + 1..right).for_each(|x| dot(rgba, x, y, 170));
        }
    };
    match guide {
        Guide::Thirds => grid(rgba, 9, 6),
        Guide::Golden => grid(rgba, 10, 7),
        Guide::Center => {
            // Even dimensions center the cross between two pixel rows and columns.
            for offset in [0, 1] {
                (left + 9..=right - 9).for_each(|x| dot(rgba, x, top + 8 + offset, 170));
                (top + 4..=bottom - 4).for_each(|y| dot(rgba, left + 13 + offset, y, 170));
            }
        }
        Guide::Diagonals => {
            for x in 1..width - 1 {
                // Round half up, as the runtime's Bresenham lines do.
                let y = (2 * x * (height - 1) + width - 1) / (2 * (width - 1));
                dot(rgba, left + x, top + y, 170);
                dot(rgba, right - x, top + y, 170);
            }
        }
    }
}

/// A guide frame centered above its label.
fn guide_pixels(label: &str, guide: Guide) -> Vec<u8> {
    let mut rgba = background();
    guide_frame(&mut rgba, 6, guide);
    text(&mut rgba, label, 11.0, 32.0, 30.5, 255);
    rgba
}

/// Guides artwork in runtime order: the module tile, then thirds, center,
/// golden ratio, and diagonals.
pub fn guides_tiles() -> Vec<Vec<u8>> {
    let mut module = background();
    guide_frame(&mut module, 11, Guide::Thirds);
    vec![
        module,
        guide_pixels("3x3", Guide::Thirds),
        guide_pixels("Ctr", Guide::Center),
        guide_pixels("Phi", Guide::Golden),
        guide_pixels("Diag", Guide::Diagonals),
    ]
}

/// Date stamp artwork in runtime order: the module tile, then the 'YY M D,
/// 'YY M D H:MM, M D 'YY, and D M 'YY styles, each named by a sample.
pub fn stamp_tiles() -> Vec<Vec<u8>> {
    ["DATE", "'26", "9:05", "M D", "D M"]
        .iter()
        .map(|label| {
            let mut rgba = background();
            text(&mut rgba, label, LABEL_SIZE, LABEL_WIDTH, 20.0, 255);
            rgba
        })
        .collect()
}

/// Light speckles in rows `top..bottom`, one in `period` pixels on average.
/// A fixed hash places them, so the artwork stays deterministic.
fn speckles(rgba: &mut [u8], top: usize, bottom: usize, period: u32) {
    for y in top..bottom {
        for x in 6..34 {
            let hash = (x as u32)
                .wrapping_mul(0x9e37_79b1)
                .wrapping_add((y as u32).wrapping_mul(0x85eb_ca77))
                .rotate_left(13)
                .wrapping_mul(0xc2b2_ae3d);
            if (hash >> 16) % period == 0 {
                dot(rgba, x, y, 220);
            }
        }
    }
}

/// Grain artwork in runtime order: the module tile, then the weak, medium,
/// and strong levels, with more speckles at each level.
pub fn grain_tiles() -> Vec<Vec<u8>> {
    let mut module = background();
    speckles(&mut module, 6, 34, 4);
    let mut tiles = vec![module];
    for (label, period) in [("Weak", 9), ("Med", 5), ("Strong", 3)] {
        let mut rgba = background();
        speckles(&mut rgba, 6, 24, period);
        text(&mut rgba, label, 11.0, 32.0, 30.5, 255);
        tiles.push(rgba);
    }
    tiles
}

/// Module artwork in runtime order. The menu addresses each tile by its
/// offset from the first Framing tile.
pub fn module_tiles() -> Vec<Vec<u8>> {
    let mut tiles = framing_tiles();
    tiles.extend(guides_tiles());
    tiles.extend(stamp_tiles());
    tiles.extend(grain_tiles());
    tiles
}

pub fn plugins_pixels() -> Vec<u8> {
    let mut rgba = background();
    text(&mut rgba, "GRK", LABEL_SIZE, LABEL_WIDTH, 20.0, 255);
    rgba
}
