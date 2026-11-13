/* RICOH GR III family: the object layouts, events, and constants that each
   GR III firmware build shares. A build header (gr3-<version>.h) defines the
   firmware addresses (FW_*) and the values that differ in its build, then
   includes this file. Comments here name no firmware address, because each
   build has its own; the build header gives them.
   A build that changes a value of this file defines it in its build header,
   and the value moves there for each build of the family. */
#ifndef GRKIT_GR3_H
#define GRKIT_GR3_H

/* Icon slots hold an 8-byte descriptor, then 40 x 40 RGBA. */
#define GRKIT_ICON_STRIDE 6408

/* Settings domain (FW_SETTINGS_DOMAIN): +0x518 -> control -> payload with the
   native Image Control type at SETTINGS_IC_TYPE. Standard is type 11. */
#define SETTINGS_IC_TYPE 0x77c
/* The native monochrome types, as a bit mask of the type: 13 to 16 are
   Monotone, Soft Monotone, Hard Monotone, and Hi-Contrast B&W (ImageTone 258
   to 261). The shot settings hold the type at the same offset as the
   settings payload. */
#define NATIVE_MONOCHROME_TYPES 0x0001e000U
/* The digital filter values of the shot settings, which the grain stage of
   recorded pictures reads: Shading and Clarity (0 to 8, 4 is neutral),
   Particle (0 is no grain), and the HDR Tone strength (0 is off). The native
   parameter build writes them for its Image Control type. */
#define SETTINGS_SHADING 0x793
#define SETTINGS_CLARITY 0x794
#define SETTINGS_GRAIN 0x795
#define SETTINGS_HDR 0x796

/* The list's row vector (first and end pointers) and its shown row count. */
#define LIST_ROWS 0xf4
#define LIST_COUNT 0x108

/* AdjModeController. The slot type of the native Image Control slot, and the
   controller of its Standard parameter page (still mode). */
#define ADJ_IC_SLOT_TYPE 12
#define IC_PARAMETERS_CONTROLLER 0x2000014
/* The shooting screen: a half-press on a page changes to it, and the release
   changes back to the page. */
#define SHOOTING_CONTROLLER 0x8000000
/* The native screens of ui_open_native (runtime/ui.h): the targets of the
   MENU rows White Balance (still) and LCD. */
#define UI_NATIVE_WHITE_BALANCE 0x200004eU
#define UI_NATIVE_LCD 0x2000018U
/* ImageControlDetailController, the parameter page. Its start method sets the
   type (+0x198), the set kind (+0x199: 0 the base set, 1 and 2 Custom), and
   the base kind of its row table (+0x19a: 0 Standard). The Image Control
   model keeps a snapshot of the adjustments from +3, one byte for each row
   kind, and a cancel writes the snapshot back through the model setters. */
/* The main title view of the page. UpdateViewMainTitle sets its text from the type. */
#define DETAIL_TITLE 0x15c
#define DETAIL_TYPE 0x198
#define DETAIL_SET_KIND 0x199
#define DETAIL_BASE_KIND 0x19a
#define MODEL_SNAPSHOT 3
/* The row kinds of the Standard adjustments, in GRKIT_RECORD_PARAMETERS order. */
#define PARAMETER_KINDS {0,1,2,3,4,5,6,13,14}
/* The last two of them, Shading and Clarity, for assembly (runtime/processing.h). */
#define PARAMETER_KIND_SHADING 13
#define PARAMETER_KIND_CLARITY 14
/* Toning (runtime/toning.h): base kind 9 of the row lists, the rows of
   Retro, adds Toning, row kind 9, to the Standard rows. Its model getter and
   setter and row read hook as adjustment 9. The page keeps the Standard type:
   the Retro type also sets its own Shading. */
#define PARAMETER_TONING_BASE 9
#define PARAMETER_TONING_KIND 9
/* The page event that opens another controller (0x2000015) from the page. */
#define DETAIL_EV_SUBPAGE 0x88

