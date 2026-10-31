//! The camera menu text in each menu language, and the label table that the
//! installer copies after the picture scratch (`runtime/abi.h`).
//!
//! The menu shows the language of the camera if it is a menu language, and
//! English otherwise. Effect names stay as their definitions give them.

use anyhow::{bail, ensure, Result};

use crate::language::Language;

/// The size of the text area after the picture scratch.
pub const TEXT_SIZE: usize = 24576;
/// Where the Image Control summaries start in the text area. The label table
/// must end before it.
pub const TEXT_SUMMARY: usize = 8192;
/// The last 16 bytes of the text area: the parameter page title of a GRKit
/// session. Words: the page, its vtable, then the descriptor of the effect name.
pub const TEXT_PARAMETER_TITLE: usize = TEXT_SIZE - 16;
/// The 512 bytes before the parameter page title: the native settings page of
/// a GRKit session (`runtime/menu.c`).
pub const TEXT_SETTINGS: usize = TEXT_PARAMETER_TITLE - 512;
/// The units of the longest string with its NUL, and of each summary slot.
pub const SUMMARY_UNITS: usize = 32;
/// The number of strings in each language (`GRKIT_TEXT_STRINGS`).
pub const STRINGS: usize = 23;
/// The name of the GRKit switch, the same in each language.
const GRKIT: &str = "GRKit";
/// The project address that About GRKit shows.
const PROJECT: &str = "github.com/ahonn/grkit";
/// The longest effect name in a record.
const NAME_UNITS: usize = 16;

/// The words of one menu language. Each list follows the rows of its module
/// after Off. The Chinese and Japanese words reuse the terms of the camera
/// menus where they have them (关闭/關閉, 弱/中/强, 颗粒感/粒紋感; オフ, 粒状感,
/// 9分割, 画面中央, 弱/中/強). Guides avoids the name of the native grid
/// (网格线, グリッド), as the camera has its own. The effects module has its own name
/// (Film Look, 胶片风格, 底片風格, フィルムルック), so that it differs from the
/// native Image Control.
/// The Date Imprint names follow the GR II menu (Date Imprint, 日付入れ撮影 in
/// its firmware), with the short Chinese name 加印日期.
/// The Korean words use the camera terms too (켜짐/꺼짐, 약/중/강, 입자감,
/// 화상비율, 9분할, 화면 중앙). The camera's Korean font draws only the 2350
/// Hangul syllables of KS X 1001 and ASCII, so each Korean text keeps to them.
/// The camera's Chinese and Japanese fonts draw each of these characters only
/// while the camera language is that language, which is when the menu shows
/// them.
struct Words {
    /// Between a module name and its value in a summary.
    separator: &'static str,
    image_control: &'static str,
    /// Before the version in the About row.
    version: &'static str,
    off: &'static str,
    /// The name in the summary of module management (runtime/manage.h).
    modules: &'static str,
    /// The Modules summary after a change that applies at the next start.
    restart: &'static str,
    safe_mode: &'static str,
    /// The row that turns a module or an effect on or off, and its On value.
    enable: &'static str,
    on: &'static str,
    /// The MENU row that orders the module pages in the GRKit root of ADJ.
    adj_order: &'static str,
    /// The settings row that lists the options of a module page.
    options: &'static str,
    /// The MENU row of the GRKit switch, its version, and its project.
    about: &'static str,
    project: &'static str,
    /// The settings row of Film Look that opens the Fn adjustments of
    /// the effect.
    adjustments: &'static str,
    /// The settings row of Film Look that selects the effect.
    film: &'static str,
    /// The guides of an options page: OK turns the option on or off, MENU
    /// goes back with the changes. The OK guide has the width of the native
    /// word (OK, 确定, 決定): empty keeps the native word.
    turn: &'static str,
    back: &'static str,
}

