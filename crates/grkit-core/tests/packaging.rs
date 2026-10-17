use std::io::{Cursor, Read, Write};

use grkit_core::{camera, effects, modules::Hidden, packaging::*, runtime, Files};
use serde_json::{json, Value};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};

fn runtime_fixture() -> Files {
    let mut binary = vec![0; 52];
    binary[..6].copy_from_slice(b"\x7fELF\x01\x01");
    binary[16..20].copy_from_slice(b"\x02\x00\x28\x00");
    runtime::runtime_files(&binary)
}

fn build() -> Value {
    json!({"target": "gr3-2.10", "profile": "production", "startup_activation": "Off"})
}

fn effect(id: &str) -> (String, Vec<u8>) {
    let curve: Vec<_> = (0..256).map(|i| i * 64).collect();
    let value = json!({"effect_id": id, "name": "Test", "flags": 0,
        "matrix": [8192, 0, 0, 0, 8192, 0, 0, 0, 8192],
        "curves": [&curve, &curve, &curve]});
    (
        effects::filename(id).unwrap(),
        effects::encode(&value, &vec![0; 6400]).unwrap(),
    )
}

fn package() -> Vec<u8> {
    assemble(
        &runtime_fixture(),
        &build(),
        &Files::from([effect("test-effect")]),
        Hidden::default(),
        &camera::GR3,
    )
    .unwrap()
}

fn with_sums(mut files: Files) -> Files {
    let sums = files
        .iter()
        .filter(|(name, _)| name.as_str() != "SHA256SUMS")
        .map(|(name, bytes)| format!("{}  {name}\n", sha256(bytes)))
        .collect::<String>();
    files.insert("SHA256SUMS".into(), sums.into_bytes());
    files
}

fn unpack(bytes: &[u8]) -> Files {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut files = Files::new();
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).unwrap();
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).unwrap();
        files.insert(entry.name().into(), bytes);
    }
    files
}

fn repack(files: &Files, compression: CompressionMethod) -> Vec<u8> {
    let mut archive = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, blob) in files {
        archive
            .start_file(
                name,
                SimpleFileOptions::default().compression_method(compression),
            )
            .unwrap();
        archive.write_all(blob).unwrap();
    }
    archive.finish().unwrap().into_inner()
}

#[test]
fn package_round_trip_is_deterministic_and_preserves_metadata() {
    let first = package();
    assert_eq!(first, package());
    let verified = verify_package(&first).unwrap();
    assert_eq!(verified["effect_ids"], json!(["test-effect"]));
    assert_eq!(verified["target"], json!("gr3-2.10"));
    let mut archive = ZipArchive::new(Cursor::new(first)).unwrap();
    for index in 0..archive.len() {
        let entry = archive.by_index(index).unwrap();
        assert_eq!(entry.compression(), CompressionMethod::Stored);
        assert_eq!(entry.last_modified().unwrap().year(), 2026);
        let expected = if entry.name().ends_with(".BIN") || entry.name().ends_with(".SH") {
            0o100755
        } else {
            0o100644
        };
        assert_eq!(entry.unix_mode(), Some(expected));
    }
}

#[test]
fn packages_ship_only_the_guide_and_checksums() {
    let files = unpack(&package());
    let documents: Vec<_> = files
        .keys()
        .filter(|name| !name.starts_with("SD_ROOT/"))
        .map(String::as_str)
        .collect();
    assert_eq!(documents, ["INSTALLATION.md", "SHA256SUMS"]);
}

#[test]
fn manifests_notices_and_extra_documents_are_rejected() {
    let files = unpack(&package());
    assert!(verify_package(&repack(
        &with_sums(files.clone()),
        CompressionMethod::Stored
    ))
    .is_ok());
    for name in ["MANIFEST.json", "NOTICES/EFFECTS.md", "RECOVERY.md"] {
        let mut extra = files.clone();
        extra.insert(name.into(), b"{}".to_vec());
        assert!(
            verify_package(&repack(&with_sums(extra), CompressionMethod::Stored)).is_err(),
            "{name}"
        );
    }
}

