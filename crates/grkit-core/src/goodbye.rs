//! The picture of Shutdown Screen: `GRKIT/GOODBYE.JPG` on the card,
//! which the camera shows when it turns off, in place of the picture of the
//! firmware (`runtime/goodbye.h`). The firmware decodes the file as its own
//! picture, so the file has the frame of those pictures: a baseline JPEG of
//! 720 by 480 pixels with 4:2:0 chroma.

use crate::jpeg;
use anyhow::{bail, ensure, Result};

/// The size of the picture, which the screen shows whole.
pub const WIDTH: usize = 720;
pub const HEIGHT: usize = 480;
/// The most bytes of the file: the buffer that the firmware reads it into.
pub const MAX_BYTES: usize = WIDTH * HEIGHT * 2;
/// The file on the card, in the data folder.
pub const FILE: &str = concat!("GRKIT", "/GOODBYE.JPG");
/// The qualities that `encode` tries, best first.
const QUALITIES: [u8; 4] = [92, 85, 75, 60];

/// Encode `rgb`, the 8-bit RGB pixels of a picture of `WIDTH` by `HEIGHT` in
/// row order, as the file: at the best quality that fits `MAX_BYTES`.
pub fn encode(rgb: &[u8]) -> Result<Vec<u8>> {
    ensure!(
        rgb.len() == WIDTH * HEIGHT * 3,
        "The shutdown picture must be {WIDTH} by {HEIGHT} pixels"
    );
    for quality in QUALITIES {
        let bytes = jpeg::encode(rgb, WIDTH, HEIGHT, quality)?;
        if bytes.len() <= MAX_BYTES {
            return Ok(bytes);
        }
    }
    bail!("The shutdown picture does not fit the file")
}

/// Whether the camera shows `bytes` as the file. A file that it does not
/// show changes nothing: the camera shows the picture of the firmware.
///
/// The camera checks the whole file as its decoder reads it, since a
/// decoder that meets data that ends early or does not decode does not end
/// and the camera does not turn off (`runtime/goodbye.h`: goodbye_picture).
/// This is the same check.
pub fn usable(bytes: &[u8]) -> bool {
    !bytes.is_empty() && bytes.len() <= MAX_BYTES && Scan::new(bytes).picture().is_some()
}

/// The reader of `usable`, in the steps of the check of the camera.
struct Scan<'a> {
    bytes: &'a [u8],
    at: usize,
    bits: u32,
    count: u32,
    /// For DC 0, DC 1, AC 0, AC 1: the number of codes of each length and
    /// the values in code order.
    tables: [Option<([u8; 17], Vec<u8>)>; 4],
    /// The ID and the quantization table of each component of the frame.
    frame: Option<[(u8, u8); 3]>,
    /// The quantization tables that a DQT segment defined.
    quantized: [bool; 4],
}

impl<'a> Scan<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            at: 0,
            bits: 0,
            count: 0,
            tables: Default::default(),
            frame: None,
            quantized: [false; 4],
        }
    }
    fn byte(&mut self) -> Option<u8> {
        let byte = *self.bytes.get(self.at)?;
        self.at += 1;
        Some(byte)
    }
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let bytes = self.bytes.get(self.at..self.at + n)?;
        self.at += n;
        Some(bytes)
    }
    fn picture(&mut self) -> Option<()> {
        (self.take(2)? == [0xff, 0xd8]).then_some(())?;
        loop {
            (self.byte()? == 0xff).then_some(())?;
            let mut marker = self.byte()?;
            while marker == 0xff {
                marker = self.byte()?;
            }
            let length = u16::from_be_bytes([self.byte()?, self.byte()?]) as usize;
            let body = self.take(length.checked_sub(2)?)?;
            match marker {
                0xda => return self.scan(body),
                0xc0 if self.frame.is_none() => self.frame = Some(frame(body)?),
                0xc4 => self.huffman(body)?,
                0xdb => {
                    (body.len() % 65 == 0
                        && body.chunks(65).all(|t| t[0] >> 4 == 0 && t[0] & 15 <= 3))
                    .then_some(())?;
                    for table in body.chunks(65) {
                        self.quantized[(table[0] & 15) as usize] = true;
                    }
                }
                // No comment segment: pictures padded with comments have
                // stopped the power-off of a camera.
                0xe0..=0xef => {}
                _ => return None,
            }
        }
    }
    fn huffman(&mut self, mut body: &[u8]) -> Option<()> {
        while !body.is_empty() {
            let (class, id) = (body[0] >> 4, body[0] & 15);
            (class <= 1 && id <= 1).then_some(())?;
            let mut codes = [0u8; 17];
            codes[1..].copy_from_slice(body.get(1..17)?);
            let total: usize = codes.iter().map(|&c| c as usize).sum();
            let values = body.get(17..17 + total)?.to_vec();
            // Baseline tables: a DC table codes sizes 0 to 11, an AC table
            // runs and sizes 1 to 10, with EOB and ZRL.
            let fits = if class == 0 {
                total <= 12 && values.iter().all(|&v| v <= 11)
            } else {
                total <= 162 && values.iter().all(|&v| v & 15 <= 10)
            };
            fits.then_some(())?;
            // Each length holds the codes that it can have but the one of
            // all ones.
            let mut code = 0u32;
            for (length, &count) in codes.iter().enumerate().skip(1) {
                code += count as u32;
                (code < 1 << length).then_some(())?;
                code <<= 1;
            }
            self.tables[(class * 2 + id) as usize] = Some((codes, values));
            body = &body[17 + total..];
        }
        Some(())
    }
    fn bit(&mut self) -> Option<u32> {
        if self.count == 0 {
            let byte = self.byte()?;
            if byte == 0xff {
                (self.byte()? == 0).then_some(())?;
            }
            self.bits = byte as u32;
            self.count = 8;
        }
        self.count -= 1;
        Some(self.bits >> self.count & 1)
    }
    fn symbol(&mut self, table: &([u8; 17], Vec<u8>)) -> Option<u8> {
        let (codes, values) = table;
        let (mut code, mut first, mut index) = (0u32, 0u32, 0usize);
        for &count in &codes[1..] {
            code = code << 1 | self.bit()?;
            if code - first < count as u32 && code >= first {
                return values.get(index + (code - first) as usize).copied();
            }
            first = (first + count as u32) << 1;
            index += count as usize;
        }
        None
    }
    fn block(&mut self, dc: &([u8; 17], Vec<u8>), ac: &([u8; 17], Vec<u8>)) -> Option<()> {
        let size = self.symbol(dc)?;
        (size <= 11).then_some(())?;
        for _ in 0..size {
            self.bit()?;
        }
        let mut k = 1;
        while k < 64 {
            let value = self.symbol(ac)?;
            match value {
                0 => return Some(()),
                0xf0 => k += 16,
                _ if value & 15 == 0 => return None,
                _ => {
                    k += (value >> 4) as usize;
                    (k <= 63).then_some(())?;
                    for _ in 0..value & 15 {
                        self.bit()?;
                    }
                    k += 1;
                }
            }
        }
        (k == 64).then_some(())
    }
    fn scan(&mut self, header: &[u8]) -> Option<()> {
        let components = self.frame?;
        (header.len() == 10 && header[0] == 3 && header[7..] == [0, 63, 0]).then_some(())?;
        let tables = std::mem::take(&mut self.tables);
        let mut selectors = Vec::new();
        for i in 0..3 {
            let (id, selector) = (header[1 + 2 * i], header[2 + 2 * i]);
            let (dc, ac) = ((selector >> 4) as usize, (selector & 15) as usize);
            let (component, quant) = components[i];
            (id == component && dc <= 1 && ac <= 1 && self.quantized[quant as usize])
                .then_some(())?;
            selectors.push((tables[dc].as_ref()?, tables[2 + ac].as_ref()?));
        }
        for _ in 0..(WIDTH / 16) * (HEIGHT / 16) {
            for component in [0, 0, 0, 0, 1, 2] {
                let (dc, ac) = selectors[component];
                self.block(dc, ac)?;
            }
        }
        // Ones fill the last byte, then EOI.
        let fill = (1 << self.count) - 1;
        (self.bits & fill == fill && self.byte()? == 0xff).then_some(())?;
        let mut marker = self.byte()?;
        while marker == 0xff {
            marker = self.byte()?;
        }
        (marker == 0xd9).then_some(())
    }
}

