/* RICOH GR IV 1.11 (also GR IV HDF): the firmware addresses and the values of
   the 1.11 build. gr4.h has the layouts and constants of each GR IV build and
   describes what each address is. The addresses are those of the pinned 1.11
   build. Cameras also run other 1.11 builds, so each address is a FIRMWARE()
   symbol that the installer relocates. Data that no firmware code loads
   directly is an offset from an address that it does load. */
#ifndef GRKIT_TARGET_H
#define GRKIT_TARGET_H

#include "gr4.h"
/* The native settings of recipes (runtime/native.h), from the firmware. */
#include "gr4-1.11-native.h"

/* Camera settings, the still aspect getter, and the settings domain after the
   vtable of the capture property singleton. */
#define FW_SETTINGS FIRMWARE(0x5323f018)
#define FW_STILL_ASPECT_GET FIRMWARE(0x5329c09c)
#define FW_SETTINGS_DOMAIN (FIRMWARE(0x5537960c) + 4)
/* The native monochrome types, as a bit mask of the type: 13 to 16 as on
   GR III, and 25 to 30, the types of GR IV Monochrome (ImageTone 0x8100 to
   0x8105, table 0x53fa1758). */
#define NATIVE_MONOCHROME_TYPES 0x7e01e000U
/* The capture aspect getter, which the live-view mode function 0x5388c7b8 and
   the mode check 0x5363fa84 call, and the still aspect resend. */
#define FW_CAPTURE_ASPECT_GET FIRMWARE(0x5375840c)
#define FW_STILL_ASPECT_RESEND FIRMWARE(0x53284454)
#define FW_STILL_ASPECT_SET FIRMWARE(0x532f8490)
/* The size classifier is 0x533a2820. */

/* Allocation and the vector helpers of View children. */
#define FW_ALLOCATE FIRMWARE(0x538e09b0)
#define FW_VECTOR_INSERT FIRMWARE(0x538eb348)
#define FW_VECTOR_ERASE FIRMWARE(0x538eb4cc)

/* View classes. */
#define FW_VIEW_SET_TOUCH FIRMWARE(0x535885a4)
#define FW_VIEW_SET_PARENT FIRMWARE(0x53588814)
#define FW_VIEW_SET_TOUCHABLE FIRMWARE(0x535888a8)
#define FW_VIEW_SET_X FIRMWARE(0x53588c5c)
#define FW_VIEW_INVALIDATE FIRMWARE(0x535899fc)
/* ScreenDevice::OnSetLevelAlarm(screen, mode): sets the live-view highlight warning. */
#define FW_LEVEL_ALARM FIRMWARE(0x53903294)
/* LiveViewController::UpdateLevelAlarm(controller, 1): sets the warning again
   while LIVEVIEW_LEVEL_PENDING is set. */
#define FW_LIVEVIEW_LEVEL_UPDATE FIRMWARE(0x5318df4c)
/* The electronic level: a static object whose second word points to the
   level, or holds 0 before the level exists (runtime/processing.h). */
#define FW_LEVEL FIRMWARE(0x553a149c)
#define FW_VIEW_SET_VISIBLE FIRMWARE(0x53589ab4)
#define FW_LINE_VIEW_INIT FIRMWARE(0x535700c0)
#define FW_LINE_VIEW_SET_STYLE FIRMWARE(0x5357013c)
#define FW_LINE_VIEW_DESTROY FIRMWARE(0x53570090)
#define FW_ICON_VIEW_INIT FIRMWARE(0x53576da0)
#define FW_ICON_VIEW_SET_ICON FIRMWARE(0x53576e38)
#define FW_ICON_VIEW_DESTROY FIRMWARE(0x53576d0c)
#define FW_STRING_VIEW_RESET FIRMWARE(0x5357338c)
/* The width of the text of a string view, in pixels. */
#define FW_STRING_VIEW_WIDTH FIRMWARE(0x535731d4)
#define FW_LIST_SET_COUNT FIRMWARE(0x535711e4)
#define FW_LIST_ROW FIRMWARE(0x53570a9c)
#define FW_LIST_SAVE FIRMWARE(0x53570b3c)
#define FW_LIST_CURSOR FIRMWARE(0x53570b6c)
#define FW_LIST_WIDTH FIRMWARE(0x53570c10)
#define FW_LIST_RESTORE FIRMWARE(0x5357157c)
#define FW_LIST_SET_CURSOR FIRMWARE(0x53571788)
#define FW_LIST_MOVE FIRMWARE(0x535715b8)
#define FW_ROW_SET_ICON FIRMWARE(0x5357dd98)
#define FW_ROW_SET_CHECK FIRMWARE(0x5357e018)

