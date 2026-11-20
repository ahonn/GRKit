//! A baseline JPEG encoder for the pictures that the camera decodes: three
//! components with 4:2:0 chroma, the quantization tables of the JPEG standard
//! scaled by a quality, and its Huffman tables. Each platform gives the same
//! bytes for the same pixels, so the site and the CLI write the same file.

use anyhow::{ensure, Result};

/// The block order of the coefficients in a JPEG stream.
const ZIGZAG: [usize; 64] = [
    0, 1, 8, 16, 9, 2, 3, 10, 17, 24, 32, 25, 18, 11, 4, 5, 12, 19, 26, 33, 40, 48, 41, 34, 27, 20,
    13, 6, 7, 14, 21, 28, 35, 42, 49, 56, 57, 50, 43, 36, 29, 22, 15, 23, 30, 37, 44, 51, 58, 59,
    52, 45, 38, 31, 39, 46, 53, 60, 61, 54, 47, 55, 62, 63,
];
/// The quantization tables of Annex K of the JPEG standard, in row order.
const LUMA_QUANTIZATION: [u8; 64] = [
    16, 11, 10, 16, 24, 40, 51, 61, 12, 12, 14, 19, 26, 58, 60, 55, 14, 13, 16, 24, 40, 57, 69, 56,
    14, 17, 22, 29, 51, 87, 80, 62, 18, 22, 37, 56, 68, 109, 103, 77, 24, 35, 55, 64, 81, 104, 113,
    92, 49, 64, 78, 87, 103, 121, 120, 101, 72, 92, 95, 98, 112, 100, 103, 99,
];
const CHROMA_QUANTIZATION: [u8; 64] = [
    17, 18, 24, 47, 99, 99, 99, 99, 18, 21, 26, 66, 99, 99, 99, 99, 24, 26, 56, 99, 99, 99, 99, 99,
    47, 66, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99,
    99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99,
];
/// The Huffman tables of Annex K: the number of codes of each length, 1 to 16
/// bits, then the values in the order of their codes.
const DC_LUMA: (&[u8; 16], &[u8]) = (
    &[0, 1, 5, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0],
    &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
);
const DC_CHROMA: (&[u8; 16], &[u8]) = (
    &[0, 3, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0],
    &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
);
const AC_LUMA: (&[u8; 16], &[u8]) = (
    &[0, 2, 1, 3, 3, 2, 4, 3, 5, 5, 4, 4, 0, 0, 1, 0x7d],
    &[
        0x01, 0x02, 0x03, 0x00, 0x04, 0x11, 0x05, 0x12, 0x21, 0x31, 0x41, 0x06, 0x13, 0x51, 0x61,
        0x07, 0x22, 0x71, 0x14, 0x32, 0x81, 0x91, 0xa1, 0x08, 0x23, 0x42, 0xb1, 0xc1, 0x15, 0x52,
        0xd1, 0xf0, 0x24, 0x33, 0x62, 0x72, 0x82, 0x09, 0x0a, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x25,
        0x26, 0x27, 0x28, 0x29, 0x2a, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x43, 0x44, 0x45,
        0x46, 0x47, 0x48, 0x49, 0x4a, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5a, 0x63, 0x64,
        0x65, 0x66, 0x67, 0x68, 0x69, 0x6a, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7a, 0x83,
        0x84, 0x85, 0x86, 0x87, 0x88, 0x89, 0x8a, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99,
        0x9a, 0xa2, 0xa3, 0xa4, 0xa5, 0xa6, 0xa7, 0xa8, 0xa9, 0xaa, 0xb2, 0xb3, 0xb4, 0xb5, 0xb6,
        0xb7, 0xb8, 0xb9, 0xba, 0xc2, 0xc3, 0xc4, 0xc5, 0xc6, 0xc7, 0xc8, 0xc9, 0xca, 0xd2, 0xd3,
        0xd4, 0xd5, 0xd6, 0xd7, 0xd8, 0xd9, 0xda, 0xe1, 0xe2, 0xe3, 0xe4, 0xe5, 0xe6, 0xe7, 0xe8,
        0xe9, 0xea, 0xf1, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8, 0xf9, 0xfa,
    ],
);
const AC_CHROMA: (&[u8; 16], &[u8]) = (
    &[0, 2, 1, 2, 4, 4, 3, 4, 7, 5, 4, 4, 0, 1, 2, 0x77],
    &[
        0x00, 0x01, 0x02, 0x03, 0x11, 0x04, 0x05, 0x21, 0x31, 0x06, 0x12, 0x41, 0x51, 0x07, 0x61,
        0x71, 0x13, 0x22, 0x32, 0x81, 0x08, 0x14, 0x42, 0x91, 0xa1, 0xb1, 0xc1, 0x09, 0x23, 0x33,
        0x52, 0xf0, 0x15, 0x62, 0x72, 0xd1, 0x0a, 0x16, 0x24, 0x34, 0xe1, 0x25, 0xf1, 0x17, 0x18,
        0x19, 0x1a, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x43, 0x44,
        0x45, 0x46, 0x47, 0x48, 0x49, 0x4a, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5a, 0x63,
        0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6a, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7a,
        0x82, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89, 0x8a, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97,
        0x98, 0x99, 0x9a, 0xa2, 0xa3, 0xa4, 0xa5, 0xa6, 0xa7, 0xa8, 0xa9, 0xaa, 0xb2, 0xb3, 0xb4,
        0xb5, 0xb6, 0xb7, 0xb8, 0xb9, 0xba, 0xc2, 0xc3, 0xc4, 0xc5, 0xc6, 0xc7, 0xc8, 0xc9, 0xca,
        0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8, 0xd9, 0xda, 0xe2, 0xe3, 0xe4, 0xe5, 0xe6, 0xe7,
        0xe8, 0xe9, 0xea, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8, 0xf9, 0xfa,
    ],
);
/// `cos(k * pi / 16)` for k = 0..8, as literals: a library cosine can differ
/// in its last bit between platforms.
const COSINE: [f64; 8] = [
    1.0,
    0.980_785_280_403_230_4,
    0.923_879_532_511_286_7,
    0.831_469_612_302_545_2,
    std::f64::consts::FRAC_1_SQRT_2,
    0.555_570_233_019_602_2,
    0.382_683_432_365_089_8,
    0.195_090_322_016_128_27,
];

