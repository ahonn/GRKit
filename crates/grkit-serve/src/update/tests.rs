use super::{
    card::JournalState,
    stage::{fail_unstaged, version_file},
    *,
};
use grkit_core::{
    camera,
    card::{
        BACKUP_FOLDER, BACKUP_RECORD, DRAFT_FILE, JOURNAL_FILE, PACKAGE_FOLDER, PACKAGE_LIST,
        REQUEST_FILE, SCAN_FILE, UPDATE_FOLDER, VERSION_FILE,
    },
    packaging, release, runtime,
    runtime::update::update_paths,
    testing, Files,
};
use serde_json::json;
use std::{fs, io::Write};

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
    let card = test_card(dir.path());
    (dir, card)
}

/// The card at `root`, which trusts the key of the tests.
fn test_card(root: &std::path::Path) -> Card {
    Card::with_keys(root, vec![testing::release_key()])
}

/// Changes the card file `path` on a computer.
fn change(card: &Card, path: &str) {
    let mut bytes = fs::read(card.path(path)).unwrap();
    bytes.extend_from_slice(b"\n");
    fs::write(card.path(path), bytes).unwrap();
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
    let request = stage_request(&card, &next, &gr3(), &request_context()).unwrap();
    assert_eq!(request.status, SoftwareStatus::Running);
    assert_eq!(request.phase, SoftwarePhase::AwaitingConfirmation);
    assert!(request.cancellable);
    assert_ne!(request.expected, request.before);
    assert_eq!(
        test_card(dir.path())
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
        test_card(dir.path()).software_request().unwrap().unwrap(),
        complete
    );
    assert!(!complete
        .expected
        .content_hashes
        .keys()
        .any(|path| path.contains("COMMIT.")));
}

#[test]
fn an_update_of_the_same_version_removes_a_module() {
    let (_dir, card) = card_with(&testing::signed_package(&camera::GR3, "1.7.0", &["first"]));
    let runtimes = testing::release_runtimes(&camera::GR3, "1.7.0");
    let modules = testing::official_modules();
    let statement = release::Statement::of_release(&runtimes, &modules, &camera::GR3)
        .unwrap()
        .to_bytes()
        .unwrap();
    let signature = release::SigningKey::from_hex(testing::RELEASE_SEED)
        .unwrap()
        .sign(&statement);
    let kept = modules
        .into_iter()
        .map(|bytes| {
            let id = grkit_core::grm::Module::decode(&bytes).unwrap().id;
            (format!("{id}.GRM"), bytes)
        })
        .filter(|(name, _)| name != "GLOW.GRM")
        .collect();
    let runtime = release::package_runtime_files(&runtimes, &camera::GR3).unwrap();
    let zip = release::assemble_update(&statement, &signature, &runtime, &kept).unwrap();
    stage_request(&card, &zip, &gr3(), &request_context()).unwrap();
    fs::rename(card.path(REQUEST_FILE), card.path(JOURNAL_FILE)).unwrap();
    fs::rename(card.path(JOURNAL_FILE), card.path(REQUEST_FILE)).unwrap();
    run_journal(&card);
    assert_eq!(
        card.software_request().unwrap().unwrap().status,
        SoftwareStatus::Succeeded
    );
    let files = card.live_files().unwrap();
    assert!(!files.keys().any(|path| path.ends_with("/GLOW.GRM")));
    assert!(files.keys().any(|path| path.ends_with("/GRAIN.GRM")));
}

#[test]
fn a_request_whose_journal_was_not_written_fails_as_unstaged() {
    let (_dir, card) = card_with(&package(&["first"]));
    let request = stage_request(&card, &package(&["second"]), &gr3(), &request_context()).unwrap();
    // As when the rename of the journal failed after the checkpoint.
    fs::remove_file(card.path(REQUEST_FILE)).unwrap();
    fail_unstaged(&card, &request.request_id);
    let failed = card.software_request().unwrap().unwrap();
    assert_eq!(failed.status, SoftwareStatus::Failed);
    assert_eq!(failed.error.as_deref(), Some(STAGING_FAILED));
    let mut next = request_context();
    next.request_id = "650e8400-e29b-41d4-a716-446655440000".into();
    stage_request(&card, &package(&["second"]), &gr3(), &next).unwrap();
    assert_eq!(card.state().unwrap(), JournalState::Waiting);
}

