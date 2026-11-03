/* The pixel kernels of Glow: each runs NEON on its whole blocks and C on
   the rest, with the same arithmetic, so that the output does not depend
   on which part took a pixel. On the host, and with GLOW_SCALAR (the NEON
   parity test), C takes every pixel. */
#ifndef GLOW_KERNELS_H
#define GLOW_KERNELS_H
#include "grkit.h"

typedef signed short s16;

#if defined(__arm__) && !defined(GLOW_SCALAR)
#define GLOW_NEON 1
#else
#define GLOW_NEON 0
#endif

/* The constants below are plain numbers without a suffix: the NEON code
   takes them as immediates (GLOW_STR). */
#define GLOW_STR_(x) #x
#define GLOW_STR(x) GLOW_STR_(x)
/* Glow in linear light of 16 bits: GLOW_FULL is 1.0. */
#define GLOW_FULL 65472
/* The soft shoulder of the glow (glow_light): it grows by half above
   SHOULDER_KNEE (0.375), by 1/32 above SHOULDER_TOP (1.375), and stops at
   SHOULDER_END. */
#define SHOULDER_KNEE 24576
#define SHOULDER_TOP 90112
#define SHOULDER_END 352256
/* BT.601 full range, times 256: R, G, B of Y, Cb, Cr (pixel_rgb) ... */
#define RGB_R_CR 359
#define RGB_G_CB 88
#define RGB_G_CR 183
#define RGB_B_CB 454
/* ... and Y, Cb, Cr of R, G, B (glow_ycc), each magnitude. */
#define YCC_Y_R 77
#define YCC_Y_G 150
#define YCC_Y_B 29
#define YCC_CB_R 43
#define YCC_CB_G 85
#define YCC_CR_G 107
#define YCC_CR_B 21
#define YCC_C 128
/* The words of a NEON block of the composite (32 pixels). */
#define BLOCK_WORDS 8U

/* Saturating add and subtract of the four bytes of a word. */
#if defined(__arm__)
static inline u32 uqadd8(u32 a, u32 b) { u32 r; __asm__("uqadd8 %0, %1, %2" : "=r"(r) : "r"(a), "r"(b)); return r; }
static inline u32 uqsub8(u32 a, u32 b) { u32 r; __asm__("uqsub8 %0, %1, %2" : "=r"(r) : "r"(a), "r"(b)); return r; }
#else
static inline u32 uqsub8(u32 a, u32 b)
{
    u32 r = 0, i;
    for (i = 0; i < 32; i += 8) {
        u32 x = (a >> i) & 255, y = (b >> i) & 255;
        r |= (x > y ? x - y : 0) << i;
    }
    return r;
}
static inline u32 uqadd8(u32 a, u32 b)
{
    u32 r = 0, i;
    for (i = 0; i < 32; i += 8) {
        u32 s = ((a >> i) & 255) + ((b >> i) & 255);
        r |= (s > 255 ? 255 : s) << i;
    }
    return r;
}
#endif

/* These run for every screennail pixel: always inline, one line for each
   channel (-Os makes a call and a loop otherwise). */
#define INLINE static inline __attribute__((always_inline))
INLINE u32 clamp_level(int v) { return v < 0 ? 0 : v > 255 ? 255 : (u32)v; }

/* R, G, B (0-255) of a pixel (BT.601, full range). */
INLINE void pixel_rgb(u32 y, u32 cb, u32 cr, u32 mono, u32 rgb[3])
{
    int c = mono ? 0 : (int)cb - 128, r = mono ? 0 : (int)cr - 128, v = (int)y;
    rgb[0] = clamp_level(v + ((RGB_R_CR * r) >> 8));
    rgb[1] = clamp_level(v - ((RGB_G_CB * c + RGB_G_CR * r) >> 8));
    rgb[2] = clamp_level(v + ((RGB_B_CB * c) >> 8));
}

/* The arguments of the kernels, in the order that the NEON code reads them
   (the _Static_asserts in glow_kernels.c bind the offsets). */
typedef struct { u8 *r, *g, *b, *top; } GlowRgb;              /* R, G, B, and max(R, G, B) */
typedef struct { const u16 *ring[7]; u16 *out; } GlowLight;  /* 7 ring rows of light, then out */
typedef struct { const u8 *rgb[3]; s16 *ycc[3]; } GlowYcc;   /* R, G, B changes, then Y, Cb, Cr */
typedef struct { s16 *cb; const s16 *dcb; s16 *cr; const s16 *dcr; const u8 *top; } GlowChroma;

/* For each byte of `blocks` blocks of 16 bytes of a main row: 1 more in
   its count when it is `level` or above. */
void glow_clip(const u8 *row, u8 *counts, u32 blocks, u32 level);
/* The sum of the 16 counts of each block into `totals`, and the counts set
   to 0. */
void glow_clip_total(u8 *counts, u16 *totals, u32 blocks);
/* R, G, B, and their largest of n color pixels whose chroma pairs start at
   the first pixel (pixel_rgb). */
void glow_rgb(const u8 *luma, const u8 *chroma, const GlowRgb *out, u32 n);
/* The glow of n fine columns: the 1 6 15 20 15 6 1 sum of the ring rows
   along the column, times part / 256, through the shoulder. */
void glow_light(const GlowLight *rows, u32 part, u32 n);
/* Y, Cb, Cr changes (times 128) of n glow levels R, G, B. */
void glow_ycc(const GlowYcc *rows, u32 n);
/* Add the glow to n luma words: each word has a value and a step (Q7),
   the value between it and the next for its 4 pixels, scaled by the
   headroom 255 - top of each pixel; then each value takes its step. */
void glow_luma(u32 *q, s16 *value, const s16 *step, u32 n, const u8 *top);
/* Add the color of the glow to n interleaved CbCr words: each shared pair
   takes the change at its center, scaled by the mean headroom of its two
   pixels; then each value takes its step. */
void glow_chroma(u32 *q, const GlowChroma *a, u32 n);
#endif
