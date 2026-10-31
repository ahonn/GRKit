/* RICOH GR IV 1.20 (also GR IV HDF), the build of 2026-05-28 that GR IV 30th
   Anniversary Edition cameras have from the factory: the firmware addresses
   and the values of that build. gr4.h has the layouts and constants of each
   GR IV build and describes what each address is. The addresses are those of
   a dump of a camera; each address is a FIRMWARE() symbol that the installer
   relocates. Data that no firmware code loads directly is an offset from an
   address that it does load. The menus, the settings texts, and the settings
   objects are those of gr4-1.20. The processing function, the Image Control
   types, and the processing slots are those of 1.11. The Fn hint IDs are one
   lower than on gr4-1.20, and the icon draw takes `bhs` at its limit. Not
   tested on a camera. */
#ifndef GRKIT_TARGET_H
#define GRKIT_TARGET_H

#include "gr4.h"

/* The icon draw compares the icon ID with this native limit as an immediate,
   and IDs from the limit are out of range (gr4-1.20: above the limit). */
#define NATIVE_ICON_LIMIT 0x2c4

/* Camera settings, the still aspect getter, and the settings domain after the
   vtable of the capture property singleton. */
#define FW_SETTINGS FIRMWARE(0x5324003c)
#define FW_STILL_ASPECT_GET FIRMWARE(0x5329f7d8)
#define FW_SETTINGS_DOMAIN (FIRMWARE(0x5537b07c) + 4)
/* The native monochrome types, as a bit mask of the type: 13 to 16 as on
   GR III, and 25 to 30 as on 1.11 (ImageTone 0x8100 to 0x8105, table
   0x53fd0000). This build has no type 31 or 32. */
#define NATIVE_MONOCHROME_TYPES 0x7e01e000U
/* The capture aspect getter, which the live-view mode function 0x538ad808 and
   the mode check 0x5365f164 call, and the still aspect resend. */
#define FW_CAPTURE_ASPECT_GET FIRMWARE(0x537785f8)
#define FW_STILL_ASPECT_RESEND FIRMWARE(0x53285744)
#define FW_STILL_ASPECT_SET FIRMWARE(0x532fc78c)
/* The size classifier is 0x533b0144. */

/* Allocation and the vector helpers of View children. */
#define FW_ALLOCATE FIRMWARE(0x53901bec)
#define FW_VECTOR_INSERT FIRMWARE(0x5390c640)
#define FW_VECTOR_ERASE FIRMWARE(0x5390c7c4)

/* View classes. */
#define FW_VIEW_SET_TOUCH FIRMWARE(0x535a76ac)
#define FW_VIEW_SET_PARENT FIRMWARE(0x535a791c)
#define FW_VIEW_SET_TOUCHABLE FIRMWARE(0x535a79b0)
#define FW_VIEW_SET_X FIRMWARE(0x535a7d64)
#define FW_VIEW_INVALIDATE FIRMWARE(0x535a8b04)
/* ScreenDevice::OnSetLevelAlarm(screen, mode): sets the live-view highlight warning. */
#define FW_LEVEL_ALARM FIRMWARE(0x5392463c)
/* LiveViewController::UpdateLevelAlarm(controller, 1): sets the warning again
   while LIVEVIEW_LEVEL_PENDING is set. */
#define FW_LIVEVIEW_LEVEL_UPDATE FIRMWARE(0x5318eb5c)
/* The electronic level: a static object whose second word points to the
   level, or holds 0 before the level exists (runtime/processing.h). */
