/* Generated from crates/grkit-core/src/abi.rs. Do not edit: change the tables
   there, then run GRKIT_UPDATE_GENERATED=1 cargo test -p grkit-core --test
   generated. */
#ifndef GRKIT_ABI_H
#define GRKIT_ABI_H

/* The private header: the start of the registry array. ARM code addresses it
   through the literal GRKIT_HEADER_REFERENCE, which the installer relocates. */
#define GRKIT_HEADER_REFERENCE 0x58012340
#define GRKIT_HOST_HEADER_SIZE 6168
/* The most module pages, layers, and plane filters that the header holds. */
#define GRKIT_MODULE_PAGES_MAX 16
#define GRKIT_MODULE_LAYERS_MAX 16
#define GRKIT_MODULE_FILTERS_MAX 8
#define GRKIT_INDICATORS_MAX 16
#define GRKIT_STATUS_ICONS_MAX 6
/* The recipes (runtime/recipes.h): the most recipes, the most recipes that are
   on, the most undo steps, and the words of one recipe. The recipes that are
   on come first. In the recipe file, word w of recipe n has the key
   GRKIT_RECIPE_KEY_BASE + w * GRKIT_RECIPE_KEY_STRIDE + n. The name of a
   recipe has GRKIT_RECIPE_NAME_WORDS words of four 7-bit characters, at most
   GRKIT_RECIPE_NAME_CHARS; word w of the name of recipe n has the key
   GRKIT_RECIPE_NAME_KEY_BASE + w * GRKIT_RECIPE_KEY_STRIDE + n. The words
   after the names have one key each from GRKIT_RECIPE_IMPORTED_KEY: the hash
   of the imported package library, the ID of recipe n at
   GRKIT_RECIPE_ID_KEY_BASE + n, the assignment of user mode u at
   GRKIT_RECIPE_DIAL_KEY_BASE + u (grkit_core::runtime::recipe_key). A recipe
   ID is a generated number, 1 to GRKIT_RECIPE_ID_MAX (runtime/recipe_id.h). A
   user mode follows (GRKIT_RECIPE_DIAL_FOLLOW), turns GRKit off
   (GRKIT_RECIPE_DIAL_OFF), or applies the recipe with its ID. The undo steps
   are not saved. */
#define GRKIT_RECIPE_SLOTS 64U
#define GRKIT_RECIPE_ON_MAX 40U
#define GRKIT_RECIPE_UNDO 5U
#define GRKIT_RECIPE_WORDS 5U
#define GRKIT_RECIPE_NAME_CHARS 16U
#define GRKIT_RECIPE_NAME_WORDS 4U
#define GRKIT_RECIPE_DIAL_POSITIONS 3U
/* The bit layout of the recipe words (grkit_core::recipes::layout): the index
   of each word, then for each field its _SHIFT and the _MASK of its value
   after the shift, and the bit of each flag. */
#define GRKIT_RECIPE_FN_WORD 1U
#define GRKIT_RECIPE_NATIVE_WORD 2U
#define GRKIT_RECIPE_EXTRA_WORD 3U
#define GRKIT_RECIPE_MODULE_WORD 4U
#define GRKIT_RECIPE_VALUE_BITS 29U
#define GRKIT_RECIPE_LOOK_MASK 3U
#define GRKIT_RECIPE_LOOK_KEPT 0U
#define GRKIT_RECIPE_LOOK_OFF 1U
#define GRKIT_RECIPE_LOOK_EFFECT 2U
#define GRKIT_RECIPE_GRAIN_HELD 4U
#define GRKIT_RECIPE_GRAIN_LEVEL_SHIFT 3U
#define GRKIT_RECIPE_GRAIN_LEVEL_MASK 7U
#define GRKIT_RECIPE_GRAIN_SIZE_SHIFT 6U
#define GRKIT_RECIPE_GRAIN_SIZE_MASK 3U
#define GRKIT_RECIPE_CODE_SHIFT 8U
#define GRKIT_RECIPE_CODE_MASK 0x00007fffU
#define GRKIT_RECIPE_NUMBER_SHIFT 23U
#define GRKIT_RECIPE_NUMBER_MAX 15U
#define GRKIT_RECIPE_USED 0x08000000U
#define GRKIT_RECIPE_HIDDEN 0x10000000U
#define GRKIT_RECIPE_TYPE_SHIFT 0U
#define GRKIT_RECIPE_TYPE_MASK 31U
#define GRKIT_RECIPE_TONE_SHIFT 5U
#define GRKIT_RECIPE_TONE_MASK 15U
#define GRKIT_RECIPE_INTENSITY_SHIFT 9U
#define GRKIT_RECIPE_INTENSITY_MASK 3U
#define GRKIT_RECIPE_FILTER_SHIFT 11U
#define GRKIT_RECIPE_FILTER_MASK 7U
#define GRKIT_RECIPE_MODE_SHIFT 14U
#define GRKIT_RECIPE_MODE_MASK 31U
#define GRKIT_RECIPE_BA_SHIFT 19U
#define GRKIT_RECIPE_GM_SHIFT 24U
#define GRKIT_RECIPE_SHIFT_MASK 31U
#define GRKIT_RECIPE_LOOK_BITS 0x00003fffU
#define GRKIT_RECIPE_MODE_BITS 0x1fffc000U
#define GRKIT_RECIPE_TEMPERATURE_MASK 0x000003ffU
#define GRKIT_RECIPE_CORRECTION_SHIFT 10U
#define GRKIT_RECIPE_CORRECTION_BITS 3U
#define GRKIT_RECIPE_CORRECTION_MASK 7U
#define GRKIT_RECIPE_NATIVE_GRAIN_SHIFT 22U
#define GRKIT_RECIPE_NATIVE_GRAIN_MASK 3U
#define GRKIT_RECIPE_NATIVE_GRAIN_SIZE_SHIFT 24U
#define GRKIT_RECIPE_NATIVE_GRAIN_SIZE_MASK 3U
#define GRKIT_RECIPE_NATIVE_GRAIN_BITS 0x03c00000U
#define GRKIT_RECIPE_SOLARIZATION_SHIFT 26U
#define GRKIT_RECIPE_SOLARIZATION_MASK 7U
#define GRKIT_RECIPE_TYPE_EXTRA_BITS 0x1fc00000U
#define GRKIT_RECIPE_MODULE_SETTING_BITS 3U
#define GRKIT_RECIPE_MODULE_VALUE_MASK 3U
#define GRKIT_RECIPE_EV_SHIFT 9U
#define GRKIT_RECIPE_EV_MASK 63U
#define GRKIT_RECIPE_EV_ZERO 31U
#define GRKIT_RECIPE_ISO_SHIFT 15U
#define GRKIT_RECIPE_ISO_MASK 63U
#define GRKIT_RECIPE_METERING_SHIFT 21U
#define GRKIT_RECIPE_METERING_MASK 7U
#define GRKIT_RECIPE_ND_SHIFT 24U
#define GRKIT_RECIPE_ND_MASK 3U
#define GRKIT_RECIPE_EXPOSURE_BITS 0x03fffe00U
#define GRKIT_RECIPE_OFF 0x10000000U
/* The module settings that a recipe holds
   (grkit_core::recipes::MODULE_SETTINGS), in the order of their bits in word
   4: GLOW.BLOOM, GLOW.HALATION, GLOW.HALATION_COLOR. Each is its state file
   ID, which finds it in the settings table. GRKIT_RECIPE_MODULE_COLOR_ONLY has
   bit i for a color-only setting, which GR IV Monochrome does not hold. */
