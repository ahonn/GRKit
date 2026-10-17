use grkit_core::{effects, packaging, runtime};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_grkit"))
        .args(args)
        .output()
        .unwrap()
}

fn success(args: &[&str]) -> Value {
    let output = run(args);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn text(path: &Path) -> &str {
    path.to_str().unwrap()
}

fn source() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../effects")
}

fn write_definition(root: &Path, id: &str) {
    let mut definition: Value =
        serde_json::from_slice(&fs::read(source().join("mono-orange-v1/effect.json")).unwrap())
            .unwrap();
    definition["effect_id"] = json!(id);
    let directory = root.join(id);
    fs::create_dir_all(&directory).unwrap();
    fs::write(
        directory.join("effect.json"),
        serde_json::to_vec(&definition).unwrap(),
    )
    .unwrap();
}

fn compile(input: &Path, output: &Path) -> Output {
    run(&[
        "effects",
        "build",
        "--source",
        text(input),
        "--out",
        text(output),
    ])
}

fn fixture(root: &Path) -> (PathBuf, PathBuf) {
    let build = root.join("runtime");
    let effects = root.join("effects");
    success(&[
        "effects",
        "build",
        "--source",
        text(&source()),
        "--out",
        text(&effects),
    ]);
    let mut binary = vec![0; 52];
    binary[..6].copy_from_slice(b"\x7fELF\x01\x01");
    binary[16..20].copy_from_slice(b"\x02\x00\x28\x00");
    let files = runtime::runtime_files(&binary);
    for (name, bytes) in &files {
        let path = build.join("sd").join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
    let manifest = json!({"profile":"production", "target":"gr3-2.10", "startup_activation":"Off",
        "files": files.iter().map(|(name,bytes)|json!({"path":name,"size":bytes.len(),"sha256":packaging::sha256(bytes)})).collect::<Vec<_>>()});
    fs::write(
        build.join("build.json"),
        packaging::json_bytes(&manifest).unwrap(),
    )
    .unwrap();
    (build, effects)
}

#[test]
fn exports_effects_without_overwriting() {
    let tmp = tempfile::tempdir().unwrap();
    let output = tmp.path().join("effects");
    let source = source();
    let args = [
        "effects",
        "build",
        "--source",
        text(&source),
        "--out",
        text(&output),
    ];
    let result = success(&args);
    assert_eq!(result["effects"], 5);
    assert_eq!(fs::read_dir(&output).unwrap().count(), 5);
    assert!(!run(&args).status.success());
}

#[test]
fn effect_compilation_is_independent_of_target_capacity() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input");
    fs::create_dir(&input).unwrap();
    let empty = tmp.path().join("empty");
    assert!(compile(&input, &empty).status.success());
    assert_eq!(fs::read_dir(empty).unwrap().count(), 0);
    for index in 0..21 {
        write_definition(&input, &format!("custom-{index:04}"));
    }
    let output = tmp.path().join("compiled");
    let result = compile(&input, &output);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&result.stdout).unwrap()["effects"],
        21
    );
    assert_eq!(fs::read_dir(&output).unwrap().count(), 21);

    let (build, _) = fixture(&tmp.path().join("package-input"));
    let archive = tmp.path().join("overflow.zip");
    let result = run(&[
        "package",
        "--build",
        text(&build),
        "--effects",
        text(&output),
        "--out",
        text(&archive),
    ]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("at most 20"));
    assert!(!archive.exists());
}

#[test]
fn actual_filename_collision_and_directory_identity_fail_before_output() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input");
    let output = tmp.path().join("compiled");
    // These distinct IDs share the first 32 SHA-256 bits used by FAT filenames.
    for id in ["collision-55045", "collision-70885"] {
        assert_eq!(effects::filename(id).unwrap(), "45560C60.GRF");
        write_definition(&input, id);
    }
    let result = compile(&input, &output);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("filename hash collision"));
    assert!(!output.exists());

    let input = tmp.path().join("mismatched");
    write_definition(&input, "custom-v1");
    fs::rename(input.join("custom-v1"), input.join("wrong-name")).unwrap();
    let result = compile(&input, &output);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("directory must match"));
    assert!(!output.exists());
}

