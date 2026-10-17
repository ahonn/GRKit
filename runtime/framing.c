#include "abi.h"
#include "firmware.h"

typedef unsigned int u32;
typedef unsigned short u16;
typedef unsigned char u8;
#ifndef WORD
#define WORD(p,o) (*(volatile u32 *)((p)+(o)))
#endif
#ifndef HALF
#define HALF(p,o) (*(volatile u16 *)((p)+(o)))
#endif
#ifndef BYTE
#define BYTE(p,o) (*(volatile u8 *)((p)+(o)))
#endif
#ifndef FRAMING_PIXELS
#define FRAMING_PIXELS(address) ((volatile u32 *)(address))
#endif
#define FRAMING_SETTINGS() (((u32 (*)(void))FW_SETTINGS)())
#ifndef FRAMING_NATIVE_ASPECT
#define FRAMING_NATIVE_ASPECT() (((u32 (*)(u32))FW_STILL_ASPECT_GET)(FRAMING_SETTINGS()))
#endif
#ifdef FW_CAPTURE_PROPERTIES
#ifndef FRAMING_CAPTURE_ASPECT
/* Requested capture StillAspectRatio (CommonProperty +0x7b6). The property
   setter writes the next copy at [property+4]; readers see the current copy
   at [property] only after a later commit. Comparing the next copy sends
   each change once. */
#define FRAMING_CAPTURE_ASPECT() BYTE(WORD(WORD(WORD(((u32 (*)(void))FW_CAPTURE_PROPERTIES)(),0),0x518),4),0x7b6)
#endif
#ifndef FRAMING_REQUEST_ASPECT
/* Native StillAspectRatio setter with the unchanged 3:2 UI value. It sends
   capture property 0xd4 again and posts no UI change event. */
#define FRAMING_REQUEST_ASPECT() (((u32 (*)(u32,u32))FW_STILL_ASPECT_SET)(FRAMING_SETTINGS(),0))
#endif
#endif
#ifndef FRAMING_INVALIDATE
/* View::Invalidate: request a redraw unless one is pending (+0xa0). */
#define FRAMING_INVALIDATE(view) (((void (*)(u32))FW_VIEW_INVALIDATE)(view))
#endif
extern volatile u32 *framing_header(void);

/* Original 5x7 column glyphs, bit zero at the top. Each pixel is 2x2 LCD pixels. */
static void ratio_badge(u32 data,u32 stride,u32 ratio) {
    static const u8 glyphs[][5]={
        {0x00,0x42,0x7f,0x40,0x00}, /* 1 */
        {0x62,0x51,0x49,0x49,0x46}, /* 2 */
        {0x18,0x14,0x12,0x7f,0x10}, /* 4 */
        {0x27,0x45,0x45,0x45,0x39}, /* 5 */
        {0x3c,0x4a,0x49,0x49,0x30}, /* 6 */
        {0x06,0x49,0x49,0x29,0x1e}, /* 9 */
        {0x00,0x36,0x36,0x00,0x00}, /* : */
        {0x22,0x41,0x49,0x49,0x36}, /* 3 */
        {0x63,0x14,0x08,0x14,0x63}, /* X */
        {0x7f,0x09,0x09,0x09,0x06}, /* P */
        {0x20,0x54,0x54,0x54,0x78}, /* a */
        {0x7c,0x08,0x04,0x04,0x78}  /* n */
    };
    /* XPan, 16:9, 4:3 */
    static const u8 labels[][5]={{8,9,10,11,255},{0,4,6,5,255},{2,6,7,255,255}};
    u32 x,y,i,dy;
    volatile u32 *row;
    for(y=46;y<68;++y) {
        row=FRAMING_PIXELS(data+y*stride*4);
        for(x=204;x<270;++x) row[x]=0x80000000;
    }
    for(i=0;labels[ratio-1][i]!=255;++i)
        for(x=0;x<5;++x) for(y=0;y<7;++y)
            if(glyphs[labels[ratio-1][i]][x]&(1U<<y))
                for(dy=0;dy<2;++dy) {
                    row=FRAMING_PIXELS(data+(50+y*2+dy)*stride*4)+208+i*12+x*2;
                    row[0]=row[1]=0xffffffff;
                }
}

