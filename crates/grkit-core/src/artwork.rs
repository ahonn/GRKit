//! Deterministic 40 × 40 RGBA menu tiles.
//!
//! Tiles follow the camera's own menu icons: a dark square with a gray frame
//! and anti-aliased text. The native list icons have a 38 x 36 frame, and the
//! icons of the ADJ toolbar a 38 x 38 frame. Effect icons can instead be gray film covers
//! (`render_box`, `render_cine`) that span the tile width. Alpha is binary;
//! text edges blend in RGB.

use ab_glyph::{point, Font, FontRef, PxScale, Rect, ScaleFont};
use anyhow::{bail, ensure, Context, Result};

/// Roboto Condensed Bold, an instance of Roboto Condensed 3.008 at weight 700
/// with the ASCII characters only, SIL Open Font License 1.1
/// (`assets/RobotoCondensed-LICENSE.txt`). The native toolbar icons, such as
/// "Focus", have bold condensed text.
const FONT: &[u8] = include_bytes!("../assets/RobotoCondensed-Bold.ttf");
/// Effect icon labels use capitals and digits, as the native text tiles do.
const LABEL_CHARACTERS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
/// The largest label size. Its capitals are about 10 pixels high, close to the
/// 11 pixels of the native toolbar text, such as "Focus". A label that does
/// not fit `LABEL_WIDTH` at this size shrinks.
const LABEL_SIZE: f32 = 18.0;
/// Text keeps at least 4 pixels from each side of the 36-pixel frame inside.
const LABEL_WIDTH: f32 = 28.0;

fn dot(rgba: &mut [u8], x: usize, y: usize, shade: u8) {
    let offset = (y * 40 + x) * 4;
    rgba[offset..offset + 4].copy_from_slice(&[shade, shade, shade, 255]);
}

/// A dark square with a gray frame on columns `left..=right` and rows
/// `top..=bottom`. The rest of the tile stays transparent.
fn frame(left: usize, top: usize, right: usize, bottom: usize) -> Vec<u8> {
    let mut rgba = vec![0; 40 * 40 * 4];
    for y in top..=bottom {
        for x in left..=right {
            let edge = x == left || x == right || y == top || y == bottom;
            dot(&mut rgba, x, y, if edge { 128 } else { 51 });
        }
    }
    rgba
}

/// The frame of a list row, as the native list icons have it.
fn background() -> Vec<u8> {
    frame(1, 2, 38, 37)
}

