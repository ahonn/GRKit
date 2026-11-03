/* Date Imprint: the plane filter. It prints the local time on each
   recorded picture, as a film data back exposes it. */
#include "grkit.h"
#include "grkit_module.h"

/* A byte of a plane. */
#define BYTE(p,o) (GRKIT_BYTES((p)+(o))[0])

/* Longest blurred line of a date stamp mask, and the largest blur radius
   (look_halo). The line copy lives on the stack, because the code region is
   read-only. */
#define BLUR_WIDTH 640U
#define BLUR_RADIUS 32U

/* Spread a mask line in place: three box passes, radius r. A line of zeros
   stays zero. The copy repeats the end values for r + 1 places on each side,
   so the window needs no test at the ends. */
static void blur_line(u8 *line,u32 step,u32 n,u32 r) {
    u16 copy[BLUR_WIDTH+2*BLUR_RADIUS+2], *at=copy+BLUR_RADIUS+1;
    u32 i,pass,sum,window=2*r+1,reciprocal;
    for(i=0;i<n && !line[i*step];++i) {}
    if(i==n || r>BLUR_RADIUS) return;
    reciprocal=divide(65536+window/2,window);
    for(pass=0;pass<3;++pass) {
        for(i=0;i<n;++i) at[i]=line[i*step];
        for(i=1;i<=r+1;++i) { at[-(int)i]=at[0]; at[n-1+i]=at[n-1]; }
        sum=at[0]*(r+1);
        for(i=1;i<=r;++i) sum+=at[i];
        for(i=0;i<n;++i) {
            line[i*step]=(u8)((sum*reciprocal)>>16);
            sum+=at[i+r+1];
            sum-=at[(int)i-(int)r];
        }
    }
}

/* The looks of the date stamp (the setting LOOK). The classic and
   GR1s looks light the picture like an LED exposure of a film data back. The
   classic digits have an amber core in a red-orange halo. GR1s digits are
   mostly opaque yellow, as on GR1s color negatives. Both are white light in
   the luma only on a monochrome picture. Their characters render into a small
   coverage mask (at most STAMP_CELLS rows); the halo is a box blur of it. The
   mask is then scaled up and added to the picture with a screen blend. The
   GR II look prints flat pixel digits, as the date imprint of that camera
   does. */
#define LOOK_CLASSIC 0U
#define LOOK_DOTS 1U
#define LOOK_GR2 2U
/* Mask digit height: up to 48 cells, or 32 when the text would exceed the
   mask width. Higher masks keep the strokes sharper after scaling. */
#define STAMP_CELLS 48U
#define STAMP_CELLS_LONG 32U

/* Classic: italic seven-segment digits. Segments: a top, b upper right, c
   lower right, d bottom, e lower left, f upper left, g middle. */
static const u8 segments[10]={0x3f,0x06,0x5b,0x4f,0x66,0x6d,0x7d,0x07,0x7f,0x6f};
/* Cover mask row y from x0 to x1 (Q8 cells), keeping the larger coverage. */
static void span(u8 *row,u32 width,int x0,int x1) {
    int i;
    if(x0<0) x0=0;
    if(x1>(int)width<<8) x1=(int)width<<8;
    for(i=x0>>8;i<<8<x1;++i) {
        int a=i<<8,b=a+256,from=x0>a ? x0 : a,to=x1<b ? x1 : b;
        u32 cover=(u32)(to-from)*255>>8;
        if(to>from && cover>row[i]) row[i]=(u8)cover;
    }
}
/* A sheared bar: rows y0..y1 of the digit, x0..x1 (Q8) at its baseline. Upper
   rows lean right by 0.12 cell per row. */
static void bar(u8 *mask,u32 width,int x,u32 top,u32 m,int x0,int x1,u32 y0,u32 y1) {
    u32 y;
    for(y=y0;y<y1;++y) {
        int lean=(int)((m-y)*31);
        span(mask+(top+y)*width,width,x+x0+lean,x+x1+lean);
    }
}
static void classic_glyph(u8 *mask,u32 width,char c,int x,u32 top,u32 m) {
    int w=(int)((m*36045)>>8), t=(int)((m*8520)>>8), g=(int)((m*1966)>>8);
    u32 rows=(u32)t>>8, half=m>>1, bits;
    if(!rows) rows=1;
    if(c=='\'') { bar(mask,width,x,top,m,(int)(m*13),(int)(m*13)+t,0,(m*19661)>>16); return; }
    if(c==':') {
        bar(mask,width,x,top,m,0,t,(m*19661)>>16,((m*19661)>>16)+rows);
        bar(mask,width,x,top,m,0,t,(m*45875)>>16,((m*45875)>>16)+rows);
        return;
    }
    if(c<'0' || c>'9') return;
    bits=segments[c-'0'];
    if(bits&1) bar(mask,width,x,top,m,t+g,w-t-g,0,rows);
    if(bits&64) bar(mask,width,x,top,m,t+g,w-t-g,half-(rows>>1),half-(rows>>1)+rows);
    if(bits&8) bar(mask,width,x,top,m,t+g,w-t-g,m-rows,m);
    if(bits&32) bar(mask,width,x,top,m,0,t,(rows>>1)+1,half-1);
    if(bits&2) bar(mask,width,x,top,m,w-t,w,(rows>>1)+1,half-1);
    if(bits&16) bar(mask,width,x,top,m,0,t,half+1,m-(rows>>1)-1);
    if(bits&4) bar(mask,width,x,top,m,w-t,w,half+1,m-(rows>>1)-1);
}

