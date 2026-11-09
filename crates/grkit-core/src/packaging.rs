//! Deterministic SD package assembly and bounded, strict package verification.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Read, Write};

use anyhow::{bail, ensure, Context, Result};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use zip::{write::SimpleFileOptions, CompressionMethod, DateTime, ZipArchive, ZipWriter};

use crate::{
    camera::{self, Camera},
    card, effects, imports,
    language::Language,
    modules, runtime, Files,
};

pub const RUNTIME_PATHS: [&str; 6] = [
    "SCRIPT/STARTUP.TTL",
    card::LOADER_FILE,
    card::RESIDENT_FILE,
    card::CORE_FILE,
    card::SCAN_FILE,
    card::INSTALL_FILE,
];
pub const MAX_ENTRIES: usize = 200;
pub const MAX_FILE_SIZE: usize = 16 * 1024 * 1024;
pub const MAX_TOTAL_SIZE: usize = 32 * 1024 * 1024;

pub fn sha256(blob: &[u8]) -> String {
    hex::encode(Sha256::digest(blob))
}

/// Preserve the existing manifest representation, including ASCII Unicode escapes.
pub fn json_bytes(value: &Value) -> Result<Vec<u8>> {
    let pretty = serde_json::to_string_pretty(value)?;
    let mut ascii = String::with_capacity(pretty.len() + 1);
    for character in pretty.chars() {
        if character.is_ascii() && character != '\u{7f}' {
            ascii.push(character);
        } else {
            let mut encoded = [0; 2];
            for unit in character.encode_utf16(&mut encoded) {
                use std::fmt::Write;
                write!(ascii, "\\u{unit:04x}")?;
            }
        }
    }
    ascii.push('\n');
    Ok(ascii.into_bytes())
}

/// Checks an archive path: printable ASCII segments, none of them `.` or `..`.
/// The localized guide names (`Language::guide_path`) are the only exceptions.
pub fn safe_path(name: &str) -> Result<&str> {
    if Language::ALL
        .into_iter()
        .any(|language| language.guide_path() == name)
    {
        return Ok(name);
    }
    ensure!(
        !name.is_empty()
            && !name.contains(['\\', ':'])
            && name.bytes().all(|byte| (32..=126).contains(&byte))
            && name
                .split('/')
                .all(|part| !part.is_empty() && part != "." && part != ".."),
        "Invalid package path: {name:?}"
    );
    Ok(name)
}

/// The fields of a runtime manifest (`build.json`) that `load_runtime` checks: the
/// target, the production profile that starts in Off, and the size and SHA-256 of each
/// file of the build's `sd/` tree. `grkit_build` adds its build records.
pub fn manifest(target: &str, files: &Files) -> Map<String, Value> {
    let records = files
        .iter()
        .map(|(path, bytes)| json!({"path": path, "size": bytes.len(), "sha256": sha256(bytes)}))
        .collect();
    Map::from_iter([
        ("target".to_owned(), json!(target)),
        ("profile".to_owned(), json!("production")),
        ("startup_activation".to_owned(), json!("Off")),
        ("files".to_owned(), Value::Array(records)),
    ])
}

pub fn read_manifest(blob: &[u8]) -> Result<Value> {
    let result = effects::parse_json(blob)?;
    ensure!(result.is_object(), "Expected a manifest object");
    let records = result
        .get("files")
        .and_then(Value::as_array)
        .context("Expected a manifest file list")?;
    let mut paths = BTreeSet::new();
    for item in records {
        let record = item.as_object().context("Invalid manifest file record")?;
        ensure!(
            record.len() == 3
                && record.contains_key("path")
                && record.contains_key("size")
                && record.contains_key("sha256"),
            "Invalid manifest file record"
        );
        ensure!(
            record["size"].as_u64().is_some(),
            "Invalid manifest file size"
        );
        let hash = record["sha256"]
            .as_str()
            .context("Invalid manifest checksum")?;
        ensure!(
            hash.len() == 64
                && hash
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
            "Invalid manifest checksum"
        );
        let path = safe_path(record["path"].as_str().context("Invalid manifest path")?)?;
        ensure!(
            paths.insert(path.to_ascii_lowercase()),
            "Duplicate or case-colliding manifest path"
        );
    }
    Ok(result)
}

/// The prefix of the string that names the runtime build in `CORE.BIN`.
pub const TARGET_MARKER: &str = "GRKIT_TARGET=";

/// The runtime build that `CORE.BIN` names, if it names one. Runtimes from
/// before a camera had several builds do not.
fn marked_target(binary: &[u8]) -> Option<&str> {
    let marker = TARGET_MARKER.as_bytes();
    let start = binary
        .windows(marker.len())
        .position(|part| part == marker)?
        + marker.len();
    let end = start + binary[start..].iter().position(|&byte| byte == 0)?;
    std::str::from_utf8(&binary[start..end]).ok()
}

