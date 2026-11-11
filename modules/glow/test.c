/* Host behavior tests for Glow (grkit module test). */
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "grkit_module.h"

void glow_apply(PictureSet *set, u32 enabled);

/* Existing optical cases exercise one stream at a time. */
static u32 test_strength = 1;
static void set_strength(u32 strength) { test_strength = strength; }
static void apply_look(PictureSet *set, u32 look)
{
    setting_set(SETTING_BLOOM, look == 1 || look == 3 ? test_strength + 1 : 0);
    setting_set(SETTING_HALATION, look == 2 || look == 3 ? test_strength + 1 : 0);
    glow_apply(set, look >= 1 && look <= 3);
}

/* Compact optical case notation: 0 Off, 1 Weak, 2 Medium, 3 Strong. */
static void set_pair(u32 bloom, u32 halation)
{
    setting_set(SETTING_BLOOM, bloom);
    setting_set(SETTING_HALATION, halation);
}

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

/* A 32 x 32 square of one color in the middle. */
static void colored_square(u8 l, u8 cb, u8 cr)
{
    for (u32 y = 224; y < 256; ++y)
        for (u32 x = 304; x < 336; ++x) {
            *grkit_test_bytes(GRKIT_TEST_PLANES + y * W + x) = l;
            *grkit_test_bytes(GRKIT_TEST_PLANES + W * H + y * W + (x & ~1U)) = cb;
            *grkit_test_bytes(GRKIT_TEST_PLANES + W * H + y * W + (x & ~1U) + 1) = cr;
        }
}

static void square(void) { colored_square(255, 128, 128); }

static void patch(u32 left, u32 top, u32 size, u8 level)
{
    for (u32 y = top; y < top + size; ++y)
        for (u32 x = left; x < left + size; ++x)
            *grkit_test_bytes(GRKIT_TEST_PLANES + y * W + x) = level;
}

/* Measure the output outside the source, not its unchanged bright core.
   The fixed window includes the local halo for every tested source phase. */
#define HALO_LEFT 272U
#define HALO_TOP 192U
#define HALO_SIZE 96U
static u32 halo_energy(u32 left, u32 top, u32 size, u8 background)
{
    u32 energy = 0;
    for (u32 y = HALO_TOP; y < HALO_TOP + HALO_SIZE; ++y)
        for (u32 x = HALO_LEFT; x < HALO_LEFT + HALO_SIZE; ++x) {
            if (x >= left && x < left + size && y >= top && y < top + size) continue;
            assert(luma(x, y) >= background);
            energy += luma(x, y) - background;
        }
    return energy;
}

static void assert_neutral(void)
{
    for (u32 y = HALO_TOP; y < HALO_TOP + HALO_SIZE; ++y)
        for (u32 x = HALO_LEFT; x < HALO_LEFT + HALO_SIZE; x += 2) {
            assert(chroma(x, y, 0) == 128);
            assert(chroma(x, y, 1) == 128);
        }
}

static u32 square_energy(PictureSet *set, u32 look, u8 level)
{
    fill(40, 128, 128);
    colored_square(level, 128, 128);
    apply_look(set, look);
    return halo_energy(304, 224, 32, 40);
}

static void test_off(PictureSet *set)
{
    /* Include non-neutral chroma and every byte value in both planes. */
    for (u32 strength = 0; strength < 3; ++strength) {
        set_strength(strength);
        for (u32 i = 0; i < 2 * W * H; ++i)
            *grkit_test_bytes(GRKIT_TEST_PLANES + i) = (u8)(i * 37U + i / W);
        apply_look(set, 0);
        for (u32 i = 0; i < 2 * W * H; ++i)
            assert(*grkit_test_bytes(GRKIT_TEST_PLANES + i) == (u8)(i * 37U + i / W));
    }
    set_strength(1);
}

static u32 test_small_sources(PictureSet *set)
{
    u32 missing = 0;
    for (u32 look = 1; look <= 2; ++look)
        for (u32 size = 1; size <= 4; size *= 2) {
            u32 low = ~0U, high = 0, misses = 0;
            for (u32 dy = 0; dy < 8; ++dy)
                for (u32 dx = 0; dx < 8; ++dx) {
                    u32 left = 312 + dx, top = 232 + dy, energy;
                    fill(40, 128, 128);
                    /* The single pixel stays below the main clip level:
                       point compensation must not hide skipped samples. */
                    patch(left, top, size, size == 1 ? 244 : 255);
                    apply_look(set, look);
                    energy = halo_energy(left, top, size, 40);
                    if (!energy) ++misses;
                    if (energy < low) low = energy;
                    if (energy > high) high = energy;
                }
            /* A fully averaged single pixel can fall below output
               quantization. Detect missing phases only when another
               phase proves that this source has a measurable response. */
            if (misses && (size != 1 || high))
                fprintf(stderr, "look %u, %u-pixel source: %u/64 phases missing, energy %u..%u\n",
                        look, size, misses, low, high);
            if (size != 1 || high) missing += misses;
        }
    return missing;
}

static u32 test_separate_planes(void)
{
    const u32 sw = W / 2, sh = H / 2;
    const u32 sy = GRKIT_TEST_PLANES + 2 * W * H;
    Plane small = {sy, sy + sw * sh, sw, sh, sw, 0, 0, 0, sw, sh};
    PictureSet set = {{plane()}, 2, 0, 0, grkit_test_scratch, 0};
    u32 failures = 0;
    set.planes[1] = small;
    for (u32 look = 1; look <= 2; ++look) {
        u32 energy[2];
        for (u32 clipped = 0; clipped < 2; ++clipped) {
            fill(40, 128, 128);
            /* A compact cluster with half bright and half dark pixels.
               The 2x2 mean reduces each bright/dark pair to a midtone:
               both screennails stay below the ordinary highlight
               threshold; only one main picture has clipped pixels. */
            for (u32 y = 232; y < 240; ++y)
                for (u32 x = 312; x < 320; ++x)
                    if (((x + y) & 1U) == 0) patch(x, y, 1, clipped ? 255 : 244);
            for (u32 y = 0; y < sh; ++y)
                for (u32 x = 0; x < sw; ++x) {
                    u32 sum = luma(2 * x, 2 * y) + luma(2 * x + 1, 2 * y)
                            + luma(2 * x, 2 * y + 1) + luma(2 * x + 1, 2 * y + 1);
                    *grkit_test_bytes(small.y + y * sw + x) = (u8)((sum + 2) / 4);
                    *grkit_test_bytes(small.c + y * sw + x) = 128;
                }
            apply_look(&set, look);
            energy[clipped] = halo_energy(312, 232, 8, 40);
        }
        if (energy[0] != 0 || energy[1] == 0) {
            fprintf(stderr, "look %u, separate planes: unclipped halo %u, clipped halo %u\n",
                    look, energy[0], energy[1]);
            ++failures;
        }
    }
    return failures;
}

static u32 distance(u32 a, u32 b) { return a > b ? a - b : b - a; }

/* Red light may add warmth, but must not remove the underlying blue: an
   excess chroma gain subtracts blue, and the halo looks like opaque ink. */
static void test_halation_preserves_background(PictureSet *set)
{
    for (u32 strength = 0; strength < 3; ++strength) {
        set_strength(strength);
        fill(40, 128, 128);
        square();
        apply_look(set, 2);
        /* Chroma is shared: compare mean blue of every pixel pair,
           including both word phases, with three codes for rounding. */
        for (u32 y = HALO_TOP; y < HALO_TOP + HALO_SIZE; ++y)
            for (u32 x = HALO_LEFT; x < HALO_LEFT + HALO_SIZE; x += 2) {
                if (x >= 304 && x < 336 && y >= 224 && y < 256) continue;
                int blue = ((int)luma(x, y) + luma(x + 1, y) + 1) / 2 + ((454 * ((int)chroma(x, y, 0) - 128)) >> 8);
                assert(blue >= 37);
            }
    }
    set_strength(1);
}

/* Judge the displayed halo, not the linear-light channel coefficients.
   Outside white, warm and red lights, red must dominate the added green.
   The ratio tells a red rim from an orange one. */
