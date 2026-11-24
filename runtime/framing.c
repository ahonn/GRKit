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

/* The capture aspect id of the crop: 0 = 3:2, 1 = 4:3, 2 = 16:9
   (FRAMING_RATIO_CAPTURES). The wide ratios, such as XPan, use id 2 with
   their own heights, and the narrow ones and 1:1 use id 1 with their own
   widths (runtime/framing.S). GR III hides ids 1 and 2 from its menu, but
   its capture and playback code support them. */
static __attribute__((unused)) u32 framing_capture_id(volatile u32 *h) {
    static const u8 ids[GRKIT_CORE_ASPECT_VALUES]={FRAMING_RATIO_CAPTURES};
    return ids[framing_crop(h)];
}
/* The geometry of a photo size: its GRKIT_HOST_FRAME_FILE | its size class
   + 1 << 8 (grkit_core::aspect::geometry::file_of_size). A Framing size, in
   either orientation, has its file and size class. A crop of a ratio with
   its own geometry (runtime/crop.h) has the file of the ratio and no size
   class: a size with the ratio p : q of the file within the rounding of each
   side (FRAMING_CROP_SLACK), and only 3:4 taller than wide. Another size
   has file 1. The playback classifier (runtime/framing.S), the RAW
   development, and the resize take the geometry of a photo from it. */
u32 framing_size_file(u32 width,u32 height) {
    static const u16 sizes[]={FRAMING_SIZES};
    static const u8 ratios[FRAMING_FILES][2]={FRAMING_FILE_RATIOS};
    u32 longer=width<height ? height : width, shorter=width<height ? width : height, i, p, q, a, b;
    _Static_assert(sizeof(sizes)==(FRAMING_FILES-2)*4*2*sizeof(sizes[0]),"four size classes for each file");
    for(i=0;i<sizeof(sizes)/sizeof(sizes[0])/2;++i)
        if(sizes[2*i]==longer && sizes[2*i+1]==shorter) return (i/4+2)|((i&3U)+1U)<<8;
    if(shorter<FRAMING_CROP_SHORTEST) return 1;
    for(i=2;i<FRAMING_FILES;++i) {
        p=ratios[i][0]; q=ratios[i][1];
        if(!p || (width<height)!=(p<q)) continue;
        a=width*q; b=height*p;
        if((a>b ? a-b : b-a)<=FRAMING_CROP_SLACK*(p+q)) return i;
    }
    return 1;
}
/* The capture aspect id of a Framing photo size or of a crop of a ratio, or
   0 for another size. */
u32 framing_size_capture(u32 width,u32 height) {
    static const u8 captures[FRAMING_FILES]={FRAMING_FILE_CAPTURES};
    return captures[framing_size_file(width,height)&0xffU];
}
/* The geometry mode of GetResolution of capture aspect id 2 (`wide`) or 1,
   as runtime/framing.S: wide_mode and narrow_mode. */
static u32 framing_mode(volatile u32 *h,u32 wide) {
    static const u8 modes[2][2][FRAMING_MODE_INDEXES]={{{FRAMING_NARROW_BY_CROP},{FRAMING_NARROW_BY_FILE}},
                                                       {{FRAMING_WIDE_BY_CROP},{FRAMING_WIDE_BY_FILE}}};
    u32 file=((volatile u8 *)h)[GRKIT_HOST_FRAME_FILE], index=file ? file : h[GRKIT_HOST_FRAME_CROP/4];
    return index<FRAMING_MODE_INDEXES ? modes[wide][file!=0][index] : 0;
}
#if defined(FW_DEVELOP_MODEL) && defined(DEVELOP_SETTINGS) && defined(CROP_FOCALS)
#define FRAMING_DEVELOP_CROP 1
/* Whether a RAW development runs (runtime/picture.c: PICTURE_DEVELOPING),
   the settings that it reads from its DNG, and the playback metadata of
   that DNG (runtime/targets). */
#ifndef FRAMING_DEVELOPING
#define FRAMING_DEVELOPING() ((BYTE(FW_DEVELOP_GUARD,0)&1U) && BYTE(FW_DEVELOP_MODEL,DEVELOP_STATE))
#endif
#ifndef FRAMING_DEVELOP_SETTINGS
#define FRAMING_DEVELOP_SETTINGS() WORD(FW_DEVELOP_MODEL,DEVELOP_SETTINGS)
#endif
#ifndef FRAMING_PLAYBACK_METADATA
#define FRAMING_PLAYBACK_METADATA() (((u32 (*)(void))FW_PLAYBACK_METADATA)())
#endif
/* The crop factor of the DNG of the RAW development where the camera does
   not have its crop (runtime/crop.h: crop_of_photo), or 0. The camera knows
   only its own factors, so it reads a DNG of 40 or 70 mm (GR IIIx 57 or 100
   mm) as one without a crop, or with the factor of its crop step. Its Exif
   35 mm focal length names the crop. */
