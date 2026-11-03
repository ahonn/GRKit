/* The pixel kernels of Glow (glow_kernels.h). */
#include "glow_kernels.h"

/* The NEON part of pass 3: 8 words (32 pixels) at a time. Values and
   steps are Q7, and each kernel advances its values after use. `top` is
   max(R, G, B) before any output writes, or Y for a monochrome plane.
   Luma uses per-pixel headroom. Chroma uses the same mean headroom for
   both components of each shared pair, sampled at that pair's center. */
#if GLOW_NEON
#define GLOW_NEON_LUMA(NAME) \
    ".p2align 2\n"                     \
    ".global " NAME "\n"               \
    ".type " NAME ", %function\n"      \
    ".thumb_func\n"                    \
    "" NAME ":\n"                      \
    "    cmp r3, #0\n"                 \
    "    beq 2f\n"                     \
    "    push {r4, lr}\n"             \
    "    ldr r4, [sp, #8]\n"           \
    "1:  vld1.16 {d20, d21}, [r1]\n"   \
    "    add r12, r1, #2\n"            \
    "    vld1.16 {d22, d23}, [r12]\n"  \
    "    vld1.16 {d24, d25}, [r2]!\n"  \
    "    vhadd.s16 q13, q10, q11\n"    \
    "    vhadd.s16 q14, q10, q13\n"    \
    "    vhadd.s16 q15, q13, q11\n"    \
    "    vqrshrun.s16 d16, q10, #7\n"  \
    "    vqrshrun.s16 d17, q14, #7\n"  \
    "    vqrshrun.s16 d18, q13, #7\n"  \
    "    vqrshrun.s16 d19, q15, #7\n"  \
    "    vadd.i16 q10, q10, q12\n"     \
    "    vst1.16 {d20, d21}, [r1]!\n"  \
    "    vld4.8 {d0-d3}, [r0]\n"       \
    "    vld4.8 {d4-d7}, [r4]!\n"      \
    "    vmvn q2, q2\n"                \
    "    vmvn q3, q3\n"                \
    "    vmull.u8 q12, d16, d4\n"      \
    "    vrshrn.u16 d16, q12, #8\n"    \
    "    vmull.u8 q12, d17, d5\n"      \
    "    vrshrn.u16 d17, q12, #8\n"    \
    "    vmull.u8 q12, d18, d6\n"      \
    "    vrshrn.u16 d18, q12, #8\n"    \
    "    vmull.u8 q12, d19, d7\n"      \
    "    vrshrn.u16 d19, q12, #8\n"    \
    "    vqadd.u8 q0, q0, q8\n"        \
    "    vqadd.u8 q1, q1, q9\n"        \
    "    vst4.8 {d0-d3}, [r0]!\n"      \
    "    subs r3, r3, #1\n"            \
    "    bne 1b\n"                     \
    "    pop {r4, lr}\n"              \
    "2:  bx lr\n"                      \
    ".size " NAME ", .-" NAME "\n"    
