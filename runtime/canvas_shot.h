#ifndef GRKIT_CANVAS_SHOT_H
#define GRKIT_CANVAS_SHOT_H
/* The canvas of a shot with a border (CORE.BORDER, runtime/canvas.h). Part
   of runtime/picture.c, after shot_word and picture_turns.

   1. Before R2Y writes the first stripe of a still, canvas_r2y lays out a
      larger picture in the bank of its recipe: a pitch with room for the
      side margins, room above and below for the top and bottom margins,
      and the previews moved behind the two planes. R2Y then writes the
      source picture inside it, at its own size.
   2. After each BackResize pass, canvas_resize puts the canvas around the
      main picture, fills the margins, and draws the previews from the whole
      canvas. A pass that crops (XPan, 1:1) or makes a smaller size (M, S)
      moves the main picture, so each pass places the canvas again, and a
      place outside the planes waits for the next pass. The first pass of a
      crop is such a place, and the review after the shot shows its
      previews: they take the border from the picture alone
      (canvas_crop), and the planes wait for the next pass.
   3. Before the encode, canvas_encode gives each image of the main picture
      the whole canvas, and canvas_inner gives the plane filters the picture
      inside the border.
   4. On a family with CANVAS_REVIEW, the review after the shot resizes the
      R2Y source, which the firmware gives the playback in a copy of its
      descriptor. canvas_review_c gives that copy the whole canvas, from the
      place of the canvas until its recipe lays out the next picture.

   A table entry (GRKIT_CANVAS_TABLE) follows a recipe from its layout to
   its encode.

   A family with a firmware image pool (CANVAS_POOL) takes the planes of a
   canvas that does not fit into the bank from the pool, while the pool keeps
   CANVAS_POOL_RESERVE free. The canvas is given back after the JPEG encode
   (canvas_encoded), or when the recipe lays out its next picture. Without
   the pool, the margins become as large as the bank allows. */
#include "canvas.h"

/* A family header that gives the canvas layout (runtime/targets) has the
   hooks of point picture.canvas. */
#ifdef CANVAS_R2Y_RECIPE
#define CANVAS_ENABLED 1

/* The words of a table entry. */
enum {
    CANVAS_E_STATE,   /* 0 free, 1 laid out, 2 placed, 3 encoding, 4 encoded */
    CANVAS_E_RECIPE,
    CANVAS_E_SHOT,
    CANVAS_E_BORDER,
    CANVAS_E_TURNS,   /* the turns | the margin limit << 16 */
    CANVAS_E_PITCH,
    CANVAS_E_BASE,    /* the Y plane; the CbCr plane follows at + PLANE */
    CANVAS_E_PLANE,
    CANVAS_E_SOURCE,  /* the Y of the R2Y source */
    CANVAS_E_CANVAS,  /* the Y of the top left of the canvas */
    CANVAS_E_SIZE,    /* canvas width | height << 16 */
    CANVAS_E_NEAR,    /* left | top << 16 */
    CANVAS_E_FAR,     /* right | bottom << 16 */
    CANVAS_E_MAIN,    /* the main picture of the placed canvas: w | h << 16 */
    CANVAS_E_BOX,     /* the screennail box, then the thumbnail box */
    CANVAS_E_POOL=CANVAS_E_BOX+2, /* the pool block of the planes, or 0 */
    CANVAS_E_WORDS
};
#define CANVAS_LAID_OUT 1U
#define CANVAS_PLACED 2U
/* The JPEG encode reads a canvas from the pool: it is given back after it. */
#define CANVAS_ENCODING 3U
/* The canvas of an encoded picture stays in the bank of its recipe, for the
   review (CANVAS_REVIEW), until the recipe lays out its next picture. The
   entry is free for another shot. */
#define CANVAS_ENCODED 4U
_Static_assert(CANVAS_E_WORDS*4<=GRKIT_CANVAS_ENTRY_SIZE,"canvas entry");

#define CANVAS_DESC(recipe,offset,word) WORD((recipe),(offset)+(word)*4)
#define CANVAS_DESC_Y 1
#define CANVAS_DESC_C 2
#define CANVAS_DESC_SIZE 4
#define CANVAS_DESC_PITCH 5
#ifndef CANVAS_FILL
#define CANVAS_FILL(descriptor,color) ((u32 (*)(u32,u32,u32,u32))FW_CANVAS_FILL)(0,(u32)(descriptor),(color),1)
#endif
/* The rows of the canvas above the source, for its top margin `top`. A
   family that keeps room there for R2Y defines its own. */
#ifndef CANVAS_ROWS_ABOVE
#define CANVAS_ROWS_ABOVE(top) ((top)>CANVAS_SOURCE_ROWS ? (top) : CANVAS_SOURCE_ROWS)
#endif
#ifdef FW_IMAGE_POOL
#define CANVAS_POOL 1
/* The image buffer allocator of the firmware: its instance, the free bytes
   and the largest free block of a kind of pool, a block of a kind (0 when
   the largest block is smaller), and the free of a block. */
#define CANVAS_POOL_THIS() ((u32 (*)(void))FW_IMAGE_POOL)()
static u32 canvas_pool_take(u32 bytes) {
    u32 total=0, largest=0;
    ((u32 (*)(u32,u32 *,u32 *,u32))FW_IMAGE_POOL_INFO)(CANVAS_POOL_THIS(),&total,&largest,CANVAS_POOL_KIND);
    if(largest<bytes || largest-bytes<CANVAS_POOL_RESERVE) return 0;
    return ((u32 (*)(u32,u32,u32))FW_IMAGE_POOL_ALLOC)(CANVAS_POOL_THIS(),bytes,CANVAS_POOL_KIND);
}
static void canvas_pool_give(u32 block) {
    ((void (*)(u32,u32))FW_IMAGE_POOL_FREE)(CANVAS_POOL_THIS(),block);
}
#endif