static void test_halation_red_rim(PictureSet *set)
{
    const u8 sources[][3] = {{255, 128, 128}, {180, 76, 181}, {77, 85, 255}};
    for (u32 strength = 0; strength < 3; ++strength)
        for (u32 source = 0; source < 3; ++source) {
            u32 red_sum = 0, green_sum = 0, red_peak = 0;
            set_strength(strength);
            fill(40, 128, 128);
            colored_square(sources[source][0], sources[source][1], sources[source][2]);
            apply_look(set, 2);
            for (u32 y = HALO_TOP; y < HALO_TOP + HALO_SIZE; ++y)
                for (u32 x = HALO_LEFT; x < HALO_LEFT + HALO_SIZE; x += 2) {
                    if (x >= 304 && x < 336 && y >= 224 && y < 256) continue;
                    /* Compare pair means because both pixels share Cb/Cr. */
                    int dy = ((int)luma(x, y) + luma(x + 1, y) + 1) / 2 - 40;
                    int cb = (int)chroma(x, y, 0) - 128, cr = (int)chroma(x, y, 1) - 128;
                    int red = dy + ((359 * cr) >> 8), green = dy - ((88 * cb + 183 * cr) >> 8);
                    if (red > 0) red_sum += (u32)red;
                    if (green > 0) green_sum += (u32)green;
                    if (red > (int)red_peak) red_peak = (u32)red;
                }
            assert(red_peak >= 4);
            assert(red_sum > 3 * green_sum);
        }
    set_strength(1);
}

/* The displayed red, green, and blue that a halo adds outside a source on a
   gray 40 background, summed over the halo window, and its luma energy. */
typedef struct { u32 red, green, blue, energy; } Rim;
static Rim halation_rim(PictureSet *set, u32 color, u8 l, u8 cb, u8 cr)
{
    Rim rim = {0, 0, 0, 0};
    setting_set(SETTING_HALATION_COLOR, color);
    fill(40, 128, 128);
    colored_square(l, cb, cr);
    apply_look(set, 2);
    for (u32 y = HALO_TOP; y < HALO_TOP + HALO_SIZE; ++y)
        for (u32 x = HALO_LEFT; x < HALO_LEFT + HALO_SIZE; x += 2) {
            if (x >= 304 && x < 336 && y >= 224 && y < 256) continue;
            int dy = ((int)luma(x, y) + luma(x + 1, y) + 1) / 2 - 40;
            int c_b = (int)chroma(x, y, 0) - 128, c_r = (int)chroma(x, y, 1) - 128;
            int red = dy + ((359 * c_r) >> 8), green = dy - ((88 * c_b + 183 * c_r) >> 8), blue = dy + ((454 * c_b) >> 8);
            if (red > 0) rim.red += (u32)red;
            if (green > 0) rim.green += (u32)green;
            if (blue > 0) rim.blue += (u32)blue;
        }
    rim.energy = halo_energy(304, 224, 32, 40);
    setting_set(SETTING_HALATION_COLOR, 1);
    return rim;
}

/* Red, Orange, Gold, and White differ in the green of the rim: Red has
   almost none, Gold about two thirds of red, and White as much as red and
   blue. Each keeps about the luma of Orange. A monochrome picture, and a
   value outside the setting, take Orange. */
static void test_halation_colors(PictureSet *set)
{
    for (u32 strength = 0; strength < 3; ++strength) {
        set_strength(strength);
        Rim rim[4];
        for (u32 color = 0; color < 4; ++color) rim[color] = halation_rim(set, color, 255, 128, 128);
        assert(rim[0].red > 8 * rim[0].green);
        assert(rim[1].red > 3 * rim[1].green && rim[1].red < 6 * rim[1].green);
        assert(2 * rim[2].green > rim[2].red && 5 * rim[2].green < 4 * rim[2].red);
        assert(rim[3].red == rim[3].green && rim[3].green == rim[3].blue);
        for (u32 color = 0; color < 4; ++color) {
            assert(4 * rim[color].energy > 3 * rim[1].energy);
            assert(4 * rim[color].energy < 5 * rim[1].energy);
        }
        /* White has no color around a colored light either. */
        setting_set(SETTING_HALATION_COLOR, 3);
        fill(40, 128, 128);
        colored_square(180, 76, 181);
        apply_look(set, 2);
        for (u32 y = HALO_TOP; y < HALO_TOP + HALO_SIZE; ++y)
            for (u32 x = HALO_LEFT; x < HALO_LEFT + HALO_SIZE; x += 2) {
                if (x >= 304 && x < 336 && y >= 224 && y < 256) continue;
                assert(chroma(x, y, 0) == 128 && chroma(x, y, 1) == 128);
            }
        setting_set(SETTING_HALATION_COLOR, 1);
    }
    /* A monochrome picture and an invalid value give the Orange picture. */
    static u8 orange[W * H * 2];
    PictureSet mono = *set;
    mono.mono = 1;
    for (u32 pass = 0; pass < 2; ++pass) {
        PictureSet *target = pass ? &mono : set;
        setting_set(SETTING_HALATION_COLOR, 1);
        fill(40, 128, 128); square();
        apply_look(target, 2);
        memcpy(orange, (const void *)grkit_test_bytes(GRKIT_TEST_PLANES), sizeof(orange));
        const u32 colors[] = {0, 2, 3, 4, ~0U};
        for (u32 k = 0; k < 5; ++k) {
            if (!pass && k < 3) continue;
            setting_set(SETTING_HALATION_COLOR, colors[k]);
            fill(40, 128, 128); square();
            apply_look(target, 2);
            assert(!memcmp(orange, (const void *)grkit_test_bytes(GRKIT_TEST_PLANES), sizeof(orange)));
        }
    }
    setting_set(SETTING_HALATION_COLOR, 1);
    set_strength(1);
}

/* A fixed light map isolates output compositing from source detection. */
static PictureSet composite_fixture(u8 y, u8 cb, u8 cr, u32 half)
{
    const u32 sw = W / 2, sh = H / 2, sy = GRKIT_TEST_PLANES + 2 * W * H;
    PictureSet set = {{plane()}, 2, 0, 0, grkit_test_scratch, 0};
    Plane small = {sy, sy + sw * sh, sw, sh, sw, 0, 0, 0, sw, sh};
    fill(y, cb, cr);
    for (u32 row = 0; row < sh; ++row)
        for (u32 x = 0; x < sw; ++x) {
            *grkit_test_bytes(small.y + row * sw + x) = x >= 150 && x < 170 && row >= 112 && row < 128 ? 255 : 40;
            *grkit_test_bytes(small.c + row * sw + x) = 128;
        }
    set.planes[0].half = half;
    set.planes[1] = small;
    return set;
}

static void test_composite_protection(void)
{
    for (u32 look = 1; look <= 2; ++look) {
        /* Swapping the two neutral luma samples must not rotate their
           shared chroma. Exercise both pairs in a word and nearby phases. */
        for (u32 x = 292; x < 304; x += 2) {
            u32 expected[2] = {0, 0};
            for (u32 swap = 0; swap < 2; ++swap) {
                PictureSet set = composite_fixture(40, 128, 128, 0);
                patch(x, 240, 1, swap ? 240 : 20);
                patch(x + 1, 240, 1, swap ? 20 : 240);
                apply_look(&set, look);
                for (u32 c = 0; c < 2; ++c) {
                    if (!swap) expected[c] = chroma(x, 240, c);
                    else assert(chroma(x, 240, c) == expected[c]);
                }
                assert(luma(x + swap, 240) > 20);
                if (look == 2) assert(expected[0] != 128 || expected[1] != 128);
            }
        }
        /* Saturated red and blue cores have no remaining RGB headroom,
           even though their luma is far below white. Cover every phase. */
        for (u32 blue = 0; blue < 2; ++blue) {
            u8 y = blue ? 30 : 77, cb = blue ? 255 : 85, cr = blue ? 107 : 255;
            PictureSet set = composite_fixture(y, cb, cr, 0);
            apply_look(&set, look);
            for (u32 row = 220; row < 260; ++row)
                for (u32 x = 280; x < 360; ++x) {
                    assert(luma(x, row) == y);
                    assert(chroma(x, row, 0) == cb && chroma(x, row, 1) == cr);
                }
        }
        /* A half-height plane must derive odd-row headroom from the same
           original chroma as a full-height plane, before the even write. */
        {
            u8 expected[40][80];
            for (u32 odd_top = 0; odd_top < 2; ++odd_top)
                for (u32 half = 0; half < 2; ++half) {
                    PictureSet set = composite_fixture(85, 96, 209, half);
                    set.planes[0].top = odd_top ? 221 : 0;
                    apply_look(&set, look);
                    for (u32 row = 220; row < 260; ++row)
                        for (u32 x = 280; x < 360; ++x) {
                            if (!half) expected[row - 220][x - 280] = luma(x, row);
                            else assert(luma(x, row) == expected[row - 220][x - 280]);
                        }
                }
        }
        /* Partial words may be loaded, but pixels and shared pairs
           outside an odd crop boundary must remain untouched. */
        for (u32 half = 0; half < 2; ++half)
        for (u32 left = 293; left <= 295; ++left) {
            PictureSet set = composite_fixture(40, 128, 128, half);
            set.planes[0].left = left;
            set.planes[0].right = 345;
            apply_look(&set, look);
            for (u32 x = left & ~3U; x < left; ++x) assert(luma(x, 240) == 40);
            for (u32 x = 345; x < 348; ++x) assert(luma(x, 240) == 40);
            if (left & 1) assert(chroma(left, 240 >> half, 0) == 128 && chroma(left, 240 >> half, 1) == 128);
            assert(chroma(344, 240 >> half, 0) == 128 && chroma(344, 240 >> half, 1) == 128);
        }
    }
}

