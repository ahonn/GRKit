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
/* The state of the Wi-Fi access point of the camera in the same payload:
   SETTINGS_WLAN_STATION while a phone is connected. */
#define SETTINGS_WLAN_STATE 0x24a
/* 1 while the Wi-Fi access point of the camera is on. */
#define SETTINGS_WLAN_ENABLED 0x7ff
/* The digital filter values of the shot settings, which the grain stage of
   recorded pictures reads: Shading and Clarity (0 to 8, 4 is neutral), grain
   on (0 is off), and the HDR Tone strength (0 is off). The native parameter
   build writes them for its Image Control type. */
#define SETTINGS_SHADING 0x7b3
#define SETTINGS_CLARITY 0x7b4
#define SETTINGS_GRAIN 0x7b5
#define SETTINGS_HDR 0x7b8
/* The ImageTone code of the first look of GR IV Monochrome. Its looks record
   0x8100 and up, in the order Standard, Solid, Soft, Hi-Contrast, Grainy,
   HDR Tone, and Solarized Effect (types 25, 29, 26, 27, 28, 30, and 32). */
#define NATIVE_TONE_MONOCHROME 0x8100U
/* Framing: the rows and the sizes of the family (runtime/firmware.h), from
   grkit_core::framing. GR IV has the native aspects 3:2, 4:3, 16:9, and 1:1,
   but Framing crops each ratio from native 3:2, as on GR III, so each ratio
   has the same live view. The wide ratios use capture aspect id 2 with their
   own heights, the narrow ones id 1 with their own widths.
   The capture request hook maps the 3:2 request to that id, and the
   live-view hooks keep the live view in 3:2. In a native 16:9 that the user
   sets in the camera menu, XPan and 2.35:1 also crop the native 16:9 photos. */
#define FRAMING_FAMILY_GR4
#include "../framing_ratios.h"
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
/* The size classifier at its aspect BL: the frame is {r3-r6,fp,ip,lr,pc},
   r6 = width, r4 = height, r5 = class pointer. */
#define CLASSIFY_WIDTH r6
#define CLASSIFY_CLASS r5
#define CLASSIFY_RETURN ldm sp, {r3,r4,r5,r6,fp,sp,pc}
/* RAW development (RawDevelopmentModel) of the photo in playback. The
   playback metadata (FW_PLAYBACK_METADATA) holds the width and the height of
   that photo at METADATA_SIZE, as u16 values 4 bytes apart. The development
   aspect 0 develops the capture aspect that the DNG records, DEVELOP_SQUARE
   is the native 1:1, and DEVELOP_NATIVE_ASPECTS has a bit for each capture
   aspect id that the camera develops itself (all four). The initial-aspect
   hook gets the model in DEVELOP_MODEL and the recorded capture aspect in
   DEVELOP_ASPECT. */
#define METADATA_SIZE 0x30
/* The playback metadata also holds the time of that photo at METADATA_TIME
   (build header), in the layout of GrkitDateTime (runtime/sdk/grkit.h).
   RawDevelopmentModel has its state at DEVELOP_STATE: 0 while no
   development runs. It holds at DEVELOP_SETTINGS the settings that it
   reads from the DNG, with the crop factor at SHOT_CROP, as a shot does.
   The playback metadata holds the Exif 35 mm focal length of the photo in
   mm, below 256 for each lens, at METADATA_FOCAL_35MM (build header). */
#define DEVELOP_STATE 4
#define DEVELOP_SETTINGS 0x24
/* The contents that playback opens (FW_CONTENTS): the handle of the open
   file, and in it the path of the file, whose first byte, the drive
   letter, is 0. */
#define CONTENTS_HANDLE 0x38
#define CONTENTS_PATH 0x30
/* The playback metadata (FW_PLAYBACK_METADATA) holds the number of the
   folder and of the file of the photo that playback shows, as the page
   shows them (100-0001), as u16 values. Playback that opens before the last
   shot is saved shows that shot, while the contents keep the photo before
   it until another photo shows. */
