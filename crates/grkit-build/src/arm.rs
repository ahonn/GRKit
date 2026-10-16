use anyhow::{ensure, Context, Result};
use std::collections::BTreeMap;

pub fn branch(source: u32, target: u32, condition: u8) -> Result<[u8; 4]> {
    let displacement = i64::from(target) - i64::from(source) - 8;
    ensure!(
        condition <= 15 && displacement % 4 == 0 && (-(1 << 25)..(1 << 25)).contains(&displacement),
        "ARM branch target is out of range"
    );
    Ok(
        ((u32::from(condition) << 28) | 0x0a000000 | ((displacement / 4) as u32 & 0xffffff))
            .to_le_bytes(),
    )
}

fn slice(bytes: &[u8], start: usize, len: usize) -> Result<&[u8]> {
    bytes
        .get(start..start.checked_add(len).context("ELF range overflow")?)
        .context("Truncated ARM ELF32")
}
fn u16_at(bytes: &[u8], start: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(slice(bytes, start, 2)?.try_into()?))
}
fn u32_at(bytes: &[u8], start: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(slice(bytes, start, 4)?.try_into()?))
}
fn string(bytes: &[u8], start: usize) -> Result<&str> {
    let rest = bytes.get(start..).context("Invalid ELF string offset")?;
    let end = rest
        .iter()
        .position(|byte| *byte == 0)
        .context("Unterminated ELF string")?;
    Ok(std::str::from_utf8(&rest[..end])?)
}

/// Extract text only after validating ELF ranges and rejecting text relocations.
pub fn elf_text(blob: &[u8]) -> Result<(Vec<u8>, BTreeMap<String, u32>)> {
    ensure!(
        blob.get(..7) == Some(b"\x7fELF\x01\x01\x01") && u16_at(blob, 18)? == 40,
        "Expected little-endian ARM ELF32"
    );
    let offset = u32_at(blob, 32)? as usize;
    let size = u16_at(blob, 46)? as usize;
    let count = u16_at(blob, 48)? as usize;
    let strings = u16_at(blob, 50)? as usize;
    ensure!(size >= 40 && count != 0, "Invalid ELF section table");
    slice(blob, offset, count * size)?;
    let mut sections = Vec::with_capacity(count);
    for i in 0..count {
        let mut section = [0u32; 10];
        for (j, field) in section.iter_mut().enumerate() {
            *field = u32_at(blob, offset + i * size + j * 4)?;
        }
        sections.push(section);
    }
    let content = |section: &[u32; 10]| slice(blob, section[4] as usize, section[5] as usize);
    let names = content(
        sections
            .get(strings)
            .context("Invalid ELF section-name table")?,
    )?;
    let mut text_index = None;
    for (index, section) in sections.iter().enumerate() {
        if string(names, section[0] as usize)? == ".text" {
            text_index = Some(index);
            break;
        }
    }
    let text_index = text_index.context("ARM ELF has no .text section")?;
    ensure!(
        !sections
            .iter()
            .any(|s| matches!(s[1], 4 | 9) && s[7] as usize == text_index && s[5] != 0),
        "Unresolved text relocations"
    );
    let mut symbols = BTreeMap::new();
    for section in &sections {
        if section[1] != 2 {
            continue;
        }
        let table = content(
            sections
                .get(section[6] as usize)
                .context("Invalid ELF symbol string table")?,
        )?;
        let entries = content(section)?;
        let stride = section[9] as usize;
        ensure!(
            stride >= 16 && entries.len() % stride == 0,
            "Invalid ELF symbol entry size"
        );
        for entry in entries.chunks_exact(stride) {
            let name = u32_at(entry, 0)? as usize;
            if name != 0 && u16_at(entry, 14)? as usize == text_index {
                symbols.insert(string(table, name)?.to_owned(), u32_at(entry, 4)?);
            }
        }
    }
    Ok((content(&sections[text_index])?.to_vec(), symbols))
}
