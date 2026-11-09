//! The runtime files of a package: the startup and loader scripts, the
//! package scripts, the core files, and the files that turn SD scripts on or
//! off.

use super::fastboot::{fast_files, install_file};
use super::loader::{core_path, loader_with, resident};
use super::ICON_STRIDE;
use crate::camera::Camera;
use crate::card::{
    INSTALL_FILE, LINUX_DATA_DIRECTORY, LINUX_DIRECTORY, LOADER_FILE, PACKAGE_FOLDER,
    RESIDENT_FILE, SCAN_FILE,
};
use crate::Files;

/// The files that enable SD scripts when the camera starts with MENU held.
pub fn activation_files(camera: &Camera) -> Files {
    modeset_files(camera, b"[SCRIPT_EN_MODE EN]\r\n")
}

/// The files that turn SD scripts off again, in the same way. The firmware
/// has this command next to the one that enables them.
pub fn deactivation_files(camera: &Camera) -> Files {
    modeset_files(camera, b"[SCRIPT_EN_MODE DIS]\r\n")
}

fn modeset_files(camera: &Camera, command: &[u8]) -> Files {
    Files::from([
        (camera.modeset.into(), command.to_vec()),
        (
            "DEVELOP.MOD".into(),
            vec![7, 1, 44, 31, 16, 3, 30, 22, 5, 45],
        ),
    ])
}

pub fn runtime_files(binary: &[u8]) -> Files {
    runtime_files_for(binary, ICON_STRIDE)
}

/// The runtime files of a target with `icon_stride`-byte icon slots.
pub fn runtime_files_for(binary: &[u8], icon_stride: usize) -> Files {
    package_runtime_files(&[binary], icon_stride)
}

/// `runtime_files_for`, with the fast-boot files of `fast`
/// (`package_runtime_files_with`).
pub fn runtime_files_with(binary: &[u8], fast: &Files, icon_stride: usize) -> Files {
    package_runtime_files_with(&[binary], fast, icon_stride)
}

/// The exit statuses of a core file that the package scripts test
/// (`runtime/install_status.h`): the core does not relocate to the running
/// build, or (`--known`) does not name it.
pub const STATUS_RELOCATION: u32 = 47;
pub const STATUS_OTHER_BUILD: u32 = 48;

/// The boot log in the data folder: one line for each step of a start, with
/// the Linux uptime in seconds. A person sends it with a report of a start
/// that failed. The scripts count lines with `grep -c`, because the camera's
/// Linux has no `wc`.
pub const BOOT_LOG: &str = "BOOT.TXT";
/// The most lines of the boot log, about 20 starts. A start that finds more
/// keeps the last three quarters.
const BOOT_LOG_LINES: u32 = 400;
/// The core file that installed at the last start, in the data folder.
const CORE_CHOICE: &str = "CORE.TXT";
/// The marker in RAM of the first SCAN.SH of a power-on.
const BOOT_MARK: &str = "/tmp/grkit-boot";

/// The core file of a runtime build: the core of one firmware build and its
/// installer, which SCAN.SH and INSTALL.SH run.
pub use crate::card::CORE_FILE;

/// The most core files that a package holds: `CORE1.BIN` to `CORE9.BIN`, so
/// each name stays 8.3.
pub const MAX_CORE_FILES: usize = 9;

/// The path of core file `index` of `count` in a package: `CORE.BIN` for one
/// build, else `CORE1.BIN` and on, in release order.
pub fn core_path_of(index: usize, count: usize) -> String {
    if count == 1 {
        CORE_FILE.to_owned()
    } else {
        format!("{PACKAGE_FOLDER}/CORE{}.BIN", index + 1)
    }
}

/// The runtime files of a package with the core file of each runtime build of
/// a camera, in release order, for `icon_stride`-byte icon slots. The builds of
/// a camera have the same scripts and plan, so the first core file scans.
///
/// With one core file, `INSTALL.SH` runs it with `--install`. With more, it
/// runs each with `--known` first, which reads one word to find the build. If
/// no core file names the build, it tries each one with `--try`, which runs
/// the relocation, and the last one with `--install`, which writes
/// `INSTALL.ERR` if it also fails. A known build thus installs without a relocation of
/// another build, and an unknown build still gets each runtime.
///
/// After Linux deletes a file in the folder, the camera does not find the next
/// new name that it looks up there, and it keeps that miss. So `SCAN.SH`, the
/// first script of a power-on, looks up each core file before it scans.
///
/// A core that installed with `--known` goes into `CORE_CHOICE`, and the next
/// start tries it first. A core of another build takes seconds to rule out on
/// GR IV, and the camera keeps its build until a firmware update. The record
/// names only a core of the package, and `--known` checks the build again, so
/// a stale record costs only that check.
///
/// Both scripts add their start and the status of the installer to the boot
/// log (`BOOT_LOG`).
pub fn package_runtime_files(binaries: &[&[u8]], icon_stride: usize) -> Files {
    package_runtime_files_with(binaries, &Files::new(), icon_stride)
}

