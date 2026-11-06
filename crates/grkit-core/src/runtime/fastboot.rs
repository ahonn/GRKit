//! The fast-boot files of a package (`runtime/fastboot/`).
//!
//! A firmware build with the fast-boot loader has two script commands that
//! read and run code from the card:
//!
//! - `SYS_buildinfo 0` writes the probe words (`FAST_PROBE`) into the first
//!   script array: a magic word, the interface version, the project number of
//!   the camera, and the firmware version.
//! - `SYS_memovr PATH 0 call` reads the start of `PATH` into the first script
//!   array and calls it there.
//!
//! The official firmware has other commands under these names. Its
//! `SYS_buildinfo` ignores the argument, so the probe words stay 0. The loader
//! sends `SYS_memovr` only after the probe found the fast-boot loader.
//!
//! A fast-boot file (`F<project><version>.BIN`, `FastRelease::path`) is the
//! installer of a runtime build, built for the RTOS side of one firmware
//! release. Its head (`runtime/fastboot/head.S`) loads the file to the top
//! of the RTOS code region and runs SCAN. `FRUN.BIN` (`FAST_RUN`) runs
//! INSTALL. So on a firmware build with the fast-boot loader, GRKit installs
//! before Linux mounts the card, and the package is the same for each build.

use anyhow::{ensure, Context, Result};

use crate::camera::{Camera, CAMERAS};
use crate::card::{PACKAGE_FOLDER, RTOS_PACKAGE_FOLDER};
use crate::Files;

/// The probe words of a firmware build with the fast-boot loader, in the
/// first script array: "SDNC", then the version of the interface.
pub const FAST_PROBE_MAGIC: u32 = 0x434e_4453;
pub const FAST_INTERFACE: u32 = 1;

/// The address of a fast-boot file in the RTOS: its head, then at
/// `FAST_BASE + FAST_HEAD_SIZE` the scan vector, then the install vector
/// (`runtime/fastboot/fast.ld`).
pub const FAST_BASE: u32 = 0x54c0_0000;
/// The bytes of the head, which the loader reads into its first script array.
pub const FAST_HEAD_SIZE: usize = 1024;
/// The most bytes of a fast-boot file: the top of the RTOS code region.
pub const FAST_MAX_SIZE: usize = 0x40_0000;

/// The name of the install file in the package folder.
pub const FAST_RUN_NAME: &str = "FRUN.BIN";
/// The install file: `ldr ip, [pc]`, `bx ip`, then the address of the install
/// vector. The loader reads it into its first script array and calls it.
pub const FAST_RUN: [u8; 12] = {
    let words = [
        0xe59f_c000_u32,
        0xe12f_ff1c,
        FAST_BASE + FAST_HEAD_SIZE as u32 + 4,
    ];
    let mut bytes = [0; 12];
    let mut i = 0;
    while i < 12 {
        bytes[i] = (words[i / 4] >> (i % 4 * 8)) as u8;
        i += 1;
    }
    bytes
};

/// The path of the install file in a package.
pub fn fast_run_path() -> String {
    format!("{PACKAGE_FOLDER}/{FAST_RUN_NAME}")
}

/// A firmware release that can have a fast-boot file: a camera and one of
/// its firmware versions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FastRelease {
    pub camera: &'static Camera,
    pub firmware: &'static str,
}

impl FastRelease {
    /// The release of `camera` with `firmware`.
    pub fn new(camera: &'static Camera, firmware: &str) -> Result<Self> {
        let firmware = camera
            .versions()
            .into_iter()
            .find(|version| *version == firmware)
            .with_context(|| format!("{} has no firmware {firmware}", camera.name))?;
        let release = Self { camera, firmware };
        ensure!(
            release.version().is_some(),
            "The firmware version {firmware} has no fast-boot number"
        );
        Ok(release)
    }

    /// The firmware version as the probe gives it: 2.10 is 210.
    fn version(&self) -> Option<u32> {
        let (major, minor) = self.firmware.split_once('.')?;
        let digits = format!("{major}{minor}");
        (digits.len() == 3 && digits.bytes().all(|b| b.is_ascii_digit()))
            .then(|| digits.parse().ok())
            .flatten()
    }

    /// The number of the release in the loader: the project number times
    /// 1000, plus the firmware version.
    pub fn number(&self) -> u32 {
        self.camera.project() * 1000 + self.version().unwrap_or_default()
    }

    /// The name of the fast-boot file, such as `F588210.BIN`.
    pub fn name(&self) -> String {
        format!("F{}.BIN", self.number())
    }

    /// The path of the fast-boot file in a package.
    pub fn path(&self) -> String {
        format!("{PACKAGE_FOLDER}/{}", self.name())
    }

