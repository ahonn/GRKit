/* RICOH GR III 2.10 (also GR IIIx 1.60): firmware entry points, data, and
   object layouts that the runtime uses. The addresses are those of the
   pinned 2.10 image. Each is a FIRMWARE() symbol that the installer
   relocates, as on GR IV. gr3.h has the values that the shared runtime takes
   from each family. */
#ifndef GRKIT_TARGET_H
#define GRKIT_TARGET_H

#include "gr3.h"

/* Camera settings and the still aspect property. */
#define FW_SETTINGS FIRMWARE(0x5320b31c)
#define FW_STILL_ASPECT_GET FIRMWARE(0x53245260)
#define FW_STILL_ASPECT_SET FIRMWARE(0x532525a4)
/* ShootingModel singleton, and the two requests with which a controller that
   needs no live view stops it (Controller live-view dispatch 0x5316d9b4,
   state 1). Each takes the model and a completion callback. */
#define FW_SHOOTING_MODEL FIRMWARE(0x5320b19c)
#define FW_LIVE_VIEW_END FIRMWARE(0x53238b40)
#define FW_LIVE_VIEW_STOP FIRMWARE(0x53238724)
#define FW_CAPTURE_PROPERTIES FIRMWARE(0x53525d5c)
/* Settings domain: +0x518 -> control -> payload with the native Image Control
   type at +0x77c. Standard is type 11. */
#define FW_SETTINGS_DOMAIN FIRMWARE(0x553458b0)
#define SETTINGS_IC_TYPE 0x77c
/* The native monochrome types, as a bit mask of the type: 13 to 16 are
   Monotone, Soft Monotone, Hard Monotone, and Hi-Contrast B&W (ImageTone 258
   to 261, table 0x53d2b6f4). The shot settings hold the type at the same
   offset as the settings payload. */
#define NATIVE_MONOCHROME_TYPES 0x0001e000U

/* Allocation and the vector helpers of View children. */
#define FW_ALLOCATE FIRMWARE(0x536af2a8)
#define FW_VECTOR_INSERT FIRMWARE(0x536b6de4)
#define FW_VECTOR_ERASE FIRMWARE(0x536b6f68)

/* View classes. */
#define FW_VIEW_SET_TOUCH FIRMWARE(0x533abeb0)
#define FW_VIEW_SET_PARENT FIRMWARE(0x533ac120)
#define FW_VIEW_SET_TOUCHABLE FIRMWARE(0x533ac1b4)
#define FW_VIEW_SET_X FIRMWARE(0x533ac568)
#define FW_VIEW_INVALIDATE FIRMWARE(0x533ad308)
/* ScreenDevice::OnSetLevelAlarm(screen, mode): sets the live-view highlight warning. */
#define FW_LEVEL_ALARM FIRMWARE(0x536ce030)
/* LiveViewController::UpdateLevelAlarm(controller, 1): sets the warning again
   while LIVEVIEW_LEVEL_PENDING is set. */
#define FW_LIVEVIEW_LEVEL_UPDATE FIRMWARE(0x5317645c)
/* The electronic level: a static object whose second word points to the
   level, or holds 0 before the level exists (runtime/processing.h). */
#define FW_LEVEL FIRMWARE(0x5536e7f0)
#define FW_VIEW_SET_VISIBLE FIRMWARE(0x533ad3c0)
#define FW_LINE_VIEW_INIT FIRMWARE(0x53393d84)
#define FW_LINE_VIEW_SET_STYLE FIRMWARE(0x53393e00)
#define FW_LINE_VIEW_DESTROY FIRMWARE(0x53393d54)
#define FW_ICON_VIEW_INIT FIRMWARE(0x5339a908)
#define FW_ICON_VIEW_SET_ICON FIRMWARE(0x5339a9a0)
#define FW_ICON_VIEW_DESTROY FIRMWARE(0x5339a874)
#define FW_STRING_VIEW_RESET FIRMWARE(0x53396f80)
/* The width of the text of a string view, in pixels. */
#define FW_STRING_VIEW_WIDTH FIRMWARE(0x53396dc8)
#define FW_LIST_SET_COUNT FIRMWARE(0x53394e8c)
#define FW_LIST_ROW FIRMWARE(0x53394758)
#define FW_LIST_SAVE FIRMWARE(0x533947f0)
#define FW_LIST_CURSOR FIRMWARE(0x53394820)
#define FW_LIST_WIDTH FIRMWARE(0x533948c4)
#define FW_LIST_RESTORE FIRMWARE(0x53395224)
#define FW_LIST_SET_CURSOR FIRMWARE(0x53395430)
#define FW_LIST_MOVE FIRMWARE(0x53395260)
/* The list\'s row vector (first and end pointers) and its shown row count. */
#define LIST_ROWS 0xf4
#define LIST_COUNT 0x108
#define FW_ROW_SET_ICON FIRMWARE(0x533a1734)
#define FW_ROW_SET_CHECK FIRMWARE(0x533a19b4)