#define GUIDE_LAST 4U
/* Committed ratio and guide style, with unknown values treated as Off. */
static u32 frame_ratio_value(volatile u32 *h) {
    u32 ratio=h[GRKIT_HOST_FRAME_RATIO/4];
    return ratio<FRAMING_RATIO_COUNT ? ratio : 0;
}
#ifdef FRAMING_XPAN_NATIVE
/* XPan crops the native aspect FRAMING_XPAN_NATIVE, and the other ratios are
   native. Set `ratio` to the photo area that the live view shows: XPan in that
   aspect, or the full area in native 3:2. Otherwise nothing shows: clear `data`. */
#define SHOWN_RATIO(data,ratio) do { \
    u32 native=FRAMING_NATIVE_ASPECT(); \
    if(native==0) (ratio)=0; \
    else if(native!=FRAMING_XPAN_NATIVE || (ratio)!=1) (data)=0; \
} while(0)
#else
/* canvas_pixels() accepts only the native 3:2 aspect, where each ratio shows. */
#define SHOWN_RATIO(data,ratio) ((void)0)
#endif
static u32 guide_value(volatile u32 *h) {
    u32 style=h[GRKIT_HOST_GUIDE/4];
    return style<=GUIDE_LAST ? style : 0;
}
#ifdef FW_CAPTURE_PROPERTIES
/* Capture aspect ids: 0 = 3:2, 1 = 4:3, 2 = 16:9 and 65:24. GR III hides
   ids 1 and 2 from its menu, but its capture and playback code support them. */
static u32 capture_id(volatile u32 *h) {
    u32 ratio=h[GRKIT_HOST_FRAME_RATIO/4];
    return ratio==3 ? 1 : ratio==1 || ratio==2 ? 2 : 0;
}
/* The still aspect property handler calls this for each requested value. The
   native menu requests 0 for 3:2, so 3:2 records the Framing ratio. Other
   native ratios pass unchanged. */
u32 framing_capture_request(u32 request) {
    return request ? request : capture_id(framing_header());
}
/* The live view keeps 3:2. Resend the native 3:2 request when the capture
   aspect differs from the Framing ratio, for example after a menu change. */
static void capture_sync(u32 controller) {
    if(BYTE(controller,LIVEVIEW_MOVIE)==0 && FRAMING_NATIVE_ASPECT()==0 &&
       FRAMING_CAPTURE_ASPECT()!=capture_id(framing_header()))
        FRAMING_REQUEST_ASPECT();
}
#else
/* Without the capture hooks a Framing ratio cannot be selected. */
#define capture_sync(controller) ((void)(controller))
#endif
/* Only the native still-photo LCD canvas owns these transient references. */
void framing_bind(u32 controller) {
    volatile u32 *h=framing_header();
    h[GRKIT_HOST_FRAME_OWNER/4]=controller;
    h[GRKIT_HOST_FRAME_CANVAS/4]=WORD(controller,LIVEVIEW_CANVAS);
    /* Shooting: id 2 geometry follows the Framing ratio again. */
    ((volatile u8 *)h)[GRKIT_HOST_FRAME_FILE]=0;
    capture_sync(controller);
    /* A selection that has not drawn on this canvas yet, such as one restored
       at power-on, needs a redraw: nothing else invalidates a still live view. */
    if(h[GRKIT_HOST_FRAME_CANVAS/4] && !h[GRKIT_HOST_FRAME_BITMAP/4] &&
       (frame_ratio_value(h) || guide_value(h)))
        FRAMING_INVALIDATE(h[GRKIT_HOST_FRAME_CANVAS/4]);
}
void framing_leave(u32 controller) {
    volatile u32 *h=framing_header();
    if(h[GRKIT_HOST_FRAME_OWNER/4]==controller) {
        h[GRKIT_HOST_FRAME_OWNER/4]=0;
        h[GRKIT_HOST_FRAME_CANVAS/4]=0;
    }
}
/* Pixel data of a supported canvas bitmap, or 0. The mask is defined only
   for a full 720x480 32-bit canvas at the origin and the native 3:2 aspect. */
