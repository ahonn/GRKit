//! Native firmware build adapters. The core crate remains independent of processes and files.
pub mod arm;
pub mod components;
pub mod dump;
pub mod fastboot;
pub mod firmware;
pub mod loader;
pub mod module;
pub mod port;
pub mod relocation;
pub mod target;
pub mod toolchain;

use anyhow::{ensure, Context, Result};
use firmware::sha256;
pub use firmware::verify_firmware;
use grkit_core::{
    runtime::{icon_stride, runtime_files_with},
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
    /// The extension points (`components::Point::bit`) that need the patch,
    /// or 0 for code that each installation publishes.
    pub points: u32,
    pub before: Vec<u8>,
    pub after: Vec<u8>,
    /// What the address moves with when the installer relocates the private
    /// code. `None` keeps it.
    pub site: Option<Anchor>,
}

/// An address that moves when the installer relocates the private code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Anchor {
    /// A firmware address of the pinned image.
    Firmware(u32),
    /// The private tail.
    Tail,
}

/// A word of a patch that the installer changes when it relocates the
/// private code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reference {
    pub patch: usize,
    pub offset: usize,
    /// A B or BL whose target is the anchor, or else a word that holds an
    /// address of the anchor.
    pub branch: bool,
    pub anchor: Anchor,
}

#[derive(Debug)]
pub struct PatchPlan {
    pub patches: Vec<Patch>,
    pub relocations: Vec<(usize, usize)>,
    /// The firmware and tail references of the private code.
    pub references: Vec<Reference>,
    /// The services that modules import: the hash of each name
    /// (`grkit_core::grm::symbol_hash`) and the offset of its code in the tail.
    pub exports: Vec<(u32, u32)>,
}
impl PatchPlan {
    /// The aligned span of the core payloads. Modules start immediately after it.
    pub(crate) fn core_size(&self, tail: u32) -> Result<u32> {
        ensure!(
            tail < components::RTOS_EXECUTABLE_END,
            "Private tail is outside the executable mapping"
        );
        let mut end = tail;
        for patch in &self.patches {
            if patch.site != Some(Anchor::Tail) {
                ensure!(
                    u64::from(patch.address) >= u64::from(components::RTOS_EXECUTABLE_END)
                        || u64::from(patch.address) + patch.after.len() as u64 <= u64::from(tail),
                    "Private payload has no tail anchor"
                );
                continue;
            }
            ensure!(patch.address >= tail, "Private code precedes the tail");
            let size = u32::try_from(patch.after.len()).context("Private payload is too large")?;
            let patch_end = patch
                .address
                .checked_add(size)
                .context("Private payload end overflows")?;
            ensure!(
                patch_end <= components::RTOS_EXECUTABLE_END,
                "Private payload exceeds the executable mapping"
            );
            end = end.max(patch_end);
        }
        if end == tail {
            return Ok(0);
        }
        Ok(components::aligned_code_end(end)? - tail)
    }
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

// Every file under runtime/, and the Rust sources and assets of a build, from
// build.rs.
include!(concat!(env!("OUT_DIR"), "/sources.rs"));

/// A file under `runtime/` by its path there, such as `targets/gr4.h`.
pub fn runtime_source(path: &str) -> Option<&'static [u8]> {
    RUNTIME_SOURCES
        .iter()
        .find(|(name, _)| *name == path)
        .map(|&(_, bytes)| bytes)
}

/// The SHA-256 of each file that can change a runtime build, by its path in
/// the repository.
pub fn source_hashes() -> BTreeMap<String, String> {
    RUNTIME_SOURCES
        .iter()
        .map(|(name, bytes)| (format!("runtime/{name}"), sha256(bytes)))
        .chain(
            BUILD_SOURCES
                .iter()
                .map(|(name, bytes)| (name.to_string(), sha256(bytes))),
        )
        .collect()
}

