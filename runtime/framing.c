/* The core of viewfinder.overlay and the C part of capture.aspect. The live
   view canvas, its bitmap, and the native aspect are firmware details, so they
   stay here. The layers of the modules paint on a checked surface. */
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
#ifndef LABEL_MAP
#define LABEL_MAP(address) ((const volatile u8 *)(address))
#endif
/* A u16 or u32 field of the label table (runtime/abi.h). */
static u32 label_field(const volatile u8 *table,u32 at,u32 bytes) {
    u32 value=0,i;
    for(i=0;i<bytes;++i) value|=(u32)table[at+i]<<(8*i);
    return value;
}
static u32 label_width(u32 label) {
    return label<GRKIT_LABEL_COUNT ? label_field(LABEL_MAP(GRKIT_LABELS(framing_header())),label*8,2) : 0;
}
/* White text with a black outline and no background, as the native live-view
   text has it: each drawn pixel is opaque, and text edges blend toward the
   outline in RGB. A transparent pixel keeps the canvas, so a label does not
   cut the Framing mask. Canvas::Draw clears the bitmap before each draw
   (framing_prepare), which removes the previous label. */
static void overlay_label(const Surface *s,u32 label,u32 left,u32 top) {
    const volatile u8 *table=LABEL_MAP(GRKIT_LABELS(framing_header())),*pixels;
    u32 width,height,x,y,value;
    volatile u32 *row;
    if(label>=GRKIT_LABEL_COUNT) return;
    width=label_field(table,label*8,2);
    height=label_field(table,label*8+2,2);
    if(!width || left+width>720 || top+height>480) return;
    /* Each row holds its spans of drawn pixels (grkit_core::artwork::
       overlay_labels): a count, then for each a u16 first column, a u16
       length, and its pixels. The table does not change while GRKit runs,
       so a plain pointer reads it. */
    pixels=table+label_field(table,label*8+4,4);
    {
        const u8 *at=(const u8 *)pixels;
        for(y=0;y<height;++y) {
            u32 spans=*at++;
            row=OVERLAY_PIXELS(s->data+(top+y)*s->stride*4)+left;
            while(spans--) {
                u32 first=at[0]|(u32)at[1]<<8, length=at[2]|(u32)at[3]<<8;
                volatile u32 *to=row+first;
                at+=4;
                if(first+length>width) return;
                for(x=0;x<length;++x) { value=at[x]; to[x]=0xff000000U|value*0x010101U; }
                at+=length;
            }
        }
    }
}

#include "settings.h"
#include "services.h"
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
/* ---- Load cue -----------------------------------------------------------
   INSTALL runs a few seconds after power-on, and the GRKit entry exists only
   after it. The first still live view after INSTALL therefore shows the
   GRKit label for CUE_SECONDS - 1 to CUE_SECONDS seconds. GRKIT_HOST_LOAD_CUE holds the
   state (runtime/abi.h). */
#define CUE_PENDING 1U
#define CUE_SECONDS 4U
#ifndef FRAMING_SECOND
/* The local second of the day. DateTime: u16 year, then u8 month, day, hour,
   minute, and second. */
static u32 framing_second(void) {
    u32 date[3];
    const u8 *dt=(const u8 *)date;
    ((void (*)(u32,u32))FW_LOCAL_TIME)(WORD(((u32 (*)(void))FW_CLOCK)(),4),(u32)date);
    return dt[4]*3600U+dt[5]*60U+dt[6];
}
#define FRAMING_SECOND() framing_second()
#endif
static u32 cue_shown(volatile u32 *h) { return h[GRKIT_HOST_LOAD_CUE/4]>CUE_PENDING; }
/* Start the cue in a still live view, and end it after CUE_SECONDS. Returns 1
   when the cue starts or ends, so the canvas needs a redraw. */
static u32 cue_update(volatile u32 *h,u32 controller) {
    u32 state=h[GRKIT_HOST_LOAD_CUE/4],now,start;
    if(!state) return 0;
    if(state==CUE_PENDING) {
        if(!h[GRKIT_HOST_FRAME_CANVAS/4] || BYTE(controller,LIVEVIEW_MOVIE)) return 0;
        h[GRKIT_HOST_LOAD_CUE/4]=FRAMING_SECOND()+2;
        return 1;
    }
    now=FRAMING_SECOND();
    start=state-2;
    /* The cue started before midnight. */
    if(now<start) now+=24U*3600U;
    if(now-start<CUE_SECONDS) return 0;
    h[GRKIT_HOST_LOAD_CUE/4]=0;
    return 1;
}
/* The module layers that INSTALL registered (GRKIT_HOST_MODULE_LAYERS), or 0
   for a count out of range. Each has three words: its paint function, its
   setting handle, and its flags with its order in the high half. */