#define GRKIT_RECIPE_MODULE_SETTINGS 3U
#define GRKIT_RECIPE_MODULE_SETTING_IDS 73603892U, 979977653U, 208924775U
#define GRKIT_RECIPE_MODULE_COLOR_ONLY 4U
#define GRKIT_RECIPE_KEY_BASE 991000000U
#define GRKIT_RECIPE_KEY_STRIDE 100U
#define GRKIT_RECIPE_NAME_KEY_BASE 991005000U
#define GRKIT_RECIPE_IMPORTED_KEY 991009000U
#define GRKIT_RECIPE_ID_KEY_BASE 991009001U
#define GRKIT_RECIPE_DIAL_KEY_BASE 991009065U
#define GRKIT_RECIPE_ID_MAX 536870909U
#define GRKIT_RECIPE_DIAL_FOLLOW 0U
#define GRKIT_RECIPE_DIAL_SAVED 536870910U
#define GRKIT_RECIPE_DIAL_OFF 536870911U
/* The settings that the camera saves in a user mode of the mode dial
   (runtime/mode_dial.h, grkit_core::abi::DIAL_SET_WORDS): GRKIT_DIAL_SET_WORDS
   words, the number of pairs + 1 (0 without a set), a word for each
   capability, the Fn adjustments and the Toning of the effect, then
   GRKIT_DIAL_SET_PAIRS pairs of a state file ID and a value. The effect word
   is 0 for Off, else GRKIT_DIAL_SET_EFFECT with the recipe code of the effect.
   In their file, word w of user mode u (0 for U1) has the key
   GRKIT_DIAL_SET_KEY_BASE + u * GRKIT_DIAL_SET_KEY_STRIDE + w, and a word is
   at most GRKIT_DIAL_SET_VALUE_MAX. */
#define GRKIT_DIAL_SET_PAIRS 93U
#define GRKIT_DIAL_SET_COUNT 0U
#define GRKIT_DIAL_SET_CORE 1U
#define GRKIT_DIAL_SET_FN 11U
#define GRKIT_DIAL_SET_TONING 12U
#define GRKIT_DIAL_SET_PAIR 13U
#define GRKIT_DIAL_SET_WORDS 199U
#define GRKIT_DIAL_SET_EFFECT 32768U
#define GRKIT_DIAL_SET_KEY_BASE 992000000U
#define GRKIT_DIAL_SET_KEY_STRIDE 1000U
#define GRKIT_DIAL_SET_VALUE_MAX 999999999U
#define GRKIT_MODULE_SETTINGS_MAX 160
/* The most modules that GRKit lists, and the words of the largest script
   array, which holds the module array. */
#define GRKIT_MODULES_MAX 10U
#define GRKIT_MODULE_ARRAY_WORDS 65536U
/* The row views of the borrowed native settings page: the ADJ Order page has a
   row for each listed module. Without a suffix, for the assembler. */
#define GRKIT_SUBMENU_ROWS 10
/* Icon slots: the host icons, then the effect icons, then the module tiles, in
   chunks of GRKIT_ICONS_PER_CHUNK (GRKIT_HOST_ICON_CHUNKS).
   GRKIT_ICON_MODULE_TILE(n) is the icon ID of module tile n. */
#define GRKIT_HOST_ICON_COUNT 6
#define GRKIT_ICONS_PER_CHUNK 40
#define GRKIT_MODULE_TILES_MAX 128
#define GRKIT_ICON_MODULE_TILE(n) (0x7000U+(n))
#define GRKIT_STATE_CHANGED(h) (++(h)[GRKIT_HOST_STATE_CHANGES/4])
#define GRKIT_RECIPES_CHANGED(h) (++(h)[GRKIT_HOST_RECIPE_CHANGES/4])
#define GRKIT_HOST_MAGIC_VALUE 0x314d5247
#define GRKIT_HOST_VERSION_VALUE 1
/* Number of Image Control records. */
#define GRKIT_HOST_COUNT 0
/* Address of record 0. The records follow the header without gaps. */
#define GRKIT_HOST_RECORDS 4
/* Selected record, or 0 for native processing. */
#define GRKIT_HOST_ACTIVE 8
/* The ScreenDevice of the last highlight warning setting
   (ScreenDevice::OnSetLevelAlarm), or 0 (runtime/framing.c). */
#define GRKIT_HOST_LEVEL_SCREEN 12
/* GRKIT_HOST_MAGIC_VALUE after the installer writes the header. */
#define GRKIT_HOST_MAGIC 16
/* GRKIT_HOST_VERSION_VALUE. */
#define GRKIT_HOST_VERSION 20
/* The mode of that setting: 0 while the warning is off. A change of the effect
   sets the warning again with this mode. */
#define GRKIT_HOST_LEVEL_MODE 24
/* Number of parameter builds. A diagnostic. */
#define GRKIT_HOST_PROCESSING_CALLS 28
/* ADJ controller that shows the private menu, or 0. */
#define GRKIT_HOST_MENU_CONTROLLER 32
/* Private menu level: 0 native, MENU_ROOT, or MENU_MODULE (runtime/modules.h). */
#define GRKIT_HOST_MENU_LEVEL 36
/* Highlighted row of the private menu. */
#define GRKIT_HOST_MENU_CURSOR 40
/* Address of the tail, an array of its own: the picture scratch, the menu
   text, then the live-view labels. INSTALL writes it. */
#define GRKIT_HOST_TAIL 44
/* Grain level, the capability CORE.GRAIN: 0 Off, 1 weak, 2 medium, 3 strong.
   The native grain stage of recorded pictures uses it instead of the native
   grain settings. */
#define GRKIT_HOST_GRAIN 48
/* Address of the module list of the Modules page (runtime/manage.h), or 0
   without a listed module. INSTALL writes it. */
#define GRKIT_HOST_MANAGE 52
/* Address of the state of the core services (runtime/services.h): the event
   subscribers, the timers, and the UI. INSTALL writes it, with the module
   array. */
#define GRKIT_HOST_SERVICES 56
/* Nonzero while the menu rebuilds the native list. The list callback and the
   native Image Control preview ignore the changes. */
#define GRKIT_HOST_MENU_REBUILDING 60
/* Native list position, saved when the private menu opens. */
#define GRKIT_HOST_MENU_LIST 64
#define GRKIT_HOST_MENU_LIST_WORDS 2
/* The module + 1 whose native settings page the menu opened (Module.settings,
   runtime/core.h), or 0. While it is set, the hooks of the borrowed native
   page show the settings of that module. A change to a controller outside the
   page clears it. */
#define GRKIT_HOST_SETTINGS 72
/* Number of ADJ events. GRKIT_HOST_MENU_EVENT_RING holds the last ones. */
#define GRKIT_HOST_MENU_EVENTS 76
/* GRKIT_HOST_STATE_CHANGES when the live view last took a redraw for it. A
   changed setting redraws the live view once (runtime/framing.c). */
#define GRKIT_HOST_FRAME_CHANGES 80
/* Icon of Off, or 0 for the native icon. */
#define GRKIT_HOST_OFF_ICON 84
/* How the private menu opened: 1 or 2 across an edge of the native slots, 3 by
   touch or on the return from the native parameters. */