/* AdjModeController: methods and the touch callback pair of a toolbar icon. */
#define FW_ADJ_SNAPSHOT FIRMWARE(0x531a0150)
#define FW_ADJ_TOUCH_ENTER FIRMWARE(0x531a16c4)
#define FW_ADJ_TOUCH_LEAVE FIRMWARE(0x531a1874)
#define FW_ADJ_SLOT_TYPE FIRMWARE(0x531a2820)
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
#define FW_SUBMENU_KINDS FIRMWARE(0x53b3fb10)
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
#define SELECT_GUIDE_OK_TEXT 0x301
#define SELECT_GUIDE_BACK_TEXT 0x302
/* Row text setters: a value page row sets the string view at +0xf8, and a
   submenu row its value view at +0xfc. Each sets a text ID. */
#define FW_ROW_TEXT FIRMWARE(0x5339e7a0)
/* Options pages show checkboxes (runtime/menu_settings.h): the constructor of
   SelectMenuController, the vtable and SetItem of CheckBoxListLineView, the
   ScrollListView init, the radio dot of a SelectMenuLineView, and the checked
   and unchecked images. */
#define FW_SELECT_MENU_NEW FIRMWARE(0x531be784)
#define CHECK_LINE_VTABLE FIRMWARE(0x53c0bb60)
#define FW_CHECK_ITEM FIRMWARE(0x533a8938)
#define FW_LIST_INIT FIRMWARE(0x53394610)
#define FW_ROW_DOT FIRMWARE(0x5339e738)
#define CHECK_IMAGE_ON 0x10fU
#define CHECK_IMAGE_OFF 0x10eU
#define ROW_TEXT_VIEW 0xf8
#define FW_ROW_VALUE FIRMWARE(0x533a6ad0)
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
#define FW_MENU_ROW_LABEL FIRMWARE(0x5339dc70)
#define FW_MENU_ROW_VALUE FIRMWARE(0x5339de6c)
#define MENU_ROW_VALUE_VIEW 0xf8
/* A native view keeps its frame {x, y, width, height} at VIEW_FRAME, and its
   vtable sets it at VIEW_SET_FRAME. The value of a MENU or submenu row shows
   in a column of VALUE_COLUMN pixels from the x of the view. */
#define VIEW_FRAME 0x20
#define VIEW_SET_FRAME 0x18
#define VALUE_COLUMN 140
#define FW_ADJ_REFRESH FIRMWARE(0x531a4114)
#define FW_ADJ_SELECT_SLOT FIRMWARE(0x531a4864)
#define FW_CHANGE_CONTROLLER FIRMWARE(0x5316c9d8)
/* The controller factory singleton, which holds one instance of each controller. */
/* The camera information page (Setup tab, Firmware Info/Options), which
   About GRKit opens: its controller, the text ID of its title, and its two
   lines, ASCII string views that the page update sets (menu_info_update)
   with FW_STRING_VIEW_SET_ASCII(view, text). */
#define INFO_CONTROLLER 0x200011aU
#define INFO_TITLE_TEXT 0x28d
#define INFO_LINES {0x168,0x16c}
#define FW_STRING_VIEW_SET_ASCII FIRMWARE(0x53390ef8)
#define FW_CONTROLLER_FACTORY FIRMWARE(0x5316f350)
/* AdjModeController fields. */
#define ADJ_CANVAS 0x4
/* The current UI settings object is at this slot, +4 in the object
   0x55063f30, which firmware code loads. LANGUAGE_SETTING is its UI language
   byte (reader 0x532465dc): 18 is Traditional Chinese and 19 Simplified
   Chinese. */