#define GLOW_NEON_CHROMA(NAME) \
    ".p2align 2\n"                      \
    ".global " NAME "\n"                \
    ".type " NAME ", %function\n"       \
    ".thumb_func\n"                     \
    "" NAME ":\n"                       \
    "    cmp r2, #0\n"                  \
    "    beq 2f\n"                      \
    "    push {r4, r5, r6, lr}\n"       \
    "    vpush {d8-d11}\n"              \
    "    ldr r3, [r1]\n"                \
    "    ldr r12, [r1, #4]\n"           \
    "    ldr r4, [r1, #8]\n"            \
    "    ldr r5, [r1, #12]\n"           \
    "    ldr r6, [r1, #16]\n"           \
    "    vmov.i8 q15, #0x80\n"          \
    "1:  vld1.16 {d20, d21}, [r3]\n"    \
    "    add lr, r3, #2\n"              \
    "    vld1.16 {d22, d23}, [lr]\n"    \
    "    vhadd.s16 q12, q10, q11\n"     \
    "    vhadd.s16 q13, q10, q12\n"     \
    "    vhadd.s16 q13, q10, q13\n"     \
    "    vhadd.s16 q14, q12, q11\n"     \
    "    vhadd.s16 q14, q12, q14\n"     \
    "    vqrshrn.s16 d8, q13, #7\n"    \
    "    vqrshrn.s16 d9, q14, #7\n"    \
    "    vld1.16 {d22, d23}, [r12]!\n"  \
    "    vadd.i16 q10, q10, q11\n"      \
    "    vst1.16 {d20, d21}, [r3]!\n"   \
    "    vld1.16 {d20, d21}, [r4]\n"    \
    "    add lr, r4, #2\n"              \
    "    vld1.16 {d22, d23}, [lr]\n"    \
    "    vhadd.s16 q12, q10, q11\n"     \
    "    vhadd.s16 q13, q10, q12\n"     \
    "    vhadd.s16 q13, q10, q13\n"     \
    "    vhadd.s16 q14, q12, q11\n"     \
    "    vhadd.s16 q14, q12, q14\n"     \
    "    vqrshrn.s16 d26, q13, #7\n"    \
    "    vqrshrn.s16 d27, q14, #7\n"    \
    "    vld1.16 {d22, d23}, [r5]!\n"   \
    "    vadd.i16 q10, q10, q11\n"      \
    "    vst1.16 {d20, d21}, [r4]!\n"   \
    "    vmov d28, d8\n"               \
    "    vmov d29, d9\n"               \
    "    vld4.8 {d20-d23}, [r6]!\n"     \
    "    vmvn q10, q10\n"               \
    "    vmvn q11, q11\n"               \
    "    vrhadd.u8 d20, d20, d21\n"     \
    "    vmov d21, d20\n"              \
    "    vrhadd.u8 d22, d22, d23\n"     \
    "    vmov d23, d22\n"               \
    "    vmovl.s8 q4, d28\n"            \
    "    vmovl.u8 q5, d20\n"            \
    "    vmul.i16 q4, q4, q5\n"         \
    "    vrshrn.i16 d4, q4, #8\n"       \
    "    vmovl.s8 q4, d26\n"            \
    "    vmovl.u8 q5, d21\n"            \
    "    vmul.i16 q4, q4, q5\n"         \
    "    vrshrn.i16 d5, q4, #8\n"       \
    "    vmovl.s8 q4, d29\n"            \
    "    vmovl.u8 q5, d22\n"            \
    "    vmul.i16 q4, q4, q5\n"         \
    "    vrshrn.i16 d6, q4, #8\n"       \
    "    vmovl.s8 q4, d27\n"            \
    "    vmovl.u8 q5, d23\n"            \
    "    vmul.i16 q4, q4, q5\n"         \
    "    vrshrn.i16 d7, q4, #8\n"       \
    "    vld4.8 {d0-d3}, [r0]\n"        \
    "    veor q0, q0, q15\n"            \
    "    veor q1, q1, q15\n"            \
    "    vqadd.s8 q0, q0, q2\n"         \
    "    vqadd.s8 q1, q1, q3\n"         \
    "    veor q0, q0, q15\n"            \
    "    veor q1, q1, q15\n"            \
    "    vst4.8 {d0-d3}, [r0]!\n"       \
    "    subs r2, r2, #1\n"             \
    "    bne 1b\n"                      \
    "    vpop {d8-d11}\n"               \
    "    pop {r4, r5, r6, lr}\n"        \
    "2:  bx lr\n"                       \
    ".size " NAME ", .-" NAME "\n"     
