/* The host test of Date Imprint (grkit module test): the stamp on a gray
   picture. tests/runtime/picture_contract.c checks the stamp in detail. */
#include <assert.h>
#include "grkit_module.h"

void date_apply(PictureSet *set, u32 style);

#define WIDTH 1200U
#define HEIGHT 800U
/* A 4:2:2 plane: luma rows, then interleaved CbCr rows of the same pitch. */
static Plane gray_plane(u32 at)
{
    Plane p = {at, at + WIDTH * HEIGHT, WIDTH, HEIGHT, WIDTH, 0, 0, 0, WIDTH, HEIGHT};
    for (u32 i = 0; i < WIDTH * HEIGHT; ++i) {
        GRKIT_BYTES(p.y + i)[0] = 64;
        GRKIT_BYTES(p.c + i)[0] = 128;
    }
    return p;
}
/* The changed pixels of a plane, and their bounds. */
static u32 bounds(const Plane *p, u32 *left, u32 *top, u32 *right, u32 *bottom)
{
    u32 n = 0;
    *left = WIDTH; *top = HEIGHT; *right = 0; *bottom = 0;
    for (u32 y = 0; y < HEIGHT; ++y)
        for (u32 x = 0; x < WIDTH; ++x)
            if (GRKIT_BYTES(p->y + y * WIDTH + x)[0] != 64) {
                ++n;
                if (x < *left) *left = x;
                if (y < *top) *top = y;
                if (x + 1 > *right) *right = x + 1;
                if (y + 1 > *bottom) *bottom = y + 1;
            }
    return n;
}
static u32 changed(const Plane *p, u32 *left, u32 *top)
{
    u32 right, bottom;
    return bounds(p, left, top, &right, &bottom);
}

static u32 pixels_hash(const Plane *p)
{
    u32 hash = 2166136261U;
    for (u32 i = 0; i < WIDTH * HEIGHT; ++i) {
        hash = (hash ^ GRKIT_BYTES(p->y + i)[0]) * 16777619U;
        hash = (hash ^ GRKIT_BYTES(p->c + i)[0]) * 16777619U;
    }
    return hash;
}

int main(void)
{
    /* Baseline pixels at the harness clock (2026-10-25 09:05), before the
       blur workspace moved from the task stack to picture scratch. */
    static const u32 expected[3][5] = {
        {1184513837U,809389240U,2473835757U,755202461U,321906744U},
        {1950141043U,2516237631U,3262944462U,4264252730U,2226400522U},
        {666516357U,2337147665U,679377069U,3903426885U,2609955457U}
    };
    static u8 guarded[GRKIT_PLANE_SCRATCH + 2];
    u32 left, top;
    /* The descriptor: STYLE (6 values), LOOK (3), POSITION (2), and ICON (2),
       a page with its native settings rows, one filter, and one status
       indicator. */
    assert(grkit_module.setting_count == 4 && grkit_module.settings[0].values == 6 &&
           grkit_module.settings[1].values == 3 && grkit_module.settings[2].values == 2 &&
           grkit_module.settings[3].values == 2);
    assert(grkit_module.registration_count == 3 && grkit_module.tile_count == 6);
    assert(SETTING_STYLE == GRKIT_SETTING(0) && SETTING_LOOK == GRKIT_SETTING(1) &&
           SETTING_POSITION == GRKIT_SETTING(2));
    /* Each look of each style lights the lower right corner of the picture
       and nothing else. */
    for (u32 look = 0; look < 3; ++look)
        for (u32 style = 1; style < 6; ++style) {
            PictureSet set = {{gray_plane(GRKIT_TEST_PLANES)}, 1, 0, 0, guarded + 1};
            for (u32 i = 0; i < sizeof guarded; ++i) guarded[i] = 0xa5;
            grkit_test_settings[SETTING_LOOK] = look;
            date_apply(&set, style);
            assert(changed(&set.planes[0], &left, &top) > 100);
            assert(left > WIDTH / 2 && top > HEIGHT / 2);
            /* Preserve every luma/chroma byte while reusing dirty scratch. */
            assert(pixels_hash(&set.planes[0]) == expected[look][style - 1]);
            assert(guarded[0] == 0xa5 && guarded[sizeof guarded - 1] == 0xa5);
        }
    /* Film: the stamp stays at the bottom right of the sensor frame in each
       posture. Upright: it goes to the bottom right of the picture turned
       upright, which is the top right of the frame for one quarter turn, and
       its text runs along the long side of its box. */
    for (u32 look = 0; look < 3; ++look)
        for (u32 position = 0; position < 2; ++position)
            for (u32 turns = 0; turns < 4; ++turns) {
                PictureSet set = {{gray_plane(GRKIT_TEST_PLANES)}, 1, 0, 0, grkit_test_scratch, turns};
                u32 right, bottom, corner = position ? turns : 0;
                grkit_test_settings[SETTING_LOOK] = look;
                grkit_test_settings[SETTING_POSITION] = position;
                date_apply(&set, 2);
                assert(bounds(&set.planes[0], &left, &top, &right, &bottom) > 100);
                /* Corner of the frame: 0 bottom right, 1 top right, 2 top left, 3 bottom left. */
                assert((corner <= 1) == (left > WIDTH / 2) && (corner >= 1 && corner <= 2) == (bottom < HEIGHT / 2));
                assert((corner & 1) == (bottom - top > right - left));
            }
    grkit_test_settings[SETTING_POSITION] = 0;
    /* A monochrome picture keeps its chroma. */
    {
        PictureSet set = {{gray_plane(GRKIT_TEST_PLANES)}, 1, 0, 1, grkit_test_scratch};
        grkit_test_settings[SETTING_LOOK] = 0;
        date_apply(&set, 1);
        for (u32 i = 0; i < WIDTH * HEIGHT; ++i) assert(GRKIT_BYTES(set.planes[0].c + i)[0] == 128);
    }
    return 0;
}
