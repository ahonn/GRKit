//! The private GR III runtime layout and production loader templates.

use anyhow::{ensure, Result};
use serde_json::{json, Value};

use crate::Files;

pub const HEADER_SIZE: usize = 256;
pub const RECORD_SIZE: usize = 0x740;
pub const ICON_STRIDE: usize = 6408;
pub const ICONS_PER_CHUNK: usize = 40;
/// Off, GRKit, four Framing tiles, five Guides tiles, and five Date Stamp tiles.
/// The menu addresses module tiles by offset, so they must stay contiguous in
/// one icon chunk.
pub const HOST_ICON_COUNT: usize = 16;
/// Product limit for private Image Control effects.
pub const MAX_COUNT: usize = 20;
/// Picture scratch after the records (`GRKIT_PICTURE_SCRATCH`): the date stamp
/// core and halo masks.
pub const PICTURE_SCRATCH: usize = 141376;
// One script array holds the header, every record, and the scratch: at most 65536 words.
const _: () = assert!(HEADER_SIZE + MAX_COUNT * RECORD_SIZE + PICTURE_SCRATCH <= 65536 * 4);
const _: () = assert!(PICTURE_SCRATCH.is_multiple_of(4));
pub const MAX_CHUNKS: usize = (MAX_COUNT + HOST_ICON_COUNT).div_ceil(ICONS_PER_CHUNK);
const _: () = assert!(MAX_COUNT + HOST_ICON_COUNT <= ICONS_PER_CHUNK);
pub const HEADER_REFERENCE: u32 = 0x58012340;
/// Header and record offsets that the loader script reads and writes. They
/// match `runtime/abi.h`, which a native test checks.
pub const HOST_COUNT: usize = 0;
pub const HOST_RECORDS: usize = 4;
pub const HOST_ACTIVE: usize = 8;
pub const HOST_STAMP: usize = 52;
pub const HOST_GUIDE: usize = 204;
pub const HOST_FRAME_RATIO: usize = 208;
pub const RECORD_CODE: usize = 0x6f8;
/// Selections kept across power-off, written by the resident loader through the
/// camera's own file system, never from Linux while the camera may write photos.
pub const STATE_FILE: &str = "STATE.TXT";
pub const LINUX_DIRECTORY: &str = "/media/storage1/SCRIPT/PLUGINS";

pub fn layout(count: usize) -> Result<Value> {
    ensure!(count <= MAX_COUNT, "Expected 0..{MAX_COUNT} effects");
    let icon_words: Vec<_> = (0..count + HOST_ICON_COUNT)
        .step_by(ICONS_PER_CHUNK)
        .map(|start| ICONS_PER_CHUNK.min(count + HOST_ICON_COUNT - start) * ICON_STRIDE / 4)
        .collect();
    Ok(
        json!({"words": (HEADER_SIZE + count * RECORD_SIZE + PICTURE_SCRATCH) / 4, "icon_words": icon_words}),
    )
}

pub fn activation_files() -> Files {
    Files::from([
        ("00078350.588".into(), b"[SCRIPT_EN_MODE EN]\r\n".to_vec()),
        (
            "DEVELOP.MOD".into(),
            vec![7, 1, 44, 31, 16, 3, 30, 22, 5, 45],
        ),
    ])
}

