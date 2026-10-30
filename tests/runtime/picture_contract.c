#include <assert.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

/* One arena stands in for camera memory: records, the matrix object, and the planes. */
#define ARENA 0x59000000U
static uint8_t arena[0x600000];
static void *mapped(uint32_t address, uint32_t bytes)
{
    assert(address >= ARENA && address - ARENA + bytes <= sizeof(arena));
    assert(address % bytes == 0);
    return arena + (address - ARENA);
}
/* A whole region, such as the stamp masks: bounds only. */
static void *region(uint32_t address, uint32_t bytes)
{
    assert(address >= ARENA && address - ARENA + bytes <= sizeof(arena));
    return arena + (address - ARENA);
}
#define WORD(p,o) (*(volatile uint32_t *)mapped((p)+(o),4))
#define HALF(p,o) (*(volatile uint16_t *)mapped((p)+(o),2))
#define BYTE(p,o) (*(volatile uint8_t *)mapped((p)+(o),1))
/* The calc stores BT.601 as `lsr #13` results: negative values keep only their
   low bits, as the native multiply leaves them. */
static const int bt601[9] = {77, 150, 29, -43, -85, 128, 128, -107, -21};
static unsigned calcs;
static void calc(uint32_t object)
{
    ++calcs;
    for (int i = 0; i < 9; ++i) WORD(object, i * 4) = ((uint32_t)(bt601[i] * 8192 + 0x1000)) >> 13;
}
#define PICTURE_YCBCR_CALC(object,context) ((void)(context), calc(object))
static unsigned flushes, native_aspect;
#define PICTURE_CACHE_FLUSH(address,length) ((void)mapped((address), 1), (void)mapped((address) + (length) - 1, 1), ++flushes)
/* 2026-10-17 09:05 */
#define PICTURE_DATE(out) do { uint8_t *d = (uint8_t *)(out); d[0] = 2026 & 255; d[1] = 2026 >> 8; \
    d[2] = 10; d[3] = 17; d[4] = 9; d[5] = 5; } while (0)
#define PICTURE_NATIVE_ASPECT() native_aspect
/* Stamp mask scratch, clear of the planes. */
#define SCRATCH (ARENA + 0x500000)
#define PICTURE_SCRATCH(h) ((void)(h), SCRATCH)
#define PICTURE_MAP(address) ((u8 *)region((address), GRKIT_PICTURE_SCRATCH))
#define PICTURE_BARRIER() ((void)0)
/* Maker-note element writes: the entry, then the value of each element. */
static uint32_t ifd_entry, ifd_writes, ifd_values[32];
#define PICTURE_IFD_SET_SHORT(entry,index,value) do { assert((index) < 32 && !((value) >> 16)); \
    ifd_entry = (entry); ifd_values[(index)] = (value); ++ifd_writes; } while (0)
/* The module settings: those of Date Imprint, STYLE (handle 0, 6 values) and
   LOOK (handle 1, 3 values). */
#define TEST_SETTINGS 0x5f000000U
static uint32_t settings_memory[2 + 3 * 3];
static volatile uint32_t *mapped_settings(uint32_t address)
{
    assert(address == TEST_SETTINGS);
    return settings_memory;
}
#define MODULE_SETTINGS_MAP(address) mapped_settings(address)
#define STAMP_STYLE settings_memory[3]
#define STAMP_LOOK settings_memory[6]
#define STAMP_POSITION settings_memory[9]
#define DATE_STAMP_STYLE_MAX 5U
/* The plane filter of Date Imprint (modules/date): the harness maps its
   apply address to the module function. */
#define TEST_DATE_APPLY 0x5e000031U
static void test_apply(uint32_t address, void *set, uint32_t value);
#define MODULE_APPLY(address,set,value) test_apply(address,set,value)
/* The GR III noise fill: the native fill writes 0x77, the allocator hands out
   one block of the arena, and the timer count is a test value. */
#define GRAIN_PLANE (ARENA + 0x400000)
#define GRAIN_BLOCK (ARENA + 0x480000)
static unsigned grain_native_fills, grain_allocations, grain_releases, grain_no_memory;
static uint32_t grain_counter_value, grain_counter_step;
#define GRAIN_NATIVE_FILL(buffer,size) (++grain_native_fills, memset(region((buffer), (size)), 0x77, (size)))
#define GRAIN_ALLOCATE(size) (++grain_allocations, (void)region(GRAIN_BLOCK, (size)), grain_no_memory ? 0U : GRAIN_BLOCK)
#define GRAIN_RELEASE(block) (assert((block) == GRAIN_BLOCK), ++grain_releases)
#define GRAIN_MAP(address) ((u8 *)region((address), 1))
/* Each read of the timer advances it by grain_counter_step. */
#define GRAIN_COUNTER(high) (*(high) = 0, (grain_counter_value += grain_counter_step) - grain_counter_step)
#define GRAIN_COUNTER_MHZ 24U
#define TEST_PROFILE (ARENA + 0x4f0000)
#define PROFILE_MAP(address) ((volatile u32 *)region((address), 4))
#include "picture.c"

static volatile u32 header[GRKIT_HOST_HEADER_SIZE / 4];
volatile u32 *framing_header(void) { return header; }
/* The services of the core that the module calls (runtime/sdk/grkit.h). */
#define GRKIT_BYTES(address) ((volatile u8 *)mapped((address), 1))
#include "../../modules/date/stamp.c"
const GrkitContext grkit_self = {0, 2, 0};
u32 setting_get(u32 handle) { return setting_value(header, handle); }
void local_time(GrkitDateTime *out) { PICTURE_DATE(out); }
/* The live-view sign of the module (date_badge). This test checks pictures,
   so the label draws nothing. */
static void overlay_label(const Surface *s,u32 label,u32 left,u32 top) { (void)s; (void)label; (void)left; (void)top; }
u32 divide(u32 n, u32 d) { return core_divide(n, d); }
/* The turns of the last plane set that the date filter got. */
static uint32_t applied_turns = 0xffffffffU;
static void test_apply(uint32_t address, void *set, uint32_t value)
{
    assert(address == TEST_DATE_APPLY);
    applied_turns = ((PictureSet *)set)->turns;
    date_apply(set, value);
}
/* Register the plane filter of Date Imprint, as INSTALL does, or none. */
static void date_module(int loaded)
{
    header[GRKIT_HOST_MODULE_FILTER_COUNT / 4] = loaded ? 1 : 0;
    header[GRKIT_HOST_MODULE_FILTERS / 4] = TEST_DATE_APPLY;
    header[GRKIT_HOST_MODULE_FILTERS / 4 + 1] = GRKIT_STAGE_MARK << 16 | 0;
}

#define RECORDS (ARENA + 0x100)
#define OBJECT (ARENA + 0x8000)
#define CONTEXT (ARENA + 0x8100)
/* The Make's settings argument: the live-view byte lies at +0x818c. */
#define SETTINGS (ARENA + 0x20000)
static void live(int on) { BYTE(SETTINGS, PICTURE_LIVE_CONTEXT) = (uint8_t)on; }

static void reset(void)
{
    memset(arena, 0, sizeof(arena));
    memset((void *)header, 0, sizeof(header));
    header[GRKIT_HOST_COUNT / 4] = 3;
    header[GRKIT_HOST_RECORDS / 4] = RECORDS;
    /* Date Imprint is loaded, and its settings are 0. */
    memset(settings_memory, 0, sizeof(settings_memory));
    settings_memory[0] = 3; settings_memory[4] = 6; settings_memory[7] = 3; settings_memory[10] = 2;
    header[GRKIT_HOST_MODULE_SETTINGS / 4] = TEST_SETTINGS;
    date_module(1);
    /* The Framing module uses CORE.ASPECT, so a stored crop applies. */
    header[GRKIT_HOST_CAPABILITIES / 4] = 1U << (GRKIT_CORE_ASPECT - GRKIT_CORE_SETTING_BASE);
}
static uint32_t record(uint32_t index) { return RECORDS + index * GRKIT_RECORD_SIZE; }
static void post(uint32_t r, const short *matrix)
{
    WORD(r, GRKIT_RECORD_FEATURES) |= GRKIT_FEATURE_POST;
    for (int i = 0; i < 9; ++i) HALF(r, GRKIT_RECORD_POST + i * 2) = (uint16_t)matrix[i];
}
static int coefficient(int i) { return (int)(WORD(OBJECT, i * 4) << 23) >> 23; }
/* Keep red: the parameters that grkit_core::selective compiles from hue 108.6,
   lower 20/35, higher 6/14, and chroma 6/16 degrees and units. */