static u32 develop_crop_factor(void) {
    u32 settings=FRAMING_DEVELOP_SETTINGS(), crop;
    if(!settings) return 0;
    crop=crop_of_photo(BYTE(FRAMING_PLAYBACK_METADATA(),METADATA_FOCAL_35MM),WORD(settings,SHOT_CROP));
    return crop ? crop_factors[crop] : 0;
}
/* The start of a RAW development (runtime/picture.c: picture_develop_start):
   a DNG of such a crop develops at the factor of its crop step, as the
   camera kept it for its shot, so that the development knows the factor.
   crop_shot_copy gives the photo the factor of the crop. */
void framing_develop_crop(void) {
    u32 own=develop_crop_factor();
    if(own) WORD(FRAMING_DEVELOP_SETTINGS(),SHOT_CROP)=crop_step_factor(own);
}
#else
#define FRAMING_DEVELOPING() 0
#define develop_crop_factor() 0U
#endif
/* The crop factor of a shot or of a RAW development, as float bits at
   `address`, in its copy of the capture settings (runtime/framing.S:
   crop_shot). The factor of a crop that the camera does not have takes the
   place of the factor of its crop step: that of the next shot
   (GRKIT_HOST_CROP_SHOT, crop_sync) while the still live view is bound, and
   that of the DNG while a development runs, where it also takes the place
   of the factor 1, as which the development can read it. */
void crop_shot_copy(u32 address) {
    volatile u32 *h=framing_header();
    u32 now=WORD(address,0), own;
    if(FRAMING_DEVELOPING()) {
        own=develop_crop_factor();
        if(own && (now==crop_factors[0] || now==crop_step_factor(own))) WORD(address,0)=own;
        return;
    }
    own=h[GRKIT_HOST_CROP_SHOT/4];
    if(own && h[GRKIT_HOST_FRAME_OWNER/4] && now==crop_step_factor(own)) WORD(address,0)=own;
}
/* The crop factor `bits` of GetResolution: in a RAW development of such a
   crop, the factor of its crop step, which the development reads
   (framing_develop_crop), is the factor of the crop, so that each size of
   the development, such as its JPEG Recorded Pixels, is that of the
   photo. */
static u32 develop_crop_bits(u32 bits) {
    u32 own;
    if(!FRAMING_DEVELOPING()) return bits;
    own=develop_crop_factor();
    return own && bits==crop_step_factor(own) ? own : bits;
}
/* GetResolution of a crop (runtime/framing.S: crop_resolution): the size
   {width | height << 16} of size class `size` of capture aspect id `aspect`
   (3 is 1:1) at the crop factor `bits`, which is not 1, or 0 for the size
   of the camera at the factor of the crop step. Up to its crop step, a crop
   is the L size of the aspect and its geometry divided by the factor, each
   side to a multiple of 16 (crop_side): the window and the photo, one pixel
   each, as a crop of the camera gives its own size for L. A smaller size
   class takes the size of the camera. A crop of the camera's own factor in
   a geometry that the camera has keeps the camera's sizes. The sizes of
   each ratio are those of grkit_core::aspect::geometry::crop_size. */