/* The native settings page of a GRKit root row (runtime/menu_settings.h). It
   borrows the MENU submenu Instant Review (controller 0x20000be, entry 10 of
   the submenu kind table FW_SUBMENU_KINDS), its three value pages (0x20000c3
   Display Time, 0x20000c4 Zoom Review, 0x20000c5), and two value pages of
   the MENU Setup tab (0x200009f, 0x20000a0), one for each row. Each is a
   plain SelectMenu page: its record has one title and no description. A
   switch in the SelectMenu start maps the page to its record; records 83, 84,
   85, 87, 88. A kind table entry holds the row count at +6, the row label IDs
   (u16) at +8, the row controllers (u32) at +0x14, a kind for each row (u32,
   0x21c for a row with a value page) at +0x28, and the row value methods (C++
   member pointers, 8 bytes) at +0x3c. A row without its kind draws its value
   over its label.
   A value method gets the submenu and the row. A value page shows the label
   ID of its SelectMenu record as title, which is the label of its row. */
#define SUBMENU_KIND_SIZE 0x64
#define SUBMENU_KIND_ROWS 6
#define SUBMENU_KIND_LABELS 8
#define SUBMENU_KIND_CONTROLLERS 0x14
#define SUBMENU_KIND_ROW_KINDS 0x28
#define SUBMENU_KIND_VALUES 0x3c
#define SETTINGS_SUBMENU 0x20000beU
#define SETTINGS_SUBMENU_KIND 10
#define SETTINGS_PAGES {0x20000c3U,0x20000c4U,0x20000c5U,0x200009fU,0x20000a0U}
/* The text IDs of the submenu title and of the title of each value page,
   which are also the row labels. The sixth row, which has no value page of
   its own, shows a label of another submenu. */
#define SETTINGS_TITLE_TEXT 0x1e7
#define SETTINGS_ROW_TEXTS {0x1e9,0x1eb,0x16e,0x194,0x1ed,0x1b0}
/* Options pages show checkboxes (runtime/menu_settings.h): the checked and
   unchecked images. A value page row sets the string view at ROW_TEXT_VIEW,
   and a submenu row its value view at ROW_VALUE_VIEW. */
#define CHECK_IMAGE_ON 0x10fU
#define CHECK_IMAGE_OFF 0x10eU
#define ROW_TEXT_VIEW 0xf8
#define ROW_VALUE_VIEW 0xfc
/* The label view of a submenu row, which its label setter sets. */
#define ROW_LABEL_VIEW 0xec

/* Rows of modules in the native MENU (runtime/menu_section.h). MENU builds its
   tabs at the first MENU of a start from the tab descriptors in the RAM
   table FW_MENU_TABS; entry MENU_TAB_GRKIT is the Customize tab. A
   descriptor holds the row count (u8) at MENU_TAB_ROWS, then for each of
   MENU_TAB_MAX_ROWS rows its filler (a C++ member pointer), its target
   controller, and its predicate (a member pointer, 0 for a row that always
   shows). A filler gets the row view and sets its label view (+0xec) and
   value view with the row setters. */
/* The MENU tab of the GRKit section (Customize, C), and the sections of a
   tab descriptor: their count, title text IDs, and first rows. The Setup
   tab has all MENU_TAB_MAX_SECTIONS sections; Customize has 3. */
#define MENU_TAB_GRKIT 3
#define MENU_TAB_SECTIONS 0x68
#define MENU_TAB_TITLES 0x6a
#define MENU_TAB_STARTS 0x80
#define MENU_TAB_MAX_SECTIONS 11
/* The TopMenu instance in the controller factory (FW_CONTROLLER_FACTORY). */
#define MENU_TOP_SLOT 0x74
#define MENU_TAB_SIZE 0x384
#define MENU_TAB_ROWS 0x8b
#define MENU_TAB_FILLERS 0x8c
#define MENU_TAB_TARGETS 0x1bc
#define MENU_TAB_PREDICATES 0x254
#define MENU_TAB_MAX_ROWS 38
/* The rows of the stock Customize tab. */
#define MENU_TAB_STOCK_ROWS 22
#define MENU_ROW_LABEL_VIEW 0xec
#define MENU_ROW_VALUE_VIEW 0xf8
/* A native view keeps its frame {x, y, width, height} at VIEW_FRAME, and its
   vtable sets it at VIEW_SET_FRAME. The value of a MENU or submenu row shows
   in a column of VALUE_COLUMN pixels from the x of the view. */
#define VIEW_FRAME 0x20
#define VIEW_SET_FRAME 0x18
#define VALUE_COLUMN 140
/* The camera information page (Setup tab, Firmware Info/Options), which
   About GRKit opens: its controller, the text ID of its title, and its two
   lines, ASCII string views that the page update sets (menu_info_update)
   with FW_STRING_VIEW_SET_ASCII(view, text). */