/* GR1s: 5 x 7 dot-matrix characters, as the data back of the GR1s prints
   them: digits, then the apostrophe and the colon. One byte for each of the 7
   rows, bit 4 the left column. The zero is slashed. The apostrophe fills the
   right 2 columns of its cell, and the colon the left 2. */
static const u8 dots[12][7]={
    {0x0e,0x11,0x13,0x15,0x19,0x11,0x0e},{0x04,0x0c,0x04,0x04,0x04,0x04,0x0e},
    {0x0e,0x11,0x01,0x02,0x04,0x08,0x1f},{0x1f,0x02,0x04,0x02,0x01,0x11,0x0e},
    {0x02,0x06,0x0a,0x12,0x1f,0x02,0x02},{0x1f,0x10,0x1e,0x01,0x01,0x11,0x0e},
    {0x06,0x08,0x10,0x1e,0x11,0x11,0x0e},{0x1f,0x01,0x02,0x04,0x08,0x08,0x08},
    {0x0e,0x11,0x11,0x0e,0x11,0x11,0x0e},{0x0e,0x11,0x11,0x0f,0x01,0x02,0x0c},
    {0x03,0x01,0x02,0x00,0x00,0x00,0x00},{0x00,0x18,0x18,0x00,0x18,0x18,0x00}
};
/* A blank digit cell: the GR1s look pads one-digit fields with it. */
#define BLANK_DIGIT '_'
/* Row pitch (Q8 cells) for digit height m: 7 rows span m. As on GR1s prints,
   columns are 0.765 of a row apart, and a dot is a short dash: 0.9 of a column
   wide and 0.57 of a row high. The dots of a row nearly touch, and the rows
   stay apart. */
static int pitch_row(u32 m) { return (int)((m*9362)>>8); }
static int pitch_column(u32 m) { return (pitch_row(m)*196)>>8; }
/* Cover a dot of half width rx and half height ry around (cx,cy), all Q8
   cells, with 4 x 4 samples per cell, keeping the larger coverage. */
static void dot(u8 *mask,u32 width,u32 height,int cx,int cy,int rx,int ry) {
    int x, y, s;
    for(y=(cy-ry)>>8;y<=(cy+ry)>>8;++y) for(x=(cx-rx)>>8;x<=(cx+rx)>>8;++x) {
        u32 inside=0, cover;
        if(x<0 || y<0 || x>=(int)width || y>=(int)height) continue;
        for(s=0;s<16;++s) {
            int dx=(x<<8)+((s&3)<<6)+32-cx, dy=(y<<8)+((s>>2)<<6)+32-cy;
            inside+=dx>=-rx && dx<=rx && dy>=-ry && dy<=ry;
        }
        cover=(inside*255)>>4;
        if(cover>mask[y*width+x]) mask[y*width+x]=(u8)cover;
    }
}
static void dots_glyph(u8 *mask,u32 width,u32 height,char c,int x,u32 top,u32 m) {
    int row=pitch_row(m), column=pitch_column(m), rx=(column*115)>>8, ry=(row*73)>>8;
    u32 i, j, k=c=='\'' ? 10 : c==':' ? 11 : (u32)(c-'0');
    if(k>11) return;
    for(j=0;j<7;++j) for(i=0;i<5;++i)
        if(dots[k][j]&(0x10>>i))
            dot(mask,width,height,x+(int)i*column+(column>>1),(int)(top<<8)+(int)j*row+(row>>1),rx,ry);
}

/* The digit height of a mask look as a fraction of the short side (Q16), the
   smallest height in pixels, and the halo blur radius in cells. Classic digits
   are 4.5% of the short side. GR1s rows span 3.17% of the short side, so that
   its dots span 2.98% as on its prints, with a halo of about half a dot row so
   the rows stay apart. */
static u32 look_size(u32 look) { return look==LOOK_DOTS ? 2080 : 2949; }
static u32 look_least(u32 look) { return look==LOOK_DOTS ? 14 : 8; }
static u32 look_halo(u32 m,u32 look) {
    u32 r=look==LOOK_DOTS ? (m*2)>>5 : (m*5)>>5;
    return r ? r : 1;
}
/* The space between the text and the right edge (`right`) and the bottom
   edge (`bottom`) of the visible area. Classic: 0.7 digit heights on both
   sides. GR1s: about 19% and 8.3% of the short side, where its prints show the
   date. */
static void look_margins(u32 look,u32 short_side,u32 size,u32 *right,u32 *bottom) {
    if(look==LOOK_DOTS) {
        *right=(short_side*12452)>>16;
        *bottom=(short_side*5440)>>16;
        return;
    }
    *right=*bottom=(size*45875)>>16;
}
/* Mask advance of one character in Q8 cells. Classic: whole cells, as 1.1
   drew them. GR1s, as on its prints: a digit is 5 columns and 2 empty ones, a
   space 1.8 columns, the apostrophe 6.2 columns, and the colon 4. */
