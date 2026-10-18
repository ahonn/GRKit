use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::Path;

use grkit_core::{camera, effects, modules::Hidden, packaging::*, runtime, Files};
use serde_json::{json, Value};
use zip::{write::SimpleFileOptions, ZipArchive, ZipWriter};

fn package() -> Vec<u8> {
    let mut binary = vec![0; 52];
    binary[..6].copy_from_slice(b"\x7fELF\x01\x01");
    binary[16..20].copy_from_slice(b"\x02\x00\x28\x00");
    let curve: Vec<_> = (0..256).map(|i| i * 64).collect();
    let effect = json!({"effect_id": "test-effect", "name": "Test", "flags": 0,
        "matrix": [8192, 0, 0, 0, 8192, 0, 0, 0, 8192],
        "curves": [&curve, &curve, &curve]});
    assemble(
        &runtime::runtime_files(&binary),
        &json!({"target": "gr3-2.10", "profile": "production", "startup_activation": "Off"}),
        &Files::from([(
            effects::filename("test-effect").unwrap(),
            effects::encode(&effect, &vec![0; 6400]).unwrap(),
        )]),
        Hidden::default(),
        &camera::GR3,
    )
    .unwrap()
}

fn unpack(bytes: &[u8]) -> Files {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).unwrap();
    (0..archive.len())
        .map(|index| {
            let mut entry = archive.by_index(index).unwrap();
            let mut blob = Vec::new();
            entry.read_to_end(&mut blob).unwrap();
            (entry.name().to_owned(), blob)
        })
        .collect()
}

fn repack(files: &Files) -> Vec<u8> {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, blob) in files {
        zip.start_file(name, SimpleFileOptions::default()).unwrap();
        zip.write_all(blob).unwrap();
    }
    zip.finish().unwrap().into_inner()
}

/// The lines after the heading `heading`, of any level, up to the next heading.
fn section(text: &str, heading: &str) -> String {
    let is_heading = |line: &str| line.starts_with('#');
    let mut lines = text.lines();
    lines
        .find(|line| is_heading(line) && line.trim_start_matches('#').trim() == heading)
        .unwrap_or_else(|| panic!("No section {heading}"));
    lines
        .take_while(|line| !is_heading(line))
        .collect::<Vec<_>>()
        .join("\n")
}

fn catalog() -> Vec<Value> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../effects");
    let mut definitions: Vec<Value> = fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().path().join("effect.json"))
        .filter(|path| path.is_file())
        .map(|path| serde_json::from_slice(&fs::read(path).unwrap()).unwrap())
        .collect();
    definitions.sort_by(|a, b| a["effect_id"].as_str().cmp(&b["effect_id"].as_str()));
    definitions
}

#[test]
fn packages_carry_the_license_notice() {
    let files = unpack(&package());
    assert_eq!(files["LICENSES.md"], LICENSES.as_bytes());
    for notice in [
        "CC BY-SA 4.0",
        "https://creativecommons.org/licenses/by-sa/4.0/",
        "Copyright (c) 2026 ahonn",
        "Copyright (c) 2026 Jan Lohse",
        "Changes to third-party LUTs",
    ] {
        assert!(LICENSES.contains(notice), "{notice}");
    }
}

#[test]
fn packages_without_the_license_notice_are_rejected() {
    let mut files = unpack(&package());
    files.remove("LICENSES.md");
    let sums = files
        .iter()
        .filter(|(name, _)| name.as_str() != "SHA256SUMS")
        .map(|(name, blob)| format!("{}  {name}\n", sha256(blob)))
        .collect::<String>();
    files.insert("SHA256SUMS".into(), sums.into_bytes());
    assert!(verify_package(&repack(&files)).is_err());
}

#[test]
fn each_catalog_effect_has_the_attribution_of_its_source() {
    let notice = include_str!("../../../effects/NOTICE.md");
    let definitions = catalog();
    assert!(!definitions.is_empty());
    for definition in &definitions {
        let name = definition["name"].as_str().unwrap();
        let attribution = &definition["attribution"];
        let generator = attribution["generator"].as_str().unwrap();
        let (heading, license) = if generator.starts_with("spektrafilm ") {
            ("spektrafilm", "CC-BY-SA-4.0")
        } else if generator.starts_with("Spectral Film LUT ") {
            ("Spectral Film LUT", "MIT")
        } else {
            panic!("{name}: no attribution for {generator}");
        };
        assert_eq!(attribution["license"], license, "{name}");
        // The package notice and the repository notice name the same effects.
        for (document, text) in [("LICENSES.md", LICENSES), ("NOTICE.md", notice)] {
            assert!(
                section(text, heading).contains(name),
                "{document} does not name {name} under {heading}"
            );
        }
    }
}
