//! Filesystem policy for explicit local inputs and newly created outputs.

use anyhow::{bail, ensure, Context, Result};
use grkit_core::packaging::{self, EffectSet, VerifiedRuntime};
use grkit_core::release_index::SiteContents;
use grkit_core::{camera, effects, imports, release, Files};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const MAX_FILE_SIZE: usize = 16 * 1024 * 1024;

pub fn read_bounded(path: &Path, limit: usize) -> Result<Vec<u8>> {
    let metadata =
        fs::symlink_metadata(path).with_context(|| format!("Cannot read {}", path.display()))?;
    ensure!(
        metadata.is_file(),
        "Expected a regular file: {}",
        path.display()
    );
    ensure!(
        metadata.len() <= limit as u64,
        "File exceeds size limit: {}",
        path.display()
    );
    let mut bytes = Vec::new();
    File::open(path)?
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= limit,
        "File exceeds size limit: {}",
        path.display()
    );
    Ok(bytes)
}

fn read_regular(path: &Path) -> Result<Vec<u8>> {
    read_bounded(path, MAX_FILE_SIZE)
}

fn directory_entries(path: &Path) -> Result<Vec<PathBuf>> {
    ensure!(
        fs::symlink_metadata(path)?.is_dir(),
        "Expected a regular directory: {}",
        path.display()
    );
    let mut result = fs::read_dir(path)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    result.sort();
    Ok(result)
}

fn basename(path: &Path) -> Result<&str> {
    path.file_name()
        .and_then(|name| name.to_str())
        .context("Expected a UTF-8 filename")
}

/// Reads each `ID/effect.json` under `source`. Each directory name must match its effect ID.
pub fn read_definitions(source: &Path) -> Result<Vec<Value>> {
    let mut definitions = Vec::new();
    for directory in directory_entries(source)? {
        let metadata = fs::symlink_metadata(&directory)?;
        ensure!(
            !metadata.is_symlink(),
            "Effect source directory must not contain symlinks"
        );
        if !metadata.is_dir() {
            continue;
        }
        let path = directory.join("effect.json");
        if !path.try_exists()? && fs::symlink_metadata(&path).is_err() {
            continue;
        }
        let definition = effects::parse_json(&read_regular(&path)?)?;
        let id = definition["effect_id"]
            .as_str()
            .context("Expected an effect ID")?;
        ensure!(
            basename(&directory)? == id,
            "Effect directory must match its stable ID"
        );
        definitions.push(definition);
    }
    Ok(definitions)
}

pub fn compile_directory(source: &Path) -> Result<Files> {
    let mut files = Files::new();
    let mut ids = BTreeSet::new();
    for definition in read_definitions(source)? {
        let blob = effects::compile_definition(&definition)?;
        let id = definition["effect_id"].as_str().unwrap();
        let name = effects::filename(id)?;
        ensure!(
            ids.insert(id.to_owned()) && !files.contains_key(&name),
            "Duplicate effect ID or FAT filename hash collision"
        );
        files.insert(name, blob);
    }
    Ok(files)
}

/// Reads a directory of compiled effects. Each file must have the package filename of
/// its effect (`effects::filename`), in any letter case.
pub fn read_effects(directory: &Path) -> Result<EffectSet> {
    read_effects_with(directory, Ok)
}

/// Reads a directory of compiled effects as `read_effects` does, and passes each file through
/// `map`, which may change its icon but not its ID.
pub fn read_effects_with(
    directory: &Path,
    mut map: impl FnMut(Vec<u8>) -> Result<Vec<u8>>,
) -> Result<EffectSet> {
    let mut effects = EffectSet::default();
    for path in directory_entries(directory)? {
        let name = basename(&path)?;
        ensure!(
            name.to_ascii_uppercase().ends_with(".GRF"),
            "Unexpected compiled effect entry: {name}"
        );
        let id = effects.insert(map(read_regular(&path)?)?)?;
        ensure!(
            name.eq_ignore_ascii_case(&effects::filename(&id)?),
            "Invalid effect filename: {name}"
        );
    }
    Ok(effects)
}

