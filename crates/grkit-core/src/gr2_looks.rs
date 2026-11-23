//! The Positive Film look of GR II, from a GR II firmware file that the user gives. The CLI
//! reads the file from a path, and the site reads it in the browser. GRKit ships none of the
//! data; the effect comes from the user's own file.
//!
//! GR II keeps its image parameters in a parameter library on the resource disk of its
//! firmware. With the adjustments at 0, Positive Film differs from Standard in three records:
//! the color matrix, the hue-sector set, and the tone curve. The effect gives the target
//! camera:
//!
//! - the look matrix: Positive's color matrix relative to Standard's, on the target's Standard;
//! - the native look: Positive's hue-sector set after the inverse of Standard's, with full
//!   weight and one level. GR III Standard has no hue sectors, and the native look takes the
//!   place of GR IV Standard's, so the target keeps none of its own;
//! - the curves: Positive's tone curve after the inverse of Standard's, so that the target's own
//!   base curve stands in for GR II Standard.
//!
//! GR III and GR IV have different Standard look matrices, so each gets an effect of its own
//! (`TARGETS`).
use anyhow::{bail, ensure, Context, Result};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::{Cursor, Read};

use crate::effects::Effect;
use crate::native_look::{NativeLook, BANKS, LEVELS};

/// The firmware file in the official GR II zip.
pub const FIRMWARE_FILE: &str = "rg2_v003.frm";
/// The largest file that `Image::read` takes: the official zip has about 13 MB.
pub const MAX_FILE_SIZE: usize = 32 * 1024 * 1024;
/// The GR II version of the supported firmware file.
pub const VERSION: &str = "3.00";
/// The SHA-256 of `FIRMWARE_FILE`.
pub const FIRMWARE_SHA256: &str =
    "3643850f2ae74ae967eb336794048f554caa1b11d05b8569824d5719f1838331";

/// The effect for GR III and GR IIIx.
pub const EFFECT_ID: &str = "gr2-positive-film";
/// The effect for GR IV.
pub const GR4_EFFECT_ID: &str = "gr2-positive-film-gr4";
/// GR III and GR IV have a Positive Film of their own, so the name tells GR II's apart.
pub const NAME: &str = "GR II Positive";
/// Each effect ID and the Standard look matrix of its cameras.
pub const TARGETS: [(&str, &[i16; 9]); 2] = [
    (EFFECT_ID, &crate::fit::STANDARD_LOOK),
    (GR4_EFFECT_ID, &crate::fit::GR4_STANDARD_LOOK),
];

const UNITY: &[u8; 32] = b"UNITY FILE V1.10 / RICOH COMPANY";
const SECTOR: usize = 0x200;
/// The partition that holds the resource disk.
const RESOURCE_PARTITION: &[u8] = b"b01firm6";
const LIBRARY_PATH: &[u8] = b"A:/isp/IQ.bin";

/// Record tables of the parameter library, their record sizes and counts.
const COLOR_MATRICES: (usize, usize, usize) = (8, 24, 11);
const HUE_SETS: (usize, usize, usize) = (9, 424, 6);
const TONE_CURVES: (usize, usize, usize) = (11, 5 * 1024, 30);
/// The records of Standard and Positive Film in each table.
const STANDARD: (usize, usize, usize) = (0, 0, 0);
const POSITIVE: (usize, usize, usize) = (1, 1, 15);

/// True when `bytes` is the official GR II zip or a GR II firmware file, so a caller can pick
/// this reader instead of another camera's.
pub fn detect(bytes: &[u8]) -> bool {
    if bytes.starts_with(UNITY) {
        return true;
    }
    if !bytes.starts_with(b"PK") {
        return false;
    }
    zip::ZipArchive::new(Cursor::new(bytes)).is_ok_and(|archive| {
        archive
            .file_names()
            .any(|name| name.is_ok_and(|name| file_name_is(&name, FIRMWARE_FILE)))
    })
}

fn file_name_is(path: &str, file: &str) -> bool {
    path.rsplit('/')
        .next()
        .is_some_and(|name| name.eq_ignore_ascii_case(file))
}