#[test]
fn legacy_deflated_packages_remain_supported() {
    let files = unpack(&package());
    assert!(verify_package(&repack(&files, CompressionMethod::Deflated)).is_ok());
}

#[test]
fn corrupted_profiles_documents_manifests_and_sums_are_rejected() {
    let original = unpack(&package());
    let profile = original.keys().find(|name| name.ends_with(".GRF")).unwrap();
    for name in [profile.as_str(), "INSTALLATION.md", "SHA256SUMS"] {
        let mut files = original.clone();
        files.get_mut(name).unwrap().push(b' ');
        assert!(
            verify_package(&repack(&files, CompressionMethod::Stored)).is_err(),
            "{name}"
        );
    }
    let mut bytes = package();
    bytes.truncate(bytes.len() / 2);
    assert!(verify_package(&bytes).is_err());
}

#[test]
fn paths_and_unexpected_entries_are_rejected() {
    for path in [
        "",
        "/absolute",
        "../outside",
        "a/../b",
        "a/./b",
        "a//b",
        "a/",
        "a\\b",
        "C:a",
        "a\0b",
        "中文",
    ] {
        assert!(safe_path(path).is_err(), "{path:?}");
    }
    for name in [
        "../outside",
        "SD_ROOT/EXTRA",
        "installation.md",
        "NOTICES/nested/extra",
    ] {
        let mut files = unpack(&package());
        files.insert(name.into(), b"bad".to_vec());
        assert!(
            verify_package(&repack(&files, CompressionMethod::Stored)).is_err(),
            "{name}"
        );
    }
}

#[test]
fn entry_limits_and_required_files_are_enforced() {
    let mut files = unpack(&package());
    files.remove("SD_ROOT/DEVELOP.MOD");
    assert!(validate_contents(&files).is_err());
    let mut files = unpack(&package());
    files.insert("EXTRA/huge".into(), vec![0; MAX_FILE_SIZE + 1]);
    assert!(validate_contents(&files).is_err());
    let mut files = unpack(&package());
    files.insert("EXTRA/one".into(), vec![0; MAX_FILE_SIZE]);
    files.insert("EXTRA/two".into(), vec![0; MAX_FILE_SIZE]);
    assert!(validate_contents(&files).is_err());
    let mut files = unpack(&package());
    for index in 0..MAX_ENTRIES {
        files.insert(format!("EXTRA/{index}"), vec![]);
    }
    assert!(verify_package(&repack(&files, CompressionMethod::Stored)).is_err());
}

#[test]
fn production_boundaries_are_enforced() {
    for (key, value) in [
        ("profile", "diagnostic"),
        ("startup_activation", "test-effect"),
        ("target", "other"),
    ] {
        let mut altered = build();
        altered[key] = json!(value);
        assert!(assemble(
            &runtime_fixture(),
            &altered,
            &Files::new(),
            Hidden::default(),
            &camera::GR3
        )
        .is_err());
    }
    for suffix in [b"--select test-effect".as_slice(), b"changed"] {
        let mut altered = runtime_fixture();
        altered
            .get_mut("SCRIPT/STARTUP.TTL")
            .unwrap()
            .extend_from_slice(suffix);
        assert!(validate_runtime(&altered).is_err());
    }
}

#[test]
fn target_capacity_and_empty_effect_selection_are_supported() {
    let mut effects: Files = (0..runtime::MAX_COUNT)
        .map(|i| effect(&format!("effect-{i}")))
        .collect();
    let bytes = assemble(
        &runtime_fixture(),
        &build(),
        &effects,
        Hidden::default(),
        &camera::GR3,
    )
    .unwrap();
    assert_eq!(
        verify_package(&bytes).unwrap()["effect_ids"]
            .as_array()
            .unwrap()
            .len(),
        runtime::MAX_COUNT
    );
    effects.extend([effect("effect-overflow")]);
    assert!(assemble(
        &runtime_fixture(),
        &build(),
        &effects,
        Hidden::default(),
        &camera::GR3
    )
    .is_err());
    let bytes = assemble(
        &runtime_fixture(),
        &build(),
        &Files::new(),
        Hidden::default(),
        &camera::GR3,
    )
    .unwrap();
    assert_eq!(verify_package(&bytes).unwrap()["effect_ids"], json!([]));
}