/// The imports of the import library at `path`, or none when it does not exist.
pub fn read_imports(path: &Path) -> Result<Vec<imports::Import>> {
    if fs::symlink_metadata(path).is_err() {
        return Ok(Vec::new());
    }
    // The largest library: each import at its largest, with its two lengths.
    let limit = imports::MAGIC.len()
        + 4
        + imports::MAX_IMPORTS * (8 + imports::MAX_DETAILS + effects::SIZE_WITH_LEVELS);
    imports::decode(&read_bounded(path, limit)?)
        .with_context(|| format!("Read the import library {}", path.display()))
}

/// Replaces the regular file at `path`, or creates it, through a new file in the same
/// directory, so that a failed write keeps the old file.
pub fn replace(path: &Path, bytes: &[u8]) -> Result<()> {
    ensure!(
        fs::symlink_metadata(path).map_or(true, |metadata| metadata.is_file()),
        "Expected a regular file: {}",
        path.display()
    );
    let directory = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut file = tempfile::NamedTempFile::new_in(directory)
        .with_context(|| format!("Cannot write {}", path.display()))?;
    file.write_all(bytes)
        .with_context(|| format!("Cannot write {}", path.display()))?;
    file.persist(path)
        .with_context(|| format!("Cannot write {}", path.display()))?;
    Ok(())
}

fn collect_files(root: &Path, relative: &Path, files: &mut BTreeSet<String>) -> Result<()> {
    for path in directory_entries(&root.join(relative))? {
        let relative = path.strip_prefix(root)?;
        let metadata = fs::symlink_metadata(&path)?;
        ensure!(
            !metadata.is_symlink(),
            "Runtime paths must not contain symlinks"
        );
        if metadata.is_dir() {
            collect_files(root, relative, files)?;
        } else {
            ensure!(metadata.is_file(), "Expected a regular runtime file");
            let name = relative
                .components()
                .map(|part| {
                    part.as_os_str()
                        .to_str()
                        .context("Expected a UTF-8 runtime path")
                })
                .collect::<Result<Vec<_>>>()?
                .join("/");
            packaging::safe_path(&name)?;
            files.insert(name);
        }
    }
    Ok(())
}

/// Reads a build directory's `build.json` bytes and every file under its `sd/` tree.
fn read_build(directory: &Path) -> Result<(Vec<u8>, Files)> {
    ensure!(
        fs::symlink_metadata(directory)?.is_dir(),
        "Expected a regular build directory"
    );
    let manifest = read_regular(&directory.join("build.json"))?;
    let sd = directory.join("sd");
    let mut names = BTreeSet::new();
    collect_files(&sd, Path::new(""), &mut names)?;
    let mut files = Files::new();
    for name in names {
        let bytes = read_regular(&sd.join(&name))?;
        files.insert(name, bytes);
    }
    Ok((manifest, files))
}

pub fn load_build(directory: &Path) -> Result<VerifiedRuntime> {
    let (manifest, files) = read_build(directory)?;
    packaging::load_runtime(&manifest, &files)
}

