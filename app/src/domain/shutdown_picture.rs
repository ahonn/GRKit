//! The picture of Shutdown Screen from a photo of the phone: the middle of
//! the photo at 3:2, as `grkit_core::goodbye` writes the file that the camera
//! shows.

use grkit_core::goodbye::{HEIGHT, WIDTH, encode};
use zune_jpeg::JpegDecoder;
use zune_jpeg::zune_core::{bytestream::ZCursor, colorspace::ColorSpace, options::DecoderOptions};

/// The most pixels of a photo that the phone decodes.
const MAX_PIXELS: usize = 64_000_000;

/// The file for the camera from a picked JPEG or PNG.
pub fn shutdown_picture(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let (width, height, rgb) = decode(bytes)?;
    encode(&cover(&rgb, width, height)).map_err(|e| e.to_string())
}

/// The RGB pixels of a JPEG or PNG.
fn decode(bytes: &[u8]) -> Result<(usize, usize, Vec<u8>), String> {
    if bytes.starts_with(&[0xFF, 0xD8]) {
        let options = DecoderOptions::default().jpeg_set_out_colorspace(ColorSpace::RGB);
        let mut decoder = JpegDecoder::new_with_options(ZCursor::new(bytes), options);
        decoder.decode_headers().map_err(|e| e.to_string())?;
        let info = decoder.info().ok_or("The photo has no size")?;
        let (width, height) = (usize::from(info.width), usize::from(info.height));
        if width * height > MAX_PIXELS {
            return Err("The photo is too large".into());
        }
        let rgb = decoder.decode().map_err(|e| e.to_string())?;
        return Ok((width, height, rgb));
    }
    let (width, height, rgba) = grkit_core::png::decode_rgba8(bytes).map_err(|e| e.to_string())?;
    let rgb = rgba
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|p| [p[0], p[1], p[2]])
        .collect();
    Ok((width, height, rgb))
}

/// The middle of the picture at the ratio of the screen, scaled to its size:
/// each pixel is the mean of the pixels of the photo that it covers.
fn cover(rgb: &[u8], width: usize, height: usize) -> Vec<u8> {
    let (crop_w, crop_h) = if width * HEIGHT > height * WIDTH {
        (height * WIDTH / HEIGHT, height)
    } else {
        (width, width * HEIGHT / WIDTH)
    };
    let (left, top) = ((width - crop_w) / 2, (height - crop_h) / 2);
    let mut out = Vec::with_capacity(WIDTH * HEIGHT * 3);
    for y in 0..HEIGHT {
        let y0 = top + y * crop_h / HEIGHT;
        let y1 = (top + (y + 1) * crop_h / HEIGHT).max(y0 + 1);
        for x in 0..WIDTH {
            let x0 = left + x * crop_w / WIDTH;
            let x1 = (left + (x + 1) * crop_w / WIDTH).max(x0 + 1);
            let mut sum = [0u64; 3];
            for sy in y0..y1 {
                for sx in x0..x1 {
                    let at = (sy * width + sx) * 3;
                    for (channel, total) in sum.iter_mut().enumerate() {
                        *total += u64::from(rgb[at + channel]);
                    }
                }
            }
            let count = ((y1 - y0) * (x1 - x0)) as u64;
            out.extend(sum.map(|total| (total / count) as u8));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wide_photo_becomes_a_usable_picture_of_its_middle() {
        // A 1504 x 608 photo: a red middle between blue sides.
        let (width, height) = (1504, 608);
        let rgb: Vec<u8> = (0..width * height)
            .flat_map(|i| {
                let x = i % width;
                if (290..1214).contains(&x) {
                    [220, 20, 20]
                } else {
                    [20, 20, 220]
                }
            })
            .collect();
        let photo = grkit_core::jpeg::encode(&rgb, width, height, 90).unwrap();
        let picture = shutdown_picture(&photo).unwrap();
        assert!(grkit_core::goodbye::usable(&picture));
        // The middle 912 x 608 fills the picture: near its edge it is red.
        let (_, _, pixels) = decode(&picture).unwrap();
        assert!(pixels[30] > 150 && pixels[32] < 100);
        assert!(shutdown_picture(b"not a photo").is_err());
    }
}