#[test]
fn compiled_directory_extras_and_changed_runtime_bytes_are_rejected() {
    for mutation in ["extra", "runtime"] {
        let tmp = tempfile::tempdir().unwrap();
        let (build, effects) = fixture(tmp.path());
        if mutation == "extra" {
            fs::write(effects.join("extra.txt"), b"unexpected").unwrap();
        } else {
            fs::write(build.join("sd/SCRIPT/STARTUP.TTL"), b"changed").unwrap();
        }
        let output = tmp.path().join("package.zip");
        let result = run(&[
            "package",
            "--build",
            text(&build),
            "--effects",
            text(&effects),
            "--out",
            text(&output),
        ]);
        assert!(!result.status.success());
        let message = String::from_utf8_lossy(&result.stderr);
        assert!(
            message.contains(if mutation == "extra" {
                "Unexpected compiled effect entry"
            } else {
                "Runtime hash or size mismatch"
            }),
            "{message}"
        );
        assert!(!output.exists());
    }
}

#[cfg(unix)]
#[test]
fn symlink_definitions_and_compiled_effects_are_rejected() {
    use std::os::unix::fs::symlink;
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input/mono-orange-v1");
    fs::create_dir_all(&input).unwrap();
    symlink(
        source().join("mono-orange-v1/effect.json"),
        input.join("effect.json"),
    )
    .unwrap();
    let output = tmp.path().join("compiled");
    assert!(!compile(input.parent().unwrap(), &output).status.success());
    assert!(!output.exists());

    let (build, effects) = fixture(&tmp.path().join("package-input"));
    let path = fs::read_dir(&effects)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let target = tmp.path().join("external.grf");
    fs::rename(&path, &target).unwrap();
    symlink(&target, &path).unwrap();
    let result = run(&[
        "package",
        "--build",
        text(&build),
        "--effects",
        text(&effects),
        "--out",
        text(&output),
    ]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("Expected a regular file"));
    assert!(!output.exists());
}

#[test]
fn package_roundtrip_and_output_ownership() {
    let tmp = tempfile::tempdir().unwrap();
    let (build, effects) = fixture(tmp.path());
    let zip = tmp.path().join("package.zip");
    let args = [
        "package",
        "--build",
        text(&build),
        "--effects",
        text(&effects),
        "--out",
        text(&zip),
    ];
    let packaged = success(&args);
    assert_eq!(packaged["entries"], 14);
    assert_eq!(success(&["verify", text(&zip)])["effects"], 5);
    let before = fs::read(&zip).unwrap();
    assert!(!run(&args).status.success());
    assert_eq!(fs::read(&zip).unwrap(), before);
    fs::write(build.join("sd/unlisted"), b"unexpected").unwrap();
    let rejected = tmp.path().join("rejected.zip");
    assert!(!run(&[
        "package",
        "--build",
        text(&build),
        "--effects",
        text(&effects),
        "--out",
        text(&rejected)
    ])
    .status
    .success());
    assert!(!rejected.exists());
}

#[test]
fn invalid_archive_and_firmware_fail_without_panicking() {
    let tmp = tempfile::tempdir().unwrap();
    let bad = tmp.path().join("invalid");
    fs::write(&bad, b"invalid input").unwrap();
    let verify = run(&["verify", text(&bad)]);
    assert_eq!(verify.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&verify.stderr).starts_with("grkit:"));
    let output = tmp.path().join("output");
    let build = run(&[
        "build",
        "--target",
        "gr3-2.10",
        "--firmware",
        text(&bad),
        "--out",
        text(&output),
        "--clang",
        "absent-compiler",
    ]);
    assert_eq!(build.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&build.stderr).contains("SHA-256"));
    assert!(!output.exists());
}

#[cfg(unix)]
#[test]
fn symlink_inputs_are_rejected() {
    use std::os::unix::fs::symlink;
    let tmp = tempfile::tempdir().unwrap();
    let linked_source = tmp.path().join("source");
    symlink(source(), &linked_source).unwrap();
    let output = tmp.path().join("output");
    assert!(!run(&[
        "effects",
        "build",
        "--source",
        text(&linked_source),
        "--out",
        text(&output)
    ])
    .status
    .success());
    assert!(!output.exists());
}

