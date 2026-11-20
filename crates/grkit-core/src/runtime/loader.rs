//! The loader script (`LOADER.TTL`): it waits for Linux and the camera, runs
//! SCAN and INSTALL, then stays resident to restore and save the state.

use super::*;
use crate::abi::{
    CONTROL_SERVICE, CONTROL_WORDS, CUE_PENDING, CUE_UPDATED, CUE_UPDATE_READY, DIAL_SET_WORDS,
    HOST_CONTROL_WORD, HOST_CUE_LABEL, HOST_DIAL_SETS, HOST_DIAL_SET_CHANGES, HOST_LOAD_CUE,
    HOST_PERSIST_WORD, HOST_PROCESSING_CALLS, HOST_PROFILE, HOST_RECIPE_CHANGES, HOST_UPDATE,
    HOST_UPDATE_VERSION, HOST_WLAN, PERSIST_CORE, PERSIST_EFFECTS, PERSIST_EFFECT_COUNT,
    PERSIST_EFFECT_WORDS, PERSIST_MODULES, PERSIST_MODULE_COUNT, PERSIST_MODULE_WORDS,
    PERSIST_READY, PERSIST_RELEASE, PERSIST_REQUEST, PERSIST_REVISION, PERSIST_WORDS, PROFILE,
    PROFILE_KINDS, PROFILE_MODULES, PROFILE_MODULE_SIZE, PROFILE_POINTS, PROFILE_SLOTS,
    PROFILE_SLOT_SIZE, UPDATE_CONFIRMED, UPDATE_READY, UPDATE_WAITING,
};
use crate::capabilities::{self, StateFile};
use crate::card::{DATA_FOLDER, LINUX_DIRECTORY, RTOS_PACKAGE_FOLDER};

/// The resident loader reads the selections at this interval. It writes the
/// state file when two reads in a row agree, so a change reaches the card
/// one to two intervals after the last change.
pub const STATE_POLL_MS: u32 = 2000;
/// The resident loader requests an export of the settings only after a change
/// counter of the header moves, and once after this number of idle polls: a
/// change that no counter shows saves within about one minute.
pub const STATE_IDLE_POLLS: u32 = 30;
/// The service runs only while the Wi-Fi access point of the camera is on
/// (`HOST_WLAN`). At each poll, the resident loader compares that with the
/// service, which marks itself in its mailbox while it runs
/// (`CONTROL_SERVICE`): it runs `SCAN.SH serve` (`SCAN_SERVE_ARGUMENT`) to
/// start the service, or `SCAN.SH stop` (`SCAN_STOP_ARGUMENT`) to stop it, at
/// most at this many polls in a row: about a minute, the time that Linux
/// and the card can take after power-on. A request returns NG until Linux
/// and the card are ready; the script cannot see its result, nor a file on
/// the card, which the camera may write pictures to at that time.
pub const SERVE_POLLS: u32 = 30;
/// The service file in the package folder (`crate::card::SERVE_FILE`).
const SERVE_NAME: &str = "SERVE.BIN";
/// The resident loader looks for an update over Wi-Fi that waits for its
/// confirmation (`crate::card::REQUEST_FILE`) at every this many polls, about
/// 10 s: the look is a card lookup, while the confirmation is a header word
/// that it reads at each poll (`update_script`).
pub const UPDATE_POLLS: u32 = 5;
/// The looks after which an update that nobody confirmed goes, about five
/// minutes.
pub const UPDATE_LOOKS: u32 = 30;
/// In the development profile (runtime/profile.h), the resident loader
/// writes the counts of the core to `PROFILE.LOG` in the data folder after
/// this many polls, and clears them.
pub const PROFILE_POLLS: u32 = 5;

/// Polls for Linux and camera readiness instead of waiting a fixed time.
/// `lcmd` requests are synchronous: each returns after the Linux command ends,
/// or with NG when Linux is not ready. The loader checks the result file after
/// each request and repeats it after a short pause.
///
/// The only fixed wait comes before `lcmd_resp on`. The RTOS builds its host
/// command tables on a worker task after the script starts, and the
/// `lcmd_resp` handler writes through a null pointer until they exist, which
/// stops the camera at power-on. The script cannot ask for the tables. A wait
/// of 1 s or more before `lcmd_resp on` avoids the fault, so the loader waits
/// 2 s.
pub fn loader() -> Vec<u8> {
    loader_for(ICON_STRIDE)
}

/// The statuses that the loader writes to `INSTALL.ERR` itself
/// (`runtime/install_status.h`): SCAN wrote no plan, the plan header is not
/// valid, or INSTALL did not install. The next line has the attempts of the
/// step.
///
/// The loader writes the status of a step before its first request, and
/// removes the file after the install. A request can wait for Linux for the
/// rest of the power-on, so a status written after a request could never
/// reach the card. A failed SCAN or INSTALL replaces it with its own status.
pub const STATUS_LOADER_NO_PLAN: u32 = 60;
pub const STATUS_LOADER_BAD_PLAN: u32 = 61;
pub const STATUS_LOADER_NO_INSTALL: u32 = 62;
/// The status of an INSTALL whose rollback failed (`runtime/install_status.h`):
/// the firmware may keep some of the patches.
const STATUS_ROLLBACK_FAILED: u32 = 24;
/// The status of an INSTALL that found no valid plan, or a plan of another
/// layout (`runtime/install_status.h`). The fast-boot file gives the check
/// in the detail words (`fast_log_script`).
pub const STATUS_PLAN: u32 = 37;
/// The statuses of an INSTALL that ran before the camera was ready
/// (`runtime/install_status.h`): language, record, icons, and settings. The
/// loader tries again. On Linux INSTALL then writes no result file; through
/// the fast-boot file the loader reads the status from the script array.
pub const STATUS_NOT_READY: [u32; 4] = [12, 14, 26, 39];

/// The resident part of the loader (`resident`): the loader includes it after
/// INSTALL, so the start does not wait for the interpreter to read it.
pub const RESIDENT_NAME: &str = "RESIDENT.TTL";

/// The loader for a target with `icon_stride`-byte icon slots.
pub fn loader_for(icon_stride: usize) -> Vec<u8> {
    loader_with(icon_stride, &[])
}