/// The firmware file of the official zip, or the bytes of a firmware file.
fn firmware_file(bytes: &[u8]) -> Result<Vec<u8>> {
    if !bytes.starts_with(b"PK") {
        return Ok(bytes.to_vec());
    }
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).context("Invalid ZIP file")?;
    let index = (0..archive.len())
        .find(|&i| {
            archive
                .by_index_raw(i)
                .is_ok_and(|entry| entry.name().is_ok_and(|n| file_name_is(&n, FIRMWARE_FILE)))
        })
        .with_context(|| format!("The ZIP file has no {FIRMWARE_FILE}"))?;
    let mut entry = archive.by_index(index)?;
    ensure!(
        entry.size() as usize <= MAX_FILE_SIZE,
        "{FIRMWARE_FILE} is too large"
    );
    let mut out = Vec::with_capacity(entry.size() as usize);
    entry.read_to_end(&mut out)?;
    Ok(out)
}

fn be32(bytes: &[u8], at: usize) -> Result<u32> {
    let b = bytes
        .get(at..at + 4)
        .context("Unexpected end of the firmware")?;
    Ok(u32::from_be_bytes(b.try_into()?))
}

fn be16(bytes: &[u8], at: usize) -> Result<u16> {
    let b = bytes
        .get(at..at + 2)
        .context("Unexpected end of the firmware")?;
    Ok(u16::from_be_bytes(b.try_into()?))
}

fn i16s<const N: usize>(bytes: &[u8], at: usize) -> Result<[i16; N]> {
    let mut out = [0; N];
    for (k, v) in out.iter_mut().enumerate() {
        *v = be16(bytes, at + 2 * k)? as i16;
    }
    Ok(out)
}

/// The partition `name` of the UNITY container of the file, still encrypted.
fn partition<'a>(file: &'a [u8], name: &[u8]) -> Result<&'a [u8]> {
    ensure!(
        file.starts_with(UNITY) && be32(file, 0x20)? as usize == file.len(),
        "Unexpected GR II firmware container"
    );
    for i in 0..be32(file, 0x24)? as usize {
        let entry = 0x40 + 0x40 * i;
        let label = file
            .get(entry + 8..entry + 0x14)
            .context("Unexpected GR II firmware container")?;
        let end = label.iter().position(|b| *b == 0).unwrap_or(label.len());
        if &label[..end] == name {
            let start = be32(file, entry)? as usize * SECTOR;
            let size = be32(file, entry + 4)? as usize;
            return file
                .get(start..start + size)
                .context("Unexpected GR II firmware container");
        }
    }
    bail!("The GR II firmware has no resource disk")
}

/// The bytes `range` of an encrypted partition, decrypted: each big-endian word of sector `n`
/// is XORed with `!(0x41C6_4E6D * n + 0x0719_69ED)`.
fn decrypt(partition: &[u8], start: usize, size: usize) -> Result<Vec<u8>> {
    ensure!(start.is_multiple_of(4), "Unaligned firmware read");
    let bytes = partition
        .get(start..start + size)
        .context("Unexpected end of the firmware")?;
    let mut out = Vec::with_capacity(size + 4);
    for (k, word) in bytes.chunks(4).enumerate() {
        let n = ((start + 4 * k) / SECTOR) as u32;
        let key = !(0x41C6_4E6Du32.wrapping_mul(n).wrapping_add(0x0719_69ED));
        let mut padded = [0; 4];
        padded[..word.len()].copy_from_slice(word);
        let plain = (u32::from_be_bytes(padded) ^ key).to_be_bytes();
        out.extend_from_slice(&plain[..word.len()]);
    }
    Ok(out)
}

