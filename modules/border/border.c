/* Border: a white or black border around each recorded picture.

   The picture keeps its size. The whole picture scales down in place and
   the border fills the space around it, so nothing of the picture is lost.
   The border is 3% (thin) or 6% (wide) of the picture width at the top and
   the bottom; the scale keeps the shape of the picture, so the sides are
   wider. Every plane of the shot (the picture, the screennail, the
   thumbnail) gets the border in its own scale, so the thumbnail matches.
   The border follows the visible area of each plane, which a Framing crop
   sets. DNG files and the live view stay as they are. */
#include "grkit.h"
#include "grkit_module.h"

#define BYTE(p, o) (GRKIT_BYTES((p) + (o))[0])
/* A row of a plane as plain bytes, so the compiler can unroll and
   vectorize its loops. The host test maps the address to its buffer. */
#ifdef GRKIT_TEST_H
#define PIXELS(address) ((u8 *)(void *)grkit_test_bytes(address))
#else
#define PIXELS(address) ((u8 *)(address))
#endif

#define STYLE_WHITE_THIN 1U
#define STYLE_WHITE_WIDE 2U
#define STYLE_BLACK_THIN 3U
#define STYLE_BLACK_WIDE 4U

/* The widest plane that the scratch holds a row and the column table of. */
#define ROW_MAX 8192U

/* The border of a plane: the picture scales into `width` x `height` at
   (`left`, `top`), inside the visible area. */
typedef struct { u32 left, top, width, height; } Inner;

/* The thickness of the border at the top and the bottom of a plane: 3% or
   6% of its visible width, at least one pixel. */
u32 border_thickness(const Plane *p, u32 style)
{
    u32 width = p->right - p->left;
    u32 percent = (style == STYLE_WHITE_WIDE || style == STYLE_BLACK_WIDE) ? 6U : 3U;
    u32 t = divide(width * percent, 100U);
    return t ? t : 1U;
}

/* The scaled picture of a plane. The scale keeps the shape: the side that
   the border limits more gets the thickness, the other side more. The
   width and the left are even, for the chroma pairs; the height and the top
   too where the chroma rows are half. Returns 0 when nothing of the picture
   would remain. */
u32 border_inner(const Plane *p, u32 style, Inner *inner)
{
    u32 width = p->right - p->left, height = p->bottom - p->top;
    u32 t = border_thickness(p, style), w, h;
    if (2 * t + 2 >= width || 2 * t + 2 >= height) return 0;
    w = width - 2 * t;
    h = height - 2 * t;
    /* w / width against h / height: the smaller ratio limits the scale. */
    if (h * width <= w * height) w = divide(h * width + height / 2, height);
    else h = divide(w * height + width / 2, width);
    w &= ~1U;
    if (p->half) h &= ~1U;
    if (w < 2 || h < 2) return 0;
    inner->width = w;
    inner->height = h;
    inner->left = (p->left + ((width - w) >> 1)) & ~1U;
    inner->top = p->top + ((height - h) >> 1);
    if (p->half) inner->top &= ~1U;
    return 1;
}

/* One grid of samples of a plane: the luma, or the chroma pairs. A sample
   has `bytes` bytes (1 luma, 2 for Cb and Cr). Row r starts at base + r *
   pitch. */
typedef struct { u32 base, pitch, bytes; } Grid;

/* The source position of each output position, in 1/256: the center of
   output i of `out` maps to that of the source of `in` positions. The
   last source position is `in` - 1. */
static u32 position(u32 i, u32 step, u32 in)
{
    u32 at = (2 * i + 1) * step;
    at = at > 0x10000U ? (at - 0x10000U) >> 9 : 0;
    return at > (in - 1) << 8 ? (in - 1) << 8 : at;
}

/* Scale the `in_w` x `in_h` samples at (`in_x`, `in_y`) of a grid into the
   `out_w` x `out_h` samples at (`out_x`, `out_y`) inside them, and fill the
   rest of the input area with `fill` (bytes of one sample). Bilinear. The
   output overwrites its input, so the rows run outward from the row where
   an output row meets its source: an output row below its source rows
   upward from there, and one above them downward. Each row is a blend of
   two source rows into `line`, then a resample of `line` into the row. */
/* Set the samples [from, to) of `row` to `fill`. */
static void fill_samples(u8 *row, u32 from, u32 to, u32 bytes, const u8 *fill)
{
    if (bytes == 1) {
        for (u32 i = from; i < to; ++i) row[i] = fill[0];
        return;
    }
    for (u32 i = from; i < to; ++i) { row[2 * i] = fill[0]; row[2 * i + 1] = fill[1]; }
}

