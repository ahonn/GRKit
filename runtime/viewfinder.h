#ifndef GRKIT_VIEWFINDER_H
#define GRKIT_VIEWFINDER_H
/* The still-viewfinder service owns the canvas lifecycle, legacy layers,
   the status row and the transient load cue. Capture-aspect changes stay
   with framing.c. A module never owns a native canvas reference. */
#include "viewfinder_indicators.h"
#include "recipe_detail_names.h"
#ifndef LABEL_MAP
#define LABEL_MAP(address) ((const volatile u8 *)(address))
#endif
/* A u16 or u32 field of the label table (runtime/abi.h). */
static GRKIT_ICON_COMPACT u32 label_field(const volatile u8 *table,u32 at,u32 bytes) {
    u32 value=0,i;
    for(i=0;i<bytes;++i) value|=(u32)table[at+i]<<(8*i);
    return value;
}
static GRKIT_ICON_COMPACT u32 label_width(u32 label) {
    return label<GRKIT_LABEL_COUNT ? label_field(LABEL_MAP(GRKIT_LABELS(framing_header())),label*8,2) : 0;
}
/* The entries of the label table: the labels, then the glyphs of printable
   ASCII from GRKIT_GLYPH_FIRST, the small glyphs of the same characters
   from GRKIT_SMALL_GLYPH_FIRST, and the caption glyphs of recipe tiles
   (grkit_core::artwork::overlay_labels). */
#define LABEL_ENTRIES (GRKIT_CAPTION_FIRST+(u32)sizeof(GRKIT_CAPTION_CHARACTERS)-1U)
_Static_assert(GRKIT_GLYPH_FIRST==GRKIT_LABEL_COUNT,"the glyphs follow the labels");
/* White text with a black outline and no background, as the native live-view
   text has it: each drawn pixel is opaque, and text edges blend toward the
   outline in RGB. A transparent pixel keeps the canvas, so a label does not
   cut the Framing mask. Canvas::Draw clears the bitmap before each draw
   (framing_prepare), which removes the previous label. */
static GRKIT_ICON_COMPACT void overlay_entry_shaded(const Surface *s,u32 label,u32 left,u32 top,u32 shade) {
    const volatile u8 *table=LABEL_MAP(GRKIT_LABELS(framing_header())),*pixels;
    u32 width,height,x,y,value;
    volatile u32 *row;
    if(label>=LABEL_ENTRIES) return;
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
                for(x=0;x<length;++x) { value=at[x]*shade>>8; to[x]=0xff000000U|value*0x010101U; }
                at+=length;
            }
        }
    }
}
/* White text, or gray text with `shade` below 256. */
static GRKIT_ICON_COMPACT void overlay_entry(const Surface *s,u32 label,u32 left,u32 top) {
    overlay_entry_shaded(s,label,left,top,256U);
}
static GRKIT_ICON_COMPACT void overlay_label(const Surface *s,u32 label,u32 left,u32 top) {
    if(label<GRKIT_LABEL_COUNT) overlay_entry(s,label,left,top);
}
/* The glyph of character c: a character outside printable ASCII shows as a
   question mark. */
static GRKIT_ICON_COMPACT u32 glyph_of(u32 c) {
    return GRKIT_GLYPH_FIRST+(c>=' ' && c<=' '+GRKIT_GLYPH_COUNT-1U ? c-' ' : '?'-' ');
}
/* The outline of a glyph is one pixel on each side, and the next glyph
   starts on the outline of the last (grkit_core::artwork::overlay_glyph). */
#define GLYPH_OUTLINES 2U
static GRKIT_ICON_COMPACT u32 glyph_advance(u32 c) {
    u32 width=label_field(LABEL_MAP(GRKIT_LABELS(framing_header())),glyph_of(c)*8,2);
    return width>GLYPH_OUTLINES ? width-GLYPH_OUTLINES : 0;
}
/* The width of a NUL-terminated text in glyphs, with the outlines at both
   ends. */
static GRKIT_ICON_COMPACT u32 overlay_text_width(const volatile u16 *text,u32 units) {
    u32 width=0,i;
    for(i=0;i<units && text[i];++i) width+=glyph_advance(text[i]);
    return width ? width+GLYPH_OUTLINES : 0;
}
/* Write a text in glyphs, its top row at `top`, until its NUL or a glyph
   that would pass `right`. */
static GRKIT_ICON_COMPACT void overlay_text(const Surface *s,const volatile u16 *text,u32 units,u32 left,u32 top,u32 right) {
    u32 i,advance;
    for(i=0;i<units && text[i];++i) {
        advance=glyph_advance(text[i]);
        if(left+advance+GLYPH_OUTLINES>right) return;
        overlay_entry(s,glyph_of(text[i]),left,top);
        left+=advance;
    }
}

/* Write ASCII text in the small glyphs, in gray `shade`, until its NUL or
   a glyph that would pass `right`. */
