/* Generated from crates/grkit-core/src/abi.rs. Do not edit: change the tables
   there, then run GRKIT_UPDATE_GENERATED=1 cargo test -p grkit-core --test
   generated. */
#ifndef GRKIT_ABI_H
#define GRKIT_ABI_H

/* The private header: the start of the registry array. ARM code addresses it
   through the literal GRKIT_HEADER_REFERENCE, which the installer relocates. */
#define GRKIT_HEADER_REFERENCE 0x58012340
#define GRKIT_HOST_HEADER_SIZE 768
/* The most module pages, layers, and plane filters that the header holds. */
#define GRKIT_MODULE_PAGES_MAX 10
#define GRKIT_MODULE_LAYERS_MAX 16
#define GRKIT_MODULE_FILTERS_MAX 8
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
#define GRKIT_HOST_ICON_COUNT 4
#define GRKIT_ICONS_PER_CHUNK 40
#define GRKIT_MODULE_TILES_MAX 128
#define GRKIT_ICON_MODULE_TILE(n) (0x7000U+(n))
#define GRKIT_STATE_CHANGED(h) (++(h)[GRKIT_HOST_STATE_CHANGES/4])
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
   on GRKit passed the event that closes ADJ to the native list. */
#define GRKIT_HOST_RETURN 252
/* Number of changes of the saved selections: the effect, CORE.ASPECT,
   CORE.GRAIN, CORE.GRAIN_SIZE, the Fn adjustments, and the module settings
   (GRKIT_STATE_CHANGED). The resident loader polls only this word and saves
   the state file after it changes. */
#define GRKIT_HOST_STATE_CHANGES 256
/* Number of module plane filters in GRKIT_HOST_MODULE_FILTERS. INSTALL writes
   it. */
#define GRKIT_HOST_MODULE_FILTER_COUNT 260
/* Address of the profile table (GRKIT_PROFILE in the tail) while the
   development profile runs, else 0. INSTALL writes it when the card has
   GRKIT/PROFILE or PROFILE.TXT. */
#define GRKIT_HOST_PROFILE 264
/* Two words for each module plane filter, in stage order: its apply function,
   and its setting handle in the low half with its stage in the high half. */
#define GRKIT_HOST_MODULE_FILTERS 268
#define GRKIT_HOST_MODULE_FILTERS_WORDS 16
/* The crop of the capture size hooks (runtime/framing.S): the Framing ratio
   where it applies in the native aspect (aspect_value), else 0. Each live-view
   update sets it, so a native aspect that the camera menu sets takes the stock
   sizes. */
#define GRKIT_HOST_FRAME_CROP 332
/* The listed modules that ADJ can show, a bit each, which ADJ Order lists
   (runtime/manage.h). The menu sets it. The text hooks read this word, so they
   do not walk the module pages. */
#define GRKIT_HOST_ADJ_MODULES 336
/* Reserved for later fields. */
#define GRKIT_HOST_RESERVED 340
#define GRKIT_HOST_RESERVED_WORDS 22
/* Address of the module settings table, or 0 (runtime/settings.h). INSTALL
   writes it. */
#define GRKIT_HOST_MODULE_SETTINGS 428
/* The ADJ preview of a module page: its setting handle plus 1 (0 for none),
   then the highlighted value. Module layers of that setting paint the
   highlighted value on the ADJ canvas. */
#define GRKIT_HOST_MODULE_PREVIEW 432
#define GRKIT_HOST_MODULE_PREVIEW_WORDS 2
/* Number of module pages in GRKIT_HOST_MODULE_PAGES. INSTALL writes it. */
#define GRKIT_HOST_MODULE_PAGE_COUNT 440
/* Three words for each module page (runtime/sdk/grkit.h), in load order: the
   address of its GrkitPage, its setting handle, and the global index of its
   tile 0 (GRKIT_ICON_MODULE_TILE). */
