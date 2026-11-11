use super::*;

/// The probe module of `tests/modules` builds into a module that imports only
/// services of the core and the installer, and registers one layer.
#[test]
#[ignore = "Requires ARM-capable Clang and LLD; run with --ignored"]
fn synthetic_probe_module_builds() -> Result<()> {
    let tools = tools()?;
    let directory = tempfile::tempdir()?;
    let bytes = build_module(
        "PROBE",
        1,
        &[Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/modules/probe/probe.c")],
        &directory.path().join("objects"),
        tools.clang.to_str().unwrap(),
        tools.lld.to_str().unwrap(),
    )?;
    let module = grkit_core::grm::Module::decode(&bytes)?;
    assert_eq!(module.id, "PROBE");
    let mut names: Vec<_> = module
        .imports
        .iter()
        .map(|import| import.name.as_str())
        .collect();
    names.sort_unstable();
    assert_eq!(
        names,
        ["grkit_self", "overlay_label", "setting_get", "setting_set"]
    );
    // The descriptor: its size, the ABI, then one registration of a layer.
    let word =
        |offset: u32| u32::from_le_bytes(module.code[offset as usize..][..4].try_into().unwrap());
    assert_eq!(
        word(module.descriptor),
        grkit_core::grm::DESCRIPTOR_FLAGS_SIZE
    );
    assert_eq!(
        word(module.descriptor + 4),
        grkit_core::grm::ABI_MAJOR << 16 | grkit_core::grm::ABI_MINOR
    );
    assert_eq!(word(module.descriptor + 12), 1);
    assert_eq!(word(word(module.descriptor + 8)), 1);
    Ok(())
}

/// The module folders of the repository (`modules/<id>`).
fn module_folders() -> Vec<std::path::PathBuf> {
    let mut folders: Vec<_> =
        fs::read_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../modules"))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.join("module.toml").is_file())
            .collect();
    folders.sort();
    folders
}

/// Each module folder passes its host test (`grkit module test`). The test
/// needs a host C compiler: `cc`, or GRKIT_HOST_CC.
#[test]
fn module_folders_pass_their_host_tests() -> Result<()> {
    let cc = std::env::var("GRKIT_HOST_CC").unwrap_or_else(|_| "cc".into());
    if std::process::Command::new(&cc)
        .arg("--version")
        .output()
        .is_err()
    {
        assert!(
            std::env::var_os("GRKIT_REQUIRE_NATIVE_TESTS").is_none(),
            "No host C compiler {cc}"
        );
        return Ok(());
    }
    for folder in module_folders() {
        let directory = tempfile::tempdir()?;
        // A module with C sources has a test.
        let tested = test_module_manifest(&folder, &directory.path().join("build"), &cc)
            .with_context(|| format!("Test {}", folder.display()))?;
        assert_eq!(
            tested,
            !module::generate(&folder)?.0.sources.is_empty(),
            "{}",
            folder.display()
        );
    }
    Ok(())
}

/// Write the module template (`grkit module new`) into a new folder.
fn template_folder(directory: &Path, id: &str) -> Result<std::path::PathBuf> {
    let folder = directory.join(id.to_ascii_lowercase());
    for (name, bytes) in grkit_core::grm::template(id)? {
        let path = folder.join(name);
        fs::create_dir_all(path.parent().unwrap())?;
        fs::write(path, bytes)?;
    }
    Ok(folder)
}

/// The module template passes its host test, as the module folders do.
#[test]
fn module_template_passes_its_host_test() -> Result<()> {
    assert!(grkit_core::grm::template("hello").is_err());
    assert!(grkit_core::grm::template("TOOLONGID").is_err());
    let cc = std::env::var("GRKIT_HOST_CC").unwrap_or_else(|_| "cc".into());
    if std::process::Command::new(&cc)
        .arg("--version")
        .output()
        .is_err()
    {
        assert!(
            std::env::var_os("GRKIT_REQUIRE_NATIVE_TESTS").is_none(),
            "No host C compiler {cc}"
        );
        return Ok(());
    }
    let directory = tempfile::tempdir()?;
    // An ID that starts with a digit and has a character that no tile shows.
    let folder = template_folder(directory.path(), "1_NEW")?;
    assert!(test_module_manifest(
        &folder,
        &directory.path().join("build"),
        &cc
    )?);
    // The camera's Korean font has no 똠, so the build rejects it.
    fs::write(
        folder.join("text/ko.toml"),
        "title = \"똠\"\nrows = [\"꺼짐\", \"켜짐\"]\n",
    )?;
    let error = test_module_manifest(&folder, &directory.path().join("build-ko"), &cc)
        .unwrap_err()
        .to_string();
    assert!(error.contains("cannot draw '똠'"), "{error}");
    Ok(())
}

/// The module template builds for the camera.
#[test]
#[ignore = "Requires ARM-capable Clang and LLD; run with --ignored"]
fn synthetic_module_template_builds() -> Result<()> {
    let tools = tools()?;
    let directory = tempfile::tempdir()?;
    let folder = template_folder(directory.path(), "HELLO")?;
    let (id, bytes) = build_module_manifest(
        &folder,
        &directory.path().join("objects"),
        tools.clang.to_str().unwrap(),
        tools.lld.to_str().unwrap(),
    )?;
    assert_eq!(id, "HELLO");
    assert_eq!(grkit_core::grm::Module::decode(&bytes)?.id, "HELLO");
    Ok(())
}

/// Each module folder builds for the camera, and its file decodes.
#[test]
#[ignore = "Requires ARM-capable Clang and LLD; run with --ignored"]
fn synthetic_module_folders_build() -> Result<()> {
    let tools = tools()?;
    // The installer link and registration checks, built for the host
    // (`tests/runtime/grm_registrations_contract.c`).
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let checker = tempfile::tempdir()?;
    let contract = checker.path().join("grm_registrations_contract");
    let cc = std::env::var("GRKIT_HOST_CC").unwrap_or_else(|_| "cc".into());
    let output = std::process::Command::new(&cc)
        .arg("-I")
        .arg(root.join("runtime"))
        .arg(root.join("tests/runtime/grm_registrations_contract.c"))
        .arg("-o")
        .arg(&contract)
        .output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for folder in module_folders() {
        let directory = tempfile::tempdir()?;
        let (id, bytes) = build_module_manifest(
            &folder,
            &directory.path().join("objects"),
            tools.clang.to_str().unwrap(),
            tools.lld.to_str().unwrap(),
        )?;
        let module = grkit_core::grm::Module::decode(&bytes)?;
        assert_eq!(module.id, id);
        assert!(module
            .imports
            .iter()
            .all(|i| grkit_core::grm::importable(&i.name)));
        // The installer accepts its pages, layers, filters, and indicators.
        let file = directory.path().join("MODULE.GRM");
        fs::write(&file, &bytes)?;
        let output = std::process::Command::new(&contract).arg(&file).output()?;
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).trim(),
            "status 0",
            "{}",
            folder.display()
        );
    }
    Ok(())
}