#define GRKIT_HOST_MENU_EDGE 88
/* MENU was pressed, and its confirm event has not arrived yet. */
#define GRKIT_HOST_BACK_PENDING 92
/* Text descriptor of the ADJ title: (length + 1) | 0x100, then the address. */
#define GRKIT_HOST_MENU_TITLE 96
#define GRKIT_HOST_MENU_TITLE_WORDS 2
/* Text descriptor of the ADJ detail line. */
#define GRKIT_HOST_MENU_DETAIL 104
#define GRKIT_HOST_MENU_DETAIL_WORDS 2
/* The last ADJ events: kind | argument << 16. */
#define GRKIT_HOST_MENU_EVENT_RING 112
#define GRKIT_HOST_MENU_EVENT_RING_WORDS 8
/* The record whose adjustments the native parameter page edits, or 0. Fn in
   GRKit Image Control stores the record with bit 0 set, and the page start
   clears bit 0. A page that native ADJ opens clears the word. */
#define GRKIT_HOST_PARAMETERS 144
/* Number of adjustment changes. The live view rebuilds its stages when it
   changes. */
#define GRKIT_HOST_PARAMETER_CHANGES 148
/* GRKIT_HOST_PARAMETER_CHANGES at the last live-view build. */
#define GRKIT_HOST_LIVE_PARAMETER_CHANGES 152
/* Icon chunk addresses: chunk n holds icon slots 40 n to 40 n + 39. The host
   icons come first in chunk 0, then the effect icons. */
#define GRKIT_HOST_ICON_CHUNKS 156
#define GRKIT_HOST_ICON_CHUNKS_WORDS 5
/* Icon of the GRKit tile. */
#define GRKIT_HOST_GRKIT_ICON 176
/* Private toolbar cell, or 0. */
#define GRKIT_HOST_TOOLBAR_WIDGET 180
/* ADJ controller of the private toolbar cell. */
#define GRKIT_HOST_TOOLBAR_OWNER 184
/* Module that the private menu shows at MENU_MODULE. At the end of a private
   session it holds the module of the root row where the next session opens
   (menu_reopen). */
#define GRKIT_HOST_MODULE 188
/* Load cue: 0 none, 1 after INSTALL, then 2 + the local second of the day when
   the still live view first shows the cue. */
#define GRKIT_HOST_LOAD_CUE 192
/* Aspect id 2 variant for the photo shown in playback: 0 none (shooting uses
   the Framing ratio), 1 not 65:24, 2 65:24. */
#define GRKIT_HOST_FRAME_FILE 196
/* Record of the last live-view build: an owned record, or 0 for native
   processing. */
#define GRKIT_HOST_LIVE_RECORD 200
/* The capabilities in use: bit n is set when a loaded module uses capability n
   (GRKIT_CORE_SETTING_BASE + n). INSTALL writes it, and the resident loader
   restores only the values of these capabilities. */
#define GRKIT_HOST_CAPABILITIES 204
/* Framing ratio: 0 native, then the Framing rows. */
#define GRKIT_HOST_FRAME_RATIO 208
/* ADJ Framing page preview: 0 none, else the highlighted ratio + 1. */
#define GRKIT_HOST_FRAME_PREVIEW 212
/* Number of Framing paints. A diagnostic. */
#define GRKIT_HOST_FRAME_DRAWS 216
/* ADJ controller of the preview. Its canvas shows the preview. ADJ can make
   its canvas after its first refresh, so a draw reads the canvas from the
   controller. The word stays set until a full draw erases the mask. */
#define GRKIT_HOST_FRAME_PREVIEW_ADJ 220
/* Address of the effect list of the Effects page (runtime/effects.h), or 0
   without a listed effect. INSTALL writes it. */
#define GRKIT_HOST_EFFECTS 224
/* Live view controller that Framing draws on, or 0. */
#define GRKIT_HOST_FRAME_OWNER 228
/* Canvas of that controller. */
#define GRKIT_HOST_FRAME_CANVAS 232
/* Bitmap of the last Framing paint, or 0 when nothing is drawn. */
#define GRKIT_HOST_FRAME_BITMAP 236
/* Grain size, the capability CORE.GRAIN_SIZE: 0 Auto (the ISO of the shot), 1
   small, 2 medium, 3 large. */
#define GRKIT_HOST_GRAIN_SIZE 240
/* Live-view-only override: 0 none, 1 native processing, else an owned record. */
#define GRKIT_HOST_PREVIEW 244
/* The grain of the shot whose grain stage was set up last: 0 the native noise,
   else the size 1 small, 2 medium, 3 large. The GR III noise fill reads it
   (runtime/picture.c). */
#define GRKIT_HOST_GRAIN_SHOT 248
/* Return to GRKit Image Control after the native parameter page: 1 opening, 2
   away. 3: ADJ exited from GRKit, so the next ADJ opens GRKit. 4: MENU or OK
   on GRKit passed the event that closes ADJ to the native list. 5: INSTALL
   restored CORE.ADJ = 1, so the next ADJ that opens after INSTALL opens GRKit
   (runtime/menu.c). */
#define GRKIT_HOST_RETURN 252
/* CORE.ADJ: 1 when the next ADJ should open on GRKit, 0 for a native entry.
   Survives enter clearing HOST_RETURN, and STATE.TXT keeps it across power-off
   (runtime/menu.c). */
#define GRKIT_HOST_ADJ_OPEN 256
/* Number of changes of the saved selections: the effect, CORE.ASPECT,
   CORE.GRAIN, CORE.GRAIN_SIZE, CORE.ADJ, CORE.ADJ_MODULE, the Fn adjustments,
   and the module settings (GRKIT_STATE_CHANGED). The resident loader polls
   this word and HOST_RECIPE_CHANGES, and saves the state file after it
   changes. */
#define GRKIT_HOST_STATE_CHANGES 260
/* Number of module plane filters in GRKIT_HOST_MODULE_FILTERS. INSTALL writes
   it. */
#define GRKIT_HOST_MODULE_FILTER_COUNT 264
/* Address of the profile table (GRKIT_PROFILE in the tail) while the
   development profile runs, else 0. INSTALL writes it when the card has
   GRKIT/PROFILE or PROFILE.TXT. */
#define GRKIT_HOST_PROFILE 268
/* Two words for each module plane filter, in stage order: its apply function,
   and its setting handle in the low half with its stage in the high half. */
#define GRKIT_HOST_MODULE_FILTERS 272
#define GRKIT_HOST_MODULE_FILTERS_WORDS 16
/* The crop of the capture size hooks (runtime/framing.S): the Framing ratio
   where it applies in the native aspect (aspect_value), else 0. Each live-view
   update sets it, so a native aspect that the camera menu sets takes the stock
   sizes. */
#define GRKIT_HOST_FRAME_CROP 336
/* The listed modules that ADJ can show, a bit each, which ADJ Order lists
   (runtime/manage.h). The menu sets it. The text hooks read this word, so they
   do not walk the module pages. */
#define GRKIT_HOST_ADJ_MODULES 340
/* Address of the module settings table, or 0 (runtime/settings.h). INSTALL
   writes it. */
#define GRKIT_HOST_MODULE_SETTINGS 344
/* The ADJ preview of a module page: its setting handle plus 1 (0 for none),
   then the highlighted value. Module layers of that setting paint the
   highlighted value on the ADJ canvas. */
#define GRKIT_HOST_MODULE_PREVIEW 348
#define GRKIT_HOST_MODULE_PREVIEW_WORDS 2
/* Number of module pages in GRKIT_HOST_MODULE_PAGES. INSTALL writes it. */
#define GRKIT_HOST_MODULE_PAGE_COUNT 356
/* Three words for each module page (runtime/sdk/grkit.h), in load order: the
   address of its GrkitPage, its setting handle, and the global index of its
   tile 0 (GRKIT_ICON_MODULE_TILE). */
