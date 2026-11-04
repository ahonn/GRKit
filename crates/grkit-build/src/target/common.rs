//! The hooks that the GR III and GR IV families share, in groups. A family
//! joins these groups and its own hooks into its hook list (`join`), in
//! installation order. Each build gives the site of each hook.

use crate::components::{
    b, b_shared, bl, bl_shared, HookSpec,
    Part::{Framing, Menu},
    Point::{CaptureAspect, MenuPage, PictureEncode, Tuning, ViewfinderOverlay},
};

/// AdjModeController: the touch thunks of the five slots, destroy, refresh,
/// event, exit, changed, and the apply of the list value to the slot. Then
/// the table loads of the text measure and draw, and the BL to the slot type
/// in the periodic update, which shows the exposure box of ISO Setting.
pub const ADJ: &[HookSpec] = &[
    b("menu_native_touch_0", Menu, MenuPage),
    b("menu_native_touch_1", Menu, MenuPage),
    b("menu_native_touch_2", Menu, MenuPage),
    b("menu_native_touch_3", Menu, MenuPage),
    b("menu_native_touch_4", Menu, MenuPage),
    b("menu_destroy", Menu, MenuPage),
    b("menu_refresh", Menu, MenuPage),
    b("menu_event", Menu, MenuPage),
    b("menu_exit", Menu, MenuPage),
    b("menu_changed", Menu, MenuPage),
    b("menu_apply", Menu, MenuPage),
    b("menu_text_measure", Menu, MenuPage),
    b("menu_text_draw", Menu, MenuPage),
    bl("menu_tick_type", Menu, MenuPage),
];

/// The ADJ Image Control item apply, then the ImageControlController (MENU
/// page) apply, event, and exit.
pub const IMAGE_CONTROL_PAGES: &[HookSpec] = &[
    b("menu_native_adj_apply", Menu, Tuning),
    b("menu_native_dialog_apply", Menu, Tuning),
    b("menu_native_page_event", Menu, Tuning),
    b("menu_native_page_exit", Menu, Tuning),
];

/// The `menu_settings_{count,draw,apply,current}_<page>` hooks of each
/// settings value page, page by page.
macro_rules! settings_values {
    ($($page:literal)*) => {
        &[$(
            b(concat!("menu_settings_count_", $page), Menu, MenuPage),
            b(concat!("menu_settings_draw_", $page), Menu, MenuPage),
            b(concat!("menu_settings_apply_", $page), Menu, MenuPage),
            b(concat!("menu_settings_current_", $page), Menu, MenuPage),
        )*]
    };
}

/// The Image Control tag, the parameter page, and the settings pages.
pub const MENU: &[HookSpec] = &join::<{ len(MENU_GROUPS) }>(MENU_GROUPS);

const MENU_GROUPS: &[&[HookSpec]] = &[MENU_PAGES, settings_values!(0 1 2 3 4)];

