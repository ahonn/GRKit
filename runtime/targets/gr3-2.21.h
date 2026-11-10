/* RICOH GR III 2.21 (also GR IIIx 1.71): firmware entry points, data, and
   object layouts that the runtime uses. The addresses are those of the
   pinned 2.21 image. Each is a FIRMWARE() symbol that the installer
   relocates, as on GR IV. gr3.h has the values that the shared runtime takes
   from each family. The values are those of 2.10, except the text IDs from
   0x2a1, which 2.21 moves by 7 when it adds its reset texts. Not tested on a
   camera. */
#ifndef GRKIT_TARGET_H
#define GRKIT_TARGET_H

#include "gr3.h"

/* Camera settings and the still aspect property. */
#define FW_SETTINGS FIRMWARE(0x5320d6f4)
#define FW_STILL_ASPECT_GET FIRMWARE(0x532479e8)
#define FW_STILL_ASPECT_SET FIRMWARE(0x53254d2c)
/* ShootingModel singleton, and the two requests with which a controller that
   needs no live view stops it (Controller live-view dispatch 0x5316ea00,
   state 1). Each takes the model and a completion callback. */
#define FW_SHOOTING_MODEL FIRMWARE(0x5320d574)
#define FW_LIVE_VIEW_END FIRMWARE(0x5323b28c)
#define FW_LIVE_VIEW_STOP FIRMWARE(0x5323ae70)
#define FW_CAPTURE_PROPERTIES FIRMWARE(0x5352879c)
/* Settings domain: +0x518 -> control -> payload with the native Image Control
   type at +0x77c. Standard is type 11. */
#define FW_SETTINGS_DOMAIN FIRMWARE(0x55345b80)
#define SETTINGS_IC_TYPE 0x77c
/* The native monochrome types, as a bit mask of the type: 13 to 16 are
   Monotone, Soft Monotone, Hard Monotone, and Hi-Contrast B&W (ImageTone 258
   to 261, table 0x53d33624). The shot settings hold the type at the same
   offset as the settings payload. */
#define NATIVE_MONOCHROME_TYPES 0x0001e000U
/* The digital filter values of the shot settings, which the grain stage of
   recorded pictures reads: Shading and Clarity (0 to 8, 4 is neutral),
   Particle (0 is no grain), and the HDR Tone strength (0 is off). The native
   parameter build writes them for its Image Control type. */
#define SETTINGS_SHADING 0x793
#define SETTINGS_CLARITY 0x794
#define SETTINGS_GRAIN 0x795
#define SETTINGS_HDR 0x796

/* Allocation and the vector helpers of View children. */
#define FW_ALLOCATE FIRMWARE(0x536b2880)
#define FW_VECTOR_INSERT FIRMWARE(0x536ba3bc)
#define FW_VECTOR_ERASE FIRMWARE(0x536ba540)

/* View classes. */
#define FW_VIEW_SET_TOUCH FIRMWARE(0x533ae8ec)
#define FW_VIEW_SET_PARENT FIRMWARE(0x533aeb5c)
#define FW_VIEW_SET_TOUCHABLE FIRMWARE(0x533aebf0)
#define FW_VIEW_SET_X FIRMWARE(0x533aefa4)
#define FW_VIEW_INVALIDATE FIRMWARE(0x533afd44)
/* ScreenDevice::OnSetLevelAlarm(screen, mode): sets the live-view highlight warning. */
#define FW_LEVEL_ALARM FIRMWARE(0x536d2274)
/* LiveViewController::UpdateLevelAlarm(controller, 1): sets the warning again
   while LIVEVIEW_LEVEL_PENDING is set. */
#define FW_LIVEVIEW_LEVEL_UPDATE FIRMWARE(0x531775c4)
/* The electronic level: a static object whose second word points to the
   level, or holds 0 before the level exists (runtime/processing.h). */