#[test]
fn hidden_modules_are_packaged_and_verified() {
    let effects = Files::from([effect("test-effect")]);
    let hidden = Hidden::from_names(["date-stamp", "framing"]).unwrap();
    let bytes = assemble(&runtime_fixture(), &build(), &effects, hidden, &camera::GR3).unwrap();
    let files = unpack(&bytes);
    assert_eq!(
        files["SD_ROOT/SCRIPT/PLUGINS/MODULES.TXT"],
        b"framing\ndate-stamp\n"
    );
    assert_eq!(
        verify_package(&bytes).unwrap()["hidden_modules"],
        json!(["framing", "date-stamp"])
    );
    // A package that hides nothing has no file and keeps its earlier bytes.
    let plain = assemble(
        &runtime_fixture(),
        &build(),
        &effects,
        Hidden::default(),
        &camera::GR3,
    )
    .unwrap();
    assert!(!unpack(&plain).contains_key("SD_ROOT/SCRIPT/PLUGINS/MODULES.TXT"));
    assert_eq!(verify_package(&plain).unwrap()["hidden_modules"], json!([]));
    assert_eq!(plain, package());
}

#[test]
fn invalid_module_files_and_empty_menus_are_rejected() {
    for text in [
        b"image-control\n".as_slice(),
        b"guides guides",
        b"Framing",
        b"framing,guides",
        b"\xff",
    ] {
        let mut files = unpack(&package());
        files.insert("SD_ROOT/SCRIPT/PLUGINS/MODULES.TXT".into(), text.to_vec());
        assert!(
            validate_contents(&with_sums(files)).is_err(),
            "{}",
            String::from_utf8_lossy(text)
        );
    }
    let mut files = unpack(&package());
    files.insert(
        "SD_ROOT/SCRIPT/PLUGINS/MODULES.TXT".into(),
        b" guides\r\n\tframing ".to_vec(),
    );
    assert_eq!(
        validate_contents(&with_sums(files)).unwrap().hidden.names(),
        ["framing", "guides"]
    );
    let all = Hidden::from_names(grkit_core::modules::NAMES).unwrap();
    assert!(assemble(
        &runtime_fixture(),
        &build(),
        &Files::new(),
        all,
        &camera::GR3
    )
    .is_err());
    let bytes = assemble(
        &runtime_fixture(),
        &build(),
        &Files::from([effect("test-effect")]),
        all,
        &camera::GR3,
    )
    .unwrap();
    assert_eq!(
        verify_package(&bytes).unwrap()["hidden_modules"],
        json!(grkit_core::modules::NAMES)
    );
}