static volatile u32 *canvas_at(volatile u32 *h,u32 i) {
    return (volatile u32 *)PICTURE_MAP(PICTURE_SCRATCH(h)+GRKIT_CANVAS_TABLE+i*GRKIT_CANVAS_ENTRY_SIZE);
}
/* An entry ends: it gives back its pool block. */
static void canvas_end(volatile u32 *e) {
#ifdef CANVAS_POOL
    u32 block=e[CANVAS_E_POOL];
    e[CANVAS_E_STATE]=0;
    e[CANVAS_E_POOL]=0;
    if(block) canvas_pool_give(block);
#else
    e[CANVAS_E_STATE]=0;
#endif
}
/* The laid out or placed entry of a recipe, or 0. With `create`, a free or
   encoded entry for it, or else the first entry that is not encoding: the
   table holds the shots that are not encoded yet, and the oldest of four
   ends. 0 when all four are encoding. */
static volatile u32 *canvas_entry(volatile u32 *h,u32 recipe,u32 create) {
    u32 i, free=GRKIT_CANVAS_ENTRIES, other=GRKIT_CANVAS_ENTRIES;
    for(i=0;i<GRKIT_CANVAS_ENTRIES;++i) {
        volatile u32 *e=canvas_at(h,i);
        u32 state=e[CANVAS_E_STATE];
        if((state==CANVAS_LAID_OUT || state==CANVAS_PLACED) && e[CANVAS_E_RECIPE]==recipe) return e;
        if((!state || state==CANVAS_ENCODED) && !e[CANVAS_E_POOL] && free==GRKIT_CANVAS_ENTRIES) free=i;
        if(state!=CANVAS_ENCODING && other==GRKIT_CANVAS_ENTRIES) other=i;
    }
    if(!create) return 0;
    if(free<GRKIT_CANVAS_ENTRIES) return canvas_at(h,free);
    return other<GRKIT_CANVAS_ENTRIES ? canvas_at(h,other) : 0;
}

/* The bytes of a descriptor's Y and CbCr after its first Y: pitch x rows. */
static u32 canvas_bytes(u32 recipe,u32 offset) {
    u32 p=CANVAS_DESC(recipe,offset,CANVAS_DESC_PITCH);
    return (p&0xffffU)*(p>>16);
}

/* The border of an in-camera RAW development: the current CORE.BORDER and
   CORE.BORDER_COLOR, or 0 without a module that uses CORE.BORDER. A shot
   takes the border of its still build from the shot ring instead. */
static u32 canvas_current(volatile u32 *h) {
    if(!(h[GRKIT_HOST_CAPABILITIES/4]&GRKIT_CORE_BORDER_BIT)) return 0;
    return (picture_header_word(GRKIT_HOST_BORDER)&0xffU)|(picture_header_word(GRKIT_HOST_BORDER_COLOR)&0xffU)<<8;
}

/* Step 1: before the first stripe of a shot, the layout of its canvas. The
   margins come from the whole source; a crop or a smaller size later has
   smaller ones. When the canvas of the style does not fit into the bank,
   its planes come from the pool, or else the margins become as large as
   fits, and the previews stay in the bank. Leaves the stock layout when the
   shot has no border, or when the bank or its layout is not as expected. A
   RAW development turns as its DNG records. */
