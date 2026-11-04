/* Date Imprint: the plane filter. It prints the local time on each
   recorded picture, as a film data back exposes it. */
#include "grkit.h"
#include "grkit_module.h"

/* A byte of a plane. */
#define BYTE(p,o) (GRKIT_BYTES((p)+(o))[0])

/* The row interpolation of the mask, and the light of the classic and
   monochrome stamps, run NEON on whole blocks (an assembly part, because
   the camera objects build soft-float), and C on the rest and on the light
   of the GR1s look, with the same arithmetic. On the host, and with
   DATE_SCALAR, C takes every pixel. */
#if defined(__arm__) && !defined(DATE_SCALAR)
#define DATE_NEON 1
#else
#define DATE_NEON 0
#endif
#define DATE_BLOCK_PAIRS 8U
/* The pixels of a row that one call of light_row takes: its core and halo
   light fit on the stack. */
#define ROW_PIXELS_VALUE 256
#define ROW_PIXELS ((u32)ROW_PIXELS_VALUE)
/* A number as text, for the assembly. */
#define DATE_STR_(x) #x
#define DATE_STR(x) DATE_STR_(x)
#define DATE_LIGHT_COLOR 0U /* Y, Cb, and Cr */
#define DATE_LIGHT_LUMA 1U  /* Y of a color picture: a row without its own chroma */
#define DATE_LIGHT_MONO 2U  /* Y of a monochrome picture */
/* light_pair for `blocks` blocks of DATE_BLOCK_PAIRS pixel pairs of the
   classic look (or monochrome): luma and its interleaved Cb Cr from the
   first pair, and the core and halo light of each pixel. */
void date_light_blocks(u8 *luma,u8 *cbcr,const u8 *core,const u8 *halo,u32 blocks,u32 mode);
/* The core and halo light of `blocks` blocks of 16 pixels of a picture row
   (stamp): from the blends of its two mask rows, `top` and `bottom` (core
   at 0, halo at `cols`), into `out` (core at 0, halo at ROW_PIXELS). */
void date_lerp_blocks(const u8 *top,const u8 *bottom,u8 *out,u32 blocks,u32 cols,u32 ty);
#if DATE_NEON
/* date_lerp_blocks: (a (65536 - ty) + b ty) >> 16 in 16 x 16 bits, as
   65536 - ty fits when ty is not 0; the core then times 320 >> 8, which is
   c + (c >> 2), at most 255. With ty 0 the light is that of `top`. */
__asm__(
    ".pushsection .text.date_neon_lerp,\"ax\",%progbits\n"
    ".syntax unified\n"
    ".fpu neon\n"
    ".thumb\n"
    ".p2align 2\n"
    ".global date_lerp_blocks\n"
    ".type date_lerp_blocks, %function\n"
    ".thumb_func\n"
    "date_lerp_blocks:\n"
    "    push {r4, r5, r6, r7, r8, lr}\n"
    "    ldr r4, [sp, #24]\n"
    "    ldr r5, [sp, #28]\n"
    "    cmp r3, #0\n"
    "    beq 9f\n"
    "    add r6, r0, r4\n"
    "    add r7, r1, r4\n"
    "    add r8, r2, #" DATE_STR(ROW_PIXELS_VALUE) "\n"
    "    cmp r5, #0\n"
    "    beq 5f\n"
    "    rsb r4, r5, #65536\n"
    "    vdup.16 d0, r4\n"
    "    vdup.16 d1, r5\n"
    "1:  vld1.8 {q8}, [r0]!\n"
    "    vld1.8 {q9}, [r1]!\n"
    "    vld1.8 {q10}, [r6]!\n"
    "    vld1.8 {q11}, [r7]!\n"
    "    vmovl.u8 q12, d16\n"
    "    vmovl.u8 q13, d18\n"
    "    vmull.u16 q14, d24, d0[0]\n"
    "    vmlal.u16 q14, d26, d1[0]\n"
    "    vmull.u16 q15, d25, d0[0]\n"
    "    vmlal.u16 q15, d27, d1[0]\n"
    "    vshrn.u32 d28, q14, #16\n"
    "    vshrn.u32 d29, q15, #16\n"
    "    vmovn.i16 d4, q14\n"
    "    vmovl.u8 q12, d17\n"
    "    vmovl.u8 q13, d19\n"
    "    vmull.u16 q14, d24, d0[0]\n"
    "    vmlal.u16 q14, d26, d1[0]\n"
    "    vmull.u16 q15, d25, d0[0]\n"
    "    vmlal.u16 q15, d27, d1[0]\n"
    "    vshrn.u32 d28, q14, #16\n"
    "    vshrn.u32 d29, q15, #16\n"
    "    vmovn.i16 d5, q14\n"
    "    vshr.u8 q3, q2, #2\n"
    "    vqadd.u8 q2, q2, q3\n"
    "    vst1.8 {q2}, [r2]!\n"
    "    vmovl.u8 q12, d20\n"
    "    vmovl.u8 q13, d22\n"
    "    vmull.u16 q14, d24, d0[0]\n"
    "    vmlal.u16 q14, d26, d1[0]\n"
    "    vmull.u16 q15, d25, d0[0]\n"
    "    vmlal.u16 q15, d27, d1[0]\n"
    "    vshrn.u32 d28, q14, #16\n"
    "    vshrn.u32 d29, q15, #16\n"
    "    vmovn.i16 d4, q14\n"
    "    vmovl.u8 q12, d21\n"
    "    vmovl.u8 q13, d23\n"
    "    vmull.u16 q14, d24, d0[0]\n"
    "    vmlal.u16 q14, d26, d1[0]\n"
    "    vmull.u16 q15, d25, d0[0]\n"
    "    vmlal.u16 q15, d27, d1[0]\n"
    "    vshrn.u32 d28, q14, #16\n"
    "    vshrn.u32 d29, q15, #16\n"
    "    vmovn.i16 d5, q14\n"
    "    vst1.8 {q2}, [r8]!\n"
    "    subs r3, r3, #1\n"
    "    bne 1b\n"
    "    b 9f\n"
    "5:  vld1.8 {q2}, [r0]!\n"
    "    vshr.u8 q3, q2, #2\n"
    "    vqadd.u8 q2, q2, q3\n"
    "    vst1.8 {q2}, [r2]!\n"
    "    vld1.8 {q2}, [r6]!\n"
    "    vst1.8 {q2}, [r8]!\n"
    "    subs r3, r3, #1\n"
    "    bne 5b\n"
    "9:  pop {r4, r5, r6, r7, r8, pc}\n"
    ".size date_lerp_blocks, .-date_lerp_blocks\n"
    ".popsection\n");