#define GRKIT_HOST_MODULE_PAGES 360
#define GRKIT_HOST_MODULE_PAGES_WORDS 48
/* The module page that the generic page callbacks of the menu serve. */
#define GRKIT_HOST_MODULE_CHOICE 552
/* Number of module layers in GRKIT_HOST_MODULE_LAYERS. INSTALL writes it. */
#define GRKIT_HOST_MODULE_LAYER_COUNT 556
/* Four words for each module layer, in painting order: its paint function,
   setting handle (GRKIT_NO_SETTING paints with value 1), flags in the low half
   with order in the high half, and an optional GrkitLayer descriptor with
   enabled conditions (0 for no conditions). */
#define GRKIT_HOST_MODULE_LAYERS 560
#define GRKIT_HOST_MODULE_LAYERS_WORDS 64
/* Key of the inputs of the last private curve composition in the gamma stage,
   or 0 (runtime/processing.h). */
#define GRKIT_HOST_GAMMA_KEY 816
/* The effect of the RAW development in playback: 1 for native processing, else
   an owned record. The start of a development sets the effect that its DNG
   records (runtime/picture.c), and its Image Control list changes it. The
   still builds of a development apply it instead of the selected effect
   (runtime/processing.h). */
#define GRKIT_HOST_DEVELOP 820
/* GRKIT_HOST_DEVELOP when the Image Control list of the development opened.
   Cancel restores it, as the list restores its native value. */
#define GRKIT_HOST_DEVELOP_SAVED 824
/* The native rows of that list. The effects follow them (runtime/menu.c). */
#define GRKIT_HOST_DEVELOP_ROWS 828
/* The text ID of the native Standard label of the development, which shows the
   name of a GRKit effect (GRKIT_HOST_DEVELOP_NAME) instead. */
#define GRKIT_HOST_DEVELOP_TEXT 832
/* The text descriptor of that name. */
#define GRKIT_HOST_DEVELOP_NAME 836
#define GRKIT_HOST_DEVELOP_NAME_WORDS 2
/* Number of registered viewfinder indicators. INSTALL writes it. */
#define GRKIT_HOST_INDICATOR_COUNT 844
/* Descriptor address and module tile base for each indicator, in module load
   order and declaration order (runtime/sdk/grkit.h). */
#define GRKIT_HOST_INDICATORS 848
#define GRKIT_HOST_INDICATORS_WORDS 32
/* Code address of an ABI 2.0/2.1 module that supplies CORE.TONE, or 0. Its
   legacy native icon applies only without an explicit slot owner. */
#define GRKIT_HOST_LEGACY_TONE_OWNER 976
/* CORE.ASPECT_MODE: 0 crops photos, 1 draws a composition frame only. */
#define GRKIT_HOST_ASPECT_MODE 980
/* Native aspect + 1 at the last live-view bind, or 0 before the first bind. */
#define GRKIT_HOST_FRAME_NATIVE 984
/* The Framing ratio of the RAW development in playback (a CORE.ASPECT value),
   or 0 for its native aspect. The start of a development clears it
   (runtime/picture.c), and the aspect list of the development sets it
   (runtime/menu.c). The development sends the capture aspect and the geometry
   of that ratio to the pipeline (runtime/framing.c). */
#define GRKIT_HOST_DEVELOP_RATIO 988
/* GRKIT_HOST_DEVELOP_RATIO when the aspect list of the development opened.
   Cancel restores it, as the list restores its native value. */
#define GRKIT_HOST_DEVELOP_RATIO_SAVED 992
/* The native rows of that list. The Framing ratios follow them. */
#define GRKIT_HOST_DEVELOP_ASPECT_ROWS 996
/* The text ID of the native aspect label that shows the name of the Framing
   ratio (GRKIT_HOST_DEVELOP_RATIO_NAME) instead. */
#define GRKIT_HOST_DEVELOP_RATIO_TEXT 1000
/* The text descriptor of that name. */
#define GRKIT_HOST_DEVELOP_RATIO_NAME 1004
#define GRKIT_HOST_DEVELOP_RATIO_NAME_WORDS 2
/* The effect record whose Fn adjustments the Standard parameter set of the
   development model holds, or 0 while the set holds its own values
   (runtime/develop.h). */
#define GRKIT_HOST_DEVELOP_FILLED 1012
/* The own values of that set while GRKIT_HOST_DEVELOP_FILLED is not 0: one
   byte for each Fn adjustment, in GRKIT_RECORD_PARAMETERS order. */
#define GRKIT_HOST_DEVELOP_NATIVE 1016
#define GRKIT_HOST_DEVELOP_NATIVE_WORDS 3
/* The effect record whose changed adjustments the development keeps while the
   set holds another one, or 0. The start of a development clears it. */
#define GRKIT_HOST_DEVELOP_CHANGED 1028
/* Those adjustments, as GRKIT_HOST_DEVELOP_NATIVE. */
#define GRKIT_HOST_DEVELOP_CHANGES 1032
#define GRKIT_HOST_DEVELOP_CHANGES_WORDS 3
/* The native Image Control type plus 1 when ADJ last opened, or 0 when it was
   not readable. A half-press on the native Image Control slot with another
   type selects that type, as OK does (runtime/menu.c). */
#define GRKIT_HOST_ADJ_IC_TYPE 1044
/* CORE.CROP: 0 Off, then a focal-length crop step (runtime/crop.h). */
#define GRKIT_HOST_CROP 1048
/* CORE.CROP_MODE: 0 crops photos where the camera can, 1 draws the frame only. */
#define GRKIT_HOST_CROP_MODE 1052
/* The native crop factor that the core set for CORE.CROP, as float bits, or 0
   while the camera keeps its own (runtime/crop.h). */
#define GRKIT_HOST_CROP_NATIVE 1056
/* The crop factor of the next shot, as float bits, where it is longer than the
   native factor of the camera (40 and 70 mm), or 0. */
#define GRKIT_HOST_CROP_SHOT 1060
/* The crop factor that the live view shows in place of the camera's, as float
   bits, or 0 for the camera's own (runtime/framing.S: crop_live). */
#define GRKIT_HOST_CROP_LIVE 1064
/* The zoom of GRKIT_HOST_CROP_LIVE (the factor times 8192), or 0, and the zoom
   of the camera's crop factor that the core set: the live-view window takes
   the first in place of the second (runtime/framing.S: crop_window). */
#define GRKIT_HOST_CROP_LIVE_ZOOM 1068
#define GRKIT_HOST_CROP_CAMERA_ZOOM 1072
/* The zoom of the last live-view window of a crop (the factor times 8192), or
   0 before one (runtime/framing.S: crop_window). */
#define GRKIT_HOST_CROP_LIVE_SEEN 1076
/* The tries to make the live view read its factor again: the count in the high
   byte, and the second of the last try plus 1 below it (runtime/crop.h:
   crop_live_retry). */
#define GRKIT_HOST_CROP_LIVE_RETRY 1080
/* GR IV: the back end of the last shot, whose queue completion the core can
   post when a crop shot finishes before the shutter release stops
   (runtime/crop_queue.S). */
#define GRKIT_HOST_CROP_QUEUE_OWNER 1084
/* GR IV: the queue completion flags of that shot, CROP_QUEUE_POSTED and
   CROP_QUEUE_RECOVERED. */
#define GRKIT_HOST_CROP_QUEUE_FLAGS 1088
/* The recipes (runtime/recipes.h): RECIPE_WORDS words for each of the
   RECIPE_SLOTS recipes. A recipe whose word 0 is 0 is empty. The resident
   loader saves each word that is not 0 in the recipe file
   (`capabilities::StateFile::Recipes`). */