/// The loader for a target with `icon_stride`-byte icon slots and the
/// fast-boot files of `fast` (`runtime::fastboot`).
///
/// The first script array (`ba`) comes before the planned arrays, as the
/// installer expects (`ARRAY_LEAD`). With fast-boot files, the loader first
/// asks the firmware for the probe words in `ba`. On a firmware build with the
/// fast-boot loader and a fast-boot file for its release, the loader runs
/// SCAN through that file and INSTALL through its install file, with no
/// Linux command and no wait. If that SCAN writes no plan, or that INSTALL
/// fails, the loader runs SCAN.SH and INSTALL.SH as on the official firmware.
pub fn loader_with(icon_stride: usize, fast: &[FastRelease]) -> Vec<u8> {
    let path = core_path;
    let (plan, done, failed) = (path("PLAN.BIN"), path("INSTALL.OK"), path("INSTALL.ERR"));
    let resident = path(RESIDENT_NAME);
    let mut lines: Vec<String> = Vec::new();
    // The results of the last Linux steps, which the Linux steps of this
    // start must not find. write_error replaces INSTALL.ERR. A start through
    // the fast-boot files writes none of them, so it removes them only when
    // it falls back to Linux.
    let linux_start: Vec<String> = ["PLAN.BIN", "PLAN.TMP", "INSTALL.OK", "INSTALL.LCK"]
        .iter()
        .map(|name| format!("filedelete '{}'", path(name)))
        .chain([
            format!("filesearch '{plan}'"),
            "if result != 0 goto reject".into(),
            format!("filesearch '{done}'"),
            "if result != 0 goto reject".into(),
        ])
        .collect();
    if fast.is_empty() {
        lines.extend(linux_start[..4].iter().cloned());
    }
    lines.extend(["intdim ba 256".into(), "fast = 0".into()]);
    if !fast.is_empty() {
        lines.extend([
            // No planned array exists yet (check_arrays).
            "allocated = 0".into(),
            // Why the start does not use the fast-boot files (fast_log).
            format!("fastreason = {FAST_OFF_OFFICIAL}"),
            // The probe words stay 0 on the official firmware, whatever the
            // memory of the array held.
            "ba[0] = 0".into(),
            "ba[1] = 0".into(),
            "sendln 'SYS_buildinfo 0'".into(),
            format!("if ba[0] <> {FAST_PROBE_MAGIC} goto probed"),
            format!("fastreason = {FAST_OFF_INTERFACE}"),
            "fastinterface = ba[1]".into(),
            format!("if ba[1] <> {FAST_INTERFACE} goto probed"),
            format!("fastreason = {FAST_OFF_RELEASE}"),
            "release = ba[2] * 1000 + ba[3]".into(),
        ]);
        // A release runs its fast-boot file and its install file, so both
        // must be on the card.
        for (index, release) in fast.iter().enumerate() {
            let next = format!("probe{}", index + 1);
            lines.extend([
                format!("if release <> {} goto {next}", release.number()),
                format!("fastreason = {FAST_OFF_FILES}"),
                format!("filesearch '{}'", release.rtos_path()),
                format!("if result = 0 goto {next}"),
                format!("filesearch '{}'", release.install_rtos_path()),
                format!("if result <> 0 fast = {}", release.number()),
                format!(":{next}"),
            ]);
        }
        lines.extend([":probed".into(), "if fast = 0 call fast_log".into()]);
    }
    lines.extend([
        "attempts = 0".into(),
        format!("failure = {STATUS_LOADER_NO_PLAN}"),
    ]);
    // The fast-boot steps write no file to the card: the status of a Linux
    // step goes to INSTALL.ERR before its first request.
    if fast.is_empty() {
        lines.push("call write_error".into());
        lines.extend(linux_start[4..].iter().cloned());
    }
    if !fast.is_empty() {
        lines.extend([
            "if fast = 0 goto linux".into(),
            format!("ba[{FAST_STATUS_INDEX}] = {FAST_STATUS_NOT_READ}"),
        ]);
        for release in fast {
            lines.push(format!(
                "if fast = {} sendln 'SYS_memovr {} 0 call'",
                release.number(),
                release.rtos_path()
            ));
        }
        // SCAN gives the plan header in the script array.
        lines.extend([
            format!("faststatus = ba[{FAST_STATUS_INDEX}]"),
            "if faststatus = 0 goto plan_ready".into(),
            format!("fastreason = {FAST_OFF_SCAN}"),
            format!("if faststatus = {FAST_STATUS_OFF} fastreason = {FAST_OFF_FAILED}"),
            "call fast_log".into(),
            "fast = 0".into(),
            ":linux".into(),
        ]);
        lines.extend(linux_start.iter().cloned());
        lines.push("call write_error".into());
    }
    lines.extend([
        // Wait for the host command tables of the RTOS before lcmd_resp.
        "mpause 2000".into(),
        "sendln 'lcmd_resp on'".into(),
        // Early requests fail before Linux mounts the card. Give up after 150 attempts.
        ":scan".into(),
        format!("sendln 'lcmd sh {LINUX_DIRECTORY}/SCAN.SH'"),
        format!("filesearch '{plan}'"),
        "if result != 0 goto plan_ready".into(),
        "mpause 200".into(),
        "attempts = attempts + 1".into(),
        "if attempts < 150 goto scan".into(),
        "goto reject".into(),
        ":plan_ready".into(),
        format!("failure = {STATUS_LOADER_BAD_PLAN}"),
    ]);
    if !fast.is_empty() {
        lines.push("if fast <> 0 goto plan_fast".into());
    }
    lines.extend([
        format!("fileopen fh '{plan}' 0"),
        "if fh < 0 goto reject".into(),
        // The words of the plan header in order (`abi::PLAN_FIELDS`).
        "call readword".into(),
        "planmagic = value".into(),
        "call readword".into(),
        "count = value".into(),
        "call readword".into(),
        "words = value".into(),
        "call readword".into(),
        "chunks = value".into(),
        // The plan size, then the script words of the modules.
        "call readword".into(),
        "call readword".into(),
        "modulewords = value".into(),
        "call readword".into(),
        "moduletiles = value".into(),
        "fileclose fh".into(),
    ]);
    if !fast.is_empty() {
        let word = |offset: usize| format!("ba[{}]", FAST_PLAN_INDEX + offset / 4);
        use crate::abi::{
            PLAN_COUNT, PLAN_HEADER_WORDS, PLAN_ICON_CHUNKS, PLAN_MAGIC, PLAN_MODULE_TILES,
            PLAN_MODULE_WORDS,
        };
        lines.extend([
            "goto plan_read".into(),
            ":plan_fast".into(),
            format!("planmagic = {}", word(PLAN_MAGIC)),
            format!("count = {}", word(PLAN_COUNT)),
            format!("words = {}", word(PLAN_HEADER_WORDS)),
            format!("chunks = {}", word(PLAN_ICON_CHUNKS)),
            format!("modulewords = {}", word(PLAN_MODULE_WORDS)),
            format!("moduletiles = {}", word(PLAN_MODULE_TILES)),
            ":plan_read".into(),
        ]);
    }
    lines.extend([
        format!(
            "if planmagic != {} goto reject",
            crate::abi::PLAN_MAGIC_VALUE
        ),
        "if count < 0 goto reject".into(),
        format!("if count > {MAX_COUNT} goto reject"),
        "if modulewords < 0 goto reject".into(),
        format!("if modulewords > {MODULE_ARRAY_WORDS} goto reject"),
        "if moduletiles < 0 goto reject".into(),
        format!("if moduletiles > {MODULE_TILES_MAX} goto reject"),
        format!(
            "expected = {} + count * {}",
            HEADER_SIZE / 4,
            RECORD_SIZE / 4
        ),
        "if words != expected goto reject".into(),
        format!(
            "expectedchunks = (count + moduletiles + {}) / {ICONS_PER_CHUNK}",
            HOST_ICON_COUNT + ICONS_PER_CHUNK - 1
        ),
        "if chunks != expectedchunks goto reject".into(),
    ]);
    if !fast.is_empty() {
        // A plan of the Linux SCAN after a failed fast-boot INSTALL: the
        // arrays exist, and the script cannot allocate them again.
        lines.push("if allocated <> 0 goto check_arrays".into());
    }
    lines.push("intdim registry words".into());
    for i in 0..MAX_CHUNKS {
        lines.extend([
            format!("if chunks <= {i} goto icons_ready"),
            format!(
                "items = count + moduletiles + {HOST_ICON_COUNT} - {}",
                i * ICONS_PER_CHUNK
            ),
            format!("if items > {ICONS_PER_CHUNK} items = {ICONS_PER_CHUNK}"),
            format!("iconwords = items * {}", icon_stride / 4),
            format!("intdim artwork{i} iconwords"),
        ]);
    }
    lines.extend([
        ":icons_ready".into(),
        // The modules (runtime/module_link.h) follow the icon chunks.
        "if modulewords <= 0 goto modules_ready".into(),
        "intdim modulecode modulewords".into(),
        ":modules_ready".into(),
        // The tail: the picture scratch, the menu text, and the labels.
        format!("intdim tailarea {TAIL_WORDS}"),
    ]);
    if !fast.is_empty() {
        lines.extend([
            "allocated = 1".into(),
            "hadcount = count".into(),
            "hadwords = words".into(),
            "hadchunks = chunks".into(),
            "hadmodwords = modulewords".into(),
            "hadmodtiles = moduletiles".into(),
            "goto arrays_ready".into(),
            ":check_arrays".into(),
            "if count <> hadcount goto plan_differs".into(),
            "if words <> hadwords goto plan_differs".into(),
            "if chunks <> hadchunks goto plan_differs".into(),
            "if modulewords <> hadmodwords goto plan_differs".into(),
            "if moduletiles <> hadmodtiles goto plan_differs".into(),
            "goto arrays_ready".into(),
            // The Linux plan needs other arrays than the fast plan: the boot
            // log gets both headers, and the start ends.
            ":plan_differs".into(),
            "call plan_log".into(),
            "goto reject".into(),
            ":arrays_ready".into(),
        ]);
    }
    lines.extend([
        // Not-ready attempts release the install lock. Retry for about 30 s,
        // every 0.5 s through Linux and 20 ms after each attempt through the
        // fast-boot file, which ends without a card access when the camera is
        // not ready, then every 10 s, so a later shooting-mode start can
        // still install.
        "attempts = 0".into(),
        "waits = 60".into(),
        "if fast <> 0 waits = 300".into(),
        format!("failure = {STATUS_LOADER_NO_INSTALL}"),
        if fast.is_empty() {
            "call write_error".into()
        } else {
            "if fast = 0 call write_error".into()
        },
        ":install".into(),
    ]);
    if !fast.is_empty() {
        lines.push(format!("ba[{FAST_STATUS_INDEX}] = {FAST_STATUS_NOT_READ}"));
    }
    for release in fast {
        lines.push(format!(
            "if fast = {} sendln 'SYS_memovr {} 0 call'",
            release.number(),
            release.install_rtos_path()
        ));
    }
    let failed_install = if fast.is_empty() {
        "reject"
    } else {
        "install_failed"
    };
    lines.push(format!(
        "if fast = 0 sendln 'lcmd sh {LINUX_DIRECTORY}/INSTALL.SH'"
    ));
    if !fast.is_empty() {
        // The status of the fast-boot INSTALL is in the script array. An
        // install file that did not run INSTALL fails as INSTALL does: the
        // file was not read or not run, or the memory of the fast-boot file
        // changed after SCAN.
        lines.extend([
            "if fast = 0 goto install_linux".into(),
            format!("faststatus = ba[{FAST_STATUS_INDEX}]"),
            "if faststatus = 0 goto installed".into(),
            format!("if faststatus >= {FAST_STATUS_DATA_CHANGED} goto install_not_run"),
        ]);
        for status in STATUS_NOT_READY {
            lines.push(format!("if faststatus = {status} goto install_wait"));
        }
        lines.extend([
            "errorstatus = faststatus".into(),
            "goto install_failed".into(),
            ":install_linux".into(),
        ]);
    }
    lines.extend([
        format!("filesearch '{done}'"),
        "if result != 0 goto installed".into(),
        // INSTALL replaces the status of the loader with its own when it fails.
        "call read_error".into(),
        format!("if errorstatus = {STATUS_LOADER_NO_INSTALL} goto install_wait"),
        format!("if errorstatus != 0 goto {failed_install}"),
        ":install_wait".into(),
        "mpause 20".into(),
        "if fast = 0 mpause 480".into(),
        "attempts = attempts + 1".into(),
        "if attempts > waits mpause 9500".into(),
        "goto install".into(),
        ":installed".into(),
        format!("filedelete '{failed}'"),
        // Without the resident part, GRKit stays installed but saves no
        // selection.
        format!("filesearch '{resident}'"),
        "if result = 0 end".into(),
        format!("include '{resident}'"),
        "end".into(),
    ]);
    if !fast.is_empty() {
        lines.extend([
            // A failed fast-boot INSTALL: run SCAN and INSTALL through Linux
            // with the arrays of the first plan. Only a failed rollback
            // leaves the firmware changed, so it stops the loader.
            ":install_not_run".into(),
            "errorstatus = faststatus".into(),
            ":install_failed".into(),
            // The detail words: a changed page or block, or a plan check.
            format!("fastaddress = ba[{FAST_DETAIL_INDEX}]"),
            format!("fastcount = ba[{}]", FAST_DETAIL_INDEX + 1),
            "if fast = 0 goto reject".into(),
            // The fast-boot INSTALL writes no INSTALL.ERR.
            "failure = errorstatus".into(),
            format!("if errorstatus = {STATUS_ROLLBACK_FAILED} goto reject"),
            "faststatus = errorstatus".into(),
            format!("fastreason = {FAST_OFF_INSTALL}"),
            "call fast_log".into(),
            "fast = 0".into(),
            "attempts = 0".into(),
            format!("failure = {STATUS_LOADER_NO_PLAN}"),
            "goto linux".into(),
        ]);
    }
    lines.extend([
        ":reject_close".into(),
        "fileclose fh".into(),
        // The status of the step, unless SCAN or INSTALL wrote their own.
        ":reject".into(),
        "call read_error".into(),
        "if errorstatus = 0 goto reject_write".into(),
        format!("if errorstatus = {STATUS_LOADER_NO_PLAN} goto reject_write"),
        format!("if errorstatus = {STATUS_LOADER_NO_INSTALL} goto reject_write"),
        "goto reject_end".into(),
        ":reject_write".into(),
        "call write_error".into(),
        ":reject_end".into(),
        "dispstr 'GR_MODULE_LOADER_REJECTED'".into(),
        "end".into(),
        ":readword".into(),
        "value = 0".into(),
        "weight = 1".into(),
        "for byteindex 0 3".into(),
        "fileread fh 1 octet".into(),
        "if result != 0 goto reject_close".into(),
        "str2code bytevalue octet".into(),
        "value = value + bytevalue * weight".into(),
        "if byteindex < 3 weight = weight * 256".into(),
        "next".into(),
        "return".into(),
        // INSTALL.ERR from the RTOS, which needs no Linux: the status of the
        // step, then its attempts.
        ":write_error".into(),
        format!("filedelete '{failed}'"),
        format!("filecreate efh '{failed}'"),
        "if efh < 0 goto write_error_end".into(),
        "int2str line failure".into(),
        "filewriteln efh line".into(),
        "int2str line attempts".into(),
        "filewriteln efh line".into(),
        "fileclose efh".into(),
        ":write_error_end".into(),
        "return".into(),
        // The status in INSTALL.ERR from its first two digits, or 0 without
        // the file. A `filereadln` of a last line without a newline blocks
        // the interpreter, so the bytes are read one by one.
        ":read_error".into(),
        "errorstatus = 0".into(),
        format!("fileopen efh '{failed}' 0"),
        "if efh < 0 goto read_error_end".into(),
        "fileread efh 1 octet".into(),
        "if result != 0 goto read_error_close".into(),
        "str2code bytevalue octet".into(),
        "errorstatus = (bytevalue - 48) * 10".into(),
        "fileread efh 1 octet".into(),
        "if result != 0 goto read_error_close".into(),
        "str2code bytevalue octet".into(),
        "errorstatus = errorstatus + bytevalue - 48".into(),
        ":read_error_close".into(),
        "fileclose efh".into(),
        ":read_error_end".into(),
        "return".into(),
    ]);
    if !fast.is_empty() {
        lines.extend(fast_log_script());
    }
    (lines.join("\r\n") + "\r\n").into_bytes()
}

/// The resident part of the loader (`RESIDENT_NAME`), which `loader_with`
/// includes after INSTALL: it saves the selections after each change
/// (`state_script`) and writes the development profile (`profile_script`).
/// It uses only its own labels.
pub fn resident() -> Vec<u8> {
    let mut lines = state_script();
    lines.extend(profile_script());
    (lines.join("\r\n") + "\r\n").into_bytes()
}

/// Why a start with fast-boot files does not use them, or stops using them
/// (`fastreason`, `fast_log_script`): the official firmware, another
/// interface of the fast-boot firmware, a release without fast-boot files in
/// the package, a missing fast-boot or install file, an earlier fast INSTALL
/// of the same file that failed (`FAST_STATUS_OFF`), or a failed SCAN or
/// INSTALL. The reasons from `FAST_OFF_SCAN` on log a status.
const FAST_OFF_OFFICIAL: u32 = 1;
const FAST_OFF_INTERFACE: u32 = 2;
const FAST_OFF_RELEASE: u32 = 3;
const FAST_OFF_FILES: u32 = 4;
const FAST_OFF_FAILED: u32 = 5;
const FAST_OFF_SCAN: u32 = 6;
const FAST_OFF_INSTALL: u32 = 7;

