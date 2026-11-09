//! Fixtures for the tests of this crate and of the crates that use it. Not a stable API.

use serde_json::{json, Value};

use crate::{effects, packaging, runtime, Files};

/// A stand-in for `CORE.BIN`: an ARM ELF32 header with nothing after it, which
/// `packaging::validate_runtime` accepts.
pub fn installer() -> Vec<u8> {
    let mut binary = vec![0; 52];
    binary[..6].copy_from_slice(b"\x7fELF\x01\x01");
    binary[16..20].copy_from_slice(b"\x02\x00\x28\x00");
    binary
}

/// The production runtime files of `target` around `installer`.
pub fn runtime_files(installer: &[u8], target: &str) -> Files {
    runtime::runtime_files_for(installer, runtime::icon_stride(target).unwrap())
}

/// A stand-in fast-boot file of `release`, which `runtime::check_fast_file`
/// accepts: a head that names the file's path, has the status word, and
/// calls the scan vector, then the marker of its runtime build.
pub fn fast_file(release: &runtime::FastRelease) -> Vec<u8> {
    let mut bytes = vec![0; runtime::FAST_HEAD_SIZE];
    let path = release.rtos_path();
    bytes[..path.len()].copy_from_slice(path.as_bytes());
    bytes[runtime::FAST_STATUS_OFFSET..runtime::FAST_ENTRY_OFFSET]
        .copy_from_slice(&runtime::FAST_STATUS_NOT_RUN.to_le_bytes());
    bytes[runtime::FAST_ENTRY_OFFSET..].copy_from_slice(&runtime::FAST_SCAN_VECTOR.to_le_bytes());
    bytes.extend_from_slice(
        format!("{}{}\0", packaging::TARGET_MARKER, release.target()).as_bytes(),
    );
    bytes.resize(bytes.len().next_multiple_of(4), 0);
    // The code ends with the file, as `_code_end` of `fast.ld` gives it.
    let end = runtime::FAST_BASE + bytes.len() as u32;
    bytes[runtime::FAST_CODE_END_OFFSET..runtime::FAST_CODE_END_OFFSET + 4]
        .copy_from_slice(&end.to_le_bytes());
    bytes
}

/// The stand-in fast-boot file of each release of `target`, by its path.
pub fn fast_files(target: &str) -> Files {
    crate::camera::CAMERAS
        .into_iter()
        .flat_map(|camera| {
            camera
                .versions()
                .into_iter()
                .filter_map(move |firmware| runtime::FastRelease::new(camera, firmware).ok())
        })
        .filter(|release| release.target() == target)
        .map(|release| (release.path(), fast_file(&release)))
        .collect()
}

/// The verified runtime of `target` with `marked_installer(target)` and
/// `fast_files(target)`, as a build with fast-boot files has them.
pub fn fast_runtime(target: &str) -> packaging::VerifiedRuntime {
    let files = runtime::runtime_files_with(
        &marked_installer(target),
        &fast_files(target),
        runtime::icon_stride(target).unwrap(),
    );
    packaging::load_runtime(&manifest(target, &files), &files).unwrap()
}

/// The `build.json` bytes of a runtime build of `target` with the `sd/` files `files`.
pub fn manifest(target: &str, files: &Files) -> Vec<u8> {
    packaging::json_bytes(&Value::Object(packaging::manifest(target, files))).unwrap()
}

/// A runtime build of `target` with the stand-in installer: its `build.json` bytes and
/// its `sd/` files.
pub fn build(target: &str) -> (Vec<u8>, Files) {
    let files = runtime_files(&installer(), target);
    (manifest(target, &files), files)
}

/// The verified runtime of `build(target)`.
pub fn runtime(target: &str) -> packaging::VerifiedRuntime {
    let (manifest, files) = build(target);
    packaging::load_runtime(&manifest, &files).unwrap()
}

/// The stand-in installer of `target`, which names its build like a built one.
pub fn marked_installer(target: &str) -> Vec<u8> {
    let mut binary = installer();
    binary.extend_from_slice(format!("{}{target}\0", packaging::TARGET_MARKER).as_bytes());
    binary
}

/// The verified runtime of `target` with `marked_installer(target)`.
pub fn marked_runtime(target: &str) -> packaging::VerifiedRuntime {
    let files = runtime_files(&marked_installer(target), target);
    packaging::load_runtime(&manifest(target, &files), &files).unwrap()
}

/// The verified runtime of each build of `camera`, in release order, as its
/// package needs them.
pub fn camera_runtimes(camera: &crate::camera::Camera) -> Vec<packaging::VerifiedRuntime> {
    camera.targets().into_iter().map(marked_runtime).collect()
}

/// The effect set of `effect(id)` for each of `ids`.
pub fn effect_set(ids: &[&str]) -> packaging::EffectSet {
    let mut set = packaging::EffectSet::default();
    for id in ids {
        set.insert(effect(id)).unwrap();
    }
    set
}

/// A compiled effect with the identity matrix, linear curves, and a transparent icon.
pub fn effect(id: &str) -> Vec<u8> {
    let curve: Vec<_> = (0..256).map(|i| i * 64).collect();
    let value = json!({"effect_id": id, "name": "Test", "flags": 0,
        "matrix": [8192, 0, 0, 0, 8192, 0, 0, 0, 8192],
        "curves": [&curve, &curve, &curve]});
    effects::encode(&value, &vec![0; effects::ICON_SIZE]).unwrap()
}

/// A module file (`crate::grm`) with the ID `id` and no registrations.
pub fn module(id: &str) -> Vec<u8> {
    crate::grm::Module {
        id: id.into(),
        version: 1,
        code: vec![0; 40],
        descriptor: 0,
        ..Default::default()
    }
    .encode()
    .expect("a valid module")
}

/// A module file for each official module (`crate::modules::OFFICIAL`).
pub fn official_modules() -> Vec<Vec<u8>> {
    crate::modules::OFFICIAL
        .iter()
        .map(|official| module(official.id))
        .collect()
}