#define GRKIT_HOST_RECIPES 1092
#define GRKIT_HOST_RECIPES_WORDS 320
/* The names of the recipes, RECIPE_NAME_WORDS words each (RECIPE_NAME_CHARS),
   right after HOST_RECIPES, which the loader saves after the recipe words. */
#define GRKIT_HOST_RECIPE_NAMES 2372
#define GRKIT_HOST_RECIPE_NAMES_WORDS 256
/* The hash of the recipe library of the package that INSTALL imported last
   (runtime/install_recipes.h), after the names: the loader saves it with the
   recipes, so a package imports once. */
#define GRKIT_HOST_RECIPE_IMPORTED 3396
/* The ID of each recipe, by slot, 1 to `runtime::RECIPE_ID_MAX`: a generated
   number, so that a recipe that the site moves to another card keeps an ID
   that no recipe of that card had. A reference to a recipe holds its ID, so
   that it stays valid when the recipes move. Each part that creates a recipe
   (the camera, INSTALL, the site) generates its ID. The loader saves the IDs
   after the import hash. */
#define GRKIT_HOST_RECIPE_IDS 3400
#define GRKIT_HOST_RECIPE_IDS_WORDS 64
/* What each user mode of the mode dial, U1 to U3, does
   (`runtime::RECIPE_DIAL_FOLLOW`, `RECIPE_DIAL_OFF`, or the ID of a recipe).
   An ID that no recipe has follows, as `RECIPE_DIAL_FOLLOW`. A generated ID is
   never one of these values, so a deleted recipe never gets the references of
   another. */
#define GRKIT_HOST_RECIPE_DIAL 3656
#define GRKIT_HOST_RECIPE_DIAL_WORDS 3
/* The count of changes of the recipes: the loader saves the recipe file after
   it changes (GRKIT_RECIPES_CHANGED). INSTALL sets 1 after an import. */
#define GRKIT_HOST_RECIPE_CHANGES 3668
/* The undo steps, the oldest first, RECIPE_WORDS words each. They are not
   saved: a start begins without them. */
#define GRKIT_HOST_RECIPE_UNDO 3672
#define GRKIT_HOST_RECIPE_UNDO_WORDS 25
/* Nonzero while the mode dial is on a user mode whose settings are temporary
   (runtime/mode_dial.h): the resident loader then does not save the main state
   file, so the file keeps the settings of the other positions. The recipe file
   still saves. */
#define GRKIT_HOST_STATE_HOLD 3772
/* The count of changes of the saved settings of the user modes
   (`HOST_DIAL_SETS`): the loader saves their file after it changes. */
#define GRKIT_HOST_DIAL_SET_CHANGES 3776
/* The settings that the camera saved in each user mode of the mode dial, U1 to
   U3, `DIAL_SET_WORDS` words each (runtime/mode_dial.h). A user mode applies
   them while it has `runtime::RECIPE_DIAL_SAVED` (`HOST_RECIPE_DIAL`). The
   resident loader saves each word that is not 0 in their file
   (`capabilities::StateFile::DialSets`), and INSTALL restores them. */
#define GRKIT_HOST_DIAL_SETS 3780
#define GRKIT_HOST_DIAL_SETS_WORDS 597

/* The capabilities of the core (grkit_core::capabilities): the handle of each
   CORE setting, and for each handle in order, its header word and its number
   of values. CORE.TONE has neither: its value is the active record, and the
   effects give its number of values (runtime/tone.h). */
#define GRKIT_CORE_SETTING_BASE 0x100U
#define GRKIT_CORE_SETTING_COUNT 10U
#define GRKIT_CORE_TONE 0x100U
#define GRKIT_CORE_TONE_VALUES 0U
#define GRKIT_CORE_TONE_BIT 0x1
#define GRKIT_CORE_ASPECT 0x101U
#define GRKIT_CORE_ASPECT_VALUES 8U
#define GRKIT_CORE_ASPECT_BIT 0x2
#define GRKIT_CORE_GRAIN 0x102U
#define GRKIT_CORE_GRAIN_VALUES 5U
#define GRKIT_CORE_GRAIN_BIT 0x4
#define GRKIT_CORE_GRAIN_SIZE 0x103U
#define GRKIT_CORE_GRAIN_SIZE_VALUES 4U
#define GRKIT_CORE_GRAIN_SIZE_BIT 0x8
#define GRKIT_CORE_ASPECT_MODE 0x104U
#define GRKIT_CORE_ASPECT_MODE_VALUES 2U
#define GRKIT_CORE_ASPECT_MODE_BIT 0x10
#define GRKIT_CORE_ADJ 0x105U
#define GRKIT_CORE_ADJ_VALUES 2U
#define GRKIT_CORE_ADJ_BIT 0x20
#define GRKIT_CORE_ADJ_MODULE 0x106U
#define GRKIT_CORE_ADJ_MODULE_VALUES 16U
#define GRKIT_CORE_ADJ_MODULE_BIT 0x40
#define GRKIT_CORE_CROP 0x107U
#define GRKIT_CORE_CROP_VALUES 5U
#define GRKIT_CORE_CROP_BIT 0x80
#define GRKIT_CORE_CROP_MODE 0x108U
#define GRKIT_CORE_CROP_MODE_VALUES 2U
#define GRKIT_CORE_CROP_MODE_BIT 0x100
#define GRKIT_CORE_RECIPE 0x109U
#define GRKIT_CORE_RECIPE_VALUES 0U
#define GRKIT_CORE_RECIPE_BIT 0x200
#define GRKIT_CORE_SETTING_WORDS 0,208,48,240,980,256,188,1048,1052,0
#define GRKIT_CORE_SETTING_VALUES 0,8,5,4,2,2,16,5,2,0

/* An Image Control record. The records follow the header. */
#define GRKIT_RECORD_SIZE 0x1100
/* The native processing record: a prefix, then the tone curves. */
#define GRKIT_RECORD_COEFFICIENTS 0x54
/* UTF-16 effect name with its NUL. */
#define GRKIT_RECORD_NAME 0x668
/* Nonzero for a monochrome effect. */
#define GRKIT_RECORD_MONOCHROME 0x690
/* Address of the effect icon. */
#define GRKIT_RECORD_ICON 0x694
/* Effect ID with its NUL. */
#define GRKIT_RECORD_ID 0x698
/* u16 ImageTone code recorded in photos: 0x8000 | hash of the effect ID. */
#define GRKIT_RECORD_CODE 0x6f8
/* GRF extension, zero for an effect without features. */
#define GRKIT_RECORD_FEATURES 0x700
/* i16[9] signed Q13 RGB matrix after the tone curves, applied through the
   YCbCr matrix. */
#define GRKIT_RECORD_POST 0x704
/* Selective color for recorded pictures: i16 cos and sin of the target hue
   (Q12), u16 inner and outer tangents toward smaller hue angles, then toward
   larger ones (Q8), and u8 chroma gate c0 and c1. Bytes 0x724..0x72b are
   reserved and zero. */
#define GRKIT_RECORD_SELECTIVE 0x716
/* Fn adjustments of the effect: saturation, hue, high/low key, contrast,
   highlight contrast, shadow contrast, sharpness, shading, clarity. Each byte
   is (native value - 4) mod 9, so zero is neutral. A native value is 0..8, and
   4 is neutral. */