#define METADATA_FOLDER 0xe
#define METADATA_FILE 0x10
/* Playback of one photo (runtime/playback_info.h): the ID of its controller,
   the slot of the controller in the controller factory
   (FW_CONTROLLER_FACTORY), which keeps it after playback ends, and its
   display, which DISP turns: PLAYBACK_DETAIL shows the detailed information,
   whose page, PLAYBACK_INFO_PAGE for the first one, follows in the next byte. Its
   canvas is the word at ADJ_CANVAS of the controller. */
#define PLAYBACK_SINGLE 0x4000001U
#define PLAYBACK_FACTORY_SLOT 0x10
#define PLAYBACK_DISPLAY 0x31c
#define PLAYBACK_DETAIL 2U
#define PLAYBACK_INFO_PAGE 0U
/* The row of icons of Effect Icons in Playback on the first page
   (runtime/viewfinder.h: playback_paint), in the row of the drive mode, which
   stays free with each Image Control: the right of its last icon, the right
   of its last icon for a photo of interval shooting, whose shot number the
   camera shows at the right of the row, and of interval composite, whose
   time the camera shows before that, the left of its first icon, which
   the camera leaves free after the icons that it adds from the left (the
   drive mode, the self-timer), and the top of the row. */
#define PLAYBACK_INFO_ROW_RIGHT 700U
#define PLAYBACK_INFO_INTERVAL_RIGHT 520U
#define PLAYBACK_INFO_COMPOSITE_RIGHT 390U
#define PLAYBACK_INFO_ROW_LEFT 330U
#define PLAYBACK_INFO_ROW_TOP 110U
/* The crop badge of the photo on that page: the place of the camera's own,
   right of the focal length. */
#define PLAYBACK_CROP_LEFT 642U
#define PLAYBACK_CROP_TOP 160U
/* RAW development, the Fn adjustments (runtime/develop.h): RawDevelopmentModel
   holds the selected Image Control: the category at DEVELOP_CATEGORY and the
   value of each category from DEVELOP_VALUES. The build header gives the
   parameter set of Standard (DEVELOP_PARAMETERS). */
#define DEVELOP_CATEGORY 0x9c
#define DEVELOP_VALUES 0x9d
#define DEVELOP_SQUARE 4
/* The value of the Standard Image Control in the development (category 0). */
#define DEVELOP_STANDARD 1
/* The Image Control list of the development (RawDevelopmentParameterSetController):
   the controller ID word, the ID of the list, the first instruction of the
   row category, which its entry hook replaces, the subtitle of the list
   (+0x160 of the controller, r4 in UpdateSubTitle), the subtitle of
   RawDevelopmentTopController (r6), the list view, and the style and the
   height of its row views. The IDs of the development controllers follow
   DEVELOP_CONTROLLERS, the first one, which each build header gives. */
#define CONTROLLER_ID 0x124
#define DEVELOP_TONE_LIST (DEVELOP_CONTROLLERS + 4U)
/* The ID of the aspect list of the development, and the first instruction
   of the row value of the model, which its entry hook replaces. */
#define DEVELOP_ASPECT_LIST (DEVELOP_CONTROLLERS + 1U)
#define DEVELOP_ASPECT_VALUE_FIRST "mov ip, sp"
#define DEVELOP_CATEGORY_FIRST "ldrb r3, [r0, #5]"
#define DEVELOP_LIST_TITLE ldr r3, [r4, #0x160]
#define DEVELOP_TOP_TITLE mov r3, r6
/* RawDevelopmentTopController: the kinds of its native rows, then the
   GRKit kind (runtime/menu.c); the register of the kind in its switches of
   the kinds; the end of the decide method of its rows (r4 the controller),
   of its row name method (the text ID in r0), and of its check of a kind
   (1 in r0); the offset from the default of its subtitle switch (r5 the
   controller) to the branch to the subtitle without a value; the offset
   from the default of its fill switch to the branch to the fill of the row;
   and the icon of the GRKit row, GRKIT_ICON_GRKIT. Each build header gives
   the register of the mark of a row in that fill (DEVELOP_TOP_FILL_MARK)
   and the title text ID of the Image Control list (DEVELOP_TONE_TITLE). */