void glow_neon_luma(u32 *pixels, s16 *values, const s16 *steps, u32 blocks, const u8 *top);
void glow_neon_chroma(u32 *pixels, const GlowChroma *a, u32 blocks);
void glow_neon_rgb(const u8 *luma, const u8 *chroma, const GlowRgb *out, u32 blocks);
void glow_neon_light(const GlowLight *rows, u32 part, u32 blocks);
void glow_neon_ycc(const GlowYcc *rows, u32 blocks);
void glow_neon_clip(const u8 *row, u8 *counts, u32 blocks, u32 level);
void glow_neon_clip_total(u8 *counts, u16 *totals, u32 blocks);
/* The offsets at which the NEON code reads its arguments. */
_Static_assert(__builtin_offsetof(GlowRgb, top) == 12, "glow_neon_rgb reads out->top at 12");
_Static_assert(__builtin_offsetof(GlowLight, out) == 28, "glow_neon_light reads rows->out at 28");
_Static_assert(__builtin_offsetof(GlowYcc, ycc[2]) == 20, "glow_neon_ycc reads rows->ycc[2] at 20");
_Static_assert(__builtin_offsetof(GlowChroma, top) == 16, "glow_neon_chroma reads a->top at 16");
__asm__(
    ".pushsection .text.glow_neon,\"ax\",%progbits\n"
    ".syntax unified\n"
    ".fpu neon\n"
    ".thumb\n"
    GLOW_NEON_LUMA("glow_neon_luma")
    GLOW_NEON_CHROMA("glow_neon_chroma")
    /* glow_neon_rgb(luma, chroma, outs, blocks): 16 pixels at a time, with
       the Cb Cr pair of each two pixels; outs = {R, G, B, max}. The same
       arithmetic as pixel_rgb in 16 bits: 359 r >> 8 = r + (103 r >> 8),
       (88 c + 183 r) >> 8 = r + ((88 c - 73 r) >> 8), and 454 c >> 8 =
       2 c + (-58 c >> 8). */
    ".p2align 2\n"
    ".global glow_neon_rgb\n"
    ".type glow_neon_rgb, %function\n"
    ".thumb_func\n"
    "glow_neon_rgb:\n"
    "    cmp r3, #0\n"
    "    beq 2f\n"
    "    push {r4, r5, r6, r7}\n"
    "    ldr r4, [r2]\n"
    "    ldr r5, [r2, #4]\n"
    "    ldr r6, [r2, #8]\n"
    "    ldr r7, [r2, #12]\n"
    "1:  vld1.8 {d0, d1}, [r0]!\n"
    "    vld2.8 {d2, d3}, [r1]!\n"
    "    vmov.i8 d30, #128\n"
    "    vmovl.u8 q12, d0\n"
    "    vmovl.u8 q13, d1\n"
    "    vsubl.u8 q2, d2, d30\n"
    "    vsubl.u8 q3, d3, d30\n"
    "    vmov.i16 q15, #(" GLOW_STR(RGB_R_CR) " - 256)\n"
    "    vmul.i16 q8, q3, q15\n"
    "    vshr.s16 q8, q8, #8\n"
    "    vadd.i16 q8, q8, q3\n"
    "    vmov q9, q8\n"
    "    vzip.16 q8, q9\n"
    "    vadd.i16 q8, q8, q12\n"
    "    vadd.i16 q9, q9, q13\n"
    "    vqmovun.s16 d28, q8\n"
    "    vqmovun.s16 d29, q9\n"
    "    vmov.i16 q15, #" GLOW_STR(RGB_G_CB) "\n"
    "    vmul.i16 q8, q2, q15\n"
    "    vmov.i16 q15, #(256 - " GLOW_STR(RGB_G_CR) ")\n"
    "    vmls.i16 q8, q3, q15\n"
    "    vshr.s16 q8, q8, #8\n"
    "    vadd.i16 q8, q8, q3\n"
    "    vmov q9, q8\n"
    "    vzip.16 q8, q9\n"
    "    vsub.i16 q8, q12, q8\n"
    "    vsub.i16 q9, q13, q9\n"
    "    vqmovun.s16 d2, q8\n"
    "    vqmovun.s16 d3, q9\n"
    "    vmov.i16 q15, #(512 - " GLOW_STR(RGB_B_CB) ")\n"
    "    vmul.i16 q8, q2, q15\n"
    "    vneg.s16 q8, q8\n"
    "    vshr.s16 q8, q8, #8\n"
    "    vadd.i16 q8, q8, q2\n"
    "    vadd.i16 q8, q8, q2\n"
    "    vmov q9, q8\n"
    "    vzip.16 q8, q9\n"
    "    vadd.i16 q8, q8, q12\n"
    "    vadd.i16 q9, q9, q13\n"
    "    vqmovun.s16 d0, q8\n"
    "    vqmovun.s16 d1, q9\n"
    "    vst1.8 {d28, d29}, [r4]!\n"
    "    vst1.8 {d2, d3}, [r5]!\n"
    "    vst1.8 {d0, d1}, [r6]!\n"
    "    vmax.u8 q14, q14, q1\n"
    "    vmax.u8 q14, q14, q0\n"
    "    vst1.8 {d28, d29}, [r7]!\n"
    "    subs r3, r3, #1\n"
    "    bne 1b\n"
    "    pop {r4, r5, r6, r7}\n"
    "2:  bx lr\n"
    ".size glow_neon_rgb, .-glow_neon_rgb\n"
    /* glow_neon_light(rows, part, blocks): 4 fine columns at a time, rows =
       {7 ring rows of light (u16), out (u16)}: the 1 6 15 20 15 6 1 sum s,
       s * part / 256, through the shoulder. */
    ".p2align 2\n"
    ".global glow_neon_light\n"
    ".type glow_neon_light, %function\n"
    ".thumb_func\n"
    "glow_neon_light:\n"
    "    cmp r2, #0\n"
    "    beq 2f\n"
    "    push {r4, r5, r6, r7, r8, r9, r10}\n"
    "    vdup.32 q11, r1\n"
    "    movw r12, #" GLOW_STR(SHOULDER_KNEE) "\n"
    "    vdup.32 q12, r12\n"
    "    movw r12, #(" GLOW_STR(SHOULDER_TOP) " & 0xffff)\n"
    "    movt r12, #(" GLOW_STR(SHOULDER_TOP) " >> 16)\n"
    "    vdup.32 q13, r12\n"
    "    movw r12, #(" GLOW_STR(SHOULDER_END) " & 0xffff)\n"
    "    movt r12, #(" GLOW_STR(SHOULDER_END) " >> 16)\n"
    "    vdup.32 q14, r12\n"
    "    movw r12, #" GLOW_STR(GLOW_FULL) "\n"
    "    vdup.32 q15, r12\n"
    "    movw r12, #6\n"
    "    movt r12, #15\n"
    "    movw r1, #20\n"
    "    vmov d7, r12, r1\n"
    "    ldr r3, [r0]\n"
    "    ldr r4, [r0, #4]\n"
    "    ldr r5, [r0, #8]\n"
    "    ldr r6, [r0, #12]\n"
    "    ldr r7, [r0, #16]\n"
    "    ldr r8, [r0, #20]\n"
    "    ldr r9, [r0, #24]\n"
    "    ldr r10, [r0, #28]\n"
    "1:  vld1.16 {d0}, [r3]!\n"
    "    vld1.16 {d1}, [r4]!\n"
    "    vld1.16 {d2}, [r5]!\n"
    "    vld1.16 {d3}, [r6]!\n"
    "    vld1.16 {d4}, [r7]!\n"
    "    vld1.16 {d5}, [r8]!\n"
    "    vld1.16 {d6}, [r9]!\n"
    "    vaddl.u16 q8, d0, d6\n"
    "    vmlal.u16 q8, d1, d7[0]\n"
    "    vmlal.u16 q8, d5, d7[0]\n"
    "    vmlal.u16 q8, d2, d7[1]\n"
    "    vmlal.u16 q8, d4, d7[1]\n"
    "    vmlal.u16 q8, d3, d7[2]\n"
    "    vshl.i32 q9, q8, #24\n"
    "    vshr.u32 q9, q9, #24\n"
    "    vmul.i32 q9, q9, q11\n"
    "    vshr.u32 q9, q9, #8\n"
    "    vshr.u32 q8, q8, #8\n"
    "    vmul.i32 q8, q8, q11\n"
    "    vadd.i32 q8, q8, q9\n"
    "    vmin.u32 q9, q8, q12\n"
    "    vmax.u32 q10, q8, q12\n"
    "    vmin.u32 q10, q10, q13\n"
    "    vsub.i32 q10, q10, q12\n"
    "    vshr.u32 q10, q10, #1\n"
    "    vadd.i32 q9, q9, q10\n"
    "    vmax.u32 q10, q8, q13\n"
    "    vmin.u32 q10, q10, q14\n"
    "    vsub.i32 q10, q10, q13\n"
    "    vshr.u32 q10, q10, #5\n"
    "    vadd.i32 q9, q9, q10\n"
    "    vmin.u32 q9, q9, q15\n"
    "    vmovn.i32 d18, q9\n"
    "    vst1.16 {d18}, [r10]!\n"
    "    subs r2, r2, #1\n"
    "    bne 1b\n"
    "    pop {r4, r5, r6, r7, r8, r9, r10}\n"
    "2:  bx lr\n"
    ".size glow_neon_light, .-glow_neon_light\n"
    /* glow_neon_ycc(rows, blocks): 8 fine columns at a time, rows = {R, G,
       B changes (u8), Y, Cb, Cr out (s16)}, as glow_ycc: (77 R + 150 G +
       29 B) >> 1, (-43 R - 85 G + 128 B) >> 1, (128 R - 107 G - 21 B) >> 1. */
    ".p2align 2\n"
    ".global glow_neon_ycc\n"
    ".type glow_neon_ycc, %function\n"
    ".thumb_func\n"
    "glow_neon_ycc:\n"
    "    cmp r1, #0\n"
    "    beq 2f\n"
    "    push {r4, r5, r6, r7}\n"
    "    ldr r2, [r0]\n"
    "    ldr r3, [r0, #4]\n"
    "    ldr r4, [r0, #8]\n"
    "    ldr r5, [r0, #12]\n"
    "    ldr r6, [r0, #16]\n"
    "    ldr r7, [r0, #20]\n"
    "    vmov.i8 d24, #" GLOW_STR(YCC_CB_R) "\n"
    "    vmov.i8 d25, #" GLOW_STR(YCC_CB_G) "\n"
    "    vmov.i8 d26, #" GLOW_STR(YCC_C) "\n"
    "    vmov.i8 d27, #" GLOW_STR(YCC_CR_G) "\n"
    "    vmov.i8 d28, #" GLOW_STR(YCC_Y_R) "\n"
    "    vmov.i8 d29, #" GLOW_STR(YCC_Y_G) "\n"
    "    vmov.i8 d30, #" GLOW_STR(YCC_Y_B) "\n"
    "    vmov.i8 d31, #" GLOW_STR(YCC_CR_B) "\n"
    "1:  vld1.8 {d0}, [r2]!\n"
    "    vld1.8 {d1}, [r3]!\n"
    "    vld1.8 {d2}, [r4]!\n"
    "    vmull.u8 q8, d0, d28\n"
    "    vmlal.u8 q8, d1, d29\n"
    "    vmlal.u8 q8, d2, d30\n"
    "    vshr.u16 q8, q8, #1\n"
    "    vmull.u8 q9, d2, d26\n"
    "    vmlsl.u8 q9, d0, d24\n"
    "    vmlsl.u8 q9, d1, d25\n"
    "    vshr.s16 q9, q9, #1\n"
    "    vmull.u8 q10, d0, d26\n"
    "    vmlsl.u8 q10, d1, d27\n"
    "    vmlsl.u8 q10, d2, d31\n"
    "    vshr.s16 q10, q10, #1\n"
    "    vst1.16 {d16, d17}, [r5]!\n"
    "    vst1.16 {d18, d19}, [r6]!\n"
    "    vst1.16 {d20, d21}, [r7]!\n"
    "    subs r1, r1, #1\n"
    "    bne 1b\n"
    "    pop {r4, r5, r6, r7}\n"
    "2:  bx lr\n"
    ".size glow_neon_ycc, .-glow_neon_ycc\n"
    /* glow_neon_clip(row, counts, blocks, level): for each byte of a row
       (blocks of 16), 1 is added to its count when it is at `level` or
       above (the compare gives 0xff, and count - 0xff is count + 1). */
    ".p2align 2\n"
    ".global glow_neon_clip\n"
    ".type glow_neon_clip, %function\n"
    ".thumb_func\n"
    "glow_neon_clip:\n"
    "    cmp r2, #0\n"
    "    beq 2f\n"
    "    vdup.8 q15, r3\n"
    "1:  vld1.8 {d0, d1}, [r0]!\n"
    "    vld1.8 {d2, d3}, [r1]\n"
    "    vcge.u8 q0, q0, q15\n"
    "    vsub.i8 q1, q1, q0\n"
    "    vst1.8 {d2, d3}, [r1]!\n"
    "    subs r2, r2, #1\n"
    "    bne 1b\n"
    "2:  bx lr\n"
    ".size glow_neon_clip, .-glow_neon_clip\n"
    /* glow_neon_clip_total(counts, totals, blocks): the sum of the 16
       counts of each block, and the counts set to 0. */
    ".p2align 2\n"
    ".global glow_neon_clip_total\n"
    ".type glow_neon_clip_total, %function\n"
    ".thumb_func\n"
    "glow_neon_clip_total:\n"
    "    cmp r2, #0\n"
    "    beq 2f\n"
    "    vmov.i8 q15, #0\n"
    "1:  vld1.8 {d0, d1}, [r0]\n"
    "    vst1.8 {d30, d31}, [r0]!\n"
    "    vpaddl.u8 q0, q0\n"
    "    vpadd.i16 d0, d0, d1\n"
    "    vpadd.i16 d0, d0, d0\n"
    "    vpadd.i16 d0, d0, d0\n"
    "    vst1.16 {d0[0]}, [r1]!\n"
    "    subs r2, r2, #1\n"
    "    bne 1b\n"
    "2:  bx lr\n"
    ".size glow_neon_clip_total, .-glow_neon_clip_total\n"
    ".popsection\n");