#define GRKIT_RECORD_PARAMETERS 0x72c
/* The default of each Fn adjustment, in the order of RECORD_PARAMETERS:
   (default - 4) mod 9 of a native value 0..8, from the Fn defaults of the
   effect (feature 64), else zero. An adjustment's native value is (stored +
   default + 4) mod 9, so a stored zero is the default. */
#define GRKIT_RECORD_FN_DEFAULTS 0x735
/* The Toning of a native look whose red or blue curve differs from green, as a
   stored Fn byte: (shown value) mod 9, -4..4, 0 the look's own toning. The
   curves of the record hold the toned curves. */
#define GRKIT_RECORD_TONING 0x73e
/* 1 + the slot of the look's own red and blue curves in the tail
   (`TONE_CURVES`), or 0 for an effect without Toning. */
#define GRKIT_RECORD_TONE_SLOT 0x73f
/* The MultiAxial parameter set that INSTALL expands from the hue sectors of
   the GRF (feature 8): the address of the set at +0, the address of the
   highlight-correction set at +4, the set (six pointers) at +8, the bounds at
   +0x20, the bank selectors at +0x40, the matrix header at +0x58, the control
   fields at +0x6c, 60 3x3 i32 Q10 matrices at +0x98, the highlight-correction
   set at +0x908, and its control fields at +0x920. Zero without hue sectors. */
#define GRKIT_RECORD_MULTIAXIAL 0x740
/* The R, G, and B tone curves in the coefficients: 256 14-bit samples each,
   GRKIT_RECORD_CURVE_SIZE bytes apart. */
#define GRKIT_RECORD_CURVES 0x68
#define GRKIT_RECORD_CURVE_SIZE 0x200
/* A GRFX0001 effect file has GRKIT_GRF_SIZE bytes, or GRKIT_GRF_MAX_SIZE with
   hue levels (feature 16). Each byte that no field holds is zero. */
#define GRKIT_GRF_SIZE 8192U
#define GRKIT_GRF_MAX_SIZE 8704U
/* The magic `GRFX0001`. */
#define GRKIT_GRF_MAGIC 0U
#define GRKIT_GRF_MAGIC_SIZE 8U
/* u32 size of the file. */
#define GRKIT_GRF_BYTES 8U
#define GRKIT_GRF_BYTES_SIZE 4U
/* u32 CRC-32 of the file, which reads these bytes as zero. */
#define GRKIT_GRF_CRC 12U
#define GRKIT_GRF_CRC_SIZE 4U
/* The effect ID: NUL-terminated and zero-padded. */
#define GRKIT_GRF_ID 16U
#define GRKIT_GRF_ID_SIZE 32U
/* The effect name: NUL-terminated and zero-padded ASCII. */
#define GRKIT_GRF_NAME 48U
#define GRKIT_GRF_NAME_SIZE 32U
/* u32 flags: 1 for a monochrome effect. */
#define GRKIT_GRF_FLAGS 80U
#define GRKIT_GRF_FLAGS_SIZE 4U
/* u32 extension features (GRKIT_FEATURE_*), zero without the extension. */
#define GRKIT_GRF_FEATURES 84U
#define GRKIT_GRF_FEATURES_SIZE 4U
/* i16[9] signed Q13 RGB matrix after the tone curves (feature 1). */
#define GRKIT_GRF_POST 88U
#define GRKIT_GRF_POST_SIZE 18U
/* Selective color (feature 2): i16 cos and sin of the target hue, four u16
   tangents, and two u8 chroma limits. */
#define GRKIT_GRF_SELECTIVE 106U
#define GRKIT_GRF_SELECTIVE_SIZE 14U
/* Reserved and zero. */
#define GRKIT_GRF_RESERVED 120U
#define GRKIT_GRF_RESERVED_SIZE 8U
/* i16[9] signed Q13 color matrix. */
#define GRKIT_GRF_MATRIX 128U
#define GRKIT_GRF_MATRIX_SIZE 18U
/* Zero. */
#define GRKIT_GRF_PADDING 146U
#define GRKIT_GRF_PADDING_SIZE 2U
/* The R, G, and B tone curves: 256 14-bit u16 samples each. */
#define GRKIT_GRF_CURVES 148U
#define GRKIT_GRF_CURVES_SIZE 1536U
/* The 40 x 40 RGBA8 icon, with binary alpha. */
#define GRKIT_GRF_ICON 1684U
#define GRKIT_GRF_ICON_SIZE 6400U
/* Hue sectors of feature 8: 11 x 6 i8 coefficients (GRKIT_HUE_MATRICES bytes),
   then the u16 control fields 10..20 of the weight curve. */
#define GRKIT_GRF_HUE 8084U
#define GRKIT_GRF_HUE_SIZE 88U
#define GRKIT_GRF_ID_MAX 31U
#define GRKIT_GRF_NAME_MAX 16U
/* A GRKR0001 recipe library has GRKIT_RCP_HEADER_SIZE bytes, then
   GRKIT_RCP_RECORD_SIZE bytes for each of at most GRKIT_RCP_MAX recipes. */
#define GRKIT_RCP_HEADER_SIZE 32U
#define GRKIT_RCP_RECORD_SIZE 64U
#define GRKIT_RCP_MAX 64U
#define GRKIT_RCP_NAME_UNITS 16U
/* A user mode of the mode dial in the header (GRKIT_RCP_DIAL): keep the value
   of the camera, follow the dial, turn GRKit off, or apply record k at
   GRKIT_RCP_DIAL_RECORD + k. */
#define GRKIT_RCP_DIAL_KEEP 0U
#define GRKIT_RCP_DIAL_FOLLOW 1U
#define GRKIT_RCP_DIAL_OFF 2U
#define GRKIT_RCP_DIAL_RECORD 3U
/* The family byte of a record: the camera line whose INSTALL imports it. */
#define GRKIT_RCP_FAMILY_GR3 1U
#define GRKIT_RCP_FAMILY_GR4 2U
#define GRKIT_RCP_FAMILY_GR4_MONOCHROME 3U
/* The magic `GRKR0001`. */
#define GRKIT_RCP_MAGIC 0U
/* u32 size of the file. */
#define GRKIT_RCP_BYTES 8U
/* u32 CRC-32 of the file, which reads these bytes as zero. */
#define GRKIT_RCP_CRC 12U
/* u32 number of recipes. */
#define GRKIT_RCP_COUNT 16U
/* u32 for each user mode of the mode dial, U1 to U3: what INSTALL sets it to
   when it imports the library (`recipes::DialStep`). */
#define GRKIT_RCP_DIAL 20U
/* The name: UTF-16LE units, zero-padded, without a NUL at 16 units. */
#define GRKIT_RCP_RECORD_NAME 0U
/* The RECIPE_WORDS words of the recipe (runtime/recipes.h). */
#define GRKIT_RCP_RECORD_WORDS 32U
/* u8 family: 1 GR III, 2 GR IV, 3 GR IV Monochrome. */
#define GRKIT_RCP_RECORD_FAMILY 52U
#define GRKIT_FEATURE_POST 1U
#define GRKIT_FEATURE_SELECTIVE 2U
#define GRKIT_FEATURE_HUE 8U
#define GRKIT_FEATURE_HUE_LEVELS 16U
#define GRKIT_FEATURE_NATIVE_LOOK 32U
#define GRKIT_FEATURE_FN_DEFAULTS 64U
/* The default Fn adjustments of feature 64: 9 i8 (-4..4) in the order of
   GRKIT_RECORD_PARAMETERS, in a file of GRKIT_GRF_MAX_SIZE bytes. */