/// `cos((2 * x + 1) * u * pi / 16)`.
fn cosine(x: usize, u: usize) -> f64 {
    let k = (2 * x + 1) * u % 32;
    match k {
        0..=7 => COSINE[k],
        8 => 0.0,
        9..=16 => -COSINE[16 - k],
        17..=23 => -COSINE[k - 16],
        24 => 0.0,
        _ => COSINE[32 - k],
    }
}

/// The quantization table of a quality, 1 to 100, as libjpeg scales it.
fn quantization(base: &[u8; 64], quality: u8) -> [u8; 64] {
    let quality = quality.clamp(1, 100) as u32;
    let scale = if quality < 50 {
        5000 / quality
    } else {
        200 - 2 * quality
    };
    base.map(|value| ((value as u32 * scale + 50) / 100).clamp(1, 255) as u8)
}

/// The code and the length of each value of a Huffman table.
fn codes(table: (&[u8; 16], &[u8])) -> [(u16, u8); 256] {
    let mut out = [(0, 0); 256];
    let (mut code, mut at) = (0u16, 0);
    for (length, &count) in table.0.iter().enumerate() {
        for _ in 0..count {
            out[table.1[at] as usize] = (code, length as u8 + 1);
            code += 1;
            at += 1;
        }
        code <<= 1;
    }
    out
}

struct Bits {
    out: Vec<u8>,
    pending: u32,
    count: u32,
}

impl Bits {
    /// Append the low `length` bits of `code`.
    fn put(&mut self, code: u16, length: u8) {
        debug_assert!(length == 16 || code >> length == 0);
        self.pending = self.pending << length | code as u32;
        self.count += length as u32;
        while self.count >= 8 {
            self.count -= 8;
            let byte = (self.pending >> self.count) as u8;
            self.out.push(byte);
            // A zero follows each 0xff of the entropy-coded data.
            if byte == 0xff {
                self.out.push(0);
            }
        }
    }
}

