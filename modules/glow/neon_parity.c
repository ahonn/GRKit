/* The NEON parity test of the Glow kernels (glow_kernels.c). It is not a
   source of the module: the ignored test synthetic_glow_neon_parity
   (crates/grkit-build/src/tests/modules.rs) builds it for 32-bit ARM Linux
   with glow_kernels.c twice, once with NEON and once with GLOW_SCALAR (C
   only, each kernel named scalar_<name>), and runs it with qemu-arm.

   Each kernel runs on the same random inputs and lengths in both builds,
   with lengths that end in a NEON block and in a C tail, and the outputs
   must be equal byte for byte. Standard output names each kernel that
   differs, and the exit status is their number. */
#include "glow_kernels.h"

void scalar_glow_clip(const u8 *row, u8 *counts, u32 blocks, u32 level);
void scalar_glow_clip_total(u8 *counts, u16 *totals, u32 blocks);
void scalar_glow_rgb(const u8 *luma, const u8 *chroma, const GlowRgb *out, u32 n);
void scalar_glow_light(const GlowLight *rows, u32 part, u32 n);
void scalar_glow_ycc(const GlowYcc *rows, u32 n);
void scalar_glow_luma(u32 *q, s16 *value, const s16 *step, u32 n, const u8 *top);
void scalar_glow_chroma(u32 *q, const GlowChroma *a, u32 n);

/* The freestanding link has no C library. */
void *memset(void *d, int v, __SIZE_TYPE__ n)
{
    u8 *p = d;
    while (n--) *p++ = (u8)v;
    return d;
}
void *memcpy(void *d, const void *s, __SIZE_TYPE__ n)
{
    u8 *p = d;
    const u8 *q = s;
    while (n--) *p++ = *q++;
    return d;
}
void __aeabi_memset(void *d, __SIZE_TYPE__ n, int v) { memset(d, v, n); }
void __aeabi_memclr(void *d, __SIZE_TYPE__ n) { memset(d, 0, n); }
void __aeabi_memclr4(void *d, __SIZE_TYPE__ n) { memset(d, 0, n); }
void __aeabi_memcpy(void *d, const void *s, __SIZE_TYPE__ n) { memcpy(d, s, n); }
void __aeabi_memcpy4(void *d, const void *s, __SIZE_TYPE__ n) { memcpy(d, s, n); }

static void sys_write(const char *text)
{
    u32 n = 0;
    while (text[n]) ++n;
    register u32 r0 __asm__("r0") = 1;
    register const char *r1 __asm__("r1") = text;
    register u32 r2 __asm__("r2") = n;
    register u32 r7 __asm__("r7") = 4;
    __asm__ volatile("svc #0" : "+r"(r0) : "r"(r1), "r"(r2), "r"(r7) : "memory");
}

static void sys_exit(u32 code)
{
    register u32 r0 __asm__("r0") = code;
    register u32 r7 __asm__("r7") = 1;
    __asm__ volatile("svc #0" : : "r"(r0), "r"(r7));
    for (;;) {}
}

static u32 seed = 0x9e3779b9U;
static u32 next(void)
{
    seed ^= seed << 13;
    seed ^= seed >> 17;
    seed ^= seed << 5;
    return seed;
}
/* A random value below n, without a division: n is at most 65536. */
static u32 below(u32 n) { return (u32)(((next() >> 16) * n) >> 16); }

static int same(const void *a, const void *b, u32 n)
{
    const u8 *p = a, *q = b;
    while (n--)
        if (*p++ != *q++) return 0;
    return 1;
}

#define CASES 400U
#define MAX_N 200U

static u8 bytes[2][4][MAX_N * 4 + 64];
static u16 halves[2][8][MAX_N + 8];
static s16 values[2][6][MAX_N + 8];
static u32 words[2][MAX_N + 8];
static u8 outs[2][4][MAX_N + 16];

static int test_clip(void)
{
    for (u32 c = 0; c < CASES; ++c) {
        u32 blocks = 1 + below(12), level = 200 + below(56), n = blocks * 16;
        for (u32 i = 0; i < n; ++i) {
            bytes[0][0][i] = (u8)(level - 8 + below(16));
            bytes[0][1][i] = bytes[1][1][i] = (u8)below(200);
        }
        glow_clip(bytes[0][0], bytes[0][1], blocks, level);
        scalar_glow_clip(bytes[0][0], bytes[1][1], blocks, level);
        if (!same(bytes[0][1], bytes[1][1], n)) return 0;
    }
    return 1;
}

static int test_clip_total(void)
{
    for (u32 c = 0; c < CASES; ++c) {
        u32 blocks = 1 + below(12), n = blocks * 16;
        for (u32 i = 0; i < n; ++i) bytes[0][1][i] = bytes[1][1][i] = (u8)next();
        glow_clip_total(bytes[0][1], halves[0][0], blocks);
        scalar_glow_clip_total(bytes[1][1], halves[1][0], blocks);
        if (!same(bytes[0][1], bytes[1][1], n) || !same(halves[0][0], halves[1][0], blocks * 2)) return 0;
    }
    return 1;
}