#endif

/* ---- C ------------------------------------------------------------------ */

/* A soft shoulder for glow in linear light (GLOW_FULL = 1.0): above 0.375
   it grows by half, above 1.375 by 1/32, so that a strong glow ends in a
   smooth top and not in a flat one with a hard edge. */
static u32 shoulder(u32 g)
{
    if (g <= SHOULDER_KNEE) return g;
    if (g <= SHOULDER_TOP) return SHOULDER_KNEE + ((g - SHOULDER_KNEE) >> 1);
    g = g <= SHOULDER_END ? SHOULDER_KNEE + ((SHOULDER_TOP - SHOULDER_KNEE) >> 1) + ((g - SHOULDER_TOP) >> 5) : 65536;
    return g > GLOW_FULL ? GLOW_FULL : g;
}

/* Y, Cb, Cr changes times 128 of glow levels R, G, B (each times 256). */
static void ycc_pixel(int r, int g, int b, s16 *y, s16 *cb, s16 *cr)
{
    *y = (s16)((YCC_Y_R * r + YCC_Y_G * g + YCC_Y_B * b) >> 9);
    *cb = (s16)((-YCC_CB_R * r - YCC_CB_G * g + YCC_C * b) >> 9);
    *cr = (s16)((YCC_C * r - YCC_CR_G * g - YCC_CR_B * b) >> 9);
}