static u32 module_layers(volatile u32 *h) {
    u32 n=h[GRKIT_HOST_MODULE_LAYER_COUNT/4];
    return n<=GRKIT_MODULE_LAYERS_MAX ? n : 0;
}
#define MODULE_LAYER(h,i) ((h)+GRKIT_HOST_MODULE_LAYERS/4+(i)*3)
/* The flags of a layer (runtime/sdk/grkit.h). */
#define MODULE_LAYER_FOLLOW_DISP 1U
#define MODULE_LAYER_FULL_DISP 2U
#define MODULE_LAYER_LIVE_ONLY 4U
#define NO_SETTING 0xffffffffU
/* The value of module layer i in the live view: its setting, or 1 without
   one. A layer is off while DISP hides what its flags need, and while a soft
   Off stops its module. */
static u32 module_layer_value(volatile u32 *h,u32 i) {
    volatile u32 *l=MODULE_LAYER(h,i);
    if(services_suspended(h,l[0])) return 0;
    if((l[2]&MODULE_LAYER_FOLLOW_DISP) && native_display()>DISPLAY_SIMPLE) return 0;
    if((l[2]&MODULE_LAYER_FULL_DISP) && native_display()!=DISPLAY_FULL) return 0;
    return l[1]==NO_SETTING ? 1 : setting_value(h,l[1]);
}
static u32 layers_active(volatile u32 *h) {
    u32 i;
    for(i=0;i<module_layers(h);++i) if(module_layer_value(h,i)) return 1;
    return 0;
}
/* Paint module layer i with `value`, which is not 0. A host test maps the
   function address. */
#ifndef MODULE_PAINT
#define MODULE_PAINT(address,surface,value) ((void (*)(const Surface *,u32))(address))(surface,value)
#endif
static void module_paint(volatile u32 *h,u32 i,const Surface *surface,u32 value) {
    u32 start=profile_begin(h);
    MODULE_PAINT(MODULE_LAYER(h,i)[0],surface,value);
    profile_end_module(h,start,MODULE_LAYER(h,i)[0],GRKIT_PROFILE_PAINT);
}
/* Paint the module layers in their order. In the ADJ preview, the layers of
   the previewed setting paint its highlighted value, DISP does not hide
   layers, and a live-only layer does not paint. */
static void module_layers_paint(volatile u32 *h,const Surface *surface,u32 preview) {
    u32 i, value;
    for(i=0;i<module_layers(h);++i) {
        if(!preview) value=module_layer_value(h,i);
        else if(MODULE_LAYER(h,i)[2]&MODULE_LAYER_LIVE_ONLY) value=0;
        else if(h[GRKIT_HOST_MODULE_PREVIEW/4] && h[GRKIT_HOST_MODULE_PREVIEW/4]-1==MODULE_LAYER(h,i)[1]) {
            value=h[GRKIT_HOST_MODULE_PREVIEW/4+1];
            if(value>=setting_limit(h,MODULE_LAYER(h,i)[1])) value=0;
        }
        else value=MODULE_LAYER(h,i)[1]==NO_SETTING ? 1 : setting_value(h,MODULE_LAYER(h,i)[1]);
        if(value && !services_suspended(h,MODULE_LAYER(h,i)[0])) module_paint(h,i,surface,value);
    }
}
/* Whether the live view has something to draw: a layer or the cue. */
static u32 overlay_active(volatile u32 *h) { return cue_shown(h) || layers_active(h); }
/* The capture area: the crop of CORE.ASPECT, or 0 for the full 3:2 area. */
static u32 capture_area(volatile u32 *h) { return framing_crop(h); }

#ifdef FRAMING_XPAN_NATIVE
/* The live view shows the Framing ratio in native 3:2. XPan and 2.35:1 also
   crop the native aspect FRAMING_XPAN_NATIVE, whose live view shows only
   them. In other native aspects nothing shows: clear `data`. */