#define FW_LEVEL FIRMWARE(0x5536e388)
#define FW_VIEW_SET_VISIBLE FIRMWARE(0x533afdfc)
#define FW_LINE_VIEW_INIT FIRMWARE(0x533967b4)
#define FW_LINE_VIEW_SET_STYLE FIRMWARE(0x53396830)
#define FW_LINE_VIEW_DESTROY FIRMWARE(0x53396784)
#define FW_ICON_VIEW_INIT FIRMWARE(0x5339d344)
#define FW_ICON_VIEW_SET_ICON FIRMWARE(0x5339d3dc)
#define FW_ICON_VIEW_DESTROY FIRMWARE(0x5339d2b0)
#define FW_STRING_VIEW_RESET FIRMWARE(0x533999ac)
/* The width of the text of a string view, in pixels. */
#define FW_STRING_VIEW_WIDTH FIRMWARE(0x533997f4)
#define FW_LIST_SET_COUNT FIRMWARE(0x533978bc)
#define FW_LIST_ROW FIRMWARE(0x53397188)
#define FW_LIST_SAVE FIRMWARE(0x53397220)
#define FW_LIST_CURSOR FIRMWARE(0x53397250)
#define FW_LIST_WIDTH FIRMWARE(0x533972f4)
#define FW_LIST_RESTORE FIRMWARE(0x53397c54)
#define FW_LIST_SET_CURSOR FIRMWARE(0x53397e60)
#define FW_LIST_MOVE FIRMWARE(0x53397c90)
/* The list\'s row vector (first and end pointers) and its shown row count. */
#define LIST_ROWS 0xf4
#define LIST_COUNT 0x108
#define FW_ROW_SET_ICON FIRMWARE(0x533a4170)
#define FW_ROW_SET_CHECK FIRMWARE(0x533a43f0)

/* AdjModeController: methods and the touch callback pair of a toolbar icon. */
#define FW_ADJ_SNAPSHOT FIRMWARE(0x531a117c)
#define FW_ADJ_TOUCH_ENTER FIRMWARE(0x531a26f0)
#define FW_ADJ_TOUCH_LEAVE FIRMWARE(0x531a28a0)
#define FW_ADJ_SLOT_TYPE FIRMWARE(0x531a3850)
/* The slot type of the native Image Control slot, and the controller of its
   Standard parameter page (still mode). */
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
/* Image Control processing (runtime/processing.S): the three type tables that
   the parameter build loads before the module_processing site, the
   instruction after those loads, and the two gamma calls of the packet tail:
   the reset of the shared gamma helper, then the special type-16 path. */
#define FW_IC_TABLE_0 FIRMWARE(0x55018fec)
#define FW_IC_TABLE_1 FIRMWARE(0x55018f48)
#define FW_IC_TABLE_2 FIRMWARE(0x550190a4)
#define FW_PROCESSING_RESUME FIRMWARE(0x536e04ec)
#define FW_PACKET_GAMMA_RESET FIRMWARE(0x536f4304)
#define FW_PACKET_SPECIAL_PATH FIRMWARE(0x536f4174)
/* Toning (runtime/toning.h): base kind 9 of the row lists at 0x53b38f14,
   the rows of Retro, adds Toning, row kind 9, to the Standard rows. Its model
   getter and setter and row read hook as adjustment 9. The page keeps the
   Standard type: the Retro type also sets its own Shading. */
#define PARAMETER_TONING_BASE 9
#define PARAMETER_TONING_KIND 9
/* The page event that opens another controller (0x2000015) from the page. */
#define DETAIL_EV_SUBPAGE 0x88
/* The native settings page of a GRKit root row (runtime/menu_settings.h). It borrows
   the MENU submenu Instant Review (controller 0x20000be, entry 10 of the
   submenu kind table), its three value pages (0x20000c3 Display Time,
   0x20000c4 Zoom Review, 0x20000c5), and two value pages of the MENU Setup
   tab (0x200009f, 0x20000a0), one for each row. Each is a plain SelectMenu
   page: its record has one title and no description. A switch in the
   SelectMenu start maps the page to its record; records 83, 84, 85, 87, 88. A kind table entry holds the row
   count at +6, the row label IDs (u16) at +8, the row controllers (u32) at
   +0x14, a kind for each row (u32, 0x21c for a row with a value page) at
   +0x28, and the row value methods (C++ member pointers, 8 bytes) at +0x3c.
   A row without its kind draws its value over its label.
   A value method gets the submenu and the row. A value page shows the label
   ID of its SelectMenu record as title, which is the label of its row. */