/// The stack frame of each function of a module source, as `-fstack-usage`
/// gives it for the module build flags: (function, bytes).
fn stack_frames(module: &str, source: &str) -> Result<Vec<(String, usize)>> {
    let tools = tools()?;
    let directory = tempfile::tempdir()?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let folder = root.join("modules").join(module);
    fs::write(
        directory.path().join("grkit_module.h"),
        module::generate(&folder)?.2,
    )?;
    let object = directory.path().join("source.o");
    let output = std::process::Command::new(&tools.clang)
        .args(crate::toolchain::CAMERA_TARGET)
        .args([
            "-mthumb",
            "-Os",
            "-ffreestanding",
            "-fno-builtin",
            "-fno-common",
            "-fno-stack-protector",
            "-fno-unwind-tables",
            "-fno-asynchronous-unwind-tables",
            "-mlong-calls",
            "-mno-movt",
            "-fstack-usage",
        ])
        .arg("-I")
        .arg(root.join("runtime/sdk"))
        .arg("-I")
        .arg(directory.path())
        .arg("-c")
        .arg(folder.join(source))
        .arg("-o")
        .arg(&object)
        .output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let usage = fs::read_to_string(object.with_extension("su"))?;
    let mut frames = Vec::new();
    for line in usage.lines() {
        let fields: Vec<_> = line.split('\t').collect();
        let function = fields[0].rsplit(':').next().unwrap_or_default().to_owned();
        frames.push((function, fields[1].parse()?));
    }
    assert!(!frames.is_empty(), "Missing stack measurements of {module}");
    Ok(frames)
}

/// Date blur runs beneath the firmware JPEG encoder. Its line workspace must
/// stay in picture scratch, rather than consume that task's remaining stack.
#[test]
#[ignore = "Requires ARM-capable Clang and LLD; run with --ignored"]
fn date_blur_stays_within_stack_budget() -> Result<()> {
    for (function, bytes) in stack_frames("date", "stamp.c")? {
        // Also catch a large workspace if the compiler inlines the blur.
        assert!(
            bytes <= 1024,
            "Date stack frame exceeds budget: {function} {bytes}"
        );
        if function == "blur_line" {
            assert!(
                bytes <= 128,
                "Date blur stack frame exceeds budget: {bytes}"
            );
        }
    }
    Ok(())
}

/// Glow also runs beneath the firmware JPEG encoder. Its row workspaces must
/// stay in picture scratch: a 992-byte frame of light_row made the encoder
/// task warn of a low stack.
#[test]
#[ignore = "Requires ARM-capable Clang and LLD; run with --ignored"]
fn glow_stays_within_stack_budget() -> Result<()> {
    for (function, bytes) in stack_frames("glow", "glow.c")? {
        assert!(
            bytes <= 640,
            "Glow stack frame exceeds budget: {function} {bytes}"
        );
    }
    Ok(())
}

/// A module that needs a compiler helper does not build, because the core does
/// not export it.
#[test]
#[ignore = "Requires ARM-capable Clang and LLD; run with --ignored"]
fn synthetic_module_rejects_unknown_imports() -> Result<()> {
    let tools = tools()?;
    let directory = tempfile::tempdir()?;
    let source = directory.path().join("divide.c");
    fs::write(
        &source,
        "#include \"grkit.h\"\nu32 divide(u32 a, u32 b) { return a / b; }\nstatic const GrkitRegistration r[] = {{0, 0}};\nGRKIT_MODULE(r, 0, 0, 0, 0, 0, 0);\n",
    )?;
    let error = build_module(
        "DIVIDE",
        1,
        &[source],
        &directory.path().join("objects"),
        tools.clang.to_str().unwrap(),
        tools.lld.to_str().unwrap(),
    )
    .unwrap_err();
    assert!(error.to_string().contains("__aeabi_uidiv"), "{error}");
    Ok(())
}