#[test]
fn cancellation_is_targeted_durable_and_cannot_remove_a_confirmed_journal() {
    let (dir, card) = card_with(&package(&["first"]));
    let ctx = request_context();
    stage_request(&card, &package(&["second"]), &gr3(), &ctx).unwrap();
    assert!(card
        .cancel_software_request("650e8400-e29b-41d4-a716-446655440000")
        .is_err());
    assert_eq!(card.state().unwrap(), JournalState::Waiting);
    fs::rename(card.path(REQUEST_FILE), card.path(JOURNAL_FILE)).unwrap();
    assert!(card.cancel_software_request(&ctx.request_id).is_err());
    assert!(card.path(JOURNAL_FILE).is_file());
    fs::rename(card.path(JOURNAL_FILE), card.path(REQUEST_FILE)).unwrap();
    let cancelled = card.cancel_software_request(&ctx.request_id).unwrap();
    assert_eq!(cancelled.status, SoftwareStatus::Cancelled);
    assert_eq!(card.state().unwrap(), JournalState::None);
    assert_eq!(
        test_card(dir.path()).software_request().unwrap().unwrap(),
        cancelled
    );
    assert_eq!(
        card.cancel_software_request(&ctx.request_id).unwrap(),
        cancelled
    );
    // A card changed on a computer: its signed release ends the request,
    // and other software keeps the request that waits.
    let mut next = request_context();
    next.request_id = "650e8400-e29b-41d4-a716-446655440000".into();
    stage_request(&card, &package(&["third"]), &gr3(), &next).unwrap();
    let scan = read(&card, SCAN_FILE).unwrap();
    change(&card, SCAN_FILE);
    assert!(card.cancel_software_request(&next.request_id).is_err());
    assert_eq!(card.state().unwrap(), JournalState::Waiting);
    fs::write(card.path(SCAN_FILE), scan).unwrap();
    change(&card, PACKAGE_LIST);
    let replaced = card.cancel_software_request(&next.request_id).unwrap();
    assert_eq!(replaced.status, SoftwareStatus::Failed);
    assert_eq!(replaced.error.as_deref(), Some(SOFTWARE_REPLACED));
    assert_eq!(card.state().unwrap(), JournalState::None);
}

#[test]
fn a_request_that_left_the_card_ends_from_the_software_of_the_card() {
    let (dir, card) = card_with(&package(&["first"]));
    let mut ctx = request_context();
    stage_request(&card, &package(&["second"]), &gr3(), &ctx).unwrap();
    // The request leaves the card without an install.
    fs::remove_file(card.path(REQUEST_FILE)).unwrap();
    let removed = card.software_request().unwrap().unwrap();
    assert_eq!(removed.status, SoftwareStatus::Failed);
    assert_eq!(removed.error.as_deref(), Some(REQUEST_REMOVED));
    assert!(!removed.cancellable);
    // The outcome survives a restart of the service.
    assert_eq!(
        test_card(dir.path()).software_request().unwrap().unwrap(),
        removed
    );
    // A computer wrote a signed release: the request ends.
    ctx.request_id = "650e8400-e29b-41d4-a716-446655440000".into();
    stage_request(&card, &package(&["third"]), &gr3(), &ctx).unwrap();
    fs::remove_file(card.path(REQUEST_FILE)).unwrap();
    change(&card, PACKAGE_LIST);
    let replaced = card.software_request().unwrap().unwrap();
    assert_eq!(replaced.status, SoftwareStatus::Failed);
    assert_eq!(replaced.error.as_deref(), Some(SOFTWARE_REPLACED));
    // Software that no key signed is uncertain, and blocks updates.
    ctx.request_id = "750e8400-e29b-41d4-a716-446655440000".into();
    stage_request(&card, &package(&["fourth"]), &gr3(), &ctx).unwrap();
    fs::remove_file(card.path(REQUEST_FILE)).unwrap();
    change(&card, SCAN_FILE);
    let unknown = card.software_request().unwrap().unwrap();
    assert_eq!(unknown.status, SoftwareStatus::Running);
    assert_eq!(unknown.phase, SoftwarePhase::Recovering);
    ctx.request_id = "850e8400-e29b-41d4-a716-446655440000".into();
    assert!(stage_request(&card, &package(&["second"]), &gr3(), &ctx).is_err());
}

