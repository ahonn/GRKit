//! Staging an update: check the package, write the files that change to the
//! update folder, and write the journal.

use super::{
    card::{Card, JournalState},
    record::{
        Record, SoftwareContext, SoftwareIdentity, SoftwarePhase, SoftwareRequest, SoftwareStatus,
        STAGING_FAILED,
    },
    summary::version_of,
    CardNotReady,
};
use anyhow::{bail, ensure, Context, Result};
use grkit_core::{
    card::{
        BACKUP_FOLDER, BACKUP_RECORD, DRAFT_FILE, PACKAGE_FOLDER, PACKAGE_LIST, REQUEST_FILE,
        STARTUP_FILE, UPDATE_FOLDER, VERSION_FILE,
    },
    packaging::{in_effect_inbox, package_owned, PackageList},
    release,
    runtime::update::{journal, update_paths, Change},
    Files,
};
use serde_json::{json, Value};
use std::fs;

/// The largest package that the service takes. A release package has about
/// 5 MB; the service keeps the package and its entries in memory while it
/// checks them.
pub const MAX_PACKAGE: usize = 16 * 1024 * 1024;
/// The free space that the card keeps after the update folder.
const SPARE_BYTES: u64 = 4 * 1024 * 1024;

/// What the service knows of the camera: the model and the firmware version
/// that the camera's own HTTP API gives, and the free space of the card.
pub struct Device {
    pub model: String,
    pub firmware: String,
    pub free_bytes: Option<u64>,
    pub serial_number: Option<String>,
}

/// Checks the package `zip` for `device`, writes its changes to the update
/// folder, and writes the journal. Returns what the update does.
pub fn stage(card: &Card, zip: &[u8], device: &Device) -> Result<Value> {
    stage_inner(card, zip, device, None)
}

pub fn stage_request(
    card: &Card,
    zip: &[u8],
    device: &Device,
    context: &SoftwareContext,
) -> Result<SoftwareRequest> {
    ensure!(context.valid(), "Invalid software request context");
    ensure!(
        card.state()? == JournalState::None,
        "A software request is already pending"
    );
    if let Some(previous) = card.software_request()? {
        ensure!(
            previous.status != SoftwareStatus::Running,
            "Recover the previous software request before writing"
        );
    }
    if let Err(error) = stage_inner(card, zip, device, Some(context)) {
        fail_unstaged(card, &context.request_id);
        return Err(error);
    }
    card.software_request()?
        .context("The software request was not retained")
}

/// A request whose journal never reached the card did not wait for a
/// confirmation: record its own failure, so that it does not read as one
/// that left the card.
pub(super) fn fail_unstaged(card: &Card, request_id: &str) {
    if card.state().ok() != Some(JournalState::None) {
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
    context: Option<&SoftwareContext>,
) -> Result<Value> {
    ensure!(zip.len() <= MAX_PACKAGE, "The package is too large");
    let (update, files) =
        release::verify_update(zip, card.release_keys()).context("Check the package")?;
    check_camera(&update.statement, device)?;
    // The package folder and the startup script; the update keeps the
    // ModeSet files of the package off the card, as SD scripts are on.
    // An update carries no Looks: the phone manages them on their own. The
    // update leaves every Look file of the card as it is, and the new package
    // list keeps the Looks that the card lists and leaves out.
    let mut new: Files = files
        .into_iter()
        .filter(|(path, _)| {
            (path == STARTUP_FILE || path.starts_with(&format!("{PACKAGE_FOLDER}/")))
                && !in_effect_inbox(path)
        })
        .collect();
    let live = card.package_files().context(CardNotReady)?;
    let live_startup = fs::read(card.path(STARTUP_FILE)).ok();
    if let Some(list) = new.get_mut(PACKAGE_LIST) {
        let card_list = live
            .get(PACKAGE_LIST)
            .map(|bytes| PackageList::parse(bytes))
            .unwrap_or_default();
        *list = PackageList::parse(list)
            .with_looks_of(&card_list)
            .to_bytes();
    }
    let changes = changes(&live, &new, live_startup.as_deref());
    let startup_path = STARTUP_FILE.to_owned();
    let request = context.map(|context| SoftwareRequest {
        request_id: context.request_id.to_ascii_lowercase(),
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
        // The card after the update: the new package and the Looks it keeps.
        expected: SoftwareIdentity::of(
            new.iter()
                .chain(live.iter().filter(|(path, _)| in_effect_inbox(path))),
        ),
        before: SoftwareIdentity::of(
            live.iter()
                .chain(live_startup.as_ref().map(|bytes| (&startup_path, bytes))),
        ),
        error: None,
    });
    let record = Record {
        request: request.clone(),
    }
    .to_bytes()?;
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
    let summary = |state: JournalState, installs: usize| {
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
        card.clear(UPDATE_FOLDER, None)?;
        // The request ends at once: its record replaces the last one, so
        // that repeated updates without a change never fill the record.
        if request.is_some() {
            card.write_checked(BACKUP_RECORD, &record)?;
        }
        return Ok(summary(JournalState::None, 0));
    }
    card.forget_deleted(&[VERSION_FILE, DRAFT_FILE, REQUEST_FILE]);
    card.warm(
        staged
            .iter()
            .map(|(path, _)| path.as_str())
            .chain([BACKUP_RECORD]),
    );
    card.clear(UPDATE_FOLDER, None)?;
    card.clear(BACKUP_FOLDER, None)?;
    for (path, data) in &staged {
        card.write_checked(path, data)?;
    }
    // The record of the request is on the card before the camera can see the
    // journal.
    card.write_checked(BACKUP_RECORD, &record)?;
    // The camera sees the journal only when it is complete, and runs it only
    // after the confirmation on the camera. It reads the version first.
    card.write_checked(VERSION_FILE, &version_file(&new))?;
    card.write_checked(DRAFT_FILE, &journal(&changes)?)?;
    fs::rename(card.path(DRAFT_FILE), card.path(REQUEST_FILE)).context("Write the journal")?;
    ensure!(
        card.state()? == JournalState::Waiting,
        "The journal is not on the card"
    );
    Ok(summary(JournalState::Waiting, staged.len()))
}

/// Checks that the release `statement` is for the camera of `device`, and
/// that it signs the firmware of the camera: the runtime of the release
/// does not load on another firmware.
fn check_camera(statement: &release::Statement, device: &Device) -> Result<()> {
    let Some(camera) = grkit_core::camera::of_api_model(&device.model) else {
        bail!("GRKit does not support the {}", device.model);
    };
    ensure!(
        statement.camera == camera,
        "The package is for the {}, not the {}",
        statement.camera.name,
        camera.name
    );
    ensure!(
        statement.firmware.contains(&device.firmware),
        "The package does not support firmware {} of the {}",
        device.firmware,
        camera.name
    );
    Ok(())
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
        if !new.contains_key(path) && package_owned(path) && !in_effect_inbox(path) {
            changes.push(Change::Remove(path.clone()));
        }
    }
    changes
}

/// The version file of the package files `files` (`card::VERSION_FILE`).
pub(super) fn version_file(files: &Files) -> Vec<u8> {
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