/// The subroutine `fast_log`: one line in the boot log (`BOOT_LOG`) of a
/// start that does not use its fast-boot files, with the reason. The person
/// sends the boot log with a report. A start that installs through the
/// fast-boot files writes no line, and every start with a line runs SCAN.SH,
/// which keeps the log short. The lines are:
///
/// - `fast-boot off: official firmware`
/// - `fast-boot off: interface N`
/// - `fast-boot RELEASE off: not in this package`
/// - `fast-boot RELEASE off: files missing`
/// - `fast-boot RELEASE off: an earlier fast install failed`
/// - `fast-boot RELEASE scan STATUS, Linux used`
/// - `fast-boot RELEASE install STATUS, Linux used`
/// - `fast-boot RELEASE install STATUS at ADDRESS count N, Linux used`
/// - `fast-boot RELEASE install 37 check C value V, Linux used`
///
/// The Linux SCAN after a failed fast INSTALL must give the plan header of
/// the fast SCAN, because the script cannot allocate its arrays again. If
/// not, `plan_log` writes `fast plan C W K M T, Linux plan C W K M T` (count,
/// header words, icon chunks, module words, module tiles), and the start
/// ends with INSTALL.ERR 61. The marker sends the next start through Linux.
///
/// STATUS is that of `runtime/install_status.h`, `FAST_STATUS_NOT_RUN` when
/// the head did not run SCAN or INSTALL, or `FAST_STATUS_NOT_READ` when the
/// firmware did not read the file. `FAST_STATUS_CODE_CHANGED` and
/// `FAST_STATUS_DATA_CHANGED` mean that other code wrote into the memory of
/// the fast-boot file after SCAN: N code pages or data blocks changed, the
/// first at ADDRESS (decimal). Status 37 (STATUS_PLAN) names the check of
/// the plan that failed and the value of INSTALL (installer.c: PLAN_CHECK_*).
/// After a failed fast INSTALL the fast-boot file writes a marker, and the
/// next starts log `off: an earlier fast install failed` until a new package.
fn fast_log_script() -> Vec<String> {
    let log = format!("{DATA_FOLDER}\\{BOOT_LOG}");
    vec![
        ":fast_log".to_string(),
        format!("foldersearch '{DATA_FOLDER}'"),
        format!("if result = 0 foldercreate '{DATA_FOLDER}'"),
        format!("filesearch '{log}'"),
        "if result != 0 goto fast_log_append".into(),
        format!("filecreate lfh '{log}'"),
        "goto fast_log_opened".into(),
        ":fast_log_append".into(),
        format!("fileopen lfh '{log}' 1"),
        ":fast_log_opened".into(),
        "if lfh < 0 goto fast_log_end".into(),
        "filewrite lfh 'fast-boot'".into(),
        format!("if fastreason = {FAST_OFF_OFFICIAL} filewrite lfh ' off: official firmware'"),
        format!("if fastreason = {FAST_OFF_INTERFACE} filewrite lfh ' off: interface '"),
        format!("if fastreason = {FAST_OFF_INTERFACE} int2str line fastinterface"),
        format!("if fastreason = {FAST_OFF_INTERFACE} filewrite lfh line"),
        format!("if fastreason >= {FAST_OFF_RELEASE} filewrite lfh ' '"),
        format!("if fastreason >= {FAST_OFF_RELEASE} int2str line release"),
        format!("if fastreason >= {FAST_OFF_RELEASE} filewrite lfh line"),
        format!("if fastreason = {FAST_OFF_RELEASE} filewrite lfh ' off: not in this package'"),
        format!("if fastreason = {FAST_OFF_FILES} filewrite lfh ' off: files missing'"),
        format!("if fastreason = {FAST_OFF_FAILED} filewrite lfh ' off: an earlier fast install failed'"),
        format!("if fastreason = {FAST_OFF_SCAN} filewrite lfh ' scan '"),
        format!("if fastreason = {FAST_OFF_INSTALL} filewrite lfh ' install '"),
        format!("if fastreason >= {FAST_OFF_SCAN} int2str line faststatus"),
        format!("if fastreason >= {FAST_OFF_SCAN} filewrite lfh line"),
        format!("if fastreason <> {FAST_OFF_INSTALL} goto fast_log_changed_end"),
        format!("if faststatus = {STATUS_PLAN} goto fast_log_check"),
        format!("if faststatus < {FAST_STATUS_DATA_CHANGED} goto fast_log_changed_end"),
        format!("if faststatus > {FAST_STATUS_CODE_CHANGED} goto fast_log_changed_end"),
        "filewrite lfh ' at '".into(),
        "int2str line fastaddress".into(),
        "filewrite lfh line".into(),
        "filewrite lfh ' count '".into(),
        "int2str line fastcount".into(),
        "filewrite lfh line".into(),
        "goto fast_log_changed_end".into(),
        ":fast_log_check".into(),
        "filewrite lfh ' check '".into(),
        "int2str line fastaddress".into(),
        "filewrite lfh line".into(),
        "filewrite lfh ' value '".into(),
        "int2str line fastcount".into(),
        "filewrite lfh line".into(),
        ":fast_log_changed_end".into(),
        format!("if fastreason >= {FAST_OFF_SCAN} filewrite lfh ', Linux used'"),
        "filewriteln lfh ''".into(),
        "fileclose lfh".into(),
        ":fast_log_end".into(),
        "return".into(),
        // `fast plan COUNT WORDS CHUNKS MODULEWORDS MODULETILES, Linux plan ...`
        ":plan_log".into(),
        format!("filesearch '{log}'"),
        "if result = 0 goto plan_log_end".into(),
        format!("fileopen lfh '{log}' 1"),
        "if lfh < 0 goto plan_log_end".into(),
        "filewrite lfh 'fast plan'".into(),
        "planvalue = hadcount".into(),
        "call plan_value".into(),
        "planvalue = hadwords".into(),
        "call plan_value".into(),
        "planvalue = hadchunks".into(),
        "call plan_value".into(),
        "planvalue = hadmodwords".into(),
        "call plan_value".into(),
        "planvalue = hadmodtiles".into(),
        "call plan_value".into(),
        "filewrite lfh ', Linux plan'".into(),
        "planvalue = count".into(),
        "call plan_value".into(),
        "planvalue = words".into(),
        "call plan_value".into(),
        "planvalue = chunks".into(),
        "call plan_value".into(),
        "planvalue = modulewords".into(),
        "call plan_value".into(),
        "planvalue = moduletiles".into(),
        "call plan_value".into(),
        "filewriteln lfh ''".into(),
        "fileclose lfh".into(),
        ":plan_log_end".into(),
        "return".into(),
        ":plan_value".into(),
        "filewrite lfh ' '".into(),
        "int2str line planvalue".into(),
        "filewrite lfh line".into(),
        "return".into(),
    ]
}

/// The resident part of the loader: it restores the selections, then saves them
/// after each change.
///
/// The private header is the `registry` array, so the script reads and writes
/// it directly. A runtime error ends the script, and the interpreter then frees
/// the arrays that the hooks use. So each array index is checked before use,
/// and no expression divides by a value from the card or from the header.
fn state_script() -> Vec<String> {
    let word = |offset: usize| format!("registry[{}]", offset / 4);
    let state_keys = capabilities::state_keys();
    let keys = state_keys.iter().enumerate();

    // INSTALL captures revision 1 before hooks can change settings. Do not
    // take the current revision as saved: App may have changed it already.
    // Missing effects retain their on-card entries until a real change.
    let mut lines: Vec<String> = vec![
        ":resident".into(),
        "stateok = 1".into(),
        format!("foldersearch '{DATA_FOLDER}'"),
        format!("if result = 0 foldercreate '{DATA_FOLDER}'"),
        format!("foldersearch '{DATA_FOLDER}'"),
        "if result = 0 stateok = 0".into(),
    ];
    lines.extend([
        // Only a runtime with the service (`serve_script`) and a mailbox
        // within the module array starts and stops it.
        "hasserve = 0".into(),
        "servepolls = 0".into(),
        "respon = 0".into(),
        format!("cword = registry[{}]", HOST_CONTROL_WORD / 4),
        "if cword < 1 goto resident_serve".into(),
        format!("if cword > modulewords - {CONTROL_WORDS} goto resident_serve"),
        format!("filesearch '{}'", core_path(SERVE_NAME)),
        "if result <> 0 hasserve = 1".into(),
        ":resident_serve".into(),
        "updatepolls = 0".into(),
        "updatelooks = 0".into(),
        // The version file of an update that installed at this start, with no
        // update that waits: the load cue reads GRKit Updated.
        format!(
            "filesearch '{}'",
            core_path(file_name(crate::card::VERSION_FILE))
        ),
        "if result = 0 goto resident_updated".into(),
        format!(
            "filesearch '{}'",
            core_path(file_name(crate::card::REQUEST_FILE))
        ),
        "if result <> 0 goto resident_updated".into(),
        format!("{} = {CUE_UPDATED}", word(HOST_CUE_LABEL)),
        format!(
            "filedelete '{}'",
            core_path(file_name(crate::card::VERSION_FILE))
        ),
        ":resident_updated".into(),
        "legacy = 0".into(),
        format!("export = {}", word(HOST_PERSIST_WORD)),
        "if modulewords <> 0 goto resident_export".into(),
        "if export <> 0 goto resident_export".into(),
        "legacy = 1".into(),
        ":resident_export".into(),
        "changes = 1".into(),
        format!("if legacy <> 0 changes = {}", word(HOST_STATE_CHANGES)),
        "saved = changes".into(),
        "last = changes".into(),
    ]);
    for file in [StateFile::Recipes, StateFile::DialSets] {
        let (changes, at, saved, _) = watch_names(file);
        lines.push(format!("{changes} = {}", word(at)));
        lines.push(format!("{saved} = 0"));
        lines.push(format!("{changes}last = {changes}"));
    }
    lines.extend([
        format!("filesearch '{}'", state_path(StateFile::Main, "TXT")),
        "if result = 0 saved = changes - 1".into(),
        // No sum of the counters is negative, so the first poll is complete.
        "hint = 0".into(),
        "quiet = 0 - 1".into(),
        "idle = 0".into(),
        format!("profile = {}", word(HOST_PROFILE)),
        "if profile <> 0 goto profile_watch".into(),
    ]);
    lines.extend(watch_idle("watch"));
    lines.extend(watch_poll("watch"));

    // Save each file after a change of one of its keys. `filerename` cannot
    // replace a file, and its result is 0 for success and failure, so each
    // step checks the files. If power fails between the steps, the restore
    // reads the temporary file. A key with the value 0 has no entry.
    for file in StateFile::ALL {
        let name = label(file);
        let (state, temporary) = (state_path(file, "TXT"), state_path(file, "TMP"));
        let main = file == StateFile::Main;
        let end = format!("save_{name}_end");
        let own: Vec<_> = keys.clone().filter(|(_, key)| key.file == file).collect();
        lines.extend([
            format!(":save_{name}"),
            format!("if stateok = 0 goto {end}"),
            if main {
                "call snapshot".into()
            } else {
                format!(":save_{name}_start")
            },
            format!("filedelete '{temporary}'"),
            format!("filecreate sfh '{temporary}'"),
            format!("if sfh < 0 goto {end}"),
        ]);
        for (i, key) in &own {
            lines.extend([
                format!("if value{i} = 0 goto save_key{i}"),
                format!("filewriteln sfh '{}'", state_key_id(&key.name)),
                format!("int2str line value{i}"),
                "filewriteln sfh line".into(),
                format!(":save_key{i}"),
            ]);
        }
        if main {
            // Every value comes from the same frozen export, including Fn
            // and module settings. No live registry read occurs during I/O.
            lines.extend([
                "pindex = 0".into(),
                ":save_parameters".into(),
                "if pindex >= pcount goto save_parameters_end".into(),
                "if legacy <> 0 goto save_legacy_parameters".into(),
                format!(
                    "pcode = {}",
                    persist_entry(PERSIST_EFFECTS, "pindex", PERSIST_EFFECT_WORDS, 0)
                ),
                format!(
                    "pvalue = {}",
                    persist_entry(PERSIST_EFFECTS, "pindex", PERSIST_EFFECT_WORDS, 1)
                ),
                format!(
                    "ptone = {}",
                    persist_entry(PERSIST_EFFECTS, "pindex", PERSIST_EFFECT_WORDS, 2)
                ),
                "goto save_parameter_values".into(),
                ":save_legacy_parameters".into(),
                "call legacy_parameters".into(),
                ":save_parameter_values".into(),
                "pkey = pcode".into(),
                "if pvalue = 0 goto save_toning".into(),
                format!("pkey = pkey + {PARAMETER_KEY_BASE}"),
                "int2str line pkey".into(),
                "filewriteln sfh line".into(),
                "int2str line pvalue".into(),
                "filewriteln sfh line".into(),
                ":save_toning".into(),
                "if ptone < 1 goto save_parameters_next".into(),
                "if ptone > 8 goto save_parameters_next".into(),
                "pkey = pcode".into(),
                format!("pkey = pkey + {TONING_KEY_BASE}"),
                "int2str line pkey".into(),
                "filewriteln sfh line".into(),
                "int2str line ptone".into(),
                "filewriteln sfh line".into(),
                ":save_parameters_next".into(),
                "pindex = pindex + 1".into(),
                "goto save_parameters".into(),
                ":save_parameters_end".into(),
                // Save zero too: a missing setting uses its declared default.
                "mindex = 0".into(),
                ":save_modules".into(),
                "if mindex >= mcount goto save_modules_end".into(),
                format!(
                    "mid = {}",
                    persist_entry(PERSIST_MODULES, "mindex", PERSIST_MODULE_WORDS, 0)
                ),
                format!(
                    "mvalue = {}",
                    persist_entry(PERSIST_MODULES, "mindex", PERSIST_MODULE_WORDS, 1)
                ),
                "if mid <= 0 goto save_modules_next".into(),
                "if mvalue < 0 goto save_modules_next".into(),
                "int2str line mid".into(),
                "filewriteln sfh line".into(),
                "int2str line mvalue".into(),
                "filewriteln sfh line".into(),
                ":save_modules_next".into(),
                "mindex = mindex + 1".into(),
                "goto save_modules".into(),
                ":save_modules_end".into(),
            ]);
        }
        if file == StateFile::Recipes {
            // The recipes, their names, the import hash, and the IDs: each
            // word that is not 0 (runtime/recipes.h), at its key
            // (state::recipe_key).
            let words = RECIPE_WORDS * RECIPE_SLOTS;
            let names = words + RECIPE_NAME_WORDS * RECIPE_SLOTS;
            lines.extend([
                "rindex = 0".into(),
                ":recipe_word".into(),
                format!("if rindex >= {RECIPE_STATE_WORDS} goto recipe_word_end"),
                format!("rvalue = registry[{} + rindex]", HOST_RECIPES / 4),
                "if rvalue <= 0 goto recipe_word_next".into(),
                format!("if rvalue > {RECIPE_VALUE_MAX} goto recipe_word_next"),
                format!("rkey = rindex % {RECIPE_WORDS}"),
                format!("rkey = rkey * {RECIPE_KEY_STRIDE}"),
                format!("rslot = rindex / {RECIPE_WORDS}"),
                format!("rkey = rkey + {RECIPE_KEY_BASE}"),
                "rkey = rkey + rslot".into(),
                format!("if rindex < {words} goto recipe_key"),
                format!("rkey = rindex - {names}"),
                format!("rkey = rkey + {RECIPE_IMPORTED_KEY}"),
                format!("if rindex >= {names} goto recipe_key"),
                format!("rslot = rindex - {words}"),
                format!("rkey = rslot % {RECIPE_NAME_WORDS}"),
                format!("rkey = rkey * {RECIPE_KEY_STRIDE}"),
                format!("rslot = rslot / {RECIPE_NAME_WORDS}"),
                format!("rkey = rkey + {RECIPE_NAME_KEY_BASE}"),
                "rkey = rkey + rslot".into(),
                ":recipe_key".into(),
                "int2str line rkey".into(),
                "filewriteln sfh line".into(),
                "int2str line rvalue".into(),
                "filewriteln sfh line".into(),
                ":recipe_word_next".into(),
                "rindex = rindex + 1".into(),
                "goto recipe_word".into(),
                ":recipe_word_end".into(),
            ]);
        }
        if file == StateFile::DialSets {
            // The words of the snapshot of each user mode that are not 0,
            // then those of what each user mode does, the word of P/Av/Tv/M
            // (runtime/mode_dial.h), and the copy of the switch, Enable at
            // Start, Distance Unit, and Uncropped RAW (runtime/manage.h), at their keys
            // (state::dial_set_key). The save of the recipe file has the same
            // variables, which keeps the variables of the script few.
            lines.extend([
                "rindex = 0".into(),
                ":set_word".into(),
                format!("if rindex >= {DIAL_SET_STATE_WORDS} goto set_word_end"),
                format!("rvalue = registry[{} + rindex]", HOST_DIAL_SETS / 4),
                "if rvalue <= 0 goto set_word_next".into(),
                format!("if rvalue > {DIAL_SET_VALUE_MAX} goto set_word_next"),
                format!("rkey = rindex / {DIAL_SET_WORDS}"),
                format!("rkey = rkey * {DIAL_SET_KEY_STRIDE}"),
                format!("rkey = rkey + {DIAL_SET_KEY_BASE}"),
                format!("rslot = rindex % {DIAL_SET_WORDS}"),
                "rkey = rkey + rslot".into(),
                "int2str line rkey".into(),
                "filewriteln sfh line".into(),
                "int2str line rvalue".into(),
                "filewriteln sfh line".into(),
                ":set_word_next".into(),
                "rindex = rindex + 1".into(),
                "goto set_word".into(),
                ":set_word_end".into(),
            ]);
        }
        lines.extend([
            "fileclose sfh".into(),
            format!("filesearch '{temporary}'"),
            format!("if result = 0 goto {end}"),
            format!("filedelete '{state}'"),
            format!("filesearch '{state}'"),
            format!("if result != 0 goto {end}"),
            format!("filerename '{temporary}' '{state}'"),
            format!("filesearch '{state}'"),
            format!("if result = 0 goto {end}"),
        ]);
        let (_, _, saved, stamp) = watch_names(file);
        lines.extend([
            format!("{saved} = {stamp}"),
            format!(":{end}"),
            "return".into(),
        ]);
    }

    // Core scalars and the subsequent effect/module loops share one lease.
    lines.push(":snapshot".into());
    lines.push("if legacy <> 0 goto legacy_snapshot".into());
    for (i, _) in keys {
        lines.push(format!("value{i} = {}", persist_word(PERSIST_CORE + i * 4)));
    }
    lines.push("return".into());
    lines.extend(persist_capture());
    lines.extend(legacy_capture());
    lines.extend(serve_script());
    lines.extend(update_script());

    lines
}

