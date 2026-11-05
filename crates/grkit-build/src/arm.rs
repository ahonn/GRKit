use anyhow::{ensure, Context, Result};
use std::collections::BTreeMap;

pub mod decode;
pub use decode::{branch_target, modified_immediate, move_immediate};

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

/// The sections of an ARM ELF32 file: the header fields of each, with its index.
struct Elf<'a> {
    blob: &'a [u8],
    sections: Vec<[u32; 10]>,
    names: &'a [u8],
}

impl<'a> Elf<'a> {
    fn parse(blob: &'a [u8]) -> Result<Self> {
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
        let names = slice(
            blob,
            sections
                .get(strings)
                .context("Invalid ELF section-name table")?[4] as usize,
            sections[strings][5] as usize,
        )?;
        Ok(Self {
            blob,
            sections,
            names,
        })
    }

    fn content(&self, section: &[u32; 10]) -> Result<&'a [u8]> {
        slice(self.blob, section[4] as usize, section[5] as usize)
    }

    fn name(&self, section: &[u32; 10]) -> Result<&'a str> {
        string(self.names, section[0] as usize)
    }

    fn text(&self) -> Result<usize> {
        for (index, section) in self.sections.iter().enumerate() {
            if self.name(section)? == ".text" {
                return Ok(index);
            }
        }
        anyhow::bail!("ARM ELF has no .text section")
    }

    /// Each symbol table entry: name, value, and section index.
    fn symbols(&self, table: usize) -> Result<Vec<(&'a str, u32, u16)>> {
        let section = self
            .sections
            .get(table)
            .context("Invalid ELF symbol table")?;
        let names = self.content(
            self.sections
                .get(section[6] as usize)
                .context("Invalid ELF symbol string table")?,
        )?;
        let entries = self.content(section)?;
        let stride = section[9] as usize;
        ensure!(
            stride >= 16 && entries.len() % stride == 0,
            "Invalid ELF symbol entry size"
        );
        entries
            .chunks_exact(stride)
            .map(|entry| {
                Ok((
                    string(names, u32_at(entry, 0)? as usize)?,
                    u32_at(entry, 4)?,
                    u16_at(entry, 14)?,
                ))
            })
            .collect()
    }
}

/// A relocation that `--emit-relocs` keeps in the linked private code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextRelocation {
    /// The byte offset in `.text`.
    pub offset: u32,
    /// The ELF relocation type (`R_ARM_*`).
    pub kind: u8,
    /// The symbol name. A section symbol has the name of its section.
    pub symbol: String,
    pub value: u32,
    /// The symbol lies in the private code.
    pub internal: bool,
}

/// Extract text only after validating ELF ranges and rejecting text relocations.
pub fn elf_text(blob: &[u8]) -> Result<(Vec<u8>, BTreeMap<String, u32>)> {
    let elf = Elf::parse(blob)?;
    let text_index = elf.text()?;
    ensure!(
        !elf.sections
            .iter()
            .any(|s| matches!(s[1], 4 | 9) && s[7] as usize == text_index && s[5] != 0),
        "Unresolved text relocations"
    );
    let (code, symbols, _) = elf_text_relocations(blob)?;
    Ok((code, symbols))
}

/// The text of a linked file, its symbols, and its kept relocations.
pub type LinkedText = (Vec<u8>, BTreeMap<String, u32>, Vec<TextRelocation>);