#define INFO_CONTROLLER 0x200011aU
#define INFO_TITLE_TEXT 0x28d
#define INFO_LINES {0x168,0x16c}

/* AdjModeController fields. */
#define ADJ_CANVAS 0x4
/* The UI language byte of the UI settings object (UI_SETTINGS_SLOT): 18 is
   Traditional Chinese and 19 Simplified Chinese. */
#define LANGUAGE_SETTING 0x9ba
/* The text ID of the first Snap focus distance, 0.3 m, then 0.4, 1, 1.5, 2,
   2.5, 3.5, and 5 m (runtime/distance.h). */
#define SNAP_DISTANCE_TEXT 62
/* The icons 1, 3, and 10 of the manual focus scale (runtime/distance.h). */
#define SCALE_ICON_ONE 0x83
#define SCALE_ICON_THREE 0x86
#define SCALE_ICON_TEN 0x84
/* The live-view display mode that DISP cycles, in the same settings object:
   0 full, 1 simple, 2 no information, 3 display off. */
#define DISPLAY_MODE_SETTING 0x912
#define ADJ_TITLE 0x160
#define ADJ_DETAIL 0x164
#define ADJ_DETAIL_VALUE 0x168
#define ADJ_CELLS 0x16c
#define ADJ_FRAME 0x180
#define ADJ_ICONS 0x184
#define ADJ_FN_GUIDE 0x1ac
/* The Fn hint of the Image Control slot (FW_GUIDE_SET_CONTENT): the Fn label,
   layout 10, and the settings icon. */
#define FN_GUIDE_TEXT 0x1cf
#define FN_GUIDE_ICON 0x233
#define ADJ_LIST 0x1f0
#define ADJ_SLOT 0x1f4
/* 1 in movie mode. */
#define ADJ_MOVIE_FIELD 0x270
#define ADJ_MOVIE(controller) BYTE(controller,ADJ_MOVIE_FIELD)
/* 1 while the held Fn action of an ADJ slot (type 13) runs. */
#define ADJ_FN_HELD 0x279

/* AdjModeController events (the first halfword of an event). */
#define EV_SWITCH 5
#define EV_SWITCH_END 8
/* A half-press of the shutter on a MENU value page, before the confirm that
   OK also sends. */
#define EV_HALF_PRESS 2
#define EV_FN 0x1b
/* An ADJ press, before the confirm that OK also sends. */
#define EV_ADJ_PRESS 0x1e
/* The argument of an Fn press. ADJ opens the native parameters on it. */
#define FN_PRESS_ARG 2
#define EV_TOUCH 0x30
#define EV_LEFT 0xce
#define EV_RIGHT 0xcf
#define EV_UP 0xd0
#define EV_DOWN 0xd1
#define EV_DIAL_UP 0xd4
#define EV_DIAL_DOWN 0xd5
#define EV_CONFIRM 0xd8

/* The playback metadata (FW_PLAYBACK_METADATA): +0x220 holds key 0x25, the
   Image Control tag. */
#define METADATA_TONE 0x220
/* The native Off icon, used until the installer provides the GRKit one. */
#define NATIVE_OFF_ICON() WORD(FW_ICON_TABLE,0x166*4)
/* The time of the photo in the playback metadata. */
#define METADATA_TIME 0x3c0
/* The EXIF Orientation of the photo in the playback metadata. */
#define METADATA_ORIENTATION 0x2b0
/* RAW development, the Fn adjustments (runtime/develop.h). RawDevelopmentModel
   holds the selected Image Control: the category at DEVELOP_CATEGORY and the
   value of each category from DEVELOP_VALUES. Each Image Control has a
   parameter set of a byte for each parameter kind; that of Standard is at
   DEVELOP_PARAMETERS. The parameter page of the development edits the set of
   the selected Image Control. */
#define DEVELOP_CATEGORY 0x82
#define DEVELOP_VALUES 0x83
#define DEVELOP_PARAMETERS 0x97
/* RAW development, the Image Control list and the aspect list
   (runtime/menu.c): the controller ID word, the ID of each list, and the
   first instruction of the row value of the model, which its entry hook
   replaces. */
#define CONTROLLER_ID 0x124
#define DEVELOP_TONE_LIST 0x4000025U
#define DEVELOP_ASPECT_LIST 0x4000022U
#define DEVELOP_ASPECT_VALUE_FIRST "ldrb r3, [r0, #5]"
/* The list view of RawDevelopmentParameterSetController, and the style and
   the height of its row views. */
