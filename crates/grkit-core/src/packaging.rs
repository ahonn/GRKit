//! Deterministic SD package assembly and bounded, strict package verification.

use std::collections::BTreeSet;
use std::io::{Cursor, Read, Write};

use anyhow::{bail, ensure, Context, Result};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use zip::{write::SimpleFileOptions, CompressionMethod, DateTime, ZipArchive, ZipWriter};

use crate::{effects, runtime, Files};

pub const RUNTIME_PATHS: [&str; 5] = [
    "SCRIPT/STARTUP.TTL",
    "SCRIPT/PLUGINS/PLUGINS.TTL",
    "SCRIPT/PLUGINS/INSTALL.BIN",
    "SCRIPT/PLUGINS/SCAN.SH",
    "SCRIPT/PLUGINS/INSTALL.SH",
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

pub fn safe_path(name: &str) -> Result<&str> {
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

pub fn validate_runtime(files: &Files) -> Result<()> {
    ensure!(
        files.len() == RUNTIME_PATHS.len() && RUNTIME_PATHS.iter().all(|p| files.contains_key(*p)),
        "Expected the complete production runtime file set"
    );
    let binary = &files["SCRIPT/PLUGINS/INSTALL.BIN"];
    ensure!(
        binary.len() >= 52
            && binary[..6] == *b"\x7fELF\x01\x01"
            && binary[16..20] == *b"\x02\x00\x28\x00"
            && !binary.windows(8).any(|w| w == b"--select"),
        "Expected a production installer without diagnostic selection"
    );
    ensure!(
        *files == runtime::runtime_files(binary),
        "Runtime scripts differ from the production templates"
    );
    Ok(())
}

/// Checks a runtime build against its `build.json` and returns the runtime files and manifest.
///
/// `files` holds every file of the build's `sd/` tree, keyed by its path in that tree.
/// The file set must equal the manifest list, and each file must match its size and SHA-256.
pub fn load_runtime(manifest: &[u8], files: &Files) -> Result<(Files, Value)> {
    let manifest = read_manifest(manifest)?;
    ensure!(
        manifest["profile"] == "production"
            && manifest["target"] == "gr3-2.10"
            && manifest["startup_activation"] == "Off",
        "Expected a production GR III 2.10 build that starts in Off"
    );
    let mut runtime = Files::new();
    for item in manifest["files"]
        .as_array()
        .context("Expected a manifest file list")?
    {
        let name = item["path"].as_str().context("Expected a runtime path")?;
        ensure!(
            RUNTIME_PATHS.contains(&name) && !runtime.contains_key(name),
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
    validate_runtime(&runtime)?;
    Ok((runtime, manifest))
}

fn is_effect_path(name: &str) -> bool {
    let Some(stem) = name
        .strip_prefix("SD_ROOT/SCRIPT/PLUGINS/EFFECTS/")
        .and_then(|s| s.strip_suffix(".GRF"))
    else {
        return false;
    };
    (1..=8).contains(&stem.len())
        && stem
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
}

pub fn validate_contents(contents: &Files) -> Result<Vec<String>> {
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
    let activation = runtime::activation_files();
    let mut mandatory: BTreeSet<String> = RUNTIME_PATHS
        .iter()
        .map(|name| format!("SD_ROOT/{name}"))
        .collect();
    mandatory.extend(activation.keys().map(|name| format!("SD_ROOT/{name}")));
    mandatory.extend(["INSTALLATION.md", "SHA256SUMS"].map(String::from));
    ensure!(
        mandatory.iter().all(|name| contents.contains_key(name)),
        "Package is missing required files"
    );
    for (name, expected) in activation {
        ensure!(
            contents[&format!("SD_ROOT/{name}")] == expected,
            "Activation resources differ from the target"
        );
    }
    let runtime = RUNTIME_PATHS
        .iter()
        .map(|name| ((*name).into(), contents[&format!("SD_ROOT/{name}")].clone()))
        .collect();
    validate_runtime(&runtime)?;
    let mut ids = BTreeSet::new();
    for (name, blob) in contents
        .iter()
        .filter(|(name, _)| !mandatory.contains(*name))
    {
        if is_effect_path(name) {
            let (effect, _) = effects::decode(blob)?;
            let id = effect["effect_id"].as_str().context("Invalid effect ID")?;
            ensure!(
                name.rsplit('/').next() == Some(effects::filename(id)?.as_str()),
                "Profile filename does not match its stable ID"
            );
            ensure!(ids.insert(id.to_owned()), "Duplicate effect ID");
        } else {
            bail!("Package contains unexpected files: {name}");
        }
    }
    ensure!(
        ids.len() <= runtime::MAX_COUNT,
        "Target effect capacity exceeded"
    );
    Ok(ids.into_iter().collect())
}

fn checksum_list(contents: &Files) -> Vec<u8> {
    contents
        .iter()
        .filter(|(name, _)| name.as_str() != "SHA256SUMS")
        .map(|(name, blob)| format!("{}  {name}\n", sha256(blob)))
        .collect::<String>()
        .into_bytes()
}

pub fn assemble(runtime: &Files, build: &Value, effects: &Files) -> Result<Vec<u8>> {
    ensure!(
        build.get("profile") == Some(&json!("production"))
            && build.get("target") == Some(&json!("gr3-2.10"))
            && build.get("startup_activation") == Some(&json!("Off")),
        "Only production builds that start in Off can be packaged"
    );
    validate_runtime(runtime)?;
    let mut contents: Files = runtime
        .iter()
        .map(|(name, blob)| (format!("SD_ROOT/{name}"), blob.clone()))
        .collect();
    contents.extend(
        runtime::activation_files()
            .into_iter()
            .map(|(name, blob)| (format!("SD_ROOT/{name}"), blob)),
    );
    for (name, blob) in effects {
        contents.insert(
            format!("SD_ROOT/SCRIPT/PLUGINS/EFFECTS/{}", safe_path(name)?),
            blob.clone(),
        );
    }
    contents.insert(
        "INSTALLATION.md".into(),
        include_bytes!("../../../resources/INSTALLATION.md").to_vec(),
    );
    contents.insert("SHA256SUMS".into(), checksum_list(&contents));
    validate_contents(&contents)?;
    let mut archive = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, blob) in contents {
        let mode = if name.ends_with(".BIN") || name.ends_with(".SH") {
            0o755
        } else {
            0o644
        };
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Stored)
            .last_modified_time(
                DateTime::from_date_and_time(2026, 1, 1, 0, 0, 0).expect("fixed valid timestamp"),
            )
            .unix_permissions(mode);
        archive.start_file(name, options)?;
        archive.write_all(&blob)?;
    }
    let bytes = archive.finish()?.into_inner();
    verify_package(&bytes)?;
    Ok(bytes)
}

pub fn verify_package(bytes: &[u8]) -> Result<Value> {
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
        let name = safe_path(entry.name())?.to_owned();
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
    let ids = validate_contents(&contents)?;
    ensure!(
        contents["SHA256SUMS"] == checksum_list(&contents),
        "SHA256SUMS differs from package"
    );
    Ok(json!({"target": "gr3-2.10", "effect_ids": ids, "entries": contents.len()}))
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