static int advance(char c,u32 m,u32 look) {
    if(look==LOOK_DOTS) {
        int column=pitch_column(m);
        if((c>='0' && c<='9') || c==BLANK_DIGIT) return column*7;
        if(c=='\'') return (column*1587)>>8;
        return c==':' ? column*4 : (column*461)>>8;
    }
    if(c>='0' && c<='9') return (int)((m*54395)>>16)<<8;
    return (int)(c==' ' ? (m*22938)>>16 : (m*19661)>>16)<<8;
}
/* The width of the text in cells, without the gap after its last digit, and
   with the lean of the classic digits. Every style ends in a digit. */
static u32 text_width(const char *text,u32 n,u32 m,u32 look) {
    u32 i;
    int total=0;
    for(i=0;i<n;++i) total+=advance(text[i],m,look);
    if(look==LOOK_DOTS) return (u32)((total-2*pitch_column(m)+255)>>8);
    return (u32)(total>>8)-((m*18000)>>16)+((m*31)>>8);
}
static void glyph(u8 *mask,u32 width,u32 height,char c,int x,u32 top,u32 m,u32 look) {
    if(look==LOOK_DOTS) dots_glyph(mask,width,height,c,x,top,m);
    else classic_glyph(mask,width,c,x,top,m);
}

/* GR II: the 1-bit digits of the GR II date imprint, 16 x 24 cells, rows 3 to
   20 (the others are empty), bit 15 the left column. Order: slash, colon,
   0 to 9. */
static const u16 gr2_glyphs[12][18]={
    {0,0,0,0x000c,0x001c,0x0038,0x0070,0x00e0,0x01c0,0x0380,0x0700,0x0e00,0x1c00,0x3800,0x3000,0,0,0},
    {0,0,0,0,0x0300,0x0300,0,0,0,0,0,0,0x0300,0x0300,0,0,0,0},
    {0x03c0,0x07e0,0x0e70,0x1c38,0x1818,0x1818,0x1818,0x1818,0x1818,0x1818,0x1818,0x1818,0x1818,0x1818,0x1c38,0x0e70,0x07e0,0x03c0},
    {0x0180,0x0380,0x0780,0x0f80,0x0180,0x0180,0x0180,0x0180,0x0180,0x0180,0x0180,0x0180,0x0180,0x0180,0x0180,0x0180,0x0ff0,0x0ff0},
    {0x03c0,0x07e0,0x0e70,0x1c38,0x1818,0x0018,0x0018,0x0038,0x0070,0x00e0,0x01c0,0x0380,0x0700,0x0e00,0x1c00,0x1800,0x1ff8,0x1ff8},
    {0x03c0,0x07e0,0x0e70,0x1c38,0x1818,0x0018,0x0038,0x0070,0x07e0,0x07e0,0x0070,0x0038,0x0018,0x1818,0x1c38,0x0e70,0x07e0,0x03c0},
    {0x0060,0x00e0,0x00e0,0x01e0,0x01e0,0x0360,0x0360,0x0660,0x0660,0x0c60,0x0c60,0x1860,0x1ff8,0x1ff8,0x0060,0x0060,0x0060,0x0060},
    {0x1ff8,0x1ff8,0x1800,0x1800,0x1800,0x1800,0x1fc0,0x1fe0,0x0070,0x0038,0x0018,0x0018,0x0018,0x0018,0x1838,0x1c70,0x0fe0,0x07c0},
    {0x03c0,0x07e0,0x0e70,0x1c38,0x1818,0x1800,0x1800,0x1800,0x1fe0,0x1ff0,0x1838,0x1818,0x1818,0x1818,0x1c38,0x0e70,0x07e0,0x03c0},
    {0x1ff8,0x1ff8,0x1818,0x0030,0x0030,0x0060,0x0060,0x00c0,0x00c0,0x0180,0x0180,0x0300,0x0300,0x0300,0x0300,0x0300,0x0300,0x0300},
    {0x03c0,0x07e0,0x0e70,0x1c38,0x1818,0x1818,0x1c38,0x0e70,0x07e0,0x07e0,0x0e70,0x1c38,0x1818,0x1818,0x1818,0x1c38,0x0ff0,0x07e0},
    {0x03c0,0x07e0,0x0e70,0x1c38,0x1818,0x1818,0x1818,0x1c18,0x0ff8,0x07f8,0x0018,0x0018,0x0018,0x1818,0x1c38,0x0e70,0x07e0,0x03c0}
};
/* Digits without leading zeros, as a data back prints them. */
static char *number(char *o,u32 v) {
    char d[4];
    u32 n=0;
    do { u32 q=0; while(v>=10) { v-=10; ++q; } d[n++]=(char)('0'+v); v=q; } while(v && n<4);
    while(n) *o++=d[--n];
    return o;
}
static char *two(char *o,u32 v) {
    u32 tens=0;
    while(v>=10) { v-=10; ++tens; }
    *o++=(char)('0'+tens); *o++=(char)('0'+v);
    return o;
}
/* Month, day, or hour. With `pad`, a one-digit value takes a blank digit cell
   first, as the GR1s prints it. */
static char *field(char *o,u32 v,u32 pad) {
    if(pad && v<10) *o++=BLANK_DIGIT;
    return number(o,v);
}
/* Format one style: 1 'YY M D, 2 'YY M D H:MM, 3 M D 'YY, 4 D M 'YY,
   5 D M 'YY H:MM. Two spaces come before the time. */