/// The runtime build of a camera with one build that a package's runtime files
/// are: the build that the installer names, or else the build whose templates
/// the files match.
fn runtime_target(runtime: &Files, camera: &Camera) -> Result<&'static str> {
    let targets = camera.targets();
    if let Some(name) = runtime
        .get(card::CORE_FILE)
        .and_then(|binary| marked_target(binary))
    {
        let target = targets
            .iter()
            .copied()
            .find(|target| *target == name)
            .with_context(|| format!("{} does not run the {name} runtime", camera.name))?;
        validate_runtime(runtime, target)?;
        return Ok(target);
    }
    let matching: Vec<_> = targets
        .iter()
        .copied()
        .filter(|target| validate_runtime(runtime, target).is_ok())
        .collect();
    match matching[..] {
        // Report why the runtime is not the first build of the camera.
        [] => Err(validate_runtime(runtime, targets[0]).unwrap_err()),
        // A runtime without the marker predates the later builds.
        [target, ..] => Ok(target),
    }
}

/// The runtime builds of a package for `camera`, from its core files
/// (`runtime::package_runtime_files`): one for each build of the camera, in
/// release order, each named by its marker. A camera with one build has one
/// core file.
fn package_targets(runtime: &Files, camera: &Camera) -> Result<Vec<&'static str>> {
    let fast = camera_fast_files(runtime, camera)?;
    let targets = camera.targets();
    if targets.len() == 1 {
        return Ok(vec![runtime_target(runtime, camera)?]);
    }
    let binaries: Vec<&[u8]> = (0..targets.len())
        .map(|index| {
            runtime
                .get(&runtime::core_path_of(index, targets.len()))
                .map(Vec::as_slice)
                .with_context(|| {
                    format!(
                        "{} needs an installer for each of {}",
                        camera.name,
                        targets.join(", ")
                    )
                })
        })
        .collect::<Result<_>>()?;
    for (binary, target) in binaries.iter().zip(&targets) {
        let name =
            marked_target(binary).context("Expected an installer that names its runtime build")?;
        ensure!(
            name == *target,
            "The installer is the {name} runtime, not {target}"
        );
        check_installer(binary)?;
    }
    ensure!(
        *runtime
            == runtime::package_runtime_files_with(
                &binaries,
                &fast,
                runtime::icon_stride(targets[0])?
            ),
        "Runtime scripts differ from the production templates"
    );
    Ok(targets)
}

/// Checks that an installer is a production ARM program.
fn check_installer(binary: &[u8]) -> Result<()> {
    ensure!(
        binary.len() >= 52
            && binary[..6] == *b"\x7fELF\x01\x01"
            && binary[16..20] == *b"\x02\x00\x28\x00"
            && !binary.windows(8).any(|w| w == b"--select"),
        "Expected a production installer without diagnostic selection"
    );
    Ok(())
}

/// The runtime files of a package with each runtime of `camera`, in release
/// order (`runtime::package_runtime_files`).
fn package_runtime(runtimes: &[VerifiedRuntime], camera: &Camera) -> Result<Files> {
    let targets: Vec<&str> = runtimes.iter().map(VerifiedRuntime::target).collect();
    ensure!(
        targets == camera.targets(),
        "A package for {} needs the runtime of each of its builds in release order: {}",
        camera.name,
        camera.targets().join(", ")
    );
    let binaries: Vec<&[u8]> = runtimes
        .iter()
        .map(|runtime| runtime.files()[card::CORE_FILE].as_slice())
        .collect();
    // The fast-boot files of the camera's releases. A build for two cameras
    // has those of both.
    let mut fast = Files::new();
    for runtime in runtimes {
        let own = runtime
            .files()
            .iter()
            .filter(|(path, _)| {
                runtime::FastRelease::of_path(path).is_some_and(|release| release.camera == camera)
            })
            .map(|(path, bytes)| (path.clone(), bytes.clone()));
        fast.extend(own);
    }
    let files =
        runtime::package_runtime_files_with(&binaries, &fast, runtime::icon_stride(targets[0])?);
    // The builds of a camera share every file but the core file and the
    // fast-boot files. The package has its own scripts that run them.
    let own = [
        card::CORE_FILE,
        card::INSTALL_FILE,
        card::SCAN_FILE,
        card::LOADER_FILE,
    ];
    for runtime in runtimes {
        for (name, blob) in runtime.files() {
            ensure!(
                own.contains(&name.as_str())
                    || runtime::FastRelease::of_path(name).is_some()
                    || runtime::FastRelease::of_install_path(name).is_some()
                    || files.get(name) == Some(blob),
                "The {} runtime has another {name} than {}",
                runtime.target(),
                targets[0]
            );
        }
    }
    Ok(files)
}