#endif
#if DATE_NEON
/* One half block (8 pixels, 4 pairs): R, G, and B from Y and the chroma
   terms of its pairs, each lit by its light (screen), into the scratch at
   OFF, 16 + OFF, and 32 + OFF. q14 holds Y in 16 bits; q5 and q15 are
   work registers. The screen of v by l is v + ((x + (x >> 8) + 128) >> 8)
   for x = (255 - v) l, which equals screen() in C. */
#define DATE_NEON_SCREEN(L) \
    "    vmvn d31, d30\n"              \
    "    vmull.u8 q5, d31, " L "\n"   \
    "    vsra.u16 q5, q5, #8\n"        \
    "    vrshrn.u16 d31, q5, #8\n"     \
    "    vadd.i8 d30, d30, d31\n"
#define DATE_NEON_TERM(T) \
    "    vmov d10, " T "\n"            \
    "    vmov d11, " T "\n"            \
    "    vzip.16 d10, d11\n"
#define DATE_NEON_HALF(V, LR, LG, LB, RC, GC, BC, OFF) \
    "    vmovl.u8 q14, " V "\n"        \
    DATE_NEON_TERM(RC)                 \
    "    vadd.i16 q5, q5, q14\n"       \
    "    vqmovun.s16 d30, q5\n"        \
    DATE_NEON_SCREEN(LR)               \
    "    add r6, r8, #" OFF "\n"       \
    "    vst1.8 {d30}, [r6]\n"         \
    DATE_NEON_TERM(GC)                 \
    "    vsub.i16 q5, q14, q5\n"       \
    "    vqmovun.s16 d30, q5\n"        \
    DATE_NEON_SCREEN(LG)               \
    "    add r6, r8, #(16 + " OFF ")\n" \
    "    vst1.8 {d30}, [r6]\n"         \
    DATE_NEON_TERM(BC)                 \
    "    vadd.i16 q5, q5, q14\n"       \
    "    vqmovun.s16 d30, q5\n"        \
    DATE_NEON_SCREEN(LB)               \
    "    add r6, r8, #(32 + " OFF ")\n" \
    "    vst1.8 {d30}, [r6]\n"
/* Y of 8 lit pixels into YOUT, and Cb then Cr of their 4 pairs into the
   scratch at COFF, as light_pair computes them. */