#define GRKIT_HOST_MODULE_PAGES 444
#define GRKIT_HOST_MODULE_PAGES_WORDS 30
/* The module page that the generic page callbacks of the menu serve. */
#define GRKIT_HOST_MODULE_CHOICE 564
/* Number of module layers in GRKIT_HOST_MODULE_LAYERS. INSTALL writes it. */
#define GRKIT_HOST_MODULE_LAYER_COUNT 568
/* Three words for each module layer, in painting order: its paint function,
   its setting handle (GRKIT_NO_SETTING paints with the value 1), and its flags
   in the low half with its order in the high half. */
#define GRKIT_HOST_MODULE_LAYERS 572
#define GRKIT_HOST_MODULE_LAYERS_WORDS 48
/* Key of the inputs of the last private curve composition in the gamma stage,
   or 0 (runtime/processing.h). */
#define GRKIT_HOST_GAMMA_KEY 764

/* The capabilities of the core (grkit_core::capabilities): the handle of each
   CORE setting, and for each handle in order, its header word and its number
   of values. CORE.TONE has neither: its value is the active record, and the
   effects give its number of values (runtime/tone.h). */
#define GRKIT_CORE_SETTING_BASE 0x100U
#define GRKIT_CORE_SETTING_COUNT 4U
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
#define GRKIT_CORE_SETTING_WORDS 0,208,48,240
#define GRKIT_CORE_SETTING_VALUES 0,8,5,4

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
#define GRKIT_FEATURE_POST 1U
#define GRKIT_FEATURE_SELECTIVE 2U
#define GRKIT_FEATURE_HUE 8U
#define GRKIT_FEATURE_HUE_LEVELS 16U
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
   the record at GRKIT_SHOT_RECORD, and the posture of the electronic level at
   GRKIT_SHOT_POSTURE (runtime/processing.h). */
#define GRKIT_SHOT_RING 141376
#define GRKIT_SHOT_ENTRIES 8
#define GRKIT_SHOT_ENTRY 8
#define GRKIT_SHOT_STRIDE 16
#define GRKIT_SHOT_KEY 0
#define GRKIT_SHOT_RECORD 4
#define GRKIT_SHOT_POSTURE 8
#define GRKIT_SHOT_LAST_TURNS 4
#define GRKIT_PICTURE_SCRATCH 141512

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
#define GRKIT_LABELS_SIZE 16384
/* The profile table after the labels (runtime/profile.h). For each of the
   first GRKIT_PROFILE_MODULES listed modules, at GRKIT_PROFILE_MODULE_SIZE n:
   its code start, its code end, and GRKIT_PROFILE_NAME_BYTES ASCII bytes of
   its name. At GRKIT_PROFILE_SLOTS, GRKIT_PROFILE_SLOT_COUNT slots of
   GRKIT_PROFILE_SLOT_SIZE bytes: calls, cycles, and the cycles of the largest
   call. The core points come first, then GRKIT_PROFILE_KINDS slots for each of
   those modules. */
#define GRKIT_PROFILE 182472
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
#define GRKIT_TAIL_SIZE 182972
#define GRKIT_TEXT_SUMMARY 8192
/* The parameter page title of a GRKit session, after the summaries: the page,
   its vtable, and the descriptor of the effect name. */
#define GRKIT_TEXT_PARAMETER_TITLE 24560
/* The native settings page of a GRKit session, before the parameter page
   title: the borrowed submenu entry, the text descriptors, and the views that
   show settings (runtime/menu.c). */
#define GRKIT_TEXT_SETTINGS 24048
#define GRKIT_TEXT_LANGUAGES 5
#define GRKIT_TEXT_RECORDS 40
#define GRKIT_TEXT(h) ((h)[GRKIT_HOST_TAIL/4]+GRKIT_PICTURE_SCRATCH)
#define GRKIT_LABELS(h) (GRKIT_TEXT(h)+GRKIT_TEXT_SIZE)
#define GRKIT_LABEL_COUNT 9
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
#define GRKIT_TEXT_STRINGS 21

#endif
