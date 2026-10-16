//! The private GR III runtime layout and production loader templates.

use anyhow::{ensure, Result};
use serde_json::{json, Value};

use crate::Files;

pub const HEADER_SIZE: usize = 256;
pub const RECORD_SIZE: usize = 0x700;
pub const ICON_STRIDE: usize = 6408;
pub const ICONS_PER_CHUNK: usize = 40;
/// Off, GRKit, and four Framing tiles. The menu addresses Framing tiles by
/// offset, so they must stay contiguous in one icon chunk.
pub const HOST_ICON_COUNT: usize = 6;
/// Product limit for private Image Control effects.
pub const MAX_COUNT: usize = 20;
// One script array holds the header and every record: at most 65536 words.
const _: () = assert!(HEADER_SIZE + MAX_COUNT * RECORD_SIZE <= 65536 * 4);
pub const MAX_CHUNKS: usize = (MAX_COUNT + HOST_ICON_COUNT).div_ceil(ICONS_PER_CHUNK);
const _: () = assert!(MAX_COUNT + HOST_ICON_COUNT <= ICONS_PER_CHUNK);
pub const HEADER_REFERENCE: u32 = 0x58012340;
pub const LINUX_DIRECTORY: &str = "/media/storage1/SCRIPT/PLUGINS";

pub fn layout(count: usize) -> Result<Value> {
    ensure!(count <= MAX_COUNT, "Expected 0..{MAX_COUNT} effects");
    let icon_words: Vec<_> = (0..count + HOST_ICON_COUNT)
        .step_by(ICONS_PER_CHUNK)
        .map(|start| ICONS_PER_CHUNK.min(count + HOST_ICON_COUNT - start) * ICON_STRIDE / 4)
        .collect();
    Ok(json!({"words": (HEADER_SIZE + count * RECORD_SIZE) / 4, "icon_words": icon_words}))
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
        "expected = 64 + count * 448".into(),
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
    lines.extend(
        [
            ":resident",
            "mpause 60000",
            "goto resident",
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
