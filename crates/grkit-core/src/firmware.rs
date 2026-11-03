//! The container of an official firmware file: its framing and its RTOS section. The
//! CLI reads GR III and GR IV files with it, and the site reads a GR IV file that the user
//! chose (`gr4_looks`).
use anyhow::{bail, ensure, Context, Result};

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

/// The RTOS image of a decoded firmware file.
pub fn rtos_section(decoded: &[u8]) -> Result<&[u8]> {
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
            return Ok(&decoded[pos + 16..end]);
        }
        pos = end;
    }
    bail!("Firmware has no RTOS section")
}
