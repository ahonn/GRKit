//! GR IV and GR IV HDF 1.20, the build of 2026-05-28 that GR IV 30th
//! Anniversary Edition cameras have from the factory. Ricoh publishes a later
//! 1.20 build (`gr4-1.20`). This build has the menus and the settings layout of
//! that build, but the processing function, the Image Control tables, the
//! parameter getters, playback detail, and Grain of 1.11. Its addresses come
//! from both builds; the image is a dump of a camera. Not tested on a camera.

use super::{gr4, Download, ImageControl, Source, Target};
use crate::components::{InstructionPatch, Layout, Site};

pub const TARGET: Target = Target {
    name: "gr4-1.20-0528",
    family: &gr4::FAMILY,
    firmware: "GR IV 1.20 (2026-05-28 build)",
    firmware_sha256: "dedeeb58ce8e0aee7522b699c1df3270b3601bc73b1a6cc31926c84a7516037b",
    decoded_sha256: "dedeeb58ce8e0aee7522b699c1df3270b3601bc73b1a6cc31926c84a7516037b",
    download: Download {
        zip: "gr4_v120_0528.zip",
        zip_sha256: "62274772b8d0580a9344410bc5ebdc134783eed63c702497770a18f5b6bac6d7",
        file: "RTOS.BIN",
        source: Source::Dump,
    },
    identities: &[0x0a001401],
    data_start: 0x55000000,
    data_source: 0x543aa600,
    data_size: 0x583c0,
    array_table: 0x553b596c,
    array_count: 0x553b4558,
    heap_min: 0x55838d94,
    heap_end: 0x60000000,
    english_slot: 0x5500211c,
    // The drawer of the icon manager singleton (0x55082f28 + 0x10).
    pixel_base_slot: 0x55082f38,
    pixel_base_indirect: true,
    native_tone_page: Some(0x81),
    layout: &LAYOUT,
    image_control: Some(&IMAGE_CONTROL),
    // As on 1.11: the settings domain follows the vtable of the capture
    // property singleton, and the icon drawer is +0x10 in the icon manager.
    relocation_bases: &[(0x5537b080, 0x5537b07c), (0x55082f38, 0x55082f28)],
};

/// The tables of 1.11: 0x22 entries for each model variant, and no 1.20
/// monochrome types.
pub const IMAGE_CONTROL: ImageControl = ImageControl {
    tables: [0x550546c0, 0x55054444, 0x55053c34],
    table_bytes: 408,
    record: 0x5501bcfc,
    coefficients: 0x54078130,
    record_tail: [0x540781b8, 0x54078350, 0x54078408, 0x54077df8],
    base_gamma: 0x540add94,
    settings_domain: 0x5537b080,
    settings_type: 0x79c,
    type_count: 31,
};

/// The tail after the RTOS image (0x544029c0), with the regions of 1.11.
pub const LAYOUT: Layout = Layout {
    define: "-DGRKIT_TARGET_GR4_120_0528",
    tail: (0x54403000, 0x54410000),
    contexts: CONTEXTS,
    sites: SITES,
    instruction_patches: INSTRUCTION_PATCHES,
    framing_patches: FRAMING_PATCHES,
    guards: GUARDS,
};

