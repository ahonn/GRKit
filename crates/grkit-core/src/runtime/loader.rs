//! The loader script (`LOADER.TTL`): it waits for Linux and the camera, runs
//! SCAN and INSTALL, then stays resident to restore and save the state.

use super::*;
use crate::abi::{
    HOST_PROCESSING_CALLS, HOST_PROFILE, PROFILE, PROFILE_KINDS, PROFILE_MODULES,
    PROFILE_MODULE_SIZE, PROFILE_POINTS, PROFILE_SLOTS, PROFILE_SLOT_SIZE,
};
use crate::capabilities::{self, StateFile};
use crate::card::{DATA_FOLDER, LINUX_DIRECTORY, RTOS_PACKAGE_FOLDER};

/// The resident loader reads the selections at this interval. It writes the
/// state file when two reads in a row agree, so a change reaches the card
/// one to two intervals after the last change.
pub const STATE_POLL_MS: u32 = 2000;
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
            "if count <> hadcount goto reject".into(),
            "if words <> hadwords goto reject".into(),
            "if chunks <> hadchunks goto reject".into(),
            "if modulewords <> hadmodwords goto reject".into(),
            "if moduletiles <> hadmodtiles goto reject".into(),
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
            format!("fastaddress = ba[{FAST_DETAIL_INDEX}]"),
            format!("fastcount = ba[{}]", FAST_DETAIL_INDEX + 1),
            ":install_failed".into(),
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
/// the package, a missing fast-boot or install file, or a failed SCAN or
/// INSTALL.
const FAST_OFF_OFFICIAL: u32 = 1;
const FAST_OFF_INTERFACE: u32 = 2;
const FAST_OFF_RELEASE: u32 = 3;
const FAST_OFF_FILES: u32 = 4;
const FAST_OFF_SCAN: u32 = 5;
const FAST_OFF_INSTALL: u32 = 6;

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
/// - `fast-boot RELEASE scan STATUS, Linux used`
/// - `fast-boot RELEASE install STATUS, Linux used`
/// - `fast-boot RELEASE install STATUS at ADDRESS count N, Linux used`
///
/// STATUS is that of `runtime/install_status.h`, `FAST_STATUS_NOT_RUN` when
/// the head did not run SCAN or INSTALL, or `FAST_STATUS_NOT_READ` when the
/// firmware did not read the file. `FAST_STATUS_CODE_CHANGED` and
/// `FAST_STATUS_DATA_CHANGED` mean that other code wrote into the memory of
/// the fast-boot file after SCAN: N code pages or data blocks changed, the
/// first at ADDRESS (decimal).
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
        format!("if fastreason = {FAST_OFF_SCAN} filewrite lfh ' scan '"),
        format!("if fastreason = {FAST_OFF_INSTALL} filewrite lfh ' install '"),
        format!("if fastreason >= {FAST_OFF_SCAN} int2str line faststatus"),
        format!("if fastreason >= {FAST_OFF_SCAN} filewrite lfh line"),
        format!("if fastreason <> {FAST_OFF_INSTALL} goto fast_log_changed_end"),
        format!("if faststatus < {FAST_STATUS_DATA_CHANGED} goto fast_log_changed_end"),
        format!("if faststatus > {FAST_STATUS_CODE_CHANGED} goto fast_log_changed_end"),
        "filewrite lfh ' at '".into(),
        "int2str line fastaddress".into(),
        "filewrite lfh line".into(),
        "filewrite lfh ' count '".into(),
        "int2str line fastcount".into(),
        "filewrite lfh line".into(),
        ":fast_log_changed_end".into(),
        format!("if fastreason >= {FAST_OFF_SCAN} filewrite lfh ', Linux used'"),
        "filewriteln lfh ''".into(),
        "fileclose lfh".into(),
        ":fast_log_end".into(),
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
    let code_word = |index: &str| {
        format!(
            "registry[{} + {index} * {} + {}]",
            HEADER_SIZE / 4,
            RECORD_SIZE / 4,
            RECORD_CODE / 4
        )
    };
    let record_word = |index: &str, offset: usize| {
        format!(
            "registry[{} + {index} * {} + {}]",
            HEADER_SIZE / 4,
            RECORD_SIZE / 4,
            offset / 4
        )
    };
    let parameter_word =
        |index: &str, word: usize| record_word(index, RECORD_PARAMETERS + word * 4);
    // The record count for the adjustments, or 0 if it is out of range.
    let parameter_count = [
        format!("pcount = {}", word(HOST_COUNT)),
        "if pcount < 0 pcount = 0".to_string(),
        format!("if pcount > {MAX_COUNT} pcount = 0"),
    ];
    // The module settings (runtime/settings.h) in the `modulecode` array that
    // the loader allocates for the modules: the count, or 0 if it is out of
    // range for the array. Entry i has its ID at 2 + 3i, its value at 3 + 3i,
    // and its number of values at 4 + 3i.
    let module_count = [
        "mcount = 0".to_string(),
        "if modulewords < 2 goto module_count_end".into(),
        "mcount = modulecode[0]".into(),
        "mmax = (modulewords - 2) / 3".into(),
        "if mcount < 0 mcount = 0".into(),
        "if mcount > mmax mcount = 0".into(),
        format!("if mcount > {MODULE_SETTINGS_MAX} mcount = 0"),
        ":module_count_end".into(),
    ];
    let state_keys = capabilities::state_keys();
    let keys = state_keys.iter().enumerate();

    // INSTALL has restored the saved values (runtime/installer.c), so the
    // values at the start are those of the file. The loader writes only after
    // the next change: a value that the camera could not restore, such as
    // the effect of a missing file, keeps its entry until then. Without a
    // state file in the data folder, as at the first start after an update
    // from GRKit 1.2, INSTALL restored the old file, and the loader saves at
    // the first poll.
    //
    // The core counts each change of a saved selection in one header word
    // (GRKIT_STATE_CHANGED), so each poll reads that word only: the
    // interpreter spends thousands of instructions on each line, more with
    // each variable of the script. The loader saves when two polls in a row
    // read the same count and the file does not have it yet. A series of
    // changes, such as Fn adjustments, then writes the card once after its
    // last change.
    let changes = word(HOST_STATE_CHANGES);
    let mut lines: Vec<String> = vec![
        ":resident".into(),
        "stateok = 1".into(),
        format!("foldersearch '{DATA_FOLDER}'"),
        format!("if result = 0 foldercreate '{DATA_FOLDER}'"),
        format!("foldersearch '{DATA_FOLDER}'"),
        "if result = 0 stateok = 0".into(),
        format!("changes = {changes}"),
        "saved = changes".into(),
        "last = changes".into(),
        format!("filesearch '{}'", state_path(StateFile::Main, "TXT")),
        "if result = 0 saved = changes - 1".into(),
        // The development profile has its own loop, so the usual poll
        // reads no other word.
        format!("profile = {}", word(HOST_PROFILE)),
        "if profile <> 0 goto profile_watch".into(),
        ":watch".into(),
        format!("mpause {STATE_POLL_MS}"),
        format!("changes = {changes}"),
        "if changes <> last goto watch_moved".into(),
        "if changes = saved goto watch".into(),
        "stamp = changes".into(),
    ];
    for file in StateFile::ALL {
        lines.push(format!("call save_{}", label(file)));
    }
    lines.extend([
        "goto watch".into(),
        ":watch_moved".into(),
        "last = changes".into(),
        "goto watch".into(),
    ]);

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
            "call snapshot".into(),
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
            // The adjustments of each effect that has a non-neutral one.
            lines.extend(parameter_count.clone());
            lines.extend([
                "pindex = 0".into(),
                ":save_parameters".into(),
                "if pindex >= pcount goto save_parameters_end".into(),
                format!("pw0 = {}", parameter_word("pindex", 0)),
                format!("pw1 = {}", parameter_word("pindex", 1)),
                format!("pw2 = {}", parameter_word("pindex", 2)),
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
                "if pvalue = 0 goto save_toning".into(),
                format!(
                    "pkey = {PARAMETER_KEY_BASE} + ({} and 32767)",
                    code_word("pindex")
                ),
                "int2str line pkey".into(),
                "filewriteln sfh line".into(),
                "int2str line pvalue".into(),
                "filewriteln sfh line".into(),
                // The Toning of a native look, a stored byte of 1..8.
                ":save_toning".into(),
                format!(
                    "ptone = ({} >> {}) and 255",
                    record_word("pindex", RECORD_TONING & !3),
                    RECORD_TONING % 4 * 8
                ),
                "if ptone < 1 goto save_parameters_next".into(),
                "if ptone > 8 goto save_parameters_next".into(),
                format!(
                    "pkey = {TONING_KEY_BASE} + ({} and 32767)",
                    code_word("pindex")
                ),
                "int2str line pkey".into(),
                "filewriteln sfh line".into(),
                "int2str line ptone".into(),
                "filewriteln sfh line".into(),
                ":save_parameters_next".into(),
                "pindex = pindex + 1".into(),
                "goto save_parameters".into(),
                ":save_parameters_end".into(),
            ]);
            // Save zero too: a missing setting uses its declared default.
            lines.extend(
                module_count
                    .iter()
                    .map(|line| line.replace("module_count_end", "save_module_count_end")),
            );
            lines.extend([
                "mindex = 0".into(),
                ":save_modules".into(),
                "if mindex >= mcount goto save_modules_end".into(),
                "mid = modulecode[2 + mindex * 3]".into(),
                "mvalue = modulecode[3 + mindex * 3]".into(),
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
        lines.extend(["saved = stamp".into(), format!(":{end}"), "return".into()]);
    }

    // The current selections. The effect is the ImageTone code of the active
    // record, or 0 for Off and for any pointer that is not a record.
    lines.extend([
        ":snapshot".into(),
        "value0 = 0".into(),
        format!("count = {}", word(HOST_COUNT)),
        "if count < 1 goto snapshot_values".into(),
        format!("if count > {MAX_COUNT} goto snapshot_values"),
        format!("offset = {} - {}", word(HOST_ACTIVE), word(HOST_RECORDS)),
        "if offset < 0 goto snapshot_values".into(),
        format!("rest = offset % {RECORD_SIZE}"),
        "if rest != 0 goto snapshot_values".into(),
        format!("index = offset / {RECORD_SIZE}"),
        "if index >= count goto snapshot_values".into(),
        format!("value0 = {} and 65535", code_word("index")),
        ":snapshot_values".into(),
    ]);
    for (i, key) in keys.clone() {
        if let Some(offset) = key.word {
            lines.push(format!("value{i} = {}", word(offset)));
        }
    }
    lines.push("return".into());

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
        "ppolls = ppolls + 1".into(),
        format!("if ppolls >= {PROFILE_POLLS} call profile_write"),
        format!("changes = registry[{}]", HOST_STATE_CHANGES / 4),
        "if changes <> last goto profile_moved".into(),
        "if changes = saved goto profile_poll".into(),
        "stamp = changes".into(),
    ]);
    for file in StateFile::ALL {
        lines.push(format!("call save_{}", label(file)));
    }
    lines.extend([
        "goto profile_poll".into(),
        ":profile_moved".into(),
        "last = changes".into(),
        "goto profile_poll".into(),
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

/// The loader path of a state file with the extension `extension`.
fn state_path(file: StateFile, extension: &str) -> String {
    format!("{DATA_FOLDER}\\{}.{extension}", file.stem())
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
        let record = |index: &str, offset: usize| {
            format!(
                "{} + {index} * {} + {}",
                HEADER_SIZE / 4,
                RECORD_SIZE / 4,
                offset / 4
            )
        };
        // The computed indexes are words of a record below `count` or `pcount`:
        // its code, and the three words of its adjustments.
        let allowed = [
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
        // Each computed record index is below `count`, which is at most MAX_COUNT.
        assert!(script.contains(&format!("if count > {MAX_COUNT} goto snapshot_values")));
        assert!(script.contains("if pcount < 0 pcount = 0"));
        assert!(script.contains(&format!("if pcount > {MAX_COUNT} pcount = 0")));
        assert_eq!(script.matches("if pindex >= pcount goto").count(), 1);
    }

    /// A series of changes writes the card once: each poll reads only the
    /// change count of the core, and the watch loop calls the saves only when
    /// two polls in a row read the same count and the file does not have it.
    /// The start sets each variable that the loop reads, because an unset
    /// variable is a runtime error.
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

    #[test]
    fn the_resident_script_saves_after_the_selections_settle() {
        let script = state_script();
        let at = |line: &str| {
            script
                .iter()
                .position(|l| l == line)
                .unwrap_or_else(|| panic!("{line}"))
        };
        let read = format!("changes = registry[{}]", HOST_STATE_CHANGES / 4);
        let (start, watch) = (at(":resident"), at(":watch"));
        let poll = at(&format!("mpause {STATE_POLL_MS}"));
        assert_eq!(script[poll + 1], read);
        let moved = at("if changes <> last goto watch_moved");
        let settled = at("if changes = saved goto watch");
        assert!(watch < poll && poll < moved && moved < settled);
        assert_eq!(script[settled + 1], "stamp = changes");
        for file in StateFile::ALL {
            assert!(settled < at(&format!("call save_{}", label(file))));
        }
        // An idle poll runs four lines: the pause, the read, two compares.
        assert_eq!(settled - poll + 1, 4);
        assert_eq!(script[at(":watch_moved") + 1], "last = changes");
        for name in ["last", "saved"] {
            assert!(script[start..watch]
                .iter()
                .any(|l| l.starts_with(&format!("{name} ="))));
        }
        assert!(script[start..watch].contains(&read));
        // A save reads the selections, and records the count that it saved.
        for file in StateFile::ALL {
            let save = at(&format!(":save_{}", label(file)));
            assert_eq!(script[save + 2], "call snapshot");
        }
        assert!(script.iter().any(|l| l == "saved = stamp"));
    }

    fn at(script: &[String], line: &str) -> usize {
        script
            .iter()
            .position(|l| l == line)
            .unwrap_or_else(|| panic!("{line}"))
    }

    /// The state files are in the data folder, which holds the loader. Without
    /// the folder, no save writes.
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
        let writes: Vec<_> = script
            .iter()
            .filter(|l| l.starts_with("filecreate") || l.starts_with("filerename"))
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
        assert_eq!(
            &lines[linux - 4..linux + 4],
            [
                "if faststatus = 0 goto plan_ready",
                "fastreason = 5",
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
            lines[tail + 1..tail + 16],
            [
                "allocated = 1",
                "hadcount = count",
                "hadwords = words",
                "hadchunks = chunks",
                "hadmodwords = modulewords",
                "hadmodtiles = moduletiles",
                "goto arrays_ready",
                ":check_arrays",
                "if count <> hadcount goto reject",
                "if words <> hadwords goto reject",
                "if chunks <> hadchunks goto reject",
                "if modulewords <> hadmodwords goto reject",
                "if moduletiles <> hadmodtiles goto reject",
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
                "fastaddress = ba[252]",
                "fastcount = ba[253]",
                ":install_failed",
                "if fast = 0 goto reject",
                "failure = errorstatus",
                "if errorstatus = 24 goto reject",
                "faststatus = errorstatus",
                "fastreason = 6",
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
            assert!(lines[call - 1].starts_with("fastreason = "));
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
            "if fastreason = 6 filewrite lfh ' install '",
            "if fastreason >= 5 filewrite lfh ', Linux used'",
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