static u32 canvas_pixels(u32 canvas,u32 bitmap) {
    u32 stride,data;
    if(!BYTE(canvas,0x9e) || WORD(canvas,0x20) || WORD(canvas,0x24) ||
       WORD(canvas,0x28)!=720 || WORD(canvas,0x2c)!=480 ||
       BYTE(bitmap,2)!=5 || HALF(bitmap,0x1c)!=720 || HALF(bitmap,0x1e)!=480) return 0;
#ifndef FRAMING_XPAN_NATIVE
    /* Native StillAspectRatio getter: 0 = 3:2, 3 = 1:1. Native square crop is untouched. */
    if(FRAMING_NATIVE_ASPECT()!=0) return 0;
#endif
    stride=HALF(bitmap,0x18);
    data=WORD(bitmap,8);
    /* The OSD buffer moves between boots (0x5f83df80 and 0x619e1200 seen). */
    if(stride<720 || stride>1024 || HALF(bitmap,0x1a)!=480 ||
       data<0x40000000 || data>0x80000000-stride*480*4 || (data&3) ||
       WORD(bitmap,4)<stride*480*4) return 0;
    return data;
}
/* Photo area {left,top,right,bottom} on the 720x480 canvas for each ratio. */
static const u16 photo_boxes[4][4]={{0,0,720,480},{0,107,720,373},{0,37,720,442},{40,0,680,480}};
/* Black at 50% alpha outside the photo area. */
static void ratio_mask(u32 data,u32 stride,u32 ratio) {
    const u16 *box=photo_boxes[ratio];
    u32 x,y;
    volatile u32 *row;
    for(y=0;y<480;++y) {
        row=FRAMING_PIXELS(data+y*stride*4);
        for(x=0;x<720;++x)
            if(y<box[1] || y>=box[3] || x<box[0] || x>=box[2]) row[x]=0x80000000;
    }
}
/* Opaque gray is the same pixel with straight or premultiplied alpha. */
#define GUIDE_COLOR 0xffc0c0c0U
#define GUIDE_ARM 20U
static void guide_pixel(u32 data,u32 stride,u32 x,u32 y) { FRAMING_PIXELS(data+y*stride*4)[x]=GUIDE_COLOR; }
/* A position at `fraction`/65536 of the span, rounded. The link has no division helpers. */
static u32 guide_at(u32 start,u32 end,u32 fraction) { return start+(((end-start)*fraction+0x8000)>>16); }
/* One vertical and one horizontal line at each fraction of the photo area. */
static void guide_grid(u32 data,u32 stride,const u16 *box,u32 fraction) {
    u32 i,j,f,x,y;
    for(i=0;i<2;++i) {
        f=i ? 65536-fraction : fraction;
        x=guide_at(box[0],box[2],f);
        y=guide_at(box[1],box[3],f);
        for(j=box[1];j<box[3];++j) guide_pixel(data,stride,x,j);
        for(j=box[0];j<box[2];++j) guide_pixel(data,stride,j,y);
    }
}
/* Corner-to-corner Bresenham lines. Every photo area is wider than tall, so
   each column has one pixel of each diagonal, rounded half up. */
static void guide_diagonals(u32 data,u32 stride,const u16 *box) {
    int run=box[2]-box[0]-1, rise=box[3]-box[1]-1, error=0;
    u32 x, y=box[1];
    for(x=0;x<=(u32)run;++x) {
        guide_pixel(data,stride,box[0]+x,y);
        guide_pixel(data,stride,box[2]-1-x,y);
        error+=rise;
        if(2*error>=run) { ++y; error-=run; }
    }
}
/* Guides stay inside the photo area of the Framing ratio. Grid fractions are
   in 1/65536: 1/3 for thirds and 0.381966 for the golden section. */
