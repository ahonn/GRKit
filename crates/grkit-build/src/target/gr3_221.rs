//! GR III 2.21, which GR IIIx 1.71 also runs. Its addresses come from 2.10
//! through the code that uses them; not tested on a camera.

use super::{gr3, Download, ImageControl, Source, Target};
use crate::components::{InstructionPatch, Layout, Site};

pub const TARGET: Target = Target {
    name: "gr3-2.21",
    family: &gr3::FAMILY,
    firmware: "GR III 2.21",
    firmware_sha256: "7522c36b74294fa854391217f151ef731a4ebbd6ef4449dcf05684828aea4bd9",
    decoded_sha256: "aed094bf84870155bbc25e3151e042f50f37e6c1deec26532c98f39570c2ee5f",
    download: Download {
        zip: "gr3_v221.zip",
        zip_sha256: "39fff62978cbd2e98d93b9adf1c9e44c98cf9501dc977819ca05d6e78dda3ca4",
        file: "fwdc239b.bin",
        source: Source::Ricoh,
    },
    // GR III 2.21 and GR IIIx 1.71.
    identities: &[0x1d141502, 0x13144701],
    data_start: 0x55000000,
    data_source: 0x54084640,
    data_size: 0x3a300,
    array_table: 0x55380b04,
    array_count: 0x5537f6f0,
    heap_min: 0x55803ef4,
    heap_end: 0x60000000,
    english_slot: 0x55002bb8,
    pixel_base_slot: 0x5501898c,
    pixel_base_indirect: false,
    native_tone_page: None,
    model_record: None,
    layout: &LAYOUT,
    image_control: Some(&IMAGE_CONTROL),
    // The English entry is +4 in the language table, which the text hooks load.
    relocation_bases: &[(0x55002bb8, 0x55002bb4)],
};

pub const IMAGE_CONTROL: ImageControl = ImageControl {
    tables: [0x55018fec, 0x55018f48, 0x550190a4],
    table_bytes: 104,
    record: 0x55019264,
    coefficients: 0x53dcb34c,
    record_tail: [0x53dcb3ac, 0x53dcb530, 0x53dcb5d4, 0x53dcb03c],
    base_gamma: 0x53dd83e8,
    settings_domain: 0x55345b80,
    settings_type: 0x77c,
    type_count: 26,
};

/// The tail after the decoded RTOS image (0x540be940).
pub const LAYOUT: Layout = Layout {
    define: "-DGRKIT_TARGET_GR3_221",
    tail: 0x540bf000,
    contexts: CONTEXTS,
    sites: SITES,
    instruction_patches: INSTRUCTION_PATCHES,
    framing_patches: FRAMING_PATCHES,
    guards: GUARDS,
};