static void keep_red(uint32_t r)
{
    static const short words[6] = {-1306, 3882, 93, 179, 27, 64};
    WORD(r, GRKIT_RECORD_FEATURES) |= GRKIT_FEATURE_SELECTIVE;
    for (int i = 0; i < 6; ++i) HALF(r, GRKIT_RECORD_SELECTIVE + i * 2) = (uint16_t)words[i];
    BYTE(r, GRKIT_RECORD_SELECTIVE + 12) = 6;
    BYTE(r, GRKIT_RECORD_SELECTIVE + 13) = 16;
}

/* The coefficients of CSP x S x P, rounded as post_matrix rounds them. */
static void composed(const int *s, const short *p, int *out)
{
    int sp[9];
    for (int i = 0; i < 3; ++i) for (int j = 0; j < 3; ++j) {
        int sum = 0;
        for (int k = 0; k < 3; ++k) sum += s[i * 3 + k] * (p ? p[k * 3 + j] : (k == j) * 8192);
        sp[i * 3 + j] = (sum + 4096) >> 13;
    }
    for (int i = 0; i < 3; ++i) for (int j = 0; j < 3; ++j) {
        int sum = 0;
        for (int k = 0; k < 3; ++k) sum += bt601[i * 3 + k] * sp[k * 3 + j];
        sum = (sum + 4096) >> 13;
        out[i * 3 + j] = sum < -256 ? -256 : sum > 255 ? 255 : sum;
    }
}
/* A native look on GR III applies saturation after the tone curves, as GR IV
   does: the BT.601 matrices with 8-bit fractions, and 1.1 to the power of the
   level, rounded to three decimals. Other cameras keep the native matrix. */
static void native_saturation_contract(const short *swap)
{
    static const double rgb[9] = {1, 0, 359 / 256.0, 1, -88 / 256.0, -183 / 256.0, 1, 454 / 256.0, 0};
    static const double ycbcr[9] = {77 / 256.0, 150 / 256.0, 29 / 256.0, -43 / 256.0, -85 / 256.0,
        128 / 256.0, 128 / 256.0, -107 / 256.0, -21 / 256.0};
    for (int level = -4; level <= 4; ++level) {
        double factor = 1;
        for (int n = 0; n < (level < 0 ? -level : level); ++n) factor = level < 0 ? factor / 1.1 : factor * 1.1;
        factor = (double)(long)(factor * 1000 + 0.5) / 1000;
        int s[9], expected[9];
        for (int i = 0; i < 3; ++i) for (int j = 0; j < 3; ++j) {
            double v = 512 * (rgb[i * 3] * ycbcr[j] + factor * (rgb[i * 3 + 1] * ycbcr[3 + j] + rgb[i * 3 + 2] * ycbcr[6 + j]));
            s[i * 3 + j] = (int)(v < 0 ? v - 0.5 : v + 0.5) * 16;
        }
        /* The default is -2 (7 mod 9), and the stored change moves it to the level. */
        reset();
        WORD(CONTEXT, 8) = record(1);
        WORD(record(1), GRKIT_RECORD_FEATURES) = GRKIT_FEATURE_NATIVE_LOOK;
        BYTE(record(1), GRKIT_RECORD_FN_DEFAULTS) = 7;
        BYTE(record(1), GRKIT_RECORD_PARAMETERS) = (uint8_t)((level + 2 + 9) % 9);
        picture_post(OBJECT, CONTEXT, SETTINGS);
#ifdef NATIVE_SATURATION_BEFORE_CURVES
        composed(s, 0, expected);
        for (int i = 0; i < 9; ++i) assert(coefficient(i) == (level ? expected[i] : bt601[i]));
        /* The saturation follows the post matrix of the effect. */
        post(record(1), swap);
        picture_post(OBJECT, CONTEXT, SETTINGS);
        composed(s, swap, expected);
        for (int i = 0; i < 9; ++i) assert(level ? coefficient(i) == expected[i] : coefficient(i) == bt601[i / 3 * 3 + 2 - i % 3]);
#else
        for (int i = 0; i < 9; ++i) assert(coefficient(i) == bt601[i]);
#endif
    }
    /* Effects without a native look keep the native saturation. */
    reset();
    WORD(CONTEXT, 8) = record(1);
    BYTE(record(1), GRKIT_RECORD_FN_DEFAULTS) = 7;
    picture_post(OBJECT, CONTEXT, SETTINGS);
    for (int i = 0; i < 9; ++i) assert(coefficient(i) == bt601[i]);
}

/* Encode configuration: four image and four code descriptors, then the planes. */
#define MESSAGE (ARENA + 0x8300)
#define CONFIG (ARENA + 0x8400)
#define DESCRIPTORS (ARENA + 0x8500)
#define PLANES (ARENA + 0x10000)
struct test_plane { uint32_t width, height, pitch, y, c; };
static struct test_plane planes[3];
/* Plane baseline: luma, then Cb at even and Cr at odd bytes of the CbCr plane. */
static uint8_t base_y = 60, base_cb = 100, base_cr = 150;
static void picture(uint32_t index, uint32_t width, uint32_t height, uint32_t pitch, uint32_t at)
{
    uint32_t image = DESCRIPTORS + index * 0x20, code = DESCRIPTORS + 0x100 + index * 0x20;
    planes[index] = (struct test_plane){width, height, pitch, at, at + pitch * height};
    base_y = 60; base_cb = 100; base_cr = 150;
    WORD(image, 4) = at; WORD(image, 8) = at + pitch * height;
    HALF(image, 0x10) = (uint16_t)width; HALF(image, 0x12) = (uint16_t)height; HALF(image, 0x14) = (uint16_t)pitch;
    BYTE(code, 0x10) = 1;
    WORD(CONFIG, 0x3c + index * 4) = image; WORD(CONFIG, 0x4c + index * 4) = code;
    for (uint32_t y = 0; y < height; ++y) for (uint32_t x = 0; x < pitch; ++x) {
        BYTE(at, y * pitch + x) = 60;
        BYTE(at + pitch * height, y * pitch + x) = (uint8_t)(x & 1 ? 150 : 100);
    }
}
/* The shot settings pointer of the encoded pictures. */
#define SHOT (ARENA + 0x9000)
/* Record r and the posture of the electronic level for a shot in the shot
   ring, as the still build stores them. */
static void store_posed(uint32_t shot, uint32_t r, uint32_t posture)
{
    uint32_t ring = SCRATCH + GRKIT_SHOT_RING, n = WORD(ring, 0);
    uint32_t entry = ring + GRKIT_SHOT_ENTRY + (n & (GRKIT_SHOT_ENTRIES - 1)) * GRKIT_SHOT_STRIDE;
    WORD(entry, GRKIT_SHOT_KEY) = shot;
    WORD(entry, GRKIT_SHOT_RECORD) = r;
    WORD(entry, GRKIT_SHOT_POSTURE) = posture;
    WORD(ring, 0) = n + 1;
}
static void store_shot(uint32_t shot, uint32_t r) { store_posed(shot, r, 1); }
static void still(uint32_t r) { store_shot(SHOT, r); }
static void encode_setup(void)
{
    reset();
    WORD(MESSAGE, 8) = CONFIG;
    WORD(CONFIG, 0x34) = SHOT;
    picture(0, 600, 400, 640, PLANES);
    picture(1, 720, 480, 736, PLANES + 0x80000);
    picture(2, 160, 120, 160, PLANES + 0x140000);
    /* Descriptor 3 is the second full-size encode of the main planes. */
    WORD(CONFIG, 0x48) = WORD(CONFIG, 0x3c); WORD(CONFIG, 0x58) = WORD(CONFIG, 0x4c);
    flushes = 0; native_aspect = 0;
}
static void fill_planes(uint8_t luma, uint8_t cb, uint8_t cr)
{
    for (u32 i = 0; i < 3; ++i) {
        const struct test_plane *p = &planes[i];
        for (u32 y = 0; y < p->height; ++y) for (u32 x = 0; x < p->pitch; ++x) {
            BYTE(p->y, y * p->pitch + x) = luma;
            BYTE(p->c, y * p->pitch + x) = x & 1 ? cr : cb;
        }
    }
    base_y = luma; base_cb = cb; base_cr = cr;
}
/* Changed pixels: their bounding box, whether all are lighter (and keep their
   chroma for a monochrome effect), whether a lit pair is orange, and the
   largest luma change. */
