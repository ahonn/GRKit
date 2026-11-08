//! Reproduce the Glow, Grain, Date, Framing, and Guides icons and the viewfinder status icons.
//! Run from the repository root with `cargo run -p grkit-core --example module-icons`.
use ab_glyph::{point, Font, FontRef, PxScale, ScaleFont};
use anyhow::{ensure, Context, Result};
use grkit_core::{artwork, png};
use std::{f64::consts::TAU, fs, path::Path};

const SIZE: usize = 40;
const BACKGROUND: u8 = 51;
// Glow strength symbols sit above the strength steps, like the Grain dots.
const GLOW_CENTER: (f64, f64) = (20.0, 15.0);
const OUTER_RADIUS: f64 = 8.5;
const INNER_RADIUS: f64 = 5.5;
const RING_WIDTH: f64 = 2.0;
// The center light: small inside Bloom's two rings, larger inside Halation's one.
const BLOOM_LIGHT: f64 = 1.5;
const HALATION_LIGHT: f64 = 3.0;
const SAMPLES: usize = 16;

// Preserve the shared list frame and clear its text to obtain a drawing surface.
fn list_tile() -> Vec<u8> {
    let mut pixels = artwork::off_pixels();
    for y in 3..37 {
        for x in 2..38 {
            let offset = (y * SIZE + x) * 4;
            pixels[offset..offset + 4].copy_from_slice(&[BACKGROUND, BACKGROUND, BACKGROUND, 255]);
        }
    }
    pixels
}

// Bloom has two rings and Halation one, both around the same center light.
// The steps below carry the strength, so the symbol stays the same.
fn glow(bloom: bool) -> Vec<u8> {
    glow_at(bloom, GLOW_CENTER)
}

fn glow_at(bloom: bool, center: (f64, f64)) -> Vec<u8> {
    let mut pixels = list_tile();
    let radii: &[f64] = if bloom {
        &[OUTER_RADIUS, INNER_RADIUS]
    } else {
        &[OUTER_RADIUS]
    };
    let light = if bloom { BLOOM_LIGHT } else { HALATION_LIGHT };
    let top = (center.1 - OUTER_RADIUS - 3.0) as usize;
    for y in top..top + 22 {
        for x in 9..31 {
            let mut total = 0u32;
            for sy in 0..SAMPLES {
                for sx in 0..SAMPLES {
                    let dx = x as f64 + (sx as f64 + 0.5) / SAMPLES as f64 - center.0;
                    let dy = y as f64 + (sy as f64 + 0.5) / SAMPLES as f64 - center.1;
                    let radius = dx.hypot(dy);
                    let gray = if radius <= light {
                        255
                    } else if radii
                        .iter()
                        .any(|r| radius <= *r && radius >= r - RING_WIDTH)
                    {
                        240
                    } else {
                        BACKGROUND
                    };
                    total += u32::from(gray);
                }
            }
            let gray = ((total as f64 / (SAMPLES * SAMPLES) as f64).round()) as u8;
            let offset = (y * SIZE + x) * 4;
            pixels[offset..offset + 4].copy_from_slice(&[gray, gray, gray, 255]);
        }
    }
    pixels
}

// Off has no steps: the rings center in the frame, and the native Off line
// strikes them.
fn glow_off(bloom: bool) -> Vec<u8> {
    let mut pixels = glow_at(bloom, (GLOW_CENTER.0, 20.0));
    artwork::strike(&mut pixels);
    pixels
}

// Move the drawing inside the list frame down by `rows`.
fn shift_down(pixels: &mut [u8], rows: usize) {
    for y in (3..37).rev() {
        for x in 2..38 {
            let offset = (y * SIZE + x) * 4;
            let value = if y >= 3 + rows {
                pixels[((y - rows) * SIZE + x) * 4]
            } else {
                BACKGROUND
            };
            pixels[offset..offset + 3].fill(value);
        }
    }
}

