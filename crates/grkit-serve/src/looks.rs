//! The effect files of the card. The camera lists the files of
//! `GRKIT/EFFECTS` at its start; the files of `GRKIT/STORED` stay on the
//! card only, and the camera never reads them. The files do not show what
//! the running camera loaded, selected, or enabled: a new, changed, or
//! removed file applies at the next start.
use anyhow::{ensure, Context, Result};
use grkit_core::{card::DATA_FOLDER_PATH, effects, packaging};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

/// Bound directory work, including unrelated entries, before returning a complete inventory.
pub const MAX_ENTRIES: usize = 256;
/// The camera lists this many Looks; excess files remain visible in this
/// inspection.
pub const MAX_LISTED: usize = grkit_core::abi::EFFECTS_LISTED;

/// The most entries that the scan of `GRKIT/STORED` reads.
pub const MAX_STORED_ENTRIES: usize = 512;
/// The Looks that `GRKIT/STORED` holds at most: room for the scan to
/// skip a few other files.
pub const MAX_STORED: usize = MAX_STORED_ENTRIES - 12;

/// One valid effect file of the library.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LookFile {
    /// The file name in its folder.
    pub file: String,
    pub id: String,
    pub name: String,
    pub content_hash: String,
    /// The whole file. Empty when the service did not keep it: the file is
    /// read again when a request needs it (`read_grf`).
    pub bytes: Vec<u8>,
}

/// The effect files of the card and what the scan skipped.
#[derive(Clone, Debug, Default)]
pub struct Scan {
    pub files: Vec<LookFile>,
    pub diagnostics: Vec<Value>,
}

/// The GRF files of a folder, before they are read, and the files that the
/// listing skipped.
#[derive(Debug, Default)]
pub struct Listing {
    pub entries: Vec<Entry>,
    pub diagnostics: Vec<Value>,
}

/// A GRF file of a folder, by name.
#[derive(Debug)]
pub struct Entry {
    pub name: String,
    entry: fs::DirEntry,
}

/// The length and the time of the last change of a file. Nothing else writes
/// the card while the camera is on, so a file with the same stamp still has
/// the content that an earlier read found. A write of the service in the
/// same second can keep the stamp: the service forgets what it read of a
/// file before it writes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stamp {
    length: u64,
    modified: Option<SystemTime>,
}

impl Entry {
    /// The stamp of a regular file; None for anything else.
    pub fn stamp(&self) -> Option<Stamp> {
        let metadata = self.entry.metadata().ok()?;
        metadata.is_file().then(|| Stamp {
            length: metadata.len(),
            modified: metadata.modified().ok(),
        })
    }

    /// Read and decode the file. Err is the code of the diagnostic.
    pub fn read(&self) -> std::result::Result<LookFile, &'static str> {
        let read = || -> Result<Vec<u8>> {
            ensure!(
                self.entry.file_type()?.is_file(),
                "Expected a regular GRF file"
            );
            read_open(fs::File::open(self.entry.path())?)
        };
        let bytes = read().map_err(|_| "unreadable_grf")?;
        let (effect, _) = effects::decode(&bytes).map_err(|_| "invalid_grf")?;
        Ok(LookFile {
            file: self.name.clone(),
            id: effect.id,
            name: effect.name,
            content_hash: packaging::sha256(&bytes),
            bytes,
        })
    }
}

/// Read the GRF at `path`: a regular file with a length of a GRF. Its link
/// is not followed.
pub fn read_grf(path: &Path) -> Result<Vec<u8>> {
    ensure!(
        fs::symlink_metadata(path)?.is_file(),
        "Expected a regular GRF file"
    );
    read_open(fs::File::open(path)?)
}

fn read_open(file: fs::File) -> Result<Vec<u8>> {
    let length = file.metadata()?.len();
    ensure!(
        length == effects::SIZE as u64 || length == effects::SIZE_WITH_LEVELS as u64,
        "Invalid GRF length"
    );
    Ok(grkit_core::bounded::read_to_limit(
        file,
        effects::SIZE_WITH_LEVELS,
    )?)
}

pub fn library(root: &Path) -> PathBuf {
    root.join(DATA_FOLDER_PATH).join("EFFECTS")
}

/// The folder of the Looks that stay on the card only.
pub fn stored(root: &Path) -> PathBuf {
    root.join(DATA_FOLDER_PATH).join("STORED")
}

/// Read the effect files of `GRKIT/EFFECTS`, sorted by ID then file name.
/// A missing or unreadable directory and a directory past the scan limit
/// are errors, not empty libraries. An unusable file is a diagnostic, as the
/// installer skips it (runtime/install_effects.h).
pub fn scan(root: &Path) -> Result<Scan> {
    let Listing {
        entries,
        mut diagnostics,
    } = list(&library(root), MAX_ENTRIES)?;
    let mut files = Vec::new();
    for entry in entries {
        match entry.read() {
            Ok(file) => files.push(file),
            Err(code) => diagnostics.push(json!({"file":entry.name,"code":code})),
        }
    }
    files.sort_by(|a, b| a.id.cmp(&b.id).then(a.file.cmp(&b.file)));
    Ok(Scan { files, diagnostics })
}