const SITES: &[Site] = &[
    ("module_parameters", 0x53934018, "d8f9ffeb"),
    ("module_processing", 0x539340e8, "0a10a0e1"),
    ("module_record", 0x539340f4, "080084e5"),
    ("module_multiaxial", 0x53934104, "0c0084e5"),
    ("module_packet", 0x53934530, "903084e5"),
    ("module_gamma_flag", 0x5393f708, "9b10d1e5"),
    ("module_gamma_curves", 0x5393fb14, "231a00eb"),
    ("menu_native_touch_0", 0x531c05a8, "fc31d0e5"),
    ("menu_native_touch_1", 0x531c05bc, "fc31d0e5"),
    ("menu_native_touch_2", 0x531c05d0, "fc31d0e5"),
    ("menu_native_touch_3", 0x531c05e4, "fc31d0e5"),
    ("menu_native_touch_4", 0x531c05f8, "fc31d0e5"),
    ("menu_destroy", 0x531bad94, "0dc0a0e1"),
    ("menu_refresh", 0x531bff98, "0dc0a0e1"),
    ("menu_event", 0x531c0404, "b030d1e1"),
    ("menu_exit", 0x531c0150, "fc11d0e5"),
    ("menu_changed", 0x531bff5c, "0dc0a0e1"),
    ("menu_apply", 0x531bfaa4, "0dc0a0e1"),
    ("menu_text_measure", 0x53591c00, "077193e7"),
    ("menu_text_draw", 0x53592610, "073193e7"),
    ("menu_tick_type", 0x531be8fc, "13feffeb"),
    ("menu_native_adj_apply", 0x531bb9a8, "0dc0a0e1"),
    ("menu_native_dialog_apply", 0x531c6464, "0dc0a0e1"),
    ("menu_native_page_event", 0x531c6f2c, "0dc0a0e1"),
    ("menu_native_page_exit", 0x531c5b88, "0dc0a0e1"),
    ("menu_icon_lookup", 0x5323eb94, "0dc0a0e1"),
    // The icon ID is compared with an immediate limit (NATIVE_ICON_LIMIT), and
    // `bhs` takes the out-of-range path (`bhi` on gr4-1.20).
    ("menu_icon_limit", 0x5323f45c, "b10f57e3"),
    ("menu_icon_entry", 0x5323f4fc, "073190e7"),
    ("menu_viewfinder_icon", 0x53188ba0, "417008eb"),
    ("menu_playback_icon", 0x531922d0, "9f8708eb"),
    ("menu_playback_detail", 0x533b4f38, "7af0ffeb"),
    ("menu_parameter_start", 0x531cc29c, "0dc0a0e1"),
    ("menu_parameter_kinds", 0x531cc314, "51edffeb"),
    ("menu_parameter_snapshot_0", 0x531cc2e0, "fa0208eb"),
    ("menu_parameter_snapshot_1", 0x531cc34c, "df0208eb"),
    ("menu_parameter_event", 0x531cc12c, "b030d1e1"),
    ("menu_parameter_get_0", 0x533c4ab0, "0dc0a0e1"),
    ("menu_parameter_get_1", 0x533c5198, "0dc0a0e1"),
    ("menu_parameter_get_2", 0x533c5880, "0dc0a0e1"),
    ("menu_parameter_get_3", 0x533c62a8, "0dc0a0e1"),
    ("menu_parameter_get_4", 0x533c6cd0, "0dc0a0e1"),
    ("menu_parameter_get_5", 0x533c76f8, "0dc0a0e1"),
    ("menu_parameter_get_6", 0x533c8120, "0dc0a0e1"),
    ("menu_parameter_get_7", 0x533ca738, "0dc0a0e1"),
    ("menu_parameter_get_8", 0x533cb160, "0dc0a0e1"),
    ("menu_parameter_set_0", 0x533c40ac, "0dc0a0e1"),
    ("menu_parameter_set_1", 0x533c40fc, "0dc0a0e1"),
    ("menu_parameter_set_2", 0x533c414c, "0dc0a0e1"),
    ("menu_parameter_set_3", 0x533c419c, "0dc0a0e1"),
    ("menu_parameter_set_4", 0x533c41ec, "0dc0a0e1"),
    ("menu_parameter_set_5", 0x533c423c, "0dc0a0e1"),
    ("menu_parameter_set_6", 0x533c428c, "0dc0a0e1"),
    ("menu_parameter_set_7", 0x533c450c, "0dc0a0e1"),
    ("menu_parameter_set_8", 0x533c455c, "0dc0a0e1"),
    ("menu_parameter_row_0", 0x533a7f08, "743f0feb"),
    ("menu_parameter_row_1", 0x533a8024, "3e3f0feb"),
    ("menu_parameter_row_2", 0x533a8140, "083f0feb"),
    ("menu_parameter_row_3", 0x533a825c, "d23e0feb"),
    ("menu_parameter_row_4", 0x533a8378, "9c3e0feb"),
    ("menu_parameter_row_5", 0x533a8494, "663e0feb"),
    ("menu_parameter_row_6", 0x533a85b0, "303e0feb"),
    ("menu_parameter_row_7", 0x533a8cb0, "093d0feb"),
    ("menu_parameter_row_8", 0x533a8dcc, "d33c0feb"),
    ("menu_settings_change", 0x53183a4c, "0dc0a0e1"),
    ("menu_info_update", 0x531fd70c, "0dc0a0e1"),
    ("menu_check_factory", 0x53186b7c, "2e3100e3"),
    ("menu_check_lines", 0x531d7bec, "96df0eeb"),
    ("menu_check_dot", 0x531db8f4, "52f90eeb"),
    ("menu_settings_kinds_r3_0", 0x531e2eac, "dc3345e3"),
    ("menu_settings_kinds_r2", 0x531e2f14, "dc2345e3"),
    ("menu_settings_kinds_r8", 0x531e3284, "dc8345e3"),
    ("menu_settings_kinds_r5", 0x531e34ac, "dc5345e3"),
    ("menu_settings_kinds_r3_1", 0x531e3580, "dc3345e3"),
    ("menu_settings_build", 0x531e3240, "1cc04be5"),
    ("menu_settings_hide_first", 0x531e32d4, "040057e3"),
    ("menu_settings_hide_next", 0x531e32f8, "040054e3"),
    ("menu_settings_row_kind", 0x531e33f0, "031198e7"),
    ("menu_settings_label", 0x531e33d4, "b810d3e1"),
    ("menu_settings_open", 0x531e2f28, "145092e5"),
    ("menu_settings_count_0", 0x531d00ac, "0600a0e3"),
    ("menu_settings_draw_0", 0x531d296c, "0dc0a0e1"),
    ("menu_settings_apply_0", 0x531d46dc, "0dc0a0e1"),
    ("menu_settings_current_0", 0x531d60f8, "0dc0a0e1"),
    ("menu_settings_count_1", 0x531d00b4, "0200a0e3"),
    ("menu_settings_draw_1", 0x531d29b0, "0dc0a0e1"),
    ("menu_settings_apply_1", 0x531d470c, "0dc0a0e1"),
    ("menu_settings_current_1", 0x531d612c, "0dc0a0e1"),
    ("menu_settings_count_2", 0x531d00bc, "0200a0e3"),
    ("menu_settings_draw_2", 0x531d29f4, "0dc0a0e1"),
    ("menu_settings_apply_2", 0x531d473c, "0dc0a0e1"),
    ("menu_settings_current_2", 0x531d6160, "0dc0a0e1"),
    ("menu_settings_count_3", 0x531d00c4, "0400a0e3"),
    ("menu_settings_draw_3", 0x531d2a38, "0dc0a0e1"),
    ("menu_settings_apply_3", 0x531d476c, "0dc0a0e1"),
    ("menu_settings_current_3", 0x531d6194, "0dc0a0e1"),
    ("menu_settings_count_4", 0x531d00cc, "0200a0e3"),
    ("menu_settings_draw_4", 0x531d2a7c, "0dc0a0e1"),
    ("menu_settings_apply_4", 0x531d479c, "0dc0a0e1"),
    ("menu_settings_current_4", 0x531d61c8, "0dc0a0e1"),
    ("framing_update", 0x5318ef60, "0dc0a0e1"),
    ("framing_exit", 0x5318f594, "0dc0a0e1"),
    ("framing_destroy", 0x531885f0, "0dc0a0e1"),
    ("framing_canvas", 0x5358cee0, "0dc0a0e1"),
    ("framing_children", 0x5358cfac, "6c0800eb"),
    ("framing_capture_request_hook", 0x5376f420, "0010d4e5"),
    // The live-view mode function (0x538ad808) and the mode check (0x5365f164).
    ("framing_live_mode", 0x538ad820, "742bfbeb"),
    ("framing_live_check", 0x5365f17c, "1d6504eb"),
    ("framing_height_0", 0x538f1050, "983d00e3"),
    ("framing_height_1", 0x538f103c, "ae3ea0e3"),
    ("framing_height_2", 0x538f1024, "7b3ea0e3"),
    ("framing_height_3", 0x538f1010, "383400e3"),
    ("framing_screennail_inset", 0x538f1154, "2610a0e3"),
    ("framing_thumbnail_inset", 0x538f120c, "1010a0e3"),
    ("framing_width_0", 0x538f0f80, "562da0e3"),
    ("framing_width_1", 0x538f0f6c, "302101e3"),
    ("framing_width_2", 0x538f0f58, "c32ea0e3"),
    ("framing_width_3", 0x538f0f44, "1e2da0e3"),
    ("framing_narrow_height_3", 0x538f0f48, "5a3ea0e3"),
    ("framing_narrow_screennail", 0x538f1168, "2800a0e3"),
    ("framing_narrow_thumbnail", 0x538f1228, "b010c3e1"),
    ("framing_classify", 0x533b0174, "53fcffeb"),
    ("framing_image_aspect", 0x533b057c, "51fbffea"),
    ("module_gamma_dirty", 0x539354b4, "0e30d4e5"),
    ("picture_post", 0x5393ba4c, "e9ffffeb"),
    ("picture_encode_entry", 0x53722b60, "0dc0a0e1"),
    // The ImageTone call of the maker-note setter, then the adjustments fill
    // of tag 0x247 (handler 0x538310d4).
    ("picture_capture_tone", 0x53831864, "1c37ffeb"),
    ("picture_capture_parameters", 0x538310e0, "6c3bffeb"),
    ("framing_level_threshold", 0x539247a0, "fe5ca0e3"),
    ("framing_level_mode", 0x5392468c, "0610a0e1"),
    // LiveViewController histogram update: the BL to the 256-to-128 bin copy.
    ("framing_histogram", 0x531886a4, "e24d18eb"),
    // DigitalFilterProcess SetEffectParameter (0x537212ec), and its stage
    // gate (0x537347ac, plan +0x25).
    ("grain_enable", 0x537214c0, "b537d4e5"),
    ("grain_strength", 0x537214e0, "b737d4e5"),
    ("grain_size", 0x53721504, "b637d4e5"),
    ("grain_gate", 0x53734c2c, "defeffeb"),
    // The grain tables: `mov r0, #0x200` after the luma amplitude load.
    ("grain_amplitude", 0x538bf418, "020ca0e3"),
];

