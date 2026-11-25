//! A Look on the sample photo, as the site shows it: the photo in linear
//! light (GRLIN), and the Look's preview table from the site, applied with
//! trilinear interpolation over an sRGB-shaped grid.

/// The sample photo: "Sakiko buys a delicious soup at Kitchen Grizzly" by
/// MIKI Yoshihito, CC BY 2.0, shot on a RICOH GR IIIx. The site's sample
/// (`web/public/samples/kitchen-grizzly.grlinear`) at half size, each pixel
/// the mean of four in linear light.
pub const SAMPLE: &[u8] = include_bytes!("../../data/look-sample.grlin");

/// The credit of the sample, in Settings under its localized "Sample Photo".
pub const SAMPLE_CREDIT: &str = "MIKI Yoshihito · CC BY 2.0 · RICOH GR IIIx";
/// The sample's page, which gives its license.
pub const SAMPLE_SOURCE: &str = "https://www.flickr.com/photos/mujitra/53311001084/";

const MAGIC: &[u8; 8] = b"GRLIN001";

/// A photo in linear light: 16-bit RGB, rows from the top.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinearPicture {
    pub width: usize,
    pub height: usize,
    pub data: Vec<u16>,
}

/// GRLIN001: eight magic bytes, LE u32 width and height, then LE u16 RGB.
pub fn decode_linear(bytes: &[u8]) -> Result<LinearPicture, String> {
    if bytes.len() < 16 || &bytes[..8] != MAGIC {
        return Err("Invalid linear sample header".into());
    }
    let word = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
    let (width, height) = (word(8), word(12));
    if width == 0 || height == 0 || bytes.len() != 16 + width * height * 6 {
        return Err("Invalid linear sample dimensions".into());
    }
    let data = bytes[16..]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u16::from_le_bytes(*pair))
        .collect();
    Ok(LinearPicture {
        width,
        height,
        data,
    })
}

/// The points on each axis of a preview table of `len` bytes, or `None`
/// when it is no cube of RGB triples.
pub fn table_size(len: usize) -> Option<usize> {
    (2..=256).find(|size| size * size * size * 3 == len)
}

/// The table grid uses the sRGB transfer only as a shaper, which keeps
/// precision in the shadows.
fn srgb_coordinate(linear: f32) -> f32 {
    if linear <= 0.003_130_8 {
        linear * 12.92
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    }
}

/// `picture` through `table` (red fastest, `size` points on each axis) as
/// 8-bit RGBA, as the site's `applyLinearLut`.
pub fn apply_linear_lut(picture: &LinearPicture, table: &[u8]) -> Result<Vec<u8>, String> {
    let size = table_size(table.len()).ok_or("Preview table size mismatch")?;
    // Per input level: the lower grid index and the weight of the upper.
    let mut low = vec![0usize; 65536];
    let mut weight = vec![0f32; 65536];
    for v in 0..65536 {
        let x = srgb_coordinate(v as f32 / 65535.0) * (size - 1) as f32;
        let i = (x.floor() as usize).min(size - 2);
        low[v] = i;
        weight[v] = x - i as f32;
    }
    let stride = [3, size * 3, size * size * 3];
    let at = |i: usize| f32::from(table[i]);
    let mut out = Vec::with_capacity(picture.width * picture.height * 4);
    for pixel in picture.data.as_chunks::<3>().0 {
        let (r, g, b) = (
            usize::from(pixel[0]),
            usize::from(pixel[1]),
            usize::from(pixel[2]),
        );
        let (fr, fg, fb) = (weight[r], weight[g], weight[b]);
        let base = low[r] * stride[0] + low[g] * stride[1] + low[b] * stride[2];
        for c in 0..3 {
            let o = base + c;
            let c00 = at(o) + (at(o + stride[0]) - at(o)) * fr;
            let c10 = at(o + stride[1]) + (at(o + stride[1] + stride[0]) - at(o + stride[1])) * fr;
            let c01 = at(o + stride[2]) + (at(o + stride[2] + stride[0]) - at(o + stride[2])) * fr;
            let c11 = at(o + stride[2] + stride[1])
                + (at(o + stride[2] + stride[1] + stride[0]) - at(o + stride[2] + stride[1])) * fr;
            let c0 = c00 + (c10 - c00) * fg;
            let c1 = c01 + (c11 - c01) * fg;
            out.push((c0 + (c1 - c0) * fb).round().clamp(0.0, 255.0) as u8);
        }
        out.push(255);
    }
    Ok(out)
}

/// The sample photo through `table`, as a PNG.
pub fn sample_preview_png(table: &[u8]) -> Result<Vec<u8>, String> {
    let picture = decode_linear(SAMPLE)?;
    let rgba = apply_linear_lut(&picture, table)?;
    grkit_core::png::encode_rgba8(picture.width, picture.height, &rgba).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A table whose output is its input: each grid point holds its own
    /// coordinates, scaled to 0 ..= 255.
    fn identity(size: usize) -> Vec<u8> {
        let mut table = Vec::with_capacity(size * size * size * 3);
        let level = |i: usize| (i as f32 * 255.0 / (size - 1) as f32).round() as u8;
        for b in 0..size {
            for g in 0..size {
                for r in 0..size {
                    table.extend([level(r), level(g), level(b)]);
                }
            }
        }
        table
    }

    #[test]
    fn the_sample_decodes_and_an_identity_table_encodes_srgb() {
        let picture = decode_linear(SAMPLE).unwrap();
        assert_eq!((picture.width, picture.height), (600, 400));
        let rgba = apply_linear_lut(&picture, &identity(33)).unwrap();
        assert_eq!(rgba.len(), 600 * 400 * 4);
        for (pixel, linear) in rgba
            .as_chunks::<4>()
            .0
            .iter()
            .zip(picture.data.as_chunks::<3>().0)
            .step_by(997)
        {
            for c in 0..3 {
                let expected = srgb_coordinate(f32::from(linear[c]) / 65535.0) * 255.0;
                assert!(
                    (f32::from(pixel[c]) - expected).abs() <= 1.0,
                    "{pixel:?} {linear:?}"
                );
            }
            assert_eq!(pixel[3], 255);
        }
    }

    #[test]
    fn a_table_that_is_no_cube_or_a_bad_header_fails() {
        assert_eq!(table_size(33 * 33 * 33 * 3), Some(33));
        assert_eq!(table_size(1000), None);
        assert!(decode_linear(b"GRLIN001\x01\0\0\0\x01\0\0\0").is_err());
        assert!(decode_linear(b"nope").is_err());
    }
}