/// Whether `name` is the path of a runtime file: one of `RUNTIME_PATHS`, a
/// fast-boot file, or the install file of one (`runtime::fastboot`).
pub fn is_runtime_path(name: &str) -> bool {
    RUNTIME_PATHS.contains(&name)
        || runtime::FastRelease::of_path(name).is_some()
        || runtime::FastRelease::of_install_path(name).is_some()
}

/// The fast-boot files of `files` (`runtime::fast_files`), each checked as
/// one of `camera` built by its runtime build.
fn camera_fast_files(files: &Files, camera: &Camera) -> Result<Files> {
    let mut fast = Files::new();
    for (release, bytes) in runtime::fast_files(files) {
        ensure!(
            release.camera == camera,
            "{} is a fast-boot file of {}, not {}",
            release.name(),
            release.camera.name,
            camera.name
        );
        runtime::check_fast_file(&release, bytes)?;
        fast.insert(release.path(), bytes.clone());
    }
    Ok(fast)
}

/// Checks the runtime files of a build for `target`. A build has the
/// fast-boot files of its releases that have one (`runtime::fastboot`).
pub fn validate_runtime(files: &Files, target: &str) -> Result<()> {
    let fast: Files = files
        .iter()
        .filter(|(path, _)| runtime::FastRelease::of_path(path).is_some())
        .map(|(path, bytes)| (path.clone(), bytes.clone()))
        .collect();
    for (release, bytes) in runtime::fast_files(&fast) {
        ensure!(
            release.target() == target,
            "{} is a fast-boot file of the {} runtime, not {target}",
            release.name(),
            release.target()
        );
        runtime::check_fast_file(&release, bytes)?;
    }
    ensure!(
        RUNTIME_PATHS.iter().all(|p| files.contains_key(*p))
            && files.keys().all(|path| is_runtime_path(path)),
        "Expected the complete production runtime file set"
    );
    let binary = &files[card::CORE_FILE];
    check_installer(binary)?;
    ensure!(
        *files == runtime::runtime_files_with(binary, &fast, runtime::icon_stride(target)?),
        "Runtime scripts differ from the production templates"
    );
    if let Some(name) = marked_target(binary) {
        ensure!(
            name == target,
            "The installer is the {name} runtime, not {target}"
        );
    }
    Ok(())
}

/// The paths of the runtime files that a `build.json` lists, under the build's `sd/` tree.
pub fn manifest_paths(manifest: &[u8]) -> Result<Vec<String>> {
    read_manifest(manifest)?["files"]
        .as_array()
        .context("Expected a manifest file list")?
        .iter()
        .map(|item| {
            item["path"]
                .as_str()
                .map(str::to_owned)
                .context("Expected a runtime path")
        })
        .collect()
}

/// The runtime files of a production build that `load_runtime` checked against its
/// manifest and the templates of its target.
#[derive(Clone, Debug)]
pub struct VerifiedRuntime {
    files: Files,
    target: String,
}

impl VerifiedRuntime {
    /// The files of the build's `sd/` tree.
    pub fn files(&self) -> &Files {
        &self.files
    }

    /// The runtime build, such as `gr3-2.10`.
    pub fn target(&self) -> &str {
        &self.target
    }
}

/// Checks a runtime build against its `build.json`.
///
/// `files` holds every file of the build's `sd/` tree, keyed by its path in that tree.
/// The file set must equal the manifest list, and each file must match its size and SHA-256.
pub fn load_runtime(manifest: &[u8], files: &Files) -> Result<VerifiedRuntime> {
    let manifest = read_manifest(manifest)?;
    let target = manifest["target"]
        .as_str()
        .context("Expected a runtime target")?
        .to_owned();
    ensure!(
        manifest["profile"] == "production"
            && runtime::icon_stride(&target).is_ok()
            && manifest["startup_activation"] == "Off",
        "Expected a production build for a supported target that starts in Off"
    );
    let mut runtime = Files::new();
    for item in manifest["files"]
        .as_array()
        .context("Expected a manifest file list")?
    {
        let name = item["path"].as_str().context("Expected a runtime path")?;
        ensure!(
            is_runtime_path(name) && !runtime.contains_key(name),
            "Invalid runtime manifest entries"
        );
        let bytes = files
            .get(name)
            .context("Runtime directory differs from its manifest")?;
        ensure!(
            item["size"].as_u64() == Some(bytes.len() as u64) && item["sha256"] == sha256(bytes),
            "Runtime hash or size mismatch: {name}"
        );
        runtime.insert(name.to_owned(), bytes.clone());
    }
    ensure!(
        runtime.len() == files.len(),
        "Runtime directory differs from its manifest"
    );
    validate_runtime(&runtime, &target)?;
    Ok(VerifiedRuntime {
        files: runtime,
        target,
    })
}

