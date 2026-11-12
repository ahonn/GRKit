/* The geometry of the CORE.BORDER canvas (runtime/canvas.h). */
#include <assert.h>
#include <stdint.h>

typedef uint32_t u32;
typedef uint8_t u8;
static u32 core_divide(u32 n, u32 d) { return n / d; }
#include "canvas.h"

static void limited(u32 style, u32 w, u32 h, u32 turns, u32 limit, u32 l, u32 t, u32 r, u32 b)
{
    u32 e[4];
    assert(canvas_margins(style, w, h, turns, limit, e) == (l < r ? l : r));
    assert(e[CANVAS_LEFT] == l && e[CANVAS_TOP] == t && e[CANVAS_RIGHT] == r && e[CANVAS_BOTTOM] == b);
}

static void margins(u32 style, u32 w, u32 h, u32 turns, u32 l, u32 t, u32 r, u32 b)
{
    limited(style, w, h, turns, CANVAS_NO_LIMIT, l, t, r, b);
}

static void inner(u32 cw, u32 ch, const u32 m[4], u32 pw, u32 ph, u32 l, u32 t, u32 r, u32 b)
{
    u32 a[4];
    assert(canvas_inner_area(cw, ch, m, pw, ph, a));
    assert(a[CANVAS_LEFT] == l && a[CANVAS_TOP] == t && a[CANVAS_RIGHT] == r && a[CANVAS_BOTTOM] == b);
}

/* Each sample of a preview inside the picture area reads only the picture:
   canvas_preview takes canvas pixel (x * cw / w) and the next one, and the
   same for rows. */
static void samples_inside(u32 cw, u32 ch, const u32 m[4], u32 pw, u32 ph)
{
    u32 a[4], fit = canvas_fit(cw, ch, pw, ph), w = fit & 0xffffU, h = fit >> 16, ox, oy, sx, sy;
    assert(canvas_inner_area(cw, ch, m, pw, ph, a));
    if (w + 2U >= pw && h + 2U >= ph) { w = pw; h = ph; }
    ox = ((pw - w) >> 1) & ~1U; oy = (ph - h) >> 1;
    sx = (cw << 16) / w; sy = (ch << 16) / h;
    for (u32 x = a[CANVAS_LEFT]; x < a[CANVAS_RIGHT]; ++x) {
        u32 c = ((x - ox) * sx) >> 16;
        assert(c >= m[CANVAS_LEFT] && c + 1U < cw - m[CANVAS_RIGHT]);
    }
    for (u32 y = a[CANVAS_TOP]; y < a[CANVAS_BOTTOM]; ++y) {
        u32 r = ((y - oy) * sy) >> 16;
        assert(r >= m[CANVAS_TOP] && r + 1U < ch - m[CANVAS_BOTTOM]);
    }
    assert(!(a[CANVAS_LEFT] & 1U) && !(a[CANVAS_RIGHT] & 1U));
}

int main(void)
{
    /* GR III 3:2: Thin 3% and Wide 6% of the short side, on each side, to
       the nearest multiple of 16. */
    margins(CANVAS_THIN, 6000, 4000, 0, 128, 128, 128, 128);
    margins(CANVAS_WIDE, 6000, 4000, 0, 240, 240, 240, 240);
    /* Instant: 4%, and three times that at the bottom of the upright
       picture. A turn moves the wide side: one turn clockwise shows the
       stored right side at the bottom. */
    margins(CANVAS_INSTANT, 6000, 4000, 0, 160, 160, 160, 480);
    margins(CANVAS_INSTANT, 6000, 4000, 1, 160, 160, 480, 160);
    margins(CANVAS_INSTANT, 6000, 4000, 2, 160, 480, 160, 160);
    margins(CANVAS_INSTANT, 6000, 4000, 3, 480, 160, 160, 160);
    /* A crop takes its own short side: XPan 6000 x 2216 gives 132. */
    margins(CANVAS_WIDE, 6000, 2216, 0, 128, 128, 128, 128);
    margins(CANVAS_THIN, 4000, 4000, 0, 128, 128, 128, 128);
    /* A portrait crop has the same short side. */
    margins(CANVAS_THIN, 4000, 6000, 0, 128, 128, 128, 128);
    /* A limit, a multiple of 16 down, keeps the canvas in the bank; Instant
       keeps its bottom three times the other sides. */
    limited(CANVAS_WIDE, 6192, 4128, 0, 200, 192, 192, 192, 192);
    limited(CANVAS_INSTANT, 6192, 4128, 0, 112, 112, 112, 112, 336);
    limited(CANVAS_THIN, 6192, 4128, 0, 200, 128, 128, 128, 128);
    /* Off, and a style out of range, have no border. */
    margins(CANVAS_OFF, 6000, 4000, 0, 0, 0, 0, 0);
    assert(canvas_style(4) == CANVAS_OFF && canvas_style(0x103) == CANVAS_INSTANT);
    /* The color is in bits 8 to 15: 0 white, 1 black. */
    assert(canvas_luma(CANVAS_WIDE) == 255 && canvas_luma(CANVAS_WIDE | 1U << 8) == 0);
    /* A preview keeps the whole canvas in its box, with even sides. */
    assert(canvas_fit(6240, 4240, 720, 480) == (706U | 480U << 16));
    assert(canvas_fit(6480, 4480, 160, 120) == (160U | 110U << 16));
    assert(canvas_fit(6272, 2488, 720, 480) == (720U | 284U << 16));
    assert(canvas_fit(4480, 4480, 720, 480) == (480U | 480U << 16));
    assert(canvas_fit(0, 4480, 720, 480) == 0);
    /* The picture inside the border, in each plane. The main picture is
       the whole canvas: GR IV Wide 6576 x 4512 with 192 on each side. */
    {
        const u32 m[4] = {192, 192, 192, 192};
        inner(6576, 4512, m, 6576, 4512, 192, 192, 6384, 4320);
        /* A GR IV screennail keeps its 720 x 480 box: the canvas is 698 x
           480 between bars of 10. The margins round away from the picture,
           so that no preview sample of the border is inside. */
        assert(canvas_fit(6576, 4512, 720, 480) == (698U | 480U << 16));
        inner(6576, 4512, m, 720, 480, 32, 21, 686, 459);
        /* The thumbnail box holds 160 x 108, with bars of 6 above and below. */
        inner(6576, 4512, m, 160, 120, 6, 11, 154, 109);
        samples_inside(6576, 4512, m, 720, 480);
        samples_inside(6576, 4512, m, 160, 120);
    }
    /* A GR III preview fitted to the canvas is all canvas. */
    {
        const u32 m[4] = {240, 240, 240, 240};
        inner(6480, 4480, m, 694, 480, 26, 26, 668, 454);
        samples_inside(6480, 4480, m, 694, 480);
        samples_inside(6480, 4480, m, 160, 110);
    }
    /* Instant portrait and XPan on GR IV, and XPan on GR III. */
    {
        const u32 instant[4] = {144, 144, 432, 144}, xpan4[4] = {144, 144, 144, 144}, xpan3[4] = {128, 128, 128, 128};
        samples_inside(6768, 4416, instant, 720, 480);
        samples_inside(6768, 4416, instant, 160, 120);
        samples_inside(6480, 2576, xpan4, 720, 480);
        samples_inside(6256, 2472, xpan3, 720, 284);
    }
    {
        const u32 m[4] = {0, 0, 0, 0};
        u32 a[4];
        assert(canvas_inner_area(0, 4512, m, 720, 480, a) == 0);
    }
    return 0;
}