const ENGLISH: Words = Words {
    separator: ": ",
    image_control: "Film Look",
    version: "Version",
    off: "Off",
    modules: "Modules",
    restart: "Restart to apply",
    safe_mode: "Safe mode",
    enable: "Enable",
    on: "On",
    adj_order: "ADJ Order",
    options: "Options",
    about: "About GRKit",
    project: "Project",
    adjustments: "Parameters",
    film: "Film Stock",
    turn: "",
    back: "Back",
};

const SIMPLIFIED_CHINESE: Words = Words {
    separator: "：",
    image_control: "胶片风格",
    version: "版本",
    off: "关闭",
    modules: "模块",
    restart: "重新开机后生效",
    safe_mode: "安全模式",
    enable: "启用",
    on: "开启",
    adj_order: "ADJ 菜单顺序",
    options: "可选项",
    about: "关于 GRKit",
    project: "项目地址",
    adjustments: "效果参数",
    film: "胶卷",
    turn: "切换",
    back: "返回",
};

const TRADITIONAL_CHINESE: Words = Words {
    separator: "：",
    image_control: "底片風格",
    version: "版本",
    off: "關閉",
    modules: "模組",
    restart: "重新開機後生效",
    safe_mode: "安全模式",
    enable: "啟用",
    on: "開啟",
    adj_order: "ADJ 選單順序",
    options: "可選項",
    about: "關於 GRKit",
    project: "專案網址",
    adjustments: "效果參數",
    film: "膠卷",
    turn: "切換",
    back: "返回",
};

const JAPANESE: Words = Words {
    // A full-width colon would push a long name and a 13-character effect
    // name under the icon column. The half-width colon keeps them clear.
    separator: ":",
    image_control: "フィルムルック",
    version: "バージョン",
    off: "オフ",
    modules: "モジュール",
    restart: "再起動後に反映",
    safe_mode: "セーフモード",
    enable: "有効",
    on: "オン",
    adj_order: "ADJ メニューの順番",
    options: "オプション",
    about: "GRKit について",
    project: "プロジェクト",
    adjustments: "パラメーター",
    film: "フィルム銘柄",
    turn: "切替",
    back: "戻る",
};

const KOREAN: Words = Words {
    separator: ": ",
    image_control: "필름 룩",
    version: "버전",
    off: "꺼짐",
    modules: "모듈",
    restart: "재시작 후 적용",
    safe_mode: "안전 모드",
    enable: "사용",
    on: "켜짐",
    adj_order: "ADJ 메뉴 순서",
    options: "옵션",
    about: "GRKit 정보",
    project: "프로젝트",
    adjustments: "파라미터",
    film: "필름 종류",
    turn: "전환",
    back: "뒤로",
};

/// The text between a page title and its value in a root row summary, such
/// as "Guides: Thirds".
pub fn separator(language: Language) -> &'static str {
    words(language).separator
}

fn words(language: Language) -> &'static Words {
    match language {
        Language::English => &ENGLISH,
        Language::SimplifiedChinese => &SIMPLIFIED_CHINESE,
        Language::TraditionalChinese => &TRADITIONAL_CHINESE,
        Language::Japanese => &JAPANESE,
        Language::Korean => &KOREAN,
    }
}

/// A run of consecutive string IDs. `runtime/abi.h` defines `TEXT_<name>` as
/// the first ID of each run.
pub struct Run {
    pub name: &'static str,
    pub count: usize,
    pub doc: &'static str,
}

const fn run(name: &'static str, count: usize, doc: &'static str) -> Run {
    Run { name, count, doc }
}