static u32 date_text(char *o,const u8 *dt,u32 style,u32 pad) {
    char *start=o;
    u32 year=dt[0]|dt[1]<<8, short_year=year>=2000 ? year-2000 : 0;
    while(short_year>=100) short_year-=100;
    if(style<=2) {
        *o++='\''; o=two(o,short_year); *o++=' '; o=field(o,dt[2],pad); *o++=' '; o=field(o,dt[3],pad);
    } else {
        o=field(o,style==3 ? dt[2] : dt[3],pad); *o++=' '; o=field(o,style==3 ? dt[3] : dt[2],pad);
        *o++=' '; *o++='\''; o=two(o,short_year);
    }
    if(style==2 || style==5) { *o++=' '; *o++=' '; o=field(o,dt[4],pad); *o++=':'; o=two(o,dt[5]); }
    return (u32)(o-start);
}
static u32 clamp255(int v) { return v<0 ? 0 : v>255 ? 255 : (u32)v; }
/* Screen blend of light l over value v: v + (255 - v) * l / 255. */
static u32 screen(u32 v,u32 l) { return v+((((255-v)*l)*257+32768)>>16); }
/* GR1s digits on color film: mostly opaque yellow (RGB 245, 235, 100) with a
   yellow glow, as its color negatives show them. */
#define DOTS_R 245
#define DOTS_G 235
#define DOTS_B 100
/* Add the stamp light to one pixel pair that shares Cb and Cr. */
static void light_pair(const Plane *p,u32 x,u32 y,const u32 *core,const u32 *halo,u32 mono,u32 chroma,u32 look) {
    /* A picture byte store may alias the plane and the light, so the pair
       reads them into locals first. A stamp lights thousands of pairs. */
    volatile u8 *luma=GRKIT_BYTES(p->y+y*p->pitch+x), *crow=GRKIT_BYTES(chroma_row(p,y)+x);
    const u32 k[2]={core[0],core[1]}, h[2]={halo[0],halo[1]};
    u32 i,yv[2];
    int cb=(int)crow[0]-128, cr=(int)crow[1]-128, sb=0, sr=0;
    /* The chroma terms of the pair, for both pixels. */
    int rc=(91881*cr)>>16, gc=(22554*cb+46802*cr)>>16, bc=(116130*cb)>>16;
    for(i=0;i<2;++i) {
        u32 lr=k[i]+((h[i]*180)>>8), lg, lb, v=luma[i], r, g, b;
        if(lr>255) lr=255;
        /* White light: the red channel holds the full core and halo. */
        if(mono) { yv[i]=screen(v,lr); continue; }
        if(look==LOOK_DOTS) { lg=((k[i]*235)>>8)+((h[i]*150)>>8); lb=((k[i]*100)>>8)+((h[i]*20)>>8); }
        else { lg=((k[i]*150)>>8)+((h[i]*40)>>8); lb=(k[i]*40)>>8; }
        r=screen(clamp255((int)v+rc),lr);
        g=screen(clamp255((int)v-gc),lg);
        b=screen(clamp255((int)v+bc),lb);
        if(look==LOOK_DOTS) {
            u32 a=(k[i]*210)>>8;
            r=(r*(256-a)+DOTS_R*a)>>8; g=(g*(256-a)+DOTS_G*a)>>8; b=(b*(256-a)+DOTS_B*a)>>8;
        }
        yv[i]=(19595*r+38470*g+7471*b+32768)>>16;
        sb+=(int)(-11059*(int)r-21709*(int)g+32768*(int)b);
        sr+=(int)(32768*(int)r-27439*(int)g-5329*(int)b);
    }
    luma[0]=(u8)yv[0];
    luma[1]=(u8)yv[1];
    if(!mono && chroma) {
        crow[0]=(u8)clamp255(128+((sb+65536)>>17));
        crow[1]=(u8)clamp255(128+((sr+65536)>>17));
    }
}
/* The horizontal blend of one mask row for each of `cols` picture columns,
   core then halo: column x samples cells x / f and the next one, with the
   weight (x mod f) * step. */
static void blend_row(u8 *out,const u8 *core,const u8 *halo,u32 mw,u32 f,u32 step,u32 cols) {
    u32 x, cx=0, fx=0;
    for(x=0;x<cols;++x) {
        u32 at=cx<mw ? cx : mw-1, next=at+1<mw ? at+1 : at, tx=fx*step;
        out[x]=(u8)((core[at]*(65536-tx)+core[next]*tx)>>16);
        out[cols+x]=(u8)((halo[at]*(65536-tx)+halo[next]*tx)>>16);
        if(++fx==f) { fx=0; ++cx; }
    }
}
/* The scratch bytes of a stamp: the core and halo masks, then for a mask
   that scales up (ss 1), the blends of two rows of each (blend_row). */
static u32 stamp_bytes(u32 mw,u32 mh,u32 f,u32 ss) { return mw*mh*2+(ss==1 ? 4*mw*f : 0); }
/* A rendered stamp mask and where it lies on its picture: one cell spans
   f / ss pixels, and cell (0,0) is at (ox, oy) of the visible area turned
   upright (upright), whose top left is (0, 0). */
typedef struct { const u8 *core,*halo; u32 mw,mh,f,ss; int ox,oy; u32 look; } Stamp;
/* The mask of a stamp at the bottom right of an upright visible area of
   width x height pixels (look_size), in `scratch`. Small pictures (the
   screennail that playback shows first, and the thumbnail) render the mask at
   twice their resolution and average 2 x 2 cells per pixel, so their strokes
   stay as sharp as on the main picture: there, one mask cell spans several
   pixels instead. Returns 0 when the mask does not fit. */