/// Reads the release files of a site directory into `SiteContents` for
/// `release_index::Index::of_site`.
pub fn read_release_site(site: &Path) -> Result<SiteContents> {
    ensure!(
        fs::symlink_metadata(site)?.is_dir(),
        "Expected a site directory: {}",
        site.display()
    );
    let mut contents = SiteContents::default();
    let release_root = site.join("release");
    ensure!(release_root.is_dir(), "The site has no release/ directory");
    for folder in directory_entries(&release_root)? {
        if !fs::symlink_metadata(&folder)?.is_dir() {
            continue;
        }
        let id = basename(&folder)?;
        // The index file itself lives under release/; skip it.
        if id == "index.json" {
            continue;
        }
        let _ = camera::find(id)?;
        let statement = read_bounded(&folder.join(release::STATEMENT_ENTRY), 1024 * 1024)?;
        let signature = read_bounded(&folder.join(release::SIGNATURE_ENTRY), 1024)?;
        contents
            .releases
            .insert(id.to_owned(), (statement, signature));
    }

    let camera_ids: Vec<String> = contents.releases.keys().cloned().collect();
    for camera_id in camera_ids {
        let camera = camera::find(&camera_id)?;
        for target in camera.targets() {
            if contents.runtimes.contains_key(target) {
                continue;
            }
            let directory = site.join("runtime").join(target);
            let (manifest, files) = read_build(&directory)?;
            let runtime = packaging::load_runtime(&manifest, &files)?;
            contents.manifests.insert(target.to_owned(), manifest);
            contents.runtimes.insert(target.to_owned(), runtime);

            let effects_dir = site.join("effects").join(target);
            ensure!(effects_dir.is_dir(), "The site has no effects/{target}");
            let mut names = BTreeSet::new();
            collect_files(&effects_dir, Path::new(""), &mut names)?;
            let mut effects = Files::new();
            for name in names {
                effects.insert(name.clone(), read_regular(&effects_dir.join(&name))?);
            }
            contents.effects.insert(target.to_owned(), effects);
        }

        let guides_dir = site.join("guides").join(&camera_id);
        ensure!(guides_dir.is_dir(), "The site has no guides/{camera_id}");
        let mut guides = BTreeMap::new();
        for path in directory_entries(&guides_dir)? {
            let name = basename(&path)?;
            let Some(language) = name.strip_suffix(".pdf") else {
                continue;
            };
            guides.insert(
                language.to_owned(),
                read_bounded(&path, packaging::MAX_FILE_SIZE)?,
            );
        }
        contents.guides.insert(camera_id, guides);
    }

    let modules_dir = site.join("modules");
    if modules_dir.is_dir() {
        for path in directory_entries(&modules_dir)? {
            let name = basename(&path)?;
            ensure!(name.ends_with(".GRM"), "Unexpected module entry: {name}");
            contents.modules.insert(
                name.to_owned(),
                read_bounded(&path, packaging::MAX_FILE_SIZE)?,
            );
        }
    }
    Ok(contents)
}

/// Writes a verified build into a site directory as a new `runtime/<target>/`.
///
/// The copy holds only `build.json` and `sd/`, so it is itself a valid `--build` input.
/// `objects/` stays out because its generated header holds firmware bytes.
pub fn export_runtime(build: &Path, site: &Path) -> Result<(String, usize)> {
    let (manifest, files) = read_build(build)?;
    let runtime = packaging::load_runtime(&manifest, &files)?;
    let target = runtime.target().to_owned();
    // A site holds one runtime for each target, so a second target can join it.
    let output = site.join("runtime").join(&target);
    validate_output(&output)?;
    let mut tree = Files::from([("build.json".to_owned(), manifest)]);
    tree.extend(
        runtime
            .files()
            .iter()
            .map(|(name, bytes)| (format!("sd/{name}"), bytes.clone())),
    );
    let count = tree.len();
    write_directory(&output, &tree)?;
    Ok((target, count))
}

/// Refuses an output path that exists, a dangling symlink included, so a command
/// cannot write into earlier output. The ancestors of the path are not checked.
pub fn validate_output(output: &Path) -> Result<()> {
    ensure!(
        fs::symlink_metadata(output).is_err(),
        "Output already exists: {}",
        output.display()
    );
    Ok(())
}

fn create_parent(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

pub fn write_directory(path: &Path, files: &Files) -> Result<()> {
    create_parent(path)?;
    fs::create_dir(path)
        .with_context(|| format!("Cannot create new directory {}", path.display()))?;
    for (name, bytes) in files {
        write_new(&path.join(name), bytes)?;
    }
    Ok(())
}

pub fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    create_parent(path)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .with_context(|| format!("Cannot create new file {}", path.display()))?;
    if let Err(error) = file.write_all(bytes) {
        drop(file);
        let _ = fs::remove_file(path);
        bail!("Cannot write {}: {error}", path.display());
    }
    Ok(())
}

/// `write_new`, for a file that only its owner can read, such as a key.
pub fn write_secret(path: &Path, bytes: &[u8]) -> Result<()> {
    use std::os::unix::fs::OpenOptionsExt;
    create_parent(path)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .with_context(|| format!("Cannot create new file {}", path.display()))?;
    file.write_all(bytes)
        .with_context(|| format!("Cannot write {}", path.display()))
}
