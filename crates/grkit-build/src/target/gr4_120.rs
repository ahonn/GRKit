//! GR IV and GR IV HDF 1.20, with the hooks of 1.11. Its addresses come
//! from 1.11 through the code that uses them; not tested on a camera.

use super::{gr4, Download, ImageControl, Source, Target};
use crate::components::{InstructionPatch, Layout, Site};

pub const TARGET: Target = Target {
    name: "gr4-1.20",
    family: &gr4::FAMILY,
    firmware: "GR IV 1.20",
    firmware_sha256: "7563d7e462f0c53b6ab2fbab7cdfbbb57151a41192ee82f4f3a2108f9d77650d",
    decoded_sha256: "f5cf55a48f65d55f4ebbd40bcd1d64d1ceca3c88432a2a4ceaa93fe1cad5fc3a",
    download: Download {
        zip: "gr4_v120.zip",
        zip_sha256: "dcc1b6fb004f0d855bb46b30a529da894c7b717501276fec2d198b3d7eb76a59",
        file: "fwdc248b.bin",
        source: Source::Ricoh,
    },
    identities: &[0x0b0a1401],
    data_start: 0x55000000,
    data_source: 0x543b3100,
    data_size: 0x58500,
    array_table: 0x553b5bcc,
    array_count: 0x553b47b8,
    heap_min: 0x55839014,
    heap_end: 0x60000000,
    english_slot: 0x5500211c,
    // The drawer of the icon manager singleton (0x55083028 + 0x10).
    pixel_base_slot: 0x55083038,
    pixel_base_indirect: true,
    native_tone_page: Some(0x81),
    layout: &LAYOUT,
    image_control: Some(&IMAGE_CONTROL),
    // As on 1.11: the settings domain follows the vtable of the capture
    // property singleton, and the icon drawer is +0x10 in the icon manager.
    relocation_bases: &[(0x5537b2a0, 0x5537b29c), (0x55083038, 0x55083028)],
};

/// GR IV 1.20 tables hold 0x26 entries for each model variant, and 1.20 adds
/// the monochrome types 31 and 32, so the settings hold types up to 32.
pub const IMAGE_CONTROL: ImageControl = ImageControl {
    tables: [0x55054800, 0x55054554, 0x55053ce4],
    table_bytes: 456,
    record: 0x5501bd58,
    coefficients: 0x5408051c,
    record_tail: [0x540805a4, 0x5408073c, 0x540807f4, 0x540801d0],
    base_gamma: 0x540b57d0,
    settings_domain: 0x5537b2a0,
    settings_type: 0x79c,
    type_count: 33,
};

/// The tail after the decoded RTOS image (0x5440b600), with the regions of
/// 1.11.
pub const LAYOUT: Layout = Layout {
    define: "-DGRKIT_TARGET_GR4_120",
    tail: (0x5440c000, 0x54419000),
    contexts: CONTEXTS,
    sites: SITES,
    instruction_patches: INSTRUCTION_PATCHES,
    framing_patches: FRAMING_PATCHES,
    guards: GUARDS,
};

