//! The record of the software request (`BACKUP_RECORD`): a base line, which
//! the service writes when it stages an update, then a checkpoint for each
//! later state of the request. A newline commits one checksummed JSON frame.

use super::card::{Card, JournalState};
use anyhow::{bail, ensure, Context, Result};
use grkit_core::{
    card::{BACKUP_FOLDER, BACKUP_RECORD, IMPORTS_FILE, JOURNAL_FILE, REQUEST_FILE, UPDATE_FOLDER},
    packaging, Files,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
};

const MAX_RECORD_BYTES: usize = 1024 * 1024;
const MAX_CHECKPOINTS: usize = 16;

/// The request that waited for the confirmation on the camera left the card
/// without an install, as when the card was changed on a computer.
pub const REQUEST_REMOVED: &str = "request_removed";
/// The journal of the request did not reach the card.
pub const STAGING_FAILED: &str = "staging_failed";
/// The card holds another signed release than the one that the request
/// expected, as when the card was written on a computer.
pub const SOFTWARE_REPLACED: &str = "software_replaced";

/// A bounded signed-software identity. Staging metadata and user data are excluded.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SoftwareIdentity {
    pub digest: String,
    pub content_hashes: BTreeMap<String, String>,
}

impl SoftwareIdentity {
    /// The identity of the card files `files`, at their card paths.
    pub(super) fn of<'a>(files: impl IntoIterator<Item = (&'a String, &'a Vec<u8>)>) -> Self {
        let content_hashes: BTreeMap<_, _> = files
            .into_iter()
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

/// Whether the card file `path` is part of the installed software.
pub(super) fn software_owned(path: &str) -> bool {
    path == grkit_core::card::STARTUP_FILE || packaging::package_owned(path)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SoftwareStatus {
    Running,
    Succeeded,
    Cancelled,
    /// Ended without installing the request; `error` says why.
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
    pub boot_id: String,
    pub card_id: String,
    pub status: SoftwareStatus,
    pub phase: SoftwarePhase,
    pub cancellable: bool,
    pub expected: SoftwareIdentity,
    pub before: SoftwareIdentity,
    /// The code of a failed request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
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

    /// Ends the request with `status`, and the error code `error` of a failure.
    fn end(&mut self, status: SoftwareStatus, error: Option<&str>) {
        self.status = status;
        self.phase = SoftwarePhase::Completed;
        self.cancellable = false;
        self.error = error.map(str::to_owned);
    }
}

#[derive(Clone, Debug)]
pub struct SoftwareContext {
    pub request_id: String,
    pub boot_id: String,
    pub card_id: String,
}
impl SoftwareContext {
    pub(super) fn valid(&self) -> bool {
        crate::protocol::valid_request_id(&self.request_id)
            && valid_context_id(&self.boot_id)
            && valid_context_id(&self.card_id)
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

/// Append-only software transaction checkpoints share the record. They are
/// not a generic request history.
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

/// The base line of the record, and with the checkpoints, the last state of
/// the request.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Record {
    pub(super) request: Option<SoftwareRequest>,
}

impl Record {
    pub(super) fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut bytes = serde_json::to_vec(self)?;
        bytes.push(b'\n');
        ensure!(
            bytes.len() <= MAX_RECORD_BYTES,
            "The software request record is too large"
        );
        Ok(bytes)
    }

    fn parse(bytes: &[u8]) -> Option<Record> {
        if bytes.len() > MAX_RECORD_BYTES {
            return None;
        }
        let mut lines = bytes.split_inclusive(|byte| *byte == b'\n');
        let base = grkit_core::effects::parse_json(lines.next()?).ok()?;
        let mut record: Record = serde_json::from_value(base).ok()?;
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
            let checkpoint: Checkpoint = serde_json::from_slice(line).ok()?;
            if !checkpoint.valid() {
                return None;
            }
            record.request = Some(checkpoint.request);
        }
        Some(record)
    }
}

impl Card {
    pub(super) fn read_record_bytes(&self) -> Result<Option<Vec<u8>>> {
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

    /// The record, or None without one. A record that does not parse while
    /// no journal is on the card is a base line that a power loss cut while
    /// the service staged an update: no request of it waits, so it reads as
    /// none, and the next update writes the record again.
    pub(super) fn read_record(&self) -> Result<Option<Record>> {
        let Some(bytes) = self.read_record_bytes()? else {
            return Ok(None);
        };
        match Record::parse(&bytes) {
            Some(record) => Ok(Some(record)),
            None if self.state()? == JournalState::None => Ok(None),
            None => bail!("The software transaction record is invalid"),
        }
    }

    pub(super) fn append_request(&self, request: SoftwareRequest) -> Result<()> {
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
            JournalState::Waiting => {
                request.phase = SoftwarePhase::AwaitingConfirmation;
                request.cancellable = true;
            }
            JournalState::Confirmed => {
                request.phase = SoftwarePhase::AwaitingRestart;
                request.cancellable = false;
            }
            JournalState::None => {
                self.settle(&mut request, &self.live_files()?);
                if request.status != SoftwareStatus::Running {
                    self.append_request(request.clone())?;
                    // No journal refers to the update folder any more. It
                    // holds a package that nobody confirmed: free its space
                    // now instead of at the next update.
                    let _ = self.clear(UPDATE_FOLDER, None);
                    // The old folder holds the files that the update
                    // replaced or removed, which have no later use. The
                    // record of the request stays.
                    if request.status == SoftwareStatus::Succeeded {
                        let _ = self.clear(BACKUP_FOLDER, Some(BACKUP_RECORD));
                    }
                }
            }
        }
        Ok(Some(request))
    }

