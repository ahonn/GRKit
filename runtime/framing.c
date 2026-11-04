/* Capture-aspect integration and the firmware entrypoints of the shared
   still-viewfinder service (viewfinder.h). */
/* The assembly wrappers interwork with these C helpers. Thumb keeps the
   shared viewfinder and picture code inside the fixed firmware tail. */
#ifdef __arm__
#pragma clang attribute push(__attribute__((target("thumb"))), apply_to=function)
#endif
#include "firmware.h"
#include "abi.h"
#ifndef FRAMING_PIXELS
#define FRAMING_PIXELS(address) ((volatile u32 *)(address))
#endif
#define OVERLAY_PIXELS(address) FRAMING_PIXELS(address)
#include "core.h"
#include "profile.h"

#define FRAMING_SETTINGS() (((u32 (*)(void))FW_SETTINGS)())
#ifndef FRAMING_NATIVE_ASPECT
#define FRAMING_NATIVE_ASPECT() (((u32 (*)(u32))FW_STILL_ASPECT_GET)(FRAMING_SETTINGS()))
#endif
/* The family header reads the requested capture aspect and sends the request
   again (FRAMING_CAPTURE_ASPECT, FRAMING_REQUEST_ASPECT). */
#if defined(FRAMING_CAPTURE_ASPECT) != defined(FRAMING_REQUEST_ASPECT)
#error "The family header defines FRAMING_CAPTURE_ASPECT and FRAMING_REQUEST_ASPECT together"
#endif
#ifndef FRAMING_INVALIDATE
/* View::Invalidate: request a redraw unless one is pending (+0xa0). */
#define FRAMING_INVALIDATE(view) (((void (*)(u32))FW_VIEW_INVALIDATE)(view))
#endif
extern volatile u32 *framing_header(void);

#ifndef FRAMING_DISPLAY_MODE
/* The native display mode that DISP cycles (runtime/targets), or 0 (full)
   before the settings exist. */
static u32 framing_display_mode(void) {
#if defined(UI_SETTINGS_SLOT) && defined(DISPLAY_MODE_SETTING)
    u32 settings=WORD(UI_SETTINGS_SLOT,0);
    return settings ? BYTE(settings,DISPLAY_MODE_SETTING) : 0;
#elif defined(DISPLAY_MODE_BYTE)
    return BYTE(DISPLAY_MODE_BYTE,0);
#else
#error "The build header defines DISPLAY_MODE_BYTE, or UI_SETTINGS_SLOT and DISPLAY_MODE_SETTING"
#endif
}
#define FRAMING_DISPLAY_MODE() framing_display_mode()
#endif

/* The capture aspect (CORE.ASPECT). */
static u32 aspect_native(void) { return FRAMING_NATIVE_ASPECT(); }
#include "aspect.h"
static u32 native_display(void) { return FRAMING_DISPLAY_MODE(); }
#include "settings.h"
#include "services.h"
#include "viewfinder.h"
/* The core exports these services to modules (runtime/sdk/grkit.h). INSTALL
   links each module call to it by name (grkit_core::grm::EXPORTS). The menu
   component exports setting_set, because a setting of CORE.ASPECT changes
   the native aspect. */
__attribute__((used)) void grkit_export_overlay_label(const Surface *s,u32 label,u32 left,u32 top) {
    overlay_label(s,label,left,top);
}
__attribute__((used)) u32 grkit_export_setting_get(u32 handle) { return setting_value(framing_header(),handle); }

/* The capture aspect id of the crop: 0 = 3:2, 1 = 4:3, 2 = 16:9. XPan and
   2.35:1 use id 2 with their own heights, and 5:4, 1:1, and 7:6 use id 1
   with their own widths (runtime/framing.S). GR III hides ids 1 and 2 from its menu, but
   its capture and playback code support them. */
static __attribute__((unused)) u32 framing_capture_id(volatile u32 *h) {
    static const u8 ids[]={0,2,2,1,2,1,1,1};
    return ids[framing_crop(h)];
}
#ifdef FRAMING_CAPTURE_ASPECT
/* The still aspect property handler calls this for each requested value. The
   native menu requests 0 for 3:2, so 3:2 records the Framing ratio. Other
   native ratios pass unchanged. Only a module that uses CORE.ASPECT hooks the
   handler. */
u32 framing_capture_request(u32 request) {
    return request ? request : framing_capture_id(framing_header());
}
/* The live view keeps 3:2. Resend the native 3:2 request when the capture
   aspect differs from the Framing ratio, for example after a menu change. */