/// The ID and the quantization table of each component of a frame header
/// that the camera shows: baseline, 8 bits, `WIDTH` by `HEIGHT`, three
/// components with their own IDs and 4:2:0 chroma.
fn frame(body: &[u8]) -> Option<[(u8, u8); 3]> {
    (body.len() == 15
        && body[0] == 8
        && u16::from_be_bytes([body[1], body[2]]) as usize == HEIGHT
        && u16::from_be_bytes([body[3], body[4]]) as usize == WIDTH
        && body[5] == 3
        && [body[7], body[10], body[13]] == [0x22, 0x11, 0x11]
        && [body[8], body[11], body[14]].iter().all(|&q| q <= 3)
        && body[6] != body[9]
        && body[6] != body[12]
        && body[9] != body[12])
        .then_some([
            (body[6], body[8]),
            (body[9], body[11]),
            (body[12], body[14]),
        ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_file_is_one_that_the_camera_shows() {
        // Noise is the picture that compresses least.
        let mut state = 1u32;
        let noise: Vec<u8> = (0..WIDTH * HEIGHT * 3)
            .map(|_| {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                (state >> 24) as u8
            })
            .collect();
        for rgb in [vec![128; WIDTH * HEIGHT * 3], noise] {
            let bytes = encode(&rgb).unwrap();
            assert!(usable(&bytes));
            // A file that ends early, or whose scan does not decode.
            assert!(!usable(&bytes[..bytes.len() - 2]));
            assert!(!usable(&bytes[..bytes.len() / 2]));
            let mut broken = bytes.clone();
            let middle = broken.len() / 2;
            broken[middle..middle + 2].copy_from_slice(&[0xff, 0xd0]);
            assert!(!usable(&broken));
            // A comment segment after SOI.
            let commented = [&bytes[..2], &[0xff, 0xfe, 0, 4, b'h', b'i'], &bytes[2..]].concat();
            assert!(!usable(&commented));
        }
        assert!(encode(&[0; 16 * 16 * 3]).is_err());
        assert!(!usable(
            &jpeg::encode(&[0; 16 * 16 * 3], 16, 16, 90).unwrap()
        ));
    }

    /// The camera and the tools agree on the file and its limits.
    #[test]
    fn the_runtime_names_the_same_file() {
        let runtime = include_str!("../../../runtime/goodbye.h");
        assert!(runtime.contains(&format!("\"C:\\\\{}\"", FILE.replace('/', "\\\\"))));
        assert!(runtime.contains(&format!("#define GOODBYE_WIDTH {WIDTH}U")));
        assert!(runtime.contains(&format!("#define GOODBYE_HEIGHT {HEIGHT}U")));
        assert!(runtime.contains("#define GOODBYE_BYTES_MAX (GOODBYE_WIDTH*GOODBYE_HEIGHT*2U)"));
    }
}
