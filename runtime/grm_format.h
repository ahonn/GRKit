#ifndef GRKIT_GRM_FORMAT_H
#define GRKIT_GRM_FORMAT_H
/* The module format (crates/grkit-core/src/grm.rs): the header of a module
   file and the layout of each descriptor struct of runtime/sdk/grkit.h, as
   32-bit ARM code has it. The installer reads modules with it
   (runtime/module_link.h), and the core reads module pages with it
   (runtime/modules.h). It has only definitions. */

#define GRM_MAGIC 0x4d4b5247U
#define GRM_FORMAT 1U
#define GRM_HEADER 64U
/* The header word of the order of the module (grkit_core::grm): its default
   place in ADJ and in MENU, at most GRM_MAX_ORDER. */
#define GRM_ORDER 56U
#define GRM_MAX_ORDER 0xffffU
#define GRM_DATA_LINK 0x10000000U
#define GRM_DATA_LOCATION 0x80000000U
#define GRM_MAX_SEGMENT 0x10000U
/* The ABI of the core (runtime/sdk/grkit.h). */
#define GRM_ABI_MAJOR 2U
#define GRM_ABI_MINOR 6U
#define GRM_POINT_OVERLAY_LAYER 1U
#define GRM_POINT_MENU_PAGE 2U
#define GRM_POINT_PICTURE_PLANE 3U
#define GRM_POINT_EVENT 4U
#define GRM_POINT_VIEWFINDER_INDICATOR 5U
/* The events of a subscriber (GRKIT_EVENT_*). */
#define GRM_EVENTS 3U
#define GRM_PAGE_CHOICE 1U
#define GRM_PAGE_SETTINGS 2U
#define GRM_PAGE_GROUP 3U
#define GRM_PAGE_MENU_ONLY 8U
#define GRM_PAGE_CHILDREN 4U
#define GRM_CHOICE_OFF_ROW 1U
#define GRM_CHOICE_CORE_ROWS 2U
#define GRM_CHOICE_OPTIONS 4U
#define GRM_LAYER_FOLLOW_DISP 1U
#define GRM_LAYER_FULL_DISP 2U
#define GRM_LAYER_LIVE_ONLY 4U
/* The largest layer order: the core sorts layers by order << 16. */
#define GRM_MAX_LAYER_ORDER 0xffffU
/* The menu languages, the tile size, the most tiles of one module, and the
   most rows of a choice page and UTF-16 units of one text. */
#define GRM_LANGUAGES 5U
#define GRM_TILE_BYTES 6400U
#define GRM_TILES 64U
#define GRM_PAGE_ROWS 64U
#define GRM_TEXT_UNITS 64U
/* The most rows of a native settings page (a page has 0 to this many; with
   1, Fn opens no settings page, runtime/menu.c), and the most values of each
   row. */
#define GRM_SETTING_ROWS 5U
#define GRM_SETTING_ROW_VALUES 8U
/* The stages of plane filters (runtime/sdk/grkit.h). */
#define GRM_STAGE_FRAME 20U
#define GRM_STAGE_MARK 30U
#define GRM_STAGE_PRINT 40U
/* The most registrations that one descriptor gives. */
#define GRM_REGISTRATIONS 16U
/* The descriptor: size, ABI, registrations and their count, settings and
   their count, used settings and their count, tiles and their count
   (runtime/sdk/grkit.h). */
#define GRM_DESCRIPTOR 40U
/* The layout of each descriptor struct of runtime/sdk/grkit.h, as 32-bit ARM
   code has it (tests/runtime/layout_contract.c checks each against the
   struct). Word 0 of each is its size, which is at least GRM_<STRUCT>. */
#define GRM_DESCRIPTOR_ABI_AT 4U
#define GRM_DESCRIPTOR_REGISTRATIONS_AT 8U
#define GRM_DESCRIPTOR_REGISTRATION_COUNT_AT 12U
#define GRM_DESCRIPTOR_SETTINGS_AT 16U
#define GRM_DESCRIPTOR_SETTING_COUNT_AT 20U
#define GRM_DESCRIPTOR_USES_AT 24U
#define GRM_DESCRIPTOR_USE_COUNT_AT 28U
#define GRM_DESCRIPTOR_TILES_AT 32U
#define GRM_DESCRIPTOR_TILE_COUNT_AT 36U
/* ABI 2.4 appends the flags of the module: a descriptor of GRM_DESCRIPTOR_FLAGS
   bytes or more has them. */
