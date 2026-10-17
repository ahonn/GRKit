//! Compare the portable decoder with the actual installer parser and C ABI.

use grkit_core::{effects, runtime};
use serde_json::{json, Value};
use std::{
    env,
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn checked(command: &mut Command) -> Output {
    let output = command
        .output()
        .unwrap_or_else(|error| panic!("Could not run {command:?}: {error}"));
    assert!(
        output.status.success(),
        "Command {command:?} failed: {}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn compiler() -> Option<OsString> {
    // Explicit configuration must never silently fall back to a different compiler.
    for variable in ["GRKIT_HOST_CC", "CC"] {
        if let Some(program) = env::var_os(variable) {
            assert!(
                !program.is_empty(),
                "{variable} must name a host C compiler executable"
            );
            checked(Command::new(&program).arg("--version"));
            return Some(program);
        }
    }
    for program in ["clang", "cc"] {
        match Command::new(program).arg("--version").output() {
            Ok(output) => {
                assert!(
                    output.status.success(),
                    "Host compiler {program} failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                return Some(program.into());
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => panic!("Could not run host compiler {program}: {error}"),
        }
    }
    assert!(
        env::var("GRKIT_REQUIRE_NATIVE_TESTS").as_deref() != Ok("1"),
        "GRKIT_REQUIRE_NATIVE_TESTS=1 requires a host C compiler; set GRKIT_HOST_CC or CC"
    );
    eprintln!("SKIPPED native C contract test: no clang or cc found; set GRKIT_HOST_CC to a host compiler or GRKIT_REQUIRE_NATIVE_TESTS=1 to require coverage");
    None
}

fn compile(compiler: &OsString, source: &Path, executable: &Path, extra_sources: &[PathBuf]) {
    checked(
        Command::new(compiler)
            .args([
                "-std=c11",
                "-O2",
                "-Wno-int-to-pointer-cast",
                "-Wno-pointer-to-int-cast",
            ])
            .arg("-I")
            .arg(root().join("runtime"))
            .arg(source)
            .args(extra_sources)
            .arg("-o")
            .arg(executable),
    );
}

fn sample(monochrome: bool) -> Value {
    let curve: Vec<u16> = (0..256).map(|i| i * 64).collect();
    let matrix = if monochrome {
        [2449, 4809, 934].repeat(3)
    } else {
        vec![8192, 0, 0, 0, 8192, 0, 0, 0, 8192]
    };
    json!({"effect_id": "custom-v1", "name": "Custom", "flags": u32::from(monochrome),
        "matrix": matrix, "curves": [&curve, &curve, &curve]})
}

fn repair_crc(bytes: &mut [u8]) {
    let checksum = effects::checksum(bytes);
    bytes[12..16].copy_from_slice(&checksum.to_le_bytes());
}

#[test]
fn rust_and_installer_c_accept_the_same_grf_records() {
    let Some(compiler) = compiler() else {
        return;
    };
    let temporary = tempfile::tempdir().unwrap();
    let harness = temporary.path().join("grf_contract.c");
    fs::write(
        &harness,
        r#"
#include <stdio.h>
#include "grf.h"
int main(int argc, char **argv) {
    if (grkit_grf_validate(NULL, GRKIT_GRF_SIZE) != 0) return 2;
    for (int i = 1; i < argc; ++i) {
        unsigned char bytes[GRKIT_GRF_SIZE + 1];
        FILE *file = fopen(argv[i], "rb");
        if (!file) return 3;
        size_t count = fread(bytes, 1, sizeof(bytes), file);
        if (ferror(file)) { fclose(file); return 4; }
        fclose(file);
        printf("%d\n", grkit_grf_validate(bytes, count));
    }
    return 0;
}
"#,
    )
    .unwrap();
    let executable = temporary.path().join("grf_contract");
    compile(
        &compiler,
        &harness,
        &executable,
        &[root().join("runtime/grf.c")],
    );

    let mut corpus: Vec<(String, Vec<u8>, Option<bool>)> = Vec::new();
    let color = effects::encode(&sample(false), &vec![0; effects::ICON_SIZE]).unwrap();
    let mono = effects::encode(&sample(true), &vec![0; effects::ICON_SIZE]).unwrap();
    for monochrome in [false, true] {
        let mut effect = sample(monochrome);
        corpus.push((
            format!("valid monochrome={monochrome}"),
            effects::encode(&effect, &vec![0; effects::ICON_SIZE]).unwrap(),
            Some(true),
        ));
        let curve: Vec<u16> = (0..256).map(|i| 16383 - i * 64).collect();
        effect["curves"] = json!([&curve, &curve, &curve]);
        corpus.push((
            format!("descending curves monochrome={monochrome}"),
            effects::encode(&effect, &[128, 64, 0, 255].repeat(1600)).unwrap(),
            Some(true),
        ));
    }
    for size in [0, 1, 8, 127, 1684, 8191] {
        corpus.push((
            format!("short buffer {size}"),
            color[..size].to_vec(),
            Some(false),
        ));
    }
    corpus.push((
        "extra byte".into(),
        [color.as_slice(), &[0]].concat(),
        Some(false),
    ));
    let malformed_fields: Vec<(usize, Vec<u8>)> = vec![
        (0, b"INVALID!".to_vec()),
        (8, 8191_u32.to_le_bytes().to_vec()),
        (16, vec![0]),
        (16, vec![b'-']),
        (16, vec![b'A']),
        (16, vec![b'x'; 32]),
        (26, vec![b'z']),
        (48, vec![0]),
        (48, vec![b' ']),
        (48, vec![128]),
        (53, vec![b' ']),
        (48, vec![b'x'; 17]),
        (55, vec![b'z']),
        (80, 2_u32.to_le_bytes().to_vec()),
        // Bit 2 is undefined.
        (84, vec![2]),
        (127, vec![1]),
        (146, vec![1]),
        (147, vec![1]),
        (148, 16384_u16.to_le_bytes().to_vec()),
        (1687, vec![128]),
        (8084, vec![1]),
        (8191, vec![1]),
    ];
    for (offset, bytes) in malformed_fields {
        let mut changed = color.clone();
        changed[offset..offset + bytes.len()].copy_from_slice(&bytes);
        repair_crc(&mut changed);
        corpus.push((
            format!("malformed field at {offset}: {bytes:?}"),
            changed,
            Some(false),
        ));
    }
    let mut bad_crc = color.clone();
    bad_crc[200] ^= 1;
    corpus.push(("unrepaired CRC".into(), bad_crc, Some(false)));
    for offset in [134, 140, 660, 1172] {
        let mut changed = mono.clone();
        changed[offset] ^= 1;
        repair_crc(&mut changed);
        corpus.push((
            format!("unequal monochrome channel at {offset}"),
            changed,
            Some(false),
        ));
    }
    let mut boundary_effect = sample(false);
    boundary_effect["effect_id"] = json!("a".repeat(31));
    boundary_effect["name"] = json!("x".repeat(16));
    boundary_effect["matrix"] = json!([-32768, 32767, 0].repeat(3));
    boundary_effect["curves"] = json!(vec![[0, 16383].repeat(128); 3]);
    let boundary = effects::encode(&boundary_effect, &vec![0; effects::ICON_SIZE]).unwrap();
    corpus.push((
        "boundary fields and nonmonotone curves".into(),
        boundary.clone(),
        Some(true),
    ));
    // Extension: each feature bit owns its field.
    let mut extended_effect = sample(false);
    extended_effect["post_matrix"] = json!([9000, -500, -308, -200, 8600, -208, 100, -300, 8392]);
    extended_effect["glow"] = json!(3);
    let extended = effects::encode(&extended_effect, &vec![0; effects::ICON_SIZE]).unwrap();
    assert_eq!(&extended[..8], effects::MAGIC);
    corpus.push((
        "extended post and glow".into(),
        extended.clone(),
        Some(true),
    ));
    let mut glow_effect = sample(true);
    glow_effect["glow"] = json!(1);
    corpus.push((
        "extended monochrome glow".into(),
        effects::encode(&glow_effect, &vec![0; effects::ICON_SIZE]).unwrap(),
        Some(true),
    ));
    let mut mono_post = mono.clone();
    mono_post[84] = 1;
    mono_post[88] = 1;
    repair_crc(&mut mono_post);
    corpus.push(("monochrome post matrix".into(), mono_post, Some(false)));
    let extended_fields: Vec<(usize, Vec<u8>)> = vec![
        (84, 0_u32.to_le_bytes().to_vec()),
        (84, 8_u32.to_le_bytes().to_vec()),
        // Bit 2 is undefined.
        (84, 2_u32.to_le_bytes().to_vec()),
        (84, 7_u32.to_le_bytes().to_vec()),
        (84, 1_u32.to_le_bytes().to_vec()),
        (84, 4_u32.to_le_bytes().to_vec()),
        (106, vec![1]),
        (107, vec![0]),
        (107, vec![4]),
        (108, vec![1]),
        (127, vec![1]),
    ];
    for (offset, bytes) in extended_fields {
        let mut changed = extended.clone();
        changed[offset..offset + bytes.len()].copy_from_slice(&bytes);
        repair_crc(&mut changed);
        corpus.push((
            format!("malformed extension at {offset}: {bytes:?}"),
            changed,
            Some(false),
        ));
    }
    // A fixed xorshift stream keeps this corpus identical on every host without a RNG dependency.
    let mut state = 83_u64;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for index in 0..512 {
        let mut changed = match index % 3 {
            0 => boundary.clone(),
            1 => mono.clone(),
            _ => extended.clone(),
        };
        let offset = (next() % changed.len() as u64) as usize;
        changed[offset] ^= (next() % 255 + 1) as u8;
        repair_crc(&mut changed);
        corpus.push((format!("mutation {index} at {offset}"), changed, None));
    }

    let mut command = Command::new(&executable);
    for (index, (_, bytes, _)) in corpus.iter().enumerate() {
        let path = temporary.path().join(format!("record-{index}.grf"));
        fs::write(&path, bytes).unwrap();
        command.arg(path);
    }
    let output = checked(&mut command);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let results: Vec<_> = stdout.lines().collect();
    assert_eq!(
        results.len(),
        corpus.len(),
        "C harness must return one result per record"
    );
    for ((description, bytes, expected), native) in corpus.iter().zip(results) {
        assert!(matches!(native, "0" | "1"), "Unexpected C result: {native}");
        let native = native == "1";
        let rust = effects::decode(bytes).is_ok();
        assert_eq!(rust, native, "Rust/C disagreement: {description}");
        if let Some(expected) = expected {
            assert_eq!(native, *expected, "{description}");
        }
    }
}

#[test]
fn native_c_abi_matches_rust_allocation_constants() {
    let Some(compiler) = compiler() else {
        return;
    };
    let temporary = tempfile::tempdir().unwrap();
    let source = temporary.path().join("abi_contract.c");
    fs::write(
        &source,
        r#"
#include <stdio.h>
#include "abi.h"
_Static_assert(GRKIT_RECORD_ID + 32 <= GRKIT_RECORD_SUMMARY, "Summary overlap");
_Static_assert(GRKIT_RECORD_SUMMARY + GRKIT_SUMMARY_UNITS * 2 <= GRKIT_RECORD_CODE, "Code overlap");
_Static_assert(GRKIT_RECORD_CODE + 2 <= GRKIT_RECORD_SIZE, "Record overflow");
_Static_assert(GRKIT_HOST_RETURN + 4 <= GRKIT_HOST_HEADER_SIZE, "Header word overflow");
_Static_assert(GRKIT_HOST_ICON_CHUNKS + 4 * 4 <= GRKIT_HOST_HEADER_SIZE, "Header overflow");
_Static_assert(GRKIT_RECORD_COEFFICIENTS + 1556 == GRKIT_RECORD_NAME, "Coefficient overlap");
_Static_assert(GRKIT_RECORD_NAME + 34 <= GRKIT_RECORD_MONOCHROME, "Name overlap");
_Static_assert(GRKIT_RECORD_CODE + 2 <= GRKIT_RECORD_FEATURES, "Extension overlap");
_Static_assert(GRKIT_RECORD_GLOW < GRKIT_RECORD_SIZE, "Extension overflow");
int main(void) {
    printf("%u %u %u %u\n", GRKIT_HOST_HEADER_SIZE, GRKIT_RECORD_SIZE, GRKIT_ICON_STRIDE,
           GRKIT_GLOW_SCRATCH);
    return 0;
}
"#,
    )
    .unwrap();
    let executable = temporary.path().join("abi_contract");
    compile(&compiler, &source, &executable, &[]);
    let output = checked(&mut Command::new(&executable));
    let actual: Vec<usize> = String::from_utf8(output.stdout)
        .unwrap()
        .split_whitespace()
        .map(|v| v.parse().unwrap())
        .collect();
    assert_eq!(
        actual,
        [
            runtime::HEADER_SIZE,
            runtime::RECORD_SIZE,
            runtime::ICON_STRIDE,
            runtime::GLOW_SCRATCH
        ]
    );
}

#[test]
fn native_menu_contract_preserves_events_icons_and_empty_catalog() {
    let Some(compiler) = compiler() else {
        return;
    };
    let temporary = tempfile::tempdir().unwrap();
    let executable = temporary.path().join("menu_contract");
    compile(
        &compiler,
        &root().join("tests/runtime/menu_contract.c"),
        &executable,
        &[],
    );
    checked(&mut Command::new(&executable));
}

#[test]
fn native_catalog_contract_skips_unusable_files_and_ignores_directory_order() {
    let Some(compiler) = compiler() else {
        return;
    };
    let temporary = tempfile::tempdir().unwrap();
    let executable = temporary.path().join("catalog_contract");
    compile(
        &compiler,
        &root().join("tests/runtime/catalog_contract.c"),
        &executable,
        &[],
    );
    checked(&mut Command::new(&executable));
}

#[test]
fn native_framing_contract_preserves_canvas_bounds_and_settings() {
    let Some(compiler) = compiler() else {
        return;
    };
    let temporary = tempfile::tempdir().unwrap();
    let executable = temporary.path().join("framing_contract");
    compile(
        &compiler,
        &root().join("tests/runtime/framing_contract.c"),
        &executable,
        &[],
    );
    checked(&mut Command::new(&executable));
}

#[test]
fn native_picture_contract_applies_post_matrix_and_glow() {
    let Some(compiler) = compiler() else {
        return;
    };
    let temporary = tempfile::tempdir().unwrap();
    let executable = temporary.path().join("picture_contract");
    compile(
        &compiler,
        &root().join("tests/runtime/picture_contract.c"),
        &executable,
        &[],
    );
    checked(&mut Command::new(&executable));
}