struct box { uint32_t left, top, right, bottom, lit; int lighter, orange; uint32_t lift; int yellow; };
static struct box changes(uint32_t index, int mono)
{
    const struct test_plane *p = &planes[index];
    struct box b = {p->width, p->height, 0, 0, 0, 1, 0, 0};
    for (uint32_t y = 0; y < p->height; ++y) for (uint32_t x = 0; x < p->pitch; ++x) {
        uint8_t luma = BYTE(p->y, y * p->pitch + x), chroma = BYTE(p->c, y * p->pitch + x);
        uint8_t stock = x & 1 ? base_cr : base_cb;
        if (luma == base_y && chroma == stock) continue;
        if (luma < base_y) b.lighter = 0;
        if (mono && chroma != stock) b.lighter = 0;
        if (luma - base_y > (int)b.lift) b.lift = luma - base_y;
        if (!(x & 1) && luma >= 110 && chroma <= 105 && BYTE(p->c, y * p->pitch + x + 1) >= 165) b.orange = 1;
        /* GR1s yellow: bright, low Cb, Cr a little above neutral. */
        if (!(x & 1) && luma >= 150 && chroma <= 90 && BYTE(p->c, y * p->pitch + x + 1) >= 130 && BYTE(p->c, y * p->pitch + x + 1) < 165) b.yellow = 1;
        if (x < b.left) b.left = x;
        if (y < b.top) b.top = y;
        if (x + 1 > b.right) b.right = x + 1;
        if (y + 1 > b.bottom) b.bottom = y + 1;
        ++b.lit;
    }
    return b;
}

#ifdef GRAIN_FILL_PITCH
/* An independent model of the fill: four xorshift128 generators seeded with
   splitmix32, then a 2D binomial filter with the edge samples repeated. */
static uint32_t ref_mix(uint32_t *x)
{
    uint32_t z = (*x += 0x9e3779b9U);
    z = (z ^ (z >> 16)) * 0x85ebca6bU;
    z = (z ^ (z >> 13)) * 0xc2b2ae35U;
    return z ^ (z >> 16);
}
/* The four xorshift128 generators of the fill. */
struct ref_state { uint32_t a[4], b[4], c[4], d[4]; };
static void ref_seed(struct ref_state *s, uint32_t seed)
{
    for (int i = 0; i < 4; ++i) { s->a[i] = ref_mix(&seed); s->b[i] = ref_mix(&seed); s->c[i] = ref_mix(&seed); s->d[i] = ref_mix(&seed) | 1U; }
}
static void ref_fill(struct ref_state *s, uint8_t *plane, uint32_t pitch, uint32_t height)
{
    for (uint32_t y = 0; y < height; ++y)
        for (uint32_t x = 0; x < pitch; x += 16)
            for (int i = 0; i < 4; ++i) {
                uint32_t t = s->a[i] ^ (s->a[i] << 11);
                s->a[i] = s->b[i]; s->b[i] = s->c[i]; s->c[i] = s->d[i];
                s->d[i] = s->d[i] ^ (s->d[i] >> 19) ^ t ^ (t >> 8);
                memcpy(plane + y * pitch + x + 4 * i, &s->d[i], 4);
            }
}
static void ref_random(uint8_t *plane, uint32_t pitch, uint32_t height, uint32_t seed)
{
    struct ref_state s;
    ref_seed(&s, seed);
    ref_fill(&s, plane, pitch, height);
}
/* The blur of an n x n tile with its edges wrapped, at half the deviation. */
static void ref_tile(const uint8_t *in, uint8_t *out, int n, uint32_t taps)
{
    static const int k3[3] = {1, 2, 1}, k5[5] = {1, 4, 6, 4, 1};
    const int *k = taps == 3 ? k3 : k5, half = (int)taps / 2, mean = 128 * (taps == 3 ? 16 : 256);
    const int gain = taps == 3 ? 5461 : 468;
    for (int y = 0; y < n; ++y)
        for (int x = 0; x < n; ++x) {
            int sum = 0;
            for (int j = -half; j <= half; ++j)
                for (int i = -half; i <= half; ++i)
                    sum += k[j + half] * k[i + half] * in[((y + j + n) % n) * n + (x + i + n) % n];
            int v = 128 + ((2 * (sum - mean) * gain + 65536) >> 17);
            out[y * n + x] = (uint8_t)(v < 0 ? 0 : v > 255 ? 255 : v);
        }
}
/* Each row, then each column, of a tile moved to the mean 128: the mean of
   its deviations, rounded half away from zero, comes off each sample, which
   stays in 0..255. */
static void ref_center(uint8_t *tile, int n)
{
    for (int pass = 0; pass < 2; ++pass)
        for (int a = 0; a < n; ++a) {
            int s = 0;
            for (int b = 0; b < n; ++b) s += tile[pass ? b * n + a : a * n + b] - 128;
            int d = s < 0 ? -((-s + n / 2) / n) : (s + n / 2) / n;
            for (int b = 0; b < n; ++b) {
                uint8_t *v = &tile[pass ? b * n + a : a * n + b];
                int w = *v - d;
                *v = (uint8_t)(w < 0 ? 0 : w > 255 ? 255 : w);
            }
        }
}
/* Medium and large grain: tiles 256 and 240 wide from the random bytes in
   that order, then the plane of their sum, each tile from a place that one
   splitmix32 value of the last state word of generator 0 gives: the start
   row from bits 8 and up, the start column in 16-sample blocks from bits 0
   to 7. */
