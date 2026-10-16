#include "abi.h"

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
#define FRAMING_SETTINGS() (((u32 (*)(void))0x5320b31c)())
#ifndef FRAMING_NATIVE_ASPECT
#define FRAMING_NATIVE_ASPECT() (((u32 (*)(u32))0x53245260)(FRAMING_SETTINGS()))
#endif
#ifndef FRAMING_CAPTURE_ASPECT
/* Requested capture StillAspectRatio (CommonProperty +0x7b6). The property
   setter writes the next copy at [property+4]; readers see the current copy
   at [property] only after a later commit. Comparing the next copy sends
   each change once. */
#define FRAMING_CAPTURE_ASPECT() BYTE(WORD(WORD(WORD(((u32 (*)(void))0x53525d5c)(),0),0x518),4),0x7b6)
#endif
#ifndef FRAMING_REQUEST_ASPECT
/* Native StillAspectRatio setter with the unchanged 3:2 UI value. It sends
   capture property 0xd4 again and posts no UI change event. */
#define FRAMING_REQUEST_ASPECT() (((u32 (*)(u32,u32))0x532525a4)(FRAMING_SETTINGS(),0))
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
    if(BYTE(controller,0x2d2)==0 && FRAMING_NATIVE_ASPECT()==0 &&
       FRAMING_CAPTURE_ASPECT()!=capture_id(framing_header()))
        FRAMING_REQUEST_ASPECT();
}
/* Only the native still-photo LCD canvas owns these transient references. */
void framing_bind(u32 controller) {
    volatile u32 *h=framing_header();
    h[GRKIT_HOST_FRAME_OWNER/4]=controller;
    h[GRKIT_HOST_FRAME_CANVAS/4]=WORD(controller,0x15c);
    /* Shooting: id 2 geometry follows the Framing ratio again. */
    ((volatile u8 *)h)[GRKIT_HOST_FRAME_FILE]=0;
    capture_sync(controller);
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
    /* Native StillAspectRatio getter: 0 = 3:2, 3 = 1:1. Native square crop is untouched. */
    if(FRAMING_NATIVE_ASPECT()!=0) return 0;
    stride=HALF(bitmap,0x18);
    data=WORD(bitmap,8);
    if(stride<720 || stride>1024 || HALF(bitmap,0x1a)!=480 ||
       data<0x60000000 || data>0x6fe00000 || (data&3) ||
       WORD(bitmap,4)<stride*480*4) return 0;
    return data;
}
/* Black at 50% alpha outside the photo area. */
static void ratio_mask(u32 data,u32 stride,u32 ratio) {
    u32 x,y,top=0,left=0,bottom=480,right=720;
    volatile u32 *row;
    if(ratio==1) { top=107; bottom=373; }
    if(ratio==2) { top=37; bottom=442; }
    if(ratio==3) { left=40; right=680; }
    for(y=0;y<480;++y) {
        row=FRAMING_PIXELS(data+y*stride*4);
        for(x=0;x<720;++x)
            if(y<top || y>=bottom || x<left || x>=right) row[x]=0x80000000;
    }
}
void framing_prepare(u32 canvas) {
    volatile u32 *h=framing_header();
    u32 ratio=h[GRKIT_HOST_FRAME_RATIO/4];
    /* Canvas::Draw clears the full native bitmap before drawing its children.
       Use this path even for Off to remove the previous mask on the next draw. */
    if((h[GRKIT_HOST_FRAME_CANVAS/4]==canvas &&
        ((ratio>=1 && ratio<=3) || h[GRKIT_HOST_FRAME_BITMAP/4])) ||
       (canvas && h[GRKIT_HOST_FRAME_PREVIEW_CANVAS/4]==canvas))
        BYTE(canvas,0xa0)=1;
}
/* The ADJ Framing page previews its highlighted ratio on the ADJ canvas,
   below the menu controls. After the page ends, one more full draw erases
   the mask and releases the canvas. */
static void preview_paint(volatile u32 *h,u32 canvas,u32 bitmap) {
    u32 preview=h[GRKIT_HOST_FRAME_PREVIEW/4],data;
    if(!preview) { h[GRKIT_HOST_FRAME_PREVIEW_CANVAS/4]=0; return; }
    if(preview<2 || preview>4) return;
    data=canvas_pixels(canvas,bitmap);
    if(data) ratio_mask(data,HALF(bitmap,0x18),preview-1);
}
void framing_paint(u32 canvas,u32 bitmap) {
    volatile u32 *h=framing_header();
    u32 ratio=h[GRKIT_HOST_FRAME_RATIO/4];
    u32 owner=h[GRKIT_HOST_FRAME_OWNER/4],data;
    if(canvas && canvas==h[GRKIT_HOST_FRAME_PREVIEW_CANVAS/4]) { preview_paint(h,canvas,bitmap); return; }
    if(canvas!=h[GRKIT_HOST_FRAME_CANVAS/4] || !owner) return;
    if(!ratio || ratio>3) { h[GRKIT_HOST_FRAME_BITMAP/4]=0; return; }
    h[GRKIT_HOST_FRAME_BITMAP/4]=bitmap;
    /* The LCD still canvas is registered in native slot zero. Unsupported
       canvas sizes and bitmap formats leave the native image untouched. */
    data=WORD(owner,4)==canvas && BYTE(owner,0x2d2)==0 ? canvas_pixels(canvas,bitmap) : 0;
    if(!data) { ++h[GRKIT_HOST_FRAME_SKIPPED/4]; return; }
    ratio_mask(data,HALF(bitmap,0x18),ratio);
    ratio_badge(data,HALF(bitmap,0x18),ratio);
    ++h[GRKIT_HOST_FRAME_DRAWS/4];
}
