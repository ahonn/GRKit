#ifndef GRKIT_CANVAS_H
#define GRKIT_CANVAS_H
/* The canvas of CORE.BORDER: a developed picture larger than the shot, with
   the shot at its own size inside and the border around it. The firmware
   develops the shot into the canvas, and the JPEG takes the whole canvas.
   This part is the geometry; the firmware side is in runtime/picture.c.

   A border value of the shot ring (GRKIT_SHOT_BORDER) is the style in bits
   0 to 7 (CORE.BORDER) and the color in bits 8 to 15 (CORE.BORDER_COLOR). */

#define CANVAS_OFF 0U
#define CANVAS_THIN 1U
#define CANVAS_WIDE 2U
#define CANVAS_INSTANT 3U
#define CANVAS_STYLES 4U
/* Margins are multiples of this, so each side keeps whole chroma pairs and
   the picture inside starts at a 16-byte boundary, as the firmware filters
   need. */
#define CANVAS_MARGIN_ALIGN 16U
/* The bottom of Instant is this many times its other sides. */
#define CANVAS_INSTANT_BOTTOM 3U
/* The canvas sides in a margin array. */
#define CANVAS_LEFT 0U
#define CANVAS_TOP 1U
#define CANVAS_RIGHT 2U
#define CANVAS_BOTTOM 3U

#ifndef GRKIT_MAYBE_UNUSED
#define GRKIT_MAYBE_UNUSED __attribute__((unused))
#endif

/* The style of a border value, or CANVAS_OFF for a value out of range. */
GRKIT_MAYBE_UNUSED static u32 canvas_style(u32 border) {
    u32 style=border&0xffU;
    return style<CANVAS_STYLES ? style : CANVAS_OFF;
}
/* The luma of the border: 255 white or 0 black. The chroma is always 128. */
GRKIT_MAYBE_UNUSED static u32 canvas_luma(u32 border) { return (border>>8&0xffU)==1U ? 0U : 255U; }

/* The margin of each side, in percent of the short side of the picture. */
GRKIT_MAYBE_UNUSED static u32 canvas_percent(u32 style) {
    static const u8 percent[CANVAS_STYLES]={0,3,6,4};
    return style<CANVAS_STYLES ? percent[style] : 0U;
}

/* No limit for canvas_margins. */
#define CANVAS_NO_LIMIT 0xffffU

/* The margins of a picture w x h, stored with `turns` quarter turns
   clockwise to show it upright (picture_turns): left, top, right, and bottom
   of the stored picture. Each is the percent of the style of the short side,
   to the nearest multiple of CANVAS_MARGIN_ALIGN, and at most `limit`, which
   keeps a canvas inside the memory of the camera. Instant makes the side at
   the bottom of the upright picture CANVAS_INSTANT_BOTTOM times as wide.
   Returns the margin of the other sides, 0 without a border. */
GRKIT_MAYBE_UNUSED static u32 canvas_margins(u32 style,u32 w,u32 h,u32 turns,u32 limit,u32 e[4]) {
    /* The stored side that is the bottom of the upright picture. */
    static const u8 bottom[4]={CANVAS_BOTTOM,CANVAS_RIGHT,CANVAS_TOP,CANVAS_LEFT};
    u32 short_side=w<h ? w : h, m=core_divide(short_side*canvas_percent(style),100U);
    m=(m+CANVAS_MARGIN_ALIGN/2U)/CANVAS_MARGIN_ALIGN*CANVAS_MARGIN_ALIGN;
    if(m>limit) m=limit/CANVAS_MARGIN_ALIGN*CANVAS_MARGIN_ALIGN;
    e[CANVAS_LEFT]=e[CANVAS_TOP]=e[CANVAS_RIGHT]=e[CANVAS_BOTTOM]=m;
    if(style==CANVAS_INSTANT) e[bottom[turns&3U]]=CANVAS_INSTANT_BOTTOM*m;
    return m;
}

/* The largest even size of the aspect of cw x ch that fits into bw x bh:
   w | h << 16. A preview of the canvas keeps its whole area. */
GRKIT_MAYBE_UNUSED static u32 canvas_fit(u32 cw,u32 ch,u32 bw,u32 bh) {
    u32 w=bw, h=bh;
    if(!cw || !ch) return 0;
    if(cw*bh>ch*bw) h=core_divide(bw*ch,cw)&~1U;
    else w=core_divide(bh*cw,ch)&~1U;
    return w|h<<16;
}

/* The margin m of a canvas side c in a plane side s, rounded outward from
   the picture, so that no sample of the margin is inside. A smaller plane
   (`scaled`) takes one more canvas pixel at the far side: a preview sample
   is the mean of a canvas pixel and the next (canvas_preview). Even for the
   chroma pairs when `even`. */
GRKIT_MAYBE_UNUSED static u32 canvas_inner_margin(u32 m,u32 s,u32 c,u32 far,u32 even) {
    u32 v=core_divide((m+far)*s+c-1U,c);
    return even ? (v+1U)&~1U : v;
}

/* The picture inside the border of a canvas cw x ch with margins m (left,
   top, right, bottom), in a plane pw x ph: left, top, right, and bottom
   (exclusive) in the plane. The plane holds the canvas at its whole size
   when it has the aspect of the canvas within 2 pixels, such as the main
   picture or a fitted preview; else the canvas is fitted in the middle of
   the plane between black bars (canvas_fit), as a preview that keeps its
   box draws it. Returns 0 without a canvas. */
GRKIT_MAYBE_UNUSED static u32 canvas_inner_area(u32 cw,u32 ch,const u32 m[4],u32 pw,u32 ph,u32 out[4]) {
    u32 fit=canvas_fit(cw,ch,pw,ph), w=fit&0xffffU, h=fit>>16, ox, oy, scaled;
    if(!fit) return 0;
    if(w+2U>=pw && h+2U>=ph) { w=pw; h=ph; }
    scaled=w!=cw || h!=ch;
    ox=((pw-w)>>1)&~1U; oy=(ph-h)>>1;
    out[CANVAS_LEFT]=ox+canvas_inner_margin(m[CANVAS_LEFT],w,cw,0,1);
    out[CANVAS_RIGHT]=ox+w-canvas_inner_margin(m[CANVAS_RIGHT],w,cw,scaled,1);
    out[CANVAS_TOP]=oy+canvas_inner_margin(m[CANVAS_TOP],h,ch,0,0);
    out[CANVAS_BOTTOM]=oy+h-canvas_inner_margin(m[CANVAS_BOTTOM],h,ch,scaled,0);
    return 1;
}
#endif