#define DATE_NEON_YCC(R, G, B, YOUT, COFF) \
    "    vmovl.u8 q9, " R "\n"         \
    "    vmovl.u8 q10, " G "\n"        \
    "    vmovl.u8 q11, " B "\n"        \
    "    vmull.u16 q12, d18, d1[0]\n"  \
    "    vmlal.u16 q12, d20, d1[1]\n"  \
    "    vmlal.u16 q12, d22, d1[2]\n"  \
    "    vmull.u16 q13, d19, d1[0]\n"  \
    "    vmlal.u16 q13, d21, d1[1]\n"  \
    "    vmlal.u16 q13, d23, d1[2]\n"  \
    "    vrshrn.u32 d24, q12, #16\n"   \
    "    vrshrn.u32 d25, q13, #16\n"   \
    "    vmovn.i16 " YOUT ", q12\n"    \
    "    vshll.u16 q12, d22, #15\n"    \
    "    vmlsl.u16 q12, d18, d2[0]\n"  \
    "    vmlsl.u16 q12, d20, d2[1]\n"  \
    "    vshll.u16 q13, d23, #15\n"    \
    "    vmlsl.u16 q13, d19, d2[0]\n"  \
    "    vmlsl.u16 q13, d21, d2[1]\n"  \
    "    vpadd.i32 d24, d24, d25\n"    \
    "    vpadd.i32 d25, d26, d27\n"    \
    "    vrshr.s32 q12, q12, #17\n"    \
    "    vqmovn.s32 d24, q12\n"        \
    "    vshll.u16 q13, d18, #15\n"    \
    "    vmlsl.u16 q13, d20, d2[2]\n"  \
    "    vmlsl.u16 q13, d22, d2[3]\n"  \
    "    vshll.u16 q15, d19, #15\n"    \
    "    vmlsl.u16 q15, d21, d2[2]\n"  \
    "    vmlsl.u16 q15, d23, d2[3]\n"  \
    "    vpadd.i32 d26, d26, d27\n"    \
    "    vpadd.i32 d27, d30, d31\n"    \
    "    vrshr.s32 q13, q13, #17\n"    \
    "    vqmovn.s32 d25, q13\n"        \
    "    vmov.i16 q13, #128\n"         \
    "    vadd.i16 q12, q12, q13\n"     \
    "    vqmovun.s16 d24, q12\n"       \
    "    add r6, r8, #" COFF "\n"      \
    "    vst1.8 {d24}, [r6]\n"
/* date_light_blocks(luma, cbcr, core, halo, blocks, mode): 8 pairs at a
   time. The chroma terms of a pair are those of light_pair in 16 bits:
   91881 cr >> 16 = cr + (26345 cr >> 16), (22554 cb + 46802 cr) >> 16 =
   cr + ((22554 cb - 18734 cr) >> 16), and 116130 cb >> 16 = 2 cb +
   (-14942 cb >> 16). A block without light keeps its bytes. Scratch on
   the stack: R at 0, G at 16, B at 32, Cb Cr of pairs 0-3 at 48 and of
   pairs 4-7 at 56. */