/// The loader path of the profile file.
fn profile_path() -> String {
    format!("{DATA_FOLDER}\\PROFILE.LOG")
}

/// The development profile of the resident loader (runtime/profile.h). The
/// watch loop of the profile saves the state as the usual one does, and after
/// each `PROFILE_POLLS` polls appends a period to `PROFILE.LOG`: a line with
/// its number and the live-view frames, then a line for each counter slot
/// with calls: its name, the calls, the cycles, and the cycles of the largest
/// call. The loader clears each slot that it writes. The module names come
/// from the profile table, 8 ASCII bytes each.
fn profile_script() -> Vec<String> {
    let table = PROFILE / 4;
    let path = profile_path();
    let mut lines: Vec<String> = vec![
        ":profile_watch".into(),
        "ppolls = 0".into(),
        "pperiod = 0".into(),
        format!("pframes = registry[{}]", HOST_PROCESSING_CALLS / 4),
    ];
    // The module names: the bytes up to the first 0.
    for module in 0..PROFILE_MODULES {
        let base = table + module * PROFILE_MODULE_SIZE / 4 + 2;
        let end = format!("profile_name{module}_end");
        lines.push(format!("pname{module} = ''"));
        for byte in 0..8 {
            lines.extend([
                format!(
                    "pc = (tailarea[{}] >> {}) and 255",
                    base + byte / 4,
                    byte % 4 * 8
                ),
                format!("if pc = 0 goto {end}"),
                "code2str ps pc".into(),
                format!("strconcat pname{module} ps"),
            ]);
        }
        lines.push(format!(":{end}"));
    }
    lines.extend([
        // A new file, or the end of the file of an earlier start.
        format!("filesearch '{path}'"),
        "if result != 0 goto profile_append".into(),
        format!("filecreate pfh '{path}'"),
        "goto profile_opened".into(),
        ":profile_append".into(),
        format!("fileopen pfh '{path}' 1"),
        ":profile_opened".into(),
        "if pfh < 0 goto profile_poll".into(),
        "filewriteln pfh 'GRKit profile: for each period, the live-view frames; for each point, calls, CPU cycles, and the cycles of the largest call'".into(),
        "fileclose pfh".into(),
        ":profile_poll".into(),
        format!("mpause {STATE_POLL_MS}"),
        "if hasserve <> 0 call serve".into(),
        "call update_poll".into(),
        "ppolls = ppolls + 1".into(),
        format!("if ppolls >= {PROFILE_POLLS} call profile_write"),
    ]);
    lines.extend(watch_poll("profile_poll"));
    lines.extend([
        ":profile_write".into(),
        "ppolls = 0".into(),
        "pperiod = pperiod + 1".into(),
        format!("pf = registry[{}]", HOST_PROCESSING_CALLS / 4),
        "pframe = pf - pframes".into(),
        "pframes = pf".into(),
        format!("fileopen pfh '{path}' 1"),
        "if pfh < 0 goto profile_write_end".into(),
        "filewrite pfh 'period '".into(),
        "int2str ps pperiod".into(),
        "filewrite pfh ps".into(),
        "filewrite pfh ' frames '".into(),
        "int2str ps pframe".into(),
        "filewriteln pfh ps".into(),
    ]);
    let mut slot_lines = |slot: usize, name: Vec<String>| {
        let at = table + (PROFILE_SLOTS + slot * PROFILE_SLOT_SIZE) / 4;
        let end = format!("profile_slot{slot}");
        lines.extend([
            format!("pc = tailarea[{at}]"),
            format!("if pc = 0 goto {end}"),
            format!("pcy = tailarea[{}]", at + 1),
            format!("pmx = tailarea[{}]", at + 2),
            format!("tailarea[{at}] = 0"),
            format!("tailarea[{}] = 0", at + 1),
            format!("tailarea[{}] = 0", at + 2),
        ]);
        lines.extend(name);
        for (value, last) in [("pc", false), ("pcy", false), ("pmx", true)] {
            lines.push(format!("int2str ps {value}"));
            lines.push("filewrite pfh ps".into());
            lines.push(if last {
                "filewriteln pfh ''".into()
            } else {
                "filewrite pfh ' '".into()
            });
        }
        lines.push(format!(":{end}"));
    };
    for (slot, (name, _)) in PROFILE_POINTS.iter().enumerate() {
        slot_lines(slot, vec![format!("filewrite pfh '{name} '")]);
    }
    for module in 0..PROFILE_MODULES {
        for (kind, (name, _)) in PROFILE_KINDS.iter().enumerate() {
            let slot = PROFILE_POINTS.len() + module * PROFILE_KINDS.len() + kind;
            slot_lines(
                slot,
                vec![
                    format!("filewrite pfh pname{module}"),
                    format!("filewrite pfh ' {name} '"),
                ],
            );
        }
    }
    lines.extend([
        "fileclose pfh".into(),
        ":profile_write_end".into(),
        "return".into(),
    ]);
    lines
}

/// The label stem of a state file in the loader.
fn label(file: StateFile) -> String {
    file.stem().to_ascii_lowercase()
}

/// The variables of the watch of a state file: its count of changes and the
/// header field of it, the count that the file has, and the count of the
/// save that runs.
fn watch_names(file: StateFile) -> (&'static str, usize, &'static str, &'static str) {
    match file {
        StateFile::Main => ("changes", HOST_STATE_CHANGES, "saved", "stamp"),
        StateFile::Recipes => ("rchanges", HOST_RECIPE_CHANGES, "rsaved", "rstamp"),
        StateFile::DialSets => ("dchanges", HOST_DIAL_SET_CHANGES, "dsaved", "dstamp"),
    }
}

/// A word of the bounded, immutable export in the module array.
fn persist_word(offset: usize) -> String {
    format!("modulecode[pbase + {}]", offset / 4)
}

fn persist_entry(offset: usize, index: &str, width: usize, field: usize) -> String {
    format!(
        "modulecode[pbase + {} + {index} * {width}]",
        offset / 4 + field
    )
}

/// The subroutine `serve`: start or stop the service with the access point
/// (`SERVE_POLLS`). The first poll comes one interval after the resident
/// loader starts, so `lcmd_resp on` comes more than 1 s after the script
/// starts (`loader`).
fn serve_script() -> Vec<String> {
    let scan = |argument: &str| format!("sendln 'lcmd sh {LINUX_DIRECTORY}/SCAN.SH {argument}'");
    vec![
        ":serve".into(),
        format!("wlan = registry[{}]", HOST_WLAN / 4),
        format!("running = modulecode[cword + {}]", CONTROL_SERVICE / 4),
        "if running <> 0 running = 1".into(),
        "if running = wlan goto serve_settled".into(),
        format!("if servepolls >= {SERVE_POLLS} return"),
        "servepolls = servepolls + 1".into(),
        "if respon = 0 sendln 'lcmd_resp on'".into(),
        "respon = 1".into(),
        "if wlan = 0 goto serve_stop".into(),
        scan(SCAN_SERVE_ARGUMENT),
        "return".into(),
        ":serve_stop".into(),
        scan(SCAN_STOP_ARGUMENT),
        "return".into(),
        ":serve_settled".into(),
        "servepolls = 0".into(),
        "return".into(),
    ]
}

/// The subroutine `update_poll`: an update over Wi-Fi (`runtime::update`). The
/// service writes its journal as `card::REQUEST_FILE`. The subroutine sets
/// GRKIT_HOST_UPDATE to UPDATE_WAITING while that file is there, so GRKit
/// Settings shows the Update row. After the confirmation there
/// (UPDATE_CONFIRMED), it renames the file to `card::JOURNAL_FILE`, which the
/// startup script runs at the next power-on, and sets UPDATE_READY. An update
/// that waits UPDATE_LOOKS looks goes, and one that the service removes is no
/// longer shown.
fn update_script() -> Vec<String> {
    let name = |path: &'static str| path.rsplit('/').next().unwrap();
    let request = core_path(name(crate::card::REQUEST_FILE));
    let journal = core_path(name(crate::card::JOURNAL_FILE));
    let version = core_path(name(crate::card::VERSION_FILE));
    let word = format!("registry[{}]", HOST_UPDATE / 4);
    let version_word = format!("registry[{}]", HOST_UPDATE_VERSION / 4);
    // The interpreter takes an assignment to an array element only on a line
    // of its own, not after `if`, so the subroutine jumps to those lines.
    vec![
        ":update_poll".into(),
        format!("updstate = {word}"),
        format!("if updstate = {UPDATE_CONFIRMED} goto update_confirmed"),
        "updatepolls = updatepolls + 1".into(),
        format!("if updatepolls < {UPDATE_POLLS} return"),
        "updatepolls = 0".into(),
        format!("filesearch '{request}'"),
        "if result <> 0 goto update_waits".into(),
        "updatelooks = 0".into(),
        format!("if updstate = {UPDATE_WAITING} goto update_none"),
        // A ready update whose journal the service removed is gone.
        format!("if updstate <> {UPDATE_READY} return"),
        format!("filesearch '{journal}'"),
        "if result <> 0 return".into(),
        ":update_none".into(),
        format!("{word} = 0"),
        "return".into(),
        ":update_waits".into(),
        "updatelooks = updatelooks + 1".into(),
        format!("if updatelooks >= {UPDATE_LOOKS} goto update_late"),
        // A ready update that the service replaced waits again.
        format!("if updstate = {UPDATE_WAITING} return"),
        "call update_version".into(),
        format!("{word} = {UPDATE_WAITING}"),
        // The load cue tells it again in live view: GRKit Update Ready.
        format!("registry[{}] = {CUE_UPDATE_READY}", HOST_CUE_LABEL / 4),
        format!("registry[{}] = {CUE_PENDING}", HOST_LOAD_CUE / 4),
        "return".into(),
        ":update_late".into(),
        format!("filedelete '{request}'"),
        format!("filedelete '{version}'"),
        "updatelooks = 0".into(),
        format!("{word} = 0"),
        "return".into(),
        ":update_confirmed".into(),
        format!("filesearch '{request}'"),
        "if result = 0 goto update_renamed".into(),
        format!("filerename '{request}' '{journal}'"),
        ":update_renamed".into(),
        "updatelooks = 0".into(),
        format!("filesearch '{journal}'"),
        "if result = 0 goto update_none".into(),
        format!("{word} = {UPDATE_READY}"),
        "return".into(),
        // The version file has six digits (`card::VERSION_FILE`). The
        // interpreter reads a file one octet at a time, as `read_error` does.
        ":update_version".into(),
        "updversion = 0".into(),
        "updpair = 0".into(),
        format!("fileopen updfh '{version}' 0"),
        "if updfh < 0 goto update_version_end".into(),
        "for updindex 1 6".into(),
        "fileread updfh 1 octet".into(),
        "if result != 0 goto update_version_close".into(),
        "str2code upddigit octet".into(),
        "upddigit = upddigit - 48".into(),
        "if upddigit < 0 upddigit = 0".into(),
        "if upddigit > 9 upddigit = 0".into(),
        // Two digits each: the version packs as major << 16, minor << 8,
        // patch, which the runtime splits without a division.
        "updpair = updpair * 10 + upddigit".into(),
        "updodd = updindex and 1".into(),
        "if updodd = 0 updversion = updversion * 256 + updpair".into(),
        "if updodd = 0 updpair = 0".into(),
        "next".into(),
        ":update_version_close".into(),
        "fileclose updfh".into(),
        ":update_version_end".into(),
        format!("{version_word} = updversion"),
        "return".into(),
    ]
}

