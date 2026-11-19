/* The NEON parity test of the grain noise (runtime/grain_noise.h). The
   ignored test synthetic_grain_neon_parity
   (crates/grkit-build/src/tests/modules.rs) builds this file three times for
   32-bit ARM Linux and runs the program with qemu-arm:

   - with GRAIN_PARITY_PLANE=neon_plane: the noise with its assembly part;
   - with GRAIN_SCALAR and GRAIN_PARITY_PLANE=scalar_plane: the C code;
   - without them: the program, which fills planes with both.

   Each case fills a plane of one pitch, height, and seed for small, medium,
   and large grain, from each byte alignment. The planes must be equal byte
   for byte, and neither build may write outside its plane. Standard output
   names each case that differs, and the exit status is their number. */
typedef unsigned char u8;
typedef unsigned short u16;
typedef unsigned int u32;

/* n / d, as the core divides: d is not 0. */
static u32 core_divide(u32 n, u32 d)
{
    u32 q = 0, bit = 1;
    while (d < n && !(d & 0x80000000U)) { d <<= 1; bit <<= 1; }
    while (bit) {
        if (n >= d) { n -= d; q |= bit; }
        d >>= 1;
        bit >>= 1;
    }
    return q;
}
#ifdef GRAIN_PARITY_PLANE
#include "grain_noise.h"
void GRAIN_PARITY_PLANE(u8 *plane, u32 pitch, u32 height, u32 seed, u32 taps, u8 *work)
{
    grain_plane(plane, pitch, height, seed, taps, work);
}
#else
/* The program takes the sizes of the noise only. */
#define GRAIN_SCALAR 1
#include "grain_noise.h"
void neon_plane(u8 *plane, u32 pitch, u32 height, u32 seed, u32 taps, u8 *work);
void scalar_plane(u8 *plane, u32 pitch, u32 height, u32 seed, u32 taps, u8 *work);

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

static char *put(char *o, u32 v)
{
    char digits[10];
    u32 n = 0;
    do { digits[n++] = (char)('0' + v % 10U); v /= 10U; } while (v);
    while (n) *o++ = digits[--n];
    return o;
}
/* A row of 6000 samples passes the period of the tiles, as the rows of the
   camera do, and 3856 rows pass the period of the rows. */
static const u32 sizes[][2] = {
    {16, 1}, {16, 3}, {48, 5}, {64, 4}, {80, 2}, {240, 3}, {256, 3}, {272, 300},
    {3840, 2}, {3856, 2}, {6000, 3}, {16, 3856}, {32, 3841},
};
#define PLANE_BYTES (32U * 3841U)
#define GUARD 32U
#define GUARD_BYTE 0xa5U
static u8 memory[2][GUARD + 3U + PLANE_BYTES + GUARD];
static u8 work[2][GRAIN_NOISE_BYTES];

static int parity_main(void)
{
    static const u32 seeds[2] = {0x12345679U, 0xfeedc0deU};
    u32 failures = 0;
    for (u32 c = 0; c < sizeof(sizes) / sizeof(sizes[0]); ++c)
    for (u32 size = 0; size < 3; ++size)
    for (u32 align = 0; align < 4; ++align)
    for (u32 s = 0; s < 2; ++s) {
        u32 pitch = sizes[c][0], height = sizes[c][1], bytes = pitch * height, differs = 0;
        u32 taps = size == 1 ? 3 : 5;
        /* The long cases run from one alignment and with one seed, and the
           tiles take the second seed from one alignment. */
        if ((bytes > 20000U && (align || s)) || (size && align && s)) continue;
        for (u32 m = 0; m < 2; ++m) memset(memory[m], (int)GUARD_BYTE, sizeof(memory[m]));
        neon_plane(memory[0] + GUARD + align, pitch, height, seeds[s], taps, size ? work[0] : 0);
        scalar_plane(memory[1] + GUARD + align, pitch, height, seeds[s], taps, size ? work[1] : 0);
        for (u32 i = 0; i < sizeof(memory[0]); ++i) differs |= memory[0][i] != memory[1][i];
        for (u32 m = 0; m < 2; ++m) {
            for (u32 i = 0; i < GUARD + align; ++i) differs |= memory[m][i] != GUARD_BYTE;
            for (u32 i = GUARD + align + bytes; i < sizeof(memory[m]); ++i) differs |= memory[m][i] != GUARD_BYTE;
        }
        if (differs) {
            char text[96], *o = text;
            const char *labels[5] = {"differs: pitch ", " height ", " size ", " align ", " seed "};
            u32 values[5] = {pitch, height, size, align, s};
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
    if (!failures) sys_write("grain noise: NEON == C\n");
    return (int)failures;
}

void _start(void) { sys_exit((u32)parity_main()); }
#endif