__asm__(
    ".pushsection .text.date_neon,\"ax\",%progbits\n"
    ".syntax unified\n"
    ".fpu neon\n"
    ".thumb\n"
    ".p2align 2\n"
    ".global date_light_blocks\n"
    ".type date_light_blocks, %function\n"
    ".thumb_func\n"
    "date_light_blocks:\n"
    "    push {r4, r5, r6, r7, r8, lr}\n"
    "    vpush {d8-d15}\n"
    "    sub sp, sp, #64\n"
    "    ldr r4, [sp, #152]\n"
    "    ldr r5, [sp, #156]\n"
    "    cmp r4, #0\n"
    "    beq 9f\n"
    "    movw r6, #26345\n"
    "    movt r6, #22554\n"
    "    movw r7, #18734\n"
    "    movt r7, #0xc5a2\n"
    "    vmov d0, r6, r7\n"
    "    movw r6, #19595\n"
    "    movt r6, #38470\n"
    "    movw r7, #7471\n"
    "    vmov d1, r6, r7\n"
    "    movw r6, #11059\n"
    "    movt r6, #21709\n"
    "    movw r7, #27439\n"
    "    movt r7, #5329\n"
    "    vmov d2, r6, r7\n"
    "    vmov.i8 d3, #180\n"
    "    vmov.i8 d4, #150\n"
    "    vmov.i8 d5, #40\n"
    "    vmov.i8 d6, #128\n"
    "    mov r8, sp\n"
    /* q4 Y, q5 core, q6 halo, q7 0xff for each pixel of a lit pair. */
    "1:  vld1.8 {q4}, [r0]\n"
    "    vld1.8 {q5}, [r2]!\n"
    "    vld1.8 {q6}, [r3]!\n"
    "    vorr q7, q5, q6\n"
    "    vtst.8 q7, q7, q7\n"
    "    vrev16.8 q8, q7\n"
    "    vorr q7, q7, q8\n"
    "    vorr d16, d14, d15\n"
    "    vmov r6, r7, d16\n"
    "    orrs r6, r6, r7\n"
    "    beq 8f\n"
    /* q8 red light: core + (halo * 180 >> 8), at most 255. */
    "    vmull.u8 q8, d12, d3\n"
    "    vshrn.u16 d16, q8, #8\n"
    "    vmull.u8 q9, d13, d3\n"
    "    vshrn.u16 d17, q9, #8\n"
    "    vqadd.u8 q8, q8, q5\n"
    "    cmp r5, #" DATE_STR(DATE_LIGHT_MONO) "\n"
    "    beq 5f\n"
    /* q9 green light: (core * 150 >> 8) + (halo * 40 >> 8); q10 blue
       light: core * 40 >> 8. */
    "    vmull.u8 q9, d10, d4\n"
    "    vshrn.u16 d18, q9, #8\n"
    "    vmull.u8 q10, d11, d4\n"
    "    vshrn.u16 d19, q10, #8\n"
    "    vmull.u8 q10, d12, d5\n"
    "    vshrn.u16 d20, q10, #8\n"
    "    vmull.u8 q11, d13, d5\n"
    "    vshrn.u16 d21, q11, #8\n"
    "    vadd.i8 q9, q9, q10\n"
    "    vmull.u8 q10, d10, d5\n"
    "    vshrn.u16 d20, q10, #8\n"
    "    vmull.u8 q11, d11, d5\n"
    "    vshrn.u16 d21, q11, #8\n"
    /* The chroma terms of the 8 pairs: q11 red, q12 green, q13 blue. */
    "    vld2.8 {d22, d23}, [r1]\n"
    "    vsubl.u8 q5, d22, d6\n"
    "    vsubl.u8 q6, d23, d6\n"
    "    vmull.s16 q12, d12, d0[0]\n"
    "    vmull.s16 q13, d13, d0[0]\n"
    "    vshrn.i32 d22, q12, #16\n"
    "    vshrn.i32 d23, q13, #16\n"
    "    vadd.i16 q11, q11, q6\n"
    "    vmull.s16 q13, d10, d0[1]\n"
    "    vmlsl.s16 q13, d12, d0[2]\n"
    "    vmull.s16 q14, d11, d0[1]\n"
    "    vmlsl.s16 q14, d13, d0[2]\n"
    "    vshrn.i32 d24, q13, #16\n"
    "    vshrn.i32 d25, q14, #16\n"
    "    vadd.i16 q12, q12, q6\n"
    "    vmull.s16 q14, d10, d0[3]\n"
    "    vmull.s16 q15, d11, d0[3]\n"
    "    vshrn.i32 d26, q14, #16\n"
    "    vshrn.i32 d27, q15, #16\n"
    "    vadd.i16 q13, q13, q5\n"
    "    vadd.i16 q13, q13, q5\n"
    DATE_NEON_HALF("d8", "d16", "d18", "d20", "d22", "d24", "d26", "0")
    DATE_NEON_HALF("d9", "d17", "d19", "d21", "d23", "d25", "d27", "8")
    "    vld1.8 {q5}, [r8]\n"
    "    add r6, r8, #16\n"
    "    vld1.8 {q6}, [r6]\n"
    "    add r6, r8, #32\n"
    "    vld1.8 {q8}, [r6]\n"
    DATE_NEON_YCC("d10", "d12", "d16", "d28", "48")
    DATE_NEON_YCC("d11", "d13", "d17", "d29", "56")
    "    cmp r5, #" DATE_STR(DATE_LIGHT_COLOR) "\n"
    "    bne 6f\n"
    /* Cb and Cr of the lit pairs. */
    "    add r6, r8, #48\n"
    "    vld1.8 {d24, d25}, [r6]\n"
    "    vuzp.32 d24, d25\n"
    "    vld2.8 {d22, d23}, [r1]\n"
    "    vmovn.i16 d30, q7\n"
    "    vmov d31, d30\n"
    "    vbsl d30, d24, d22\n"
    "    vbsl d31, d25, d23\n"
    "    vst2.8 {d30, d31}, [r1]\n"
    /* Y of the lit pairs. */
    "6:  vbsl q7, q14, q4\n"
    "    vst1.8 {q7}, [r0]!\n"
    "    add r1, r1, #16\n"
    "    subs r4, r4, #1\n"
    "    bne 1b\n"
    "    b 9f\n"
    /* Monochrome: Y lit by the red light. */
    "5:  vmvn q9, q4\n"
    "    vmull.u8 q10, d18, d16\n"
    "    vsra.u16 q10, q10, #8\n"
    "    vrshrn.u16 d22, q10, #8\n"
    "    vmull.u8 q10, d19, d17\n"
    "    vsra.u16 q10, q10, #8\n"
    "    vrshrn.u16 d23, q10, #8\n"
    "    vadd.i8 q14, q4, q11\n"
    "    b 6b\n"
    "8:  add r0, r0, #16\n"
    "    add r1, r1, #16\n"
    "    subs r4, r4, #1\n"
    "    bne 1b\n"
    "9:  add sp, sp, #64\n"
    "    vpop {d8-d15}\n"
    "    pop {r4, r5, r6, r7, r8, pc}\n"
    ".size date_light_blocks, .-date_light_blocks\n"
    ".popsection\n");
