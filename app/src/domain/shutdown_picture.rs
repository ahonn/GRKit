//! The picture of Shutdown Screen from a photo of the phone: the photo
//! stands upright, the person picks a part of it at the ratio of the
//! camera's screen, and `grkit_core::goodbye` writes the file that the
//! camera shows.

use grkit_core::goodbye::{HEIGHT, WIDTH, encode};
use zune_jpeg::JpegDecoder;
use zune_jpeg::zune_core::{bytestream::ZCursor, colorspace::ColorSpace, options::DecoderOptions};

/// The longest side of a photo that the phone keeps: the picker scales
/// larger photos down to it, and `Photo::decode` the others.
pub const MAX_SIDE: usize = 2048;
/// The most pixels of a photo that the phone decodes.
const MAX_PIXELS: usize = 64_000_000;

/// Why a picked file gives no photo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhotoError {
    /// Not a JPEG or a PNG.
    Format,
    /// A JPEG or PNG that does not decode, or is too large.
    Unreadable,
}

/// An upright photo, at most `MAX_SIDE` pixels on its longer side.
pub struct Photo {
    pub width: usize,
    pub height: usize,
    rgb: Vec<u8>,
}

/// A part of a photo at the ratio of the screen, in fractions of the width
/// and the height of the photo.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Crop {
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
}

impl Crop {
    /// The middle of a photo of `width` by `height` pixels.
    pub fn middle(width: usize, height: usize) -> Crop {
        let (crop_w, crop_h) = if width * HEIGHT > height * WIDTH {
            ((height * WIDTH / HEIGHT) as f32 / width as f32, 1.0)
        } else {
            (1.0, (width * HEIGHT / WIDTH) as f32 / height as f32)
        };
        Crop {
            left: (1.0 - crop_w) / 2.0,
            top: (1.0 - crop_h) / 2.0,
            width: crop_w,
            height: crop_h,
        }
    }

    /// The part that a zooming scroll view of the photo shows: its offset,
    /// the size of its zoomed content, and its own size, in points. A
    /// bounce past an edge does not move the part out of the photo.
    pub fn shown(offset: (f32, f32), content: (f32, f32), view: (f32, f32)) -> Option<Crop> {
        if content.0 <= 0.0 || content.1 <= 0.0 {
            return None;
        }
        let width = (view.0 / content.0).clamp(0.0, 1.0);
        let height = (view.1 / content.1).clamp(0.0, 1.0);
        Some(Crop {
            left: (offset.0 / content.0).clamp(0.0, 1.0 - width),
            top: (offset.1 / content.1).clamp(0.0, 1.0 - height),
            width,
            height,
        })
    }
}

impl Photo {
    /// The photo of a picked JPEG or PNG, turned as its EXIF orientation
    /// says.
    pub fn decode(bytes: &[u8]) -> Result<Photo, PhotoError> {
        let (width, height, rgb, orientation) = if bytes.starts_with(&[0xFF, 0xD8]) {
            decode_jpeg(bytes)?
        } else if bytes.starts_with(b"\x89PNG") {
            let (width, height, rgba) =
                grkit_core::png::decode_rgba8(bytes).map_err(|_| PhotoError::Unreadable)?;
            let rgb = rgba
                .as_chunks::<4>()
                .0
                .iter()
                .flat_map(|p| [p[0], p[1], p[2]])
                .collect();
            (width, height, rgb, 1)
        } else {
            return Err(PhotoError::Format);
        };
        // The JPEG encoder takes sizes of whole blocks of 16 pixels: the
        // edges beyond the last whole block go.
        let scale = (width.max(height) as f32 / MAX_SIDE as f32).max(1.0);
        let block = |side: usize| ((side as f32 / scale) as usize) / 16 * 16;
        let (out_w, out_h) = (block(width), block(height));
        if out_w == 0 || out_h == 0 {
            return Err(PhotoError::Unreadable);
        }
        let source =
            |out: usize, side: usize| ((out as f32 * scale).round() as usize).clamp(1, side);
        let (source_w, source_h) = (source(out_w, width), source(out_h, height));
        let rect = (
            (width - source_w) / 2,
            (height - source_h) / 2,
            source_w,
            source_h,
        );
        let small = resample(&rgb, width, rect, (out_w, out_h));
        Ok(turn(small, out_w, out_h, orientation))
    }