/// The kernels of Glow that have NEON code (`modules/glow/glow_kernels.c`)
/// give the bytes of their C code on random inputs and lengths: the test
/// program `modules/glow/neon_parity.c` links the file built with NEON and
/// built with `GLOW_SCALAR`, and runs under qemu-arm (`GRKIT_QEMU_ARM`).
#[test]
#[ignore = "Requires ARM-capable Clang and LLD and qemu-arm; run with --ignored"]
fn synthetic_glow_neon_parity() -> Result<()> {
    const KERNELS: [&str; 7] = [
        "glow_clip",
        "glow_clip_total",
        "glow_rgb",
        "glow_light",
        "glow_ycc",
        "glow_luma",
        "glow_chroma",
    ];
    let tools = tools()?;
    let directory = tempfile::tempdir()?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let glow = root.join("modules/glow");
    let sdk = root.join("runtime/sdk");
    let compile = |source: &str, object: &str, mode: &str, extra: &[String]| -> Result<()> {
        toolchain::run(
            &tools.clang,
            toolchain::CAMERA_TARGET
                .iter()
                .map(|flag| flag.to_string())
                .chain(
                    [
                        mode,
                        "-O2",
                        "-ffreestanding",
                        "-fno-builtin",
                        "-fno-stack-protector",
                        "-c",
                    ]
                    .map(String::from),
                )
                .chain(extra.iter().cloned())
                .chain([
                    "-I".into(),
                    sdk.display().to_string(),
                    glow.join(source).display().to_string(),
                    "-o".into(),
                    directory.path().join(object).display().to_string(),
                ]),
        )
    };
    let scalar: Vec<String> = std::iter::once("-DGLOW_SCALAR".to_string())
        .chain(KERNELS.iter().map(|name| format!("-D{name}=scalar_{name}")))
        .collect();
    compile("glow_kernels.c", "neon.o", "-mthumb", &[])?;
    compile("glow_kernels.c", "scalar.o", "-mthumb", &scalar)?;
    // ARM mode: the entry point of the program.
    compile("neon_parity.c", "parity.o", "-marm", &[])?;
    let program = directory.path().join("parity");
    toolchain::run(
        &tools.lld,
        ["-static", "-e", "_start", "-o"]
            .map(String::from)
            .into_iter()
            .chain([program.display().to_string()])
            .chain(
                ["parity.o", "neon.o", "scalar.o"]
                    .map(|object| directory.path().join(object).display().to_string()),
            ),
    )?;
    let qemu = std::env::var("GRKIT_QEMU_ARM").unwrap_or_else(|_| "qemu-arm".into());
    let result = std::process::Command::new(&qemu)
        .arg(&program)
        .output()
        .with_context(|| format!("Run {qemu}"))?;
    let output = String::from_utf8_lossy(&result.stdout);
    ensure!(
        result.status.success() && output.contains("NEON == C"),
        "Glow kernels: {output}{}",
        String::from_utf8_lossy(&result.stderr)
    );
    Ok(())
}

/// The NEON code of Date Imprint (`modules/date/stamp.c`) gives the bytes of
/// its C code: the test program `modules/date/neon_parity.c` links the file
/// built with NEON and built with `DATE_SCALAR`, stamps the same random
/// planes with both, and runs under qemu-arm (`GRKIT_QEMU_ARM`).
#[test]
#[ignore = "Requires ARM-capable Clang and LLD and qemu-arm; run with --ignored"]
fn synthetic_date_neon_parity() -> Result<()> {
    let tools = tools()?;
    let directory = tempfile::tempdir()?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let date = root.join("modules/date");
    let sdk = root.join("runtime/sdk");
    // The setting handles of modules/date, as `grkit module build` declares them.
    fs::write(
        directory.path().join("grkit_module.h"),
        "#define SETTING_STYLE GRKIT_SETTING(0)\n#define SETTING_LOOK GRKIT_SETTING(1)\n\
         #define SETTING_POSITION GRKIT_SETTING(2)\n",
    )?;
    let compile = |source: &str, object: &str, mode: &str, extra: &[&str]| -> Result<()> {
        toolchain::run(
            &tools.clang,
            toolchain::CAMERA_TARGET
                .iter()
                .map(|flag| flag.to_string())
                .chain(
                    [
                        mode,
                        "-Os",
                        "-ffreestanding",
                        "-fno-builtin",
                        "-fno-stack-protector",
                        "-c",
                    ]
                    .map(String::from),
                )
                .chain(extra.iter().map(|flag| flag.to_string()))
                .chain([
                    "-I".into(),
                    sdk.display().to_string(),
                    "-I".into(),
                    directory.path().display().to_string(),
                    date.join(source).display().to_string(),
                    "-o".into(),
                    directory.path().join(object).display().to_string(),
                ]),
        )
    };
    compile("stamp.c", "neon.o", "-mthumb", &[])?;
    compile(
        "stamp.c",
        "scalar.o",
        "-mthumb",
        &["-DDATE_SCALAR", "-Ddate_apply=scalar_date_apply"],
    )?;
    // ARM mode: the entry point of the program.
    compile("neon_parity.c", "parity.o", "-marm", &[])?;
    let program = directory.path().join("parity");
    toolchain::run(
        &tools.lld,
        ["-static", "-e", "_start", "-o"]
            .map(String::from)
            .into_iter()
            .chain([program.display().to_string()])
            .chain(
                ["parity.o", "neon.o", "scalar.o"]
                    .map(|object| directory.path().join(object).display().to_string()),
            ),
    )?;
    let qemu = std::env::var("GRKIT_QEMU_ARM").unwrap_or_else(|_| "qemu-arm".into());
    let result = std::process::Command::new(&qemu)
        .arg(&program)
        .output()
        .with_context(|| format!("Run {qemu}"))?;
    let output = String::from_utf8_lossy(&result.stdout);
    ensure!(
        result.status.success() && output.contains("NEON == C"),
        "Date stamp: {output}{}",
        String::from_utf8_lossy(&result.stderr)
    );
    Ok(())
}

