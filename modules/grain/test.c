/* The host test of Grain (grkit module test): the Film-like grain on flat
   pictures, and its tables against their formulas. The System style leaves
   the planes to the native grain stage; tests/runtime/picture_contract.c
   checks that the core turns that stage off in the Film-like style. */
#include <assert.h>
#include <math.h>
#include "grkit_module.h"

void grain_apply(PictureSet *set, u32 level);
void grain_draw(PictureSet *set, u32 level, u32 size);

#define WIDTH 1024U
#define HEIGHT 768U
/* A screennail of a quarter of the width. */
#define SMALL_WIDTH 256U
#define SMALL_HEIGHT 192U

/* A flat 4:2:2 plane at `at`: luma rows, then interleaved CbCr rows of the
   same pitch. Returns the address after it. */
static u32 flat(Plane *p, u32 at, u32 width, u32 height, u8 y, u8 cb, u8 cr)
{
    Plane q = {at, at + width * height, width, height, width, 0, 0, 0, width, height};
    *p = q;
    for (u32 i = 0; i < width * height; ++i) {
        GRKIT_BYTES(p->y + i)[0] = y;
        GRKIT_BYTES(p->c + i)[0] = i & 1U ? cr : cb;
    }
    return at + 2U * width * height;
}

static PictureSet set_of_color(u8 y, u8 cb, u8 cr, u32 planes, u32 mono)
{
    PictureSet s;
    u32 at = GRKIT_TEST_PLANES;
    s.count = planes;
    s.reserved = 0;
    s.mono = mono;
    s.scratch = grkit_test_scratch;
    s.turns = 0;
    s.reserved_shot = 0;
    at = flat(&s.planes[0], at, WIDTH, HEIGHT, y, cb, cr);
    if (planes > 1U) flat(&s.planes[1], at, SMALL_WIDTH, SMALL_HEIGHT, y, cb, cr);
    return s;
}

typedef struct {
    double mean, sd, corr;
} Stats;
/* Statistics of the luma (channel 0), Cb (1), or Cr (2) of a plane. */
static Stats stats(const Plane *p, u32 channel)
{
    Stats r = {0, 0, 0};
    u32 width = channel ? p->width / 2U : p->width, n = 0;
    double sum = 0, squares = 0, products = 0;
    for (u32 y = 0; y < p->height; ++y)
        for (u32 x = 0; x < width; ++x) {
            u32 address = channel ? p->c + y * p->pitch + 2U * x + channel - 1U : p->y + y * p->pitch + x;
            double v = GRKIT_BYTES(address)[0];
            sum += v;
            squares += v * v;
            ++n;
        }
    r.mean = sum / n;
    r.sd = squares / n - r.mean * r.mean;
    for (u32 y = 0; y < p->height; ++y)
        for (u32 x = 0; x + 1U < width; ++x) {
            u32 address = channel ? p->c + y * p->pitch + 2U * x + channel - 1U : p->y + y * p->pitch + x;
            u32 next = channel ? address + 2U : address + 1U;
            products += (GRKIT_BYTES(address)[0] - r.mean) * (GRKIT_BYTES(next)[0] - r.mean);
        }
    r.corr = r.sd > 0 ? products / (p->height * (width - 1U)) / r.sd : 0;
    r.sd = r.sd > 0 ? __builtin_sqrt(r.sd) : 0;
    return r;
}

static PictureSet set_of(u8 y, u32 planes, u32 mono) { return set_of_color(y, 128, 128, planes, mono); }

/* The model of the tone of grain.c (see the comment there). */
u32 grain_tone_of(u32 v);
extern const u16 grain_kernel_table[19][4];
extern const u8 grain_blend_weight[8];
extern const u16 grain_blend_norm[8];
static double tone_formula(double v)
{
    const double dm = 2.3, k = 4 * 2.6 / dm, dmax = 2.2;
    if (v < 0.5) v = 0.5;
    double d = -2.2 * log10(v / 255), q = d / dm;
    q = q < 1e-6 ? 1e-6 : q > 1 - 1e-6 ? 1 - 1e-6 : q;
    double x = log(q / (1 - q)) / k, slope = k * d * (1 - d / dm), p = 0.5 - x / dmax;
    if (slope < 0) slope = 0;
    p = p < 1e-6 ? 1e-6 : p > 1 - 1e-6 ? 1 - 1e-6 : p;
    return v * slope * sqrt(p * (1 - p));
}