#endif

/* Longest blurred line of a date stamp mask, and the largest blur radius
   (look_halo). The line copy shares picture scratch with the later row blends. */
#define BLUR_WIDTH 640U
#define BLUR_RADIUS 32U
#define BLUR_BYTES (BLUR_WIDTH+2*BLUR_RADIUS+2)

/* Spread a mask line in place: three box passes, radius r. A line of zeros
   stays zero. The copy repeats the end values for r + 1 places on each side,
   so the window needs no test at the ends. */
static void blur_line(u8 *line,u32 step,u32 n,u32 r,u8 *copy) {
    u8 *at=copy+BLUR_RADIUS+1;
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
/* Add the light of `pairs` pixel pairs of row y from column x (even): the
   core and halo light of each pixel. A pair without light stays as it is. */
static void light_row(const Plane *p,u32 x,u32 y,const u8 *core,const u8 *halo,u32 pairs,u32 mono,u32 chroma,u32 look) {
    u32 i=0;
#if DATE_NEON
    if(look!=LOOK_DOTS) {
        u32 blocks=pairs/DATE_BLOCK_PAIRS;
        date_light_blocks((u8 *)(p->y+y*p->pitch+x),(u8 *)(chroma_row(p,y)+x),core,halo,blocks,
                          mono ? DATE_LIGHT_MONO : chroma ? DATE_LIGHT_COLOR : DATE_LIGHT_LUMA);
        i=blocks*DATE_BLOCK_PAIRS;
    }
#endif
    for(;i<pairs;++i) {
        u32 c[2]={core[2*i],core[2*i+1]}, l[2]={halo[2*i],halo[2*i+1]};
        if(c[0]|c[1]|l[0]|l[1]) light_pair(p,x+2*i,y,c,l,mono,chroma,look);
    }
}
/* The horizontal blend of one row of `mw` mask cells, `column` bytes apart,
   for each of `cols` picture columns, core then halo: column x samples cells
   x / f and the next one, with the weight (x mod f) * step. */
static void blend_row(u8 *out,const u8 *core,const u8 *halo,int column,u32 mw,u32 f,u32 step,u32 cols) {
    u32 x, cx=0, fx=0;
    for(x=0;x<cols;++x) {
        u32 at=cx<mw ? cx : mw-1, next=at+1<mw ? at+1 : at, tx=fx*step;
        int a=(int)at*column, b=(int)next*column;
        out[x]=(u8)((core[a]*(65536-tx)+core[b]*tx)>>16);
        out[cols+x]=(u8)((halo[a]*(65536-tx)+halo[b]*tx)>>16);
        if(++fx==f) { fx=0; ++cx; }
    }
}
/* Core and halo masks, followed by workspace reused first for blur, then
   for the blends of two rows of each mask when scaling up (ss 1), the
   light of a part of a picture row (light_row), and whether each mask row
   has light. The light of a smaller picture (stamp_sample), and the lit
   rows of a mask that does not scale up, take the blur workspace. The stack
   of the JPEG encoder task has no room for them. A mask for a `turned`
   picture also draws across its rows (mask_view). */
static u32 stamp_rows(u32 w,u32 h,u32 f) { return 4*w*f+2*ROW_PIXELS+h; }
static u32 stamp_bytes(u32 mw,u32 mh,u32 f,u32 ss,u32 turned) {
    u32 rows=ss==1 ? stamp_rows(mw,mh,f) : 0;
    if(ss==1 && turned && stamp_rows(mh,mw,f)>rows) rows=stamp_rows(mh,mw,f);
    return mw*mh*2+(rows>BLUR_BYTES ? rows : BLUR_BYTES);
}
_Static_assert(2*ROW_PIXELS_VALUE<=BLUR_BYTES,"The light of a row fits in the blur workspace");
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
static u32 stamp_mask(u8 *scratch,u32 width,u32 height,const char *text,u32 n,u32 look,u32 turned,Stamp *out) {
    u32 size, f, ss=1, m, pad, cw, mw, mh, margin, rise;
    u32 i, x, y, total;
    int caret;
    u8 *core=scratch, *halo, *copy;
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
        if(mw<=BLUR_WIDTH && stamp_bytes(mw,mh,f,ss,turned)<=GRKIT_PLANE_SCRATCH) break;
    }
    if(mw>BLUR_WIDTH || mh>BLUR_WIDTH || stamp_bytes(mw,mh,f,ss,turned)>GRKIT_PLANE_SCRATCH) return 0;
    look_margins(look,width<height ? width : height,size,&margin,&rise);
    if((((mw-pad)*f)>>(ss-1))+margin>width || (((mh-pad)*f)>>(ss-1))+rise>height) return 0;
    halo=core+mw*mh;
    copy=halo+mw*mh;
    for(i=0;i<mw*mh;++i) core[i]=0;
    for(i=0,caret=(int)pad<<8;i<n;++i) { glyph(core,mw,mh,text[i],caret,pad,m,look); caret+=advance(text[i],m,look); }
    for(i=0;i<mw*mh;++i) halo[i]=core[i];
    /* Classic strokes soften with a core blur. The dots are anti-aliased. */
    for(y=0;y<mh;++y) {
        if(look!=LOOK_DOTS) blur_line(core+y*mw,1,mw,1,copy);
        blur_line(halo+y*mw,1,mw,look_halo(m,look),copy);
    }
    for(x=0;x<mw;++x) {
        if(look!=LOOK_DOTS) blur_line(core+x,mw,mh,1,copy);
        blur_line(halo+x,mw,mh,look_halo(m,look),copy);
    }
    *out=(Stamp){core,halo,mw,mh,f,ss,(int)width-(int)margin-(int)(((mw-pad)*f)>>(ss-1)),
                 (int)height-(int)rise-(int)(((mh-pad)*f)>>(ss-1)),look};
    return 1;
}
/* A mask as a picture shows it: cell (a, b), a across and b down, is at
   core[b * row + a * column] and halo[b * row + a * column], for w x h
   cells. */