/* AdjModeController: methods and the touch callback pair of a toolbar icon. */
#define FW_ADJ_TOUCH_ENTER FIRMWARE(0x531bb7c0)
#define FW_ADJ_TOUCH_LEAVE FIRMWARE(0x531bb8ec)
#define FW_ADJ_SLOT_TYPE FIRMWARE(0x531bd580)
#define FW_ADJ_SNAPSHOT FIRMWARE(0x531bcdf0)
/* The settings page: the MENU submenu kind table, the text IDs of Instant
   Review, of the five value pages and of the sixth row (a label of another
   submenu), the row text setters, and the OK and MENU guides of a value
   page. */
#define FW_SUBMENU_KINDS FIRMWARE(0x53da6718)
#define SETTINGS_TITLE_TEXT 0x204
#define SETTINGS_ROW_TEXTS {0x206,0x208,0x188,0x1a8,0x1ae,0x1ce}
#define SELECT_GUIDE_OK_TEXT 0x338
#define SELECT_GUIDE_BACK_TEXT 0x339
#define FW_ROW_TEXT FIRMWARE(0x5357ade0)
/* Options pages show checkboxes (runtime/menu_settings.h): the constructor of
   SelectMenuController, the vtable and SetItem of CheckBoxListLineView, the
   ScrollListView init, the radio dot of a SelectMenuLineView, and the checked
   and unchecked images. */
#define FW_SELECT_MENU_NEW FIRMWARE(0x531db284)
#define CHECK_LINE_VTABLE FIRMWARE(0x53e99d60)
#define FW_CHECK_ITEM FIRMWARE(0x53585030)
#define FW_LIST_INIT FIRMWARE(0x5357094c)
#define FW_ROW_DOT FIRMWARE(0x5357ad40)
#define CHECK_IMAGE_ON 0xfeU
#define CHECK_IMAGE_OFF 0xfdU
#define FW_ROW_VALUE FIRMWARE(0x535831e8)
/* The RAM table of the MENU tab descriptors, and the MENU row setters of
   the label and of the value text. */
#define FW_MENU_TABS FIRMWARE(0x55001e78)
/* The rows of the stock Customize tab (runtime/menu_section.h). */
#define MENU_TAB_STOCK_ROWS 23
#define FW_MENU_ROW_LABEL FIRMWARE(0x5357a1c0)
#define FW_MENU_ROW_VALUE FIRMWARE(0x5357a474)
#define FW_ADJ_REFRESH FIRMWARE(0x531bf3c8)
#define FW_ADJ_SELECT_SLOT FIRMWARE(0x531bf94c)
#define FW_CHANGE_CONTROLLER FIRMWARE(0x53182e4c)
/* The controller factory singleton, which holds one instance of each controller. */
#define INFO_TITLE_TEXT 0x2ab
#define INFO_LABEL_TEXTS {0x2ad,0x2af,0x2b1}
#define FW_STRING_VIEW_SET_ASCII FIRMWARE(0x5356d1c8)
#define FW_CONTROLLER_FACTORY FIRMWARE(0x53185f14)
/* The UI language byte of the static settings object 0x55084dd8 (reader
   0x5329b068, +0x1497), with the GR III values. Not confirmed on a camera. */
#define LANGUAGE_SETTING_BYTE (FIRMWARE(0x55084dd8) + 0x1497)
/* The live-view display mode that DISP cycles, in the same object, with the
   GR III values: 0 full, 1 simple, 2 no information, 3 display off. */
#define DISPLAY_MODE_BYTE (FIRMWARE(0x55084dd8) + 0x1409)
#define FW_GUIDE_SET_CONTENT FIRMWARE(0x53577c6c)
/* The Fn label and the settings icon of the Fn hint, as the native ADJ
   refresh gives them to FW_GUIDE_SET_CONTENT. */