/* The clipped pixels (luma `level` or above) of a main row, counted for
   each byte of n blocks of 16 bytes, and the totals of the blocks: the C
   part of glow_neon_clip and glow_neon_clip_total. A count holds at most
   CLIP_ROWS rows. */
static void clip_blocks(const u8 *row, u8 *counts, u32 n, u32 level)
{
    u32 i;
    for (i = 0; i < n << 4; ++i) counts[i] = (u8)(counts[i] + (row[i] >= level));
}
static void clip_total(u8 *counts, u16 *totals, u32 n)
{
    u32 b, i;
    for (b = 0; b < n; ++b) {
        u32 c = 0;
        for (i = 0; i < 16; ++i) {
            c += counts[(b << 4) + i];
            counts[(b << 4) + i] = 0;
        }
        totals[b] = (u16)c;
    }
}

/* The C part of glow_neon_light: the same arithmetic, for the host and for
   the columns after the last block. */
static void light_words(const GlowLight *rows, u32 part, u32 from, u32 to)
{
    const u16 *const *l = rows->ring;
    u16 *out = rows->out;
    u32 x;
    for (x = from; x < to; ++x) {
        u32 s = l[0][x] + l[6][x] + 6 * (l[1][x] + l[5][x]) + 15 * (l[2][x] + l[4][x]) + 20 * l[3][x];
        /* Keep the fractional light before gain. Multiplying the full sum
           first can overflow for high gains, so split its integer part. */
        u32 fine = (s >> 8) * part + (((s & 255) * part) >> 8);
        out[x] = (u16)shoulder(fine);
    }
}