#define FW_LEVEL FIRMWARE(0x553a2f44)
#define FW_VIEW_SET_VISIBLE FIRMWARE(0x535a8bbc)
#define FW_LINE_VIEW_INIT FIRMWARE(0x5358f1c0)
#define FW_LINE_VIEW_SET_STYLE FIRMWARE(0x5358f23c)
#define FW_LINE_VIEW_DESTROY FIRMWARE(0x5358f190)
#define FW_ICON_VIEW_INIT FIRMWARE(0x53595ea4)
#define FW_ICON_VIEW_SET_ICON FIRMWARE(0x53595f3c)
#define FW_ICON_VIEW_DESTROY FIRMWARE(0x53595e10)
#define FW_STRING_VIEW_RESET FIRMWARE(0x5359248c)
/* The width of the text of a string view, in pixels. */
#define FW_STRING_VIEW_WIDTH FIRMWARE(0x535922d4)
#define FW_LIST_SET_COUNT FIRMWARE(0x535902e4)
#define FW_LIST_ROW FIRMWARE(0x5358fb9c)
#define FW_LIST_SAVE FIRMWARE(0x5358fc3c)
#define FW_LIST_CURSOR FIRMWARE(0x5358fc6c)
#define FW_LIST_WIDTH FIRMWARE(0x5358fd10)
#define FW_LIST_RESTORE FIRMWARE(0x5359067c)
#define FW_LIST_SET_CURSOR FIRMWARE(0x53590888)
#define FW_LIST_MOVE FIRMWARE(0x535906b8)
#define FW_ROW_SET_ICON FIRMWARE(0x5359ce9c)
#define FW_ROW_SET_CHECK FIRMWARE(0x5359d11c)

/* AdjModeController: methods and the touch callback pair of a toolbar icon. */
#define FW_ADJ_TOUCH_ENTER FIRMWARE(0x531bc394)
#define FW_ADJ_TOUCH_LEAVE FIRMWARE(0x531bc4c0)
#define FW_ADJ_SLOT_TYPE FIRMWARE(0x531be150)
#define FW_ADJ_SNAPSHOT FIRMWARE(0x531bd9c0)
/* The settings page: the MENU submenu kind table, the text IDs of Instant
   Review, of the five value pages and of the sixth row, the OK and MENU
   guides of a value page (those of gr4-1.20), and the row text
   setters. */
#define FW_SUBMENU_KINDS FIRMWARE(0x53dc9520)
#define SETTINGS_TITLE_TEXT 0x228
#define SETTINGS_ROW_TEXTS {0x22a,0x22c,0x1a8,0x1c8,0x1ce,0x1f2}
#define SELECT_GUIDE_OK_TEXT 0x364
#define SELECT_GUIDE_BACK_TEXT 0x365
#define FW_ROW_TEXT FIRMWARE(0x53599ee4)
/* Options pages show checkboxes (runtime/menu_settings.h): the constructor of
   SelectMenuController, the vtable and SetItem of CheckBoxListLineView, the
   ScrollListView init, the radio dot of a SelectMenuLineView, and the checked
   and unchecked images. */
#define FW_SELECT_MENU_NEW FIRMWARE(0x531dc07c)
#define CHECK_LINE_VTABLE FIRMWARE(0x53ec80c8)
#define FW_CHECK_ITEM FIRMWARE(0x535a4138)
#define FW_LIST_INIT FIRMWARE(0x5358fa4c)
/* Gives a list value to each of its rows; the ADJ list init passes 0x10.
   GR III keeps it in the list, so it needs no call for added rows. */
#define FW_LIST_ROWS_VALUE FIRMWARE(0x53590d34)
#define FW_ROW_DOT FIRMWARE(0x53599e44)
#define CHECK_IMAGE_ON 0x110U
#define CHECK_IMAGE_OFF 0x10fU
#define FW_ROW_VALUE FIRMWARE(0x535a22ec)
/* The RAM table of the MENU tab descriptors, and the MENU row setters of
   the label and of the value text. */
#define FW_MENU_TABS FIRMWARE(0x55001e78)
/* The rows of the stock Customize tab (runtime/menu_section.h). */
#define MENU_TAB_STOCK_ROWS 24
#define FW_MENU_ROW_LABEL FIRMWARE(0x535992c4)
#define FW_MENU_ROW_VALUE FIRMWARE(0x53599578)
#define FW_ADJ_REFRESH FIRMWARE(0x531bff98)
#define FW_ADJ_SELECT_SLOT FIRMWARE(0x531c051c)
#define FW_CHANGE_CONTROLLER FIRMWARE(0x53183a4c)
/* The controller factory singleton, which holds one instance of each controller. */
#define INFO_TITLE_TEXT 0x2d5
#define INFO_LABEL_TEXTS {0x2d7,0x2d9,0x2db}
#define FW_STRING_VIEW_SET_ASCII FIRMWARE(0x5358c2c4)
#define FW_CONTROLLER_FACTORY FIRMWARE(0x53186b14)
/* The UI language byte of the static settings object 0x55085cd8 (reader
   0x5329e360, +0x14ef as on gr4-1.20), with the GR III values. Not confirmed
   on a camera. */