static void capture_sync(u32 controller) {
    volatile u32 *h=framing_header();
    if(CAPABILITY_USED(h,GRKIT_CORE_ASPECT) && BYTE(controller,LIVEVIEW_MOVIE)==0 &&
       FRAMING_NATIVE_ASPECT()==0 && FRAMING_CAPTURE_ASPECT()!=framing_capture_id(h))
        FRAMING_REQUEST_ASPECT();
}
#ifdef FW_CAPTURE_ASPECT_GET
/* The live view follows the capture aspect where the native firmware has a
   live-view mode for each aspect (GR IV). A capture id 1 or 2 with the native
   3:2 UI aspect comes from the Framing request above, so the live view keeps
   mode 3:2. The live-view mode function and the mode check call this instead
   of the capture aspect getter. */
u32 framing_live_aspect(u32 property) {
    u32 id=((u32 (*)(u32))FW_CAPTURE_ASPECT_GET)(property);
    return (id==1 || id==2) && FRAMING_NATIVE_ASPECT()==0 ? 0 : id;
}
#endif
#else
/* Without the capture hooks a Framing ratio cannot be selected. */
#define capture_sync(controller) ((void)(controller))
#endif

#ifdef DEVELOP_SQUARE
/* RAW development of the DNG in playback. A DNG keeps the full sensor image.
   It records the capture aspect id, and its photo has the size of the crop:
   a Framing ratio records the id that it crops (16:9 for XPan and 2.35:1,
   4:3 for 5:4, 7:6, and 1:1). The development takes the size of its photo
   from the GetResolution hooks (runtime/framing.S), which read
   GRKIT_HOST_FRAME_FILE in playback, so it sets that byte from the size of
   the DNG, as the playback classifier does for a JPEG. */
static const unsigned short develop_sizes[]={XPAN_SIZES,SCOPE_SIZES,
#ifdef NARROW_SIZES
    NARROW_SIZES,
#endif
};
#ifndef FRAMING_PLAYBACK_METADATA
#define FRAMING_PLAYBACK_METADATA() (((u32 (*)(void))FW_PLAYBACK_METADATA)())
#endif
/* The size of the photo in playback, landscape: width in the low half. */
static u32 develop_size(void) {
    u32 data=FRAMING_PLAYBACK_METADATA();
    u32 width=HALF(data,METADATA_SIZE), height=HALF(data,METADATA_SIZE+4);
    return width<height ? height|width<<16 : width|height<<16;
}
/* GRKIT_HOST_FRAME_FILE for a photo size: 2 XPan, 3 2.35:1, 4 5:4, 5 7:6,
   else 1. */
static u32 develop_file(u32 size) {
    u32 i;
    for(i=0;i+1<sizeof(develop_sizes)/sizeof(develop_sizes[0]);i+=2)
        if(develop_sizes[i]==(size&0xffff) && develop_sizes[i+1]==size>>16) return i/8+2;
    return 1;
}
/* The initial development aspect for the recorded capture aspect, or -1 for
   the native choice. A Framing size develops the recorded aspect (0) with
   the geometry of the size. A Framing 1:1 records the 4:3 id with a native
   1:1 size, so it develops the native 1:1. Another recorded aspect that the
   camera cannot develop also develops as recorded, with its stock size. */
u32 framing_develop_select(u32 model,u32 aspect) {
    u32 size=develop_size(), geometry=develop_file(size);
    (void)model;
    ((volatile u8 *)framing_header())[GRKIT_HOST_FRAME_FILE]=(u8)geometry;
    if(geometry>1) return 0;
    if(aspect==1 && (size&0xffff)==size>>16) return DEVELOP_SQUARE;
    return aspect<4 && DEVELOP_NATIVE_ASPECTS>>aspect&1U ? (u32)-1 : 0;
}
/* Each time the development sends its aspect to the pipeline: a Framing
   ratio of its aspect list (GRKIT_HOST_DEVELOP_RATIO) takes the geometry of
   the ratio, the recorded aspect (0) the geometry of the DNG, and a native
   aspect that the user selects the stock sizes. */
void framing_develop_geometry(u32 model,u32 value) {
    volatile u32 *h=framing_header();
    u32 ratio=h[GRKIT_HOST_DEVELOP_RATIO/4];
    (void)model;
    ((volatile u8 *)h)[GRKIT_HOST_FRAME_FILE]=(u8)(ratio ? develop_file_of(ratio) : value ? 1 : develop_file(develop_size()));
}
/* The capture aspect that the development sends for its aspect: the
   capture aspect of a Framing ratio, else the native conversion. The aspect
   list keeps a native value for a ratio (runtime/menu.c), which the native
   conversion accepts. */
u32 framing_develop_capture(u32 capture) {
    u32 ratio=framing_header()[GRKIT_HOST_DEVELOP_RATIO/4];
    return ratio ? develop_capture(ratio) : capture;
}
#endif

