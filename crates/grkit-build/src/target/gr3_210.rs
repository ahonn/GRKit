//! GR III 2.10, which GR IIIx 1.60 also runs, and the RTOS builds that move
//! by less than the relocation radius (GR III 2.20, GR IIIx 1.70).

use super::{gr3, Download, ImageControl, Source, Target};
use crate::components::{InstructionPatch, Layout, Site};

pub const TARGET: Target = Target {
    name: "gr3-2.10",
    family: &gr3::FAMILY,
    firmware: "GR III 2.10",
    firmware_sha256: "d6a3c9c288d784080e7b283222ecdd8752adfeed6faa36dab44f73115354bfd4",
    decoded_sha256: "53da690797f35ad6dc2c429d8ca1d0de8e333c211ed959816813982cd093b703",
    download: Download {
        zip: "gr3_v210.zip",
        zip_sha256: "e895b439e4b3b96ac04f50072f02a63061ec7de5dcc1b070890baa09bc659470",
        file: "fwdc239b.bin",
        source: Source::Ricoh,
    },
    identities: &[],
    data_start: 0x55000000,
    data_source: 0x5407b540,
    data_size: 0x3a080,
    array_table: 0x55380f6c,
    array_count: 0x5537fb58,
    heap_min: 0x55804374,
    heap_end: 0x60000000,
    english_slot: 0x55002bb0,
    pixel_base_slot: 0x55018734,
    pixel_base_indirect: false,
    native_tone_page: None,
    layout: &LAYOUT,
    image_control: Some(&IMAGE_CONTROL),
    // The English entry is +4 in the language table, which the text hooks load.
    relocation_bases: &[(0x55002bb0, 0x55002bac)],
};

pub const IMAGE_CONTROL: ImageControl = ImageControl {
    tables: [0x55018d94, 0x55018cf0, 0x55018e4c],
    table_bytes: 104,
    record: 0x5501900c,
    coefficients: 0x53dc2754,
    record_tail: [0x53dc27b4, 0x53dc2938, 0x53dc29dc, 0x53dc2444],
    base_gamma: 0x53dcf7f0,
    settings_domain: 0x553458b0,
    settings_type: 0x77c,
    type_count: 26,
};

/// The tail after the decoded RTOS image (0x540b55c0).
pub const LAYOUT: Layout = Layout {
    define: "-DGRKIT_TARGET_GR3_210",
    tail: (0x540b6000, 0x540c6000),
    contexts: CONTEXTS,
    sites: SITES,
    instruction_patches: INSTRUCTION_PATCHES,
    framing_patches: FRAMING_PATCHES,
    guards: GUARDS,
};