/// The parameter library of the resource disk, decrypted.
fn library(file: &[u8]) -> Result<Vec<u8>> {
    let part = partition(file, RESOURCE_PARTITION)?;
    let header = decrypt(part, 0, SECTOR)?;
    let disk_size = be32(&header, 0x164)? as usize;
    ensure!(
        be32(&header, 0x168)? as usize == part.len() && disk_size + SECTOR == part.len(),
        "Unexpected GR II resource disk"
    );
    // Disk offsets count from the end of the header sector.
    let directory = be32(&decrypt(part, SECTOR, 4)?, 0)? as usize;
    ensure!(directory < disk_size, "Unexpected GR II resource disk");
    let entries = decrypt(part, SECTOR + directory, disk_size - directory)?;
    for entry in entries.chunks_exact(0x40) {
        let end = entry[..32].iter().position(|b| *b == 0).unwrap_or(32);
        if end == 0 {
            break;
        }
        if &entry[..end] == LIBRARY_PATH {
            let (offset, size) = (be32(entry, 32)? as usize, be32(entry, 36)? as usize);
            ensure!(offset + size <= disk_size, "Unexpected GR II resource disk");
            return decrypt(part, SECTOR + offset, size);
        }
    }
    bail!("The GR II firmware has no parameter library")
}

/// The record tables of the parameter library.
struct Library {
    bytes: Vec<u8>,
    /// The start of each record table; table `t` ends where `t + 1` starts.
    tables: Vec<usize>,
}

impl Library {
    fn read(bytes: Vec<u8>) -> Result<Self> {
        let message = "Unexpected GR II parameter library";
        ensure!(
            bytes.starts_with(b"0200")
                && bytes.get(0x10..0x14) == Some(b"M6ME".as_slice())
                && be32(&bytes, 4)? as usize == bytes.len(),
            message
        );
        let start = be32(&bytes, 8)? as usize;
        ensure!(start + be32(&bytes, 0x1c)? as usize == bytes.len(), message);
        let mut tables = Vec::new();
        for k in 0..26 {
            tables.push(be32(&bytes, 0x24 + 4 * k)? as usize);
        }
        tables.push(bytes.len());
        ensure!(
            be32(&bytes, 0x8c)? == 0
                && tables[0] == start
                && tables.windows(2).all(|w| w[0] <= w[1]),
            message
        );
        let library = Self { bytes, tables };
        for (table, size, count) in [COLOR_MATRICES, HUE_SETS, TONE_CURVES] {
            ensure!(
                library.tables[table + 1] - library.tables[table] == size * count,
                message
            );
        }
        Ok(library)
    }

    fn record(&self, (table, size, _): (usize, usize, usize), index: usize) -> &[u8] {
        let at = self.tables[table] + index * size;
        &self.bytes[at..at + size]
    }

    /// A color matrix, row-major, 256 = 1.
    fn color_matrix(&self, index: usize) -> Result<[i16; 9]> {
        let record = self.record(COLOR_MATRICES, index);
        let matrix = i16s::<9>(record, 4)?;
        ensure!(
            record[..4] == [77, 150, 29, 1]
                && be16(record, 22)? == 0
                && matrix
                    .chunks_exact(3)
                    .all(|row| row.iter().map(|v| i32::from(*v)).sum::<i32>() == 256),
            "Unexpected GR II color matrix {index}"
        );
        Ok(matrix)
    }

    /// A hue-sector set: its bounds, selectors, and bank matrices.
    fn hue_set(&self, index: usize) -> Result<HueSet> {
        let set = self.record(HUE_SETS, index);
        let message = || format!("Unexpected GR II hue-sector set {index}");
        ensure!(
            i16s::<6>(set, 0)? == [-43, -85, 128, 128, -107, -21],
            message()
        );
        let mut bounds = [0u16; 16];
        for (k, v) in bounds.iter_mut().enumerate() {
            *v = be16(set, 12 + 2 * k)?;
        }
        let selectors: [u8; 20] = set[44..64].try_into()?;
        let mut banks = [[[0.0; 3]; 3]; BANKS];
        for (bank, slot) in banks.iter_mut().enumerate() {
            let values = i16s::<15>(set, 64 + 30 * bank)?;
            ensure!(
                values[9..].iter().all(|v| *v == 0)
                    && values[..9].chunks(3).all(|row| row
                        .iter()
                        .map(|v| i32::from(*v))
                        .sum::<i32>()
                        == 1024),
                message()
            );
            *slot = std::array::from_fn(|r| {
                std::array::from_fn(|c| f64::from(values[r * 3 + c]) / 1024.0)
            });
        }
        Ok(HueSet {
            bounds,
            selectors,
            banks,
        })
    }

