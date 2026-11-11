/* The host test of Border (grkit module test): the border of each style
   around a picture scaled in place, on a 640 x 480 plane and on a cropped
   visible area. */
#include <assert.h>
#include "grkit_module.h"

typedef struct { u32 left, top, width, height; } Inner;
void border_apply(PictureSet *set, u32 style);
u32 border_thickness(const Plane *p, u32 style);
u32 border_inner(const Plane *p, u32 style, Inner *inner);

#define W 640U
#define H 480U
#define LUMA GRKIT_TEST_PLANES
#define CHROMA (GRKIT_TEST_PLANES + W * H)

static Plane plane(void)
{
    Plane p = {LUMA, CHROMA, W, H, W, 0, 0, 0, W, H};
    return p;
}

static void set_luma(u32 x, u32 y, u8 v) { *grkit_test_bytes(LUMA + y * W + x) = v; }
static void set_chroma(u32 x, u32 y, u32 k, u8 v) { *grkit_test_bytes(CHROMA + y * W + (x & ~1U) + k) = v; }
static u8 luma(u32 x, u32 y) { return *grkit_test_bytes(LUMA + y * W + x); }
static u8 chroma(u32 x, u32 y, u32 k) { return *grkit_test_bytes(CHROMA + y * W + (x & ~1U) + k); }

/* Luma that rises along x (horizontal) or y, Cb along x, Cr constant. */
static u8 ramp(u32 at, u32 size) { return (u8)(at * 255U / (size - 1)); }
static void fill_ramps(u32 vertical)
{
    for (u32 y = 0; y < H; ++y)
        for (u32 x = 0; x < W; ++x) {
            set_luma(x, y, vertical ? ramp(y, H) : ramp(x, W));
            if (!(x & 1)) { set_chroma(x, y, 0, ramp(x, W)); set_chroma(x, y, 1, 160); }
        }
}
static void fill(u8 l)
{
    for (u32 y = 0; y < H; ++y)
        for (u32 x = 0; x < W; ++x) { set_luma(x, y, l); set_chroma(x, y, x & 1, x & 1 ? 160 : 90); }
}

/* The source position of output i of `out` samples over `in` samples, as
   the filter maps the centers. */
static double source(u32 i, u32 out, u32 in)
{
    double at = (i + 0.5) * in / out - 0.5;
    return at < 0 ? 0 : at > in - 1 ? in - 1 : at;
}
static int near(int a, double b) { return a - b <= 2.0 && b - a <= 2.0; }