fn save(path: &Path, pixels: &[u8]) -> Result<()> {
    let encoded = png::encode_rgba8(SIZE, SIZE, pixels)?;
    // Do not rewrite unrelated PNG encodings when their pixels already match.
    if path.exists() && png::decode_rgba8(&fs::read(path)?)?.2 == pixels {
        return Ok(());
    }
    fs::write(path, encoded)?;
    println!("Updated {}", path.display());
    Ok(())
}

fn draw_text(
    pixels: &mut [u8],
    label: &str,
    size: f32,
    baseline: f32,
    top: f32,
    bottom: f32,
) -> Result<()> {
    let font = FontRef::try_from_slice(include_bytes!("../assets/RobotoCondensed-Bold.ttf"))?;
    let scaled = font.as_scaled(PxScale::from(size));
    let mut caret = 0.0;
    let mut previous = None;
    let mut outlines = Vec::new();
    for character in label.chars() {
        let id = scaled.glyph_id(character);
        if let Some(previous) = previous {
            caret += scaled.kern(previous, id);
        }
        if let Some(outline) =
            font.outline_glyph(id.with_scale_and_position(scaled.scale(), point(caret, baseline)))
        {
            outlines.push(outline);
        }
        caret += scaled.h_advance(id);
        previous = Some(id);
    }
    let left = outlines
        .iter()
        .map(|o| o.px_bounds().min.x)
        .reduce(f32::min)
        .context("Empty label")?;
    let right = outlines
        .iter()
        .map(|o| o.px_bounds().max.x)
        .reduce(f32::max)
        .context("Empty label")?;
    ensure!(right - left <= 30.0, "Label too wide: {label}");
    let dx = (20.0 - (left + right) / 2.0).round() as i32;
    for outline in outlines {
        let bounds = outline.px_bounds();
        ensure!(
            bounds.min.y >= top && bounds.max.y <= bottom,
            "Label outside safe rows: {label}, {bounds:?}"
        );
        outline.draw(|x, y, coverage| {
            let x = bounds.min.x as i32 + x as i32 + dx;
            let y = bounds.min.y as usize + y as usize;
            let offset = (y * SIZE + x as usize) * 4;
            let gray =
                (f32::from(BACKGROUND) + (255.0 - f32::from(BACKGROUND)) * coverage).round() as u8;
            pixels[offset..offset + 3].fill(gray);
        });
    }
    Ok(())
}

// Antialias onto the opaque list fill. Alpha remains binary at the frame edge.
fn paint(pixels: &mut [u8], bounds: [usize; 4], ink: u8, inside: impl Fn(f64, f64) -> bool) {
    let [left, top, right, bottom] = bounds;
    for y in top..bottom {
        for x in left..right {
            let mut count = 0;
            for sy in 0..SAMPLES {
                for sx in 0..SAMPLES {
                    if inside(
                        x as f64 + (sx as f64 + 0.5) / SAMPLES as f64,
                        y as f64 + (sy as f64 + 0.5) / SAMPLES as f64,
                    ) {
                        count += 1;
                    }
                }
            }
            let offset = (y * SIZE + x) * 4;
            let old = f64::from(pixels[offset]);
            let gray = (old + (f64::from(ink) - old) * count as f64 / (SAMPLES * SAMPLES) as f64)
                .round() as u8;
            pixels[offset..offset + 3].fill(gray);
        }
    }
}

// Every level uses a prefix of one deterministic, evenly dispersed point set.
// Keep particle size fixed: Grain Size is a separate camera setting.
fn grain(height: usize, count: usize) -> Vec<u8> {
    let mut pixels = list_tile();
    let low = 6.8;
    let high_x = 33.2;
    let high_y = 6.0 + height as f64 - 0.8;
    let mut points = vec![
        (low, low + 3.5),
        (high_x, high_y - 3.5),
        (14.5, low),
        (25.5, high_y),
    ];
    let mut state = 0x47524149u32;
    let mut random = || {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        f64::from(state) / f64::from(u32::MAX)
    };
    while points.len() < count {
        let mut best = (low, low);
        let mut distance = -1.0;
        for _ in 0..128 {
            let candidate = (
                low + random() * (high_x - low),
                low + random() * (high_y - low),
            );
            let nearest = points
                .iter()
                .map(|&(x, y)| (candidate.0 - x).powi(2) + (candidate.1 - y).powi(2))
                .fold(f64::INFINITY, f64::min);
            if nearest > distance {
                best = candidate;
                distance = nearest;
            }
        }
        points.push(best);
    }
    for (x, y) in points {
        let radius: f64 = 0.75;
        paint(
            &mut pixels,
            [
                (x - radius).floor() as usize,
                (y - radius).floor() as usize,
                (x + radius).ceil() as usize,
                (y + radius).ceil() as usize,
            ],
            240,
            |sx, sy| (sx - x).powi(2) + (sy - y).powi(2) <= radius.powi(2),
        );
    }
    pixels
}