#define GRKIT_GRF_FN_DEFAULTS 8624U
#define GRKIT_GRF_FN_DEFAULTS_END 8633U
#define GRKIT_HUE_SIZE 88U
#define GRKIT_HUE_MATRICES 66U
/* Hue levels of feature 16: the sectors of levels 1 to 3 after the first
   GRKIT_GRF_SIZE bytes. */
#define GRKIT_HUE_LEVEL_OFFSET 8192U
#define GRKIT_HUE_LEVEL_SIZE 198U
/* The fixed breakpoints of the weight curve, control fields 18..20. */
#define GRKIT_HUE_BORDERS { 164, 655, 4095 }
/* The quadrant bounds and the bank of each segment: 11 equal sectors, sector k
   in bank k + 1. */
#define GRKIT_HUE_BOUNDS { 1490, 745, 0, 0, 1304, 559, 0, 0, 1862, 1118, 373, 0, 1676, 931, 187, 0 }
#define GRKIT_HUE_SELECTORS { 3, 2, 1, 0, 0, 11, 10, 9, 0, 0, 9, 8, 7, 6, 0, 6, 5, 4, 3, 0 }
/* Control fields 1..8 that spread the levels, and those for a capture with
   highlight correction, which halves the stage's input. */
#define GRKIT_HUE_LEVEL_CONTROL { 656, 656, 656, 586, 150, 650, 1150, 1650 }
#define GRKIT_HUE_LEVEL_CONTROL_HALVED { 1312, 1312, 1312, 1172, 75, 325, 575, 825 }
/* A native look (feature 32, grkit_core::native_look): 16 u16 bounds, 20 u8
   bank selectors, 21 u16 control fields, and 5 u16 level gains from
   GRKIT_GRF_HUE; and 12 x 6 i16 level-0 coefficients and 4 x 12 x 6 i8
   corrections of levels 1 to 4 from GRKIT_GRF_SIZE. */
#define GRKIT_NATIVE_BOUNDS 8084U
#define GRKIT_NATIVE_SELECTORS 8116U
#define GRKIT_NATIVE_CONTROL 8136U
#define GRKIT_NATIVE_GAINS 8178U
#define GRKIT_NATIVE_HEAD_END 8188U
#define GRKIT_NATIVE_MATRICES 8192U
#define GRKIT_NATIVE_CORRECTIONS 8336U
#define GRKIT_NATIVE_END 8624U
/* The plan that SCAN writes: this header, the GRF file of each installed
   effect, then an entry of GRKIT_PLANNED_EFFECT_SIZE bytes for each listed
   effect. */
#define GRKIT_PLAN_HEADER_SIZE 40
#define GRKIT_PLAN_MAGIC_VALUE 0x314d5247
#define GRKIT_PLAN_SAFE 1U
#define GRKIT_PLAN_PROFILE 2U
/* GRKIT_PLAN_MAGIC_VALUE. */
#define GRKIT_PLAN_MAGIC 0
/* Number of installed effects. */
#define GRKIT_PLAN_COUNT 4
/* Words of the registry array: the private header and the records. */
#define GRKIT_PLAN_HEADER_WORDS 8
/* Number of icon chunk arrays. */
#define GRKIT_PLAN_ICON_CHUNKS 12
/* Bytes of the plan. */
#define GRKIT_PLAN_SIZE 16
/* Words of the module array, or 0 without modules. */
#define GRKIT_PLAN_MODULE_WORDS 20
/* Artwork tiles of the usable modules. */
#define GRKIT_PLAN_MODULE_TILES 24
/* GRKIT_PLAN_SAFE and GRKIT_PLAN_PROFILE. */
#define GRKIT_PLAN_MODES 28
/* Effect files past the listed ones. */
#define GRKIT_PLAN_OVERFLOW 32
/* Number of listed effects. */
#define GRKIT_PLAN_LISTED 36
#define GRKIT_PLANNED_EFFECT_SIZE 92
/* The head of its GRF file, which holds its ID and name. */
#define GRKIT_PLANNED_HEAD 0
#define GRKIT_PLANNED_HEAD_WORDS 16
/* Its file name with its NUL. */
#define GRKIT_PLANNED_FILE 64
#define GRKIT_PLANNED_FILE_WORDS 4
/* Its status at this start (runtime/effects.h). */
#define GRKIT_PLANNED_STATUS 80
/* Its choice in the GRKit section of MENU. */
#define GRKIT_PLANNED_CHOICE 84
/* Its record, or EFFECTS_NO_RECORD. */
#define GRKIT_PLANNED_RECORD 88
/* The longest menu string, with its NUL: an Image Control summary is a prefix
   and a 16-character effect name. */
#define GRKIT_SUMMARY_UNITS 32

/* Picture scratch after the records: the date stamp core and halo masks, then
   the shot ring. */
#define GRKIT_PICTURE_MASKS 141376
/* Shot ring: the record of each recent non-live-view parameter build, keyed by
   its shot settings pointer, which the maker-note setter (+0x18) and the JPEG
   encode configuration (+0x34) also hold. A later shot can start processing
   before an earlier one is written, so a single "last build" word is not
   enough. Word 0 counts the writes, and word 1 (GRKIT_SHOT_LAST_TURNS) holds
   the turns of the last shot with a known posture. Entry n is at
   GRKIT_SHOT_ENTRY + GRKIT_SHOT_STRIDE n and holds the key at GRKIT_SHOT_KEY,
   the record at GRKIT_SHOT_RECORD, the posture of the electronic level at
   GRKIT_SHOT_POSTURE, and the effective capture crop at GRKIT_SHOT_FRAME_CROP
   (runtime/processing.h). */
#define GRKIT_SHOT_RING 141376
#define GRKIT_SHOT_ENTRIES 8
#define GRKIT_SHOT_ENTRY 8
#define GRKIT_SHOT_STRIDE 16
#define GRKIT_SHOT_KEY 0
#define GRKIT_SHOT_RECORD 4
#define GRKIT_SHOT_POSTURE 8
#define GRKIT_SHOT_FRAME_CROP 12
#define GRKIT_SHOT_LAST_TURNS 4
#define GRKIT_PICTURE_SCRATCH 141512

/* The layout of GrkitShot (runtime/sdk/grkit.h) that picture_shot fills:
   its size in bytes, the byte offset of each field, and the bits of
   `known`. tests/runtime/layout_contract.c checks them against the SDK. */
#define GRKIT_SHOT_INFO_BYTES 52
#define GRKIT_SHOT_INFO_SIZE 0
#define GRKIT_SHOT_INFO_KNOWN 4
#define GRKIT_SHOT_INFO_AV 8
#define GRKIT_SHOT_INFO_TV 12
#define GRKIT_SHOT_INFO_SV 16
#define GRKIT_SHOT_INFO_SCENE_EV 20
#define GRKIT_SHOT_INFO_COMPENSATION 24
#define GRKIT_SHOT_INFO_EXPOSURE_US 28
#define GRKIT_SHOT_INFO_F_NUMBER 32
#define GRKIT_SHOT_INFO_ISO 34
#define GRKIT_SHOT_INFO_FOCAL 36
#define GRKIT_SHOT_INFO_FOCAL_35MM 38
#define GRKIT_SHOT_INFO_PROGRAM 40
#define GRKIT_SHOT_INFO_METERING 41
#define GRKIT_SHOT_INFO_FLASH 42
#define GRKIT_SHOT_INFO_TAKEN 44
#define GRKIT_SHOT_INFO_KNOWN_EXPOSURE 1
#define GRKIT_SHOT_INFO_KNOWN_COMPENSATION 2
#define GRKIT_SHOT_INFO_KNOWN_FOCAL 4
#define GRKIT_SHOT_INFO_KNOWN_TAKEN 8
#define GRKIT_SHOT_INFO_KNOWN_FLASH 16
#define GRKIT_SHOT_INFO_KNOWN_PROGRAM 32
#define GRKIT_SHOT_INFO_KNOWN_METERING 64
#define GRKIT_SHOT_INFO_KNOWN_ALL 127