pub use crate::card::EFFECTS_FOLDER;

pub use crate::card::PACKAGE_LIST;

/// The compiled effects of a package, each under its package filename (`effects::filename`).
#[derive(Clone, Debug, Default)]
pub struct EffectSet {
    files: Files,
}

impl EffectSet {
    /// Adds one compiled effect and returns its ID. It refuses an effect that does not
    /// decode, a second effect with the same filename, and more than `runtime::MAX_COUNT`
    /// effects.
    pub fn insert(&mut self, blob: Vec<u8>) -> Result<String> {
        let id = effects::decode(&blob)?.0.id;
        let name = effects::filename(&id)?;
        ensure!(
            !self.files.contains_key(&name),
            "Duplicate effect ID or filename hash collision: {id}"
        );
        ensure!(
            self.files.len() < runtime::MAX_COUNT,
            "This target supports at most {} effects",
            runtime::MAX_COUNT
        );
        self.files.insert(name, blob);
        Ok(id)
    }
}

/// The contents of `PACKAGE_LIST`: the file names in name order, one on
/// each line.
fn package_list<'a>(names: impl IntoIterator<Item = &'a String>) -> Vec<u8> {
    let mut names: Vec<&String> = names.into_iter().collect();
    names.sort();
    names
        .into_iter()
        .map(|name| format!("{name}\n"))
        .collect::<String>()
        .into_bytes()
}

fn is_effect_path(name: &str) -> bool {
    let Some(stem) = name
        .strip_prefix("SD_ROOT/SCRIPT/GRKIT/EFFECTS/")
        .and_then(|s| s.strip_suffix(".GRF"))
    else {
        return false;
    };
    (1..=8).contains(&stem.len())
        && stem
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
}

/// What a valid package holds besides the runtime.
#[derive(Debug)]
pub struct Contents {
    /// The camera whose ModeSet file the package has.
    pub camera: &'static Camera,
    /// The runtime builds of the camera that the package has, in release order:
    /// one for each build.
    pub targets: Vec<&'static str>,
    pub effect_ids: Vec<String>,
    /// The IDs of the module files (`grkit_core::grm`) in the package.
    pub modules: Vec<String>,
    /// The language of the guide.
    pub language: Language,
    /// The number of imports of the import library (`card::IMPORTS_FILE`), which the camera
    /// does not install, or 0 without one.
    pub imports: usize,
    /// The number of package entries, the runtime included.
    pub entries: usize,
}