typedef struct { const u8 *core,*halo; int row,column; u32 w,h; } MaskView;
/* The view of the mask of `st` on the stored picture, for a picture that
   `turns` quarter turns clockwise show upright (upright): the mask turned
   back by the same turns. */
static MaskView mask_view(const Stamp *st,u32 turns) {
    int mw=(int)st->mw, mh=(int)st->mh, at;
    MaskView v;
    switch(turns&3) {
    case 0: at=0; v.row=mw; v.column=1; break;
    case 1: at=mw-1; v.row=-1; v.column=mw; break;
    case 2: at=mh*mw-1; v.row=-mw; v.column=-1; break;
    default: at=(mh-1)*mw; v.row=1; v.column=-mw; break;
    }
    v.core=st->core+at; v.halo=st->halo+at;
    v.w=turns&1 ? st->mh : st->mw;
    v.h=turns&1 ? st->mw : st->mh;
    return v;
}
/* Draw a mask view whose cells each span f x f pixels (ss 1) on the plane,
   with cell (0, 0) at pixel (ox, oy) of the plane; `work` is the scratch
   after the masks (stamp_bytes). Returns the column of cell (0, 0), on an
   even column so CbCr pairs line up. */
static int stamp_draw(u8 *work,const Plane *p,const MaskView *v,u32 f,int ox,int oy,u32 mono,u32 look) {
    u32 x, y, sy, dy, step, w=v->w, h=v->h, cols=w*f, shown=0xffffffffU, start, end;
    /* The horizontal blends of the two mask rows that a picture row
       samples, for each column: f picture rows sample the same pair, so
       each row of the pair blends once (`top`, `bottom`). */
    u8 *top=work, *bottom=top+2*cols;
    u8 *row=bottom+2*cols, *row_core=row, *row_halo=row+ROW_PIXELS, *lit=row+2*ROW_PIXELS;
    /* Whether each mask row has light, in the core or in the halo. A pixel
       that samples only empty rows, or only empty cells, stays as it is. */
    for(y=0;y<h;++y) {
        const u8 *c=v->core+(int)y*v->row, *l=v->halo+(int)y*v->row;
        u32 any=0;
        for(x=0;x<w && !any;++x) any=c[(int)x*v->column]|l[(int)x*v->column];
        lit[y]=(u8)(any!=0);
    }
    ox&=~1;
    step=divide(65536,f);
    /* The pairs inside the picture: column x (even) from `start`, while
       x < `end`; the second pixel of a last pair past the mask is dark. */
    start=(int)p->left>ox ? ((u32)((int)p->left-ox)+1U)&~1U : 0;
    end=(int)p->right-1>ox ? (u32)((int)p->right-1-ox) : 0;
    if(end>cols) end=cols;
    for(y=0,sy=0,dy=0;y<h*f;++y) {
        int py=oy+(int)y;
        u32 ty=dy*step, sy1=sy+1<h ? sy+1 : sy, chroma=!p->half || !(py&1);
        if(py>=(int)p->top && py<(int)p->bottom && (lit[sy] || lit[sy1])) {
            if(shown!=sy) {
                blend_row(top,v->core+(int)sy*v->row,v->halo+(int)sy*v->row,v->column,w,f,step,cols);
                blend_row(bottom,v->core+(int)sy1*v->row,v->halo+(int)sy1*v->row,v->column,w,f,step,cols);
                shown=sy;
            }
            for(x=start;x<end;x+=ROW_PIXELS) {
                u32 pairs=(end-x+1)/2, j=0;
                if(pairs>ROW_PIXELS/2) pairs=ROW_PIXELS/2;
#if DATE_NEON
                {
                    u32 inside=x<cols ? cols-x : 0, blocks;
                    if(inside>2*pairs) inside=2*pairs;
                    blocks=inside/16U;
                    date_lerp_blocks(top+x,bottom+x,row,blocks,cols,ty);
                    j=blocks*16U;
                }
#endif
                for(;j<2*pairs;++j) {
                    u32 at=x+j, ct, cb, ht, hb;
                    row_core[j]=row_halo[j]=0;
                    if(at>=cols) continue;
                    ct=top[at]; cb=bottom[at]; ht=top[cols+at]; hb=bottom[cols+at];
                    if(!(ct|cb|ht|hb)) continue;
                    {
                        u32 c=(ct*(65536-ty)+cb*ty)>>16;
                        /* The blur lowers the peak of thin strokes: an LED core saturates. */
                        c=(c*320)>>8;
                        row_core[j]=(u8)(c>255 ? 255 : c);
                        row_halo[j]=(u8)((ht*(65536-ty)+hb*ty)>>16);
                    }
                }
                light_row(p,(u32)(ox+(int)x),(u32)py,row_core,row_halo,pairs,mono,chroma,look);
            }
        }
        if(++dy==f) { dy=0; ++sy; }
    }
    return ox;
}
/* The stamp on the main picture of a landscape shot, in the layout of its
   mask. Returns 1 and describes the mask in `out` when it drew. */