const SITES: &[Site] = &[
    ("module_parameters", 0x5393aa10, "c4f9ffeb"),
    ("module_processing", 0x5393af68, "0a10a0e1"),
    ("module_record", 0x5393af74, "080084e5"),
    ("module_multiaxial", 0x5393af84, "0c0084e5"),
    ("module_packet", 0x5393ae74, "903084e5"),
    ("module_gamma_flag", 0x53946150, "9b10d1e5"),
    ("module_gamma_curves", 0x5394655c, "ad1a00eb"),
    ("menu_native_touch_0", 0x531c0f10, "fc31d0e5"),
    ("menu_native_touch_1", 0x531c0f24, "fc31d0e5"),
    ("menu_native_touch_2", 0x531c0f38, "fc31d0e5"),
    ("menu_native_touch_3", 0x531c0f4c, "fc31d0e5"),
    ("menu_native_touch_4", 0x531c0f60, "fc31d0e5"),
    ("menu_destroy", 0x531bb6f8, "0dc0a0e1"),
    ("menu_refresh", 0x531c0900, "0dc0a0e1"),
    ("menu_event", 0x531c0d6c, "b030d1e1"),
    ("menu_exit", 0x531c0ab8, "fc11d0e5"),
    ("menu_changed", 0x531c08c4, "0dc0a0e1"),
    ("menu_text_measure", 0x53595dc8, "077193e7"),
    ("menu_text_draw", 0x535967d8, "073193e7"),
    ("menu_native_adj_apply", 0x531bc30c, "0dc0a0e1"),
    ("menu_native_dialog_apply", 0x531c6dcc, "0dc0a0e1"),
    ("menu_native_page_event", 0x531c7894, "0dc0a0e1"),
    ("menu_native_page_exit", 0x531c64f0, "0dc0a0e1"),
    ("menu_icon_lookup", 0x5323f7a0, "0dc0a0e1"),
    // 1.20 compares the icon ID with an immediate limit (NATIVE_ICON_LIMIT).
    ("menu_icon_limit", 0x53240064, "b10f57e3"),
    ("menu_icon_entry", 0x53240104, "073190e7"),
    ("menu_viewfinder_icon", 0x531891e0, "477608eb"),
    ("menu_playback_icon", 0x53192918, "f38d08eb"),
    ("menu_playback_detail", 0x533b6fdc, "17f0ffeb"),
    ("menu_parameter_start", 0x531ccea8, "0dc0a0e1"),
    ("menu_parameter_kinds", 0x531ccf20, "c5ecffeb"),
    ("menu_parameter_snapshot_0", 0x531cceec, "170d08eb"),
    ("menu_parameter_snapshot_1", 0x531ccf58, "fc0c08eb"),
    ("menu_parameter_event", 0x531ccd38, "b030d1e1"),
    ("menu_parameter_get_0", 0x533c77a0, "0dc0a0e1"),
    ("menu_parameter_get_1", 0x533c7e88, "0dc0a0e1"),
    ("menu_parameter_get_2", 0x533c8570, "0dc0a0e1"),
    ("menu_parameter_get_3", 0x533c9088, "0dc0a0e1"),
    ("menu_parameter_get_4", 0x533c9b00, "0dc0a0e1"),
    ("menu_parameter_get_5", 0x533ca578, "0dc0a0e1"),
    ("menu_parameter_get_6", 0x533caff0, "0dc0a0e1"),
    ("menu_parameter_get_7", 0x533cd7e8, "0dc0a0e1"),
    ("menu_parameter_get_8", 0x533ce260, "0dc0a0e1"),
    ("menu_parameter_set_0", 0x533c6d4c, "0dc0a0e1"),
    ("menu_parameter_set_1", 0x533c6d9c, "0dc0a0e1"),
    ("menu_parameter_set_2", 0x533c6dec, "0dc0a0e1"),
    ("menu_parameter_set_3", 0x533c6e3c, "0dc0a0e1"),
    ("menu_parameter_set_4", 0x533c6e8c, "0dc0a0e1"),
    ("menu_parameter_set_5", 0x533c6edc, "0dc0a0e1"),
    ("menu_parameter_set_6", 0x533c6f2c, "0dc0a0e1"),
    ("menu_parameter_set_7", 0x533c71ac, "0dc0a0e1"),
    ("menu_parameter_set_8", 0x533c71fc, "0dc0a0e1"),
    ("menu_parameter_row_0", 0x533a9d60, "a74c0feb"),
    ("menu_parameter_row_1", 0x533a9e7c, "714c0feb"),
    ("menu_parameter_row_2", 0x533a9f98, "3b4c0feb"),
    ("menu_parameter_row_3", 0x533aa0b4, "054c0feb"),
    ("menu_parameter_row_4", 0x533aa1d0, "cf4b0feb"),
    ("menu_parameter_row_5", 0x533aa2ec, "994b0feb"),
    ("menu_parameter_row_6", 0x533aa408, "634b0feb"),
    ("menu_parameter_row_7", 0x533aab08, "3c4a0feb"),
    ("menu_parameter_row_8", 0x533aac24, "064a0feb"),
    ("menu_settings_change", 0x5318408c, "0dc0a0e1"),
    ("menu_info_update", 0x531fe318, "0dc0a0e1"),
    ("menu_check_factory", 0x531871bc, "2e3100e3"),
    ("menu_check_lines", 0x531d87f8, "05ed0eeb"),
    ("menu_check_dot", 0x531dc500, "c1060feb"),
    ("menu_settings_kinds_r3_0", 0x531e3ab8, "dd3345e3"),
    ("menu_settings_kinds_r2", 0x531e3b20, "dd2345e3"),
    ("menu_settings_kinds_r8", 0x531e3e90, "dd8345e3"),
    ("menu_settings_kinds_r5", 0x531e40b8, "dd5345e3"),
    ("menu_settings_kinds_r3_1", 0x531e418c, "dd3345e3"),
    ("menu_settings_build", 0x531e3e4c, "1cc04be5"),
    ("menu_settings_hide_first", 0x531e3ee0, "040057e3"),
    ("menu_settings_hide_next", 0x531e3f04, "040054e3"),
    ("menu_settings_row_kind", 0x531e3ffc, "031198e7"),
    ("menu_settings_label", 0x531e3fe0, "b810d3e1"),
    ("menu_settings_open", 0x531e3b34, "145092e5"),
    ("menu_settings_count_0", 0x531d0cb8, "0600a0e3"),
    ("menu_settings_draw_0", 0x531d3578, "0dc0a0e1"),
    ("menu_settings_apply_0", 0x531d52e8, "0dc0a0e1"),
    ("menu_settings_current_0", 0x531d6d04, "0dc0a0e1"),
    ("menu_settings_count_1", 0x531d0cc0, "0200a0e3"),
    ("menu_settings_draw_1", 0x531d35bc, "0dc0a0e1"),
    ("menu_settings_apply_1", 0x531d5318, "0dc0a0e1"),
    ("menu_settings_current_1", 0x531d6d38, "0dc0a0e1"),
    ("menu_settings_count_2", 0x531d0cc8, "0200a0e3"),
    ("menu_settings_draw_2", 0x531d3600, "0dc0a0e1"),
    ("menu_settings_apply_2", 0x531d5348, "0dc0a0e1"),
    ("menu_settings_current_2", 0x531d6d6c, "0dc0a0e1"),
    ("menu_settings_count_3", 0x531d0cd0, "0400a0e3"),
    ("menu_settings_draw_3", 0x531d3644, "0dc0a0e1"),
    ("menu_settings_apply_3", 0x531d5378, "0dc0a0e1"),
    ("menu_settings_current_3", 0x531d6da0, "0dc0a0e1"),
    ("menu_settings_count_4", 0x531d0cd8, "0200a0e3"),
    ("menu_settings_draw_4", 0x531d3688, "0dc0a0e1"),
    ("menu_settings_apply_4", 0x531d53a8, "0dc0a0e1"),
    ("menu_settings_current_4", 0x531d6dd4, "0dc0a0e1"),
    ("framing_update", 0x5318f5a8, "0dc0a0e1"),
    ("framing_exit", 0x5318fbdc, "0dc0a0e1"),
    ("framing_destroy", 0x53188c30, "0dc0a0e1"),
    ("framing_canvas", 0x535910ac, "0dc0a0e1"),
    ("framing_children", 0x53591178, "6b0800eb"),
    ("framing_capture_request_hook", 0x53774744, "0010d4e5"),
    // The live-view mode function (0x538b40c8) and the mode check (0x5366440c).
    ("framing_live_mode", 0x538b40e0, "1e26fbeb"),
    ("framing_live_check", 0x53664424, "4d6504eb"),
    ("framing_height_0", 0x538f7920, "983d00e3"),
    ("framing_height_1", 0x538f790c, "ae3ea0e3"),
    ("framing_height_2", 0x538f78f4, "7b3ea0e3"),
    ("framing_height_3", 0x538f78e0, "383400e3"),
    ("framing_screennail_inset", 0x538f7a24, "2610a0e3"),
    ("framing_thumbnail_inset", 0x538f7adc, "1010a0e3"),
    ("framing_width_0", 0x538f7850, "562da0e3"),
    ("framing_width_1", 0x538f783c, "302101e3"),
    ("framing_width_2", 0x538f7828, "c32ea0e3"),
    ("framing_width_3", 0x538f7814, "1e2da0e3"),
    ("framing_narrow_height_3", 0x538f7818, "5a3ea0e3"),
    ("framing_narrow_screennail", 0x538f7a38, "2800a0e3"),
    ("framing_narrow_thumbnail", 0x538f7af8, "b010c3e1"),
    ("framing_classify", 0x533b208c, "51fcffeb"),
    ("framing_image_aspect", 0x533b2494, "4ffbffea"),
    ("module_gamma_dirty", 0x5393befc, "0e30d4e5"),
    ("picture_post", 0x53942494, "e9ffffeb"),
    ("picture_encode_entry", 0x53727e10, "0dc0a0e1"),
    // The ImageTone call of the maker-note setter, then the adjustments fill
    // of tag 0x247 (handler 0x53836c84).
    ("picture_capture_tone", 0x53837414, "c636ffeb"),
    ("picture_capture_parameters", 0x53836c90, "163bffeb"),
    ("framing_level_threshold", 0x5392b148, "fe5ca0e3"),
    ("framing_level_mode", 0x5392b034, "0610a0e1"),
    // LiveViewController histogram update: the BL to the 256-to-128 bin copy.
    ("framing_histogram", 0x53188ce4, "046118eb"),
    // DigitalFilterProcess SetEffectParameter (0x53726594), and its stage
    // gate (0x53739a5c, plan +0x25).
    ("grain_enable", 0x53726770, "b537d4e5"),
    ("grain_strength", 0x53726790, "b737d4e5"),
    ("grain_size", 0x537267b4, "b637d4e5"),
    ("grain_gate", 0x53739f40, "c5feffeb"),
    // The grain tables: `mov r0, #0x200` after the luma amplitude load.
    ("grain_amplitude", 0x538c5cd8, "020ca0e3"),
];

