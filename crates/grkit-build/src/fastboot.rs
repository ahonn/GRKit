//! The fast-boot files of a runtime build (`grkit_core::runtime::fastboot`):
//! for each firmware release of the build with RTOS calls in
//! `runtime/fastboot/targets/<camera>-<firmware>.h`, the installer of the
//! build compiled with `-DRTOS_NATIVE`, with `runtime/fastboot/backend.c`
//! and the head (`head.S`), linked by `fast.ld`.

use crate::{components::path, toolchain, Toolchain};
use anyhow::{ensure, Context, Result};
use grkit_core::{camera, runtime::FastRelease, Files};
use std::{fs, path::Path};

/// The firmware releases with RTOS calls in `runtime/fastboot/targets/`, and
/// the calls.
fn releases() -> Result<Vec<(FastRelease, &'static [u8])>> {
    crate::RUNTIME_SOURCES
        .iter()
        .filter_map(|(name, bytes)| {
            let name = name.strip_prefix("fastboot/targets/")?.strip_suffix(".h")?;
            Some((name, *bytes))
        })
        .map(|(name, bytes)| {
            let (id, firmware) = name
                .split_once('-')
                .with_context(|| format!("The RTOS calls {name} name no camera"))?;
            Ok((FastRelease::new(camera::find(id)?, firmware)?, bytes))
        })
        .collect()
}

/// The fast-boot files of the runtime build for `target`, by their path in
/// the package. `objects` holds the generated `module_payload.h` of the
/// build, and `runtime` the runtime sources.
pub fn files(tools: &Toolchain, objects: &Path, runtime: &Path, target: &str) -> Result<Files> {
    let releases: Vec<_> = releases()?
        .into_iter()
        .filter(|(release, _)| release.target() == target)
        .collect();
    let mut files = Files::new();
    if releases.is_empty() {
        return Ok(files);
    }
    let directory = objects.join("fastboot");
    fs::create_dir_all(&directory)?;
    // The address of the loader for head.S and the installer (fast.ld).
    let fast_base = format!("-DFAST_BASE={:#x}", grkit_core::runtime::FAST_BASE);
    // The flags of the production installer, with RTOS_NATIVE.
    let compile = |source: &Path, include: &Path, output: &Path| {
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
                "-DRTOS_NATIVE",
                fast_base.as_str(),
                "-I",
                path(include)?,
                "-c",
                path(source)?,
                "-o",
                path(output)?,
            ],
        )
    };
    let installer = directory.join("installer.o");
    compile(&runtime.join("installer.c"), objects, &installer)?;
    let sources = runtime.join("fastboot");
    for (release, calls) in releases {
        let name = release.name();
        let build = directory.join(name.trim_end_matches(".BIN"));
        fs::create_dir_all(&build)?;
        fs::write(build.join("target.h"), calls)?;
        fs::write(
            build.join("fast.h"),
            format!(
                "/* The path of this fast-boot file, which its head reads. */\n\
                 #define FAST_PATH {:?}\n\
                 /* The status word of the head in the script array. */\n\
                 #define FAST_STATUS_INDEX {}\n\
                 /* The detail words of a changed status, before the status word. */\n\
                 #define FAST_DETAIL_INDEX {}\n\
                 /* The status when INSTALL found that its data changed. */\n\
                 #define FAST_STATUS_DATA_CHANGED {}\n\
                 /* The plan header that SCAN gives the loader in the script array. */\n\
                 #define FAST_PLAN_INDEX {}\n\
                 #define FAST_PLAN_WORDS {}\n\
                 /* The RAM file of the whole plan. */\n\
                 #define FAST_PLAN_FILE {:?}\n\
                 /* The pack of the effects and the modules of a start, and its folders. */\n\
                 #define FAST_PACK_FILE {:?}\n\
                 #define FAST_PACK_FOLDERS {{{:?}, {:?}, {:?}}}\n",
                release.rtos_path(),
                grkit_core::runtime::FAST_STATUS_INDEX,
                grkit_core::runtime::FAST_DETAIL_INDEX,
                grkit_core::runtime::FAST_STATUS_DATA_CHANGED,
                grkit_core::runtime::FAST_PLAN_INDEX,
                grkit_core::runtime::FAST_PLAN_WORDS,
                grkit_core::card::LINUX_RAM_PLAN,
                format!("{}\\CACHE.PAK", grkit_core::card::DATA_FOLDER),
                format!("{}\\EFFECTS", grkit_core::card::DATA_FOLDER),
                format!("{}\\MODULES", grkit_core::card::RTOS_PACKAGE_FOLDER),
                format!("{}\\MODULES", grkit_core::card::DATA_FOLDER)
            ),
        )?;
        let (backend, head) = (build.join("backend.o"), build.join("head.o"));
        compile(&sources.join("backend.c"), &build, &backend)?;
        compile(&sources.join("head.S"), &build, &head)?;
        let output = build.join(&name);
        toolchain::run(
            &tools.lld,
            [
                "-m",
                "armelf_linux_eabi",
                "-static",
                "--build-id=none",
                "-T",
                path(&sources.join("fast.ld"))?,
                "--oformat=binary",
                path(&head)?,
                path(&installer)?,
                path(&backend)?,
                "-o",
                path(&output)?,
            ],
        )?;
        let bytes = fs::read(&output)?;
        grkit_core::runtime::check_fast_file(&release, &bytes)
            .with_context(|| format!("Build {name}"))?;
        ensure!(files.insert(release.path(), bytes).is_none());
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_rtos_call_file_names_a_release() {
        let names: Vec<_> = releases()
            .unwrap()
            .iter()
            .map(|(release, _)| release.name())
            .collect();
        assert_eq!(
            names,
            [
                "F588210.BIN",
                "F588220.BIN",
                "F588221.BIN",
                "F609160.BIN",
                "F609170.BIN",
                "F609171.BIN",
                "F636111.BIN",
                "F636120.BIN",
                "F636121.BIN"
            ]
        );
    }
}