static void canvas_layout(volatile u32 *h,volatile u32 *e,u32 module,u32 config,u32 recipe) {
    static const u16 previews[3]={CANVAS_QUICK,CANVAS_SCREENNAIL,CANVAS_THUMBNAIL};
    u32 shot=WORD(recipe,CANVAS_SHOT), border, style, size, pitch, sw, sh, m[4], turns, limit, margin;
    u32 bank=WORD(recipe,CANVAS_BANK), end=bank+WORD(recipe,CANVAS_BANK_SIZE);
    u32 low=0xffffffffU, high=0, stride, rows, plane, base, source, moved, i, block=0;
    canvas_end(e);
    /* R2Y writes over the encoded canvas of the bank. */
    for(i=0;i<GRKIT_CANVAS_ENTRIES;++i) {
        volatile u32 *other=canvas_at(h,i);
        if(other[CANVAS_E_STATE]==CANVAS_ENCODED && other[CANVAS_E_RECIPE]==recipe) canvas_end(other);
    }
    if(PICTURE_DEVELOPING()) border=canvas_current(h);
    else if(!shot_word(h,shot,GRKIT_SHOT_BORDER,&border)) return;
    if(!(style=canvas_style(border))) return;
    size=CANVAS_DESC(recipe,CANVAS_SOURCE,CANVAS_DESC_SIZE);
    pitch=CANVAS_DESC(recipe,CANVAS_SOURCE,CANVAS_DESC_PITCH)&0xffffU;
    sw=size&0xffffU; sh=size>>16;
    if(sw<640 || sh<480 || sw>8192 || sh>8192 || pitch<sw || end<=bank) return;
    /* The stock layout: main, then the source CANVAS_SOURCE_ROWS later. */
    if(CANVAS_DESC(recipe,CANVAS_MAIN,CANVAS_DESC_Y)!=CANVAS_DESC(recipe,CANVAS_SOURCE,CANVAS_DESC_Y)-CANVAS_SOURCE_ROWS*pitch) return;
    turns=PICTURE_DEVELOPING() ? orientation_turns(PICTURE_PHOTO_ORIENTATION()) : picture_turns(h,shot);
    /* The previews keep their places to each other, behind the planes. */
    for(i=0;i<3;++i) {
        u32 y=CANVAS_DESC(recipe,previews[i],CANVAS_DESC_Y), c=CANVAS_DESC(recipe,previews[i],CANVAS_DESC_C), n=canvas_bytes(recipe,previews[i]);
        if(y<low) low=y;
        if(c<low) low=c;
        if(y+n>high) high=y+n;
        if(c+n>high) high=c+n;
    }
    if(high<=low) return;
    base=(bank+63U)&~63U;
    for(limit=CANVAS_NO_LIMIT;;limit=margin-CANVAS_MARGIN_ALIGN) {
        if(!(margin=canvas_margins(style,sw,sh,turns,limit,m))) return;
        stride=(sw+m[CANVAS_LEFT]+m[CANVAS_RIGHT]+63U)&~63U;
        rows=CANVAS_ROWS_ABOVE(m[CANVAS_TOP])+sh+(m[CANVAS_BOTTOM]>CANVAS_SOURCE_ROWS ? m[CANVAS_BOTTOM] : CANVAS_SOURCE_ROWS)+16U;
        rows=(rows+7U)&~7U;
        plane=stride*rows;
        moved=base+2U*plane;
        if(moved+(high-low)<=end) break;
#ifdef CANVAS_POOL
        if(limit==CANVAS_NO_LIMIT && (block=canvas_pool_take(2U*plane+64U))) {
            base=(block+63U)&~63U;
            moved=low;
            break;
        }
#endif
    }
    source=base+CANVAS_ROWS_ABOVE(m[CANVAS_TOP])*stride+m[CANVAS_LEFT];
    for(i=0;i<3;++i) {
        CANVAS_DESC(recipe,previews[i],CANVAS_DESC_Y)+=moved-low;
        CANVAS_DESC(recipe,previews[i],CANVAS_DESC_C)+=moved-low;
    }
    if(WORD(config,CANVAS_CONFIG_QUICK)) {
        WORD(config,CANVAS_CONFIG_QUICK)=CANVAS_DESC(recipe,CANVAS_QUICK,CANVAS_DESC_Y);
        WORD(config,CANVAS_CONFIG_QUICK+4)=CANVAS_DESC(recipe,CANVAS_QUICK,CANVAS_DESC_C);
    }
#ifdef CANVAS_CONFIG_MAIN
    if(WORD(config,CANVAS_CONFIG_MAIN)==CANVAS_DESC(recipe,CANVAS_MAIN,CANVAS_DESC_Y))
        WORD(config,CANVAS_CONFIG_MAIN)=source-CANVAS_SOURCE_ROWS*stride;
#endif
    CANVAS_DESC(recipe,CANVAS_SOURCE,CANVAS_DESC_Y)=source;
    CANVAS_DESC(recipe,CANVAS_SOURCE,CANVAS_DESC_C)=source+plane;
    CANVAS_DESC(recipe,CANVAS_SOURCE,CANVAS_DESC_PITCH)=(CANVAS_DESC(recipe,CANVAS_SOURCE,CANVAS_DESC_PITCH)&0xffff0000U)|stride;
    CANVAS_DESC(recipe,CANVAS_MAIN,CANVAS_DESC_Y)=source-CANVAS_SOURCE_ROWS*stride;
    CANVAS_DESC(recipe,CANVAS_MAIN,CANVAS_DESC_C)=source+plane-CANVAS_SOURCE_ROWS*stride;
    CANVAS_DESC(recipe,CANVAS_MAIN,CANVAS_DESC_PITCH)=(CANVAS_DESC(recipe,CANVAS_MAIN,CANVAS_DESC_PITCH)&0xffff0000U)|stride;
    WORD(config,CANVAS_CONFIG_SOURCE)=source;
    WORD(config,CANVAS_CONFIG_SOURCE+4)=source+plane;
    WORD(module,CANVAS_MODULE_SOURCE)=source;
    WORD(module,CANVAS_MODULE_SOURCE+4)=source+plane;
    e[CANVAS_E_RECIPE]=recipe; e[CANVAS_E_SHOT]=shot; e[CANVAS_E_BORDER]=border; e[CANVAS_E_POOL]=block;
    e[CANVAS_E_TURNS]=turns|(limit<CANVAS_NO_LIMIT ? margin : CANVAS_NO_LIMIT)<<16;
    e[CANVAS_E_PITCH]=stride; e[CANVAS_E_BASE]=base; e[CANVAS_E_PLANE]=plane; e[CANVAS_E_SOURCE]=source;
    e[CANVAS_E_CANVAS]=0; e[CANVAS_E_MAIN]=0; e[CANVAS_E_BOX]=0; e[CANVAS_E_BOX+1]=0;
    /* An entry of an earlier picture in this bank that was not encoded,
       such as the preview of a RAW development, ends. */
    for(i=0;i<GRKIT_CANVAS_ENTRIES;++i) {
        volatile u32 *other=canvas_at(h,i);
        if(other!=e && other[CANVAS_E_STATE] && other[CANVAS_E_STATE]!=CANVAS_ENCODING && other[CANVAS_E_BASE]==base) canvas_end(other);
    }
    PICTURE_BARRIER();
    e[CANVAS_E_STATE]=CANVAS_LAID_OUT;
}

/* Before each R2Y stripe send of a still. `module` is the StillMake module.
   The first stripe starts at the source; each stripe writes with the pitch
   of the canvas. */
void canvas_r2y_c(u32 module,u32 config) {
    volatile u32 *h=framing_header(), *e;
    u32 recipe=config-CANVAS_R2Y_RECIPE;
    if(WORD(module,CANVAS_MODULE_SOURCE)==WORD(config,CANVAS_CONFIG_SOURCE)) {
        e=canvas_entry(h,recipe,1);
        if(e && !(e[CANVAS_E_STATE] && e[CANVAS_E_RECIPE]==recipe &&
                  e[CANVAS_E_SOURCE]==CANVAS_DESC(recipe,CANVAS_SOURCE,CANVAS_DESC_Y)))
            canvas_layout(h,e,module,config,recipe);
    }
    e=canvas_entry(h,recipe,0);
    if(!e) return;
#ifdef CANVAS_MODULE_PITCH_WORD
    if(WORD(module,CANVAS_MODULE_PITCH_WORD)==HALF(module,CANVAS_MODULE_PITCH))
        WORD(module,CANVAS_MODULE_PITCH_WORD)=e[CANVAS_E_PITCH];
#endif
    HALF(module,CANVAS_MODULE_PITCH)=(u16)e[CANVAS_E_PITCH];
}