pub fn materialize_runtime(directory: &Path) -> Result<()> {
    for (name, bytes) in RUNTIME_SOURCES {
        let path = directory.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, bytes)?;
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

/// Build the module `id` from C `sources` into a module file (`grkit_core::grm`).
/// `directory` must not exist; it receives the objects.
pub fn build_module(
    id: &str,
    version: u32,
    sources: &[PathBuf],
    directory: &Path,
    clang: &str,
    lld: &str,
) -> Result<Vec<u8>> {
    let tools = Toolchain::discover(clang, lld)?;
    fs::create_dir_all(directory.parent().unwrap_or(Path::new(".")))?;
    fs::create_dir(directory)
        .with_context(|| format!("Create new build directory {}", directory.display()))?;
    let directory = directory.canonicalize().map(crate::plain_path)?;
    let runtime = directory.join("runtime");
    materialize_runtime(&runtime)?;
    let sources: Vec<PathBuf> = sources
        .iter()
        .map(|source| {
            source
                .canonicalize()
                .map(crate::plain_path)
                .with_context(|| format!("Read {}", source.display()))
        })
        .collect::<Result<_>>()?;
    let sources: Vec<&Path> = sources.iter().map(PathBuf::as_path).collect();
    module::build(
        id,
        version,
        &sources,
        &runtime.join("sdk"),
        &directory,
        &tools,
    )?
    .encode()
}

/// Build the module of `module.toml` in `module_directory` into a module file
/// (`grkit_core::grm`). `directory` must not exist; it receives the generated
/// files and the objects. Returns the module ID and the file.
pub fn build_module_manifest(
    module_directory: &Path,
    directory: &Path,
    clang: &str,
    lld: &str,
) -> Result<(String, Vec<u8>)> {
    let tools = Toolchain::discover(clang, lld)?;
    fs::create_dir_all(directory.parent().unwrap_or(Path::new(".")))?;
    fs::create_dir(directory)
        .with_context(|| format!("Create new build directory {}", directory.display()))?;
    let directory = directory.canonicalize().map(crate::plain_path)?;
    let runtime = directory.join("runtime");
    materialize_runtime(&runtime)?;
    let module = module::build_manifest(
        &module_directory
            .canonicalize()
            .map(crate::plain_path)
            .with_context(|| format!("Read {}", module_directory.display()))?,
        &runtime.join("sdk"),
        &directory,
        &tools,
    )?;
    Ok((module.id.clone(), module.encode()?))
}

/// Run the host test of the module in `module_directory` (`test.c`) with the
/// host compiler `cc`. `directory` must not exist; it receives the build.
/// Return whether a test ran (`module::test_manifest`).
pub fn test_module_manifest(module_directory: &Path, directory: &Path, cc: &str) -> Result<bool> {
    fs::create_dir_all(directory.parent().unwrap_or(Path::new(".")))?;
    fs::create_dir(directory)
        .with_context(|| format!("Create new build directory {}", directory.display()))?;
    let directory = directory.canonicalize().map(crate::plain_path)?;
    let runtime = directory.join("runtime");
    materialize_runtime(&runtime)?;
    module::test_manifest(
        &module_directory
            .canonicalize()
            .map(crate::plain_path)
            .with_context(|| format!("Read {}", module_directory.display()))?,
        &runtime.join("sdk"),
        &directory,
        Path::new(cc),
    )
}

/// The export table of the core: for each service of `grkit_core::grm::EXPORTS`,
/// the hash of its name and the offset of its code from the start of the tail.
fn core_exports(
    layout: &components::Layout,
    components: &[&components::Component],
) -> Result<Vec<(u32, u32)>> {
    grkit_core::grm::EXPORTS
        .iter()
        .map(|&(name, symbol)| {
            let (base, offset) = components
                .iter()
                .find_map(|c| {
                    c.symbols
                        .get(symbol)
                        .map(|offset| (c.patches[0].address, *offset))
                })
                .with_context(|| {
                    format!("The core has no symbol {symbol} for the export {name}")
                })?;
            Ok((
                grkit_core::grm::symbol_hash(name),
                base - layout.tail + offset,
            ))
        })
        .collect()
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
    let output = output.canonicalize().map(crate::plain_path)?;
    let objects = output.join("objects");
    let runtime = objects.join("runtime");
    materialize_runtime(&runtime)?;
    fs::write(runtime.join("hooks.h"), components::hooks_header(image)?)?;
    let mut built: Vec<(&str, components::Component)> = Vec::new();
    let mut processing_end = image.target.layout.tail;
    if image.target.family.processing_source.is_some() {
        let component = components::processing(&objects, &runtime, image, tools)?;
        processing_end = component.patches[0].address + component.code_size as u32;
        built.push(("processing", component));
    }
    let menu = components::menu(&objects, &runtime, image, tools, processing_end)?;
    let menu_base = menu.patches[0].address;
    let framing = components::framing(
        &objects,
        &runtime,
        image,
        tools,
        menu_base + menu.code_size as u32,
    )?;
    built.push(("menu", menu));
    built.push(("framing", framing));
    let components: Vec<_> = built.iter().map(|(_, component)| component).collect();
    // The installer publishes every code payload before any firmware change.
    let mut patches: Vec<_> = components.iter().map(|c| c.patches[0].clone()).collect();
    let mut first = Vec::new();
    for component in &components {
        first.push(patches.len());
        patches.extend_from_slice(&component.patches[1..]);
    }
    let relocations = components
        .iter()
        .enumerate()
        .flat_map(|(index, c)| c.relocations.iter().map(move |offset| (index, *offset)))
        .collect();
    // Component patch 0 is the payload; patch j > 0 follows the payloads.
    let references = components
        .iter()
        .enumerate()
        .flat_map(|(index, c)| {
            let first = first[index];
            c.references.iter().map(move |reference| Reference {
                patch: if reference.patch == 0 {
                    index
                } else {
                    first + reference.patch - 1
                },
                ..*reference
            })
        })
        .collect();
    let exports = core_exports(image.target.layout, &components)?;
    let plan = PatchPlan {
        patches,
        relocations,
        references,
        exports,
    };
    validate_plan(&plan)?;
    let core_size = plan.core_size(image.target.layout.tail)?;
    let core_end = image.target.layout.tail + core_size;
    let generated = loader::header(&plan, image, env!("CARGO_PKG_VERSION"))?;
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
    let fast = fastboot::files(tools, &objects, &runtime, image.target.name)?;
    let files = runtime_files_with(&fs::read(binary)?, &fast, icon_stride(image.target.name)?);
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
    let Value::Object(records) = json!({
        "schema_version": 1, "version": env!("CARGO_PKG_VERSION"),
        "toolchain": tools.manifest(), "source_revision": null, "source_dirty": null,
        "firmware_sha256": image.sha256, "decoded_firmware_sha256": image.decoded_sha256,
        "source_sha256": source_hashes(), "generated_header_sha256": sha256(generated.as_bytes()),
        "components": built.iter().map(|(name, component)| (name.to_string(),
            json!({"sha256": sha256(&component.patches[0].after), "size": component.code_size, "limit": component.limit,
                "address": component.patches[0].address,
                "end": component.patches[0].address + component.code_size as u32})))
            .collect::<serde_json::Map<_, _>>(),
        "code_memory": {"base": image.target.layout.tail, "end": core_end,
            "used": core_size, "available": components::RTOS_EXECUTABLE_END - core_end,
            "limit": components::RTOS_EXECUTABLE_END},
        // The firmware changes of each extension point, which a package
        // without its modules does not make.
        "points": components::Point::ALL.iter().map(|point| (point.name().to_owned(),
            json!(plan.patches.iter().filter(|patch| patch.points & point.bit() != 0).count())))
            .collect::<serde_json::Map<_, _>>(),
    }) else {
        unreachable!("an object literal")
    };
    let mut manifest = grkit_core::packaging::manifest(image.target.name, &files);
    manifest.extend(records);
    let manifest = Value::Object(manifest);
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

/// Drops the Windows verbatim prefix (`\\?\`) that
/// `canonicalize` adds, because Clang resolves `#include "..."` against it.
pub(crate) fn plain_path(path: PathBuf) -> PathBuf {
    match path.to_str().and_then(|s| s.strip_prefix(r"\\?\")) {
        Some(rest) if !rest.starts_with(r"UNC\") => PathBuf::from(rest),
        _ => path,
    }
}
