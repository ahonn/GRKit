use super::*;

/// The version of headers that tests pin, so a new GRKit version keeps the pins.
const TEST_VERSION: &str = "0.0.0";

// Fixed contracts of the installer header on the synthetic GR III image, with its relocation tables.
// These checks deliberately do not derive their expected bytes from the Rust producer.
#[test]
fn generated_header_matches_fixed_synthetic_contract() -> Result<()> {
    let plan = PatchPlan {
        exports: vec![],
        patches: vec![
            Patch {
                address: 0x53002080,
                points: 0,
                before: vec![0; 8],
                after: hex::decode("4023015801020304")?,
                site: None,
            },
            Patch {
                address: 0x531a48f0,
                points: 0,
                before: hex::decode("f431d0e5")?,
                after: hex::decode("000000ea")?,
                site: None,
            },
        ],
        relocations: vec![(0, 0)],
        references: Vec::new(),
    };
    validate_plan(&plan)?;
    let header = loader::header(&plan, &synthetic_gr3(), TEST_VERSION)?;
    // The card layout: the package in SCRIPT/GRKIT, the data in GRKIT.
    for line in [
        "#define PLAN_PATH \"/media/storage1/SCRIPT/GRKIT/PLAN.BIN\"",
        "#define INBOX_PATH \"/media/storage1/SCRIPT/GRKIT/EFFECTS\"",
        "#define PACKAGE_PATH \"/media/storage1/SCRIPT/GRKIT/PACKAGE.TXT\"",
        "#define IMPORTED_PATH \"/media/storage1/GRKIT/IMPORTED.TXT\"",
        "#define PACKAGE_MODULES_PATH \"/media/storage1/SCRIPT/GRKIT/MODULES\"",
        "#define LIBRARY_PATH \"/media/storage1/GRKIT/EFFECTS\"",
        "#define STATE_PATH \"/media/storage1/GRKIT/STATE.TXT\"",
        "#define REPORT_PATH \"/media/storage1/GRKIT/REPORT.TXT\"",
        "#define SAFE_PATH \"/media/storage1/GRKIT/SAFE\"",
        "#define SAFE_TEXT_PATH \"/media/storage1/GRKIT/SAFE.TXT\"",
        "#define PROFILE_PATH \"/media/storage1/GRKIT/PROFILE\"",
        "#define PROFILE_TEXT_PATH \"/media/storage1/GRKIT/PROFILE.TXT\"",
        "#define MODULES_PATH \"/media/storage1/GRKIT/MODULES\"",
        "#define LEGACY_STATE_PATH \"/media/storage1/SCRIPT/PLUGINS/STATE.TXT\"",
        "#define LEGACY_EFFECTS_PATH \"/media/storage1/SCRIPT/PLUGINS/EFFECTS\"",
        "#define LEGACY_STATE_COUNT 6U",
    ] {
        assert!(header.lines().any(|l| l == line), "{line}");
    }
    // The installer names its runtime build for the package check.
    assert!(header.starts_with(
        "static const char target_marker[] __attribute__((used)) = \"GRKIT_TARGET=gr3-2.10\";\n"
    ));
    // GR III has one runtime, so it names no build for `--known`.
    assert!(header.contains("#define RTOS_IDENTITY 0x53002038U\n#define RTOS_IDENTITY_COUNT 0U\n"));
    snapshot("installer-gr3-synthetic.h", &summarized(&header));
    Ok(())
}

#[test]
fn generated_header_marks_only_rtos_tail_patches_as_unchecked() -> Result<()> {
    let plan = PatchPlan {
        exports: vec![],
        patches: vec![
            Patch {
                address: 0x53002080,
                points: 0,
                before: vec![0; 8],
                after: hex::decode("4023015801020304")?,
                site: None,
            },
            Patch {
                address: target::GR3_210.layout.tail.0,
                points: 0,
                before: vec![0; 4],
                after: vec![1, 2, 3, 4],
                site: None,
            },
        ],
        relocations: vec![(0, 0)],
        references: Vec::new(),
    };
    let header = loader::header(&plan, &synthetic_gr3(), TEST_VERSION)?;
    assert!(header.contains("{0x53002080,8,0,before_0,after_0},"));
    assert!(header.contains("{0x540b6000,4,1,before_1,after_1},"));
    Ok(())
}

