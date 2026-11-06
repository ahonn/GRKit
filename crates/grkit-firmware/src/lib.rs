//! The fast-boot firmware: an official firmware update file of GR III,
//! GR IIIx, or GR IV with three changes in the RTOS (docs/fastboot.md).
//!
//! 1. The sections of the RTOS code region become writable.
//! 2. `SYS_buildinfo INDEX` writes the probe words into a script array
//!    (`fastboot/patch/probe.S`).
//! 3. `SYS_memovr PATH INDEX [call]` reads a card file into a script array
//!    and can call it (`fastboot/patch/stub.S`).
//!
//! `fastboot/releases/` has the changes of each release. A person patches
//! the official file that they downloaded, or the official ZIP that holds
//! it. This crate keeps no firmware: it refuses a file whose SHA-256 is not
//! that of a supported official file, and it checks the result against the
//! SHA-256 of the release.

pub mod update;

use anyhow::{bail, ensure, Context, Result};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
    io::{Cursor, Read},
};

/// The probe words that `SYS_buildinfo` writes: "SDNC", then the version of
/// the interface.
pub const MAGIC: u32 = 0x434e_4453;
pub const INTERFACE: u32 = 1;

/// The RTOS link address.
const RTOS_BASE: u32 = 0x5300_0000;
/// The L1 translation table in the RTOS image, and its entries for the
/// 1 MB sections of the code region, 0x53000000 to 0x55000000.
const L1_TABLE: usize = 0x4000;
const CODE_SECTIONS: std::ops::Range<usize> = 0x530..0x550;
/// The low 20 bits of each of those entries in the official image: a
/// section, read-only (APX set). The change clears APX and sets AP[0], so
/// the section is writable.
const SECTION_ATTRIBUTES: u32 = 0x1c826;
const APX: u32 = 0x8000;
const AP0: u32 = 0x0400;

/// The release tables, by name (`fastboot/releases/<name>.json`).
const TABLES: [(&str, &str); 6] = [
    (
        "gr3-2.10",
        include_str!("../../../fastboot/releases/gr3-2.10.json"),
    ),
    (
        "gr3-2.20",
        include_str!("../../../fastboot/releases/gr3-2.20.json"),
    ),
    (
        "gr3x-1.60",
        include_str!("../../../fastboot/releases/gr3x-1.60.json"),
    ),
    (
        "gr3x-1.70",
        include_str!("../../../fastboot/releases/gr3x-1.70.json"),
    ),
    (
        "gr4-1.11",
        include_str!("../../../fastboot/releases/gr4-1.11.json"),
    ),
    (
        "gr4-1.20",
        include_str!("../../../fastboot/releases/gr4-1.20.json"),
    ),
];

/// A file by its size and SHA-256.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Digest256 {
    #[serde(default)]
    pub name: Option<String>,
    pub size: usize,
    pub sha256: String,
}

impl Digest256 {
    fn matches(&self, bytes: &[u8]) -> bool {
        self.size == bytes.len() && self.sha256 == sha256(bytes)
    }
}

/// A shell handler of the official RTOS that the release replaces with
/// `code`, which `fastboot/patch/` assembles with the `calls` of the release.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Handler {
    pub command: String,
    pub address: String,
    /// The SHA-256 of the official bytes that `code` replaces.
    pub before_sha256: String,
    pub code: String,
}

/// The changes of one firmware release (`fastboot/releases/`).
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Release {
    /// The camera ID of GRKit, such as `gr3`.
    pub camera: String,
    pub firmware: String,
    /// The project number that the probe gives, such as 588.
    pub project: u32,
    /// The official update file.
    pub update: Digest256,
    /// The RTOS section of its decoded payload.
    pub rtos: Digest256,
    /// The firmware functions that the handlers call, by their name in
    /// `fastboot/patch/`.
    pub calls: BTreeMap<String, String>,
    pub handlers: Vec<Handler>,
    /// The update file with the changes.
    pub result: Digest256,
}

impl Release {
    /// The name of the release, such as `gr3-2.10`.
    pub fn name(&self) -> String {
        format!("{}-{}", self.camera, self.firmware)
    }
}

