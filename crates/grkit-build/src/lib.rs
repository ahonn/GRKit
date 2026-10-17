//! Native firmware build adapters. The core crate remains independent of processes and files.
pub mod arm;
pub mod components;
pub mod firmware;
pub mod loader;
pub mod target;
pub mod toolchain;

use anyhow::{ensure, Context, Result};
use firmware::sha256;
pub use firmware::verify_firmware;
use grkit_core::{
    runtime::{icon_stride, runtime_files_for},
    Files,
};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};
pub use toolchain::Toolchain;

#[derive(Debug, Clone)]
pub struct Patch {
    pub address: u32,
    pub before: Vec<u8>,
    pub after: Vec<u8>,
}
#[derive(Debug)]
pub struct PatchPlan {
    pub patches: Vec<Patch>,
    pub relocations: Vec<(usize, usize)>,
}
#[derive(Debug)]
pub struct BuildResult {
    pub output: PathBuf,
    pub files: Files,
    pub manifest: Value,
}

pub fn validate_plan(plan: &PatchPlan) -> Result<()> {
    let mut regions = Vec::new();
    for patch in &plan.patches {
        ensure!(
            !patch.before.is_empty()
                && patch.before.len() == patch.after.len()
                && patch.address % 4 == 0,
            "Invalid patch dimensions"
        );
        let start = u64::from(patch.address);
        let end = start + patch.after.len() as u64;
        ensure!(end <= u64::from(u32::MAX) + 1, "Patch address overflow");
        ensure!(
            !regions.iter().any(|&(s, e)| start < e && s < end),
            "Overlapping patches"
        );
        regions.push((start, end));
    }
    let mut seen = BTreeSet::new();
    for &(index, offset) in &plan.relocations {
        ensure!(
            seen.insert((index, offset)) && offset % 4 == 0,
            "Invalid relocation"
        );
        let patch = plan.patches.get(index).context("Invalid relocation")?;
        let end = offset.checked_add(4).context("Invalid relocation")?;
        ensure!(
            patch.after.get(offset..end) == Some(target::HEADER_REFERENCE.to_le_bytes().as_slice()),
            "Relocation does not identify the private header"
        );
    }
    Ok(())
}

pub const RUNTIME_SOURCES: &[(&str, &[u8])] = &[
    ("abi.h", include_bytes!("../../../runtime/abi.h")),
    ("firmware.h", include_bytes!("../../../runtime/firmware.h")),
    (
        "targets/gr3-2.10.h",
        include_bytes!("../../../runtime/targets/gr3-2.10.h"),
    ),
    (
        "targets/gr4-1.11.h",
        include_bytes!("../../../runtime/targets/gr4-1.11.h"),
    ),
    ("catalog.h", include_bytes!("../../../runtime/catalog.h")),
    (
        "visibility.h",
        include_bytes!("../../../runtime/visibility.h"),
    ),
    ("grf.c", include_bytes!("../../../runtime/grf.c")),
    ("grf.h", include_bytes!("../../../runtime/grf.h")),
    (
        "installer.c",
        include_bytes!("../../../runtime/installer.c"),
    ),
    ("menu.S", include_bytes!("../../../runtime/menu.S")),
    ("menu.c", include_bytes!("../../../runtime/menu.c")),
    ("modules.h", include_bytes!("../../../runtime/modules.h")),
    ("framing.c", include_bytes!("../../../runtime/framing.c")),
    ("framing.S", include_bytes!("../../../runtime/framing.S")),
    ("picture.c", include_bytes!("../../../runtime/picture.c")),
    ("picture.S", include_bytes!("../../../runtime/picture.S")),
    (
        "menu_labels.h",
        include_bytes!("../../../runtime/menu_labels.h"),
    ),
    (
        "processing.S",
        include_bytes!("../../../runtime/processing.S"),
    ),
    (
        "targets/gr4-1.11-processing.S",
        include_bytes!("../../../runtime/targets/gr4-1.11-processing.S"),
    ),
];

pub fn source_hashes() -> BTreeMap<String, String> {
    let mut sources: BTreeMap<_, _> = RUNTIME_SOURCES
        .iter()
        .map(|(name, bytes)| (format!("runtime/{name}"), sha256(bytes)))
        .collect();
    for (name, bytes) in [
        ("lib.rs", include_bytes!("lib.rs").as_slice()),
        ("arm.rs", include_bytes!("arm.rs").as_slice()),
        ("firmware.rs", include_bytes!("firmware.rs").as_slice()),
        ("target.rs", include_bytes!("target.rs").as_slice()),
        ("toolchain.rs", include_bytes!("toolchain.rs").as_slice()),
        ("components.rs", include_bytes!("components.rs").as_slice()),
        ("loader.rs", include_bytes!("loader.rs").as_slice()),
    ] {
        sources.insert(format!("crates/grkit-build/src/{name}"), sha256(bytes));
    }
    for (name, bytes) in [
        (
            "artwork.rs",
            include_bytes!("../../grkit-core/src/artwork.rs").as_slice(),
        ),
        (
            "runtime.rs",
            include_bytes!("../../grkit-core/src/runtime.rs").as_slice(),
        ),
    ] {
        sources.insert(format!("crates/grkit-core/src/{name}"), sha256(bytes));
    }
    for (name, bytes) in [
        (
            "Cargo.toml",
            include_bytes!("../../../Cargo.toml").as_slice(),
        ),
        (
            "Cargo.lock",
            include_bytes!("../../../Cargo.lock").as_slice(),
        ),
        (
            "crates/grkit-build/Cargo.toml",
            include_bytes!("../Cargo.toml").as_slice(),
        ),
        (
            "crates/grkit-core/Cargo.toml",
            include_bytes!("../../grkit-core/Cargo.toml").as_slice(),
        ),
        (
            "crates/grkit-core/src/lib.rs",
            include_bytes!("../../grkit-core/src/lib.rs").as_slice(),
        ),
    ] {
        sources.insert(name.to_owned(), sha256(bytes));
    }
    sources
}

