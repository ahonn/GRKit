//! The card, from its root on Linux: its package files, the journal of an
//! update, and the file operations that follow the rules of the card
//! (`super`).

use super::summary::CachedSummary;
use anyhow::{Context, Result};
use grkit_core::{
    card::{
        DRAFT_FILE, IMPORTS_FILE, JOURNAL_FILE, PACKAGE_FOLDER, REQUEST_FILE, STARTUP_FILE,
        VERSION_FILE,
    },
    release, Files,
};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Mutex,
};

/// The error of a card whose package folder cannot be read yet, as soon
/// after power-on. It is not the same as a card without an update; the
/// client can try again.
#[derive(Debug)]
pub struct CardNotReady;

impl std::fmt::Display for CardNotReady {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("The card is not ready yet. Try again.")
    }
}

impl std::error::Error for CardNotReady {}

/// Whether `error` comes from a card that is not ready (`CardNotReady`).
pub fn card_not_ready(error: &anyhow::Error) -> bool {
    error.downcast_ref::<CardNotReady>().is_some()
}

/// The state of the journal of an update on the card.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum JournalState {
    /// No update waits.
    None,
    /// An update waits for the confirmation on the camera.
    Waiting,
    /// The camera confirmed the update, which it installs at the next power-on.
    Confirmed,
}

/// Writes `bytes` over the file `path`, as the card cannot replace a file,
/// and reads them back.
pub(crate) fn write_checked(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    let mut stored = Vec::with_capacity(bytes.len());
    fs::File::open(path)?
        .take(bytes.len() as u64 + 1)
        .read_to_end(&mut stored)?;
    if stored != bytes {
        return Err(std::io::Error::other("the file differs after the write"));
    }
    Ok(())
}

/// Looks up one name that no file of `directory` has, after a delete there:
/// on the camera, the lookup of the next new name in a folder can fail after
/// a delete, and Linux keeps that answer.
pub(crate) fn look_up_unused(directory: &Path) {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |time| time.subsec_nanos() ^ time.as_secs() as u32);
    let _ = fs::metadata(directory.join(format!("{stamp:08X}.TMP")));
}

/// The card, from its root on Linux, and the keys of the releases that it
/// trusts.
pub struct Card {
    root: PathBuf,
    keys: Vec<[u8; 32]>,
    pub(super) summary: Mutex<Option<CachedSummary>>,
}

