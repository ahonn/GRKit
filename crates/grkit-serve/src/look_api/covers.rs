//! The covers of the Looks. A GRF holds only the pixels of its icon. The
//! parameters that draw it travel with the Look in the import library of the
//! card (`card::IMPORTS_FILE`), and the signed release statement keeps the
//! original cover of each official Look. A cover shows only while it draws
//! the icon of the file on the card.
//!
//! The service reads the library at most once, when a request first needs
//! it, and then keeps it: nothing else writes the card while the camera is
//! on. A library that cannot be read stays so for this start. The service
//! then shows none of its covers and never writes over it, because the file
//! can hold the imports of a browser.
use super::{write_checked, Look};
use crate::looks::LookFile;
use anyhow::{ensure, Context};
use grkit_core::{
    card,
    cover::{self, Cover, Scheme, Variant},
    effects,
    imports::{self, Import},
    packaging,
    release::Statement,
};
use std::{collections::BTreeMap, fs, io::Read, path::Path};

/// The largest library: each import at its largest, with its two lengths.
const MAX_LIBRARY: usize = imports::MAGIC.len()
    + 4
    + imports::MAX_IMPORTS * (8 + imports::MAX_DETAILS + effects::SIZE_WITH_LEVELS);

/// What the signed release statement says of the official Looks.
#[derive(Clone, Default)]
pub(super) struct Official {
    /// The SHA-256 of each official Look file, by ID.
    pub looks: BTreeMap<String, String>,
    /// The `effects::body_hash` of each official Look. A statement before
    /// covers has none, and its official Looks then keep their names.
    bodies: BTreeMap<String, String>,
    /// The original cover of each official Look that has one.
    covers: BTreeMap<String, Cover>,
}

impl Official {
    pub fn of(statement: Statement) -> Self {
        Self {
            looks: statement.official_looks,
            bodies: statement.official_look_bodies,
            covers: statement
                .official_covers
                .into_iter()
                .filter_map(|(id, cover)| Some((id, Cover::from_json(&cover).ok()?)))
                .collect(),
        }
    }

    /// True when `bytes` is the official Look `id` with any name and cover.
    pub fn body(&self, id: &str, bytes: &[u8]) -> bool {
        self.bodies
            .get(id)
            .is_some_and(|body| effects::body_hash(bytes).is_ok_and(|hash| hash == *body))
    }

    /// True when the official Look `id` stays official with another name.
    pub fn renamable(&self, id: &str) -> bool {
        self.bodies.contains_key(id)
    }

    /// True when `bytes` is the official Look `id` with a name and a cover
    /// that the service can show: the official file, or the official body
    /// with `cover` or with the icon of a variant of the statement cover.
    pub fn verified(&self, id: &str, bytes: &[u8], icon: &[u8], cover: bool) -> bool {
        let hash = packaging::sha256(bytes);
        self.looks.get(id).is_some_and(|expected| *expected == hash)
            || (self.body(id, bytes) && (cover || self.variant_of_icon(id, icon).is_some()))
    }

    /// The variant of the statement cover of the official Look `id` that
    /// draws `icon`.
    fn variant_of_icon(&self, id: &str, icon: &[u8]) -> Option<Variant> {
        self.covers.get(id)?.variant_of_icon(icon)
    }

    /// True when `bytes` has the name of the official file `id`: the file
    /// with the icon of the statement cover, or with its own icon, is the
    /// official file.
    fn official_name(&self, id: &str, bytes: &[u8]) -> bool {
        let (Some(expected), Ok((effect, icon))) = (self.looks.get(id), effects::decode(bytes))
        else {
            return false;
        };
        let statement = self.covers.get(id).and_then(|cover| cover.pixels().ok());
        statement.into_iter().chain([icon]).any(|icon| {
            effect
                .encode(&icon)
                .is_ok_and(|file| packaging::sha256(&file) == *expected)
        })
    }

    pub fn source(&self, file: &LookFile) -> &'static str {
        match self.looks.get(&file.id) {
            None => "custom",
            Some(hash) if *hash == file.content_hash || self.body(&file.id, &file.bytes) => {
                "official"
            }
            // An official ID with another body: not a Look that the phone made.
            Some(_) => "unknown",
        }
    }
}

#[derive(Default)]
enum Library {
    #[default]
    Unread,
    Ready(Loaded),
    /// Damaged, too large, or not readable.
    Unusable,
}

/// The imports of the card, and the cover of each by Look ID.
struct Loaded {
    imports: Vec<Import>,
    covers: BTreeMap<String, Cover>,
}

impl Loaded {
    fn new(imports: Vec<Import>) -> Self {
        let covers = imports
            .iter()
            .filter_map(|import| Some((import.id().ok()?, import.cover()?)))
            .collect();
        Self { imports, covers }
    }
}

/// A library to write: the imports and their file.
pub(super) struct Staged {
    id: String,
    imports: Vec<Import>,
    bytes: Vec<u8>,
}

