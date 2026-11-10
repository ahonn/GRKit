/* RICOH GR IV 1.21 (also GR IV HDF, and GR IV Monochrome 1.11, which has the
   same firmware file): the firmware addresses and the values of the 1.21
   build. gr4.h has the layouts and constants of each GR IV build and
   describes what each address is. The addresses are those of the pinned 1.21
   build; each address is a FIRMWARE() symbol that the installer relocates.
   Data that no firmware code loads directly is an offset from an address that
   it does load. The values are those of 1.20, except the controller IDs of
   the RAW development lists: 1.21 adds a controller before them. Not tested
   on a camera. */
#ifndef GRKIT_TARGET_H
#define GRKIT_TARGET_H

#include "gr4.h"

/* The icon draw compares the icon ID with this native limit as an immediate
   (1.11 loads 0x296 into r2). */
#define NATIVE_ICON_LIMIT 0x2c4

/* Camera settings, the still aspect getter, and the settings domain after the
   vtable of the capture property singleton. */
#define FW_SETTINGS FIRMWARE(0x53241240)
#define FW_STILL_ASPECT_GET FIRMWARE(0x532a0c30)
#define FW_SETTINGS_DOMAIN (FIRMWARE(0x5537b2a4) + 4)
/* The native monochrome types, as bit masks of the type: 13 to 16 as on
   GR III, 25 to 30 as on 1.11, and the 1.20 types 31 (ImageTone 0x8107) and
   32 (0x8106) of the table 0x53fd9750. NATIVE_MONOCHROME_TYPES_HIGH holds the
   types from 32. */
#define NATIVE_MONOCHROME_TYPES 0xfe01e000U
#define NATIVE_MONOCHROME_TYPES_HIGH 0x00000001U
/* The capture aspect getter, which the live-view mode function 0x538b4d70 and
   the mode check 0x53664d4c call, and the still aspect resend. */
#define FW_CAPTURE_ASPECT_GET FIRMWARE(0x5377e5f0)
#define FW_STILL_ASPECT_RESEND FIRMWARE(0x53286b9c)
#define FW_STILL_ASPECT_SET FIRMWARE(0x532fdbe4)
/* The size classifier is 0x533b28b4. */

/* Allocation and the vector helpers of View children. */
#define FW_ALLOCATE FIRMWARE(0x53909154)
#define FW_VECTOR_INSERT FIRMWARE(0x53913ba8)
#define FW_VECTOR_ERASE FIRMWARE(0x53913d2c)

/* View classes. */
#define FW_VIEW_SET_TOUCH FIRMWARE(0x535ac19c)
#define FW_VIEW_SET_PARENT FIRMWARE(0x535ac40c)
#define FW_VIEW_SET_TOUCHABLE FIRMWARE(0x535ac4a0)
#define FW_VIEW_SET_X FIRMWARE(0x535ac854)
#define FW_VIEW_INVALIDATE FIRMWARE(0x535ad5f4)
/* ScreenDevice::OnSetLevelAlarm(screen, mode): sets the live-view highlight warning. */
#define FW_LEVEL_ALARM FIRMWARE(0x5392c5a8)
/* LiveViewController::UpdateLevelAlarm(controller, 1): sets the warning again
   while LIVEVIEW_LEVEL_PENDING is set. */
#define FW_LIVEVIEW_LEVEL_UPDATE FIRMWARE(0x5318f17c)
/* The electronic level: a static object whose second word points to the
   level, or holds 0 before the level exists (runtime/processing.h). */
#define FW_LEVEL FIRMWARE(0x553a31a8)
#define FW_VIEW_SET_VISIBLE FIRMWARE(0x535ad6ac)
#define FW_LINE_VIEW_INIT FIRMWARE(0x53593cb0)
#define FW_LINE_VIEW_SET_STYLE FIRMWARE(0x53593d2c)
#define FW_LINE_VIEW_DESTROY FIRMWARE(0x53593c80)
#define FW_ICON_VIEW_INIT FIRMWARE(0x5359a994)
#define FW_ICON_VIEW_SET_ICON FIRMWARE(0x5359aa2c)
#define FW_ICON_VIEW_DESTROY FIRMWARE(0x5359a900)
#define FW_STRING_VIEW_RESET FIRMWARE(0x53596f7c)
/* The width of the text of a string view, in pixels. */
#define FW_STRING_VIEW_WIDTH FIRMWARE(0x53596dc4)
#define FW_LIST_SET_COUNT FIRMWARE(0x53594dd4)
#define FW_LIST_ROW FIRMWARE(0x5359468c)
#define FW_LIST_SAVE FIRMWARE(0x5359472c)
#define FW_LIST_CURSOR FIRMWARE(0x5359475c)
#define FW_LIST_WIDTH FIRMWARE(0x53594800)
#define FW_LIST_RESTORE FIRMWARE(0x5359516c)
#define FW_LIST_SET_CURSOR FIRMWARE(0x53595378)
#define FW_LIST_MOVE FIRMWARE(0x535951a8)
#define FW_ROW_SET_ICON FIRMWARE(0x535a198c)
#define FW_ROW_SET_CHECK FIRMWARE(0x535a1c0c)