/// The DCT of a block of samples around zero, quantized, in zigzag order.
fn transform(block: &[f64; 64], table: &[u8; 64]) -> [i32; 64] {
    let mut rows = [0.0; 64];
    for y in 0..8 {
        for u in 0..8 {
            rows[y * 8 + u] = (0..8).map(|x| block[y * 8 + x] * cosine(x, u)).sum();
        }
    }
    let mut out = [0; 64];
    for (place, &at) in ZIGZAG.iter().enumerate() {
        let (v, u) = (at / 8, at % 8);
        let sum: f64 = (0..8).map(|y| rows[y * 8 + u] * cosine(y, v)).sum();
        let scale = |k: usize| if k == 0 { COSINE[4] } else { 1.0 };
        let value = sum * scale(u) * scale(v) / 4.0 / table[at] as f64;
        out[place] = value.round() as i32;
    }
    out
}

/// The number of bits of a coefficient and its bits after the Huffman code.
fn magnitude(value: i32) -> (u8, u16) {
    let size = (32 - value.unsigned_abs().leading_zeros()) as u8;
    let bits = if value < 0 {
        value - 1 + (1 << size)
    } else {
        value
    };
    (size, bits as u16)
}

fn block(
    bits: &mut Bits,
    coefficients: &[i32; 64],
    previous: &mut i32,
    dc: &[(u16, u8); 256],
    ac: &[(u16, u8); 256],
) {
    let (size, extra) = magnitude(coefficients[0] - *previous);
    *previous = coefficients[0];
    bits.put(dc[size as usize].0, dc[size as usize].1);
    bits.put(extra, size);
    let mut run = 0;
    for &value in &coefficients[1..] {
        if value == 0 {
            run += 1;
            continue;
        }
        for _ in 0..run / 16 {
            bits.put(ac[0xf0].0, ac[0xf0].1);
        }
        let (size, extra) = magnitude(value);
        let symbol = (run % 16) << 4 | size as usize;
        bits.put(ac[symbol].0, ac[symbol].1);
        bits.put(extra, size);
        run = 0;
    }
    if run > 0 {
        bits.put(ac[0].0, ac[0].1);
    }
}

fn segment(out: &mut Vec<u8>, marker: u8, body: &[u8]) {
    out.extend([0xff, marker]);
    out.extend(((body.len() + 2) as u16).to_be_bytes());
    out.extend(body);
}