#[test]
fn generated_header_gives_the_points_of_each_patch() -> Result<()> {
    let plan = PatchPlan {
        exports: vec![],
        patches: vec![Patch {
            address: target::GR3_210.layout.tail.0,
            points: 0,
            before: vec![0; 4],
            after: vec![1, 2, 3, 4],
            site: None,
        }],
        relocations: vec![],
        references: Vec::new(),
    };
    let header = loader::header(&plan, &synthetic_gr3(), TEST_VERSION)?;
    assert!(header.contains("{0x540b6000,4,1,before_0,after_0},"));
    // Code in the tail belongs to no extension point.
    assert!(header.contains("static const u32 patch_points[] = {0x0};"));
    // CORE.TONE needs the tuning and picture encode points (bits 1 and 4),
    // CORE.ASPECT the overlay and capture aspect points (bits 2 and 3),
    // CORE.GRAIN the grain point (bit 5), CORE.GRAIN_SIZE the grain size
    // point (bit 6), and CORE.RECIPE none.
    assert!(header.contains(
        "static const char *const core_setting_names[] = {\"CORE.TONE\",\"CORE.ASPECT\",\"CORE.GRAIN\",\"CORE.GRAIN_SIZE\",\"CORE.RECIPE\"};"
    ));
    assert!(header.contains("static const u32 core_setting_points[] = {0x12,0xc,0x20,0x40,0x0};"));
    Ok(())
}

#[test]
fn menu_assembly_symbols_are_typed_functions() {
    // Thumb menu C reaches these A32 symbols with BL. Without a function type,
    // the linker keeps BL instead of BLX and the call runs A32 code as Thumb.
    let source = include_str!("../../../../runtime/menu.S");
    let lines: Vec<_> = source.lines().map(str::trim).collect();
    let mut globals = 0;
    for (index, line) in lines.iter().enumerate() {
        let Some(name) = line.strip_prefix(".global ") else {
            continue;
        };
        globals += 1;
        assert_eq!(
            lines.get(index + 1).copied(),
            Some(format!(".type {name}, %function").as_str()),
            "{name}"
        );
    }
    assert!(globals > 0);
}

#[test]
fn production_scripts_and_activation_match_fixed_contracts() {
    let files = grkit_core::runtime::runtime_files(b"ELF");
    assert_eq!(files.len(), 5);
    assert_eq!(files["SCRIPT/GRKIT/CORE.BIN"], b"ELF");
    for (path, name) in [
        ("SCRIPT/STARTUP.TTL", "STARTUP.TTL"),
        ("SCRIPT/GRKIT/LOADER.TTL", "LOADER.TTL"),
        ("SCRIPT/GRKIT/SCAN.SH", "SCAN.SH"),
        ("SCRIPT/GRKIT/INSTALL.SH", "INSTALL.SH"),
    ] {
        snapshot(name, std::str::from_utf8(&files[path]).unwrap());
    }
    assert!(!files
        .values()
        .any(|data| data.windows(8).any(|part| part == b"--select")));
    // Each camera gets the same files under its own ModeSet name.
    for (camera, modeset) in [
        (&grkit_core::camera::GR3, "00078350.588"),
        (&grkit_core::camera::GR3X, "00078490.609"),
        (&grkit_core::camera::GR4, "00078560.636"),
    ] {
        let activation = grkit_core::runtime::activation_files(camera);
        assert_eq!(activation.len(), 2);
        assert_eq!(activation[modeset], b"[SCRIPT_EN_MODE EN]\r\n");
        assert_eq!(
            activation["DEVELOP.MOD"],
            [7, 1, 44, 31, 16, 3, 30, 22, 5, 45]
        );
    }
}