static void test_daylight_transition(PictureSet *set)
{
    u32 first = 0, last = ~0U;
    grkit_test_shot.known = GRKIT_SHOT_EXPOSURE;
    /* A midtone surround exercises the gate. Very bright backgrounds
       already have no halo at night and cannot prove that this gate works. */
    for (u32 ev = 160; ev <= 184; ++ev) {
        grkit_test_shot.scene_ev = (s32)(ev << 4);
        fill(128, 128, 128);
        square();
        apply_look(set, 2);
        u32 energy = halo_energy(304, 224, 32, 128);
        if (ev == 160) first = energy;
        assert(energy <= last);
        last = energy;
    }
    assert(last > 0 && last < first);
    grkit_test_shot.known = 0;
}

/* Local daylight rejection: a light inside a bright local surround (a
   sunlit specular highlight on a lit surface) loses its Halation in
   daylight, although the wide surround and the whole picture are dark, so
   that neither the surround weight nor day_keep can remove it. */
static u32 local_gate_energy(PictureSet *set, s32 ev)
{
    static u8 before[160][160];
    u32 energy = 0;
    fill(40, 128, 128);
    patch(264, 184, 112, 200);
    square();
    for (u32 y = 0; y < 160; ++y)
        for (u32 x = 0; x < 160; ++x) before[y][x] = luma(240 + x, 160 + y);
    grkit_test_shot.known = GRKIT_SHOT_EXPOSURE;
    grkit_test_shot.scene_ev = ev;
    apply_look(set, 2);
    grkit_test_shot.known = 0;
    for (u32 y = 0; y < 160; ++y)
        for (u32 x = 0; x < 160; ++x) {
            u8 now = luma(240 + x, 160 + y);
            assert(now >= before[y][x]);
            energy += now - before[y][x];
        }
    return energy;
}

static void test_local_daylight_gate(PictureSet *set)
{
    u32 night = local_gate_energy(set, 5 * 256), day = local_gate_energy(set, 11 * 256 + 128);
    assert(night > 0);
    assert(day * 4 < night);
}

/* The halo energy of a white square on `background` at a scene EV. */
static u32 ev_energy(PictureSet *set, u32 look, s32 ev, u8 background)
{
    grkit_test_shot.known = GRKIT_SHOT_EXPOSURE;
    grkit_test_shot.scene_ev = ev << 8;
    fill(background, 128, 128);
    square();
    apply_look(set, look);
    grkit_test_shot.known = 0;
    return halo_energy(304, 224, 32, background);
}

/* Halation fades out in daylight from 11 EV to 13 EV, unless the picture
   is mostly dark (a lit window at night can meter like daylight). Bloom
   does not depend on the scene EV. */
static void test_daylight_fade(PictureSet *set)
{
    u32 dusk = ev_energy(set, 2, 11, 128), day = ev_energy(set, 2, 12, 128);
    assert(dusk > 0 && day > 0 && day < dusk);
    /* Full daylight: no change at all. */
    fill(128, 128, 128);
    square();
    grkit_test_shot.known = GRKIT_SHOT_EXPOSURE;
    grkit_test_shot.scene_ev = 13 << 8;
    apply_look(set, 2);
    grkit_test_shot.known = 0;
    for (u32 y = 0; y < H; ++y)
        for (u32 x = 0; x < W; ++x) {
            u8 expected = x >= 304 && x < 336 && y >= 224 && y < 256 ? 255 : 128;
            assert(luma(x, y) == expected && chroma(x, y, 0) == 128 && chroma(x, y, 1) == 128);
        }
    u32 saved_strength = test_strength;
    for (u32 strength = 0; strength < 3; ++strength) {
        set_strength(strength);
        /* A dark picture keeps its glow at a daylight EV. */
        assert(ev_energy(set, 2, 14, 20) == ev_energy(set, 2, 9, 20) && ev_energy(set, 2, 9, 20) > 0);
        /* Bloom keeps its glow at any EV. Strong adds its lift below 10 EV. */
        u32 day = ev_energy(set, 1, 14, 128), dusk = ev_energy(set, 1, 9, 128);
        assert(day > 0 && (strength == 2 ? dusk > day : dusk == day));
        if (strength == 2) assert(ev_energy(set, 1, 10, 128) == day);
    }
    set_strength(saved_strength);
}

/* The halo of a light square on a background, at a scene EV (1/256 EV), or
   without one when ev < 0. */
static u32 shot_energy(PictureSet *set, u32 look, s32 ev, u8 background)
{
    grkit_test_shot.known = ev < 0 ? 0 : GRKIT_SHOT_EXPOSURE;
    grkit_test_shot.scene_ev = ev;
    fill(background, 128, 128);
    square();
    apply_look(set, look);
    grkit_test_shot.known = 0;
    return halo_energy(304, 224, 32, background);
}

/* Strong lifts a lamp in a lit room (a light surround) below 10 EV. Weak and
   Medium, and a shot without an EV, keep the surround weight alone. */
static void test_strong_lift(PictureSet *set)
{
    for (u32 look = 1; look <= 2; ++look) {
        set_strength(1);
        assert(shot_energy(set, look, 7 << 8, 180) == shot_energy(set, look, -1, 180));
        set_strength(2);
        u32 room = shot_energy(set, look, 7 << 8, 180), plain = shot_energy(set, look, -1, 180);
        assert(room > 4 * plain && room > 0);
        /* The lift fades out by 10 EV. */
        assert(shot_energy(set, look, 10 << 8, 180) == plain);
    }
    set_strength(1);
}

/* The light that a large light area (the left half) gives the dark side next
   to it, at a scene EV (1/256 EV), or without one when ev < 0. */
static u32 veil_energy(PictureSet *set, u32 look, s32 ev)
{
    grkit_test_shot.known = ev < 0 ? 0 : GRKIT_SHOT_EXPOSURE;
    grkit_test_shot.scene_ev = ev;
    fill(30, 128, 128);
    for (u32 y = 0; y < H; ++y)
        for (u32 x = 0; x < W / 2; ++x) *grkit_test_bytes(GRKIT_TEST_PLANES + y * W + x) = 255;
    apply_look(set, look);
    grkit_test_shot.known = 0;
    u32 energy = 0;
    for (u32 y = 0; y < H; ++y)
        for (u32 x = W / 2; x < W / 2 + 96; ++x) energy += luma(x, y) - 30U;
    return energy;
}

/* Strong does not lay its lift over the dark side of a large light area:
   without veil_guard, that side takes more than three times the light. */
static void test_strong_veil(PictureSet *set)
{
    set_strength(2);
    for (u32 look = 1; look <= 2; ++look) {
        u32 lifted = veil_energy(set, look, 7 << 8), plain = veil_energy(set, look, -1);
        assert(plain > 0 && 4 * lifted < 5 * plain);
    }
    set_strength(1);
}

/* Strong: the glow on the dark side of a slanted edge of a large light
   area (the bottom of a screen) is even along the edge. A fine pixel that
   holds both the light and the dark side must not escape the veil guard,
   and the cut must not change in steps of fine pixels. Before, the glow 2
   rows below the edge went from 33 to 71 (Bloom) and from 9 to 24
   (Halation) along it. The edge steps one row each 8 columns, so the
   distance to it varies by a row; small values also round. */
static void test_strong_veil_edge(PictureSet *set)
{
    set_strength(2);
    for (u32 look = 1; look <= 2; ++look) {
        u32 most = 0, least = ~0U;
        grkit_test_shot.known = GRKIT_SHOT_EXPOSURE;
        grkit_test_shot.scene_ev = 7 << 8;
        fill(30, 128, 128);
        /* Light down to row 200 + x / 8: one row lower each 8 columns. */
        for (u32 x = 0; x < W; ++x)
            for (u32 y = 0; y < 200 + x / 8; ++y) *grkit_test_bytes(GRKIT_TEST_PLANES + y * W + x) = 255;
        apply_look(set, look);
        grkit_test_shot.known = 0;
        for (u32 x = 96; x < W - 96; ++x) {
            u32 glow = luma(x, 200 + x / 8 + 2) - 30U;
            if (glow > most) most = glow;
            if (glow < least) least = glow;
        }
        assert(least > 0 && most < least + least / 2 + 4);
    }
    set_strength(1);
}

/* Bloom: a lamp clipped to a white core, with an unclipped orange
   fall-off around it, glows orange and not white. */