/* The margins of the IIP: four rectangles of the canvas around the picture. */
static void canvas_rect(u32 y,u32 plane,u32 pitch,u32 w,u32 hh,u32 color) {
    u32 d[8], rows=hh, n;
    if(!w || !hh) return;
#ifdef CANVAS_FILL_PIXELS
    /* The fill of a family with this limit takes a larger rectangle in
       parts and then checks it as if its pitch were its width: the margins
       go in parts below the limit. */
    rows=core_divide(CANVAS_FILL_PIXELS-1U,w);
    if(!rows) rows=1;
#endif
    for(;hh;hh-=n,y+=n*pitch) {
        n=hh<rows ? hh : rows;
        d[0]=CANVAS_FILL_FORMAT; d[1]=y; d[2]=y+plane; d[3]=0;
        d[4]=w|n<<16; d[5]=pitch|n<<16; d[6]=0; d[7]=CANVAS_FILL_TAG;
        CANVAS_FILL(d,color);
    }
}
static void canvas_fill(volatile u32 *e) {
    u32 y=e[CANVAS_E_CANVAS], plane=e[CANVAS_E_PLANE], pitch=e[CANVAS_E_PITCH];
    u32 cw=e[CANVAS_E_SIZE]&0xffffU, ch=e[CANVAS_E_SIZE]>>16;
    u32 l=e[CANVAS_E_NEAR]&0xffffU, t=e[CANVAS_E_NEAR]>>16, r=e[CANVAS_E_FAR]&0xffffU, b=e[CANVAS_E_FAR]>>16;
    u32 color=canvas_luma(e[CANVAS_E_BORDER]) ? CANVAS_FILL_WHITE : CANVAS_FILL_BLACK, ih=ch-t-b;
    canvas_rect(y,plane,pitch,cw,t,color);
    canvas_rect(y+(t+ih)*pitch,plane,pitch,cw,b,color);
    canvas_rect(y+t*pitch,plane,pitch,l,ih,color);
    canvas_rect(y+t*pitch+cw-r,plane,pitch,r,ih,color);
}

/* Place the canvas around a main picture at y, w x h: 1 when the canvas
   lies inside the planes, else 0. */
static u32 canvas_place(volatile u32 *e,u32 y,u32 w,u32 hh) {
    u32 base=e[CANVAS_E_BASE], pitch=e[CANVAS_E_PITCH], m[4], row, column, cw, ch;
    if(y<base || y>=base+e[CANVAS_E_PLANE] || !w || !hh) return 0;
    row=core_divide(y-base,pitch); column=y-base-row*pitch;
    canvas_margins(canvas_style(e[CANVAS_E_BORDER]),w,hh,e[CANVAS_E_TURNS]&3U,e[CANVAS_E_TURNS]>>16,m);
    cw=w+m[CANVAS_LEFT]+m[CANVAS_RIGHT]; ch=hh+m[CANVAS_TOP]+m[CANVAS_BOTTOM];
    if(row<m[CANVAS_TOP] || column<m[CANVAS_LEFT] || column+w+m[CANVAS_RIGHT]>pitch ||
       (row-m[CANVAS_TOP]+ch)*pitch>e[CANVAS_E_PLANE]) return 0;
    e[CANVAS_E_CANVAS]=y-m[CANVAS_TOP]*pitch-m[CANVAS_LEFT];
    e[CANVAS_E_SIZE]=cw|ch<<16;
    e[CANVAS_E_NEAR]=m[CANVAS_LEFT]|m[CANVAS_TOP]<<16;
    e[CANVAS_E_FAR]=m[CANVAS_RIGHT]|m[CANVAS_BOTTOM]<<16;
    e[CANVAS_E_MAIN]=w|hh<<16;
    return 1;
}

/* The place of the canvas of `e` in a preview box: the fitted size, its
   offset in the box, and the canvas samples per preview sample in 1/65536
   (canvas_sample). Returns 0 for a canvas or a box that is too small. */
enum { CANVAS_GRID_W, CANVAS_GRID_H, CANVAS_GRID_X, CANVAS_GRID_Y, CANVAS_GRID_STEP_X, CANVAS_GRID_STEP_Y, CANVAS_GRID_WORDS };
static u32 canvas_preview_grid(volatile u32 *e,u32 box,u32 g[CANVAS_GRID_WORDS]) {
    u32 cw=e[CANVAS_E_SIZE]&0xffffU, ch=e[CANVAS_E_SIZE]>>16, fit=canvas_fit(cw,ch,box&0xffffU,box>>16);
    g[CANVAS_GRID_W]=fit&0xffffU; g[CANVAS_GRID_H]=fit>>16;
    if(g[CANVAS_GRID_W]<2 || g[CANVAS_GRID_H]<2 || cw<2 || ch<2) return 0;
#ifdef CANVAS_PREVIEW_BOX
    /* The preview keeps the size of its box: black bars around the canvas,
       as on a stock XPan picture. */
    g[CANVAS_GRID_X]=(((box&0xffffU)-g[CANVAS_GRID_W])>>1)&~1U;
    g[CANVAS_GRID_Y]=((box>>16)-g[CANVAS_GRID_H])>>1;
#else
    g[CANVAS_GRID_X]=g[CANVAS_GRID_Y]=0;
#endif
    g[CANVAS_GRID_STEP_X]=core_divide(cw<<16,g[CANVAS_GRID_W]);
    g[CANVAS_GRID_STEP_Y]=core_divide(ch<<16,g[CANVAS_GRID_H]);
    return 1;
}