static u32 stamp_mask(u8 *scratch,u32 width,u32 height,const char *text,u32 n,u32 look,Stamp *out) {
    u32 size, f, ss=1, m, pad, cw, mw, mh, margin, rise;
    u32 i, x, y, total;
    int caret;
    u8 *core=scratch, *halo;
    size=((width<height ? width : height)*look_size(look))>>16;
    if(size<look_least(look)) size=look_least(look);
    for(i=0;i<2;++i) {
        u32 cells=i ? STAMP_CELLS_LONG : STAMP_CELLS;
        f=size<=cells ? 1 : divide(size+cells-1,cells);
        ss=f==1 && 2*size<=cells ? 2 : 1;
        m=ss==2 ? 2*size : divide(size+f/2,f);
        pad=look_halo(m,look)*2+2;
        total=text_width(text,n,m,look);
        cw=total+1;
        mw=cw+2*pad; mh=m+2*pad;
        if(mw<=BLUR_WIDTH && stamp_bytes(mw,mh,f,ss)<=GRKIT_PLANE_SCRATCH) break;
    }
    if(mw>BLUR_WIDTH || mh>BLUR_WIDTH || stamp_bytes(mw,mh,f,ss)>GRKIT_PLANE_SCRATCH) return 0;
    look_margins(look,width<height ? width : height,size,&margin,&rise);
    if((((mw-pad)*f)>>(ss-1))+margin>width || (((mh-pad)*f)>>(ss-1))+rise>height) return 0;
    halo=core+mw*mh;
    for(i=0;i<mw*mh;++i) core[i]=0;
    for(i=0,caret=(int)pad<<8;i<n;++i) { glyph(core,mw,mh,text[i],caret,pad,m,look); caret+=advance(text[i],m,look); }
    for(i=0;i<mw*mh;++i) halo[i]=core[i];
    /* Classic strokes soften with a core blur. The dots are anti-aliased. */
    for(y=0;y<mh;++y) {
        if(look!=LOOK_DOTS) blur_line(core+y*mw,1,mw,1);
        blur_line(halo+y*mw,1,mw,look_halo(m,look));
    }
    for(x=0;x<mw;++x) {
        if(look!=LOOK_DOTS) blur_line(core+x,mw,mh,1);
        blur_line(halo+x,mw,mh,look_halo(m,look));
    }
    *out=(Stamp){core,halo,mw,mh,f,ss,(int)width-(int)margin-(int)(((mw-pad)*f)>>(ss-1)),
                 (int)height-(int)rise-(int)(((mh-pad)*f)>>(ss-1)),look};
    return 1;
}
/* The stamp on the main picture of a landscape shot, in the layout of its
   mask. Returns 1 and describes the mask in `out` when it drew. */
static u32 stamp(u8 *scratch,const Plane *p,const char *text,u32 n,u32 mono,u32 look,Stamp *out) {
    u32 x, y, sy, dy, step, mw, mh, f, ss;
    const u8 *core, *halo;
    u8 lit[BLUR_WIDTH];
    if(!stamp_mask(scratch,p->right-p->left,p->bottom-p->top,text,n,look,out)) return 0;
    core=out->core; halo=out->halo; mw=out->mw; mh=out->mh; f=out->f; ss=out->ss;
    /* Whether each mask row has light, in the core or in the halo. A pixel
       that samples only empty rows, or only empty cells, stays as it is. */
    for(y=0;y<mh;++y) {
        const u8 *c=core+y*mw, *h=halo+y*mw;
        u32 any=0;
        for(x=0;x<mw && !any;++x) any=c[x]|h[x];
        lit[y]=(u8)(any!=0);
    }
    if(ss==2) {
        /* Pixel origin of the mask, on an even column so CbCr pairs line up. */
        int ox=(int)p->left+out->ox, oy=(int)p->top+out->oy;
        ox&=~1;
        out->ox=ox-(int)p->left;
        for(y=0;2*y+1<mh;++y) {
            int py=oy+(int)y;
            u32 chroma=!p->half || !(py&1);
            if(py<(int)p->top || py>=(int)p->bottom || !(lit[2*y] | lit[2*y+1])) continue;
            for(x=0;2*x+3<mw;x+=2) {
                u32 c[2],l[2],k,any=0;
                for(k=0;k<2;++k) {
                    u32 at=2*y*mw+2*(x+k);
                    c[k]=(core[at]+core[at+1]+core[at+mw]+core[at+mw+1]+2)>>2;
                    c[k]=(c[k]*320)>>8; if(c[k]>255) c[k]=255;
                    l[k]=(halo[at]+halo[at+1]+halo[at+mw]+halo[at+mw+1]+2)>>2;
                    any|=c[k]|l[k];
                }
                {
                    int px=ox+(int)x;
                    if(any && px>=(int)p->left && px+1<(int)p->right) light_pair(p,(u32)px,(u32)py,c,l,mono,chroma,look);
                }
            }
        }
        return 1;
    }
    /* Pixel origin of the mask, on an even column so CbCr pairs line up. */
    {
        int ox=(int)p->left+out->ox, oy=(int)p->top+out->oy;
        u32 cols=mw*f;
        /* The horizontal blends of the two mask rows that a picture row
           samples, for each column: f picture rows sample the same pair, so
           each row of the pair blends once (`top`, `bottom`). They follow the
           masks in the scratch. */
        u8 *top=(u8 *)halo+mw*mh, *bottom=top+2*cols;
        u32 shown=0xffffffffU;
        ox&=~1;
        out->ox=ox-(int)p->left;
        step=divide(65536,f);
        for(y=0,sy=0,dy=0;y<mh*f;++y) {
            int py=oy+(int)y;
            u32 ty=dy*step, sy1=sy+1<mh ? sy+1 : sy, chroma=!p->half || !(py&1);
            if(py>=(int)p->top && py<(int)p->bottom && (lit[sy] || lit[sy1])) {
                if(shown!=sy) {
                    blend_row(top,core+sy*mw,halo+sy*mw,mw,f,step,cols);
                    blend_row(bottom,core+sy1*mw,halo+sy1*mw,mw,f,step,cols);
                    shown=sy;
                }
                for(x=0;x<cols;x+=2) {
                    u32 c[2],l[2],k,any=0;
                    for(k=0;k<2 && x+k<cols;++k) {
                        u32 ct=top[x+k], cb=bottom[x+k], ht=top[cols+x+k], hb=bottom[cols+x+k];
                        if(!(ct|cb|ht|hb)) { c[k]=l[k]=0; continue; }
                        c[k]=(ct*(65536-ty)+cb*ty)>>16;
                        /* The blur lowers the peak of thin strokes: an LED core saturates. */
                        c[k]=(c[k]*320)>>8; if(c[k]>255) c[k]=255;
                        l[k]=(ht*(65536-ty)+hb*ty)>>16;
                        any|=c[k]|l[k];
                    }
                    if(k<2) c[1]=l[1]=0;
                    {
                        int px=ox+(int)x;
                        if(any && px>=(int)p->left && px+1<(int)p->right) light_pair(p,(u32)px,(u32)py,c,l,mono,chroma,look);
                    }
                }
            }
            if(++dy==f) { dy=0; ++sy; }
        }
    }
    return 1;
}
/* The upright point (Q16) of a point (x, y) (Q16) of the visible area of `p`,
   for a picture that `turns` quarter turns clockwise show upright: the
   visible area turned upright, with its top left at (0, 0). */