// List tiles: strength steps below a symbol, and diagrams with a caption.
// Diagrams keep 1-pixel lines; captions are larger than the earlier 12.
const CAPTION_SIZE: f32 = 14.0;
const CAPTION_BASELINE: f32 = 33.0;
const DIAGRAM_FILL: u8 = 17;
// Gray lines read thinner than white ones at 1 pixel.
const DIAGRAM_FRAME: u8 = 210;
const DIAGRAM_LINE: u8 = 160;

// The native strength wedge (ISO NR, Shading): one ramp along the bottom,
// rising to the right and split into `count` segments with 3-pixel gaps.
// Only the first `level` segments are drawn. Coverage is per sample.
fn wedge(level: usize, count: usize) -> impl Fn(f64, f64) -> bool {
    wedge_from(level, count, 4.0, 12.0)
}

// The wedge with heights from `low` at the left to `high` at the right.
fn wedge_from(level: usize, count: usize, low: f64, high: f64) -> impl Fn(f64, f64) -> bool {
    const LEFT: f64 = 2.0;
    const RIGHT: f64 = 37.0;
    const BOTTOM: f64 = 37.0;
    const GAP: f64 = 3.0;
    let segment = (RIGHT - LEFT - GAP * (count - 1) as f64) / count as f64;
    move |x, y| {
        let offset = x - LEFT;
        if !(0.0..RIGHT - LEFT).contains(&offset) {
            return false;
        }
        let index = (offset / (segment + GAP)) as usize;
        let inside = offset - index as f64 * (segment + GAP) < segment;
        let height = low + (high - low) * offset / (RIGHT - LEFT);
        index < level && inside && y >= BOTTOM - height && y < BOTTOM
    }
}

fn list_steps(pixels: &mut [u8], level: usize, count: usize) {
    paint(pixels, [2, 24, 37, 37], 255, wedge(level, count));
}

fn fill_rect(pixels: &mut [u8], [left, top, right, bottom]: [usize; 4], gray: u8) {
    for y in top..bottom {
        for x in left..right {
            pixels[(y * SIZE + x) * 4..(y * SIZE + x) * 4 + 3].fill(gray);
        }
    }
}

// A dark area with a 1-pixel light border: `[left, top, right, bottom)`.
fn frame(pixels: &mut [u8], [left, top, right, bottom]: [usize; 4]) {
    fill_rect(pixels, [left, top, right, bottom], DIAGRAM_FRAME);
    fill_rect(
        pixels,
        [left + 1, top + 1, right - 1, bottom - 1],
        DIAGRAM_FILL,
    );
}

// A photo of `ratio` (width / height), centered in the 30 x 16 area above the
// caption. The diagram and the caption together are centered on the tile.
fn ratio_tile(ratio: f64, label: &str) -> Result<Vec<u8>> {
    let mut pixels = list_tile();
    let (width, height) = if ratio >= 30.0 / 16.0 {
        (30, (30.0 / ratio).round() as usize)
    } else {
        ((16.0 * ratio).round() as usize, 16)
    };
    let left = 20 - width / 2;
    let top = 5 + (16 - height) / 2;
    frame(&mut pixels, [left, top, left + width, top + height]);
    draw_text(
        &mut pixels,
        label,
        CAPTION_SIZE,
        CAPTION_BASELINE,
        21.0,
        36.0,
    )?;
    Ok(pixels)
}

