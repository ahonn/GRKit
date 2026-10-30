/* RICOH GR IV family: the object layouts, events, and constants that each
   GR IV firmware build shares. A build header (gr4-<version>.h) defines the
   firmware addresses (FW_*) and the values that differ in its build, then
   includes this file. Comments here name no firmware address, because each
   build has its own; the build header gives them.
   A build that changes a value of this file defines it in its build header,
   and the value moves there for each build of the family. */
#ifndef GRKIT_GR4_H
#define GRKIT_GR4_H

/* Icon slots hold a 12-byte descriptor, then 40 x 40 RGBA. */
#define GRKIT_ICON_STRIDE 6412

/* Camera settings (FW_SETTINGS) and the still aspect getter
   (FW_STILL_ASPECT_GET). The native ids are those of GR III: 0 = 3:2, 1 = 4:3,
   2 = 16:9, 3 = 1:1. GR IV offers all four.
   Settings domain (FW_SETTINGS_DOMAIN): the capture property singleton starts
   with a vtable, then the domain. Domain +0x518 -> control -> payload with the
   native Image Control type at +0x79c. Standard is type 11. The shot settings
   hold the type at the same offset as the settings payload. */
#define SETTINGS_IC_TYPE 0x79c
/* Framing: the rows of every family (runtime/firmware.h). GR IV has the
   native aspects 3:2, 4:3, 16:9, and 1:1, but Framing crops each ratio from
   native 3:2, as on GR III, so each ratio has the same live view. 16:9,
   XPan, and 2.35:1 use capture aspect id 2, XPan and 2.35:1 with their own
   heights. 4:3, 5:4, and 1:1 use id 1, 5:4 and 1:1 with their own widths.
   The capture request hook maps the 3:2 request to that id, and the
   live-view hooks keep the live view in 3:2. In a native 16:9 that the user
   sets in the camera menu, XPan and 2.35:1 also crop the native 16:9 photos. */
#define FRAMING_RATIO_ROWS 0,1,4,2,3,6,7,5
#define FRAMING_XPAN_NATIVE 2
/* FW_CAPTURE_ASPECT_GET reads the capture aspect (property 0xdf,
   CommonProperty +0x7da); the live-view mode function and the mode check call
   it. FW_STILL_ASPECT_RESEND resends the still aspect: with the UI aspect
   (+0x49f) equal to the applied aspect (+0xf8b) it posts no UI event and, in
   still mode, sends property 0xdf with the applied value. */
/* The next copy of the capture aspect: domain +0x518 -> control, +4 -> next. */
#define FRAMING_CAPTURE_ASPECT() BYTE(WORD(WORD(WORD(FW_SETTINGS_DOMAIN,0),0x518),4),0x7da)
#define FRAMING_REQUEST_ASPECT() (((void (*)(u32))FW_STILL_ASPECT_RESEND)(FRAMING_SETTINGS()))
#define MENU_FRAMING_REQUEST() (((void (*)(u32))FW_STILL_ASPECT_RESEND)(CALL0(FW_SETTINGS)()))
/* The native StillAspectRatio, and its setter as the native menu calls it:
   FW_STILL_ASPECT_SET writes the UI aspect (+0x49f) and runs the resend, which
   applies a change, posts the UI event, and switches the live view. */
#define MENU_NATIVE_ASPECT() CALL1(FW_STILL_ASPECT_GET)(CALL0(FW_SETTINGS)())
#define MENU_SET_NATIVE_ASPECT(id) CALL2(FW_STILL_ASPECT_SET)(CALL0(FW_SETTINGS)(),(id))
/* GR IV stops no live view for a native aspect change. Its live-view mode
   function and mode check read the capture aspect (framing_live_mode,
   framing_live_check). */
#define MENU_STOP_LIVE_VIEW() ((void)0)
/* ImageResolutionTranslatorKb636 GetResolution id 2: the stock 16:9 height,
   the 65:24 height (width x 24 / 65), and the 2.35:1 height (width / 2.35) of
   each size, rounded to a multiple of 8. */