/* Menu text after the picture scratch, in each menu language: 0 English, 1
   Simplified Chinese, 2 Traditional Chinese, 3 Japanese, 4 Korean. The
   installer copies the label table of the build to offset 0: for each
   language, GRKIT_TEXT_STRINGS u16 byte offsets from the text start, then
   NUL-terminated UTF-16 strings. At GRKIT_TEXT_SUMMARY, the installer writes
   the Image Control summary of each record in each language,
   GRKIT_SUMMARY_UNITS units each, language-major. */
#define GRKIT_TEXT_SIZE 24576
/* Live-view labels after the menu text. The installer copies the label table
   of the build (grkit_core::artwork::overlay_labels): for label n, a u16
   width, a u16 height, and the u32 offset of its pixels from the table start
   at 8 n. Each pixel is one byte of text coverage, row by row. */
#define GRKIT_LABELS_SIZE 28672
/* The profile table after the labels (runtime/profile.h). For each of the
   first GRKIT_PROFILE_MODULES listed modules, at GRKIT_PROFILE_MODULE_SIZE n:
   its code start, its code end, and GRKIT_PROFILE_NAME_BYTES ASCII bytes of
   its name. At GRKIT_PROFILE_SLOTS, GRKIT_PROFILE_SLOT_COUNT slots of
   GRKIT_PROFILE_SLOT_SIZE bytes: calls, cycles, and the cycles of the largest
   call. The core points come first, then GRKIT_PROFILE_KINDS slots for each of
   those modules. */
#define GRKIT_PROFILE 194760
#define GRKIT_PROFILE_SIZE 500
#define GRKIT_PROFILE_MODULES 8
#define GRKIT_PROFILE_MODULE_SIZE 16
#define GRKIT_PROFILE_NAME_BYTES 8
#define GRKIT_PROFILE_SLOTS 128
#define GRKIT_PROFILE_SLOT_SIZE 12
#define GRKIT_PROFILE_SLOT_COUNT 31
#define GRKIT_PROFILE_MODULE_BASE 7
#define GRKIT_PROFILE_KINDS 3
#define GRKIT_PROFILE_FRAME_BIND 0
#define GRKIT_PROFILE_FRAME_PREPARE 1
#define GRKIT_PROFILE_FRAME_PAINT 2
#define GRKIT_PROFILE_PICTURE_POST 3
#define GRKIT_PROFILE_PICTURE_ENCODE 4
#define GRKIT_PROFILE_MENU_ICON 5
#define GRKIT_PROFILE_PICTURE_GRAIN 6
#define GRKIT_PROFILE_PAINT 0
#define GRKIT_PROFILE_FILTER 1
#define GRKIT_PROFILE_CALL 2
/* Everything after the records: the script array must hold it. */
#define GRKIT_TAIL_SIZE 199356
/* The own red and blue curves of the native looks with Toning: for each of
   GRKIT_TONE_SLOTS slots, 256 u16 red samples, then the blue ones. */
#define GRKIT_TONE_CURVES 195260
#define GRKIT_TONE_SLOTS 4
#define GRKIT_TONE_SLOT_SIZE 1024
#define GRKIT_TEXT_SUMMARY 8192
/* The parameter page title of a GRKit session, after the summaries: the page,
   its vtable, and the descriptor of the effect name. */
#define GRKIT_TEXT_PARAMETER_TITLE 24560
/* The native settings page of a GRKit session, before the parameter page
   title: the borrowed submenu entry, the text descriptors, and the views that
   show settings (runtime/menu.c). */
#define GRKIT_TEXT_SETTINGS 24048
/* The value of a group page in MENU and on the ADJ root, before the native
   settings page: the titles of the children that are on, joined by a slash,
   GRKIT_TEXT_GROUP_UNITS units with the NUL. */
#define GRKIT_TEXT_GROUP_VALUE 23536
#define GRKIT_TEXT_GROUP_UNITS 256
#define GRKIT_TEXT_LANGUAGES 5
#define GRKIT_TEXT_RECORDS 40
#define GRKIT_TEXT(h) ((h)[GRKIT_HOST_TAIL/4]+GRKIT_PICTURE_SCRATCH)
#define GRKIT_LABELS(h) (GRKIT_TEXT(h)+GRKIT_TEXT_SIZE)
#define GRKIT_LABEL_COUNT 18
#define GRKIT_MODULE_LABEL_COUNT 14
/* String IDs of the label table, in grkit_core::menu_text order. A run of IDs
   follows the rows or values that it names. */
#define TEXT_OFF 0
#define TEXT_IMAGE_OFF_SUMMARY 1
/* The Image Control summary before an effect name, for the installer. */
#define TEXT_IMAGE_PREFIX 2
/* The About row: the GRKit version. */
#define TEXT_VERSION 3
/* The Modules root row: as usual, and in safe mode. */
#define TEXT_MODULES_SUMMARY 4
/* The settings row that turns a module or an effect on or off. */
#define TEXT_ENABLE 6
#define TEXT_ON 7
/* The GRKit row of MENU in safe mode. */
#define TEXT_SAFE_MODE 8
/* The GRKit row of MENU after a change for the next start. */
#define TEXT_RESTART 9
/* The Image Control settings row that opens the Fn adjustments. */
#define TEXT_ADJUSTMENTS 10
/* The ADJ Order row of MENU. */
#define TEXT_ADJ 11
/* The settings row that lists the options of a module page. */
#define TEXT_OPTIONS 12
/* About GRKit: the title, the Version row and its value, the Project row and
   its value. */
#define TEXT_ABOUT 13
/* The Film Look settings row that selects the effect. */
#define TEXT_FILM 18
/* The guides of an options value page: OK turns an option on or off, MENU goes
   back. */
#define TEXT_GUIDE 19
/* The switch that stops each module at once: its ADJ summary while on, and
   while off, then its MENU row. */
#define TEXT_SWITCH 21
/* Between a group title and its value in the ADJ root summary. */
#define TEXT_SEPARATOR 24
/* The page of recipes: the Undo and Save rows, the text before a saved name,
   the text before and after the name to delete, the message when the recipes
   are full, the text of a missing effect, the name of a recipe without a look,
   the message of a value that the camera does not have, the message of an
   apply whose Grain has no effect, then the two buttons of Fn on a camera
   recipe: Rename and Delete, then the message of an apply whose Glow module is
   not loaded, and the mark after the name of a recipe whose effect is not
   installed, then the Apply row of the Recipes page of MENU. */
#define TEXT_RECIPE 25
/* The names of the native Image Control types, in the order of
   runtime/native.h. */
#define TEXT_NATIVE_TYPE 40
/* Enable at Start, which decides the switch at each start: its MENU row, then
   its Keep Last value. Its On and Off are TEXT_ON and TEXT_OFF. */
#define TEXT_START 61
/* The Mode Dial row of the Recipes page of MENU, then the values of a user
   mode besides a recipe: None, GRKit Off, and Saved Settings. */
#define TEXT_DIAL 63
#define GRKIT_TEXT_STRINGS 67

#endif