#define DEVELOP_TOP_ROWS 9
#define DEVELOP_TOP_ROW r3
#define DEVELOP_TOP_DECIDE_END sub sp, r11, #16; ldm sp, {r4, r11, sp, pc}
#define DEVELOP_TOP_NAME_END sub sp, r11, #16; ldm sp, {r4, r11, sp, pc}
#define DEVELOP_TOP_AVAILABLE_END sub sp, r11, #16; ldm sp, {r4, r11, sp, pc}
#define DEVELOP_TOP_NO_VALUE 0x28
#define DEVELOP_TOP_FILL_ROW 8
#define DEVELOP_TOP_ICON 0x7ffe
#define DEVELOP_LIST_VIEW 0x168
#define DEVELOP_ROW_STYLE 0x1aU
#define DEVELOP_ROW_HEIGHT 0x3aU
#define DEVELOP_NATIVE_ASPECTS 0xfU
#define DEVELOP_MODEL r0
#define DEVELOP_ASPECT r1
/* Resize: the register of the source size at the end of its getter, and the
   aspect that the resize job gives a 16:9 and a 4:3 source size. */
#define RESIZE_SIZE r4
#define RESIZE_JOB_WIDE 1
#define RESIZE_JOB_NARROW 2

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
/* The last two of them, Shading and Clarity, for assembly (runtime/processing.h). */
#define PARAMETER_KIND_SHADING 14
#define PARAMETER_KIND_CLARITY 15
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
/* The string view of the submenu title view (submenu +0x15c): the title
   setter (1.11: 0x5357f2b0) sets the text ID of this view. */
#define SUBMENU_TITLE_STRING 0xf0
#define SELECT_MENU_SIZE 0x1e4U
/* The string view of the SelectMenuController title, whose text ID its
   start sets from its kind. */
#define SELECT_TITLE_STRING 0x160
/* The text ID of a string view (a halfword), which FW_STRING_VIEW_RESET sets:
   the view measures its text when the ID changes. */
#define STRING_VIEW_TEXT 0xea
/* The methods of the SelectMenuController vtable. */
#define SELECT_VTABLE_SLOTS 16U
/* The Start and Stop methods of a controller, which a controller change
   calls. */
#define SELECT_START_SLOT 5U
#define SELECT_STOP_SLOT 15U
/* The events that a SelectMenu page gets after MENU (CANCEL), and after OK
   and MENU (DONE): DONE returns the page. */
#define SELECT_EV_CANCEL 0xe2U
#define SELECT_EV_DONE 0xa9U
/* The fewest row views of a SelectMenuController. Its constructor builds a
   view for each row of its longest kind (+0x1a5). The longest native kind has
   21 rows on each build, and no GRKit hook borrows it (1.11: kind table
   0x53da2d20). The firmware can build the controller before INSTALL, so a
   value page on it shows at most these rows. A long value page shows on an
   own controller (runtime/menu_settings.h). */
#define SELECT_MENU_ROWS 21U
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
/* Each label and value view, which the View of a recipe hides. */
#define INFO_LABELS_VALUES {0x164,0x16c,0x174,0x168,0x170,0x178}
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
/* 1 while the held Fn action of an ADJ slot runs. */
#define ADJ_FN_HELD 0x281

/* AdjModeController events (the first halfword of an event). Measured, except
   the dial events, which follow the +6 offset of the direction events. The
   touch event is the one that the native handler tests like GR III 0x30. */
#define EV_SWITCH 5
#define EV_SWITCH_END 8
/* A half-press of the shutter on a MENU value page, before the confirm that
   OK also sends. */