const SITES: &[Site] = &[
    ("module_parameters", 0x536e0364, "abfdffeb"),
    ("module_processing", 0x536e04cc, "ec0f08e3"),
    ("module_packet", 0x536e08cc, "903084e5"),
    ("module_gamma_flag", 0x536ec8d4, "9b30d4e5"),
    ("module_gamma_curves", 0x536eca04, "891900eb"),
    ("menu_native_touch_0", 0x531a5920, "f431d0e5"),
    ("menu_native_touch_1", 0x531a5934, "f431d0e5"),
    ("menu_native_touch_2", 0x531a5948, "f431d0e5"),
    ("menu_native_touch_3", 0x531a595c, "f431d0e5"),
    ("menu_native_touch_4", 0x531a5970, "f431d0e5"),
    ("menu_destroy", 0x531a0f50, "0dc0a0e1"),
    ("menu_refresh", 0x531a5144, "0dc0a0e1"),
    ("menu_event", 0x531a5de8, "b030d1e1"),
    ("menu_exit", 0x531a4eb8, "7022d0e5"),
    ("menu_changed", 0x531a5d2c, "0dc0a0e1"),
    ("menu_apply", 0x531a4130, "0dc0a0e1"),
    ("menu_text_measure", 0x53399128, "077193e7"),
    ("menu_text_draw", 0x53399b30, "073193e7"),
    ("menu_tick_type", 0x531a4ff4, "15faffeb"),
    ("menu_icon_width", 0x5320c390, "9e3200e3"),
    ("menu_icon_height", 0x5320c3c4, "9e3200e3"),
    ("menu_icon_guard", 0x5320cba4, "9e1200e3"),
    ("menu_icon_descriptor", 0x5320cbe8, "033192e7"),
    ("menu_native_adj_apply", 0x531a1bcc, "0dc0a0e1"),
    ("menu_native_dialog_apply", 0x531ac9c8, "0dc0a0e1"),
    ("menu_native_page_event", 0x531ad2a0, "0dc0a0e1"),
    ("menu_native_page_exit", 0x531ac1e0, "0dc0a0e1"),
    ("menu_viewfinder_icon", 0x53172230, "27e804eb"),
    ("menu_playback_icon", 0x5317acf0, "65fa04eb"),
    ("menu_playback_detail", 0x532ba49c, "eef1ffeb"),
    ("menu_parameter_start", 0x531b2050, "0dc0a0e1"),
    ("menu_parameter_kinds", 0x531b20c4, "d2eeffeb"),
    ("menu_parameter_snapshot_0", 0x531b2090, "a14e04eb"),
    ("menu_parameter_snapshot_1", 0x531b20f8, "874e04eb"),
    ("menu_parameter_event", 0x531b1edc, "b030d1e1"),
    ("menu_parameter_get_0", 0x532c4fdc, "0dc0a0e1"),
    ("menu_parameter_get_1", 0x532c4ffc, "0dc0a0e1"),
    ("menu_parameter_get_2", 0x532c501c, "0dc0a0e1"),
    ("menu_parameter_get_3", 0x532c503c, "0dc0a0e1"),
    ("menu_parameter_get_4", 0x532c505c, "0dc0a0e1"),
    ("menu_parameter_get_5", 0x532c507c, "0dc0a0e1"),
    ("menu_parameter_get_6", 0x532c509c, "0dc0a0e1"),
    ("menu_parameter_get_7", 0x532c51bc, "0dc0a0e1"),
    ("menu_parameter_get_8", 0x532c51dc, "0dc0a0e1"),
    ("menu_parameter_set_0", 0x532bff24, "0dc0a0e1"),
    ("menu_parameter_set_1", 0x532c0478, "0dc0a0e1"),
    ("menu_parameter_set_2", 0x532c09cc, "0dc0a0e1"),
    ("menu_parameter_set_3", 0x532c1040, "0dc0a0e1"),
    ("menu_parameter_set_4", 0x532c16b4, "0dc0a0e1"),
    ("menu_parameter_set_5", 0x532c1d28, "0dc0a0e1"),
    ("menu_parameter_set_6", 0x532c239c, "0dc0a0e1"),
    ("menu_parameter_set_7", 0x532c3548, "0dc0a0e1"),
    ("menu_parameter_set_8", 0x532c3bbc, "0dc0a0e1"),
    ("menu_parameter_row_0", 0x532ae76c, "c8780aeb"),
    ("menu_parameter_row_1", 0x532ae888, "92780aeb"),
    ("menu_parameter_row_2", 0x532ae9a4, "5c780aeb"),
    ("menu_parameter_row_3", 0x532aeac0, "26780aeb"),
    ("menu_parameter_row_4", 0x532aebdc, "f0770aeb"),
    ("menu_parameter_row_5", 0x532aecf8, "ba770aeb"),
    ("menu_parameter_row_6", 0x532aee14, "84770aeb"),
    ("menu_parameter_row_7", 0x532af430, "85760aeb"),
    ("menu_parameter_row_8", 0x532af54c, "4f760aeb"),
    ("menu_settings_change", 0x5316da14, "0dc0a0e1"),
    ("menu_info_update", 0x531e1b20, "0dc0a0e1"),
    ("menu_check_factory", 0x53170474, "413100e3"),
    ("menu_check_lines", 0x531bb610, "8a6e07eb"),
    ("menu_check_dot", 0x531bf188, "f98707eb"),
    ("menu_settings_kinds_r3_0", 0x531c6d80, "b43345e3"),
    ("menu_settings_kinds_r2", 0x531c6de8, "b42345e3"),
    ("menu_settings_kinds_r8", 0x531c7154, "b48345e3"),
    ("menu_settings_kinds_r5", 0x531c737c, "b45345e3"),
    ("menu_settings_kinds_r3_1", 0x531c7450, "b43345e3"),
    ("menu_settings_build", 0x531c7110, "1cc04be5"),
    ("menu_settings_hide_first", 0x531c71a4, "040057e3"),
    ("menu_settings_hide_next", 0x531c71c8, "040054e3"),
    ("menu_settings_row_kind", 0x531c72c0, "031198e7"),
    ("menu_settings_label", 0x531c72a4, "b810d3e1"),
    ("menu_settings_open", 0x531c6dfc, "145092e5"),
    ("menu_settings_count_0", 0x531b4cb0, "0600a0e3"),
    ("menu_settings_draw_0", 0x531b6b60, "0dc0a0e1"),
    ("menu_settings_apply_0", 0x531b8540, "0dc0a0e1"),
    ("menu_settings_current_0", 0x531b9d10, "0dc0a0e1"),
    ("menu_settings_count_1", 0x531b4cb8, "0200a0e3"),
    ("menu_settings_draw_1", 0x531b6ba0, "0dc0a0e1"),
    ("menu_settings_apply_1", 0x531b8570, "0dc0a0e1"),
    ("menu_settings_current_1", 0x531b9d44, "0dc0a0e1"),
    ("menu_settings_count_2", 0x531b4cc0, "0200a0e3"),
    ("menu_settings_draw_2", 0x531b6be0, "0dc0a0e1"),
    ("menu_settings_apply_2", 0x531b85a0, "0dc0a0e1"),
    ("menu_settings_current_2", 0x531b9d78, "0dc0a0e1"),
    ("menu_settings_count_3", 0x531b4cd0, "0200a0e3"),
    ("menu_settings_draw_3", 0x531b6c60, "0dc0a0e1"),
    ("menu_settings_apply_3", 0x531b8600, "0dc0a0e1"),
    ("menu_settings_current_3", 0x531b9de0, "0dc0a0e1"),
    ("menu_settings_count_4", 0x531b4cd8, "0200a0e3"),
    ("menu_settings_draw_4", 0x531b6ca0, "0dc0a0e1"),
    ("menu_settings_apply_4", 0x531b8630, "0dc0a0e1"),
    ("menu_settings_current_4", 0x531b9e14, "0dc0a0e1"),
    ("framing_update", 0x53177984, "0dc0a0e1"),
    ("framing_exit", 0x53177f4c, "0dc0a0e1"),
    ("framing_destroy", 0x53171cbc, "0dc0a0e1"),
    ("framing_canvas", 0x53394544, "0dc0a0e1"),
    ("framing_children", 0x53394610, "500800eb"),
    ("framing_capture_request_hook", 0x53544f5c, "0010d4e5"),
    ("picture_dng_origin", 0x535fded4, "d9faffeb"),
    ("picture_dng_size", 0x535fdf8c, "dffaffeb"),
    ("framing_height_0", 0x536a6420, "d33ea0e3"),
    ("framing_height_1", 0x536a640c, "a93ea0e3"),
    ("framing_height_2", 0x536a63f8, "763ea0e3"),
    ("framing_height_3", 0x536a63e0, "383400e3"),
    ("framing_screennail_inset", 0x536a6570, "2610a0e3"),
    ("framing_thumbnail_inset", 0x536a6628, "1010a0e3"),
    ("framing_width_0", 0x536a6368, "d02401e3"),
    ("framing_width_1", 0x536a632c, "b02001e3"),
    ("framing_width_2", 0x536a6354, "bb2ea0e3"),
    ("framing_width_3", 0x536a6340, "1e2da0e3"),
    ("framing_narrow_height_3", 0x536a6344, "5a3ea0e3"),
    ("framing_narrow_screennail", 0x536a6584, "2800a0e3"),
    ("framing_narrow_thumbnail", 0x536a6644, "b010c3e1"),
    ("framing_classify", 0x532b6158, "0dfcffeb"),
    ("framing_image_aspect", 0x532b644c, "50fbffea"),
    // RawDevelopmentModel: the BL of the initial aspect for a capture aspect
    // that GR III cannot develop, and the BL to ConvertToCommonPropertyAspectRatio
    // before the aspect goes to the pipeline.
    ("framing_develop_aspect", 0x5328c294, "5ac7ffeb"),
    ("framing_develop_push", 0x5328c77c, "a4ffffeb"),
    ("framing_develop_size", 0x5328c734, "b6ffffeb"),
    // ResizeModel: `mov r0, r4` at the end of the size getter (r4 the size),
    // and the BL to the size setter in GetResizeResolution.
    ("framing_resize_source", 0x532d8e24, "0400a0e1"),
    ("framing_resize_target", 0x532d8758, "e3820feb"),
    // The entry of the PlayDefs aspect of an image size (PlayJobEditResize,
    // PlaybackEditProcess::ConvertToAspectRatio).
    ("framing_play_aspect", 0x534a52b4, "0dc0a0e1"),
    ("module_gamma_dirty", 0x536e23e8, "0e30d4e5"),
    ("picture_post", 0x536e89b4, "e9ffffeb"),
    ("picture_encode_entry", 0x53507214, "0dc0a0e1"),
    // PentaxMakernoteEntryDataSetter: the ImageTone call of tag 0x4f, then
    // the BL to the adjustments fill of tag 0x247 (handler 0x5360a450).
    ("picture_capture_tone", 0x5360abe8, "e1fdfeeb"),
    ("picture_capture_parameters", 0x5360a45c, "3302ffeb"),
    // The JPEG header setup (0x536063ac for a write, 0x5360660c for a rewrite):
    // the BL to the APP1 Exif setup (0x53606210), which sets the APP7 offset
    // at writer +0x48.
    ("picture_jpeg_header", 0x53606400, "82ffffeb"),
    ("picture_jpeg_rewrite", 0x53606658, "ecfeffeb"),
    // RawDevelopmentModel: the BL to the Image Control conversion of the start.
    ("picture_develop_tone", 0x5328c350, "63fcffeb"),
    ("framing_level_threshold", 0x536d23d8, "fe5ca0e3"),
    ("framing_level_mode", 0x536d22c4, "0610a0e1"),
    // LiveViewController histogram update: the BL to the 256-to-128 bin copy.
    ("framing_histogram", 0x53171d94, "44f40feb"),
    // The Image Control list of a RAW development: the entries of
    // GetListItemCount, the row category and value of RawDevelopmentModel, the
    // row fill, IsCurrentValue, the Image Control choice, SetupScrollListView,
    // and the Fn test; the BL to the model restore in Restore; the BLs to the
    // Image Control label in UpdateSubTitle of the list and of
    // RawDevelopmentTopController; the BL to the Image Control icon of its rows.
    ("menu_develop_count", 0x5318aaa0, "0dc0a0e1"),
    ("menu_develop_category", 0x53285c58, "0530d0e5"),
    ("menu_develop_value", 0x53285c78, "0dc0a0e1"),
    ("menu_develop_row", 0x5318b280, "0dc0a0e1"),
    ("menu_develop_current", 0x5318b9e0, "0dc0a0e1"),
    ("menu_develop_decide", 0x5318b4d0, "0dc0a0e1"),
    ("menu_develop_open", 0x5318c324, "0dc0a0e1"),
    ("menu_develop_fn", 0x5318bee8, "0dc0a0e1"),
    ("menu_develop_restore", 0x5318bbd4, "b0ff03eb"),
    ("menu_develop_list_text", 0x5318c260, "c5fd03eb"),
    ("menu_develop_top_text", 0x53187580, "fd1004eb"),
    ("menu_develop_top_icon", 0x531877d0, "561004eb"),
    // The aspect list of a RAW development: the entries of the row value of
    // RawDevelopmentModel and of the aspect choice; the BL to the model
    // restore in Restore; the BLs to the aspect label in UpdateSubTitle of the
    // list and of RawDevelopmentTopController; the BL to the aspect icon of its
    // rows.
    ("menu_develop_aspect_value", 0x53285b6c, "0530d0e5"),
    ("menu_develop_aspect_decide", 0x5318b41c, "0dc0a0e1"),
    ("menu_develop_aspect_restore", 0x5318bba4, "000304eb"),
    ("menu_develop_aspect_list_text", 0x5318c1f4, "310104eb"),
    ("menu_develop_aspect_top_text", 0x531875c8, "3c1404eb"),
    ("menu_develop_aspect_top_icon", 0x53187824, "961304eb"),
    // DigitalFilterProcess SetEffectParameter (0x53505a8c), and its stage
    // gate (0x53515c00, plan +0x24).
    ("grain_particle", 0x53505bf0, "9537d5e5"),
    ("grain_gate", 0x5351605c, "e7feffeb"),
    // The granulation (0x5367e314): the BL to its noise fill (0x5368df04).
    ("grain_fill", 0x5367e3ec, "c43e00eb"),
    // The granulation (0x5367e314): the first test of its amplitude choice.
    ("grain_amplitude", 0x5367e400, "020055e3"),
    ("menu_parameter_get_9", 0x532c50fc, "0dc0a0e1"),
    ("menu_parameter_set_9", 0x532c2ea0, "0dc0a0e1"),
    ("menu_parameter_row_9", 0x532af14c, "e9760aeb"),
];