int main(void)
{
    PictureSet set;
    Inner in;
    Plane p;
    /* The descriptor: STYLE with 5 values, a page, a frame-stage filter, and
       a status-row indicator. */
    assert(grkit_module.setting_count == 1 && grkit_module.settings[0].values == 5);
    assert(grkit_module.tile_count == 6 && SETTING_STYLE == GRKIT_SETTING(0));
    assert(grkit_module.registration_count == 3);

    set.count = 1; set.mono = 0; set.scratch = grkit_test_scratch; set.turns = 0;
    set.planes[0] = p = plane();

    /* Thin: 3% of 640 is 19 pixels at the top and the bottom. The height
       limits the scale (442 / 480), so the picture is 588 x 442 at (26, 19),
       and the sides are 26 pixels. */
    assert(border_thickness(&p, 1) == 19);
    assert(border_inner(&p, 1, &in) && in.width == 588 && in.height == 442 && in.left == 26 && in.top == 19);
    /* Wide: 6% is 38 pixels; the picture is 538 x 404 at (50, 38): the
       left is even for the chroma pairs, so the right side is 52. */
    assert(border_thickness(&p, 2) == 38);
    assert(border_inner(&p, 2, &in) && in.width == 538 && in.height == 404 && in.left == 50 && in.top == 38);

    /* White, thin, on a horizontal ramp: white luma and neutral chroma
       around, and each picture pixel is the ramp at its source position. */
    fill_ramps(0);
    border_apply(&set, 1);
    assert(luma(0, 0) == 255 && luma(639, 479) == 255 && luma(25, 240) == 255 && luma(320, 18) == 255);
    assert(luma(614, 240) == 255 && luma(320, 461) == 255);
    assert(chroma(0, 0, 0) == 128 && chroma(0, 0, 1) == 128 && chroma(24, 240, 0) == 128);
    for (u32 y = 19; y < 461; y += 37)
        for (u32 x = 26; x < 614; ++x) {
            double expected = source(x - 26, 588, W) * 255.0 / (W - 1);
            assert(near(luma(x, y), expected));
        }
    for (u32 x = 26; x < 614; x += 2) {
        double expected = source((x - 26) >> 1, 294, W / 2) * 2 * 255.0 / (W - 1);
        assert(near(chroma(x, 240, 0), expected) && chroma(x, 240, 1) == 160);
    }
    /* On a vertical ramp: each row is the ramp at its source row, so the
       rows that the scale overwrote were read first. */
    fill_ramps(1);
    border_apply(&set, 1);
    for (u32 y = 19; y < 461; ++y) {
        double expected = source(y - 19, 442, H) * 255.0 / (H - 1);
        assert(near(luma(26, y), expected) && near(luma(320, y), expected) && near(luma(613, y), expected));
    }
    assert(luma(320, 18) == 255 && luma(320, 461) == 255);

    /* Black, thin and wide, on a flat picture: the picture keeps its value. */
    fill(100);
    border_apply(&set, 3);
    assert(luma(0, 0) == 0 && luma(25, 240) == 0 && luma(26, 240) == 100 && luma(320, 19) == 100);
    assert(chroma(320, 240, 0) == 90 && chroma(320, 240, 1) == 160);
    fill(100);
    border_apply(&set, 4);
    assert(luma(49, 240) == 0 && luma(50, 240) == 100 && luma(587, 240) == 100 && luma(588, 240) == 0);
    assert(luma(320, 37) == 0 && luma(320, 38) == 100);
    /* Off and an unknown style change nothing. */
    fill(100);
    border_apply(&set, 0);
    border_apply(&set, 9);
    assert(luma(0, 0) == 100 && luma(639, 479) == 100);

    /* A cropped visible area: the border and the picture stay inside it. */
    fill(100);
    set.planes[0].left = 100; set.planes[0].right = 540; set.planes[0].top = 60; set.planes[0].bottom = 420;
    assert(border_thickness(&set.planes[0], 1) == 13);
    assert(border_inner(&set.planes[0], 1, &in) && in.top == 73 && in.top + in.height == 407);
    border_apply(&set, 1);
    assert(luma(99, 240) == 100 && luma(100, 240) == 255 && luma(in.left - 1, 240) == 255 && luma(in.left, 240) == 100);
    assert(luma(320, 59) == 100 && luma(320, 60) == 255 && luma(320, 72) == 255 && luma(320, 73) == 100);
    assert(luma(320, 419) == 255 && luma(320, 420) == 100 && luma(540, 240) == 100);

    /* Monochrome: the chroma stays. */
    fill(100);
    set.planes[0] = plane(); set.mono = 1;
    border_apply(&set, 1);
    assert(luma(0, 0) == 255 && chroma(0, 0, 0) == 90 && chroma(0, 0, 1) == 160);

    /* Half chroma rows: one chroma row for two luma rows, an even top. */
    set.mono = 0; set.planes[0].half = 1;
    assert(border_inner(&set.planes[0], 1, &in) && (in.top & 1) == 0 && (in.height & 1) == 0);
    for (u32 y = 0; y < H; ++y)
        for (u32 x = 0; x < W; ++x) {
            set_luma(x, y, 100);
            if (y < H / 2) set_chroma(x, y, x & 1, x & 1 ? 160 : 90);
        }
    border_apply(&set, 1);
    assert(luma(0, 1) == 255 && luma(320, 240) == 100);
    assert(chroma(0, 0, 0) == 128 && chroma(320, 120, 0) == 90 && chroma(320, 120, 1) == 160);
    assert(chroma(320, in.top / 2 - 1, 0) == 128 && chroma(320, in.top / 2, 0) == 90);

    /* A plane too small for a picture inside is all border. */
    fill(100);
    set.planes[0] = plane(); set.planes[0].right = 4; set.planes[0].bottom = 4;
    border_apply(&set, 1);
    assert(luma(0, 0) == 255 && luma(3, 3) == 255 && luma(4, 0) == 100);
    return 0;
}
