//! An update of the package over Wi-Fi (`grkit_core::runtime::update`).
//!
//! The service checks the package (`grkit_core::release::verify_update`):
//! an official key signed its code, and the decoders check its data. It
//! checks that the package is for this camera and its firmware. Then it writes the files that
//! change to the update folder, reads each one back, and writes the journal.
//! The camera installs the update at the next power-on.
//!
//! The card has rules that the steps follow:
//! - It cannot rename a folder or replace a file, so the service never
//!   renames over a file, and the journal moves the files.
//! - After Linux deletes a file in a folder, a new name in that folder may not
//!   be found until the next power-on (GR IV). The service looks up each name
//!   that it creates in a folder before it deletes a file there, and it never
//!   deletes a folder.
//! - Linux keeps the result of a lookup of a name that does not exist, so it
//!   does not see a file that the camera creates under that name later. The
//!   service never looks up the journal by its name, which the camera
//!   creates; it reads the folder instead (`Card::journal_files`).

use anyhow::{bail, ensure, Context, Result};
use grkit_core::{
    camera::{Camera, CAMERAS},
    card::{
        BACKUP_FOLDER, BACKUP_RECORD, DRAFT_FILE, EFFECTS_FOLDER, IMPORTS_FILE, JOURNAL_FILE,
        MODULES_FOLDER, PACKAGE_FOLDER, PACKAGE_LIST, REQUEST_FILE, SCAN_FILE, STARTUP_FILE,
        UPDATE_FOLDER, VERSION_FILE,
    },
    packaging, release, runtime,
    runtime::update::{journal, update_paths, Change},
    Files,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    path::PathBuf,
};

/// The largest package that the service takes: the package limit, and room
/// for the ZIP structure.
pub const MAX_PACKAGE: usize = packaging::MAX_TOTAL_SIZE + 8 * 1024 * 1024;
/// The free space that the card keeps after the update folder.
const SPARE_BYTES: u64 = 4 * 1024 * 1024;

/// A bounded signed-software identity. Staging metadata and user data are excluded.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SoftwareIdentity {
    pub digest: String,
    pub content_hashes: BTreeMap<String, String>,
}

impl SoftwareIdentity {
    fn of(files: &Files) -> Self {
        let content_hashes: BTreeMap<_, _> = files
            .iter()
            .filter(|(path, _)| software_owned(path))
            .map(|(path, bytes)| (path.clone(), packaging::sha256(bytes)))
            .collect();
        let digest =
            packaging::sha256(&serde_json::to_vec(&content_hashes).expect("hashes serialize"));
        Self {
            digest,
            content_hashes,
        }
    }
    fn valid(&self) -> bool {
        self.content_hashes.len() <= 1024
            && self
                .content_hashes
                .iter()
                .all(|(path, hash)| path.len() <= 192 && software_owned(path) && valid_hash(hash))
            && self.digest
                == packaging::sha256(
                    &serde_json::to_vec(&self.content_hashes).expect("hashes serialize"),
                )
    }
}