u32 crop_resolution_size(u32 size,u32 aspect,u32 bits) {
    static const u16 widths[]={FRAMING_STOCK_WIDTHS}, heights[]={FRAMING_STOCK_HEIGHTS}, native[]={FRAMING_NATIVE_HEIGHTS};
    static const u16 wide_heights[]={FRAMING_WIDE_HEIGHT_0}, narrow_widths[]={FRAMING_NARROW_WIDTH_0};
    static const u8 wide_framed[]={FRAMING_WIDE_FRAMED}, narrow_framed[]={FRAMING_NARROW_FRAMED};
    volatile u32 *h=framing_header();
    u32 width=widths[0], height=native[0], framed=0, mode;
    bits=develop_crop_bits(bits);
    /* A factor below 1, such as 0.0 that an early build saved, or not a
       number, is no crop: the division below would not end. */
    if(bits<0x3f800000U || bits>=0x7f800000U || size>crop_step(bits) || aspect>3) return 0;
    if(aspect==2) {
        mode=framing_mode(h,1);
        height=wide_heights[mode];
        framed=wide_framed[mode];
    } else if(aspect==1) {
        mode=framing_mode(h,0);
        width=narrow_widths[mode];
        height=heights[0];
        framed=narrow_framed[mode];
    } else if(aspect==3) {
        /* The native 1:1 side of L is the 4:3 height. */
        width=height=heights[0];
    }
    if(!framed && (bits==CROP_NATIVE_35 || bits==CROP_NATIVE_50)) return 0;
    return crop_side(width,bits)|crop_side(height,bits)<<16;
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
   aspect differs from the Framing ratio, for example after a menu change.
   As a crop change, the request waits while the back end makes a shot
   (runtime/crop.h: crop_busy). */
static void capture_sync(u32 controller) {
    volatile u32 *h=framing_header();
    if(CAPABILITY_USED(h,GRKIT_CORE_ASPECT) && BYTE(controller,LIVEVIEW_MOVIE)==0 && !crop_busy(h) &&
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
   a Framing ratio records the id that it crops (FRAMING_RATIO_CAPTURES). The
   development takes the size of its photo from the GetResolution hooks
   (runtime/framing.S), which read GRKIT_HOST_FRAME_FILE in playback, so it
   sets that byte from the size of the DNG, as the playback classifier does
   for a JPEG. */
#ifndef FRAMING_PLAYBACK_METADATA
#define FRAMING_PLAYBACK_METADATA() (((u32 (*)(void))FW_PLAYBACK_METADATA)())
#endif
/* The size of the photo in playback: width in the low half. */
static u32 develop_size(void) {
    u32 data=FRAMING_PLAYBACK_METADATA();
    return HALF(data,METADATA_SIZE)|HALF(data,METADATA_SIZE+4)<<16;
}
/* GRKIT_HOST_FRAME_FILE for a photo size: its Framing geometry, also that
   of a crop of a ratio, else 1. */
static u32 develop_file(u32 size) { return framing_size_file(size&0xffffU,size>>16)&0xffU; }
/* The Framing ratio that the photo of the DNG records, or 0: the record of
   the DNG, which the start of a development reads (runtime/picture.c). */
#ifndef FRAMING_DEVELOP_RATIO
u32 picture_develop_ratio(void);
#define FRAMING_DEVELOP_RATIO() picture_develop_ratio()
#endif
static u32 develop_recorded_ratio(volatile u32 *h) {
    return h[GRKIT_HOST_SERVICES/4] ? services_word(h,SERVICES_DEVELOP)[DEVELOP_COPY_ASPECT/4] : 0;
}
/* The geometry of the photo of the DNG: that of its size, else that of the
   Framing ratio `ratio` that it records. A photo with a Border has the size
   of its canvas, which no geometry has, so it developed at the stock size
   of its capture aspect, such as 4:3 for 7:6. */
static u32 develop_photo_file(u32 ratio) {
    u32 file=develop_file(develop_size());
    return file==1 && ratio ? develop_file_of(ratio) : file;
}
/* The initial development aspect for the recorded capture aspect, or -1 for
   the native choice. A Framing size develops the recorded aspect (0) with
   the geometry of the size. A Framing 1:1 records the 4:3 id with a native
   1:1 size, so it develops the native 1:1. Another recorded aspect that the
   camera cannot develop also develops as recorded, with its stock size. */
u32 framing_develop_select(u32 model,u32 aspect) {
    u32 size=develop_size(), geometry=develop_photo_file(FRAMING_DEVELOP_RATIO());
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
    ((volatile u8 *)h)[GRKIT_HOST_FRAME_FILE]=(u8)(ratio ? develop_file_of(ratio) : value ? 1 : develop_photo_file(develop_recorded_ratio(h)));
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
   (GRKIT_HOST_FRAME_FILE, which the playback classifier sets) again. The
   sizes are those of each mode of runtime/framing.S, mode 0 the stock one.
   1:1 sizes are stock sizes: no file has the 1:1 mode. The getter gives the
   longer side first, so a 3:4 photo arrives as {height, width} and takes
   the 4:3 size with that height. Its resize target stays 3:4, because the
   resize takes the target from the GetResolution hooks. A crop of a ratio
   (runtime/crop.h), 3:4 also as {height, width}, takes the stock size of
   the smallest size class of its geometry that holds it, so the camera
   offers only smaller sizes. */
static const u16 resize_wide[4][FRAMING_WIDE_MODES]={{FRAMING_WIDE_HEIGHT_0},{FRAMING_WIDE_HEIGHT_1},
                                                     {FRAMING_WIDE_HEIGHT_2},{FRAMING_WIDE_HEIGHT_3}};
static const u16 resize_narrow[4][FRAMING_NARROW_MODES]={{FRAMING_NARROW_WIDTH_0},{FRAMING_NARROW_WIDTH_1},
                                                         {FRAMING_NARROW_WIDTH_2},{FRAMING_NARROW_WIDTH_3}};
static const u16 resize_widths[4]={FRAMING_STOCK_WIDTHS}, resize_heights[4]={FRAMING_STOCK_HEIGHTS};
static const u8 resize_wide_modes[FRAMING_MODE_INDEXES]={FRAMING_WIDE_BY_FILE};
static const u8 resize_narrow_modes[FRAMING_MODE_INDEXES]={FRAMING_NARROW_BY_FILE};
void framing_resize_stock(u32 size) {
    u32 width=WORD(size,0), height=WORD(size,4), i, file, wide, narrow, side=height, turned;
    for(i=0;i<4;++i) for(file=2;file<FRAMING_FILES;++file) {
        wide=resize_wide_modes[file]; narrow=resize_narrow_modes[file];
        if(wide && width==resize_widths[i] && height==resize_wide[i][wide]) { WORD(size,4)=resize_wide[i][0]; return; }
        if(narrow && height==resize_heights[i] && width==resize_narrow[i][narrow]) { WORD(size,0)=resize_narrow[i][0]; return; }
        if(narrow && width==resize_heights[i] && height==resize_narrow[i][narrow]) { WORD(size,0)=resize_narrow[i][0]; WORD(size,4)=resize_heights[i]; return; }
    }
    file=framing_size_file(width,height);
    /* A 3:4 crop as {height, width}, which the playback classifier saw as
       3:4 (GRKIT_HOST_FRAME_FILE): a 4:3 size also has the ratio of 3:4
       turned. */
    turned=framing_size_file(height,width);
    if(file==1 && width>height && turned>1 && turned==((volatile u8 *)framing_header())[GRKIT_HOST_FRAME_FILE]) {
        file=turned;
        side=width;
    }
    if(file>>8) return;
    file&=0xffU; wide=resize_wide_modes[file]; narrow=resize_narrow_modes[file];
    for(i=4;i--;) {
        if(wide && resize_widths[i]>=width) { WORD(size,0)=resize_widths[i]; WORD(size,4)=resize_wide[i][0]; return; }
        if(narrow && resize_heights[i]>=side) { WORD(size,0)=resize_narrow[i][0]; WORD(size,4)=resize_heights[i]; return; }
    }
}
/* The store of a resize target size {width, height}. */
/* This function is a direct ARM BL patch target, without an assembly wrapper. */
#ifdef __arm__
#pragma clang attribute pop
#endif
void framing_resize_target(u32 size,u32 width,u32 height) {
    u32 file=((volatile u8 *)framing_header())[GRKIT_HOST_FRAME_FILE], i, wide=0, narrow=0;
    if(file<FRAMING_FILES) { wide=resize_wide_modes[file]; narrow=resize_narrow_modes[file]; }
    for(i=0;i<4;++i) {
        if(wide && width==resize_widths[i] && height==resize_wide[i][0]) height=resize_wide[i][wide];
        if(narrow && height==resize_heights[i] && width==resize_narrow[i][0]) width=resize_narrow[i][narrow];
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
    u32 capture=framing_size_capture(WORD(size,0),WORD(size,4));
    if(capture==2) return RESIZE_JOB_WIDE;
    if(capture==1) return RESIZE_JOB_NARROW;
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
    /* The camera's crop factor follows CORE.CROP in the still live view. */
#ifdef CROP_NATIVE_BITS
    if(BYTE(controller,LIVEVIEW_MOVIE)==0) crop_sync(h);
#if CROP_LIVE_FACTOR
    if(crop_live_due(h,BYTE(controller,LIVEVIEW_MOVIE)!=0) && !crop_busy(h)) crop_live_try(h,FRAMING_SECOND());
#endif
#endif
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
/* The paint after the native children of the canvas (runtime/framing.S). */
void framing_paint_after(u32 canvas,u32 bitmap) { paint_after(framing_header(),canvas,bitmap); }
#ifdef __arm__
#pragma clang attribute pop
#endif
/* The live-view crop badge of the camera (runtime/framing.S: crop_badge):
   0 hides it where the core draws its own (runtime/crop.h). */
u32 crop_badge_native(void) {
    return crop_badge_label(framing_header())>=GRKIT_LABEL_COUNT;
}
/* The camera's grid guide in the live view (runtime/framing.S:
   framing_grid): not 0 hides it (runtime/viewfinder.h). */
u32 framing_grid_hidden(void) {
    return native_grid_hidden(framing_header());
}