pub fn materialize_runtime(directory: &Path) -> Result<()> {
    fs::create_dir_all(directory.join("targets"))?;
    for (name, bytes) in RUNTIME_SOURCES {
        fs::write(directory.join(name), bytes)?;
    }
    Ok(())
}

pub fn build_runtime(
    target: &'static target::Target,
    firmware: &Path,
    output: &Path,
    clang: &str,
    lld: &str,
) -> Result<BuildResult> {
    let image = verify_firmware(firmware, target)?;
    let tools = Toolchain::discover(clang, lld)?;
    build_image(&image, output, &tools)
}

/// Internal build entry; only the public firmware entry verifies release identity.
fn build_image(
    image: &firmware::FirmwareImage,
    output: &Path,
    tools: &Toolchain,
) -> Result<BuildResult> {
    if let Some(parent) = output.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    fs::create_dir(output)
        .with_context(|| format!("Create new build directory {}", output.display()))?;
    let output = output.canonicalize()?;
    let objects = output.join("objects");
    let runtime = objects.join("runtime");
    materialize_runtime(&runtime)?;
    fs::write(runtime.join("hooks.h"), components::hooks_header(image)?)?;
    // A target without a layout gets the loader and the installer only.
    let mut built: Vec<(&str, components::Component)> = Vec::new();
    if let Some(layout) = image.target.layout {
        if let Some(region) = layout.processing {
            let component =
                components::processing(&objects, &runtime, image, tools, layout, region)?;
            built.push(("processing", component));
        }
        let component = components::menu(&objects, &runtime, image, tools, layout)?;
        built.push(("menu", component));
        if let Some(region) = layout.framing {
            let component = components::framing(&objects, &runtime, image, tools, layout, region)?;
            built.push(("framing", component));
        }
    }
    let components: Vec<_> = built.iter().map(|(_, component)| component).collect();
    // The installer publishes every code payload before any firmware change.
    let mut patches: Vec<_> = components.iter().map(|c| c.patches[0].clone()).collect();
    for component in &components {
        patches.extend_from_slice(&component.patches[1..]);
    }
    let relocations = components
        .iter()
        .enumerate()
        .flat_map(|(index, c)| c.relocations.iter().map(move |offset| (index, *offset)))
        .collect();
    let plan = PatchPlan {
        patches,
        relocations,
    };
    validate_plan(&plan)?;
    let generated = loader::header(&plan, image)?;
    fs::write(objects.join("module_payload.h"), &generated)?;
    let source = runtime.join("installer.c");
    let object = objects.join("module.o");
    let binary = objects.join("INSTALL.BIN");
    use components::path;
    toolchain::run(
        &tools.clang,
        [
            "--target=armv7-linux-gnueabi",
            "-march=armv7-a",
            "-marm",
            "-O2",
            "-ffreestanding",
            "-fno-builtin",
            "-fno-stack-protector",
            "-fomit-frame-pointer",
            "-fno-pic",
            "-I",
            path(&objects)?,
            "-c",
            path(&source)?,
            "-o",
            path(&object)?,
        ],
    )?;
    toolchain::run(
        &tools.lld,
        [
            "-m",
            "armelf_linux_eabi",
            "-static",
            "-e",
            "_start",
            "--build-id=none",
            "-Ttext=0x10000",
            "-o",
            path(&binary)?,
            path(&object)?,
        ],
    )?;
    let files = runtime_files_for(&fs::read(binary)?, icon_stride(image.target.name)?);
    ensure!(
        !files
            .values()
            .any(|bytes| bytes.windows(8).any(|part| part == b"--select")),
        "Production runtime contains diagnostic selection"
    );
    for (path, bytes) in &files {
        let destination = output.join("sd").join(path);
        fs::create_dir_all(destination.parent().unwrap())?;
        fs::write(destination, bytes)?;
    }
    let manifest = json!({
        "schema_version": 1, "target": image.target.name, "profile": "production", "startup_activation": "Off",
        "toolchain": tools.manifest(), "source_revision": null, "source_dirty": null,
        "firmware_sha256": image.sha256, "decoded_firmware_sha256": image.decoded_sha256,
        "source_sha256": source_hashes(), "generated_header_sha256": sha256(generated.as_bytes()),
        "components": built.iter().map(|(name, component)| (name.to_string(),
            json!({"sha256": sha256(&component.patches[0].after), "size": component.code_size})))
            .collect::<serde_json::Map<_, _>>(),
        "files": files.iter().map(|(path, bytes)| json!({"path": path, "size": bytes.len(), "sha256": sha256(bytes)})).collect::<Vec<_>>()
    });
    fs::write(
        output.join("build.json"),
        format!("{}\n", serde_json::to_string_pretty(&manifest)?),
    )?;
    Ok(BuildResult {
        output,
        files,
        manifest,
    })
}

#[cfg(test)]
mod tests;