#define FW_SUBMENU_KINDS FIRMWARE(0x53b441c8)
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
/* The guides of a SelectMenu value page: OK and MENU. */
#define SELECT_GUIDE_OK_TEXT 0x308
#define SELECT_GUIDE_BACK_TEXT 0x309
/* Row text setters: a value page row sets the string view at +0xf8, and a
   submenu row its value view at +0xfc. Each sets a text ID. */
#define FW_ROW_TEXT FIRMWARE(0x533a11dc)
/* Options pages show checkboxes (runtime/menu_settings.h): the constructor of
   SelectMenuController, the vtable and SetItem of CheckBoxListLineView, the
   ScrollListView init, the radio dot of a SelectMenuLineView, and the checked
   and unchecked images. */
#define FW_SELECT_MENU_NEW FIRMWARE(0x531bf7c8)
#define CHECK_LINE_VTABLE FIRMWARE(0x53c139b8)
#define FW_CHECK_ITEM FIRMWARE(0x533ab374)
#define FW_LIST_INIT FIRMWARE(0x53397040)
#define FW_ROW_DOT FIRMWARE(0x533a1174)
#define CHECK_IMAGE_ON 0x10fU
#define CHECK_IMAGE_OFF 0x10eU
#define ROW_TEXT_VIEW 0xf8
#define FW_ROW_VALUE FIRMWARE(0x533a950c)
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
#define FW_MENU_TABS FIRMWARE(0x55001eb8)
#define FW_MENU_ROW_LABEL FIRMWARE(0x533a06ac)
#define FW_MENU_ROW_VALUE FIRMWARE(0x533a08a8)
#define MENU_ROW_VALUE_VIEW 0xf8
/* A native view keeps its frame {x, y, width, height} at VIEW_FRAME, and its
   vtable sets it at VIEW_SET_FRAME. The value of a MENU or submenu row shows
   in a column of VALUE_COLUMN pixels from the x of the view. */
#define VIEW_FRAME 0x20
#define VIEW_SET_FRAME 0x18
#define VALUE_COLUMN 140
#define FW_ADJ_REFRESH FIRMWARE(0x531a5144)
#define FW_ADJ_SELECT_SLOT FIRMWARE(0x531a5894)
#define FW_CHANGE_CONTROLLER FIRMWARE(0x5316da14)
/* The controller factory singleton, which holds one instance of each controller. */
/* The camera information page (Setup tab, Firmware Info/Options), which
   About GRKit opens: its controller, the text ID of its title, and its two
   lines, ASCII string views that the page update sets (menu_info_update)
   with FW_STRING_VIEW_SET_ASCII(view, text). */
#define INFO_CONTROLLER 0x200011aU
#define INFO_TITLE_TEXT 0x28d
#define INFO_LINES {0x168,0x16c}
#define FW_STRING_VIEW_SET_ASCII FIRMWARE(0x53393928)
#define FW_CONTROLLER_FACTORY FIRMWARE(0x5317040c)
/* AdjModeController fields. */
#define ADJ_CANVAS 0x4
/* The current UI settings object is at this slot, +4 in the object
   0x55064200, which firmware code loads. LANGUAGE_SETTING is its UI language
   byte (reader 0x53248d64): 18 is Traditional Chinese and 19 Simplified
   Chinese. */
#define UI_SETTINGS_SLOT (FIRMWARE(0x55064200) + 4)
#define LANGUAGE_SETTING 0x9ba
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
/* The ADJ refresh fills the Fn hint only on a native slot that offers Fn. For
   the Image Control slot, it uses the Fn label, layout 10, and the settings
   icon. A new ADJ starts with an empty hint. */