static void test_bloom_rim_color(PictureSet *set)
{
    fill(40, 128, 128);
    for (u32 y = 220; y < 260; ++y)
        for (u32 x = 300; x < 340; ++x) {
            /* R 230, G 120, B 40: bright, below the clip */
            *grkit_test_bytes(GRKIT_TEST_PLANES + y * W + x) = 144;
            *grkit_test_bytes(GRKIT_TEST_PLANES + W * H + y * W + (x & ~1U)) = 69;
            *grkit_test_bytes(GRKIT_TEST_PLANES + W * H + y * W + (x & ~1U) + 1) = 190;
        }
    square();
    apply_look(set, 1);
    /* Outside the fall-off, left of it. */
    assert(luma(292, 240) > 40);
    assert(chroma(292, 240, 0) <= 124 && chroma(292, 240, 1) >= 133);
}

/* Bloom: the color of a fall-off reaches only a few cells into a clipped
   area (RIM_DECAY). A wide white area with an orange fall-off on its left
   side glows orange on the left, and neutral on the right. */
static void test_bloom_rim_reach(PictureSet *set)
{
    fill(40, 128, 128);
    for (u32 y = 176; y < 304; ++y)
        for (u32 x = 152; x < 168; ++x) {
            *grkit_test_bytes(GRKIT_TEST_PLANES + y * W + x) = 144;
            *grkit_test_bytes(GRKIT_TEST_PLANES + W * H + y * W + (x & ~1U)) = 69;
            *grkit_test_bytes(GRKIT_TEST_PLANES + W * H + y * W + (x & ~1U) + 1) = 190;
        }
    patch(168, 176, 128, 255);
    for (u32 y = 176; y < 304; ++y)
        for (u32 x = 168; x < 296; x += 2) {
            *grkit_test_bytes(GRKIT_TEST_PLANES + W * H + y * W + x) = 128;
            *grkit_test_bytes(GRKIT_TEST_PLANES + W * H + y * W + x + 1) = 128;
        }
    apply_look(set, 1);
    /* Left of the fall-off: orange. Right of the white area, far from the
       fall-off: a glow without the orange. */
    assert(luma(146, 240) > 40 && chroma(146, 240, 1) >= 131);
    assert(luma(300, 240) > 40);
    assert(chroma(300, 240, 0) >= 126 && chroma(300, 240, 1) <= 130);
}

/* Halation keeps the color model of film (Look.rim 0): a white core with a
   blue fall-off still makes its orange-red rim, not a blue glow. */
static void test_halation_no_rim(PictureSet *set)
{
    u32 white, blue;
    white = square_energy(set, 2, 255);
    fill(40, 128, 128);
    for (u32 y = 220; y < 260; ++y)
        for (u32 x = 300; x < 340; ++x) {
            /* R 40, G 60, B 230 */
            *grkit_test_bytes(GRKIT_TEST_PLANES + y * W + x) = 69;
            *grkit_test_bytes(GRKIT_TEST_PLANES + W * H + y * W + (x & ~1U)) = 217;
            *grkit_test_bytes(GRKIT_TEST_PLANES + W * H + y * W + (x & ~1U) + 1) = 114;
        }
    square();
    apply_look(set, 2);
    blue = 0;
    for (u32 y = 200; y < 280; ++y) blue += luma(292, y) - 40;
    assert(white > 0 && blue > 0);
    assert(chroma(292, 240, 1) > 128);
}

/* day_keep: a daylight EV fades Halation out unless the picture is mostly
   dark (DARK_SCENE of the cells under DARK_LUMA). A light in a dark
   picture with a thin bright band keeps its halo; with a wide band, the
   picture is not dark and gets none. */
static u32 band_energy(PictureSet *set, u32 band)
{
    fill(20, 128, 128);
    for (u32 y = 0; y < band; ++y)
        for (u32 x = 0; x < W; ++x) *grkit_test_bytes(GRKIT_TEST_PLANES + y * W + x) = 128;
    square();
    grkit_test_shot.known = GRKIT_SHOT_EXPOSURE;
    grkit_test_shot.scene_ev = 14 << 8;
    apply_look(set, 2);
    grkit_test_shot.known = 0;
    return halo_energy(304, 224, 32, 20);
}

static void test_dark_scene(PictureSet *set)
{
    assert(band_energy(set, 40) > 0);
    assert(band_energy(set, 120) == 0);
}

/* A stored strength out of range glows as Medium. */
static void test_strength_range(PictureSet *set)
{
    for (u32 look = 1; look <= 2; ++look) {
        set_strength(7);
        assert(square_energy(set, look, 255) == 0);
    }
    set_strength(1);
}

/* The planes of a camera: a main picture, the 720 x 480 screennail that the
   light comes from, and a thumbnail with a letterbox (3:2). Every plane
   glows at the light, nothing glows far from it, and the scratch that the
   core leaves from earlier calls does not change the output. */
#define MAIN_W 1440U
#define MAIN_H 960U
static void camera_planes(PictureSet *set)
{
    u32 at = GRKIT_TEST_PLANES, sizes[3][4] = {{MAIN_W, MAIN_H, 0, MAIN_H}, {720, 480, 0, 480}, {160, 120, 8, 112}};
    for (u32 k = 0; k < 3; ++k) {
        u32 w = sizes[k][0], h = sizes[k][1];
        Plane p = {at, at + w * h, w, h, w, 0, 0, sizes[k][2], w, sizes[k][3]};
        set->planes[k] = p;
        /* Dark, with a white square at the middle third of the middle. */
        for (u32 y = 0; y < h; ++y)
            for (u32 x = 0; x < w; ++x) {
                u32 light = x >= w * 15 / 32 && x < w * 17 / 32 && y >= h * 15 / 32 && y < h * 17 / 32;
                *grkit_test_bytes(p.y + y * w + x) = light ? 255 : 30;
                *grkit_test_bytes(p.c + y * w + x) = 128;
            }
        at += 2 * w * h;
    }
    set->count = 3;
}

/* The glow on a wide plane is where its light is. The plane takes its glow
   from a coarse grid of the source plane, and the scale between the two
   lost up to 0.6 of a grid column by the middle of a 6000-pixel row, so
   the glow moved to the right with the column: about 9 pixels at the
   middle of a 6000 x 4000 main picture, 18 at its right side. Here a
   narrow source makes each grid column about 47 pixels of the plane. */
static void test_wide_plane_alignment(void)
{
    enum { SW = 256, SH = 128, PW = 6000, PH = 200, LX = 200, LY = 60 };
    u32 at = GRKIT_TEST_PLANES, sizes[2][2] = {{PW, PH}, {SW, SH}};
    PictureSet set = {{plane()}, 2, 0, 0, grkit_test_scratch, 0};
    for (u32 k = 0; k < 2; ++k) {
        u32 w = sizes[k][0], h = sizes[k][1];
        Plane p = {at, at + w * h, w, h, w, 0, 0, 0, w, h};
        set.planes[k] = p;
        /* Dark, with a light 8 x 8 source pixels at (LX, LY) of the source. */
        for (u32 y = 0; y < h; ++y)
            for (u32 x = 0; x < w; ++x) {
                u32 sx = x * SW / w, sy = y * SH / h, light = sx >= LX && sx < LX + 8 && sy >= LY && sy < LY + 8;
                *grkit_test_bytes(p.y + y * w + x) = light ? 255 : 30;
                *grkit_test_bytes(p.c + y * w + x) = 128;
            }
        at += 2 * w * h;
    }
    apply_look(&set, 1);
    /* The center of the light on the plane, and the mean column of the glow
       of a row through it, with the light itself left out. */
    {
        const Plane *p = &set.planes[0];
        u32 row = (2 * LY + 8) * PH / (2 * SH), x0 = (LX * PW + SW - 1) / SW, x1 = ((LX + 8) * PW + SW - 1) / SW;
        double center = (x0 + x1) / 2.0, sum = 0, moment = 0;
        for (u32 x = 0; x < PW; ++x) {
            u32 glow = *grkit_test_bytes(p->y + row * PW + x) - 30U;
            if (x >= x0 && x < x1) continue;
            sum += glow;
            moment += glow * (x + 0.5);
        }
        assert(sum > 0 && moment / sum - center < 4 && center - moment / sum < 4);
    }
}

static u32 plane_sum(const Plane *p)
{
    u32 s = 0;
    for (u32 i = 0; i < 2 * p->width * p->height; ++i) s = s * 31 + *grkit_test_bytes(p->y + i);
    return s;
}