/* The level of a value times 128, rounded: 0-255 as vqrshrun does, and the
   change, -128 to 127, as vqrshrn does. Rounding keeps a glow without blue
   from making the blue lower. */
static inline u32 level_of(s16 v) { int c = ((int)v + 64) >> 7; return c < 0 ? 0 : c > 255 ? 255 : (u32)c; }
static inline int change_of(s16 v) { int c = ((int)v + 64) >> 7; return c < -128 ? -128 : c > 127 ? 127 : c; }

/* The word part of the NEON code, for the host and for the words after the
   last full block. */
static void luma_words(u32 *q, s16 *value, const s16 *step, u32 n, const u8 *top)
{
    while (n--) {
        int v[4], vn = value[1], i;
        u32 w = *q, add = 0;
        v[0] = *value;
        v[2] = (v[0] + vn) >> 1;
        v[1] = (v[0] + v[2]) >> 1;
        v[3] = (v[2] + vn) >> 1;
        for (i = 0; i < 4; ++i) add |= ((level_of((s16)v[i]) * (255 - top[i]) + 128) >> 8) << (8 * i);
        *q = uqadd8(w, add);
        *value = (s16)(*value + *step);
        ++q; ++value; ++step; top += 4;
    }
}

static void chroma_words(u32 *q, const u8 *top, s16 *cb, const s16 *dcb, s16 *cr, const s16 *dcr, u32 n)
{
    while (n--) {
        int c[4], i, bm = (*cb + cb[1]) >> 1, rm = (*cr + cr[1]) >> 1;
        u32 w = *q, out = 0;
        /* Centers of the two shared pairs: 1/8 and 5/8 of a word.
           Keep the halving order identical to the NEON interpolation. */
        c[0] = change_of((s16)((*cb + ((*cb + bm) >> 1)) >> 1));
        c[1] = change_of((s16)((*cr + ((*cr + rm) >> 1)) >> 1));
        c[2] = change_of((s16)((bm + ((bm + cb[1]) >> 1)) >> 1));
        c[3] = change_of((s16)((rm + ((rm + cr[1]) >> 1)) >> 1));
        for (i = 0; i < 4; ++i) {
            u32 pair = (u32)i & ~1U;
            int inv = (510 - top[pair] - top[pair + 1] + 1) >> 1;
            int v = (int)((w >> (8 * i)) & 255);
            v += (c[i] * inv + 128) >> 8;
            out |= (u32)(v < 0 ? 0 : v > 255 ? 255 : v) << (8 * i);
        }
        *q = out;
        *cb = (s16)(*cb + *dcb);
        *cr = (s16)(*cr + *dcr);
        ++q; top += 4; ++cb; ++dcb; ++cr; ++dcr;
    }
}

