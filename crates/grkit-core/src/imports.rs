//! The import library of the site: each effect that a browser imported from a LUT, with the
//! details that the page shows of it. A package keeps the library at `card::IMPORTS_FILE`. The
//! camera does not read it, so the library can hold more imports than a package installs, and
//! another browser restores each of them from the package or the card.
//!
//! The file is `MAGIC`, the import count (u32), then for each import: the length (u32) and the
//! UTF-8 text of a JSON object with the details, then the length (u32) and the bytes of its
//! compiled `.GRF` effect. Numbers are little-endian. The page defines the details; this module
//! only checks that they are a JSON object.

use std::collections::BTreeSet;

use anyhow::{ensure, Context, Result};
use serde_json::Value;

use crate::effects;

/// The start of an import library, with its format version.
pub const MAGIC: &[u8; 8] = b"GRKIMP01";
/// The most imports that a library holds. A browser keeps any number of imports, so the
/// limit only bounds a damaged file.
pub const MAX_IMPORTS: usize = 1024;
/// The most bytes of the details of one import.
pub const MAX_DETAILS: usize = 4096;

/// One import of the library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Import {
    /// The JSON object text of what the page shows of the import.
    pub details: String,
    /// The compiled `.GRF` effect.
    pub effect: Vec<u8>,
}

/// Checks one import: details that are a JSON object within `MAX_DETAILS`, and a complete
/// effect. Returns the effect ID.
fn check(import: &Import) -> Result<String> {
    ensure!(
        import.details.len() <= MAX_DETAILS,
        "The details of an import are larger than {MAX_DETAILS} bytes"
    );
    let details: Value =
        serde_json::from_str(&import.details).context("The details of an import are not JSON")?;
    ensure!(
        details.is_object(),
        "The details of an import are not a JSON object"
    );
    Ok(effects::decode(&import.effect)?.0.id)
}

/// The library file of `imports`. Each effect ID occurs once.
pub fn encode(imports: &[Import]) -> Result<Vec<u8>> {
    ensure!(
        imports.len() <= MAX_IMPORTS,
        "An import library holds at most {MAX_IMPORTS} imports"
    );
    let mut ids = BTreeSet::new();
    let mut bytes = MAGIC.to_vec();
    bytes.extend((imports.len() as u32).to_le_bytes());
    for import in imports {
        let id = check(import)?;
        ensure!(
            ids.insert(id.clone()),
            "The import {id} is twice in the library"
        );
        bytes.extend((import.details.len() as u32).to_le_bytes());
        bytes.extend(import.details.as_bytes());
        bytes.extend((import.effect.len() as u32).to_le_bytes());
        bytes.extend(&import.effect);
    }
    Ok(bytes)
}

/// The imports of a library file, in file order. It fails for a file that `encode` does not
/// write.
pub fn decode(bytes: &[u8]) -> Result<Vec<Import>> {
    let mut reader = Reader(
        bytes
            .strip_prefix(MAGIC.as_slice())
            .context("Not a GRKit import library")?,
    );
    let count = reader.u32()?;
    ensure!(
        count <= MAX_IMPORTS,
        "An import library holds at most {MAX_IMPORTS} imports"
    );
    let mut imports = Vec::with_capacity(count);
    let mut ids = BTreeSet::new();
    for _ in 0..count {
        let size = reader.u32()?;
        ensure!(
            size <= MAX_DETAILS,
            "The details of an import are larger than {MAX_DETAILS} bytes"
        );
        let details = String::from_utf8(reader.take(size)?.to_vec())
            .context("The details of an import are not UTF-8")?;
        let size = reader.u32()?;
        ensure!(
            size <= effects::SIZE_WITH_LEVELS,
            "An effect of the import library is too large"
        );
        let import = Import {
            details,
            effect: reader.take(size)?.to_vec(),
        };
        let id = check(&import)?;
        ensure!(
            ids.insert(id.clone()),
            "The import {id} is twice in the library"
        );
        imports.push(import);
    }
    ensure!(
        reader.0.is_empty(),
        "The import library has bytes after its last import"
    );
    Ok(imports)
}

/// The unread bytes of a library file.
struct Reader<'a>(&'a [u8]);

impl<'a> Reader<'a> {
    fn take(&mut self, size: usize) -> Result<&'a [u8]> {
        ensure!(self.0.len() >= size, "Truncated import library");
        let (head, tail) = self.0.split_at(size);
        self.0 = tail;
        Ok(head)
    }

    fn u32(&mut self) -> Result<usize> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()) as usize)
    }
}