/* AdjModeController: methods and the touch callback pair of a toolbar icon. */
#define FW_ADJ_TOUCH_ENTER FIRMWARE(0x531bd304)
#define FW_ADJ_TOUCH_LEAVE FIRMWARE(0x531bd430)
#define FW_ADJ_SLOT_TYPE FIRMWARE(0x531bf0c0)
#define FW_ADJ_SNAPSHOT FIRMWARE(0x531be930)
/* The settings page: the MENU submenu kind table, the text IDs of Instant
   Review, of the five value pages (1.20 moves some of them) and of the sixth
   row (a label of another submenu), the OK and MENU guides of a value page,
   and the row
   text setters. */
#define FW_SUBMENU_KINDS FIRMWARE(0x53dd2598)
#define SETTINGS_TITLE_TEXT 0x228
#define SETTINGS_ROW_TEXTS {0x22a,0x22c,0x1a8,0x1c8,0x1ce,0x1f2}
#define SELECT_GUIDE_OK_TEXT 0x364
#define SELECT_GUIDE_BACK_TEXT 0x365
#define FW_ROW_TEXT FIRMWARE(0x5359e9d4)
/* Options pages show checkboxes (runtime/menu_settings.h): the constructor of
   SelectMenuController, the vtable and SetItem of CheckBoxListLineView, the
   ScrollListView init, the radio dot of a SelectMenuLineView, and the checked
   and unchecked images. */
#define FW_SELECT_MENU_NEW FIRMWARE(0x531dd290)
#define CHECK_LINE_VTABLE FIRMWARE(0x53ed1620)
#define FW_CHECK_ITEM FIRMWARE(0x535a8c28)
#define FW_LIST_INIT FIRMWARE(0x5359453c)
/* Gives a list value to each of its rows; the ADJ list init passes 0x10.
   GR III keeps it in the list, so it needs no call for added rows. */
#define FW_LIST_ROWS_VALUE FIRMWARE(0x53595824)
#define FW_ROW_DOT FIRMWARE(0x5359e934)
#define CHECK_IMAGE_ON 0x111U
#define CHECK_IMAGE_OFF 0x110U
#define FW_ROW_VALUE FIRMWARE(0x535a6ddc)
/* The RAM table of the MENU tab descriptors, and the MENU row setters of
   the label and of the value text. */
#define FW_MENU_TABS FIRMWARE(0x55001e78)
/* The rows of the stock Customize tab (runtime/menu_section.h). */
#define MENU_TAB_STOCK_ROWS 24
#define FW_MENU_ROW_LABEL FIRMWARE(0x5359ddb4)
#define FW_MENU_ROW_VALUE FIRMWARE(0x5359e068)
#define FW_ADJ_REFRESH FIRMWARE(0x531c0f08)
#define FW_ADJ_SELECT_SLOT FIRMWARE(0x531c148c)
#define FW_CHANGE_CONTROLLER FIRMWARE(0x5318408c)
/* The controller factory singleton, which holds one instance of each controller. */
#define INFO_TITLE_TEXT 0x2d5
#define INFO_LABEL_TEXTS {0x2d7,0x2d9,0x2db}
#define FW_STRING_VIEW_SET_ASCII FIRMWARE(0x53590db8)
#define FW_CONTROLLER_FACTORY FIRMWARE(0x53187154)
/* The UI language byte of the static settings object 0x550832a8 (reader
   0x5329f7b8, +0x14ef; 1.11 +0x1497), with the GR III values. Not confirmed
   on a camera. */
#define LANGUAGE_SETTING_BYTE (FIRMWARE(0x550832a8) + 0x14ef)
/* The live-view display mode that DISP cycles, in the same object (reader
   0x5329f4ac, +0x1461; 1.11 +0x1409), with the GR III values: 0 full,
   1 simple, 2 no information, 3 display off. */
#define DISPLAY_MODE_BYTE (FIRMWARE(0x550832a8) + 0x1461)
#define FW_GUIDE_SET_CONTENT FIRMWARE(0x5359b860)
/* The Fn label and the settings icon of the Fn hint, as the native ADJ
   refresh gives them to FW_GUIDE_SET_CONTENT. 1.20 moves both IDs. */
#define FN_GUIDE_TEXT 0x1fd
#define FN_GUIDE_ICON 0x26a
/* 1 in movie mode. GR IV computes the mode from the camera settings. */
#define ADJ_MOVIE(controller) (((u32 (*)(u32))FIRMWARE(0x531be840))(controller))
/* The native ADJ handler acts on EV_FN_ACTION at 0x531beec8. */

/* Playback metadata singleton and the maker-note IFD entry setter: (entry, 0,
   value) stores a u16 value. */
#define FW_PLAYBACK_METADATA FIRMWARE(0x532418a4)
#define FW_IFD_SET_SHORT FIRMWARE(0x5380eb40)
/* DNG LONG tag element writer and native sensor rectangle helper. */
#define FW_IFD_SET_LONG FIRMWARE(0x5380eb58)
#define FW_DNG_NATIVE_RECT FIRMWARE(0x538a66c0)
/* RawDevelopmentModel: the guard of its static instance, and the instance.
   The family header gives its state (DEVELOP_STATE). */