/// Defaults are optional, bounded by the setting range, and emitted into the SDK descriptor.
#[test]
fn module_setting_defaults_are_validated() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let folder = template_folder(directory.path(), "DEFAULT")?;
    let path = folder.join("module.toml");
    let original = fs::read_to_string(&path)?;
    let (manifest, _, _) = module::generate(&folder)?;
    assert!(manifest.settings.iter().all(|s| s.default == 0));
    let text = original.replace("[[settings]]", "[[settings]]\ndefault = 1");
    fs::write(&path, &text)?;
    let (manifest, code, _) = module::generate(&folder)?;
    assert!(manifest.settings.iter().all(|s| s.default == 1));
    assert!(code.contains(".default_value = 1"));
    fs::write(&path, text.replace("default = 1", "default = 65536"))?;
    assert!(module::generate(&folder)
        .unwrap_err()
        .to_string()
        .contains("default is outside"));
    Ok(())
}

#[test]
fn settings_page_has_independent_rows_and_one_tile() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let folder = template_folder(directory.path(), "SETTINGS")?;
    let manifest = r#"id = "SETTINGS"
version = 1
tiles = ["tiles/0-module.png"]
[[settings]]
key = "FIRST"
values = 4
[[settings]]
key = "SECOND"
values = 4
[page]
kind = "settings"
settings = ["FIRST", "SECOND"]
"#;
    fs::write(folder.join("module.toml"), manifest)?;
    for language in grkit_core::language::Language::ALL {
        fs::write(
            folder.join(format!("text/{}.toml", language.code())),
            r#"title = "Settings"
[settings.FIRST]
label = "First"
values = ["Off", "Weak", "Medium", "Strong"]
[settings.SECOND]
label = "Second"
values = ["Off", "Weak", "Medium", "Strong"]
"#,
        )?;
    }
    let (_, code, _) = module::generate(&folder)?;
    assert!(code.contains(".kind = GRKIT_PAGE_SETTINGS"));
    assert!(code.contains(".setting_rows = 2"));
    assert!(!code.contains("page_summaries"));
    for invalid in [
        manifest.replace(
            "kind = \"settings\"",
            "kind = \"settings\"\nsetting = \"FIRST\"",
        ),
        manifest.replace("settings = [\"FIRST\", \"SECOND\"]", "settings = []"),
        manifest.replace("tiles = [\"tiles/0-module.png\"]", "tiles = []"),
    ] {
        fs::write(folder.join("module.toml"), invalid)?;
        assert!(module::generate(&folder).is_err());
    }
    Ok(())
}

const GROUP_MANIFEST: &str = r#"id = "GROUP"
version = 1
tiles = ["tiles/0-module.png", "tiles/0-module.png", "tiles/0-module.png"]
[[settings]]
key = "FIRST_ENABLED"
values = 2
[[settings]]
key = "FIRST_STRENGTH"
values = 3
default = 1
[[settings]]
key = "SECOND_ENABLED"
values = 2
[[settings]]
key = "SECOND_STRENGTH"
values = 3
default = 1
[page]
kind = "group"
[[page.children]]
key = "FIRST"
settings = ["FIRST_ENABLED", "FIRST_STRENGTH"]
[[page.children]]
key = "SECOND"
settings = ["SECOND_ENABLED", "SECOND_STRENGTH"]
"#;

const GROUP_TEXT: &str = r#"title = "Group"
[children.FIRST]
title = "First"
[children.SECOND]
title = "Second"
[settings.FIRST_ENABLED]
label = "Enable"
values = ["Off", "On"]
[settings.FIRST_STRENGTH]
label = "Strength"
values = ["Weak", "Medium", "Strong"]
[settings.SECOND_ENABLED]
label = "Enable"
values = ["Off", "On"]
[settings.SECOND_STRENGTH]
label = "Strength"
values = ["Weak", "Medium", "Strong"]
"#;

fn group_folder(directory: &Path) -> Result<std::path::PathBuf> {
    let folder = template_folder(directory, "GROUP")?;
    fs::write(folder.join("module.toml"), GROUP_MANIFEST)?;
    for language in grkit_core::language::Language::ALL {
        fs::write(
            folder.join(format!("text/{}.toml", language.code())),
            GROUP_TEXT,
        )?;
    }
    Ok(folder)
}