static void test_camera_planes(void)
{
    for (u32 look = 1; look <= 3; ++look) {
        PictureSet set = {{plane()}, 1, 0, 0, grkit_test_scratch, 0};
        u32 clean[3], dirty[3];
        for (u32 i = 0; i < GRKIT_PLANE_SCRATCH; ++i) grkit_test_scratch[i] = 0;
        camera_planes(&set);
        apply_look(&set, look);
        for (u32 k = 0; k < 3; ++k) {
            const Plane *p = &set.planes[k];
            u32 cx = p->width / 2, cy = (p->top + p->bottom) / 2, edge = p->width * 15 / 32;
            clean[k] = plane_sum(p);
            /* Just left of the light it glows; at the left border, not. */
            assert(*grkit_test_bytes(p->y + cy * p->pitch + edge - 2) > 30);
            assert(*grkit_test_bytes(p->y + cy * p->pitch) == 30);
            (void)cx;
        }
        for (u32 i = 0; i < GRKIT_PLANE_SCRATCH; ++i) grkit_test_scratch[i] = (u8)(0xa5 ^ i);
        camera_planes(&set);
        apply_look(&set, look);
        for (u32 k = 0; k < 3; ++k) {
            dirty[k] = plane_sum(&set.planes[k]);
            assert(dirty[k] == clean[k]);
        }
    }
    for (u32 i = 0; i < GRKIT_PLANE_SCRATCH; ++i) grkit_test_scratch[i] = 0;
}

static u32 test_highlight_continuity(PictureSet *set)
{
    u32 failures = 0;
    for (u32 look = 1; look <= 2; ++look) {
        u32 energy[36];
        for (u32 level = 220; level <= 255; ++level)
            energy[level - 220] = square_energy(set, look, (u8)level);
        /* A one-code transition must not exceed all eight adjacent code
           transitions combined. One output code per measured pixel covers
           quantization of the summed 8-bit halo. This detects a hard gain
           switch without prescribing a curve, threshold, or effect gain. */
        for (u32 i = 5; i + 4 < 36; ++i) {
            u32 neighbors = 0;
            u32 jump = distance(energy[i], energy[i - 1]);
            u32 quantization = HALO_SIZE * HALO_SIZE - 32U * 32U;
            for (u32 j = i - 4; j <= i + 4; ++j)
                if (j != i) neighbors += distance(energy[j], energy[j - 1]);
            if (jump > neighbors + quantization) {
                fprintf(stderr, "look %u, level %u: halo jump %u exceeds neighbors %u + quantization %u\n",
                        look, i + 220, jump, neighbors, quantization);
                ++failures;
            }
        }
    }
    return failures;
}

/* A block optimization must not cut off the interpolation before its first
   nonzero sample. Move a small light through all 32 output-block phases. */
static void test_halo_block_edges(PictureSet *set)
{
    for (u32 look = 1; look <= 2; ++look)
        for (u32 dx = 0; dx < 32; ++dx) {
            u32 left = 300 + dx, first = 0, peak = 0;
            fill(40, 128, 128);
            patch(left, 238, 2, 255);
            apply_look(set, look);
            for (u32 x = left - 24; x < left; ++x) {
                u32 light = luma(x, 239) - 40U;
                if (light && !first) first = light;
                if (light > peak) peak = light;
            }
            assert(peak > 0);
            /* One code of rounding per comparison term is sufficient. */
            assert(first * 4 <= peak + 4);
        }
}

static void test_bloom_broad_source(PictureSet *set)
{
    u32 energy[2], sizes[2] = {8, 240};
    for (u32 i = 0; i < 2; ++i) {
        u32 size = sizes[i], left = W / 2 - size / 2, top = H / 2 - size / 2;
        fill(40, 128, 128);
        patch(left, top, size, 255);
        apply_look(set, 1);
        energy[i] = 0;
        for (u32 y = 0; y < H; ++y)
            for (u32 x = 0; x < W; ++x)
                if (x < left || x >= left + size || y < top || y >= top + size)
                    energy[i] += luma(x, y) - 40U;
    }
    assert(energy[0] > 0);
    /* Per unit edge length, a broad white surface scatters less than
       two thirds as much as the compact lamp. */
    assert(energy[1] * sizes[0] * 3 < energy[0] * sizes[1] * 2);
}

/* A saturated source may cross the emission threshold before white,
   but its halo must still grow with measured brightness. Keep the source
   position and size fixed so the response is not a sampling-phase test. */
static void test_halation_source_response(PictureSet *set)
{
    const u8 levels[] = {224, 232, 240, 248, 255};
    for (u32 strength = 0; strength < 3; ++strength) {
        u32 energies[5];
        set_strength(strength);
        for (u32 k = 0; k < 5; ++k) {
            u32 r = levels[k];
            fill(40, 128, 128);
            colored_square((77 * r + 128) >> 8, 128 + ((-43 * (int)r + 128) >> 8), (r == 255 ? 255 : 128 + ((128 * r + 128) >> 8)));
            apply_look(set, 2);
            energies[k] = halo_energy(304, 224, 32, 40);
        }
        assert(energies[0] > 0);
        for (u32 k = 1; k < 5; ++k) assert(energies[k] > energies[k - 1]);
        assert(energies[1] * 5 < energies[4] * 4);
    }
    set_strength(1);
}

/* A film can map its brightest input below 255. Its white core must stay
   unchanged while the same source still produces a red outer halo. */
static void test_halation_film_white(void)
{
    const u8 whites[][3] = {{227, 227, 227}, {212, 216, 206}};
    const u8 sources[][3] = {{227, 128, 128}, {214, 124, 127}};
    for (u32 source = 0; source < 2; ++source)
        for (u32 half = 0; half < 2; ++half)
            for (u32 strength = 0; strength < 3; ++strength) {
                PictureSet set = {{plane()}, 1, 0, 0, grkit_test_scratch, 0};
                u32 red = 0;
                set.planes[0].half = half;
                set_strength(strength);
                for (u32 k = 0; k < 3; ++k) grkit_test_white[k] = whites[source][k];
                fill(40, 128, 128);
                patch(304, 224, 32, sources[source][0]);
                for (u32 y = 224; y < 256; ++y)
                    for (u32 x = 304; x < 336; x += 2) {
                        *grkit_test_bytes(GRKIT_TEST_PLANES + W * H + (y >> half) * W + x) = sources[source][1];
                        *grkit_test_bytes(GRKIT_TEST_PLANES + W * H + (y >> half) * W + x + 1) = sources[source][2];
                    }
                apply_look(&set, 2);
                for (u32 y = 224; y < 256; ++y)
                    for (u32 x = 304; x < 336; ++x) {
                        assert(luma(x, y) == sources[source][0]);
                        assert(chroma(x, y >> half, 0) == sources[source][1]);
                        assert(chroma(x, y >> half, 1) == sources[source][2]);
                    }
                assert(halo_energy(304, 224, 32, 40) > 0);
                for (u32 y = HALO_TOP; y < HALO_TOP + HALO_SIZE; ++y)
                    for (u32 x = HALO_LEFT; x < HALO_LEFT + HALO_SIZE; x += 2) {
                        if (x >= 304 && x < 336 && y >= 224 && y < 256) continue;
                        if (chroma(x, y >> half, 1) > 128) ++red;
                    }
                assert(red > 0);
            }
    for (u32 k = 0; k < 3; ++k) grkit_test_white[k] = 255;
    set_strength(1);
}

/* Half-height chroma belongs to both rows. A bright core in either row
   must protect that shared color without suppressing the dark-row luma. */
static void test_halation_half_core(void)
{
    const u8 whites[][3] = {{255, 255, 255}, {227, 227, 227}, {212, 216, 206}, {255, 255, 255}};
    const u8 sources[][3] = {{255, 128, 128}, {227, 128, 128}, {214, 124, 127}, {77, 85, 255}};
    for (u32 source = 0; source < 4; ++source)
        for (u32 strength = 0; strength < 3; ++strength)
            for (u32 swap = 0; swap < 2; ++swap) {
                PictureSet set = composite_fixture(40, 128, 128, 1);
                u32 row = 238 + swap;
                set_strength(strength);
                for (u32 k = 0; k < 3; ++k) grkit_test_white[k] = whites[source][k];
                for (u32 x = 300; x < 302; ++x)
                    *grkit_test_bytes(set.planes[0].y + row * W + x) = sources[source][0];
                *grkit_test_bytes(set.planes[0].c + 119 * W + 300) = sources[source][1];
                *grkit_test_bytes(set.planes[0].c + 119 * W + 301) = sources[source][2];
                apply_look(&set, 2);
                for (u32 x = 300; x < 302; ++x) assert(luma(x, row) == sources[source][0]);
                assert(chroma(300, 119, 0) == sources[source][1]);
                assert(chroma(300, 119, 1) == sources[source][2]);
                assert(luma(300, 239 - swap) > 40);
                /* A neighboring pair with two dark rows retains its halo. */
                assert(luma(298, 238) > 40 && luma(298, 239) > 40);
                assert(chroma(298, 119, 1) > 128);
            }
    for (u32 k = 0; k < 3; ++k) grkit_test_white[k] = 255;
    set_strength(1);

    /* A shared chroma row crossing a vertical crop boundary is outside
       the writable region. Interior pairs must still receive the halo.
       Include an odd physical height and the normal final paired row. */
    for (u32 edge = 0; edge < 4; ++edge) {
        PictureSet set = composite_fixture(40, 128, 128, 1);
        Plane *p = &set.planes[0], *source = &set.planes[1];
        u32 row;
        if (edge == 0) {
            p->top = 239;
            source->top = 119;
            row = p->top;
        } else {
            p->bottom = edge == 1 ? 241 : edge == 2 ? H - 1 : H;
            p->height = edge == 2 ? H - 1 : H;
            source->bottom = edge == 1 ? 120 : H / 2;
            row = p->bottom - 1;
            for (u32 y = source->bottom - 16; y < source->bottom; ++y)
                for (u32 x = 150; x < 170; ++x)
                    *grkit_test_bytes(source->y + y * source->pitch + x) = 255;
        }
        apply_look(&set, 2);
        assert(luma(310, row) > 40);
        if (edge < 3) {
            for (u32 x = 0; x < W; ++x) {
                assert(chroma(x, row >> 1, 0) == 128);
                assert(chroma(x, row >> 1, 1) == 128);
                assert(luma(x, edge == 0 ? row - 1 : row + 1) == 40);
            }
        } else assert(chroma(310, row >> 1, 1) > 128);
        row = edge == 0 ? row + 1 : (row - 2) & ~1U;
        assert(luma(310, row) > 40 && chroma(310, row >> 1, 1) > 128);
    }
}