#[test]
fn malformed_definition_is_rejected_before_output() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input/test");
    fs::create_dir_all(&input).unwrap();
    fs::write(input.join("effect.json"), b"{\"schema\":1,\"schema\":2}").unwrap();
    let output = tmp.path().join("output");
    let result = run(&[
        "effects",
        "build",
        "--source",
        text(&tmp.path().join("input")),
        "--out",
        text(&output),
    ]);
    assert_eq!(result.status.code(), Some(1));
    assert!(!output.exists());
}

#[cfg(unix)]
#[test]
fn literal_backslash_cannot_alias_manifest_path() {
    let tmp = tempfile::tempdir().unwrap();
    let (build, effects) = fixture(tmp.path());
    fs::write(build.join("sd/SCRIPT\\STARTUP.TTL"), b"unlisted").unwrap();
    let output = tmp.path().join("package.zip");
    assert!(!run(&[
        "package",
        "--build",
        text(&build),
        "--effects",
        text(&effects),
        "--out",
        text(&output)
    ])
    .status
    .success());
    assert!(!output.exists());
}

fn warm_cube(path: &Path) {
    let mut text = String::from("TITLE \"Warm\"\nLUT_3D_SIZE 5\n");
    for b in 0..5 {
        for g in 0..5 {
            for r in 0..5 {
                let [r, g, b] = [r, g, b].map(|v| v as f64 / 4.0);
                let row = [r * 0.94 + 0.05, g * 0.92 + 0.04, b * 0.86 + 0.03];
                text.push_str(&format!("{:.6} {:.6} {:.6}\n", row[0], row[1], row[2]));
            }
        }
    }
    fs::write(path, text).unwrap();
}

fn import(cube: &Path, firmware: &Path, id: &str, label: &str, effects: &Path) -> Output {
    run(&[
        "effects",
        "import",
        "--cube",
        text(cube),
        "--firmware",
        text(firmware),
        "--input",
        "srgb",
        "--exposure",
        "-0.5",
        "--id",
        id,
        "--name",
        "Warm",
        "--label",
        label,
        "--shape",
        "film",
        "--out",
        text(effects),
    ])
}

#[test]
fn cube_import_rejects_invalid_firmware_and_ids_before_output() {
    let tmp = tempfile::tempdir().unwrap();
    let cube = tmp.path().join("warm.cube");
    warm_cube(&cube);
    let firmware = tmp.path().join("firmware.bin");
    fs::write(&firmware, b"not firmware").unwrap();
    let effects = tmp.path().join("effects");
    let failed = import(&cube, &firmware, "warm-v1", "FV", &effects);
    assert_eq!(failed.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&failed.stderr).contains("SHA-256"));
    let failed = import(&cube, &firmware, "../warm", "FV", &effects);
    assert!(String::from_utf8_lossy(&failed.stderr).contains("Invalid effect ID"));
    assert!(!effects.exists());
}

#[test]
fn standard_base_gamma_is_read_and_checked() {
    use grkit_build::{firmware, target};
    let offset = (firmware::STANDARD_BASE_GAMMA - target::RTOS_BASE) as usize;
    let image = |table: &[u16; 256]| firmware::FirmwareImage {
        body: [
            vec![0; offset],
            table.iter().flat_map(|v| v.to_le_bytes()).collect(),
        ]
        .concat(),
        sha256: "synthetic".into(),
        decoded_sha256: "synthetic".into(),
    };
    let mut table: [u16; 256] = std::array::from_fn(|i| match i {
        0 => 0,
        1..=8 => 944 * i as u16,
        _ => 7552 + ((i - 8) * 8832 / 247) as u16,
    });
    assert_eq!(
        firmware::standard_base_gamma(&image(&table)).unwrap(),
        table
    );
    for (index, value) in [(8, 7551), (0, 1), (255, 15999), (100, 0)] {
        let saved = table[index];
        table[index] = value;
        assert!(
            firmware::standard_base_gamma(&image(&table)).is_err(),
            "{index}"
        );
        table[index] = saved;
    }
}