#define FN_GUIDE_TEXT 0x1de
#define FN_GUIDE_ICON 0x240
/* 1 in movie mode. GR IV computes the mode from the camera settings. */
#define ADJ_MOVIE(controller) (((u32 (*)(u32))FIRMWARE(0x531bcd00))(controller))
/* The native ADJ handler acts on EV_FN_ACTION at 0x531bd388. */

/* Playback metadata singleton and the maker-note IFD entry setter: (entry, 0,
   value) stores a u16 value. */
#define FW_PLAYBACK_METADATA FIRMWARE(0x5323f67c)
#define FW_IFD_SET_SHORT FIRMWARE(0x537e7148)

/* Picture processing: the YCbCr matrix calculation. */
#define FW_YCBCR_CALC FIRMWARE(0x5391a650)
/* The processing hooks (gr4-processing.S) keep the type table index in the
   frame slot fp-0x34. The function stores the highlight correction flag of
   the capture in fp-0x30 before the hooks. */
#define PROCESSING_INDEX_SLOT 0x34
#define PROCESSING_HIGHLIGHT_SLOT 0x30

/* Date stamp: cache maintenance and the local time. */
#define FW_CACHE_FLUSH FIRMWARE(0x53a95804)
#define FW_CLOCK FIRMWARE(0x538f21a0)
#define FW_LOCAL_TIME FIRMWARE(0x538ea9b0)

/* The UI service (runtime/ui.h): the view factory, the App task timers,
   the message box, its progress bar, and the texts of its buttons. */
#define FW_VIEW_FACTORY FIRMWARE(0x53589ee4)
#define FW_TIMER_SERVICE FIRMWARE(0x5323ed18)
#define FW_TIMER_START FIRMWARE(0x5325f59c)
#define FW_TIMER_CANCEL FIRMWARE(0x5325f320)
/* The Set command of the RTOS shell command UserData (runtime/recipes.h):
   set(argc, argv) with argv = {"UserData", "Set", name, value}. It finds the
   setter of a native setting by its name and the stored value by its label. */
#define FW_USERDATA_SET FIRMWARE(0x534a4650)
/* GR IV Monochrome in recipes (runtime/native_look.h): the setter of the
   Image Control type of set No0, set(settings object, stored value), which
   UserData and the native menu call; and the pointer to the current model
   record. */
#define FW_IMAGE_CONTROL_SET FIRMWARE(0x532b6f80)
#define FW_MODEL_RECORD FIRMWARE(0x553a0850)
#define FW_MESSAGE_BACKGROUND FIRMWARE(0x53578bcc)
#define FW_MESSAGE_SET FIRMWARE(0x53578c74)
#define FW_MESSAGE_BUTTONS FIRMWARE(0x53578fc0)
#define FW_MESSAGE_ENABLE_BUTTONS FIRMWARE(0x53579144)
#define FW_MESSAGE_CURSOR FIRMWARE(0x5357903c)
#define FW_MESSAGE_GET_CURSOR FIRMWARE(0x5357972c)
#define FW_PROGRESS_PERCENT FIRMWARE(0x53576770)
#define UI_EXECUTE_TEXT 0x2d8
#define UI_CANCEL_TEXT 0x2d9
/* The slider of ui_slider: the setters of a slider configuration (init,
   range, and one flag) and its apply to a SliderView, and the icons that the
   native sliders use. */
#define FW_SLIDER_INIT FIRMWARE(0x5357b7d4)
#define FW_SLIDER_RANGE FIRMWARE(0x5357b830)
#define FW_SLIDER_FLAG FIRMWARE(0x5357b850)
#define FW_SLIDER_APPLY FIRMWARE(0x5357b968)
#define UI_SLIDER_ICONS {0x23a,0x4a,0xb9,0x248}
/* The keyboard of the camera (runtime/ui.h): its controller ID, the getter
   of SoftwareKeyboardModel, which holds the use of the keyboard (the getter
   ignores its argument, the model factory), and the setter of the use,
   which asserts that no use is set. */
#define KEYBOARD_CONTROLLER 0x20000f7U
#define FW_KEYBOARD_MODEL FIRMWARE(0x53240134)
#define FW_KEYBOARD_SET_FUNCTION FIRMWARE(0x533e5508)

#endif