/// The kind of a software request: only an update.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SoftwareDomain {
    Update,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SoftwareStatus {
    Running,
    Succeeded,
    Cancelled,
    /// Ended without a change of the installed software; `error` says why.
    Failed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SoftwarePhase {
    AwaitingConfirmation,
    AwaitingRestart,
    Cancelling,
    Recovering,
    Completed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SoftwareRequest {
    pub request_id: String,
    pub domain: SoftwareDomain,
    pub boot_id: String,
    pub card_id: String,
    pub status: SoftwareStatus,
    pub phase: SoftwarePhase,
    pub cancellable: bool,
    pub expected: SoftwareIdentity,
    pub before: SoftwareIdentity,
    /// The code of a failed request. Absent from other checkpoints, so that
    /// their bytes and checksums stay as before.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// The camera deleted the request that waited for the confirmation on the
/// camera: nobody confirmed it in time.
pub const CONFIRMATION_EXPIRED: &str = "confirmation_expired";
/// The journal of the request did not reach the card.
pub const STAGING_FAILED: &str = "staging_failed";

#[derive(Clone, Debug)]
pub struct SoftwareContext {
    pub request_id: String,
    pub boot_id: String,
    pub card_id: String,
}
impl SoftwareContext {
    fn valid(&self) -> bool {
        crate::protocol::valid_request_id(&self.request_id)
            && valid_context_id(&self.boot_id)
            && valid_context_id(&self.card_id)
    }
}
impl SoftwareRequest {
    fn valid(&self) -> bool {
        crate::protocol::valid_request_id(&self.request_id)
            && valid_context_id(&self.boot_id)
            && valid_context_id(&self.card_id)
            && self.expected.valid()
            && self.before.valid()
            && self.error.is_some() == (self.status == SoftwareStatus::Failed)
            && self.error.as_deref().is_none_or(valid_context_id)
            && match self.status {
                SoftwareStatus::Succeeded | SoftwareStatus::Cancelled | SoftwareStatus::Failed => {
                    self.phase == SoftwarePhase::Completed && !self.cancellable
                }
                SoftwareStatus::Running => {
                    self.phase != SoftwarePhase::Completed
                        && (!self.cancellable || self.phase == SoftwarePhase::AwaitingConfirmation)
                }
            }
    }
}
fn valid_context_id(value: &str) -> bool {
    !value.is_empty() && value.len() <= 128 && value.bytes().all(|c| c.is_ascii_graphic())
}
fn valid_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
fn software_owned(path: &str) -> bool {
    path == STARTUP_FILE || package_owned(path)
}

const MAX_RECORD_BYTES: usize = 1024 * 1024;
const MAX_CHECKPOINTS: usize = 16;

/// Append-only software transaction checkpoints share RECORD.JSN. They are not
/// a generic request history. A newline commits one checksummed JSON frame.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Checkpoint {
    request: SoftwareRequest,
    sha256: String,
}
impl Checkpoint {
    fn new(request: SoftwareRequest) -> Self {
        let sha256 = packaging::sha256(&serde_json::to_vec(&request).expect("request serializes"));
        Self { request, sha256 }
    }
    fn valid(&self) -> bool {
        self.request.valid()
            && self.sha256
                == packaging::sha256(
                    &serde_json::to_vec(&self.request).expect("request serializes"),
                )
    }
}

/// What the service knows of the camera: the model and the firmware version
/// that the camera's own HTTP API gives, and the free space of the card.
pub struct Device {
    pub model: String,
    pub firmware: String,
    pub free_bytes: Option<u64>,
    pub serial_number: Option<String>,
}

/// The camera of `model`, a model name of the camera's HTTP API such as
/// `RICOH GR III`. An HDF model has the package of its camera.
fn camera_of(model: &str) -> Option<&'static Camera> {
    let name = model.strip_prefix("RICOH ").unwrap_or(model);
    CAMERAS
        .into_iter()
        .find(|camera| camera.name == name || camera.hdf == name)
}

/// The recipe family of a camera model of the camera's HTTP API.
pub fn recipe_family(model: &str) -> Option<grkit_core::recipes::Family> {
    use grkit_core::recipes::Family;
    if model.contains("Monochrome") {
        return Some(Family::Gr4Monochrome);
    }
    match camera_of(model)?.id {
        "gr3" | "gr3x" => Some(Family::Gr3),
        "gr4" => Some(Family::Gr4),
        _ => None,
    }
}

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

/// The card, from its root on Linux.
pub struct Card {
    root: PathBuf,
}

impl Card {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Card { root: root.into() }
    }

    pub(crate) fn root(&self) -> &std::path::Path {
        &self.root
    }

    fn path(&self, card_path: &str) -> PathBuf {
        self.root.join(card_path)
    }

    /// The files of the package folder on the card, at their card paths: the
    /// files of the folder and of its folders, one level down.
    fn package_files(&self) -> Result<Files> {
        let mut files = Files::new();
        let folder = self.path(PACKAGE_FOLDER);
        for entry in fs::read_dir(&folder).with_context(|| format!("Read {PACKAGE_FOLDER}"))? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if entry.file_type()?.is_dir() {
                for inner in fs::read_dir(entry.path())? {
                    let inner = inner?;
                    if inner.file_type()?.is_file() {
                        let inner_name = inner.file_name().to_string_lossy().into_owned();
                        files.insert(
                            format!("{PACKAGE_FOLDER}/{name}/{inner_name}"),
                            fs::read(inner.path())?,
                        );
                    }
                }
            } else {
                files.insert(format!("{PACKAGE_FOLDER}/{name}"), fs::read(entry.path())?);
            }
        }
        Ok(files)
    }

    /// The camera deletes an unconfirmed request on its own side. Linux then
    /// keeps the names that it created (COMMIT.REQ, COMMIT.VER) and answers
    /// an I/O error for them, and a rename onto such a name fails. Drop the
    /// kernel's cached names when one of `paths` answers so.
    fn forget_deleted(&self, paths: &[&str]) {
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
    fn warm<'a>(&self, paths: impl IntoIterator<Item = &'a str>) {
        for path in paths {
            let _ = fs::symlink_metadata(self.path(path));
        }
    }

    /// Deletes the files in the folder `folder` and in its folders, and
    /// keeps the folders.
    fn clear(&self, folder: &str) -> Result<()> {
        self.clear_except(folder, None)
    }

    /// Deletes the files as `clear` does, except the file `keep`.
    fn clear_except(&self, folder: &str, keep: Option<&str>) -> Result<()> {
        let path = self.path(folder);
        if !path.exists() {
            return Ok(());
        }
        for entry in fs::read_dir(&path)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let card_path = format!("{folder}/{name}");
            if entry.file_type()?.is_dir() {
                self.clear_except(&card_path, keep)?;
            } else if !keep.is_some_and(|keep| keep.eq_ignore_ascii_case(&card_path)) {
                fs::remove_file(entry.path()).with_context(|| format!("Delete {card_path}"))?;
            }
        }
        Ok(())
    }

    /// Writes `bytes` to `path`, then reads them back.
    fn write_checked(&self, path: &str, bytes: &[u8]) -> Result<()> {
        let target = self.path(path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).with_context(|| format!("Create the folder of {path}"))?;
        }
        let mut file = fs::File::create(&target).with_context(|| format!("Create {path}"))?;
        file.write_all(bytes)
            .with_context(|| format!("Write {path}"))?;
        file.sync_all().with_context(|| format!("Write {path}"))?;
        drop(file);
        ensure!(
            fs::read(&target).with_context(|| format!("Read back {path}"))? == bytes,
            "{path} differs after the write"
        );
        Ok(())
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
    /// folder lists. An error means the package folder cannot be read.
    fn journal_files(&self) -> Result<Vec<&'static str>> {
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

    /// The state of the update: `none`, `waiting` for the confirmation on
    /// the camera, or `confirmed` and waiting for the next power-on. An
    /// error means the package folder cannot be read.
    pub fn state(&self) -> Result<&'static str> {
        let files = self.journal_files()?;
        Ok(if files.contains(&JOURNAL_FILE) {
            "confirmed"
        } else if files.contains(&REQUEST_FILE) {
            "waiting"
        } else {
            "none"
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
}

impl Card {
    fn read_record_bytes(&self) -> Result<Option<Vec<u8>>> {
        let file = match fs::File::open(self.path(BACKUP_RECORD)) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error).context("Read the software transaction record"),
        };
        let mut bytes = Vec::new();
        file.take(MAX_RECORD_BYTES as u64 + 1)
            .read_to_end(&mut bytes)?;
        ensure!(
            bytes.len() <= MAX_RECORD_BYTES,
            "The software transaction record exceeds its size limit"
        );
        Ok(Some(bytes))
    }
    fn read_record(&self) -> Result<Option<Record>> {
        self.read_record_bytes()?
            .map(|bytes| {
                Record::parse(&bytes).context("The software transaction record is invalid")
            })
            .transpose()
    }
    fn append_request(&self, request: SoftwareRequest) -> Result<()> {
        ensure!(request.valid(), "The software request is invalid");
        let mut before = self
            .read_record_bytes()?
            .context("The software transaction record is missing")?;
        ensure!(
            Record::parse(&before).is_some(),
            "The software transaction record is invalid"
        );
        // A torn final append is uncommitted. Truncate only that tail; never
        // rewrite the complete base record or an acknowledged checkpoint.
        let complete_length = before
            .iter()
            .rposition(|byte| *byte == b'\n')
            .map(|index| index + 1)
            .context("The base software record is incomplete")?;
        before.truncate(complete_length);
        ensure!(
            before.iter().filter(|byte| **byte == b'\n').count() <= MAX_CHECKPOINTS,
            "The software checkpoint limit has been reached"
        );
        let mut frame = serde_json::to_vec(&Checkpoint::new(request))?;
        frame.push(b'\n');
        ensure!(
            before.len() + frame.len() <= MAX_RECORD_BYTES,
            "The software transaction record exceeds its size limit"
        );
        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(self.path(BACKUP_RECORD))?;
        file.set_len(complete_length as u64)?;
        file.sync_all()?;
        file.write_all(&frame)?;
        file.sync_all()?;
        drop(file);
        let after = self
            .read_record_bytes()?
            .context("The software transaction record disappeared")?;
        ensure!(
            after.len() == before.len() + frame.len()
                && after.starts_with(&before)
                && after.ends_with(&frame),
            "The software checkpoint differs after writing"
        );
        Ok(())
    }

    /// Recover the current/latest request from durable software metadata. A
    /// missing journal requires complete installed-identity verification. Once a
    /// terminal outcome is verified, append its checkpoint for later boots.
    pub fn software_request(&self) -> Result<Option<SoftwareRequest>> {
        self.ensure_readable()?;
        let Some(mut request) = self.read_record()?.and_then(|record| record.request) else {
            return Ok(None);
        };
        if request.status != SoftwareStatus::Running {
            return Ok(Some(request));
        }
        match self.state()? {
            "waiting" => {
                request.phase = SoftwarePhase::AwaitingConfirmation;
                request.cancellable = true;
            }
            "confirmed" => {
                request.phase = SoftwarePhase::AwaitingRestart;
                request.cancellable = false;
            }
            _ => {
                let actual = SoftwareIdentity::of(&self.live_files()?);
                if actual == request.expected {
                    request.status = SoftwareStatus::Succeeded;
                    request.phase = SoftwarePhase::Completed;
                } else if request.phase == SoftwarePhase::Cancelling && actual == request.before {
                    request.status = SoftwareStatus::Cancelled;
                    request.phase = SoftwarePhase::Completed;
                } else if request.phase == SoftwarePhase::AwaitingConfirmation
                    && actual == request.before
                {
                    // Only the camera removes a request that it did not
                    // confirm (after its confirmation time), and the
                    // installed software is unchanged.
                    request.status = SoftwareStatus::Failed;
                    request.phase = SoftwarePhase::Completed;
                    request.error = Some(CONFIRMATION_EXPIRED.into());
                } else {
                    request.phase = SoftwarePhase::Recovering;
                }
                request.cancellable = false;
                if request.status != SoftwareStatus::Running {
                    self.append_request(request.clone())?;
                    // No journal refers to the update folder any more. It
                    // holds a package that nobody confirmed: free its space
                    // now instead of at the next update.
                    let _ = self.clear(UPDATE_FOLDER);
                    // The old folder holds the files that the update
                    // replaced or removed, which have no later use. The
                    // record of the request stays.
                    if request.status == SoftwareStatus::Succeeded {
                        let _ = self.clear_except(BACKUP_FOLDER, Some(BACKUP_RECORD));
                    }
                }
            }
        }
        Ok(Some(request))
    }

    /// Cancel only the identified waiting transaction. Never unlink the RTOS's
    /// confirmed journal, even if confirmation races this request.
    pub fn cancel_software_request(&self, target_request_id: &str) -> Result<SoftwareRequest> {
        ensure!(
            crate::protocol::valid_request_id(target_request_id),
            "Invalid target request ID"
        );
        let mut request = self
            .software_request()?
            .context("The software request is unknown")?;
        ensure!(
            request.request_id.eq_ignore_ascii_case(target_request_id),
            "The target software request does not match"
        );
        if request.status == SoftwareStatus::Cancelled {
            return Ok(request);
        }
        ensure!(
            request.status == SoftwareStatus::Running && self.state()? == "waiting",
            "The software request cannot be cancelled"
        );
        request.phase = SoftwarePhase::Cancelling;
        request.cancellable = false;
        self.append_request(request.clone())?;
        // Atomic unlink vs the RTOS rename decides who crossed the boundary.
        ensure!(
            self.state()? == "waiting",
            "The camera already confirmed the software request"
        );
        // The Looks can create the import library after this delete.
        self.warm([IMPORTS_FILE]);
        fs::remove_file(self.path(REQUEST_FILE))
            .context("The camera may already have confirmed the software request")?;
        ensure!(
            !self.journal_files()?.contains(&JOURNAL_FILE),
            "The camera already confirmed the software request"
        );
        ensure!(
            SoftwareIdentity::of(&self.live_files()?) == request.before,
            "Recover changed software before completing cancellation"
        );
        request.status = SoftwareStatus::Cancelled;
        request.phase = SoftwarePhase::Completed;
        self.append_request(request.clone())?;
        // The staged content stays for the next update to reclaim. VERSION_FILE
        // is harmless without a request and does not affect software identity.
        Ok(request)
    }
}