#[test]
fn group_page_registers_ordered_children_with_independent_settings() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let folder = group_folder(directory.path())?;
    let (manifest, code, _) = module::generate(&folder)?;
    let page = manifest.page.unwrap();
    assert_eq!(page.kind, module::PageKind::Group);
    assert_eq!(page.children[0].key, "FIRST");
    assert_eq!(page.children[1].key, "SECOND");
    assert!(code.contains(".kind = GRKIT_PAGE_GROUP, .setting = GRKIT_NO_SETTING"));
    assert!(code.contains(".rows = 2, .children = page_children"));
    assert!(code.contains("page_children[] = {&page_child_0, &page_child_1}"));
    assert!(
        code.contains(".setting = 0, .flags = GRKIT_PAGE_MENU_ONLY, .title = child_0_page_title")
    );
    assert!(
        code.contains(".setting = 2, .flags = GRKIT_PAGE_MENU_ONLY, .title = child_1_page_title")
    );
    assert!(
        code.contains(".setting = 1, .label = child_0_row_1_label, .values = child_0_row_1_values")
    );
    assert!(
        code.contains(".setting = 3, .label = child_1_row_1_label, .values = child_1_row_1_values")
    );
    assert!(code.contains(".registration_count = 3"));
    let registrations = code
        .split("GrkitRegistration registrations[] = ")
        .nth(1)
        .unwrap();
    assert!(registrations.find("&page}").unwrap() < registrations.find("&page_child_0}").unwrap());
    assert!(
        registrations.find("&page_child_0}").unwrap()
            < registrations.find("&page_child_1}").unwrap()
    );
    assert!(!code.contains("page_labels") && !code.contains("page_summaries"));
    Ok(())
}

/// A color-only module gets the flag in its descriptor; others have none.
#[test]
fn color_only_modules_get_the_descriptor_flag() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let folder = group_folder(directory.path())?;
    let (_, code, _) = module::generate(&folder)?;
    assert!(!code.contains(".flags = GRKIT_MODULE_COLOR_ONLY"));
    fs::write(
        folder.join("module.toml"),
        GROUP_MANIFEST.replace("version = 1\n", "version = 1\ncolor_only = true\n"),
    )?;
    let (manifest, code, _) = module::generate(&folder)?;
    assert!(manifest.color_only);
    assert!(code.contains(".flags = GRKIT_MODULE_COLOR_ONLY}"));
    Ok(())
}

/// A color-only setting gets its flag. It may be a later row of a page, but
/// not the first row of a page or of a group child.
#[test]
fn color_only_settings_are_later_rows() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let folder = group_folder(directory.path())?;
    let path = folder.join("module.toml");
    let color = GROUP_MANIFEST.replace(
        "key = \"FIRST_STRENGTH\"\nvalues = 3",
        "key = \"FIRST_STRENGTH\"\nvalues = 3\nsaved = true\ncolor_only = true",
    );
    fs::write(&path, &color)?;
    let (_, code, _) = module::generate(&folder)?;
    assert!(code.contains(
        ".key = \"FIRST_STRENGTH\", .values = 3, .flags = GRKIT_SETTING_SAVED | GRKIT_SETTING_COLOR_ONLY"
    ));
    assert!(code.contains(".key = \"FIRST_ENABLED\", .values = 2, .flags = 0"));
    fs::write(
        &path,
        color.replace(
            "settings = [\"FIRST_ENABLED\", \"FIRST_STRENGTH\"]",
            "settings = [\"FIRST_STRENGTH\", \"FIRST_ENABLED\"]",
        ),
    )?;
    assert!(module::generate(&folder)
        .unwrap_err()
        .to_string()
        .contains("cannot be the first row"));
    Ok(())
}

const INDICATOR: &str = r#"
[[indicators]]
id = "first"
placement = "status_row"
enabled = { setting = "FIRST_ENABLED", equals = 1 }
icon = { setting = "FIRST_STRENGTH" }
"#;

#[test]
fn layers_share_enabled_conditions_and_allow_unconditional_declarations() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let folder = group_folder(directory.path())?;
    let layers = r#"
[[layers]]
paint = "draw_guides"
setting = "FIRST_STRENGTH"
order = 20
follow_disp = true
enabled = { all = [{ setting = "FIRST_ENABLED", equals = 0 }, { setting = "SECOND_ENABLED", not_equals = 1 }] }
[[layers]]
paint = "draw_mask"
order = 10
"#;
    fs::write(
        folder.join("module.toml"),
        format!("{GROUP_MANIFEST}{layers}"),
    )?;
    let (_, code, _) = module::generate(&folder)?;
    assert!(code.contains(".setting = 0, .comparison = GRKIT_CONDITION_EQUALS, .value = 0"));
    assert!(code.contains(".setting = 2, .comparison = GRKIT_CONDITION_NOT_EQUALS, .value = 1"));
    assert!(code.contains(".condition_count = 2, .conditions = layer_0_conditions"));
    assert!(code.contains(".condition_count = 0, .conditions = 0"));
    Ok(())
}

#[test]
fn layers_reject_invalid_enabled_conditions() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let folder = group_folder(directory.path())?;
    let invalid = [
        "{ all = [] }".to_owned(),
        "{ setting = \"MISSING\", equals = 0 }".to_owned(),
        "{ setting = \"FIRST_ENABLED\", equals = 2 }".to_owned(),
        "{ setting = \"FIRST_ENABLED\" }".to_owned(),
        "{ setting = \"FIRST_ENABLED\", equals = 0, not_equals = 1 }".to_owned(),
        format!(
            "{{ all = [{}] }}",
            ["{ setting = \"FIRST_ENABLED\", equals = 1 }"; 5].join(",")
        ),
    ];
    for enabled in invalid {
        fs::write(
            folder.join("module.toml"),
            format!(
                "{GROUP_MANIFEST}\n[[layers]]\npaint = \"draw\"\norder = 0\nenabled = {enabled}\n"
            ),
        )?;
        assert!(module::generate(&folder).is_err(), "accepted {enabled}");
    }
    Ok(())
}