const SITES: &[Site] = &[
    ("module_parameters", 0x536dc124, "abfdffeb"),
    ("module_processing", 0x536dc28c, "940d08e3"),
    ("module_packet", 0x536dc68c, "903084e5"),
    ("module_gamma_flag", 0x536e8694, "9b30d4e5"),
    ("module_gamma_curves", 0x536e87c4, "891900eb"),
    ("menu_native_touch_0", 0x531a48f0, "f431d0e5"),
    ("menu_native_touch_1", 0x531a4904, "f431d0e5"),
    ("menu_native_touch_2", 0x531a4918, "f431d0e5"),
    ("menu_native_touch_3", 0x531a492c, "f431d0e5"),
    ("menu_native_touch_4", 0x531a4940, "f431d0e5"),
    ("menu_destroy", 0x5319ff24, "0dc0a0e1"),
    ("menu_refresh", 0x531a4114, "0dc0a0e1"),
    ("menu_event", 0x531a4db8, "b030d1e1"),
    ("menu_exit", 0x531a3e88, "7022d0e5"),
    ("menu_changed", 0x531a4cfc, "0dc0a0e1"),
    ("menu_text_measure", 0x533966fc, "077193e7"),
    ("menu_text_draw", 0x53397108, "073193e7"),
    ("menu_icon_width", 0x53209fb8, "9e3200e3"),
    ("menu_icon_height", 0x53209fec, "9e3200e3"),
    ("menu_icon_guard", 0x5320a7cc, "9e1200e3"),
    ("menu_icon_descriptor", 0x5320a810, "033192e7"),
    ("menu_native_adj_apply", 0x531a0ba0, "0dc0a0e1"),
    ("menu_native_dialog_apply", 0x531ab998, "0dc0a0e1"),
    ("menu_native_page_event", 0x531ac270, "0dc0a0e1"),
    ("menu_native_page_exit", 0x531ab1b0, "0dc0a0e1"),
    ("menu_viewfinder_icon", 0x531710c4, "97e204eb"),
    ("menu_playback_icon", 0x53179b7c, "daf404eb"),
    ("menu_playback_detail", 0x532b7cfc, "eef1ffeb"),
    ("menu_parameter_start", 0x531b1020, "0dc0a0e1"),
    ("menu_parameter_kinds", 0x531b1094, "d2eeffeb"),
    ("menu_parameter_snapshot_0", 0x531b1060, "c54804eb"),
    ("menu_parameter_snapshot_1", 0x531b10c8, "ab4804eb"),
    ("menu_parameter_event", 0x531b0eac, "b030d1e1"),
    ("menu_parameter_get_0", 0x532c283c, "0dc0a0e1"),
    ("menu_parameter_get_1", 0x532c285c, "0dc0a0e1"),
    ("menu_parameter_get_2", 0x532c287c, "0dc0a0e1"),
    ("menu_parameter_get_3", 0x532c289c, "0dc0a0e1"),
    ("menu_parameter_get_4", 0x532c28bc, "0dc0a0e1"),
    ("menu_parameter_get_5", 0x532c28dc, "0dc0a0e1"),
    ("menu_parameter_get_6", 0x532c28fc, "0dc0a0e1"),
    ("menu_parameter_get_7", 0x532c2a1c, "0dc0a0e1"),
    ("menu_parameter_get_8", 0x532c2a3c, "0dc0a0e1"),
    ("menu_parameter_set_0", 0x532bd784, "0dc0a0e1"),
    ("menu_parameter_set_1", 0x532bdcd8, "0dc0a0e1"),
    ("menu_parameter_set_2", 0x532be22c, "0dc0a0e1"),
    ("menu_parameter_set_3", 0x532be8a0, "0dc0a0e1"),
    ("menu_parameter_set_4", 0x532bef14, "0dc0a0e1"),
    ("menu_parameter_set_5", 0x532bf588, "0dc0a0e1"),
    ("menu_parameter_set_6", 0x532bfbfc, "0dc0a0e1"),
    ("menu_parameter_set_7", 0x532c0da8, "0dc0a0e1"),
    ("menu_parameter_set_8", 0x532c141c, "0dc0a0e1"),
    ("menu_parameter_row_0", 0x532abfc4, "d8770aeb"),
    ("menu_parameter_row_1", 0x532ac0e0, "a2770aeb"),
    ("menu_parameter_row_2", 0x532ac1fc, "6c770aeb"),
    ("menu_parameter_row_3", 0x532ac318, "36770aeb"),
    ("menu_parameter_row_4", 0x532ac434, "00770aeb"),
    ("menu_parameter_row_5", 0x532ac550, "ca760aeb"),
    ("menu_parameter_row_6", 0x532ac66c, "94760aeb"),
    ("menu_parameter_row_7", 0x532acc88, "95750aeb"),
    ("menu_parameter_row_8", 0x532acda4, "5f750aeb"),
    ("menu_settings_change", 0x5316c9d8, "0dc0a0e1"),
    ("menu_info_update", 0x531e0abc, "0dc0a0e1"),
    ("menu_check_factory", 0x5316f3b8, "3b3100e3"),
    ("menu_check_lines", 0x531ba5cc, "0f6807eb"),
    ("menu_check_dot", 0x531be144, "7b8107eb"),
    ("menu_settings_kinds_r3_0", 0x531c5d20, "b33345e3"),
    ("menu_settings_kinds_r2", 0x531c5d88, "b32345e3"),
    ("menu_settings_kinds_r8", 0x531c60f4, "b38345e3"),
    ("menu_settings_kinds_r5", 0x531c631c, "b35345e3"),
    ("menu_settings_kinds_r3_1", 0x531c63f0, "b33345e3"),
    ("menu_settings_build", 0x531c60b0, "1cc04be5"),
    ("menu_settings_hide_first", 0x531c6144, "040057e3"),
    ("menu_settings_hide_next", 0x531c6168, "040054e3"),
    ("menu_settings_row_kind", 0x531c6260, "031198e7"),
    ("menu_settings_label", 0x531c6244, "b810d3e1"),
    ("menu_settings_open", 0x531c5d9c, "145092e5"),
    ("menu_settings_count_0", 0x531b3c84, "0600a0e3"),
    ("menu_settings_draw_0", 0x531b5b34, "0dc0a0e1"),
    ("menu_settings_apply_0", 0x531b7514, "0dc0a0e1"),
    ("menu_settings_current_0", 0x531b8ce4, "0dc0a0e1"),
    ("menu_settings_count_1", 0x531b3c8c, "0200a0e3"),
    ("menu_settings_draw_1", 0x531b5b74, "0dc0a0e1"),
    ("menu_settings_apply_1", 0x531b7544, "0dc0a0e1"),
    ("menu_settings_current_1", 0x531b8d18, "0dc0a0e1"),
    ("menu_settings_count_2", 0x531b3c94, "0200a0e3"),
    ("menu_settings_draw_2", 0x531b5bb4, "0dc0a0e1"),
    ("menu_settings_apply_2", 0x531b7574, "0dc0a0e1"),
    ("menu_settings_current_2", 0x531b8d4c, "0dc0a0e1"),
    ("menu_settings_count_3", 0x531b3ca4, "0200a0e3"),
    ("menu_settings_draw_3", 0x531b5c34, "0dc0a0e1"),
    ("menu_settings_apply_3", 0x531b75d4, "0dc0a0e1"),
    ("menu_settings_current_3", 0x531b8db4, "0dc0a0e1"),
    ("menu_settings_count_4", 0x531b3cac, "0200a0e3"),
    ("menu_settings_draw_4", 0x531b5c74, "0dc0a0e1"),
    ("menu_settings_apply_4", 0x531b7604, "0dc0a0e1"),
    ("menu_settings_current_4", 0x531b8de8, "0dc0a0e1"),
    // SoftwareKeyboardController: the entry of its text length method, then
    // the tail calls of its copyright text load and store to the settings
    // getter (0x5324642c) and setter (0x5326cc14).
    ("menu_keyboard_length", 0x531daf3c, "0dc0a0e1"),
    ("menu_keyboard_load", 0x531dbb10, "45aa01ea"),
    ("menu_keyboard_store", 0x531dbd40, "b34302ea"),
    ("framing_update", 0x5317681c, "0dc0a0e1"),
    ("framing_exit", 0x53176de4, "0dc0a0e1"),
    ("framing_destroy", 0x53170b50, "0dc0a0e1"),
    ("framing_canvas", 0x53391b14, "0dc0a0e1"),
    ("framing_children", 0x53391be0, "500800eb"),
    ("framing_capture_request_hook", 0x535423f4, "0010d4e5"),
    ("framing_height_0", 0x536a2e48, "d33ea0e3"),
    ("framing_height_1", 0x536a2e34, "a93ea0e3"),
    ("framing_height_2", 0x536a2e20, "763ea0e3"),
    ("framing_height_3", 0x536a2e08, "383400e3"),
    ("framing_screennail_inset", 0x536a2f98, "2610a0e3"),
    ("framing_thumbnail_inset", 0x536a3050, "1010a0e3"),
    ("framing_width_0", 0x536a2d90, "d02401e3"),
    ("framing_width_1", 0x536a2d54, "b02001e3"),
    ("framing_width_2", 0x536a2d7c, "bb2ea0e3"),
    ("framing_width_3", 0x536a2d68, "1e2da0e3"),
    ("framing_narrow_height_3", 0x536a2d6c, "5a3ea0e3"),
    ("framing_narrow_screennail", 0x536a2fac, "2800a0e3"),
    ("framing_narrow_thumbnail", 0x536a306c, "b010c3e1"),
    ("framing_classify", 0x532b39b8, "0dfcffeb"),
    ("framing_image_aspect", 0x532b3cac, "50fbffea"),
    ("module_gamma_dirty", 0x536de1a8, "0e30d4e5"),
    ("picture_post", 0x536e4774, "e9ffffeb"),
    ("picture_encode_entry", 0x535047d4, "0dc0a0e1"),
    // PentaxMakernoteEntryDataSetter: the ImageTone call of tag 0x4f, then
    // the BL to the adjustments fill of tag 0x247 (handler 0x536078e8).
    ("picture_capture_tone", 0x53608080, "e1fdfeeb"),
    ("picture_capture_parameters", 0x536078f4, "3302ffeb"),
    ("framing_level_threshold", 0x536ce194, "fe5ca0e3"),
    ("framing_level_mode", 0x536ce080, "0610a0e1"),
    // LiveViewController histogram update: the BL to the 256-to-128 bin copy.
    ("framing_histogram", 0x53170c28, "c5ed0feb"),
    // DigitalFilterProcess SetEffectParameter (0x5350304c), and its stage
    // gate (0x535131c0, plan +0x24).
    ("grain_particle", 0x535031b0, "9537d5e5"),
    ("grain_gate", 0x5351361c, "e7feffeb"),
    // The granulation (0x5367ad3c): the BL to its noise fill (0x5368a92c).
    ("grain_fill", 0x5367ae14, "c43e00eb"),
    // The granulation (0x5367ad3c): the first test of its amplitude choice.
    ("grain_amplitude", 0x5367ae28, "020055e3"),
];

