/* RICOH GR III 2.10 (also GR IIIx 1.60): firmware entry points, data, and
   object layouts that the runtime uses. */
#ifndef GRKIT_TARGET_H
#define GRKIT_TARGET_H

/* Camera settings and the still aspect property. */
#define FW_SETTINGS 0x5320b31c
#define FW_STILL_ASPECT_GET 0x53245260
#define FW_STILL_ASPECT_SET 0x532525a4
#define FW_CAPTURE_PROPERTIES 0x53525d5c
/* Settings domain: +0x518 -> control -> payload with the native Image Control
   type at +0x77c. Standard is type 11. */
#define FW_SETTINGS_DOMAIN 0x553458b0
#define SETTINGS_IC_TYPE 0x77c

/* Allocation and the vector helpers of View children. */
#define FW_ALLOCATE 0x536af2a8
#define FW_VECTOR_INSERT 0x536b6de4
#define FW_VECTOR_ERASE 0x536b6f68

/* View classes. */
#define FW_VIEW_SET_TOUCH 0x533abeb0
#define FW_VIEW_SET_PARENT 0x533ac120
#define FW_VIEW_SET_TOUCHABLE 0x533ac1b4
#define FW_VIEW_SET_X 0x533ac568
#define FW_VIEW_INVALIDATE 0x533ad308
#define FW_VIEW_SET_VISIBLE 0x533ad3c0
#define FW_LINE_VIEW_INIT 0x53393d84
#define FW_LINE_VIEW_SET_STYLE 0x53393e00
#define FW_LINE_VIEW_DESTROY 0x53393d54
#define FW_ICON_VIEW_INIT 0x5339a908
#define FW_ICON_VIEW_SET_ICON 0x5339a9a0
#define FW_ICON_VIEW_DESTROY 0x5339a874
#define FW_STRING_VIEW_RESET 0x53396f80
#define FW_LIST_SET_COUNT 0x53394e8c
#define FW_LIST_ROW 0x53394758
#define FW_LIST_SAVE 0x533947f0
#define FW_LIST_CURSOR 0x53394820
#define FW_LIST_RESTORE 0x53395224
#define FW_LIST_SET_CURSOR 0x53395430
#define FW_ROW_SET_ICON 0x533a1734
#define FW_ROW_SET_CHECK 0x533a19b4

/* AdjModeController: methods and the touch callback pair of a toolbar icon. */
#define FW_ADJ_SNAPSHOT 0x531a0150
#define FW_ADJ_TOUCH_ENTER 0x531a16c4
#define FW_ADJ_TOUCH_LEAVE 0x531a1874
#define FW_ADJ_SLOT_TYPE 0x531a2820
/* The slot type of the native Image Control slot, and the controller of its
   Standard parameter page (still mode). */
#define ADJ_IC_SLOT_TYPE 12
#define IC_PARAMETERS_CONTROLLER 0x2000014
#define FW_ADJ_REFRESH 0x531a4114
#define FW_ADJ_SELECT_SLOT 0x531a4864
#define FW_CHANGE_CONTROLLER 0x5316c9d8
/* AdjModeController fields. */
#define ADJ_CANVAS 0x4
#define ADJ_TITLE 0x160
#define ADJ_DETAIL 0x164
#define ADJ_DETAIL_VALUE 0x168
#define ADJ_CELLS 0x16c
#define ADJ_FRAME 0x180
#define ADJ_ICONS 0x184
#define ADJ_FN_GUIDE 0x1ac
#define ADJ_LIST 0x1f0
#define ADJ_SLOT 0x1f4
/* 1 in movie mode. */
#define ADJ_MOVIE(controller) BYTE(controller,0x270)
#define ADJ_ACTION 0x279

/* AdjModeController events (the first halfword of an event). */
#define EV_SWITCH 5
#define EV_SWITCH_END 8
#define EV_FN 0x1b
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
#define FW_PLAYBACK_METADATA 0x5320b97c
#define METADATA_TONE 0x220
#define FW_ICON_TABLE 0x550020fc
/* The native Off icon, used until the installer provides the GRKit one. */
#define NATIVE_OFF_ICON() WORD(FW_ICON_TABLE,0x166*4)
/* Maker-note IFD entry setter: (entry, 0, value) stores a u16 value. */
#define FW_IFD_SET_SHORT 0x535d02c8
/* LiveViewController: nonzero in movie mode, and the LCD canvas. */
#define LIVEVIEW_MOVIE 0x2d2
#define LIVEVIEW_CANVAS 0x15c

/* Picture processing. GAMMA_BUILT: the word of the gamma object that is 1
   after the native curves are built. */
#define GAMMA_BUILT 0x3230
#define FW_YCBCR_CALC 0x536e4720
#define FW_CACHE_FLUSH 0x5383c8a8
#define FW_CLOCK 0x536bd838
#define FW_LOCAL_TIME 0x536b6510

#endif
