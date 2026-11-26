//! What the card holds: the installed package and the hashes of its files,
//! which the card keeps in memory while its files stay the same.

use super::{card::Card, record::software_owned, CardNotReady};
use anyhow::{Context, Result};
use grkit_core::{
    card::{PACKAGE_LIST, SCAN_FILE, STARTUP_FILE},
    packaging::{self, PackageList},
    runtime, Files,
};
use serde_json::{json, Value};
use std::{collections::BTreeMap, fs, sync::Arc, time::SystemTime};

/// The name, size, and modification time of each file that `live_files`
/// reads.
type Stamp = Vec<(String, u64, Option<SystemTime>)>;

/// The summary of the card at the stamp of its files.
pub(super) type CachedSummary = (Stamp, Arc<Summary>);

/// What `Card::summary` derives from the package files.
pub(super) struct Summary {
    package: Value,
    hashes: Option<BTreeMap<String, String>>,
}

impl Card {
    /// Actual installed bytes, including the startup script. The content
    /// identity changes even when two builds have the same version label.
    pub fn inventory(&self) -> Result<Value> {
        let summary = self.summary(true)?;
        Ok(
            json!({"package": summary.package, "contentHashes": summary.hashes,
            "state": self.state()?}),
        )
    }

    /// The installed package: its GRKit version, the module IDs and effect
    /// file names of `PACKAGE_LIST`, and the effects that the list leaves
    /// out. An error means the package folder cannot be read.
    pub fn package(&self) -> Result<Value> {
        Ok(self.summary(false).context(CardNotReady)?.package.clone())
    }

    /// Drops the summary: the card changed, and a write can keep the size
    /// and the time of a file. The card's own writes drop it; a request
    /// that changes the card in other ways drops it after its answer.
    pub fn forget_summary(&self) {
        *self
            .summary
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
    }

    /// The package of the props and of the update resource, and with
    /// `hashes` the content hashes of the installed files, which only the
    /// update resource needs and which take every package file.
    fn summary(&self, hashes: bool) -> Result<Arc<Summary>> {
        let stamp = self.stamp()?;
        let mut cache = self
            .summary
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some((cached, summary)) = cache.as_ref() {
            if *cached == stamp && (!hashes || summary.hashes.is_some()) {
                return Ok(Arc::clone(summary));
            }
        }
        let summary = Arc::new(if hashes {
            let files = self.live_files()?;
            Summary {
                package: package_of(&files),
                hashes: Some(
                    files
                        .iter()
                        .filter(|(path, _)| software_owned(path))
                        .map(|(path, bytes)| (path.clone(), packaging::sha256(bytes)))
                        .collect(),
                ),
            }
        } else {
            Summary {
                package: package_of(&self.package_identity()),
                hashes: None,
            }
        });
        *cache = Some((stamp, Arc::clone(&summary)));
        Ok(summary)
    }

    /// The files that the package reads: the list, SCAN.SH, and the core
    /// file that SCAN.SH runs.
    fn package_identity(&self) -> Files {
        let mut files = Files::new();
        for path in [PACKAGE_LIST, SCAN_FILE] {
            if let Ok(bytes) = fs::read(self.path(path)) {
                files.insert(path.to_owned(), bytes);
            }
        }
        if let Some(core) = files
            .get(SCAN_FILE)
            .and_then(|scan| runtime::scanned_core(scan))
        {
            if let Ok(bytes) = fs::read(self.path(&core)) {
                files.insert(core, bytes);
            }
        }
        files
    }

    /// The stamp of the files that `live_files` reads, without reading a file.
    fn stamp(&self) -> Result<Stamp> {
        let mut stamp = Stamp::new();
        self.walk_package(|path, entry| {
            let metadata = entry.metadata()?;
            stamp.push((path, metadata.len(), metadata.modified().ok()));
            Ok(())
        })?;
        if let Ok(metadata) = fs::metadata(self.path(STARTUP_FILE)) {
            stamp.push((
                STARTUP_FILE.to_owned(),
                metadata.len(),
                metadata.modified().ok(),
            ));
        }
        stamp.sort();
        Ok(stamp)
    }
}

/// The package of the package files `files`.
fn package_of(files: &Files) -> Value {
    let list = files
        .get(PACKAGE_LIST)
        .map(|bytes| PackageList::parse(bytes))
        .unwrap_or_default();
    json!({
        "version": version_of(files),
        "modules": list.modules().collect::<Vec<_>>(),
        "effects": list.effects().collect::<Vec<_>>(),
        "left_out": list.left_out,
    })
}

/// The GRKit version that the core files of the package files `files` name.
pub(super) fn version_of(files: &Files) -> Option<String> {
    // Only the core file that the package's SCAN.SH runs: a core file left
    // on the card by an earlier package names that package's version.
    let core = runtime::scanned_core(files.get(SCAN_FILE)?)?;
    packaging::marked_version(files.get(&core)?).map(str::to_owned)
}