pub fn sha256(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// The releases of `fastboot/releases/`.
pub fn releases() -> Vec<Release> {
    TABLES
        .iter()
        .map(|(name, text)| {
            let release: Release = serde_json::from_str(text)
                .unwrap_or_else(|error| panic!("fastboot/releases/{name}.json: {error}"));
            assert_eq!(release.name(), *name, "the name of {name}.json");
            release
        })
        .collect()
}

/// The release of an official update file.
pub fn release_of(update: &[u8]) -> Result<Release> {
    releases()
        .into_iter()
        .find(|release| release.update.matches(update))
        .with_context(|| {
            format!(
                "Not an official update file of a supported release: {}",
                releases()
                    .iter()
                    .map(Release::name)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
}

/// The largest update file that `official_update` takes from a ZIP. The
/// official update files have about 40 MB.
pub const MAX_UPDATE_SIZE: u64 = 64 * 1024 * 1024;

/// The update file of `bytes`: the update file in an official ZIP, by the
/// name of a release update file in any folder, or else `bytes` itself.
/// `release_of` checks the file.
pub fn official_update(bytes: &[u8]) -> Result<Cow<'_, [u8]>> {
    if !bytes.starts_with(b"PK\x03\x04") {
        return Ok(Cow::Borrowed(bytes));
    }
    let names: BTreeSet<String> = releases()
        .into_iter()
        .filter_map(|release| release.update.name)
        .map(|name| name.to_ascii_lowercase())
        .collect();
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).context("Not a ZIP file")?;
    // A ZIP made on macOS can also hold `__MACOSX/._<name>`, which has
    // another name.
    let index = (0..archive.len())
        .find(|&index| {
            archive.by_index_raw(index).is_ok_and(|entry| {
                entry.name().is_ok_and(|name| {
                    names.contains(&name.rsplit('/').next().unwrap_or("").to_ascii_lowercase())
                })
            })
        })
        .context("The ZIP file has no firmware update file of a supported camera")?;
    let entry = archive.by_index(index)?;
    ensure!(
        entry.size() <= MAX_UPDATE_SIZE,
        "The update file in the ZIP file is too large"
    );
    let mut file = Vec::with_capacity(entry.size() as usize);
    // The size in the ZIP file can be wrong: the read stops past the limit.
    entry.take(MAX_UPDATE_SIZE + 1).read_to_end(&mut file)?;
    if file.len() as u64 > MAX_UPDATE_SIZE {
        bail!("The update file in the ZIP file is too large");
    }
    Ok(Cow::Owned(file))
}

/// A patched update file.
#[derive(Debug)]
pub struct Patched {
    pub release: Release,
    pub bytes: Vec<u8>,
}

/// The fast-boot update file of an official update file, or of the official
/// ZIP that holds it.
pub fn patch(file: &[u8]) -> Result<Patched> {
    let update = official_update(file)?;
    let release = release_of(&update)?;
    patch_with(&update, release)
}

/// The fast-boot update file of `update`, the official update file of
/// `release` (`release_of`).
pub fn patch_with(update: &[u8], release: Release) -> Result<Patched> {
    let bytes = patch_release(update, &release)?;
    // The message gives the size and SHA-256 of the new file, which a new
    // table records as its result.
    ensure!(
        release.result.matches(&bytes),
        "The patched file of {} has {} bytes and SHA-256 {}, not the result of its table",
        release.name(),
        bytes.len(),
        sha256(&bytes)
    );
    Ok(Patched { release, bytes })
}

/// `update` with the changes of `release`, before the check of the result.
fn patch_release(update: &[u8], release: &Release) -> Result<Vec<u8>> {
    let file = update::Update::read(update)?;
    let section = file.section("RTOS")?;
    let mut target = file.decoded.clone();
    let rtos = &mut target[section];
    ensure!(
        release.rtos.matches(rtos),
        "The RTOS of the update file is not that of {}",
        release.name()
    );
    for index in CODE_SECTIONS {
        let at = L1_TABLE + index * 4;
        let entry = u32::from_le_bytes(rtos[at..at + 4].try_into().unwrap());
        ensure!(
            entry & 0xfffff == SECTION_ATTRIBUTES,
            "The code region of the RTOS has another layout"
        );
        rtos[at..at + 4].copy_from_slice(&((entry & !APX) | AP0).to_le_bytes());
    }
    for handler in &release.handlers {
        let address = u32::from_str_radix(handler.address.trim_start_matches("0x"), 16)
            .with_context(|| format!("The address of {}", handler.command))?;
        let code = hex::decode(&handler.code)?;
        let start = address
            .checked_sub(RTOS_BASE)
            .context("A handler before the RTOS")? as usize;
        let place = rtos
            .get_mut(start..start + code.len())
            .context("A handler past the RTOS")?;
        ensure!(
            sha256(place) == handler.before_sha256,
            "The RTOS has another {} handler",
            handler.command
        );
        place.copy_from_slice(&code);
    }
    file.encode(&target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_release_table_is_complete() {
        let names: Vec<_> = std::fs::read_dir(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fastboot/releases"
        ))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
        assert_eq!(names.len(), TABLES.len(), "{names:?}");
        for release in releases() {
            assert!(names.contains(&format!("{}.json", release.name())));
            let commands: Vec<_> = release
                .handlers
                .iter()
                .map(|h| h.command.as_str())
                .collect();
            assert_eq!(commands, ["SYS_memovr", "SYS_buildinfo"]);
            for key in [
                "strtoul",
                "ttl_table",
                "bf_stat",
                "bf_open",
                "bf_read",
                "bf_close",
            ] {
                assert!(release.calls.contains_key(key), "{key}");
            }
            assert!([588, 609, 636].contains(&release.project));
        }
    }

    /// A release for the sample of `update::tests`: its RTOS has the L1
    /// entries of the code region and two handlers.
    fn sample() -> (Vec<u8>, Release) {
        let mut rtos = vec![0u8; 0x7200];
        for index in CODE_SECTIONS {
            let entry = ((index as u32) << 20) | SECTION_ATTRIBUTES;
            rtos[L1_TABLE + index * 4..][..4].copy_from_slice(&entry.to_le_bytes());
        }
        rtos[0x7000..0x7008].copy_from_slice(b"handler1");
        rtos[0x7100..0x7108].copy_from_slice(b"handler2");
        let file = update::tests::sample(&rtos);
        let handler = |command: &str, address: u32, before: &[u8], code: &str| Handler {
            command: command.into(),
            address: format!("{address:#x}"),
            before_sha256: sha256(before),
            code: code.into(),
        };
        let release = Release {
            camera: "gr3".into(),
            firmware: "2.10".into(),
            project: 588,
            update: Digest256 {
                name: None,
                size: file.len(),
                sha256: sha256(&file),
            },
            rtos: Digest256 {
                name: None,
                size: rtos.len(),
                sha256: sha256(&rtos),
            },
            calls: BTreeMap::new(),
            handlers: vec![
                handler(
                    "SYS_memovr",
                    RTOS_BASE + 0x7000,
                    b"handler1",
                    "0102030405060708",
                ),
                handler("SYS_buildinfo", RTOS_BASE + 0x7100, b"ha", "1112"),
            ],
            result: Digest256 {
                name: None,
                size: 0,
                sha256: String::new(),
            },
        };
        (file, release)
    }

    #[test]
    fn a_patch_changes_the_code_region_and_the_handlers_only() {
        let (file, release) = sample();
        let patched = patch_release(&file, &release).unwrap();
        let before = update::Update::read(&file).unwrap();
        let after = update::Update::read(&patched).unwrap();
        let section = after.section("RTOS").unwrap();
        let rtos = &after.decoded[section.clone()];
        for index in CODE_SECTIONS {
            let entry = u32::from_le_bytes(rtos[L1_TABLE + index * 4..][..4].try_into().unwrap());
            assert_eq!(entry, ((index as u32) << 20) | 0x14c26);
        }
        assert_eq!(&rtos[0x7000..0x7008], [1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(&rtos[0x7100..0x7108], b"\x11\x12ndler2");
        let changed: Vec<usize> = (0..before.decoded.len())
            .filter(|&i| before.decoded[i] != after.decoded[i])
            .collect();
        let end = before.decoded.len() - 4;
        assert!(
            changed.iter().all(|i| section.contains(i) || *i >= end),
            "{changed:?}"
        );
        assert_eq!(update::sum32(&patched), 0);
    }

    fn zip(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (name, bytes) in files {
            zip.start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            std::io::Write::write_all(&mut zip, bytes).unwrap();
        }
        zip.finish().unwrap().into_inner()
    }

    #[test]
    fn the_update_file_comes_from_an_official_zip() {
        let bytes = zip(&[
            ("__MACOSX/gr3_v220/._fwdc239b.bin", b"resource fork"),
            ("gr3_v220/readme.txt", b"readme"),
            ("gr3_v220/FWDC239B.BIN", b"update"),
        ]);
        assert_eq!(&*official_update(&bytes).unwrap(), b"update");
        assert_eq!(&*official_update(b"update").unwrap(), b"update");
        let other = zip(&[("fwdc000b.bin", b"update")]);
        assert!(official_update(&other).is_err());
        // The file of the ZIP is still checked.
        let error = patch(&bytes).unwrap_err().to_string();
        assert!(error.contains("Not an official update file"), "{error}");
    }

    #[test]
    fn a_patch_refuses_other_files() {
        let (file, mut release) = sample();
        assert!(release_of(&file).is_err());
        release.handlers[0].before_sha256 = sha256(b"another");
        assert!(patch_release(&file, &release).is_err());
        let (file, mut release) = sample();
        release.rtos.sha256 = sha256(b"another");
        assert!(patch_release(&file, &release).is_err());
    }
}