#[derive(Default)]
pub(super) struct Covers {
    pub official: Official,
    library: Library,
    /// The cover of each Look by ID, with the content hash that it is for,
    /// and how a built-in Look counts for the scheme (`vote`). Drawing a cover
    /// to compare it costs time on the camera.
    resolved: BTreeMap<String, Resolved>,
    /// The scheme that the phone last chose in this start. A group without
    /// built-in Looks that count follows it (`Scheme::of_or`).
    pub chosen: Option<Scheme>,
}

struct Resolved {
    hash: String,
    cover: Option<Cover>,
    monochrome: bool,
    /// The library records a cover that is no variant of the base cover.
    individual: bool,
    vote: Option<(bool, Variant)>,
}

/// True for a built-in Look: an official Look, or a native look of the
/// camera (`cover::native_cover`). Its cover follows the scheme of the card
/// unless it has an individual cover.
pub(super) fn built_in(source: &str) -> bool {
    matches!(source, "official" | "native")
}

impl Covers {
    pub fn new(official: Official) -> Self {
        Self {
            official,
            ..Default::default()
        }
    }

    /// Set the cover of each Look.
    pub fn resolve(&mut self, root: &Path, looks: &mut [Look]) {
        let mut resolved = BTreeMap::new();
        for look in looks {
            let entry = match self.resolved.remove(&look.file.id) {
                Some(entry) if entry.hash == look.file.content_hash => entry,
                _ => {
                    let base = self.base(&look.file.id, look.source);
                    let cover = self.find(root, &look.file, base.as_ref());
                    // A cover that the library records and that is no
                    // variant is individual. An icon that no cover draws
                    // exactly, such as one of another build of the drawing,
                    // counts by the nearer variant.
                    let individual = base
                        .as_ref()
                        .zip(cover.as_ref())
                        .is_some_and(|(base, cover)| base.variant_of(cover).is_none());
                    let decoded = effects::decode(&look.file.bytes).ok();
                    // A native look is a color look.
                    let monochrome = look.source == "official"
                        && decoded
                            .as_ref()
                            .is_some_and(|(effect, _)| effect.monochrome());
                    // How a built-in Look without an individual cover counts
                    // for the scheme of the card (`Scheme::of`). A Look whose
                    // film box is gray has one cover for each scheme.
                    let vote = base
                        .filter(|base| {
                            !individual
                                && base.variant_of(&base.film_box()) == Some(Variant::FilmBox)
                        })
                        .zip(decoded)
                        .map(|(base, (_, icon))| (monochrome, base.nearest_variant(&icon)));
                    Resolved {
                        hash: look.file.content_hash.clone(),
                        cover,
                        monochrome,
                        individual,
                        vote,
                    }
                }
            };
            look.cover.clone_from(&entry.cover);
            resolved.insert(look.file.id.clone(), entry);
        }
        self.resolved = resolved;
    }

    /// The scheme of the built-in Looks on the card (`resolve`).
    pub fn scheme(&self) -> Scheme {
        Scheme::of_or(
            self.resolved.values().filter_map(|entry| entry.vote),
            self.chosen.unwrap_or_default(),
        )
    }

    /// The base cover of a built-in Look, whose variants follow the scheme:
    /// the statement cover of an official Look, the cover of a native look.
    pub fn base(&self, id: &str, source: &str) -> Option<Cover> {
        match source {
            "official" => self.official.covers.get(id).cloned(),
            "native" => cover::native_cover(id),
            _ => None,
        }
    }

    /// The cover that `scheme` gives the built-in Look `id` (`resolve`).
    pub fn default_cover(&self, id: &str, source: &str, scheme: Scheme) -> Option<Cover> {
        let base = self.base(id, source)?;
        Some(scheme.cover(&base, self.resolved.get(id)?.monochrome))
    }

    /// Whether the built-in Look `id` has an individual cover, and whether it
    /// is monochrome (`resolve`).
    pub fn kind(&self, id: &str) -> Option<(bool, bool)> {
        self.resolved
            .get(id)
            .map(|entry| (entry.individual, entry.monochrome))
    }

    /// The cover of the import of `file`, else for a built-in Look the base
    /// cover or its film box cover, when it draws the icon of the file.
    fn find(&mut self, root: &Path, file: &LookFile, base: Option<&Cover>) -> Option<Cover> {
        let library = match self.library(root) {
            Library::Ready(loaded) => loaded.covers.get(&file.id).cloned(),
            _ => None,
        };
        let statement = base
            .map(|cover| [cover.clone(), cover.film_box()])
            .into_iter()
            .flatten();
        let (_, icon) = effects::decode(&file.bytes).ok()?;
        library
            .into_iter()
            .chain(statement)
            .find(|cover| cover.matches(&icon))
    }

    fn library(&mut self, root: &Path) -> &Library {
        if matches!(self.library, Library::Unread) {
            self.library = read(root);
        }
        &self.library
    }