    /// The tone curve of a record: 1024 linear input codes to 8-bit output. The record holds
    /// five curves; the four color curves of Standard and Positive Film are the same.
    fn tone_curve(&self, index: usize) -> Result<[u8; 1024]> {
        let record = self.record(TONE_CURVES, index);
        let curve: [u8; 1024] = record[..1024].try_into()?;
        ensure!(
            (1..4).all(|c| record[c * 1024..(c + 1) * 1024] == curve)
                && curve[0] == 0
                && curve[1023] == 255
                && curve.windows(2).all(|w| w[0] <= w[1]),
            "Unexpected GR II tone curve {index}"
        );
        Ok(curve)
    }
}

/// A GR II hue-sector set: bounds and selectors in the layout of `NativeLook`, and bank
/// matrices whose rows sum to 1.
struct HueSet {
    bounds: [u16; 16],
    selectors: [u8; 20],
    banks: [[[f64; 3]; 3]; BANKS],
}

/// The native look that turns colors after `relative` (Positive's color matrix relative to
/// Standard's) from Standard's hue sectors into Positive's: bank b is
/// `P_b · relative · S_b⁻¹ · relative⁻¹`. The two sets share their bounds and selectors. GR II
/// has no weight or levels, so the weight is full and every level is the bank matrix.
fn relative_look(
    standard: &HueSet,
    positive: &HueSet,
    relative: &[[f64; 3]; 3],
) -> Result<NativeLook> {
    ensure!(
        standard.bounds == positive.bounds && standard.selectors == positive.selectors,
        "The GR II hue-sector sets of Standard and Positive Film have different sectors"
    );
    let back = inverse(relative)?;
    let mut levels = [[[0i32; 9]; LEVELS]; BANKS];
    for (bank, slot) in levels.iter_mut().enumerate() {
        let undo = product(&inverse(&standard.banks[bank])?, &back);
        let m = product(&product(&positive.banks[bank], relative), &undo);
        // Round the off-diagonals and give the diagonal the rest, so that gray stays gray.
        let mut q10: [i32; 9] = std::array::from_fn(|k| (m[k / 3][k % 3] * 1024.0).round() as i32);
        for r in 0..3 {
            q10[r * 4] = 1024
                - (0..3)
                    .filter(|&c| c != r)
                    .map(|c| q10[r * 3 + c])
                    .sum::<i32>();
        }
        *slot = [q10; LEVELS];
    }
    NativeLook::from_levels(positive.bounds, positive.selectors, FULL_WEIGHT, &levels)
}

/// MultiAxial control fields for one level and full weight: fields 1..8 keep every pixel on
/// level 0, fields 10..13 start the weight at 1.0 (1024), the slopes are 0, and fields 18..20
/// are the breakpoints of `hue::BORDERS`.
const FULL_WEIGHT: [u16; 21] = [
    0, 0xfff, 0xfff, 0xfff, 0xfff, 0xfff, 0xfff, 0xfff, 0xfff, 0, 1024, 1024, 1024, 1024, 0, 0, 0,
    0, 164, 655, 4095,
];
const _: () = assert!(
    FULL_WEIGHT[18] == crate::hue::BORDERS[0]
        && FULL_WEIGHT[19] == crate::hue::BORDERS[1]
        && FULL_WEIGHT[20] == crate::hue::BORDERS[2]
);

/// The 8-bit output of a tone curve at input `x` of 0..=1023, between codes linearly.
fn sample(curve: &[u8; 1024], x: f64) -> f64 {
    let j = (x.floor() as usize).min(1022);
    let f = x - j as f64;
    f64::from(curve[j]) * (1.0 - f) + f64::from(curve[j + 1]) * f
}