/// Polls for Linux and camera readiness instead of waiting a fixed time.
/// `lcmd` requests are synchronous: each returns after the Linux command ends,
/// or with NG when Linux is not ready. The loader checks the result file after
/// each request and repeats it after a short pause.
pub fn loader() -> Vec<u8> {
    let path = |name: &str| format!("C:\\SCRIPT\\PLUGINS\\{name}");
    let (plan, done, failed) = (path("MODULE.BIN"), path("INSTALL.OK"), path("INSTALL.ERR"));
    let mut lines: Vec<String> = vec!["sendln 'lcmd_resp on'".into()];
    for name in [
        "MODULE.BIN",
        "MODULE.TMP",
        "INSTALL.OK",
        "INSTALL.ERR",
        "INSTALL.LCK",
    ] {
        lines.push(format!("filedelete '{}'", path(name)));
    }
    lines.extend([
        format!("filesearch '{plan}'"),
        "if result != 0 goto reject".into(),
        format!("filesearch '{done}'"),
        "if result != 0 goto reject".into(),
        // Early requests fail before Linux mounts the card. Give up after 150 attempts.
        "attempts = 0".into(),
        ":scan".into(),
        format!("sendln 'lcmd sh {LINUX_DIRECTORY}/SCAN.SH'"),
        format!("filesearch '{plan}'"),
        "if result != 0 goto plan_ready".into(),
        "mpause 200".into(),
        "attempts = attempts + 1".into(),
        "if attempts < 150 goto scan".into(),
        "goto reject".into(),
        ":plan_ready".into(),
        format!("fileopen fh '{plan}' 0"),
        "if fh < 0 goto reject".into(),
        "call readword".into(),
        "if value != 827150919 goto reject_close".into(),
        "call readword".into(),
        "count = value".into(),
        "if count < 0 goto reject_close".into(),
        format!("if count > {MAX_COUNT} goto reject_close"),
        "call readword".into(),
        "words = value".into(),
        "call readword".into(),
        "chunks = value".into(),
        "fileclose fh".into(),
        format!(
            "expected = {} + count * {}",
            (HEADER_SIZE + PICTURE_SCRATCH) / 4,
            RECORD_SIZE / 4
        ),
        "if words != expected goto reject".into(),
        format!(
            "expectedchunks = (count + {}) / {ICONS_PER_CHUNK}",
            HOST_ICON_COUNT + ICONS_PER_CHUNK - 1
        ),
        "if chunks != expectedchunks goto reject".into(),
        "intdim registry words".into(),
    ]);
    for i in 0..MAX_CHUNKS {
        lines.extend([
            format!("if chunks <= {i} goto icons_ready"),
            format!(
                "items = count + {HOST_ICON_COUNT} - {}",
                i * ICONS_PER_CHUNK
            ),
            format!("if items > {ICONS_PER_CHUNK} items = {ICONS_PER_CHUNK}"),
            format!("iconwords = items * {}", ICON_STRIDE / 4),
            format!("intdim artwork{i} iconwords"),
        ]);
    }
    lines.extend([
        ":icons_ready".into(),
        // Not-ready attempts release the install lock. Retry every 0.5 s for about
        // 30 s, then every 10 s, so a later shooting-mode start can still install.
        "attempts = 0".into(),
        ":install".into(),
        format!("sendln 'lcmd sh {LINUX_DIRECTORY}/INSTALL.SH'"),
        format!("filesearch '{done}'"),
        "if result != 0 goto resident".into(),
        format!("filesearch '{failed}'"),
        "if result != 0 goto reject".into(),
        "mpause 500".into(),
        "attempts = attempts + 1".into(),
        "if attempts > 60 mpause 9500".into(),
        "goto install".into(),
    ]);
    // The private header is the `registry` array, so the script reads and
    // writes it directly. The state is one integer:
    // code + 65536 * (ratio + 4 * (guide + 8 * stamp)), code 0 for Off.
    let word = |offset: usize| format!("registry[{}]", offset / 4);
    let state = path(STATE_FILE);
    lines.extend([
        ":resident".into(),
        "call restore".into(),
        ":watch".into(),
        "mpause 2000".into(),
        "call snapshot".into(),
        "if state = saved goto watch".into(),
        format!("filecreate sfh '{state}'"),
        "if sfh < 0 goto watch".into(),
        "int2str line state".into(),
        "filewrite sfh line".into(),
        "fileclose sfh".into(),
        "saved = state".into(),
        "goto watch".into(),
        ":snapshot".into(),
        format!("active = {}", word(HOST_ACTIVE)),
        "code = 0".into(),
        "if active = 0 goto snapshot_values".into(),
        format!("index = (active - {}) / {RECORD_SIZE}", word(HOST_RECORDS)),
        format!(
            "code = registry[{} + index * {} + {}]",
            HEADER_SIZE / 4,
            RECORD_SIZE / 4,
            RECORD_CODE / 4
        ),
        ":snapshot_values".into(),
        format!(
            "state = code + 65536 * ({} + 4 * ({} + 8 * {}))",
            word(HOST_FRAME_RATIO),
            word(HOST_GUIDE),
            word(HOST_STAMP)
        ),
        "return".into(),
        // An unreadable or out-of-range file restores nothing. An effect that is
        // no longer installed stays Off. `fileopen` creates a missing file, and a
        // read of an empty file stops the interpreter, so check the size first.
        ":restore".into(),
        "size = 0".into(),
        format!("filestat '{state}' size"),
        "if size < 1 goto restore_end".into(),
        "if size > 10 goto restore_end".into(),
        format!("fileopen sfh '{state}' 0"),
        "if sfh < 0 goto restore_end".into(),
        "fileread sfh size line".into(),
        "fileclose sfh".into(),
        "str2int stored line".into(),
        "if result = 0 goto restore_end".into(),
        "if stored < 0 goto restore_end".into(),
        "rest = stored / 65536".into(),
        "code = stored - rest * 65536".into(),
        "upper = rest / 4".into(),
        "ratio = rest - upper * 4".into(),
        "rest = upper / 8".into(),
        "guide = upper - rest * 8".into(),
        "if guide > 4 goto restore_end".into(),
        "if rest > 4 goto restore_end".into(),
        format!("{} = ratio", word(HOST_FRAME_RATIO)),
        format!("{} = guide", word(HOST_GUIDE)),
        format!("{} = rest", word(HOST_STAMP)),
        "index = 0".into(),
        ":restore_find".into(),
        "if code = 0 goto restore_end".into(),
        format!("if index >= {} goto restore_end", word(HOST_COUNT)),
        format!(
            "if registry[{} + index * {} + {}] <> code goto restore_next",
            HEADER_SIZE / 4,
            RECORD_SIZE / 4,
            RECORD_CODE / 4
        ),
        format!(
            "{} = {} + index * {RECORD_SIZE}",
            word(HOST_ACTIVE),
            word(HOST_RECORDS)
        ),
        "goto restore_end".into(),
        ":restore_next".into(),
        "index = index + 1".into(),
        "goto restore_find".into(),
        // Write only after the next change, so a missing effect keeps its file entry
        // until the user selects something else.
        ":restore_end".into(),
        "call snapshot".into(),
        "saved = state".into(),
        "return".into(),
    ]);
    lines.extend(
        [
            ":reject_close",
            "fileclose fh",
            ":reject",
            "dispstr 'GR_MODULE_LOADER_REJECTED'",
            "end",
            ":readword",
            "value = 0",
            "weight = 1",
            "for byteindex 0 3",
            "fileread fh 1 octet",
            "if result != 0 goto reject_close",
            "str2code bytevalue octet",
            "value = value + bytevalue * weight",
            "if byteindex < 3 weight = weight * 256",
            "next",
            "return",
        ]
        .map(String::from),
    );
    (lines.join("\r\n") + "\r\n").into_bytes()
}

pub fn runtime_files(binary: &[u8]) -> Files {
    let target = "C:\\SCRIPT\\PLUGINS\\PLUGINS.TTL";
    let startup = format!("filesearch '{target}'\r\nif result = 0 goto done\r\ninclude '{target}'\r\n:done\r\nend\r\n");
    let wrapper = |mode: &str| {
        format!("{LINUX_DIRECTORY}/INSTALL.BIN --{mode}\nstatus=$?\necho GR_MODULE_{}_EXIT_$status\nexit 0\n", mode.to_uppercase()).into_bytes()
    };
    Files::from([
        ("SCRIPT/STARTUP.TTL".into(), startup.into_bytes()),
        ("SCRIPT/PLUGINS/PLUGINS.TTL".into(), loader()),
        ("SCRIPT/PLUGINS/INSTALL.BIN".into(), binary.to_vec()),
        ("SCRIPT/PLUGINS/SCAN.SH".into(), wrapper("scan")),
        ("SCRIPT/PLUGINS/INSTALL.SH".into(), wrapper("install")),
    ])
}