const CONTEXTS: &[(u32, &str)] = &[
    (0x536dc124, "abfdffeb"),
    // The instructions that module_processing replaces. Its stock path in
    // runtime/processing.S does the same with literal words for the addresses.
    (
        0x536dc28c,
        "940d08e3f0cc08e3010545e301c545e37b9f86e201e190e74c0e08e3010545e3",
    ),
    (0x536dc688, "943196e5903084e58b4e00eb"),
    (0x536de1a8, "0e30d4e5"),
    (0x536e8694, "9b30d4e5"),
    (0x536e87c4, "891900eb"),
    // Gamma stage frame, the end of its custom-curve path (table-build flag and
    // GAMMA_BUILT), and its epilogue, which the curve cache relies on.
    (
        0x536e8660,
        "0dc0a0e17c3705e3f0df2de9053445e304b04ce22cd04de20140a0e14c300be5003093e548100be530300be544000be5",
    ),
    (0x536e89cc, "44301be5034a83e20130a0e30a32c4e50030a0e3303284e5"),
    (0x536e8780, "28d04be2f0af9de8"),
    // LiveViewController's Image Control widget: load native type, map, assign.
    (
        0x531710bc,
        "e83494e5001ad3e597e204eb0040a0e10500a0e10410a0e131a608eb",
    ),
];