static void guide_lines(u32 data,u32 stride,u32 ratio,u32 style) {
    const u16 *box=photo_boxes[ratio];
    u32 x,y,i;
    if(style==1) guide_grid(data,stride,box,21845);
    if(style==3) guide_grid(data,stride,box,25033);
    if(style==4) guide_diagonals(data,stride,box);
    if(style==2) {
        x=guide_at(box[0],box[2],32768);
        y=guide_at(box[1],box[3],32768);
        for(i=0;i<=2*GUIDE_ARM;++i) {
            guide_pixel(data,stride,x-GUIDE_ARM+i,y);
            guide_pixel(data,stride,x,y-GUIDE_ARM+i);
        }
    }
}
void framing_prepare(u32 canvas) {
    volatile u32 *h=framing_header();
    /* Canvas::Draw clears the full native bitmap before drawing its children.
       Use this path even for Off to remove the previous drawing on the next draw. */
    if((h[GRKIT_HOST_FRAME_CANVAS/4]==canvas &&
        (frame_ratio_value(h) || guide_value(h) || h[GRKIT_HOST_FRAME_BITMAP/4])) ||
       (canvas && h[GRKIT_HOST_FRAME_PREVIEW_CANVAS/4]==canvas))
        BYTE(canvas,0xa0)=1;
}
/* The ADJ Framing and Guides pages preview their highlighted row on the ADJ
   canvas, below the menu controls. After the page ends, one more full draw
   erases the drawing and releases the canvas. */
static void preview_paint(volatile u32 *h,u32 canvas,u32 bitmap) {
    u32 preview=h[GRKIT_HOST_FRAME_PREVIEW/4],style=h[GRKIT_HOST_GUIDE_PREVIEW/4],data;
    if(!preview) { h[GRKIT_HOST_FRAME_PREVIEW_CANVAS/4]=0; return; }
    if(preview>4 || style>GUIDE_LAST || (preview<2 && !style)) return;
    data=canvas_pixels(canvas,bitmap);
#ifdef FRAMING_XPAN_NATIVE
    --preview;
    SHOWN_RATIO(data,preview);
    ++preview;
#endif
    if(!data) return;
    if(preview>=2) ratio_mask(data,HALF(bitmap,0x18),preview-1);
    guide_lines(data,HALF(bitmap,0x18),preview-1,style);
}
void framing_paint(u32 canvas,u32 bitmap) {
    volatile u32 *h=framing_header();
    u32 ratio=frame_ratio_value(h),style=guide_value(h);
    u32 owner=h[GRKIT_HOST_FRAME_OWNER/4],data,stride;
    if(canvas && canvas==h[GRKIT_HOST_FRAME_PREVIEW_CANVAS/4]) { preview_paint(h,canvas,bitmap); return; }
    if(canvas!=h[GRKIT_HOST_FRAME_CANVAS/4] || !owner) return;
    if(!ratio && !style) { h[GRKIT_HOST_FRAME_BITMAP/4]=0; return; }
    h[GRKIT_HOST_FRAME_BITMAP/4]=bitmap;
    /* The LCD still canvas is registered in native slot zero. Unsupported
       canvas sizes and bitmap formats leave the native image untouched. */
    data=WORD(owner,4)==canvas && BYTE(owner,LIVEVIEW_MOVIE)==0 ? canvas_pixels(canvas,bitmap) : 0;
    SHOWN_RATIO(data,ratio);
    if(!data) { ++h[GRKIT_HOST_FRAME_SKIPPED/4]; return; }
    stride=HALF(bitmap,0x18);
    if(ratio) ratio_mask(data,stride,ratio);
    guide_lines(data,stride,ratio,style);
    /* The badge covers the guides that cross it. */
    if(ratio) ratio_badge(data,stride,ratio);
    ++h[GRKIT_HOST_FRAME_DRAWS/4];
}