#define DEVELOP_LIST_VIEW 0x168
#define DEVELOP_ROW_STYLE 0x1aU
#define DEVELOP_ROW_HEIGHT 0x3aU
/* The first instruction of the row category, which its entry hook replaces.
   The subtitle of the list is at +0x160 of the controller (r4 in
   UpdateSubTitle), the subtitle of RawDevelopmentTopController in r6. */
#define DEVELOP_CATEGORY_FIRST "ldrb r3, [r0, #5]"
#define DEVELOP_LIST_TITLE ldr r3, [r4, #0x160]
#define DEVELOP_TOP_TITLE mov r3, r6

/* LiveViewController: nonzero in movie mode, and the LCD canvas. */
#define LIVEVIEW_MOVIE 0x2d2
#define LIVEVIEW_CANVAS 0x15c
/* LiveViewController: the byte that makes UpdateLevelAlarm set the
   highlight warning again (FW_LIVEVIEW_LEVEL_UPDATE). */
#define LIVEVIEW_LEVEL_PENDING 0x4df
/* The posture word of the electronic level: 1 landscape, 2 upside down, 3
   and 4 the two vertical postures, 5 and 6 the lens down or up
   (runtime/picture.c: picture_turns). */
#define LEVEL_POSTURE 0x7c
/* Picture processing. GAMMA_BUILT: the word of the gamma object that is 1
   after the native curves are built. */
#define GAMMA_BUILT 0x3230
/* GAMMA_UPLOAD: the byte of the gamma object that makes the table build use
   the composed curves. */
#define GAMMA_UPLOAD 0x320a
/* DigitalFilterProcess plan: the grain stage flag that the stage gate sets. */
#define GRAIN_GATE_FLAG 0x24
/* Grain size. The shot settings hold the float APEX Sv of the shot at
   GRAIN_SV, which the EXIF ISO also uses. The granulation keeps the pitch and
   the height of its noise plane at fp - GRAIN_FILL_PITCH and
   fp - GRAIN_FILL_HEIGHT when it calls the noise fill FW_GRAIN_NOISE. */
#define GRAIN_SV 0xb68
#define GRAIN_FILL_PITCH 0x650
#define GRAIN_FILL_HEIGHT 0x644
/* The system counter behind the generic timer of the cores runs at 40 MHz
   (the GR III device tree and kernel log). Only Linux writes CNTFRQ, so the
   RTOS cores read 0 there. */
#define SYSTEM_COUNTER_MHZ 40U

/* The icons that the native sliders use (runtime/ui.h: ui_slider). */
#define UI_SLIDER_ICONS {0x230,0x40,0xc7,0x240}
/* The keyboard of the camera (runtime/ui.h): the controller ID of its
   copyright use, which the Copyright Information page opens.
   SoftwareKeyboardController sets its use from the controller ID at its
   start, in the byte KEYBOARD_FUNCTION; the copyright use is
   KEYBOARD_COPYRIGHT. */
#define KEYBOARD_CONTROLLER 0x2000105U
#define KEYBOARD_FUNCTION 0x1f0
#define KEYBOARD_COPYRIGHT 2U

/* Framing: the rows of every family (runtime/firmware.h). GR III has the
   native aspects 3:2 and 1:1. Framing captures each ratio through a hidden
   aspect id from native 3:2, so the live view stays in 3:2. 5:4 and 1:1 use
   the 4:3 id with narrower widths. */
#define FRAMING_RATIO_ROWS 0,1,4,2,3,6,7,5
/* Requested capture StillAspectRatio (CommonProperty +0x7b6). The property
   setter writes the next copy at [property+4]; readers see the current copy
   at [property] only after a later commit. Comparing the next copy sends
   each change once. */
#define FRAMING_CAPTURE_ASPECT() BYTE(WORD(WORD(WORD(((u32 (*)(void))FW_CAPTURE_PROPERTIES)(),0),0x518),4),0x7b6)
/* Native StillAspectRatio setter with the unchanged 3:2 UI value. It sends
   capture property 0xd4 again and posts no UI change event. */
#define FRAMING_REQUEST_ASPECT() (((u32 (*)(u32,u32))FW_STILL_ASPECT_SET)(FRAMING_SETTINGS(),0))
/* Resend the native StillAspectRatio value, as a native menu change does.
   The Framing capture hook maps a 3:2 request to the Framing ratio. */