/* The samples [x0, x1) of row r of the fitted canvas in a preview, with
   yd and cd at the first sample of the row: each sample is the mean of
   2 x 2 luma around its middle (canvas_sample), with the CbCr pair there.
   A sample outside `inside` (left, top, right, bottom in canvas samples)
   takes the color of the border. The R2Y and the IIP write the canvas
   without the CPU cache, so its rows are flushed before they are read. */
static void canvas_preview_span(volatile u32 *e,const u32 g[CANVAS_GRID_WORDS],const u32 inside[4],u8 *yd,u8 *cd,u32 r,u32 x0,u32 x1) {
    u32 y=e[CANVAS_E_CANVAS], plane=e[CANVAS_E_PLANE], pitch=e[CANVAS_E_PITCH];
    u32 cw=e[CANVAS_E_SIZE]&0xffffU, sy=canvas_sample(r,g[CANVAS_GRID_STEP_Y],e[CANVAS_E_SIZE]>>16);
    u32 luma=canvas_luma(e[CANVAS_E_BORDER]), x;
    const u8 *ys, *cs;
    if(sy<inside[CANVAS_TOP] || sy>=inside[CANVAS_BOTTOM]) {
        for(x=x0;x<x1;++x) { yd[x]=(u8)luma; cd[x]=128; }
        return;
    }
    PICTURE_CACHE_FLUSH(y+sy*pitch,cw);
    PICTURE_CACHE_FLUSH(y+(sy+1U)*pitch,cw);
    PICTURE_CACHE_FLUSH(y+plane+sy*pitch,cw);
    ys=PICTURE_MAP(y+sy*pitch); cs=PICTURE_MAP(y+plane+sy*pitch);
    for(x=x0;x<x1;++x) {
        u32 sx=canvas_sample(x,g[CANVAS_GRID_STEP_X],cw);
        if(sx<inside[CANVAS_LEFT] || sx>=inside[CANVAS_RIGHT]) { yd[x]=(u8)luma; cd[x]=128; continue; }
        yd[x]=(u8)((ys[sx]+ys[sx+1]+ys[sx+pitch]+ys[sx+pitch+1]+2U)>>2);
        cd[x]=cs[(sx&~1U)+(x&1U)];
    }
}

/* A preview of the whole canvas in a descriptor of R, fit into its box
   (canvas_preview_grid, canvas_preview_span). The preview is flushed after
   it is written, for the encode. With `drawn` 0, the margins are not filled
   yet: a sample in a margin takes the color of the border, and the planes
   are read only inside the picture. */
static void canvas_preview(volatile u32 *e,u32 recipe,u32 offset,u32 box,u32 drawn) {
    u32 cw=e[CANVAS_E_SIZE]&0xffffU, ch=e[CANVAS_E_SIZE]>>16, g[CANVAS_GRID_WORDS], fit, r;
    u32 py=CANVAS_DESC(recipe,offset,CANVAS_DESC_Y), pc=CANVAS_DESC(recipe,offset,CANVAS_DESC_C);
    u32 ps=CANVAS_DESC(recipe,offset,CANVAS_DESC_PITCH)&0xffffU;
    /* The picture inside the margins, in canvas samples: [left, right) x
       [top, bottom). Each sample of `drawn` is inside. */
    u32 inside[4]={0,0,cw,ch};
    if(!drawn) {
        inside[CANVAS_LEFT]=e[CANVAS_E_NEAR]&0xffffU; inside[CANVAS_TOP]=e[CANVAS_E_NEAR]>>16;
        inside[CANVAS_RIGHT]=cw-(e[CANVAS_E_FAR]&0xffffU); inside[CANVAS_BOTTOM]=ch-(e[CANVAS_E_FAR]>>16);
    }
    if(ps<(box&0xffffU) || !canvas_preview_grid(e,box,g)) return;
    fit=g[CANVAS_GRID_W]|g[CANVAS_GRID_H]<<16;
#ifdef CANVAS_PREVIEW_BOX
    {
        u32 bw=box&0xffffU, bh=box>>16, x;
        for(r=0;r<bh;++r) {
            u8 *yb=PICTURE_MAP(py+r*ps), *cb=PICTURE_MAP(pc+r*ps);
            if(r>=g[CANVAS_GRID_Y] && r<g[CANVAS_GRID_Y]+g[CANVAS_GRID_H] && !g[CANVAS_GRID_X] && g[CANVAS_GRID_W]==bw) continue;
            for(x=0;x<bw;++x) { yb[x]=0; cb[x]=128; }
        }
        py+=g[CANVAS_GRID_Y]*ps+g[CANVAS_GRID_X]; pc+=g[CANVAS_GRID_Y]*ps+g[CANVAS_GRID_X];
        fit=box;
    }
#endif
    for(r=0;r<g[CANVAS_GRID_H];++r)
        canvas_preview_span(e,g,inside,PICTURE_MAP(py+r*ps),PICTURE_MAP(pc+r*ps),r,0,g[CANVAS_GRID_W]);
    PICTURE_CACHE_FLUSH(CANVAS_DESC(recipe,offset,CANVAS_DESC_Y),ps*(fit>>16));
    PICTURE_CACHE_FLUSH(CANVAS_DESC(recipe,offset,CANVAS_DESC_C),ps*(fit>>16));
    CANVAS_DESC(recipe,offset,CANVAS_DESC_SIZE)=fit;
}

/* The screennail and the thumbnail of R from the placed canvas of `e`
   (canvas_preview), each fit into the box of its stock preview. */
static void canvas_previews(volatile u32 *e,u32 recipe,u32 drawn) {
    static const u16 previews[2]={CANVAS_SCREENNAIL,CANVAS_THUMBNAIL};
    u32 i;
    for(i=0;i<2;++i) {
        if(!e[CANVAS_E_BOX+i]) e[CANVAS_E_BOX+i]=CANVAS_DESC(recipe,previews[i],CANVAS_DESC_SIZE);
        canvas_preview(e,recipe,previews[i],e[CANVAS_E_BOX+i],drawn);
    }
}

