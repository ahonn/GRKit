use crate::target;
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
    pub body: Vec<u8>,
    pub sha256: String,
    pub decoded_sha256: String,
}

impl FirmwareImage {
    pub fn read(&self, address: u32, size: usize) -> Result<&[u8]> {
        let address =
            if (target::DATA_START..target::DATA_START + target::DATA_SIZE).contains(&address) {
                address - target::DATA_START + target::DATA_SOURCE
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

    pub fn stock_record(&self) -> Result<(u32, &[u8])> {
        let word =
            |address| -> Result<u32> { Ok(u32::from_le_bytes(self.read(address, 4)?.try_into()?)) };
        let address = word(target::TABLES[0] + 21 * 4)?;
        ensure!(
            address == 0x5501900c && word(address + 4)? == 0x53dc2754,
            "Unsupported stock processing record"
        );
        Ok((address, self.read(address, 84)?))
    }
}

pub fn verify_firmware(path: &Path) -> Result<FirmwareImage> {
    let data = std::fs::read(path).with_context(|| format!("Read firmware {}", path.display()))?;
    let identity = sha256(&data);
    ensure!(
        identity == target::FIRMWARE_SHA256,
        "Firmware SHA-256 does not match GR III 2.10"
    );
    let decoded = decode(&data)?;
    let decoded_identity = sha256(&decoded);
    ensure!(
        decoded_identity == target::DECODED_SHA256,
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
                body: decoded[pos + 16..end].to_vec(),
                sha256: identity,
                decoded_sha256: decoded_identity,
            });
        }
        pos = end;
    }
    bail!("Firmware has no RTOS section")
}

/// The still-path Standard base gamma: 256 little-endian u16 of 14-bit output.
pub const STANDARD_BASE_GAMMA: u32 = 0x53dcf7f0;

/// Reads the Standard base gamma and rejects a table without its known shape.
pub fn standard_base_gamma(image: &FirmwareImage) -> Result<[u16; 256]> {
    let bytes = image.read(STANDARD_BASE_GAMMA, 512)?;
    let table: [u16; 256] =
        std::array::from_fn(|i| u16::from_le_bytes([bytes[i * 2], bytes[i * 2 + 1]]));
    ensure!(
        table[0] == 0
            && table.windows(2).all(|pair| pair[0] <= pair[1])
            && table[8] == 7552
            && (16000..=16384).contains(&table[255]),
        "Unexpected Standard base gamma at {STANDARD_BASE_GAMMA:#x}"
    );
    Ok(table)
}
