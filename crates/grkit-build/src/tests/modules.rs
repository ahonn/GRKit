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
    assert_eq!(word(module.descriptor), grkit_core::grm::DESCRIPTOR_SIZE);
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
