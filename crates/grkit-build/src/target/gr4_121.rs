//! GR IV and GR IV HDF 1.21, and GR IV Monochrome 1.11, which has the same
//! firmware file. Its addresses come from 1.20 through the code that uses
//! them; not tested on a camera.

use super::{gr4, Download, ImageControl, Source, Target};
use crate::components::{InstructionPatch, Layout, Site};

pub const TARGET: Target = Target {
    name: "gr4-1.21",
    family: &gr4::FAMILY,
    firmware: "GR IV 1.21",
    firmware_sha256: "bf4d6c35f6da1c16ed5f2943af570eab8fef326f7501a2b3ce5413fa55ec5c99",
    decoded_sha256: "38352e38a626e14f599cb3df740b115b515a05a3c6bf45de55fb70b06bdd0bdf",
    download: Download {
        zip: "gr4_v121.zip",
        zip_sha256: "ace9fc65f6e4fa3b1096d590ba4bddad4f4814e075a4d85a973cb943f166da73",
        file: "fwdc248b.bin",
        source: Source::Ricoh,
    },
    identities: &[0x0c0a1501],
    data_start: 0x55000000,
    data_source: 0x543b4900,
    data_size: 0x58500,
    array_table: 0x553b5bd4,
    array_count: 0x553b47c0,
    heap_min: 0x55839014,
    heap_end: 0x60000000,
    english_slot: 0x5500211c,
    // The drawer of the icon manager singleton (0x55083028 + 0x10).
    pixel_base_slot: 0x55083038,
    pixel_base_indirect: true,
    native_tone_page: Some(0x81),
    model_record: Some((0x553a2558, 0x19)),
    layout: &LAYOUT,
    image_control: Some(&IMAGE_CONTROL),
    // As on 1.11: the settings domain follows the vtable of the capture
    // property singleton, and the icon drawer is +0x10 in the icon manager.
    relocation_bases: &[(0x5537b2a8, 0x5537b2a4), (0x55083038, 0x55083028)],
};

/// The tables and types of 1.20.
pub const IMAGE_CONTROL: ImageControl = ImageControl {
    tables: [0x55054800, 0x55054554, 0x55053ce4],
    table_bytes: 456,
    record: 0x5501bd58,
    coefficients: 0x54081d0c,
    record_tail: [0x54081d94, 0x54081f2c, 0x54081fe4, 0x540819c0],
    base_gamma: 0x540b6fc0,
    settings_domain: 0x5537b2a8,
    settings_type: 0x79c,
    type_count: 33,
};

/// The tail after the decoded RTOS image (0x5440ce00), with the regions of
/// 1.11.
pub const LAYOUT: Layout = Layout {
    define: "-DGRKIT_TARGET_GR4_121",
    tail: 0x5440d000,
    contexts: CONTEXTS,
    sites: SITES,
    instruction_patches: INSTRUCTION_PATCHES,
    framing_patches: FRAMING_PATCHES,
    guards: GUARDS,
};