/// Checks the package entries.
pub fn validate_contents(contents: &Files) -> Result<Contents> {
    ensure!(contents.len() <= MAX_ENTRIES, "Too many package entries");
    let mut paths = BTreeSet::new();
    let mut total = 0usize;
    for (name, blob) in contents {
        safe_path(name)?;
        ensure!(
            paths.insert(name.to_ascii_lowercase()),
            "Case-colliding package paths"
        );
        ensure!(
            blob.len() <= MAX_FILE_SIZE,
            "Package entry exceeds size limit"
        );
        total = total
            .checked_add(blob.len())
            .context("Package size overflow")?;
    }
    ensure!(total <= MAX_TOTAL_SIZE, "Package exceeds total size limit");
    let cameras: Vec<_> = camera::CAMERAS
        .into_iter()
        .filter(|camera| contents.contains_key(&format!("SD_ROOT/{}", camera.modeset)))
        .collect();
    let [camera] = cameras[..] else {
        bail!("Package must have the ModeSet file of exactly one camera");
    };
    let activation = runtime::activation_files(camera);
    let cores = camera.targets().len();
    // The fast-boot files and their install files, which package_targets
    // checks.
    let fast_paths: Vec<String> = contents
        .keys()
        .filter_map(|name| name.strip_prefix("SD_ROOT/"))
        .filter(|path| {
            runtime::FastRelease::of_path(path).is_some()
                || runtime::FastRelease::of_install_path(path).is_some()
        })
        .map(str::to_owned)
        .collect();
    let runtime_paths: Vec<String> = RUNTIME_PATHS
        .iter()
        .filter(|name| **name != card::CORE_FILE)
        .map(|name| (*name).to_owned())
        .chain((0..cores).map(|index| runtime::core_path_of(index, cores)))
        .chain(fast_paths)
        .collect();
    let mut mandatory: BTreeSet<String> = runtime_paths
        .iter()
        .map(|name| format!("SD_ROOT/{name}"))
        .collect();
    mandatory.extend(activation.keys().map(|name| format!("SD_ROOT/{name}")));
    mandatory.extend(["LICENSES.md", "SHA256SUMS"].map(String::from));
    mandatory.insert(format!("SD_ROOT/{PACKAGE_LIST}"));
    ensure!(
        mandatory.iter().all(|name| contents.contains_key(name)),
        "Package is missing required files"
    );
    for (name, expected) in activation {
        ensure!(
            contents[&format!("SD_ROOT/{name}")] == expected,
            "Activation resources differ from the camera"
        );
    }
    let runtime = runtime_paths
        .iter()
        .map(|name| (name.clone(), contents[&format!("SD_ROOT/{name}")].clone()))
        .collect();
    let targets = package_targets(&runtime, camera)?;
    let mut ids = BTreeSet::new();
    let mut module_ids = BTreeSet::new();
    let mut native_slots = BTreeMap::new();
    let mut status_icons = 0;
    let mut listed = Vec::new();
    let mut language = None;
    let mut library = 0;
    for (name, blob) in contents
        .iter()
        .filter(|(name, _)| !mandatory.contains(*name))
    {
        if is_effect_path(name) {
            let id = effects::decode(blob)?.0.id;
            ensure!(
                name.rsplit('/').next() == Some(effects::filename(&id)?.as_str()),
                "Profile filename does not match its stable ID"
            );
            ensure!(ids.insert(id), "Duplicate effect ID");
            listed.push(name.rsplit('/').next().unwrap_or_default().to_owned());
        } else if let Some(file) = name
            .strip_prefix("SD_ROOT/")
            .and_then(|path| path.strip_prefix(crate::grm::FOLDER))
            .and_then(|path| path.strip_prefix('/'))
        {
            let module = crate::grm::Module::decode(blob)
                .with_context(|| format!("Read the module {file}"))?;
            ensure!(
                file == format!("{}.GRM", module.id),
                "The module file {file} is not named by its ID {}",
                module.id
            );
            for placement in module
                .indicator_placements()
                .with_context(|| format!("Read the viewfinder indicators of {}", module.id))?
            {
                match placement {
                    crate::grm::IndicatorPlacement::NativeSlot(slot) => {
                        if let Some(owner) = native_slots.insert(slot, module.id.clone()) {
                            bail!("Viewfinder slot image_control is registered by both {owner} and {}", module.id);
                        }
                    }
                    crate::grm::IndicatorPlacement::StatusRow => {
                        status_icons += 1;
                        ensure!(
                            status_icons <= crate::abi::STATUS_ICONS_MAX,
                            "The package exceeds the viewfinder status-row capacity of {} icons at module {}",
                            crate::abi::STATUS_ICONS_MAX,
                            module.id
                        );
                    }
                }
            }
            listed.push(file.to_owned());
            module_ids.insert(module.id);
        } else if let Some(guide) = Language::ALL
            .into_iter()
            .find(|language| language.guide_path() == *name)
        {
            ensure!(language.is_none(), "Package has more than one guide");
            check_guide(blob)?;
            language = Some(guide);
        } else if *name == format!("SD_ROOT/{}", card::IMPORTS_FILE) {
            library = imports::decode(blob)
                .context("Read the import library")?
                .len();
        } else {
            bail!("Package contains unexpected files: {name}");
        }
    }
    ensure!(
        contents[&format!("SD_ROOT/{PACKAGE_LIST}")] == package_list(listed.iter()),
        "{PACKAGE_LIST} must list the effect and module files of the package"
    );
    ensure!(
        ids.len() <= runtime::MAX_COUNT,
        "Target effect capacity exceeded"
    );
    ensure!(
        camera.effects || ids.is_empty(),
        "{} has no Image Control yet, so a package for it cannot have effects",
        camera.name
    );
    // The effects show only on the page of Image Control, which shows only
    // with effects.
    let needs_effects =
        |id: &String| modules::official(id).is_some_and(|module| module.needs_effects);
    ensure!(
        ids.is_empty() || module_ids.iter().any(needs_effects),
        "The effects need the Image Control module"
    );
    ensure!(
        module_ids
            .iter()
            .any(|id| !needs_effects(id) || !ids.is_empty()),
        "The package shows nothing: add a module other than Image Control, or add effects"
    );
    Ok(Contents {
        camera,
        targets,
        effect_ids: ids.into_iter().collect(),
        modules: module_ids.into_iter().collect(),
        language: language.context("Package has no installation guide")?,
        imports: library,
        entries: contents.len(),
    })
}

/// Checks that `pdf` is a PDF document. The release builds the guides.
fn check_guide(pdf: &[u8]) -> Result<()> {
    ensure!(
        pdf.starts_with(b"%PDF-"),
        "The installation guide is not a PDF document"
    );
    Ok(())
}