    /// The path of the fast-boot file in the RTOS, which its head reads.
    pub fn rtos_path(&self) -> String {
        format!("{RTOS_PACKAGE_FOLDER}\\{}", self.name())
    }

    /// The runtime build of the release: that of the first release of the
    /// camera with the version. A later release with the same version is a
    /// factory build, which no fast-boot firmware has.
    pub fn target(&self) -> &'static str {
        self.camera
            .releases
            .iter()
            .find(|release| release.firmware == self.firmware)
            .map(|release| release.target)
            .expect("a release of the camera")
    }

    /// The release of a fast-boot file at `path` in a package.
    pub fn of_path(path: &str) -> Option<Self> {
        CAMERAS.into_iter().find_map(|camera| {
            camera.versions().into_iter().find_map(|firmware| {
                let release = Self::new(camera, firmware).ok()?;
                (release.path() == path).then_some(release)
            })
        })
    }
}

/// The fast-boot files of `files`, by release, in path order.
pub fn fast_files(files: &Files) -> Vec<(FastRelease, &Vec<u8>)> {
    files
        .iter()
        .filter_map(|(path, bytes)| Some((FastRelease::of_path(path)?, bytes)))
        .collect()
}

/// Checks a fast-boot file of `release`: it has the size of a head and a
/// blob, its head names its own path, and it names its runtime build.
pub fn check_fast_file(release: &FastRelease, bytes: &[u8]) -> Result<()> {
    let name = release.name();
    ensure!(
        bytes.len() > FAST_HEAD_SIZE && bytes.len() <= FAST_MAX_SIZE,
        "{name} does not have the size of a fast-boot file"
    );
    let path = release.rtos_path();
    ensure!(
        bytes[..FAST_HEAD_SIZE]
            .windows(path.len())
            .any(|part| part == path.as_bytes()),
        "The head of {name} does not read {path}"
    );
    let marker = format!("{}{}\0", crate::packaging::TARGET_MARKER, release.target());
    ensure!(
        bytes
            .windows(marker.len())
            .any(|part| part == marker.as_bytes()),
        "{name} is not the {} runtime",
        release.target()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::{GR3, GR3X, GR4};

    #[test]
    fn fast_releases_have_short_names_and_their_runtime_build() {
        let cases = [
            (&GR3, "2.10", "F588210.BIN", "gr3-2.10"),
            (&GR3, "2.20", "F588220.BIN", "gr3-2.10"),
            (&GR3X, "1.60", "F609160.BIN", "gr3-2.10"),
            (&GR3X, "1.70", "F609170.BIN", "gr3-2.10"),
            (&GR4, "1.11", "F636111.BIN", "gr4-1.11"),
            // Not the factory build of 1.20.
            (&GR4, "1.20", "F636120.BIN", "gr4-1.20"),
        ];
        for (camera, firmware, name, target) in cases {
            let release = FastRelease::new(camera, firmware).unwrap();
            assert_eq!(release.name(), name);
            assert_eq!(release.target(), target);
            assert_eq!(release.path(), format!("SCRIPT/GRKIT/{name}"));
            assert_eq!(release.rtos_path(), format!("C:\\SCRIPT\\GRKIT\\{name}"));
            assert_eq!(FastRelease::of_path(&release.path()), Some(release));
        }
        assert!(FastRelease::new(&GR3, "1.11").is_err());
        assert_eq!(FastRelease::of_path("SCRIPT/GRKIT/F588230.BIN"), None);
        assert_eq!(FastRelease::of_path(&fast_run_path()), None);
    }

    #[test]
    fn fast_run_calls_the_install_vector() {
        let words: Vec<u32> = FAST_RUN
            .chunks(4)
            .map(|word| u32::from_le_bytes(word.try_into().unwrap()))
            .collect();
        assert_eq!(words, [0xe59fc000, 0xe12fff1c, 0x54c00404]);
        assert_eq!(fast_run_path(), "SCRIPT/GRKIT/FRUN.BIN");
    }

    #[test]
    fn fast_files_need_their_path_and_runtime_build() {
        let release = FastRelease::new(&GR3, "2.20").unwrap();
        let file = crate::testing::fast_file(&release);
        check_fast_file(&release, &file).unwrap();
        let other = FastRelease::new(&GR4, "1.11").unwrap();
        assert!(check_fast_file(&other, &file).is_err());
        assert!(check_fast_file(&release, &file[..FAST_HEAD_SIZE]).is_err());
        let mut moved = file.clone();
        moved.drain(..FAST_HEAD_SIZE);
        assert!(check_fast_file(&release, &moved).is_err());
    }
}