const SITES: &[Site] = &[
    ("module_parameters", 0x5393bfd8, "c4f9ffeb"),
    ("module_processing", 0x5393c530, "0a10a0e1"),
    ("module_record", 0x5393c53c, "080084e5"),
    ("module_multiaxial", 0x5393c54c, "0c0084e5"),
    ("module_packet", 0x5393c43c, "903084e5"),
    ("module_gamma_flag", 0x53947718, "9b10d1e5"),
    ("module_gamma_curves", 0x53947b24, "ad1a00eb"),
    ("menu_native_touch_0", 0x531c1518, "fc31d0e5"),
    ("menu_native_touch_1", 0x531c152c, "fc31d0e5"),
    ("menu_native_touch_2", 0x531c1540, "fc31d0e5"),
    ("menu_native_touch_3", 0x531c1554, "fc31d0e5"),
    ("menu_native_touch_4", 0x531c1568, "fc31d0e5"),
    ("menu_destroy", 0x531bbd04, "0dc0a0e1"),
    ("menu_refresh", 0x531c0f08, "0dc0a0e1"),
    ("menu_event", 0x531c1374, "b030d1e1"),
    ("menu_exit", 0x531c10c0, "fc11d0e5"),
    ("menu_changed", 0x531c0ecc, "0dc0a0e1"),
    ("menu_apply", 0x531c0a14, "0dc0a0e1"),
    ("menu_text_measure", 0x535966f0, "077193e7"),
    ("menu_text_draw", 0x53597100, "073193e7"),
    ("menu_tick_type", 0x531bf86c, "13feffeb"),
    ("menu_native_adj_apply", 0x531bc918, "0dc0a0e1"),
    ("menu_native_dialog_apply", 0x531c73d4, "0dc0a0e1"),
    ("menu_native_page_event", 0x531c7e9c, "0dc0a0e1"),
    ("menu_native_page_exit", 0x531c6af8, "0dc0a0e1"),
    ("menu_icon_lookup", 0x5323fda8, "0dc0a0e1"),
    // As on 1.20, an immediate limit (NATIVE_ICON_LIMIT).
    ("menu_icon_limit", 0x5324066c, "b10f57e3"),
    ("menu_icon_entry", 0x5324070c, "073190e7"),
    ("menu_viewfinder_icon", 0x531891b8, "7d7808eb"),
    ("menu_playback_icon", 0x53192918, "099008eb"),
    ("menu_playback_detail", 0x533b783c, "15f0ffeb"),
    ("menu_parameter_start", 0x531cd4b0, "0dc0a0e1"),
    ("menu_parameter_kinds", 0x531cd528, "c5ecffeb"),
    ("menu_parameter_snapshot_0", 0x531cd4f4, "af0d08eb"),
    ("menu_parameter_snapshot_1", 0x531cd560, "940d08eb"),
    ("menu_parameter_event", 0x531cd340, "b030d1e1"),
    ("menu_parameter_get_0", 0x533c8008, "0dc0a0e1"),
    ("menu_parameter_get_1", 0x533c86f0, "0dc0a0e1"),
    ("menu_parameter_get_2", 0x533c8dd8, "0dc0a0e1"),
    ("menu_parameter_get_3", 0x533c98f0, "0dc0a0e1"),
    ("menu_parameter_get_4", 0x533ca368, "0dc0a0e1"),
    ("menu_parameter_get_5", 0x533cade0, "0dc0a0e1"),
    ("menu_parameter_get_6", 0x533cb858, "0dc0a0e1"),
    ("menu_parameter_get_7", 0x533ce050, "0dc0a0e1"),
    ("menu_parameter_get_8", 0x533ceac8, "0dc0a0e1"),
    ("menu_parameter_set_0", 0x533c75b4, "0dc0a0e1"),
    ("menu_parameter_set_1", 0x533c7604, "0dc0a0e1"),
    ("menu_parameter_set_2", 0x533c7654, "0dc0a0e1"),
    ("menu_parameter_set_3", 0x533c76a4, "0dc0a0e1"),
    ("menu_parameter_set_4", 0x533c76f4, "0dc0a0e1"),
    ("menu_parameter_set_5", 0x533c7744, "0dc0a0e1"),
    ("menu_parameter_set_6", 0x533c7794, "0dc0a0e1"),
    ("menu_parameter_set_7", 0x533c7a14, "0dc0a0e1"),
    ("menu_parameter_set_8", 0x533c7a64, "0dc0a0e1"),
    ("menu_parameter_row_0", 0x533aa610, "9f4d0feb"),
    ("menu_parameter_row_1", 0x533aa72c, "694d0feb"),
    ("menu_parameter_row_2", 0x533aa848, "334d0feb"),
    ("menu_parameter_row_3", 0x533aa964, "fd4c0feb"),
    ("menu_parameter_row_4", 0x533aaa80, "c74c0feb"),
    ("menu_parameter_row_5", 0x533aab9c, "914c0feb"),
    ("menu_parameter_row_6", 0x533aacb8, "5b4c0feb"),
    ("menu_parameter_row_7", 0x533ab3b8, "344b0feb"),
    ("menu_parameter_row_8", 0x533ab4d4, "fe4a0feb"),
    ("menu_settings_change", 0x5318408c, "0dc0a0e1"),
    ("menu_info_update", 0x531fe920, "0dc0a0e1"),
    ("menu_check_factory", 0x531871bc, "2e3100e3"),
    ("menu_check_lines", 0x531d8e00, "cded0eeb"),
    ("menu_check_dot", 0x531dcb08, "89070feb"),
    ("menu_settings_kinds_r3_0", 0x531e40c0, "dd3345e3"),
    ("menu_settings_kinds_r2", 0x531e4128, "dd2345e3"),
    ("menu_settings_kinds_r8", 0x531e4498, "dd8345e3"),
    ("menu_settings_kinds_r5", 0x531e46c0, "dd5345e3"),
    ("menu_settings_kinds_r3_1", 0x531e4794, "dd3345e3"),
    ("menu_settings_build", 0x531e4454, "1cc04be5"),
    ("menu_settings_hide_first", 0x531e44e8, "040057e3"),
    ("menu_settings_hide_next", 0x531e450c, "040054e3"),
    ("menu_settings_row_kind", 0x531e4604, "031198e7"),
    ("menu_settings_label", 0x531e45e8, "b810d3e1"),
    ("menu_settings_open", 0x531e413c, "145092e5"),
    ("menu_settings_count_0", 0x531d12c0, "0600a0e3"),
    ("menu_settings_draw_0", 0x531d3b80, "0dc0a0e1"),
    ("menu_settings_apply_0", 0x531d58f0, "0dc0a0e1"),
    ("menu_settings_current_0", 0x531d730c, "0dc0a0e1"),
    ("menu_settings_count_1", 0x531d12c8, "0200a0e3"),
    ("menu_settings_draw_1", 0x531d3bc4, "0dc0a0e1"),
    ("menu_settings_apply_1", 0x531d5920, "0dc0a0e1"),
    ("menu_settings_current_1", 0x531d7340, "0dc0a0e1"),
    ("menu_settings_count_2", 0x531d12d0, "0200a0e3"),
    ("menu_settings_draw_2", 0x531d3c08, "0dc0a0e1"),
    ("menu_settings_apply_2", 0x531d5950, "0dc0a0e1"),
    ("menu_settings_current_2", 0x531d7374, "0dc0a0e1"),
    ("menu_settings_count_3", 0x531d12d8, "0400a0e3"),
    ("menu_settings_draw_3", 0x531d3c4c, "0dc0a0e1"),
    ("menu_settings_apply_3", 0x531d5980, "0dc0a0e1"),
    ("menu_settings_current_3", 0x531d73a8, "0dc0a0e1"),
    ("menu_settings_count_4", 0x531d12e0, "0200a0e3"),
    ("menu_settings_draw_4", 0x531d3c90, "0dc0a0e1"),
    ("menu_settings_apply_4", 0x531d59b0, "0dc0a0e1"),
    ("menu_settings_current_4", 0x531d73dc, "0dc0a0e1"),
    ("framing_update", 0x5318f580, "0dc0a0e1"),
    ("framing_exit", 0x5318fbb4, "0dc0a0e1"),
    ("framing_destroy", 0x53188c08, "0dc0a0e1"),
    ("framing_canvas", 0x535919d4, "0dc0a0e1"),
    ("framing_children", 0x53591aa0, "6b0800eb"),
    ("framing_capture_request_hook", 0x537753d4, "0010d4e5"),
    // The live-view mode function (0x538b4d70) and the mode check (0x53664d4c).
    ("framing_live_mode", 0x538b4d88, "1826fbeb"),
    ("framing_live_check", 0x53664d64, "216604eb"),
    ("picture_dng_origin", 0x5382b4c0, "dafaffeb"),
    ("picture_dng_size", 0x5382b578, "e0faffeb"),
    ("framing_height_0", 0x538f85b8, "983d00e3"),
    ("framing_height_1", 0x538f85a4, "ae3ea0e3"),
    ("framing_height_2", 0x538f858c, "7b3ea0e3"),
    ("framing_height_3", 0x538f8578, "383400e3"),
    ("framing_screennail_inset", 0x538f86bc, "2610a0e3"),
    ("framing_thumbnail_inset", 0x538f8774, "1010a0e3"),
    ("framing_width_0", 0x538f84e8, "562da0e3"),
    ("framing_width_1", 0x538f84d4, "302101e3"),
    ("framing_width_2", 0x538f84c0, "c32ea0e3"),
    ("framing_width_3", 0x538f84ac, "1e2da0e3"),
    ("framing_narrow_height_3", 0x538f84b0, "5a3ea0e3"),
    ("framing_narrow_screennail", 0x538f86d0, "2800a0e3"),
    ("framing_narrow_thumbnail", 0x538f8790, "b010c3e1"),
    ("framing_classify", 0x533b28e4, "67fcffeb"),
    ("framing_image_aspect", 0x533b2cec, "65fbffea"),
    ("framing_develop_aspect", 0x5338433c, "57ffffeb"),
    ("framing_develop_push", 0x533845b4, "96ffffeb"),
    ("framing_develop_size", 0x53384568, "a9ffffeb"),
    ("framing_resize_source", 0x533ec5c0, "0400a0e1"),
    ("framing_resize_target", 0x533ebe70, "d19a14eb"),
    ("framing_play_aspect", 0x536bb7b4, "0dc0a0e1"),
    ("module_gamma_dirty", 0x5393d4c4, "0e30d4e5"),
    ("picture_post", 0x53943a5c, "e9ffffeb"),
    ("picture_encode_entry", 0x53728860, "0dc0a0e1"),
    // The ImageTone call of the maker-note setter, then the adjustments fill
    // of tag 0x247 (handler 0x53837b4c).
    ("picture_capture_tone", 0x538382dc, "6336ffeb"),
    ("picture_capture_parameters", 0x53837b58, "b33affeb"),
    // The JPEG header setup (0x53833a74 for a write, 0x53833cd4 for a rewrite):
    // the BL to the APP1 Exif setup (0x538338d8), which sets the APP7 offset
    // at writer +0x48.
    ("picture_jpeg_header", 0x53833ac8, "82ffffeb"),
    ("picture_jpeg_rewrite", 0x53833d20, "ecfeffeb"),
    ("picture_develop_tone", 0x533843f8, "b6faffeb"),
    ("framing_level_threshold", 0x5392c70c, "fe5ca0e3"),
    ("framing_level_mode", 0x5392c5f8, "0610a0e1"),
    // LiveViewController histogram update: the BL to the 256-to-128 bin copy.
    ("framing_histogram", 0x53188cbc, "546418eb"),
    ("menu_develop_count", 0x531a2bcc, "0dc0a0e1"),
    ("menu_develop_category", 0x5337a6b4, "0530d0e5"),
    ("menu_develop_value", 0x5337a6e4, "0dc0a0e1"),
    ("menu_develop_row", 0x531a33b4, "0dc0a0e1"),
    ("menu_develop_current", 0x531a3b14, "0dc0a0e1"),
    ("menu_develop_decide", 0x531a3604, "0dc0a0e1"),
    ("menu_develop_open", 0x531a448c, "0dc0a0e1"),
    ("menu_develop_fn", 0x531a400c, "0dc0a0e1"),
    ("menu_develop_restore", 0x531a3d08, "a87e07eb"),
    ("menu_develop_list_text", 0x531a43c8, "ac7c07eb"),
    ("menu_develop_top_text", 0x5319fc20, "968e07eb"),
    ("menu_develop_top_icon", 0x5319fed4, "d68d07eb"),
    // The aspect list of a RAW development: the entries of the row value of
    // RawDevelopmentModel and of the aspect choice; the BL to the model
    // restore in Restore; the BLs to the aspect label in UpdateSubTitle of the
    // list and of RawDevelopmentTopController; the BL to the aspect icon of its
    // rows.
    ("menu_develop_aspect_value", 0x5337a540, "0dc0a0e1"),
    ("menu_develop_aspect_decide", 0x531a3550, "0dc0a0e1"),
    ("menu_develop_aspect_restore", 0x531a3cd8, "448207eb"),
    ("menu_develop_aspect_list_text", 0x531a435c, "638007eb"),
    ("menu_develop_aspect_top_text", 0x5319fc68, "209207eb"),
    ("menu_develop_aspect_top_icon", 0x5319ff58, "539107eb"),
    // DigitalFilterProcess SetEffectParameter (0x53726fe4), and its stage
    // gate (0x5373a4ac, plan +0x25).
    ("grain_enable", 0x537271c0, "b537d4e5"),
    ("grain_strength", 0x537271e0, "b737d4e5"),
    ("grain_size", 0x53727204, "b637d4e5"),
    ("grain_gate", 0x5373a990, "c5feffeb"),
    // The grain tables: `mov r0, #0x200` after the luma amplitude load.
    ("grain_amplitude", 0x538c6980, "020ca0e3"),
];

