use grkit_core::{packaging, runtime, testing, Files};
use grkit_wasm::Builder;
use serde_json::{json, Value};

fn runtime_files() -> Files {
    testing::runtime_files(&testing::installer(), "gr3-2.10")
}

fn manifest(files: &Files) -> Vec<u8> {
    testing::manifest("gr3-2.10", files)
}

use testing::effect;

/// A stand-in for a guide PDF, which the site fetches.
const PDF: &[u8] = b"%PDF-1.7\n% GRKit test guide\n";

/// A builder with the runtime and the guide, and no module.
fn builder_without_modules(files: &Files) -> Builder {
    let mut builder = Builder::new(&manifest(files)).unwrap();
    for path in builder.runtime_paths(0).unwrap().to_vec() {
        builder.add_runtime_file(0, &path, &files[&path]).unwrap();
    }
    builder.set_guide(PDF);
    builder
}

/// A builder with the runtime, the guide, and each official module.
fn builder_with_runtime(files: &Files) -> Builder {
    let mut builder = builder_without_modules(files);
    for module in testing::official_modules() {
        builder.add_module(&module).unwrap();
    }
    builder
}

fn error(result: anyhow::Result<impl Sized>) -> String {
    format!("{:#}", result.err().expect("expected an error"))
}

#[test]
fn builds_a_verified_package_from_a_published_runtime() {
    let files = runtime_files();
    let mut builder = builder_with_runtime(&files);
    let paths: Vec<_> = files.keys().cloned().collect();
    assert_eq!(builder.runtime_paths(0).unwrap(), paths.as_slice());
    assert_eq!(builder.add_effect(&effect("test-a")).unwrap(), "test-a");
    assert_eq!(builder.add_effect(&effect("test-b")).unwrap(), "test-b");

    let package = builder.build().unwrap();
    assert_eq!(package, builder.build().unwrap());
    let verified = packaging::verify_package(&package).unwrap();
    assert_eq!(verified.effect_ids, ["test-a", "test-b"]);
}

#[test]
fn a_package_without_effects_is_valid() {
    let package = builder_with_runtime(&runtime_files()).build().unwrap();
    assert_eq!(
        packaging::verify_package(&package).unwrap().effect_ids,
        Vec::<String>::new()
    );
}

#[test]
fn modules_are_added_once_and_checked() {
    let mut builder = builder_without_modules(&runtime_files());
    // Image Control shows nothing without effects.
    assert_eq!(
        builder.add_module(&testing::module("FILMLOOK")).unwrap(),
        "FILMLOOK"
    );
    assert!(error(builder.add_module(&testing::module("FILMLOOK"))).contains("Module added twice"));
    assert!(builder.add_module(b"GRKM").is_err());
    assert!(error(builder.build()).contains("shows nothing"));
    builder.add_effect(&effect("test-a")).unwrap();
    let package = builder.build().unwrap();
    assert_eq!(
        packaging::verify_package(&package).unwrap().modules,
        ["FILMLOOK"]
    );
}

#[test]
fn the_camera_selects_the_modeset_file() {
    let mut builder = builder_with_runtime(&runtime_files());
    assert!(error(builder.set_camera("gr2")).contains("Unknown camera"));
    // GR IV needs its own runtimes.
    builder.set_camera("gr4").unwrap();
    assert!(error(builder.build()).contains("needs the runtime of each of its builds"));
    builder.set_camera("gr3x").unwrap();
    let package = builder.build().unwrap();
    let verified = packaging::verify_package(&package).unwrap();
    assert_eq!(verified.camera.id, "gr3x");
    builder.set_camera("gr3").unwrap();
    let package = builder.build().unwrap();
    assert_eq!(
        packaging::verify_package(&package).unwrap().camera.id,
        "gr3"
    );
}

