use crate::target::{self, Target};
use anyhow::{ensure, Context, Result};
use sha2::{Digest, Sha256};
use std::path::Path;

pub use grkit_core::firmware::{decode, rtos_section};

pub fn sha256(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
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

/// Reads the firmware file of `target` and returns its RTOS image. The file of
/// a `Source::Dump` target is the RTOS image.
pub fn verify_firmware(path: &Path, target: &'static Target) -> Result<FirmwareImage> {
    let data = std::fs::read(path).with_context(|| format!("Read firmware {}", path.display()))?;
    let identity = sha256(&data);
    ensure!(
        identity == target.firmware_sha256,
        "Firmware SHA-256 does not match {}",
        target.firmware
    );
    if target.download.source == target::Source::Dump {
        ensure!(
            identity == target.decoded_sha256,
            "Decoded firmware SHA-256 mismatch"
        );
        return Ok(FirmwareImage {
            target,
            body: data,
            sha256: identity.clone(),
            decoded_sha256: identity,
        });
    }
    let decoded = decode(&data)?;
    let decoded_identity = sha256(&decoded);
    ensure!(
        decoded_identity == target.decoded_sha256,
        "Decoded firmware SHA-256 mismatch"
    );
    Ok(FirmwareImage {
        target,
        body: rtos_section(&decoded)?.to_vec(),
        sha256: identity,
        decoded_sha256: decoded_identity,
    })
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