/// `package_runtime_files`, with the fast-boot files of `fast` by their path
/// (`runtime::fastboot`). The loader runs each of them on its firmware
/// release, and the install file of each comes with it. `fast` holds no
/// other file, and each passed `runtime::check_fast_file`.
pub fn package_runtime_files_with(binaries: &[&[u8]], fast: &Files, icon_stride: usize) -> Files {
    let releases: Vec<_> = fast_files(fast)
        .into_iter()
        .map(|(release, _)| release)
        .collect();
    assert_eq!(releases.len(), fast.len(), "only fast-boot files");
    let count = binaries.len();
    assert!((1..=MAX_CORE_FILES).contains(&count));
    let target = core_path("LOADER.TTL");
    let startup = format!("filesearch '{target}'\r\nif result = 0 goto done\r\ninclude '{target}'\r\n:done\r\nend\r\n");
    let name = |index: usize| {
        core_path_of(index, count)
            .rsplit('/')
            .next()
            .unwrap()
            .to_owned()
    };
    // Each script body sets `status` to the exit status of the installer.
    let log = format!("{LINUX_DATA_DIRECTORY}/{BOOT_LOG}");
    let wrapper = |body: String, mode: &str, first: &str| {
        format!(
            "{first}read t _ </proc/uptime; echo \"$t {mode}-start\" >>{log}\n\
             {body}read t _ </proc/uptime; echo \"$t {mode} $status\" >>{log}\n\
             echo GR_MODULE_{}_EXIT_$status\nexit 0\n",
            mode.to_uppercase()
        )
        .into_bytes()
    };
    // The first SCAN.SH of a power-on starts the boot log of the start, with
    // only the last lines of the earlier starts. A marker in RAM tells the
    // first run, because the loader repeats SCAN.SH until it finds a plan.
    let start = format!(
        "mkdir -p {LINUX_DATA_DIRECTORY}\n\
         if [ ! -e {BOOT_MARK} ]; then\n\
         \x20 : >{BOOT_MARK}\n\
         \x20 n=$(grep -c '' {log} 2>/dev/null)\n\
         \x20 [ \"${{n:-0}}\" -gt {lines} ] && tail -n {keep} {log} >{BOOT_MARK} && cat {BOOT_MARK} >{log}\n\
         \x20 read t _ </proc/uptime; echo \"$t load\" >>{log}\n\
         fi\n",
        lines = BOOT_LOG_LINES,
        keep = BOOT_LOG_LINES * 3 / 4,
    );
    let single = |mode: &str| format!("{LINUX_DIRECTORY}/{} --{mode}\nstatus=$?\n", name(0));
    let names = |range: std::ops::Range<usize>| range.map(name).collect::<Vec<_>>().join(" ");
    let (scan, install) = match count {
        1 => (single("scan"), single("install")),
        count => (
            format!(
                "for core in {}; do [ -e {LINUX_DIRECTORY}/$core ]; done\n{}",
                names(1..count),
                single("scan")
            ),
            format!(
                "k={LINUX_DATA_DIRECTORY}/{CORE_CHOICE}\n\
                 last=; [ -e $k ] && read last _ <$k\n\
                 status={other}; used=\n\
                 case \" {all} \" in *\" $last \"*) {LINUX_DIRECTORY}/$last --known; status=$?; used=$last ;; esac\n\
                 if [ $status = {other} ]; then\n\
                 \x20 for core in {all}; do\n\
                 \x20   [ \"$core\" = \"$last\" ] && continue\n\
                 \x20   {LINUX_DIRECTORY}/$core --known\n\
                 \x20   status=$?; used=$core\n\
                 \x20   case $status in {other}) ;; *) break ;; esac\n\
                 \x20 done\n\
                 fi\n\
                 [ $status = 0 ] && [ \"$used\" != \"$last\" ] && echo $used >$k\n\
                 case $status in {other})\n\
                 \x20 for core in {tried}; do\n\
                 \x20   {LINUX_DIRECTORY}/$core --try\n\
                 \x20   status=$?\n\
                 \x20   case $status in {relocation}) ;; *) break ;; esac\n\
                 \x20 done\n\
                 \x20 case $status in {relocation}) {LINUX_DIRECTORY}/{last} --install; status=$? ;; esac\n\
                 \x20 ;;\n\
                 esac\n",
                all = names(0..count),
                tried = names(0..count - 1),
                last = name(count - 1),
                other = STATUS_OTHER_BUILD,
                relocation = STATUS_RELOCATION,
            ),
        ),
    };
    let mut files = Files::from([
        ("SCRIPT/STARTUP.TTL".into(), startup.into_bytes()),
        (LOADER_FILE.into(), loader_with(icon_stride, &releases)),
        (RESIDENT_FILE.into(), resident()),
        (SCAN_FILE.into(), wrapper(scan, "scan", &start)),
        (INSTALL_FILE.into(), wrapper(install, "install", "")),
    ]);
    for (index, binary) in binaries.iter().enumerate() {
        files.insert(core_path_of(index, count), binary.to_vec());
    }
    for (release, bytes) in fast_files(fast) {
        files.insert(release.path(), bytes.clone());
        let install = install_file(bytes).expect("a checked fast-boot file");
        files.insert(release.install_path(), install);
    }
    files
}