#define FW_GUIDE_SET_CONTENT FIRMWARE(0x5339e210)
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

/* Playback metadata singleton (+0x220 holds key 0x25, the Image Control tag)
   and the native icon table. */
#define FW_PLAYBACK_METADATA FIRMWARE(0x5320dd54)
#define METADATA_TONE 0x220
#define FW_ICON_TABLE FIRMWARE(0x550020fc)
/* The native Off icon, used until the installer provides the GRKit one. */
#define NATIVE_OFF_ICON() WORD(FW_ICON_TABLE,0x166*4)
/* RawDevelopmentModel: the guard of its static instance, and the instance.
   The family header gives its state (DEVELOP_STATE). */
#define FW_DEVELOP_GUARD FIRMWARE(0x550665a0)
#define FW_DEVELOP_MODEL FIRMWARE(0x550669b0)
/* The time of the photo in the playback metadata (runtime/targets/gr3.h). */
#define METADATA_TIME 0x3c0
/* The EXIF Orientation of the photo in the playback metadata. */
#define METADATA_ORIENTATION 0x2b0
/* RAW development, the Image Control list (runtime/menu.c): the label and
   the icon of an Image Control (model, category, value), the child view of a
   list row, the controller ID word, the ID of the list, and the first
   instruction of the row category, which its entry hook replaces. The
   subtitle of the list is at +0x160 of the controller (r4 in UpdateSubTitle),
   the subtitle of RawDevelopmentTopController in r6. */
#define FW_DEVELOP_TONE_TEXT FIRMWARE(0x5328b97c)
#define FW_DEVELOP_TONE_ICON FIRMWARE(0x5328b930)
/* RAW development, the Fn adjustments (runtime/develop.h). RawDevelopmentModel
   holds the selected Image Control: the category at DEVELOP_CATEGORY and the
   value of each category from DEVELOP_VALUES. Each Image Control has a
   parameter set of a byte for each parameter kind; that of Standard is at
   DEVELOP_PARAMETERS. The parameter page of the development edits the set of
   the selected Image Control. */
#define DEVELOP_CATEGORY 0x82
#define DEVELOP_VALUES 0x83
#define DEVELOP_PARAMETERS 0x97
/* RAW development, the aspect list (runtime/menu.c): the row value of the
   model (model, row), the capture aspect of a development aspect (model,
   value), and the label and the icon of a development aspect (model,
   value). */
#define FW_DEVELOP_ASPECT_VALUE FIRMWARE(0x53285b6c)
#define FW_DEVELOP_ASPECT_CAPTURE FIRMWARE(0x5328c614)
#define FW_DEVELOP_ASPECT_TEXT FIRMWARE(0x5328c6c0)
#define FW_DEVELOP_ASPECT_ICON FIRMWARE(0x5328c684)
#define FW_ROW_CHILD FIRMWARE(0x533a4770)
#define CONTROLLER_ID 0x124
#define DEVELOP_TONE_LIST 0x4000025U
/* The ID of the aspect list of the development, and the first instruction
   of the row value of the model, which its entry hook replaces. */
#define DEVELOP_ASPECT_LIST 0x4000022U
#define DEVELOP_ASPECT_VALUE_FIRST "ldrb r3, [r0, #5]"
/* The list view of RawDevelopmentParameterSetController, and the style and
   the height of its row views. */
#define DEVELOP_LIST_VIEW 0x168
#define DEVELOP_ROW_STYLE 0x1aU
#define DEVELOP_ROW_HEIGHT 0x3aU
/* The children of its row views: their maker (row, count, layout), the
   layout, and the setups of the icon and of the text that the list init does. */