    /// The photo as a JPEG, for the phone to show.
    pub fn preview(&self) -> Vec<u8> {
        grkit_core::jpeg::encode(&self.rgb, self.width, self.height, 85).unwrap_or_default()
    }

    /// The file for the camera from `crop` of the photo.
    pub fn picture(&self, crop: Crop) -> Result<Vec<u8>, String> {
        let px = |fraction: f32, size: usize| ((fraction * size as f32).round() as usize).min(size);
        let (left, top) = (px(crop.left, self.width), px(crop.top, self.height));
        let rect = (
            left.min(self.width - 1),
            top.min(self.height - 1),
            px(crop.width, self.width).clamp(1, self.width - left.min(self.width - 1)),
            px(crop.height, self.height).clamp(1, self.height - top.min(self.height - 1)),
        );
        encode(&resample(&self.rgb, self.width, rect, (WIDTH, HEIGHT))).map_err(|e| e.to_string())
    }
}

/// The RGB pixels of a JPEG and its EXIF orientation.
fn decode_jpeg(bytes: &[u8]) -> Result<(usize, usize, Vec<u8>, u16), PhotoError> {
    let options = DecoderOptions::default().jpeg_set_out_colorspace(ColorSpace::RGB);
    let mut decoder = JpegDecoder::new_with_options(ZCursor::new(bytes), options);
    decoder
        .decode_headers()
        .map_err(|_| PhotoError::Unreadable)?;
    let info = decoder.info().ok_or(PhotoError::Unreadable)?;
    let (width, height) = (usize::from(info.width), usize::from(info.height));
    if width * height > MAX_PIXELS {
        return Err(PhotoError::Unreadable);
    }
    let orientation = decoder.exif().map_or(1, |exif| orientation(exif));
    let rgb = decoder.decode().map_err(|_| PhotoError::Unreadable)?;
    Ok((width, height, rgb, orientation))
}

/// The Orientation tag (0x0112) of IFD0 of EXIF data that starts at its TIFF
/// header, or 1 (upright) without one.
fn orientation(exif: &[u8]) -> u16 {
    let tiff = exif.strip_prefix(b"Exif\0\0").unwrap_or(exif);
    let little = match tiff.get(..2) {
        Some(b"II") => true,
        Some(b"MM") => false,
        _ => return 1,
    };
    let u16_at = |at: usize| {
        let b: [u8; 2] = tiff.get(at..at + 2)?.try_into().ok()?;
        Some(if little {
            u16::from_le_bytes(b)
        } else {
            u16::from_be_bytes(b)
        })
    };
    let u32_at = |at: usize| {
        let b: [u8; 4] = tiff.get(at..at + 4)?.try_into().ok()?;
        Some(if little {
            u32::from_le_bytes(b)
        } else {
            u32::from_be_bytes(b)
        })
    };
    let found = (|| {
        let ifd = u32_at(4)? as usize;
        (0..usize::from(u16_at(ifd)?))
            .map(|i| ifd + 2 + 12 * i)
            .find(|&entry| u16_at(entry) == Some(0x0112))
            .and_then(|entry| u16_at(entry + 8))
    })();
    found.filter(|o| (1..=8).contains(o)).unwrap_or(1)
}

/// The photo upright: EXIF orientations 2 to 8 mirror or turn the stored
/// pixels.
fn turn(rgb: Vec<u8>, width: usize, height: usize, orientation: u16) -> Photo {
    if orientation <= 1 {
        return Photo { width, height, rgb };
    }
    let turned = orientation >= 5;
    let (out_w, out_h) = if turned {
        (height, width)
    } else {
        (width, height)
    };
    let mut out = Vec::with_capacity(rgb.len());
    for y in 0..out_h {
        for x in 0..out_w {
            let (sx, sy) = match orientation {
                2 => (width - 1 - x, y),
                3 => (width - 1 - x, height - 1 - y),
                4 => (x, height - 1 - y),
                5 => (y, x),
                6 => (y, height - 1 - x),
                7 => (width - 1 - y, height - 1 - x),
                _ => (width - 1 - y, x),
            };
            let at = (sy * width + sx) * 3;
            out.extend_from_slice(&rgb[at..at + 3]);
        }
    }
    Photo {
        width: out_w,
        height: out_h,
        rgb: out,
    }
}