const INSTRUCTION_PATCHES: &[InstructionPatch] = &[
    // ImageTone extraction stores 0x10000 | (tag - 0x100) instead of 0 for
    // non-native tags, so playback can match a GRKit code. Native getters still
    // accept only 1..12.
    (0x535c251c, "0020a083", "01288083"),
];

/// Data and single-instruction changes that let capture use the hidden GR IV
/// aspect ids 1 (4:3) and 2 (16:9) while the live view stays 3:2. GR III
/// implements only live-view modes 0 (3:2) and 3 (1:1).
const FRAMING_PATCHES: &[InstructionPatch] = &[
    // Live-view mode table for capture ids 1, 2, 3 (read at 0x53669424): 0, 0, 3.
    (0x53d553c8, "02010300", "00000300"),
    // Second mode lookup at 0x53437170: `bhi` -> `bne`, so only id 3 reads its
    // table. Without it the live view times out after a 4:3 or 16:9 capture.
    (0x534371d8, "1500008a", "1500001a"),
    // Pipeline row 0x536a3200: return row 0 for ids 1 and 2 without the
    // ModelDefs.cpp:115 assertion, as the stock fallback does.
    (0x536a3228, "f83b0ae3", "080000ea"),
    // AF and touch image rectangle 0x535ee890: ids 1 and 2 use the 3:2
    // rectangle. Stock writes zeros for id 2 and leaves id 1 stale.
    (0x535ee8b4, "0c00000a", "0300000a"),
    (0x535ee8c0, "000052e3", "020052e1"),
];

/// The startup literals for the `.data` copy: source, destination, and size.
/// Source plus size is the end of the RTOS image.
const GUARDS: &[(u32, &str)] = &[(0x53000450, "40b507540000005580a00300")];