/// The stock instructions around the processing hooks.
const CONTEXTS: &[(u32, &str)] = &[
    // Normalizer BL, then the type-table getters, the record store, and the
    // MultiAxial set store. As on 1.11, the table index is in fp-0x34.
    (0x53934018, "d8f9ffeb"),
    (
        0x539340e4,
        "34a01be50a10a0e10600a0e18d4c00eb080084e50a10a0e10600a0e1944c00eb0c0084e5",
    ),
    // Packet: the last field store before the native tail.
    (0x5393452c, "943195e5903084e5255000eb"),
    // Gamma stage: the custom-curve flag and the curve getter BL.
    (0x5393f708, "9b10d1e5"),
    (0x5393fb14, "231a00eb"),
    // Gamma stage frame, the end of its custom-curve path (table-build flag and
    // GAMMA_BUILT), and its epilogue, which the curve cache relies on.
    (
        0x5393f6e8,
        "0dc0a0e1543903e3f0df2de904b04ce20120a0e1383445e334d04de254100be5",
    ),
    (0x5393f70c, "58300be5003093e5000051e3142092e550000be5"),
    (
        0x5393fad8,
        "50301be5034a83e20130a0e30a38c4e50030a0e32c3884e5",
    ),
    (0x5393f9d0, "28d04be2f0af9de8"),
    // LiveViewController's Image Control widget: load native type, map, assign.
    (0x53188b98, "ec3494e5401ad3e5417008eb"),
];

const INSTRUCTION_PATCHES: &[InstructionPatch] = &[
    // ImageTone extraction, as on 1.11.
    (0x537f9abc, "0020a0e3", "ff2c80e2"),
];

const FRAMING_PATCHES: &[InstructionPatch] = &[];

/// The startup `.data` literals, as on GR III, and the icon catalog getter
/// (`movw r0` / `movt r0`, 0x55014280), which patched firmware moves into the
/// RTOS tail when it adds icons.
const GUARDS: &[(u32, &str)] = &[
    (0x53000450, "00a63a5400000055c0830500"),
    (0x533fd22c, "800204e3010545e3"),
];