/// Extract text, its symbols, and the relocations that a link with
/// `--emit-relocs` keeps for it.
pub fn elf_text_relocations(blob: &[u8]) -> Result<LinkedText> {
    let elf = Elf::parse(blob)?;
    let text_index = elf.text()?;
    // The private code is .text only (with .rodata linked into it). Static
    // variables in .data or .bss would be dropped without this check.
    for section in &elf.sections {
        let name = elf.name(section)?;
        ensure!(
            section[2] & 2 == 0 || section[5] == 0 || name == ".text",
            "Section {name} is not part of the private code: use no writable static data"
        );
    }
    let text = elf.sections[text_index];
    let mut symbols = BTreeMap::new();
    for (index, section) in elf.sections.iter().enumerate() {
        if section[1] != 2 {
            continue;
        }
        for (name, value, shndx) in elf.symbols(index)? {
            if !name.is_empty() && shndx as usize == text_index {
                symbols.insert(name.to_owned(), value);
            }
        }
    }
    let mut relocations = Vec::new();
    for section in &elf.sections {
        if !matches!(section[1], 4 | 9) || section[7] as usize != text_index || section[5] == 0 {
            continue;
        }
        ensure!(section[1] == 9, "Unexpected RELA text relocations");
        let table = elf.symbols(section[6] as usize)?;
        let entries = elf.content(section)?;
        ensure!(entries.len() % 8 == 0, "Invalid ELF relocation size");
        for entry in entries.chunks_exact(8) {
            let address = u32_at(entry, 0)?;
            let info = u32_at(entry, 4)?;
            let &(name, value, shndx) = table
                .get((info >> 8) as usize)
                .context("Invalid ELF relocation symbol")?;
            // A linked file gives addresses, an object file offsets.
            let offset = if text[3] != 0 {
                address.checked_sub(text[3])
            } else {
                Some(address)
            }
            .filter(|offset| *offset < text[5])
            .context("ELF relocation outside .text")?;
            let internal = shndx as usize == text_index;
            let name = if name.is_empty() && internal {
                ".text"
            } else {
                name
            };
            relocations.push(TextRelocation {
                offset,
                kind: info as u8,
                symbol: name.to_owned(),
                value,
                internal,
            });
        }
    }
    relocations.sort_by_key(|relocation| relocation.offset);
    Ok((elf.content(&text)?.to_vec(), symbols, relocations))
}

/// The symbols that an object file defines.
pub fn defined_symbols(blob: &[u8]) -> Result<Vec<String>> {
    let elf = Elf::parse(blob)?;
    let mut result = Vec::new();
    for (index, section) in elf.sections.iter().enumerate() {
        if section[1] == 2 {
            for (name, _, shndx) in elf.symbols(index)? {
                if !name.is_empty() && shndx != 0 {
                    result.push(name.to_owned());
                }
            }
        }
    }
    Ok(result)
}

/// The undefined symbols of an object file.
pub fn undefined_symbols(blob: &[u8]) -> Result<Vec<String>> {
    let elf = Elf::parse(blob)?;
    let mut result = Vec::new();
    for (index, section) in elf.sections.iter().enumerate() {
        if section[1] == 2 {
            for (name, _, shndx) in elf.symbols(index)? {
                if !name.is_empty() && shndx == 0 {
                    result.push(name.to_owned());
                }
            }
        }
    }
    Ok(result)
}

/// A relocation that `--emit-relocs` keeps in a linked module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleRelocation {
    /// The segment of the relocated word, and its offset there.
    pub segment: grkit_core::grm::Segment,
    pub offset: u32,
    /// The ELF relocation type (`R_ARM_*`).
    pub kind: u8,
    pub symbol: String,
    /// The segment of the symbol, or `None` for an absolute symbol.
    pub target: Option<grkit_core::grm::Segment>,
}

/// The segments of a linked module (`crate::module`), its symbols, and its
/// kept relocations.
#[derive(Debug, Clone, Default)]
pub struct LinkedModule {
    pub code: Vec<u8>,
    pub data: Vec<u8>,
    pub bss_size: u32,
    pub symbols: BTreeMap<String, (grkit_core::grm::Segment, u32)>,
    pub relocations: Vec<ModuleRelocation>,
}

