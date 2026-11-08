/* The host test of Border (grkit module test): the edge of each style on a
   640 x 480 plane and on a cropped visible area. */
#include <assert.h>
#include "grkit_module.h"

void border_apply(PictureSet *set, u32 style);
u32 border_thickness(const Plane *p, u32 style);

#define W 640U
#define H 480U

static Plane plane(void)
{
    Plane p = {GRKIT_TEST_PLANES, GRKIT_TEST_PLANES + W * H, W, H, W, 0, 0, 0, W, H};
    return p;
}

static void fill(u8 luma, u8 cb, u8 cr)
{
    for (u32 i = 0; i < W * H; ++i) *grkit_test_bytes(GRKIT_TEST_PLANES + i) = luma;
    for (u32 i = 0; i < W * H; i += 2) {
        *grkit_test_bytes(GRKIT_TEST_PLANES + W * H + i) = cb;
        *grkit_test_bytes(GRKIT_TEST_PLANES + W * H + i + 1) = cr;
    }
}

static u8 luma(u32 x, u32 y) { return *grkit_test_bytes(GRKIT_TEST_PLANES + y * W + x); }
static u8 chroma(u32 x, u32 y, u32 k) { return *grkit_test_bytes(GRKIT_TEST_PLANES + W * H + y * W + (x & ~1U) + k); }

int main(void)
{
    PictureSet set;
    /* The descriptor: STYLE with 5 values, a page, a frame-stage filter, and
       a status-row indicator. */
    assert(grkit_module.setting_count == 1 && grkit_module.settings[0].values == 5);
    assert(grkit_module.tile_count == 6 && SETTING_STYLE == GRKIT_SETTING(0));
    assert(grkit_module.registration_count == 3);

    set.count = 1; set.mono = 0; set.scratch = grkit_test_scratch; set.turns = 0;
    set.planes[0] = plane();
    /* White, thin: 3% of 640 is 19 pixels, luma 255 and neutral chroma. */
    fill(100, 90, 160);
    border_apply(&set, 1);
    assert(border_thickness(&set.planes[0], 1) == 19);
    assert(luma(0, 0) == 255 && luma(639, 479) == 255 && luma(18, 240) == 255 && luma(320, 18) == 255);
    assert(luma(19, 240) == 100 && luma(320, 19) == 100 && luma(320, 240) == 100 && luma(620, 240) == 100);
    assert(luma(621, 240) == 255 && luma(320, 461) == 255 && luma(320, 460) == 100);
    assert(chroma(0, 0, 0) == 128 && chroma(0, 0, 1) == 128 && chroma(320, 240, 0) == 90 && chroma(320, 240, 1) == 160);
    /* White, wide: 6%, 38 pixels. */
    fill(100, 128, 128);
    border_apply(&set, 2);
    assert(luma(37, 240) == 255 && luma(38, 240) == 100);
    /* Black, thin and wide. */
    fill(100, 128, 128);
    border_apply(&set, 3);
    assert(luma(0, 0) == 0 && luma(18, 240) == 0 && luma(19, 240) == 100);
    fill(100, 128, 128);
    border_apply(&set, 4);
    assert(luma(37, 240) == 0 && luma(38, 240) == 100);
    /* Off and an unknown style change nothing. */
    fill(100, 128, 128);
    border_apply(&set, 0);
    border_apply(&set, 9);
    assert(luma(0, 0) == 100);
    /* A cropped visible area: the edge hugs the area, not the plane. */
    fill(100, 128, 128);
    set.planes[0].left = 100; set.planes[0].right = 540; set.planes[0].top = 60; set.planes[0].bottom = 420;
    border_apply(&set, 1);
    assert(border_thickness(&set.planes[0], 1) == 13);
    assert(luma(99, 240) == 100 && luma(100, 240) == 255 && luma(112, 240) == 255 && luma(113, 240) == 100);
    assert(luma(320, 59) == 100 && luma(320, 60) == 255 && luma(320, 419) == 255 && luma(320, 420) == 100);
    /* Monochrome: the chroma stays. */
    fill(100, 90, 160);
    set.planes[0] = plane(); set.mono = 1;
    border_apply(&set, 1);
    assert(luma(0, 0) == 255 && chroma(0, 0, 0) == 90 && chroma(0, 0, 1) == 160);
    /* Subsampled chroma rows (half): every luma row paints, chroma on even rows. */
    fill(100, 90, 160);
    set.mono = 0; set.planes[0].half = 1;
    border_apply(&set, 1);
    assert(luma(0, 1) == 255 && chroma(0, 0, 0) == 128);
    return 0;
}