const CONTEXTS: &[(u32, &str)] = &[
    (0x536e0364, "abfdffeb"),
    // The instructions that module_processing replaces. Its stock path in
    // runtime/processing.S does the same with literal words for the addresses.
    (
        0x536e04cc,
        "ec0f08e348cf08e3010545e301c545e37b9f86e201e190e7a40009e3010545e3",
    ),
    (0x536e08c8, "943196e5903084e58b4e00eb"),
    (0x536e23e8, "0e30d4e5"),
    (0x536ec8d4, "9b30d4e5"),
    (0x536eca04, "891900eb"),
    // Gamma stage frame, the end of its custom-curve path (table-build flag and
    // GAMMA_BUILT), and its epilogue, which the curve cache relies on.
    (
        0x536ec8a0,
        "0dc0a0e13c380ee3f0df2de9053445e304b04ce22cd04de20140a0e14c300be5003093e548100be530300be544000be5",
    ),
    (0x536ecc0c, "44301be5034a83e20130a0e30a32c4e50030a0e3303284e5"),
    (0x536ec9c0, "28d04be2f0af9de8"),
    // LiveViewController's Image Control widget: load native type, map, assign.
    (0x53172228, "e83494e5001ad3e527e804eb0040a0e10500a0e10410a0e165ac08eb"),
];