#define XPAN_HEIGHT_0 3480, 2288, 2632
#define XPAN_HEIGHT_1 2784, 1824, 2104
#define XPAN_HEIGHT_2 1968, 1296, 1488
#define XPAN_HEIGHT_3 1080, 712, 816
#define XPAN_SIZES 6192, 2288, 4944, 1824, 3504, 1296, 1920, 712
#define SCOPE_SIZES 6192, 2632, 4944, 2104, 3504, 1488, 1920, 816
/* GetResolution id 1: the stock 4:3 width, the 5:4 width (height x 5 / 4),
   the 1:1 width, and the 7:6 width (height x 7 / 6, rounded to a multiple
   of 8) of each size. The pipeline crops the width from the center. The
   1:1 sizes are the native 1:1 sizes, so the XS 1:1 also takes its own
   height (NARROW_HEIGHT_3: the stock 4:3 and the 1:1 XS height). */
#define NARROW_WIDTH_0 5504, 5160, 4128, 4816
#define NARROW_WIDTH_1 4400, 4120, 3296, 3848
#define NARROW_WIDTH_2 3120, 2920, 2336, 2728
#define NARROW_WIDTH_3 1920, 1800, 1280, 1680
#define NARROW_HEIGHT_3 1440, 1280
/* The 5:4 sizes, then the 7:6 sizes (width, height), largest first. */
#define NARROW_SIZES 5160, 4128, 4120, 3296, 2920, 2336, 1800, 1440, \
    4816, 4128, 3848, 3296, 2728, 2336, 1680, 1440
/* The size classifier at its aspect BL: the frame is {r3-r6,fp,ip,lr,pc},
   r6 = width, r4 = height, r5 = class pointer. */
#define CLASSIFY_WIDTH r6
#define CLASSIFY_CLASS r5
#define CLASSIFY_RETURN ldm sp, {r3,r4,r5,r6,fp,sp,pc}

/* View classes. Their vtables and object sizes equal those of GR III 2.10.
   The list's row vector (first and end pointers) and its shown row count. */
#define LIST_ROWS 0xf4
#define LIST_COUNT 0x108

/* The slot type of the native Image Control slot, and the controller of its
   Standard parameter page (still mode). */
#define ADJ_IC_SLOT_TYPE 15
#define IC_PARAMETERS_CONTROLLER 0x2000017
/* The shooting screen: a half-press on a page changes to it, and the release
   changes back to the page. */
#define SHOOTING_CONTROLLER 0x8000000
/* The native screens of ui_open_native (runtime/ui.h): the targets of the
   MENU rows White Balance (still) and LCD, the same on each GR IV build. */
#define UI_NATIVE_WHITE_BALANCE 0x2000064U
#define UI_NATIVE_LCD 0x200001dU
/* ImageControlDetailController, as on GR III with other fields: the type
   (+0x19c), the set kind (+0x19d), and the base kind of its row table
   (+0x19e, 0 Standard). The model snapshot holds one byte for each row kind
   from +3. GR IV has one more row kind before shading, so shading and clarity
   are kinds 14 and 15. */
/* The main title view of the page, as on GR III. */
#define DETAIL_TITLE 0x15c
#define DETAIL_TYPE 0x19c
#define DETAIL_SET_KIND 0x19d
#define DETAIL_BASE_KIND 0x19e
#define MODEL_SNAPSHOT 3
#define PARAMETER_KINDS {0,1,2,3,4,5,6,14,15}
#define DETAIL_EV_SUBPAGE 0x8c
/* The native settings page of a GRKit root row, as on GR III: the MENU
   submenu Instant Review (0x20000d4, entry 11 of FW_SUBMENU_KINDS), its three
   value pages (0x20000d9 Display Time, 0x20000da Zoom Review, 0x20000db), and
   two plain value pages of the MENU Setup tab (0x20000b5, 0x20000b6), whose
   records follow those of Instant Review. The text IDs of the title and the
   rows are in the build header. */