/// The input of 0..=1023 at which a non-decreasing tone curve reaches `y`: the middle of a
/// flat run at `y`, else between the codes around `y`.
fn invert(curve: &[u8; 1024], y: f64) -> f64 {
    let first = curve
        .iter()
        .position(|v| f64::from(*v) >= y)
        .unwrap_or(1023);
    if f64::from(curve[first]) == y {
        let last = curve.iter().rposition(|v| f64::from(*v) == y).unwrap();
        return (first + last) as f64 / 2.0;
    }
    if first == 0 {
        return 0.0;
    }
    let (a, b) = (f64::from(curve[first - 1]), f64::from(curve[first]));
    (first - 1) as f64 + (y - a) / (b - a)
}

/// The effect curve that turns a picture toned like `standard` into one toned like `look`:
/// `look` after the inverse of `standard`, 256 points of 14 bits.
fn relative_curve(standard: &[u8; 1024], look: &[u8; 1024]) -> [u16; 256] {
    let mut out: [u16; 256] = std::array::from_fn(|i| {
        let y = i as f64 * 255.0 / 255.0;
        let v = sample(look, invert(standard, y)) / 255.0 * 16383.0;
        v.round().clamp(0.0, 16383.0) as u16
    });
    // Flat runs of the 8-bit curves can step back by a fraction; keep the curve rising.
    for i in 1..256 {
        out[i] = out[i].max(out[i - 1]);
    }
    out
}

/// The GR II parameter records of Standard and Positive Film.
pub struct Image {
    library: Library,
}

impl Image {
    /// Reads the official GR II zip or its firmware file, and checks that it is version
    /// `VERSION`.
    pub fn read(bytes: &[u8]) -> Result<Self> {
        ensure!(bytes.len() <= MAX_FILE_SIZE, "The file is too large");
        let file = firmware_file(bytes)?;
        ensure!(
            hex::encode(Sha256::digest(&file)) == FIRMWARE_SHA256,
            "This is not a supported GR II firmware file. Supported versions: {VERSION}"
        );
        Ok(Self {
            library: Library::read(library(&file)?)?,
        })
    }

    pub fn version(&self) -> &'static str {
        VERSION
    }

    /// The effect `id` for a camera whose Standard look matrix is `standard` (one of
    /// `TARGETS`).
    pub fn effect(&self, id: &str, standard: &[i16; 9]) -> Result<Effect> {
        let lib = &self.library;
        let source = q8_rows(&lib.color_matrix(POSITIVE.0)?);
        let base = q8_rows(&lib.color_matrix(STANDARD.0)?);
        let relative = product(&source, &inverse(&base)?);
        let matrix = product(&relative, &q13_rows(standard));
        let curve = relative_curve(&lib.tone_curve(STANDARD.2)?, &lib.tone_curve(POSITIVE.2)?);
        Ok(Effect {
            id: id.to_owned(),
            name: NAME.to_owned(),
            flags: 0,
            matrix: to_q13(&matrix)?,
            curves: [curve; 3],
            post_matrix: None,
            selective: None,
            hue_sectors: None,
            native_look: Some(relative_look(
                &lib.hue_set(STANDARD.1)?,
                &lib.hue_set(POSITIVE.1)?,
                &relative,
            )?),
            fn_defaults: None,
        })
    }

    /// The effect.json definitions of the effects of `TARGETS`.
    pub fn definitions(&self) -> Result<Vec<Value>> {
        TARGETS
            .iter()
            .map(|(id, standard)| {
                let mut value = self.effect(id, standard)?.to_json();
                if let Some(object) = value.as_object_mut() {
                    object.insert("schema".to_owned(), json!("grkit-effect-v1"));
                    object.insert(
                        "icon".to_owned(),
                        json!({"template": "cine", "brand": "RICOH", "name": "GR II", "number": "POSITIVE"}),
                    );
                    object.insert(
                        "attribution".to_owned(),
                        json!({
                            "origin": "native-look",
                            "source": format!("GR II {VERSION} firmware of the user"),
                        }),
                    );
                }
                Ok(value)
            })
            .collect()
    }
}