#[derive(Clone, Copy)]
enum Guide {
    Thirds,
    Center,
    Golden,
    Diagonals,
    ThirdsCenter,
    Spiral,
    Symmetry,
}

// The distance from (x, y) to the segment from a to b.
fn segment_distance((x, y): (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let t = (((x - a.0) * dx + (y - a.1) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
    (x - a.0 - t * dx).hypot(y - a.1 - t * dy)
}

// The golden spiral of guides.c, stretched to the frame `[x, y, w, h]`
// without a flip: it ends near the bottom-right corner. Returns the curve and
// the division lines.
type Point = (f64, f64);

fn spiral_points([x, y, w, h]: [f64; 4]) -> (Vec<Point>, Vec<[Point; 2]>) {
    const CUT: f64 = 0.618_034;
    let (mut l, mut t, mut r, mut b) = (x, y, x + w, y + h);
    let (mut points, mut divisions) = (Vec::new(), Vec::new());
    for k in 0..6 {
        divisions.push(match k % 4 {
            0 => {
                let c = l + (r - l) * CUT;
                [(c, t), (c, b)]
            }
            1 => {
                let c = t + (b - t) * CUT;
                [(l, c), (r, c)]
            }
            2 => {
                let c = r - (r - l) * CUT;
                [(c, t), (c, b)]
            }
            _ => {
                let c = b - (b - t) * CUT;
                [(l, c), (r, c)]
            }
        });
        let (cx, cy, rx, ry, start) = match k % 4 {
            0 => {
                let c = l + (r - l) * CUT;
                let arc = (c, b, c - l, b - t, 0.5);
                l = c;
                arc
            }
            1 => {
                let c = t + (b - t) * CUT;
                let arc = (l, c, r - l, c - t, 1.0);
                t = c;
                arc
            }
            2 => {
                let c = r - (r - l) * CUT;
                let arc = (c, t, r - c, b - t, 1.5);
                r = c;
                arc
            }
            _ => {
                let c = b - (b - t) * CUT;
                let arc = (r, c, r - l, b - c, 0.0);
                b = c;
                arc
            }
        };
        // A quarter turn from the angle `start` (in half turns), clockwise on screen.
        for i in 0..=16 {
            let angle = std::f64::consts::PI * (start + 0.5 + i as f64 / 32.0);
            points.push((cx + rx * angle.cos(), cy + ry * angle.sin()));
        }
    }
    (points, divisions)
}

// A guide style in the frame `[left, top, right, bottom)`: 1-pixel gray
// lines inside a 1-pixel border.
fn guides(pixels: &mut [u8], [left, top, right, bottom]: [usize; 4], guide: Guide) {
    frame(pixels, [left, top, right, bottom]);
    let (inner_left, inner_top) = (left + 1, top + 1);
    let (width, height) = (right - left - 2, bottom - top - 2);
    let lines = |pixels: &mut [u8], fractions: &[f64]| {
        for &f in fractions {
            let x = inner_left + (width as f64 * f).round() as usize;
            let y = inner_top + (height as f64 * f).round() as usize;
            fill_rect(
                pixels,
                [x, inner_top, x + 1, inner_top + height],
                DIAGRAM_LINE,
            );
            fill_rect(
                pixels,
                [inner_left, y, inner_left + width, y + 1],
                DIAGRAM_LINE,
            );
        }
    };
    let cross = |pixels: &mut [u8], arm: usize, thick: usize| {
        let (cx, cy) = (inner_left + width / 2, inner_top + height / 2);
        fill_rect(
            pixels,
            [cx - arm, cy - thick / 2, cx + arm, cy - thick / 2 + thick],
            DIAGRAM_LINE,
        );
        fill_rect(
            pixels,
            [cx - thick / 2, cy - arm, cx - thick / 2 + thick, cy + arm],
            DIAGRAM_LINE,
        );
    };
    match guide {
        Guide::Thirds => lines(pixels, &[1.0 / 3.0, 2.0 / 3.0]),
        Guide::Golden => lines(pixels, &[0.382, 0.618]),
        Guide::Center => cross(pixels, 4, 1),
        Guide::ThirdsCenter => {
            lines(pixels, &[1.0 / 3.0, 2.0 / 3.0]);
            cross(pixels, 2, 1);
        }
        Guide::Spiral => {
            let frame = [
                inner_left as f64,
                inner_top as f64,
                width as f64,
                height as f64,
            ];
            let (points, divisions) = spiral_points(frame);
            paint(
                pixels,
                [
                    inner_left,
                    inner_top,
                    inner_left + width,
                    inner_top + height,
                ],
                DIAGRAM_LINE,
                |x, y| {
                    points
                        .windows(2)
                        .chain(divisions.iter().map(|d| &d[..]))
                        .any(|p| segment_distance((x, y), p[0], p[1]) <= 0.5)
                },
            );
        }
        Guide::Symmetry => {
            // The diagonals and the reciprocals of dynamic symmetry, as guides.c.
            let (x0, y0, w, h) = (
                inner_left as f64,
                inner_top as f64,
                width as f64,
                height as f64,
            );
            let across = h * h / w;
            let segments = [
                ((x0, y0), (x0 + w, y0 + h)),
                ((x0 + w, y0), (x0, y0 + h)),
                ((x0, y0), (x0 + across, y0 + h)),
                ((x0 + w, y0), (x0 + w - across, y0 + h)),
                ((x0, y0 + h), (x0 + across, y0)),
                ((x0 + w, y0 + h), (x0 + w - across, y0)),
            ];
            paint(
                pixels,
                [
                    inner_left,
                    inner_top,
                    inner_left + width,
                    inner_top + height,
                ],
                DIAGRAM_LINE,
                |x, y| {
                    segments
                        .iter()
                        .any(|&(a, b)| segment_distance((x, y), a, b) <= 0.5)
                },
            );
        }
        Guide::Diagonals => {
            let (x0, y0) = (inner_left as f64, inner_top as f64);
            let (w, h) = (width as f64, height as f64);
            paint(
                pixels,
                [
                    inner_left,
                    inner_top,
                    inner_left + width,
                    inner_top + height,
                ],
                DIAGRAM_LINE,
                |x, y| {
                    let u = (x - x0) / w;
                    let v = (y - y0) / h;
                    ((u - v).abs() * h).min((u + v - 1.0).abs() * h) <= 0.6
                },
            );
        }
    }
}

fn guides_tile(guide: Guide, label: &str) -> Result<Vec<u8>> {
    let mut pixels = list_tile();
    guides(&mut pixels, [6, 5, 34, 21], guide);
    draw_text(
        &mut pixels,
        label,
        CAPTION_SIZE,
        CAPTION_BASELINE,
        21.0,
        36.0,
    )?;
    Ok(pixels)
}

// Viewfinder status icons: white marks with a one-pixel black outline on a
// transparent background, like the native status-row icons. Marks use binary
// coverage, so the outline stays crisp over the live view.
const VIEWFINDER_SYMBOL: (f64, f64) = (20.0, 13.5);

fn marks() -> Vec<u8> {
    vec![0; SIZE * SIZE]
}

fn mark(marks: &mut [u8], ink: u8, inside: impl Fn(f64, f64) -> bool) {
    for y in 2..38 {
        for x in 2..38 {
            let mut count = 0;
            for sy in 0..SAMPLES {
                for sx in 0..SAMPLES {
                    if inside(
                        x as f64 + (sx as f64 + 0.5) / SAMPLES as f64,
                        y as f64 + (sy as f64 + 0.5) / SAMPLES as f64,
                    ) {
                        count += 1;
                    }
                }
            }
            if count * 2 >= SAMPLES * SAMPLES {
                marks[y * SIZE + x] = ink;
            }
        }
    }
}

// In the viewfinder every segment shows: gray up to the full scale, then
// white up to the current level, so the strength reads at a glance.
const INACTIVE_STEP: u8 = 128;

// A lower wedge than in the list leaves room for a symbol of the size of the
// native status icons.
fn mark_steps(marks: &mut [u8], level: usize, count: usize) {
    mark(marks, INACTIVE_STEP, wedge_from(count, count, 3.0, 10.0));
    mark(marks, 255, wedge_from(level, count, 3.0, 10.0));
}

fn mark_rings(marks: &mut [u8], radii: &[f64], light: f64) {
    mark(marks, 255, |x, y| {
        let radius = (x - VIEWFINDER_SYMBOL.0).hypot(y - VIEWFINDER_SYMBOL.1);
        radius <= light || radii.iter().any(|r| radius <= *r && radius >= r - 2.0)
    });
}

// Sparse 2 x 2 grains whose outlines stay separate.
fn mark_grain(marks: &mut [u8]) {
    for (x, y) in [
        (8, 4),
        (16, 3),
        (25, 5),
        (32, 3),
        (12, 9),
        (21, 10),
        (29, 9),
        (7, 14),
        (16, 15),
        (25, 14),
        (32, 16),
        (11, 20),
        (20, 21),
        (28, 20),
    ] {
        for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
            marks[(y + dy) * SIZE + x + dx] = 255;
        }
    }
}

fn outlined(marks: &[u8]) -> Vec<u8> {
    let mut pixels = vec![0; SIZE * SIZE * 4];
    for y in 1..SIZE - 1 {
        for x in 1..SIZE - 1 {
            let ink = marks[y * SIZE + x];
            let offset = (y * SIZE + x) * 4;
            if ink != 0 {
                pixels[offset..offset + 4].copy_from_slice(&[ink, ink, ink, 255]);
            } else if (y - 1..=y + 1).any(|ny| (x - 1..=x + 1).any(|nx| marks[ny * SIZE + nx] != 0))
            {
                pixels[offset..offset + 4].copy_from_slice(&[0, 0, 0, 255]);
            }
        }
    }
    pixels
}

// Date Imprint formats: the order of year, month, and day, with HM below for
// the formats that print the time. The text block centers on the list frame.
// Capitals occupy rows 15..24 for one line, or 10..29 for two lines.
fn date(order: &str, with_time: bool) -> Result<Vec<u8>> {
    let mut pixels = list_tile();
    if with_time {
        draw_text(&mut pixels, order, 16.0, 20.0, 8.0, 20.0)?;
        draw_text(&mut pixels, "HM", 12.0, 30.0, 22.0, 30.0)?;
    } else {
        draw_text(&mut pixels, order, 16.0, 25.0, 13.0, 25.0)?;
    }
    Ok(pixels)
}

fn main() -> Result<()> {
    let modules = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../modules");
    for (bloom, files) in [
        (
            true,
            ["1-bloom-weak.png", "1-bloom.png", "3-bloom-strong.png"],
        ),
        (
            false,
            [
                "4-halation-weak.png",
                "2-halation.png",
                "6-halation-strong.png",
            ],
        ),
    ] {
        for (level, file) in files.into_iter().enumerate() {
            let mut pixels = glow(bloom);
            // Check the rasterized outer ring, not just the nominal ink. Sample
            // every five degrees and find the brightest pixel near the ring.
            for angle in 0..72 {
                let angle = angle as f64 * TAU / 72.0;
                let peak = (0..=16)
                    .map(|step| {
                        let radius = OUTER_RADIUS - RING_WIDTH - 0.5 + step as f64 / 8.0;
                        let x = (GLOW_CENTER.0 - 0.5 + radius * angle.cos()).round() as usize;
                        let y = (GLOW_CENTER.1 - 0.5 + radius * angle.sin()).round() as usize;
                        pixels[(y * SIZE + x) * 4]
                    })
                    .max()
                    .unwrap();
                ensure!(peak >= 176, "Dim contour in {file}: {peak}, angle {angle}");
            }
            list_steps(&mut pixels, level + 1, 3);
            save(&modules.join("glow/tiles").join(file), &pixels).context(file)?;
        }
    }
    save(&modules.join("glow/tiles/off-bloom.png"), &glow_off(true))?;
    save(
        &modules.join("glow/tiles/off-halation.png"),
        &glow_off(false),
    )?;
    save(&modules.join("grain/tiles/0-module.png"), &grain(28, 70))?;
    // One density for every level: the steps give the strength.
    for (level, file) in ["1-weak.png", "2-medium.png", "3-strong.png", "4-extra.png"]
        .into_iter()
        .enumerate()
    {
        let mut pixels = grain(16, 32);
        list_steps(&mut pixels, level + 1, 4);
        save(&modules.join("grain/tiles").join(file), &pixels)?;
    }
    // Off has no steps: the dots center in the frame, as the Glow rings do.
    let mut off = grain(16, 32);
    shift_down(&mut off, 6);
    artwork::strike(&mut off);
    save(&modules.join("grain/tiles/off.png"), &off)?;
    for (file, order, with_time) in [
        ("1-yy-m-d.png", "YMD", false),
        ("2-yy-m-d-time.png", "YMD", true),
        ("3-m-d-yy.png", "MDY", false),
        ("4-d-m-yy.png", "DMY", false),
        ("5-d-m-yy-time.png", "DMY", true),
    ] {
        save(
            &modules.join("date/tiles").join(file),
            &date(order, with_time)?,
        )?;
    }
    let mut root = list_tile();
    frame(&mut root, [7, 11, 33, 29]);
    save(&modules.join("framing/tiles/0-module.png"), &root)?;
    for (file, ratio, label) in [
        ("1-xpan.png", 65.0 / 24.0, "XPan"),
        ("2-16-9.png", 16.0 / 9.0, "16:9"),
        ("3-4-3.png", 4.0 / 3.0, "4:3"),
        ("4-2-35.png", 2.35, "2.35"),
        ("5-1-1.png", 1.0, "1:1"),
        ("6-5-4.png", 5.0 / 4.0, "5:4"),
        ("7-7-6.png", 7.0 / 6.0, "7:6"),
    ] {
        save(
            &modules.join("framing/tiles").join(file),
            &ratio_tile(ratio, label)?,
        )?;
    }
    let mut root = list_tile();
    guides(&mut root, [5, 8, 35, 32], Guide::Thirds);
    save(&modules.join("guides/tiles/0-module.png"), &root)?;
    for (file, guide, label) in [
        ("1-thirds.png", Guide::Thirds, "3x3"),
        ("2-center.png", Guide::Center, "Ctr"),
        ("3-golden.png", Guide::Golden, "Phi"),
        ("4-diagonals.png", Guide::Diagonals, "Diag"),
        ("5-thirds-center.png", Guide::ThirdsCenter, "3x3+"),
        ("6-spiral.png", Guide::Spiral, "Fib"),
        ("7-symmetry.png", Guide::Symmetry, "Dyn"),
    ] {
        save(
            &modules.join("guides/tiles").join(file),
            &guides_tile(guide, label)?,
        )?;
    }
    // Viewfinder status icons (module.toml `[[indicators]]`).
    for (level, file) in [
        "vf-1-weak.png",
        "vf-2-medium.png",
        "vf-3-strong.png",
        "vf-4-extra.png",
    ]
    .into_iter()
    .enumerate()
    {
        let mut m = marks();
        mark_grain(&mut m);
        mark_steps(&mut m, level + 1, 4);
        save(&modules.join("grain/tiles").join(file), &outlined(&m))?;
    }
    for (prefix, radii, light) in [
        ("vf-bloom", &[10.5, 6.0][..], BLOOM_LIGHT),
        ("vf-halation", &[10.5][..], HALATION_LIGHT),
    ] {
        for (level, strength) in ["weak", "medium", "strong"].into_iter().enumerate() {
            let mut m = marks();
            mark_rings(&mut m, radii, light);
            mark_steps(&mut m, level + 1, 3);
            save(
                &modules
                    .join("glow/tiles")
                    .join(format!("{prefix}-{strength}.png")),
                &outlined(&m),
            )?;
        }
    }
    Ok(())
}