#define EV_HALF_PRESS 2
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

/* The keyboard of the camera (runtime/ui.h): its controller ID
   (KEYBOARD_CONTROLLER). SoftwareKeyboardController copies its use from
   the model at its start (OnStart) to the byte
   KEYBOARD_FUNCTION; the copyright use is KEYBOARD_COPYRIGHT. The model
   (FW_KEYBOARD_MODEL) holds the use in its byte KEYBOARD_MODEL_FUNCTION, 0
   for none, and the keyboard clears it when it stops. */
#define KEYBOARD_CONTROLLER 0x20000f7U
#define KEYBOARD_FUNCTION 0x240
#define KEYBOARD_COPYRIGHT 3U
#define KEYBOARD_MODEL_FUNCTION 0

/* Each native aspect has its own visible area in a preview picture. */
#define PICTURE_NATIVE_ID(native) ((native)<=3 ? (native) : 0)

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
#define SHOT_SETTINGS 0x6e4
#define SHOT_APEX 0xb98
#define SHOT_TIME 0x940
#define SHOT_TIME_SET 0x974
#define SHOT_LONG_MODE (SHOT_SETTINGS + 0x8c)
#define SHOT_LONG_FIRST 9
#define SHOT_TAKEN 0x96c
#define SHOT_FOCAL 0x6b4
#define SHOT_CROP 0x7dc
/* CORE.CROP (runtime/crop.h): the side of the frame of each crop in
   1/65536 of the uncropped view. runtime/framing_ratios.h gives the factor
   of each crop, so that the Exif focal length (18.3 mm x 1.5339 x the
   factor) names it. The shot request object holds the copy of the capture
   settings of a shot at CROP_SHOT_COPY, with its factor at SHOT_CROP. */
#define CROP_SCALES 65536,52327,45990,37086,26280
/* The Exif 35 mm focal length of a photo of each crop that the camera does
   not have, 40 and 70 mm, 0 for the others (runtime/crop.h: crop_of_photo). */
#define CROP_FOCALS 0,0,40,0,70
/* The icon of no crop, which hides the live-view crop badge. */
#define CROP_BADGE_OFF 0
/* The live view zooms to the factor of a crop (runtime/framing.S:
   crop_live). */
#define CROP_LIVE_FACTOR 1
#define CROP_SHOT_COPY 0x4688
/* The instruction that the shot hook displaces (runtime/framing.S). */
#define CROP_SHOT_DISPLACED mov r1, r5
/* The queue completion of a crop shot (runtime/crop_queue.S). The back end
   of a shot: the byte that the release stop sets (CROP_BACKEND_RELEASED),
   the byte that the completion check requires to be zero, the owner of its
   events, its recipe queue, and the recipe of the shot. A recipe: its
   state, its progress, and its table ID. A recipe table: an opcode word for
   each step from CROP_TABLE_OPS, and a halfword argument for each step from
   CROP_TABLE_ARGS. The step with opcode 1 and argument 8 is the completion
   step of a shot, and opcode 12 ends the table. The event poster takes
   CROP_QUEUE_EVENT for the queue completion. */
#define CROP_BACKEND_RELEASED 0x2dd
#define CROP_BACKEND_HELD 0x2de
#define CROP_BACKEND_OWNER 0x7b4
#define CROP_BACKEND_QUEUE 0x7b8
#define CROP_BACKEND_RECIPE 0x620
#define CROP_RECIPE_STATE 0x4
#define CROP_RECIPE_PROGRESS 0x14
#define CROP_RECIPE_TABLE 0x18
#define CROP_TABLE_OPS 0x4
#define CROP_TABLE_ARGS 0x204
#define CROP_TABLE_STEPS 128
#define CROP_QUEUE_EVENT 21
/* DNGMetaDataWriter::TiffEntryDataSetter: RAW image info and shot metadata. */
#define DNG_SETTER_INFO 0x8c
#define DNG_SETTER_SHOT 0x94
#define SHOT_FLASH_ON 0xf8
#define SHOT_FLASH_MODE 0x780
#define SHOT_FLASH_FIRED 0x71b08
#define SHOT_PROGRAM (SHOT_SETTINGS + 0x15a)
#define SHOT_PROGRAM_CODES 2,4,3,0,1,1,1,2
#define SHOT_METERING (SHOT_SETTINGS + 0x8f)