/// The string IDs in table order.
pub const RUNS: &[Run] = &[
    run("OFF", 1, ""),
    run("IMAGE_OFF_SUMMARY", 1, ""),
    run(
        "IMAGE_PREFIX",
        1,
        "The Image Control summary before an effect name, for the installer.",
    ),
    run("VERSION", 1, "The About row: the GRKit version."),
    run(
        "MODULES_SUMMARY",
        2,
        "The Modules root row: as usual, and in safe mode.",
    ),
    run(
        "ENABLE",
        1,
        "The settings row that turns a module or an effect on or off.",
    ),
    run("ON", 1, ""),
    run("SAFE_MODE", 1, "The GRKit row of MENU in safe mode."),
    run(
        "RESTART",
        1,
        "The GRKit row of MENU after a change for the next start.",
    ),
    run(
        "ADJUSTMENTS",
        1,
        "The Image Control settings row that opens the Fn adjustments.",
    ),
    run("ADJ", 1, "The ADJ Order row of MENU."),
    run(
        "OPTIONS",
        1,
        "The settings row that lists the options of a module page.",
    ),
    run(
        "ABOUT",
        5,
        "About GRKit: the title, the Version row and its value, the Project row and its value.",
    ),
    run(
        "FILM",
        1,
        "The Film Look settings row that selects the effect.",
    ),
    run(
        "GUIDE",
        2,
        "The guides of an options value page: OK turns an option on or off, MENU goes back.",
    ),
    run(
        "SWITCH",
        2,
        "The GRKit switch, which stops each module at once: its summary while on, and while off.",
    ),
];

/// The first string ID of each run, in table order.
pub fn ids() -> impl Iterator<Item = (&'static Run, usize)> {
    RUNS.iter().scan(0, |next, run| {
        let id = *next;
        *next += run.count;
        Some((run, id))
    })
}

/// The first string ID of a run.
fn id(name: &str) -> usize {
    ids()
        .find(|(run, _)| run.name == name)
        .map(|(_, id)| id)
        .expect("a menu text run")
}

/// The strings of one run in one language. `version` is the GRKit version.
fn run_strings(w: &Words, name: &str, version: &str) -> Vec<String> {
    let summary = |module: &str, value: &str| format!("{module}{}{value}", w.separator);
    let words = |values: &[&str]| values.iter().map(|value| value.to_string()).collect();
    match name {
        "OFF" => words(&[w.off]),
        "IMAGE_OFF_SUMMARY" => vec![summary(w.image_control, w.off)],
        "IMAGE_PREFIX" => vec![format!("{}{}", w.image_control, w.separator)],
        "VERSION" => vec![format!("{} {version}", w.version)],
        "MODULES_SUMMARY" => vec![w.modules.to_string(), summary(w.modules, w.safe_mode)],
        "ENABLE" => words(&[w.enable]),
        "ON" => words(&[w.on]),
        "SAFE_MODE" => words(&[w.safe_mode]),
        "RESTART" => words(&[w.restart]),
        "ADJUSTMENTS" => words(&[w.adjustments]),
        "ADJ" => words(&[w.adj_order]),
        "OPTIONS" => words(&[w.options]),
        "ABOUT" => vec![
            w.about.to_string(),
            w.version.to_string(),
            version.to_string(),
            w.project.to_string(),
            PROJECT.to_string(),
        ],
        "FILM" => words(&[w.film]),
        "GUIDE" => words(&[w.turn, w.back]),
        "SWITCH" => vec![summary(GRKIT, w.on), summary(GRKIT, w.off)],
        _ => unreachable!("Unknown menu text run {name}"),
    }
}

/// The strings of one language in the ID order of `runtime/abi.h`. `version`
/// is the GRKit version that the About row shows.
pub fn strings(language: Language, version: &str) -> Vec<String> {
    let w = words(language);
    RUNS.iter()
        .flat_map(|run| run_strings(w, run.name, version))
        .collect()
}