fn checksum_list(contents: &Files) -> Vec<u8> {
    contents
        .iter()
        .filter(|(name, _)| name.as_str() != "SHA256SUMS")
        .map(|(name, blob)| format!("{}  {name}\n", sha256(blob)))
        .collect::<String>()
        .into_bytes()
}

/// The license notice of every package. CC BY-SA 4.0 and the MIT License
/// require the attribution and the notices in each copy of an effect, so the
/// package carries them with the effects.
pub const LICENSES: &str = concat!(
    include_str!("../../../resources/LICENSES.md"),
    "\n## Changes to the LUTs\n\n```text\n",
    include_str!("../../../effects/sources/CHANGELOG.txt"),
    "```\n\n## GRKit license\n\n```text\n",
    include_str!("../../../LICENSE"),
    "```\n\n## Spectral Film LUT license\n\n```text\n",
    include_str!("../../../effects/sources/SPECTRAL_FILM_LUT_LICENSE.txt"),
    "```\n\n## t3mujinpack license\n\n```text\n",
    include_str!("../../../effects/sources/T3MUJINPACK_LICENSE.txt"),
    "```\n",
);

/// The installation guide of a package: its language and the PDF that the
/// release built from `guide::source` for the package's camera.
pub struct Guide<'a> {
    pub language: Language,
    pub pdf: &'a [u8],
}

/// A package ZIP and what it holds.
#[derive(Debug)]
pub struct Package {
    pub bytes: Vec<u8>,
    pub contents: Contents,
}

/// The files of a package that the camera does not install.
#[derive(Debug, Default, Clone, Copy)]
pub struct Extras<'a> {
    /// The import library of the site (`card::IMPORTS_FILE`, `crate::imports`).
    pub imports: Option<&'a [u8]>,
}

/// Builds the package ZIP for one camera from the runtime of each of its
/// builds, in release order. `modules` holds the module files
/// (`grkit_core::grm`).
pub fn assemble(
    runtimes: &[VerifiedRuntime],
    effects: &EffectSet,
    modules: &[Vec<u8>],
    camera: &Camera,
    guide: &Guide,
) -> Result<Package> {
    assemble_with(runtimes, effects, modules, camera, guide, Extras::default())
}

/// `assemble`, with the files of `extras`.
pub fn assemble_with(
    runtimes: &[VerifiedRuntime],
    effects: &EffectSet,
    modules: &[Vec<u8>],
    camera: &Camera,
    guide: &Guide,
    extras: Extras,
) -> Result<Package> {
    let mut contents: Files = package_runtime(runtimes, camera)?
        .iter()
        .map(|(name, blob)| (format!("SD_ROOT/{name}"), blob.clone()))
        .collect();
    contents.extend(
        runtime::activation_files(camera)
            .into_iter()
            .map(|(name, blob)| (format!("SD_ROOT/{name}"), blob)),
    );
    for (name, blob) in &effects.files {
        contents.insert(format!("SD_ROOT/{EFFECTS_FOLDER}/{name}"), blob.clone());
    }
    let mut listed: Vec<String> = effects.files.keys().cloned().collect();
    ensure!(
        modules.len() <= runtime::MODULES_MAX,
        "A package holds at most {} modules",
        runtime::MODULES_MAX
    );
    for bytes in modules {
        let id = crate::grm::Module::decode(bytes)?.id;
        let path = format!("SD_ROOT/{}/{id}.GRM", crate::grm::FOLDER);
        ensure!(
            contents.insert(path, bytes.clone()).is_none(),
            "The module {id} is twice in the package"
        );
        listed.push(format!("{id}.GRM"));
    }
    listed.sort();
    contents.insert(
        format!("SD_ROOT/{PACKAGE_LIST}"),
        package_list(listed.iter()),
    );
    check_guide(guide.pdf)?;
    contents.insert(guide.language.guide_path(), guide.pdf.to_vec());
    if let Some(bytes) = extras.imports {
        contents.insert(format!("SD_ROOT/{}", card::IMPORTS_FILE), bytes.to_vec());
    }
    contents.insert("LICENSES.md".into(), LICENSES.as_bytes().to_vec());
    contents.insert("SHA256SUMS".into(), checksum_list(&contents));
    validate_contents(&contents)?;
    let bytes = write_zip(contents)?;
    // Read the ZIP back as `grkit verify` does, so the writer cannot drift from the checks.
    let contents = verify_package(&bytes)?;
    Ok(Package { bytes, contents })
}