/// Encode `rgb`, 8-bit RGB pixels in row order, as a baseline JPEG with 4:2:0
/// chroma. The width and the height are multiples of 16.
pub fn encode(rgb: &[u8], width: usize, height: usize, quality: u8) -> Result<Vec<u8>> {
    ensure!(
        width > 0 && height > 0 && width.is_multiple_of(16) && height.is_multiple_of(16),
        "The size of the picture must be a multiple of 16 pixels"
    );
    ensure!(
        width <= 0xffff && height <= 0xffff && rgb.len() == width * height * 3,
        "The pixels do not match the size of the picture"
    );
    let tables = [
        quantization(&LUMA_QUANTIZATION, quality),
        quantization(&CHROMA_QUANTIZATION, quality),
    ];
    let mut out = vec![0xff, 0xd8];
    segment(&mut out, 0xe0, b"JFIF\0\x01\x01\0\0\x01\0\x01\0\0");
    for (id, table) in tables.iter().enumerate() {
        let mut body = vec![id as u8];
        body.extend(ZIGZAG.iter().map(|&at| table[at]));
        segment(&mut out, 0xdb, &body);
    }
    let mut frame = vec![8];
    frame.extend((height as u16).to_be_bytes());
    frame.extend((width as u16).to_be_bytes());
    // Y with two samples in each direction, then Cb and Cr with one.
    frame.extend([3, 1, 0x22, 0, 2, 0x11, 1, 3, 0x11, 1]);
    segment(&mut out, 0xc0, &frame);
    for (class, table) in [
        (0x00, DC_LUMA),
        (0x10, AC_LUMA),
        (0x01, DC_CHROMA),
        (0x11, AC_CHROMA),
    ] {
        let mut body = vec![class];
        body.extend(table.0);
        body.extend(table.1);
        segment(&mut out, 0xc4, &body);
    }
    segment(&mut out, 0xda, &[3, 1, 0x00, 2, 0x11, 3, 0x11, 0, 63, 0]);

    let (dc_luma, ac_luma) = (codes(DC_LUMA), codes(AC_LUMA));
    let (dc_chroma, ac_chroma) = (codes(DC_CHROMA), codes(AC_CHROMA));
    let mut bits = Bits {
        out,
        pending: 0,
        count: 0,
    };
    let mut previous = [0; 3];
    for top in (0..height).step_by(16) {
        for left in (0..width).step_by(16) {
            // The 16 by 16 pixels of the unit as Y, Cb, and Cr around zero.
            let mut unit = [[0.0f64; 256]; 3];
            for y in 0..16 {
                for x in 0..16 {
                    let at = ((top + y) * width + left + x) * 3;
                    let (r, g, b) = (rgb[at] as f64, rgb[at + 1] as f64, rgb[at + 2] as f64);
                    unit[0][y * 16 + x] = 0.299 * r + 0.587 * g + 0.114 * b - 128.0;
                    unit[1][y * 16 + x] = -0.168_735_9 * r - 0.331_264_1 * g + 0.5 * b;
                    unit[2][y * 16 + x] = 0.5 * r - 0.418_687_6 * g - 0.081_312_4 * b;
                }
            }
            for part in 0..4 {
                let (x0, y0) = (part % 2 * 8, part / 2 * 8);
                let mut samples = [0.0; 64];
                for y in 0..8 {
                    for x in 0..8 {
                        samples[y * 8 + x] = unit[0][(y0 + y) * 16 + x0 + x];
                    }
                }
                let coefficients = transform(&samples, &tables[0]);
                block(
                    &mut bits,
                    &coefficients,
                    &mut previous[0],
                    &dc_luma,
                    &ac_luma,
                );
            }
            for component in 1..3 {
                let mut samples = [0.0; 64];
                for y in 0..8 {
                    for x in 0..8 {
                        let at = y * 32 + x * 2;
                        let plane = &unit[component];
                        samples[y * 8 + x] =
                            (plane[at] + plane[at + 1] + plane[at + 16] + plane[at + 17]) / 4.0;
                    }
                }
                let coefficients = transform(&samples, &tables[1]);
                block(
                    &mut bits,
                    &coefficients,
                    &mut previous[component],
                    &dc_chroma,
                    &ac_chroma,
                );
            }
        }
    }
    // Ones fill the last byte.
    if bits.count > 0 {
        let fill = 8 - bits.count as u8;
        bits.put((1 << fill) - 1, fill);
    }
    let mut out = bits.out;
    out.extend([0xff, 0xd9]);
    Ok(out)
}