/// The record of the software request (`BACKUP_RECORD`): a base line, which
/// the service writes when it stages an update, then a checkpoint
/// (`Checkpoint`) for each later state of the request.
#[derive(Debug, PartialEq)]
struct Record {
    request: Option<SoftwareRequest>,
}

impl Record {
    fn to_bytes(&self) -> Vec<u8> {
        json!({ "request": self.request })
            .to_string()
            .into_bytes()
            .into_iter()
            .chain(*b"\n")
            .collect()
    }

    /// An earlier format kept more fields in the base line, which are
    /// ignored, and also recorded rollback requests, which read as no
    /// request.
    fn parse(bytes: &[u8]) -> Option<Record> {
        if bytes.len() > MAX_RECORD_BYTES {
            return None;
        }
        let rollback = |request: &Value| request["domain"] == "rollback";
        let mut lines = bytes.split_inclusive(|byte| *byte == b'\n');
        let base: Value = grkit_core::effects::parse_json(lines.next()?).ok()?;
        let request = base.get("request").unwrap_or(&Value::Null);
        let mut record = Record {
            request: if rollback(request) {
                None
            } else {
                serde_json::from_value(request.clone()).ok()?
            },
        };
        if record
            .request
            .as_ref()
            .is_some_and(|request| !request.valid())
        {
            return None;
        }
        for (index, line) in lines.enumerate() {
            if index >= MAX_CHECKPOINTS {
                return None;
            }
            // A torn append never replaces the previous complete request state.
            if !line.ends_with(b"\n") {
                break;
            }
            record.request = match serde_json::from_slice::<Checkpoint>(line) {
                Ok(checkpoint) if checkpoint.valid() => Some(checkpoint.request),
                Err(_) if rollback(&serde_json::from_slice::<Value>(line).ok()?["request"]) => None,
                _ => return None,
            };
        }
        Some(record)
    }
}

impl Card {
    /// The package files of the card with the startup script, at their card
    /// paths.
    fn live_files(&self) -> Result<Files> {
        let mut files = self.package_files()?;
        if let Ok(startup) = fs::read(self.path(STARTUP_FILE)) {
            files.insert(STARTUP_FILE.to_owned(), startup);
        }
        Ok(files)
    }

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