#[test]
fn indicators_and_adj_children_share_strength_icons_without_using_strength_as_enabled() -> Result<()>
{
    let directory = tempfile::tempdir()?;
    let folder = group_folder(directory.path())?;
    let manifest = GROUP_MANIFEST.replace(
        "key = \"FIRST\"\nsettings =",
        "key = \"FIRST\"\nicon = { setting = \"FIRST_STRENGTH\" }\nsettings =",
    );
    fs::write(folder.join("module.toml"), format!("{manifest}{INDICATOR}"))?;
    let (_, code, _) = module::generate(&folder)?;
    assert!(code.contains("GrkitIcon child_0_icon"));
    assert!(code.contains(".icon = &child_0_icon"));
    assert!(code.contains(".kind = GRKIT_ICON_TILE, .setting = 1, .first_tile = 0"));
    assert!(code.contains(".setting = 0, .comparison = GRKIT_CONDITION_EQUALS, .value = 1"));
    assert!(code.contains(".placement = GRKIT_INDICATOR_STATUS_ROW, .slot = 0"));
    assert!(code.contains(".point = GRKIT_POINT_VIEWFINDER_INDICATOR"));
    assert!(code.contains(".registration_count = 4"));
    Ok(())
}

#[test]
fn indicators_accept_fixed_tiles_combined_conditions_and_native_tone() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let folder = group_folder(directory.path())?;
    let manifest = GROUP_MANIFEST.replace("version = 1", "version = 1\nuses = [\"CORE.TONE\"]");
    let indicators = r#"
[[indicators]]
id = "combined"
placement = "status_row"
enabled = { all = [{ setting = "FIRST_ENABLED", not_equals = 0 }, { setting = "SECOND_ENABLED", equals = 0 }] }
icon = { tile = 2 }
[[indicators]]
id = "tone"
placement = "native_slot"
slot = "image_control"
enabled = { setting = "CORE.TONE", not_equals = 0 }
icon = { setting = "CORE.TONE" }
"#;
    fs::write(
        folder.join("module.toml"),
        format!("{manifest}{indicators}"),
    )?;
    let (_, code, _) = module::generate(&folder)?;
    assert!(code.contains(".condition_count = 2"));
    assert!(code.contains(".kind = GRKIT_ICON_TILE, .setting = GRKIT_NO_SETTING, .first_tile = 2"));
    assert!(code.contains(".kind = GRKIT_ICON_TONE, .setting = GRKIT_USE(0), .first_tile = 0"));
    assert!(
        code.contains(".placement = GRKIT_INDICATOR_NATIVE_SLOT, .slot = GRKIT_SLOT_IMAGE_CONTROL")
    );
    Ok(())
}

#[test]
fn status_row_indicators_show_core_labels() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let folder = group_folder(directory.path())?;
    let manifest = GROUP_MANIFEST.replace("version = 1", "version = 1\nuses = [\"CORE.ASPECT\"]");
    let indicators = r#"
[[indicators]]
id = "date"
placement = "status_row"
enabled = { setting = "FIRST_ENABLED", equals = 1 }
icon = { label = "DATE" }
[[indicators]]
id = "ratio"
placement = "status_row"
enabled = { setting = "CORE.ASPECT", not_equals = 0 }
icon = { setting = "CORE.ASPECT", label = "ratio" }
"#;
    fs::write(
        folder.join("module.toml"),
        format!("{manifest}{indicators}"),
    )?;
    let (_, code, _) = module::generate(&folder)?;
    assert!(code.contains(".kind = GRKIT_ICON_LABEL, .setting = GRKIT_NO_SETTING, .first_tile = 8"));
    assert!(code.contains(".kind = GRKIT_ICON_LABEL, .setting = GRKIT_USE(0), .first_tile = 0"));
    // With a setting of the module, value n shows the nth label after the named one.
    let style = format!(
        "{manifest}\n[[indicators]]\nid = \"strength\"\nplacement = \"status_row\"\nenabled = {{ setting = \"FIRST_ENABLED\", equals = 1 }}\nicon = {{ setting = \"FIRST_STRENGTH\", label = \"DATE\" }}\n"
    );
    fs::write(folder.join("module.toml"), style)?;
    let (_, code, _) = module::generate(&folder)?;
    assert!(code.contains(".kind = GRKIT_ICON_LABEL, .setting = 1, .first_tile = 8"));
    let invalid = [
        // Not a core label, the load cue, a tile offset, or another setting.
        "icon = { label = \"XYZ\" }",
        "icon = { label = \"GRKit Loaded\" }",
        "icon = { label = \"DATE\", first_tile = 1 }",
        "icon = { setting = \"FIRST_STRENGTH\", label = \"ratio\" }",
        "icon = { setting = \"CORE.ASPECT\", label = \"DATE\" }",
        // Three values from the last label run past the labels.
        "icon = { setting = \"FIRST_STRENGTH\", label = \"DMY HM\" }",
    ];
    for icon in invalid {
        let manifest = format!(
            "{manifest}\n[[indicators]]\nid = \"bad\"\nplacement = \"status_row\"\nenabled = {{ setting = \"FIRST_ENABLED\", equals = 1 }}\n{icon}\n"
        );
        fs::write(folder.join("module.toml"), manifest)?;
        assert!(module::generate(&folder).is_err(), "accepted {icon}");
    }
    // Only the status row draws a label: not the Image Control slot or ADJ.
    let slot = format!(
        "{manifest}\n[[indicators]]\nid = \"slot\"\nplacement = \"native_slot\"\nslot = \"image_control\"\nenabled = {{ setting = \"FIRST_ENABLED\", equals = 1 }}\nicon = {{ label = \"DATE\" }}\n"
    );
    fs::write(folder.join("module.toml"), slot)?;
    assert!(module::generate(&folder).is_err());
    let child = manifest.replace(
        "key = \"FIRST\"\nsettings =",
        "key = \"FIRST\"\nicon = { label = \"DATE\" }\nsettings =",
    );
    fs::write(folder.join("module.toml"), child)?;
    assert!(module::generate(&folder).is_err());
    Ok(())
}

