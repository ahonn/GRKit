/* RICOH GR III 2.21 (also GR IIIx 1.71): the firmware addresses and the
   values of the 2.21 build. gr3.h has the layouts and constants of each GR
   III build. The addresses are those of the pinned 2.21 image. Each is a
   FIRMWARE() symbol that the installer relocates, as on GR IV. The text IDs
   from 0x2a1 are those of 2.10 moved by 7, because 2.21 adds its reset
   texts. Not tested on a camera. */
#ifndef GRKIT_TARGET_H
#define GRKIT_TARGET_H

#include "gr3.h"
/* The native settings of recipes (runtime/native.h), from the firmware. */
#include "gr3-2.21-native.h"

/* Camera settings and the still aspect property. */
#define FW_SETTINGS FIRMWARE(0x5320d6f4)
#define FW_STILL_ASPECT_GET FIRMWARE(0x532479e8)
#define FW_STILL_ASPECT_SET FIRMWARE(0x53254d2c)
/* The crop factor of CORE.CROP (runtime/crop.h): its getter and setter on
   the camera settings (FW_SETTINGS), with the factor as a float in s0. */
#define FW_CROP_GET FIRMWARE(0x532479f4)
#define FW_CROP_SET FIRMWARE(0x53254e0c)
/* The project type that the RTOS sets at boot: 1 on GR III, 0 on GR IIIx
   (2 before it is set). It takes no lock. */
#define FW_PROJECT_TYPE FIRMWARE(0x536b7ac4)
/* ShootingModel singleton, and the two requests with which a controller that
   needs no live view stops it (Controller live-view dispatch 0x5316ea00,
   state 1). Each takes the model and a completion callback. */
#define FW_SHOOTING_MODEL FIRMWARE(0x5320d574)
#define FW_LIVE_VIEW_END FIRMWARE(0x5323b28c)
#define FW_LIVE_VIEW_STOP FIRMWARE(0x5323ae70)
#define FW_CAPTURE_PROPERTIES FIRMWARE(0x5352879c)
/* Settings domain (gr3.h: SETTINGS_IC_TYPE). The native monochrome types are
   those of the ImageTone table 0x53d33624 (gr3.h: NATIVE_MONOCHROME_TYPES). */
#define FW_SETTINGS_DOMAIN FIRMWARE(0x55345b80)

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
#define FW_ROW_SET_ICON FIRMWARE(0x533a4170)
#define FW_ROW_SET_CHECK FIRMWARE(0x533a43f0)

/* AdjModeController: methods and the touch callback pair of a toolbar icon. */
#define FW_ADJ_SNAPSHOT FIRMWARE(0x531a117c)
#define FW_ADJ_TOUCH_ENTER FIRMWARE(0x531a26f0)
#define FW_ADJ_TOUCH_LEAVE FIRMWARE(0x531a28a0)
#define FW_ADJ_SLOT_TYPE FIRMWARE(0x531a3850)
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
/* The row lists of the parameter page hold the Toning rows of Retro at
   0x53b38f14 (gr3.h: PARAMETER_TONING_BASE). */
/* The submenu kind table of the native settings page of a GRKit root row
   (gr3.h: SUBMENU_KIND_*). */
#define FW_SUBMENU_KINDS FIRMWARE(0x53b441c8)
/* The guides of a SelectMenu value page: OK and MENU. */
#define SELECT_GUIDE_OK_TEXT 0x308
#define SELECT_GUIDE_BACK_TEXT 0x309
/* Row text setters: a value page row sets the string view at ROW_TEXT_VIEW,
   and a submenu row its value view at ROW_VALUE_VIEW. Each sets a text ID. */
#define FW_ROW_TEXT FIRMWARE(0x533a11dc)
/* Options pages show checkboxes (runtime/menu_settings.h): the constructor of
   SelectMenuController, the vtable and SetItem of CheckBoxListLineView, the
   ScrollListView init, and the radio dot of a SelectMenuLineView. */
#define FW_SELECT_MENU_NEW FIRMWARE(0x531bf7c8)
#define CHECK_LINE_VTABLE FIRMWARE(0x53c139b8)
#define FW_CHECK_ITEM FIRMWARE(0x533ab374)
#define FW_LIST_INIT FIRMWARE(0x53397040)
#define FW_ROW_DOT FIRMWARE(0x533a1174)
#define FW_ROW_VALUE FIRMWARE(0x533a950c)
/* The MENU tab descriptors and the row setters (gr3.h: MENU_TAB_*). */
#define FW_MENU_TABS FIRMWARE(0x55001eb8)
#define FW_MENU_ROW_LABEL FIRMWARE(0x533a06ac)
#define FW_MENU_ROW_VALUE FIRMWARE(0x533a08a8)
#define FW_ADJ_REFRESH FIRMWARE(0x531a5144)
#define FW_ADJ_SELECT_SLOT FIRMWARE(0x531a5894)
#define FW_CHANGE_CONTROLLER FIRMWARE(0x5316da14)
/* The setter of an ASCII string view (gr3.h: INFO_LINES). */
#define FW_STRING_VIEW_SET_ASCII FIRMWARE(0x53393928)
/* The controller factory singleton, which holds one instance of each controller. */
#define FW_CONTROLLER_FACTORY FIRMWARE(0x5317040c)
/* The current UI settings object is at this slot, +4 in the object
   0x55064200, which firmware code loads. Its UI language byte
   (gr3.h: LANGUAGE_SETTING) has the reader 0x53248d64. */