const MENU_PAGES: &[HookSpec] = &[
    // LiveViewController's Image Control widget: the BL to the icon mapper.
    bl("menu_viewfinder_icon", Menu, Tuning),
    // Image Control tag for GRKit photos: the playback DISP page icon, and the
    // playback parameter detail, which renders GRKit photos as Standard.
    bl("menu_playback_icon", Menu, Tuning),
    bl("menu_playback_detail", Menu, Tuning),
    // The native parameter page edits the Fn adjustments of a GRKit effect
    // (`runtime/menu.c`). ImageControlDetailController: its start method, the
    // BL to its kind reader, the two BLs to the snapshot loader, and its event
    // handler. Then the Image Control model getters and setters of the
    // Standard adjustments (saturation, hue, key, contrast, highlight, shadow,
    // sharpness, shading, clarity), and the BL to the settings getter in the
    // row text of each adjustment.
    b("menu_parameter_start", Menu, Tuning),
    bl("menu_parameter_kinds", Menu, Tuning),
    bl("menu_parameter_snapshot_0", Menu, Tuning),
    bl("menu_parameter_snapshot_1", Menu, Tuning),
    b("menu_parameter_event", Menu, Tuning),
    b("menu_parameter_get_0", Menu, Tuning),
    b("menu_parameter_get_1", Menu, Tuning),
    b("menu_parameter_get_2", Menu, Tuning),
    b("menu_parameter_get_3", Menu, Tuning),
    b("menu_parameter_get_4", Menu, Tuning),
    b("menu_parameter_get_5", Menu, Tuning),
    b("menu_parameter_get_6", Menu, Tuning),
    b("menu_parameter_get_7", Menu, Tuning),
    b("menu_parameter_get_8", Menu, Tuning),
    b("menu_parameter_set_0", Menu, Tuning),
    b("menu_parameter_set_1", Menu, Tuning),
    b("menu_parameter_set_2", Menu, Tuning),
    b("menu_parameter_set_3", Menu, Tuning),
    b("menu_parameter_set_4", Menu, Tuning),
    b("menu_parameter_set_5", Menu, Tuning),
    b("menu_parameter_set_6", Menu, Tuning),
    b("menu_parameter_set_7", Menu, Tuning),
    b("menu_parameter_set_8", Menu, Tuning),
    bl("menu_parameter_row_0", Menu, Tuning),
    bl("menu_parameter_row_1", Menu, Tuning),
    bl("menu_parameter_row_2", Menu, Tuning),
    bl("menu_parameter_row_3", Menu, Tuning),
    bl("menu_parameter_row_4", Menu, Tuning),
    bl("menu_parameter_row_5", Menu, Tuning),
    bl("menu_parameter_row_6", Menu, Tuning),
    bl("menu_parameter_row_7", Menu, Tuning),
    bl("menu_parameter_row_8", Menu, Tuning),
    // A GRKit root row opens a borrowed native MENU submenu as its settings
    // page (`runtime/menu_settings.h`, `Module.settings`). The Instant Review submenu
    // shows the rows, and the Display Time and Zoom Review value pages show
    // the values of the two settings. ChangeController (its entry) ends the
    // session at any other controller. The five MOVT of the submenu kind
    // table base load a private copy of the Instant Review entry instead.
    // Then the row count, row text, apply, and current-value methods of the
    // two value pages.
    b("menu_settings_change", Menu, MenuPage),
    // The update of the camera information page, which About GRKit shows.
    b("menu_info_update", Menu, MenuPage),
    // An options page shows checkboxes (`runtime/menu_settings.h`): the
    // controller factory gives it a second SelectMenuController, the list of
    // that controller gets checkbox rows, and its start does not hide their
    // checkboxes as radio dots.
    b("menu_check_factory", Menu, MenuPage),
    bl("menu_check_lines", Menu, MenuPage),
    bl("menu_check_dot", Menu, MenuPage),
    bl("menu_settings_kinds_r3_0", Menu, MenuPage),
    bl("menu_settings_kinds_r2", Menu, MenuPage),
    bl("menu_settings_kinds_r8", Menu, MenuPage),
    bl("menu_settings_kinds_r5", Menu, MenuPage),
    bl("menu_settings_kinds_r3_1", Menu, MenuPage),
    // The submenu shows up to `GRKIT_SUBMENU_ROWS` rows (`runtime/menu.S`):
    // the store of the fifth row type in its constructor, the two compares of
    // the rows to hide, the loads of the row kind and the row label in its
    // draw, and the load of the row controller at OK.
    bl("menu_settings_build", Menu, MenuPage),
    bl("menu_settings_hide_first", Menu, MenuPage),
    bl("menu_settings_hide_next", Menu, MenuPage),
    bl("menu_settings_row_kind", Menu, MenuPage),
    bl("menu_settings_label", Menu, MenuPage),
    bl("menu_settings_open", Menu, MenuPage),
];

/// Framing and Guides: LiveViewController update, exit, and destroy, then
/// Canvas::Draw entry and its child-draw BL. Then the still aspect property
/// handler, which replaces `ldrb r1, [r4]`, the requested capture aspect, so
/// a native 3:2 request records the Framing ratio.
pub const VIEWFINDER: &[HookSpec] = &[
    b("framing_update", Framing, ViewfinderOverlay),
    b("framing_exit", Framing, ViewfinderOverlay),
    b("framing_destroy", Framing, ViewfinderOverlay),
    b("framing_canvas", Framing, ViewfinderOverlay),
    bl("framing_children", Framing, ViewfinderOverlay),
    bl("framing_capture_request_hook", Framing, CaptureAspect),
];