#define GRM_DESCRIPTOR_FLAGS_AT 40U
#define GRM_DESCRIPTOR_FLAGS 44U
#define GRM_MODULE_COLOR_ONLY 1U
/* GrkitRegistration (no size word). */
#define GRM_REGISTRATION 8U
#define GRM_REGISTRATION_IMPL_AT 4U
/* GrkitSetting. */
#define GRM_SETTING 16U /* ABI 2.0 minimum */
#define GRM_SETTING_DEFAULT 20U
#define GRM_SETTING_KEY_AT 4U
#define GRM_SETTING_VALUES_AT 8U
#define GRM_SETTING_FLAGS_AT 12U
#define GRM_SETTING_DEFAULT_AT 16U
/* GrkitLayer. */
#define GRM_LAYER 20U /* ABI 2.0 minimum */
#define GRM_LAYER_CONDITIONAL 28U
#define GRM_LAYER_FLAGS_AT 4U
#define GRM_LAYER_PAINT_AT 8U
#define GRM_LAYER_SETTING_AT 12U
#define GRM_LAYER_ORDER_AT 16U
#define GRM_LAYER_CONDITION_COUNT_AT 20U
#define GRM_LAYER_CONDITIONS_AT 24U
/* The icon shared by an indicator and an ADJ child. */
#define GRM_ICON 16U
#define GRM_ICON_KIND_AT 4U
#define GRM_ICON_SETTING_AT 8U
#define GRM_ICON_FIRST_TILE_AT 12U
#define GRM_ICON_TILE 1U
#define GRM_ICON_TONE 2U
/* A core live-view label, for a status-row indicator (GRKIT_ICON_LABEL), and
   the number of core labels that a module can name
   (GRKIT_MODULE_LABEL_COUNT). */
#define GRM_ICON_LABEL 3U
#define GRM_LABELS 14U
#define GRM_CONDITION 12U
#define GRM_CONDITION_SETTING_AT 0U
#define GRM_CONDITION_COMPARISON_AT 4U
#define GRM_CONDITION_VALUE_AT 8U
#define GRM_CONDITION_EQUALS 1U
#define GRM_CONDITION_NOT_EQUALS 2U
#define GRM_CONDITIONS_MAX 4U
#define GRM_INDICATOR 24U
#define GRM_INDICATOR_PLACEMENT_AT 4U
#define GRM_INDICATOR_SLOT_AT 8U
#define GRM_INDICATOR_CONDITION_COUNT_AT 12U
#define GRM_INDICATOR_CONDITIONS_AT 16U
#define GRM_INDICATOR_ICON_AT 20U
#define GRM_INDICATOR_NATIVE_SLOT 1U
#define GRM_INDICATOR_STATUS_ROW 2U
#define GRM_SLOT_IMAGE_CONTROL 1U
/* GrkitPage. */
#define GRM_PAGE 40U
#define GRM_GROUP_PAGE 44U
#define GRM_PAGE_CHILDREN_AT 40U
#define GRM_PAGE_ICON 48U
#define GRM_PAGE_ICON_AT 44U
/* A page with the Fn setting of a row (ABI 2.5). */
#define GRM_PAGE_FN 56U
#define GRM_PAGE_FN_VALUE_AT 48U
#define GRM_PAGE_FN_SETTING_AT 52U
#define GRM_PAGE_KIND_AT 4U
#define GRM_PAGE_SETTING_AT 8U
#define GRM_PAGE_FLAGS_AT 12U
#define GRM_PAGE_TITLE_AT 16U
#define GRM_PAGE_SUMMARIES_AT 20U
#define GRM_PAGE_LABELS_AT 24U
#define GRM_PAGE_ROWS_AT 28U
#define GRM_PAGE_SETTING_ROWS_AT 32U
#define GRM_PAGE_SETTINGS_AT 36U
/* GrkitSettingRow. */
#define GRM_ROW 16U
#define GRM_ROW_SETTING_AT 4U
#define GRM_ROW_LABEL_AT 8U
#define GRM_ROW_VALUES_AT 12U
/* GrkitPlaneFilter. */
#define GRM_FILTER 16U
#define GRM_FILTER_STAGE_AT 4U
#define GRM_FILTER_SETTING_AT 8U
#define GRM_FILTER_APPLY_AT 12U
/* GrkitSubscriber. */
#define GRM_SUBSCRIBER 12U
#define GRM_SUBSCRIBER_EVENTS_AT 4U
#define GRM_SUBSCRIBER_HANDLE_AT 8U
/* The most settings and used settings of one module, the longest setting key
   with its NUL, and the most values of a setting. */
#define GRM_SETTINGS 16U
#define GRM_USES 8U
#define GRM_KEY 16U
#define GRM_SETTING_VALUES 65536U
/* A full setting name: module ID, '.', key, NUL. */
#define GRM_NAME (8U + 1U + GRM_KEY)
#define GRM_SETTING_PUBLIC 1U
#define GRM_SETTING_SAVED 2U
/* The rows of the setting show only on a camera with a color sensor (ABI 2.4). */
#define GRM_SETTING_COLOR_ONLY 4U
/* A used setting that is optional: its name starts with '?'. Its handle is
   GRM_NO_SETTING when no module has it. */
#define GRM_NO_SETTING 0xffffffffU
/* A page or a layer names setting n of the module, or GRM_USED | n for used
   setting n. */
#define GRM_USED 0x80000000U

#endif