impl Card {
    /// The card at `root`, which trusts the official releases.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self::with_keys(root, release::OFFICIAL_KEYS.to_vec())
    }

    pub fn with_keys(root: impl Into<PathBuf>, keys: Vec<[u8; 32]>) -> Self {
        Card {
            root: root.into(),
            keys,
            summary: Mutex::new(None),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The keys of the releases that the card trusts.
    pub fn release_keys(&self) -> &[[u8; 32]] {
        &self.keys
    }

    pub(super) fn path(&self, card_path: &str) -> PathBuf {
        self.root.join(card_path)
    }

    /// Calls `visit` with the card path of each file of the package folder
    /// and of its folders, one level down.
    pub(super) fn walk_package(
        &self,
        mut visit: impl FnMut(String, &fs::DirEntry) -> Result<()>,
    ) -> Result<()> {
        let folder = self.path(PACKAGE_FOLDER);
        for entry in fs::read_dir(&folder).with_context(|| format!("Read {PACKAGE_FOLDER}"))? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if entry.file_type()?.is_dir() {
                for inner in fs::read_dir(entry.path())? {
                    let inner = inner?;
                    if inner.file_type()?.is_file() {
                        let inner_name = inner.file_name().to_string_lossy().into_owned();
                        visit(format!("{PACKAGE_FOLDER}/{name}/{inner_name}"), &inner)?;
                    }
                }
            } else {
                visit(format!("{PACKAGE_FOLDER}/{name}"), &entry)?;
            }
        }
        Ok(())
    }

    /// The files of the package folder on the card, at their card paths.
    pub(super) fn package_files(&self) -> Result<Files> {
        let mut files = Files::new();
        self.walk_package(|path, entry| {
            files.insert(path, fs::read(entry.path())?);
            Ok(())
        })?;
        Ok(files)
    }

    /// The package files of the card with the startup script, at their card
    /// paths.
    pub(super) fn live_files(&self) -> Result<Files> {
        let mut files = self.package_files()?;
        if let Ok(startup) = fs::read(self.path(STARTUP_FILE)) {
            files.insert(STARTUP_FILE.to_owned(), startup);
        }
        Ok(files)
    }

    /// The camera deletes an unconfirmed request on its own side. Linux then
    /// keeps the names that it created (COMMIT.REQ, COMMIT.VER) and answers
    /// an I/O error for them, and a rename onto such a name fails. Drop the
    /// kernel's cached names when one of `paths` answers so.
    pub(super) fn forget_deleted(&self, paths: &[&str]) {
        let stale = paths.iter().any(|path| {
            fs::symlink_metadata(self.path(path))
                .is_err_and(|error| error.kind() != std::io::ErrorKind::NotFound)
        });
        if stale {
            // 2: free the cached names and file nodes, not file contents.
            let _ = fs::write("/proc/sys/vm/drop_caches", "2");
        }
    }

    /// Looks up `paths`, so that the card finds them after a delete in their
    /// folders. A path that does not exist is fine.
    pub(super) fn warm<'a>(&self, paths: impl IntoIterator<Item = &'a str>) {
        for path in paths {
            let _ = fs::symlink_metadata(self.path(path));
        }
    }

    /// Deletes the files in the folder `folder` and in its folders, except
    /// the file `keep`, and keeps the folders.
    pub(super) fn clear(&self, folder: &str, keep: Option<&str>) -> Result<()> {
        let path = self.path(folder);
        if !path.exists() {
            return Ok(());
        }
        for entry in fs::read_dir(&path)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let card_path = format!("{folder}/{name}");
            if entry.file_type()?.is_dir() {
                self.clear(&card_path, keep)?;
            } else if !keep.is_some_and(|keep| keep.eq_ignore_ascii_case(&card_path)) {
                fs::remove_file(entry.path()).with_context(|| format!("Delete {card_path}"))?;
            }
        }
        self.forget_summary();
        Ok(())
    }

    /// Writes `bytes` to the card path `path`, then reads them back.
    pub(super) fn write_checked(&self, path: &str, bytes: &[u8]) -> Result<()> {
        let target = self.path(path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).with_context(|| format!("Create the folder of {path}"))?;
        }
        self.forget_summary();
        write_checked(&target, bytes).with_context(|| format!("Write {path}"))
    }

    /// Reads the package folder. Missing or an I/O error means the card is
    /// not ready yet; the client can retry.
    pub fn ensure_readable(&self) -> Result<()> {
        fs::read_dir(self.path(PACKAGE_FOLDER))
            .map(|_| ())
            .context(CardNotReady)
    }

    /// The journal files in the package folder: of `JOURNAL_FILE`,
    /// `REQUEST_FILE`, `DRAFT_FILE`, and `VERSION_FILE`, those that the
    /// folder lists. The service never looks up the journal by its name
    /// (`super`). An error means the package folder cannot be read.
    pub(super) fn journal_files(&self) -> Result<Vec<&'static str>> {
        let names: Vec<String> = fs::read_dir(self.path(PACKAGE_FOLDER))
            .context(CardNotReady)?
            .map(|entry| {
                entry.map(|entry| entry.file_name().to_string_lossy().to_ascii_uppercase())
            })
            .collect::<std::io::Result<_>>()
            .context(CardNotReady)?;
        Ok([JOURNAL_FILE, REQUEST_FILE, DRAFT_FILE, VERSION_FILE]
            .into_iter()
            .filter(|path| names.iter().any(|name| path.ends_with(&format!("/{name}"))))
            .collect())
    }

    /// The state of the journal of an update. An error means the package
    /// folder cannot be read.
    pub fn state(&self) -> Result<JournalState> {
        let files = self.journal_files()?;
        Ok(if files.contains(&JOURNAL_FILE) {
            JournalState::Confirmed
        } else if files.contains(&REQUEST_FILE) {
            JournalState::Waiting
        } else {
            JournalState::None
        })
    }

    /// Removes the journal of an update that has not been installed, and
    /// returns whether there was one. The files in the update folder stay
    /// until the next update.
    pub fn cancel(&self) -> Result<bool> {
        // The service creates these names again after the deletes, and the
        // Looks write the import library.
        self.warm([REQUEST_FILE, DRAFT_FILE, VERSION_FILE, IMPORTS_FILE]);
        let files = self.journal_files()?;
        let had = files.contains(&JOURNAL_FILE) || files.contains(&REQUEST_FILE);
        // The journal of the camera first, before a delete in the folder.
        for path in files {
            fs::remove_file(self.path(path)).with_context(|| format!("Delete {path}"))?;
        }
        Ok(had)
    }

    /// Whether the card holds a release that one of its keys signed: the
    /// statement verifies, the card has each runtime file of the statement
    /// with its hash, and each module of the card is a module of the
    /// statement. Then the software of the card is known even though no
    /// request of the service put it there.
    pub(super) fn holds_signed_release(&self, live: &Files) -> bool {
        let Ok(statement) = crate::trust::installed_release(&self.root, &self.keys) else {
            return false;
        };
        let modules = format!("{}/", grkit_core::card::MODULES_FOLDER);
        statement.runtime.iter().all(|(path, hash)| {
            live.get(path)
                .is_some_and(|bytes| grkit_core::packaging::sha256(bytes) == *hash)
        }) && live.iter().all(|(path, bytes)| {
            path.strip_prefix(&modules).is_none_or(|name| {
                statement.modules.get(name) == Some(&grkit_core::packaging::sha256(bytes))
            })
        })
    }
}