/// The recorded picture geometry, then the effect extension and picture
/// processing.
pub const PICTURE: &[HookSpec] = &[
    // DNG-only DefaultCropOrigin and DefaultCropSize setters. JPEG output
    // keeps the capture geometry; RAW defaults use native 3:2 for Framing.
    bl("picture_dng_origin", Framing, CaptureAspect),
    bl("picture_dng_size", Framing, CaptureAspect),
    // GetResolution id 2 heights (L, then each smaller size or crop step),
    // then the id 2 top insets of the screennail and the thumbnail.
    bl("framing_height_0", Framing, CaptureAspect),
    bl("framing_height_1", Framing, CaptureAspect),
    bl("framing_height_2", Framing, CaptureAspect),
    bl("framing_height_3", Framing, CaptureAspect),
    bl("framing_screennail_inset", Framing, CaptureAspect),
    bl("framing_thumbnail_inset", Framing, CaptureAspect),
    // GetResolution id 1 widths (L, M, S, XS) and the XS height, then the
    // id 1 left inset of the screennail and the second store of the thumbnail
    // insets: 5:4 and 1:1 narrow the 4:3 capture aspect.
    bl("framing_width_0", Framing, CaptureAspect),
    bl("framing_width_1", Framing, CaptureAspect),
    bl("framing_width_2", Framing, CaptureAspect),
    bl("framing_width_3", Framing, CaptureAspect),
    bl("framing_narrow_height_3", Framing, CaptureAspect),
    bl("framing_narrow_screennail", Framing, CaptureAspect),
    bl("framing_narrow_thumbnail", Framing, CaptureAspect),
    // Playback size classifier and image aspect getter.
    bl("framing_classify", Framing, CaptureAspect),
    b("framing_image_aspect", Framing, CaptureAspect),
    // RAW development of a DNG (RawDevelopmentModel): the BL that turns the
    // recorded capture aspect into the initial development aspect, the BL
    // that turns the development aspect into the capture aspect that the
    // development sends to the pipeline, and the BL that does so for the
    // size of the developed photo.
    bl("framing_develop_aspect", Framing, CaptureAspect),
    bl("framing_develop_push", Framing, CaptureAspect),
    bl("framing_develop_size", Framing, CaptureAspect),
    // Resize of a photo in playback: the end of the getter of its size, which
    // the camera looks up in its resize size table, and the BL that stores a
    // size of that table as a resize target.
    bl("framing_resize_source", Framing, CaptureAspect),
    bl("framing_resize_target", Framing, CaptureAspect),
    // The entry of the aspect of an image size in playback edits (PlayDefs),
    // from which the resize job takes the GetResolution size of its target
    // and an edit the insets of its previews.
    b("framing_play_aspect", Framing, CaptureAspect),
    // Effect extension and picture processing (`runtime/picture.*`), linked
    // into the Framing region. The live-view diff before the gamma stage
    // flag; the YCbCrMatrix Make call to its calc, which applies the
    // post-curve matrix; the JpegEncodeMacroProcess::Execute entry, for the
    // date stamp before the encode; and the maker-note setter of tag 0x4f
    // (ImageTone, the effect code of the shot) and tag 0x247 (the Fn
    // adjustments of the shot).
    b("module_gamma_dirty", Framing, Tuning),
    bl("picture_post", Framing, Tuning),
    b("picture_encode_entry", Framing, PictureEncode),
    bl("picture_capture_tone", Framing, Tuning),
    bl("picture_capture_parameters", Framing, Tuning),
    // The JPEG header setup of a write and of an in-camera rewrite: the BL to
    // the APP1 Exif setup, after which the XMP segment with the name of the
    // look goes before APP7.
    bl("picture_jpeg_header", Framing, Tuning),
    bl("picture_jpeg_rewrite", Framing, Tuning),
    // RawDevelopmentModel: the BL that gives the initial Image Control of a
    // development, where the development takes the effect of its DNG and
    // develops its native aspect.
    bl_shared("picture_develop_tone", Framing, Tuning, CaptureAspect),
    // ScreenDevice::OnSetLevelAlarm, which sets the live-view highlight
    // warning: the `mov r5, #0xfe00` of its threshold, and the `mov r1, r6`
    // before its tail call, where r4 is the screen and r6 the mode.
    bl("framing_level_threshold", Framing, Tuning),
    bl("framing_level_mode", Framing, Tuning),
    // LiveViewController's live-view histogram update: the BL to the copy of
    // the 256 luma bins into its 128 bins, which the histogram view draws.
    bl("framing_histogram", Framing, Tuning),
];

