/* The host test of Crop (grkit module test): the mask outside the frame. */
#include <assert.h>
#include "grkit_module.h"

void crop_mask(const Surface *s, u32 value);

/* The frame of 50 mm in the full 3:2 view, as the core gives it. */
static const u16 frame50[4] = {158, 105, 561, 374};

static int inside(const u16 *box, u32 x, u32 y) { return x >= box[0] && x < box[2] && y >= box[1] && y < box[3]; }

int main(void)
{
    /* The descriptor: MASK, the uses CORE.CROP and CORE.CROP_MODE, a page
       of CORE.CROP with its settings rows, and the mask layer. */
    assert(grkit_module.setting_count == 1 && grkit_module.use_count == 2);
    assert(SETTING_MASK == GRKIT_SETTING(0));
    assert(grkit_module.registration_count == 2 && grkit_module.tile_count == 6);
    /* Dark covers each pixel outside the frame at 50%, and no other. */
    {
        Surface s = grkit_test_surface(frame50);
        s.area = 3;
        grkit_test_clear();
        crop_mask(&s, 3);
        for (u32 y = 0; y < 480; ++y)
            for (u32 x = 0; x < 720; ++x)
                assert(grkit_test_pixel(x, y) == (inside(frame50, x, y) ? GRKIT_TEST_CLEAR : 0x80000000U));
    }
    /* Area 0: no frame, such as Off or a crop of the live view itself. */
    {
        Surface s = grkit_test_surface(frame50);
        s.area = 0;
        grkit_test_clear();
        crop_mask(&s, 3);
        assert(grkit_test_pixel(10, 10) == GRKIT_TEST_CLEAR && grkit_test_pixel(360, 240) == GRKIT_TEST_CLEAR);
    }
    /* Black fills the outside opaque; Frame Lines leave it clear and draw 2
       white pixels inside each edge of the frame. */
    {
        Surface s = grkit_test_surface(frame50);
        s.area = 3;
        grkit_test_settings[SETTING_MASK] = 1;
        grkit_test_clear();
        crop_mask(&s, 3);
        assert(grkit_test_pixel(10, 10) == 0xff000000U && grkit_test_pixel(360, 240) == GRKIT_TEST_CLEAR);
        grkit_test_settings[SETTING_MASK] = 2;
        grkit_test_clear();
        crop_mask(&s, 3);
        assert(grkit_test_pixel(10, 10) == GRKIT_TEST_CLEAR && grkit_test_pixel(360, 240) == GRKIT_TEST_CLEAR);
        assert(grkit_test_pixel(360, 105) == 0xffffffffU && grkit_test_pixel(360, 106) == 0xffffffffU);
        assert(grkit_test_pixel(360, 107) == GRKIT_TEST_CLEAR && grkit_test_pixel(360, 373) == 0xffffffffU);
        assert(grkit_test_pixel(158, 240) == 0xffffffffU && grkit_test_pixel(159, 240) == 0xffffffffU);
        assert(grkit_test_pixel(160, 240) == GRKIT_TEST_CLEAR && grkit_test_pixel(560, 240) == 0xffffffffU);
    }
    return 0;
}