/* A pass of a crop (a Framing ratio, XPan, CORE.CROP) moves the main
   picture to the top left of the crop in the R2Y source, but keeps the size
   of the source. The next pass gives it the size of the crop. The crop lies
   in the middle of the source, so the move gives its size: w | h << 16, or
   0 when the main picture of size `size` at y is not such a move. The size
   can differ from the size of the next pass by the alignment of the crop. */
static u32 canvas_crop(volatile u32 *e,u32 y,u32 size,u32 source_size) {
    u32 source=e[CANVAS_E_SOURCE], pitch=e[CANVAS_E_PITCH], w=size&0xffffU, hh=size>>16, row, column;
    if(size!=source_size || y<=source || !pitch) return 0;
    row=core_divide(y-source,pitch); column=y-source-row*pitch;
    if(2U*column>=w || 2U*row>=hh) return 0;
    return (w-2U*column)|(hh-2U*row)<<16;
}

/* After each BackResize pass of a still. */
void canvas_resize_c(u32 module,u32 config) {
    volatile u32 *h=framing_header(), *e;
    u32 recipe=config-CANVAS_RESIZE_RECIPE, y, size, i;
    static const u16 previews[2]={CANVAS_SCREENNAIL,CANVAS_THUMBNAIL};
    (void)module;
    e=canvas_entry(h,recipe,0);
    if(!e) return;
    y=CANVAS_DESC(recipe,CANVAS_MAIN,CANVAS_DESC_Y);
    size=CANVAS_DESC(recipe,CANVAS_MAIN,CANVAS_DESC_SIZE);
    if((CANVAS_DESC(recipe,CANVAS_MAIN,CANVAS_DESC_PITCH)&0xffffU)!=e[CANVAS_E_PITCH]) return;
    if(e[CANVAS_E_STATE]!=CANVAS_PLACED || size!=e[CANVAS_E_MAIN] ||
       e[CANVAS_E_CANVAS]+(e[CANVAS_E_NEAR]>>16)*e[CANVAS_E_PITCH]+(e[CANVAS_E_NEAR]&0xffffU)!=y) {
        /* The first pass, or a pass that moved the main picture. A place
           outside the planes waits for the next pass, and the encode keeps
           the stock picture without one: the previews get their boxes
           again. */
        e[CANVAS_E_STATE]=CANVAS_LAID_OUT;
        if(!canvas_place(e,y,size&0xffffU,size>>16)) {
            u32 crop=canvas_crop(e,y,size,CANVAS_DESC(recipe,CANVAS_SOURCE,CANVAS_DESC_SIZE));
            for(i=0;i<2;++i) if(e[CANVAS_E_BOX+i]) CANVAS_DESC(recipe,previews[i],CANVAS_DESC_SIZE)=e[CANVAS_E_BOX+i];
            /* The review after the shot shows the previews of this pass:
               they take the border around the crop, from the picture
               without the margins. The planes stay as they are until the
               next pass places the canvas. */
            if(crop && canvas_place(e,y,crop&0xffffU,crop>>16)) canvas_previews(e,recipe,0);
            return;
        }
        canvas_fill(e);
        e[CANVAS_E_STATE]=CANVAS_PLACED;
    }
    /* Each pass draws the previews again from the main picture. */
    canvas_previews(e,recipe,1);
}

/* A descriptor `d` takes the whole canvas of an entry. */
static void canvas_describe(volatile u32 *e,u32 d) {
    u32 y=e[CANVAS_E_CANVAS], size=e[CANVAS_E_SIZE];
    WORD(d,4)=y; WORD(d,8)=y+e[CANVAS_E_PLANE];
    WORD(d,0x10)=size;
    WORD(d,0x14)=e[CANVAS_E_PITCH]|(size&0xffff0000U);
    WORD(d,0x18)=0;
}

/* Before the encode of the pictures of `config`: each image of the main
   picture of a placed canvas takes the whole canvas. Gives the entry in
   `out`, or returns 0 without a canvas. The entry then is encoding while the
   encode reads its pool block, or else encoded with CANVAS_REVIEW, or free. */
static u32 canvas_encode(volatile u32 *h,u32 config,u32 out[CANVAS_E_WORDS]) {
    u32 i, j, n=0;
    for(i=0;i<GRKIT_CANVAS_ENTRIES && !n;++i) {
        volatile u32 *e=canvas_at(h,i);
        u32 base=e[CANVAS_E_BASE], plane=e[CANVAS_E_PLANE];
        /* The shot settings of a RAW development are those of the playback,
           so only the place of its pictures tells its canvas. */
        if(e[CANVAS_E_STATE]!=CANVAS_PLACED || (WORD(config,0x34)!=e[CANVAS_E_SHOT] && !PICTURE_DEVELOPING())) continue;
        /* The four images of the config, from +0x3c. */
        for(j=0;j<4;++j) {
            u32 d=WORD(config,0x3c+j*4);
            if(d<0x40000000U || d>0xbfffff00U || WORD(d,4)<base || WORD(d,4)>=base+plane) continue;
            canvas_describe(e,d);
            ++n;
        }
        if(n) {
            for(j=0;j<CANVAS_E_WORDS;++j) out[j]=e[j];
#ifdef CANVAS_REVIEW
            e[CANVAS_E_STATE]=e[CANVAS_E_POOL] ? CANVAS_ENCODING : CANVAS_ENCODED;
#else
            e[CANVAS_E_STATE]=e[CANVAS_E_POOL] ? CANVAS_ENCODING : 0U;
#endif
        }
    }
    return n!=0;
}

#ifdef CANVAS_POOL
/* After the encode of the pictures of `config`: the canvas that it read
   gives back its pool block. The encode returns when the JPEG is done. */
