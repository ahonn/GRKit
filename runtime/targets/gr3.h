/* RICOH GR III family: the values that the shared runtime takes from each
   family, as GR III builds have them. A build header (gr3-<version>.h)
   defines the firmware addresses (FW_*) and the other values of its build,
   then includes this file. Comments here name no firmware address, because
   each build has its own; the build header gives them. */
#ifndef GRKIT_GR3_H
#define GRKIT_GR3_H

/* Icon slots hold an 8-byte descriptor, then 40 x 40 RGBA. */
#define GRKIT_ICON_STRIDE 6408

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
/* The fewest row views of a SelectMenuController. Its constructor builds a
   view for each row of its longest kind (+0x199). The longest native kind has
   21 rows, and no GRKit hook borrows it (2.10: kind table 0x53b3c6c0). The
   firmware can build the controller before INSTALL, so a value page on it
   shows at most these rows. A long value page shows on an own controller
   (runtime/menu_settings.h). */
#define SELECT_MENU_ROWS 21U
#define CHECK_LINE_TYPE 0x2aU
#define CHECK_TEXT_VIEW 0xf0

/* CORE.CUSTOM_SLOTS (runtime/custom_slots.h): the native Custom sets in the
   working record of the settings (FW_SETTINGS: the record at +4, the
   header at +0). Each field is one byte, followed by a flag byte that must
   stay 0. The settings of GR III 2.20 and GR IIIx 1.60 and 1.70 have the same
   layout.
   - The header byte 0 is the CustomSettingBox: 0 in the normal modes, else
     the U mode whose box the record holds.
   - CUSTOM_SLOT_ACTIVE is the active set (StillActiveImageControlSet): 0
     for No0, else Custom n.
   - CUSTOM_SLOT_TYPE(n) is the type of set n (StillImageControlNo<n>).
   - CUSTOM_SLOT_FIELDS gives, for each type from Standard (11) to Negative
     Film (22), the field of each parameter in set No0 (custom_block.h
     order), or 0 when the type does not have it. Set n is n strides
     further: CUSTOM_SLOT_STRIDE for the main block, CUSTOM_SLOT_LATE_STRIDE
     from CUSTOM_SLOT_LATE (the Cross Processing and Negative Film blocks).
   - CUSTOM_SLOT_FILTER is the R value of filter group 1 of set No0: R, G,
     and B are 16-bit values 4 bytes apart, and the groups 1 to 4 are 12
     bytes apart. */
#define CUSTOM_SLOT_BOX(settings) BYTE(WORD(settings,0),0)
#define CUSTOM_SLOT_RECORD(settings) WORD(settings,4)
#define CUSTOM_SLOT_ACTIVE 0x1c0
#define CUSTOM_SLOT_TYPE(n) (0x1c2+2*(n))
#define CUSTOM_SLOT_STRIDE 0xe2
#define CUSTOM_SLOT_LATE 0x9c0
#define CUSTOM_SLOT_LATE_STRIDE 0x14
#define CUSTOM_SLOT_FILTER 0x248
#define CUSTOM_SLOT_FIELDS { \
    {0x1c8,0x1d4,0x1e0,0x1f2,0x204,0x216,0x228,0x278,0x28a,0,0,0,0},       /* Standard */ \
    {0x1ca,0x1d6,0x1e2,0x1f4,0x206,0x218,0x22a,0x27a,0x28c,0,0,0,0},       /* Vivid */ \
    {0,0,0x1e4,0x1f6,0x208,0x21a,0x22c,0x27c,0x28e,0x23a,0,0x2a4,0x29a},   /* Monotone */ \
    {0,0,0x1e6,0x1f8,0x20a,0x21c,0x22e,0x27e,0,0x23c,0,0x2a6,0x29c},       /* Soft Monotone */ \
    {0,0,0x1e8,0x1fa,0x20c,0x21e,0x230,0x280,0x290,0x23e,0,0,0x29e},       /* Hard Monotone */ \
    {0,0,0x1ea,0x1fc,0x20e,0x220,0x232,0x282,0x292,0x240,0,0x2a8,0x2a0},   /* Hi-Contrast B&W */ \
    {0x1cc,0x1d8,0x1ec,0x1fe,0x210,0x222,0x234,0x284,0x294,0,0,0,0},       /* Positive Film */ \
    {0x1ce,0x1da,0x1ee,0x200,0x212,0x224,0x236,0x286,0x296,0x242,0,0,0},   /* Bleach Bypass */ \
    {0x1d0,0x1dc,0x1f0,0x202,0x214,0x226,0x238,0x288,0x298,0x244,0,0,0},   /* Retro */ \
    {0x1d2,0x1de,0,0,0,0,0,0,0,0x246,0x2a2,0,0},                           /* HDR Tone */ \
    {0x9c4,0x9c6,0x9c8,0x9ca,0x9cc,0x9ce,0x9d0,0x9d2,0x9d4,0x9d6,0,0,0},   /* Cross Processing */ \
    {0xa42,0xa44,0xa46,0xa48,0xa4a,0xa4c,0xa4e,0xa50,0xa52,0,0,0,0xa54},   /* Negative Film */ \
}
/* The App task command that selects set n with its type, as the native
   effect property does: kind 0 (set), then the parameters. It stores the set
   and the type again and rebuilds the live view of the active set. */
#define CUSTOM_SLOT_COMMAND_SIZE 0x54
#define CUSTOM_SLOT_COMMAND_SELECT 0x33

/* Visible-area id of a native aspect. GR III offers only 3:2 (0) and 1:1 (3). */
#define PICTURE_NATIVE_ID(native) ((native)==3 ? 3 : 0)

#endif