/// The stock instructions around the processing hooks.
const CONTEXTS: &[(u32, &str)] = &[
    // Normalizer BL, then the type-table getters, the record store, and the
    // MultiAxial set store. 1.20 keeps the table index in fp-0x30.
    (0x5393aa10, "c4f9ffeb"),
    (
        0x5393af64,
        "30a01be50a10a0e10600a0e1094c00eb080084e50a10a0e10600a0e1104c00eb0c0084e5",
    ),
    // Packet: the last field store before the native tail.
    (0x5393ae70, "943195e5903084e5f05000eb"),
    // Gamma stage: the custom-curve flag and the curve getter BL.
    (0x53946150, "9b10d1e5"),
    (0x5394655c, "ad1a00eb"),
    // Gamma stage frame, the end of its custom-curve path (table-build flag and
    // GAMMA_BUILT), and its epilogue, which the curve cache relies on.
    (
        0x53946130,
        "0dc0a0e184340ce3f0df2de904b04ce20120a0e1383445e334d04de254100be5",
    ),
    (0x53946154, "58300be5003093e5000051e3142092e550000be5"),
    (
        0x53946520,
        "50301be5034a83e20130a0e30a38c4e50030a0e32c3884e5",
    ),
    (0x53946418, "28d04be2f0af9de8"),
    // LiveViewController's Image Control widget: load native type, map, assign.
    (0x531891d8, "ec3494e5401ad3e5477608eb"),
];

const INSTRUCTION_PATCHES: &[InstructionPatch] = &[
    // ImageTone extraction, as on 1.11.
    (0x537ff004, "0020a0e3", "ff2c80e2"),
];

const FRAMING_PATCHES: &[InstructionPatch] = &[];

/// The startup `.data` literals, as on GR III, and the icon catalog getter
/// (`movw r0` / `movt r0`, 0x55014280), which patched firmware moves into the
/// RTOS tail when it adds icons.
const GUARDS: &[(u32, &str)] = &[
    (0x53000450, "00313b540000005500850500"),
    (0x534013f8, "800204e3010545e3"),
];
