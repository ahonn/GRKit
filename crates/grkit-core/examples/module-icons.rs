//! Reproduce Glow, Grain, and Date icons and render module captions.
//! Run from the repository root with `cargo run -p grkit-core --example module-icons`.
use ab_glyph::{point, Font, FontRef, PxScale, ScaleFont};
use anyhow::{ensure, Context, Result};
use grkit_core::{artwork, png};
use std::{f64::consts::TAU, fs, path::Path};

const SIZE: usize = 40;
const BACKGROUND: u8 = 51;
const OUTER_RADIUS: f64 = 14.0;
const LIGHT_RADIUS: f64 = 4.0;
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

fn glow(bloom: bool, width: f64, ink: u8, optical_inset: f64) -> Vec<u8> {
    let mut pixels = list_tile();
    let radii: &[f64] = if bloom {
        &[
            OUTER_RADIUS - optical_inset,
            10.5 - optical_inset / 2.0,
            7.0,
        ]
    } else {
        &[OUTER_RADIUS]
    };
    for y in 6..34 {
        for x in 6..34 {
            let mut total = 0u32;
            for sy in 0..SAMPLES {
                for sx in 0..SAMPLES {
                    let dx = x as f64 + (sx as f64 + 0.5) / SAMPLES as f64 - 20.0;
                    let dy = y as f64 + (sy as f64 + 0.5) / SAMPLES as f64 - 20.0;
                    let radius = dx.hypot(dy);
                    let gray = if radius <= LIGHT_RADIUS {
                        255
                    } else if radii.iter().any(|r| radius <= *r && radius >= r - width) {
                        ink
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

// Preserve each diagram and frame. Caption placement uses a font baseline,
// never the first non-background pixel, which can be a faint antialiased edge.
fn caption(path: &Path, label: &str) -> Result<()> {
    let (width, height, mut pixels) = png::decode_rgba8(&fs::read(path)?)?;
    ensure!((width, height) == (SIZE, SIZE), "Invalid tile size");
    for y in 24..37 {
        for x in 2..38 {
            let offset = (y * SIZE + x) * 4;
            pixels[offset..offset + 4].copy_from_slice(&[BACKGROUND, BACKGROUND, BACKGROUND, 255]);
        }
    }
    draw_text(&mut pixels, label, 12.0, 32.0, 24.0, 35.0)?;
    save(path, &pixels)
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

fn date(order: &str, with_time: bool) -> Result<Vec<u8>> {
    let mut pixels = list_tile();
    // Center the whole text block on the list frame, not each line separately.
    // Capitals occupy rows 15..24 for one line, or 10..29 for two lines.
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
    // Dense concentric rings appear larger even at equal diameters. Bloom uses
    // a small optical inset and restrained stroke growth to balance the family.
    // The center light and the contrast of the weak contour remain unchanged.
    for (bloom, files, widths, insets) in [
        (
            true,
            ["1-bloom-weak.png", "1-bloom.png", "3-bloom-strong.png"],
            [1.5, 1.8, 2.1],
            [0.0, 0.5, 1.0],
        ),
        (
            false,
            [
                "4-halation-weak.png",
                "2-halation.png",
                "6-halation-strong.png",
            ],
            [1.5, 2.25, 3.0],
            [0.0; 3],
        ),
    ] {
        for (((file, width), ink), inset) in files
            .into_iter()
            .zip(widths)
            .zip([224, 240, 255])
            .zip(insets)
        {
            let pixels = glow(bloom, width, ink, inset);
            // Check the rasterized contour, not just the nominal ink. Sample
            // every five degrees and find the brightest pixel near the ring.
            for angle in 0..72 {
                let angle = angle as f64 * TAU / 72.0;
                let peak = (0..=24)
                    .map(|step| {
                        let radius = 11.5 + step as f64 / 8.0;
                        let x = (19.5 + radius * angle.cos()).round() as usize;
                        let y = (19.5 + radius * angle.sin()).round() as usize;
                        pixels[(y * SIZE + x) * 4]
                    })
                    .max()
                    .unwrap();
                ensure!(peak >= 176, "Dim contour in {file}: {peak}, angle {angle}");
            }
            save(&modules.join("glow/tiles").join(file), &pixels).context(file)?;
        }
    }
    save(&modules.join("grain/tiles/0-module.png"), &grain(28, 70))?;
    for (file, label, count) in [
        ("1-weak.png", "Weak", 18),
        ("2-medium.png", "Med", 32),
        ("3-strong.png", "Strong", 46),
        ("4-extra.png", "Extra", 60),
    ] {
        let mut pixels = grain(18, count);
        draw_text(&mut pixels, label, 12.0, 32.0, 24.0, 35.0)?;
        save(&modules.join("grain/tiles").join(file), &pixels)?;
    }
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
    for (family, labels) in [
        (
            "framing",
            &[
                ("1-xpan.png", "XPan"),
                ("2-16-9.png", "16:9"),
                ("3-4-3.png", "4:3"),
                ("4-2-35.png", "2.35"),
                ("5-1-1.png", "1:1"),
                ("6-5-4.png", "5:4"),
                ("7-7-6.png", "7:6"),
            ][..],
        ),
        (
            "guides",
            &[
                ("1-thirds.png", "3x3"),
                ("2-center.png", "Ctr"),
                ("3-golden.png", "Phi"),
                ("4-diagonals.png", "Diag"),
                ("5-thirds-center.png", "3x3+"),
            ][..],
        ),
    ] {
        for (file, label) in labels {
            caption(&modules.join(family).join("tiles").join(file), label)?;
        }
    }
    Ok(())
}