    /// The outcome of a running request whose journal left the card, from
    /// the software `live` that the card holds. Software that is neither the
    /// old nor the new one, nor a signed release, stays uncertain.
    fn settle(&self, request: &mut SoftwareRequest, live: &Files) {
        let actual = SoftwareIdentity::of(live);
        let unchanged = actual == request.before;
        if actual == request.expected {
            request.end(SoftwareStatus::Succeeded, None);
        } else if unchanged && request.phase == SoftwarePhase::Cancelling {
            request.end(SoftwareStatus::Cancelled, None);
        } else if unchanged && request.phase == SoftwarePhase::AwaitingConfirmation {
            // Nothing on the camera removes a request that waits: it left
            // the card some other way, and the installed software is
            // unchanged.
            request.end(SoftwareStatus::Failed, Some(REQUEST_REMOVED));
        } else if self.holds_signed_release(live) {
            request.end(SoftwareStatus::Failed, Some(SOFTWARE_REPLACED));
        } else {
            request.phase = SoftwarePhase::Recovering;
            request.cancellable = false;
        }
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
            request.status == SoftwareStatus::Running && self.state()? == JournalState::Waiting,
            "The software request cannot be cancelled"
        );
        // The software of the card decides the outcome before anything
        // changes: a cancellation must not leave a card whose software is
        // unknown.
        let live = self.live_files()?;
        let replaced = SoftwareIdentity::of(&live) != request.before;
        ensure!(
            !replaced || self.holds_signed_release(&live),
            "The software on the card changed and is not a signed release"
        );
        request.phase = SoftwarePhase::Cancelling;
        request.cancellable = false;
        self.append_request(request.clone())?;
        // The RTOS renames the request when the person confirms it, with a
        // FAT driver of its own: Linux sees the folder as it last read it,
        // and an unlink writes that folder sector back. Read the folder from
        // the card again just before the unlink, so that the window between
        // the two drivers is as short as it can be. It is not closed: the
        // camera can still confirm between this read and the unlink.
        self.forget_card_reads();
        ensure!(
            self.state()? == JournalState::Waiting,
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
        if replaced {
            request.end(SoftwareStatus::Failed, Some(SOFTWARE_REPLACED));
        } else {
            request.end(SoftwareStatus::Cancelled, None);
        }
        self.append_request(request.clone())?;
        // The staged content stays for the next update to reclaim. VERSION_FILE
        // is harmless without a request and does not affect software identity.
        Ok(request)
    }
}
