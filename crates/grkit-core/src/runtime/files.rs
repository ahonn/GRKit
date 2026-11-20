//! The runtime files of a package: the startup and loader scripts, the
//! package scripts, the core files, and the files that turn SD scripts on or
//! off.

use super::fastboot::{fast_files, install_file};
use super::loader::{core_path, loader_with, resident};
use super::ICON_STRIDE;
use crate::camera::Camera;
use crate::card::{
    INSTALL_FILE, LINUX_DATA_DIRECTORY, LINUX_DIRECTORY, LOADER_FILE, PACKAGE_FOLDER,
    RESIDENT_FILE, SCAN_FILE, SERVE_FILE,
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
pub const CORE_CHOICE: &str = "CORE.TXT";
/// The marker in RAM of the first SCAN.SH of a power-on.
const BOOT_MARK: &str = "/tmp/grkit-boot";
/// The marker in RAM of the service (`SERVE_FILE`) while SCAN.SH keeps it
/// running: the process ID of the service. SCAN.SH removes it as it stops
/// starting the service.
pub const SERVE_MARK: &str = "/tmp/grkit-serve";
/// SCAN.SH starts a service that ended without a stop again, after a pause of
/// 1 s, at most this many times for each start request.
const SERVE_RESTARTS: u32 = 1;
/// The argument of the copy of the service that runs from RAM, which a stop
/// checks, so that a process ID that another program took is left alone.
pub const SERVE_FROM_RAM: &str = "--from-ram";
/// The argument with which the service only marks itself as stopped in its
/// mailbox and ends. A stop runs it when no service runs: a service that
/// failed leaves its mark, so that the resident loader does not start it
/// again until the access point goes off.
pub const SERVE_MARK_STOPPED: &str = "--mark-stopped";
/// The arguments of SCAN.SH that only start or stop the service, with which
/// the resident loader follows the Wi-Fi access point (`runtime::loader`).
pub const SCAN_SERVE_ARGUMENT: &str = "serve";
pub const SCAN_STOP_ARGUMENT: &str = "stop";

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

/// The core file that `scan`, the SCAN.SH of a package, runs to scan: the
/// first core file of the package (`core_path_of(0, count)`). Another core
/// file on the card, such as one left from an earlier package, is not one
/// that the package uses.
pub fn scanned_core(scan: &[u8]) -> Option<String> {
    let scan = std::str::from_utf8(scan).ok()?;
    let prefix = format!("{LINUX_DIRECTORY}/");
    scan.lines().find_map(|line| {
        let name = line.strip_prefix(&prefix)?.strip_suffix(" --scan")?;
        (!name.is_empty() && !name.contains('/')).then(|| format!("{PACKAGE_FOLDER}/{name}"))
    })
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
    let startup = super::update::startup_script(&target);
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
    // With `SCAN_SERVE_ARGUMENT`, SCAN.SH starts the service (`SERVE_FILE`)
    // of a runtime that has one, unless it runs; with `SCAN_STOP_ARGUMENT`,
    // it asks the service to stop, which ends after the request that it
    // answers. A subshell starts the service again when it ends without a
    // stop (`SERVE_RESTARTS`); it waits for the service, so it does not wake
    // while the service runs. When no service runs, a stop clears the mark
    // of the service (`SERVE_MARK_STOPPED`) and removes `SERVE_MARK`, which
    // also ends a pause before a restart. A process that is not yet the copy
    // in RAM is still starting: the loader asks again. The subshell runs apart from the script, with
    // no input or output, so that `lcmd` does not wait for it. Either way
    // SCAN.SH ends there: it writes nothing to the card, because the camera
    // may then be saving pictures.
    //
    // The first SCAN.SH of a power-on also starts the boot log of the start,
    // with only the last lines of the earlier starts. A marker in RAM tells
    // the first run, because the loader repeats SCAN.SH until it finds a
    // plan.
    let start = format!(
        "case \"$1\" in\n\
         {SCAN_SERVE_ARGUMENT})\n\
         \x20 if [ ! -e {SERVE_MARK} ] && [ -e {serve} ]; then\n\
         \x20   : >{SERVE_MARK}\n\
         \x20   (n=0\n\
         \x20   while :; do\n\
         \x20     (cd / && exec {serve}) &\n\
         \x20     echo $! >{SERVE_MARK}\n\
         \x20     wait $! && break\n\
         \x20     n=$((n + 1))\n\
         \x20     [ $n -gt {SERVE_RESTARTS} ] && break\n\
         \x20     sleep 1\n\
         \x20     [ -e {SERVE_MARK} ] || break\n\
         \x20   done\n\
         \x20   rm -f {SERVE_MARK}) </dev/null >/dev/null 2>&1 &\n\
         \x20 fi\n\
         \x20 exit 0 ;;\n\
         {SCAN_STOP_ARGUMENT})\n\
         \x20 if [ -e {SERVE_MARK} ] && read pid _ <{SERVE_MARK} && [ -e /proc/$pid ]; then\n\
         \x20   grep -q -- {SERVE_FROM_RAM} /proc/$pid/cmdline && kill -TERM \"$pid\"\n\
         \x20 else\n\
         \x20   rm -f {SERVE_MARK}\n\
         \x20   [ -e {serve} ] && {serve} {SERVE_MARK_STOPPED}\n\
         \x20 fi\n\
         \x20 exit 0 ;;\n\
         esac\n\
         mkdir -p {LINUX_DATA_DIRECTORY}\n\
         if [ ! -e {BOOT_MARK} ]; then\n\
         \x20 : >{BOOT_MARK}\n\
         \x20 n=$(grep -c '' {log} 2>/dev/null)\n\
         \x20 [ \"${{n:-0}}\" -gt {lines} ] && tail -n {keep} {log} >{BOOT_MARK} && cat {BOOT_MARK} >{log}\n\
         \x20 read t _ </proc/uptime; echo \"$t load\" >>{log}\n\
         fi\n",
        lines = BOOT_LOG_LINES,
        keep = BOOT_LOG_LINES * 3 / 4,
        serve = format_args!("{LINUX_DIRECTORY}/{}", SERVE_FILE.rsplit('/').next().unwrap()),
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
        (crate::card::STARTUP_FILE.into(), startup.into_bytes()),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_scan_script_names_the_first_core_file() {
        let binary: &[u8] = b"core";
        for count in [1, 2, MAX_CORE_FILES] {
            let files = package_runtime_files(&vec![binary; count], ICON_STRIDE);
            let scan = &files[SCAN_FILE];
            assert_eq!(scanned_core(scan), Some(core_path_of(0, count)));
        }
        assert_eq!(scanned_core(b"echo\n"), None);
        assert_eq!(scanned_core(&[0xff, 0xfe]), None);
    }
}