enum { REF_A = 256, REF_B = 240 };
static void ref_grain(uint8_t *plane, uint32_t pitch, uint32_t height, uint32_t taps, uint32_t seed)
{
    static uint8_t ra[REF_A * REF_A], rb[REF_B * REF_B], ta[REF_A * REF_A], tb[REF_B * REF_B];
    struct ref_state s;
    ref_seed(&s, seed);
    ref_fill(&s, ra, REF_A, REF_A);
    ref_fill(&s, rb, REF_B, REF_B);
    ref_tile(ra, ta, REF_A, taps);
    ref_tile(rb, tb, REF_B, taps);
    ref_center(ta, REF_A);
    ref_center(tb, REF_B);
    uint32_t place = s.d[0], pa = ref_mix(&place), pb = ref_mix(&place);
    uint32_t ya = (pa >> 8) % REF_A, xa = (pa & 255) % (REF_A / 16) * 16;
    uint32_t yb = (pb >> 8) % REF_B, xb = (pb & 255) % (REF_B / 16) * 16;
    for (uint32_t y = 0; y < height; ++y)
        for (uint32_t x = 0; x < pitch; ++x) {
            int a = ta[(ya + y) % REF_A * REF_A + (xa + x) % REF_A];
            int b = tb[(yb + y) % REF_B * REF_B + (xb + x) % REF_B];
            int s = a + b - 256, v = 128 + s + ((2 * s * 13573 + 32768) >> 16);
            plane[y * pitch + x] = (uint8_t)(v < 0 ? 0 : v > 255 ? 255 : v);
        }
}
/* Mean, standard deviation, and the correlation of horizontal neighbours. */
static void grain_stats(const uint8_t *p, uint32_t n, double *mean, double *std, double *corr)
{
    double s = 0, ss = 0, c = 0;
    for (uint32_t i = 0; i < n; ++i) { s += p[i]; ss += (double)p[i] * p[i]; }
    *mean = s / n;
    *std = ss / n - *mean * *mean;
    for (uint32_t i = 0; i + 1 < n; ++i) c += (p[i] - *mean) * (p[i + 1] - *mean);
    *corr = c / (n - 1) / *std;
    *std = __builtin_sqrt(*std);
}
static void grain_contract(void)
{
    enum { PITCH = 1024, HEIGHT = 64, SIZE = PITCH * HEIGHT };
    static uint8_t want[SIZE], noise[SIZE];
    uint8_t *plane = region(GRAIN_PLANE, SIZE);
    double mean, std, corr;
    grain_counter_value = 0x12345678U;
    /* The native noise for a shot without GRKit grain, and for a plane that
       the fill cannot take. */
    for (int c = 0; c < 4; ++c) {
        static const uint32_t shot[4] = {0, 4, 2, 2}, pitch[4] = {PITCH, PITCH, 1000, PITCH}, size[4] = {SIZE, SIZE, 1000 * HEIGHT, SIZE - 1};
        header[GRKIT_HOST_GRAIN_SHOT / 4] = shot[c];
        grain_native_fills = 0;
        grain_noise(GRAIN_PLANE, size[c], pitch[c], HEIGHT);
        assert(grain_native_fills == 1 && plane[0] == 0x77);
    }
    /* Small: one random byte each, the same bytes as the model, which are
       uniform and unrelated to their neighbours. */
    header[GRKIT_HOST_GRAIN_SHOT / 4] = 1;
    grain_native_fills = grain_allocations = 0;
    grain_noise(GRAIN_PLANE, SIZE, PITCH, HEIGHT);
    ref_random(noise, PITCH, HEIGHT, 0x12345678U);
    assert(!grain_native_fills && !grain_allocations && !memcmp(plane, noise, SIZE));
    grain_stats(plane, SIZE, &mean, &std, &corr);
    assert(mean > 126 && mean < 129 && std > 72 && std < 76 && corr > -0.02 && corr < 0.02);
    /* Another timer count gives another pattern. */
    grain_counter_value = 0x12345679U;
    grain_noise(GRAIN_PLANE, SIZE, PITCH, HEIGHT);
    assert(memcmp(plane, noise, SIZE));
    /* Medium and large: the sum of two wrapped tiles of the model's filter.
       The noise has about the standard deviation of one random byte, wider
       grain, and no seam: neighbours across a 128-sample line correlate as
       the others do. */
    for (uint32_t shot = 2; shot <= 3; ++shot) {
        uint32_t taps = shot == 2 ? 3 : 5;
        header[GRKIT_HOST_GRAIN_SHOT / 4] = shot;
        grain_allocations = grain_releases = 0;
        grain_noise(GRAIN_PLANE, SIZE, PITCH, HEIGHT);
        assert(grain_allocations == 1 && grain_releases == 1);
        ref_grain(want, PITCH, HEIGHT, taps, 0x12345679U);
        assert(!memcmp(plane, want, SIZE));
        grain_stats(plane, SIZE, &mean, &std, &corr);
        /* Each tile row and column has the mean 128, so the plane does too,
           and no row repeats a mean along its length as a line. Rows of
           tiles without that keep a mean with an RMS of 7 to 8 here; the
           rest comes from the part of the last 240-sample repeat. */
        assert(mean > 127 && mean < 129 && std > 66 && std < 76);
        double rows = 0;
        for (uint32_t y = 0; y < HEIGHT; ++y) {
            double row = 0;
            for (uint32_t x = 0; x < PITCH; ++x) row += plane[y * PITCH + x];
            row = row / PITCH - mean;
            rows += row * row;
        }
        assert(__builtin_sqrt(rows / HEIGHT) < 2);
        assert(taps == 3 ? corr > 0.6 && corr < 0.72 : corr > 0.74 && corr < 0.84);
        double seam = 0, other = 0;
        for (uint32_t y = 0; y < HEIGHT; ++y)
            for (uint32_t x = 0; x + 1 < PITCH; ++x) {
                double d = ((double)plane[y * PITCH + x] - mean) * ((double)plane[y * PITCH + x + 1] - mean);
                if (x % 128 == 127) seam += d; else other += d;
            }
        seam /= HEIGHT * (PITCH / 128); other /= HEIGHT * (PITCH - 1 - PITCH / 128);
        assert(seam > other * 0.8 && seam < other * 1.2);
    }
    /* A pitch that is not a multiple of 64 or of a tile, and fewer rows
       than taps: the last generator steps and the tile wrap. */
    for (uint32_t shot = 1; shot <= 3; ++shot) {
        static const uint32_t pitch = 48, rows[2] = {7, 2};
        for (int h = 0; h < 2; ++h) {
            uint32_t size = pitch * rows[h];
            header[GRKIT_HOST_GRAIN_SHOT / 4] = shot;
            grain_noise(GRAIN_PLANE, size, pitch, rows[h]);
            if (shot == 1) ref_random(noise, pitch, rows[h], 0x12345679U);
            else ref_grain(noise, pitch, rows[h], shot == 2 ? 3 : 5, 0x12345679U);
            assert(!memcmp(plane, noise, size));
        }
    }
    /* The profile counts the microseconds of the generic timer: two reads
       24000 ticks apart at 24 MHz are 1000 us. Without the profile, nothing
       is counted. */
    memset(region(TEST_PROFILE, GRKIT_PROFILE_SIZE), 0, GRKIT_PROFILE_SIZE);
    volatile u32 *slot = PROFILE_MAP(TEST_PROFILE + GRKIT_PROFILE_SLOTS + GRKIT_PROFILE_PICTURE_GRAIN * GRKIT_PROFILE_SLOT_SIZE);
    header[GRKIT_HOST_GRAIN_SHOT / 4] = 3;
    grain_counter_step = 24000;
    grain_noise(GRAIN_PLANE, SIZE, PITCH, HEIGHT);
    assert(slot[0] == 0);
    header[GRKIT_HOST_PROFILE / 4] = TEST_PROFILE;
    grain_noise(GRAIN_PLANE, SIZE, PITCH, HEIGHT);
    assert(slot[0] == 1 && slot[1] == 1000 && slot[2] == 1000);
    header[GRKIT_HOST_PROFILE / 4] = 0;
    grain_counter_step = 0;
    grain_counter_value = 0x12345679U;
    /* Without memory for the row sums, a larger size gives small grain. */
    grain_no_memory = 1;
    grain_releases = 0;
    grain_noise(GRAIN_PLANE, SIZE, PITCH, HEIGHT);
    ref_random(noise, PITCH, HEIGHT, 0x12345679U);
    assert(!grain_releases && !memcmp(plane, noise, SIZE));
    grain_no_memory = 0;
    header[GRKIT_HOST_GRAIN_SHOT / 4] = 0;
}
#endif