/// End to end with the official firmware, which the repository cannot contain.
#[test]
fn imports_a_cube_into_a_buildable_effect_without_overwriting() {
    let Ok(firmware) = std::env::var("GRKIT_TEST_FIRMWARE") else {
        eprintln!(
            "SKIPPED cube import end to end: set GRKIT_TEST_FIRMWARE to GR III 2.10 firmware"
        );
        return;
    };
    let firmware = PathBuf::from(firmware);
    let tmp = tempfile::tempdir().unwrap();
    let cube = tmp.path().join("warm.cube");
    warm_cube(&cube);
    let effects = tmp.path().join("effects");
    let output = import(&cube, &firmware, "warm-v1", "FV", &effects);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let summary: Value = serde_json::from_slice(&output.stdout).unwrap();
    eprintln!("{summary}");
    assert_eq!(summary["effect_id"], "warm-v1");
    assert!(summary["fit"]["mean_de"].as_f64().unwrap() < 2.0);
    assert!(summary["fit"]["csp_max"].as_i64().unwrap() <= 240);
    let path = effects.join("warm-v1/effect.json");
    let written = fs::read(&path).unwrap();
    let definition = effects::parse_json(&written).unwrap();
    assert_eq!(definition["attribution"]["source"], "warm.cube");
    assert_eq!(definition["attribution"]["exposure"], -0.5);
    assert_eq!(
        definition.get("post_matrix").is_some(),
        summary["post_matrix"] == true
    );

    let failed = import(&cube, &firmware, "warm-v1", "FV", &effects);
    assert_eq!(failed.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&failed.stderr).contains("already exists"));
    assert_eq!(fs::read(&path).unwrap(), written);
    assert!(!import(&cube, &firmware, "bad-v1", "AB", &effects)
        .status
        .success());
    assert!(!effects.join("bad-v1").exists());

    let compiled = tmp.path().join("compiled");
    let built = success(&[
        "effects",
        "build",
        "--source",
        text(&effects),
        "--out",
        text(&compiled),
    ]);
    assert_eq!(built["effects"], 1);
    let name = effects::filename("warm-v1").unwrap();
    let (effect, _) = effects::decode(&fs::read(compiled.join(name)).unwrap()).unwrap();
    assert_eq!(effect["matrix"], definition["matrix"]);
}

#[test]
fn site_export_publishes_only_the_verified_runtime() {
    let tmp = tempfile::tempdir().unwrap();
    let (build, effects) = fixture(tmp.path());
    fs::create_dir(build.join("objects")).unwrap();
    fs::write(build.join("objects/module_payload.h"), b"firmware bytes").unwrap();
    let site = tmp.path().join("site");
    let result = success(&["site", "--build", text(&build), "--out", text(&site)]);
    assert_eq!(result["target"], "gr3-2.10");
    assert_eq!(result["files"], 6);

    let exported = site.join("runtime/gr3-2.10");
    assert_eq!(
        fs::read(exported.join("build.json")).unwrap(),
        fs::read(build.join("build.json")).unwrap()
    );
    assert!(!exported.join("objects").exists());
    let mut entries: Vec<_> = fs::read_dir(&exported)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    entries.sort();
    assert_eq!(entries, ["build.json", "sd"]);

    // The exported tree is itself a valid build input.
    let package = tmp.path().join("package.zip");
    success(&[
        "package",
        "--build",
        text(&exported),
        "--effects",
        text(&effects),
        "--out",
        text(&package),
    ]);
    assert!(
        !run(&["site", "--build", text(&build), "--out", text(&site)])
            .status
            .success()
    );
}

#[test]
fn site_export_rejects_changed_runtime_files() {
    let tmp = tempfile::tempdir().unwrap();
    let (build, _) = fixture(tmp.path());
    fs::write(build.join("sd/SCRIPT/STARTUP.TTL"), b"changed").unwrap();
    let site = tmp.path().join("site");
    let result = run(&["site", "--build", text(&build), "--out", text(&site)]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("Runtime hash or size mismatch"));
    assert!(!site.exists());
}