#[test]
fn indicators_reject_invalid_conditions_icons_and_placements() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let folder = group_folder(directory.path())?;
    let invalid = [
        INDICATOR.replace("id = \"first\"", "id = \"\""),
        INDICATOR.replace("status_row", "append"),
        INDICATOR.replace("status_row", "native_slot"),
        INDICATOR.replace(
            "placement = \"status_row\"",
            "placement = \"status_row\"\nslot = \"image_control\"",
        ),
        INDICATOR.replace(
            "placement = \"status_row\"",
            "placement = \"native_slot\"\nslot = \"unknown\"",
        ),
        INDICATOR.replace("equals = 1", "equals = 2"),
        INDICATOR.replace("equals = 1", "equals = 1, not_equals = 0"),
        INDICATOR.replace(", equals = 1", ""),
        INDICATOR.replace("FIRST_ENABLED", "MISSING"),
        INDICATOR.replace(
            "{ setting = \"FIRST_ENABLED\", equals = 1 }",
            "{ all = [] }",
        ),
        INDICATOR.replace(
            "{ setting = \"FIRST_ENABLED\", equals = 1 }",
            &format!(
                "{{ all = [{}] }}",
                ["{ setting = \"FIRST_ENABLED\", equals = 1 }"; 5].join(",")
            ),
        ),
        INDICATOR.replace(
            "icon = { setting = \"FIRST_STRENGTH\" }",
            "icon = { tile = 3 }",
        ),
        INDICATOR.replace(
            "icon = { setting = \"FIRST_STRENGTH\" }",
            "icon = { tile = 1, first_tile = 0 }",
        ),
        INDICATOR.replace(
            "icon = { setting = \"FIRST_STRENGTH\" }",
            "icon = { setting = \"FIRST_STRENGTH\", first_tile = 1 }",
        ),
        INDICATOR.replace(
            "icon = { setting = \"FIRST_STRENGTH\" }",
            "icon = { setting = \"FIRST_STRENGTH\", first_tile = 4294967295 }",
        ),
        INDICATOR.replace(
            "icon = { setting = \"FIRST_STRENGTH\" }",
            "icon = { setting = \"FIRST_STRENGTH\", tile = 0 }",
        ),
        INDICATOR.replace(
            "icon = { setting = \"FIRST_STRENGTH\" }",
            "icon = { setting = \"MISSING\" }",
        ),
        format!("{INDICATOR}{INDICATOR}"),
        (0..7)
            .map(|i| INDICATOR.replace("id = \"first\"", &format!("id = \"item{i}\"")))
            .collect(),
    ];
    for indicators in invalid {
        fs::write(
            folder.join("module.toml"),
            format!("{GROUP_MANIFEST}{indicators}"),
        )?;
        assert!(module::generate(&folder).is_err(), "accepted {indicators}");
    }
    let native = INDICATOR.replace(
        "placement = \"status_row\"",
        "placement = \"native_slot\"\nslot = \"image_control\"",
    );
    fs::write(
        folder.join("module.toml"),
        format!(
            "{GROUP_MANIFEST}{native}{}",
            native.replace("id = \"first\"", "id = \"second\"")
        ),
    )?;
    assert!(module::generate(&folder)
        .unwrap_err()
        .to_string()
        .contains("image_control"));

    let invalid_child = GROUP_MANIFEST.replace(
        "key = \"FIRST\"\nsettings =",
        "key = \"FIRST\"\nicon = { setting = \"FIRST_STRENGTH\", first_tile = 1 }\nsettings =",
    );
    fs::write(folder.join("module.toml"), invalid_child)?;
    assert!(module::generate(&folder)
        .unwrap_err()
        .to_string()
        .contains("needs 3 tiles"));
    Ok(())
}

#[test]
fn indicator_only_module_needs_no_page_or_text_files() -> Result<()> {
    let directory = tempfile::tempdir()?;
    fs::write(
        directory.path().join("module.toml"),
        r#"id = "TONE"
version = 1
uses = ["CORE.TONE"]
[[indicators]]
id = "tone"
placement = "native_slot"
slot = "image_control"
enabled = { setting = "CORE.TONE", not_equals = 0 }
icon = { setting = "CORE.TONE" }
"#,
    )?;
    let (_, code, _) = module::generate(directory.path())?;
    assert!(code.contains(".registration_count = 1"));
    Ok(())
}