static void upright(const Plane *p,u32 turns,long long x,long long y,long long *u,long long *v) {
    long long w=(long long)(p->right-p->left)<<16, h=(long long)(p->bottom-p->top)<<16;
    x-=(long long)p->left<<16;
    y-=(long long)p->top<<16;
    switch(turns&3) {
    case 0: *u=x; *v=y; break;
    case 1: *u=h-y; *v=x; break;
    case 2: *u=w-x; *v=h-y; break;
    default: *u=y; *v=w-x; break;
    }
}
/* The box of `p` (left, top, right, bottom) that shows the upright box
   [u0, u1) x [v0, v1) (upright). */
static void stored_box(const Plane *p,u32 turns,int u0,int v0,int u1,int v1,int *box) {
    int l=(int)p->left, t=(int)p->top, w=(int)(p->right-p->left), h=(int)(p->bottom-p->top);
    switch(turns&3) {
    case 0: box[0]=l+u0; box[1]=t+v0; box[2]=l+u1; box[3]=t+v1; break;
    case 1: box[0]=l+v0; box[1]=t+h-u1; box[2]=l+v1; box[3]=t+h-u0; break;
    case 2: box[0]=l+w-u1; box[1]=t+h-v1; box[2]=l+w-u0; box[3]=t+h-v0; break;
    default: box[0]=l+w-v1; box[1]=t+u0; box[2]=l+w-v0; box[3]=t+u1; break;
    }
}
/* Bilinear mask value at cell coordinates (Q16, cell centers at +0.5), or 0 outside. */
static u32 mask_at(const u8 *mask,u32 mw,u32 mh,int cx,int cy) {
    int x=cx-32768, y=cy-32768, i=x>>16, j=y>>16;
    u32 tx=(u32)x&0xffff, ty=(u32)y&0xffff, a, b, c, d;
    if(i<-1 || j<-1 || i>=(int)mw || j>=(int)mh) return 0;
    a=i>=0 && j>=0 ? mask[j*mw+i] : 0;
    b=i+1<(int)mw && j>=0 ? mask[j*mw+i+1] : 0;
    c=i>=0 && j+1<(int)mh ? mask[(j+1)*mw+i] : 0;
    d=i+1<(int)mw && j+1<(int)mh ? mask[(j+1)*mw+i+1] : 0;
    return (((a*(65536-tx)+b*tx)>>16)*(65536-ty)+((c*(65536-tx)+d*tx)>>16)*ty)>>16;
}
/* The stamp of the main picture `src` on the picture `dst`: each pixel of
   dst samples the mask at four points, so the stamp keeps the same place and
   the same total width as on the main picture, only softer on a smaller
   picture. A picture that `turns` quarter turns show upright gets the stamp
   at the bottom right of its upright view, turned with it. */