/// Request a view on the App task. A timeout leaves the request outstanding
/// for the next poll. Only the consumer releases a completed view, including
/// invalid views, so a delayed response cannot overwrite a view during I/O.
fn persist_capture() -> Vec<String> {
    vec![
        ":capture".into(),
        "captured = 0".into(),
        "if legacy <> 0 goto legacy_capture".into(),
        format!("if modulewords < {PERSIST_WORDS} return"),
        format!("pbase = registry[{}]", HOST_PERSIST_WORD / 4),
        "if pbase < 2 return".into(),
        format!("pmax = modulewords - {PERSIST_WORDS}"),
        "if pbase > pmax return".into(),
        format!("ticket = {}", persist_word(PERSIST_REQUEST)),
        format!("released = {}", persist_word(PERSIST_RELEASE)),
        "if ticket <> released goto capture_wait".into(),
        "if ticket = 2147483647 ticket = 0".into(),
        "ticket = ticket + 1".into(),
        format!("{} = ticket", persist_word(PERSIST_REQUEST)),
        ":capture_wait".into(),
        "attempts = 0".into(),
        ":capture_poll".into(),
        format!("ready = {}", persist_word(PERSIST_READY)),
        "if ready = ticket goto capture_ready".into(),
        "if attempts >= 5 return".into(),
        "mpause 50".into(),
        "attempts = attempts + 1".into(),
        "goto capture_poll".into(),
        ":capture_ready".into(),
        // A positive RTOS delay executes a system DMB. TTL has no barrier
        // instruction: synchronize after READY before loading its payload.
        "mpause 1".into(),
        format!("pcount = {}", persist_word(PERSIST_EFFECT_COUNT)),
        "if pcount < 0 goto capture_invalid".into(),
        format!("if pcount > {MAX_COUNT} goto capture_invalid"),
        format!("mcount = {}", persist_word(PERSIST_MODULE_COUNT)),
        "if mcount < 0 goto capture_invalid".into(),
        format!("if mcount > {MODULE_SETTINGS_MAX} goto capture_invalid"),
        format!("changes = {}", persist_word(PERSIST_REVISION)),
        "if changes <= 0 goto capture_invalid".into(),
        "captured = 1".into(),
        "return".into(),
        ":capture_invalid".into(),
        "call release_capture".into(),
        "return".into(),
        ":release_capture".into(),
        // Complete payload reads before App may reuse the buffer. Zero
        // delay returns early in the firmware and provides no barrier.
        "mpause 1".into(),
        format!("{} = ticket", persist_word(PERSIST_RELEASE)),
        "return".into(),
    ]
}

/// Without listed modules INSTALL creates no services or module array and
/// installs no menu/dial hooks. Preserve this older core/effect-only path.
fn legacy_capture() -> Vec<String> {
    let word = |offset: usize| format!("registry[{}]", offset / 4);
    let record = |index: &str, offset: usize| {
        format!(
            "registry[{} + {index} * {} + {}]",
            HEADER_SIZE / 4,
            RECORD_SIZE / 4,
            offset / 4
        )
    };
    let mut lines = vec![
        ":legacy_capture".into(),
        format!("changes = {}", word(HOST_STATE_CHANGES)),
        format!("pcount = {}", word(HOST_COUNT)),
        "if pcount < 0 pcount = 0".into(),
        format!("if pcount > {MAX_COUNT} pcount = 0"),
        "mcount = 0".into(),
        "captured = 1".into(),
        "return".into(),
        ":legacy_snapshot".into(),
        "value0 = 0".into(),
        "if pcount < 1 goto legacy_values".into(),
        format!("offset = {} - {}", word(HOST_ACTIVE), word(HOST_RECORDS)),
        "if offset < 0 goto legacy_values".into(),
        format!("rest = offset % {RECORD_SIZE}"),
        "if rest <> 0 goto legacy_values".into(),
        format!("index = offset / {RECORD_SIZE}"),
        "if index >= pcount goto legacy_values".into(),
        format!("value0 = {} and 65535", record("index", RECORD_CODE)),
        ":legacy_values".into(),
    ];
    for (i, key) in capabilities::state_keys().iter().enumerate() {
        if let Some(offset) = key.word {
            lines.push(format!("value{i} = {}", word(offset)));
        }
    }
    lines.extend([
        "return".into(),
        ":legacy_parameters".into(),
        format!("pcode = {} and 32767", record("pindex", RECORD_CODE)),
        format!("pw0 = {}", record("pindex", RECORD_PARAMETERS)),
        format!("pw1 = {}", record("pindex", RECORD_PARAMETERS + 4)),
        format!("pw2 = {}", record("pindex", RECORD_PARAMETERS + 8)),
        "pvalue = 0".into(),
    ]);
    for digit in (0..PARAMETER_COUNT).rev() {
        lines.extend([
            format!("pdigit = (pw{} >> {}) and 255", digit / 4, digit % 4 * 8),
            "if pdigit > 8 pdigit = 0".into(),
            "pvalue = pvalue * 9 + pdigit".into(),
        ]);
    }
    lines.extend([
        format!(
            "ptone = ({} >> {}) and 255",
            record("pindex", RECORD_TONING & !3),
            RECORD_TONING % 4 * 8
        ),
        "return".into(),
    ]);
    lines
}

/// The idle loop of the resident loader. An export costs a request to the App
/// task and about 60 script lines, so a poll asks for one only when the sum
/// of the change counters differs from `quiet`, its value at the last poll
/// that left nothing to save. Each counter only grows, so the sum moves with
/// each of them. The export revision still decides what the loader saves.
fn watch_idle(name: &str) -> Vec<String> {
    let counters = StateFile::ALL
        .map(|file| format!("registry[{}]", watch_names(file).1 / 4))
        .join(" + ");
    vec![
        format!(":{name}"),
        format!("mpause {STATE_POLL_MS}"),
        // Each poll, also an idle one: the service start and the update
        // confirmation do not wait for a change of the settings.
        "if hasserve <> 0 call serve".into(),
        "call update_poll".into(),
        format!("hint = {counters}"),
        format!("if hint <> quiet idle = {STATE_IDLE_POLLS}"),
        "idle = idle + 1".into(),
        format!("if idle <= {STATE_IDLE_POLLS} goto {name}"),
        "idle = 0".into(),
        // A poll that leaves a save pending keeps the next polls complete.
        "quiet = hint - 1".into(),
    ]
}

/// Debounce each file independently. Temporary U-mode edits neither change
/// the main export revision nor postpone its save. Release the export on
/// every completed capture, including a skipped or failed save.
fn watch_poll(name: &str) -> Vec<String> {
    let mut lines = vec![
        "call capture".into(),
        format!("if captured = 0 goto {name}_state_saved"),
        format!("if changes <> last goto {name}_state_release"),
        format!("if changes = saved goto {name}_state_release"),
        "stamp = changes".into(),
        "call save_state".into(),
        format!(":{name}_state_release"),
        "last = changes".into(),
        format!("if legacy <> 0 goto {name}_state_saved"),
        "call release_capture".into(),
        format!(":{name}_state_saved"),
    ];
    for file in [StateFile::Recipes, StateFile::DialSets] {
        let (changes, at, saved, stamp) = watch_names(file);
        let next = format!("{name}_{}_saved", label(file));
        lines.extend([
            format!("{changes} = registry[{}]", at / 4),
            format!("if {changes} <> {changes}last goto {next}"),
            format!("if {changes} = {saved} goto {next}"),
            format!("{stamp} = {changes}"),
            format!("call save_{}", label(file)),
            format!(":{next}"),
            format!("{changes}last = {changes}"),
        ]);
    }
    // Nothing is left to save: the next polls wait for a counter.
    lines.push(format!("if captured = 0 goto {name}"));
    for file in StateFile::ALL {
        let (changes, _, saved, _) = watch_names(file);
        lines.push(format!("if {changes} <> {saved} goto {name}"));
    }
    lines.extend(["quiet = hint".into(), format!("goto {name}")]);
    lines
}

/// The loader path of a state file with the extension `extension`.
fn state_path(file: StateFile, extension: &str) -> String {
    format!("{DATA_FOLDER}\\{}.{extension}", file.stem())
}