/// The effect that the site previews for `effect`. The preview models GR III, so the GR IV
/// effect moves from GR IV Standard to GR III Standard; the two then give the same picture.
/// Other effects stay as they are.
pub fn for_preview(mut effect: Effect) -> Result<Effect> {
    if effect.id == GR4_EFFECT_ID {
        let gr4 = q13_rows(&crate::fit::GR4_STANDARD_LOOK);
        let gr3 = q13_rows(&crate::fit::STANDARD_LOOK);
        let matrix = product(&product(&q13_rows(&effect.matrix), &inverse(&gr4)?), &gr3);
        effect.matrix = to_q13(&matrix)?;
    }
    Ok(effect)
}

fn q13_rows(m: &[i16; 9]) -> [[f64; 3]; 3] {
    std::array::from_fn(|r| std::array::from_fn(|c| f64::from(m[r * 3 + c]) / 8192.0))
}

fn to_q13(m: &[[f64; 3]; 3]) -> Result<[i16; 9]> {
    let mut q13 = [0i16; 9];
    for (k, slot) in q13.iter_mut().enumerate() {
        let v = (m[k / 3][k % 3] * 8192.0).round();
        ensure!(
            (-32768.0..=32767.0).contains(&v),
            "The look matrix leaves the Q13 range"
        );
        *slot = v as i16;
    }
    Ok(q13)
}

fn q8_rows(m: &[i16; 9]) -> [[f64; 3]; 3] {
    std::array::from_fn(|r| std::array::from_fn(|c| f64::from(m[r * 3 + c]) / 256.0))
}

fn product(a: &[[f64; 3]; 3], b: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    std::array::from_fn(|r| std::array::from_fn(|c| (0..3).map(|k| a[r][k] * b[k][c]).sum()))
}

fn inverse(m: &[[f64; 3]; 3]) -> Result<[[f64; 3]; 3]> {
    let cofactor = |r: usize, c: usize| {
        let (r1, r2) = ((r + 1) % 3, (r + 2) % 3);
        let (c1, c2) = ((c + 1) % 3, (c + 2) % 3);
        m[r1][c1] * m[r2][c2] - m[r1][c2] * m[r2][c1]
    };
    let det: f64 = (0..3).map(|c| m[0][c] * cofactor(0, c)).sum();
    ensure!(det.abs() > 1e-9, "Singular color matrix");
    Ok(std::array::from_fn(|r| {
        std::array::from_fn(|c| cofactor(c, r) / det)
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn other_files_name_the_supported_version() {
        assert!(!detect(b"not firmware"));
        let error = Image::read(b"not firmware").err().unwrap().to_string();
        assert!(
            error.contains("GR II") && error.contains(VERSION),
            "{error}"
        );
    }

    #[test]
    fn decryption_follows_the_sector() {
        let plain = vec![0u8; 2 * SECTOR];
        let cipher = decrypt(&plain, 0, plain.len()).unwrap();
        assert_eq!(&cipher[..4], &(!0x0719_69EDu32).to_be_bytes());
        let next = !(0x41C6_4E6Du32.wrapping_add(0x0719_69ED));
        assert_eq!(&cipher[SECTOR..SECTOR + 4], &next.to_be_bytes());
        // XOR twice gives the input back, also from a start inside a sector.
        assert_eq!(decrypt(&cipher, 8, 600).unwrap(), &plain[8..608]);
    }

    #[test]
    fn a_curve_relative_to_itself_is_linear() {
        let curve: [u8; 1024] =
            std::array::from_fn(|i| ((i as f64 / 1023.0).powf(0.45) * 255.0).round() as u8);
        let relative = relative_curve(&curve, &curve);
        for (i, v) in relative.iter().enumerate() {
            let linear = i as f64 * 16383.0 / 255.0;
            assert!((f64::from(*v) - linear).abs() <= 33.0, "{i}: {v}");
        }
    }
}
