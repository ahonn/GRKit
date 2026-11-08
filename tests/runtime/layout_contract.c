/* The module descriptor of the SDK (runtime/sdk/grkit.h) as the installer
   reads it (runtime/module_link.h), and the SDK copies of the core types.
   A test compiles this file for 32-bit ARM
   with -fsyntax-only, so each check holds for the layout of the camera. */
typedef unsigned char u8;
typedef unsigned int u32;
#include "module_link.h"
#include "grkit.h"

#define AT(type, field, offset) \
    _Static_assert(__builtin_offsetof(type, field) == (offset), #type "." #field " moved")
#define SIZE(type, size) _Static_assert(sizeof(type) == (size), #type " changed its size")

SIZE(GrkitModule, GRM_DESCRIPTOR_FLAGS);
AT(GrkitModule, size, 0);
AT(GrkitModule, abi, GRM_DESCRIPTOR_ABI_AT);
AT(GrkitModule, registrations, GRM_DESCRIPTOR_REGISTRATIONS_AT);
AT(GrkitModule, registration_count, GRM_DESCRIPTOR_REGISTRATION_COUNT_AT);
AT(GrkitModule, settings, GRM_DESCRIPTOR_SETTINGS_AT);
AT(GrkitModule, setting_count, GRM_DESCRIPTOR_SETTING_COUNT_AT);
AT(GrkitModule, uses, GRM_DESCRIPTOR_USES_AT);
AT(GrkitModule, use_count, GRM_DESCRIPTOR_USE_COUNT_AT);
AT(GrkitModule, tiles, GRM_DESCRIPTOR_TILES_AT);
AT(GrkitModule, tile_count, GRM_DESCRIPTOR_TILE_COUNT_AT);
AT(GrkitModule, flags, GRM_DESCRIPTOR_FLAGS_AT);
_Static_assert(GRKIT_MODULE_COLOR_ONLY == GRM_MODULE_COLOR_ONLY, "the module flags agree");

SIZE(GrkitRegistration, GRM_REGISTRATION);
AT(GrkitRegistration, point, 0);
AT(GrkitRegistration, impl, GRM_REGISTRATION_IMPL_AT);

SIZE(GrkitSetting, GRM_SETTING_DEFAULT);
AT(GrkitSetting, size, 0);
AT(GrkitSetting, key, GRM_SETTING_KEY_AT);
AT(GrkitSetting, values, GRM_SETTING_VALUES_AT);
AT(GrkitSetting, flags, GRM_SETTING_FLAGS_AT);
AT(GrkitSetting, default_value, GRM_SETTING_DEFAULT_AT);

SIZE(GrkitLayer, GRM_LAYER_CONDITIONAL);
AT(GrkitLayer, size, 0);
AT(GrkitLayer, flags, GRM_LAYER_FLAGS_AT);
AT(GrkitLayer, paint, GRM_LAYER_PAINT_AT);
AT(GrkitLayer, setting, GRM_LAYER_SETTING_AT);
AT(GrkitLayer, order, GRM_LAYER_ORDER_AT);
AT(GrkitLayer, condition_count, GRM_LAYER_CONDITION_COUNT_AT);
AT(GrkitLayer, conditions, GRM_LAYER_CONDITIONS_AT);

SIZE(GrkitPage, GRM_PAGE_FN);
AT(GrkitPage, size, 0);
AT(GrkitPage, kind, GRM_PAGE_KIND_AT);
AT(GrkitPage, setting, GRM_PAGE_SETTING_AT);
AT(GrkitPage, flags, GRM_PAGE_FLAGS_AT);
AT(GrkitPage, title, GRM_PAGE_TITLE_AT);
AT(GrkitPage, summaries, GRM_PAGE_SUMMARIES_AT);
AT(GrkitPage, labels, GRM_PAGE_LABELS_AT);
AT(GrkitPage, rows, GRM_PAGE_ROWS_AT);
AT(GrkitPage, setting_rows, GRM_PAGE_SETTING_ROWS_AT);
AT(GrkitPage, settings, GRM_PAGE_SETTINGS_AT);
AT(GrkitPage, children, GRM_PAGE_CHILDREN_AT);
AT(GrkitPage, icon, GRM_PAGE_ICON_AT);
AT(GrkitPage, fn_value, GRM_PAGE_FN_VALUE_AT);
AT(GrkitPage, fn_setting, GRM_PAGE_FN_SETTING_AT);

SIZE(GrkitIcon, GRM_ICON);
AT(GrkitIcon, kind, GRM_ICON_KIND_AT);
AT(GrkitIcon, setting, GRM_ICON_SETTING_AT);
AT(GrkitIcon, first_tile, GRM_ICON_FIRST_TILE_AT);
SIZE(GrkitCondition, GRM_CONDITION);
AT(GrkitCondition, setting, GRM_CONDITION_SETTING_AT);
AT(GrkitCondition, comparison, GRM_CONDITION_COMPARISON_AT);
AT(GrkitCondition, value, GRM_CONDITION_VALUE_AT);
SIZE(GrkitIndicator, GRM_INDICATOR);
AT(GrkitIndicator, placement, GRM_INDICATOR_PLACEMENT_AT);
AT(GrkitIndicator, slot, GRM_INDICATOR_SLOT_AT);
AT(GrkitIndicator, condition_count, GRM_INDICATOR_CONDITION_COUNT_AT);
AT(GrkitIndicator, conditions, GRM_INDICATOR_CONDITIONS_AT);
AT(GrkitIndicator, icon, GRM_INDICATOR_ICON_AT);

SIZE(GrkitSettingRow, GRM_ROW);
AT(GrkitSettingRow, size, 0);
AT(GrkitSettingRow, setting, GRM_ROW_SETTING_AT);
AT(GrkitSettingRow, label, GRM_ROW_LABEL_AT);
AT(GrkitSettingRow, values, GRM_ROW_VALUES_AT);

SIZE(GrkitPlaneFilter, GRM_FILTER);
AT(GrkitPlaneFilter, size, 0);
AT(GrkitPlaneFilter, stage, GRM_FILTER_STAGE_AT);
AT(GrkitPlaneFilter, setting, GRM_FILTER_SETTING_AT);
AT(GrkitPlaneFilter, apply, GRM_FILTER_APPLY_AT);

SIZE(GrkitSubscriber, GRM_SUBSCRIBER);
AT(GrkitSubscriber, size, 0);
AT(GrkitSubscriber, events, GRM_SUBSCRIBER_EVENTS_AT);
AT(GrkitSubscriber, handle, GRM_SUBSCRIBER_HANDLE_AT);

/* The context that the installer writes: the words before `uses`
   (CONTEXT_WORDS of the installer). */
AT(GrkitContext, settings, 0);
AT(GrkitContext, setting_count, 4);
AT(GrkitContext, tiles, 8);
AT(GrkitContext, uses, 12);

#define SAME(a, b) _Static_assert((a) == (b), #a " differs from " #b)
SAME(GRKIT_ABI_MAJOR, GRM_ABI_MAJOR);
SAME(GRKIT_ABI_MINOR, GRM_ABI_MINOR);
SAME(GRKIT_POINT_OVERLAY_LAYER, GRM_POINT_OVERLAY_LAYER);
SAME(GRKIT_POINT_MENU_PAGE, GRM_POINT_MENU_PAGE);
SAME(GRKIT_POINT_PICTURE_PLANE, GRM_POINT_PICTURE_PLANE);
SAME(GRKIT_POINT_EVENT, GRM_POINT_EVENT);
SAME(GRKIT_POINT_VIEWFINDER_INDICATOR, GRM_POINT_VIEWFINDER_INDICATOR);
SAME(GRKIT_ICON_TILE, GRM_ICON_TILE);
SAME(GRKIT_ICON_TONE, GRM_ICON_TONE);
SAME(GRKIT_ICON_LABEL, GRM_ICON_LABEL);
SAME(GRKIT_CONDITION_EQUALS, GRM_CONDITION_EQUALS);
SAME(GRKIT_CONDITION_NOT_EQUALS, GRM_CONDITION_NOT_EQUALS);
SAME(GRKIT_INDICATOR_NATIVE_SLOT, GRM_INDICATOR_NATIVE_SLOT);
SAME(GRKIT_INDICATOR_STATUS_ROW, GRM_INDICATOR_STATUS_ROW);
SAME(GRKIT_SLOT_IMAGE_CONTROL, GRM_SLOT_IMAGE_CONTROL);
SAME(GRKIT_EVENT_SYSTEM_READY | GRKIT_EVENT_SETTING_CHANGED, GRM_EVENTS);
SAME(GRKIT_PAGE_CHOICE, GRM_PAGE_CHOICE);
SAME(GRKIT_PAGE_SETTINGS, GRM_PAGE_SETTINGS);
SAME(GRKIT_CHOICE_OFF_ROW, GRM_CHOICE_OFF_ROW);
SAME(GRKIT_CHOICE_CORE_ROWS, GRM_CHOICE_CORE_ROWS);
SAME(GRKIT_CHOICE_OPTIONS, GRM_CHOICE_OPTIONS);
SAME(GRKIT_LAYER_FOLLOW_DISP, GRM_LAYER_FOLLOW_DISP);
SAME(GRKIT_LAYER_FULL_DISP, GRM_LAYER_FULL_DISP);
SAME(GRKIT_LAYER_LIVE_ONLY, GRM_LAYER_LIVE_ONLY);
SAME(GRKIT_STAGE_FRAME, GRM_STAGE_FRAME);
SAME(GRKIT_STAGE_MARK, GRM_STAGE_MARK);
SAME(GRKIT_SETTING_PUBLIC, GRM_SETTING_PUBLIC);
SAME(GRKIT_SETTING_SAVED, GRM_SETTING_SAVED);
SAME(GRKIT_NO_SETTING, GRM_NO_SETTING);
SAME(GRKIT_USE(0), GRM_USED);
SAME(GRKIT_LANGUAGES, GRM_LANGUAGES);
SAME(GRKIT_TILE_BYTES, GRM_TILE_BYTES);
SAME(GRKIT_SETTING_ROWS, GRM_SETTING_ROWS);
SAME(GRKIT_PAGE_GROUP, GRM_PAGE_GROUP);
SAME(GRKIT_PAGE_MENU_ONLY, GRM_PAGE_MENU_ONLY);
SAME(GRKIT_PAGE_CHILDREN, GRM_PAGE_CHILDREN);
SAME(GRKIT_SETTING_ROW_VALUES, GRM_SETTING_ROW_VALUES);

/* The SDK keeps its own copy of the overlay surface and the picture planes
   (runtime/sdk/grkit.h), because a module includes only the SDK. The core's
   types (runtime/core.h) come in under other names here, and each SDK field
   must sit where the core writes it. The SDK names the core's `record` of a
   PictureSet `reserved`. */
#define Surface CoreSurface
#define Plane CorePlane
#define PictureSet CorePictureSet
#define chroma_row core_chroma_row
#define overlay_label core_overlay_label
#include "core.h"
#undef Surface
#undef Plane
#undef PictureSet
#undef chroma_row
#undef overlay_label

#define LIKE(type, core, field) AT(core, field, __builtin_offsetof(type, field))
SIZE(CoreSurface, sizeof(Surface));
LIKE(Surface, CoreSurface, data);
LIKE(Surface, CoreSurface, stride);
LIKE(Surface, CoreSurface, area);
LIKE(Surface, CoreSurface, box);
SIZE(CorePlane, sizeof(Plane));
LIKE(Plane, CorePlane, y);
LIKE(Plane, CorePlane, c);
LIKE(Plane, CorePlane, width);
LIKE(Plane, CorePlane, height);
LIKE(Plane, CorePlane, pitch);
LIKE(Plane, CorePlane, half);
LIKE(Plane, CorePlane, left);
LIKE(Plane, CorePlane, top);
LIKE(Plane, CorePlane, right);
LIKE(Plane, CorePlane, bottom);
SIZE(CorePictureSet, sizeof(PictureSet));
LIKE(PictureSet, CorePictureSet, planes);
LIKE(PictureSet, CorePictureSet, count);
AT(CorePictureSet, record, __builtin_offsetof(PictureSet, reserved));
LIKE(PictureSet, CorePictureSet, mono);
LIKE(PictureSet, CorePictureSet, scratch);
LIKE(PictureSet, CorePictureSet, turns);
AT(CorePictureSet, shot, __builtin_offsetof(PictureSet, reserved_shot));
/* A plane filter may use all of the scratch that the core gives each call. */
SAME(GRKIT_PLANE_SCRATCH, GRKIT_PICTURE_MASKS);

/* GrkitShot: the bytes that picture_shot writes (GRKIT_SHOT_INFO_* in
   abi.h, runtime/picture.c) and the bits of `known`. */
SIZE(GrkitShot, GRKIT_SHOT_INFO_BYTES);
AT(GrkitShot, size, GRKIT_SHOT_INFO_SIZE);
AT(GrkitShot, known, GRKIT_SHOT_INFO_KNOWN);
AT(GrkitShot, av, GRKIT_SHOT_INFO_AV);
AT(GrkitShot, tv, GRKIT_SHOT_INFO_TV);
AT(GrkitShot, sv, GRKIT_SHOT_INFO_SV);
AT(GrkitShot, scene_ev, GRKIT_SHOT_INFO_SCENE_EV);
AT(GrkitShot, compensation, GRKIT_SHOT_INFO_COMPENSATION);
AT(GrkitShot, exposure_us, GRKIT_SHOT_INFO_EXPOSURE_US);
AT(GrkitShot, f_number, GRKIT_SHOT_INFO_F_NUMBER);
AT(GrkitShot, iso, GRKIT_SHOT_INFO_ISO);
AT(GrkitShot, focal, GRKIT_SHOT_INFO_FOCAL);
AT(GrkitShot, focal_35mm, GRKIT_SHOT_INFO_FOCAL_35MM);
AT(GrkitShot, program, GRKIT_SHOT_INFO_PROGRAM);
AT(GrkitShot, metering, GRKIT_SHOT_INFO_METERING);
AT(GrkitShot, flash, GRKIT_SHOT_INFO_FLASH);
AT(GrkitShot, taken, GRKIT_SHOT_INFO_TAKEN);
AT(GrkitShot, taken.month, GRKIT_SHOT_INFO_TAKEN + 2);
AT(GrkitShot, taken.second, GRKIT_SHOT_INFO_TAKEN + 6);
_Static_assert(GRKIT_SHOT_EXPOSURE == GRKIT_SHOT_INFO_KNOWN_EXPOSURE &&
               GRKIT_SHOT_COMPENSATION == GRKIT_SHOT_INFO_KNOWN_COMPENSATION &&
               GRKIT_SHOT_FOCAL == GRKIT_SHOT_INFO_KNOWN_FOCAL &&
               GRKIT_SHOT_TAKEN == GRKIT_SHOT_INFO_KNOWN_TAKEN &&
               GRKIT_SHOT_FLASH == GRKIT_SHOT_INFO_KNOWN_FLASH &&
               GRKIT_SHOT_PROGRAM == GRKIT_SHOT_INFO_KNOWN_PROGRAM &&
               GRKIT_SHOT_METERING == GRKIT_SHOT_INFO_KNOWN_METERING,
               "GRKIT_SHOT_* bits moved");
