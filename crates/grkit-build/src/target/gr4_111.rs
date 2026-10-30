//! GR IV and GR IV HDF 1.11, which GR IV Monochrome also runs. The runtime
//! has the private menu, Image Control, Framing (XPan only), Guides, Date
//! Stamp, and Grain.

use super::{gr4, Download, ImageControl, Source, Target};
use crate::components::{InstructionPatch, Layout, Site};

pub const TARGET: Target = Target {
    name: "gr4-1.11",
    family: &gr4::FAMILY,
    firmware: "GR IV 1.11",
    firmware_sha256: "a2f664dfca034059eb0fd6e18ab08684c326b4a034d85c164dad7e1ec9b5655f",
    decoded_sha256: "c4e597c7c9ca1bc181b30e35139ed90a4fd876ddcca14f12be0e00b6702233ae",
    download: Download {
        zip: "gr4_v111.zip",
        zip_sha256: "ded47aa6385886e4b1f8f41f8551866ab0f1ecd0aef89934a43ef0e456539e8f",
        file: "fwdc248b.bin",
        source: Source::Ricoh,
    },
    // Build 7 (2025-12-01, the Ricoh download) and build 8 (2026-02-18).
    identities: &[0x070a0b01, 0x080a0b01],
    data_start: 0x55000000,
    data_source: 0x5437b640,
    data_size: 0x57480,
    array_table: 0x553b3ec4,
    array_count: 0x553b2ab0,
    heap_min: 0x558372d4,
    heap_end: 0x60000000,
    english_slot: 0x5500211c,
    // The drawer of the icon manager singleton (0x55082028 + 0x10). GR IV draws
    // icons from a copy of the icon pixels, not from the ICONBIN load address.
    pixel_base_slot: 0x55082038,
    pixel_base_indirect: true,
    native_tone_page: Some(0x81),
    layout: &LAYOUT,
    image_control: Some(&IMAGE_CONTROL),
    // The settings domain follows the vtable of the capture property
    // singleton, and the icon drawer is +0x10 in the icon manager singleton.
    relocation_bases: &[(0x55379610, 0x5537960c), (0x55082038, 0x55082028)],
};

/// GR IV tables hold 0x22 entries for each of three model variants, which are
/// equal in 1.11. GR IV computes the base gamma with the GR III function from
/// the same curves, so its base gamma is the GR III one.
pub const IMAGE_CONTROL: ImageControl = ImageControl {
    tables: [0x55053780, 0x55053504, 0x55052cf4],
    table_bytes: 408,
    record: 0x5501adbc,
    coefficients: 0x54049850,
    record_tail: [0x540498d8, 0x54049a70, 0x54049b28, 0x54049518],
    base_gamma: 0x5407f4b4,
    settings_domain: 0x55379610,
    settings_type: 0x79c,
    type_count: 31,
};

/// The tail after the decoded RTOS image (0x543d2ac0).
pub const LAYOUT: Layout = Layout {
    define: "-DGRKIT_TARGET_GR4_111",
    tail: (0x543d3000, 0x543e3000),
    contexts: CONTEXTS,
    sites: SITES,
    instruction_patches: INSTRUCTION_PATCHES,
    framing_patches: FRAMING_PATCHES,
    guards: GUARDS,
};