static void stamp_sample(const Plane *dst,const Plane *src,const Stamp *st,u32 mono,u32 turns) {
    u32 sw=src->right-src->left, sh=src->bottom-src->top, dw=dst->right-dst->left, dh=dst->bottom-dst->top;
    u32 kx=divide(sw<<16,dw), ky=divide(sh<<16,dh), rx=divide(dw<<16,sw), ry=divide(dh<<16,sh);
    u32 rf=divide(65536,st->f), x, y, k;
    /* Q16 cell coordinate of a Q16 upright coordinate. */
    #define CELL(v,o) ((int)(((long long)((v)-((long long)(o)<<16))*(long long)(st->ss*rf))>>16))
    int ext=(int)((st->mw*st->f)>>(st->ss-1)), eyt=(int)((st->mh*st->f)>>(st->ss-1)), box[4];
    int x0, x1, y0, y1;
    stored_box(src,turns,st->ox,st->oy,st->ox+ext,st->oy+eyt,box);
    x0=(int)dst->left+(int)((((long long)(box[0]-(int)src->left))*rx)>>16)-2;
    x1=(int)dst->left+(int)((((long long)(box[2]-(int)src->left))*rx)>>16)+2;
    y0=(int)dst->top+(int)((((long long)(box[1]-(int)src->top))*ry)>>16)-2;
    y1=(int)dst->top+(int)((((long long)(box[3]-(int)src->top))*ry)>>16)+2;
    if(x0<(int)dst->left) x0=(int)dst->left;
    if(y0<(int)dst->top) y0=(int)dst->top;
    if(x1>(int)dst->right) x1=(int)dst->right;
    if(y1>(int)dst->bottom) y1=(int)dst->bottom;
    x0&=~1;
    for(y=(u32)y0;(int)y<y1;++y) {
        u32 chroma=!dst->half || !(y&1);
        for(x=(u32)x0;(int)x+1<x1;x+=2) {
            u32 c[2],l[2],any=0;
            for(k=0;k<2;++k) {
                long long sx=((long long)src->left<<16)+(long long)(x+k-dst->left)*kx+(kx>>1);
                long long sy=((long long)src->top<<16)+(long long)(y-dst->top)*ky+(ky>>1);
                u32 core=0, halo=0, q;
                for(q=0;q<4;++q) {
                    long long px=sx+(q&1 ? (long long)(kx>>2) : -(long long)(kx>>2));
                    long long py=sy+(q&2 ? (long long)(ky>>2) : -(long long)(ky>>2));
                    long long u, v;
                    int cx, cy;
                    upright(src,turns,px,py,&u,&v);
                    cx=CELL(u,st->ox); cy=CELL(v,st->oy);
                    core+=mask_at(st->core,st->mw,st->mh,cx,cy);
                    halo+=mask_at(st->halo,st->mw,st->mh,cx,cy);
                }
                c[k]=((core>>2)*320)>>8; if(c[k]>255) c[k]=255;
                l[k]=halo>>2;
                any|=c[k]|l[k];
            }
            if(any && x+1<dst->right) light_pair(dst,x,y,c,l,mono,chroma,st->look);
        }
    }
    #undef CELL
}

/* GR II: the date in the order of the style, with slashes and a four-digit
   year. Month, day, and hour are two characters with a leading space, as the
   GR II prints them (%4d/%2d/%2d %2d:%02d). */
static char *spaced(char *o,u32 v) {
    if(v<10) *o++=' ';
    return number(o,v);
}
static u32 gr2_text(char *o,const u8 *dt,u32 style) {
    char *start=o;
    u32 year=dt[0]|dt[1]<<8;
    if(style<=2) { o=number(o,year); *o++='/'; o=spaced(o,dt[2]); *o++='/'; o=spaced(o,dt[3]); }
    else {
        o=spaced(o,style==3 ? dt[2] : dt[3]); *o++='/'; o=spaced(o,style==3 ? dt[3] : dt[2]);
        *o++='/'; o=number(o,year);
    }
    if(style==2 || style==5) { *o++=' '; o=spaced(o,dt[4]); *o++=':'; o=two(o,dt[5]); }
    return (u32)(o-start);
}
/* Whether GR II glyph pixel (x,y) of the text is ink: x across the text, y in
   the 24 rows of a cell. Spaces and unknown characters are empty. */
static u32 gr2_ink(const char *text,u32 n,u32 x,u32 y) {
    u32 i=x>>4, k;
    char c;
    if(i>=n || y<3 || y>20) return 0;
    c=text[i];
    k=c=='/' ? 0 : c==':' ? 1 : c>='0' && c<='9' ? (u32)(c-'0')+2 : 12;
    return k<12 && (gr2_glyphs[k][y-3]>>(15-(x&15))&1);
}
/* The GR II imprint, painted on `dst` with the geometry of the main picture
   `src`. A glyph pixel spans short side / GR2_SHORT pixels of the main picture
   (Q16 `s`), so the digits keep the size of the GR II imprint on its 3264-row
   pictures, where a glyph pixel is 3 x 3 pixels: 18 rows of digits are about
   1.65% of the short side. A picture with at most GR2_SHORT rows gets 1 x 1
   pixels. The text ends 3 cells before the right edge, and the cells end 48
   glyph pixels above the bottom. The pixels get flat orange (Y 146, Cb 46,
   Cr 225), or white luma on a monochrome picture. Each picture samples the
   geometry at 2 x 2 points for each pixel. */