/// Read a module that the module link script linked: `.text` at 0, `.data`
/// at `grm::DATA_LINK`, and `.bss` after it. Any other allocated section with
/// content is an error, because the installer would not load it.
pub fn elf_module(blob: &[u8]) -> Result<LinkedModule> {
    use grkit_core::grm::{Segment, DATA_LINK};
    let elf = Elf::parse(blob)?;
    let mut sections = BTreeMap::new();
    for (index, section) in elf.sections.iter().enumerate() {
        let name = elf.name(section)?;
        // SHF_ALLOC sections with a size.
        if section[2] & 2 == 0 || section[5] == 0 {
            continue;
        }
        let segment = match name {
            ".text" => {
                ensure!(section[3] == 0, "Module .text is not at address 0");
                Segment::Code
            }
            ".data" | ".bss" => Segment::Data,
            _ => anyhow::bail!("Module section {name} is not part of a segment"),
        };
        sections.insert(
            index,
            (name, segment, section[3], section[5], section[1] == 8),
        );
    }
    let text = *sections
        .iter()
        .find(|(_, s)| s.0 == ".text")
        .context("Module has no code")?
        .0;
    let code = elf.content(&elf.sections[text])?.to_vec();
    let mut data = Vec::new();
    let mut bss_size = 0;
    for (&index, &(name, _, address, size, nobits)) in &sections {
        match name {
            ".data" => {
                ensure!(
                    address == DATA_LINK,
                    "Module .data is not at the data link address"
                );
                data = elf.content(&elf.sections[index])?.to_vec();
            }
            ".bss" => {
                ensure!(nobits, "Module .bss has content");
                bss_size = size;
            }
            _ => {}
        }
    }
    if let Some(&(_, _, address, _, _)) = sections.values().find(|s| s.0 == ".bss") {
        ensure!(
            address == DATA_LINK + data.len() as u32,
            "Module .bss does not follow .data"
        );
    }
    let segment_of = |shndx: u16| sections.get(&(shndx as usize)).map(|s| s.1);
    let mut symbols = BTreeMap::new();
    let mut relocations = Vec::new();
    for (index, section) in elf.sections.iter().enumerate() {
        if section[1] == 2 {
            for (name, value, shndx) in elf.symbols(index)? {
                if let (false, Some(segment)) = (name.is_empty(), segment_of(shndx)) {
                    let offset = match segment {
                        Segment::Code => value,
                        Segment::Data => value - DATA_LINK,
                    };
                    symbols.insert(name.to_owned(), (segment, offset));
                }
            }
        }
        if !matches!(section[1], 4 | 9) || section[5] == 0 {
            continue;
        }
        let Some(&(target_name, segment, address, size, _)) = sections.get(&(section[7] as usize))
        else {
            continue;
        };
        ensure!(
            section[1] == 9,
            "Unexpected RELA relocations in {target_name}"
        );
        let table = elf.symbols(section[6] as usize)?;
        let entries = elf.content(section)?;
        ensure!(entries.len() % 8 == 0, "Invalid ELF relocation size");
        for entry in entries.chunks_exact(8) {
            let at = u32_at(entry, 0)?;
            let info = u32_at(entry, 4)?;
            let &(name, _, shndx) = table
                .get((info >> 8) as usize)
                .context("Invalid ELF relocation symbol")?;
            let offset = at
                .checked_sub(address)
                .filter(|offset| *offset < size)
                .with_context(|| format!("ELF relocation outside {target_name}"))?;
            let target = segment_of(shndx);
            let name = match (name.is_empty(), target) {
                (true, Some(Segment::Code)) => ".text",
                (true, Some(Segment::Data)) => ".data",
                _ => name,
            };
            relocations.push(ModuleRelocation {
                segment,
                offset,
                kind: info as u8,
                symbol: name.to_owned(),
                target,
            });
        }
    }
    relocations.sort_by_key(|r| (r.segment == Segment::Data, r.offset));
    Ok(LinkedModule {
        code,
        data,
        bss_size,
        symbols,
        relocations,
    })
}