    /// The package of the props and of the update resource, and with
    /// `hashes` the content hashes of the installed files, which only the
    /// update resource needs and which take every package file. The summary
    /// stays in memory while the name, size, and time of each file stay the
    /// same, and until the service changes the card itself
    /// (`forget_summary`).
    fn summary(&self, hashes: bool) -> Result<std::sync::Arc<Summary>> {
        let stamp = self.stamp()?;
        let mut cache = SUMMARY
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some((root, cached, summary)) = cache.as_ref() {
            if *root == self.root && *cached == stamp && (!hashes || summary.hashes.is_some()) {
                return Ok(std::sync::Arc::clone(summary));
            }
        }
        let summary = std::sync::Arc::new(if hashes {
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
        *cache = Some((self.root.clone(), stamp, std::sync::Arc::clone(&summary)));
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

    /// The name, size, and modification time of each file that `live_files`
    /// reads, without reading a file.
    fn stamp(&self) -> Result<Vec<(String, u64, Option<std::time::SystemTime>)>> {
        let mut stamp = Vec::new();
        let mut add = |name: String, metadata: fs::Metadata| {
            stamp.push((name, metadata.len(), metadata.modified().ok()));
        };
        let folder = self.path(PACKAGE_FOLDER);
        for entry in fs::read_dir(&folder).with_context(|| format!("Read {PACKAGE_FOLDER}"))? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if entry.file_type()?.is_dir() {
                for inner in fs::read_dir(entry.path())? {
                    let inner = inner?;
                    if inner.file_type()?.is_file() {
                        let inner_name = inner.file_name().to_string_lossy().into_owned();
                        add(format!("{name}/{inner_name}"), inner.metadata()?);
                    }
                }
            } else {
                add(name, entry.metadata()?);
            }
        }
        if let Ok(metadata) = fs::metadata(self.path(STARTUP_FILE)) {
            add(STARTUP_FILE.to_owned(), metadata);
        }
        stamp.sort();
        Ok(stamp)
    }
}

/// What `Card::summary` derives from the package files.
struct Summary {
    package: Value,
    hashes: Option<BTreeMap<String, String>>,
}

/// The package of the package files `files`.
fn package_of(files: &Files) -> Value {
    let (modules, effects, left_out) = match files.get(PACKAGE_LIST) {
        Some(bytes) => parse_package_list(&String::from_utf8_lossy(bytes)),
        None => (Vec::new(), Vec::new(), Vec::new()),
    };
    json!({
        "version": version_of(files),
        "modules": modules,
        "effects": effects,
        "left_out": left_out,
    })
}

/// The summary of the last card that a request read: its root, the stamp of
/// its files, and the summary.
type CachedSummary = (
    PathBuf,
    Vec<(String, u64, Option<std::time::SystemTime>)>,
    std::sync::Arc<Summary>,
);
static SUMMARY: std::sync::Mutex<Option<CachedSummary>> = std::sync::Mutex::new(None);

/// Drop the cached summary: the service changed the card, and a write can
/// keep the size and the time of a file.
pub fn forget_summary() {
    *SUMMARY
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
}

/// The module IDs, effect file names, and left-out effect names of a
/// package list, each sorted.
fn parse_package_list(list: &str) -> (Vec<String>, Vec<String>, Vec<String>) {
    let mut modules = Vec::new();
    let mut effects = Vec::new();
    let mut left_out = Vec::new();
    for line in list.lines() {
        if let Some(name) = line.strip_prefix('-') {
            left_out.push(name.to_owned());
        } else if let Some(id) = line.strip_suffix(".GRM") {
            modules.push(id.to_owned());
        } else if line.ends_with(".GRF") {
            effects.push(line.to_owned());
        }
    }
    modules.sort();
    effects.sort();
    left_out.sort();
    (modules, effects, left_out)
}

/// Whether the camera puts a card file of `path` there with each package.
/// The update removes only these when the new package does not have them:
/// the runtime files, the modules, and the package effects. Other files of
/// the package folder, such as the import library of the site, stay.
fn package_owned(path: &str) -> bool {
    if path == PACKAGE_LIST {
        return true;
    }
    if matches!(
        path,
        "SCRIPT/GRKIT/RELEASE.JSN" | "SCRIPT/GRKIT/RELEASE.SIG"
    ) || packaging::is_runtime_path(path)
    {
        return true;
    }
    let Some(rest) = path
        .strip_prefix(PACKAGE_FOLDER)
        .and_then(|rest| rest.strip_prefix('/'))
    else {
        return false;
    };
    let in_folder = |folder: &str, extension: &str| {
        rest.strip_prefix(folder)
            .and_then(|name| name.strip_prefix('/'))
            .is_some_and(|name| !name.contains('/') && name.ends_with(extension))
    };
    let modules = MODULES_FOLDER.rsplit('/').next().unwrap_or_default();
    in_folder(modules, ".GRM")
        || in_folder("EFFECTS", ".GRF")
        || (rest.starts_with("CORE") && rest.ends_with(".BIN") && !rest.contains('/'))
}

/// A Look file in the inbox of the package effects.
fn effect_inbox(path: &str) -> bool {
    path.strip_prefix(EFFECTS_FOLDER)
        .and_then(|name| name.strip_prefix('/'))
        .is_some_and(|name| !name.contains('/') && name.ends_with(".GRF"))
}

/// The package list `new` with the Looks of the card's list `live`, the
/// listed and the left-out ones, in place of its own: names sorted, then
/// the left-out names sorted, as packages write it.
fn carry_looks(new: &[u8], live: Option<&[u8]>) -> Vec<u8> {
    let look = |line: &&str| line.starts_with('-') || line.ends_with(".GRF");
    let new = String::from_utf8_lossy(new);
    let live = live.map(String::from_utf8_lossy).unwrap_or_default();
    let mut names: Vec<&str> = new
        .lines()
        .filter(|line| !line.is_empty() && !look(line))
        .chain(
            live.lines()
                .filter(|line| look(line) && !line.starts_with('-')),
        )
        .collect();
    let mut left_out: Vec<&str> = live.lines().filter(|line| line.starts_with('-')).collect();
    names.sort();
    names.dedup();
    left_out.sort();
    left_out.dedup();
    names
        .into_iter()
        .chain(left_out)
        .map(|line| format!("{line}\n"))
        .collect::<String>()
        .into_bytes()
}

/// The changes from the package files on the card, `live`, to `new`: each
/// new file that differs from the card, and each package file that the new
/// package does not have. The startup script is a change only if it differs.
fn changes(live: &Files, new: &Files, live_startup: Option<&[u8]>) -> Vec<Change> {
    let mut changes = Vec::new();
    for (path, bytes) in new {
        let current = if path == STARTUP_FILE {
            live_startup
        } else {
            live.get(path).map(Vec::as_slice)
        };
        if current != Some(bytes.as_slice()) {
            changes.push(Change::Install(path.clone()));
        }
    }
    for path in live.keys() {
        if !new.contains_key(path) && package_owned(path) && !effect_inbox(path) {
            changes.push(Change::Remove(path.clone()));
        }
    }
    changes
}

/// The GRKit version that the core files of the package files `files` name.
fn version_of(files: &Files) -> Option<String> {
    // Only the core file that the package's SCAN.SH runs: a core file left
    // on the card by an earlier package names that package's version.
    let core = runtime::scanned_core(files.get(SCAN_FILE)?)?;
    packaging::marked_version(files.get(&core)?).map(str::to_owned)
}

/// The version file of the package files `files` (`card::VERSION_FILE`).
fn version_file(files: &Files) -> Vec<u8> {
    version_digits(version_of(files).as_deref())
}

/// The version file of `version`: six digits, or zeros.
fn version_digits(version: Option<&str>) -> Vec<u8> {
    let parts: Option<Vec<u32>> = version
        .map(|version| version.split('.').map(|part| part.parse().ok()).collect())
        .and_then(|parts: Option<Vec<u32>>| parts);
    match parts.as_deref() {
        Some([major, minor, patch]) if [major, minor, patch].iter().all(|part| **part < 100) => {
            format!("{major:02}{minor:02}{patch:02}").into_bytes()
        }
        _ => b"000000".to_vec(),
    }
}

/// Checks the package `zip` for `device`, writes its changes to the update
/// folder, and writes the journal. Returns what the update does.
pub fn stage(card: &Card, zip: &[u8], device: &Device, keys: &[[u8; 32]]) -> Result<Value> {
    stage_inner(card, zip, device, keys, None)
}

pub fn stage_request(
    card: &Card,
    zip: &[u8],
    device: &Device,
    keys: &[[u8; 32]],
    context: &SoftwareContext,
) -> Result<SoftwareRequest> {
    ensure!(context.valid(), "Invalid software request context");
    ensure!(
        card.state()? == "none",
        "A software request is already pending"
    );
    if let Some(previous) = card.software_request()? {
        ensure!(
            previous.status != SoftwareStatus::Running,
            "Recover the previous software request before writing"
        );
    }
    if let Err(error) = stage_inner(card, zip, device, keys, Some(context)) {
        fail_unstaged(card, &context.request_id);
        return Err(error);
    }
    card.software_request()?
        .context("The software request was not retained")
}

/// A request whose journal never reached the card did not wait for a
/// confirmation: record its own failure, so that it does not read as one
/// that the camera let expire.
fn fail_unstaged(card: &Card, request_id: &str) {
    if card.state().ok() != Some("none") {
        return;
    }
    let Ok(Some(mut request)) = card
        .read_record()
        .map(|record| record.and_then(|r| r.request))
    else {
        return;
    };
    if request.request_id.eq_ignore_ascii_case(request_id)
        && request.status == SoftwareStatus::Running
    {
        request.status = SoftwareStatus::Failed;
        request.phase = SoftwarePhase::Completed;
        request.cancellable = false;
        request.error = Some(STAGING_FAILED.into());
        let _ = card.append_request(request);
    }
}

fn stage_inner(
    card: &Card,
    zip: &[u8],
    device: &Device,
    keys: &[[u8; 32]],
    context: Option<&SoftwareContext>,
) -> Result<Value> {
    ensure!(zip.len() <= MAX_PACKAGE, "The package is too large");
    let (update, files) = release::verify_update(zip, keys).context("Check the package")?;
    check_camera(update.statement.camera, device)?;
    // The package folder and the startup script; the update keeps the
    // ModeSet files of the package off the card, as SD scripts are on.
    let mut new: Files = files
        .into_iter()
        .filter(|(path, _)| path == STARTUP_FILE || path.starts_with(&format!("{PACKAGE_FOLDER}/")))
        .collect();
    // An update carries no Looks: the phone manages them on their own. The
    // update leaves every Look file of the card as it is, and the new package
    // list keeps the Looks that the card lists and leaves out.
    new.retain(|path, _| !effect_inbox(path));
    let live = card.package_files().context(CardNotReady)?;
    let live_startup = fs::read(card.path(STARTUP_FILE)).ok();
    if let Some(list) = new.get_mut(PACKAGE_LIST) {
        *list = carry_looks(list, live.get(PACKAGE_LIST).map(Vec::as_slice));
    }
    let changes = changes(&live, &new, live_startup.as_deref());
    // The card after the update: the new package and the Looks it keeps.
    let mut expected = new.clone();
    expected.extend(
        live.iter()
            .filter(|(path, _)| effect_inbox(path))
            .map(|(path, bytes)| (path.clone(), bytes.clone())),
    );
    let mut before = live.clone();
    if let Some(startup) = &live_startup {
        before.insert(STARTUP_FILE.to_owned(), startup.clone());
    }
    let record = Record {
        request: context.map(|context| SoftwareRequest {
            request_id: context.request_id.to_ascii_lowercase(),
            domain: SoftwareDomain::Update,
            boot_id: context.boot_id.clone(),
            card_id: context.card_id.clone(),
            status: if changes.is_empty() {
                SoftwareStatus::Succeeded
            } else {
                SoftwareStatus::Running
            },
            phase: if changes.is_empty() {
                SoftwarePhase::Completed
            } else {
                SoftwarePhase::AwaitingConfirmation
            },
            cancellable: !changes.is_empty(),
            expected: SoftwareIdentity::of(&expected),
            before: SoftwareIdentity::of(&before),
            error: None,
        }),
    };
    ensure!(
        record.to_bytes().len() <= MAX_RECORD_BYTES,
        "The software request record is too large"
    );
    let staged: Vec<(String, &Vec<u8>)> = changes
        .iter()
        .filter_map(|change| match change {
            Change::Install(path) => Some(path),
            Change::Remove(_) => None,
        })
        .map(|path| Ok((update_paths(path)?.0, &new[path])))
        .collect::<Result<_>>()?;
    let bytes: u64 = staged.iter().map(|(_, data)| data.len() as u64).sum();
    if let Some(free) = device.free_bytes {
        ensure!(
            free >= bytes + SPARE_BYTES,
            "The card needs {} more bytes for the update",
            bytes + SPARE_BYTES - free
        );
    }
    // A new update replaces an update that has not been installed.
    card.cancel()?;
    let summary = |state: &str, installs: usize| {
        json!({
            "state": state,
            "camera": update.statement.camera.id,
            "version": update.statement.version,
            "modules": update.modules,
            "effects": update.effect_ids.len(),
            "install": installs,
            "remove": changes.len() - installs,
            "bytes": bytes,
        })
    };
    if changes.is_empty() {
        card.clear(UPDATE_FOLDER)?;
        if let Some(request) = record.request.clone() {
            if card.read_record()?.is_some() {
                card.append_request(request)?;
            } else {
                card.write_checked(BACKUP_RECORD, &record.to_bytes())?;
            }
        }
        return Ok(summary("none", 0));
    }
    card.forget_deleted(&[VERSION_FILE, DRAFT_FILE, REQUEST_FILE]);
    card.warm(
        staged
            .iter()
            .map(|(path, _)| path.as_str())
            .chain([BACKUP_RECORD]),
    );
    card.clear(UPDATE_FOLDER)?;
    card.clear(BACKUP_FOLDER)?;
    for (path, data) in &staged {
        card.write_checked(path, data)?;
    }
    // The record of the request is on the card before the camera can see the
    // journal.
    card.write_checked(BACKUP_RECORD, &record.to_bytes())?;
    // The camera sees the journal only when it is complete, and runs it only
    // after the confirmation on the camera. It reads the version first.
    card.write_checked(VERSION_FILE, &version_file(&new))?;
    card.write_checked(DRAFT_FILE, &journal(&changes)?)?;
    fs::rename(card.path(DRAFT_FILE), card.path(REQUEST_FILE)).context("Write the journal")?;
    ensure!(card.state()? == "waiting", "The journal is not on the card");
    Ok(summary("waiting", staged.len()))
}

/// Checks that the package for `package` is for the camera of `device` and
/// its firmware.
fn check_camera(package: &Camera, device: &Device) -> Result<()> {
    let Some(camera) = camera_of(&device.model) else {
        bail!("GRKit does not support the {}", device.model);
    };
    ensure!(
        package == camera,
        "The package is for the {}, not the {}",
        package.name,
        camera.name
    );
    ensure!(
        camera.versions().contains(&device.firmware.as_str()),
        "The package does not support firmware {} of the {}",
        device.firmware,
        camera.name
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use grkit_core::{camera, testing};

    /// A package with `effects`, which the key of the tests signed. An
    /// update carries no Looks, so each effect set gets its own version: the
    /// runtime is what an update changes.
    fn package(effects: &[&str]) -> Vec<u8> {
        let version = match effects.first().copied() {
            Some("second" | "other-effect") => "1.7.1",
            Some("third") => "1.7.2",
            Some("fourth") => "1.7.3",
            _ => "1.7.0",
        };
        testing::signed_package(&camera::GR3, version, effects)
    }

    fn keys() -> [[u8; 32]; 1] {
        [testing::release_key()]
    }

    fn gr3() -> Device {
        Device {
            model: "RICOH GR III".into(),
            firmware: "2.21".into(),
            free_bytes: Some(1 << 30),
            serial_number: Some("test-camera".into()),
        }
    }

    /// A card with the package `zip` on it, as a copy of `SD_ROOT` puts it.
    fn card_with(zip: &[u8]) -> (tempfile::TempDir, Card) {
        let dir = tempfile::tempdir().unwrap();
        for (path, bytes) in packaging::card_files(zip).unwrap() {
            let target = dir.path().join(&path);
            fs::create_dir_all(target.parent().unwrap()).unwrap();
            fs::write(target, bytes).unwrap();
        }
        let card = Card::new(dir.path());
        (dir, card)
    }

    fn read(card: &Card, path: &str) -> Option<Vec<u8>> {
        fs::read(card.path(path)).ok()
    }

    fn request_context() -> SoftwareContext {
        SoftwareContext {
            request_id: "550e8400-e29b-41d4-a716-446655440000".into(),
            boot_id: "service-boot".into(),
            card_id: "card-identity".into(),
        }
    }

    #[test]
    fn request_survives_restart_and_success_requires_installed_hashes() {
        let (dir, card) = card_with(&package(&["first"]));
        let next = package(&["second"]); // Deliberately the same version.
        let request = stage_request(&card, &next, &gr3(), &keys(), &request_context()).unwrap();
        assert_eq!(request.status, SoftwareStatus::Running);
        assert_eq!(request.phase, SoftwarePhase::AwaitingConfirmation);
        assert!(request.cancellable);
        assert_ne!(request.expected, request.before);
        assert_eq!(
            Card::new(dir.path())
                .software_request()
                .unwrap()
                .unwrap()
                .request_id,
            request.request_id
        );
        fs::rename(card.path(REQUEST_FILE), card.path(JOURNAL_FILE)).unwrap();
        assert_eq!(
            card.software_request().unwrap().unwrap().phase,
            SoftwarePhase::AwaitingRestart
        );
        fs::rename(card.path(JOURNAL_FILE), card.path(REQUEST_FILE)).unwrap();
        run_journal(&card);
        let complete = card.software_request().unwrap().unwrap();
        assert_eq!(complete.status, SoftwareStatus::Succeeded);
        assert_eq!(
            complete.expected,
            SoftwareIdentity::of(&card.live_files().unwrap())
        );
        assert_eq!(
            Card::new(dir.path()).software_request().unwrap().unwrap(),
            complete
        );
        assert!(!complete
            .expected
            .content_hashes
            .keys()
            .any(|path| path.contains("COMMIT.")));
    }

    #[test]
    fn a_request_whose_journal_was_not_written_fails_as_unstaged() {
        let (_dir, card) = card_with(&package(&["first"]));
        let request = stage_request(
            &card,
            &package(&["second"]),
            &gr3(),
            &keys(),
            &request_context(),
        )
        .unwrap();
        // As when the rename of the journal failed after the checkpoint.
        fs::remove_file(card.path(REQUEST_FILE)).unwrap();
        fail_unstaged(&card, &request.request_id);
        let failed = card.software_request().unwrap().unwrap();
        assert_eq!(failed.status, SoftwareStatus::Failed);
        assert_eq!(failed.error.as_deref(), Some(STAGING_FAILED));
        let mut next = request_context();
        next.request_id = "650e8400-e29b-41d4-a716-446655440000".into();
        stage_request(&card, &package(&["second"]), &gr3(), &keys(), &next).unwrap();
        assert_eq!(card.state().unwrap(), "waiting");
    }
    #[test]
    fn an_expired_request_fails_durably_and_a_changed_card_still_blocks_updates() {
        let (dir, card) = card_with(&package(&["first"]));
        stage_request(
            &card,
            &package(&["second"]),
            &gr3(),
            &keys(),
            &request_context(),
        )
        .unwrap();
        // The camera deletes a request that nobody confirms in time.
        fs::remove_file(card.path(REQUEST_FILE)).unwrap();
        let expired = card.software_request().unwrap().unwrap();
        assert_eq!(expired.status, SoftwareStatus::Failed);
        assert_eq!(expired.phase, SoftwarePhase::Completed);
        assert_eq!(expired.error.as_deref(), Some(CONFIRMATION_EXPIRED));
        assert!(!expired.cancellable);
        // The outcome survives a restart of the service.
        assert_eq!(
            Card::new(dir.path()).software_request().unwrap().unwrap(),
            expired
        );
        let mut next = request_context();
        next.request_id = "650e8400-e29b-41d4-a716-446655440000".into();
        stage_request(&card, &package(&["third"]), &gr3(), &keys(), &next).unwrap();
        // Software that is neither the old nor the new one is uncertain.
        fs::remove_file(card.path(REQUEST_FILE)).unwrap();
        let live = card.path(grkit_core::card::PACKAGE_LIST);
        let mut list = fs::read(&live).unwrap();
        list.extend_from_slice(b"\n");
        fs::write(&live, list).unwrap();
        let recovered = card.software_request().unwrap().unwrap();
        assert_eq!(recovered.status, SoftwareStatus::Running);
        assert_eq!(recovered.phase, SoftwarePhase::Recovering);
        assert!(!recovered.cancellable);
        next.request_id = "750e8400-e29b-41d4-a716-446655440000".into();
        assert!(stage_request(&card, &package(&["fourth"]), &gr3(), &keys(), &next).is_err());
    }

    #[test]
    fn cancellation_is_targeted_durable_and_cannot_remove_a_confirmed_journal() {
        let (dir, card) = card_with(&package(&["first"]));
        let ctx = request_context();
        stage_request(&card, &package(&["second"]), &gr3(), &keys(), &ctx).unwrap();
        assert!(card
            .cancel_software_request("650e8400-e29b-41d4-a716-446655440000")
            .is_err());
        assert_eq!(card.state().unwrap(), "waiting");
        fs::rename(card.path(REQUEST_FILE), card.path(JOURNAL_FILE)).unwrap();
        assert!(card.cancel_software_request(&ctx.request_id).is_err());
        assert!(card.path(JOURNAL_FILE).is_file());
        fs::rename(card.path(JOURNAL_FILE), card.path(REQUEST_FILE)).unwrap();
        let cancelled = card.cancel_software_request(&ctx.request_id).unwrap();
        assert_eq!(cancelled.status, SoftwareStatus::Cancelled);
        assert_eq!(card.state().unwrap(), "none");
        assert_eq!(
            Card::new(dir.path()).software_request().unwrap().unwrap(),
            cancelled
        );
        assert_eq!(
            card.cancel_software_request(&ctx.request_id).unwrap(),
            cancelled
        );
    }

    #[test]
    fn interrupted_cancellation_recovers_without_overwriting_the_record() {
        let (dir, card) = card_with(&package(&["first"]));
        let ctx = request_context();
        let mut request =
            stage_request(&card, &package(&["second"]), &gr3(), &keys(), &ctx).unwrap();
        request.phase = SoftwarePhase::Cancelling;
        request.cancellable = false;
        card.append_request(request).unwrap();
        fs::remove_file(card.path(REQUEST_FILE)).unwrap();
        let before = card.read_record_bytes().unwrap().unwrap();
        // A torn terminal checkpoint is ignored, then safely truncated when
        // recovery appends its verified terminal outcome.
        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(card.path(BACKUP_RECORD))
            .unwrap();
        file.write_all(b"{\"request\":").unwrap();
        file.sync_all().unwrap();
        drop(file);
        let result = Card::new(dir.path()).software_request().unwrap().unwrap();
        assert_eq!(result.status, SoftwareStatus::Cancelled);
        assert!(card
            .read_record_bytes()
            .unwrap()
            .unwrap()
            .starts_with(&before));
    }

    #[test]
    fn a_succeeded_update_deletes_the_old_files_and_keeps_its_request() {
        fn files(card: &Card, folder: &str) -> Vec<String> {
            let mut found = Vec::new();
            for entry in fs::read_dir(card.path(folder)).unwrap() {
                let entry = entry.unwrap();
                let path = format!("{folder}/{}", entry.file_name().to_string_lossy());
                if entry.file_type().unwrap().is_dir() {
                    found.extend(files(card, &path));
                } else {
                    found.push(path);
                }
            }
            found
        }
        let (dir, card) = card_with(&package(&["first"]));
        let ctx = request_context();
        stage_request(&card, &package(&["second"]), &gr3(), &keys(), &ctx).unwrap();
        run_journal(&card);
        // The journal moved the files that the update replaced or removed.
        assert!(files(&card, BACKUP_FOLDER).len() > 1);
        let complete = card.software_request().unwrap().unwrap();
        assert_eq!(complete.status, SoftwareStatus::Succeeded);
        assert_eq!(files(&card, BACKUP_FOLDER), [BACKUP_RECORD]);
        assert_eq!(
            Card::new(dir.path()).software_request().unwrap().unwrap(),
            complete
        );
    }

    #[test]
    fn a_record_of_the_earlier_format_still_has_its_request() {
        let (_dir, card) = card_with(&package(&["first"]));
        let request = stage_request(
            &card,
            &package(&["second"]),
            &gr3(),
            &keys(),
            &request_context(),
        )
        .unwrap();
        // The earlier format also kept what a rollback needed.
        let hash = "0".repeat(64);
        let base = json!({
            "version": "1.6.2",
            "installed": {PACKAGE_LIST: hash},
            "added": [],
            "backup": [PACKAGE_LIST],
            "backupHashes": {PACKAGE_LIST: hash},
            "before": request.before,
            "request": request,
        });
        fs::write(card.path(BACKUP_RECORD), format!("{base}\n")).unwrap();
        assert_eq!(card.software_request().unwrap().as_ref(), Some(&request));
        // A rollback request of that format reads as no request.
        let mut rollback = serde_json::to_value(Checkpoint::new(request)).unwrap();
        rollback["request"]["domain"] = "rollback".into();
        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(card.path(BACKUP_RECORD))
            .unwrap();
        writeln!(file, "{rollback}").unwrap();
        assert_eq!(card.software_request().unwrap(), None);
    }

    #[test]
    fn a_noop_request_succeeds_and_an_invalid_context_writes_nothing() {
        let (_dir, card) = card_with(&package(&["first"]));
        let mut invalid = request_context();
        invalid.request_id = "not-a-uuid".into();
        assert!(stage_request(&card, &package(&["second"]), &gr3(), &keys(), &invalid).is_err());
        assert!(!card.path(BACKUP_RECORD).exists());
        let next = package(&["second"]);
        stage_request(&card, &next, &gr3(), &keys(), &request_context()).unwrap();
        run_journal(&card);
        let mut second = request_context();
        second.request_id = "650e8400-e29b-41d4-a716-446655440000".into();
        let noop = stage_request(&card, &next, &gr3(), &keys(), &second).unwrap();
        assert_eq!(noop.status, SoftwareStatus::Succeeded);
        assert_eq!(noop.request_id, second.request_id);
    }

    #[test]
    fn an_update_stages_only_the_files_that_change_and_keeps_the_looks() {
        let (_dir, card) = card_with(&package(&["test-effect"]));
        // The card also leaves a Look out.
        let mut list = read(&card, PACKAGE_LIST).unwrap();
        list.extend_from_slice(b"-LEFT.GRF\n");
        fs::write(card.path(PACKAGE_LIST), &list).unwrap();
        let next = package(&["other-effect"]);
        let summary = stage(&card, &next, &gr3(), &keys()).unwrap();
        assert_eq!(summary["state"], "waiting");
        assert_eq!(summary["camera"], "gr3");
        // The new core is staged; the same files are not.
        let (core, _) = update_paths("SCRIPT/GRKIT/CORE1.BIN").unwrap();
        assert!(read(&card, &core).is_some());
        let (scan, _) = update_paths(SCAN_FILE).unwrap();
        assert!(read(&card, &scan).is_none());
        // No Look of either package moves: the card keeps its Looks, and the
        // new list names them as the card's list does.
        let journal = String::from_utf8(read(&card, REQUEST_FILE).unwrap()).unwrap();
        assert!(!journal.contains("EFFECTS"), "{journal}");
        assert_eq!(summary["remove"], 0);
        // The list names the same modules and the card's Looks: it stays.
        let (staged_list, _) = update_paths(PACKAGE_LIST).unwrap();
        assert!(read(&card, &staged_list).is_none());
        assert!(read(&card, DRAFT_FILE).is_none());
        assert!(read(&card, JOURNAL_FILE).is_none());
        assert_eq!(card.state().unwrap(), "waiting");
    }

    #[test]
    fn the_new_list_keeps_the_looks_of_the_card() {
        let new = b"A.GRM\nNEW.GRF\nB.GRM\n-GONE.GRF\n";
        let live = b"A.GRM\nOLD.GRF\nMINE.GRF\n-LEFT.GRF\n";
        assert_eq!(
            String::from_utf8(carry_looks(new, Some(live))).unwrap(),
            "A.GRM\nB.GRM\nMINE.GRF\nOLD.GRF\n-LEFT.GRF\n"
        );
    }

    #[test]
    fn an_update_keeps_the_files_that_no_package_has() {
        let (_dir, card) = card_with(&package(&["test-effect"]));
        fs::write(card.path("SCRIPT/GRKIT/IMPORTS.BIN"), b"imports").unwrap();
        stage(&card, &package(&["other-effect"]), &gr3(), &keys()).unwrap();
        let journal = String::from_utf8(read(&card, REQUEST_FILE).unwrap()).unwrap();
        assert!(!journal.contains("IMPORTS.BIN"));
    }

    #[test]
    fn a_new_update_replaces_the_files_of_the_last_one() {
        let (_dir, card) = card_with(&package(&["test-effect"]));
        stage(&card, &package(&["other-effect"]), &gr3(), &keys()).unwrap();
        // The same package as the card: no update waits.
        let summary = stage(&card, &package(&["test-effect"]), &gr3(), &keys()).unwrap();
        assert_eq!(summary["state"], "none");
        assert_eq!(summary["install"], 0);
        assert_eq!(summary["remove"], 0);
        assert_eq!(card.state().unwrap(), "none");
        let staged_effects =
            fs::read_dir(card.path(&format!("{UPDATE_FOLDER}/EFFECTS"))).map_or(0, Iterator::count);
        assert_eq!(staged_effects, 0);
    }

    #[test]
    fn an_update_empties_the_backup_folder_for_its_files() {
        let (_dir, card) = card_with(&package(&["test-effect"]));
        fs::create_dir_all(card.path(BACKUP_FOLDER)).unwrap();
        fs::write(card.path(&format!("{BACKUP_FOLDER}/CORE1.BIN")), b"old").unwrap();
        stage(&card, &package(&["other-effect"]), &gr3(), &keys()).unwrap();
        assert!(read(&card, &format!("{BACKUP_FOLDER}/CORE1.BIN")).is_none());
    }

    #[test]
    fn the_version_file_has_the_version_that_the_core_names() {
        let core = |version: &str| {
            let mut binary = testing::installer();
            binary
                .extend_from_slice(format!("{}{version}\0", packaging::VERSION_MARKER).as_bytes());
            runtime::package_runtime_files(&[&binary, &binary], runtime::ICON_STRIDE)
        };
        assert_eq!(version_file(&core("1.7.0")), b"010700");
        assert_eq!(version_file(&core("12.3.45")), b"120345");
        assert_eq!(version_file(&core("1.7")), b"000000");
        assert_eq!(version_file(&core("100.0.0")), b"000000");
        assert_eq!(version_file(&Files::new()), b"000000");
        // A core file left by an earlier package does not name the version.
        let mut left = core("1.7.0");
        let mut old = testing::installer();
        old.extend_from_slice(format!("{}1.6.2\0", packaging::VERSION_MARKER).as_bytes());
        left.insert(format!("{PACKAGE_FOLDER}/CORE.BIN"), old);
        assert_eq!(version_file(&left), b"010700");
        let (_dir, card) = card_with(&package(&["test-effect"]));
        stage(&card, &package(&["other-effect"]), &gr3(), &keys()).unwrap();
        assert_eq!(read(&card, VERSION_FILE).unwrap(), b"010701");
        card.cancel().unwrap();
        assert!(read(&card, VERSION_FILE).is_none());
    }

    #[test]
    fn a_cancel_removes_the_journal() {
        let (_dir, card) = card_with(&package(&["test-effect"]));
        stage(&card, &package(&["other-effect"]), &gr3(), &keys()).unwrap();
        assert!(card.cancel().unwrap());
        assert_eq!(card.state().unwrap(), "none");
        // A confirmed update can be removed before the next power-on too.
        stage(&card, &package(&["other-effect"]), &gr3(), &keys()).unwrap();
        fs::rename(card.path(REQUEST_FILE), card.path(JOURNAL_FILE)).unwrap();
        assert_eq!(card.state().unwrap(), "confirmed");
        assert!(card.cancel().unwrap());
        assert_eq!(card.state().unwrap(), "none");
        assert!(!card.cancel().unwrap());
    }

    #[test]
    fn the_package_must_be_for_this_camera_and_its_firmware() {
        let (_dir, card) = card_with(&package(&["test-effect"]));
        let next = package(&["other-effect"]);
        for (model, firmware, error) in [
            ("RICOH GR IV", "1.21", "is for the GR III, not the GR IV"),
            ("RICOH GR III", "9.99", "does not support firmware 9.99"),
            ("RICOH GR II", "4.10", "does not support the RICOH GR II"),
        ] {
            let device = Device {
                model: model.into(),
                firmware: firmware.into(),
                free_bytes: None,
                serial_number: None,
            };
            let message = format!("{:#}", stage(&card, &next, &device, &keys()).unwrap_err());
            assert!(message.contains(error), "{message}");
        }
        // An HDF model has the package of its camera.
        let hdf = Device {
            model: "RICOH GR III HDF".into(),
            ..gr3()
        };
        stage(&card, &next, &hdf, &keys()).unwrap();
    }

    #[test]
    fn a_package_that_is_not_valid_or_does_not_fit_changes_nothing() {
        let (_dir, card) = card_with(&package(&["test-effect"]));
        let mut broken = package(&["other-effect"]);
        let at = broken.len() / 2;
        broken[at] ^= 0xff;
        assert!(stage(&card, &broken, &gr3(), &keys()).is_err());
        let full = Device {
            free_bytes: Some(1024),
            ..gr3()
        };
        let message = format!(
            "{:#}",
            stage(&card, &package(&["other-effect"]), &full, &keys()).unwrap_err()
        );
        assert!(message.contains("more bytes"), "{message}");
        assert_eq!(card.state().unwrap(), "none");
        assert!(!card.path(UPDATE_FOLDER).exists());
    }

    #[test]
    fn only_a_package_that_a_release_key_signed_is_staged() {
        let (_dir, card) = card_with(&package(&["test-effect"]));
        let next = package(&["other-effect"]);
        // Another key.
        let message = format!(
            "{:#}",
            stage(&card, &next, &gr3(), &grkit_core::release::OFFICIAL_KEYS).unwrap_err()
        );
        assert!(message.contains("No GRKit release key"), "{message}");
        // No signature.
        let unsigned = packaging::assemble(
            &testing::release_runtimes(&camera::GR3, "1.7.0"),
            &testing::effect_set(&["other-effect"]),
            &testing::official_modules(),
            &camera::GR3,
            &packaging::Guide {
                language: grkit_core::language::Language::English,
                pdf: b"%PDF-1.7\n",
            },
        )
        .unwrap()
        .bytes;
        let message = format!(
            "{:#}",
            stage(&card, &unsigned, &gr3(), &keys()).unwrap_err()
        );
        assert!(message.contains("not signed"), "{message}");
        assert_eq!(card.state().unwrap(), "none");
        assert!(!card.path(UPDATE_FOLDER).exists());
    }

    /// Runs the journal on the card as the startup script does: each step
    /// that moves a file, in order.
    fn run_journal(card: &Card) {
        let text = String::from_utf8(read(card, REQUEST_FILE).unwrap()).unwrap();
        let path = |name: &str| name.trim_start_matches("C:\\").replace('\\', "/");
        for line in text.lines() {
            let Some(rest) = line.strip_prefix("filerename '") else {
                continue;
            };
            let (from, to) = rest.trim_end_matches('\'').split_once("' '").unwrap();
            let (from, to) = (card.path(&path(from)), card.path(&path(to)));
            // The guards of the journal skip a step that is done.
            if from.exists() && !to.exists() {
                fs::create_dir_all(to.parent().unwrap()).unwrap();
                fs::rename(from, to).unwrap();
            }
        }
        fs::remove_file(card.path(REQUEST_FILE)).unwrap();
    }

    #[test]
    fn the_package_lists_its_modules_effects_and_left_out() {
        let (_dir, card) = card_with(&package(&["test-effect"]));
        let info = card.package().unwrap();
        assert_eq!(info["version"], "1.7.0");
        let modules = info["modules"].as_array().unwrap();
        assert!(modules.iter().any(|id| id == "FILMLOOK"));
        assert!(modules
            .windows(2)
            .all(|pair| { pair[0].as_str().unwrap() <= pair[1].as_str().unwrap() }));
        let effect = grkit_core::effects::filename("test-effect").unwrap();
        assert_eq!(info["effects"], json!([effect]));
        assert_eq!(info["left_out"], json!([]));
        let list = card.path(PACKAGE_LIST);
        let mut text = String::from_utf8(fs::read(&list).unwrap()).unwrap();
        text.push_str("-LEFT_OUT.GRF\n");
        fs::write(&list, text).unwrap();
        assert_eq!(card.package().unwrap()["left_out"], json!(["LEFT_OUT.GRF"]));
    }

    /// The summary of the package files answers repeated reads without
    /// reading the files again, and a change of the service shows at once,
    /// even one that keeps the size and the time of a file.
    #[test]
    fn the_package_summary_follows_each_change_of_the_card() {
        let (_dir, card) = card_with(&package(&["test-effect"]));
        let first = card.inventory().unwrap();
        assert_eq!(card.inventory().unwrap(), first);
        let list = card.path(PACKAGE_LIST);
        let text = fs::read(&list).unwrap();
        let time = fs::metadata(&list).unwrap().modified().unwrap();
        let mut changed = text.clone();
        let last = changed.len() - 2;
        changed[last] = if changed[last] == b'F' { b'G' } else { b'F' };
        fs::write(&list, &changed).unwrap();
        fs::File::options()
            .write(true)
            .open(&list)
            .unwrap()
            .set_modified(time)
            .unwrap();
        forget_summary();
        let after = card.inventory().unwrap();
        assert_ne!(after["contentHashes"], first["contentHashes"]);
        assert_ne!(after["package"], first["package"]);
    }

    #[test]
    fn a_missing_package_folder_is_not_ready() {
        let dir = tempfile::tempdir().unwrap();
        let card = Card::new(dir.path());
        assert!(card_not_ready(&card.ensure_readable().unwrap_err()));
        assert!(card_not_ready(&card.state().unwrap_err()));
        assert!(card_not_ready(&card.package().unwrap_err()));
    }

    #[test]
    fn the_package_owns_its_runtime_modules_and_effects() {
        for path in [
            "SCRIPT/GRKIT/CORE.BIN",
            "SCRIPT/GRKIT/CORE3.BIN",
            "SCRIPT/GRKIT/SERVE.BIN",
            "SCRIPT/GRKIT/MODULES/GLOW.GRM",
            "SCRIPT/GRKIT/EFFECTS/1234ABCD.GRF",
        ] {
            assert!(package_owned(path), "{path}");
        }
        for path in [
            "SCRIPT/GRKIT/IMPORTS.BIN",
            "SCRIPT/GRKIT/COMMIT.TTL",
            "SCRIPT/GRKIT/MODULES/README.TXT",
            "GRKIT/EFFECTS/1234ABCD.GRF",
        ] {
            assert!(!package_owned(path), "{path}");
        }
    }
}
