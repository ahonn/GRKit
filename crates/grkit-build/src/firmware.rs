use crate::target::{self, Target};
use anyhow::{bail, ensure, Context, Result};
use sha2::{Digest, Sha256};
use std::path::Path;

pub fn sha256(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// Decode the pinned container framing, including overlapping back references.
pub fn decode(data: &[u8]) -> Result<Vec<u8>> {
    const LIMIT: usize = 256 * 1024 * 1024;
    let mut pos = 128usize;
    let mut out = Vec::new();
    loop {
        let pair = data
            .get(pos..pos + 2)
            .context("Invalid firmware: missing terminator")?;
        let tag = u16::from_be_bytes([pair[0], pair[1]]);
        pos += 2;
        if tag == 0 {
            return Ok(out);
        }
        let end = pos + usize::from(tag & 0x7fff);
        ensure!(end <= data.len(), "Invalid firmware: frame boundary");
        if tag & 0x8000 != 0 {
            ensure!(
                out.len() + end - pos < LIMIT,
                "Invalid firmware: decoded size"
            );
            out.extend_from_slice(&data[pos..end]);
            pos = end;
            continue;
        }
        while pos < end {
            ensure!(pos + 2 <= end, "Invalid firmware: flags boundary");
            let flags = u16::from_be_bytes([data[pos], data[pos + 1]]);
            pos += 2;
            for bit in (0..16).rev() {
                if pos == end {
                    break;
                }
                if flags >> bit & 1 == 0 {
                    ensure!(out.len() + 1 < LIMIT, "Invalid firmware: decoded size");
                    out.push(data[pos]);
                    pos += 1;
                    continue;
                }
                ensure!(pos + 2 <= end, "Invalid firmware: reference boundary");
                let (a, b) = (data[pos], data[pos + 1]);
                pos += 2;
                let distance = (usize::from(a & 0xf8) << 5) | usize::from(b);
                let mut count = usize::from(a & 7);
                if count == 7 {
                    loop {
                        ensure!(pos < end, "Invalid firmware: length boundary");
                        let ext = data[pos];
                        pos += 1;
                        count += usize::from(ext);
                        if ext != 255 {
                            break;
                        }
                    }
                }
                if distance == 0 {
                    break;
                }
                ensure!(distance <= out.len(), "Invalid firmware: back reference");
                ensure!(
                    out.len() + count + 3 < LIMIT,
                    "Invalid firmware: decoded size"
                );
                for _ in 0..count + 3 {
                    out.push(out[out.len() - distance]);
                }
            }
        }
        ensure!(pos == end, "Invalid firmware: frame end");
    }
}

#[derive(Debug)]
pub struct FirmwareImage {
    pub target: &'static Target,
    /// The decoded RTOS image, loaded at `target::RTOS_BASE`.
    pub body: Vec<u8>,
    pub sha256: String,
    pub decoded_sha256: String,
}

impl FirmwareImage {
    /// Reads RTOS memory. The data section reads from its copy source in the image.
    pub fn read(&self, address: u32, size: usize) -> Result<&[u8]> {
        let data = self.target.data_start..self.target.data_start + self.target.data_size;
        let address = if data.contains(&address) {
            address - self.target.data_start + self.target.data_source
        } else {
            address
        };
        let offset = address
            .checked_sub(target::RTOS_BASE)
            .context("Address outside supported firmware")? as usize;
        let end = offset
            .checked_add(size)
            .context("Address outside supported firmware")?;
        self.body
            .get(offset..end)
            .context("Address outside supported firmware")
    }

    /// The native Standard record (84 bytes) that private records copy.
    pub fn stock_record(&self) -> Result<(u32, &[u8])> {
        let image_control = self
            .target
            .image_control
            .with_context(|| format!("{} has no Image Control", self.target.firmware))?;
        let word =
            |address| -> Result<u32> { Ok(u32::from_le_bytes(self.read(address, 4)?.try_into()?)) };
        let address = word(image_control.tables[0] + 11 * 4)?;
        ensure!(
            address == image_control.record && word(address + 4)? == image_control.coefficients,
            "Unsupported stock processing record"
        );
        Ok((address, self.read(address, 84)?))
    }
}

/// Reads the official firmware of `target` and returns its RTOS image.
pub fn verify_firmware(path: &Path, target: &'static Target) -> Result<FirmwareImage> {
    let data = std::fs::read(path).with_context(|| format!("Read firmware {}", path.display()))?;
    let identity = sha256(&data);
    ensure!(
        identity == target.firmware_sha256,
        "Firmware SHA-256 does not match {}",
        target.firmware
    );
    let decoded = decode(&data)?;
    let decoded_identity = sha256(&decoded);
    ensure!(
        decoded_identity == target.decoded_sha256,
        "Decoded firmware SHA-256 mismatch"
    );
    let mut pos = 0;
    while pos + 16 <= decoded.len() {
        let name = &decoded[pos..pos + 8];
        let group = u16::from_le_bytes(decoded[pos + 10..pos + 12].try_into()?);
        let size = u32::from_le_bytes(decoded[pos + 12..pos + 16].try_into()?) as usize;
        let end = (pos + 16usize)
            .checked_add(size)
            .context("Truncated firmware section")?;
        ensure!(end <= decoded.len(), "Truncated firmware section");
        if name == b"RTOS\0\0\0\0" {
            ensure!(group == 1, "Unexpected RTOS group");
            return Ok(FirmwareImage {
                target,
                body: decoded[pos + 16..end].to_vec(),
                sha256: identity,
                decoded_sha256: decoded_identity,
            });
        }
        pos = end;
    }
    bail!("Firmware has no RTOS section")
}

/// Reads the still-path Standard base gamma (256 little-endian u16 of 14-bit
/// output) and rejects a table without its known shape.
pub fn standard_base_gamma(image: &FirmwareImage) -> Result<[u16; 256]> {
    let address = image
        .target
        .image_control
        .with_context(|| format!("{} has no Image Control", image.target.firmware))?
        .base_gamma;
    let bytes = image.read(address, 512)?;
    let table: [u16; 256] =
        std::array::from_fn(|i| u16::from_le_bytes([bytes[i * 2], bytes[i * 2 + 1]]));
    ensure!(
        table[0] == 0
            && table.windows(2).all(|pair| pair[0] <= pair[1])
            && table[8] == 7552
            && (16000..=16384).contains(&table[255]),
        "Unexpected Standard base gamma at {address:#x}"
    );
    Ok(table)
}