#ifdef RESIZE_SIZE
/* Resize of the JPEG in playback. The camera resizes only the sizes of its
   resize size table: the stock sizes of each aspect and size class. A
   Framing size takes the stock size of its size class, the 16:9 size with
   its width or the 4:3 size with its height, so the camera resizes it. A
   resize target of that class then takes the Framing geometry of the photo
   (GRKIT_HOST_FRAME_FILE, which the playback classifier sets) again. 1:1
   sizes are stock sizes. */
static const unsigned short resize_wide[4][3]={{XPAN_HEIGHT_0},{XPAN_HEIGHT_1},{XPAN_HEIGHT_2},{XPAN_HEIGHT_3}};
static const unsigned short resize_narrow[4][4]={{NARROW_WIDTH_0},{NARROW_WIDTH_1},{NARROW_WIDTH_2},{NARROW_WIDTH_3}};
static const unsigned short resize_xpan[]={XPAN_SIZES}, resize_five[]={NARROW_SIZES};
/* The width of a 16:9 size class, and the height of a 4:3 size class. */
#define RESIZE_WIDTH(i) resize_xpan[(i)*2]
#define RESIZE_HEIGHT(i) resize_five[(i)*2+1]
void framing_resize_stock(u32 size) {
    u32 width=WORD(size,0), height=WORD(size,4), i;
    for(i=0;i<4;++i) {
        if(width==RESIZE_WIDTH(i) && (height==resize_wide[i][1] || height==resize_wide[i][2])) height=resize_wide[i][0];
        if(height==RESIZE_HEIGHT(i) && (width==resize_narrow[i][1] || width==resize_narrow[i][3])) width=resize_narrow[i][0];
    }
    WORD(size,0)=width; WORD(size,4)=height;
}
/* The store of a resize target size {width, height}. */
/* This function is a direct ARM BL patch target, without an assembly wrapper. */
#ifdef __arm__
#pragma clang attribute pop
#endif
void framing_resize_target(u32 size,u32 width,u32 height) {
    u32 file=((volatile u8 *)framing_header())[GRKIT_HOST_FRAME_FILE], i;
    for(i=0;i<4;++i) {
        if((file==2 || file==3) && width==RESIZE_WIDTH(i) && height==resize_wide[i][0]) height=resize_wide[i][file-1];
        if((file==4 || file==5) && height==RESIZE_HEIGHT(i) && width==resize_narrow[i][0]) width=resize_narrow[i][file==4 ? 1 : 3];
    }
    WORD(size,0)=width; WORD(size,4)=height;
}
#ifdef __arm__
#pragma clang attribute push(__attribute__((target("thumb"))), apply_to=function)
#endif
/* The aspect of an image size {width, height} in playback edits: a Framing
   size has the aspect of the id that it crops, so a resize takes its target
   size from the GetResolution hooks, and an edit the insets of its
   previews. */
u32 framing_resize_play_aspect(u32 aspect,u32 size) {
    u32 width=WORD(size,0), height=WORD(size,4), file;
    file=develop_file(width<height ? height|width<<16 : width|height<<16);
    if(file==2 || file==3) return RESIZE_JOB_WIDE;
    if(file==4 || file==5) return RESIZE_JOB_NARROW;
    return aspect;
}
#endif
/* Only the native still-photo LCD canvas owns these transient references. */
/* The highlight warning after INSTALL (runtime/level.h). */
static void level_boot(volatile u32 *h,u32 controller);
static void bind_canvas(volatile u32 *h,u32 controller) {
    /* Shooting: the capture sizes follow the crop that applies in the native
       aspect. A native aspect from the camera menu reads as Framing Off and
       keeps its stock sizes. */
    ((volatile u8 *)h)[GRKIT_HOST_FRAME_FILE]=0;
    h[GRKIT_HOST_FRAME_CROP/4]=capture_aspect_value(h);
    capture_sync(controller);
    viewfinder_bind(h,controller);
    level_boot(h,controller);
}
void framing_bind(u32 controller) {
    volatile u32 *h=framing_header();
    u32 start=profile_begin(h);
    bind_canvas(h,controller);
    profile_end(h,start,GRKIT_PROFILE_FRAME_BIND);
}
void framing_leave(u32 controller) {
    viewfinder_leave(framing_header(),controller);
}
#include "level.h"
/* Keep the firmware hook entrypoints while the service owns drawing. */
void framing_prepare(u32 canvas) { viewfinder_prepare(canvas); }
/* The paint of a full draw, the module layers included. */
void framing_paint(u32 canvas,u32 bitmap) {
    volatile u32 *h=framing_header();
    u32 start=profile_begin(h);
    paint_canvas(h,canvas,bitmap);
    profile_end(h,start,GRKIT_PROFILE_FRAME_PAINT);
}
#ifdef __arm__
#pragma clang attribute pop
#endif
