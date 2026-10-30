//! Host contracts of the native C code: the installer parser and the C ABI
//! against the Rust tables, and the behavior of the menu, Framing, picture,
//! catalog, directory, module plan, relocation, and selection code.

use grkit_core::{
    effects,
    grm::{self, Import, Module, Segment},
    language, menu_text, runtime,
    selective::Selective,
};
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

/// The definition that selects the GR III target in `runtime/firmware.h`, as
/// `components::Layout::define` passes it to the ARM build.
const GR3_TARGET: &str = "-DGRKIT_TARGET_GR3_210";

/// The definition of each runtime target that `runtime/firmware.h` selects, so
/// a new target is compiled without a change here.
fn targets() -> Vec<String> {
    let selector = fs::read_to_string(root().join("runtime/firmware.h")).unwrap();
    let targets: Vec<String> = selector
        .lines()
        .filter_map(|line| {
            let name = line.split("defined(").nth(1)?.split(')').next()?;
            name.starts_with("GRKIT_TARGET_")
                .then(|| format!("-D{name}"))
        })
        .collect();
    assert!(targets.contains(&GR3_TARGET.to_string()));
    targets
}

/// Builds a host C harness for the GR III target.
fn compile(compiler: &OsString, source: &Path, executable: &Path, extra_sources: &[PathBuf]) {
    compile_for(GR3_TARGET, compiler, source, executable, extra_sources);
}

fn compile_for(
    target: &str,
    compiler: &OsString,
    source: &Path,
    executable: &Path,
    extra_sources: &[PathBuf],
) {
    compile_with(target, compiler, source, executable, extra_sources, None);
}

