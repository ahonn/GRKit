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
use serde_json::{json, Value};

use crate::cover::Cover;
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

impl Import {
    /// The ID of the effect.
    pub fn id(&self) -> Result<String> {
        Ok(effects::decode(&self.effect)?.0.id)
    }

    /// The `cover` of the details, or None when it is absent or invalid.
    pub fn cover(&self) -> Option<Cover> {
        let details: Value = serde_json::from_str(&self.details).ok()?;
        Cover::from_json(details.get("cover")?).ok()
    }
}

/// The import of the effect `id`.
pub fn find<'a>(imports: &'a [Import], id: &str) -> Option<&'a Import> {
    imports
        .iter()
        .find(|import| import.id().is_ok_and(|other| other == id))
}

/// Puts `effect` in the library with `cover`, which must draw its icon. An import of the same
/// effect ID gets the new effect and the cover, and keeps its other details. A new import
/// goes last with only the cover, and with `"kind": "cover"` for an `official` Look: the site
/// keeps the custom cover of an official Look so.
pub fn upsert(
    imports: &mut Vec<Import>,
    effect: Vec<u8>,
    cover: &Cover,
    official: bool,
) -> Result<()> {
    let (decoded, icon) = effects::decode(&effect)?;
    ensure!(
        cover.matches(&icon),
        "The cover does not draw the icon of {}",
        decoded.id
    );
    let at = imports
        .iter()
        .position(|import| import.id().is_ok_and(|id| id == decoded.id));
    let mut details = match at {
        Some(at) => serde_json::from_str(&imports[at].details)
            .context("The details of an import are not JSON")?,
        None if official => json!({"kind": "cover"}),
        None => json!({}),
    };
    details
        .as_object_mut()
        .context("The details of an import are not a JSON object")?
        .insert("cover".into(), cover.to_json());
    let import = Import {
        details: details.to_string(),
        effect,
    };
    place(imports, at, import)
}

/// Records the changes of an official Look in its import: `{"kind": "cover"}` with `cover`,
/// which must draw the icon of `effect`, when it is an individual cover, and the name of
/// `effect` when `renamed`. A variant of `statement`, the statement cover of the Look, is not
/// an individual cover: the card shows its scheme (`cover::Scheme`). An existing
/// import gets the new effect and keeps its other details. Without a cover and a name, the
/// import goes away. It fails when nothing changes: no import to remove.
pub fn record_official(
    imports: &mut Vec<Import>,
    effect: Vec<u8>,
    cover: Option<&Cover>,
    statement: Option<&Cover>,
    renamed: bool,
) -> Result<()> {
    let (decoded, icon) = effects::decode(&effect)?;
    let cover = cover
        .filter(|cover| statement.is_none_or(|statement| statement.variant_of(cover).is_none()));
    if cover.is_none() && !renamed {
        ensure!(
            remove(imports, &decoded.id).is_some(),
            "The Look has no import"
        );
        return Ok(());
    }
    let at = imports
        .iter()
        .position(|import| import.id().is_ok_and(|id| id == decoded.id));
    let mut details = match at {
        Some(at) => serde_json::from_str(&imports[at].details)
            .context("The details of an import are not JSON")?,
        None => json!({"kind": "cover"}),
    };
    let fields = details
        .as_object_mut()
        .context("The details of an import are not a JSON object")?;
    match cover {
        Some(cover) => {
            ensure!(
                cover.matches(&icon),
                "The cover does not draw the icon of {}",
                decoded.id
            );
            fields.insert("cover".into(), cover.to_json())
        }
        None => fields.remove("cover"),
    };
    match renamed {
        true => fields.insert("name".into(), decoded.name.into()),
        false => fields.remove("name"),
    };
    let import = Import {
        details: details.to_string(),
        effect,
    };
    place(imports, at, import)
}

/// Puts `import` at `at`, or last as a new import.
fn place(imports: &mut Vec<Import>, at: Option<usize>, import: Import) -> Result<()> {
    check(&import)?;
    match at {
        Some(at) => imports[at] = import,
        None => {
            ensure!(
                imports.len() < MAX_IMPORTS,
                "An import library holds at most {MAX_IMPORTS} imports"
            );
            imports.push(import);
        }
    }
    Ok(())
}

/// Removes the import of the effect `id`, and returns it.
pub fn remove(imports: &mut Vec<Import>, id: &str) -> Option<Import> {
    let at = imports
        .iter()
        .position(|import| import.id().is_ok_and(|other| other == id))?;
    Some(imports.remove(at))
}

/// The library file of `imports`, or None without an import: the site writes no file then,
/// and a package without imports has no `card::IMPORTS_FILE`. A writer removes the file.
pub fn library_file(imports: &[Import]) -> Result<Option<Vec<u8>>> {
    if imports.is_empty() {
        return Ok(None);
    }
    encode(imports).map(Some)
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
