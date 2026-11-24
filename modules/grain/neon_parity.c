/* The NEON parity test of Grain (grain.c). It is not a source of the module:
   the ignored test synthetic_grain_film_neon_parity
   (crates/grkit-build/src/tests/modules.rs) builds it for 32-bit ARM with
   grain.c twice, once with NEON and once with GRAIN_SCALAR (C only, each
   global named scalar_*), and runs it with qemu-arm.

   Each case draws the Film-like grain on the same random planes in both
   builds: a main picture, a screennail, and a thumbnail, with 4:2:2 or 4:2:0
   chroma, a visible area that can start and end on an odd sample or row,
   each strength and size, color and monochrome. The planes must be equal
   byte for byte. Standard output names each case that differs, and the exit
   status is their number. */
#include "grkit.h"
#include "grkit_module.h"

void grain_draw(PictureSet *set, u32 level, u32 size);
void scalar_grain_draw(PictureSet *set, u32 level, u32 size);

/* What the core gives the module. */
const GrkitContext grkit_self = {0};
u32 setting_get(u32 handle) { return handle == SETTING_STYLE ? 1U : 2U; }
void picture_shot(const PictureSet *set, GrkitShot *shot)
{
    (void)set;
    shot->known = 0;
}
u32 divide(u32 n, u32 d)
{
    u32 q = 0, bit = 1;
    if (!d) return 0;
    while (d < n && !(d & 0x80000000U)) { d <<= 1; bit <<= 1; }
    while (bit) {
        if (n >= d) { n -= d; q |= bit; }
        d >>= 1;
        bit >>= 1;
    }
    return q;
}

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
void __aeabi_memclr8(void *d, __SIZE_TYPE__ n) { memset(d, 0, n); }

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

static u32 seed = 0x6c8e9cf5U;
static u32 next(void)
{
    seed ^= seed << 13;
    seed ^= seed >> 17;
    seed ^= seed << 5;
    return seed;
}
/* A random value below n, without a division: n is at most 65536. */
static u32 below(u32 n) { return (u32)(((next() >> 16) * n) >> 16); }

/* Main picture, screennail, and thumbnail: luma, then chroma. */
static const u32 sizes[3][2] = {{1200, 800}, {720, 480}, {160, 120}};
#define PLANE_BYTES (1200U * 800U * 2U + 720U * 480U * 2U + 160U * 120U * 2U)
static u8 memory[2][PLANE_BYTES];
static u8 scratch[GRKIT_PLANE_SCRATCH];

static PictureSet planes(u8 *base, u32 half, u32 mono, const u32 *area)
{
    PictureSet set = {0};
    u32 at = 0;
    for (u32 i = 0; i < 3; ++i) {
        u32 w = sizes[i][0], h = sizes[i][1];
        Plane *p = &set.planes[i];
        p->y = (u32)(base + at);
        p->c = p->y + w * h;
        p->width = w; p->height = h; p->pitch = w; p->half = half;
        p->left = 0; p->top = 0; p->right = w; p->bottom = h;
        at += w * h * 2;
    }
    /* The visible area of the main picture. */
    set.planes[0].left = area[0];
    set.planes[0].top = area[1];
    set.planes[0].right = sizes[0][0] - area[2];
    set.planes[0].bottom = sizes[0][1] - area[3];
    set.count = 3;
    set.mono = mono;
    set.scratch = scratch;
    return set;
}

static int same(const u8 *a, const u8 *b, u32 n)
{
    while (n--)
        if (*a++ != *b++) return 0;
    return 1;
}

static char *put(char *o, u32 v)
{
    char digits[10];
    u32 n = 0;
    do { digits[n++] = (char)('0' + v % 10U); v /= 10U; } while (v);
    while (n) *o++ = digits[--n];
    return o;
}

#define CASES 32U

int parity_main(void)
{
    u32 failures = 0;
    for (u32 c = 0; c < CASES; ++c) {
        u32 level = 1 + c % 4U, size = 1 + below(3), half = below(2), mono = below(5) == 0;
        u32 area[4] = {0, 0, 0, 0};
        if (c & 1U)
            for (u32 i = 0; i < 4; ++i) area[i] = below(70);
        for (u32 i = 0; i < PLANE_BYTES; ++i) memory[0][i] = memory[1][i] = (u8)next();
        {
            PictureSet a = planes(memory[0], half, mono, area), b = planes(memory[1], half, mono, area);
            grain_draw(&a, level, size);
            scalar_grain_draw(&b, level, size);
        }
        if (!same(memory[0], memory[1], PLANE_BYTES)) {
            char text[96], *o = text;
            const char *labels[5] = {"differs: case ", " level ", " size ", " half ", " mono "};
            u32 values[5] = {c, level, size, half, mono};
            for (u32 i = 0; i < 5; ++i) {
                for (const char *l = labels[i]; *l; ++l) *o++ = *l;
                o = put(o, values[i]);
            }
            *o++ = '\n';
            *o = 0;
            sys_write(text);
            ++failures;
        }
    }
    if (!failures) sys_write("film grain: NEON == C\n");
    return (int)failures;
}

void _start(void) { sys_exit((u32)parity_main()); }