int main(void)
{
    assert(SETTING_STYLE == GRKIT_SETTING(0) && SETTING_ICON == GRKIT_SETTING(1));

    /* The tables follow their formulas. */
    double peak = 0;
    for (u32 v = 0; v < 256U; ++v)
        if (tone_formula(v) > peak) peak = tone_formula(v);
    /* The tone of grain.c is within 11 of 255 of the model. */
    for (u32 v = 0; v < 256U; ++v) assert(fabs(grain_tone_of(v) - 255 * tone_formula(v) / peak) <= 11.0);
    assert(grain_tone_of(0) == 0 && grain_tone_of(255) == 0 && grain_tone_of(128) > 240);
    for (u32 i = 0; i < 19U; ++i) {
        const u16 *k = grain_kernel_table[i];
        double sigma = 0.40 + 0.05 * i, w[4], total = 0;
        assert(k[0] + 2U * (k[1] + k[2] + k[3]) == 4096U);
        for (u32 d = 0; d < 4U; ++d) {
            w[d] = exp(-(double)(d * d) / (2 * sigma * sigma));
            total += d ? 2 * w[d] : w[d];
        }
        for (u32 d = 1; d < 4U; ++d) assert(fabs(k[d] - 4096 * w[d] / total) <= 0.5 + 1e-9);
    }

    for (u32 d = 0; d < 8U; ++d) {
        double w = grain_blend_weight[d] / 256.0;
        assert(grain_blend_weight[d] == 128U + 16U * d);
        assert(fabs(grain_blend_norm[d] - 4096 / sqrt(w * w + (1 - w) * (1 - w))) <= 0.5 + 1e-9);
    }

    /* System: the filter leaves the planes to the native grain stage. */
    PictureSet s = set_of(128, 1, 0);
    grkit_test_settings[SETTING_STYLE] = 0;
    grain_apply(&s, 2);
    Stats st = stats(&s.planes[0], 0);
    assert(st.sd == 0 && st.mean == 128);

    /* Medium on middle gray: grain in Y and in Cb and Cr, the mean kept. */
    s = set_of(128, 2, 0);
    grain_draw(&s, 2, 2);
    st = stats(&s.planes[0], 0);
    Stats cb = stats(&s.planes[0], 1), cr = stats(&s.planes[0], 2);
    assert(fabs(st.mean - 128) < 0.6 && fabs(cb.mean - 128) < 0.6 && fabs(cr.mean - 128) < 0.6);
    assert(st.sd > 4.5 && st.sd < 7.5);
    /* Independent layers: Cb and Cr get grain about as strong as Y. */
    assert(cb.sd > 2 && cb.sd < st.sd * 1.5 && cr.sd > 2 && cr.sd < st.sd * 1.5);
    /* The blend at the edges of the cells keeps the deviation of the grain:
       samples near an edge vary as those inside a cell. */
    {
        double edge = 0, inside = 0;
        u32 edges = 0, insides = 0;
        for (u32 y = 0; y < HEIGHT; ++y)
            for (u32 x = 0; x < WIDTH; ++x) {
                double v = GRKIT_BYTES(s.planes[0].y + y * WIDTH + x)[0] - st.mean;
                u32 tx = x % 256U, ty = y % 128U;
                if ((tx < 4U || tx >= 252U) && (ty < 4U || ty >= 124U)) edge += v * v, ++edges;
                else if (tx >= 64U && tx < 192U && ty >= 32U && ty < 96U) inside += v * v, ++insides;
            }
        double ratio = sqrt(edge / edges) / sqrt(inside / insides);
        assert(ratio > 0.92 && ratio < 1.08);
    }
    /* No lines: each row and each column of Y keeps about the mean of the
       picture. */
    double rows = 0, columns = 0;
    for (u32 y = 0; y < HEIGHT; ++y) {
        double row = 0;
        for (u32 x = 0; x < WIDTH; ++x) row += GRKIT_BYTES(s.planes[0].y + y * WIDTH + x)[0];
        row = row / WIDTH - st.mean;
        rows += row * row;
    }
    for (u32 x = 0; x < WIDTH; ++x) {
        double column = 0;
        for (u32 y = 0; y < HEIGHT; ++y) column += GRKIT_BYTES(s.planes[0].y + y * WIDTH + x)[0];
        column = column / HEIGHT - st.mean;
        columns += column * column;
    }
    /* As random grain would: about sd / sqrt(width / 4). */
    assert(sqrt(rows / HEIGHT) < st.sd * 0.06 && sqrt(columns / WIDTH) < st.sd * 0.06);
    /* The grain does not repeat: the picture does not correlate with itself
       at the size of a cell or of the texture. */
    static const u32 periods[3] = {128, 256, 512};
    for (u32 i = 0; i < 3U; ++i)
        for (u32 vertical = 0; vertical < 2U; ++vertical) {
            u32 dx = vertical ? 0 : periods[i], dy = vertical ? periods[i] : 0;
            double products = 0;
            for (u32 y = 0; y + dy < HEIGHT; ++y)
                for (u32 x = 0; x + dx < WIDTH; ++x)
                    products += (GRKIT_BYTES(s.planes[0].y + y * WIDTH + x)[0] - st.mean) *
                                (GRKIT_BYTES(s.planes[0].y + (y + dy) * WIDTH + x + dx)[0] - st.mean);
            assert(fabs(products / ((HEIGHT - dy) * (WIDTH - dx)) / (st.sd * st.sd)) < 0.05);
        }
    /* The screennail, a quarter of the width, keeps a quarter of it. */
    Stats small = stats(&s.planes[1], 0);
    assert(small.sd > st.sd * 0.15 && small.sd < st.sd * 0.4);

    /* The grain follows the print: none at paper white or in black, less in
       the shadows and highlights than in the middle. */
    static const u8 tones[4] = {255, 12, 60, 220};
    for (u32 i = 0; i < 4U; ++i) {
        s = set_of(tones[i], 1, 0);
        grain_draw(&s, 2, 2);
        Stats t = stats(&s.planes[0], 0);
        if (i < 2U) assert(t.sd == 0);
        else assert(t.sd > 0.5 && t.sd < st.sd * 0.85);
    }

    /* Each strength is stronger than the one before. */
    double last = 0;
    for (u32 level = 1; level <= 4U; ++level) {
        s = set_of(128, 1, 0);
        grain_draw(&s, level, 2);
        Stats l = stats(&s.planes[0], 0);
        assert(l.sd > last * 1.2);
        last = l.sd;
    }

    /* Each size is wider than the one before: neighbours correlate more. */
    last = 0;
    for (u32 size = 1; size <= 3U; ++size) {
        s = set_of(128, 1, 0);
        grain_draw(&s, 2, size);
        Stats z = stats(&s.planes[0], 0);
        assert(z.corr > last + 0.05 && z.corr < 0.97);
        last = z.corr;
    }

    /* A mono picture: grain in Y, and its CbCr kept. */
    s = set_of(128, 1, 1);
    grain_draw(&s, 3, 2);
    st = stats(&s.planes[0], 0);
    cb = stats(&s.planes[0], 1);
    assert(st.sd > 3 && cb.sd == 0 && cb.mean == 128);

    /* A saturated color keeps grain: its tone comes from Y, not from its
       R, G, and B. */
    s = set_of_color(100, 90, 200, 1, 0);
    grain_draw(&s, 2, 2);
    st = stats(&s.planes[0], 0);
    assert(st.sd > 1);

    /* The same picture gets the same grain, another picture other grain. */
    s = set_of(128, 1, 0);
    grain_draw(&s, 2, 2);
    u8 first[512];
    for (u32 i = 0; i < 512U; ++i) first[i] = GRKIT_BYTES(s.planes[0].y + 300U * WIDTH + i)[0];
    s = set_of(128, 1, 0);
    grain_draw(&s, 2, 2);
    for (u32 i = 0; i < 512U; ++i) assert(GRKIT_BYTES(s.planes[0].y + 300U * WIDTH + i)[0] == first[i]);
    s = set_of(129, 1, 0);
    grain_draw(&s, 2, 2);
    u32 same = 0;
    for (u32 i = 0; i < 512U; ++i) same += GRKIT_BYTES(s.planes[0].y + 300U * WIDTH + i)[0] == first[i] + 1U;
    assert(same < 256U);

    /* Only the visible area changes, also in 4:2:0. */
    for (u32 half = 0; half < 2U; ++half) {
        s = set_of(128, 1, 0);
        s.planes[0].half = half;
        s.planes[0].left = 101;
        s.planes[0].top = 51;
        s.planes[0].right = WIDTH - 99U;
        s.planes[0].bottom = HEIGHT - 49U;
        grain_draw(&s, 4, 3);
        for (u32 y = 0; y < HEIGHT; ++y)
            for (u32 x = 0; x < WIDTH; ++x)
                if (x < 101U || x >= WIDTH - 99U || y < 51U || y >= HEIGHT - 49U)
                    assert(GRKIT_BYTES(s.planes[0].y + y * WIDTH + x)[0] == 128);
        u32 rows = half ? HEIGHT / 2U : HEIGHT, top = half ? 25U : 51U, bottom = half ? (HEIGHT - 49U + 1U) / 2U : HEIGHT - 49U;
        for (u32 y = 0; y < rows; ++y)
            for (u32 x = 0; x < WIDTH / 2U; ++x)
                if (x < 50U || x >= (WIDTH - 99U + 1U) / 2U || y < top || y >= bottom) {
                    assert(GRKIT_BYTES(s.planes[0].c + y * WIDTH + 2U * x)[0] == 128);
                    assert(GRKIT_BYTES(s.planes[0].c + y * WIDTH + 2U * x + 1U)[0] == 128);
                }
    }

    /* No strength, no size, or no plane: nothing changes. */
    s = set_of(128, 1, 0);
    grain_draw(&s, 0, 2);
    grain_draw(&s, 2, 0);
    s.count = 0;
    grain_draw(&s, 2, 2);
    st = stats(&s.planes[0], 0);
    assert(st.sd == 0);
    return 0;
}
