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
/// The units of the value of a group page in MENU and on the ADJ root, with
/// its NUL: the titles of its children that are on, joined by a slash.
pub const GROUP_UNITS: usize = 256;
/// The value of a group page, before the native settings page.
pub const TEXT_GROUP_VALUE: usize = TEXT_SETTINGS - 2 * GROUP_UNITS;
/// The units of the longest string with its NUL, and of each summary slot.
pub const SUMMARY_UNITS: usize = 32;
/// The number of strings in each language (`GRKIT_TEXT_STRINGS`).
pub const STRINGS: usize = 71;
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
    /// The MENU row of the switch that turns all modules off and on again.
    enable_grkit: &'static str,
    /// The MENU row that decides the switch at each start, and its value
    /// that keeps the switch of the last session. Its other values are the
    /// On and Off of the switch.
    enable_at_start: &'static str,
    keep_last: &'static str,
    /// The Mode Dial row of the Recipes page of MENU, and the values of a
    /// user mode besides a recipe: None, which follows the dial, GRKit Off,
    /// and Saved Settings, the settings that Save Settings saved in the user
    /// mode. U1 to U3 have the names of the dial.
    mode_dial: &'static str,
    dial_none: &'static str,
    dial_off: &'static str,
    dial_saved: &'static str,
    /// The GRKit Settings row of MENU and the title of its page.
    grkit_settings: &'static str,
    /// The Distance Unit row of GRKit Settings, then its two values.
    distance_unit: &'static str,
    meters: &'static str,
    feet: &'static str,
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
    /// The page of camera recipes (runtime/recipe_page.h): the undo row,
    /// the save row, the toast after a save (before the name), the confirm
    /// of a delete (before and after the name), the message when the
    /// recipes are full, the label and message of a recipe whose effect is
    /// not installed, and the name of a recipe without a look.
    recipe_undo: &'static str,
    recipe_save: &'static str,
    recipe_saved: &'static str,
    recipe_delete: (&'static str, &'static str),
    recipe_full: &'static str,
    recipe_missing: &'static str,
    recipe_name: &'static str,
    /// The message of a recipe with a native value that this camera does
    /// not have.
    recipe_unsupported: &'static str,
    /// The message after an apply that left out the Grain of the recipe: no
    /// Grain module runs.
    recipe_no_grain: &'static str,
    /// The buttons of the box that Fn on a camera recipe shows: Rename, which
    /// opens the keyboard of the camera, and Delete. The words are those of
    /// the native Customize rows (Rename, 重命名, 名称変更, 이름 변경) and of
    /// the native Delete (删除, 刪除, 消去, 삭제).
    recipe_actions: [&'static str; 2],
    /// The Apply row of the page of a recipe in MENU.
    recipe_apply: &'static str,
    /// The message after an apply that left out the Glow of the recipe: the
    /// Glow module is not loaded.
    recipe_no_glow: &'static str,
    /// The mark after the name of a recipe whose effect is not installed.
    recipe_missing_mark: &'static str,
    /// The names of the native Image Control types in the order of
    /// `grkit_core::native::TYPES`, as the camera menus name them. The last
    /// seven are the types of GR IV Monochrome, with the names of its
    /// firmware.
    native_types: [&'static str; 21],
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
    enable_grkit: "Enable GRKit",
    enable_at_start: "Enable at Start",
    keep_last: "Keep Last",
    mode_dial: "Mode Dial",
    dial_none: "None",
    dial_off: "GRKit Off",
    dial_saved: "Saved Settings",
    grkit_settings: "GRKit Settings",
    distance_unit: "Distance Unit",
    meters: "Meters",
    feet: "Feet",
    adj_order: "ADJ Order",
    options: "Options",
    about: "About GRKit",
    project: "Project",
    adjustments: "Parameters",
    film: "Film Stock",
    turn: "",
    back: "Back",
    recipe_undo: "Undo",
    recipe_save: "Save Settings",
    recipe_saved: "Saved: ",
    recipe_delete: ("Delete ", "?"),
    recipe_full: "Full. Fn on a recipe deletes it",
    recipe_missing: "Effect not installed",
    recipe_name: "Recipe",
    recipe_unsupported: "Not available on this camera",
    recipe_no_grain: "Grain needs the Grain module",
    recipe_actions: ["Rename", "Delete"],
    recipe_apply: "Apply",
    recipe_no_glow: "Glow needs the Glow module",
    recipe_missing_mark: "effect missing",
    native_types: [
        "Standard",
        "Vivid",
        "Monotone",
        "Soft Monotone",
        "Hard Monotone",
        "Hi-Contrast B&W",
        "Negative Film",
        "Positive Film",
        "Bleach Bypass",
        "Retro",
        "HDR Tone",
        "Cross Processing",
        "Cinema (Yellow)",
        "Cinema (Green)",
        "Standard",
        "Soft",
        "Hi-Contrast",
        "Grainy",
        "Solid",
        "HDR Tone",
        "Solarized Effect",
    ],
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
    enable_grkit: "启用 GRKit",
    enable_at_start: "开机启用",
    keep_last: "保持上次",
    mode_dial: "模式转盘",
    dial_none: "无",
    dial_off: "关闭 GRKit",
    dial_saved: "已保存设置",
    grkit_settings: "GRKit 设置",
    distance_unit: "距离单位",
    meters: "米",
    feet: "英尺",
    adj_order: "ADJ 菜单顺序",
    options: "可选项",
    about: "关于 GRKit",
    project: "项目地址",
    adjustments: "效果参数",
    film: "胶卷",
    turn: "切换",
    back: "返回",
    recipe_undo: "撤销",
    recipe_save: "保存设置",
    recipe_saved: "已保存：",
    recipe_delete: ("删除", "？"),
    recipe_full: "配方已满，在配方上按 Fn 可删除",
    recipe_missing: "未安装此效果",
    recipe_name: "配方",
    recipe_unsupported: "此相机不支持这个配方",
    recipe_no_grain: "颗粒感需要颗粒感模块",
    recipe_actions: ["重命名", "删除"],
    recipe_apply: "应用",
    recipe_no_glow: "光晕需要光晕模块",
    recipe_missing_mark: "缺少效果",
    native_types: [
        "标准",
        "鲜艳",
        "单色",
        "柔和单色",
        "硬调单色",
        "高对比度黑白",
        "负片",
        "正片",
        "漂白效果",
        "复古",
        "HDR 色调",
        "交叉冲印",
        "电影（黄）",
        "电影（绿）",
        "标准",
        "柔和",
        "高对比度",
        "颗粒感",
        "无颗粒感",
        "HDR色调",
        "中途曝光效果",
    ],
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
    enable_grkit: "啟用 GRKit",
    enable_at_start: "開機啟用",
    keep_last: "保持上次",
    mode_dial: "模式轉盤",
    dial_none: "無",
    dial_off: "關閉 GRKit",
    dial_saved: "已儲存設定",
    grkit_settings: "GRKit 設定",
    distance_unit: "距離單位",
    meters: "公尺",
    feet: "英尺",
    adj_order: "ADJ 選單順序",
    options: "可選項",
    about: "關於 GRKit",
    project: "專案網址",
    adjustments: "效果參數",
    film: "膠卷",
    turn: "切換",
    back: "返回",
    recipe_undo: "復原",
    recipe_save: "儲存設定",
    recipe_saved: "已儲存：",
    recipe_delete: ("刪除", "？"),
    recipe_full: "配方已滿，在配方上按 Fn 可刪除",
    recipe_missing: "未安裝此效果",
    recipe_name: "配方",
    recipe_unsupported: "此相機不支援這個配方",
    recipe_no_grain: "粒紋感需要粒紋感模組",
    recipe_actions: ["重命名", "刪除"],
    recipe_apply: "套用",
    recipe_no_glow: "光暈需要光暈模組",
    recipe_missing_mark: "缺少效果",
    native_types: [
        "標準",
        "鮮豔",
        "單色",
        "柔和單色",
        "硬調單色",
        "高對比度黑白",
        "負片",
        "正片",
        "漂白效果",
        "復古",
        "HDR 色調",
        "交叉沖印",
        "電影（黃）",
        "電影（綠）",
        "標準",
        "柔和",
        "高對比度",
        "顆粒感",
        "無顆粒感",
        "HDR色調",
        "中途曝光效果",
    ],
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
    enable_grkit: "GRKit を有効",
    enable_at_start: "起動時に有効",
    keep_last: "前回のまま",
    mode_dial: "モードダイヤル",
    dial_none: "なし",
    dial_off: "GRKit オフ",
    dial_saved: "保存した設定",
    grkit_settings: "GRKit 設定",
    distance_unit: "距離の単位",
    meters: "メートル",
    feet: "フィート",
    adj_order: "ADJ メニューの順番",
    options: "オプション",
    about: "GRKit について",
    project: "プロジェクト",
    adjustments: "パラメーター",
    film: "フィルム銘柄",
    turn: "切替",
    back: "戻る",
    recipe_undo: "元に戻す",
    recipe_save: "設定を保存",
    recipe_saved: "保存しました:",
    recipe_delete: ("", "を削除しますか?"),
    recipe_full: "上限です。Fn でレシピを削除",
    recipe_missing: "エフェクト未インストール",
    recipe_name: "レシピ",
    recipe_unsupported: "このカメラでは使えません",
    recipe_no_grain: "粒状感には粒状感モジュールが必要",
    recipe_actions: ["名称変更", "消去"],
    recipe_apply: "適用",
    recipe_no_glow: "グローにはグローモジュールが必要",
    recipe_missing_mark: "エフェクトなし",
    native_types: [
        "スタンダード",
        "ビビッド",
        "モノトーン",
        "ソフトモノトーン",
        "ハードモノトーン",
        "ハイコントラスト白黒",
        "ネガフィルム調",
        "ポジフィルム調",
        "ブリーチバイパス",
        "レトロ",
        "HDR調",
        "クロスプロセス",
        "シネマ調(イエロー)",
        "シネマ調(グリーン)",
        "スタンダード",
        "ソフト",
        "ハイコントラスト",
        "グレイニー",
        "ソリッド",
        "HDR調",
        "ソラリゼーション調",
    ],
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
    enable_grkit: "GRKit 사용",
    enable_at_start: "시작 시 사용",
    keep_last: "이전 상태",
    mode_dial: "모드 다이얼",
    dial_none: "없음",
    dial_off: "GRKit 끄기",
    dial_saved: "저장된 설정",
    grkit_settings: "GRKit 설정",
    distance_unit: "거리 단위",
    meters: "미터",
    feet: "피트",
    adj_order: "ADJ 메뉴 순서",
    options: "옵션",
    about: "GRKit 정보",
    project: "프로젝트",
    adjustments: "파라미터",
    film: "필름 종류",
    turn: "전환",
    back: "뒤로",
    recipe_undo: "실행 취소",
    recipe_save: "설정 저장",
    recipe_saved: "저장됨: ",
    recipe_delete: ("", " 삭제?"),
    recipe_full: "가득 참. 레시피에서 Fn으로 삭제",
    recipe_missing: "효과가 설치되지 않음",
    recipe_name: "레시피",
    recipe_unsupported: "이 카메라에서 쓸 수 없음",
    recipe_no_grain: "입자감에는 입자감 모듈이 필요함",
    recipe_actions: ["이름 변경", "삭제"],
    recipe_apply: "적용",
    recipe_no_glow: "글로우에는 글로우 모듈이 필요함",
    recipe_missing_mark: "효과 없음",
    native_types: [
        "스탠다드",
        "비비드",
        "모노톤",
        "소프트 모노톤",
        "하드 모노톤",
        "하이 콘트라스트 흑백",
        "네거티브 필름",
        "포지티브 필름",
        "블리치 바이패스",
        "레트로",
        "HDR 톤",
        "크로스 프로세스",
        "시네마(옐로)",
        "시네마(그린)",
        "표준",
        "소프트",
        "하이 콘트라스트",
        "그레이니",
        "솔리드",
        "HDR톤",
        "솔라리제이션톤",
    ],
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
        3,
        "The switch that stops each module at once: its ADJ summary while on, and while off, then its MENU row.",
    ),
    run(
        "SEPARATOR",
        1,
        "Between a group title and its value in the ADJ root summary.",
    ),
    run(
        "RECIPE",
        15,
        "The page of recipes: the Undo and Save rows, the text before a saved name, \
         the text before and after the name to delete, the message when the recipes are \
         full, the text of a missing effect, the name of a recipe without a look, the \
         message of a value that the camera does not have, \
         the message of an apply whose Grain has no effect, then the two buttons of Fn on a camera recipe: Rename and Delete, then the \
         message of an apply whose Glow module is not loaded, and the mark after the name of a \
         recipe whose effect is not installed, then the Apply row of the Recipes page of MENU.",
    ),
    run(
        "NATIVE_TYPE",
        21,
        "The names of the native Image Control types, in the order of runtime/native.h.",
    ),
    run(
        "START",
        2,
        "Enable at Start, which decides the switch at each start: its MENU row, then its \
         Keep Last value. Its On and Off are TEXT_ON and TEXT_OFF.",
    ),
    run(
        "DIAL",
        4,
        "The Mode Dial row of the Recipes page of MENU, then the values of a user mode \
         besides a recipe: None, GRKit Off, and Saved Settings.",
    ),
    run(
        "UNIT",
        4,
        "GRKit Settings, the MENU row and the title of the settings page of the core, \
         then its Distance Unit row and the values Meters and Feet.",
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
        "SWITCH" => vec![
            summary(w.enable, w.on),
            summary(w.enable, w.off),
            w.enable_grkit.into(),
        ],
        "SEPARATOR" => vec![w.separator.into()],
        "RECIPE" => words(&[
            w.recipe_undo,
            w.recipe_save,
            w.recipe_saved,
            w.recipe_delete.0,
            w.recipe_delete.1,
            w.recipe_full,
            w.recipe_missing,
            w.recipe_name,
            w.recipe_unsupported,
            w.recipe_no_grain,
        ])
        .into_iter()
        .chain(words(&w.recipe_actions))
        .chain(words(&[w.recipe_no_glow, w.recipe_missing_mark]))
        .chain(words(&[w.recipe_apply]))
        .collect(),
        "NATIVE_TYPE" => words(&w.native_types),
        "START" => words(&[w.enable_at_start, w.keep_last]),
        "DIAL" => words(&[w.mode_dial, w.dial_none, w.dial_off, w.dial_saved]),
        "UNIT" => words(&[w.grkit_settings, w.distance_unit, w.meters, w.feet]),
        _ => unreachable!("Unknown menu text run {name}"),
    }
}