/// The ZIP of the files that turn SD scripts off on `camera`: its ModeSet
/// file with the disable command, `DEVELOP.MOD`, and their checksums. It holds
/// no GRKit code.
pub fn assemble_scripts_off(camera: &Camera) -> Result<Vec<u8>> {
    let mut contents: Files = runtime::deactivation_files(camera)
        .into_iter()
        .map(|(name, blob)| (format!("SD_ROOT/{name}"), blob))
        .collect();
    contents.insert("SHA256SUMS".into(), checksum_list(&contents));
    write_zip(contents)
}

/// The paths of the firmware dump ZIP, without its checksums.
pub const DUMP_FILES: [&str; 4] = [
    "README.txt",
    "SD_ROOT/SCRIPT/STARTUP.TTL",
    "SD_ROOT/SCRIPT/GRDUMP/DUMP.SH",
    "SD_ROOT/SCRIPT/GRDUMP/DUMP.BIN",
];

/// The ZIP of the firmware dump tool: exactly `DUMP_FILES` and their
/// checksums. The native build crate compiles the program.
pub fn assemble_dump(mut contents: Files) -> Result<Vec<u8>> {
    ensure!(
        contents.len() == DUMP_FILES.len()
            && DUMP_FILES
                .iter()
                .all(|name| contents.get(*name).is_some_and(|bytes| !bytes.is_empty())),
        "The dump tool needs exactly {DUMP_FILES:?}"
    );
    contents.insert("SHA256SUMS".into(), checksum_list(&contents));
    write_zip(contents)
}

/// The time of a ZIP entry with `blob`: a time in 2026 or 2027 that follows from
/// the bytes, so the ZIP stays deterministic and a file whose bytes change
/// gets another time. A copy of the ZIP keeps the times, and the camera takes a
/// file with the size and the time that it recorded as unchanged
/// (`runtime/install_library.h`, `runtime/fastboot/backend.c`). Months have 28
/// days, so each date is valid, and seconds are even, as FAT keeps them.
fn entry_time(blob: &[u8]) -> DateTime {
    let digest = Sha256::digest(blob);
    let hash = u64::from_le_bytes(digest[..8].try_into().expect("8 bytes"));
    let day = (hash % (2 * 12 * 28)) as u16;
    let seconds = ((hash / (2 * 12 * 28)) % (24 * 60 * 30)) as u32 * 2;
    DateTime::from_date_and_time(
        2026 + day / (12 * 28),
        (day % (12 * 28) / 28 + 1) as u8,
        (day % 28 + 1) as u8,
        (seconds / 3600) as u8,
        (seconds / 60 % 60) as u8,
        (seconds % 60) as u8,
    )
    .expect("a valid time")
}

/// A deterministic ZIP of `contents`, with stored entries and the time of
/// each entry from its bytes (`entry_time`).
fn write_zip(contents: Files) -> Result<Vec<u8>> {
    let mut archive = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, blob) in contents {
        let mode = if name.ends_with(".BIN") || name.ends_with(".SH") {
            0o755
        } else {
            0o644
        };
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Stored)
            .last_modified_time(entry_time(&blob))
            .unix_permissions(mode);
        archive.start_file(name, options)?;
        archive.write_all(&blob)?;
    }
    Ok(archive.finish()?.into_inner())
}

/// Checks a package ZIP: its entries, its contents (`validate_contents`), and `SHA256SUMS`.
pub fn verify_package(bytes: &[u8]) -> Result<Contents> {
    Ok(read_package(bytes)?.0)
}

/// The files that a package puts on the card: its `SD_ROOT` entries, at their
/// paths from the card root. The page writes them to a card as a merge copy of
/// `SD_ROOT` does. The ZIP is checked as `verify_package` checks it.
pub fn card_files(bytes: &[u8]) -> Result<Files> {
    Ok(read_package(bytes)?
        .1
        .into_iter()
        .filter_map(|(name, blob)| {
            name.strip_prefix("SD_ROOT/")
                .map(|path| (path.to_owned(), blob))
        })
        .collect())
}

/// The files that a GRKit ZIP of any release puts on the card, at their paths from the card
/// root, so the site can read a ZIP as the card that holds it. The archive limits apply, but the
/// package checks do not: a ZIP of an older release, whose runtime this version does not know,
/// still gives its files. The caller checks each file that it reads.
pub fn package_files(bytes: &[u8]) -> Result<Files> {
    Ok(read_entries(bytes)?
        .into_iter()
        .filter_map(|(name, blob)| {
            name.strip_prefix("SD_ROOT/")
                .map(|path| (path.to_owned(), blob))
        })
        .collect())
}

/// Reads and checks a package ZIP, and returns what it holds and its entries.
fn read_package(bytes: &[u8]) -> Result<(Contents, Files)> {
    let contents = read_entries(bytes)?;
    let result = validate_contents(&contents)?;
    ensure!(
        contents["SHA256SUMS"] == checksum_list(&contents),
        "SHA256SUMS differs from package"
    );
    Ok((result, contents))
}