int main(void)
{
#ifdef GRAIN_FILL_PITCH
    grain_contract();
#endif
    static const short identity[9] = {8192, 0, 0, 0, 8192, 0, 0, 0, 8192};
    /* Native records and GRFX0001 records keep the native matrix bits. */
    reset();
    WORD(CONTEXT, 8) = 0x5501900c;
    picture_post(OBJECT, CONTEXT, SETTINGS);
    assert(calcs == 1);
    for (int i = 0; i < 9; ++i) assert(coefficient(i) == bt601[i]);
    WORD(CONTEXT, 8) = record(1);
    uint32_t stock[9];
    picture_post(OBJECT, CONTEXT, SETTINGS);
    for (int i = 0; i < 9; ++i) stock[i] = WORD(OBJECT, i * 4);
    /* An identity post matrix gives the same coefficients, sign-extended. */
    post(record(1), identity);
    picture_post(OBJECT, CONTEXT, SETTINGS);
    for (int i = 0; i < 9; ++i) {
        assert(coefficient(i) == bt601[i]);
        assert((WORD(OBJECT, i * 4) & 0x1ff) == (stock[i] & 0x1ff));
    }
    /* CSP x P: P swaps red and blue, so each row swaps its R and B columns. */
    static const short swap[9] = {0, 0, 8192, 0, 8192, 0, 8192, 0, 0};
    post(record(1), swap);
    picture_post(OBJECT, CONTEXT, SETTINGS);
    for (int row = 0; row < 3; ++row) {
        assert(coefficient(row * 3) == bt601[row * 3 + 2]);
        assert(coefficient(row * 3 + 1) == bt601[row * 3 + 1]);
        assert(coefficient(row * 3 + 2) == bt601[row * 3]);
    }
    /* Q13 rounding to nearest: 77 x 1.5 = 115.5 rounds to 116, -43 x 1.5 to -64. */
    static const short gain[9] = {12288, 0, 0, 0, 8192, 0, 0, 0, 8192};
    post(record(1), gain);
    picture_post(OBJECT, CONTEXT, SETTINGS);
    assert(coefficient(0) == 116 && coefficient(3) == -64 && coefficient(6) == 192);
    /* Results saturate to the 9-bit register range instead of wrapping. */
    static const short strong[9] = {32767, 0, 0, 0, 32767, 0, 0, 0, -32768};
    post(record(1), strong);
    picture_post(OBJECT, CONTEXT, SETTINGS);
    assert(coefficient(0) == 255 && coefficient(1) == 255 && coefficient(2) == -116);
    assert(coefficient(4) == -256 && coefficient(5) == -256 && coefficient(6) == 255);
    assert((int32_t)WORD(OBJECT, 4 * 4) == -256);
    /* Only owned records count: an address between records or past the catalog. */
    WORD(CONTEXT, 8) = record(1) + 4;
    picture_post(OBJECT, CONTEXT, SETTINGS);
    for (int i = 0; i < 9; ++i) assert(coefficient(i) == bt601[i]);
    WORD(record(3), GRKIT_RECORD_FEATURES) = GRKIT_FEATURE_POST;
    WORD(CONTEXT, 8) = record(3);
    picture_post(OBJECT, CONTEXT, SETTINGS);
    for (int i = 0; i < 9; ++i) assert(coefficient(i) == bt601[i]);

    native_saturation_contract(swap);

    /* Selective color: the live view drops the chroma rows, so the viewfinder shows
       the luma that removed hues keep. Still builds keep the whole matrix. */
    reset();
    WORD(CONTEXT, 8) = record(1);
    keep_red(record(1));
    live(0);
    picture_post(OBJECT, CONTEXT, SETTINGS);
    for (int i = 0; i < 9; ++i) assert(coefficient(i) == bt601[i]);
    live(1);
    picture_post(OBJECT, CONTEXT, SETTINGS);
    for (int i = 0; i < 9; ++i) assert(coefficient(i) == (i < 3 ? bt601[i] : 0));
    /* With a post matrix, the luma row keeps the product. */
    post(record(1), swap);
    picture_post(OBJECT, CONTEXT, SETTINGS);
    assert(coefficient(0) == bt601[2] && coefficient(1) == bt601[1] && coefficient(2) == bt601[0]);
    for (int i = 3; i < 9; ++i) assert(coefficient(i) == 0);
    /* Other live-view records keep their chroma. */
    WORD(CONTEXT, 8) = record(1) + 4;
    picture_post(OBJECT, CONTEXT, SETTINGS);
    for (int i = 0; i < 9; ++i) assert(coefficient(i) == bt601[i]);
    WORD(CONTEXT, 8) = record(2);
    picture_post(OBJECT, CONTEXT, SETTINGS);
    for (int i = 0; i < 9; ++i) assert(coefficient(i) == bt601[i]);
    live(0);

    /* Date text for each style: no leading zeros except the year and minutes.
       The GR1s look pads a one-digit month, day, or hour with a blank digit
       cell ('_'), as its prints show. */
    {
        static const char *const expected[] = {"'26 10 17", "'26 10 17  9:05", "10 17 '26", "17 10 '26", "17 10 '26  9:05"};
        static const char *const early[] = {"'07 1 5", "'07 1 5  13:07", "1 5 '07", "5 1 '07", "5 1 '07  13:07"};
        static const char *const padded[] = {"'26 10 17", "'26 10 17  _9:05", "10 17 '26", "17 10 '26", "17 10 '26  _9:05"};
        static const char *const early_padded[] = {"'07 _1 _5", "'07 _1 _5  13:07", "_1 _5 '07", "_5 _1 '07", "_5 _1 '07  13:07"};
        static const uint8_t date[6] = {2026 & 255, 2026 >> 8, 10, 17, 9, 5};
        static const uint8_t january[6] = {2007 & 255, 2007 >> 8, 1, 5, 13, 7};
        char text[20];
        for (u32 style = 1; style <= DATE_STAMP_STYLE_MAX; ++style) {
            u32 n = date_text(text, date, style, 0);
            assert(n == strlen(expected[style - 1]) && !memcmp(text, expected[style - 1], n));
            n = date_text(text, january, style, 0);
            assert(n == strlen(early[style - 1]) && !memcmp(text, early[style - 1], n));
            n = date_text(text, date, style, 1);
            assert(n == strlen(padded[style - 1]) && !memcmp(text, padded[style - 1], n));
            n = date_text(text, january, style, 1);
            assert(n == strlen(early_padded[style - 1]) && !memcmp(text, early_padded[style - 1], n));
        }
    }
    /* Off touches nothing. */
    encode_setup();
    picture_encode(0, MESSAGE);
    assert(flushes == 0);
    for (u32 i = 0; i < 3; ++i) assert(changes(i, 0).lit == 0);
    /* On a dark neutral picture the stamp lights an orange core at the bottom
       right of the visible area, once per plane set, and only lightens. */
    for (u32 style = 1; style <= DATE_STAMP_STYLE_MAX; ++style) {
        encode_setup();
        fill_planes(40, 128, 128);
        STAMP_STYLE = style;
        picture_encode(0, MESSAGE);
        assert(flushes == 12);
        for (u32 i = 0; i < 3; ++i) {
            const struct test_plane *p = &planes[i];
            u32 bottom = p->height - (i == 2 ? 8 : 0);
            struct box b = changes(i, 0);
            assert(b.lit && b.lighter);
            assert(i == 2 || b.orange);
            assert(b.right <= p->width && b.bottom <= bottom);
            assert(b.left > p->width / 3 && b.top > p->height / 2);
        }
    }
    /* The turns of a shot follow the posture of the electronic level at its
       still build: 1 landscape, 3 and 4 the vertical postures, 2 upside
       down. With the lens up or down (5, 6), the last turns stay. */
    {
        static const u32 posture[7] = {3, 6, 2, 5, 4, 1, 6};
        static const u32 turns[7] = {1, 1, 2, 2, 3, 0, 0};
        encode_setup();
        fill_planes(40, 128, 128);
        STAMP_STYLE = 1;
        for (u32 k = 0; k < 7; ++k) {
            store_posed(SHOT, 0, posture[k]);
            picture_encode(0, MESSAGE);
            assert(applied_turns == turns[k]);
        }
        /* The ring holds noise after power-on: without a known posture, a
           shot gets no turns. */
        encode_setup();
        fill_planes(40, 128, 128);
        STAMP_STYLE = 1;
        WORD(SCRATCH + GRKIT_SHOT_RING, GRKIT_SHOT_LAST_TURNS) = 0x77;
        store_posed(SHOT, 0, 6);
        picture_encode(0, MESSAGE);
        assert(applied_turns == 0);
    }
    /* Upright position: on each plane the stamp goes to the bottom right of
       the picture turned upright, which is the top right of the frame for
       one quarter turn, and its text runs along the long side of its box.
       The film position keeps it at the bottom right of the frame. */
    for (u32 position = 0; position < 2; ++position)
        for (u32 k = 0; k < 4; ++k) {
            static const u32 posture_of[4] = {1, 3, 2, 4};
            u32 corner = position ? k : 0;
            encode_setup();
            fill_planes(40, 128, 128);
            STAMP_STYLE = 2;
            STAMP_POSITION = position;
            store_posed(SHOT, 0, posture_of[k]);
            picture_encode(0, MESSAGE);
            for (u32 i = 0; i < 3; ++i) {
                const struct test_plane *p = &planes[i];
                struct box b = changes(i, 0);
                assert(b.lit && b.lighter);
                assert((corner <= 1) == (b.left > p->width / 2));
                assert((corner >= 1 && corner <= 2) == (b.bottom < p->height / 2));
            }
            {
                struct box b = changes(0, 0);
                assert((corner & 1) == (b.bottom - b.top > b.right - b.left));
            }
        }
    /* Longer styles reach further left. */
    encode_setup();
    fill_planes(40, 128, 128);
    STAMP_STYLE = 1;
    picture_encode(0, MESSAGE);
    u32 short_left = changes(0, 0).left;
    encode_setup();
    fill_planes(40, 128, 128);
    STAMP_STYLE = 2;
    picture_encode(0, MESSAGE);
    struct box long_box = changes(0, 0);
    assert(long_box.left < short_left);
    /* Day first with time has the same characters as style 2, so its mask has
       the same size and it fits wherever style 2 fits. Only the first glyph
       differs: an apostrophe for style 2, a digit for style 5. */
    encode_setup();
    fill_planes(40, 128, 128);
    STAMP_STYLE = 5;
    picture_encode(0, MESSAGE);
    {
        struct box b = changes(0, 0);
        assert(b.right == long_box.right && b.top == long_box.top && b.bottom == long_box.bottom);
        assert(abs((int)b.left - (int)long_box.left) <= 8);
    }
    /* A screen blend fades on bright areas, as an exposure on film does. */
    encode_setup();
    fill_planes(40, 128, 128);
    STAMP_STYLE = 1;
    picture_encode(0, MESSAGE);
    u32 dark_lift = changes(0, 0).lift;
    encode_setup();
    fill_planes(240, 128, 128);
    STAMP_STYLE = 1;
    picture_encode(0, MESSAGE);
    {
        struct box b = changes(0, 0);
        assert(b.lighter && b.lift <= 15 && dark_lift >= 100);
    }
    /* 16:9 Framing (a crop also where the camera has 16:9): the screennail
       stamp stays above the bottom bar. */
    encode_setup();
    fill_planes(40, 128, 128);
    STAMP_STYLE = 3;
    header[GRKIT_HOST_FRAME_RATIO / 4] = 2;
    picture_encode(0, MESSAGE);
    {
        struct box b = changes(1, 0);
        assert(b.lit && b.bottom <= 480 - 38 && b.top > 240);
    }
#ifndef FRAMING_XPAN_NATIVE
    /* 2.35:1 Framing: above its bar, which is lower than the XPan bar. */
    encode_setup();
    fill_planes(40, 128, 128);
    STAMP_STYLE = 3;
    header[GRKIT_HOST_FRAME_RATIO / 4] = 4;
    picture_encode(0, MESSAGE);
    {
        struct box sn = changes(1, 0), th = changes(2, 0);
        assert(sn.lit && sn.bottom <= 480 - 87 && sn.bottom > 480 - 107 && sn.top > 240);
        assert(th.lit && th.bottom <= 120 - 26 && th.bottom > 120 - 30);
    }
#else
    /* GR IV: XPan applies to native 3:2 and native 16:9 captures. The
       screennail and thumbnail stamps stay above the bar of the capture. */
    {
        struct box sn[3][2], th[3][2];
        for (u32 native = 0; native <= 2; native += 2)
            for (u32 ratio = 0; ratio <= 1; ++ratio) {
                encode_setup();
                fill_planes(40, 128, 128);
                STAMP_STYLE = 3;
                header[GRKIT_HOST_FRAME_RATIO / 4] = ratio;
                native_aspect = native;
                picture_encode(0, MESSAGE);
                sn[native][ratio] = changes(1, 0);
                th[native][ratio] = changes(2, 0);
                assert(sn[native][ratio].lit && th[native][ratio].lit);
            }
        assert(sn[0][0].bottom > 480 - 38 && th[0][0].bottom > 120 - 16);
        assert(!memcmp(&sn[0][1], &sn[2][1], sizeof(struct box)));
        assert(!memcmp(&th[0][1], &th[2][1], sizeof(struct box)));
        assert(sn[2][0].bottom <= 480 - 38 && sn[2][0].bottom > 480 - 107);
        assert(th[2][0].bottom <= 120 - 16 && th[2][0].bottom > 120 - 30);
        assert(sn[2][1].bottom <= 480 - 107 && sn[2][1].top > 240);
        assert(th[2][1].bottom <= 120 - 30);
        /* 2.35:1 also applies to native 3:2 and native 16:9, above its bar. */
        for (u32 native = 0; native <= 2; native += 2) {
            encode_setup();
            fill_planes(40, 128, 128);
            STAMP_STYLE = 3;
            header[GRKIT_HOST_FRAME_RATIO / 4] = 4;
            native_aspect = native;
            picture_encode(0, MESSAGE);
            struct box b = changes(1, 0), t = changes(2, 0);
            assert(b.lit && b.bottom <= 480 - 87 && b.bottom > 480 - 107);
            assert(t.lit && t.bottom <= 120 - 26 && t.bottom > 120 - 30);
        }
        native_aspect = 0;
    }
#endif
    /* 5:4 Framing (native 3:2 on each camera): left of its right bar, which
       is narrower than the 1:1 bar. 1:1 is a crop too: its stamp stays left
       of the wider 1:1 bar. */
    {
        struct box crop, off, square;
        encode_setup();
        fill_planes(40, 128, 128);
        STAMP_STYLE = 3;
        picture_encode(0, MESSAGE);
        off = changes(1, 0);
        encode_setup();
        fill_planes(40, 128, 128);
        STAMP_STYLE = 3;
        header[GRKIT_HOST_FRAME_RATIO / 4] = 6;
        picture_encode(0, MESSAGE);
        crop = changes(1, 0);
        {
            struct box th = changes(2, 0);
            assert(th.lit && th.right <= 160 - 6 && th.left > 80);
        }
        assert(crop.lit && crop.right <= 720 - 60 && crop.left > 360 && crop.top > 240);
        assert(crop.right < off.right);
        encode_setup();
        fill_planes(40, 128, 128);
        STAMP_STYLE = 3;
        header[GRKIT_HOST_FRAME_RATIO / 4] = 5;
        picture_encode(0, MESSAGE);
        square = changes(1, 0);
        {
            struct box th = changes(2, 0);
            assert(th.lit && th.right <= 160 - 20 && th.left > 80);
        }
        assert(square.lit && square.right <= 720 - 120 && square.left > 360 && square.top > 240);
        assert(square.right < crop.right);
        /* 7:6: between the 5:4 and the 1:1 bars, 80 pixels and 10 in the
           thumbnail. */
        {
            struct box seven, th;
            encode_setup();
            fill_planes(40, 128, 128);
            STAMP_STYLE = 3;
            header[GRKIT_HOST_FRAME_RATIO / 4] = 7;
            picture_encode(0, MESSAGE);
            seven = changes(1, 0);
            th = changes(2, 0);
            assert(th.lit && th.right <= 160 - 10 && th.right > 160 - 20 && th.left > 80);
            assert(seven.lit && seven.right <= 720 - 80 && seven.left > 360 && seven.top > 240);
            assert(seven.right < crop.right && seven.right > square.right);
        }
    }
    /* A monochrome GRKit effect keeps neutral chroma: the stamp lights only Y. */
    encode_setup();
    fill_planes(40, 128, 128);
    STAMP_STYLE = 1;
    still(record(2));
    WORD(record(2), GRKIT_RECORD_MONOCHROME) = 1;
    picture_encode(0, MESSAGE);
    for (u32 i = 0; i < 3; ++i) { struct box b = changes(i, 1); assert(b.lit && b.lighter && !b.orange); }
    /* Its light is white: brighter in luma than the amber light of a color picture. */
    assert(changes(0, 1).lift > dark_lift);
    /* A native monochrome Image Control type of the shot also keeps neutral
       chroma. A GRKit record decides over the native type. */
    {
        static const uint8_t mono[] = {13, 14, 15, 16
#ifdef FRAMING_XPAN_NATIVE
            , 25, 26, 27, 28, 29, 30
#endif
#ifdef NATIVE_MONOCHROME_TYPES_HIGH
            , 31, 32
#endif
        };
        static const uint8_t color[] = {11, 12, 17, 22
#ifdef FRAMING_XPAN_NATIVE
            , 23, 24
#endif
        };
        for (u32 i = 0; i < sizeof(mono); ++i) {
            encode_setup();
            fill_planes(40, 128, 128);
            STAMP_STYLE = 1;
            BYTE(SHOT, SETTINGS_IC_TYPE) = mono[i];
            picture_encode(0, MESSAGE);
            for (u32 j = 0; j < 3; ++j) { struct box b = changes(j, 1); assert(b.lit && b.lighter && !b.orange); }
        }
        for (u32 i = 0; i < sizeof(color); ++i) {
            encode_setup();
            fill_planes(40, 128, 128);
            STAMP_STYLE = 1;
            BYTE(SHOT, SETTINGS_IC_TYPE) = color[i];
            picture_encode(0, MESSAGE);
            assert(changes(0, 0).orange);
        }
        encode_setup();
        fill_planes(40, 128, 128);
        STAMP_STYLE = 1;
        BYTE(SHOT, SETTINGS_IC_TYPE) = 13;
        still(record(2));
        picture_encode(0, MESSAGE);
        assert(changes(0, 0).orange);
    }
    /* The GR1s look prints yellow dots on a color picture, as GR1s color
       negatives show them, and white dots on a monochrome one. */
    {
        encode_setup();
        fill_planes(40, 128, 128);
        STAMP_STYLE = 1;
        STAMP_LOOK = 1;
        picture_encode(0, MESSAGE);
        struct box b = changes(0, 0);
        assert(b.lit && b.lighter && b.yellow && !b.orange && b.left > planes[0].width / 3);
        encode_setup();
        fill_planes(40, 128, 128);
        STAMP_STYLE = 1;
        STAMP_LOOK = 1;
        BYTE(SHOT, SETTINGS_IC_TYPE) = 13;
        picture_encode(0, MESSAGE);
        for (u32 j = 0; j < 3; ++j) { struct box m = changes(j, 1); assert(m.lit && m.lighter && !m.orange); }
        /* The dots end about 19% of the short side before the right edge and
           8.3% above the bottom, as on the prints of the GR1s: 91 and 39
           pixels on the 720 x 480 main picture. The halo reaches a little
           further. */
        struct box d = changes(1, 1);
        /* "'26 10 17" spans 80 x 17 pixels with its halo: digits advance 7
           columns, as on the prints. */
        assert(d.right - d.left >= 76 && d.right - d.left <= 84 && d.bottom - d.top >= 15 && d.bottom - d.top <= 19);
        assert(d.right <= 720 - 91 + 4 && d.right > 720 - 91 - 4);
        assert(d.bottom <= 480 - 39 + 4 && d.bottom > 480 - 39 - 4);
    }
    /* The GR II look paints flat GR II orange (Y 146, Cb 46, Cr 225) in its
       16 x 24 cells: on a 720 x 480 picture one glyph pixel is one pixel. The
       text ends 3 cells before the right edge, and the ink rows end 51 rows
       above the bottom. A monochrome picture gets white luma only. */
    {
        encode_setup();
        fill_planes(40, 128, 128);
        STAMP_STYLE = 1;
        STAMP_LOOK = 2;
        picture_encode(0, MESSAGE);
        const struct test_plane *p = &planes[1];
        struct box b = changes(1, 0);
        /* "2026/10/17": ten cells end at 720 - 48. */
        assert(b.lit && b.right <= 720 - 48 && b.right > 720 - 48 - 16);
        assert(b.left >= 720 - 16 * 13 && b.left < 720 - 16 * 13 + 16);
        assert(b.top == 480 - 72 + 3 && b.bottom == 480 - 72 + 21);
        u32 flat = 0;
        for (u32 y = b.top; y < b.bottom; ++y) for (u32 x = b.left; x < b.right; ++x) {
            uint8_t luma = BYTE(p->y, y * p->pitch + x);
            assert(luma == 40 || luma == 146);
            if (luma == 146) {
                ++flat;
                assert(BYTE(p->c, y * p->pitch + (x & ~1u)) == 46 && BYTE(p->c, y * p->pitch + (x | 1)) == 225);
            }
        }
        assert(flat > 100);
        /* The main picture and the thumbnail get the stamp at the same place. */
        assert(changes(0, 0).lit && changes(2, 0).lit);
        encode_setup();
        fill_planes(40, 128, 128);
        STAMP_STYLE = 1;
        STAMP_LOOK = 2;
        BYTE(SHOT, SETTINGS_IC_TYPE) = 13;
        picture_encode(0, MESSAGE);
        for (u32 j = 0; j < 3; ++j) { struct box m = changes(j, 1); assert(m.lit && m.lighter && !m.orange); }
        /* On a larger main picture the glyph pixels scale with the short
           side, as the GR II imprint on its 3264 rows: a 6000 x 4000 main
           picture gets 3.68 pixels for each glyph pixel. The 720 x 480 plane
           samples that geometry: the ink rows lie at main rows 3746 to 3812,
           and the text ends at main column 6000 - 48 * 3.68. */
        {
            static const Plane main = {0, 0, 6000, 4000, 6000, 1, 0, 0, 6000, 4000};
            Plane small = {planes[1].y, planes[1].c, 720, 480, planes[1].pitch, 0, 0, 0, 720, 480};
            encode_setup();
            fill_planes(40, 128, 128);
            gr2_paint(&small, &main, "2026/10/17", 10, 0, 0);
            struct box g = changes(1, 0);
            assert(g.lit && g.top >= 449 && g.top <= 450 && g.bottom >= 457 && g.bottom <= 459);
            assert(g.right <= 699 && g.right >= 696);
            assert(g.left >= 720 - 16 * 13 * 3676 / 8333 - 2 && g.left <= 720 - 16 * 13 * 3676 / 8333 + 8);
        }
        /* An unknown look prints the classic digits. */
        encode_setup();
        fill_planes(40, 128, 128);
        STAMP_STYLE = 1;
        STAMP_LOOK = 7;
        picture_encode(0, MESSAGE);
        assert(changes(0, 0).orange);
        STAMP_LOOK = 0;
    }
    /* Unusable descriptors are skipped. */
    encode_setup();
    STAMP_STYLE = 1;
    HALF(WORD(CONFIG, 0x3c), 0x14) = 500;
    WORD(CONFIG, 0x40) = 0;
    HALF(WORD(CONFIG, 0x44), 0x10) = 161;
    picture_encode(0, MESSAGE);
    assert(flushes == 0);
    for (u32 i = 0; i < 3; ++i) assert(changes(i, 0).lit == 0);

    /* The largest picture draws the stamp and the previews scale its mask, so
       the stamp keeps its place and total width when playback switches from the
       screennail to the main picture. */
    for (u32 style = 1; style <= DATE_STAMP_STYLE_MAX; ++style) {
        reset();
        WORD(MESSAGE, 8) = CONFIG;
        picture(0, 1200, 800, 1216, PLANES);
        picture(1, 720, 480, 736, PLANES + 0x200000);
        picture(2, 160, 120, 160, PLANES + 0x2c0000);
        WORD(CONFIG, 0x48) = WORD(CONFIG, 0x3c); WORD(CONFIG, 0x58) = WORD(CONFIG, 0x4c);
        flushes = 0; native_aspect = 0;
        fill_planes(40, 128, 128);
        STAMP_STYLE = style;
        picture_encode(0, MESSAGE);
        struct box big = changes(0, 0), sn = changes(1, 0), th = changes(2, 0);
        assert(big.lit && big.lighter && big.orange && sn.lit && sn.lighter && th.lit && th.lighter);
        /* Screennail: 1200 / 720 per pixel. Thumbnail: 1200 / 160 across, 800 / 104 down. */
        assert(abs((int)sn.right - (int)(big.right * 720 / 1200)) <= 2);
        assert(abs((int)sn.left - (int)(big.left * 720 / 1200)) <= 2);
        assert(abs((int)sn.bottom - (int)(big.bottom * 480 / 800)) <= 2);
        assert(abs((int)th.right - (int)(big.right * 160 / 1200)) <= 2);
        assert(abs((int)th.left - (int)(big.left * 160 / 1200)) <= 2);
        assert(abs((int)th.bottom - (int)(8 + big.bottom * 104 / 800)) <= 2);
    }
    /* Glow is gone: a record with the old glow bit and strength byte leaves a
       picture with highlights untouched when no stamp is selected. */
    encode_setup();
    still(record(1));
    WORD(record(1), GRKIT_RECORD_FEATURES) = 4;
    BYTE(record(1), GRKIT_RECORD_POST + 19) = 3;
    for (u32 y = 180; y < 220; ++y) for (u32 x = 280; x < 320; ++x) BYTE(PLANES, y * 640 + x) = 250;
    picture_encode(0, MESSAGE);
    assert(flushes == 0);
    for (u32 y = 0; y < 400; ++y) for (u32 x = 0; x < 600; ++x) {
        u32 inside = y >= 180 && y < 220 && x >= 280 && x < 320;
        assert(BYTE(PLANES, y * 640 + x) == (inside ? 250 : 60));
    }
    /* Selective color keeps red and crimson and removes green, skin, and orange,
       on every picture of the set. Luma and neutral pairs stay unchanged. */
    {
        /* Cb, Cr, and whether the pair keeps its chroma. */
        static const uint8_t cases[][3] = {
            {85, 255, 1}, {118, 215, 1}, {62, 44, 0}, {102, 157, 0}, {63, 195, 0}, {128, 128, 1},
        };
        for (u32 k = 0; k < sizeof cases / sizeof *cases; ++k) {
            encode_setup();
            fill_planes(90, cases[k][0], cases[k][1]);
            still(record(0));
            keep_red(record(0));
            picture_encode(0, MESSAGE);
            assert(flushes == 12);
            for (u32 i = 0; i < 3; ++i) {
                const struct test_plane *p = &planes[i];
                for (u32 y = 0; y < p->height; ++y) for (u32 x = 0; x < p->width; ++x) {
                    uint8_t c = BYTE(p->c, y * p->pitch + x);
                    assert(BYTE(p->y, y * p->pitch + x) == 90);
                    assert(c == (cases[k][2] ? cases[k][x & 1] : 128));
                }
            }
        }
    }
    /* A 4:2:0 plane has chroma rows for only half of its height. */
    encode_setup();
    fill_planes(90, 62, 44);
    BYTE(DESCRIPTORS + 0x100, 0x10) = 2;
    still(record(0));
    keep_red(record(0));
    picture_encode(0, MESSAGE);
    for (u32 y = 0; y < 400; ++y) for (u32 x = 0; x < 600; ++x)
        assert(BYTE(planes[0].c, y * 640 + x) == (y < 200 ? 128 : (x & 1 ? 44 : 62)));
    /* The stamp draws after selective color, so it keeps its orange. */
    encode_setup();
    fill_planes(40, 128, 128);
    STAMP_STYLE = 1;
    still(record(0));
    keep_red(record(0));
    picture_encode(0, MESSAGE);
    assert(flushes == 12 && changes(0, 0).orange);
    /* A monochrome record or a pointer that no record owns changes nothing. */
    for (u32 k = 0; k < 2; ++k) {
        encode_setup();
        fill_planes(90, 62, 44);
        keep_red(record(0));
        WORD(record(0), GRKIT_RECORD_MONOCHROME) = k == 0;
        still(k == 0 ? record(0) : record(0) + 4);
        picture_encode(0, MESSAGE);
        assert(flushes == 0);
        assert(changes(0, 0).lit == 0 && changes(1, 0).lit == 0 && changes(2, 0).lit == 0);
    }
    /* The encode reads the record of its own shot. A later shot that has
       started processing does not change it, the newest entry of a reused
       shot pointer wins, and a shot without an entry gets no effect. */
    for (u32 k = 0; k < 4; ++k) {
        encode_setup();
        fill_planes(90, 62, 44);
        keep_red(record(0));
        keep_red(record(1));
        WORD(record(1), GRKIT_RECORD_MONOCHROME) = 1;
        if (k == 0) { still(record(0)); store_shot(SHOT + 0x100, record(1)); }
        if (k == 1) { still(record(1)); still(record(0)); }
        if (k == 2) store_shot(SHOT + 0x100, record(0));
        if (k == 3) {
            /* Older entries leave the ring after eight later builds. */
            still(record(0));
            for (u32 i = 0; i < GRKIT_SHOT_ENTRIES; ++i) store_shot(SHOT + 0x100 + i * 4, record(1));
        }
        picture_encode(0, MESSAGE);
        assert((changes(0, 0).lit != 0) == (k < 2));
    }
    /* Without the Date Imprint module, a stored style encodes nothing. */
    encode_setup();
    fill_planes(40, 128, 128);
    STAMP_STYLE = 1;
    date_module(0);
    picture_encode(0, MESSAGE);
    assert(flushes == 0);
    for (u32 i = 0; i < 3; ++i) assert(changes(i, 0).lit == 0);
    /* The maker note of a GRKit shot records the Fn adjustments of its record
       in the nine Standard elements of tag 0x247, as shown values. Stored
       bytes are the shown value mod 9, and a byte above 8 counts as 0. */
    {
        static const uint8_t stored[9] = {2, 7, 0, 4, 5, 8, 1, 9, 6};
        static const uint32_t elements[9] = {0, 1, 2, 3, 4, 5, 6, 9, 10};
        static const uint32_t shown[9] = {2, 0xfffe, 0, 4, 0xfffc, 0xffff, 1, 0, 0xfffd};
        reset();
        for (u32 i = 0; i < 9; ++i) BYTE(record(2), GRKIT_RECORD_PARAMETERS + i) = stored[i];
        memset(ifd_values, 0xaa, sizeof(ifd_values));
        ifd_writes = 0;
        picture_record_parameters(0x1234, SHOT);
        assert(ifd_writes == 0);
        still(record(2));
        picture_record_parameters(0x1234, SHOT);
        assert(ifd_writes == 9 && ifd_entry == 0x1234);
        for (u32 i = 0; i < 9; ++i) assert(ifd_values[elements[i]] == shown[i]);
        /* A native look's defaults add to the stored changes: saturation -2
           with a change of +2 records 0, sharpness -3 with +1 records -2. */
        BYTE(record(2), GRKIT_RECORD_FN_DEFAULTS + 0) = 7;
        BYTE(record(2), GRKIT_RECORD_FN_DEFAULTS + 6) = 6;
        picture_record_parameters(0x1234, SHOT);
        assert(ifd_values[0] == 0 && ifd_values[6] == 0xfffe && ifd_values[3] == 4);
        BYTE(record(2), GRKIT_RECORD_FN_DEFAULTS + 0) = 0;
        BYTE(record(2), GRKIT_RECORD_FN_DEFAULTS + 6) = 0;
        picture_record_parameters(0x1234, SHOT);
        /* The other elements keep the native fill. */
        for (u32 i = 0; i < 32; ++i)
            if (i > 10 || i == 7 || i == 8) assert(ifd_values[i] == 0xaaaaaaaaU);
        /* Another shot keeps the native adjustments. */
        ifd_writes = 0;
        picture_record_parameters(0x1234, SHOT + 4);
        assert(ifd_writes == 0);
    }
    /* Division helper used for reciprocals. */
    for (u32 n = 0; n < 70000; n += 37) for (u32 d = 1; d < 400; d += 13) assert(divide(n, d) == n / d);
    return 0;
}