    /// The library after `change` to the import of the Look `id`, or None
    /// when the library is unusable or cannot take the change.
    pub fn stage(
        &mut self,
        root: &Path,
        id: &str,
        change: impl FnOnce(&mut Vec<Import>) -> anyhow::Result<()>,
    ) -> Option<Staged> {
        let Library::Ready(loaded) = self.library(root) else {
            return None;
        };
        let mut imports = loaded.imports.clone();
        change(&mut imports).ok()?;
        let bytes = imports::encode(&imports).ok()?;
        Some(Staged {
            id: id.to_owned(),
            imports,
            bytes,
        })
    }

    /// Write a staged library. On a failure the service keeps the library
    /// that it had, and writes it whole at the next change.
    pub fn commit(&mut self, root: &Path, staged: Staged) -> bool {
        if !write_checked(&root.join(card::IMPORTS_FILE), &staged.bytes) {
            return false;
        }
        self.library = Library::Ready(Loaded::new(staged.imports));
        self.resolved.remove(&staged.id);
        true
    }

    /// The library after a change of the official Look `id` to `effect`,
    /// whose icon `cover` draws, when known. The import keeps the cover
    /// unless it is a variant of the statement cover, and the name unless it is the official
    /// name; without either it goes away. Ok(None) when the library does
    /// not change, and Err when it must change but cannot.
    pub fn stage_official(
        &mut self,
        root: &Path,
        id: &str,
        effect: &[u8],
        cover: Option<&Cover>,
    ) -> Result<Option<Staged>, ()> {
        let statement = self.official.covers.get(id).cloned();
        let cover = cover.filter(|cover| {
            statement
                .as_ref()
                .is_none_or(|statement| statement.variant_of(cover).is_none())
        });
        let renamed = !self.official.official_name(id, effect);
        let staged = self.stage(root, id, |imports| {
            imports::record_official(imports, effect.to_vec(), cover, statement.as_ref(), renamed)
        });
        match staged {
            None if cover.is_some() || renamed => Err(()),
            staged => Ok(staged),
        }
    }

    /// The library after a change of the native look `id` to `effect`, whose
    /// icon `cover` draws. As the site does, the import of a native look
    /// keeps its cover, also a variant, which then is no individual cover.
    /// Only an existing import changes: it holds what the site needs to
    /// restore the look. Ok(None) without an import or a cover.
    pub fn stage_native(
        &mut self,
        root: &Path,
        id: &str,
        effect: &[u8],
        cover: Option<&Cover>,
    ) -> Result<Option<Staged>, ()> {
        let Some(cover) = cover else {
            return Ok(None);
        };
        let exists = match self.library(root) {
            Library::Ready(loaded) => imports::find(&loaded.imports, id).is_some(),
            _ => return Err(()),
        };
        if !exists {
            return Ok(None);
        }
        self.stage(root, id, |imports| {
            imports::upsert(imports, effect.to_vec(), cover, false)
        })
        .map(Some)
        .ok_or(())
    }

    /// Give the import of a renamed Look its new file, with `redrawn` or its
    /// own cover. A Look without an import gets none.
    pub fn renamed(&mut self, root: &Path, id: &str, effect: Vec<u8>, redrawn: Option<Cover>) {
        let staged = self.stage(root, id, |imports| {
            let import = imports::find(imports, id).context("The Look has no import")?;
            let cover = match redrawn {
                Some(cover) => cover,
                None => import.cover().context("The import has no cover")?,
            };
            imports::upsert(imports, effect, &cover, false)
        });
        if let Some(staged) = staged {
            self.commit(root, staged);
        }
    }

    /// Remove the import of a removed Look. An import that stays is
    /// harmless: no file has its ID.
    pub fn removed(&mut self, root: &Path, id: &str) {
        let staged = self.stage(root, id, |imports| {
            ensure!(
                imports::remove(imports, id).is_some(),
                "The Look has no import"
            );
            Ok(())
        });
        if let Some(staged) = staged {
            self.commit(root, staged);
        }
    }
}

/// Read the library of the card. A card without one has an empty library.
fn read(root: &Path) -> Library {
    let path = root.join(card::IMPORTS_FILE);
    let read = || -> std::io::Result<Option<Vec<u8>>> {
        let file = match fs::File::open(&path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            file => file?,
        };
        if !file.metadata()?.is_file() {
            return Err(std::io::ErrorKind::InvalidData.into());
        }
        let mut bytes = Vec::new();
        file.take(MAX_LIBRARY as u64 + 1).read_to_end(&mut bytes)?;
        Ok(Some(bytes))
    };
    match read() {
        Ok(None) => Library::Ready(Loaded::new(Vec::new())),
        Ok(Some(bytes)) if bytes.len() <= MAX_LIBRARY => imports::decode(&bytes)
            .map_or(Library::Unusable, |imports| {
                Library::Ready(Loaded::new(imports))
            }),
        _ => Library::Unusable,
    }
}