#[test]
fn malformed_manifest_records_are_rejected_without_panicking() {
    for value in [
        json!([]),
        json!(null),
        json!(42),
        json!({"files": null}),
        json!({"files": [null]}),
        json!({"files": [{"path": "x", "size": true, "sha256": "0".repeat(64)}]}),
        json!({"files": [{"path": "x", "size": -1, "sha256": "0".repeat(64)}]}),
        json!({"files": [{"path": "x", "size": 1.0, "sha256": "0".repeat(64)}]}),
    ] {
        assert!(
            read_manifest(&json_bytes(&value).unwrap()).is_err(),
            "{value}"
        );
    }
    assert!(read_manifest(br#"{"files":[],"files":[]}"#).is_err());
    assert!(read_manifest(
        &json_bytes(&json!({"files": [
            {"path": "X", "size": 0, "sha256": "0".repeat(64)},
            {"path": "x", "size": 0, "sha256": "0".repeat(64)}
        ]}))
        .unwrap()
    )
    .is_err());
}

#[test]
fn json_matches_existing_ascii_pretty_format() {
    assert_eq!(
        json_bytes(&json!({"z": [1, true], "a": "中😀\u{7f}"})).unwrap(),
        b"{\n  \"a\": \"\\u4e2d\\ud83d\\ude00\\u007f\",\n  \"z\": [\n    1,\n    true\n  ]\n}\n"
    );
}

#[test]
fn runtime_layout_matches_target_capacity() {
    assert_eq!(runtime::MAX_COUNT, 20);
    assert_eq!(
        runtime::layout(0).unwrap(),
        json!({"words": 35408, "icon_words": [25632]})
    );
    assert_eq!(
        runtime::layout(20).unwrap(),
        json!({"words": 44688, "icon_words": [57672]})
    );
    assert!(runtime::layout(21).is_err());
}

#[test]
fn allocation_includes_host_icons_at_chunk_boundaries() {
    for count in 0..=runtime::MAX_COUNT {
        let layout = runtime::layout(count).unwrap();
        let words = layout["words"].as_u64().unwrap() as usize;
        let icons: Vec<_> = layout["icon_words"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as usize)
            .collect();
        // Picture scratch for the date stamp masks.
        assert_eq!(words * 4, 256 + count * 0x740 + 141376, "count {count}");
        assert_eq!(
            icons.iter().sum::<usize>() * 4,
            (count + 16) * 6408,
            "count {count}"
        );
        assert!(std::iter::once(&words)
            .chain(&icons)
            .all(|v| (1..=65536).contains(v)));
        // The capacity fits one 40-icon chunk with the sixteen host icons.
        assert_eq!(icons.len(), 1);
    }
    assert!(runtime::layout(usize::MAX).is_err());
}

#[test]
fn duplicate_central_directory_entries_are_not_hidden_by_the_zip_index() {
    let original = package();
    let end = original.len() - 22;
    assert_eq!(&original[end..end + 4], b"PK\x05\x06");
    let start = u32::from_le_bytes(original[end + 16..end + 20].try_into().unwrap()) as usize;
    let variable: usize = [28, 30, 32]
        .iter()
        .map(|offset| {
            u16::from_le_bytes(
                original[start + offset..start + offset + 2]
                    .try_into()
                    .unwrap(),
            ) as usize
        })
        .sum();
    let record = &original[start..start + 46 + variable];
    let mut mutated = original[..end].to_vec();
    mutated.extend_from_slice(record);
    mutated.extend_from_slice(&original[end..]);
    let new_end = end + record.len();
    for offset in [8, 10] {
        let count = u16::from_le_bytes(
            mutated[new_end + offset..new_end + offset + 2]
                .try_into()
                .unwrap(),
        );
        mutated[new_end + offset..new_end + offset + 2].copy_from_slice(&(count + 1).to_le_bytes());
    }
    let size = u32::from_le_bytes(mutated[new_end + 12..new_end + 16].try_into().unwrap());
    mutated[new_end + 12..new_end + 16]
        .copy_from_slice(&(size + record.len() as u32).to_le_bytes());
    assert!(verify_package(&mutated).is_err());
}

#[test]
fn symlink_and_encrypted_entries_are_rejected() {
    for encrypted in [false, true] {
        let mut bytes = package();
        let end = bytes.len() - 22;
        let central = u32::from_le_bytes(bytes[end + 16..end + 20].try_into().unwrap()) as usize;
        if encrypted {
            bytes[central + 8] |= 1;
        } else {
            bytes[central + 5] = 3;
            bytes[central + 38..central + 42].copy_from_slice(&(0o120777u32 << 16).to_le_bytes());
        }
        assert!(verify_package(&bytes).is_err());
    }
}

#[test]
fn runtime_builds_must_match_their_manifest_exactly() {
    let files = runtime_fixture();
    let manifest = |files: &Files| {
        let mut value = build();
        value["files"] = files
            .iter()
            .map(
                |(path, bytes)| json!({"path": path, "size": bytes.len(), "sha256": sha256(bytes)}),
            )
            .collect();
        json_bytes(&value).unwrap()
    };
    let (runtime, loaded) = load_runtime(&manifest(&files), &files).unwrap();
    assert_eq!(runtime, files);
    assert_eq!(loaded["target"], json!("gr3-2.10"));

    let mut extra = files.clone();
    extra.insert("SCRIPT/EXTRA.SH".into(), b"x".to_vec());
    let mut missing = files.clone();
    missing.remove("SCRIPT/STARTUP.TTL");
    let mut changed = files.clone();
    changed.insert("SCRIPT/STARTUP.TTL".into(), b"changed".to_vec());
    for (actual, expected) in [
        (&extra, "differs from its manifest"),
        (&missing, "differs from its manifest"),
        (&changed, "hash or size mismatch"),
    ] {
        let message = format!("{:#}", load_runtime(&manifest(&files), actual).unwrap_err());
        assert!(message.contains(expected), "{message}");
    }
    let unknown = format!("{:#}", load_runtime(&manifest(&extra), &extra).unwrap_err());
    assert!(
        unknown.contains("Invalid runtime manifest entries"),
        "{unknown}"
    );
}

#[test]
fn each_camera_gets_its_modeset_file_and_guide() {
    let effects = Files::from([effect("test-effect")]);
    let gr3 = unpack(&package());
    assert_eq!(
        gr3["INSTALLATION.md"],
        include_bytes!("../../../resources/INSTALLATION.md")
    );
    let bytes = assemble(
        &runtime_fixture(),
        &build(),
        &effects,
        Hidden::default(),
        &camera::GR3X,
    )
    .unwrap();
    let verified = verify_package(&bytes).unwrap();
    assert_eq!(verified["camera"], "gr3x");
    assert_eq!(verified["target"], "gr3-2.10");
    let gr3x = unpack(&bytes);
    assert!(gr3x.contains_key("SD_ROOT/00078490.609"));
    assert!(!gr3x.contains_key("SD_ROOT/00078350.588"));
    let guide = String::from_utf8(gr3x["INSTALLATION.md"].clone()).unwrap();
    assert!(guide.contains("For RICOH GR IIIx firmware 1.60 only."));
    assert!(guide.contains("`00078490.609`") && !guide.contains("00078350.588"));
    // Every runtime and effect file is the same for both cameras.
    for (name, blob) in &gr3 {
        if !name.ends_with(".588") && name != "INSTALLATION.md" && name != "SHA256SUMS" {
            assert_eq!(&gr3x[name], blob, "{name}");
        }
    }
}

#[test]
fn the_guide_names_the_camera_once_and_the_modeset_file_once() {
    // installation_guide replaces these strings, so each must occur exactly once.
    let guide = include_str!("../../../resources/INSTALLATION.md");
    assert_eq!(guide.matches("GR III firmware 2.10").count(), 1);
    assert_eq!(guide.matches(camera::GR3.modeset).count(), 1);
}

#[test]
fn a_package_has_the_modeset_file_of_exactly_one_camera() {
    let mut files = unpack(&package());
    files.insert(
        "SD_ROOT/00078490.609".into(),
        files["SD_ROOT/00078350.588"].clone(),
    );
    assert!(validate_contents(&with_sums(files)).is_err());
    let mut files = unpack(&package());
    files.remove("SD_ROOT/00078350.588");
    assert!(validate_contents(&with_sums(files)).is_err());
    let mut files = unpack(&package());
    files.insert("SD_ROOT/00078350.588".into(), b"changed".to_vec());
    assert!(validate_contents(&with_sums(files)).is_err());
    assert!(camera::find("gr4").is_err());
    assert_eq!(camera::find("gr3x").unwrap(), &camera::GR3X);
}

#[test]
fn the_loader_waits_before_its_first_request() {
    let loader = String::from_utf8(runtime::loader()).unwrap();
    let first_request = loader.find("sendln").unwrap();
    let wait = loader
        .find(&format!("mpause {}", runtime::FIRST_REQUEST_DELAY_MS))
        .unwrap();
    assert!(wait < first_request);
    const { assert!(runtime::FIRST_REQUEST_DELAY_MS >= 3000) };
}