const SITES: &[Site] = &[
    ("module_parameters", 0x53912c70, "d8f9ffeb"),
    ("module_processing", 0x53912d40, "0a10a0e1"),
    ("module_record", 0x53912d4c, "080084e5"),
    ("module_multiaxial", 0x53912d5c, "0c0084e5"),
    ("module_packet", 0x53913188, "903084e5"),
    ("module_gamma_flag", 0x5391e360, "9b10d1e5"),
    ("module_gamma_curves", 0x5391e76c, "231a00eb"),
    ("menu_native_touch_0", 0x531bf9d8, "fc31d0e5"),
    ("menu_native_touch_1", 0x531bf9ec, "fc31d0e5"),
    ("menu_native_touch_2", 0x531bfa00, "fc31d0e5"),
    ("menu_native_touch_3", 0x531bfa14, "fc31d0e5"),
    ("menu_native_touch_4", 0x531bfa28, "fc31d0e5"),
    ("menu_destroy", 0x531ba1c0, "0dc0a0e1"),
    ("menu_refresh", 0x531bf3c8, "0dc0a0e1"),
    ("menu_event", 0x531bf834, "b030d1e1"),
    ("menu_exit", 0x531bf580, "fc11d0e5"),
    ("menu_changed", 0x531bf38c, "0dc0a0e1"),
    ("menu_text_measure", 0x53572b00, "077193e7"),
    ("menu_text_draw", 0x5357350c, "073193e7"),
    ("menu_native_adj_apply", 0x531badd4, "0dc0a0e1"),
    ("menu_native_dialog_apply", 0x531c5894, "0dc0a0e1"),
    ("menu_native_page_event", 0x531c635c, "0dc0a0e1"),
    ("menu_native_page_exit", 0x531c4fb8, "0dc0a0e1"),
    ("menu_icon_lookup", 0x5323db68, "0dc0a0e1"),
    ("menu_icon_limit", 0x5323e434, "020057e1"),
    ("menu_icon_entry", 0x5323e4d8, "073190e7"),
    ("menu_viewfinder_icon", 0x53187f90, "2e3d08eb"),
    ("menu_playback_icon", 0x531916c0, "5a5408eb"),
    ("menu_playback_detail", 0x533a7614, "7af0ffeb"),
    ("menu_parameter_start", 0x531cb6cc, "0dc0a0e1"),
    ("menu_parameter_kinds", 0x531cb744, "51edffeb"),
    ("menu_parameter_snapshot_0", 0x531cb710, "86cf07eb"),
    ("menu_parameter_snapshot_1", 0x531cb77c, "6bcf07eb"),
    ("menu_parameter_event", 0x531cb55c, "b030d1e1"),
    ("menu_parameter_get_0", 0x533b7110, "0dc0a0e1"),
    ("menu_parameter_get_1", 0x533b77f8, "0dc0a0e1"),
    ("menu_parameter_get_2", 0x533b7ee0, "0dc0a0e1"),
    ("menu_parameter_get_3", 0x533b8908, "0dc0a0e1"),
    ("menu_parameter_get_4", 0x533b9330, "0dc0a0e1"),
    ("menu_parameter_get_5", 0x533b9d58, "0dc0a0e1"),
    ("menu_parameter_get_6", 0x533ba780, "0dc0a0e1"),
    ("menu_parameter_get_7", 0x533bcd98, "0dc0a0e1"),
    ("menu_parameter_get_8", 0x533bd7c0, "0dc0a0e1"),
    ("menu_parameter_set_0", 0x533b670c, "0dc0a0e1"),
    ("menu_parameter_set_1", 0x533b675c, "0dc0a0e1"),
    ("menu_parameter_set_2", 0x533b67ac, "0dc0a0e1"),
    ("menu_parameter_set_3", 0x533b67fc, "0dc0a0e1"),
    ("menu_parameter_set_4", 0x533b684c, "0dc0a0e1"),
    ("menu_parameter_set_5", 0x533b689c, "0dc0a0e1"),
    ("menu_parameter_set_6", 0x533b68ec, "0dc0a0e1"),
    ("menu_parameter_set_7", 0x533b6b6c, "0dc0a0e1"),
    ("menu_parameter_set_8", 0x533b6bbc, "0dc0a0e1"),
    ("menu_parameter_row_0", 0x5339a6a8, "11f50eeb"),
    ("menu_parameter_row_1", 0x5339a7c4, "dbf40eeb"),
    ("menu_parameter_row_2", 0x5339a8e0, "a5f40eeb"),
    ("menu_parameter_row_3", 0x5339a9fc, "6ff40eeb"),
    ("menu_parameter_row_4", 0x5339ab18, "39f40eeb"),
    ("menu_parameter_row_5", 0x5339ac34, "03f40eeb"),
    ("menu_parameter_row_6", 0x5339ad50, "cdf30eeb"),
    ("menu_parameter_row_7", 0x5339b450, "a6f20eeb"),
    ("menu_parameter_row_8", 0x5339b56c, "70f20eeb"),
    ("menu_settings_change", 0x53182e4c, "0dc0a0e1"),
    ("menu_info_update", 0x531fc888, "0dc0a0e1"),
    ("menu_check_factory", 0x53185f7c, "2e3100e3"),
    ("menu_check_lines", 0x531d6e04, "d0660eeb"),
    ("menu_check_dot", 0x531daafc, "8f800eeb"),
    ("menu_settings_kinds_r3_0", 0x531e206c, "da3345e3"),
    ("menu_settings_kinds_r2", 0x531e20d4, "da2345e3"),
    ("menu_settings_kinds_r8", 0x531e2444, "da8345e3"),
    ("menu_settings_kinds_r5", 0x531e266c, "da5345e3"),
    ("menu_settings_kinds_r3_1", 0x531e2740, "da3345e3"),
    ("menu_settings_build", 0x531e2400, "1cc04be5"),
    ("menu_settings_hide_first", 0x531e2494, "040057e3"),
    ("menu_settings_hide_next", 0x531e24b8, "040054e3"),
    ("menu_settings_row_kind", 0x531e25b0, "031198e7"),
    ("menu_settings_label", 0x531e2594, "b810d3e1"),
    ("menu_settings_open", 0x531e20e8, "145092e5"),
    ("menu_settings_count_0", 0x531cf4d4, "0600a0e3"),
    ("menu_settings_draw_0", 0x531d1cb0, "0dc0a0e1"),
    ("menu_settings_apply_0", 0x531d39ac, "0dc0a0e1"),
    ("menu_settings_current_0", 0x531d5364, "0dc0a0e1"),
    ("menu_settings_count_1", 0x531cf4dc, "0200a0e3"),
    ("menu_settings_draw_1", 0x531d1cf4, "0dc0a0e1"),
    ("menu_settings_apply_1", 0x531d39dc, "0dc0a0e1"),
    ("menu_settings_current_1", 0x531d5398, "0dc0a0e1"),
    ("menu_settings_count_2", 0x531cf4e4, "0200a0e3"),
    ("menu_settings_draw_2", 0x531d1d38, "0dc0a0e1"),
    ("menu_settings_apply_2", 0x531d3a0c, "0dc0a0e1"),
    ("menu_settings_current_2", 0x531d53cc, "0dc0a0e1"),
    ("menu_settings_count_3", 0x531cf4ec, "0400a0e3"),
    ("menu_settings_draw_3", 0x531d1d7c, "0dc0a0e1"),
    ("menu_settings_apply_3", 0x531d3a3c, "0dc0a0e1"),
    ("menu_settings_current_3", 0x531d5400, "0dc0a0e1"),
    ("menu_settings_count_4", 0x531cf4f4, "0200a0e3"),
    ("menu_settings_draw_4", 0x531d1dc0, "0dc0a0e1"),
    ("menu_settings_apply_4", 0x531d3a6c, "0dc0a0e1"),
    ("menu_settings_current_4", 0x531d5434, "0dc0a0e1"),
    ("framing_update", 0x5318e350, "0dc0a0e1"),
    ("framing_exit", 0x5318e984, "0dc0a0e1"),
    ("framing_destroy", 0x531879e0, "0dc0a0e1"),
    ("framing_canvas", 0x5356dde4, "0dc0a0e1"),
    ("framing_children", 0x5356deb0, "6b0800eb"),
    ("framing_capture_request_hook", 0x5374f2ac, "0010d4e5"),
    // The live-view mode function (0x5388c7b8) and the mode check (0x5363fa84).
    ("framing_live_mode", 0x5388c7d0, "0d2ffbeb"),
    ("framing_live_check", 0x5363fa9c, "5a6204eb"),
    ("framing_height_0", 0x538cfff8, "983d00e3"),
    ("framing_height_1", 0x538cffe4, "ae3ea0e3"),
    ("framing_height_2", 0x538cffcc, "7b3ea0e3"),
    ("framing_height_3", 0x538cffb8, "383400e3"),
    ("framing_screennail_inset", 0x538d00fc, "2610a0e3"),
    ("framing_thumbnail_inset", 0x538d01b4, "1010a0e3"),
    ("framing_width_0", 0x538cff28, "562da0e3"),
    ("framing_width_1", 0x538cff14, "302101e3"),
    ("framing_width_2", 0x538cff00, "c32ea0e3"),
    ("framing_width_3", 0x538cfeec, "1e2da0e3"),
    ("framing_narrow_height_3", 0x538cfef0, "5a3ea0e3"),
    ("framing_narrow_screennail", 0x538d0110, "2800a0e3"),
    ("framing_narrow_thumbnail", 0x538d01d0, "b010c3e1"),
    ("framing_classify", 0x533a2850, "6dfcffeb"),
    ("framing_image_aspect", 0x533a2c58, "6bfbffea"),
    ("module_gamma_dirty", 0x5391410c, "0e30d4e5"),
    ("picture_post", 0x5391a6a4, "e9ffffeb"),
    ("picture_encode_entry", 0x537034b4, "0dc0a0e1"),
    // The ImageTone call of the maker-note setter, then the adjustments fill
    // of tag 0x247 (handler 0x53810074).
    ("picture_capture_tone", 0x53810804, "1a37ffeb"),
    ("picture_capture_parameters", 0x53810080, "6a3bffeb"),
    ("framing_level_threshold", 0x539033f8, "fe5ca0e3"),
    ("framing_level_mode", 0x539032e4, "0610a0e1"),
    // LiveViewController histogram update: the BL to the 256-to-128 bin copy.
    ("framing_histogram", 0x53187a94, "06cd17eb"),
    // DigitalFilterProcess SetEffectParameter (0x53701c40), and its stage
    // gate (0x537150fc, plan +0x25).
    ("grain_enable", 0x53701e14, "b537d4e5"),
    ("grain_strength", 0x53701e34, "b737d4e5"),
    ("grain_size", 0x53701e58, "b637d4e5"),
    ("grain_gate", 0x5371557c, "defeffeb"),
    // The grain tables: `mov r0, #0x200` after the luma amplitude load.
    ("grain_amplitude", 0x5389e3c8, "020ca0e3"),
];

