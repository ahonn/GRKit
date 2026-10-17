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
#include "picture.c"

static volatile u32 header[64];
volatile u32 *framing_header(void) { return header; }

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
static void encode_setup(void)
{
    reset();
    WORD(MESSAGE, 8) = CONFIG;
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
struct box { uint32_t left, top, right, bottom, lit; int lighter, orange; uint32_t lift; };
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
        if (x < b.left) b.left = x;
        if (y < b.top) b.top = y;
        if (x + 1 > b.right) b.right = x + 1;
        if (y + 1 > b.bottom) b.bottom = y + 1;
        ++b.lit;
    }
    return b;
}

int main(void)
{
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

    /* Date text for each style: no leading zeros except the year and minutes. */
    {
        static const char *const expected[] = {"'26 10 17", "'26 10 17  9:05", "10 17 '26", "17 10 '26"};
        static const char *const early[] = {"'07 1 5", "'07 1 5  13:07", "1 5 '07", "5 1 '07"};
        static const uint8_t date[6] = {2026 & 255, 2026 >> 8, 10, 17, 9, 5};
        static const uint8_t january[6] = {2007 & 255, 2007 >> 8, 1, 5, 13, 7};
        char text[20];
        for (u32 style = 1; style <= 4; ++style) {
            u32 n = date_text(text, date, style);
            assert(n == strlen(expected[style - 1]) && !memcmp(text, expected[style - 1], n));
            n = date_text(text, january, style);
            assert(n == strlen(early[style - 1]) && !memcmp(text, early[style - 1], n));
        }
    }
    /* Off touches nothing. */
    encode_setup();
    picture_encode(0, MESSAGE);
    assert(flushes == 0);
    for (u32 i = 0; i < 3; ++i) assert(changes(i, 0).lit == 0);
    /* On a dark neutral picture the stamp lights an orange core at the bottom
       right of the visible area, once per plane set, and only lightens. */
    for (u32 style = 1; style <= 4; ++style) {
        encode_setup();
        fill_planes(40, 128, 128);
        header[GRKIT_HOST_STAMP / 4] = style;
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
    /* Longer styles reach further left. */
    encode_setup();
    fill_planes(40, 128, 128);
    header[GRKIT_HOST_STAMP / 4] = 1;
    picture_encode(0, MESSAGE);
    u32 short_left = changes(0, 0).left;
    encode_setup();
    fill_planes(40, 128, 128);
    header[GRKIT_HOST_STAMP / 4] = 2;
    picture_encode(0, MESSAGE);
    assert(changes(0, 0).left < short_left);
    /* A screen blend fades on bright areas, as an exposure on film does. */
    encode_setup();
    fill_planes(40, 128, 128);
    header[GRKIT_HOST_STAMP / 4] = 1;
    picture_encode(0, MESSAGE);
    u32 dark_lift = changes(0, 0).lift;
    encode_setup();
    fill_planes(240, 128, 128);
    header[GRKIT_HOST_STAMP / 4] = 1;
    picture_encode(0, MESSAGE);
    {
        struct box b = changes(0, 0);
        assert(b.lighter && b.lift <= 15 && dark_lift >= 100);
    }
    /* 16:9 Framing: the screennail stamp stays above the bottom bar. */
    encode_setup();
    fill_planes(40, 128, 128);
    header[GRKIT_HOST_STAMP / 4] = 3;
    header[GRKIT_HOST_FRAME_RATIO / 4] = 2;
    picture_encode(0, MESSAGE);
    {
        struct box b = changes(1, 0);
        assert(b.lit && b.bottom <= 480 - 38 && b.top > 240);
    }
    /* A monochrome GRKit effect keeps neutral chroma: the stamp lights only Y. */
    encode_setup();
    fill_planes(40, 128, 128);
    header[GRKIT_HOST_STAMP / 4] = 1;
    header[GRKIT_HOST_STILL / 4] = record(2);
    WORD(record(2), GRKIT_RECORD_MONOCHROME) = 1;
    picture_encode(0, MESSAGE);
    for (u32 i = 0; i < 3; ++i) { struct box b = changes(i, 1); assert(b.lit && b.lighter && !b.orange); }
    /* Unusable descriptors are skipped. */
    encode_setup();
    header[GRKIT_HOST_STAMP / 4] = 1;
    HALF(WORD(CONFIG, 0x3c), 0x14) = 500;
    WORD(CONFIG, 0x40) = 0;
    HALF(WORD(CONFIG, 0x44), 0x10) = 161;
    picture_encode(0, MESSAGE);
    assert(flushes == 0);
    for (u32 i = 0; i < 3; ++i) assert(changes(i, 0).lit == 0);

    /* The largest picture draws the stamp and the previews scale its mask, so
       the stamp keeps its place and total width when playback switches from the
       screennail to the main picture. */
    for (u32 style = 1; style <= 4; ++style) {
        reset();
        WORD(MESSAGE, 8) = CONFIG;
        picture(0, 1200, 800, 1216, PLANES);
        picture(1, 720, 480, 736, PLANES + 0x200000);
        picture(2, 160, 120, 160, PLANES + 0x2c0000);
        WORD(CONFIG, 0x48) = WORD(CONFIG, 0x3c); WORD(CONFIG, 0x58) = WORD(CONFIG, 0x4c);
        flushes = 0; native_aspect = 0;
        fill_planes(40, 128, 128);
        header[GRKIT_HOST_STAMP / 4] = style;
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
    header[GRKIT_HOST_STILL / 4] = record(1);
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
            header[GRKIT_HOST_STILL / 4] = record(0);
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
    header[GRKIT_HOST_STILL / 4] = record(0);
    keep_red(record(0));
    picture_encode(0, MESSAGE);
    for (u32 y = 0; y < 400; ++y) for (u32 x = 0; x < 600; ++x)
        assert(BYTE(planes[0].c, y * 640 + x) == (y < 200 ? 128 : (x & 1 ? 44 : 62)));
    /* The stamp draws after selective color, so it keeps its orange. */
    encode_setup();
    fill_planes(40, 128, 128);
    header[GRKIT_HOST_STAMP / 4] = 1;
    header[GRKIT_HOST_STILL / 4] = record(0);
    keep_red(record(0));
    picture_encode(0, MESSAGE);
    assert(flushes == 12 && changes(0, 0).orange);
    /* A monochrome record or a pointer that no record owns changes nothing. */
    for (u32 k = 0; k < 2; ++k) {
        encode_setup();
        fill_planes(90, 62, 44);
        keep_red(record(0));
        WORD(record(0), GRKIT_RECORD_MONOCHROME) = k == 0;
        header[GRKIT_HOST_STILL / 4] = k == 0 ? record(0) : record(0) + 4;
        picture_encode(0, MESSAGE);
        assert(flushes == 0);
        assert(changes(0, 0).lit == 0 && changes(1, 0).lit == 0 && changes(2, 0).lit == 0);
    }
    /* Division helper used for reciprocals. */
    for (u32 n = 0; n < 70000; n += 37) for (u32 d = 1; d < 400; d += 13) assert(divide(n, d) == n / d);
    return 0;
}
