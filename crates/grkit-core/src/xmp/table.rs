//! Big-table payloads embedded in XMP (`crs:Table_<fingerprint>`).
//!
//! Encoding: XML-safe base-85 text, then a 4-byte little-endian uncompressed length, then zlib.
//! The binary layouts follow the DNG specification 1.6 (`RGBTables`, `ProfileLookTableData`)
//! and the public DNG SDK.

use anyhow::{bail, ensure, Context, Result};
use flate2::read::ZlibDecoder;
use std::io::Read;

pub const BTT_LOOK_TABLE: u32 = 0;
pub const BTT_RGB_TABLE: u32 = 1;
const RGB_TABLE_VERSION: u32 = 1;
const LOOK_TABLE_VERSION1: u32 = 1;
const LOOK_TABLE_VERSION2: u32 = 2;

/// XML-safe base-85 alphabet used by Adobe big tables (Z85-like, XMP-adjusted).
pub const BASE85: &[u8; 85] =
    b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ.-:+=^!/*?`'|()[]{}@%$#";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Primaries {
    Srgb = 0,
    Adobe = 1,
    ProPhoto = 2,
    P3 = 3,
    Rec2020 = 4,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gamma {
    Linear = 0,
    Srgb = 1,
    Gamma18 = 2,
    Gamma22 = 3,
    Rec2020 = 4,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gamut {
    Clip = 0,
    Extend = 1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LookEncoding {
    Linear = 0,
    Srgb = 1,
}

/// A decoded RGB big table (1D or 3D). Samples are absolute 16-bit RGB0 quads.
#[derive(Clone, Debug)]
pub struct RgbTable {
    pub dimensions: u32,
    pub divisions: u32,
    pub samples: Vec<u16>,
    pub primaries: Primaries,
    pub gamma: Gamma,
    pub gamut: Gamut,
    pub min_amount: f64,
    pub max_amount: f64,
}

/// A decoded HSV look table. Entries are value-outer, hue-middle, saturation-inner.
#[derive(Clone, Debug)]
pub struct LookTable {
    pub hue_divisions: u32,
    pub sat_divisions: u32,
    pub val_divisions: u32,
    pub entries: Vec<[f32; 3]>,
    pub encoding: LookEncoding,
    pub min_amount: f64,
    pub max_amount: f64,
}

#[derive(Clone, Debug)]
pub enum BigTable {
    Rgb(RgbTable),
    Look(LookTable),
}

/// Decodes a `crs:Table_*` base-85 payload into an RGB or look table.
pub fn decode_big_table(encoded: &str) -> Result<BigTable> {
    let compressed = decode_base85(encoded)?;
    ensure!(
        compressed.len() >= 5,
        "Embedded table is too short after base-85 decode"
    );
    let uncompressed_size = u32::from_le_bytes(compressed[..4].try_into().unwrap()) as usize;
    ensure!(
        uncompressed_size > 0 && uncompressed_size <= 64 * 1024 * 1024,
        "Embedded table uncompressed size is out of range"
    );
    let mut raw = Vec::new();
    ZlibDecoder::new(&compressed[4..])
        .take(uncompressed_size as u64 + 1)
        .read_to_end(&mut raw)
        .context("Embedded table zlib data is damaged")?;
    ensure!(
        raw.len() == uncompressed_size,
        "Embedded table zlib size mismatch"
    );
    decode_table_binary(&raw)
}

pub fn decode_table_binary(data: &[u8]) -> Result<BigTable> {
    let mut r = Reader::new(data);
    let kind = r.u32()?;
    match kind {
        BTT_LOOK_TABLE => Ok(BigTable::Look(decode_look(&mut r)?)),
        BTT_RGB_TABLE => Ok(BigTable::Rgb(decode_rgb(&mut r)?)),
        _ => bail!("Unknown embedded table type {kind}"),
    }
}

fn decode_rgb(r: &mut Reader<'_>) -> Result<RgbTable> {
    let version = r.u32()?;
    ensure!(
        version == RGB_TABLE_VERSION,
        "Unknown RGB table version {version}"
    );
    let dimensions = r.u32()?;
    let divisions = r.u32()?;
    match dimensions {
        1 => ensure!(
            (2..=4096).contains(&divisions),
            "Invalid 1D RGB table divisions"
        ),
        3 => ensure!(
            (2..=32).contains(&divisions),
            "Invalid 3D RGB table divisions"
        ),
        _ => bail!("Invalid RGB table dimensions {dimensions}"),
    }
    let nop: Vec<u16> = (0..divisions)
        .map(|i| ((i * 0x0FFFF + (divisions >> 1)) / (divisions - 1)) as u16)
        .collect();
    let count = match dimensions {
        1 => divisions as usize,
        _ => (divisions as usize).pow(3),
    };
    let mut samples = Vec::with_capacity(count * 4);
    if dimensions == 1 {
        for &n in &nop {
            samples.push(r.u16()?.wrapping_add(n));
            samples.push(r.u16()?.wrapping_add(n));
            samples.push(r.u16()?.wrapping_add(n));
            samples.push(0);
        }
    } else {
        for r_i in 0..divisions as usize {
            for g_i in 0..divisions as usize {
                for b_i in 0..divisions as usize {
                    samples.push(r.u16()?.wrapping_add(nop[r_i]));
                    samples.push(r.u16()?.wrapping_add(nop[g_i]));
                    samples.push(r.u16()?.wrapping_add(nop[b_i]));
                    samples.push(0);
                }
            }
        }
    }
    let primaries = match r.u32()? {
        0 => Primaries::Srgb,
        1 => Primaries::Adobe,
        2 => Primaries::ProPhoto,
        3 => Primaries::P3,
        4 => Primaries::Rec2020,
        v => bail!("Unknown RGB table primaries {v}"),
    };
    let gamma = match r.u32()? {
        0 => Gamma::Linear,
        1 => Gamma::Srgb,
        2 => Gamma::Gamma18,
        3 => Gamma::Gamma22,
        4 => Gamma::Rec2020,
        v => bail!("Unknown RGB table gamma {v}"),
    };
    let gamut = match r.u32()? {
        0 => Gamut::Clip,
        1 => Gamut::Extend,
        v => bail!("Unknown RGB table gamut {v}"),
    };
    let min_amount = r.f64()?;
    let max_amount = r.f64()?;
    ensure!(
        (0.0..=1.0).contains(&min_amount) && max_amount >= 1.0,
        "Invalid RGB table amount range"
    );
    if r.remaining() >= 4 {
        let _flags = r.u32()?;
    }
    Ok(RgbTable {
        dimensions,
        divisions,
        samples,
        primaries,
        gamma,
        gamut,
        min_amount,
        max_amount,
    })
}

fn decode_look(r: &mut Reader<'_>) -> Result<LookTable> {
    let version = r.u32()?;
    ensure!(
        version == LOOK_TABLE_VERSION1 || version == LOOK_TABLE_VERSION2,
        "Unknown look table version {version}"
    );
    let hue_divisions = r.u32()?;
    let sat_divisions = r.u32()?;
    let val_divisions = r.u32()?;
    ensure!(
        hue_divisions >= 1
            && sat_divisions >= 2
            && val_divisions >= 1
            && (hue_divisions as u64) * (sat_divisions as u64) * (val_divisions as u64)
                <= 1_000_000,
        "Invalid look table divisions"
    );
    let count = (hue_divisions * sat_divisions * val_divisions) as usize;
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        entries.push([r.f32()?, r.f32()?, r.f32()?]);
    }
    let encoding = match r.u32()? {
        0 => LookEncoding::Linear,
        1 => LookEncoding::Srgb,
        v => bail!("Unknown look table encoding {v}"),
    };
    let (min_amount, max_amount) = if version == LOOK_TABLE_VERSION1 {
        (1.0, 1.0)
    } else {
        let min_amount = r.f64()?;
        let max_amount = r.f64()?;
        ensure!(
            (0.0..=1.0).contains(&min_amount) && max_amount >= 1.0,
            "Invalid look table amount range"
        );
        (min_amount, max_amount)
    };
    if r.remaining() >= 4 {
        let _flags = r.u32()?;
    }
    Ok(LookTable {
        hue_divisions,
        sat_divisions,
        val_divisions,
        entries,
        encoding,
        min_amount,
        max_amount,
    })
}

/// Looks up a 3D RGB table with trilinear interpolation. Input and output are 0..1.
pub fn lookup_rgb_3d(table: &RgbTable, rgb: [f64; 3]) -> [f64; 3] {
    debug_assert_eq!(table.dimensions, 3);
    let n = table.divisions as usize;
    let last = (n - 1) as f64;
    let pos = |v: f64| {
        let x = v.clamp(0.0, 1.0) * last;
        let low = (x as usize).min(n - 2);
        (low, x - low as f64)
    };
    let (r, g, b) = (pos(rgb[0]), pos(rgb[1]), pos(rgb[2]));
    let mut out = [0.0; 3];
    for corner in 0..8 {
        let (dr, dg, db) = (corner & 1, (corner >> 1) & 1, corner >> 2);
        let weight = [(r, dr), (g, dg), (b, db)]
            .iter()
            .map(|((_, t), d)| if *d == 1 { *t } else { 1.0 - t })
            .product::<f64>();
        if weight == 0.0 {
            continue;
        }
        let index = (r.0 + dr) * n * n + (g.0 + dg) * n + (b.0 + db);
        let sample = &table.samples[index * 4..index * 4 + 3];
        for (slot, value) in out.iter_mut().zip(sample) {
            *slot += weight * (*value as f64 / 65535.0);
        }
    }
    out
}

/// Applies `out = in + amount * (table(in) - in)`.
pub fn apply_amount(input: [f64; 3], mapped: [f64; 3], amount: f64) -> [f64; 3] {
    std::array::from_fn(|c| input[c] + amount * (mapped[c] - input[c]))
}

/// Looks up a look table. `hsv` is hue in turns 0..1, saturation and value in 0..1.
pub fn lookup_look(table: &LookTable, hsv: [f64; 3]) -> [f32; 3] {
    let (h_div, s_div, v_div) = (
        table.hue_divisions as usize,
        table.sat_divisions as usize,
        table.val_divisions as usize,
    );
    let h = hsv[0].rem_euclid(1.0) * h_div as f64;
    let s = hsv[1].clamp(0.0, 1.0) * (s_div - 1) as f64;
    let v = if v_div <= 1 {
        0.0
    } else {
        hsv[2].clamp(0.0, 1.0) * (v_div - 1) as f64
    };
    let h0 = (h.floor() as usize) % h_div;
    let h1 = (h0 + 1) % h_div;
    let hf = h - h.floor();
    let s0 = (s.floor() as usize).min(s_div - 2);
    let s1 = s0 + 1;
    let sf = s - s0 as f64;
    let (v0, v1, vf) = if v_div <= 1 {
        (0usize, 0usize, 0.0)
    } else {
        let v0 = (v.floor() as usize).min(v_div - 2);
        (v0, v0 + 1, v - v0 as f64)
    };
    let entry = |hv: usize, sv: usize, vv: usize| {
        let index = vv * h_div * s_div + hv * s_div + sv;
        table.entries[index]
    };
    let mut out = [0.0f32; 3];
    for (wh, hi) in [(1.0 - hf, h0), (hf, h1)] {
        for (ws, si) in [(1.0 - sf, s0), (sf, s1)] {
            for (wv, vi) in [(1.0 - vf, v0), (vf, v1)] {
                let w = (wh * ws * wv) as f32;
                if w == 0.0 {
                    continue;
                }
                let e = entry(hi, si, vi);
                for c in 0..3 {
                    out[c] += w * e[c];
                }
            }
        }
    }
    out
}

pub fn decode_base85(text: &str) -> Result<Vec<u8>> {
    let mut decode = [0xFFu8; 128];
    for (i, &ch) in BASE85.iter().enumerate() {
        decode[ch as usize] = i as u8;
    }
    let mut out = Vec::with_capacity(text.len().div_ceil(5) * 4);
    let mut phase = 0u32;
    let mut value = 0u32;
    for &b in text.as_bytes() {
        if !(32..=127).contains(&b) {
            continue;
        }
        let d = decode[b as usize];
        if d > 85 {
            continue;
        }
        phase += 1;
        match phase {
            1 => value = d as u32,
            2 => value += d as u32 * 85,
            3 => value += d as u32 * 85 * 85,
            4 => value += d as u32 * 85 * 85 * 85,
            _ => {
                value += d as u32 * 85 * 85 * 85 * 85;
                out.extend(value.to_le_bytes());
                phase = 0;
            }
        }
    }
    if phase > 1 {
        let mut buf = [0u8; 3];
        let mut n = 0;
        if phase > 3 {
            buf[2] = (value >> 16) as u8;
            n = 3;
        }
        if phase > 2 {
            buf[1] = (value >> 8) as u8;
            n = n.max(2);
        }
        buf[0] = value as u8;
        n = n.max(1);
        out.extend_from_slice(&buf[..n]);
    }
    Ok(out)
}

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        ensure!(
            self.pos + n <= self.data.len(),
            "Embedded table is truncated"
        );
        let slice = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(slice)
    }

    fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }

    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    fn f32(&mut self) -> Result<f32> {
        Ok(f32::from_bits(self.u32()?))
    }

    fn f64(&mut self) -> Result<f64> {
        Ok(f64::from_bits(u64::from_le_bytes(
            self.take(8)?.try_into().unwrap(),
        )))
    }
}