/// The frame of a baseline JPEG: its width, its height, and the sampling of
/// each component as horizontal << 4 | vertical. `None` for other data, also
/// for a progressive JPEG.
pub fn baseline_frame(bytes: &[u8]) -> Option<(u16, u16, Vec<u8>)> {
    if bytes.get(..2)? != [0xff, 0xd8] {
        return None;
    }
    let mut at = 2;
    loop {
        let header = bytes.get(at..at + 4)?;
        if header[0] != 0xff {
            return None;
        }
        let length = u16::from_be_bytes([header[2], header[3]]) as usize;
        let body = bytes.get(at + 4..at + 2 + length)?;
        match header[1] {
            0xc0 => {
                let components = *body.get(5)? as usize;
                let sampling = (0..components)
                    .map(|i| body.get(7 + 3 * i).copied())
                    .collect::<Option<Vec<_>>>()?;
                return Some((
                    u16::from_be_bytes([body[3], body[4]]),
                    u16::from_be_bytes([body[1], body[2]]),
                    sampling,
                ));
            }
            // Another frame type, or the scan before a frame.
            0xc1..=0xc3 | 0xc5..=0xc7 | 0xc9..=0xcb | 0xcd..=0xcf | 0xda => return None,
            _ => at += 2 + length,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reads the entropy-coded data of a picture of `units` 16 by 16 units to
    /// its end, as a strict decoder does: the first coefficient of each
    /// block, and the bits that are left in the last byte.
    fn scan(bytes: &[u8], units: usize) -> (Vec<i32>, Vec<u16>) {
        let start = bytes.windows(2).position(|w| w == [0xff, 0xda]).unwrap() + 14;
        // The data without the zero after each 0xff, and without the end marker.
        let mut data = Vec::new();
        let mut at = start;
        while bytes[at..at + 2] != [0xff, 0xd9] {
            data.push(bytes[at]);
            at += if bytes[at] == 0xff { 2 } else { 1 };
        }
        assert_eq!(at + 2, bytes.len());
        let mut place = 0;
        let bit = |place: &mut usize| {
            let value = (data[*place / 8] >> (7 - *place % 8) & 1) as u16;
            *place += 1;
            value
        };
        let symbol = |place: &mut usize, table: &[(u16, u8); 256]| {
            let (mut code, mut length) = (0u16, 0u8);
            loop {
                code = code << 1 | bit(place);
                length += 1;
                assert!(length <= 16, "No code of the table matches");
                if let Some(value) = (0..256).find(|&v| table[v] == (code, length)) {
                    return value;
                }
            }
        };
        let tables = [
            (codes(DC_LUMA), codes(AC_LUMA)),
            (codes(DC_CHROMA), codes(AC_CHROMA)),
        ];
        let mut first = Vec::new();
        let mut previous = [0i32; 3];
        for _ in 0..units {
            for component in [0, 0, 0, 0, 1, 2] {
                let (dc, ac) = &tables[(component > 0) as usize];
                let size = symbol(&mut place, dc);
                let mut extra = 0i32;
                for _ in 0..size {
                    extra = extra << 1 | bit(&mut place) as i32;
                }
                if size > 0 && extra < 1 << (size - 1) {
                    extra += 1 - (1 << size);
                }
                previous[component] += extra;
                first.push(previous[component]);
                let mut index = 1;
                while index < 64 {
                    let value = symbol(&mut place, ac);
                    if value == 0 {
                        break;
                    }
                    index += (value >> 4) + 1;
                    for _ in 0..value & 15 {
                        bit(&mut place);
                    }
                }
                assert!(index <= 64);
            }
        }
        let rest = (place..data.len() * 8).map(|at| {
            let mut at = at;
            bit(&mut at)
        });
        (first, rest.collect())
    }

    #[test]
    fn a_flat_picture_encodes_its_level_in_the_first_coefficient() {
        // Gray 200 is Y 72 around zero: a DC of 8 * 72 / 16 at quality 50.
        let bytes = encode(&vec![200; 32 * 16 * 3], 32, 16, 50).unwrap();
        assert_eq!(&bytes[..2], [0xff, 0xd8]);
        assert_eq!(
            baseline_frame(&bytes),
            Some((32, 16, vec![0x22, 0x11, 0x11]))
        );
        assert_eq!(scan(&bytes, 2).0[0], 36);
    }

    /// The camera decodes the data to its end and shows nothing for data that
    /// ends early: each block is whole, and only ones fill the last byte.
    #[test]
    fn the_data_of_a_picture_is_whole_to_its_end() {
        let mut state = 7u32;
        for size in [(16, 16), (48, 32), (64, 16)] {
            let rgb: Vec<u8> = (0..size.0 * size.1 * 3)
                .map(|_| {
                    state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                    (state >> 24) as u8
                })
                .collect();
            for quality in [30, 92] {
                let bytes = encode(&rgb, size.0, size.1, quality).unwrap();
                let (first, rest) = scan(&bytes, size.0 * size.1 / 256);
                assert_eq!(first.len(), size.0 * size.1 / 256 * 6);
                assert!(rest.len() < 8 && rest.iter().all(|&bit| bit == 1));
            }
        }
    }

    #[test]
    fn a_size_that_is_not_whole_units_is_refused() {
        assert!(encode(&[0; 30 * 16 * 3], 30, 16, 90).is_err());
        assert!(encode(&[0; 32 * 16], 32, 16, 90).is_err());
    }

    #[test]
    fn other_data_has_no_baseline_frame() {
        assert_eq!(baseline_frame(b"\x89PNG"), None);
        // A progressive frame.
        let mut bytes = encode(&[0; 16 * 16 * 3], 16, 16, 90).unwrap();
        let frame = bytes.windows(2).position(|w| w == [0xff, 0xc0]).unwrap();
        bytes[frame + 1] = 0xc2;
        assert_eq!(baseline_frame(&bytes), None);
    }
}