/// List the effect files of `GRKIT/STORED`, with the limits of `scan`. A
/// card without the folder stores no Looks.
pub fn list_stored(root: &Path) -> Result<Listing> {
    let directory = stored(root);
    match fs::symlink_metadata(&directory) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Listing::default()),
        _ => list(&directory, MAX_STORED_ENTRIES),
    }
}

/// The GRF files of `directory`, by name. The `._` companions that macOS
/// writes next to each file that it copies are no effect files
/// (runtime/catalog.h): they count against a limit of their own, one for each
/// file.
fn list(directory: &Path, limit: usize) -> Result<Listing> {
    ensure!(
        fs::symlink_metadata(directory)?.file_type().is_dir(),
        "The Looks library is not a directory"
    );
    let mut entries = Vec::new();
    let mut companions = 0;
    for entry in fs::read_dir(directory).context("Read the Looks library")? {
        let entry = entry.context("Read a Looks directory entry")?;
        let companion = entry.file_name().as_encoded_bytes().starts_with(b"._");
        let count = if companion {
            companions += 1;
            companions
        } else {
            entries.push(entry);
            entries.len()
        };
        ensure!(
            count <= limit,
            "The Looks library exceeds the inspection limit"
        );
    }
    entries.sort_by_key(|entry| entry.file_name());
    let mut listing = Listing::default();
    for entry in entries {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            listing.diagnostics.push(json!({"code":"invalid_filename"}));
            continue;
        };
        if !name.to_ascii_lowercase().ends_with(".grf") {
            continue;
        }
        if !effects::valid_filename(name) {
            listing
                .diagnostics
                .push(json!({"file":name,"code":"invalid_filename"}));
            continue;
        }
        listing.entries.push(Entry {
            name: name.to_owned(),
            entry,
        });
    }
    Ok(listing)
}

/// The file names that the package list leaves out (`-NAME.GRF`): the
/// installer does not list these files of the library
/// (runtime/install_library.h).
pub fn left_out(root: &Path) -> BTreeSet<String> {
    fs::read(root.join(grkit_core::card::PACKAGE_LIST))
        .map(|list| packaging::PackageList::parse(&list).left_out)
        .unwrap_or_default()
}

/// Inspect actual GRFs for diagnostics. `official_hashes` must come from a
/// trusted catalog, never from an unsigned on-card package list. Without
/// provenance, source is unknown. This is a disk inspection: it does not
/// show what the running camera loaded, selected, or enabled.
pub fn inventory(root: &Path, official_hashes: &BTreeSet<String>) -> Result<Value> {
    let Scan {
        files,
        mut diagnostics,
    } = scan(root)?;
    let mut looks = Vec::new();
    let mut identities: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut codes: BTreeMap<u16, BTreeSet<String>> = BTreeMap::new();
    for file in files {
        let code = base_code(&file.id);
        identities
            .entry(file.id.clone())
            .or_default()
            .push(file.file.clone());
        codes.entry(code).or_default().insert(file.id.clone());
        looks.push(json!({
            "id":file.id,"name":file.name,"file":file.file,
            "contentHash":file.content_hash,"size":file.bytes.len(),"baseEffectCode":code,
            "source":if official_hashes.contains(&file.content_hash) { "official" } else { "unknown" }
        }));
    }
    for (id, files) in &identities {
        if files.len() > 1 {
            diagnostics.push(json!({"code":"duplicate_id","id":id,"files":files}));
        }
    }
    for (code, ids) in codes {
        if ids.len() > 1 {
            diagnostics
                .push(json!({"code":"effect_code_collision","baseEffectCode":code,"ids":ids}));
        }
    }
    if identities.len() > MAX_LISTED {
        diagnostics.push(
            json!({"code":"listed_capacity_exceeded","count":identities.len(),"limit":MAX_LISTED}),
        );
    }
    Ok(
        json!({"looks":looks,"diagnostics":diagnostics,"runtimeAvailable":false,
        "limits":{"listedLooks":MAX_LISTED,"inspectionEntries":MAX_ENTRIES}}),
    )
}

// The installer's raw ID code, before its catalog-dependent collision fixup.
pub fn base_code(id: &str) -> u16 {
    let hash = id.bytes().fold(2166136261u32, |hash, byte| {
        (hash ^ u32::from(byte)).wrapping_mul(16777619)
    });
    0x8000 | (hash as u16 & 0x7fff)
}

#[cfg(test)]
mod tests {
    use super::*;
    use grkit_core::testing;

    fn library_fixture() -> (tempfile::TempDir, std::path::PathBuf) {
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join(DATA_FOLDER_PATH).join("EFFECTS");
        fs::create_dir_all(&directory).unwrap();
        (root, directory)
    }

