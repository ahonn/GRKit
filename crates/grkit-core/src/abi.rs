//! The private memory layout that the loader, the installer, and the ARM code
//! share. These tables are its only source: `header()` generates
//! `runtime/abi.h`, and the `generated` test keeps the checked-in file current.

use std::fmt::Write;

use crate::{capabilities, effects, hue, menu_text, selective};

/// A byte range of the private header, of a record, or of a GRF file.
pub struct Field {
    /// The C name without the `GRKIT_` prefix.
    pub name: &'static str,
    pub offset: usize,
    pub size: usize,
    pub doc: &'static str,
}

/// Declares a `usize` constant for each field and a table of the fields.
macro_rules! fields {
    ($table:ident { $($(#[doc = $doc:literal])* $name:ident = $offset:expr, $size:expr;)* }) => {
        $($(#[doc = $doc])* pub const $name: usize = $offset;)*
        pub const $table: &[Field] = &[$(Field {
            name: stringify!($name),
            offset: $offset,
            size: $size,
            doc: concat!($($doc, "\n"),*),
        }),*];
    };
}

// The capacity of the runtime. `runtime` exports these too.
/// The icon slots of one icon chunk, which one script array holds.
pub const ICONS_PER_CHUNK: usize = 40;
/// The largest script array in words, which holds the modules.
pub const MODULE_ARRAY_WORDS: usize = 65536;
/// The most modules that GRKit lists at a start, from the package and from
/// `GRKIT/MODULES` together. The package modules come first, then the first
/// other ones in their default order. The GRKit section of MENU has a row
/// for each listed module, and each build keeps room for them in the
/// Customize tab (runtime/menu_section.h).
pub const MODULES_MAX: usize = 10;
/// The most settings of all modules (`runtime/settings.h`).
pub const MODULE_SETTINGS_MAX: usize = 160;
/// The most artwork tiles of all modules. The installer puts them in the
/// icon chunks after the effect icons.
pub const MODULE_TILES_MAX: usize = 128;
/// Off, GRKit, Version, and Modules.
/// The menu addresses the Version and Modules icons by their offset from the
/// GRKit icon, so the host icons come first in chunk 0, and they must fit in
/// it. The effect icons follow them.
pub const HOST_ICON_COUNT: usize = 4;
const _: () = assert!(HOST_ICON_COUNT <= ICONS_PER_CHUNK);
/// Product limit for private Image Control effects. Each effect takes a
/// record of the registry array and an icon slot, about 10.5 KB of script
/// memory. The registry holds only the header and the records, and they fit
/// one array of 65536 words. The tail is an array of its own.
pub const MAX_COUNT: usize = 40;
/// The chunk addresses that the header holds (`GRKIT_HOST_ICON_CHUNKS`).
pub const ICON_CHUNK_SLOTS: usize = 5;

/// The fields of the header.
pub const HOST_HEADER_SIZE: usize = 768;
/// The placeholder for the header address in ARM code. The installer replaces
/// each literal word with this value by the address of the `registry` array.
pub const HEADER_REFERENCE: u32 = 0x58012340;
/// `GRM1`.
pub const HOST_MAGIC_VALUE: u32 = 0x314d5247;
pub const HOST_VERSION_VALUE: u32 = 1;
/// The most module pages that the header holds: one for each listed module.
pub const MODULE_PAGES_MAX: usize = MODULES_MAX;
/// The most module layers, and the most module plane filters, of all modules
/// that the header holds.
pub const MODULE_LAYERS_MAX: usize = 16;
pub const MODULE_FILTERS_MAX: usize = 8;

fields!(HEADER_FIELDS {
    /// Number of Image Control records.
    HOST_COUNT = 0, 4;
    /// Address of record 0. The records follow the header without gaps.
    HOST_RECORDS = 4, 4;
    /// Selected record, or 0 for native processing.
    HOST_ACTIVE = 8, 4;
    /// The ScreenDevice of the last highlight warning setting
    /// (ScreenDevice::OnSetLevelAlarm), or 0 (runtime/framing.c).
    HOST_LEVEL_SCREEN = 12, 4;
    /// GRKIT_HOST_MAGIC_VALUE after the installer writes the header.
    HOST_MAGIC = 16, 4;
    /// GRKIT_HOST_VERSION_VALUE.
    HOST_VERSION = 20, 4;
    /// The mode of that setting: 0 while the warning is off. A change of the
    /// effect sets the warning again with this mode.
    HOST_LEVEL_MODE = 24, 4;
    /// Number of parameter builds. A diagnostic.
    HOST_PROCESSING_CALLS = 28, 4;
    /// ADJ controller that shows the private menu, or 0.
    HOST_MENU_CONTROLLER = 32, 4;
    /// Private menu level: 0 native, MENU_ROOT, or MENU_MODULE (runtime/modules.h).
    HOST_MENU_LEVEL = 36, 4;
    /// Highlighted row of the private menu.
    HOST_MENU_CURSOR = 40, 4;
    /// Address of the tail, an array of its own: the picture scratch, the
    /// menu text, then the live-view labels. INSTALL writes it.
    HOST_TAIL = 44, 4;
    /// Grain level, the capability CORE.GRAIN: 0 Off, 1 weak, 2 medium, 3
    /// strong. The native grain stage of recorded pictures uses it instead of
    /// the native grain settings.
    HOST_GRAIN = 48, 4;
    /// Address of the module list of the Modules page (runtime/manage.h), or
    /// 0 without a listed module. INSTALL writes it.
    HOST_MANAGE = 52, 4;
    /// Address of the state of the core services (runtime/services.h): the
    /// event subscribers, the timers, and the UI. INSTALL writes it, with
    /// the module array.
    HOST_SERVICES = 56, 4;
    /// Nonzero while the menu rebuilds the native list. The list callback and the
    /// native Image Control preview ignore the changes.
    HOST_MENU_REBUILDING = 60, 4;
    /// Native list position, saved when the private menu opens.
    HOST_MENU_LIST = 64, 8;
    /// The module + 1 whose native settings page the menu opened (Module.settings,
    /// runtime/core.h), or 0. While it is set, the
    /// hooks of the borrowed native page show the settings of that module. A
    /// change to a controller outside the page clears it.
    HOST_SETTINGS = 72, 4;
    /// Number of ADJ events. GRKIT_HOST_MENU_EVENT_RING holds the last ones.
    HOST_MENU_EVENTS = 76, 4;
    /// GRKIT_HOST_STATE_CHANGES when the live view last took a redraw for
    /// it. A changed setting redraws the live view once (runtime/framing.c).
    HOST_FRAME_CHANGES = 80, 4;
    /// Icon of Off, or 0 for the native icon.
    HOST_OFF_ICON = 84, 4;
    /// How the private menu opened: 1 or 2 across an edge of the native slots,
    /// 3 by touch or on the return from the native parameters.
    HOST_MENU_EDGE = 88, 4;
    /// MENU was pressed, and its confirm event has not arrived yet.
    HOST_BACK_PENDING = 92, 4;
    /// Text descriptor of the ADJ title: (length + 1) | 0x100, then the address.
    HOST_MENU_TITLE = 96, 8;
    /// Text descriptor of the ADJ detail line.
    HOST_MENU_DETAIL = 104, 8;
    /// The last ADJ events: kind | argument << 16.
    HOST_MENU_EVENT_RING = 112, 32;
    /// The record whose adjustments the native parameter page edits, or 0. Fn
    /// in GRKit Image Control stores the record with bit 0 set, and the page
    /// start clears bit 0. A page that native ADJ opens clears the word.
    HOST_PARAMETERS = 144, 4;
    /// Number of adjustment changes. The live view rebuilds its stages when it
    /// changes.
    HOST_PARAMETER_CHANGES = 148, 4;
    /// GRKIT_HOST_PARAMETER_CHANGES at the last live-view build.
    HOST_LIVE_PARAMETER_CHANGES = 152, 4;
    /// Icon chunk addresses: chunk n holds icon slots 40 n to 40 n + 39. The
    /// host icons come first in chunk 0, then the effect icons.
    HOST_ICON_CHUNKS = 156, 4 * ICON_CHUNK_SLOTS;
    /// Icon of the GRKit tile.
    HOST_GRKIT_ICON = 176, 4;
    /// Private toolbar cell, or 0.
    HOST_TOOLBAR_WIDGET = 180, 4;
    /// ADJ controller of the private toolbar cell.
    HOST_TOOLBAR_OWNER = 184, 4;
    /// Module that the private menu shows at MENU_MODULE. At the end of a
    /// private session it holds the module of the root row where the next
    /// session opens (menu_reopen).
    HOST_MODULE = 188, 4;
    /// Load cue: 0 none, 1 after INSTALL, then 2 + the local second of the
    /// day when the still live view first shows the cue.
    HOST_LOAD_CUE = 192, 4;
    /// Aspect id 2 variant for the photo shown in playback: 0 none (shooting uses
    /// the Framing ratio), 1 not 65:24, 2 65:24.
    HOST_FRAME_FILE = 196, 4;
    /// Record of the last live-view build: an owned record, or 0 for native processing.
    HOST_LIVE_RECORD = 200, 4;
    /// The capabilities in use: bit n is set when a loaded module uses
    /// capability n (GRKIT_CORE_SETTING_BASE + n). INSTALL writes it, and the
    /// resident loader restores only the values of these capabilities.
    HOST_CAPABILITIES = 204, 4;
    /// Framing ratio: 0 native, then the Framing rows.
    HOST_FRAME_RATIO = 208, 4;
    /// ADJ Framing page preview: 0 none, else the highlighted ratio + 1.
    HOST_FRAME_PREVIEW = 212, 4;
    /// Number of Framing paints. A diagnostic.
    HOST_FRAME_DRAWS = 216, 4;
    /// ADJ controller of the preview. Its canvas shows the preview. ADJ can
    /// make its canvas after its first refresh, so a draw reads the canvas
    /// from the controller. The word stays set until a full draw erases the
    /// mask.
    HOST_FRAME_PREVIEW_ADJ = 220, 4;
    /// Address of the effect list of the Effects page (runtime/effects.h), or
    /// 0 without a listed effect. INSTALL writes it.
    HOST_EFFECTS = 224, 4;
    /// Live view controller that Framing draws on, or 0.
    HOST_FRAME_OWNER = 228, 4;
    /// Canvas of that controller.
    HOST_FRAME_CANVAS = 232, 4;
    /// Bitmap of the last Framing paint, or 0 when nothing is drawn.
    HOST_FRAME_BITMAP = 236, 4;
    /// Grain size, the capability CORE.GRAIN_SIZE: 0 Auto (the ISO of the
    /// shot), 1 small, 2 medium, 3 large.
    HOST_GRAIN_SIZE = 240, 4;
    /// Live-view-only override: 0 none, 1 native processing, else an owned record.
    HOST_PREVIEW = 244, 4;
    /// The grain of the shot whose grain stage was set up last: 0 the native
    /// noise, else the size 1 small, 2 medium, 3 large. The GR III noise fill
    /// reads it (runtime/picture.c).
    HOST_GRAIN_SHOT = 248, 4;
    /// Return to GRKit Image Control after the native parameter page: 1 opening,
    /// 2 away. 3: ADJ exited from GRKit, so the next ADJ opens GRKit. 4: MENU
    /// or OK on GRKit passed the event that closes ADJ to the native list.
    HOST_RETURN = 252, 4;
    /// Number of changes of the saved selections: the effect, CORE.ASPECT,
    /// CORE.GRAIN, CORE.GRAIN_SIZE, the Fn adjustments, and the module settings
    /// (GRKIT_STATE_CHANGED). The resident loader polls only this word and
    /// saves the state file after it changes.
    HOST_STATE_CHANGES = 256, 4;
    /// Number of module plane filters in GRKIT_HOST_MODULE_FILTERS. INSTALL
    /// writes it.
    HOST_MODULE_FILTER_COUNT = 260, 4;
    /// Address of the profile table (GRKIT_PROFILE in the tail) while the
    /// development profile runs, else 0. INSTALL writes it when the card has
    /// GRKIT/PROFILE or PROFILE.TXT.
    HOST_PROFILE = 264, 4;
    /// Two words for each module plane filter, in stage order: its apply
    /// function, and its setting handle in the low half with its stage in the
    /// high half.
    HOST_MODULE_FILTERS = 268, 8 * MODULE_FILTERS_MAX;
    /// The crop of the capture size hooks (runtime/framing.S): the Framing
    /// ratio where it applies in the native aspect (aspect_value), else 0.
    /// Each live-view update sets it, so a native aspect that the camera menu
    /// sets takes the stock sizes.
    HOST_FRAME_CROP = 268 + 8 * MODULE_FILTERS_MAX, 4;
    /// The listed modules that ADJ can show, a bit each, which ADJ Order
    /// lists (runtime/manage.h). The menu sets it. The text hooks read this
    /// word, so they do not walk the module pages.
    HOST_ADJ_MODULES = 272 + 8 * MODULE_FILTERS_MAX, 4;
    /// Reserved for later fields.
    HOST_RESERVED = 276 + 8 * MODULE_FILTERS_MAX, HOST_MODULE_SETTINGS - 276 - 8 * MODULE_FILTERS_MAX;
    /// Address of the module settings table, or 0 (runtime/settings.h).
    /// INSTALL writes it.
    HOST_MODULE_SETTINGS = HOST_MODULE_PREVIEW - 4, 4;
    /// The ADJ preview of a module page: its setting handle plus 1 (0 for
    /// none), then the highlighted value. Module layers of that setting paint
    /// the highlighted value on the ADJ canvas.
    HOST_MODULE_PREVIEW = HOST_MODULE_PAGE_COUNT - 8, 8;
    /// Number of module pages in GRKIT_HOST_MODULE_PAGES. INSTALL writes it.
    HOST_MODULE_PAGE_COUNT = HOST_MODULE_PAGES - 4, 4;
    /// Three words for each module page (runtime/sdk/grkit.h), in load order:
    /// the address of its GrkitPage, its setting handle, and the global index
    /// of its tile 0 (GRKIT_ICON_MODULE_TILE).
    HOST_MODULE_PAGES = HOST_MODULE_CHOICE - 12 * MODULE_PAGES_MAX, 12 * MODULE_PAGES_MAX;
    /// The module page that the generic page callbacks of the menu serve.
    HOST_MODULE_CHOICE = HOST_MODULE_LAYER_COUNT - 4, 4;
    /// Number of module layers in GRKIT_HOST_MODULE_LAYERS. INSTALL writes it.
    HOST_MODULE_LAYER_COUNT = HOST_MODULE_LAYERS - 4, 4;
    /// Three words for each module layer, in painting order: its paint
    /// function, its setting handle (GRKIT_NO_SETTING paints with the value
    /// 1), and its flags in the low half with its order in the high half.
    HOST_MODULE_LAYERS = HOST_GAMMA_KEY - 12 * MODULE_LAYERS_MAX, 12 * MODULE_LAYERS_MAX;
    /// Key of the inputs of the last private curve composition in the gamma
    /// stage, or 0 (runtime/processing.h).
    HOST_GAMMA_KEY = HOST_HEADER_SIZE - 4, 4;
});

pub const RECORD_SIZE: usize = 0x1100;

fields!(RECORD_FIELDS {
    /// The native processing record: a prefix, then the tone curves.
    RECORD_COEFFICIENTS = 0x54, 0x668 - 0x54;
    /// UTF-16 effect name with its NUL.
    RECORD_NAME = 0x668, 34;
    /// Nonzero for a monochrome effect.
    RECORD_MONOCHROME = 0x690, 4;
    /// Address of the effect icon.
    RECORD_ICON = 0x694, 4;
    /// Effect ID with its NUL.
    RECORD_ID = 0x698, 32;
    /// u16 ImageTone code recorded in photos: 0x8000 | hash of the effect ID.
    RECORD_CODE = 0x6f8, 2;
    /// GRF extension, zero for an effect without features.
    RECORD_FEATURES = 0x700, 4;
    /// i16[9] signed Q13 RGB matrix after the tone curves, applied through the
    /// YCbCr matrix.
    RECORD_POST = 0x704, 18;
    /// Selective color for recorded pictures: i16 cos and sin of the target hue
    /// (Q12), u16 inner and outer tangents toward smaller hue angles, then toward
    /// larger ones (Q8), and u8 chroma gate c0 and c1. Bytes 0x724..0x72b are
    /// reserved and zero.
    RECORD_SELECTIVE = 0x716, 0x72c - 0x716;
    /// Fn adjustments of the effect: saturation, hue, high/low key, contrast,
    /// highlight contrast, shadow contrast, sharpness, shading, clarity. Each
    /// byte is (native value - 4) mod 9, so zero is neutral. A native value is
    /// 0..8, and 4 is neutral.
    RECORD_PARAMETERS = 0x72c, 9;
    /// The MultiAxial parameter set that INSTALL expands from the hue sectors of the GRF
    /// (feature 8): the address of the set at +0, the address of the highlight-correction
    /// set at +4, the set (six pointers) at +8, the bounds at +0x20, the bank selectors at
    /// +0x40, the matrix header at +0x58, the control fields at +0x6c, 60 3x3 i32 Q10
    /// matrices at +0x98, the highlight-correction set at +0x908, and its control fields
    /// at +0x920. Zero without hue sectors.
    RECORD_MULTIAXIAL = 0x740, 0x980;
});

/// The R, G, and B tone curves inside `RECORD_COEFFICIENTS`: 256 14-bit samples
/// each, `RECORD_CURVE_SIZE` bytes apart. They end where the coefficients end.
pub const RECORD_CURVES: usize = 0x68;
pub const RECORD_CURVE_SIZE: usize = 0x200;
const _: () = assert!(
    RECORD_CURVES > RECORD_COEFFICIENTS && RECORD_CURVES + 3 * RECORD_CURVE_SIZE == RECORD_NAME
);

// The GRFX0001 effect file (`effects`), which SCAN and INSTALL read. A file
// has `effects::SIZE` bytes, or `effects::SIZE_WITH_LEVELS` with the hue
// levels of feature 16 after the first `effects::SIZE` bytes. Each byte that
// no field holds is zero.
fields!(GRF_FIELDS {
    /// The magic `GRFX0001`.
    GRF_MAGIC = 0, 8;
    /// u32 size of the file.
    GRF_BYTES = 8, 4;
    /// u32 CRC-32 of the file, which reads these bytes as zero.
    GRF_CRC = 12, 4;
    /// The effect ID: NUL-terminated and zero-padded.
    GRF_ID = 16, 32;
    /// The effect name: NUL-terminated and zero-padded ASCII.
    GRF_NAME = 48, 32;
    /// u32 flags: 1 for a monochrome effect.
    GRF_FLAGS = 80, 4;
    /// u32 extension features (GRKIT_FEATURE_*), zero without the extension.
    GRF_FEATURES = 84, 4;
    /// i16[9] signed Q13 RGB matrix after the tone curves (feature 1).
    GRF_POST = 88, 18;
    /// Selective color (feature 2): i16 cos and sin of the target hue, four u16
    /// tangents, and two u8 chroma limits.
    GRF_SELECTIVE = 106, selective::SIZE;
    /// Reserved and zero.
    GRF_RESERVED = 120, 8;
    /// i16[9] signed Q13 color matrix.
    GRF_MATRIX = 128, 18;
    /// Zero.
    GRF_PADDING = 146, 2;
    /// The R, G, and B tone curves: 256 14-bit u16 samples each.
    GRF_CURVES = 148, 3 * 512;
    /// The 40 x 40 RGBA8 icon, with binary alpha.
    GRF_ICON = 1684, effects::ICON_SIZE;
    /// Hue sectors of feature 8: 11 x 6 i8 coefficients (GRKIT_HUE_MATRICES
    /// bytes), then the u16 control fields 10..20 of the weight curve.
    GRF_HUE = 8084, hue::SIZE;
});

// INSTALL copies a GRF into a record (runtime/installer.c): the matrix and the
// curves to the coefficients, then the post matrix, the selective color, and
// the reserved bytes.
const _: () = assert!(
    RECORD_NAME - RECORD_COEFFICIENTS == GRF_ICON - GRF_MATRIX
        && RECORD_CURVES - RECORD_COEFFICIENTS == GRF_CURVES - GRF_MATRIX
        && RECORD_SELECTIVE - RECORD_POST == GRF_SELECTIVE - GRF_POST
        && RECORD_PARAMETERS - RECORD_POST == GRF_MATRIX - GRF_POST
);

/// The plan file that SCAN writes and INSTALL and the loader read
/// (`PLAN.BIN`): this header, the GRF file of each installed effect
/// (`GRKIT_GRF_MAX_SIZE` bytes each), then an entry for each listed effect.
/// The loader reads the words up to `PLAN_MODULE_TILES` in order.
pub const PLAN_HEADER_SIZE: usize = 40;
/// The first word of the plan.
pub const PLAN_MAGIC_VALUE: u32 = HOST_MAGIC_VALUE;
/// The modes of `PLAN_MODES`.
pub const PLAN_SAFE: u32 = 1;
pub const PLAN_PROFILE: u32 = 2;

fields!(PLAN_FIELDS {
    /// GRKIT_PLAN_MAGIC_VALUE.
    PLAN_MAGIC = 0, 4;
    /// Number of installed effects.
    PLAN_COUNT = 4, 4;
    /// Words of the registry array: the private header and the records.
    PLAN_HEADER_WORDS = 8, 4;
    /// Number of icon chunk arrays.
    PLAN_ICON_CHUNKS = 12, 4;
    /// Bytes of the plan.
    PLAN_SIZE = 16, 4;
    /// Words of the module array, or 0 without modules.
    PLAN_MODULE_WORDS = 20, 4;
    /// Artwork tiles of the usable modules.
    PLAN_MODULE_TILES = 24, 4;
    /// GRKIT_PLAN_SAFE and GRKIT_PLAN_PROFILE.
    PLAN_MODES = 28, 4;
    /// Effect files past the listed ones.
    PLAN_OVERFLOW = 32, 4;
    /// Number of listed effects.
    PLAN_LISTED = 36, 4;
});

/// A listed effect in the plan.
pub const PLANNED_EFFECT_SIZE: usize = 92;

fields!(PLANNED_FIELDS {
    /// The head of its GRF file, which holds its ID and name.
    PLANNED_HEAD = 0, 64;
    /// Its file name with its NUL.
    PLANNED_FILE = 64, 16;
    /// Its status at this start (runtime/effects.h).
    PLANNED_STATUS = 80, 4;
    /// Its choice in the GRKit section of MENU.
    PLANNED_CHOICE = 84, 4;
    /// Its record, or EFFECTS_NO_RECORD.
    PLANNED_RECORD = 88, 4;
});

/// Bytes of the date stamp core and halo masks at the start of the picture scratch.
pub const PICTURE_MASKS: usize = 141376;
/// Entries of the shot ring.
pub const SHOT_ENTRIES: usize = 8;
/// The shot ring: a write count, the turns of the last shot with a known
/// posture (`SHOT_LAST_TURNS`), then entry n at `SHOT_ENTRY + SHOT_STRIDE * n`,
/// which holds the key at `SHOT_KEY`, the record at `SHOT_RECORD`, and the
/// posture of the electronic level at `SHOT_POSTURE`.
pub const SHOT_LAST_TURNS: usize = 4;
pub const SHOT_ENTRY: usize = 8;
pub const SHOT_STRIDE: usize = 16;
pub const SHOT_KEY: usize = 0;
pub const SHOT_RECORD: usize = 4;
pub const SHOT_POSTURE: usize = 8;
/// The picture scratch after the records: the masks, then the shot ring.
pub const PICTURE_SCRATCH: usize = PICTURE_MASKS + SHOT_ENTRY + SHOT_STRIDE * SHOT_ENTRIES;
/// Bytes of the live-view labels after the menu text (`artwork::overlay_labels`).
pub const LABELS_SIZE: usize = 16384;
/// The profile table after the labels (runtime/profile.h): for each of the
/// first `PROFILE_MODULES` listed modules its code start, its code end, and 8
/// ASCII bytes of its name, then a
/// counter slot of calls, cycles, and the largest call for each core point
/// and for each module and kind.
pub const PROFILE: usize = PICTURE_SCRATCH + menu_text::TEXT_SIZE + LABELS_SIZE;
pub const PROFILE_MODULE_SIZE: usize = 16;
pub const PROFILE_NAME_BYTES: usize = 8;
/// The modules that the profile counts. The loader writes the lines of each
/// in its own script code, so more modules make the resident script longer.
pub const PROFILE_MODULES: usize = 8;
const _: () = assert!(PROFILE_MODULES <= MODULES_MAX);
pub const PROFILE_SLOTS: usize = PROFILE_MODULES * PROFILE_MODULE_SIZE;
pub const PROFILE_SLOT_SIZE: usize = 12;
/// The core points, in slot order: the name in the profile file, and the C
/// name of the slot without the `GRKIT_PROFILE_` prefix.
pub const PROFILE_POINTS: [(&str, &str); 7] = [
    ("frame.bind", "FRAME_BIND"),
    ("frame.prepare", "FRAME_PREPARE"),
    ("frame.paint", "FRAME_PAINT"),
    ("picture.post", "PICTURE_POST"),
    ("picture.encode", "PICTURE_ENCODE"),
    ("menu.icon", "MENU_ICON"),
    ("picture.grain", "PICTURE_GRAIN"),
];
/// The module kinds, in slot order: a layer paint, a plane filter, and an
/// event handler, timer, or UI callback in the App task.
pub const PROFILE_KINDS: [(&str, &str); 3] =
    [("paint", "PAINT"), ("filter", "FILTER"), ("call", "CALL")];
pub const PROFILE_SLOT_COUNT: usize = PROFILE_POINTS.len() + PROFILE_MODULES * PROFILE_KINDS.len();
pub const PROFILE_SIZE: usize = PROFILE_SLOTS + PROFILE_SLOT_COUNT * PROFILE_SLOT_SIZE;
/// Everything after the records: the picture scratch, the menu text, the
/// live-view labels, then the profile table.
pub const TAIL_SIZE: usize = PROFILE + PROFILE_SIZE;
// The summaries of every record in every menu language end before the
// parameter page title.
const _: () = assert!(
    menu_text::TEXT_SUMMARY
        + MAX_COUNT * crate::language::Language::ALL.len() * menu_text::SUMMARY_UNITS * 2
        <= menu_text::TEXT_SETTINGS
);

/// Wraps `text` into a C comment.
pub(crate) fn comment(out: &mut String, text: &str) {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return;
    }
    let mut lines = vec![String::new()];
    for word in words {
        let line = lines.last_mut().unwrap();
        if !line.is_empty() && line.len() + 1 + word.len() > 76 {
            lines.push(word.to_owned());
        } else {
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(word);
        }
    }
    for (index, line) in lines.iter().enumerate() {
        out.push_str(if index == 0 { "/* " } else { "   " });
        out.push_str(line);
        out.push_str(if index + 1 == lines.len() {
            " */\n"
        } else {
            "\n"
        });
    }
}

fn define(out: &mut String, doc: &str, name: &str, value: impl std::fmt::Display) {
    comment(out, doc);
    writeln!(out, "#define GRKIT_{name} {value}").unwrap();
}

fn table(out: &mut String, fields: &[Field], hex: bool) {
    for field in fields {
        if hex {
            define(out, field.doc, field.name, format!("{:#x}", field.offset));
        } else {
            define(out, field.doc, field.name, field.offset);
        }
        if field.size > 4 && field.size % 4 == 0 && !hex {
            writeln!(out, "#define GRKIT_{}_WORDS {}", field.name, field.size / 4).unwrap();
        }
    }
}

fn array(out: &mut String, doc: &str, name: &str, values: &[impl std::fmt::Display]) {
    let values: Vec<String> = values.iter().map(ToString::to_string).collect();
    define(out, doc, name, format!("{{ {} }}", values.join(", ")));
}

/// The GRF file: the offset and the size of each field, the file sizes, and
/// the longest ID and name.
fn grf_table(out: &mut String) {
    comment(
        out,
        "A GRFX0001 effect file has GRKIT_GRF_SIZE bytes, or GRKIT_GRF_MAX_SIZE with \
         hue levels (feature 16). Each byte that no field holds is zero.",
    );
    writeln!(out, "#define GRKIT_GRF_SIZE {}U", effects::SIZE).unwrap();
    writeln!(
        out,
        "#define GRKIT_GRF_MAX_SIZE {}U",
        effects::SIZE_WITH_LEVELS
    )
    .unwrap();
    for field in GRF_FIELDS {
        define(out, field.doc, field.name, format!("{}U", field.offset));
        writeln!(out, "#define GRKIT_{}_SIZE {}U", field.name, field.size).unwrap();
    }
    writeln!(out, "#define GRKIT_GRF_ID_MAX {}U", effects::ID_MAX).unwrap();
    writeln!(out, "#define GRKIT_GRF_NAME_MAX {}U", effects::NAME_MAX).unwrap();
}

/// The GRF extension features, and the hue sectors that INSTALL expands into the
/// MultiAxial set of a record (`effects`, `hue`).
fn hue_tables(out: &mut String) {
    writeln!(out, "#define GRKIT_FEATURE_POST {}U", effects::FEATURE_POST).unwrap();
    writeln!(
        out,
        "#define GRKIT_FEATURE_SELECTIVE {}U",
        effects::FEATURE_SELECTIVE
    )
    .unwrap();
    writeln!(
        out,
        "#define GRKIT_FEATURE_HUE {}U",
        effects::FEATURE_HUE_SECTORS
    )
    .unwrap();
    writeln!(
        out,
        "#define GRKIT_FEATURE_HUE_LEVELS {}U",
        effects::FEATURE_HUE_LEVELS
    )
    .unwrap();
    writeln!(out, "#define GRKIT_HUE_SIZE {}U", hue::SIZE).unwrap();
    writeln!(out, "#define GRKIT_HUE_MATRICES {}U", hue::SECTORS * 6).unwrap();
    define(
        out,
        "Hue levels of feature 16: the sectors of levels 1 to 3 after the first \
         GRKIT_GRF_SIZE bytes.",
        "HUE_LEVEL_OFFSET",
        format!("{}U", effects::SIZE),
    );
    writeln!(out, "#define GRKIT_HUE_LEVEL_SIZE {}U", hue::LEVEL_SIZE).unwrap();
    array(
        out,
        "The fixed breakpoints of the weight curve, control fields 18..20.",
        "HUE_BORDERS",
        &hue::BORDERS,
    );
    array(
        out,
        "The quadrant bounds and the bank of each segment: 11 equal sectors, sector k \
         in bank k + 1.",
        "HUE_BOUNDS",
        &hue::BOUNDS,
    );
    array(out, "", "HUE_SELECTORS", &hue::SELECTORS);
    array(
        out,
        "Control fields 1..8 that spread the levels, and those for a capture with \
         highlight correction, which halves the stage's input.",
        "HUE_LEVEL_CONTROL",
        &hue::LEVEL_CONTROL,
    );
    array(
        out,
        "",
        "HUE_LEVEL_CONTROL_HALVED",
        &hue::LEVEL_CONTROL_HALVED,
    );
}

/// The contents of `runtime/abi.h`.
pub fn header() -> String {
    let mut out = String::new();
    comment(
        &mut out,
        "Generated from crates/grkit-core/src/abi.rs. Do not edit: change the \
         tables there, then run GRKIT_UPDATE_GENERATED=1 cargo test -p grkit-core \
         --test generated.",
    );
    out.push_str("#ifndef GRKIT_ABI_H\n#define GRKIT_ABI_H\n\n");

    comment(
        &mut out,
        "The private header: the start of the registry array. ARM code addresses \
         it through the literal GRKIT_HEADER_REFERENCE, which the installer relocates.",
    );
    writeln!(out, "#define GRKIT_HEADER_REFERENCE {HEADER_REFERENCE:#x}").unwrap();
    writeln!(out, "#define GRKIT_HOST_HEADER_SIZE {HOST_HEADER_SIZE}").unwrap();
    comment(
        &mut out,
        "The most module pages, layers, and plane filters that the header holds.",
    );
    writeln!(out, "#define GRKIT_MODULE_PAGES_MAX {MODULE_PAGES_MAX}").unwrap();
    writeln!(out, "#define GRKIT_MODULE_LAYERS_MAX {MODULE_LAYERS_MAX}").unwrap();
    writeln!(out, "#define GRKIT_MODULE_FILTERS_MAX {MODULE_FILTERS_MAX}").unwrap();
    writeln!(
        out,
        "#define GRKIT_MODULE_SETTINGS_MAX {}",
        MODULE_SETTINGS_MAX
    )
    .unwrap();
    comment(
        &mut out,
        "The most modules that GRKit lists, and the words of the largest script \
         array, which holds the module array.",
    );
    writeln!(out, "#define GRKIT_MODULES_MAX {}U", MODULES_MAX).unwrap();
    writeln!(
        out,
        "#define GRKIT_MODULE_ARRAY_WORDS {}U",
        MODULE_ARRAY_WORDS
    )
    .unwrap();
    comment(
        &mut out,
        "The row views of the borrowed native settings page: the ADJ Order page \
         has a row for each listed module. Without a suffix, for the assembler.",
    );
    writeln!(out, "#define GRKIT_SUBMENU_ROWS {}", MODULES_MAX).unwrap();
    comment(
        &mut out,
        "Icon slots: the host icons, then the effect icons, then the module \
         tiles, in chunks of GRKIT_ICONS_PER_CHUNK (GRKIT_HOST_ICON_CHUNKS). \
         GRKIT_ICON_MODULE_TILE(n) is the icon ID of module tile n.",
    );
    writeln!(out, "#define GRKIT_HOST_ICON_COUNT {}", HOST_ICON_COUNT).unwrap();
    writeln!(out, "#define GRKIT_ICONS_PER_CHUNK {}", ICONS_PER_CHUNK).unwrap();
    writeln!(out, "#define GRKIT_MODULE_TILES_MAX {}", MODULE_TILES_MAX).unwrap();
    writeln!(out, "#define GRKIT_ICON_MODULE_TILE(n) (0x7000U+(n))").unwrap();
    // A change of a saved selection: the loader saves after the count changes.
    writeln!(
        out,
        "#define GRKIT_STATE_CHANGED(h) (++(h)[GRKIT_HOST_STATE_CHANGES/4])"
    )
    .unwrap();
    writeln!(out, "#define GRKIT_HOST_MAGIC_VALUE {HOST_MAGIC_VALUE:#x}").unwrap();
    writeln!(out, "#define GRKIT_HOST_VERSION_VALUE {HOST_VERSION_VALUE}").unwrap();
    table(&mut out, HEADER_FIELDS, false);
    out.push('\n');

    comment(
        &mut out,
        "The capabilities of the core (grkit_core::capabilities): the handle of \
         each CORE setting, and for each handle in order, its header word and its \
         number of values. CORE.TONE has neither: its value is the active record, \
         and the effects give its number of values (runtime/tone.h).",
    );
    writeln!(
        out,
        "#define GRKIT_CORE_SETTING_BASE {:#x}U",
        capabilities::HANDLE_BASE
    )
    .unwrap();
    writeln!(
        out,
        "#define GRKIT_CORE_SETTING_COUNT {}U",
        capabilities::CAPABILITIES.len()
    )
    .unwrap();
    for (i, capability) in capabilities::CAPABILITIES.iter().enumerate() {
        writeln!(
            out,
            "#define GRKIT_CORE_{} {:#x}U",
            capability.key,
            capabilities::HANDLE_BASE as usize + i
        )
        .unwrap();
        writeln!(
            out,
            "#define GRKIT_CORE_{}_VALUES {}U",
            capability.key, capability.values
        )
        .unwrap();
        // The bit in GRKIT_HOST_CAPABILITIES, without a suffix for assembly.
        writeln!(
            out,
            "#define GRKIT_CORE_{}_BIT {:#x}",
            capability.key,
            1u32 << i
        )
        .unwrap();
    }
    let list = |values: Vec<String>| values.join(",");
    writeln!(
        out,
        "#define GRKIT_CORE_SETTING_WORDS {}",
        list(
            capabilities::CAPABILITIES
                .iter()
                .map(|c| match c.state {
                    capabilities::State::Word(word) => word.to_string(),
                    capabilities::State::Effect => "0".into(),
                })
                .collect()
        )
    )
    .unwrap();
    writeln!(
        out,
        "#define GRKIT_CORE_SETTING_VALUES {}",
        list(
            capabilities::CAPABILITIES
                .iter()
                .map(|c| c.values.to_string())
                .collect()
        )
    )
    .unwrap();
    out.push('\n');

    comment(
        &mut out,
        "An Image Control record. The records follow the header.",
    );
    writeln!(out, "#define GRKIT_RECORD_SIZE {RECORD_SIZE:#x}").unwrap();
    table(&mut out, RECORD_FIELDS, true);
    define(
        &mut out,
        "The R, G, and B tone curves in the coefficients: 256 14-bit samples \
         each, GRKIT_RECORD_CURVE_SIZE bytes apart.",
        "RECORD_CURVES",
        format!("{RECORD_CURVES:#x}"),
    );
    writeln!(
        out,
        "#define GRKIT_RECORD_CURVE_SIZE {RECORD_CURVE_SIZE:#x}"
    )
    .unwrap();
    grf_table(&mut out);
    hue_tables(&mut out);
    define(
        &mut out,
        "The plan that SCAN writes: this header, the GRF file of each installed \
         effect, then an entry of GRKIT_PLANNED_EFFECT_SIZE bytes for each listed \
         effect.",
        "PLAN_HEADER_SIZE",
        PLAN_HEADER_SIZE,
    );
    writeln!(out, "#define GRKIT_PLAN_MAGIC_VALUE {PLAN_MAGIC_VALUE:#x}").unwrap();
    writeln!(out, "#define GRKIT_PLAN_SAFE {PLAN_SAFE}U").unwrap();
    writeln!(out, "#define GRKIT_PLAN_PROFILE {PLAN_PROFILE}U").unwrap();
    table(&mut out, PLAN_FIELDS, false);
    writeln!(
        out,
        "#define GRKIT_PLANNED_EFFECT_SIZE {PLANNED_EFFECT_SIZE}"
    )
    .unwrap();
    table(&mut out, PLANNED_FIELDS, false);
    define(
        &mut out,
        "The longest menu string, with its NUL: an Image Control summary is a \
         prefix and a 16-character effect name.",
        "SUMMARY_UNITS",
        menu_text::SUMMARY_UNITS,
    );
    // The icon slot differs between camera families, so the family header in
    // runtime/targets defines GRKIT_ICON_STRIDE.
    out.push('\n');

    define(
        &mut out,
        "Picture scratch after the records: the date stamp core and halo masks, \
         then the shot ring.",
        "PICTURE_MASKS",
        PICTURE_MASKS,
    );
    define(
        &mut out,
        "Shot ring: the record of each recent non-live-view parameter build, keyed \
         by its shot settings pointer, which the maker-note setter (+0x18) and the \
         JPEG encode configuration (+0x34) also hold. A later shot can start \
         processing before an earlier one is written, so a single \"last build\" \
         word is not enough. Word 0 counts the writes, and word 1 \
         (GRKIT_SHOT_LAST_TURNS) holds the turns of the last shot with a known posture. \
         Entry n is at GRKIT_SHOT_ENTRY + GRKIT_SHOT_STRIDE n and holds the key at \
         GRKIT_SHOT_KEY, the record at GRKIT_SHOT_RECORD, and the posture of the \
         electronic level at GRKIT_SHOT_POSTURE (runtime/processing.h).",
        "SHOT_RING",
        PICTURE_MASKS,
    );
    for (name, value) in [
        ("ENTRIES", SHOT_ENTRIES),
        ("ENTRY", SHOT_ENTRY),
        ("STRIDE", SHOT_STRIDE),
        ("KEY", SHOT_KEY),
        ("RECORD", SHOT_RECORD),
        ("POSTURE", SHOT_POSTURE),
        ("LAST_TURNS", SHOT_LAST_TURNS),
    ] {
        writeln!(out, "#define GRKIT_SHOT_{name} {value}").unwrap();
    }
    writeln!(out, "#define GRKIT_PICTURE_SCRATCH {PICTURE_SCRATCH}").unwrap();
    out.push('\n');

    define(
        &mut out,
        "Menu text after the picture scratch, in each menu language: 0 English, 1 \
         Simplified Chinese, 2 Traditional Chinese, 3 Japanese, 4 Korean. The installer copies the label \
         table of the build to offset 0: for each language, GRKIT_TEXT_STRINGS u16 \
         byte offsets from the text start, then NUL-terminated UTF-16 strings. At \
         GRKIT_TEXT_SUMMARY, the installer writes the Image Control summary of each \
         record in each language, GRKIT_SUMMARY_UNITS units each, language-major.",
        "TEXT_SIZE",
        menu_text::TEXT_SIZE,
    );
    define(
        &mut out,
        "Live-view labels after the menu text. The installer copies the label \
         table of the build (grkit_core::artwork::overlay_labels): for label n, a \
         u16 width, a u16 height, and the u32 offset of its pixels from the table \
         start at 8 n. Each pixel is one byte of text coverage, row by row.",
        "LABELS_SIZE",
        LABELS_SIZE,
    );
    define(
        &mut out,
        "The profile table after the labels (runtime/profile.h). For each of the \
         first GRKIT_PROFILE_MODULES listed modules, at GRKIT_PROFILE_MODULE_SIZE \
         n: its code start, its code end, and \
         GRKIT_PROFILE_NAME_BYTES ASCII bytes of its name. At GRKIT_PROFILE_SLOTS, \
         GRKIT_PROFILE_SLOT_COUNT slots of GRKIT_PROFILE_SLOT_SIZE bytes: calls, \
         cycles, and the cycles of the largest call. The core points come first, \
         then GRKIT_PROFILE_KINDS slots for each of those modules.",
        "PROFILE",
        PROFILE,
    );
    for (name, value) in [
        ("SIZE", PROFILE_SIZE),
        ("MODULES", PROFILE_MODULES),
        ("MODULE_SIZE", PROFILE_MODULE_SIZE),
        ("NAME_BYTES", PROFILE_NAME_BYTES),
        ("SLOTS", PROFILE_SLOTS),
        ("SLOT_SIZE", PROFILE_SLOT_SIZE),
        ("SLOT_COUNT", PROFILE_SLOT_COUNT),
        ("MODULE_BASE", PROFILE_POINTS.len()),
        ("KINDS", PROFILE_KINDS.len()),
    ] {
        writeln!(out, "#define GRKIT_PROFILE_{name} {value}").unwrap();
    }
    for (slot, (_, name)) in PROFILE_POINTS.iter().enumerate() {
        writeln!(out, "#define GRKIT_PROFILE_{name} {slot}").unwrap();
    }
    for (kind, (_, name)) in PROFILE_KINDS.iter().enumerate() {
        writeln!(out, "#define GRKIT_PROFILE_{name} {kind}").unwrap();
    }
    define(
        &mut out,
        "Everything after the records: the script array must hold it.",
        "TAIL_SIZE",
        TAIL_SIZE,
    );
    writeln!(
        out,
        "#define GRKIT_TEXT_SUMMARY {}",
        menu_text::TEXT_SUMMARY
    )
    .unwrap();
    define(
        &mut out,
        "The parameter page title of a GRKit session, after the summaries: the \
         page, its vtable, and the descriptor of the effect name.",
        "TEXT_PARAMETER_TITLE",
        menu_text::TEXT_PARAMETER_TITLE,
    );
    define(
        &mut out,
        "The native settings page of a GRKit session, before the parameter page \
         title: the borrowed submenu entry, the text descriptors, and the views \
         that show settings (runtime/menu.c).",
        "TEXT_SETTINGS",
        menu_text::TEXT_SETTINGS,
    );
    writeln!(
        out,
        "#define GRKIT_TEXT_LANGUAGES {}",
        crate::language::Language::ALL.len()
    )
    .unwrap();
    writeln!(out, "#define GRKIT_TEXT_RECORDS {}", MAX_COUNT).unwrap();
    out.push_str("#define GRKIT_TEXT(h) ((h)[GRKIT_HOST_TAIL/4]+GRKIT_PICTURE_SCRATCH)\n");
    out.push_str("#define GRKIT_LABELS(h) (GRKIT_TEXT(h)+GRKIT_TEXT_SIZE)\n");
    writeln!(
        out,
        "#define GRKIT_LABEL_COUNT {}",
        crate::artwork::OVERLAY_LABELS.len()
    )
    .unwrap();
    comment(
        &mut out,
        "String IDs of the label table, in grkit_core::menu_text order. A run of \
         IDs follows the rows or values that it names.",
    );
    for (run, id) in menu_text::ids() {
        comment(&mut out, run.doc);
        writeln!(out, "#define TEXT_{} {id}", run.name).unwrap();
    }
    writeln!(out, "#define GRKIT_TEXT_STRINGS {}", menu_text::STRINGS).unwrap();
    out.push_str("\n#endif\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(fields: &[Field], size: usize, cover: bool) {
        let mut next = 0;
        for field in fields {
            assert!(field.size > 0, "{} is empty", field.name);
            if cover {
                assert_eq!(
                    field.offset, next,
                    "{} leaves a gap or overlaps",
                    field.name
                );
            } else {
                assert!(
                    field.offset >= next,
                    "{} overlaps the field before it",
                    field.name
                );
            }
            next = field.offset + field.size;
        }
        assert!(next <= size, "The fields end at {next}, after {size}");
        if cover {
            assert_eq!(next, size, "The fields leave the end of the header unnamed");
        }
    }

    #[test]
    fn every_header_byte_belongs_to_exactly_one_field() {
        assert!(HEADER_FIELDS
            .iter()
            .all(|field| field.offset % 4 == 0 && field.size % 4 == 0));
        check(HEADER_FIELDS, HOST_HEADER_SIZE, true);
    }

    #[test]
    fn record_fields_do_not_overlap() {
        check(RECORD_FIELDS, RECORD_SIZE, false);
    }

    #[test]
    fn the_grf_fields_follow_each_other_in_the_file() {
        check(GRF_FIELDS, effects::SIZE, false);
        assert!(GRF_FIELDS
            .windows(2)
            .all(|pair| pair[0].offset + pair[0].size == pair[1].offset));
        assert_eq!(GRF_FIELDS[0].offset, 0);
    }

    #[test]
    fn the_plan_fields_cover_the_plan_header_and_entry() {
        check(PLAN_FIELDS, PLAN_HEADER_SIZE, true);
        check(PLANNED_FIELDS, PLANNED_EFFECT_SIZE, true);
        // The loader reads the words up to PLAN_MODULE_TILES in this order.
        assert_eq!(
            [
                PLAN_MAGIC,
                PLAN_COUNT,
                PLAN_HEADER_WORDS,
                PLAN_ICON_CHUNKS,
                PLAN_SIZE,
                PLAN_MODULE_WORDS,
                PLAN_MODULE_TILES
            ],
            [0, 4, 8, 12, 16, 20, 24]
        );
    }
}