#define MENU_FRAMING_REQUEST() do { u32 s=CALL0(FW_SETTINGS)(); CALL2(FW_STILL_ASPECT_SET)(s,CALL1(FW_STILL_ASPECT_GET)(s)); } while(0)
/* The native StillAspectRatio, and its setter as the native menu calls it:
   a change posts the UI change event and switches the live view. */
#define MENU_NATIVE_ASPECT() CALL1(FW_STILL_ASPECT_GET)(CALL0(FW_SETTINGS)())
/* GR IIIx runs this runtime with its 40 mm lens: the crops of CORE.CROP
   are 50, 57, 71, and 100 mm there (runtime/crop.h). */
#ifndef CROP_LENS_40MM
#define CROP_LENS_40MM() ((((u32 (*)(void))FW_PROJECT_TYPE)()&0xffU)==0)
#endif
#define MENU_SET_NATIVE_ASPECT(id) CALL2(FW_STILL_ASPECT_SET)(CALL0(FW_SETTINGS)(),(id))
/* GR III picks the live-view mode of the native aspect when the live view
   starts, so a native aspect change needs a stopped live view, as the native
   MENU has it. ADJ keeps the live view running. Stop it as a controller that
   needs no live view does: with an empty completion callback (std::function,
   16 bytes, no manager). Closing ADJ returns to the live-view controller,
   which starts the live view again in the mode of the new aspect. */
#define MENU_STOP_LIVE_VIEW() do { \
    u32 done[4]={0,0,0,0}, model=CALL0(FW_SHOOTING_MODEL)(); \
    CALL2(FW_LIVE_VIEW_END)(model,(u32)done); \
    CALL2(FW_LIVE_VIEW_STOP)(model,(u32)done); \
} while(0)
/* GetResolution id 2: the stock 16:9 height, the 65:24 height, and the 2.35:1
   height of each size. The heights are width x 24 / 65 and width / 2.35,
   rounded to a multiple of 8. */
#define XPAN_HEIGHT_0 3376, 2216, 2552
#define XPAN_HEIGHT_1 2704, 1776, 2040
#define XPAN_HEIGHT_2 1888, 1240, 1432
#define XPAN_HEIGHT_3 1080, 712, 816
/* The 65:24 sizes, then the 2.35:1 sizes (width, height), largest first. */
#define XPAN_SIZES 6000, 2216, 4800, 1776, 3360, 1240, 1920, 712
#define SCOPE_SIZES 6000, 2552, 4800, 2040, 3360, 1432, 1920, 816
/* GetResolution id 1: the stock 4:3 width, the 5:4 width (height x 5 / 4),
   the 1:1 width, and the 7:6 width (height x 7 / 6, rounded to a multiple
   of 8) of each size. The pipeline crops the width from the center. The
   1:1 sizes are the native 1:1 sizes, so the XS 1:1 also takes its own
   height (NARROW_HEIGHT_3: the stock 4:3 and the 1:1 XS height). */
#define NARROW_WIDTH_0 5328, 5000, 4000, 4664
#define NARROW_WIDTH_1 4272, 4000, 3200, 3736
#define NARROW_WIDTH_2 2992, 2800, 2240, 2616
#define NARROW_WIDTH_3 1920, 1800, 1280, 1680
#define NARROW_HEIGHT_3 1440, 1280
/* The 5:4 sizes, then the 7:6 sizes (width, height), largest first. */
#define NARROW_SIZES 5000, 4000, 4000, 3200, 2800, 2240, 1800, 1440, \
    4664, 4000, 3736, 3200, 2616, 2240, 1680, 1440
/* The size classifier at its aspect BL: the frame is {r4-r7,fp,ip,lr,pc},
   r7 = width, r4 = height, r6 = class pointer. */
#define CLASSIFY_WIDTH r7
#define CLASSIFY_CLASS r6
#define CLASSIFY_RETURN ldm sp, {r4,r5,r6,r7,fp,sp,pc}
/* RAW development (RawDevelopmentModel) of the photo in playback. The
   playback metadata (FW_PLAYBACK_METADATA) holds the width and the height of
   that photo at METADATA_SIZE, as u16 values 4 bytes apart. The development
   aspect 0 develops the capture aspect that the DNG records, DEVELOP_SQUARE
   is the native 1:1, and DEVELOP_NATIVE_ASPECTS has a bit for each capture
   aspect id that the camera develops itself (3:2 and 1:1). The
   initial-aspect hook gets the model in DEVELOP_MODEL and the recorded
   capture aspect in DEVELOP_ASPECT. */