#define SHOWN_AREA(data,area) do { \
    u32 native=FRAMING_NATIVE_ASPECT(); \
    if(native!=0 && (native!=FRAMING_XPAN_NATIVE || ((area)!=1 && (area)!=4))) (data)=0; \
} while(0)
#else
/* canvas_pixels() accepts only the native 3:2 aspect, where each ratio shows. */
#define SHOWN_AREA(data,area) ((void)0)
#endif

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
/* Only the native still-photo LCD canvas owns these transient references. */
/* The highlight warning after INSTALL (runtime/level.h). */
static void level_boot(volatile u32 *h,u32 controller);
static void bind_canvas(volatile u32 *h,u32 controller) {
    h[GRKIT_HOST_FRAME_OWNER/4]=controller;
    h[GRKIT_HOST_FRAME_CANVAS/4]=WORD(controller,LIVEVIEW_CANVAS);
    /* Shooting: the capture sizes follow the crop that applies in the native
       aspect. A native aspect from the camera menu reads as Framing Off and
       keeps its stock sizes. */
    ((volatile u8 *)h)[GRKIT_HOST_FRAME_FILE]=0;
    h[GRKIT_HOST_FRAME_CROP/4]=aspect_value(h);
    capture_sync(controller);
    /* Nothing else invalidates a still live view, so these need a redraw: a
       selection that has not drawn on this canvas yet, such as one restored
       at power-on, a cue that starts or ends, and a setting that changed
       since the last redraw, such as a ratio or a mask style from ADJ or a
       native settings page. A crop does not restart the live view. */
    if(h[GRKIT_HOST_FRAME_CANVAS/4]) {
        u32 changes=h[GRKIT_HOST_STATE_CHANGES/4], changed=h[GRKIT_HOST_FRAME_CHANGES/4]!=changes;
        h[GRKIT_HOST_FRAME_CHANGES/4]=changes;
        if(cue_update(h,controller) || changed || (!h[GRKIT_HOST_FRAME_BITMAP/4] && overlay_active(h)))
            FRAMING_INVALIDATE(h[GRKIT_HOST_FRAME_CANVAS/4]);
    }
    /* The first shooting screen after INSTALL: the modules are ready. This
       hook runs in the App task. */
    if(h[GRKIT_HOST_FRAME_CANVAS/4]) services_ready(h);
    level_boot(h,controller);
}
void framing_bind(u32 controller) {
    volatile u32 *h=framing_header();
    u32 start=profile_begin(h);
    bind_canvas(h,controller);
    profile_end(h,start,GRKIT_PROFILE_FRAME_BIND);
}
void framing_leave(u32 controller) {
    volatile u32 *h=framing_header();
    if(h[GRKIT_HOST_FRAME_OWNER/4]==controller) {
        h[GRKIT_HOST_FRAME_OWNER/4]=0;
        h[GRKIT_HOST_FRAME_CANVAS/4]=0;
    }
}
/* Pixel data of a full 720x480 32-bit canvas at the origin, or 0. */
static u32 canvas_bitmap(u32 canvas,u32 bitmap) {
    u32 stride,data;
    if(!BYTE(canvas,0x9e) || WORD(canvas,0x20) || WORD(canvas,0x24) ||
       WORD(canvas,0x28)!=720 || WORD(canvas,0x2c)!=480 ||
       BYTE(bitmap,2)!=5 || HALF(bitmap,0x1c)!=720 || HALF(bitmap,0x1e)!=480) return 0;
    stride=HALF(bitmap,0x18);
    data=WORD(bitmap,8);
    /* The OSD buffer moves between boots (0x5f83df80 and 0x619e1200 seen). */
    if(stride<720 || stride>1024 || HALF(bitmap,0x1a)!=480 ||
       data<0x40000000 || data>0x80000000-stride*480*4 || (data&3) ||
       WORD(bitmap,4)<stride*480*4) return 0;
    return data;
}
/* Pixel data of a supported canvas bitmap, or 0. The layers are defined only
   for a full canvas and the native 3:2 aspect. */
static u32 canvas_pixels(u32 canvas,u32 bitmap) {
#ifndef FRAMING_XPAN_NATIVE
    /* Native StillAspectRatio getter: 0 = 3:2, 3 = 1:1. Native square crop is untouched. */
    if(FRAMING_NATIVE_ASPECT()!=0) return 0;
#endif
    return canvas_bitmap(canvas,bitmap);
}
/* Photo area {left,top,right,bottom} on the 720x480 canvas for each capture
   area: 3:2, XPan, 16:9, 4:3, 2.35:1, 1:1, 5:4, 7:6. */
static const u16 photo_boxes[8][4]={{0,0,720,480},{0,107,720,373},{0,37,720,442},{40,0,680,480},
                                    {0,87,720,393},{120,0,600,480},{60,0,660,480},{80,0,640,480}};