#define GR2_SHORT 1088U
#define GR2_Y 146
#define GR2_CB 46
#define GR2_CR 225
static void gr2_paint(const Plane *dst,const Plane *src,const char *text,u32 n,u32 mono,u32 turns) {
    u32 sw=src->right-src->left, sh=src->bottom-src->top, dw=dst->right-dst->left, dh=dst->bottom-dst->top;
    u32 short_side=sw<sh ? sw : sh, s=short_side>GR2_SHORT ? divide(short_side<<16,GR2_SHORT) : 65536;
    u32 kx, ky, rs, x, y, k, q;
    int bx, by, x0, x1, y0, y1, box[4];
    /* The text box in the upright view (upright). */
    bx=(int)(turns&1 ? sh : sw)-(int)((16*(n+3)*s)>>16); by=(int)(turns&1 ? sw : sh)-(int)((72*s)>>16);
    if(bx<0 || by<0) return;
    kx=divide(sw<<16,dw); ky=divide(sh<<16,dh); rs=divide(0xffffffffU,s)+1;
    /* The box of the ink rows in dst pixels. */
    stored_box(src,turns,bx,by+(int)((3*s)>>16),bx+(int)((16*n*s)>>16),by+(int)((21*s)>>16),box);
    x0=(int)dst->left+(int)divide((u32)(box[0]-(int)src->left)<<8,(kx+128)>>8)-1;
    x1=(int)dst->left+(int)divide((u32)(box[2]-(int)src->left)<<8,(kx+128)>>8)+2;
    y0=(int)dst->top+(int)divide((u32)(box[1]-(int)src->top)<<8,(ky+128)>>8)-1;
    y1=(int)dst->top+(int)divide((u32)(box[3]-(int)src->top)<<8,(ky+128)>>8)+2;
    if(x0<(int)dst->left) x0=(int)dst->left;
    if(y0<(int)dst->top) y0=(int)dst->top;
    if(x1>(int)dst->right) x1=(int)dst->right;
    if(y1>(int)dst->bottom) y1=(int)dst->bottom;
    x0&=~1;
    for(y=(u32)y0;(int)y<y1;++y) {
        u32 crow=chroma_row(dst,y), chroma=!dst->half || !(y&1);
        for(x=(u32)x0;(int)x+1<x1 && x+1<dst->right;x+=2) {
            u32 cover[2], most=0;
            for(k=0;k<2;++k) {
                cover[k]=0;
                for(q=0;q<4;++q) {
                    /* Q16 source pixel of a sample, then its glyph pixel. */
                    u32 px=(src->left<<16)+(x+k-dst->left)*kx+(q&1 ? (3*kx)>>2 : kx>>2);
                    u32 py=(src->top<<16)+(y-dst->top)*ky+(q&2 ? (3*ky)>>2 : ky>>2);
                    long long u, v;
                    int gx, gy;
                    upright(src,turns,px,py,&u,&v);
                    gx=(int)(u>>16)-bx; gy=(int)(v>>16)-by;
                    if(gx>=0 && gy>=0) cover[k]+=gr2_ink(text,n,((u32)gx*rs)>>16,((u32)gy*rs)>>16);
                }
                if(cover[k]>most) most=cover[k];
            }
            if(!most) continue;
            for(k=0;k<2;++k) {
                u32 at=y*dst->pitch+x+k, v=BYTE(dst->y,at), target=mono ? 255 : GR2_Y;
                BYTE(dst->y,at)=(u8)((int)v+(((int)target-(int)v)*(int)cover[k])/4);
            }
            if(!mono && chroma) {
                u32 cb=BYTE(crow,x), cr=BYTE(crow,x+1);
                BYTE(crow,x)=(u8)((int)cb+((GR2_CB-(int)cb)*(int)most)/4);
                BYTE(crow,x+1)=(u8)((int)cr+((GR2_CR-(int)cr)*(int)most)/4);
            }
        }
    }
}
/* The largest picture draws the stamp. The smaller ones scale its mask, so
   playback keeps the stamp's place and width when it switches from the
   screennail to the main picture. The core calls this only for a style,
   not for Off. */
void date_apply(PictureSet *set, u32 style)
{
    GrkitDateTime now;
    const u8 *dt = (const u8 *)&now;
    u32 n, i, main = 0, look = setting_get(SETTING_LOOK);
    /* Upright: the stamp follows the posture of the camera. Film: it stays
       at the bottom right of the sensor frame, as on film. */
    u32 turns = setting_get(SETTING_POSITION) ? set->turns & 3 : 0;
    char text[20];
    Stamp st;
    local_time(&now);
    for (i = 1; i < set->count; ++i)
        if (set->planes[i].width * set->planes[i].height > set->planes[main].width * set->planes[main].height) main = i;
    if (look == LOOK_GR2) {
        n = gr2_text(text, dt, style);
        for (i = 0; i < set->count; ++i) gr2_paint(&set->planes[i], &set->planes[main], text, n, set->mono, turns);
        return;
    }
    n = date_text(text, dt, style, look == LOOK_DOTS);
    if (!turns) {
        if (stamp(set->scratch, &set->planes[main], text, n, set->mono, look, &st))
            for (i = 0; i < set->count; ++i)
                if (i != main) stamp_sample(&set->planes[i], &set->planes[main], &st, set->mono, 0);
        return;
    }
    /* A turned picture samples the mask on each picture, the main one too. */
    {
        const Plane *p = &set->planes[main];
        u32 w = p->right - p->left, h = p->bottom - p->top;
        if (stamp_mask(set->scratch, turns & 1 ? h : w, turns & 1 ? w : h, text, n, look, &st))
            for (i = 0; i < set->count; ++i) stamp_sample(&set->planes[i], p, &st, set->mono, turns);
    }
}