#define SUBMENU_KIND_SIZE 0x64
#define SUBMENU_KIND_ROWS 6
#define SUBMENU_KIND_LABELS 8
#define SUBMENU_KIND_CONTROLLERS 0x14
#define SUBMENU_KIND_ROW_KINDS 0x28
#define SUBMENU_KIND_VALUES 0x3c
#define SETTINGS_SUBMENU 0x20000d4U
#define SETTINGS_SUBMENU_KIND 11
#define SETTINGS_PAGES {0x20000d9U,0x20000daU,0x20000dbU,0x20000b5U,0x20000b6U}
#define ROW_TEXT_VIEW 0xf8
/* The size of SelectMenuController, and the view type and the text view of
   CheckBoxListLineView (runtime/menu_settings.h). */
#define SELECT_MENU_SIZE 0x1e4U
#define CHECK_LINE_TYPE 0x2aU
#define CHECK_TEXT_VIEW 0xf0
#define ROW_VALUE_VIEW 0xfc
/* The label view of a submenu row, which its label setter sets. */
#define ROW_LABEL_VIEW 0xec
/* Rows of modules in the native MENU (runtime/menu_section.h). MENU builds its
   tabs at the first MENU of a start from the tab descriptors in the RAM
   table FW_MENU_TABS of the build header; entry MENU_TAB_GRKIT is the
   Customize tab. A descriptor holds the row count (u8) at MENU_TAB_ROWS, then for each of
   MENU_TAB_MAX_ROWS rows its filler (a C++ member pointer), its target
   controller, and its predicate (a member pointer, 0 for a row that always
   shows). A filler gets the row view and sets its label view (+0xec) and
   value view with the row setters. */
/* The MENU tab of the GRKit section (Customize, C), and the sections of a
   tab descriptor: their count, title text IDs, and first rows. The Setup
   tab has all MENU_TAB_MAX_SECTIONS sections; Customize has 3. */
#define MENU_TAB_GRKIT 3
/* The camera information page (Setup tab, Camera Information), which About
   GRKit opens: its controller, and three rows of a label (a text ID of the
   build, INFO_LABEL_TEXTS) and a value, an ASCII string view that the page
   update sets (menu_info_update) with FW_STRING_VIEW_SET_ASCII(view, text).
   About GRKit shows the version and the project address in the first two
   rows, and hides the third. */
#define INFO_CONTROLLER 0x200010bU
#define INFO_LABELS {0x164,0x16c,0x174}
#define INFO_VALUES {0x168,0x170,0x178}
#define MENU_TAB_SECTIONS 0x68
#define MENU_TAB_TITLES 0x6a
#define MENU_TAB_STARTS 0x80
#define MENU_TAB_MAX_SECTIONS 11
/* The TopMenu instance in the controller factory (FW_CONTROLLER_FACTORY). */
#define MENU_TOP_SLOT 0x80
#define MENU_TAB_SIZE 0x384
#define MENU_TAB_ROWS 0x8b
#define MENU_TAB_FILLERS 0x8c
#define MENU_TAB_TARGETS 0x1bc
#define MENU_TAB_PREDICATES 0x254
#define MENU_TAB_MAX_ROWS 38
#define MENU_ROW_LABEL_VIEW 0xec
#define MENU_ROW_VALUE_VIEW 0xfc
/* A native view keeps its frame {x, y, width, height} at VIEW_FRAME, and its
   vtable sets it at VIEW_SET_FRAME. The value of a MENU or submenu row shows
   in a column of VALUE_COLUMN pixels from the x of the view. */
#define VIEW_FRAME 0x20
#define VIEW_SET_FRAME 0x18
#define VALUE_COLUMN 140
/* AdjModeController fields. GR IV adds two FunctionGuideViews before the list,
   which moves the list and the slot byte by 8. */
#define ADJ_CANVAS 0x4
#define ADJ_TITLE 0x160
#define ADJ_DETAIL 0x164
#define ADJ_DETAIL_VALUE 0x168
#define ADJ_CELLS 0x16c
#define ADJ_FRAME 0x180
#define ADJ_ICONS 0x184
#define ADJ_FN_GUIDE 0x1ac
/* The Image Control slot method of the ADJ refresh fills the Fn hint with the
   Fn label (FN_GUIDE_TEXT), layout 10, and the settings icon (FN_GUIDE_ICON),
   and moves it to x 300 (FW_GUIDE_SET_CONTENT). A new ADJ starts with an
   empty hint. Text and icon IDs differ between builds, so the build header
   gives them. */
