/* The host test of HDF Glow (grkit module test): a light on a dark picture. */
#include <assert.h>
#include "grkit_module.h"

void hdf_apply(PictureSet *set, u32 strength);
u32 hdf_grid_shift(u32 width, u32 height);

/* A 4:2:2 plane of w x h: luma rows, then interleaved CbCr rows of the same
   pitch, dark grey with a disc of colour (y, cb, cr) of radius r at (cx, cy). */
static Plane scene(u32 w, u32 h, u32 cx, u32 cy, u32 r, u8 y, u8 cb, u8 cr)
{
    Plane p = {GRKIT_TEST_PLANES, GRKIT_TEST_PLANES + w * h, w, h, w, 0, 0, 0, w, h};
    for (u32 j = 0; j < h; ++j)
        for (u32 i = 0; i < w; ++i) {
            int dx = (int)i - (int)cx, dy = (int)j - (int)cy;
            u32 in = (u32)(dx * dx + dy * dy) <= r * r;
            GRKIT_BYTES(p.y + j * w + i)[0] = in ? y : 20;
            GRKIT_BYTES(p.c + j * w + (i & ~1U))[0] = in ? cb : 128;
            GRKIT_BYTES(p.c + j * w + (i & ~1U) + 1)[0] = in ? cr : 128;
        }
    return p;
}
static u32 luma(const Plane *p, u32 x, u32 y) { return GRKIT_BYTES(p->y + y * p->pitch + x)[0]; }
static u32 cb_at(const Plane *p, u32 x, u32 y) { return GRKIT_BYTES(p->c + y * p->pitch + (x & ~1U))[0]; }
static u32 cr_at(const Plane *p, u32 x, u32 y) { return GRKIT_BYTES(p->c + y * p->pitch + (x & ~1U) + 1)[0]; }

static PictureSet one(Plane p, u32 mono)
{
    PictureSet set = {{p}, 1, 0, mono, grkit_test_scratch, 0};
    return set;
}

int main(void)
{
    /* The descriptor: STRENGTH (4 values) and HALO (2), a page with its
       settings rows, and one filter. */
    assert(grkit_module.setting_count == 2 && grkit_module.settings[0].values == 4 &&
           grkit_module.settings[1].values == 2);
    assert(grkit_module.registration_count == 2 && grkit_module.tile_count == 4);
    assert(SETTING_STRENGTH == GRKIT_SETTING(0) && SETTING_HALO == GRKIT_SETTING(1));

    /* The grids of the GR III pictures fit the scratch memory: 24 MP in cells
       of 32 pixels, the screennail in 16, the thumbnail in single pixels. */
    assert(hdf_grid_shift(6000, 4000) == 5 && hdf_grid_shift(4000, 6000) == 5);
    assert(hdf_grid_shift(1620, 1080) == 4 && hdf_grid_shift(160, 120) == 0);

    /* A white light glows: brighter around it, the light stays white, the far
       corner stays as it was, and a stronger setting glows more. */
    {
        u32 near[4] = {0};
        for (u32 s = 1; s <= 3; ++s) {
            Plane p = scene(600, 400, 300, 200, 6, 255, 128, 128);
            PictureSet set = one(p, 0);
            grkit_test_settings[SETTING_HALO] = 0;
            hdf_apply(&set, s);
            near[s] = luma(&p, 320, 200);
            assert(luma(&p, 300, 200) == 255);
            assert(luma(&p, 5, 5) == 20 && luma(&p, 595, 395) == 20);
            /* A white light gives a neutral glow. */
            assert(cb_at(&p, 320, 200) >= 126 && cb_at(&p, 320, 200) <= 130);
            assert(cr_at(&p, 320, 200) >= 126 && cr_at(&p, 320, 200) <= 130);
        }
        assert(near[1] > 24 && near[1] < near[2] && near[2] < near[3]);
    }

    /* An orange lamp glows orange: Cr up and Cb down around it. */
    {
        Plane p = scene(600, 400, 300, 200, 8, 180, 60, 200);
        PictureSet set = one(p, 0);
        grkit_test_settings[SETTING_HALO] = 0;
        hdf_apply(&set, 3);
        assert(luma(&p, 322, 200) > 30);
        assert(cr_at(&p, 322, 200) > 132 && cb_at(&p, 322, 200) < 124);
    }

    /* The CineStill halo adds red around a white light, not on it. */
    {
        u32 plain, red;
        Plane p = scene(600, 400, 300, 200, 6, 255, 128, 128);
        PictureSet set = one(p, 0);
        grkit_test_settings[SETTING_HALO] = 0;
        hdf_apply(&set, 1);
        plain = cr_at(&p, 316, 200);
        p = scene(600, 400, 300, 200, 6, 255, 128, 128);
        set = one(p, 0);
        grkit_test_settings[SETTING_HALO] = 1;
        hdf_apply(&set, 1);
        red = cr_at(&p, 316, 200);
        assert(red > plain + 4 && cb_at(&p, 316, 200) < 128);
        assert(luma(&p, 300, 200) == 255 && cr_at(&p, 300, 200) <= 130);
        grkit_test_settings[SETTING_HALO] = 0;
    }

    /* A monochrome picture glows in luma only. */
    {
        Plane p = scene(600, 400, 300, 200, 6, 255, 128, 128);
        PictureSet set = one(p, 1);
        grkit_test_settings[SETTING_HALO] = 1;
        hdf_apply(&set, 2);
        assert(luma(&p, 320, 200) > 24);
        for (u32 x = 280; x < 340; ++x) assert(cb_at(&p, x, 200) == 128 && cr_at(&p, x, 200) == 128);
        grkit_test_settings[SETTING_HALO] = 0;
    }

    /* Only the visible area changes. */
    {
        Plane p = scene(600, 400, 340, 200, 6, 255, 128, 128);
        PictureSet set;
        p.left = 310;
        set = one(p, 0);
        hdf_apply(&set, 3);
        for (u32 x = 280; x < 310; ++x) assert(luma(&p, x, 200) == 20);
        assert(luma(&p, 320, 200) > 24);
    }

    /* A dark picture without lights stays as it was. */
    {
        Plane p = scene(600, 400, 300, 200, 0, 20, 128, 128);
        PictureSet set = one(p, 0);
        hdf_apply(&set, 3);
        for (u32 y = 0; y < 400; y += 7)
            for (u32 x = 0; x < 600; x += 5) assert(luma(&p, x, y) == 20 && cr_at(&p, x, y) == 128);
    }

    /* A large picture with 4:2:0 chroma samples every second pixel and still
       glows around a small light. */
    {
        u32 w = 2400, h = 1600;
        Plane p = scene(w, h, 1200, 800, 10, 255, 128, 128);
        PictureSet set;
        p.half = 1;
        set = one(p, 0);
        assert(hdf_grid_shift(w, h) >= 4);
        hdf_apply(&set, 2);
        assert(luma(&p, 1240, 800) > 24 && luma(&p, 5, 5) == 20);
    }
    return 0;
}