/// Builds a host C harness, optionally with one more include directory.
fn compile_with(
    target: &str,
    compiler: &OsString,
    source: &Path,
    executable: &Path,
    extra_sources: &[PathBuf],
    include: Option<&Path>,
) {
    let mut command = Command::new(compiler);
    command.args([
        "-std=c11",
        "-O2",
        "-Wno-int-to-pointer-cast",
        "-Wno-pointer-to-int-cast",
        // As the runtime build: a component defines each static function
        // that a shared header declares and it uses.
        "-Werror=undefined-internal",
        // The harness links the code without the firmware symbols.
        "-DGRKIT_HOST_ADDRESSES",
        target,
    ]);
    if let Some(include) = include {
        command.arg("-I").arg(include);
    }
    checked(
        command
            .arg("-I")
            .arg(root().join("runtime"))
            // Module code that a harness links in (runtime/sdk/grkit.h).
            .arg("-I")
            .arg(root().join("runtime/sdk"))
            .arg(source)
            .args(extra_sources)
            .arg("-o")
            .arg(executable)
            // A harness can call sqrt. On Linux it is in libm, after the sources.
            .arg("-lm"),
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
        unsigned char bytes[GRKIT_GRF_MAX_SIZE + 1];
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
        // The selective color bit without its field, and the removed glow bit.
        (84, vec![2]),
        (84, vec![4]),
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
    let mut selective_effect = sample(false);
    selective_effect["selective_color"] = Selective::from_definition(
        &json!({"hue": 108.6, "lower": [20, 35], "higher": [6, 14], "chroma": [6, 16]}),
    )
    .unwrap()
    .to_json();
    let selective = effects::encode(&selective_effect, &vec![0; effects::ICON_SIZE]).unwrap();
    corpus.push(("selective color".into(), selective.clone(), Some(true)));
    let selective_fields: Vec<(&str, usize, Vec<u8>)> = vec![
        ("short direction", 106, vec![0, 0, 0, 0]),
        (
            "long direction",
            106,
            [4096_u16, 4096]
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .collect(),
        ),
        (
            "equal lower tangents",
            110,
            [93_u16, 93].iter().flat_map(|v| v.to_le_bytes()).collect(),
        ),
        (
            "reversed higher tangents",
            114,
            [64_u16, 27].iter().flat_map(|v| v.to_le_bytes()).collect(),
        ),
        (
            "outer tangent past 1024",
            116,
            1025_u16.to_le_bytes().to_vec(),
        ),
        ("equal chroma limits", 118, vec![16, 16]),
        ("chroma limit past 181", 119, vec![182]),
        ("reserved byte", 120, vec![1]),
        ("monochrome flag", 80, 1_u32.to_le_bytes().to_vec()),
    ];
    for (description, offset, bytes) in selective_fields {
        let mut changed = selective.clone();
        changed[offset..offset + bytes.len()].copy_from_slice(&bytes);
        repair_crc(&mut changed);
        corpus.push((
            format!("selective color: {description}"),
            changed,
            Some(false),
        ));
    }
    let mut stray = color.clone();
    stray[112] = 1;
    repair_crc(&mut stray);
    corpus.push(("selective field without its bit".into(), stray, Some(false)));
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
    let extended = effects::encode(&extended_effect, &vec![0; effects::ICON_SIZE]).unwrap();
    assert_eq!(&extended[..8], effects::MAGIC);
    corpus.push(("extended post".into(), extended.clone(), Some(true)));
    // Hue sectors: 66 coefficients, then the control fields 10..20 at 8150.
    let mut hue_effect = sample(false);
    let matrices: Vec<Vec<i64>> = (0..11).map(|k| vec![3 - k, -2, k, 0, -4, 5 + k]).collect();
    hue_effect["hue_sectors"] = json!({"matrices": matrices, "weight": [0, 128, 255, 200]});
    let hue = effects::encode(&hue_effect, &vec![0; effects::ICON_SIZE]).unwrap();
    corpus.push(("hue sectors".into(), hue.clone(), Some(true)));
    let hue_fields: Vec<(&str, usize, Vec<u8>)> = vec![
        ("coefficient -128", 8084, vec![0x80]),
        ("row past the register", 8084, vec![0x9c, 0x9c]),
        (
            "start weight past 1024",
            8150,
            1025_u16.to_le_bytes().to_vec(),
        ),
        ("slope past 9 bits", 8158, 0x200_u16.to_le_bytes().to_vec()),
        ("moved breakpoint", 8166, 165_u16.to_le_bytes().to_vec()),
        ("byte after the sectors", 8172, vec![1]),
        ("without the feature bit", 84, 0_u32.to_le_bytes().to_vec()),
    ];
    for (description, offset, bytes) in hue_fields {
        let mut changed = hue.clone();
        changed[offset..offset + bytes.len()].copy_from_slice(&bytes);
        repair_crc(&mut changed);
        corpus.push((format!("hue sectors: {description}"), changed, Some(false)));
    }
    let mut mono_hue = mono.clone();
    mono_hue[84] = 8;
    mono_hue[8084..8172].copy_from_slice(&hue[8084..8172]);
    repair_crc(&mut mono_hue);
    corpus.push(("monochrome hue sectors".into(), mono_hue, Some(false)));
    // Hue levels: feature 16 with feature 8, 198 coefficients at 8192, and 8704 bytes.
    let mut level_effect = hue_effect.clone();
    let level: Vec<Vec<i64>> = (0..11).map(|k| vec![k - 5, 2, 0, -k, 7, 1]).collect();
    level_effect["hue_sectors"]["levels"] = json!([&level, &level, &level]);
    let levels = effects::encode(&level_effect, &vec![0; effects::ICON_SIZE]).unwrap();
    assert_eq!(levels.len(), effects::SIZE_WITH_LEVELS);
    corpus.push(("hue levels".into(), levels.clone(), Some(true)));
    let level_fields: Vec<(&str, usize, Vec<u8>)> = vec![
        ("coefficient -128", 8192, vec![0x80]),
        ("row past the register", 8300, vec![0x9c, 0x9c]),
        ("byte after the levels", 8390, vec![1]),
        ("last byte", 8703, vec![1]),
        ("without the levels bit", 84, 8_u32.to_le_bytes().to_vec()),
        ("without the sectors bit", 84, 16_u32.to_le_bytes().to_vec()),
        ("smaller size field", 8, 8192_u32.to_le_bytes().to_vec()),
    ];
    for (description, offset, bytes) in level_fields {
        let mut changed = levels.clone();
        changed[offset..offset + bytes.len()].copy_from_slice(&bytes);
        repair_crc(&mut changed);
        corpus.push((format!("hue levels: {description}"), changed, Some(false)));
    }
    let mut short_levels = levels[..effects::SIZE].to_vec();
    short_levels[8..12].copy_from_slice(&(effects::SIZE as u32).to_le_bytes());
    repair_crc(&mut short_levels);
    corpus.push(("hue levels in 8192 bytes".into(), short_levels, Some(false)));
    let mut long_hue = [hue.as_slice(), &[0; 512]].concat();
    long_hue[8..12].copy_from_slice(&(effects::SIZE_WITH_LEVELS as u32).to_le_bytes());
    repair_crc(&mut long_hue);
    corpus.push(("8704 bytes without levels".into(), long_hue, Some(false)));
    // Glow is not an effect field.
    let mut glow_effect = sample(false);
    glow_effect["glow"] = json!(1);
    assert!(effects::encode(&glow_effect, &vec![0; effects::ICON_SIZE]).is_err());
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
        (84, 4_u32.to_le_bytes().to_vec()),
        (106, vec![1]),
        // Byte 107 held a glow strength; it is reserved.
        (107, vec![1]),
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
        let mut changed = match index % 4 {
            0 => boundary.clone(),
            1 => mono.clone(),
            2 => selective.clone(),
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
_Static_assert(GRKIT_RECORD_ID + 32 <= GRKIT_RECORD_CODE, "Code overlap");
_Static_assert(GRKIT_RECORD_CODE + 2 <= GRKIT_RECORD_SIZE, "Record overflow");
_Static_assert(GRKIT_HOST_RETURN + 4 <= GRKIT_HOST_HEADER_SIZE, "Header word overflow");
_Static_assert(GRKIT_HOST_ICON_CHUNKS + 4 * 4 <= GRKIT_HOST_HEADER_SIZE, "Header overflow");
_Static_assert(GRKIT_RECORD_COEFFICIENTS + 1556 == GRKIT_RECORD_NAME, "Coefficient overlap");
_Static_assert(GRKIT_RECORD_NAME + 34 <= GRKIT_RECORD_MONOCHROME, "Name overlap");
_Static_assert(GRKIT_RECORD_CODE + 2 <= GRKIT_RECORD_FEATURES, "Extension overlap");
_Static_assert(GRKIT_RECORD_POST + 20 <= GRKIT_RECORD_SIZE, "Extension overflow");
int main(void) {
    printf("%u %u %u %u %u %u %u %u %u %u %u %u %u %u %u %u %u\n", GRKIT_HOST_HEADER_SIZE, GRKIT_RECORD_SIZE,
           GRKIT_PICTURE_SCRATCH, GRKIT_HOST_COUNT, GRKIT_HOST_RECORDS, GRKIT_HOST_ACTIVE, GRKIT_HOST_MODULE_FILTERS,
           GRKIT_HOST_FRAME_RATIO, GRKIT_RECORD_CODE, GRKIT_HOST_MANAGE, GRKIT_TEXT_SIZE,
           GRKIT_TEXT_SUMMARY, GRKIT_TEXT_STRINGS, GRKIT_TEXT_LANGUAGES, GRKIT_TEXT_RECORDS, GRKIT_SUMMARY_UNITS, GRKIT_TAIL_SIZE);
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
            runtime::PICTURE_SCRATCH,
            runtime::HOST_COUNT,
            runtime::HOST_RECORDS,
            runtime::HOST_ACTIVE,
            grkit_core::abi::HOST_MODULE_FILTERS,
            runtime::HOST_FRAME_RATIO,
            runtime::RECORD_CODE,
            grkit_core::abi::HOST_MANAGE,
            menu_text::TEXT_SIZE,
            menu_text::TEXT_SUMMARY,
            menu_text::STRINGS,
            language::Language::ALL.len(),
            runtime::MAX_COUNT,
            menu_text::SUMMARY_UNITS,
            runtime::TAIL_SIZE,
        ]
    );
}

#[test]
fn native_menu_contract_preserves_events_icons_and_empty_catalog() {
    let Some(compiler) = compiler() else {
        return;
    };
    let temporary = tempfile::tempdir().unwrap();
    // The harness reads the label table that the installer copies.
    let table = grkit_core::menu_text::table("0.0.0").unwrap();
    let bytes: Vec<String> = table.iter().map(u8::to_string).collect();
    fs::write(
        temporary.path().join("menu_text.h"),
        format!(
            "static const unsigned char menu_text_table[] __attribute__((aligned(2))) = {{{}}};\n",
            bytes.join(",")
        ),
    )
    .unwrap();
    // The harness gives the behavior of each target where it differs.
    for (index, target) in targets().iter().enumerate() {
        let executable = temporary.path().join(format!("menu_contract_{index}"));
        compile_with(
            target,
            &compiler,
            &root().join("tests/runtime/menu_contract.c"),
            &executable,
            &[],
            Some(temporary.path()),
        );
        checked(&mut Command::new(&executable));
    }
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
fn native_directory_contract_skips_long_names_and_a_stale_listing() {
    let Some(compiler) = compiler() else {
        return;
    };
    let temporary = tempfile::tempdir().unwrap();
    let executable = temporary.path().join("directory_contract");
    compile(
        &compiler,
        &root().join("tests/runtime/directory_contract.c"),
        &executable,
        &[],
    );
    checked(&mut Command::new(&executable));
}

#[test]
fn native_state_contract_reads_the_pairs_of_the_loader() {
    let Some(compiler) = compiler() else {
        return;
    };
    let temporary = tempfile::tempdir().unwrap();
    let executable = temporary.path().join("state_contract");
    compile(
        &compiler,
        &root().join("tests/runtime/state_contract.c"),
        &executable,
        &[],
    );
    checked(&mut Command::new(&executable));
}

/// Each target, because the native tables and the grain form differ by
/// camera family.
#[test]
fn native_recipes_contract_saves_applies_and_undoes_recipes() {
    let Some(compiler) = compiler() else {
        return;
    };
    let temporary = tempfile::tempdir().unwrap();
    for target in targets() {
        let executable = temporary
            .path()
            .join(format!("recipes_contract{}", target.to_lowercase()));
        compile_for(
            &target,
            &compiler,
            &root().join("tests/runtime/recipes_contract.c"),
            &executable,
            &[],
        );
        checked(&mut Command::new(&executable));
    }
}

/// The numeric `#define`s of a runtime header: name and value.
fn defines(path: &str) -> std::collections::HashMap<String, u64> {
    let text = fs::read_to_string(root().join(path)).unwrap();
    text.lines()
        .filter_map(|line| {
            let mut words = line.strip_prefix("#define ")?.split_whitespace();
            let (name, value) = (words.next()?, words.next()?.trim_end_matches('U'));
            let value = match value.strip_prefix("0x") {
                Some(hex) => u64::from_str_radix(hex, 16).ok()?,
                None => value.parse().ok()?,
            };
            Some((name.to_owned(), value))
        })
        .collect()
}

/// The module format of the Rust writer (`grkit_core::grm`) and of the
/// installer (`runtime/module_link.h`), and the page layout of the core
/// (`runtime/modules.h`), which reads module pages as the installer does.
#[test]
fn the_module_format_constants_agree() {
    let link = defines("runtime/module_link.h");
    let core = defines("runtime/modules.h");
    for (name, value) in [
        ("GRM_MAGIC", u64::from(grm::MAGIC)),
        ("GRM_FORMAT", grm::FORMAT_VERSION.into()),
        ("GRM_HEADER", grm::HEADER_SIZE as u64),
        ("GRM_DATA_LINK", grm::DATA_LINK.into()),
        ("GRM_DATA_LOCATION", grm::DATA_LOCATION.into()),
        ("GRM_MAX_SEGMENT", grm::MAX_SEGMENT as u64),
        ("GRM_ABI_MAJOR", grm::ABI_MAJOR.into()),
        ("GRM_ABI_MINOR", grm::ABI_MINOR.into()),
        ("GRM_DESCRIPTOR", grm::DESCRIPTOR_SIZE.into()),
        ("GRM_MAX_ORDER", grm::MAX_ORDER.into()),
    ] {
        assert_eq!(link[name], value, "{name}");
    }
    for (core_name, link_name) in [
        ("PAGE_SETTING", "GRM_PAGE_SETTING_AT"),
        ("PAGE_FLAGS", "GRM_PAGE_FLAGS_AT"),
        ("PAGE_TITLE", "GRM_PAGE_TITLE_AT"),
        ("PAGE_SUMMARIES", "GRM_PAGE_SUMMARIES_AT"),
        ("PAGE_LABELS", "GRM_PAGE_LABELS_AT"),
        ("PAGE_SETTING_ROWS", "GRM_PAGE_SETTING_ROWS_AT"),
        ("PAGE_SETTING_LIST", "GRM_PAGE_SETTINGS_AT"),
        ("ROW_SETTING", "GRM_ROW_SETTING_AT"),
        ("ROW_LABEL", "GRM_ROW_LABEL_AT"),
        ("ROW_VALUES", "GRM_ROW_VALUES_AT"),
        ("ROW_SIZE", "GRM_ROW"),
        ("CHOICE_OFF_ROW", "GRM_CHOICE_OFF_ROW"),
        ("CHOICE_OPTIONS", "GRM_CHOICE_OPTIONS"),
    ] {
        assert_eq!(core[core_name], link[link_name], "{core_name}");
    }
}

/// The SDK descriptor structs have the layout that the installer reads, on
/// the 32-bit ARM of the camera (`tests/runtime/layout_contract.c`).
#[test]
fn the_sdk_descriptor_has_the_layout_that_the_installer_reads() {
    let clang = env::var_os("GRKIT_CLANG").unwrap_or_else(|| "clang".into());
    if Command::new(&clang).arg("--version").output().is_err() {
        assert!(
            env::var("GRKIT_REQUIRE_NATIVE_TESTS").as_deref() != Ok("1"),
            "GRKIT_REQUIRE_NATIVE_TESTS=1 requires clang for the ARM layout check; set GRKIT_CLANG"
        );
        eprintln!("SKIPPED ARM layout check: no clang found; set GRKIT_CLANG");
        return;
    }
    checked(
        Command::new(&clang)
            .args(["--target=armv7a-none-eabi", "-std=c11", "-fsyntax-only"])
            .arg("-I")
            .arg(root().join("runtime"))
            .arg("-I")
            .arg(root().join("runtime/sdk"))
            .arg(root().join("tests/runtime/layout_contract.c")),
    );
}

/// A module file that the Rust writer encodes links in the installer as the
/// writer means it: the header fields, the settings and used settings of the
/// descriptor, each relocated word, and each import
/// (`tests/runtime/grm_file_contract.c`).
#[test]
fn the_installer_links_a_module_that_rust_writes() {
    let Some(compiler) = compiler() else {
        return;
    };
    const CODE_BASE: u32 = 0x5400_0000;
    const DATA_BASE: u32 = 0x5900_0000;
    let mut code = vec![0u8; 112];
    let mut put = |at: usize, value: u32| code[at..at + 4].copy_from_slice(&value.to_le_bytes());
    // A pointer into the data, a Thumb pointer into the code, and an import.
    put(0, grm::DATA_LINK + 4);
    put(4, 0x41);
    // The descriptor at 16: one setting at 56, one used setting at 80.
    put(16, grm::DESCRIPTOR_SIZE);
    put(20, grm::ABI_MAJOR << 16 | grm::ABI_MINOR);
    put(32, 56);
    put(36, 1);
    put(40, 80);
    put(44, 1);
    // The setting: MODE, 3 values, saved.
    put(56, 16);
    put(60, 72);
    put(64, 3);
    put(68, 2);
    put(80, 84);
    code[72..77].copy_from_slice(b"MODE\0");
    code[84..97].copy_from_slice(b"?CORE.ASPECT\0");
    let mut data = vec![0u8; 8];
    data[0..4].copy_from_slice(&(grm::DATA_LINK + 8).to_le_bytes());
    let module = Module {
        id: "CROSS".into(),
        version: 7,
        order: 42,
        code,
        data,
        bss_size: 12,
        descriptor: 16,
        code_relocations: vec![0, 4, 32, 40, 60, 80],
        data_relocations: vec![0],
        imports: vec![
            Import {
                name: "overlay_label".into(),
                segment: Segment::Code,
                offset: 8,
            },
            Import {
                name: "divide".into(),
                segment: Segment::Data,
                offset: 4,
            },
        ],
    };
    let temporary = tempfile::tempdir().unwrap();
    let file = temporary.path().join("CROSS.GRM");
    fs::write(&file, module.encode().unwrap()).unwrap();
    let executable = temporary.path().join("grm_file_contract");
    compile(
        &compiler,
        &root().join("tests/runtime/grm_file_contract.c"),
        &executable,
        &[],
    );
    let output = checked(Command::new(&executable).arg(&file));
    // The link that the writer means: a code or data link address moves to
    // the base of its segment, and an import gets the value of its symbol.
    let link = |bytes: &[u8], relocations: &[u32], segment: Segment| {
        let mut words: Vec<u32> = bytes
            .chunks_exact(4)
            .map(|w| u32::from_le_bytes(w.try_into().unwrap()))
            .collect();
        for &offset in relocations {
            let word = &mut words[offset as usize / 4];
            *word = if *word >= grm::DATA_LINK {
                DATA_BASE + (*word - grm::DATA_LINK)
            } else {
                CODE_BASE + *word
            };
        }
        for import in module.imports.iter().filter(|i| i.segment == segment) {
            words[import.offset as usize / 4] = match import.name.as_str() {
                "overlay_label" => 0x1111_1111,
                _ => 0x2222_2222,
            };
        }
        words
    };
    let mut expected = format!(
        "status 0\nid CROSS\nversion 7\norder 42\nabi {}\ncode 112\ndata 8\nbss 12\n\
         descriptor 16\nsettings 1\nuses 1\nsetting MODE 3 2\nuse ?CORE.ASPECT\n",
        grm::ABI_MAJOR << 16 | grm::ABI_MINOR
    );
    for word in link(&module.code, &module.code_relocations, Segment::Code) {
        expected.push_str(&format!("c {word:08x}\n"));
    }
    for word in link(&module.data, &module.data_relocations, Segment::Data) {
        expected.push_str(&format!("d {word:08x}\n"));
    }
    assert_eq!(String::from_utf8_lossy(&output.stdout), expected);
}

#[test]
fn the_package_scripts_test_the_install_statuses_of_the_core() {
    let header = fs::read_to_string(root().join("runtime/install_status.h")).unwrap();
    for (name, value) in [
        ("STATUS_RELOCATION", grkit_core::runtime::STATUS_RELOCATION),
        (
            "STATUS_OTHER_BUILD",
            grkit_core::runtime::STATUS_OTHER_BUILD,
        ),
        (
            "STATUS_LOADER_NO_PLAN",
            grkit_core::runtime::STATUS_LOADER_NO_PLAN,
        ),
        (
            "STATUS_LOADER_BAD_PLAN",
            grkit_core::runtime::STATUS_LOADER_BAD_PLAN,
        ),
        (
            "STATUS_LOADER_NO_INSTALL",
            grkit_core::runtime::STATUS_LOADER_NO_INSTALL,
        ),
    ] {
        let line = format!("    {name} = {value},");
        assert!(header.lines().any(|l| l == line), "{line}");
    }
}

#[test]
fn native_module_link_contract_links_and_rejects_modules() {
    let Some(compiler) = compiler() else {
        return;
    };
    let temporary = tempfile::tempdir().unwrap();
    let executable = temporary.path().join("module_link_contract");
    compile(
        &compiler,
        &root().join("tests/runtime/module_link_contract.c"),
        &executable,
        &[],
    );
    checked(&mut Command::new(&executable));
}

#[test]
fn native_install_contract_plans_and_links_the_modules_of_the_card() {
    let Some(compiler) = compiler() else {
        return;
    };
    let temporary = tempfile::tempdir().unwrap();
    let executable = temporary.path().join("install_contract");
    compile(
        &compiler,
        &root().join("tests/runtime/install_contract.c"),
        &executable,
        &[],
    );
    checked(&mut Command::new(&executable));
}

#[test]
fn native_relocation_contract_finds_moved_windows_and_rejects_changes() {
    let Some(compiler) = compiler() else {
        return;
    };
    let temporary = tempfile::tempdir().unwrap();
    let executable = temporary.path().join("relocation_contract");
    compile(
        &compiler,
        &root().join("tests/runtime/relocation_contract.c"),
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
    let labels = grkit_core::artwork::overlay_labels()
        .iter()
        .map(u8::to_string)
        .collect::<Vec<_>>()
        .join(",");
    std::fs::write(
        temporary.path().join("overlay_labels.h"),
        format!("static const unsigned char overlay_labels[] = {{{labels}}};\n"),
    )
    .unwrap();
    // The module headers of Framing and Guides, which the harness compiles
    // in one file: the index of each setting in its module.toml, and each
    // setting at its default 0.
    let mut module = String::from("#ifndef GRKIT_MODULE_H\n#define GRKIT_MODULE_H\n");
    for name in ["guides", "framing"] {
        let manifest: toml::Table =
            std::fs::read_to_string(root().join("modules").join(name).join("module.toml"))
                .unwrap()
                .parse()
                .unwrap();
        for (index, setting) in manifest["settings"].as_array().unwrap().iter().enumerate() {
            module += &format!(
                "#define SETTING_{} {index}U\n",
                setting["key"].as_str().unwrap()
            );
        }
    }
    module += "const GrkitContext grkit_self;\n\
               u32 setting_get(u32 handle) { (void)handle; return 0; }\n#endif\n";
    std::fs::write(temporary.path().join("grkit_module.h"), module).unwrap();
    for (index, target) in targets().iter().enumerate() {
        let executable = temporary.path().join(format!("framing_contract_{index}"));
        compile_with(
            target,
            &compiler,
            &root().join("tests/runtime/framing_contract.c"),
            &executable,
            &[],
            Some(temporary.path()),
        );
        checked(&mut Command::new(&executable));
    }
}

#[test]
fn native_selective_table_matches_the_site_preview() {
    let Some(compiler) = compiler() else {
        return;
    };
    let temporary = tempfile::tempdir().unwrap();
    let executable = temporary.path().join("selective_table");
    compile(
        &compiler,
        &root().join("tests/runtime/selective_table.c"),
        &executable,
        &[],
    );
    for definition in [
        json!({"hue": 108.6, "lower": [20, 35], "higher": [6, 14], "chroma": [6, 16]}),
        json!({"hue": 250, "lower": [0, 75], "higher": [10, 11], "chroma": [0, 181]}),
        json!({"hue": 0, "lower": [5, 40], "higher": [30, 60], "chroma": [40, 41]}),
        json!({"hue": 359.9, "lower": [44.9, 45], "higher": [0.5, 2], "chroma": [1, 100]}),
    ] {
        let selective = Selective::from_definition(&definition).unwrap();
        let output = checked(Command::new(&executable).arg(hex::encode(selective.to_bytes())));
        assert_eq!(output.stdout, selective.table(), "{definition}");
    }
}

#[test]
fn native_picture_contract_applies_post_matrix_and_stamp() {
    let Some(compiler) = compiler() else {
        return;
    };
    let temporary = tempfile::tempdir().unwrap();
    // The header that `grkit module build` generates for modules/date: the
    // handles of its settings STYLE, LOOK, and POSITION.
    fs::write(
        temporary.path().join("grkit_module.h"),
        "#define SETTING_STYLE GRKIT_SETTING(0)\n#define SETTING_LOOK GRKIT_SETTING(1)\n\
         #define SETTING_POSITION GRKIT_SETTING(2)\n",
    )
    .unwrap();
    for (index, target) in targets().iter().enumerate() {
        let executable = temporary.path().join(format!("picture_contract_{index}"));
        compile_with(
            target,
            &compiler,
            &root().join("tests/runtime/picture_contract.c"),
            &executable,
            &[],
            Some(temporary.path()),
        );
        checked(&mut Command::new(&executable));
    }
}

/// The menu, Framing, and picture contracts above run for each target. This
/// check also compiles the code of each target as it is, without the test
/// replacements of the harnesses.
#[test]
fn runtime_c_sources_compile_for_each_target() {
    let Some(compiler) = compiler() else {
        return;
    };
    for target in targets() {
        for source in ["menu.c", "framing.c", "picture.c"] {
            checked(
                Command::new(&compiler)
                    .args([
                        "-std=c11",
                        "-fsyntax-only",
                        "-Wno-int-to-pointer-cast",
                        "-Wno-pointer-to-int-cast",
                        target.as_str(),
                    ])
                    .arg("-I")
                    .arg(root().join("runtime"))
                    .arg(root().join("runtime").join(source)),
            );
        }
    }
    // Without a target definition, firmware.h stops the build instead of
    // selecting the GR III addresses.
    let output = Command::new(&compiler)
        .args(["-std=c11", "-fsyntax-only", "-x", "c"])
        .arg(root().join("runtime/firmware.h"))
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("GRKIT_TARGET_"));
}

/// Modules are target neutral: each module source includes only the module
/// SDK and the header that `grkit module build` generates, and names no
/// firmware symbol or address.
#[test]
fn modules_hold_no_firmware_detail() {
    let mut checked = 0;
    let folders = [(
        "modules",
        &["#include \"grkit.h\"", "#include \"grkit_module.h\""][..],
    )];
    for (folder, includes) in folders {
        for module in fs::read_dir(root().join(folder)).unwrap() {
            for file in fs::read_dir(module.unwrap().path()).unwrap() {
                let path = file.unwrap().path();
                // test.c of a module is its host test (`grkit module test`).
                if !matches!(path.extension().and_then(|e| e.to_str()), Some("c" | "h"))
                    || path.file_name().is_some_and(|name| name == "test.c")
                {
                    continue;
                }
                let text = fs::read_to_string(&path).unwrap();
                let name = path.display();
                for line in text.lines().map(str::trim) {
                    if line.starts_with("#include") {
                        assert!(includes.contains(&line), "{name}: {line}");
                    }
                }
                for symbol in ["FW_", "HOOK_", "firmware.h", "LIVEVIEW_", "ADJ_"] {
                    assert!(!text.contains(symbol), "{name} names {symbol}");
                }
                // RTOS code and data addresses: 0x53000000..0x56000000.
                for (at, _) in text.match_indices("0x5") {
                    let digits: String = text[at + 2..]
                        .chars()
                        .take_while(char::is_ascii_hexdigit)
                        .collect();
                    assert!(
                        digits.len() < 8 || !matches!(&digits[..2], "53" | "54" | "55"),
                        "{name} has an address 0x{digits}"
                    );
                }
                checked += 1;
            }
        }
    }
    assert!(checked >= 3, "Expected the module sources");
}