/* The ADJ canvas of the preview, or 0 (runtime/modules.h: frame_preview). */
static u32 preview_canvas(volatile u32 *h) {
    u32 adj=h[GRKIT_HOST_FRAME_PREVIEW_ADJ/4];
    return adj ? WORD(adj,ADJ_CANVAS) : 0;
}
static void prepare_canvas(volatile u32 *h,u32 canvas) {
    /* Canvas::Draw clears the full native bitmap before drawing its children.
       Use this path even for Off to remove the previous drawing on the next draw. */
    if((h[GRKIT_HOST_FRAME_CANVAS/4]==canvas &&
        (overlay_active(h) || h[GRKIT_HOST_FRAME_BITMAP/4])) ||
       (canvas && preview_canvas(h)==canvas))
        BYTE(canvas,0xa0)=1;
}
void framing_prepare(u32 canvas) {
    volatile u32 *h=framing_header();
    u32 start=profile_begin(h);
    prepare_canvas(h,canvas);
    profile_end(h,start,GRKIT_PROFILE_FRAME_PREPARE);
}
/* Module pages preview their highlighted value on the ADJ canvas, below the
   menu controls (runtime/modules.h: frame_preview). The page of CORE.ASPECT
   previews the area of its highlighted ratio. After the page ends, one more
   full draw erases the drawing and releases the canvas. */
static void preview_paint(volatile u32 *h,u32 canvas,u32 bitmap) {
    u32 preview=h[GRKIT_HOST_FRAME_PREVIEW/4],data,area;
    Surface surface;
    if(!preview) { h[GRKIT_HOST_FRAME_PREVIEW_ADJ/4]=0; return; }
    if(preview>sizeof(photo_boxes)/sizeof(photo_boxes[0]) || !module_layers(h)) return;
    data=canvas_pixels(canvas,bitmap);
    area=preview-1;
    SHOWN_AREA(data,area);
    if(!data) return;
    surface.data=data; surface.stride=HALF(bitmap,0x18); surface.area=area; surface.box=photo_boxes[area];
    module_layers_paint(h,&surface,1);
}
static void paint_canvas(volatile u32 *h,u32 canvas,u32 bitmap) {
    u32 owner=h[GRKIT_HOST_FRAME_OWNER/4],full,data,cue,area;
    Surface surface;
    if(canvas && canvas==preview_canvas(h)) { preview_paint(h,canvas,bitmap); return; }
    if(canvas!=h[GRKIT_HOST_FRAME_CANVAS/4] || !owner) return;
    if(!overlay_active(h)) { h[GRKIT_HOST_FRAME_BITMAP/4]=0; return; }
    h[GRKIT_HOST_FRAME_BITMAP/4]=bitmap;
    /* The LCD still canvas is registered in native slot zero. Unsupported
       canvas sizes and bitmap formats leave the native image untouched. */
    full=WORD(owner,4)==canvas && BYTE(owner,LIVEVIEW_MOVIE)==0 ? canvas_bitmap(canvas,bitmap) : 0;
    data=full && layers_active(h) ? canvas_pixels(canvas,bitmap) : 0;
    /* The cue does not depend on the photo area, so it shows in each native
       aspect. It hides only while DISP turns the live view off. */
    cue=full && cue_shown(h) && native_display()!=DISPLAY_OFF ? full : 0;
    area=capture_area(h);
    SHOWN_AREA(data,area);
    if(!data && !cue) return;
    if(data) {
        surface.data=data; surface.stride=HALF(bitmap,0x18); surface.area=area; surface.box=photo_boxes[area];
        module_layers_paint(h,&surface,0);
    }
    if(cue) {
        /* Centered, on the baseline of the native focus distance. */
        surface.data=cue; surface.stride=HALF(bitmap,0x18); surface.area=area; surface.box=photo_boxes[area];
        overlay_label(&surface,LABEL_GRKIT,360-label_width(LABEL_GRKIT)/2,LABEL_TOP);
    }
    ++h[GRKIT_HOST_FRAME_DRAWS/4];
}
#include "level.h"
/* The paint of a full draw, the module layers included. */
void framing_paint(u32 canvas,u32 bitmap) {
    volatile u32 *h=framing_header();
    u32 start=profile_begin(h);
    paint_canvas(h,canvas,bitmap);
    profile_end(h,start,GRKIT_PROFILE_FRAME_PAINT);
}