/* The mode dial (runtime/mode_dial.h). The switch model of the App
   (DIAL_SWITCHES, build header) keeps the state of each switch in a byte at
   DIAL_SWITCH_STATES + 8 x its ID. The mode dial is switch DIAL_SWITCH: P 3,
   Tv 5, Av 6, M 8, Sn 9, and U1 to U3 from DIAL_CODE_U1. The AE model
   (DIAL_AE_MODEL) applies a position: it keeps the code of the applied
   position at DIAL_AE_APPLIED and its user mode (1 to 3, 0 for another
   position) at DIAL_AE_USER_MODE, then sets the user mode model, whose
   holder (DIAL_USER_MODES) points to its state with the loaded user mode in
   the first byte. The settings of a position are loaded when the AE model
   applied it and the user mode model holds its user mode. */
#define DIAL_SWITCH_STATES 0x114
#define DIAL_SWITCH 39
#define DIAL_CODE_U1 11U
#define DIAL_AE_APPLIED 0x13
#define DIAL_AE_USER_MODE 0x14
#define DIAL_POSITION() BYTE(DIAL_SWITCHES,DIAL_SWITCH_STATES+DIAL_SWITCH*8)
#define DIAL_NATIVE_SETTLED(position) \
    (BYTE(DIAL_AE_MODEL,DIAL_AE_APPLIED)==(position) && \
     BYTE(WORD(DIAL_USER_MODES,0),0)==BYTE(DIAL_AE_MODEL,DIAL_AE_USER_MODE))
/* The native slots of the user modes (runtime/mode_dial.h): user mode u
   has slot u. The settings object holds the current settings record at +4,
   and at DIAL_RECORD_COPY (build header) the copy of that record that the
   firmware keeps, which holds the same values: the firmware copies the
   record there each time it loads a whole record. The core reads the names
   in the copy: the name of user mode u is at DIAL_USER_NAME + 0x21 x (u -
   1), ASCII of at most 32 characters with 0 after it, and whether it is
   registered at DIAL_USER_REGIST + u - 1.
   - The assign copies the current settings into slot u
     (FW_USER_MODE_ASSIGN); the native Assign User Mode Dial calls it too,
     also when the person enters no name, and the hook of the save is at its
     entry.
   - The setter of a name (FW_USER_MODE_NAME) also sets the registered
     flag.
   DIAL_SHARED_CODES and DIAL_SHARED_NAMES give P, Av, Tv, Sn, and M, the
   positions that share their settings, in the order of the Mode Dial page. */
#define DIAL_SHARED_CODES 3U,6U,5U,9U,8U
#define DIAL_SHARED_NAMES {'P',0},{'A','v',0},{'T','v',0},{'S','n',0},{'M',0}
#define DIAL_USER_NAME 0x794U
#define DIAL_USER_NAME_STRIDE 0x21U
#define DIAL_USER_REGIST 0x8bdU
#define DIAL_SLOT_NAME_AT(slot) (CALL0(FW_SETTINGS)()+DIAL_RECORD_COPY+DIAL_USER_NAME+DIAL_USER_NAME_STRIDE*((slot)-1U))
#define DIAL_SLOT_REGIST_AT(slot) (CALL0(FW_SETTINGS)()+DIAL_RECORD_COPY+DIAL_USER_REGIST+(slot)-1U)
#define DIAL_SLOT_OF(u) (u)
#define DIAL_SLOT_SAVE(slot) CALL2(FW_USER_MODE_ASSIGN)(DIAL_USER_MODES,(slot))
#define DIAL_SLOT_NAME_SET(slot,name,registered) \
    CALL4(FW_USER_MODE_NAME)(CALL0(FW_USER_MODE_NAMES)(),(slot),(u32)(name),(registered))