#[test]
fn a_package_needs_the_guide_pdf() {
    let files = runtime_files();
    let mut builder = Builder::new(&manifest(&files)).unwrap();
    for path in builder.runtime_paths(0).unwrap().to_vec() {
        builder.add_runtime_file(0, &path, &files[&path]).unwrap();
    }
    assert!(error(builder.build()).contains("Set the installation guide first"));
    builder.set_guide(b"not a PDF");
    assert!(error(builder.build()).contains("not a PDF document"));
    builder.set_guide(PDF);
    builder.add_module(&testing::module("FRAMING")).unwrap();
    assert!(builder.build().is_ok());
}

#[test]
fn the_language_names_the_guide() {
    let mut builder = builder_with_runtime(&runtime_files());
    assert!(error(builder.set_language("fr")).contains("Unknown language"));
    builder.set_language("zh-Hant").unwrap();
    let package = builder.build().unwrap();
    assert_eq!(
        packaging::verify_package(&package).unwrap().language.code(),
        "zh-Hant"
    );
    builder.set_language("en").unwrap();
    let package = builder.build().unwrap();
    assert_eq!(
        packaging::verify_package(&package).unwrap().language.code(),
        "en"
    );
}

#[test]
fn missing_or_changed_runtime_files_are_rejected() {
    let files = runtime_files();
    let mut missing = Builder::new(&manifest(&files)).unwrap();
    missing
        .add_runtime_file(0, "SCRIPT/STARTUP.TTL", &files["SCRIPT/STARTUP.TTL"])
        .unwrap();
    assert!(error(missing.build()).contains("Runtime directory differs from its manifest"));

    let mut changed = Builder::new(&manifest(&files)).unwrap();
    for path in changed.runtime_paths(0).unwrap().to_vec() {
        let bytes = if path == "SCRIPT/STARTUP.TTL" {
            b"changed".to_vec()
        } else {
            files[&path].clone()
        };
        changed.add_runtime_file(0, &path, &bytes).unwrap();
    }
    assert!(error(changed.build()).contains("Runtime hash or size mismatch: SCRIPT/STARTUP.TTL"));
}

#[test]
fn unknown_and_duplicate_runtime_files_are_rejected() {
    let files = runtime_files();
    let mut builder = Builder::new(&manifest(&files)).unwrap();
    assert!(error(builder.add_runtime_file(0, "SCRIPT/EXTRA.SH", b"x"))
        .contains("Not a runtime file of this build"));
    let path = "SCRIPT/STARTUP.TTL";
    builder.add_runtime_file(0, path, &files[path]).unwrap();
    assert!(
        error(builder.add_runtime_file(0, path, &files[path])).contains("Duplicate runtime file")
    );
}

#[test]
fn non_production_manifests_are_rejected() {
    let files = runtime_files();
    let mut value: Value = serde_json::from_slice(&manifest(&files)).unwrap();
    value["profile"] = json!("diagnostic");
    let mut builder = Builder::new(&packaging::json_bytes(&value).unwrap()).unwrap();
    for path in builder.runtime_paths(0).unwrap().to_vec() {
        builder.add_runtime_file(0, &path, &files[&path]).unwrap();
    }
    assert!(error(builder.build()).contains("Expected a production build for a supported target"));
    assert!(error(Builder::new(b"{}")).contains("Expected a manifest file list"));
}

#[test]
fn duplicate_invalid_and_excess_effects_are_rejected() {
    let mut builder = builder_with_runtime(&runtime_files());
    builder.add_effect(&effect("test-a")).unwrap();
    assert!(error(builder.add_effect(&effect("test-a"))).contains("Duplicate effect ID"));
    assert!(error(builder.add_effect(b"not an effect")).contains("GRFX"));

    let mut full = builder_with_runtime(&runtime_files());
    for index in 0..runtime::MAX_COUNT {
        full.add_effect(&effect(&format!("test-{index}"))).unwrap();
    }
    assert!(error(full.add_effect(&effect("test-extra")))
        .contains(&format!("at most {} effects", runtime::MAX_COUNT)));
    assert!(full.build().is_ok());
}

