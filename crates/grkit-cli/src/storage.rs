//! Filesystem policy for explicit local inputs and newly created outputs.

use anyhow::{bail, ensure, Context, Result};
use grkit_core::{effects, packaging, runtime, Files};
use serde_json::Value;
use std::collections::BTreeSet;
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

pub fn read_effects(directory: &Path) -> Result<Files> {
    let mut files = Files::new();
    let mut ids = BTreeSet::new();
    for path in directory_entries(directory)? {
        let name = basename(&path)?;
        ensure!(
            name.to_ascii_uppercase().ends_with(".GRF"),
            "Unexpected compiled effect entry: {name}"
        );
        let bytes = read_regular(&path)?;
        let (effect, _) = effects::decode(&bytes)?;
        let id = effect["effect_id"]
            .as_str()
            .context("Expected an effect ID")?;
        let canonical = effects::filename(id)?;
        ensure!(
            name.to_ascii_uppercase() == canonical && !files.contains_key(&canonical),
            "Invalid or colliding effect filename: {name}"
        );
        ensure!(ids.insert(id.to_owned()), "Duplicate effect ID");
        files.insert(canonical, bytes);
        ensure!(
            files.len() <= runtime::MAX_COUNT,
            "This target supports at most {} effects",
            runtime::MAX_COUNT
        );
    }
    Ok(files)
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

pub fn load_build(directory: &Path) -> Result<(Files, Value)> {
    let (manifest, files) = read_build(directory)?;
    packaging::load_runtime(&manifest, &files)
}

/// Writes a verified build into a new site directory as `runtime/<target>/`.
///
/// The copy holds only `build.json` and `sd/`, so it is itself a valid `--build` input.
/// `objects/` stays out because its generated header holds firmware bytes.
pub fn export_runtime(build: &Path, site: &Path) -> Result<(String, usize)> {
    let (manifest, files) = read_build(build)?;
    let (runtime, parsed) = packaging::load_runtime(&manifest, &files)?;
    let target = parsed["target"]
        .as_str()
        .context("Expected a build target")?
        .to_owned();
    validate_output(site)?;
    let mut tree = Files::from([("build.json".to_owned(), manifest)]);
    tree.extend(
        runtime
            .into_iter()
            .map(|(name, bytes)| (format!("sd/{name}"), bytes)),
    );
    let count = tree.len();
    write_directory(&site.join("runtime").join(&target), &tree)?;
    Ok((target, count))
}

// Resolve existing ancestors too, so symlink aliases cannot conceal nested outputs.
pub fn validate_output(output: &Path) -> Result<()> {
    ensure!(
        fs::symlink_metadata(output).is_err(),
        "Output directory already exists: {}",
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