#define UI_SETTINGS_SLOT (FIRMWARE(0x55063f30) + 4)
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
#define FW_GUIDE_SET_CONTENT FIRMWARE(0x5339b7d4)
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
#define FW_PLAYBACK_METADATA FIRMWARE(0x5320b97c)
#define METADATA_TONE 0x220
#define FW_ICON_TABLE FIRMWARE(0x550020fc)
/* The native Off icon, used until the installer provides the GRKit one. */
#define NATIVE_OFF_ICON() WORD(FW_ICON_TABLE,0x166*4)
/* Maker-note IFD entry setter: (entry, 0, value) stores a u16 value. */
#define FW_IFD_SET_SHORT FIRMWARE(0x535d02c8)
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
#define FW_YCBCR_CALC FIRMWARE(0x536e4720)
/* DigitalFilterProcess plan: the grain stage flag that the stage gate sets. */
#define GRAIN_GATE_FLAG 0x24
/* The granulation after its amplitude choice (grain_amplitude). */
#define FW_GRAIN_AMPLITUDE_DONE FIRMWARE(0x5367ae40)
/* Grain size. The shot settings hold the float APEX Sv of the shot at
   GRAIN_SV, which the EXIF ISO also uses (0x535f5e80). The granulation
   (0x5367ad3c) keeps the pitch and the height of its noise plane at
   fp - GRAIN_FILL_PITCH and fp - GRAIN_FILL_HEIGHT when it calls the noise
   fill FW_GRAIN_NOISE (buffer, pitch x height), which writes a random byte
   for each sample. FW_RELEASE frees a block of FW_ALLOCATE. */
#define GRAIN_SV 0xb68
#define GRAIN_FILL_PITCH 0x650
#define GRAIN_FILL_HEIGHT 0x644
#define FW_GRAIN_NOISE FIRMWARE(0x5368a92c)
#define FW_RELEASE FIRMWARE(0x536af2e4)
/* The system counter behind the generic timer of the cores runs at 40 MHz
   (the GR III device tree and kernel log). Only Linux writes CNTFRQ, so the
   RTOS cores read 0 there. */
#define SYSTEM_COUNTER_MHZ 40U
#define FW_CACHE_FLUSH FIRMWARE(0x5383c8a8)
#define FW_CLOCK FIRMWARE(0x536bd838)
#define FW_LOCAL_TIME FIRMWARE(0x536b6510)

/* The UI service (runtime/ui.h): the view factory, the App task timers,
   the message box, its progress bar, and the texts of its buttons. */
#define FW_VIEW_FACTORY FIRMWARE(0x533ad7f0)
#define FW_TIMER_SERVICE FIRMWARE(0x5320b01c)
#define FW_TIMER_START FIRMWARE(0x5322b468)
#define FW_TIMER_CANCEL FIRMWARE(0x5322b1ec)
#define FW_MESSAGE_BACKGROUND FIRMWARE(0x5339c71c)
#define FW_MESSAGE_SET FIRMWARE(0x5339c844)
#define FW_MESSAGE_BUTTONS FIRMWARE(0x5339ca54)
#define FW_MESSAGE_ENABLE_BUTTONS FIRMWARE(0x5339cbd8)
#define FW_MESSAGE_CURSOR FIRMWARE(0x5339cad0)
#define FW_MESSAGE_GET_CURSOR FIRMWARE(0x5339d190)
#define FW_PROGRESS_PERCENT FIRMWARE(0x5339a2d8)
#define UI_EXECUTE_TEXT 0x2a9
#define UI_CANCEL_TEXT 0x2aa
/* The slider of ui_slider: the setters of a slider configuration (init,
   range, and one flag) and its apply to a SliderView, and the icons that the
   native sliders use. */
#define FW_SLIDER_INIT FIRMWARE(0x5339f194)
#define FW_SLIDER_RANGE FIRMWARE(0x5339f1f0)
#define FW_SLIDER_FLAG FIRMWARE(0x5339f210)
#define FW_SLIDER_APPLY FIRMWARE(0x5339f328)
#define UI_SLIDER_ICONS {0x230,0x40,0xc7,0x240}

#endif