static u32 stamp(u8 *scratch,const Plane *p,const char *text,u32 n,u32 mono,u32 look,Stamp *out) {
    u32 x, y, mw, mh;
    const u8 *core, *halo;
    u8 *lit;
    if(!stamp_mask(scratch,p->right-p->left,p->bottom-p->top,text,n,look,0,out)) return 0;
    core=out->core; halo=out->halo; mw=out->mw; mh=out->mh;
    if(out->ss==1) {
        MaskView v=mask_view(out,0);
        out->ox=stamp_draw(scratch+2*mw*mh,p,&v,out->f,(int)p->left+out->ox,(int)p->top+out->oy,mono,look)-(int)p->left;
        return 1;
    }
    /* Whether each mask row has light (stamp_draw), in the blur workspace. */
    lit=scratch+2*mw*mh;
    for(y=0;y<mh;++y) {
        const u8 *c=core+y*mw, *h=halo+y*mw;
        u32 any=0;
        for(x=0;x<mw && !any;++x) any=c[x]|h[x];
        lit[y]=(u8)(any!=0);
    }
    {
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
/* Bilinear core and halo values at cell coordinates (Q16, cell centers at
   +0.5), added to `core` and `halo`; nothing outside the masks. */
static void mask_add(const Stamp *st,int cx,int cy,u32 *core,u32 *halo) {
    int x=cx-32768, y=cy-32768, i=x>>16, j=y>>16, mw=(int)st->mw, mh=(int)st->mh;
    u32 tx=(u32)x&0xffff, ty=(u32)y&0xffff, k;
    u32 at, left=i>=0, right=i+1<mw, up=j>=0, down=j+1<mh;
    if(i<-1 || j<-1 || i>=mw || j>=mh) return;
    at=(u32)(j*mw+i);
    for(k=0;k<2;++k) {
        const u8 *m=k ? st->halo : st->core;
        u32 a=left && up ? m[at] : 0, b=right && up ? m[at+1] : 0;
        u32 c=left && down ? m[at+(u32)mw] : 0, d=right && down ? m[at+(u32)mw+1] : 0;
        u32 v=(((a*(65536-tx)+b*tx)>>16)*(65536-ty)+((c*(65536-tx)+d*tx)>>16)*ty)>>16;
        if(k) *halo+=v; else *core+=v;
    }
}
/* The products that give the Q16 cell coordinates of one sample point of
   `src` (Q16 picture point px, py): the cell coordinate is the product >> 16.
   An upright coordinate depends on one picture coordinate only, so each
   product is linear in px or in py. */
static void sample_products(const Plane *src,const Stamp *st,u32 turns,u32 rf,long long px,long long py,long long *pu,long long *pv) {
    long long u, v, scale=(long long)(st->ss*rf);
    upright(src,turns,px,py,&u,&v);
    *pu=(u-((long long)st->ox<<16))*scale;
    *pv=(v-((long long)st->oy<<16))*scale;
}
/* The stamp of the main picture `src` on the picture `dst`: each pixel of
   dst samples the mask at four points, so the stamp keeps the same place and
   the same total width as on the main picture, only softer on a smaller
   picture. A picture that `turns` quarter turns show upright gets the stamp
   at the bottom right of its upright view, turned with it. */
static void stamp_sample(const Plane *dst,const Plane *src,const Stamp *st,u32 mono,u32 turns) {
    u32 sw=src->right-src->left, sh=src->bottom-src->top, dw=dst->right-dst->left, dh=dst->bottom-dst->top;
    u32 kx=divide(sw<<16,dw), ky=divide(sh<<16,dh), rx=divide(dw<<16,sw), ry=divide(dh<<16,sh);
    u32 rf=divide(65536,st->f), x, y, q, odd=turns&1;
    int ext=(int)((st->mw*st->f)>>(st->ss-1)), eyt=(int)((st->mh*st->f)>>(st->ss-1)), box[4];
    int x0, x1, y0, y1;
    /* For each of the two sample columns and rows of a pixel (a quarter
       pixel left and right, above and below), the product of the column at
       x0 and its step for each pixel, and the products of the current row.
       A column gives the cell x of an upright picture, or its cell y after an
       odd number of quarter turns; a row the other one. */
    long long column0[2], column_step[2], row[2];
    /* The light of a part of a row, in the blur workspace after the masks. */
    u8 *row_core=(u8 *)st->halo+st->mw*st->mh, *row_halo=row_core+ROW_PIXELS;
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
    if(x0>=x1 || y0>=y1) return;
    for(q=0;q<2;++q) {
        long long dx=q ? (long long)(kx>>2) : -(long long)(kx>>2), pu, pv, nu, nv;
        long long sx=((long long)src->left<<16)+(long long)((u32)x0-dst->left)*kx+(kx>>1)+dx;
        long long sy=((long long)src->top<<16)+(kx>>1);
        sample_products(src,st,turns,rf,sx,sy,&pu,&pv);
        sample_products(src,st,turns,rf,sx+kx,sy,&nu,&nv);
        column0[q]=odd ? pv : pu;
        column_step[q]=odd ? nv-pv : nu-pu;
    }
    for(y=(u32)y0;(int)y<y1;++y) {
        u32 chroma=!dst->half || !(y&1);
        long long column[2]={column0[0],column0[1]};
        int cy[2], cx[2];
        for(q=0;q<2;++q) {
            long long dy=q ? (long long)(ky>>2) : -(long long)(ky>>2), pu, pv;
            long long sy=((long long)src->top<<16)+(long long)(y-dst->top)*ky+(ky>>1)+dy;
            sample_products(src,st,turns,rf,((long long)src->left<<16),sy,&pu,&pv);
            row[q]=odd ? pu : pv;
        }
        for(q=0;q<2;++q) (odd ? cx : cy)[q]=(int)(row[q]>>16);
        for(x=(u32)x0;(int)x+1<x1;x+=ROW_PIXELS) {
            u32 pairs=((u32)x1-x)/2, j;
            if(pairs>ROW_PIXELS/2) pairs=ROW_PIXELS/2;
            for(j=0;j<2*pairs;++j) {
                u32 core=0, halo=0, c;
                for(q=0;q<2;++q) {
                    (odd ? cy : cx)[q]=(int)(column[q]>>16);
                    column[q]+=column_step[q];
                }
                /* Sample q: column q & 1, row q >> 1. */
                for(q=0;q<4;++q)
                    mask_add(st,odd ? cx[q>>1] : cx[q&1],odd ? cy[q&1] : cy[q>>1],&core,&halo);
                c=((core>>2)*320)>>8;
                row_core[j]=(u8)(c>255 ? 255 : c);
                row_halo[j]=(u8)(halo>>2);
            }
            light_row(dst,x,y,row_core,row_halo,pairs,mono,chroma,st->look);
        }
    }
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
    /* A turned picture renders the mask upright. The smaller pictures sample
       it; the main picture draws it turned back (mask_view), cell for cell as
       a landscape picture does. */
    {
        const Plane *p = &set->planes[main];
        u32 w = p->right - p->left, h = p->bottom - p->top;
        if (!stamp_mask(set->scratch, turns & 1 ? h : w, turns & 1 ? w : h, text, n, look, 1, &st)) return;
        for (i = 0; i < set->count; ++i)
            if (i != main || st.ss != 1) stamp_sample(&set->planes[i], p, &st, set->mono, turns);
        if (st.ss == 1) {
            MaskView v = mask_view(&st, turns);
            int box[4];
            stored_box(p, turns, st.ox, st.oy, st.ox + (int)(st.mw * st.f), st.oy + (int)(st.mh * st.f), box);
            (void)stamp_draw(set->scratch + 2 * st.mw * st.mh, p, &v, st.f, box[0], box[1], set->mono, look);
        }
    }
}
