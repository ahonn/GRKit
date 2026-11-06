//! Checks of the release tables that need tools or files outside the
//! repository.

use grkit_firmware::{patch, releases, sha256, INTERFACE, MAGIC};
use std::path::{Path, PathBuf};
use std::process::Command;

fn run(program: &str, args: &[&str]) {
    let status = Command::new(program)
        .args(args)
        .status()
        .unwrap_or_else(|error| panic!("{program}: {error}"));
    assert!(status.success(), "{program} {args:?}");
}

/// The handler code of each table is the source of `fastboot/patch/`,
/// assembled at address 0 with the calls of the release.
#[test]
#[ignore = "Requires ARM-capable Clang and LLD; run with --ignored"]
fn fastboot_code_matches_its_source() {
    let clang = std::env::var("GRKIT_CLANG").unwrap_or_else(|_| "clang".into());
    let lld = std::env::var("GRKIT_LLD").unwrap_or_else(|_| "ld.lld".into());
    let sources = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fastboot/patch");
    let directory = tempfile::tempdir().unwrap();
    for release in releases() {
        for (handler, name) in release.handlers.iter().zip(["stub", "probe"]) {
            let mut defines: Vec<String> = release
                .calls
                .iter()
                .map(|(call, address)| format!("-D{}={address}", call.to_uppercase()))
                .collect();
            defines.extend([
                format!("-DMAGIC={MAGIC:#x}"),
                format!("-DINTERFACE={INTERFACE}"),
                format!("-DPROJECT={}", release.project),
                format!("-DFIRMWARE={}", release.firmware.replace('.', "")),
            ]);
            let object = directory.path().join(format!("{name}.o"));
            let binary = directory.path().join(format!("{name}.bin"));
            let source = sources.join(format!("{name}.S"));
            let mut args = vec![
                "--target=armv7-linux-gnueabi",
                "-march=armv7-a",
                "-marm",
                "-c",
            ];
            args.extend(defines.iter().map(String::as_str));
            args.extend([source.to_str().unwrap(), "-o", object.to_str().unwrap()]);
            run(&clang, &args);
            run(
                &lld,
                &[
                    "-m",
                    "armelf_linux_eabi",
                    "-static",
                    "--build-id=none",
                    "-e",
                    name,
                    "-Ttext=0",
                    "--image-base=0",
                    "--oformat=binary",
                    object.to_str().unwrap(),
                    "-o",
                    binary.to_str().unwrap(),
                ],
            );
            assert_eq!(
                hex::encode(std::fs::read(&binary).unwrap()),
                handler.code,
                "{} {}",
                release.name(),
                handler.command
            );
        }
    }
}

fn files(directory: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files(&path, out);
        } else {
            out.push(path);
        }
    }
}

/// Each official update file in GRKIT_TEST_FASTBOOT_FIRMWARE patches to the
/// result of its table.
#[test]
#[ignore = "Requires the official update files in GRKIT_TEST_FASTBOOT_FIRMWARE"]
fn fastboot_official_files_patch_to_their_result() {
    let directory = PathBuf::from(std::env::var("GRKIT_TEST_FASTBOOT_FIRMWARE").unwrap());
    let mut paths = Vec::new();
    files(&directory, &mut paths);
    let mut found = Vec::new();
    for path in paths {
        let bytes = std::fs::read(&path).unwrap();
        let Ok(patched) = patch(&bytes) else { continue };
        assert_eq!(sha256(&patched.bytes), patched.release.result.sha256);
        // The patched file is not an official file of any release.
        assert!(grkit_firmware::release_of(&patched.bytes).is_err());
        found.push(patched.release.name());
    }
    found.sort();
    found.dedup();
    assert_eq!(found.len(), releases().len(), "{found:?}");
}