static GRKIT_ICON_COMPACT void small_text(const Surface *s,const char *text,u32 left,u32 top,u32 right,u32 shade) {
    const volatile u8 *table=LABEL_MAP(GRKIT_LABELS(framing_header()));
    u32 c,entry,width;
    for(;*text;++text) {
        c=(u8)*text;
        entry=GRKIT_SMALL_GLYPH_FIRST+(c>=' ' && c<=' '+GRKIT_GLYPH_COUNT-1U ? c-' ' : '?'-' ');
        width=label_field(table,entry*8,2);
        if(width<=GLYPH_OUTLINES || left+width>right) return;
        overlay_entry_shaded(s,entry,left,top,shade);
        left+=width-GLYPH_OUTLINES;
    }
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
static GRKIT_ICON_COMPACT u32 framing_second(void) {
    u32 date[3];
    const u8 *dt=(const u8 *)date;
    ((void (*)(u32,u32))FW_LOCAL_TIME)(WORD(((u32 (*)(void))FW_CLOCK)(),4),(u32)date);
    return dt[4]*3600U+dt[5]*60U+dt[6];
}
#define FRAMING_SECOND() framing_second()
#endif
static GRKIT_ICON_COMPACT u32 cue_shown(volatile u32 *h) { return h[GRKIT_HOST_LOAD_CUE/4]>CUE_PENDING; }
/* Start the cue in a still live view, and end it after CUE_SECONDS. Returns 1
   when the cue starts or ends, so the canvas needs a redraw. */
static GRKIT_ICON_COMPACT u32 cue_update(volatile u32 *h,u32 controller) {
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
   for a count out of range. Each has four words: its paint function, setting
   handle, flags with order in the high half, and optional conditions descriptor. */
static GRKIT_ICON_COMPACT u32 module_layers(volatile u32 *h) {
    u32 n=h[GRKIT_HOST_MODULE_LAYER_COUNT/4];
    return n<=GRKIT_MODULE_LAYERS_MAX ? n : 0;
}
#define MODULE_LAYER(h,i) ((h)+GRKIT_HOST_MODULE_LAYERS/4+(i)*4)
/* The flags of a layer (runtime/sdk/grkit.h). */
#define MODULE_LAYER_FOLLOW_DISP 1U
#define MODULE_LAYER_FULL_DISP 2U
#define MODULE_LAYER_LIVE_ONLY 4U
#define NO_SETTING 0xffffffffU
/* The value of module layer i in the live view: its setting, or 1 without
   one. A layer is off while DISP hides what its flags need, and while a soft
   Off stops its module. */
static GRKIT_ICON_COMPACT u32 module_layer_value(volatile u32 *h,u32 i) {
    volatile u32 *l=MODULE_LAYER(h,i);
    if(services_suspended(h,l[0]) || !viewfinder_layer_enabled(h,l[3],0)) return 0;
    if((l[2]&MODULE_LAYER_FOLLOW_DISP) && native_display()>DISPLAY_SIMPLE) return 0;
    if((l[2]&MODULE_LAYER_FULL_DISP) && native_display()!=DISPLAY_FULL) return 0;
    return l[1]==NO_SETTING ? 1 : setting_value(h,l[1]);
}
static GRKIT_ICON_COMPACT u32 layers_active(volatile u32 *h) {
    u32 i;
    for(i=0;i<module_layers(h);++i) if(module_layer_value(h,i)) return 1;
    return 0;
}
/* Paint module layer i with `value`, which is not 0. A host test maps the
   function address. */
#ifndef MODULE_PAINT
#define MODULE_PAINT(address,surface,value) ((void (*)(const Surface *,u32))(address))(surface,value)
#endif
static GRKIT_ICON_COMPACT void module_paint(volatile u32 *h,u32 i,const Surface *surface,u32 value) {
    u32 start=profile_begin(h);
    MODULE_PAINT(MODULE_LAYER(h,i)[0],surface,value);
    profile_end_module(h,start,MODULE_LAYER(h,i)[0],GRKIT_PROFILE_PAINT);
}
/* Photo area {left,top,right,bottom} on the 720x480 canvas: native 3:2,
   XPan, 16:9, 4:3, 2.35:1, 1:1, 5:4, and 7:6. Both capture and composition
   layers use this geometry, but they can select different areas. */
static const u16 photo_boxes[8][4]={{0,0,720,480},{0,107,720,373},{0,37,720,442},{40,0,680,480},
                                    {0,87,720,393},{120,0,600,480},{60,0,660,480},{80,0,640,480}};
/* The native aspect IDs are 3:2, 4:3, 16:9, and 1:1. The caller rejects
   unknown native aspects before drawing layers. */
static GRKIT_ICON_COMPACT u32 native_area(void) {
    static const u8 areas[]={0,3,2,5};
    u32 native=FRAMING_NATIVE_ASPECT();
    return native<sizeof(areas) ? areas[native] : 0;
}
/* A composition ratio applies to native 3:2. The existing capture modes
   can also apply their wide crops to native 16:9 on GR IV. */
static GRKIT_ICON_COMPACT u32 composition_area(volatile u32 *h,u32 ratio) {
    u32 native=FRAMING_NATIVE_ASPECT();
    if(!framing_row(ratio)) return 0;
    return !native || (!framing_guides_only(h) && FRAMING_CROP_KEEPS(native,ratio)) ? ratio : 0;
}
/* General overlays use the framing area: the crop, or the composition frame
   in Guides Only mode, so that guides divide the frame that the photographer
   composes in. Without one, they use the native area, such as 4:3 or 1:1. */
static GRKIT_ICON_COMPACT u32 layer_area(u32 framing) {
    return framing ? framing : native_area();
}
/* The CORE.CROP value that the layers paint: the highlighted value in the
   ADJ preview of its page, else the selection. A crop that the camera
   applies to the live view already has no frame (runtime/crop.h). */
static GRKIT_ICON_COMPACT u32 crop_painted(volatile u32 *h,u32 preview) {
    return preview ? crop_shown(h) : crop_selection(h);
}
/* Paint the module layers in their order. In the ADJ preview, the layers of
   the previewed setting paint its highlighted value, DISP does not hide
   layers, and a live-only layer does not paint. A layer bound to CORE.ASPECT
   receives the framing area, or 0 without one. A crop (CORE.CROP) frames a
   part of the layer area: a layer bound to CORE.CROP receives that frame
   with the crop as its area, or area 0 without one, and the other layers
   receive the frame instead of the layer area. */
static GRKIT_ICON_COMPACT void module_layers_paint(volatile u32 *h,const Surface *surface,u32 preview,u32 composition) {
    u32 i, value, crop=crop_painted(h,preview), framed;
    u16 crop_frame[4];
    Surface frame=*surface, focal=*surface, cropped=*surface;
    frame.area=composition; frame.box=photo_boxes[composition];
    framed=crop && crop_box(surface->box,crop_scale(crop,CROP_VIEW_Q16(h)),crop_frame);
    if(framed) { focal.box=crop_frame; cropped.box=crop_frame; }
    cropped.area=framed ? crop : 0;
    for(i=0;i<module_layers(h);++i) {
        u32 aspect=MODULE_LAYER(h,i)[1]==GRKIT_CORE_ASPECT, crop_layer=MODULE_LAYER(h,i)[1]==GRKIT_CORE_CROP;
        if(!preview) value=module_layer_value(h,i);
        else if(MODULE_LAYER(h,i)[2]&MODULE_LAYER_LIVE_ONLY) value=0;
        else if(h[GRKIT_HOST_MODULE_PREVIEW/4] && h[GRKIT_HOST_MODULE_PREVIEW/4]-1==MODULE_LAYER(h,i)[1]) {
            value=h[GRKIT_HOST_MODULE_PREVIEW/4+1];
            if(value>=setting_limit(h,MODULE_LAYER(h,i)[1])) value=0;
        }
        else value=MODULE_LAYER(h,i)[1]==NO_SETTING ? 1 : setting_value(h,MODULE_LAYER(h,i)[1]);
        if(aspect && !composition) value=0;
        if(crop_layer && !framed) value=0;
        if(preview && !viewfinder_layer_enabled(h,MODULE_LAYER(h,i)[3],1)) value=0;
        if(value && !services_suspended(h,MODULE_LAYER(h,i)[0]))
            module_paint(h,i,aspect ? &frame : crop_layer ? &cropped : framed ? &focal : surface,value);
    }
}
/* Status is screen information, independent of the capture area's geometry. */
static GRKIT_ICON_COMPACT u32 status_shown(volatile u32 *h) {
    u32 owner=h[GRKIT_HOST_FRAME_OWNER/4];
    return owner && !BYTE(owner,LIVEVIEW_MOVIE) && native_display()==DISPLAY_FULL && viewfinder_status_active(h);
}
/* The crop badge that the live view shows (runtime/crop.h), or
   GRKIT_LABEL_COUNT: only with the native information, as the camera's own. */
static GRKIT_ICON_COMPACT u32 crop_badge_shown(volatile u32 *h) {
    u32 owner=h[GRKIT_HOST_FRAME_OWNER/4];
    if(!owner || BYTE(owner,LIVEVIEW_MOVIE) || native_display()!=DISPLAY_FULL) return GRKIT_LABEL_COUNT;
    return crop_badge_label(h,FRAMING_NATIVE_ASPECT());
}
/* Whether the live view has something to draw: layers, status, the crop
   badge, or the cue. */
static GRKIT_ICON_COMPACT u32 overlay_active(volatile u32 *h) {
    return cue_shown(h) || layers_active(h) || status_shown(h) || crop_badge_shown(h)<GRKIT_LABEL_COUNT;
}
static GRKIT_ICON_COMPACT void viewfinder_bind(volatile u32 *h,u32 controller) {
    u32 native=FRAMING_NATIVE_ASPECT()+1, native_changed=h[GRKIT_HOST_FRAME_NATIVE/4]!=native;
    h[GRKIT_HOST_FRAME_NATIVE/4]=native;
    h[GRKIT_HOST_FRAME_OWNER/4]=controller;
    h[GRKIT_HOST_FRAME_CANVAS/4]=WORD(controller,LIVEVIEW_CANVAS);
    /* Nothing else invalidates a still live view, so these need a redraw: a
       selection that has not drawn on this canvas yet, such as one restored
       at power-on, a cue that starts or ends, and a setting that changed
       since the last redraw, such as a ratio or a mask style from ADJ or a
       native settings page. A crop does not restart the live view. */
    if(h[GRKIT_HOST_FRAME_CANVAS/4]) {
        u32 changes=h[GRKIT_HOST_STATE_CHANGES/4], changed=h[GRKIT_HOST_FRAME_CHANGES/4]!=changes;
        h[GRKIT_HOST_FRAME_CHANGES/4]=changes;
        if(cue_update(h,controller) || changed || (native_changed &&
           (h[GRKIT_HOST_FRAME_BITMAP/4] || overlay_active(h))) ||
           (!h[GRKIT_HOST_FRAME_BITMAP/4] && overlay_active(h)))
            FRAMING_INVALIDATE(h[GRKIT_HOST_FRAME_CANVAS/4]);
    }
    /* The first shooting screen after INSTALL: the modules are ready. This
       hook runs in the App task. */
    if(h[GRKIT_HOST_FRAME_CANVAS/4]) services_ready(h);
}
static GRKIT_ICON_COMPACT void viewfinder_leave(volatile u32 *h,u32 controller) {
    if(h[GRKIT_HOST_FRAME_OWNER/4]!=controller) return;
    h[GRKIT_HOST_FRAME_OWNER/4]=0;
    h[GRKIT_HOST_FRAME_CANVAS/4]=0;
    h[GRKIT_HOST_FRAME_BITMAP/4]=0;
}
/* The native views of a canvas, the same on each target: the vector of
   child views (begin, end), the rectangle in screen pixels (x, y, width,
   height), and the visible flag. A hidden view hides its children, which
   keep their own flags and rectangles. */
#define VIEW_CHILDREN 0xc
#define VIEW_SCREEN 0x44
#define VIEW_VISIBLE 0x9e
/* Pixel data of a full 720x480 32-bit canvas at the origin, or 0. */
static GRKIT_ICON_COMPACT u32 canvas_bitmap(u32 canvas,u32 bitmap) {
    u32 stride,data;
    if(!BYTE(canvas,VIEW_VISIBLE) || WORD(canvas,0x20) || WORD(canvas,0x24) ||
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
/* Layers use a full canvas with a known native photo area. */
static GRKIT_ICON_COMPACT u32 canvas_pixels(u32 canvas,u32 bitmap) {
    if(FRAMING_NATIVE_ASPECT()>3) return 0;
    return canvas_bitmap(canvas,bitmap);
}
/* The status row is on the second row of the native information, at most
   from STATUS_LEFT to STATUS_RIGHT. Right of STATUS_RIGHT is the native focus
   distance. Native views of other states show in the same row, such as the
   white balance shift under its icon or the dial guide of Snap mode. The row
   takes the rightmost free part of that range that holds it: STATUS_GAP from
   each visible native view (status_free). The order of the indicators comes
   from module order, then declaration order; inactive entries leave no gap.
   When no free part holds all of them, the row takes the widest one, and the
   indicators that do not fit there are not shown, nor are those after them.
   The row waits while the load cue shows in the middle of the same row; the
   end of the cue redraws the canvas. */
#define STATUS_LEFT 7
#define STATUS_RIGHT 580
#define STATUS_GAP 20
/* The most native views that block the row, and the depth of the view tree
   that status_views reads. The live view has about 150 views, 3 levels deep,
   and at most 5 of them in the row. */
#define STATUS_BLOCKS 16U
#define STATUS_DEPTH 4U
#define STATUS_CHILDREN 64U
/* A one-line label stays at LABEL_TOP, so its capitals share the baseline of
   the native focus distance next to the row. The marks of an icon and of a
   two-line label fill rows 2 to 37 of their 40 rows; from STATUS_TOP they
   center on the one-line text. */
#define STATUS_TOP 40U
#define STATUS_LINE_HEIGHT 32U
#define STATUS_STEP 44U
/* A core label takes its own width and this gap. The marks of an icon reach
   one pixel from its tile edge, so the space after a label looks close to
   the space between two icons. */
#define STATUS_LABEL_GAP 10U
/* The columns that the visible native views of `view` cover in the row,
   from STATUS_GAP before to STATUS_GAP after each, into block[*n]: a pair of
   left and right columns. Only views without children count: a parent spans
   its children, and some parents span the full screen. Only views inside
   the rows of the row count: a view that crosses them, such as the focus
   frame after a touch at the top or a frame line, moves with the subject,
   and the row stays where it is. A view as wide as the screen draws no
   information there either. The native Draw runs after the status row, on
   the same views, in the same task. */
static GRKIT_ICON_COMPACT void status_views(u32 view,u32 depth,int (*block)[2],u32 *n) {
    u32 at=WORD(view,VIEW_CHILDREN),end=WORD(view,VIEW_CHILDREN+4),child;
    int x,y,w,h;
    if(!BYTE(view,VIEW_VISIBLE)) return;
    if(at!=end) {
        if(depth>=STATUS_DEPTH || end<at || end-at>STATUS_CHILDREN*4 || (at&3)) return;
        for(;at!=end;at+=4) {
            child=WORD(at,0);
            if(child && !(child&3)) status_views(child,depth+1,block,n);
        }
        return;
    }
    x=(int)WORD(view,VIEW_SCREEN); y=(int)WORD(view,VIEW_SCREEN+4);
    w=(int)WORD(view,VIEW_SCREEN+8); h=(int)WORD(view,VIEW_SCREEN+12);
    if(w<=0 || h<=0 || w>=720 || y<(int)STATUS_TOP || y+h>(int)STATUS_TOP+40 || *n>=STATUS_BLOCKS) return;
    block[*n][0]=x-STATUS_GAP; block[*n][1]=x+w+STATUS_GAP; ++*n;
}
/* The free part of the row that ends at `right`: its left column, or `right`
   when `right` lies in a block. */
static GRKIT_ICON_COMPACT int status_free(int right,int (*block)[2],u32 n) {
    int left=STATUS_LEFT;
    u32 i;
    for(i=0;i<n;++i) {
        if(block[i][0]<right && block[i][1]>right) return right;
        if(block[i][1]<=right && block[i][1]>left) left=block[i][1];
    }
    return left<right ? left : right;
}
static GRKIT_ICON_COMPACT void viewfinder_status_paint(volatile u32 *h,const Surface *surface) {
    const volatile u8 *labels=LABEL_MAP(GRKIT_LABELS(framing_header()));
    /* For each shown indicator: the address of its icon pixels, or its core
       label with STATUS_LABEL_BIT (icon addresses are below 0x80000000); and
       its first and last column from the start of the row. */
#define STATUS_LABEL_BIT 0x80000000U
    u32 item[GRKIT_STATUS_ICONS_MAX],first[GRKIT_STATUS_ICONS_MAX],last[GRKIT_STATUS_ICONS_MAX];
    int block[STATUS_BLOCKS][2],right,room,fit=-1,widest=0,widest_right=STATUS_RIGHT;
    u32 i,n=0,blocks=0,next=0,left,x,y;
    if(cue_shown(h)) return;
    for(i=0;i<viewfinder_indicators(h) && n<GRKIT_STATUS_ICONS_MAX;++i) {
        u32 address=INDICATOR_ENTRY(h,i)[0],id,icon=0,w=40,gap=STATUS_STEP-40;
        if(!address || INDICATOR_WORD(address,4)!=INDICATOR_STATUS_ROW) continue;
        id=viewfinder_indicator_icon(h,i);
        if(id==GRKIT_ICON_OFF) continue;
        if(id-GRKIT_ICON_CORE_LABEL(0)<GRKIT_LABEL_COUNT) {
            u32 label=id-GRKIT_ICON_CORE_LABEL(0);
            w=label_width(label);
            if(!w || label_field(labels,label*8+2,2)>40) break;
            icon=label|STATUS_LABEL_BIT;
            gap=STATUS_LABEL_GAP;
        } else {
            if(id-GRKIT_ICON_MODULE_TILE(0)<GRKIT_MODULE_TILES_MAX) icon=module_tile_address(h,id);
            else if(id-0x8000U<h[GRKIT_HOST_COUNT/4])
                icon=WORD(h[GRKIT_HOST_RECORDS/4]+(id-0x8000U)*GRKIT_RECORD_SIZE,GRKIT_RECORD_ICON);
            if(!icon) continue;
        }
        if(next+w>STATUS_RIGHT-STATUS_LEFT) break;
        item[n]=icon; first[n]=next; last[n++]=next+w;
        next+=w+gap;
    }
    if(!n) return;
    status_views(h[GRKIT_HOST_FRAME_CANVAS/4],0,block,&blocks);
    /* A free part ends at STATUS_RIGHT or at the left of a block. Take the
       rightmost one that holds the row, else the widest one. */
    for(i=0;i<=blocks;++i) {
        right=i<blocks ? block[i][0] : STATUS_RIGHT;
        if(right>STATUS_RIGHT) continue;
        room=right-status_free(right,block,blocks);
        if(room>=(int)last[n-1]) { if(right>fit) fit=right; }
        else if(room>widest) { widest=room; widest_right=right; }
    }
    if(fit<0) {
        while(n && (int)last[n-1]>widest) --n;
        if(!n) return;
        fit=widest_right;
    }
    left=(u32)fit-last[n-1];
    for(i=0;i<n;++i) {
        u32 icon=item[i],at=left+first[i];
        if(icon&STATUS_LABEL_BIT) {
            u32 label=icon&~STATUS_LABEL_BIT;
            overlay_label(surface,label,at,label_field(labels,label*8+2,2)>STATUS_LINE_HEIGHT ? STATUS_TOP : LABEL_TOP);
            continue;
        }
        /* INSTALL stores the same binary-alpha RGBA pixels after each
           native-compatible descriptor, whose size differs by family. */
        icon+=GRKIT_ICON_STRIDE-GRKIT_GRF_ICON_SIZE;
        for(y=0;y<40;++y) {
            volatile u32 *row=OVERLAY_PIXELS(surface->data+(STATUS_TOP+y)*surface->stride*4)+at;
            for(x=0;x<40;++x) {
                u32 pixel=WORD(icon,(y*40+x)*4);
                if(pixel>>24) row[x]=pixel;
            }
        }
    }
#undef STATUS_LABEL_BIT
}
/* The ADJ canvas of the preview, or 0 (runtime/modules.h: frame_preview). */
static GRKIT_ICON_COMPACT u32 preview_canvas(volatile u32 *h) {
    u32 adj=h[GRKIT_HOST_FRAME_PREVIEW_ADJ/4];
    return adj ? WORD(adj,ADJ_CANVAS) : 0;
}
static GRKIT_ICON_COMPACT void prepare_canvas(volatile u32 *h,u32 canvas) {
    /* Canvas::Draw clears the full native bitmap before drawing its children.
       Use this path even for Off to remove the previous drawing on the next draw. */
    if((h[GRKIT_HOST_FRAME_CANVAS/4]==canvas &&
        (overlay_active(h) || h[GRKIT_HOST_FRAME_BITMAP/4])) ||
       (canvas && preview_canvas(h)==canvas))
        BYTE(canvas,0xa0)=1;
}
static GRKIT_ICON_COMPACT void viewfinder_prepare(u32 canvas) {
    volatile u32 *h=framing_header();
    u32 start=profile_begin(h);
    prepare_canvas(h,canvas);
    profile_end(h,start,GRKIT_PROFILE_FRAME_PREPARE);
}
/* Module pages preview their highlighted value on the ADJ canvas, below the
   menu controls (runtime/modules.h: frame_preview). The page of CORE.ASPECT
   previews the area of its highlighted ratio. After the page ends, one more
   full draw erases the drawing and releases the canvas. */
/* ---- The settings of a recipe: the panel of ADJ and the View page -------
   The items of runtime/recipe_details.h (runtime/services.h:
   SERVICES_DETAILS), with their icons: the icons of GRKit (effects and
   module tiles), or the camera's own icons from the firmware icon table.

   While the cursor of ADJ is on a recipe, the ADJ canvas shows them at the
   left of the live view (runtime/modules.h: frame_preview_set), below the
   title and the detail line of ADJ, which draw above the canvas: on black
   at 50% from PANEL_LEFT, PANEL_TOP, PANEL_WIDTH x PANEL_HEIGHT, the part of
   ADJ that the native controls leave free. Each group starts a row, and the
   items of a group follow each other and wrap. An item is its icon, then
   its short value where the icon does not show the value; the look is its
   icon and its name. The groups have more space between them than the rows
   of a group, and the block is centered in the black from top to bottom. */
#define PANEL_LEFT 16U
#define PANEL_TOP 100U
#define PANEL_WIDTH 432U
#define PANEL_HEIGHT 316U
#define PANEL_MARGIN 12U
#define PANEL_ICON 40U
#define PANEL_ITEM_GAP 10U
#define PANEL_TEXT_GAP 4U
#define PANEL_ROW_GAP 4U
#define PANEL_GROUP_GAP 10U
#define PANEL_FILL 0x80000000U
/* The address of the pixels of GRKit icon `id`, as the status row finds
   them, or 0. */
static GRKIT_ICON_COMPACT u32 panel_icon(volatile u32 *h,u32 id) {
    u32 icon=0;
    if(id-GRKIT_ICON_MODULE_TILE(0)<GRKIT_MODULE_TILES_MAX) icon=module_tile_address(h,id);
    else if(id-0x8000U<h[GRKIT_HOST_COUNT/4])
        icon=WORD(h[GRKIT_HOST_RECORDS/4]+(id-0x8000U)*GRKIT_RECORD_SIZE,GRKIT_RECORD_ICON);
    return icon ? icon+GRKIT_ICON_STRIDE-GRKIT_GRF_ICON_SIZE : 0;
}
/* The pixels of the camera's own icon `id` of the firmware icon table: its
   descriptor gives its size and its pixels as an offset in words from the
   base of the icon drawer, which the descriptor of the GRKit icon shows
   (runtime/installer.c: icon_sized). 0 for an icon that is not 40 x 40. */
static GRKIT_ICON_COMPACT u32 native_icon(volatile u32 *h,u32 id) {
    u32 own=h[GRKIT_HOST_GRKIT_ICON/4], d, width, height, offset, base;
    if(!own || !id) return 0;
    d=WORD(FW_ICON_TABLE,id*4);
    if(!d) return 0;
#if GRKIT_ICON_STRIDE-GRKIT_GRF_ICON_SIZE==12
    width=WORD(d,0)>>16; height=WORD(d,4); offset=WORD(d,8);
    base=own+12U-WORD(own,8)*4U;
#else
    width=WORD(d,0)&0xffffU; height=WORD(d,0)>>16; offset=WORD(d,4);
    base=own+8U-WORD(own,4)*4U;
#endif
    if(width!=PANEL_ICON || height!=PANEL_ICON) return 0;
    return base+offset*4U;
}
static GRKIT_ICON_COMPACT u32 detail_pixels(volatile u32 *h,u32 id) {
    if(!id) return 0;
    return id<DETAIL_NATIVE_ICONS ? native_icon(h,id) : panel_icon(h,id);
}
static GRKIT_ICON_COMPACT void panel_icon_paint(const Surface *s,u32 pixels,u32 left,u32 top) {
    u32 x,y,pixel;
    for(y=0;y<PANEL_ICON;++y) {
        volatile u32 *row=OVERLAY_PIXELS(s->data+(top+y)*s->stride*4)+left;
        for(x=0;x<PANEL_ICON;++x) { pixel=WORD(pixels,(y*PANEL_ICON+x)*4); if(pixel>>24) row[x]=pixel; }
    }
}
static GRKIT_ICON_COMPACT const volatile u16 *detail_text(volatile u32 *item,u32 at) {
    return (const volatile u16 *)(item+at/4);
}
/* Lay the items out from row `top` and give the height of the block; with a
   surface, paint them too. */
static GRKIT_ICON_COMPACT u32 panel_layout(volatile u32 *h,const Surface *s,u32 top) {
    volatile u32 *base=services_word(h,SERVICES_DETAILS), *item;
    u32 n=base[DETAIL_COUNT/4], left=PANEL_LEFT+PANEL_MARGIN, right=PANEL_LEFT+PANEL_WIDTH-PANEL_MARGIN;
    u32 bottom=PANEL_TOP+PANEL_HEIGHT-PANEL_MARGIN, x=left, y=top, group=0xffffffffU, i, g, icon, text_width, width;
    const volatile u16 *text;
    if(n>DETAIL_ITEMS_MAX) n=DETAIL_ITEMS_MAX;
    for(i=0;i<n;++i) {
        item=DETAIL_ITEM_AT(base,i);
        icon=item[DETAIL_ITEM_ICON/4];
        g=DETAIL_GROUP(item[DETAIL_ITEM_META/4]);
        /* The look writes its name; another item writes its short value
           after its icon, or its value without an icon. */
        text=detail_text(item,g==DETAIL_GROUP_LOOK || !icon ? DETAIL_ITEM_VALUE : DETAIL_ITEM_SHORT);
        text_width=overlay_text_width(text,DETAIL_VALUE_UNITS);
        width=(icon ? PANEL_ICON : 0)+(icon && text_width ? (g==DETAIL_GROUP_LOOK ? PANEL_ITEM_GAP : PANEL_TEXT_GAP) : 0)+text_width;
        if(g!=group) {
            if(group!=0xffffffffU)
                y+=PANEL_ICON+(group==DETAIL_GROUP_LOOK && g==DETAIL_GROUP_LEVELS ? PANEL_ROW_GAP : PANEL_GROUP_GAP);
            x=left;
            group=g;
        }
        else if(x+width>right) { y+=PANEL_ICON+PANEL_ROW_GAP; x=left; }
        if(s && y+PANEL_ICON<=bottom) {
            u32 pixels=detail_pixels(h,icon), at=x;
            if(pixels) panel_icon_paint(s,pixels,x,y);
            if(icon) at+=PANEL_ICON+(g==DETAIL_GROUP_LOOK ? PANEL_ITEM_GAP : PANEL_TEXT_GAP);
            if(text_width) overlay_text(s,text,DETAIL_VALUE_UNITS,at,y+(PANEL_ICON-GRKIT_GLYPH_HEIGHT)/2U,right);
        }
        x+=width+PANEL_ITEM_GAP;
    }
    return group==0xffffffffU ? 0 : y+PANEL_ICON-top;
}
static GRKIT_ICON_COMPACT void panel_paint(volatile u32 *h,const Surface *s) {
    u32 x, y, height=panel_layout(h,0,0), inner=PANEL_HEIGHT-2U*PANEL_MARGIN;
    for(y=PANEL_TOP;y<PANEL_TOP+PANEL_HEIGHT;++y) {
        volatile u32 *row=OVERLAY_PIXELS(s->data+y*s->stride*4);
        for(x=PANEL_LEFT;x<PANEL_LEFT+PANEL_WIDTH;++x) row[x]=PANEL_FILL;
    }
    if(height) panel_layout(h,s,PANEL_TOP+PANEL_MARGIN+(height<inner ? (inner-height)/2U : 0));
}
/* The View of a recipe on the camera information page, which hides its own
   lines (runtime/menu_about.h: view_fill): the items of the page that shows
   in VIEW_COLUMNS columns of VIEW_ROWS, below the title, each its icon, its
   English name in the small glyphs in gray, and its value below the name in
   the glyphs. With more than one page, the number of the page and of the
   pages, such as 1/2, at the bottom right. The live view paints them after
   the native children of the page, above its black. */
#define VIEW_LEFT 36U
#define VIEW_TOP 94U
#define VIEW_COLUMN 332U
#define VIEW_COLUMNS 2U
#define VIEW_ROWS 7U
#define VIEW_PITCH 46U
#define VIEW_TEXT 48U
#define VIEW_VALUE_TOP 17U
#define VIEW_NAME_SHADE 160U
/* The page number: right-aligned at VIEW_PAGE_RIGHT, on the line of the
   MENU icon of the page. */
#define VIEW_PAGE_RIGHT 696U
#define VIEW_PAGE_TOP 436U
_Static_assert(VIEW_COLUMNS*VIEW_ROWS==DETAIL_PAGE_ITEMS,"a page of the View holds its columns");
_Static_assert(DETAIL_ITEMS_MAX<=9U*DETAIL_PAGE_ITEMS,"the page number has one digit");
static GRKIT_ICON_COMPACT void view_paint(volatile u32 *h,const Surface *s) {
    volatile u32 *base=services_word(h,SERVICES_DETAILS), *item;
    u32 n=base[DETAIL_COUNT/4], pages=detail_pages(n), page=base[DETAIL_VIEW_PAGE/4], i, end;
    u32 x=VIEW_LEFT, y=VIEW_TOP, row=0, name, pixels, right;
    u16 number[6];
    if(page>=pages) page=pages-1U;
    i=page*DETAIL_PAGE_ITEMS;
    end=i+detail_page_items(page,n);
    for(;i<end;++i,y+=VIEW_PITCH) {
        /* Down each column, then the next column. */
        if(row++==VIEW_ROWS) { row=1; x+=VIEW_COLUMN; y=VIEW_TOP; }
        item=DETAIL_ITEM_AT(base,i);
        right=x+VIEW_COLUMN-8U;
        pixels=detail_pixels(h,item[DETAIL_ITEM_ICON/4]);
        if(pixels) panel_icon_paint(s,pixels,x,y+3U);
        name=DETAIL_NAME(item[DETAIL_ITEM_META/4]);
        if(name && name<DETAIL_NAMES) small_text(s,detail_names[name],x+VIEW_TEXT,y,right,VIEW_NAME_SHADE);
        overlay_text(s,detail_text(item,DETAIL_ITEM_VALUE),DETAIL_VALUE_UNITS,x+VIEW_TEXT,y+VIEW_VALUE_TOP,right);
    }
    if(pages<2U) return;
    /* DETAIL_ITEMS_MAX items give fewer than 10 pages. */
    number[0]=(u16)('1'+page); number[1]='/'; number[2]=(u16)('0'+pages); number[3]=0;
    overlay_text(s,number,4,VIEW_PAGE_RIGHT-overlay_text_width(number,4),VIEW_PAGE_TOP,VIEW_PAGE_RIGHT);
}
static GRKIT_ICON_COMPACT void preview_paint(volatile u32 *h,u32 canvas,u32 bitmap) {
    u32 preview=h[GRKIT_HOST_FRAME_PREVIEW/4],ratio=FRAME_PREVIEW_RATIO(preview),data,area,composition;
    Surface surface;
    if(!preview) { h[GRKIT_HOST_FRAME_PREVIEW_ADJ/4]=0; return; }
    data=canvas_pixels(canvas,bitmap);
    if(!data) return;
    surface.data=data; surface.stride=HALF(bitmap,0x18);
    if(ratio && ratio<=sizeof(photo_boxes)/sizeof(photo_boxes[0]) && module_layers(h)) {
        composition=composition_area(h,ratio-1);
        area=layer_area(composition);
        surface.area=area; surface.box=photo_boxes[area];
        module_layers_paint(h,&surface,1,composition);
    }
    if(FRAME_PREVIEW_PANEL(preview) && h[GRKIT_HOST_SERVICES/4]) {
        surface.area=0; surface.box=photo_boxes[0];
        panel_paint(h,&surface);
    }
}
static GRKIT_ICON_COMPACT void paint_canvas(volatile u32 *h,u32 canvas,u32 bitmap) {
    u32 owner=h[GRKIT_HOST_FRAME_OWNER/4],full,data,cue,status,framing,area;
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
    status=full && status_shown(h) ? full : 0;
    framing=aspect_value(h);
    area=layer_area(framing);
    if(!data && !cue && !status) return;
    if(data) {
        surface.data=data; surface.stride=HALF(bitmap,0x18); surface.area=area; surface.box=photo_boxes[area];
        module_layers_paint(h,&surface,0,framing);
    }
    if(status) {
        surface.data=status; surface.stride=HALF(bitmap,0x18); surface.area=0; surface.box=photo_boxes[0];
        viewfinder_status_paint(h,&surface);
    }
    if(cue) {
        /* Centered, on the baseline of the native focus distance. */
        surface.data=cue; surface.stride=HALF(bitmap,0x18); surface.area=area; surface.box=photo_boxes[area];
        overlay_label(&surface,LABEL_GRKIT,360-label_width(LABEL_GRKIT)/2,LABEL_TOP);
    }
    ++h[GRKIT_HOST_FRAME_DRAWS/4];
}

/* After the native children of the canvas: the crop badge, opaque, in the
   place of the camera's own, which it covers where the camera does not hide
   it (runtime/crop.h). */
static GRKIT_ICON_COMPACT void view_paint(volatile u32 *h,const Surface *s);
static GRKIT_ICON_COMPACT void paint_after(volatile u32 *h,u32 canvas,u32 bitmap) {
    u32 owner=h[GRKIT_HOST_FRAME_OWNER/4],full,badge,view;
    Surface surface;
    view=h[GRKIT_HOST_SERVICES/4] ? services_word(h,SERVICES_DETAILS+DETAIL_VIEW_CONTROLLER)[0] : 0;
    if(view && canvas==WORD(view,ADJ_CANVAS)) {
        full=canvas_bitmap(canvas,bitmap);
        if(!full) return;
        surface.data=full; surface.stride=HALF(bitmap,0x18); surface.area=0; surface.box=photo_boxes[0];
        view_paint(h,&surface);
        return;
    }
    if(canvas!=h[GRKIT_HOST_FRAME_CANVAS/4] || !owner) return;
    badge=crop_badge_shown(h);
    if(badge>=GRKIT_LABEL_COUNT) return;
    full=WORD(owner,4)==canvas ? canvas_bitmap(canvas,bitmap) : 0;
    if(!full) return;
    surface.data=full; surface.stride=HALF(bitmap,0x18); surface.area=0; surface.box=photo_boxes[0];
    overlay_label(&surface,badge,CROP_BADGE_LEFT,CROP_BADGE_TOP);
}

#endif
