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

/* The size of SelectMenuController, and the view type and the text view of
   CheckBoxListLineView (runtime/menu_settings.h). */
/* The string view of the submenu title view (submenu +0x15c): the title
   setter (2.10: 0x533a2ba0) sets the text ID of this view. */
#define SUBMENU_TITLE_STRING 0xf0
#define SELECT_MENU_SIZE 0x1d4U
/* The fewest row views of a SelectMenuController. Its constructor builds a
   view for each row of its longest kind (+0x199). The longest native kind has
   21 rows, and no GRKit hook borrows it (2.10: kind table 0x53b3c6c0). The
   firmware can build the controller before INSTALL, so a value page shows at
   most these rows. */
#define SELECT_MENU_ROWS 21U
#define CHECK_LINE_TYPE 0x2aU
#define CHECK_TEXT_VIEW 0xf0

/* Visible-area id of a native aspect. GR III offers only 3:2 (0) and 1:1 (3). */
#define PICTURE_NATIVE_ID(native) ((native)==3 ? 3 : 0)

#endif