#define METADATA_SIZE 0x30
/* The playback metadata also holds the time of that photo at METADATA_TIME
   (build header), in the layout of GrkitDateTime (runtime/sdk/grkit.h).
   RawDevelopmentModel has its state at DEVELOP_STATE: 0 while no
   development runs. */
#define DEVELOP_STATE 4
#define DEVELOP_SQUARE 2
/* The value of the Standard Image Control in the development (category 0). */
#define DEVELOP_STANDARD 1
#define DEVELOP_NATIVE_ASPECTS 0x9U
#define DEVELOP_MODEL r4
#define DEVELOP_ASPECT r2
/* Resize: the register of the source size at the end of its getter, and the
   aspect that the resize job gives a 16:9 and a 4:3 source size. */
#define RESIZE_SIZE r4
#define RESIZE_JOB_WIDE 1
#define RESIZE_JOB_NARROW 2

/* The size of SelectMenuController, and the view type and the text view of
   CheckBoxListLineView (runtime/menu_settings.h). */
/* The string view of the submenu title view (submenu +0x15c): the title
   setter (2.10: 0x533a2ba0) sets the text ID of this view. */
#define SUBMENU_TITLE_STRING 0xf0
#define SELECT_MENU_SIZE 0x1d4U
/* The string view of the SelectMenuController title, whose text ID its
   start sets from its kind. */
#define SELECT_TITLE_STRING 0x160
/* The text ID of a string view (a halfword), which FW_STRING_VIEW_RESET sets:
   the view measures its text when the ID changes. */
#define STRING_VIEW_TEXT 0xea
/* The methods of the SelectMenuController vtable. */
#define SELECT_VTABLE_SLOTS 15U
/* The Start and Stop methods of a controller, which a controller change
   calls. */
#define SELECT_START_SLOT 5U
#define SELECT_STOP_SLOT 14U
/* The events that a SelectMenu page gets after MENU (CANCEL), and after OK
   and MENU (DONE): DONE returns the page. */
#define SELECT_EV_CANCEL 0xdaU
#define SELECT_EV_DONE 0xa2U
/* The fewest row views of a SelectMenuController. Its constructor builds a
   view for each row of its longest kind (+0x199). The longest native kind has
   21 rows, and no GRKit hook borrows it (2.10: kind table 0x53b3c6c0). The
   firmware can build the controller before INSTALL, so a value page on it
   shows at most these rows. A long value page shows on an own controller
   (runtime/menu_settings.h). */
#define SELECT_MENU_ROWS 21U
#define CHECK_LINE_TYPE 0x2aU
#define CHECK_TEXT_VIEW 0xf0

/* Visible-area id of a native aspect. GR III offers only 3:2 (0) and 1:1 (3). */
#define PICTURE_NATIVE_ID(native) ((native)==3 ? 3 : 0)

/* The shooting data of a shot (picture_shot), at these offsets from its shot
   settings pointer: the data that the camera writes into the EXIF of the
   picture. The exposure is in APEX values (float): Av, Tv, and Sv, then the
   exposure compensation. A time or bulb exposure
   (SHOT_LONG_MODE values SHOT_LONG_FIRST and the next) holds its time in
   microseconds (u64) at SHOT_TIME; another shot holds it there (u32) when the
   byte SHOT_TIME_SET is set. The capture time: the year (u16), then the
   month, the day, a gap, the hour, the minute, and the second. The focal
   length is a float in mm; times 1.5339 (runtime/picture.c) and the crop
   factor (float) at SHOT_CROP, it is the focal length of 35 mm film. The flash: on
   (byte), its mode (byte, 1 or 3 with red-eye reduction), and fired (byte).
   The program and the metering are bytes of the settings of the shot, with
   their EXIF codes. */
#define SHOT_SETTINGS 0x6c4
#define SHOT_APEX 0xb60
#define SHOT_TIME 0x918
#define SHOT_TIME_SET 0x94c
#define SHOT_LONG_MODE (SHOT_SETTINGS + 0x8b)
#define SHOT_LONG_FIRST 7
#define SHOT_TAKEN 0x944
#define SHOT_FOCAL 0x694
#define SHOT_CROP 0x7b8
/* CORE.CROP (runtime/crop.h): the size of 3:2 L, which the sensor crops
   by the factor of a crop; the factor of each value as float bits: Off,
   the native 35 mm, 40 mm, the native 50 mm, and 70 mm, so that the
   Exif focal length (18.3 mm x 1.5339 x the factor) names each crop; and
   the side of its frame in 1/65536 of the uncropped view. The shot request object holds the copy of the capture
   settings of a shot at CROP_SHOT_COPY, with its factor at SHOT_CROP. */