static int test_rgb(void)
{
    for (u32 c = 0; c < CASES; ++c) {
        u32 n = 1 + below(MAX_N);
        for (u32 i = 0; i < n + 2; ++i) {
            bytes[0][0][i] = (u8)next();
            bytes[0][1][i] = (u8)next();
        }
        GlowRgb a = {outs[0][0], outs[0][1], outs[0][2], outs[0][3]};
        GlowRgb b = {outs[1][0], outs[1][1], outs[1][2], outs[1][3]};
        scalar_glow_rgb(bytes[0][0], bytes[0][1], &b, n);
        glow_rgb(bytes[0][0], bytes[0][1], &a, n);
        if (!same(a.r, b.r, n) || !same(a.g, b.g, n) || !same(a.b, b.b, n) || !same(a.top, b.top, n)) return 0;
    }
    return 1;
}

static int test_light(void)
{
    for (u32 c = 0; c < CASES; ++c) {
        u32 n = 1 + below(MAX_N), part = below(2048);
        GlowLight a, b;
        for (u32 j = 0; j < 7; ++j) {
            for (u32 i = 0; i < n; ++i) halves[0][j][i] = (u16)(below(4) ? below(65281) : 0);
            a.ring[j] = b.ring[j] = halves[0][j];
        }
        a.out = halves[1][0];
        b.out = halves[1][1];
        glow_light(&a, part, n);
        scalar_glow_light(&b, part, n);
        if (!same(a.out, b.out, n * 2)) return 0;
    }
    return 1;
}

static int test_ycc(void)
{
    for (u32 c = 0; c < CASES; ++c) {
        u32 n = 1 + below(MAX_N);
        for (u32 k = 0; k < 3; ++k)
            for (u32 i = 0; i < n; ++i) bytes[0][k][i] = (u8)next();
        GlowYcc a = {{bytes[0][0], bytes[0][1], bytes[0][2]}, {values[0][0], values[0][1], values[0][2]}};
        GlowYcc b = {{bytes[0][0], bytes[0][1], bytes[0][2]}, {values[1][0], values[1][1], values[1][2]}};
        glow_ycc(&a, n);
        scalar_glow_ycc(&b, n);
        for (u32 k = 0; k < 3; ++k)
            if (!same(a.ycc[k], b.ycc[k], n * 2)) return 0;
    }
    return 1;
}

static int test_luma(void)
{
    for (u32 c = 0; c < CASES; ++c) {
        u32 n = 1 + below(40);
        for (u32 i = 0; i < n * 4; ++i) bytes[0][0][i] = (u8)next();
        for (u32 i = 0; i <= n; ++i) {
            words[0][i] = words[1][i] = next();
            values[0][0][i] = values[1][0][i] = (s16)below(32641);
            values[0][1][i] = (s16)((int)below(1025) - 512);
        }
        glow_luma(words[0], values[0][0], values[0][1], n, bytes[0][0]);
        scalar_glow_luma(words[1], values[1][0], values[0][1], n, bytes[0][0]);
        if (!same(words[0], words[1], n * 4) || !same(values[0][0], values[1][0], (n + 1) * 2)) return 0;
    }
    return 1;
}

static int test_chroma(void)
{
    for (u32 c = 0; c < CASES; ++c) {
        u32 n = 1 + below(40);
        for (u32 i = 0; i < n * 4; ++i) bytes[0][0][i] = (u8)next();
        for (u32 i = 0; i <= n; ++i) {
            words[0][i] = words[1][i] = next();
            for (u32 k = 0; k < 2; ++k) {
                values[0][2 * k][i] = values[1][2 * k][i] = (s16)((int)below(32513) - 16256);
                values[0][2 * k + 1][i] = (s16)((int)below(1025) - 512);
            }
        }
        GlowChroma a = {values[0][0], values[0][1], values[0][2], values[0][3], bytes[0][0]};
        GlowChroma b = {values[1][0], values[0][1], values[1][2], values[0][3], bytes[0][0]};
        glow_chroma(words[0], &a, n);
        scalar_glow_chroma(words[1], &b, n);
        if (!same(words[0], words[1], n * 4) || !same(values[0][0], values[1][0], (n + 1) * 2) ||
            !same(values[0][2], values[1][2], (n + 1) * 2)) return 0;
    }
    return 1;
}

int parity_main(void)
{
    static const struct { const char *name; int (*run)(void); } tests[] = {
        {"glow_clip", test_clip}, {"glow_clip_total", test_clip_total}, {"glow_rgb", test_rgb},
        {"glow_light", test_light}, {"glow_ycc", test_ycc}, {"glow_luma", test_luma}, {"glow_chroma", test_chroma},
    };
    u32 failures = 0;
    for (u32 i = 0; i < sizeof tests / sizeof tests[0]; ++i)
        if (!tests[i].run()) {
            sys_write("differs: ");
            sys_write(tests[i].name);
            sys_write("\n");
            ++failures;
        }
    if (!failures) sys_write("glow kernels: NEON == C\n");
    return (int)failures;
}

void _start(void) { sys_exit((u32)parity_main()); }