#define UI_SETTINGS_SLOT (FIRMWARE(0x55064200) + 4)
/* The icon setup of a manual focus scale (scale), which its update calls
   when the scale turns between normal and macro. */
#define FW_FOCUS_SCALE_ICONS FIRMWARE(0x53395bc0)
/* The ADJ refresh fills the Fn hint only on a native slot that offers Fn. For
   the Image Control slot, it uses the Fn label, layout 10, and the settings
   icon. A new ADJ starts with an empty hint. */
#define FW_GUIDE_SET_CONTENT FIRMWARE(0x5339e210)

/* Playback metadata singleton (gr3.h: METADATA_*) and the native icon table. */
#define FW_PLAYBACK_METADATA FIRMWARE(0x5320dd54)
#define FW_ICON_TABLE FIRMWARE(0x550020fc)
/* RawDevelopmentModel: the guard of its static instance, and the instance.
   The family header gives its state (DEVELOP_STATE). */
#define FW_DEVELOP_GUARD FIRMWARE(0x550665a0)
#define FW_DEVELOP_MODEL FIRMWARE(0x550669b0)
/* RAW development, the Image Control list (runtime/menu.c): the label and
   the icon of an Image Control (model, category, value). */
#define FW_DEVELOP_TONE_TEXT FIRMWARE(0x5328b97c)
#define FW_DEVELOP_TONE_ICON FIRMWARE(0x5328b930)
/* RAW development, the aspect list (runtime/menu.c): the row value of the
   model (model, row), the capture aspect of a development aspect (model,
   value), and the label and the icon of a development aspect (model,
   value). */
#define FW_DEVELOP_ASPECT_VALUE FIRMWARE(0x53285b6c)
#define FW_DEVELOP_ASPECT_CAPTURE FIRMWARE(0x5328c614)
#define FW_DEVELOP_ASPECT_TEXT FIRMWARE(0x5328c6c0)
#define FW_DEVELOP_ASPECT_ICON FIRMWARE(0x5328c684)
/* The child view of a list row. */
#define FW_ROW_CHILD FIRMWARE(0x533a4770)
/* The children of the row views of RawDevelopmentParameterSetController:
   their maker (row, count, layout), the layout, and the setups of the icon
   and of the text that the list init does. */
#define FW_ROW_CHILDREN FIRMWARE(0x533a4b18)
#define DEVELOP_ROW_LAYOUT FIRMWARE(0x53b37cc8)
#define FW_ICON_VIEW_STYLE FIRMWARE(0x5339d46c)
#define FW_STRING_VIEW_STYLE FIRMWARE(0x53393a28)
/* Maker-note IFD entry setter: (entry, 0, value) stores a u16 value. */
#define FW_IFD_SET_SHORT FIRMWARE(0x535d2e30)
/* DNG LONG tag element writer and native sensor rectangle helper. */
#define FW_IFD_SET_LONG FIRMWARE(0x535d2e48)
#define FW_DNG_NATIVE_RECT FIRMWARE(0x536626c4)

/* Picture processing. */
#define FW_YCBCR_CALC FIRMWARE(0x536e8960)
/* The granulation after its amplitude choice (grain_amplitude). */
#define FW_GRAIN_AMPLITUDE_DONE FIRMWARE(0x5367e418)
/* Grain size (gr3.h: GRAIN_SV). The EXIF ISO reads the Sv at 0x535f89e8, and
   the granulation 0x5367e314 calls the noise fill FW_GRAIN_NOISE (buffer,
   pitch x height), which writes a random byte for each sample. FW_RELEASE
   frees a block of FW_ALLOCATE. */
#define FW_GRAIN_NOISE FIRMWARE(0x5368df04)
#define FW_RELEASE FIRMWARE(0x536b28bc)
#define FW_CACHE_FLUSH FIRMWARE(0x53840cc0)
/* The IIP rectangle fill of the canvas (runtime/targets/gr3.h). */
#define FW_CANVAS_FILL FIRMWARE(0x534cb7a4)
#define FW_CLOCK FIRMWARE(0x536c0f2c)
#define FW_LOCAL_TIME FIRMWARE(0x536b9ae8)

/* The UI service (runtime/ui.h): the view factory, the App task timers,
   the message box, its progress bar, and the texts of its buttons. */
#define FW_VIEW_FACTORY FIRMWARE(0x533b022c)
#define FW_TIMER_SERVICE FIRMWARE(0x5320d3f4)
#define FW_TIMER_START FIRMWARE(0x5322dbb8)
#define FW_TIMER_CANCEL FIRMWARE(0x5322d93c)
/* The mode dial (runtime/targets/gr3.h): the switch model of the App, and
   the settings object whose slot +4 is UI_SETTINGS_SLOT. */
#define DIAL_SWITCHES FIRMWARE(0x55064918)
#define DIAL_SETTINGS FIRMWARE(0x55064200)
/* The Set command of the RTOS shell command UserData (runtime/recipes.h):
   set(argc, argv) with argv = {"UserData", "Set", name, value}. It finds the
   setter of a native setting by its name and the stored value by its label. */
#define FW_USERDATA_SET FIRMWARE(0x533492bc)
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
   range, and one flag) and its apply to a SliderView (gr3.h: UI_SLIDER_ICONS). */
#define FW_SLIDER_INIT FIRMWARE(0x533a1bd0)
#define FW_SLIDER_RANGE FIRMWARE(0x533a1c2c)
#define FW_SLIDER_FLAG FIRMWARE(0x533a1c4c)
#define FW_SLIDER_APPLY FIRMWARE(0x533a1d64)

#endif