static void canvas_encoded(volatile u32 *h,u32 config) {
    u32 i, j;
    for(i=0;i<GRKIT_CANVAS_ENTRIES;++i) {
        volatile u32 *e=canvas_at(h,i);
        u32 base=e[CANVAS_E_BASE], end=base+2U*e[CANVAS_E_PLANE];
        if(e[CANVAS_E_STATE]!=CANVAS_ENCODING) continue;
        for(j=0;j<4;++j) {
            u32 d=WORD(config,0x3c+j*4);
            if(d<0x40000000U || d>0xbfffff00U || WORD(d,4)<base || WORD(d,4)>=end) continue;
            canvas_end(e);
            break;
        }
    }
}
#endif

#ifdef CANVAS_REVIEW
/* The copy `d` of a descriptor (as in R) that the review resizes: when its
   picture is the R2Y source of a placed or encoded canvas, it takes the
   whole canvas. A pool canvas that the encode gave back stays out. */
void canvas_review_c(u32 d) {
    volatile u32 *h=framing_header();
    u32 i, y=WORD(d,4);
    for(i=0;i<GRKIT_CANVAS_ENTRIES;++i) {
        volatile u32 *e=canvas_at(h,i);
        u32 state=e[CANVAS_E_STATE];
        if((state==CANVAS_PLACED || state==CANVAS_ENCODING || state==CANVAS_ENCODED) && e[CANVAS_E_SOURCE]==y) {
            canvas_describe(e,d);
            return;
        }
    }
}
#endif

/* The filter step of DigitalFilterProcess runs the Shading and the Clarity
   of an Image Control on the source and the destination picture of a job:
   job + CANVAS_FILTER_JOB_SOURCE and + CANVAS_FILTER_JOB_DESTINATION point
   to their descriptors, and the step takes the whole picture of each, with
   the center of the Shading in its middle. A shot and the Execute of a RAW
   development run it on the main picture, which is the picture inside the
   canvas. The preview of a RAW development runs it on the screennail, which
   holds the whole canvas: the border would take the Shading, and the
   vignette its middle. So a job picture that is the screennail or the
   thumbnail of a placed canvas takes a copy of its descriptor for the
   picture inside the border (canvas_inner_area).
   The step moves the start of each row up and its end down to a multiple
   of CANVAS_FILTER_ALIGN bytes. So the copy starts and ends on such a
   multiple outside the picture, and after the step the columns of the
   border in it are drawn again from the canvas (canvas_preview_span).
   The copies live in the frame of the hook (runtime/picture.S): two saved
   descriptor pointers, then two copies. */
#define CANVAS_FILTER_JOB_SOURCE 4
#define CANVAS_FILTER_JOB_DESTINATION 8
#define CANVAS_FILTER_ALIGN 16U
/* The words of a descriptor copy: the descriptor and what follows it. */
#define CANVAS_FILTER_DESC_WORDS 16U

/* The placed canvas that has the preview picture of descriptor d: its
   entry, with the preview (0 screennail, 1 thumbnail) in *which; or 0. */
static volatile u32 *canvas_filter_entry(u32 d,u32 *which) {
    static const u16 previews[2]={CANVAS_SCREENNAIL,CANVAS_THUMBNAIL};
    volatile u32 *h=framing_header();
    u32 i, j;
    if(d<0x40000000U || d>0xbfffff00U) return 0;
    for(i=0;i<GRKIT_CANVAS_ENTRIES;++i) {
        volatile u32 *e=canvas_at(h,i);
        u32 recipe=e[CANVAS_E_RECIPE];
        if(e[CANVAS_E_STATE]!=CANVAS_PLACED || !recipe) continue;
        for(j=0;j<2;++j)
            if(WORD(d,CANVAS_DESC_Y*4)==CANVAS_DESC(recipe,previews[j],CANVAS_DESC_Y)) {
                *which=j;
                return e;
            }
    }
    return 0;
}

/* The columns of the step in the preview of descriptor d of the placed
   canvas of e: the picture inside the border in `a` (canvas_inner_area),
   and the aligned columns [*left, *right) around it. Returns 0 for no
   picture inside, or a row that does not start on an aligned byte. */
static u32 canvas_filter_area(volatile u32 *e,u32 d,u32 a[4],u32 *left,u32 *right) {
    u32 m[4], size=WORD(d,CANVAS_DESC_SIZE*4), pitch=WORD(d,CANVAS_DESC_PITCH*4)&0xffffU;
    u32 row=WORD(d,CANVAS_DESC_Y*4), r;
    m[CANVAS_LEFT]=e[CANVAS_E_NEAR]&0xffffU; m[CANVAS_TOP]=e[CANVAS_E_NEAR]>>16;
    m[CANVAS_RIGHT]=e[CANVAS_E_FAR]&0xffffU; m[CANVAS_BOTTOM]=e[CANVAS_E_FAR]>>16;
    if(!canvas_inner_area(e[CANVAS_E_SIZE]&0xffffU,e[CANVAS_E_SIZE]>>16,m,size&0xffffU,size>>16,a)) return 0;
    if(a[CANVAS_RIGHT]<=a[CANVAS_LEFT]+CANVAS_FILTER_ALIGN || a[CANVAS_BOTTOM]<=a[CANVAS_TOP]+2U) return 0;
    row+=a[CANVAS_TOP]*pitch;
    if((row|pitch)&(CANVAS_FILTER_ALIGN-1U)) return 0;
    *left=a[CANVAS_LEFT]&~(CANVAS_FILTER_ALIGN-1U);
    r=(a[CANVAS_RIGHT]+CANVAS_FILTER_ALIGN-1U)&~(CANVAS_FILTER_ALIGN-1U);
    /* The last columns of a row without room after it stay out. */
    *right=r<=pitch ? r : a[CANVAS_RIGHT]&~(CANVAS_FILTER_ALIGN-1U);
    return 1;
}