/// The stock instructions around the processing hooks.
const CONTEXTS: &[(u32, &str)] = &[
    // Normalizer BL, then the type-table getters, the record store, and the
    // MultiAxial set store. As on 1.20, the table index is in fp-0x30.
    (0x5393bfd8, "c4f9ffeb"),
    (
        0x5393c52c,
        "30a01be50a10a0e10600a0e1094c00eb080084e50a10a0e10600a0e1104c00eb0c0084e5",
    ),
    // Packet: the last field store before the native tail.
    (0x5393c438, "943195e5903084e5f05000eb"),
    // Gamma stage: the custom-curve flag and the curve getter BL.
    (0x53947718, "9b10d1e5"),
    (0x53947b24, "ad1a00eb"),
    // Gamma stage frame, the end of its custom-curve path (table-build flag and
    // GAMMA_BUILT), and its epilogue, which the curve cache relies on.
    (
        0x539476f8,
        "0dc0a0e1743c0de3f0df2de904b04ce20120a0e1383445e334d04de254100be5",
    ),
    (0x5394771c, "58300be5003093e5000051e3142092e550000be5"),
    (
        0x53947ae8,
        "50301be5034a83e20130a0e30a38c4e50030a0e32c3884e5",
    ),
    (0x539479e0, "28d04be2f0af9de8"),
    // LiveViewController's Image Control widget: load native type, map, assign.
    (0x531891b0, "ec3494e5401ad3e57d7808eb"),
];

const INSTRUCTION_PATCHES: &[InstructionPatch] = &[
    // ImageTone extraction, as on 1.11.
    (0x537ffccc, "0020a0e3", "ff2c80e2"),
];

const FRAMING_PATCHES: &[InstructionPatch] = &[];

/// The startup `.data` literals, as on GR III, and the icon catalog getter
/// (`movw r0` / `movt r0`, 0x55014280), which patched firmware moves into the
/// RTOS tail when it adds icons.
const GUARDS: &[(u32, &str)] = &[
    (0x53000450, "00493b540000005500850500"),
    (0x53401d20, "800204e3010545e3"),
];