#define CROP_WIDTH 6000
#define CROP_HEIGHT 4000
#define CROP_FACTORS 0x3f800000,0x3fa00000,0x3fb6db6e,0x3fe49249,0x40200000
#define CROP_SCALES 65536,52429,45875,36700,26214
/* The JPEG codec takes sizes in multiples of 16. */
#define CROP_ALIGN 16
/* The icon of no crop, which hides the live-view crop badge. */
#define CROP_BADGE_OFF 0
/* The native factors of 35 and 50 mm, which the camera's own crop code
   takes (runtime/framing.S). */
#define CROP_NATIVE_35 0x3fa00000
#define CROP_NATIVE_50 0x3fe49249
/* The live view zooms to the factor of a crop (runtime/framing.S:
   crop_live). */
#define CROP_LIVE_FACTOR 1
#define CROP_SHOT_COPY 0x44c8
/* The instruction that the shot hook displaces (runtime/framing.S). */
#define CROP_SHOT_DISPLACED mov r0, r4
/* DNGMetaDataWriter::TiffEntryDataSetter: RAW image info and shot metadata. */
#define DNG_SETTER_INFO 0x7c
#define DNG_SETTER_SHOT 0x84
#define SHOT_FLASH_ON 0xde
#define SHOT_FLASH_MODE 0x760
#define SHOT_FLASH_FIRED 0x71aa8
#define SHOT_PROGRAM (SHOT_SETTINGS + 0x8b)
#define SHOT_PROGRAM_CODES 2,4,3,0,1,1,1,1,1
#define SHOT_METERING (SHOT_SETTINGS + 0x8e)

/* The native processing applies saturation before the tone curves. GR IV
   applies it after them, so a native look moves it (runtime/picture.c). */
#define NATIVE_SATURATION_BEFORE_CURVES 1

/* The mode dial (runtime/mode_dial.h). The switch model of the App
   (DIAL_SWITCHES, build header) keeps the state of each switch in a byte at
   DIAL_SWITCH_STATES + 8 x its ID. The mode dial is switch DIAL_SWITCH: P 3,
   Tv 5, Av 6, M 8, and U1 to U3 from DIAL_CODE_U1. The first byte of the
   header of the settings object (DIAL_SETTINGS, build header) is the user
   mode box that the camera loaded, 0 for P to M. The settings of a position
   are loaded when that box is not 0 for a user mode, and 0 for another
   position. */
#define DIAL_SWITCH_STATES 0x114
#define DIAL_SWITCH 38
#define DIAL_CODE_U1 10U
#define DIAL_POSITION() BYTE(DIAL_SWITCHES,DIAL_SWITCH_STATES+DIAL_SWITCH*8)
#define DIAL_NATIVE_SETTLED(position) \
    ((BYTE(WORD(DIAL_SETTINGS,0),0)!=0)==((u32)((position)-DIAL_CODE_U1)<3U))
/* The native slots of the user modes (runtime/mode_dial.h) are the six
   boxes of the settings object. Its word 1 points to the current settings
   record, and word 2 + n to that of box n (1 to 6); a record has
   DIAL_RECORD_SIZE bytes. A user mode loads the box that the byte at
   DIAL_BOX_ASSIGN + 2 x (u - 1) of the current record gives, 1 to 6. Each
   of the eight records holds the name of each box, ASCII of at most 32
   characters with 0 after it, and whether the box is registered; the
   setters of the firmware write the eight records, as the core does. The
   save copies the current settings into box n (FW_BOX_SAVE), as the native
   save does. DIAL_SHARED_TEXT names P, Tv, Av, and M, the positions that
   share their settings. */