static void scale_grid(const Grid *g, u32 in_x, u32 in_y, u32 in_w, u32 in_h,
                       u32 out_x, u32 out_y, u32 out_w, u32 out_h, const u8 *fill,
                       u8 *line, u32 *columns)
{
    u32 bytes = g->bytes, xstep = divide(in_w << 16, out_w), ystep = divide(in_h << 16, out_h);
    u32 i, j, meet, pass, span = in_w * bytes;
    for (i = 0; i < out_w; ++i) columns[i] = position(i, xstep, in_w);
    /* The first output row at or above its source. */
    for (meet = 0; meet < out_h; ++meet)
        if (out_y + meet <= in_y + (position(meet, ystep, in_h) >> 8)) break;
    for (pass = 0; pass < 2; ++pass) {
        u32 count = pass ? meet : out_h - meet;
        for (u32 n = 0; n < count; ++n) {
            /* Pass 0: meet upward to the last row. Pass 1: meet - 1 down
               to the first. */
            u32 k = pass ? meet - 1 - n : meet + n, v = position(k, ystep, in_h);
            u32 r0 = in_y + (v >> 8), f = v & 0xff, g0 = 256 - f;
            const u8 *a = PIXELS(g->base + r0 * g->pitch) + in_x * bytes;
            u8 *out = PIXELS(g->base + (out_y + k) * g->pitch);
            if (f) {
                const u8 *b = a + g->pitch;
                for (i = 0; i < span; ++i) line[i] = (u8)((a[i] * g0 + b[i] * f + 128) >> 8);
            } else {
                for (i = 0; i < span; ++i) line[i] = a[i];
            }
            fill_samples(out, in_x, out_x, bytes, fill);
            if (bytes == 1) {
                u8 *o = out + out_x;
                for (i = 0; i < out_w; ++i) {
                    u32 u = columns[i], x = u >> 8, w = u & 0xff;
                    o[i] = w ? (u8)((line[x] * (256 - w) + line[x + 1] * w + 128) >> 8) : line[x];
                }
            } else {
                u8 *o = out + 2 * out_x;
                for (i = 0; i < out_w; ++i) {
                    u32 u = columns[i], x = 2 * (u >> 8), w = u & 0xff;
                    if (w) {
                        o[2 * i] = (u8)((line[x] * (256 - w) + line[x + 2] * w + 128) >> 8);
                        o[2 * i + 1] = (u8)((line[x + 1] * (256 - w) + line[x + 3] * w + 128) >> 8);
                    } else {
                        o[2 * i] = line[x];
                        o[2 * i + 1] = line[x + 1];
                    }
                }
            }
            fill_samples(out, out_x + out_w, in_x + in_w, bytes, fill);
        }
    }
    /* The border rows above and below the picture, after the picture has
       read its source rows there. */
    for (j = in_y; j < in_y + in_h; ++j) {
        if (j >= out_y && j < out_y + out_h) continue;
        fill_samples(PIXELS(g->base + j * g->pitch), in_x, in_x + in_w, bytes, fill);
    }
}

/* Fill the visible area of a plane with the border: a plane too small for
   a picture inside. */
static void fill_plane(const Plane *p, u32 mono, u8 luma)
{
    for (u32 y = p->top; y < p->bottom; ++y) {
        u32 row = p->y + y * p->pitch;
        for (u32 x = p->left; x < p->right; ++x) BYTE(row, x) = luma;
        if (mono || (p->half && (y & 1))) continue;
        row = chroma_row(p, y);
        for (u32 x = p->left & ~1U; x < p->right; ++x) BYTE(row, x) = 128;
    }
}

static void border_plane(const Plane *p, u32 mono, u32 style, u8 *scratch)
{
    static const u8 neutral[2] = {128, 128};
    u8 luma = (style == STYLE_BLACK_THIN || style == STYLE_BLACK_WIDE) ? 0 : 255;
    u32 *columns = (u32 *)scratch;
    u8 *line = scratch + ROW_MAX * 4;
    Inner inner;
    if (p->right <= p->left || p->bottom <= p->top || p->right - p->left > ROW_MAX) return;
    if (!border_inner(p, style, &inner)) {
        fill_plane(p, mono, luma);
        return;
    }
    {
        Grid g = {p->y, p->pitch, 1};
        scale_grid(&g, p->left, p->top, p->right - p->left, p->bottom - p->top,
                   inner.left, inner.top, inner.width, inner.height, &luma, line, columns);
    }
    if (mono) return;
    {
        /* The chroma pairs: half the columns, and half the rows where `half`
           is set. */
        u32 shift = p->half ? 1U : 0U, left = p->left >> 1, right = (p->right + 1) >> 1;
        u32 top = p->top >> shift, bottom = (p->bottom + shift) >> shift;
        Grid g = {p->c, p->pitch, 2};
        scale_grid(&g, left, top, right - left, bottom - top, inner.left >> 1, inner.top >> shift,
                   inner.width >> 1, inner.height >> shift, neutral, line, columns);
    }
}

/* The filter: the border on every plane of the shot. */
void border_apply(PictureSet *set, u32 style)
{
    u32 i;
    if (style == 0 || style > STYLE_BLACK_WIDE) return;
    for (i = 0; i < set->count; ++i) border_plane(&set->planes[i], set->mono, style, set->scratch);
}