/// The Image Control list of a RAW development lists the effects after the
/// native rows (`runtime/menu.c`). RawDevelopmentParameterSetController: the
/// entries of its row count, of the row to category and to value of the
/// model, of its row fill, of its current-row test, of the choice of a row,
/// of its list setup, and of its Fn test; the BL to the model restore at
/// Cancel; the BLs to the label of an Image Control in its subtitle and in
/// the subtitle of RawDevelopmentTopController; and the BL to the icon of an
/// Image Control in the row list of RawDevelopmentTopController.
/// The row count, the row fill, the current-row test, and the list setup
/// also serve the aspect list (`DEVELOP_ASPECT`).
pub const DEVELOP_TONE: &[HookSpec] = &[
    b_shared("menu_develop_count", Menu, Tuning, CaptureAspect),
    b("menu_develop_category", Menu, Tuning),
    b("menu_develop_value", Menu, Tuning),
    b_shared("menu_develop_row", Menu, Tuning, CaptureAspect),
    b_shared("menu_develop_current", Menu, Tuning, CaptureAspect),
    b("menu_develop_decide", Menu, Tuning),
    b_shared("menu_develop_open", Menu, Tuning, CaptureAspect),
    b("menu_develop_fn", Menu, Tuning),
    bl("menu_develop_restore", Menu, Tuning),
    bl("menu_develop_list_text", Menu, Tuning),
    bl("menu_develop_top_text", Menu, Tuning),
    bl("menu_develop_top_icon", Menu, Tuning),
];

/// The aspect list of a RAW development lists the Framing ratios after the
/// native rows (`runtime/menu.c`). The entries of the row to value of the
/// model and of the choice of a row in RawDevelopmentParameterSetController;
/// the BL to the model restore at Cancel; the BLs to the label of an aspect
/// in its subtitle and in the subtitle of RawDevelopmentTopController; and
/// the BL to the icon of an aspect in the row list of
/// RawDevelopmentTopController.
pub const DEVELOP_ASPECT: &[HookSpec] = &[
    b("menu_develop_aspect_value", Menu, CaptureAspect),
    b("menu_develop_aspect_decide", Menu, CaptureAspect),
    bl("menu_develop_aspect_restore", Menu, CaptureAspect),
    bl("menu_develop_aspect_list_text", Menu, CaptureAspect),
    bl("menu_develop_aspect_top_text", Menu, CaptureAspect),
    bl("menu_develop_aspect_top_icon", Menu, CaptureAspect),
];

/// The number of hooks in `groups`.
pub const fn len(groups: &[&[HookSpec]]) -> usize {
    let mut count = 0;
    let mut group = 0;
    while group < groups.len() {
        count += groups[group].len();
        group += 1;
    }
    count
}

/// The hooks of `groups` in order, as one list of `N = len(groups)` hooks.
pub const fn join<const N: usize>(groups: &[&[HookSpec]]) -> [HookSpec; N] {
    const UNSET: HookSpec = b("", Menu, MenuPage);
    let mut hooks = [UNSET; N];
    let mut count = 0;
    let mut group = 0;
    while group < groups.len() {
        let mut index = 0;
        while index < groups[group].len() {
            let hook = &groups[group][index];
            hooks[count] = HookSpec {
                symbol: hook.symbol,
                part: hook.part,
                point: hook.point,
                also: hook.also,
                link: hook.link,
            };
            count += 1;
            index += 1;
        }
        group += 1;
    }
    assert!(count == N, "The hook count of the groups is not N");
    hooks
}