#define FW_DEVELOP_GUARD FIRMWARE(0x55089274)
#define FW_DEVELOP_MODEL FIRMWARE(0x5508968c)
/* The time of the photo in the playback metadata (runtime/targets/gr4.h). */
#define METADATA_TIME 0x3e4
/* The EXIF Orientation of the photo in the playback metadata. */
#define METADATA_ORIENTATION 0x2c8
/* RAW development, the Image Control list (runtime/menu.c): the label and
   the icon of an Image Control (model, category, value), the children of a
   list row view (their maker and layout, a child, and the setups of the icon
   and of the text). */
#define FW_DEVELOP_TONE_TEXT FIRMWARE(0x53383680)
#define FW_DEVELOP_TONE_ICON FIRMWARE(0x53383634)
/* RAW development: the ID of the first development controller (gr4.h).
   1.21 adds a controller before it (1.20: 0x400001e). */
#define DEVELOP_CONTROLLERS 0x400001fU
/* RAW development, the Fn adjustments (runtime/develop.h): the category
   and the values of the selected Image Control in RawDevelopmentModel,
   and the parameter set of Standard (runtime/targets/gr3-2.10.h). */
#define DEVELOP_CATEGORY 0x9c
#define DEVELOP_VALUES 0x9d
#define DEVELOP_PARAMETERS 0xb4
/* RAW development, the aspect list (runtime/menu.c): the row value of the
   model (model, row), the capture aspect of a development aspect (model,
   value), and the label and the icon of a development aspect (model,
   value). */
#define FW_DEVELOP_ASPECT_VALUE FIRMWARE(0x5337a540)
#define FW_DEVELOP_ASPECT_CAPTURE FIRMWARE(0x53384414)
#define FW_DEVELOP_ASPECT_TEXT FIRMWARE(0x533844f0)
#define FW_DEVELOP_ASPECT_ICON FIRMWARE(0x533844ac)
#define FW_ROW_CHILDREN FIRMWARE(0x535a23e0)
#define DEVELOP_ROW_LAYOUT FIRMWARE(0x53dc49a8)
#define FW_ROW_CHILD FIRMWARE(0x535a1f8c)
#define FW_ICON_VIEW_STYLE FIRMWARE(0x5359aabc)
#define FW_STRING_VIEW_STYLE FIRMWARE(0x53590eb8)

/* Picture processing: the YCbCr matrix calculation. */
#define FW_YCBCR_CALC FIRMWARE(0x53943a08)
/* The processing hooks (gr4-processing.S) keep the type table index in the
   frame slot fp-0x30 (1.11 fp-0x34). The function stores the highlight
   correction flag of the capture in fp-0x34 (1.11 fp-0x30) before the hooks. */
#define PROCESSING_INDEX_SLOT 0x30
#define PROCESSING_HIGHLIGHT_SLOT 0x34

/* Date stamp: cache maintenance and the local time. */
#define FW_CACHE_FLUSH FIRMWARE(0x53abf110)
#define FW_CLOCK FIRMWARE(0x5391aaa0)
#define FW_LOCAL_TIME FIRMWARE(0x53913210)

/* The UI service (runtime/ui.h): the view factory, the App task timers,
   the message box, its progress bar, and the texts of its buttons. */
#define FW_VIEW_FACTORY FIRMWARE(0x535adadc)
#define FW_TIMER_SERVICE FIRMWARE(0x53240f40)
#define FW_TIMER_START FIRMWARE(0x53261a4c)
#define FW_TIMER_CANCEL FIRMWARE(0x532617d0)
#define FW_MESSAGE_BACKGROUND FIRMWARE(0x5359c7c0)
#define FW_MESSAGE_SET FIRMWARE(0x5359c868)
#define FW_MESSAGE_BUTTONS FIRMWARE(0x5359cbb4)
#define FW_MESSAGE_ENABLE_BUTTONS FIRMWARE(0x5359cd38)
#define FW_MESSAGE_CURSOR FIRMWARE(0x5359cc30)
#define FW_MESSAGE_GET_CURSOR FIRMWARE(0x5359d320)
#define FW_PROGRESS_PERCENT FIRMWARE(0x5359a364)
#define UI_EXECUTE_TEXT 0x304
#define UI_CANCEL_TEXT 0x305
/* The slider of ui_slider: the setters of a slider configuration (init,
   range, and one flag) and its apply to a SliderView, and the icons that the
   native sliders use. */
#define FW_SLIDER_INIT FIRMWARE(0x5359f3c8)
#define FW_SLIDER_RANGE FIRMWARE(0x5359f424)
#define FW_SLIDER_FLAG FIRMWARE(0x5359f444)
#define FW_SLIDER_APPLY FIRMWARE(0x5359f55c)
#define UI_SLIDER_ICONS {0x260,0x55,0xc4,0x272}

#endif