/* The canvas of CORE.BORDER (runtime/canvas_shot.h). A still develops in
   StillMake into the bank of its recipe R, as on GR III: R2Y writes the
   source picture in stripes, then BackResize makes the main picture and the
   previews, which the JPEG encode takes. Each picture is a 32-byte
   descriptor in R: +4 Y, +8 interleaved CbCr (4:2:2), +0x10 u16 width and
   height, +0x14 u16 pitch and rows. The R2Y hook gets the StillMake module
   and the develop config, R + CANVAS_R2Y_RECIPE; the BackResize hook gets
   its module and its config, R + CANVAS_RESIZE_RECIPE. */
#define CANVAS_R2Y_RECIPE 0xa80
#define CANVAS_R2Y_MODULE_REG r0
#define CANVAS_R2Y_MODULE_BIAS 0
#define CANVAS_R2Y_CONFIG_REG r1
#define CANVAS_RESIZE_RECIPE 0xa18
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
   (the key of the shot ring). The bank has room for a canvas of about 61 MB:
   at the full 3:2 size, margins up to about 4.6% of the short side. A larger
   canvas takes its planes from the image pool of kind CANVAS_POOL_KIND
   (IMAGE2, 1 GB) of the firmware allocator, while the largest free block
   keeps CANVAS_POOL_RESERVE bytes after it. */
#define CANVAS_BANK 0x368
#define CANVAS_BANK_SIZE 0x3cc
#define CANVAS_SHOT 0xab4
#define CANVAS_POOL_KIND 1U
#define CANVAS_POOL_RESERVE (256U<<20)
/* The source starts this many rows after the main picture in the bank. */
#define CANVAS_SOURCE_ROWS 144U
/* The develop config: the main Y, the source Y and CbCr, then the Y and
   CbCr of the R2Y resize output. The module: the stripe descriptor of R2Y,
   with its Y and CbCr, and its u16 output pitch. The firmware sets the
   first stripe before the hook and the others from the develop config. */
#define CANVAS_CONFIG_MAIN 0x10
#define CANVAS_CONFIG_SOURCE 0x18
#define CANVAS_CONFIG_QUICK 0x20
#define CANVAS_MODULE_SOURCE 0x93bc
#define CANVAS_MODULE_PITCH 0x93cc
/* A second output pitch of the module, a word, which the hook sets with the
   first one. */
#define CANVAS_MODULE_PITCH_WORD 0x95e4
/* The playback and the review need each preview at the size of its box. */
#define CANVAS_PREVIEW_BOX 1
/* The review after a shot shows the screennail for a moment, then a picture
   that the playback resizes from a copy of the R2Y source descriptor, which
   it can take before or after the encode starts. The hook canvas_review
   gives that copy the whole canvas. (On GR III the review resizes the main
   picture, which the encode descriptors give.) */
#define CANVAS_REVIEW 1
/* FW_CANVAS_FILL(0, descriptor, color, 1) fills the rectangle of a
   descriptor through the IIP and returns when it is done; the last argument
   leaves out its cache maintenance. Its descriptor adds +0 format (1 for
   4:2:2), +0x18 u16 x and y, and +0x1c tag 2. */
#define CANVAS_FILL_FORMAT 1U
#define CANVAS_FILL_TAG 2U
#define CANVAS_FILL_BLACK 1U
#define CANVAS_FILL_WHITE 2U

/* The bytes of the path buffer of the ending screen service, which gives the
   picture of the power-off in the builds without FW_GOODBYE_INDEXES
   (runtime/goodbye.h). */
#define FW_GOODBYE_PATH 0x40U

#endif