#define DIAL_SHARED_TEXT 'P','/','A','v','/','T','v','/','M',0
#define DIAL_RECORD_SIZE 0xb26U
#define DIAL_RECORD() WORD(DIAL_SETTINGS,4)
#define DIAL_BOX_ASSIGN 0x7e0U
#define DIAL_BOXES 6U
#define DIAL_NAME_RECORDS 8U
#define DIAL_NAME_RECORD(i) WORD(DIAL_SETTINGS,4U+4U*(i))
#define DIAL_BOX_NAME 0x7e6U
#define DIAL_BOX_NAME_STRIDE 0x22U
#define DIAL_BOX_REGIST 0x8b2U
#define DIAL_BOX_NAME_FIELD(slot) (DIAL_BOX_NAME+DIAL_BOX_NAME_STRIDE*((slot)-1U))
#define DIAL_BOX_REGIST_FIELD(slot) (DIAL_BOX_REGIST+2U*((slot)-1U))
#define DIAL_SLOT_NAME_AT(slot) (DIAL_RECORD()+DIAL_BOX_NAME_FIELD(slot))
#define DIAL_SLOT_REGIST_AT(slot) (DIAL_RECORD()+DIAL_BOX_REGIST_FIELD(slot))
#define DIAL_SLOT_OF(u) dial_box_of(u)
#define DIAL_SLOT_SAVE(slot) CALL2(FW_BOX_SAVE)(DIAL_SETTINGS,(slot))
#define DIAL_SLOT_NAME_SET(slot,name,registered) dial_names_write((slot),(name),(registered))

/* The canvas of CORE.BORDER (runtime/canvas_shot.h). A still develops in
   StillMake into the bank of its recipe R: R2Y writes the source picture in
   stripes, then BackResize makes the main picture and the previews, which
   the JPEG encode takes. Each picture is a 32-byte descriptor in R: +4 Y,
   +8 interleaved CbCr (4:2:2), +0x10 u16 width and height, +0x14 u16 pitch
   and rows. The R2Y hook gets the config of the stripe send, which is
   R + CANVAS_R2Y_RECIPE, and the StillMake module + CANVAS_R2Y_MODULE_BIAS;
   the BackResize hook gets its config, R + CANVAS_RESIZE_RECIPE, and the
   module. */
#define CANVAS_R2Y_RECIPE 0x96c
#define CANVAS_R2Y_MODULE_REG r4
#define CANVAS_R2Y_MODULE_BIAS 0x9000
#define CANVAS_R2Y_CONFIG_REG r6
#define CANVAS_RESIZE_RECIPE 0x904
#define CANVAS_RESIZE_MODULE_REG r7
#define CANVAS_RESIZE_CONFIG_REG r4
/* The instruction that the BackResize hook replaces. */
#define CANVAS_RESIZE_STOCK ldr r2, [fp, #-0x28]
/* The descriptors of R: the R2Y source, the R2Y resize output (a quick
   view), the main picture, the screennail, and the thumbnail. */
#define CANVAS_SOURCE 0x70
#define CANVAS_QUICK 0x90
#define CANVAS_MAIN 0xb0
#define CANVAS_SCREENNAIL 0xd0
#define CANVAS_THUMBNAIL 0xf0
/* The developed bank of R, its size, and the shot settings of the picture
   (the key of the shot ring). */
#define CANVAS_BANK 0x304
#define CANVAS_BANK_SIZE 0x364
#define CANVAS_SHOT 0x44c8
/* The source starts this many rows after the main picture in the bank. */
#define CANVAS_SOURCE_ROWS 144U
/* Above the top margin, the canvas keeps the guard rows of the stock layout,
   which R2Y may write. */
#define CANVAS_ROWS_ABOVE(top) (2U*CANVAS_SOURCE_ROWS+(top))
/* The stripe send config: the source Y and CbCr, then the Y and CbCr of the
   R2Y resize output. The module: the Y and CbCr of the stripe, and the u16
   output pitch of R2Y. */
#define CANVAS_CONFIG_SOURCE 0x18
#define CANVAS_CONFIG_QUICK 0x20
#define CANVAS_MODULE_SOURCE 0x9950
#define CANVAS_MODULE_PITCH 0x9960
/* FW_CANVAS_FILL(0, descriptor, color, 1) fills the rectangle of a
   descriptor through the IIP and returns when it is done; the last argument
   leaves out its cache maintenance. Its descriptor adds +0 format (1 for
   4:2:2), +0x18 u16 x and y, and +0x1c tag 2. */
#define CANVAS_FILL_FORMAT 1U
#define CANVAS_FILL_TAG 2U
#define CANVAS_FILL_BLACK 1U
#define CANVAS_FILL_WHITE 2U

#endif