/* Each public setting controls only its own stream. All sixteen settings
   have distinct outputs on a white light, and either strength increases
   its halo without needing to change the other setting. */
static void test_combined_strengths(PictureSet *set)
{
    u32 hashes[4][4], energy[4][4];
    grkit_test_shot.known = 0;
    for (u32 b = 0; b <= 3; ++b)
        for (u32 h = 0; h <= 3; ++h) {
            fill(40, 128, 128);
            square();
            set_pair(b, h);
            glow_apply(set, 1);
            hashes[b][h] = plane_sum(&set->planes[0]);
            energy[b][h] = halo_energy(304, 224, 32, 40);
            assert(setting_get(SETTING_BLOOM) == b && setting_get(SETTING_HALATION) == h);
            if (b) assert(energy[b][h] > energy[b - 1][h]);
            if (h) assert(energy[b][h] > energy[b][h - 1]);
            for (u32 i = 0; i <= b; ++i)
                for (u32 j = 0; j <= 3 && (i < b || j < h); ++j)
                    assert(hashes[b][h] != hashes[i][j]);
        }
    assert(energy[0][0] == 0);
    /* A filter value of 0 never edits a pixel. */
    set_pair(3, 3);
    fill(40, 120, 140); square();
    u32 unfiltered = plane_sum(&set->planes[0]);
    glow_apply(set, 0);
    assert(plane_sum(&set->planes[0]) == unfiltered);
    /* A value outside a setting disables only that stream. */
    const u32 invalid[] = {4, ~0U};
    for (u32 i = 0; i < sizeof invalid / sizeof invalid[0]; ++i)
        for (u32 stream = 0; stream < 2; ++stream) {
            set_pair(3, 3);
            setting_set(stream ? SETTING_HALATION : SETTING_BLOOM, invalid[i]);
            fill(40, 128, 128); square();
            glow_apply(set, 1);
            assert(plane_sum(&set->planes[0]) == (stream ? hashes[3][0] : hashes[0][3]));
        }
    setting_set(SETTING_BLOOM, 4);
    setting_set(SETTING_HALATION, ~0U);
    fill(40, 120, 140); square();
    u32 before = plane_sum(&set->planes[0]);
    glow_apply(set, 1);
    assert(plane_sum(&set->planes[0]) == before);
}

/* The two rings and maps are independent despite sharing the scratch and
   source planes. Exercise every combination with crops, 4:2:0, monochrome,
   film white, and a dirty scratch. Guard bytes check the fixed budget. */
static void test_combined_scratch(void)
{
    static _Alignas(8) u8 scratch[GRKIT_PLANE_SCRATCH + 32];
    for (u32 mode = 0; mode < 8; ++mode) {
        PictureSet set = {{plane()}, 1, 0, mode & 1, scratch + 16, 0};
        Plane *p = &set.planes[0];
        p->half = (mode >> 1) & 1;
        if (mode & 4) { p->left = 3; p->top = 5; p->right -= 3; p->bottom -= 3; }
        grkit_test_white[0] = mode & 4 ? 212 : 255;
        grkit_test_white[1] = mode & 4 ? 216 : 255;
        grkit_test_white[2] = mode & 4 ? 206 : 255;
        for (u32 b = 0; b <= 3; ++b)
            for (u32 h = 0; h <= 3; ++h) {
                u32 expected = 0;
                set_pair(b, h);
                for (u32 dirty = 0; dirty < 2; ++dirty) {
                    for (u32 i = 0; i < sizeof scratch; ++i) scratch[i] = (u8)(dirty ? 0xa5 ^ i : 0);
                    fill(40, 128, 128); square();
                    glow_apply(&set, 1);
                    u32 actual = plane_sum(p);
                    if (dirty) assert(actual == expected);
                    else expected = actual;
                    assert(luma(0, 0) == 40 && luma(W - 1, H - 1) == 40);
                    for (u32 i = 0; i < 16; ++i) {
                        assert(scratch[i] == (u8)(dirty ? 0xa5 ^ i : 0));
                        u32 j = sizeof scratch - 16 + i;
                        assert(scratch[j] == (u8)(dirty ? 0xa5 ^ j : 0));
                    }
                }
            }
    }
    grkit_test_white[0] = grkit_test_white[1] = grkit_test_white[2] = 255;
}

/* Daylight eliminates Halation before the two streams meet. Enabling it
   must then leave Bloom byte-identical, including film-white protection. */
static void test_empty_halation(void)
{
    static u8 expected[W * H * 2];
    grkit_test_shot.known = GRKIT_SHOT_EXPOSURE;
    grkit_test_shot.scene_ev = 14 * 256;
    for (u32 mode = 0; mode < 4; ++mode) {
        PictureSet set = {{plane()}, 1, 0, mode & 1, grkit_test_scratch, 0};
        set.planes[0].half = mode >> 1;
        grkit_test_white[0] = 212; grkit_test_white[1] = 216; grkit_test_white[2] = 206;
        for (u32 b = 1; b <= 3; ++b) {
            for (u32 h = 0; h <= 3; ++h) {
                fill(100, 128, 128); square();
                set_pair(b, h);
                glow_apply(&set, 1);
                for (u32 i = 0; i < sizeof expected; ++i) {
                    u8 actual = *grkit_test_bytes(GRKIT_TEST_PLANES + i);
                    if (!h) expected[i] = actual;
                    else assert(actual == expected[i]);
                }
            }
        }
        for (u32 h = 1; h <= 3; ++h) {
            fill(100, 128, 128); square();
            for (u32 i = 0; i < sizeof expected; ++i)
                expected[i] = *grkit_test_bytes(GRKIT_TEST_PLANES + i);
            set_pair(0, h);
            glow_apply(&set, 1);
            for (u32 i = 0; i < sizeof expected; ++i)
                assert(expected[i] == *grkit_test_bytes(GRKIT_TEST_PLANES + i));
        }
    }
    grkit_test_shot.known = 0;
    grkit_test_white[0] = grkit_test_white[1] = grkit_test_white[2] = 255;
}

/* Separated light bands repeatedly reuse both fine-row buffers and the
   seven-row rings. Empty spans must not retain a previous band's glow or
   protection, and a previous shot's scratch must not change the output. */