/* ---- the kernels --------------------------------------------------------- */

void glow_clip(const u8 *row, u8 *counts, u32 blocks, u32 level)
{
#if GLOW_NEON
    glow_neon_clip(row, counts, blocks, level);
#else
    clip_blocks(row, counts, blocks, level);
#endif
}

void glow_clip_total(u8 *counts, u16 *totals, u32 blocks)
{
#if GLOW_NEON
    glow_neon_clip_total(counts, totals, blocks);
#else
    clip_total(counts, totals, blocks);
#endif
}

void glow_rgb(const u8 *luma, const u8 *chroma, const GlowRgb *out, u32 n)
{
    u32 x = 0;
#if GLOW_NEON
    glow_neon_rgb(luma, chroma, out, n >> 4);
    x = n & ~15U;
#endif
    for (; x < n; ++x) {
        u32 rgb[3], top;
        pixel_rgb(luma[x], chroma[x & ~1U], chroma[(x & ~1U) + 1], 0, rgb);
        out->r[x] = (u8)rgb[0];
        out->g[x] = (u8)rgb[1];
        out->b[x] = (u8)rgb[2];
        top = rgb[0] > rgb[1] ? rgb[0] : rgb[1];
        out->top[x] = (u8)(top > rgb[2] ? top : rgb[2]);
    }
}

void glow_light(const GlowLight *rows, u32 part, u32 n)
{
    u32 x = 0;
#if GLOW_NEON
    glow_neon_light(rows, part, n >> 2);
    x = n & ~3U;
#endif
    light_words(rows, part, x, n);
}

void glow_ycc(const GlowYcc *rows, u32 n)
{
    u32 x = 0;
#if GLOW_NEON
    glow_neon_ycc(rows, n >> 3);
    x = n & ~7U;
#endif
    for (; x < n; ++x)
        ycc_pixel((int)rows->rgb[0][x] << 8, (int)rows->rgb[1][x] << 8, (int)rows->rgb[2][x] << 8,
                  &rows->ycc[0][x], &rows->ycc[1][x], &rows->ycc[2][x]);
}

void glow_luma(u32 *q, s16 *value, const s16 *step, u32 n, const u8 *top)
{
    u32 k = 0;
#if GLOW_NEON
    k = n / BLOCK_WORDS * BLOCK_WORDS;
    glow_neon_luma(q, value, step, k / BLOCK_WORDS, top);
#endif
    luma_words(q + k, value + k, step + k, n - k, top + 4 * k);
}

void glow_chroma(u32 *q, const GlowChroma *a, u32 n)
{
    u32 k = 0;
#if GLOW_NEON
    k = n / BLOCK_WORDS * BLOCK_WORDS;
    glow_neon_chroma(q, a, k / BLOCK_WORDS);
#endif
    chroma_words(q + k, a->top + 4 * k, a->cb + k, a->dcb + k, a->cr + k, a->dcr + k, n - k);
}