    #[test]
    fn reads_actual_content_without_inventing_runtime_or_provenance() {
        let (root, dir) = library_fixture();
        let bytes = testing::effect("sample");
        fs::write(dir.join("SAMPLE.GRF"), &bytes).unwrap();
        let result = inventory(root.path(), &BTreeSet::new()).unwrap();
        let look = &result["looks"][0];
        assert_eq!(look["id"], "sample");
        assert_eq!(look["contentHash"], packaging::sha256(&bytes));
        assert_eq!(look["source"], "unknown");
        for field in ["enabled", "selected", "loaded"] {
            assert!(look.get(field).is_none());
        }
        let trusted = BTreeSet::from([packaging::sha256(&bytes)]);
        assert_eq!(
            inventory(root.path(), &trusted).unwrap()["looks"][0]["source"],
            "official"
        );
    }

    #[test]
    fn reports_duplicate_and_corrupt_files_without_hiding_valid_copies() {
        let (root, dir) = library_fixture();
        for name in ["ONE.GRF", "TWO.GRF"] {
            fs::write(dir.join(name), testing::effect("same")).unwrap();
        }
        fs::write(dir.join("BAD.GRF"), vec![0; effects::SIZE]).unwrap();
        fs::write(
            dir.join("LARGE.GRF"),
            vec![0; effects::SIZE_WITH_LEVELS + 1],
        )
        .unwrap();
        let result = inventory(root.path(), &BTreeSet::new()).unwrap();
        assert_eq!(result["looks"].as_array().unwrap().len(), 2);
        let codes: Vec<_> = result["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v["code"].as_str().unwrap())
            .collect();
        assert!(codes.contains(&"duplicate_id"));
        assert!(codes.contains(&"invalid_grf"));
        assert!(codes.contains(&"unreadable_grf"));
        assert!(!codes.contains(&"effect_code_collision"));
    }

    #[test]
    fn detects_different_ids_with_the_same_runtime_base_code() {
        let mut seen = BTreeMap::new();
        let (first, second) = (0..=32768)
            .find_map(|i| {
                let id = format!("look-{i}");
                seen.insert(base_code(&id), id.clone())
                    .map(|first| (first, id))
            })
            .unwrap();
        let (root, dir) = library_fixture();
        fs::write(dir.join("ONE.GRF"), testing::effect(&first)).unwrap();
        fs::write(dir.join("TWO.GRF"), testing::effect(&second)).unwrap();
        let result = inventory(root.path(), &BTreeSet::new()).unwrap();
        assert_eq!(result["diagnostics"][0]["code"], "effect_code_collision");
    }

    #[cfg(unix)]
    #[test]
    fn does_not_follow_grf_symlinks() {
        let (root, dir) = library_fixture();
        let outside = root.path().join("outside.grf");
        fs::write(&outside, testing::effect("outside")).unwrap();
        std::os::unix::fs::symlink(outside, dir.join("LINK.GRF")).unwrap();
        let result = inventory(root.path(), &BTreeSet::new()).unwrap();
        assert!(result["looks"].as_array().unwrap().is_empty());
        assert_eq!(result["diagnostics"][0]["code"], "unreadable_grf");
    }

    #[test]
    fn the_package_list_names_the_files_that_it_leaves_out() {
        let root = tempfile::tempdir().unwrap();
        assert!(left_out(root.path()).is_empty());
        let list = root.path().join(grkit_core::card::PACKAGE_LIST);
        fs::create_dir_all(list.parent().unwrap()).unwrap();
        fs::write(&list, "ONE.GRF\n-two.grf FILM.GRM\n-THREE.GRF\n").unwrap();
        assert_eq!(
            left_out(root.path()),
            BTreeSet::from(["TWO.GRF".to_owned(), "THREE.GRF".to_owned()])
        );
    }

    #[test]
    fn missing_library_and_excess_entries_are_not_empty_successes() {
        let root = tempfile::tempdir().unwrap();
        assert!(inventory(root.path(), &BTreeSet::new()).is_err());
        let (root, dir) = library_fixture();
        for i in 0..=MAX_ENTRIES {
            fs::write(dir.join(format!("{i}.txt")), b"").unwrap();
        }
        assert!(inventory(root.path(), &BTreeSet::new()).is_err());
    }

    #[test]
    fn mac_companion_files_are_no_looks_and_do_not_fill_the_limit() {
        let (root, dir) = library_fixture();
        for i in 0..MAX_ENTRIES {
            fs::write(dir.join(format!("L{i}.GRF")), b"").unwrap();
            fs::write(dir.join(format!("._L{i}.GRF")), b"").unwrap();
        }
        let scan = scan(root.path()).unwrap();
        assert_eq!(scan.diagnostics.len(), MAX_ENTRIES);
        assert!(scan
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic["code"] == "unreadable_grf"));
    }
}