/// The frame of the ADJ toolbar, as the native toolbar icons have it.
fn toolbar_background() -> Vec<u8> {
    frame(1, 1, 38, 38)
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

/// Roboto Condensed Black, the weight 900 instance of the same release, with
/// the same characters and license as `FONT`. Only the cover icons use it.
const COVER_FONT: &[u8] = include_bytes!("../assets/RobotoCondensed-Black.ttf");
/// Cover shades: the light bands, the dark text on them, and the body.
const LIGHT: u8 = 214;
const DARK: u8 = 40;
const BODY: u8 = 51;
/// The cover rows, with a 3-pixel transparent margin above and below. The cover spans the full tile width, so that a name
/// such as KODACHROME fits on one line.
const COVER: i32 = 3;
const COVER_END: i32 = 37;
const COVER_WIDTH: i32 = 40;
/// A cover text may compress to this horizontal scale before it is rejected.
const MIN_COMPRESSION: f32 = 0.75;

fn fill(rgba: &mut [u8], top: i32, bottom: i32, shade: u8) {
    for y in top..bottom {
        for x in 0..COVER_WIDTH {
            dot(rgba, x as usize, y as usize, shade);
        }
    }
}

/// Draw `label` in Roboto Condensed Black, centered on x = 20, with `capital` pixels of
/// capital height on the `baseline` row. The text compresses horizontally to
/// fit `width` pixels. Returns `None`, and draws nothing, if the text would
/// need a horizontal scale below `min_compression`.
#[allow(clippy::too_many_arguments)]
fn cover_text(
    rgba: &mut [u8],
    label: &str,
    capital: f32,
    baseline: i32,
    tracking: f32,
    width: f32,
    min_compression: f32,
    ink: u8,
) -> Option<()> {
    let font = FontRef::try_from_slice(COVER_FONT).expect("embedded font is valid");
    let unit = PxScale::from(100.0);
    let size = 100.0 * capital
        / font
            .outline_glyph(font.glyph_id('H').with_scale(unit))
            .expect("the font has H")
            .px_bounds()
            .height();
    let mut compression = 1.0f32;
    loop {
        let scale = PxScale {
            x: size * compression,
            y: size,
        };
        let scaled = font.as_scaled(scale);
        let mut caret = 0.0;
        let outlines: Vec<_> = label
            .chars()
            .filter_map(|character| {
                let id = scaled.glyph_id(character);
                let glyph = id.with_scale_and_position(scale, point(caret, 0.0));
                caret += scaled.h_advance(id) + tracking * size * compression;
                font.outline_glyph(glyph)
            })
            .collect();
        let left = outlines
            .iter()
            .map(|o| o.px_bounds().min.x)
            .reduce(f32::min)?;
        let right = outlines
            .iter()
            .map(|o| o.px_bounds().max.x)
            .reduce(f32::max)?;
        let actual = right - left;
        if actual > width {
            compression *= width / actual * 0.999;
            if compression < min_compression {
                return None;
            }
            continue;
        }
        // Whole-pixel offsets keep the stems sharp, as in `text`.
        let dx = (20.0 - actual / 2.0 - left).round() as i32;
        for outline in outlines {
            let origin = outline.px_bounds().min;
            outline.draw(|x, y, coverage| {
                blend(
                    rgba,
                    origin.x as i32 + x as i32 + dx,
                    origin.y as i32 + y as i32 + baseline,
                    ink,
                    coverage,
                )
            });
        }
        return Some(());
    }
}

/// Five-pixel capitals for cover text too small for an outline font.
/// Each string is one row, and `#` is ink.
fn pixel_glyph(character: char) -> Option<[&'static str; 5]> {
    Some(match character {
        'A' => [".#.", "#.#", "###", "#.#", "#.#"],
        'B' => ["##.", "#.#", "##.", "#.#", "##."],
        'C' => [".##", "#..", "#..", "#..", ".##"],
        'D' => ["##.", "#.#", "#.#", "#.#", "##."],
        'E' => ["###", "#..", "##.", "#..", "###"],
        'F' => ["###", "#..", "##.", "#..", "#.."],
        'G' => [".##", "#..", "#.#", "#.#", ".##"],
        'H' => ["#.#", "#.#", "###", "#.#", "#.#"],
        'I' => ["#", "#", "#", "#", "#"],
        'J' => ["..#", "..#", "..#", "#.#", ".#."],
        'K' => ["#.#", "#.#", "##.", "#.#", "#.#"],
        'L' => ["#..", "#..", "#..", "#..", "###"],
        'M' => ["#...#", "##.##", "#.#.#", "#...#", "#...#"],
        'N' => ["#..#", "##.#", "#.##", "#..#", "#..#"],
        'O' => [".#.", "#.#", "#.#", "#.#", ".#."],
        'P' => ["##.", "#.#", "##.", "#..", "#.."],
        'Q' => [".#.", "#.#", "#.#", "##.", ".##"],
        'R' => ["##.", "#.#", "##.", "#.#", "#.#"],
        'S' => [".##", "#..", ".#.", "..#", "##."],
        'T' => ["###", ".#.", ".#.", ".#.", ".#."],
        'U' => ["#.#", "#.#", "#.#", "#.#", "###"],
        'V' => ["#.#", "#.#", "#.#", "#.#", ".#."],
        'W' => ["#...#", "#...#", "#.#.#", "##.##", "#...#"],
        'X' => ["#.#", "#.#", ".#.", "#.#", "#.#"],
        'Y' => ["#.#", "#.#", ".#.", ".#.", ".#."],
        'Z' => ["###", "..#", ".#.", "#..", "###"],
        '0' => ["###", "#.#", "#.#", "#.#", "###"],
        '1' => [".#", "##", ".#", ".#", ".#"],
        '2' => ["##.", "..#", ".#.", "#..", "###"],
        '3' => ["##.", "..#", ".#.", "..#", "##."],
        '4' => ["#.#", "#.#", "###", "..#", "..#"],
        '5' => ["###", "#..", "##.", "..#", "##."],
        '6' => [".##", "#..", "###", "#.#", "###"],
        '7' => ["###", "..#", ".#.", ".#.", ".#."],
        '8' => ["###", "#.#", "###", "#.#", "###"],
        '9' => ["###", "#.#", "###", "..#", "##."],
        '-' => ["..", "..", "##", "..", ".."],
        '+' => ["...", ".#.", "###", ".#.", "..."],
        ' ' => [".", ".", ".", ".", "."],
        _ => return None,
    })
}

/// Draw `label` in the pixel font, centered on x = 20, with its top row at `top`.
/// Narrow forms of the five-pixel `M` and `W`, for a text that does not fit
/// with the regular forms. They read less clearly, so only such a text uses them.
fn narrow_glyph(character: char) -> Option<[&'static str; 5]> {
    match character {
        'M' => Some(["#.#", "###", "###", "#.#", "#.#"]),
        'W' => Some(["#.#", "#.#", "###", "###", "#.#"]),
        other => pixel_glyph(other),
    }
}

/// The width of glyphs with one blank column between them.
fn pixel_width(glyphs: &[[&str; 5]]) -> i32 {
    glyphs
        .iter()
        .map(|rows| rows[0].len() as i32 + 1)
        .sum::<i32>()
        - 1
}

fn pixel_text(rgba: &mut [u8], label: &str, top: i32, ink: u8) -> Result<()> {
    let glyphs_of = |glyph: fn(char) -> Option<[&'static str; 5]>| {
        label
            .chars()
            .map(glyph)
            .collect::<Option<Vec<_>>>()
            .context("Expected capitals, digits, spaces, '-', or '+' in the cover text")
    };
    let mut glyphs = glyphs_of(pixel_glyph)?;
    if pixel_width(&glyphs) > COVER_WIDTH {
        glyphs = glyphs_of(narrow_glyph)?;
    }
    let width = pixel_width(&glyphs);
    ensure!(
        width <= COVER_WIDTH,
        "Cover text {label:?} is {width} pixels wide, more than {COVER_WIDTH}"
    );
    let mut x = 20 - (width + 1) / 2;
    for rows in glyphs {
        for (dy, row) in rows.iter().enumerate() {
            for (dx, bit) in row.bytes().enumerate() {
                if bit == b'#' {
                    blend(rgba, x + dx as i32, top + dy as i32, ink, 1.0);
                }
            }
        }
        x += rows[0].len() as i32 + 1;
    }
    Ok(())
}

fn check_cover_label(label: &str, what: &str) -> Result<()> {
    ensure!(
        (1..=12).contains(&label.len()) && label.bytes().all(|c| c.is_ascii_graphic() || c == b' '),
        "Expected 1 to 12 printable ASCII characters in the cover {what}"
    );
    Ok(())
}

/// The bottom of a cover.
#[derive(Clone, Copy)]
enum Bottom {
    /// The dark body continues to the bottom row.
    None,
    /// A light band, as on professional film boxes.
    Band,
    /// Three light stripes, as on motion picture film labels.
    Stripes,
}

/// The capital heights, in pixels, that a cover tries for a large name, largest first.
const LARGE_NAME: [f32; 5] = [9.0, 8.0, 7.0, 6.0, 5.0];
/// The capital heights of a name that is too wide for the pixel font, largest first.
const SMALL_NAME: [f32; 2] = [5.0, 4.0];
/// Capital heights of a cover number, the full size first.
const NUMBER_CAPITALS: [f32; 3] = [11.0, 10.0, 9.0];
/// A name may compress to this horizontal scale before a smaller size is tried.
const NAME_COMPRESSION: f32 = 0.6;
/// The width of a name in Roboto Condensed Black: the tile width less one pixel on each side.
const NAME_WIDTH: f32 = 38.0;

/// Draws `name` in Roboto Condensed Black at the largest capital height in `capitals` that fits,
/// centered vertically on `center`. Returns false, and draws nothing, if no size fits.
fn fitted_name(rgba: &mut [u8], name: &str, capitals: &[f32], center: f32) -> bool {
    capitals.iter().any(|&capital| {
        let baseline = (center + capital / 2.0).round() as i32;
        cover_text(
            rgba,
            name,
            capital,
            baseline,
            -0.02,
            NAME_WIDTH,
            NAME_COMPRESSION,
            255,
        )
        .is_some()
    })
}

/// A gray cover: a light band with the brand, then a dark body with the film
/// name and a large number. The brand uses Roboto Condensed Black when it fits without
/// compression, and the pixel font otherwise. The name uses the pixel font. A
/// name that the pixel font cannot draw in 40 pixels uses a smaller Roboto Condensed Black.
/// An empty `number` means no number: the name then uses Roboto Condensed Black, as large
/// as fits, centered on the body. A name that fits no large size uses the pixel
/// font, and then the smaller Roboto Condensed Black.
fn cover(brand: &str, name: &str, number: &str, bottom: Bottom) -> Result<Vec<u8>> {
    check_cover_label(brand, "brand")?;
    check_cover_label(name, "name")?;
    if !number.is_empty() {
        check_cover_label(number, "number")?;
    }
    let mut rgba = vec![0; 40 * 40 * 4];
    fill(&mut rgba, COVER, COVER_END, BODY);
    fill(&mut rgba, COVER, 12, LIGHT);
    if cover_text(&mut rgba, brand, 6.0, 10, -0.04, 28.0, 1.0, DARK).is_none() {
        pixel_text(&mut rgba, brand, 5, DARK)?;
    }
    // The number sits one row lower without a bottom band, and one row
    // higher above the stripes. `body_end` is the first row below the body.
    let (name_top, baseline, body_end) = match bottom {
        Bottom::None => (15, 33, COVER_END),
        Bottom::Band => {
            fill(&mut rgba, 34, COVER_END, LIGHT);
            (14, 32, 34)
        }
        Bottom::Stripes => {
            for row in [32, 34, 36] {
                fill(&mut rgba, row, row + 1, LIGHT);
            }
            (14, 31, 32)
        }
    };
    if number.is_empty() {
        // The body starts below the brand band, on row 12.
        let center = (12 + body_end) as f32 / 2.0;
        if !fitted_name(&mut rgba, name, &LARGE_NAME, center) {
            // The 5-pixel font, centered on the body.
            let pixel = pixel_text(&mut rgba, name, 12 + (body_end - 12 - 5) / 2, 255);
            if pixel.is_err() && !fitted_name(&mut rgba, name, &SMALL_NAME, center) {
                pixel.with_context(|| format!("Cover name {name:?} does not fit"))?;
            }
        }
        return Ok(rgba);
    }
    // `pixel_text` draws nothing when it fails, so a smaller Roboto Condensed Black can take its place.
    let pixel = pixel_text(&mut rgba, name, name_top, 255);
    if pixel.is_err() && !fitted_name(&mut rgba, name, &SMALL_NAME, name_top as f32 + 2.5) {
        pixel.with_context(|| format!("Cover name {name:?} does not fit"))?;
    }
    // A number that does not fit at full size takes a smaller size with the same
    // vertical center.
    NUMBER_CAPITALS
        .iter()
        .find_map(|&capital| {
            let baseline = baseline - ((NUMBER_CAPITALS[0] - capital) / 2.0).round() as i32;
            cover_text(
                &mut rgba,
                number,
                capital,
                baseline,
                -0.04,
                34.0,
                MIN_COMPRESSION,
                255,
            )
        })
        .with_context(|| format!("Cover number {number:?} does not fit"))?;
    Ok(rgba)
}

/// A still film box. Professional films have a light band at the bottom.
/// An empty `number` gives a large name and no number.
pub fn render_box(brand: &str, name: &str, number: &str, band: bool) -> Result<Vec<u8>> {
    cover(
        brand,
        name,
        number,
        if band { Bottom::Band } else { Bottom::None },
    )
}

/// A motion picture film label, with three light stripes at the bottom.
/// An empty `number` gives a large name and no number.
pub fn render_cine(brand: &str, name: &str, number: &str) -> Result<Vec<u8>> {
    cover(brand, name, number, Bottom::Stripes)
}

pub fn off_pixels() -> Vec<u8> {
    render_tile("OFF", "plain").expect("constant tile is valid")
}

/// The GRKit entry of the ADJ toolbar.
pub fn grkit_pixels() -> Vec<u8> {
    let mut rgba = toolbar_background();
    text(&mut rgba, "GRK", LABEL_SIZE, LABEL_WIDTH, 20.0, 255);
    rgba
}

/// The About row, which gives the version. Its own tile keeps a second GRK
/// tile out of the GRKit root.
pub fn version_pixels() -> Vec<u8> {
    let mut rgba = background();
    text(&mut rgba, "VER", LABEL_SIZE, LABEL_WIDTH, 20.0, 255);
    rgba
}

/// The Modules row, which lists the modules.
pub fn modules_pixels() -> Vec<u8> {
    let mut rgba = background();
    text(&mut rgba, "MOD", LABEL_SIZE, LABEL_WIDTH, 20.0, 255);
    rgba
}

/// The Modules tile with a line through it: the root row of Enable GRKit
/// while it is Off.
pub fn modules_off_pixels() -> Vec<u8> {
    let mut rgba = modules_pixels();
    strike(&mut rgba);
    rgba
}

/// The Fn hint of a row where Fn turns a setting to its next value: a
/// clockwise open circle arrow, white with a black edge on a transparent
/// tile, as the native settings icon of the Fn hint has it. The arrow has a
/// gap at the top right and its head at the top, pointing right.
pub fn rotate_pixels() -> Vec<u8> {
    // 4 x 4 samples in each pixel. The black edge is the white shape grown
    // by EDGE samples.
    const SAMPLES: usize = 4;
    const SIDE: usize = 40 * SAMPLES;
    const EDGE: i32 = 5;
    const CENTER: (f32, f32) = (20.0, 21.0);
    const RADIUS: f32 = 11.0;
    const HALF_WIDTH: f32 = 2.0;
    // The arc runs clockwise (y down) from START to END degrees.
    const START: f32 = -15.0;
    const END: f32 = 268.0;
    const HEAD: f32 = 6.5;
    let end = END.to_radians();
    let tip = (CENTER.0 + RADIUS * end.cos(), CENTER.1 + RADIUS * end.sin());
    let (along, across) = ((-end.sin(), end.cos()), (end.cos(), end.sin()));
    let head = [
        (tip.0 + along.0 * HEAD, tip.1 + along.1 * HEAD),
        (
            tip.0 + across.0 * HEAD - along.0 * 0.5,
            tip.1 + across.1 * HEAD - along.1 * 0.5,
        ),
        (
            tip.0 - across.0 * HEAD - along.0 * 0.5,
            tip.1 - across.1 * HEAD - along.1 * 0.5,
        ),
    ];
    let in_head = |x: f32, y: f32| {
        let side = |(x1, y1): (f32, f32), (x2, y2): (f32, f32)| {
            (x2 - x1) * (y - y1) - (y2 - y1) * (x - x1)
        };
        let s = [
            side(head[0], head[1]),
            side(head[1], head[2]),
            side(head[2], head[0]),
        ];
        s.iter().all(|v| *v >= 0.0) || s.iter().all(|v| *v <= 0.0)
    };
    let mut white = vec![false; SIDE * SIDE];
    for j in 0..SIDE {
        for i in 0..SIDE {
            let (x, y) = (
                (i as f32 + 0.5) / SAMPLES as f32,
                (j as f32 + 0.5) / SAMPLES as f32,
            );
            let (dx, dy) = (x - CENTER.0, y - CENTER.1);
            let mut angle = dy.atan2(dx).to_degrees();
            while angle < START {
                angle += 360.0;
            }
            let on_arc = ((dx * dx + dy * dy).sqrt() - RADIUS).abs() <= HALF_WIDTH && angle <= END;
            white[j * SIDE + i] = on_arc || in_head(x, y);
        }
    }
    let mut edge = vec![false; SIDE * SIDE];
    for j in 0..SIDE as i32 {
        for i in 0..SIDE as i32 {
            if !white[j as usize * SIDE + i as usize] {
                continue;
            }
            for dy in -EDGE..=EDGE {
                for dx in -EDGE..=EDGE {
                    let (x, y) = (i + dx, j + dy);
                    if dx * dx + dy * dy <= EDGE * EDGE
                        && (0..SIDE as i32).contains(&x)
                        && (0..SIDE as i32).contains(&y)
                    {
                        edge[y as usize * SIDE + x as usize] = true;
                    }
                }
            }
        }
    }
    // Alpha is binary: a pixel that the edge covers at least half is opaque,
    // and its shade is the white share of the covered samples.
    let mut rgba = vec![0; 40 * 40 * 4];
    for y in 0..40 {
        for x in 0..40 {
            let (mut w, mut e) = (0, 0);
            for b in 0..SAMPLES {
                for a in 0..SAMPLES {
                    let at = (y * SAMPLES + b) * SIDE + x * SAMPLES + a;
                    w += usize::from(white[at]);
                    e += usize::from(edge[at]);
                }
            }
            if 2 * e >= SAMPLES * SAMPLES {
                dot(&mut rgba, x, y, ((255 * w + e / 2) / e) as u8);
            }
        }
    }
    rgba
}

/// Mark a list tile as Off, as the native Off icons do: dim the drawing
/// halfway to the body shade, then draw a white line with a black edge on
/// the diagonal x = y, from the top left to the bottom right of the frame.
/// Both have square ends, and the black edge passes the white ends. The
/// widths and the 16 x 16 samples of each pixel reproduce the shades of the
/// native Off icons. The line stops 1.5 pixels shorter at each end than the
/// native one, clear of the frame corners.
pub fn strike(rgba: &mut [u8]) {
    // Half widths across the line, and the ends along it, as (x + y) / 2.
    const WHITE: (f32, f32, f32) = (1.55, 5.25, 34.75);
    const BLACK: (f32, f32, f32) = (2.83, 4.2, 35.8);
    let inside = |(half, start, end): (f32, f32, f32), x: f32, y: f32| {
        (x - y).abs() / std::f32::consts::SQRT_2 <= half && (start..=end).contains(&((x + y) / 2.0))
    };
    for y in 3..37 {
        for x in 2..38 {
            let at = (y * 40 + x) * 4;
            for channel in &mut rgba[at..at + 3] {
                if *channel > BODY {
                    *channel = BODY + (*channel - BODY) / 2;
                }
            }
            let (mut white, mut black) = (0u32, 0u32);
            for sample in 0..256 {
                let sx = x as f32 + ((sample % 16) as f32 + 0.5) / 16.0;
                let sy = y as f32 + ((sample / 16) as f32 + 0.5) / 16.0;
                white += u32::from(inside(WHITE, sx, sy));
                black += u32::from(inside(BLACK, sx, sy));
            }
            if black == 0 {
                continue;
            }
            let (white, black) = (white as f32 / 256.0, black as f32 / 256.0);
            for channel in &mut rgba[at..at + 3] {
                *channel = (f32::from(*channel) * (1.0 - black) + 255.0 * white).round() as u8;
            }
        }
    }
}

/// The live-view labels in label order: the load cue, then the Framing ratios
/// in capture area order (runtime/framing.c: photo_boxes), so label n names
/// capture area n, then the sign of Date Imprint (LABEL_DATE in
/// runtime/sdk/grkit.h), then its formats in Date Imprint STYLE order
/// (label LABEL_DATE + STYLE): the order of year, month, and day, and HM
/// when the time prints.
pub const OVERLAY_LABELS: [&str; 14] = [
    "GRKit Loaded",
    "XPan",
    "16:9",
    "4:3",
    "2.35",
    "1:1",
    "5:4",
    "7:6",
    "DATE",
    "YMD",
    "YMD HM",
    "MDY",
    "DMY",
    "DMY HM",
];
/// The width of the black outline around the text. The native live-view text
/// has one black pixel around the gray edge of each stroke, and keeps the
/// gaps between characters transparent.
pub const OVERLAY_OUTLINE: usize = 1;
/// The capital height of the native live-view text, such as the focus
/// distance "2.5m".
pub const OVERLAY_CAPITAL: usize = 24;
/// Rows above the capitals for ascenders and round overshoot, such as the d
/// and the G of "GRKit Loaded", and rows below the baseline for overshoot.
const OVERLAY_ABOVE: usize = 2;
const OVERLAY_BELOW: usize = 1;
/// The height of each label: the capitals, the rows above and below them,
/// and the outline.
pub const OVERLAY_LABEL_HEIGHT: usize =
    OVERLAY_ABOVE + OVERLAY_CAPITAL + OVERLAY_BELOW + 2 * OVERLAY_OUTLINE;
/// Roboto Condensed Light, an instance of Roboto Condensed 3.008 at weight 300
/// with the ASCII characters only, SIL Open Font License 1.1
/// (`assets/RobotoCondensed-LICENSE.txt`). Its shapes and weight are close
/// to those of the native live-view text.
const OVERLAY_FONT: &[u8] = include_bytes!("../assets/RobotoCondensed-Light.ttf");
/// The native text is narrower than Roboto Condensed: its digits are about 13
/// pixels wide.
const OVERLAY_CONDENSE: f32 = 0.87;
/// The native text has more space between its characters.
const OVERLAY_TRACKING: f32 = 3.0;

/// The capital heights of a Date Imprint format that prints the time: the
/// date on the first line, HM on a smaller second line. The two lines,
/// with their outline, are as high as the marks of a status-row icon, such
/// as Grain or Bloom: 36 rows. A format without the time is one line of the
/// ratio size, as high as XPan.
const OVERLAY_DATE_CAPITAL: usize = 17;
const OVERLAY_TIME_CAPITAL: usize = 13;
/// Rows between the date line and the time line. Fewer rows let the
/// outlines of the two lines touch.
const OVERLAY_LINE_GAP: usize = 4;
/// The height of a two-line label: the marks of a status-row icon, 36 rows
/// with the outline, and the rows above and below the capitals.
pub const OVERLAY_TWO_LINE_HEIGHT: usize = OVERLAY_ABOVE + 36 + OVERLAY_BELOW;
const _: () = assert!(
    OVERLAY_DATE_CAPITAL + OVERLAY_LINE_GAP + OVERLAY_TIME_CAPITAL + 2 * OVERLAY_OUTLINE == 36
);
/// The tracking of the time line. Tracking that scales with its small
/// capitals merges the outlines of H and M.
const OVERLAY_TIME_TRACKING: f32 = 3.0;

/// The lines of label `index` with their capital heights and tracking. The
/// tracking of the date line scales with its capitals.
fn overlay_lines(index: usize) -> Vec<(&'static str, usize, f32)> {
    let label = OVERLAY_LABELS[index];
    let date = OVERLAY_LABELS
        .iter()
        .position(|candidate| *candidate == "DATE")
        .expect("the labels have DATE");
    match label.split_once(' ') {
        Some((date_line, time)) if index > date => vec![
            (
                date_line,
                OVERLAY_DATE_CAPITAL,
                OVERLAY_TRACKING * OVERLAY_DATE_CAPITAL as f32 / OVERLAY_CAPITAL as f32,
            ),
            (time, OVERLAY_TIME_CAPITAL, OVERLAY_TIME_TRACKING),
        ],
        _ => vec![(label, OVERLAY_CAPITAL, OVERLAY_TRACKING)],
    }
}

/// The text coverage of label `index`: its width, its height, then one
/// coverage value for each pixel, row by row. Each line is centered, its
/// capitals start below the outline and `OVERLAY_ABOVE` rows, and its tracking
/// comes from `overlay_lines`. A one-line label is `OVERLAY_LABEL_HEIGHT`
/// high, and a two-line label `OVERLAY_TWO_LINE_HEIGHT`.
fn overlay_coverage(index: usize) -> (usize, usize, Vec<f32>) {
    let font = FontRef::try_from_slice(OVERLAY_FONT).expect("embedded font is valid");
    let unit = font
        .outline_glyph(font.glyph_id('H').with_scale(100.0))
        .expect("the font has H")
        .px_bounds()
        .height()
        / 100.0;
    let mut lines = Vec::new();
    for (text, capital, tracking) in overlay_lines(index) {
        let size = capital as f32 / unit;
        let scale = PxScale {
            x: size * OVERLAY_CONDENSE,
            y: size,
        };
        let scaled = font.as_scaled(scale);
        let mut caret = 0.0;
        let mut previous = None;
        let mut outlines = Vec::new();
        for character in text.chars() {
            let id = scaled.glyph_id(character);
            if let Some(previous) = previous {
                caret += scaled.kern(previous, id);
            }
            outlines
                .extend(font.outline_glyph(id.with_scale_and_position(scale, point(caret, 0.0))));
            caret += scaled.h_advance(id) + tracking;
            previous = Some(id);
        }
        let bounds = outlines
            .iter()
            .map(|outline| outline.px_bounds())
            .reduce(|a, b| Rect {
                min: point(a.min.x.min(b.min.x), a.min.y.min(b.min.y)),
                max: point(a.max.x.max(b.max.x), a.max.y.max(b.max.y)),
            })
            .expect("a label has outlines");
        lines.push((outlines, bounds, capital));
    }
    let text_width = |bounds: &Rect| bounds.width().ceil() as usize;
    let width = lines
        .iter()
        .map(|(_, bounds, _)| text_width(bounds))
        .max()
        .unwrap()
        + 2 * OVERLAY_OUTLINE;
    let height = 2 * OVERLAY_OUTLINE
        + OVERLAY_ABOVE
        + lines.iter().map(|(_, _, capital)| capital).sum::<usize>()
        + OVERLAY_LINE_GAP * (lines.len() - 1)
        + OVERLAY_BELOW;
    let mut coverage = vec![0f32; width * height];
    let mut top = OVERLAY_OUTLINE + OVERLAY_ABOVE;
    for (outlines, bounds, capital) in lines {
        // Whole-pixel offsets keep vertical and horizontal stems sharp. The
        // baseline is the row after the capitals.
        let center = (width - 2 * OVERLAY_OUTLINE - text_width(&bounds)) / 2;
        let dx = (OVERLAY_OUTLINE + center) as i32 - bounds.min.x.floor() as i32;
        let dy = (top + capital) as i32;
        for outline in outlines {
            let origin = outline.px_bounds().min;
            outline.draw(|x, y, value| {
                let (x, y) = (
                    origin.x as i32 + x as i32 + dx,
                    origin.y as i32 + y as i32 + dy,
                );
                if (0..width as i32).contains(&x) && (0..height as i32).contains(&y) {
                    let cell = &mut coverage[y as usize * width + x as usize];
                    *cell = (*cell + value).min(1.0);
                }
            });
        }
        top += capital + OVERLAY_LINE_GAP;
    }
    (width, height, coverage)
}

/// The width, the height, and the pixels of label `index`: 0 where the label is
/// transparent, else the gray of
/// an opaque pixel, 1 for the black outline and up to 255 for white text.
/// Text edges blend toward the outline.
fn overlay_label(index: usize) -> (usize, usize, Vec<u8>) {
    let (width, height, coverage) = overlay_coverage(index);
    // The rasterizer leaves tiny coverage between overlapping contours, so
    // only coverage that rounds to a gray counts as text.
    let coverage: Vec<f32> = coverage
        .into_iter()
        .map(|value| if value * 255.0 < 0.5 { 0.0 } else { value })
        .collect();
    let radius = OVERLAY_OUTLINE as i32;
    let inked = |x: i32, y: i32| {
        (0..width as i32).contains(&x)
            && (0..height as i32).contains(&y)
            && coverage[y as usize * width + x as usize] > 0.0
    };
    let mut pixels = vec![0u8; width * height];
    for y in 0..height as i32 {
        for x in 0..width as i32 {
            let value = coverage[y as usize * width + x as usize];
            // A round outline: each pixel within the radius of the text.
            let outlined = (-radius..=radius).any(|oy| {
                (-radius..=radius)
                    .any(|ox| ox * ox + oy * oy <= radius * radius + 1 && inked(x + ox, y + oy))
            });
            pixels[y as usize * width + x as usize] = if value > 0.0 {
                ((value * 255.0).round() as u8).max(1)
            } else if outlined {
                1
            } else {
                0
            };
        }
    }
    (width, height, pixels)
}

/// The label table that the installer copies after the menu text
/// (`GRKIT_LABELS` in `runtime/abi.h`): for label n, a u16 width, a u16
/// height, and the u32 offset of its rows at 8 n, then the rows of each
/// label (`overlay_label`). Each row holds only its drawn pixels: a u8 span
/// count, then for each span a u16 first column, a u16 length, and its
/// pixels. A full redraw of the live view paints the labels, so it skips
/// the transparent pixels without a test for each. The live view draws them
/// as its own text: white with a black outline, without a background.
pub fn overlay_labels() -> Vec<u8> {
    let mut table = vec![0; OVERLAY_LABELS.len() * 8];
    for index in 0..OVERLAY_LABELS.len() {
        let (width, height, pixels) = overlay_label(index);
        let offset = table.len() as u32;
        table[index * 8..index * 8 + 2].copy_from_slice(&(width as u16).to_le_bytes());
        table[index * 8 + 2..index * 8 + 4].copy_from_slice(&(height as u16).to_le_bytes());
        table[index * 8 + 4..index * 8 + 8].copy_from_slice(&offset.to_le_bytes());
        for row in pixels.chunks(width) {
            let mut spans = Vec::new();
            let mut x = 0;
            while x < width {
                if row[x] == 0 {
                    x += 1;
                    continue;
                }
                let first = x;
                while x < width && row[x] != 0 {
                    x += 1;
                }
                spans.push((first, x - first));
            }
            table.push(u8::try_from(spans.len()).expect("a label row has at most 255 spans"));
            for (first, length) in spans {
                table.extend((first as u16).to_le_bytes());
                table.extend((length as u16).to_le_bytes());
                table.extend(&row[first..first + length]);
            }
        }
    }
    // Word alignment for the next copy.
    table.resize(table.len().next_multiple_of(4), 0);
    table
}

/// Label `index` of a label table (`overlay_labels`): its width, its height,
/// and each of its pixels, 0 where it is transparent.
pub fn overlay_label_pixels(table: &[u8], index: usize) -> (usize, usize, Vec<u8>) {
    let field = |at: usize, bytes: usize| {
        (0..bytes).fold(0usize, |value, i| {
            value | usize::from(table[at + i]) << (8 * i)
        })
    };
    let (width, height, mut at) = (
        field(index * 8, 2),
        field(index * 8 + 2, 2),
        field(index * 8 + 4, 4),
    );
    let mut pixels = vec![0; width * height];
    for y in 0..height {
        let spans = usize::from(table[at]);
        at += 1;
        for _ in 0..spans {
            let (first, length) = (field(at, 2), field(at + 2, 2));
            at += 4;
            pixels[y * width + first..y * width + first + length]
                .copy_from_slice(&table[at..at + length]);
            at += length;
        }
    }
    (width, height, pixels)
}