static void test_sparse_glow_rows(void)
{
    static const u32 bands[] = {32, 112, 208, 312, 416};
    static u8 expected[W * H * 2];
    grkit_test_shot.known = 0;
    for (u32 mode = 0; mode < 4; ++mode) {
        PictureSet set = {{plane()}, 1, 0, mode & 1, grkit_test_scratch, 0};
        set.planes[0].half = mode >> 1;
        grkit_test_white[0] = 212; grkit_test_white[1] = 216; grkit_test_white[2] = 206;
        for (u32 streams = 1; streams <= 3; ++streams) {
            set_pair(streams & 1 ? 2 : 0, streams & 2 ? 2 : 0);
            for (u32 phase = 0; phase <= 3; phase += 3) {
                for (u32 reused = 0; reused < 2; ++reused) {
                    if (!reused) memset(grkit_test_scratch, 0xa5, GRKIT_PLANE_SCRATCH);
                    fill(40, 128, 128);
                    for (u32 b = 0; b < sizeof bands / sizeof bands[0]; ++b)
                        for (u32 y = bands[b] + phase; y < bands[b] + phase + 4; ++y)
                            for (u32 x = 304; x < 336; ++x)
                                *grkit_test_bytes(GRKIT_TEST_PLANES + y * W + x) = 255;
                    glow_apply(&set, 1);
                    u32 halo = 0;
                    for (u32 y = 0; y < H; ++y) {
                        u32 near = 0, source = 0;
                        for (u32 b = 0; b < sizeof bands / sizeof bands[0]; ++b) {
                            u32 top = bands[b] + phase;
                            near |= y + 28 >= top && y < top + 32;
                            source |= y >= top && y < top + 4;
                        }
                        for (u32 x = 0; x < W; ++x) {
                            if (!near) {
                                assert(luma(x, y) == 40);
                                assert(chroma(x, set.planes[0].half ? y >> 1 : y, 0) == 128);
                                assert(chroma(x, set.planes[0].half ? y >> 1 : y, 1) == 128);
                            }
                            if (!source && luma(x, y) > 40) ++halo;
                        }
                    }
                    assert(halo > 0);
                    for (u32 i = 0; i < sizeof expected; ++i) {
                        u8 actual = *grkit_test_bytes(GRKIT_TEST_PLANES + i);
                        if (reused) assert(expected[i] == actual);
                        else expected[i] = actual;
                    }
                }
            }
        }
    }
    grkit_test_white[0] = grkit_test_white[1] = grkit_test_white[2] = 255;
}

/* A smooth bright screen in a dark room under a film white below 255.
   Halation starts inside Bloom's glow there, and Bloom must keep a smooth
   glow across that start: the change between neighbors steps at most one
   level more than with Bloom alone, and Halation does not take Bloom's
   light away. */
#define SKY_LEFT 180U
#define SKY_TOP 84U
#define SKY_RIGHT 520U
#define SKY_BOTTOM 312U
static u32 sky_change(PictureSet *set, u32 bloom, u32 halation, u8 *jump)
{
    static u8 before[W * H * 2];
    fill(38, 128, 128);
    for (u32 y = SKY_TOP; y < SKY_BOTTOM; ++y)
        for (u32 x = SKY_LEFT; x < SKY_RIGHT; x += 2) {
            int cb = 0, cr = 0;
            for (u32 k = 0; k < 2; ++k) {
                /* Clouds of two slow waves, so that the level lines of the
                   light are not along the fine grid, darker to the bottom. */
                u32 tx = (x + k) % 128 < 64 ? (x + k) % 128 : 128 - (x + k) % 128, ty = y % 96 < 48 ? y % 96 : 96 - y % 96;
                int c = (int)(2 * tx + 2 * ty + 24), v = 256 - (int)((y - SKY_TOP) * 64 / (SKY_BOTTOM - SKY_TOP));
                int r = (110 + c * 100 / 255) * v >> 8, g = (175 + c * 70 / 255) * v >> 8, b = (222 + c * 33 / 255) * v >> 8;
                *grkit_test_bytes(GRKIT_TEST_PLANES + y * W + x + k) = (u8)((77 * r + 150 * g + 29 * b + 128) >> 8);
                cb += -43 * r - 85 * g + 128 * b;
                cr += 128 * r - 107 * g - 21 * b;
            }
            *grkit_test_bytes(GRKIT_TEST_PLANES + W * H + y * W + x) = (u8)(128 + ((cb / 2 + 128) >> 8));
            *grkit_test_bytes(GRKIT_TEST_PLANES + W * H + y * W + x + 1) = (u8)(128 + ((cr / 2 + 128) >> 8));
        }
    for (u32 i = 0; i < sizeof before; ++i) before[i] = *grkit_test_bytes(GRKIT_TEST_PLANES + i);
    set_pair(bloom, halation);
    glow_apply(set, 1);
    u32 sum = 0;
    *jump = 0;
    for (u32 y = SKY_TOP + 8; y + 9 < SKY_BOTTOM; ++y)
        for (u32 x = SKY_LEFT + 8; x + 9 < SKY_RIGHT; ++x) {
            int d = luma(x, y) - before[y * W + x];
            int right = luma(x + 1, y) - before[y * W + x + 1], down = luma(x, y + 1) - before[(y + 1) * W + x];
            u32 j = (u32)(abs(d - right) > abs(d - down) ? abs(d - right) : abs(d - down));
            if (j > *jump) *jump = (u8)j;
            sum += (u32)d;
        }
    return sum;
}

static void test_combined_film_white(PictureSet *set)
{
    grkit_test_shot.known = 0;
    grkit_test_white[0] = 212; grkit_test_white[1] = 216; grkit_test_white[2] = 206;
    for (u32 strength = 1; strength <= 3; ++strength) {
        u8 bloom_jump, both_jump;
        u32 bloom = sky_change(set, strength, 0, &bloom_jump);
        u32 both = sky_change(set, strength, strength, &both_jump);
        /* A step of 3 between two rows is the slope of Strong over the
           sky at a few pixels. The patches that the test guards against
           gave 8 to 17. */
        assert(bloom > 0 && bloom_jump <= (strength == 3 ? 3 : 2));
        assert(both_jump <= bloom_jump + 1);
        assert(both * 10 >= bloom * 9);
    }
    grkit_test_white[0] = grkit_test_white[1] = grkit_test_white[2] = 255;
}

static void assert_setting_default(const char *key, u32 values, u32 initial)
{
    for (u32 i = 0; i < grkit_module.setting_count; ++i)
        if (!strcmp(grkit_module.settings[i].key, key)) {
            assert(grkit_module.settings[i].values == values);
            assert(grkit_module.settings[i].default_value == initial);
            return;
        }
    assert(!"missing setting");
}

/* Exercise the generated registrations, as the core does after loading saved
   settings. No viewfinder or setting-change event is needed for this gate. */
static void test_filter_gate(PictureSet *set)
{
    static u8 expected[W * H * 2];
    for (u32 i = 0; i < grkit_module.setting_count; ++i)
        setting_set(GRKIT_SETTING(i), grkit_module.settings[i].default_value);
    for (u32 b = 0; b <= 3; ++b)
        for (u32 h = 0; h <= 3; ++h) {
            set_pair(b, h);
            fill(40, 128, 128); square();
            glow_apply(set, 1);
            memcpy(expected, (const void *)grkit_test_bytes(GRKIT_TEST_PLANES), sizeof(expected));
            fill(40, 128, 128); square();
            u32 active = 0, filters = 0;
            for (u32 i = 0; i < grkit_module.registration_count; ++i) {
                const GrkitRegistration *r = &grkit_module.registrations[i];
                if (r->point != GRKIT_POINT_PICTURE_PLANE) continue;
                const GrkitPlaneFilter *f = r->impl;
                ++filters;
                u32 value = setting_get(GRKIT_SETTING(f->setting));
                if (value) {
                    ++active;
                    f->apply(set, value);
                }
            }
            /* The core skips plane access and cache cleaning without an
               active filter. Both enabled must still apply the look once. */
            assert(active == (u32)(b != 0) + (u32)(h != 0));
            assert(filters == 2);
            assert(!memcmp(expected, (const void *)grkit_test_bytes(GRKIT_TEST_PLANES), sizeof(expected)));
        }
}

/* Each effect has one setting: 0 Off, then Weak, Medium, and Strong. Turning
   one effect off leaves the look of the other unchanged. */
static void test_settings(PictureSet *set)
{
    assert(grkit_module.setting_count == 5);
    assert_setting_default("BLOOM", 4, 0);
    assert_setting_default("HALATION", 4, 0);
    assert_setting_default("HALATION_COLOR", 4, 1);
    /* The viewfinder icons show by default. */
    assert_setting_default("BLOOM_ICON", 2, 0);
    assert_setting_default("HALATION_ICON", 2, 0);
    set_pair(0, 0);
    fill(40, 128, 128); square();
    u32 off = plane_sum(&set->planes[0]);
    glow_apply(set, 1);
    assert(plane_sum(&set->planes[0]) == off);
    for (u32 b = 1; b <= 3; ++b)
        for (u32 h = 1; h <= 3; ++h) {
            set_pair(b, h);
            fill(40, 128, 128); square();
            glow_apply(set, 1);
            u32 both = plane_sum(&set->planes[0]);
            set_pair(0, h);
            fill(40, 128, 128); square();
            glow_apply(set, 1);
            u32 halation_only = plane_sum(&set->planes[0]);
            set_pair(b, 0);
            fill(40, 128, 128); square();
            glow_apply(set, 1);
            u32 bloom_only = plane_sum(&set->planes[0]);
            assert(halation_only != off && bloom_only != off);
            assert(halation_only != both && bloom_only != both);
            set_pair(b, h);
            fill(40, 128, 128); square();
            glow_apply(set, 1);
            assert(plane_sum(&set->planes[0]) == both);
        }
}