/// The stock instructions around the processing hooks.
const CONTEXTS: &[(u32, &str)] = &[
    // Normalizer BL, then the type-table getters, the record store, and the
    // MultiAxial set store.
    (0x53912c70, "d8f9ffeb"),
    (
        0x53912d3c,
        "34a01be50a10a0e10600a0e18d4c00eb080084e50a10a0e10600a0e1944c00eb0c0084e5",
    ),
    // Packet: the last field store before the native tail.
    (0x53913184, "943195e5903084e5255000eb"),
    // Gamma stage: the custom-curve flag and the curve getter BL.
    (0x5391e360, "9b10d1e5"),
    (0x5391e76c, "231a00eb"),
    // Gamma stage frame, the end of its custom-curve path (table-build flag and
    // GAMMA_BUILT), and its epilogue, which the curve cache relies on.
    (
        0x5391e340,
        "0dc0a0e1bc3904e3f0df2de904b04ce20120a0e1353445e334d04de254100be5",
    ),
    (0x5391e364, "58300be5003093e5000051e3142092e550000be5"),
    (
        0x5391e730,
        "50301be5034a83e20130a0e30a38c4e50030a0e32c3884e5",
    ),
    (0x5391e628, "28d04be2f0af9de8"),
    // LiveViewController's Image Control widget: load native type, map, assign.
    (0x53187f88, "ec3494e5401ad3e52e3d08eb"),
];

const INSTRUCTION_PATCHES: &[InstructionPatch] = &[
    // ImageTone extraction compares each native tag value. Its default case
    // stores 0x10000 | (tag - 0x100), as on GR III, instead of 0.
    (0x537d8a54, "0020a0e3", "ff2c80e2"),
];

const FRAMING_PATCHES: &[InstructionPatch] = &[];

/// The startup `.data` literals, as on GR III, and the icon catalog getter
/// (`movw r0` / `movt r0`, 0x55013410), which patched firmware moves into the
/// RTOS tail when it adds icons.
const GUARDS: &[(u32, &str)] = &[
    (0x53000450, "40b637540000005580740500"),
    (0x533ef604, "100403e3010545e3"),
];
