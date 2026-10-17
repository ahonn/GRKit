#include <assert.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

/* One arena stands in for camera memory: records, the matrix object, and the planes. */
#define ARENA 0x59000000U
static uint8_t arena[0x300000];
static void *mapped(uint32_t address, uint32_t bytes)
{
    assert(address >= ARENA && address - ARENA + bytes <= sizeof(arena));
    assert(address % bytes == 0);
    return arena + (address - ARENA);
}
/* A whole region, such as the glow map: bounds only. */
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
static unsigned flushes;
#define PICTURE_CACHE_FLUSH(address,length) ((void)mapped((address), 1), (void)mapped((address) + (length) - 1, 1), ++flushes)
#define PICTURE_SCRATCH(h) ((void)(h), ARENA + 0x2c0000)
#define PICTURE_MAP(address) ((u8 *)region((address), GRKIT_GLOW_SCRATCH))
#include "picture.c"

static volatile u32 header[64];
volatile u32 *framing_header(void) { return header; }

#define RECORDS (ARENA + 0x100)
#define OBJECT (ARENA + 0x8000)
#define CONTEXT (ARENA + 0x8100)

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

/* Encode configuration: four image and four code descriptors, then the planes. */
#define MESSAGE (ARENA + 0x8300)
#define CONFIG (ARENA + 0x8400)
#define DESCRIPTORS (ARENA + 0x8500)
#define PLANES (ARENA + 0x10000)
struct test_plane { uint32_t width, height, pitch, y, c; };
static struct test_plane planes[3];
static void picture(uint32_t index, uint32_t width, uint32_t height, uint32_t pitch, uint32_t at)
{
    uint32_t image = DESCRIPTORS + index * 0x20, code = DESCRIPTORS + 0x100 + index * 0x20;
    planes[index] = (struct test_plane){width, height, pitch, at, at + pitch * height};
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
    flushes = 0;
}
/* Number of pixels that differ from the planes that picture() wrote. */
static uint32_t changes(uint32_t index)
{
    const struct test_plane *p = &planes[index];
    uint32_t lit = 0;
    for (uint32_t y = 0; y < p->height; ++y) for (uint32_t x = 0; x < p->pitch; ++x)
        lit += BYTE(p->y, y * p->pitch + x) != 60 || BYTE(p->c, y * p->pitch + x) != (x & 1 ? 150 : 100);
    return lit;
}