fn units(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

/// The label table: for each language, `STRINGS` u16 byte offsets from the
/// table start, then the NUL-terminated UTF-16LE strings. `version` is the
/// GRKit version that the About row shows.
pub fn table(version: &str) -> Result<Vec<u8>> {
    ensure!(
        !version.is_empty() && version.bytes().all(|byte| byte.is_ascii_graphic()),
        "The version {version:?} is not printable ASCII"
    );
    let languages = Language::ALL;
    let mut offsets = Vec::new();
    let mut strings_area: Vec<u16> = Vec::new();
    let strings_start = languages.len() * STRINGS * 2;
    for language in languages {
        let list = strings(language, version);
        ensure!(
            list.len() == STRINGS,
            "Menu text has {} strings",
            list.len()
        );
        for text in &list {
            if let Some(c) = language.undrawable(text) {
                bail!("The camera cannot draw {c:?} of the menu text {text:?}");
            }
            let encoded = units(text);
            ensure!(
                encoded.len() < SUMMARY_UNITS,
                "Menu text is too long: {text:?}"
            );
            let offset = strings_start + strings_area.len() * 2;
            offsets.push(u16::try_from(offset)?);
            strings_area.extend(encoded);
            strings_area.push(0);
        }
        let prefix = units(&list[id("IMAGE_PREFIX")]).len();
        ensure!(
            prefix + NAME_UNITS < SUMMARY_UNITS,
            "The Image Control prefix leaves no room for an effect name"
        );
    }
    let mut out: Vec<u8> = offsets.iter().flat_map(|v| v.to_le_bytes()).collect();
    out.extend(strings_area.iter().flat_map(|v| v.to_le_bytes()));
    ensure!(
        out.len() <= TEXT_SUMMARY,
        "Menu text table exceeds {TEXT_SUMMARY} bytes"
    );
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const VERSION: &str = "0.0.0";

    #[test]
    fn the_table_fits_and_points_at_each_string() {
        let table = table(VERSION).unwrap();
        for (index, language) in Language::ALL.into_iter().enumerate() {
            for (id, expected) in strings(language, VERSION).iter().enumerate() {
                let at = index * STRINGS + id;
                let offset = u16::from_le_bytes([table[at * 2], table[at * 2 + 1]]) as usize;
                let read: Vec<u16> = table[offset..]
                    .chunks_exact(2)
                    .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
                    .take_while(|unit| *unit != 0)
                    .collect();
                assert_eq!(String::from_utf16(&read).unwrap(), *expected);
            }
        }
    }

    #[test]
    fn each_run_has_its_declared_number_of_strings() {
        for language in Language::ALL {
            for run in RUNS {
                assert_eq!(
                    run_strings(words(language), run.name, VERSION).len(),
                    run.count,
                    "{} in {language:?}",
                    run.name
                );
            }
        }
        assert_eq!(RUNS.iter().map(|run| run.count).sum::<usize>(), STRINGS);
    }

    #[test]
    fn english_keeps_the_menu_text_of_earlier_builds() {
        let english = strings(Language::English, VERSION);
        assert_eq!(english[id("OFF")], "Off");
        assert_eq!(english[id("IMAGE_OFF_SUMMARY")], "Film Look: Off");
        assert_eq!(english[id("IMAGE_PREFIX")], "Film Look: ");
    }

    #[test]
    fn the_switch_summary_gives_its_value_in_each_language() {
        for language in Language::ALL {
            let w = words(language);
            let list = strings(language, VERSION);
            assert_eq!(list[id("SWITCH")], format!("GRKit{}{}", w.separator, w.on));
            assert_eq!(
                list[id("SWITCH") + 1],
                format!("GRKit{}{}", w.separator, w.off)
            );
        }
    }

    #[test]
    fn the_about_row_shows_the_version_in_each_language() {
        assert_eq!(
            strings(Language::English, "1.2.3")[id("VERSION")],
            "Version 1.2.3"
        );
        assert_eq!(
            strings(Language::SimplifiedChinese, "1.2.3")[id("VERSION")],
            "版本 1.2.3"
        );
        assert_eq!(
            strings(Language::TraditionalChinese, "1.2.3")[id("VERSION")],
            "版本 1.2.3"
        );
        assert_eq!(
            strings(Language::Japanese, "1.2.3")[id("VERSION")],
            "バージョン 1.2.3"
        );
    }

    #[test]
    fn the_table_rejects_a_version_that_the_menu_cannot_show() {
        assert!(table("").is_err());
        assert!(table("1.0 beta").is_err());
        assert!(table("1.0.0-测试").is_err());
        assert!(table(&"9".repeat(SUMMARY_UNITS)).is_err());
    }
}