/// The camera names of the native Image Control types in a language, in the
/// order of `native::TYPES`.
pub fn native_type_names(language: Language) -> &'static [&'static str; 21] {
    &words(language).native_types
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
        assert_eq!(english[id("SEPARATOR")], ": ");
        assert_eq!(
            strings(Language::SimplifiedChinese, VERSION)[id("SEPARATOR")],
            "："
        );
    }

    #[test]
    fn the_switch_summary_gives_its_value_in_each_language() {
        for language in Language::ALL {
            let w = words(language);
            let list = strings(language, VERSION);
            assert_eq!(
                list[id("SWITCH")],
                format!("{}{}{}", w.enable, w.separator, w.on)
            );
            assert_eq!(
                list[id("SWITCH") + 1],
                format!("{}{}{}", w.enable, w.separator, w.off)
            );
            assert!(list[id("SWITCH") + 2].contains("GRKit"));
        }
    }

    /// Enable at Start shows the On and Off of the switch, so only its row
    /// and Keep Last are new texts.
    #[test]
    fn enable_at_start_has_its_row_and_keep_last_in_each_language() {
        let expected = [
            (Language::English, "Enable at Start", "Keep Last"),
            (Language::SimplifiedChinese, "开机启用", "保持上次"),
            (Language::TraditionalChinese, "開機啟用", "保持上次"),
            (Language::Japanese, "起動時に有効", "前回のまま"),
            (Language::Korean, "시작 시 사용", "이전 상태"),
        ];
        for (language, row, keep) in expected {
            let list = strings(language, VERSION);
            assert_eq!(list[id("START")], row);
            assert_eq!(list[id("START") + 1], keep);
            assert_eq!(language.undrawable(row), None);
            assert_eq!(language.undrawable(keep), None);
        }
        assert!(strings(Language::English, VERSION)[id("START")..]
            .iter()
            .all(|text| text.is_ascii()));
    }

    /// GRKit Settings holds Enable at Start and Distance Unit, whose
    /// values are Meters and Feet.
    #[test]
    fn grkit_settings_has_its_rows_and_units_in_each_language() {
        let expected = [
            (
                Language::English,
                ["GRKit Settings", "Distance Unit", "Meters", "Feet"],
            ),
            (
                Language::SimplifiedChinese,
                ["GRKit 设置", "距离单位", "米", "英尺"],
            ),
            (
                Language::TraditionalChinese,
                ["GRKit 設定", "距離單位", "公尺", "英尺"],
            ),
            (
                Language::Japanese,
                ["GRKit 設定", "距離の単位", "メートル", "フィート"],
            ),
            (
                Language::Korean,
                ["GRKit 설정", "거리 단위", "미터", "피트"],
            ),
        ];
        for (language, texts) in expected {
            let list = strings(language, VERSION);
            for (i, text) in texts.iter().enumerate() {
                assert_eq!(list[id("UNIT") + i], *text);
                assert_eq!(language.undrawable(text), None);
            }
        }
    }

    /// The Mode Dial page names U1 to U3 as the dial does and shows a recipe
    /// by its name, and its Save Settings row is that of the Recipes page,
    /// so only its row, None, GRKit Off, and Saved Settings are new texts.
    #[test]
    fn the_mode_dial_has_its_row_and_values_in_each_language() {
        let expected = [
            (
                Language::English,
                ["Mode Dial", "None", "GRKit Off", "Saved Settings"],
            ),
            (
                Language::SimplifiedChinese,
                ["模式转盘", "无", "关闭 GRKit", "已保存设置"],
            ),
            (
                Language::TraditionalChinese,
                ["模式轉盤", "無", "關閉 GRKit", "已儲存設定"],
            ),
            (
                Language::Japanese,
                ["モードダイヤル", "なし", "GRKit オフ", "保存した設定"],
            ),
            (
                Language::Korean,
                ["모드 다이얼", "없음", "GRKit 끄기", "저장된 설정"],
            ),
        ];
        for (language, texts) in expected {
            let list = strings(language, VERSION);
            for (k, text) in texts.iter().enumerate() {
                assert_eq!(list[id("DIAL") + k], *text);
                assert_eq!(language.undrawable(text), None);
            }
        }
        assert!(strings(Language::English, VERSION)[id("DIAL")..]
            .iter()
            .all(|text| text.is_ascii()));
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