#define FW_ROW_CHILDREN FIRMWARE(0x533a4b18)
#define DEVELOP_ROW_LAYOUT FIRMWARE(0x53b37cc8)
#define FW_ICON_VIEW_STYLE FIRMWARE(0x5339d46c)
#define FW_STRING_VIEW_STYLE FIRMWARE(0x53393a28)
#define DEVELOP_CATEGORY_FIRST "ldrb r3, [r0, #5]"
#define DEVELOP_LIST_TITLE ldr r3, [r4, #0x160]
#define DEVELOP_TOP_TITLE mov r3, r6
/* Maker-note IFD entry setter: (entry, 0, value) stores a u16 value. */
#define FW_IFD_SET_SHORT FIRMWARE(0x535d2e30)
/* DNG LONG tag element writer and native sensor rectangle helper. */
#define FW_IFD_SET_LONG FIRMWARE(0x535d2e48)
#define FW_DNG_NATIVE_RECT FIRMWARE(0x536626c4)
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
#define FW_YCBCR_CALC FIRMWARE(0x536e8960)
/* DigitalFilterProcess plan: the grain stage flag that the stage gate sets. */
#define GRAIN_GATE_FLAG 0x24
/* The granulation after its amplitude choice (grain_amplitude). */
#define FW_GRAIN_AMPLITUDE_DONE FIRMWARE(0x5367e418)
/* Grain size. The shot settings hold the float APEX Sv of the shot at
   GRAIN_SV, which the EXIF ISO also uses (0x535f89e8). The granulation
   (0x5367e314) keeps the pitch and the height of its noise plane at
   fp - GRAIN_FILL_PITCH and fp - GRAIN_FILL_HEIGHT when it calls the noise
   fill FW_GRAIN_NOISE (buffer, pitch x height), which writes a random byte
   for each sample. FW_RELEASE frees a block of FW_ALLOCATE. */
#define GRAIN_SV 0xb68
#define GRAIN_FILL_PITCH 0x650
#define GRAIN_FILL_HEIGHT 0x644
#define FW_GRAIN_NOISE FIRMWARE(0x5368df04)
#define FW_RELEASE FIRMWARE(0x536b28bc)
/* The system counter behind the generic timer of the cores runs at 40 MHz
   (the GR III device tree and kernel log). Only Linux writes CNTFRQ, so the
   RTOS cores read 0 there. */
#define SYSTEM_COUNTER_MHZ 40U
#define FW_CACHE_FLUSH FIRMWARE(0x53840cc0)
#define FW_CLOCK FIRMWARE(0x536c0f2c)
#define FW_LOCAL_TIME FIRMWARE(0x536b9ae8)

/* The UI service (runtime/ui.h): the view factory, the App task timers,
   the message box, its progress bar, and the texts of its buttons. */
#define FW_VIEW_FACTORY FIRMWARE(0x533b022c)
#define FW_TIMER_SERVICE FIRMWARE(0x5320d3f4)
#define FW_TIMER_START FIRMWARE(0x5322dbb8)
#define FW_TIMER_CANCEL FIRMWARE(0x5322d93c)
#define FW_MESSAGE_BACKGROUND FIRMWARE(0x5339f158)
#define FW_MESSAGE_SET FIRMWARE(0x5339f280)
#define FW_MESSAGE_BUTTONS FIRMWARE(0x5339f490)
#define FW_MESSAGE_ENABLE_BUTTONS FIRMWARE(0x5339f614)
#define FW_MESSAGE_CURSOR FIRMWARE(0x5339f50c)
#define FW_MESSAGE_GET_CURSOR FIRMWARE(0x5339fbcc)
#define FW_PROGRESS_PERCENT FIRMWARE(0x5339cd14)
#define UI_EXECUTE_TEXT 0x2b0
#define UI_CANCEL_TEXT 0x2b1
/* The slider of ui_slider: the setters of a slider configuration (init,
   range, and one flag) and its apply to a SliderView, and the icons that the
   native sliders use. */
#define FW_SLIDER_INIT FIRMWARE(0x533a1bd0)
#define FW_SLIDER_RANGE FIRMWARE(0x533a1c2c)
#define FW_SLIDER_FLAG FIRMWARE(0x533a1c4c)
#define FW_SLIDER_APPLY FIRMWARE(0x533a1d64)
#define UI_SLIDER_ICONS {0x230,0x40,0xc7,0x240}

#endif