/* Before the filter step of `job`, with the frame of the hook. */
void canvas_filter_c(u32 job,u32 *frame) {
    u32 k, n=0, d, which, a[4], left, right, pitch, i;
    frame[0]=WORD(job,CANVAS_FILTER_JOB_SOURCE);
    frame[1]=WORD(job,CANVAS_FILTER_JOB_DESTINATION);
    for(k=0;k<2;++k) {
        volatile u32 *e;
        u32 *copy=frame+2+n*CANVAS_FILTER_DESC_WORDS, offset;
        d=frame[k];
        /* A job that takes one picture twice takes one copy. */
        if(k==1 && d==frame[0]) {
            WORD(job,CANVAS_FILTER_JOB_DESTINATION)=WORD(job,CANVAS_FILTER_JOB_SOURCE);
            continue;
        }
        e=canvas_filter_entry(d,&which);
        if(!e || !canvas_filter_area(e,d,a,&left,&right)) continue;
        pitch=WORD(d,CANVAS_DESC_PITCH*4)&0xffffU;
        offset=a[CANVAS_TOP]*pitch+left;
        for(i=0;i<CANVAS_FILTER_DESC_WORDS;++i) copy[i]=WORD(d,i*4);
        copy[CANVAS_DESC_Y]+=offset;
        copy[CANVAS_DESC_C]+=offset;
        copy[CANVAS_DESC_SIZE]=(right-left)|(a[CANVAS_BOTTOM]-a[CANVAS_TOP])<<16;
        copy[CANVAS_DESC_PITCH]=pitch|(a[CANVAS_BOTTOM]-a[CANVAS_TOP])<<16;
        WORD(job,k ? CANVAS_FILTER_JOB_DESTINATION : CANVAS_FILTER_JOB_SOURCE)=(u32)copy;
        ++n;
    }
}

/* The columns [x0, x1) of the rows of `a` in the preview of descriptor d
   take the canvas again. */
static void canvas_filter_redraw(volatile u32 *e,u32 d,u32 which,const u32 a[4],u32 x0,u32 x1) {
    u32 g[CANVAS_GRID_WORDS], inside[4]={0,0,e[CANVAS_E_SIZE]&0xffffU,e[CANVAS_E_SIZE]>>16};
    u32 y=WORD(d,CANVAS_DESC_Y*4), c=WORD(d,CANVAS_DESC_C*4), pitch=WORD(d,CANVAS_DESC_PITCH*4)&0xffffU;
    u32 r, x, ox, oy;
    if(x0>=x1 || !canvas_preview_grid(e,e[CANVAS_E_BOX+which],g)) return;
    ox=g[CANVAS_GRID_X]; oy=g[CANVAS_GRID_Y];
    for(r=a[CANVAS_TOP];r<a[CANVAS_BOTTOM];++r) {
        u8 *yd=PICTURE_MAP(y+r*pitch), *cd=PICTURE_MAP(c+r*pitch);
        PICTURE_CACHE_FLUSH(y+r*pitch+x0,x1-x0);
        PICTURE_CACHE_FLUSH(c+r*pitch+x0,x1-x0);
        for(x=x0;x<x1 && x<ox;++x) { yd[x]=0; cd[x]=128; }
        if(r>=oy && r<oy+g[CANVAS_GRID_H] && x<ox+g[CANVAS_GRID_W]) {
            u32 end=x1<ox+g[CANVAS_GRID_W] ? x1 : ox+g[CANVAS_GRID_W];
            canvas_preview_span(e,g,inside,yd+ox,cd+ox,r-oy,x-ox,end-ox);
            x=end;
        }
        for(;x<x1;++x) { yd[x]=0; cd[x]=128; }
        PICTURE_CACHE_FLUSH(y+r*pitch+x0,x1-x0);
        PICTURE_CACHE_FLUSH(c+r*pitch+x0,x1-x0);
    }
}

/* After the filter step: the border columns that the step took are drawn
   again, and the job takes its own descriptors again. */
void canvas_filter_end(u32 job,const u32 *frame) {
    u32 d=frame[1], which, a[4], left, right, width;
    volatile u32 *e;
    if(WORD(job,CANVAS_FILTER_JOB_DESTINATION)!=d && (e=canvas_filter_entry(d,&which))!=0 &&
       canvas_filter_area(e,d,a,&left,&right)) {
        width=WORD(d,CANVAS_DESC_SIZE*4)&0xffffU;
        canvas_filter_redraw(e,d,which,a,left,a[CANVAS_LEFT]);
        canvas_filter_redraw(e,d,which,a,a[CANVAS_RIGHT],right<width ? right : width);
    }
    WORD(job,CANVAS_FILTER_JOB_SOURCE)=frame[0];
    WORD(job,CANVAS_FILTER_JOB_DESTINATION)=frame[1];
}

/* The plane filters see the picture inside the border: the visible area of
   each plane is the picture, in the scale of the canvas in that plane. */
static void canvas_inner(const u32 e[CANVAS_E_WORDS],PictureSet *set) {
    u32 m[4], a[4], i;
    m[CANVAS_LEFT]=e[CANVAS_E_NEAR]&0xffffU; m[CANVAS_TOP]=e[CANVAS_E_NEAR]>>16;
    m[CANVAS_RIGHT]=e[CANVAS_E_FAR]&0xffffU; m[CANVAS_BOTTOM]=e[CANVAS_E_FAR]>>16;
    for(i=0;i<set->count;++i) {
        Plane *p=&set->planes[i];
        if(!canvas_inner_area(e[CANVAS_E_SIZE]&0xffffU,e[CANVAS_E_SIZE]>>16,m,p->width,p->height,a)) continue;
        p->left=a[CANVAS_LEFT]; p->top=a[CANVAS_TOP]; p->right=a[CANVAS_RIGHT]; p->bottom=a[CANVAS_BOTTOM];
    }
}
#else
#define CANVAS_ENABLED 0
#endif
#endif