/// A GR IV package takes the runtime of each GR IV build, in release order.
#[test]
fn a_package_adds_each_build_of_the_camera() {
    let camera = grkit_core::camera::find("gr4").unwrap();
    let builds: Vec<(Vec<u8>, Files)> = camera
        .targets()
        .into_iter()
        .map(|target| {
            let files = testing::runtime_files(&testing::marked_installer(target), target);
            (testing::manifest(target, &files), files)
        })
        .collect();
    let mut builder = Builder::new(&builds[0].0).unwrap();
    builder.set_camera("gr4").unwrap();
    builder.set_guide(PDF);
    builder.add_module(&testing::module("FRAMING")).unwrap();
    for (manifest, _) in &builds[1..] {
        builder.add_build(manifest).unwrap();
    }
    assert!(error(builder.runtime_paths(3)).contains("No runtime build 3"));
    for (number, (_, files)) in builds.iter().enumerate() {
        for path in builder.runtime_paths(number).unwrap().to_vec() {
            builder
                .add_runtime_file(number, &path, &files[&path])
                .unwrap();
        }
    }
    let package = builder.build().unwrap();
    assert_eq!(
        packaging::verify_package(&package).unwrap().targets,
        ["gr4-1.11", "gr4-1.20", "gr4-1.20-0528"]
    );
}

#[test]
fn the_card_files_are_the_sd_root_of_the_package() {
    let files = runtime_files();
    let mut builder = builder_with_runtime(&files);
    builder.add_effect(&effect("test-a")).unwrap();
    let card = builder.card_files().unwrap();
    // Each runtime file, the ModeSet files, the effect, each module, and the package list.
    for path in files.keys() {
        assert_eq!(card[path], files[path], "{path}");
    }
    assert!(card.contains_key("SCRIPT/GRKIT/PACKAGE.TXT"));
    assert!(card
        .keys()
        .any(|path| path.starts_with("SCRIPT/GRKIT/EFFECTS/")));
    assert!(card
        .keys()
        .any(|path| path.starts_with("SCRIPT/GRKIT/MODULES/")));
    // The guide, the licenses, and the checksums stay out of the card.
    assert!(card.keys().all(|path| !path.ends_with(".pdf")));
    assert!(!card.contains_key("LICENSES.md") && !card.contains_key("SHA256SUMS"));
    let package = builder.build().unwrap();
    assert_eq!(card, packaging::card_files(&package).unwrap());
}

#[test]
fn an_effect_label_gives_the_id_the_name_and_the_icon() {
    let label = grkit_wasm::effect_label(&effect("test-a")).unwrap();
    assert_eq!(label.id, "test-a");
    assert!(!label.name.is_empty());
    assert_eq!(label.icon.len(), 40 * 40 * 4);
    assert!(!label.monochrome);
    assert!(error(grkit_wasm::effect_label(b"not an effect")).contains("GRFX0001"));
}

#[test]
fn a_package_keeps_the_import_library() {
    use grkit_core::imports::{encode, Import};
    let mut builder = builder_with_runtime(&runtime_files());
    builder.add_effect(&effect("lut-warm")).unwrap();
    assert!(builder.set_imports(b"not a library").is_err());
    let library = encode(&[
        Import {
            details: "{}".into(),
            effect: effect("lut-warm"),
        },
        Import {
            details: "{}".into(),
            effect: effect("lut-cold"),
        },
    ])
    .unwrap();
    builder.set_imports(&library).unwrap();
    let verified = packaging::verify_package(&builder.build().unwrap()).unwrap();
    assert_eq!(verified.effect_ids, ["lut-warm"]);
    assert_eq!(verified.imports, 2);
    let card = builder.card_files().unwrap();
    assert_eq!(card[grkit_core::card::IMPORTS_FILE], library);
}