/// The name of a card file of the package folder, after its folder.
fn file_name(path: &'static str) -> &'static str {
    path.rsplit('/').next().unwrap()
}

/// The loader path of a file of the package folder.
pub(super) fn core_path(name: &str) -> String {
    format!("{RTOS_PACKAGE_FOLDER}\\{name}")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A runtime error ends the script and frees the arrays, so the resident
    /// part must not index outside the header or divide by a variable.
    #[test]
    fn the_resident_script_cannot_stop_on_its_own_arithmetic() {
        let script = state_script().join("\n");
        let recipes = format!("{} + rindex", HOST_RECIPES / 4);
        let sets = format!("{} + rindex", HOST_DIAL_SETS / 4);
        let record = |index: &str, offset: usize| {
            format!(
                "{} + {index} * {} + {}",
                HEADER_SIZE / 4,
                RECORD_SIZE / 4,
                offset / 4
            )
        };
        let allowed = [
            recipes.clone(),
            sets.clone(),
            record("index", RECORD_CODE),
            record("pindex", RECORD_CODE),
            record("pindex", RECORD_PARAMETERS),
            record("pindex", RECORD_PARAMETERS + 4),
            record("pindex", RECORD_PARAMETERS + 8),
            record("pindex", RECORD_TONING & !3),
        ];
        for (at, _) in script.match_indices("registry[") {
            let index = &script[at + 9..];
            let index = &index[..index.find(']').unwrap()];
            match index.parse::<usize>() {
                Ok(word) => assert!(word < HEADER_SIZE / 4, "registry[{word}]"),
                Err(_) => assert!(allowed.iter().any(|a| a == index), "registry[{index}]"),
            }
        }
        for line in script.lines() {
            for operator in [" / ", " % "] {
                if let Some(at) = line.find(operator) {
                    let divisor = line[at + 3..].split_whitespace().next().unwrap();
                    assert!(
                        divisor.parse::<u32>().is_ok_and(|value| value > 0),
                        "{line}"
                    );
                }
            }
        }
        // A `filereadln` of a last line without a newline blocks the interpreter.
        assert!(!script.contains("filereadln"));
        // Bound the complete buffer before any access, then bound each
        // variable-length table before the save loops can consume it.
        let capture = persist_capture();
        let first_read = capture
            .iter()
            .position(|line| line.contains("modulecode["))
            .unwrap();
        assert!(at(&capture, "if pbase > pmax return") < first_read);
        assert!(capture.contains(&format!("pmax = modulewords - {PERSIST_WORDS}")));
        assert!(capture.contains(&format!("if pcount > {MAX_COUNT} goto capture_invalid")));
        assert!(capture.contains(&format!(
            "if mcount > {MODULE_SETTINGS_MAX} goto capture_invalid"
        )));
        assert!(capture.contains(&"if pcount < 0 goto capture_invalid".into()));
        assert!(capture.contains(&"if mcount < 0 goto capture_invalid".into()));
        assert_eq!(script.matches("if pindex >= pcount goto").count(), 1);
        assert_eq!(script.matches("if mindex >= mcount goto").count(), 1);
        const {
            assert!(PERSIST_EFFECTS / 4 + MAX_COUNT * PERSIST_EFFECT_WORDS <= PERSIST_WORDS);
            assert!(
                PERSIST_MODULES / 4 + MODULE_SETTINGS_MAX * PERSIST_MODULE_WORDS <= PERSIST_WORDS
            );
        }
        // Each recipe index is below RECIPE_STATE_WORDS, which ends in the
        // header.
        assert!(script.contains(&format!(
            "if rindex >= {RECIPE_STATE_WORDS} goto recipe_word_end"
        )));
        const { assert!(HOST_RECIPES / 4 + RECIPE_STATE_WORDS <= HEADER_SIZE / 4) };
        // The names, the import hash, and the IDs follow the recipe words, so
        // one run of words holds them.
        const {
            assert!(crate::abi::HOST_RECIPE_NAMES == HOST_RECIPES + 4 * RECIPE_WORDS * RECIPE_SLOTS)
        };
        const {
            assert!(
                crate::abi::HOST_RECIPE_IMPORTED
                    == crate::abi::HOST_RECIPE_NAMES + 4 * RECIPE_NAME_WORDS * RECIPE_SLOTS
            )
        };
        const {
            use crate::abi::{HOST_RECIPE_IDS, HOST_RECIPE_IMPORTED};
            assert!(HOST_RECIPE_IDS == HOST_RECIPE_IMPORTED + 4);
            assert!(HOST_RECIPE_IDS / 4 + RECIPE_SLOTS == HOST_RECIPES / 4 + RECIPE_STATE_WORDS)
        };
        assert_eq!(script.matches(&recipes).count(), 1);
        // Each index of the settings of the user modes is below
        // DIAL_SET_STATE_WORDS, which ends in the header with the copy of
        // the switch, Enable at Start, Distance Unit, and Uncropped RAW.
        assert!(script.contains(&format!(
            "if rindex >= {DIAL_SET_STATE_WORDS} goto set_word_end"
        )));
        const {
            assert!(
                HOST_DIAL_SETS / 4 + DIAL_SET_STATE_WORDS
                    == crate::abi::HOST_MANAGE_COPY / 4 + crate::abi::MANAGE_COPY_WORDS
                    && HOST_DIAL_SETS / 4 + DIAL_SET_STATE_WORDS <= HEADER_SIZE / 4
            )
        };
        assert_eq!(script.matches(&sets).count(), 1);
    }

    /// The interpreter reads a whole file when it includes it, so the start
    /// reads only the steps up to INSTALL. The resident part comes after
    /// INSTALL, in its own file, which jumps only to its own labels.
    #[test]
    fn the_loader_includes_its_resident_part_after_install() {
        let releases = [FastRelease::new(&crate::camera::GR4, "1.20").unwrap()];
        for loader in [loader(), loader_with(ICON_STRIDE, &releases)] {
            let loader = String::from_utf8(loader).unwrap();
            let lines: Vec<&str> = loader.split("\r\n").collect();
            let installed = lines.iter().position(|l| *l == ":installed").unwrap();
            assert_eq!(
                lines[installed + 1..installed + 6],
                [
                    "filedelete 'C:\\SCRIPT\\GRKIT\\INSTALL.ERR'",
                    "filesearch 'C:\\SCRIPT\\GRKIT\\RESIDENT.TTL'",
                    "if result = 0 end",
                    "include 'C:\\SCRIPT\\GRKIT\\RESIDENT.TTL'",
                    "end",
                ]
            );
            assert!(!lines.contains(&":resident"));
            assert!(!lines.contains(&":watch"));
        }
        let resident = String::from_utf8(resident()).unwrap();
        assert!(resident.starts_with(":resident\r\n"));
        let lines: Vec<&str> = resident.split("\r\n").collect();
        let labels: Vec<&str> = lines.iter().filter_map(|l| l.strip_prefix(':')).collect();
        for line in &lines {
            for keyword in ["goto ", "call "] {
                // A jump, not text in a string.
                if let Some(at) = line.find(keyword).filter(|at| !line[..*at].contains('\'')) {
                    let target = line[at + keyword.len()..]
                        .split_whitespace()
                        .next()
                        .unwrap();
                    assert!(labels.contains(&target), "{line}");
                }
            }
        }
        // The two parts together are smaller than one loader before the split.
        assert!(loader().len() < 12 * 1024, "{}", loader().len());
    }

    /// The resident loader starts the service while the access point is on
    /// and stops it when the access point goes off, at most SERVE_POLLS
    /// requests in a row, after `lcmd_resp on`. A runtime without the service
    /// or a mailbox makes no request.
    #[test]
    fn the_resident_script_runs_the_service_with_the_access_point() {
        let script = state_script();
        let gate = at(&script, "hasserve = 0");
        let checked = at(&script, "if result <> 0 hasserve = 1");
        assert_eq!(
            script[checked - 1],
            "filesearch 'C:\\SCRIPT\\GRKIT\\SERVE.BIN'"
        );
        assert!(
            script[gate..checked]
                .iter()
                .filter(|line| line.contains("goto resident_serve"))
                .count()
                == 2
        );
        assert!(crate::card::SERVE_FILE.ends_with(&format!("/{SERVE_NAME}")));
        let watch = at(&script, ":watch");
        assert!(checked < watch);
        assert_eq!(script[watch + 2], "if hasserve <> 0 call serve");
        let serve = &script[at(&script, ":serve")..at(&script, ":serve_settled") + 3];
        let line = |text: &str| serve.iter().position(|line| line == text).unwrap();
        let resp = line("if respon = 0 sendln 'lcmd_resp on'");
        let start = line("sendln 'lcmd sh /media/storage1/SCRIPT/GRKIT/SCAN.SH serve'");
        let stop = line("sendln 'lcmd sh /media/storage1/SCRIPT/GRKIT/SCAN.SH stop'");
        assert!(
            line("if running = wlan goto serve_settled")
                < line(&format!("if servepolls >= {SERVE_POLLS} return"))
        );
        assert!(line(&format!("if servepolls >= {SERVE_POLLS} return")) < resp);
        assert!(
            resp < line("if wlan = 0 goto serve_stop")
                && line("if wlan = 0 goto serve_stop") < start
        );
        assert!(start < line(":serve_stop") && line(":serve_stop") < stop);
        assert_eq!(serve[serve.len() - 2], "servepolls = 0");
        // The subroutine follows a return, so no path falls into it.
        assert_eq!(script[at(&script, ":serve") - 1], "return");
    }

    /// The resident loader shows an update over Wi-Fi that waits, renames its
    /// journal after the confirmation, and lets an update go after
    /// UPDATE_LOOKS looks. It looks at the card only at every UPDATE_POLLS
    /// polls, and reads the confirmation at each poll.
    #[test]
    fn the_resident_script_makes_a_confirmed_update_run_at_the_next_start() {
        let script = state_script();
        let watch = at(&script, ":watch");
        assert_eq!(script[watch + 3], "call update_poll");
        assert!(at(&script, "updatepolls = 0") < watch && at(&script, "updatelooks = 0") < watch);
        let word = format!("registry[{}]", HOST_UPDATE / 4);
        let update = at(&script, ":update_poll");
        assert_eq!(script[update - 1], "return");
        let body = &script[update..];
        let line = |text: &str| at(body, text);
        // The confirmation, a header word, at each poll; the card only at
        // every UPDATE_POLLS polls.
        assert!(
            line(&format!(
                "if updstate = {UPDATE_CONFIRMED} goto update_confirmed"
            )) < line(&format!("if updatepolls < {UPDATE_POLLS} return"))
        );
        assert!(
            line(&format!("if updatepolls < {UPDATE_POLLS} return"))
                < line("filesearch 'C:\\SCRIPT\\GRKIT\\COMMIT.REQ'")
        );
        let waits = line(":update_waits");
        assert_eq!(
            body[waits..waits + 9],
            [
                ":update_waits".to_owned(),
                "updatelooks = updatelooks + 1".into(),
                format!("if updatelooks >= {UPDATE_LOOKS} goto update_late"),
                format!("if updstate = {UPDATE_WAITING} return"),
                "call update_version".into(),
                format!("{word} = {UPDATE_WAITING}"),
                format!("registry[{}] = {CUE_UPDATE_READY}", HOST_CUE_LABEL / 4),
                format!("registry[{}] = {CUE_PENDING}", HOST_LOAD_CUE / 4),
                "return".into(),
            ]
        );
        // At the start after an update installed, the cue reads GRKit
        // Updated, and the version file goes.
        let updated = at(&script, ":resident_updated");
        assert!(updated < watch);
        assert_eq!(
            script[updated - 6..updated],
            [
                "filesearch 'C:\\SCRIPT\\GRKIT\\COMMIT.VER'".to_owned(),
                "if result = 0 goto resident_updated".into(),
                "filesearch 'C:\\SCRIPT\\GRKIT\\COMMIT.REQ'".into(),
                "if result <> 0 goto resident_updated".into(),
                format!("registry[{}] = {CUE_UPDATED}", HOST_CUE_LABEL / 4),
                "filedelete 'C:\\SCRIPT\\GRKIT\\COMMIT.VER'".into(),
            ]
        );
        // The version of the update, before the row shows.
        let version = line(":update_version");
        assert_eq!(
            body[version + 3],
            "fileopen updfh 'C:\\SCRIPT\\GRKIT\\COMMIT.VER' 0"
        );
        assert_eq!(
            body[line(":update_version_end") + 1],
            format!("registry[{}] = updversion", HOST_UPDATE_VERSION / 4)
        );
        // An update that nobody confirms goes.
        let late = line(":update_late");
        assert_eq!(body[late + 1], "filedelete 'C:\\SCRIPT\\GRKIT\\COMMIT.REQ'");
        assert_eq!(body[late + 2], "filedelete 'C:\\SCRIPT\\GRKIT\\COMMIT.VER'");
        assert_eq!(body[late + 4], format!("{word} = 0"));
        // After the confirmation, the journal runs at the next start.
        let confirmed = line(":update_confirmed");
        assert_eq!(
            body[confirmed..confirmed + 10],
            [
                ":update_confirmed".to_owned(),
                "filesearch 'C:\\SCRIPT\\GRKIT\\COMMIT.REQ'".into(),
                "if result = 0 goto update_renamed".into(),
                "filerename 'C:\\SCRIPT\\GRKIT\\COMMIT.REQ' 'C:\\SCRIPT\\GRKIT\\COMMIT.TTL'".into(),
                ":update_renamed".into(),
                "updatelooks = 0".into(),
                "filesearch 'C:\\SCRIPT\\GRKIT\\COMMIT.TTL'".into(),
                "if result = 0 goto update_none".into(),
                format!("{word} = {UPDATE_READY}"),
                "return".into(),
            ]
        );
        assert!(crate::card::REQUEST_FILE.ends_with("/COMMIT.REQ"));
        assert!(crate::card::JOURNAL_FILE.ends_with("/COMMIT.TTL"));
    }

    /// The interpreter stops at a variable that has the name of a label
    /// ("Type mismatch"): no script names a variable as a label.
    #[test]
    fn no_script_names_a_variable_as_a_label() {
        let fast = FastRelease::new(&crate::camera::GR3, "2.21").unwrap();
        for script in [loader(), loader_with(ICON_STRIDE, &[fast]), resident()] {
            let text = String::from_utf8(script).unwrap();
            let labels: std::collections::BTreeSet<&str> = text
                .lines()
                .filter_map(|line| line.strip_prefix(':'))
                .collect();
            for line in text.lines() {
                let statement = match line.strip_prefix("if ") {
                    Some(rest) => rest.splitn(4, ' ').nth(3).unwrap_or_default(),
                    None => line,
                };
                if let Some((target, _)) = statement.split_once(" = ") {
                    let name = target.split('[').next().unwrap_or_default();
                    assert!(!labels.contains(name), "{line}");
                }
            }
        }
    }

    /// The interpreter takes an assignment to an array element only on a line
    /// of its own: no script assigns one after `if`.
    #[test]
    fn no_script_assigns_an_array_element_after_if() {
        let fast = FastRelease::new(&crate::camera::GR3, "2.21").unwrap();
        for script in [loader(), loader_with(ICON_STRIDE, &[fast]), resident()] {
            for line in String::from_utf8(script).unwrap().lines() {
                let Some(rest) = line.strip_prefix("if ") else {
                    continue;
                };
                let statement = rest.splitn(4, ' ').nth(3).unwrap_or_default();
                let target = statement.split(" = ").next().unwrap_or_default();
                assert!(
                    !(statement.contains(" = ") && target.contains('[')),
                    "{line}"
                );
            }
        }
    }

    /// The main file follows only its immutable content revision. Other
    /// files and temporary U-mode changes cannot starve its debounce.
    #[test]
    fn the_resident_script_saves_after_the_selections_settle() {
        let script = state_script();
        let start = at(&script, ":watch");
        assert_eq!(script[start + watch_idle("watch").len()], "call capture");
        assert!(!script
            .iter()
            .any(|line| line.contains("hold") || line.contains("total =")));
        for (name, poll) in [
            ("watch", watch_poll("watch")),
            ("profile_poll", watch_poll("profile_poll")),
        ] {
            let save = at(&poll, "call save_state");
            assert!(
                at(
                    &poll,
                    &format!("if changes <> last goto {name}_state_release")
                ) < save
            );
            assert!(
                at(
                    &poll,
                    &format!("if changes = saved goto {name}_state_release")
                ) < save
            );
            let release = at(&poll, "call release_capture");
            assert!(save < release);
            for file in [StateFile::Recipes, StateFile::DialSets] {
                let (changes, offset, saved, _) = watch_names(file);
                let read = at(&poll, &format!("{changes} = registry[{}]", offset / 4));
                assert!(release < read);
                assert_eq!(
                    poll[read + 1],
                    format!(
                        "if {changes} <> {changes}last goto {name}_{}_saved",
                        label(file)
                    )
                );
                assert_eq!(
                    poll[read + 2],
                    format!("if {changes} = {saved} goto {name}_{}_saved", label(file))
                );
                assert!(script[..start].contains(&format!("{saved} = 0")));
                assert!(script[..start].contains(&format!("{changes}last = {changes}")));
            }
        }
        // INSTALL's baseline is fixed even if App edits occur before resident.
        assert!(script[..start].contains(&"changes = 1".into()));
        assert!(script[..start].contains(&"saved = changes".into()));
        assert!(script[..start].contains(&"last = changes".into()));
        for file in StateFile::ALL {
            let (_, _, saved, stamp) = watch_names(file);
            let ack = at(&script, &format!("{saved} = {stamp}"));
            assert_eq!(
                script[ack - 1],
                format!("if result = 0 goto save_{}_end", label(file))
            );
            assert_eq!(
                script[ack - 2],
                format!("filesearch '{}'", state_path(file, "TXT"))
            );
        }
    }

    /// An idle poll reads the change counters and asks for no export. A
    /// counter that moved, a pending save, and each `STATE_IDLE_POLLS` idle
    /// polls give a complete poll.
    #[test]
    fn the_resident_exports_only_after_a_counter_moves() {
        let script = state_script();
        let start = at(&script, ":watch");
        let idle = watch_idle("watch");
        assert_eq!(script[start..start + idle.len()], idle);
        assert_eq!(idle[1], format!("mpause {STATE_POLL_MS}"));
        assert_eq!(
            idle[4],
            format!(
                "hint = registry[{}] + registry[{}] + registry[{}]",
                HOST_STATE_CHANGES / 4,
                HOST_RECIPE_CHANGES / 4,
                HOST_DIAL_SET_CHANGES / 4
            )
        );
        assert_eq!(
            idle[5..8],
            [
                format!("if hint <> quiet idle = {STATE_IDLE_POLLS}"),
                "idle = idle + 1".to_string(),
                format!("if idle <= {STATE_IDLE_POLLS} goto watch"),
            ]
        );
        // After the service start and the update poll, the idle loop has no
        // call, no export word, and no file command.
        assert_eq!(idle[3], "call update_poll");
        assert!(!idle[4..8].iter().any(|line| line.contains("call ")
            || line.contains("modulecode[")
            || line.contains("file")));
        // A complete poll starts as not quiet, and the first poll is complete.
        assert_eq!(idle[idle.len() - 1], "quiet = hint - 1");
        assert!(script[..start].contains(&"quiet = 0 - 1".into()));
        assert!(script[..start].contains(&"hint = 0".into()));
        assert!(script[..start].contains(&"idle = 0".into()));
        // Only a poll with an export and no pending save ends quiet.
        for name in ["watch", "profile_poll"] {
            let poll = watch_poll(name);
            let quiet = at(&poll, "quiet = hint");
            assert_eq!(poll[quiet + 1], format!("goto {name}"));
            assert_eq!(quiet + 2, poll.len());
            assert_eq!(
                poll[quiet - 4..quiet],
                [
                    format!("if captured = 0 goto {name}"),
                    format!("if changes <> saved goto {name}"),
                    format!("if rchanges <> rsaved goto {name}"),
                    format!("if dchanges <> dsaved goto {name}"),
                ]
            );
            assert!(at(&poll, "call release_capture") < quiet - 4);
        }
    }

    /// A timed-out request remains outstanding. A completed request is held
    /// through the final write, and malformed counts release without saving.
    #[test]
    fn the_resident_keeps_the_export_leased_through_io() {
        let capture = persist_capture();
        let request = at(
            &capture,
            &format!("{} = ticket", persist_word(PERSIST_REQUEST)),
        );
        assert!(at(&capture, "if ticket <> released goto capture_wait") < request);
        assert!(request < at(&capture, ":capture_wait"));
        assert!(at(&capture, "if attempts >= 5 return") < at(&capture, ":capture_ready"));
        let release = at(
            &capture,
            &format!("{} = ticket", persist_word(PERSIST_RELEASE)),
        );
        assert!(at(&capture, ":capture_invalid") < release);
        assert_eq!(capture[at(&capture, ":capture_ready") + 1], "mpause 1");
        assert_eq!(capture[release - 1], "mpause 1");
        let script = state_script();
        let save = &script[at(&script, ":save_state")..at(&script, ":save_state_end")];
        assert!(!save.iter().any(|line| line.contains("registry[")
            || line.contains(&format!("{} =", persist_word(PERSIST_RELEASE)))));
        let snapshot = &script[at(&script, ":snapshot")..at(&script, ":capture")];
        assert!(!snapshot.iter().any(|line| line.contains("registry[")));
        assert_eq!(
            snapshot
                .iter()
                .filter(|line| line.contains("modulecode["))
                .count(),
            capabilities::state_keys().len()
        );
    }

    /// The services-free installation keeps its original counter baseline
    /// and never accesses a nonexistent module array or waits for App export.
    #[test]
    fn module_free_installs_keep_the_legacy_state_path() {
        let script = state_script();
        let start = at(&script, "legacy = 0");
        let enabled = at(&script, "legacy = 1");
        assert!(start < enabled);
        assert!(script[start..enabled].contains(&"if modulewords <> 0 goto resident_export".into()));
        assert!(script[start..enabled].contains(&"if export <> 0 goto resident_export".into()));
        let baseline = at(
            &script,
            &format!(
                "if legacy <> 0 changes = registry[{}]",
                HOST_STATE_CHANGES / 4
            ),
        );
        assert!(baseline < at(&script, "saved = changes"));
        let capture = persist_capture();
        assert_eq!(capture[2], "if legacy <> 0 goto legacy_capture");
        let legacy = legacy_capture();
        assert!(!legacy.iter().any(|line| line.contains("modulecode[")));
        assert!(legacy.contains(&"mcount = 0".into()));
        assert!(legacy.contains(&format!("if pcount > {MAX_COUNT} pcount = 0")));
        let poll = watch_poll("watch");
        let release = at(&poll, "call release_capture");
        assert_eq!(poll[release - 1], "if legacy <> 0 goto watch_state_saved");
    }

    fn at(script: &[String], line: &str) -> usize {
        script
            .iter()
            .position(|l| l == line)
            .unwrap_or_else(|| panic!("{line}"))
    }

    /// The state files are in the data folder, which holds the loader. Without
    /// the folder, no save writes. The one other write renames the journal of
    /// a confirmed update over Wi-Fi in the package folder (`update_script`).
    #[test]
    fn the_loader_saves_in_the_data_folder() {
        let script = state_script();
        let check = at(&script, "if result = 0 stateok = 0");
        assert_eq!(script[check - 1], "foldersearch 'C:\\GRKIT'");
        assert!(check < at(&script, ":watch"));
        for file in StateFile::ALL {
            let save = at(&script, &format!(":save_{}", label(file)));
            assert_eq!(
                script[save + 1],
                format!("if stateok = 0 goto save_{}_end", label(file))
            );
        }
        let update = "filerename 'C:\\SCRIPT\\GRKIT\\COMMIT.REQ' 'C:\\SCRIPT\\GRKIT\\COMMIT.TTL'";
        assert_eq!(script.iter().filter(|l| *l == update).count(), 1);
        let writes: Vec<_> = script
            .iter()
            .filter(|l| l.starts_with("filecreate") || l.starts_with("filerename"))
            .filter(|l| *l != update)
            .collect();
        assert_eq!(writes.len(), 2 * StateFile::ALL.len());
        assert!(
            writes.iter().all(|l| l.contains("C:\\GRKIT\\")),
            "{writes:?}"
        );
    }

    /// A start that stops leaves its step in INSTALL.ERR: the loader writes
    /// the status of each step before its first request, keeps the status
    /// of a failed SCAN or INSTALL, and removes the file after the install.
    #[test]
    fn the_loader_leaves_the_status_of_each_step() {
        let script = loader();
        let script = String::from_utf8_lossy(&script);
        let lines: Vec<_> = script.lines().collect();
        let at = |line: &str| {
            lines
                .iter()
                .position(|l| *l == line)
                .unwrap_or_else(|| panic!("{line}"))
        };
        let after = |from: usize, line: &str| {
            from + lines[from..]
                .iter()
                .position(|l| *l == line)
                .unwrap_or_else(|| panic!("{line}"))
        };
        // No step deletes the status of the last start before it writes its own.
        let first = at("call write_error");
        assert!(lines[..first].iter().all(|l| !l.contains("INSTALL.ERR")));
        assert_eq!(lines[first - 1], "failure = 60");
        assert!(first < at(":scan"));
        let install = at(":install");
        assert_eq!(lines[install - 2], "failure = 62");
        assert_eq!(lines[install - 1], "call write_error");
        assert!(at("failure = 61") > at(":plan_ready"));
        // A status of INSTALL stops the loader; its own status waits.
        let read = after(install, "call read_error");
        assert_eq!(lines[read + 1], "if errorstatus = 62 goto install_wait");
        assert_eq!(lines[read + 2], "if errorstatus != 0 goto reject");
        let installed = at(":installed");
        assert_eq!(
            lines[installed + 1],
            "filedelete 'C:\\SCRIPT\\GRKIT\\INSTALL.ERR'"
        );
        // A stop writes the status of the step only over a status of the
        // loader, or without a file.
        let reject = at(":reject");
        assert_eq!(
            &lines[reject + 1..reject + 9],
            [
                "call read_error",
                "if errorstatus = 0 goto reject_write",
                "if errorstatus = 60 goto reject_write",
                "if errorstatus = 62 goto reject_write",
                "goto reject_end",
                ":reject_write",
                "call write_error",
                ":reject_end",
            ]
        );
        // The status file is read byte by byte: a filereadln of a last line
        // without a newline blocks the interpreter.
        assert!(!script.contains("filereadln"));
    }

    /// INSTALL restores the state before the hooks start (runtime/state.h),
    /// so the loader only writes the file. Its start is the restored values.
    #[test]
    fn the_loader_reads_no_state_file() {
        let script = loader();
        let script = String::from_utf8_lossy(&script);
        let opens: Vec<_> = script
            .lines()
            .filter(|l| l.starts_with("fileopen"))
            .collect();
        // The plan, the status of INSTALL.ERR, and the profile file, which
        // the loader appends to.
        assert!(
            opens.iter().all(|l| l.contains("PLAN.BIN")
                || l.ends_with("INSTALL.ERR' 0")
                || l.ends_with("PROFILE.LOG' 1")),
            "{opens:?}"
        );
        assert!(!script.contains("SCRIPT\\PLUGINS"));
        let state = state_script();
        let start = at(&state, "saved = changes");
        assert!(start < at(&state, ":watch"));
    }

    /// INSTALL restores the state of GRKit 1.2 when the data folder has no
    /// state file, so the loader then saves at the first poll.
    #[test]
    fn the_loader_saves_at_once_without_a_state_file() {
        let state = state_script();
        let force = at(&state, "if result = 0 saved = changes - 1");
        assert_eq!(state[force - 1], "filesearch 'C:\\GRKIT\\STATE.TXT'");
        assert!(at(&state, "saved = changes") < force);
        assert!(force < at(&state, ":watch"));
    }

    /// A core key with value 0 has no entry, because a missing core key restores 0.
    #[test]
    fn the_loader_writes_only_keys_with_a_value() {
        let script = state_script();
        for (i, key) in capabilities::state_keys().iter().enumerate() {
            let write = at(
                &script,
                &format!("filewriteln sfh '{}'", state_key_id(&key.name)),
            );
            assert_eq!(
                script[write - 1],
                format!("if value{i} = 0 goto save_key{i}")
            );
            assert_eq!(script[write + 3], format!(":save_key{i}"));
        }
    }
    /// Module settings must persist zero even when their default is nonzero.
    #[test]
    fn the_loader_saves_module_zero() {
        let script = state_script();
        let start = at(&script, ":save_modules");
        let end = at(&script, ":save_modules_next");
        let lines = &script[start..end];
        assert!(lines.contains(&"if mid <= 0 goto save_modules_next".to_owned()));
        assert!(lines.contains(&"if mvalue < 0 goto save_modules_next".to_owned()));
        assert!(!lines.contains(&"if mvalue <= 0 goto save_modules_next".to_owned()));
        assert!(lines.contains(&"int2str line mvalue".to_owned()));
    }

    /// The official firmware has another command under the name of the
    /// fast-boot loader, so the loader sends it only after the probe found
    /// the fast-boot loader and its file is on the card. A first array comes
    /// before the planned arrays, with or without fast-boot files.
    #[test]
    fn the_loader_runs_code_from_the_card_only_after_the_probe() {
        use crate::camera::GR3;
        let releases = ["2.10", "2.20"].map(|firmware| FastRelease::new(&GR3, firmware).unwrap());
        let script = String::from_utf8(loader_with(ICON_STRIDE, &releases)).unwrap();
        let lines: Vec<_> = script.lines().collect();
        let at = |line: &str| {
            lines
                .iter()
                .position(|l| *l == line)
                .unwrap_or_else(|| panic!("{line}"))
        };
        let first_array = lines.iter().position(|l| l.starts_with("intdim ")).unwrap();
        assert_eq!(lines[first_array], "intdim ba 256");
        let probe = at("sendln 'SYS_buildinfo 0'");
        assert!(first_array < probe);
        // The probe words are 0 before the probe, so stale memory never
        // passes it on the official firmware.
        assert_eq!(lines[probe - 2..probe], ["ba[0] = 0", "ba[1] = 0"]);
        assert_eq!(
            lines[probe + 1..probe + 7],
            [
                "if ba[0] <> 1129202771 goto probed",
                "fastreason = 2",
                "fastinterface = ba[1]",
                "if ba[1] <> 1 goto probed",
                "fastreason = 3",
                "release = ba[2] * 1000 + ba[3]",
            ]
        );
        let probed = at(":probed");
        // Only the probe sets a release, and only with its file on the card.
        for (index, line) in lines.iter().enumerate() {
            if line.contains("fast = ")
                && !line.ends_with("fast = 0")
                && !line.starts_with("if fast")
            {
                assert!(probe < index && index < probed, "{line}");
                assert!(lines[index - 1].starts_with("filesearch "), "{line}");
            }
        }
        for (index, release) in releases.iter().enumerate() {
            let number = release.number();
            let path = release.rtos_path();
            let install = release.install_rtos_path();
            // Both files of the release must be on the card.
            let set = at(&format!("if result <> 0 fast = {number}"));
            assert_eq!(
                lines[set - 5..set],
                [
                    format!("if release <> {number} goto probe{}", index + 1),
                    "fastreason = 4".into(),
                    format!("filesearch '{path}'"),
                    format!("if result = 0 goto probe{}", index + 1),
                    format!("filesearch '{install}'"),
                ]
            );
            at(&format!(
                "if fast = {number} sendln 'SYS_memovr {path} 0 call'"
            ));
            assert!(
                at(":install")
                    < at(&format!(
                        "if fast = {number} sendln 'SYS_memovr {install} 0 call'"
                    ))
            );
        }
        for line in lines.iter().filter(|l| l.contains("SYS_memovr")) {
            assert!(line.starts_with("if fast "), "{line}");
        }
        // The status word is "not read" before the fast-boot file runs.
        let scan = at("if fast = 0 goto linux");
        assert_eq!(lines[scan + 1], "ba[254] = 999");
        assert_eq!(lines[scan + 2 + releases.len()], "faststatus = ba[254]");
        // A fast-boot SCAN without a plan notes its status and falls back to
        // Linux. Only the Linux steps write INSTALL.ERR before their requests.
        let linux = at(":linux");
        // A SCAN that an earlier failed fast INSTALL turned off (995) notes
        // that instead.
        assert_eq!(
            &lines[linux - 5..linux + 4],
            [
                "if faststatus = 0 goto plan_ready",
                "fastreason = 6",
                "if faststatus = 995 fastreason = 5",
                "call fast_log",
                "fast = 0",
                ":linux",
                "filedelete 'C:\\SCRIPT\\GRKIT\\PLAN.BIN'",
                "filedelete 'C:\\SCRIPT\\GRKIT\\PLAN.TMP'",
                "filedelete 'C:\\SCRIPT\\GRKIT\\INSTALL.OK'",
            ]
        );
        // The fast-boot steps find no file of the Linux steps and leave none,
        // so only the Linux steps remove them and check that they are gone.
        assert_eq!(
            &lines[linux + 4..linux + 12],
            [
                "filedelete 'C:\\SCRIPT\\GRKIT\\INSTALL.LCK'",
                "filesearch 'C:\\SCRIPT\\GRKIT\\PLAN.BIN'",
                "if result != 0 goto reject",
                "filesearch 'C:\\SCRIPT\\GRKIT\\INSTALL.OK'",
                "if result != 0 goto reject",
                "call write_error",
                "mpause 2000",
                "sendln 'lcmd_resp on'",
            ]
        );
        assert!(lines[..linux].iter().all(|l| !l.starts_with("filedelete")));
        assert!(at("if fast = 0 goto linux") < linux);
        assert_eq!(
            lines.iter().filter(|l| **l == "call write_error").count(),
            2
        );
        assert!(at("if fast = 0 call write_error") > linux);
        // The fast-boot SCAN gives the plan header in the script array: the
        // loader reads no plan file.
        let fast_plan = at(":plan_fast");
        assert_eq!(
            &lines[fast_plan - 1..fast_plan + 8],
            [
                "goto plan_read",
                ":plan_fast",
                "planmagic = ba[128]",
                "count = ba[129]",
                "words = ba[130]",
                "chunks = ba[131]",
                "modulewords = ba[133]",
                "moduletiles = ba[134]",
                ":plan_read",
            ]
        );
        assert_eq!(lines[at(":plan_ready") + 2], "if fast <> 0 goto plan_fast");
        at("if fast = 0 sendln 'lcmd sh /media/storage1/SCRIPT/GRKIT/INSTALL.SH'");

        // Without fast-boot files, the loader asks for no probe and runs no
        // code from the card.
        let plain = String::from_utf8(loader()).unwrap();
        assert!(!plain.contains("SYS_buildinfo"));
        assert!(!plain.contains("SYS_memovr"));
        assert!(!plain.contains("allocated"));
        assert!(plain.contains("\r\nintdim ba 256\r\nfast = 0\r\n"));
        assert!(plain.contains("\r\nif errorstatus != 0 goto reject\r\n"));
    }

    /// A failed fast-boot INSTALL runs SCAN and INSTALL through Linux. The
    /// planned arrays exist then, so the second plan must have their sizes,
    /// and the loader does not allocate them again.
    #[test]
    fn a_failed_fast_install_falls_back_to_linux_with_the_same_arrays() {
        use crate::camera::GR4;
        let releases = [FastRelease::new(&GR4, "1.20").unwrap()];
        let script = String::from_utf8(loader_with(ICON_STRIDE, &releases)).unwrap();
        let lines: Vec<_> = script.lines().collect();
        let at = |line: &str| {
            lines
                .iter()
                .position(|l| *l == line)
                .unwrap_or_else(|| panic!("{line}"))
        };
        assert!(at("allocated = 0") < at(":linux"));
        // The arrays are allocated once.
        let check = at("if allocated <> 0 goto check_arrays");
        assert_eq!(lines[check + 1], "intdim registry words");
        let tail = at(&format!("intdim tailarea {TAIL_WORDS}"));
        assert_eq!(
            lines[tail + 1..tail + 20],
            [
                "allocated = 1",
                "hadcount = count",
                "hadwords = words",
                "hadchunks = chunks",
                "hadmodwords = modulewords",
                "hadmodtiles = moduletiles",
                "goto arrays_ready",
                ":check_arrays",
                "if count <> hadcount goto plan_differs",
                "if words <> hadwords goto plan_differs",
                "if chunks <> hadchunks goto plan_differs",
                "if modulewords <> hadmodwords goto plan_differs",
                "if moduletiles <> hadmodtiles goto plan_differs",
                "goto arrays_ready",
                // Another plan: both headers go to the boot log.
                ":plan_differs",
                "call plan_log",
                "goto reject",
                ":arrays_ready",
                "attempts = 0",
            ]
        );
        assert_eq!(
            lines
                .iter()
                .filter(|l| l.starts_with("intdim registry"))
                .count(),
            1
        );
        // An install file that did not run INSTALL, also after a change of
        // the memory of the fast-boot file (996, 997), or a failure status of
        // INSTALL, goes to the fallback. It notes the status, removes the
        // results of the fast-boot steps, and runs SCAN.SH.
        // The fast-boot INSTALL gives its status in the script array: it
        // writes no INSTALL.OK, and the loader tries again after a not-ready
        // status.
        let shell = at("if fast = 0 sendln 'lcmd sh /media/storage1/SCRIPT/GRKIT/INSTALL.SH'");
        assert_eq!(
            lines[shell + 1..shell + 13],
            [
                "if fast = 0 goto install_linux",
                "faststatus = ba[254]",
                "if faststatus = 0 goto installed",
                "if faststatus >= 996 goto install_not_run",
                "if faststatus = 12 goto install_wait",
                "if faststatus = 14 goto install_wait",
                "if faststatus = 26 goto install_wait",
                "if faststatus = 39 goto install_wait",
                "errorstatus = faststatus",
                "goto install_failed",
                ":install_linux",
                "filesearch 'C:\\SCRIPT\\GRKIT\\INSTALL.OK'",
            ]
        );
        assert_eq!(lines[at(":install") + 1], "ba[254] = 999");
        at("if errorstatus != 0 goto install_failed");
        let not_run = at(":install_not_run");
        assert_eq!(
            lines[not_run..not_run + 19],
            [
                ":install_not_run",
                "errorstatus = faststatus",
                ":install_failed",
                // The detail words: a changed page or block, or a plan check.
                "fastaddress = ba[252]",
                "fastcount = ba[253]",
                "if fast = 0 goto reject",
                "failure = errorstatus",
                "if errorstatus = 24 goto reject",
                "faststatus = errorstatus",
                "fastreason = 7",
                "call fast_log",
                "fast = 0",
                "attempts = 0",
                "failure = 60",
                "goto linux",
                ":reject_close",
                "fileclose fh",
                ":reject",
                "call read_error",
            ]
        );
        // The fallback is reachable only through its labels.
        assert_eq!(lines[not_run - 1], "end");
    }

    /// A start that does not use its fast-boot files, or stops using them,
    /// writes the reason to the boot log, which a person sends with a
    /// report. A start that installs through them writes nothing there.
    #[test]
    fn a_start_without_the_fast_boot_files_notes_why_in_the_boot_log() {
        use crate::camera::GR4;
        let releases = [FastRelease::new(&GR4, "1.20").unwrap()];
        let script = String::from_utf8(loader_with(ICON_STRIDE, &releases)).unwrap();
        let lines: Vec<_> = script.lines().collect();
        let at = |line: &str| {
            lines
                .iter()
                .position(|l| *l == line)
                .unwrap_or_else(|| panic!("{line}"))
        };
        // Three calls: after the probe, after SCAN, after INSTALL. Each sets
        // the reason first, and each but the first ends the fast-boot steps.
        let calls: Vec<_> = lines
            .iter()
            .enumerate()
            .filter(|(_, l)| l.contains("call fast_log"))
            .map(|(index, _)| index)
            .collect();
        assert_eq!(calls.len(), 3);
        assert_eq!(lines[calls[0]], "if fast = 0 call fast_log");
        assert_eq!(lines[calls[0] - 1], ":probed");
        for &call in &calls[1..] {
            assert_eq!(lines[call], "call fast_log");
            assert!(
                lines[call - 1].starts_with("fastreason = ")
                    || lines[call - 1] == "if faststatus = 995 fastreason = 5"
            );
            assert_eq!(lines[call + 1], "fast = 0");
        }
        // The subroutine appends one line to GRKIT/BOOT.TXT.
        let log = at(":fast_log");
        assert!(log > at("end"));
        for line in [
            "fileopen lfh 'C:\\GRKIT\\BOOT.TXT' 1",
            "filecreate lfh 'C:\\GRKIT\\BOOT.TXT'",
            "if result = 0 foldercreate 'C:\\GRKIT'",
            "filewrite lfh 'fast-boot'",
            "if fastreason = 1 filewrite lfh ' off: official firmware'",
            "if fastreason = 4 filewrite lfh ' off: files missing'",
            "if fastreason = 5 filewrite lfh ' off: an earlier fast install failed'",
            "if fastreason = 7 filewrite lfh ' install '",
            "if faststatus = 37 goto fast_log_check",
            "filewrite lfh ' check '",
            "if fastreason >= 6 filewrite lfh ', Linux used'",
            "filewrite lfh 'fast plan'",
            "filewrite lfh ', Linux plan'",
            "filewriteln lfh ''",
            "fileclose lfh",
        ] {
            assert!(lines[log..].contains(&line), "{line}");
        }
        assert_eq!(lines.last(), Some(&"return"));
        // Without fast-boot files, the loader has no reason to note.
        let plain = String::from_utf8(loader()).unwrap();
        assert!(!plain.contains("fast_log"));
        assert!(!plain.contains("fastreason"));
    }
}