#[test]
fn interrupted_cancellation_recovers_without_overwriting_the_record() {
    let (dir, card) = card_with(&package(&["first"]));
    let ctx = request_context();
    let mut request = stage_request(&card, &package(&["second"]), &gr3(), &ctx).unwrap();
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
    let result = test_card(dir.path()).software_request().unwrap().unwrap();
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
    stage_request(&card, &package(&["second"]), &gr3(), &ctx).unwrap();
    run_journal(&card);
    // The journal moved the files that the update replaced or removed.
    assert!(files(&card, BACKUP_FOLDER).len() > 1);
    let complete = card.software_request().unwrap().unwrap();
    assert_eq!(complete.status, SoftwareStatus::Succeeded);
    assert_eq!(files(&card, BACKUP_FOLDER), [BACKUP_RECORD]);
    assert_eq!(
        test_card(dir.path()).software_request().unwrap().unwrap(),
        complete
    );
}

#[test]
fn a_noop_request_succeeds_and_an_invalid_context_writes_nothing() {
    let (_dir, card) = card_with(&package(&["first"]));
    let mut invalid = request_context();
    invalid.request_id = "not-a-uuid".into();
    assert!(stage_request(&card, &package(&["second"]), &gr3(), &invalid).is_err());
    assert!(!card.path(BACKUP_RECORD).exists());
    let next = package(&["second"]);
    stage_request(&card, &next, &gr3(), &request_context()).unwrap();
    run_journal(&card);
    let mut second = request_context();
    second.request_id = "650e8400-e29b-41d4-a716-446655440000".into();
    let noop = stage_request(&card, &next, &gr3(), &second).unwrap();
    assert_eq!(noop.status, SoftwareStatus::Succeeded);
    assert_eq!(noop.request_id, second.request_id);
    // Updates without a change replace the record instead of filling it.
    for n in 0..20 {
        second.request_id = format!("650e8400-e29b-41d4-a716-4466554400{n:02}");
        stage_request(&card, &next, &gr3(), &second).unwrap();
    }
}

#[test]
fn a_record_that_a_power_loss_cut_reads_as_no_request() {
    let (_dir, card) = card_with(&package(&["first"]));
    stage_request(&card, &package(&["second"]), &gr3(), &request_context()).unwrap();
    // The power went off while the service wrote the base line, before the
    // journal.
    fs::remove_file(card.path(REQUEST_FILE)).unwrap();
    fs::write(card.path(BACKUP_RECORD), b"{\"request\":{\"requ").unwrap();
    assert_eq!(card.software_request().unwrap(), None);
    let mut next = request_context();
    next.request_id = "650e8400-e29b-41d4-a716-446655440000".into();
    stage_request(&card, &package(&["second"]), &gr3(), &next).unwrap();
    assert_eq!(card.state().unwrap(), JournalState::Waiting);
}

#[test]
fn an_update_stages_only_the_files_that_change_and_keeps_the_looks() {
    let (_dir, card) = card_with(&package(&["test-effect"]));
    // The card also leaves a Look out.
    let mut list = read(&card, PACKAGE_LIST).unwrap();
    list.extend_from_slice(b"-LEFT.GRF\n");
    fs::write(card.path(PACKAGE_LIST), &list).unwrap();
    let next = package(&["other-effect"]);
    let summary = stage(&card, &next, &gr3()).unwrap();
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
    assert_eq!(card.state().unwrap(), JournalState::Waiting);
}

#[test]
fn an_update_keeps_the_files_that_no_package_has() {
    let (_dir, card) = card_with(&package(&["test-effect"]));
    fs::write(card.path("SCRIPT/GRKIT/IMPORTS.BIN"), b"imports").unwrap();
    stage(&card, &package(&["other-effect"]), &gr3()).unwrap();
    let journal = String::from_utf8(read(&card, REQUEST_FILE).unwrap()).unwrap();
    assert!(!journal.contains("IMPORTS.BIN"));
}