int main(void)
{
    static const short identity[9] = {8192, 0, 0, 0, 8192, 0, 0, 0, 8192};
    /* Native records and GRFX0001 records keep the native matrix bits. */
    reset();
    WORD(CONTEXT, 8) = 0x5501900c;
    picture_post(OBJECT, CONTEXT);
    assert(calcs == 1);
    for (int i = 0; i < 9; ++i) assert(coefficient(i) == bt601[i]);
    WORD(CONTEXT, 8) = record(1);
    uint32_t stock[9];
    picture_post(OBJECT, CONTEXT);
    for (int i = 0; i < 9; ++i) stock[i] = WORD(OBJECT, i * 4);
    /* An identity post matrix gives the same coefficients, sign-extended. */
    post(record(1), identity);
    picture_post(OBJECT, CONTEXT);
    for (int i = 0; i < 9; ++i) {
        assert(coefficient(i) == bt601[i]);
        assert((WORD(OBJECT, i * 4) & 0x1ff) == (stock[i] & 0x1ff));
    }
    /* CSP x P: P swaps red and blue, so each row swaps its R and B columns. */
    static const short swap[9] = {0, 0, 8192, 0, 8192, 0, 8192, 0, 0};
    post(record(1), swap);
    picture_post(OBJECT, CONTEXT);
    for (int row = 0; row < 3; ++row) {
        assert(coefficient(row * 3) == bt601[row * 3 + 2]);
        assert(coefficient(row * 3 + 1) == bt601[row * 3 + 1]);
        assert(coefficient(row * 3 + 2) == bt601[row * 3]);
    }
    /* Q13 rounding to nearest: 77 x 1.5 = 115.5 rounds to 116, -43 x 1.5 to -64. */
    static const short gain[9] = {12288, 0, 0, 0, 8192, 0, 0, 0, 8192};
    post(record(1), gain);
    picture_post(OBJECT, CONTEXT);
    assert(coefficient(0) == 116 && coefficient(3) == -64 && coefficient(6) == 192);
    /* Results saturate to the 9-bit register range instead of wrapping. */
    static const short strong[9] = {32767, 0, 0, 0, 32767, 0, 0, 0, -32768};
    post(record(1), strong);
    picture_post(OBJECT, CONTEXT);
    assert(coefficient(0) == 255 && coefficient(1) == 255 && coefficient(2) == -116);
    assert(coefficient(4) == -256 && coefficient(5) == -256 && coefficient(6) == 255);
    assert((int32_t)WORD(OBJECT, 4 * 4) == -256);
    /* Only owned records count: an address between records or past the catalog. */
    WORD(CONTEXT, 8) = record(1) + 4;
    picture_post(OBJECT, CONTEXT);
    for (int i = 0; i < 9; ++i) assert(coefficient(i) == bt601[i]);
    WORD(record(3), GRKIT_RECORD_FEATURES) = GRKIT_FEATURE_POST;
    WORD(CONTEXT, 8) = record(3);
    picture_post(OBJECT, CONTEXT);
    for (int i = 0; i < 9; ++i) assert(coefficient(i) == bt601[i]);

    /* An effect without glow touches nothing. */
    encode_setup();
    picture_encode(0, MESSAGE);
    assert(flushes == 0);
    for (u32 i = 0; i < 3; ++i) assert(changes(i) == 0);
    /* Unusable descriptors are skipped. */
    encode_setup();
    header[GRKIT_HOST_STILL / 4] = record(1);
    WORD(record(1), GRKIT_RECORD_FEATURES) = GRKIT_FEATURE_GLOW;
    BYTE(record(1), GRKIT_RECORD_GLOW) = 3;
    HALF(WORD(CONFIG, 0x3c), 0x14) = 500;
    WORD(CONFIG, 0x40) = 0;
    HALF(WORD(CONFIG, 0x44), 0x10) = 161;
    picture_encode(0, MESSAGE);
    assert(flushes == 0);
    for (u32 i = 0; i < 3; ++i) assert(changes(i) == 0);

    /* Soft glow: a bright square lifts its surroundings toward white and moves
       their chroma toward neutral. It never darkens, and dark areas far away stay. */
    encode_setup();
    header[GRKIT_HOST_STILL / 4] = record(1);
    WORD(record(1), GRKIT_RECORD_FEATURES) = GRKIT_FEATURE_GLOW;
    BYTE(record(1), GRKIT_RECORD_GLOW) = 3;
    for (u32 y = 180; y < 220; ++y) for (u32 x = 280; x < 320; ++x) BYTE(PLANES, y * 640 + x) = 250;
    picture_encode(0, MESSAGE);
    {
        u32 lifted = 0;
        for (u32 y = 0; y < 400; ++y) for (u32 x = 0; x < 600; ++x) {
            u32 inside = y >= 180 && y < 220 && x >= 280 && x < 320;
            u8 luma = BYTE(PLANES, y * 640 + x), chroma = BYTE(PLANES + 640 * 400, y * 640 + x);
            int stock = x & 1 ? 150 : 100;
            assert(luma >= (inside ? 250 : 60));
            assert(abs((int)chroma - 128) <= abs(stock - 128));
            if (!inside && luma > 60) ++lifted;
        }
        assert(lifted > 1000);
        /* Near the square, chroma moves strictly toward neutral. */
        assert(BYTE(PLANES + 640 * 400, 200 * 640 + 330) > 100 && BYTE(PLANES + 640 * 400, 200 * 640 + 331) < 150);
        assert(BYTE(PLANES, 200 * 640 + 340) > BYTE(PLANES, 200 * 640 + 400));
        assert(BYTE(PLANES, 5 * 640 + 5) == 60 && BYTE(PLANES, 395 * 640 + 595) == 60);
    }
    /* Without highlights, glow changes nothing, even on bright mid tones below Y 200. */
    encode_setup();
    for (u32 y = 0; y < 400; ++y) for (u32 x = 0; x < 640; ++x) BYTE(PLANES, y * 640 + x) = 190;
    header[GRKIT_HOST_STILL / 4] = record(1);
    WORD(record(1), GRKIT_RECORD_FEATURES) = GRKIT_FEATURE_GLOW;
    BYTE(record(1), GRKIT_RECORD_GLOW) = 3;
    picture_encode(0, MESSAGE);
    for (u32 y = 0; y < 400; ++y) for (u32 x = 0; x < 600; ++x) assert(BYTE(PLANES, y * 640 + x) == 190);
    encode_setup();
    header[GRKIT_HOST_STILL / 4] = record(1);
    WORD(record(1), GRKIT_RECORD_FEATURES) = GRKIT_FEATURE_GLOW;
    BYTE(record(1), GRKIT_RECORD_GLOW) = 3;
    picture_encode(0, MESSAGE);
    for (u32 i = 0; i < 3; ++i) assert(changes(i) == 0);
    assert(flushes == 12);
    /* Division helper used for reciprocals. */
    for (u32 n = 0; n < 70000; n += 37) for (u32 d = 1; d < 400; d += 13) assert(divide(n, d) == n / d);
    return 0;
}