#define FN_GUIDE_X 300
#define ADJ_LIST 0x1f8
#define ADJ_SLOT 0x1fc
#define ADJ_ACTION 0x281

/* AdjModeController events (the first halfword of an event). Measured, except
   the dial events, which follow the +6 offset of the direction events. The
   touch event is the one that the native handler tests like GR III 0x30. */
#define EV_SWITCH 5
#define EV_SWITCH_END 8
#define EV_FN 0x1b
/* An Fn press comes as EV_FN with argument 2, as on GR III, then EV_FN_ACTION,
   then EV_FN with argument 1 at the release. The native ADJ handler ignores
   EV_FN and acts on EV_FN_ACTION. */
#define FN_PRESS_ARG 2
#define EV_FN_ACTION 0x187
/* A press of the rear dial, which opens and closes ADJ, before the confirm
   that OK also sends. */
#define EV_ADJ_PRESS 0x23
#define EV_TOUCH 0x31
#define EV_LEFT 0xd4
#define EV_RIGHT 0xd5
#define EV_UP 0xd6
#define EV_DOWN 0xd7
#define EV_DIAL_UP 0xda
#define EV_DIAL_DOWN 0xdb
#define EV_CONFIRM 0xe0

/* GR IV draws ADJ before the toolbar exists, so the new cell needs a redraw. */
#define TOOLBAR_CREATED(widget,icon) \
    (CALL1(FW_VIEW_INVALIDATE)(widget), CALL1(FW_VIEW_INVALIDATE)(icon))

/* The installer always provides the GRKit Off icon. */
#define NATIVE_OFF_ICON() 0

/* Playback metadata singleton (FW_PLAYBACK_METADATA): +0x228 holds key 0x25,
   the Image Control tag. */
#define METADATA_TONE 0x228

/* LiveViewController: nonzero in movie mode, and the LCD canvas. GR IV adds a
   FrameView at 0x2d0, which moves the movie flag by 4. */
#define LIVEVIEW_MOVIE 0x2d6
#define LIVEVIEW_CANVAS 0x15c

/* LiveViewController: the byte that makes UpdateLevelAlarm set the
   highlight warning again (FW_LIVEVIEW_LEVEL_UPDATE). */
#define LIVEVIEW_LEVEL_PENDING 0x4e3
/* The posture word of the electronic level: 1 landscape, 2 upside down, 3
   and 4 the two vertical postures, 5 and 6 the lens down or up
   (runtime/picture.c: picture_turns). */
#define LEVEL_POSTURE 0x64
/* Picture processing. GAMMA_BUILT: the word of the gamma object that is 1
   after the native curves are built. */
#define GAMMA_BUILT 0x382c
/* GAMMA_UPLOAD: the byte of the gamma object that makes the table build use
   the composed curves. */
#define GAMMA_UPLOAD 0x380a
/* DigitalFilterProcess plan: the grain stage flag that the stage gate sets. */
#define GRAIN_GATE_FLAG 0x25

/* The keyboard of the camera (runtime/ui.h): SoftwareKeyboardController
   copies its use from the model at its start (OnStart) to the byte
   KEYBOARD_FUNCTION; the copyright use is KEYBOARD_COPYRIGHT. The model
   (FW_KEYBOARD_MODEL) holds the use in its byte KEYBOARD_MODEL_FUNCTION, 0
   for none, and the keyboard clears it when it stops. */
#define KEYBOARD_FUNCTION 0x240
#define KEYBOARD_COPYRIGHT 3U
#define KEYBOARD_MODEL_FUNCTION 0

/* Each native aspect has its own visible area in a preview picture. */
#define PICTURE_NATIVE_ID(native) ((native)<=3 ? (native) : 0)

#endif