#[test]
fn a_new_update_replaces_the_files_of_the_last_one() {
    let (_dir, card) = card_with(&package(&["test-effect"]));
    stage(&card, &package(&["other-effect"]), &gr3()).unwrap();
    // The same package as the card: no update waits.
    let summary = stage(&card, &package(&["test-effect"]), &gr3()).unwrap();
    assert_eq!(summary["state"], "none");
    assert_eq!(summary["install"], 0);
    assert_eq!(summary["remove"], 0);
    assert_eq!(card.state().unwrap(), JournalState::None);
    let staged_effects =
        fs::read_dir(card.path(&format!("{UPDATE_FOLDER}/EFFECTS"))).map_or(0, Iterator::count);
    assert_eq!(staged_effects, 0);
}

#[test]
fn an_update_empties_the_backup_folder_for_its_files() {
    let (_dir, card) = card_with(&package(&["test-effect"]));
    fs::create_dir_all(card.path(BACKUP_FOLDER)).unwrap();
    fs::write(card.path(&format!("{BACKUP_FOLDER}/CORE1.BIN")), b"old").unwrap();
    stage(&card, &package(&["other-effect"]), &gr3()).unwrap();
    assert!(read(&card, &format!("{BACKUP_FOLDER}/CORE1.BIN")).is_none());
}

#[test]
fn the_version_file_has_the_version_that_the_core_names() {
    let core = |version: &str| {
        let mut binary = testing::installer();
        binary.extend_from_slice(format!("{}{version}\0", packaging::VERSION_MARKER).as_bytes());
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
    stage(&card, &package(&["other-effect"]), &gr3()).unwrap();
    assert_eq!(read(&card, VERSION_FILE).unwrap(), b"010701");
    card.cancel().unwrap();
    assert!(read(&card, VERSION_FILE).is_none());
}

#[test]
fn a_cancel_removes_the_journal() {
    let (_dir, card) = card_with(&package(&["test-effect"]));
    stage(&card, &package(&["other-effect"]), &gr3()).unwrap();
    assert!(card.cancel().unwrap());
    assert_eq!(card.state().unwrap(), JournalState::None);
    // A confirmed update can be removed before the next power-on too.
    stage(&card, &package(&["other-effect"]), &gr3()).unwrap();
    fs::rename(card.path(REQUEST_FILE), card.path(JOURNAL_FILE)).unwrap();
    assert_eq!(card.state().unwrap(), JournalState::Confirmed);
    assert!(card.cancel().unwrap());
    assert_eq!(card.state().unwrap(), JournalState::None);
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
        let message = format!("{:#}", stage(&card, &next, &device).unwrap_err());
        assert!(message.contains(error), "{message}");
    }
    // An HDF model has the package of its camera.
    let hdf = Device {
        model: "RICOH GR III HDF".into(),
        ..gr3()
    };
    stage(&card, &next, &hdf).unwrap();
}

#[test]
fn a_package_that_is_not_valid_or_does_not_fit_changes_nothing() {
    let (_dir, card) = card_with(&package(&["test-effect"]));
    let mut broken = package(&["other-effect"]);
    let at = broken.len() / 2;
    broken[at] ^= 0xff;
    assert!(stage(&card, &broken, &gr3()).is_err());
    let full = Device {
        free_bytes: Some(1024),
        ..gr3()
    };
    let message = format!(
        "{:#}",
        stage(&card, &package(&["other-effect"]), &full).unwrap_err()
    );
    assert!(message.contains("more bytes"), "{message}");
    assert_eq!(card.state().unwrap(), JournalState::None);
    assert!(!card.path(UPDATE_FOLDER).exists());
}

#[test]
fn only_a_package_that_a_release_key_signed_is_staged() {
    let (_dir, card) = card_with(&package(&["test-effect"]));
    let next = package(&["other-effect"]);
    // Another key.
    let message = format!(
        "{:#}",
        stage(&Card::new(card.root()), &next, &gr3()).unwrap_err()
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
    let message = format!("{:#}", stage(&card, &unsigned, &gr3()).unwrap_err());
    assert!(message.contains("not signed"), "{message}");
    assert_eq!(card.state().unwrap(), JournalState::None);
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
    card.forget_summary();
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