/// `rect` (left, top, width, height) of a picture `width` pixels wide,
/// scaled to `size`: each pixel is the mean of the pixels that it covers.
fn resample(
    rgb: &[u8],
    width: usize,
    rect: (usize, usize, usize, usize),
    size: (usize, usize),
) -> Vec<u8> {
    let (left, top, crop_w, crop_h) = rect;
    let (out_w, out_h) = size;
    let mut out = Vec::with_capacity(out_w * out_h * 3);
    for y in 0..out_h {
        let y0 = top + y * crop_h / out_h;
        let y1 = (top + (y + 1) * crop_h / out_h).max(y0 + 1);
        for x in 0..out_w {
            let x0 = left + x * crop_w / out_w;
            let x1 = (left + (x + 1) * crop_w / out_w).max(x0 + 1);
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

    /// A JPEG of `width` by `height` with a red left half and a blue right
    /// half, and an EXIF orientation when given.
    fn photo(width: usize, height: usize, orientation: Option<u16>) -> Vec<u8> {
        let rgb: Vec<u8> = (0..width * height)
            .flat_map(|i| {
                if i % width < width / 2 {
                    [220, 20, 20]
                } else {
                    [20, 20, 220]
                }
            })
            .collect();
        let jpeg = grkit_core::jpeg::encode(&rgb, width, height, 90).unwrap();
        let Some(orientation) = orientation else {
            return jpeg;
        };
        // An APP1 segment after SOI: "Exif", a little-endian TIFF header,
        // and IFD0 with one Orientation entry.
        let mut tiff = b"II*\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0".to_vec();
        tiff.extend(orientation.to_le_bytes());
        tiff.extend([0; 6]);
        let mut app1 = b"Exif\0\0".to_vec();
        app1.extend(tiff);
        let mut out = jpeg[..2].to_vec();
        out.extend([0xFF, 0xE1]);
        out.extend(((app1.len() + 2) as u16).to_be_bytes());
        out.extend(app1);
        out.extend(&jpeg[2..]);
        out
    }

    #[test]
    fn a_turned_photo_stands_upright() {
        // Orientation 6: the camera was turned to the right, so the photo
        // turns a quarter to the right and its red half goes to the top.
        let turned = Photo::decode(&photo(320, 192, Some(6))).unwrap();
        assert_eq!((turned.width, turned.height), (192, 320));
        let pixel = |p: &Photo, x: usize, y: usize| p.rgb[(y * p.width + x) * 3];
        assert!(pixel(&turned, 96, 20) > 150 && pixel(&turned, 96, 300) < 100);
        assert_eq!(
            Photo::decode(b"RIFF....WEBP").err(),
            Some(PhotoError::Format)
        );
        assert_eq!(
            Photo::decode(&[0xFF, 0xD8, 0]).err(),
            Some(PhotoError::Unreadable)
        );
    }

    #[test]
    fn the_shown_part_of_the_photo_becomes_the_picture() {
        let photo = Photo::decode(&photo(640, 192, None)).unwrap();
        // The middle 288 x 192 of the photo.
        let middle = Crop::middle(640, 192);
        assert!((middle.left - 0.275).abs() < 1e-6 && (middle.width - 0.45).abs() < 1e-6);
        // A view of 150 x 100 points on the photo at 300 x 100, scrolled to
        // its left end and past it: the red left half.
        let left = Crop::shown((-12.0, 0.0), (300.0, 100.0), (150.0, 100.0)).unwrap();
        assert_eq!(
            left,
            Crop {
                left: 0.0,
                top: 0.0,
                width: 0.5,
                height: 1.0
            }
        );
        let picture = photo.picture(left).unwrap();
        assert!(grkit_core::goodbye::usable(&picture));
        let shown = Photo::decode(&picture).unwrap();
        assert!(shown.rgb[(240 * WIDTH + 700) * 3] > 150);
    }
}