const INSTRUCTION_PATCHES: &[InstructionPatch] = &[
    // ImageTone extraction stores 0x10000 | (tag - 0x100) instead of 0 for
    // non-native tags, so playback can match a GRKit code. Native getters still
    // accept only 1..12.
    (0x535c5084, "0020a083", "01288083"),
];

/// Data and single-instruction changes that let capture use the hidden GR IV
/// aspect ids 1 (4:3) and 2 (16:9) while the live view stays 3:2. GR III
/// implements only live-view modes 0 (3:2) and 3 (1:1).
const FRAMING_PATCHES: &[InstructionPatch] = &[
    // Live-view mode table for capture ids 1, 2, 3 (read at 0x5366c9fc): 0, 0, 3.
    (0x53d5d658, "02010300", "00000300"),
    // Second mode lookup at 0x53439bb0: `bhi` -> `bne`, so only id 3 reads its
    // table. Without it the live view times out after a 4:3 or 16:9 capture.
    (0x53439c18, "1500008a", "1500001a"),
    // Pipeline row 0x536a67d8: return row 0 for ids 1 and 2 without the
    // ModelDefs.cpp:115 assertion, as the stock fallback does.
    (0x536a6800, "883e02e3", "080000ea"),
    // AF and touch image rectangle 0x535f13f8: ids 1 and 2 use the 3:2
    // rectangle. Stock writes zeros for id 2 and leaves id 1 stale.
    (0x535f141c, "0c00000a", "0300000a"),
    (0x535f1428, "000052e3", "020052e1"),
];

/// The startup literals for the `.data` copy: source, destination, and size.
/// Source plus size is the end of the RTOS image.
const GUARDS: &[(u32, &str)] = &[(0x53000450, "404608540000005500a30300")];