#define LANGUAGE_SETTING_BYTE (FIRMWARE(0x55085cd8) + 0x14ef)
/* The live-view display mode that DISP cycles, in the same object (reader
   0x5329e054, +0x1461 as on gr4-1.20), with the GR III values: 0 full,
   1 simple, 2 no information, 3 display off. */
#define DISPLAY_MODE_BYTE (FIRMWARE(0x55085cd8) + 0x1461)
#define FW_GUIDE_SET_CONTENT FIRMWARE(0x53596d70)
/* The Fn label and the settings icon of the Fn hint, as the native ADJ
   refresh gives them to FW_GUIDE_SET_CONTENT. Each is one lower than on
   gr4-1.20. */
#define FN_GUIDE_TEXT 0x1fc
#define FN_GUIDE_ICON 0x269
/* 1 in movie mode. GR IV computes the mode from the camera settings. */
#define ADJ_MOVIE(controller) (((u32 (*)(u32))FIRMWARE(0x531bd8d0))(controller))
/* The native ADJ handler acts on EV_FN_ACTION at 0x531bdf58. */

/* Playback metadata singleton and the maker-note IFD entry setter: (entry, 0,
   value) stores a u16 value. */
#define FW_PLAYBACK_METADATA FIRMWARE(0x532406a0)
#define FW_IFD_SET_SHORT FIRMWARE(0x538081b0)

/* Picture processing: the YCbCr matrix calculation. */
#define FW_YCBCR_CALC FIRMWARE(0x5393b9f8)
/* The processing hooks (gr4-processing.S) keep the type table index in the
   frame slot fp-0x34, as on 1.11. The function stores the highlight
   correction flag of the capture in fp-0x30 before the hooks. */
#define PROCESSING_INDEX_SLOT 0x34
#define PROCESSING_HIGHLIGHT_SLOT 0x30

/* Date stamp: cache maintenance and the local time. */
#define FW_CACHE_FLUSH FIRMWARE(0x53ab6b48)
#define FW_CLOCK FIRMWARE(0x53913498)
#define FW_LOCAL_TIME FIRMWARE(0x5390bca8)

/* The UI service (runtime/ui.h): the view factory, the App task timers,
   the message box, its progress bar, and the texts of its buttons. */
#define FW_VIEW_FACTORY FIRMWARE(0x535a8fec)
#define FW_TIMER_SERVICE FIRMWARE(0x5323fd3c)
#define FW_TIMER_START FIRMWARE(0x532605f4)
#define FW_TIMER_CANCEL FIRMWARE(0x53260378)
#define FW_MESSAGE_BACKGROUND FIRMWARE(0x53597cd0)
#define FW_MESSAGE_SET FIRMWARE(0x53597d78)
#define FW_MESSAGE_BUTTONS FIRMWARE(0x535980c4)
#define FW_MESSAGE_ENABLE_BUTTONS FIRMWARE(0x53598248)
#define FW_MESSAGE_CURSOR FIRMWARE(0x53598140)
#define FW_MESSAGE_GET_CURSOR FIRMWARE(0x53598830)
#define FW_PROGRESS_PERCENT FIRMWARE(0x53595874)
#define UI_EXECUTE_TEXT 0x304
#define UI_CANCEL_TEXT 0x305
/* The slider of ui_slider: the setters of a slider configuration (init,
   range, and one flag) and its apply to a SliderView, and the icons that the
   native sliders use. */
#define FW_SLIDER_INIT FIRMWARE(0x5359a8d8)
#define FW_SLIDER_RANGE FIRMWARE(0x5359a934)
#define FW_SLIDER_FLAG FIRMWARE(0x5359a954)
#define FW_SLIDER_APPLY FIRMWARE(0x5359aa6c)
#define UI_SLIDER_ICONS {0x25f,0x54,0xc3,0x271}

#endif