int main(void)
{
    PictureSet set = {{plane()}, 1, 0, 0, grkit_test_scratch, 0};
    test_filter_gate(&set);
    test_settings(&set);
    set_strength(1);

    test_off(&set);
    test_halation_preserves_background(&set);
    test_halation_red_rim(&set);
    test_halation_colors(&set);
    test_halation_source_response(&set);
    test_halation_film_white();
    test_halation_half_core();
    test_composite_protection();
    test_daylight_transition(&set);
    test_daylight_fade(&set);
    test_strong_lift(&set);
    test_strong_veil(&set);
    test_strong_veil_edge(&set);
    test_local_daylight_gate(&set);
    test_halo_block_edges(&set);
    test_bloom_broad_source(&set);
    test_bloom_rim_color(&set);
    test_bloom_rim_reach(&set);
    test_halation_no_rim(&set);
    test_dark_scene(&set);
    test_strength_range(&set);
    test_camera_planes();
    test_wide_plane_alignment();
    test_combined_strengths(&set);
    test_combined_scratch();
    test_empty_halation();
    test_sparse_glow_rows();
    test_combined_film_white(&set);
    set_strength(1);

    /* A dark picture has too little light: no change. */
    fill(40, 128, 128);
    apply_look(&set, 1);
    for (u32 y = 0; y < H; y += 13)
        for (u32 x = 0; x < W; x += 7) assert(luma(x, y) == 40 && chroma(x, y, 1) == 128);

    /* Bloom: a compact glow next to the source (6 pixels out), in the
       color of the white source, and none in the far corner. */
    fill(40, 128, 128);
    square();
    apply_look(&set, 1);
    assert(luma(298, 240) > 40 && luma(0, 0) == 40);
    assert(chroma(298, 240, 0) >= 125 && chroma(298, 240, 0) <= 128);
    assert(chroma(298, 240, 1) >= 128 && chroma(298, 240, 1) <= 132);
    /* Broad-source suppression may reduce its level, but the halo still
       rises toward the source rather than wrapping or forming a dark ring. */
    assert(luma(300, 240) > luma(298, 240));

    /* Halation: a thin warm rim along the edge (Cr up, Cb down). */
    fill(40, 128, 128);
    square();
    apply_look(&set, 2);
    assert(chroma(301, 240, 1) > 128 && chroma(301, 240, 0) < 128 && luma(301, 240) > 40);
    assert(luma(0, 0) == 40 && chroma(0, 0, 1) == 128);

    /* Scene EV cannot by itself rule out a bright source: in a dark
       picture at 11.5 EV, a light keeps its halo, and a light on a bright
       surface gets none. test_local_daylight_gate checks the local gate
       alone. */
    {
        u32 night[3] = {luma(301, 240), chroma(301, 240, 0), chroma(301, 240, 1)};
        grkit_test_shot.known = GRKIT_SHOT_EXPOSURE;
        grkit_test_shot.scene_ev = 5 * 256;
        fill(40, 128, 128);
        square();
        apply_look(&set, 2);
        assert(luma(301, 240) == night[0] && chroma(301, 240, 0) == night[1] && chroma(301, 240, 1) == night[2]);
        grkit_test_shot.scene_ev = 11 * 256 + 128;
        fill(40, 128, 128);
        square();
        apply_look(&set, 2);
        assert(luma(301, 240) > 40 && chroma(301, 240, 1) > 128);
        fill(220, 128, 128);
        square();
        apply_look(&set, 2);
        assert(luma(301, 240) <= 221 && chroma(301, 240, 1) <= 129);
        grkit_test_shot.known = 0;
        fill(40, 128, 128);
        square();
        apply_look(&set, 2);
    }

    /* Halation of a blue source (R 45, G 45, B 255) is much weaker than
       that of a white one: the red layer gets little of its light. */
    {
        u32 white = luma(301, 240) - 40U;
        fill(40, 128, 128);
        colored_square(70, 240, 110);
        apply_look(&set, 2);
        assert((luma(301, 240) - 40U) * 4 < white);
    }

    /* An effect whose white is 227: a square at that white glows as a
       clipped one does, and much more than at the white 255. */
    {
        u32 clipped, at_white, below;
        fill(40, 128, 128);
        square();
        apply_look(&set, 2);
        clipped = luma(301, 240) - 40U;
        fill(40, 128, 128);
        colored_square(227, 128, 128);
        apply_look(&set, 2);
        below = luma(301, 240) - 40U;
        grkit_test_white[0] = grkit_test_white[1] = grkit_test_white[2] = 227;
        fill(40, 128, 128);
        colored_square(227, 128, 128);
        apply_look(&set, 2);
        at_white = luma(301, 240) - 40U;
        grkit_test_white[0] = grkit_test_white[1] = grkit_test_white[2] = 255;
        assert(at_white * 4 >= clipped * 3);
        assert(below * 2 < at_white);
    }

    /* Halation of a large area at the effect white (a sign, a white coat):
       its inside stays as it was, while a small source of the same level
       keeps its rim. */
    {
        u32 rim, x, y;
        grkit_test_white[0] = grkit_test_white[1] = grkit_test_white[2] = 227;
        fill(40, 128, 128);
        for (y = 80; y < 400; ++y)
            for (x = 160; x < 480; ++x) *grkit_test_bytes(GRKIT_TEST_PLANES + y * W + x) = 200;
        apply_look(&set, 2);
        assert(luma(320, 240) <= 202 && chroma(320, 240, 1) <= 130);
        fill(40, 128, 128);
        colored_square(227, 128, 128);
        apply_look(&set, 2);
        rim = luma(301, 240) - 40U;
        grkit_test_white[0] = grkit_test_white[1] = grkit_test_white[2] = 255;
        assert(rim > 10);
    }

    /* A large area at the effect white keeps a rim outside it, and the rim
       barely depends on where its edge falls in the coarse cell grid. */
    {
        u32 s, x, y, low = 255, high = 0;
        grkit_test_white[0] = grkit_test_white[1] = grkit_test_white[2] = 227;
        for (s = 0; s < 8; ++s) {
            u32 rim;
            fill(40, 128, 128);
            for (y = 80; y < 400; ++y)
                for (x = 160 + s; x < 480; ++x) *grkit_test_bytes(GRKIT_TEST_PLANES + y * W + x) = 227;
            apply_look(&set, 2);
            rim = 0;
            for (x = 152 + s; x < 160 + s; ++x) rim += luma(x, 240) - 40U;
            assert(rim > 0 && luma(320, 240) <= 231);
            low = rim < low ? rim : low;
            high = rim > high ? rim : high;
        }
        grkit_test_white[0] = grkit_test_white[1] = grkit_test_white[2] = 255;
        /* One output code per pixel covers quantization of this eight-pixel rim. */
        assert(high * 2 < low * 3 + 16);
    }

    /* Bloom responds to both a lamp with a soft surround and a bright
       surface with a hard edge, within its compact halo (10 pixels out).
       Neutral sources keep neutral halos. */
    {
        u32 lamp, hard, x, y;
        fill(30, 128, 128);
        for (y = 216; y < 264; ++y)
            for (x = 296; x < 344; ++x) *grkit_test_bytes(GRKIT_TEST_PLANES + y * W + x) = 200;
        square();
        apply_look(&set, 1);
        lamp = luma(294, 240) - 30U;
        assert_neutral();
        fill(30, 128, 128);
        square();
        apply_look(&set, 1);
        hard = luma(294, 240) - 30U;
        assert_neutral();
        assert(hard > 0 && lamp > 0);
    }

    /* Both looks have strictly increasing measured halo energy. Source
       pixels are excluded, so clipping the source cannot satisfy this. */
    {
        for (u32 look = 1; look <= 2; ++look) {
            u32 energy[3];
            for (u32 s = 0; s < 3; ++s) {
                set_strength(s);
                energy[s] = square_energy(&set, look, 255);
            }
            assert(energy[0] > 0 && energy[0] < energy[1] && energy[1] < energy[2]);
        }
        set_strength(1);
    }

    /* Collect the failures of the small sources before failing, so one
       missing source does not hide the diagnostics of the others. */
    {
        u32 failures = test_small_sources(&set);
        u32 energy = square_energy(&set, 1, 224);
        if (!energy) {
            fprintf(stderr, "Bloom: non-clipped level-224 surface has no halo\n");
            ++failures;
        }
        assert_neutral();
        failures += test_highlight_continuity(&set);
        failures += test_separate_planes();
        assert(failures == 0);
    }

    /* A monochrome picture keeps neutral chroma. */
    fill(40, 128, 128);
    square();
    set.mono = 1;
    apply_look(&set, 2);
    assert(chroma(301, 240, 1) == 128 && luma(301, 240) > 40);
    return 0;
}