#[test]
fn group_page_rejects_invalid_structure_and_text() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let folder = group_folder(directory.path())?;
    let manifest_path = folder.join("module.toml");
    let invalid_manifests = [
        GROUP_MANIFEST.split("[[page.children]]").next().unwrap().to_owned(),
        GROUP_MANIFEST.replace("kind = \"group\"", "kind = \"settings\""),
        GROUP_MANIFEST.replace("kind = \"group\"", "kind = \"group\"\nsetting = \"FIRST_ENABLED\""),
        GROUP_MANIFEST.replace("kind = \"group\"", "kind = \"group\"\nsettings = [\"FIRST_ENABLED\"]"),
        GROUP_MANIFEST.replace("kind = \"group\"", "kind = \"group\"\noptions = true"),
        GROUP_MANIFEST.replace("key = \"SECOND\"", "key = \"FIRST\""),
        GROUP_MANIFEST.replace("values = 2", "values = 3"),
        GROUP_MANIFEST.replace("key = \"FIRST\"", "key = \"first\""),
        GROUP_MANIFEST.replace("settings = [\"FIRST_ENABLED\", \"FIRST_STRENGTH\"]", "settings = []"),
        GROUP_MANIFEST.replace("settings = [\"FIRST_ENABLED\", \"FIRST_STRENGTH\"]", "settings = [\"MISSING\"]"),
        GROUP_MANIFEST.replace("settings = [\"FIRST_ENABLED\", \"FIRST_STRENGTH\"]", "settings = [\"FIRST_ENABLED\", \"FIRST_ENABLED\", \"FIRST_ENABLED\", \"FIRST_ENABLED\", \"FIRST_ENABLED\", \"FIRST_ENABLED\"]"),
        GROUP_MANIFEST.replace("tiles = [\"tiles/0-module.png\", \"tiles/0-module.png\", \"tiles/0-module.png\"]", "tiles = [\"tiles/0-module.png\", \"tiles/0-module.png\"]"),
        GROUP_MANIFEST.replace("version = 1", "version = 1\nuses = [\"?OPTION.ENABLED\"]").replace("settings = [\"FIRST_ENABLED\", \"FIRST_STRENGTH\"]", "settings = [\"OPTION.ENABLED\"]"),
        format!("{GROUP_MANIFEST}{}", (0..3).map(|i| format!("[[page.children]]\nkey = \"EXTRA{i}\"\nsettings = [\"FIRST_ENABLED\"]\n")).collect::<String>()),
    ];
    for manifest in invalid_manifests {
        fs::write(&manifest_path, &manifest)?;
        assert!(module::generate(&folder).is_err(), "accepted {manifest}");
    }
    fs::write(&manifest_path, GROUP_MANIFEST)?;
    let text_path = folder.join("text/en.toml");
    for text in [
        GROUP_TEXT.replace("children.FIRST", "children.MISSING"),
        GROUP_TEXT.replace("settings.FIRST_ENABLED", "settings.MISSING"),
        GROUP_TEXT.replace(
            "title = \"Group\"",
            "title = \"Group\"\nrows = [\"First\", \"Second\"]",
        ),
        GROUP_TEXT.replace("values = [\"Off\", \"On\"]", "values = [\"Off\"]"),
        GROUP_TEXT.replace(
            "title = \"First\"",
            &format!("title = \"{}\"", "A".repeat(65)),
        ),
        format!("{GROUP_TEXT}\n[children.EXTRA]\ntitle = \"Extra\"\n"),
    ] {
        fs::write(&text_path, &text)?;
        assert!(module::generate(&folder).is_err(), "accepted {text}");
    }
    fs::write(&text_path, GROUP_TEXT)?;
    fs::write(
        folder.join("text/ko.toml"),
        GROUP_TEXT.replace("title = \"First\"", "title = \"똠\""),
    )?;
    assert!(module::generate(&folder)
        .unwrap_err()
        .to_string()
        .contains("camera cannot draw"));
    Ok(())
}

/// A copy of the Guides module folder, whose page has an `fn_cycle`.
fn guides_folder(directory: &Path) -> Result<std::path::PathBuf> {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../modules/guides");
    let folder = directory.join("guides");
    for sub in ["text", "tiles"] {
        fs::create_dir_all(folder.join(sub))?;
        for entry in fs::read_dir(source.join(sub))? {
            let entry = entry?;
            fs::copy(entry.path(), folder.join(sub).join(entry.file_name()))?;
        }
    }
    for file in ["module.toml", "guides.c", "test.c"] {
        fs::copy(source.join(file), folder.join(file))?;
    }
    Ok(folder)
}

/// Fn on the row of one page value steps another setting (ABI 2.5). The
/// value is a row other than Off, and the setting is another one with at
/// least two values.
#[test]
fn fn_cycle_names_a_page_value_and_another_setting() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let folder = guides_folder(directory.path())?;
    let path = folder.join("module.toml");
    let original = fs::read_to_string(&path)?;
    let (_, code, _) = module::generate(&folder)?;
    assert!(code.contains(".fn_value = 6, .fn_setting = 4}"));
    let line = "fn_cycle = { value = 6, setting = \"SPIRAL\" }";
    for (replacement, error) in [
        (
            "fn_cycle = { value = 0, setting = \"SPIRAL\" }",
            "other than Off",
        ),
        (
            "fn_cycle = { value = 8, setting = \"SPIRAL\" }",
            "other than Off",
        ),
        (
            "fn_cycle = { value = 6, setting = \"STYLE\" }",
            "other than the page setting",
        ),
        (
            "fn_cycle = { value = 6, setting = \"NONE\" }",
            "has no setting NONE",
        ),
    ] {
        fs::write(&path, original.replace(line, replacement))?;
        let message = module::generate(&folder).unwrap_err().to_string();
        assert!(message.contains(error), "{message}");
    }
    fs::write(
        &path,
        original.replace(
            "key = \"SPIRAL\"\nvalues = 4\ndefault = 2",
            "key = \"SPIRAL\"\nvalues = 1",
        ),
    )?;
    assert!(module::generate(&folder)
        .unwrap_err()
        .to_string()
        .contains("at least 2 values"));
    Ok(())
}