/// The entries of a ZIP within the package limits: safe, unique names, no links, and bounded
/// sizes.
fn read_entries(bytes: &[u8]) -> Result<Files> {
    let declared_entries = declared_entry_count(bytes)?;
    let mut archive = ZipArchive::new(Cursor::new(bytes)).context("Invalid ZIP archive")?;
    // zip indexes names in a map. Check the raw count so duplicate names cannot disappear.
    ensure!(
        archive.len() == declared_entries,
        "Duplicate or inconsistent archive entries"
    );
    let mut contents = Files::new();
    let mut total = 0u64;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let raw_name = entry.name().context("Invalid archive entry name")?;
        let name = safe_path(&raw_name)?.to_owned();
        ensure!(
            !contents.contains_key(&name)
                && !entry.is_dir()
                && !entry.encrypted()
                && entry.unix_mode().unwrap_or(0) & 0o170000 != 0o120000,
            "Invalid archive entry"
        );
        ensure!(
            entry.size() <= MAX_FILE_SIZE as u64,
            "Archive entry exceeds size limit"
        );
        total = total
            .checked_add(entry.size())
            .context("Archive size overflow")?;
        ensure!(
            total <= MAX_TOTAL_SIZE as u64,
            "Archive exceeds total size limit"
        );
        // Bound actual decompression as well as the untrusted central-directory size.
        let expected_size = entry.size();
        let mut blob = Vec::with_capacity(expected_size as usize);
        (&mut entry)
            .take(MAX_FILE_SIZE as u64 + 1)
            .read_to_end(&mut blob)?;
        ensure!(
            blob.len() as u64 == expected_size,
            "Archive entry size mismatch"
        );
        contents.insert(name, blob);
    }
    Ok(contents)
}

fn declared_entry_count(bytes: &[u8]) -> Result<usize> {
    ensure!(bytes.len() >= 22, "Truncated ZIP archive");
    let start = bytes.len().saturating_sub(22 + u16::MAX as usize);
    let end = (start..=bytes.len() - 22)
        .rev()
        .find(|&position| {
            bytes[position..position + 4] == *b"PK\x05\x06"
                && position
                    + 22
                    + u16::from_le_bytes(bytes[position + 20..position + 22].try_into().unwrap())
                        as usize
                    == bytes.len()
        })
        .context("Invalid ZIP end record")?;
    let count = u16::from_le_bytes(bytes[end + 10..end + 12].try_into().unwrap());
    let count = if count == u16::MAX {
        ensure!(
            end >= 20 && bytes[end - 20..end - 16] == *b"PK\x06\x07",
            "Missing ZIP64 locator"
        );
        let offset = u64::from_le_bytes(bytes[end - 12..end - 4].try_into().unwrap());
        let position = usize::try_from(offset).context("ZIP64 offset exceeds platform limit")?;
        ensure!(
            position <= bytes.len().saturating_sub(56)
                && bytes[position..position + 4] == *b"PK\x06\x06",
            "Invalid ZIP64 end record"
        );
        u64::from_le_bytes(bytes[position + 32..position + 40].try_into().unwrap())
    } else {
        u64::from(count)
    };
    ensure!(count <= MAX_ENTRIES as u64, "Archive exceeds entry limit");
    Ok(count as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A copied file keeps its ZIP time, and the camera takes a file with the
    /// size and the time that it recorded as unchanged, so files with other
    /// bytes get other times, and the same bytes the same time.
    #[test]
    fn zip_entries_with_other_bytes_get_other_times() {
        let mut times = std::collections::BTreeSet::new();
        for i in 0..400u32 {
            let mut blob = vec![0u8; 8704];
            blob[..4].copy_from_slice(&i.to_le_bytes());
            let t = entry_time(&blob);
            assert!((2026..=2027).contains(&t.year()));
            assert!((1..=12).contains(&t.month()) && (1..=28).contains(&t.day()));
            assert_eq!(t.second() % 2, 0);
            times.insert((
                t.year(),
                t.month(),
                t.day(),
                t.hour(),
                t.minute(),
                t.second(),
            ));
        }
        assert_eq!(times.len(), 400);
        let blob = vec![7u8; 100];
        let (a, b) = (entry_time(&blob), entry_time(&blob));
        assert_eq!(
            (
                a.year(),
                a.month(),
                a.day(),
                a.hour(),
                a.minute(),
                a.second()
            ),
            (
                b.year(),
                b.month(),
                b.day(),
                b.hour(),
                b.minute(),
                b.second()
            )
        );
    }
}
