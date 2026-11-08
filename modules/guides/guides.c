/* Guides: lines over the still live view, inside the framing area, or the
   native photo area without one (Surface.box). The core calls the two
   layers with the style, and not at Off; guides_paint also not while DISP
   hides the native information (module.toml). Each paints only for its
   value of DISP. */
#include "grkit.h"
#include "grkit_module.h"

/* The native grid guide (GridView) writes palette entry 16, black at 50%
   alpha, in 2-pixel lines: COLOR 0 and OPACITY 0. The other colors are white
   and the yellow of the native highlight (red ff, green e1, blue 00). A
   pixel holds red in its low byte and alpha in its high byte (grkit.h);
   with 50% alpha, straight and premultiplied alpha give the same black
   only, so the white and the yellow use straight alpha as the native
   palette does. */
static const u32 guide_rgb[3] = {0x000000U, 0xffffffU, 0x00e1ffU};
#define GUIDE_ARM 20U
/* A 2-pixel vertical line on columns x - 1 and x, or a horizontal line on rows
   y - 1 and y, as the native grid draws on each side of a division. Each
   line steps its row address and keeps the color in a register. */
static void guide_column(u32 data, u32 stride, u32 color, u32 x, u32 top, u32 bottom)
{
    u32 at = data + top * stride * 4;
    for (u32 y = top; y < bottom; ++y, at += stride * 4) {
        volatile u32 *row = GRKIT_PIXELS(at);
        row[x - 1] = color;
        row[x] = color;
    }
}
static void guide_row(u32 data, u32 stride, u32 color, u32 y, u32 left, u32 right)
{
    volatile u32 *above = GRKIT_PIXELS(data + (y - 1) * stride * 4), *at = GRKIT_PIXELS(data + y * stride * 4);
    for (u32 x = left; x < right; ++x) {
        above[x] = color;
        at[x] = color;
    }
}
/* A position at `fraction`/65536 of the span, rounded. Modules have no
   division helper. */
static u32 guide_at(u32 start, u32 end, u32 fraction) { return start + (((end - start) * fraction + 0x8000) >> 16); }
/* The native thirds: the divisions are at multiples of floor(span / 3).
   21846/65536 gives the exact floor for each span up to 720. */
static void guide_thirds(u32 data, u32 stride, u32 color, const u16 *box)
{
    u32 column = ((box[2] - box[0]) * 21846U) >> 16, row = ((box[3] - box[1]) * 21846U) >> 16;
    for (u32 i = 1; i < 3; ++i) {
        guide_column(data, stride, color, box[0] + i * column, box[1], box[3]);
        guide_row(data, stride, color, box[1] + i * row, box[0], box[2]);
    }
}
/* The golden section: one vertical and one horizontal line at each side. */
static void guide_golden(u32 data, u32 stride, u32 color, const u16 *box)
{
    for (u32 i = 0; i < 2; ++i) {
        u32 f = i ? 65536 - 25033 : 25033;
        guide_column(data, stride, color, guide_at(box[0], box[2], f), box[1], box[3]);
        guide_row(data, stride, color, guide_at(box[1], box[3], f), box[0], box[2]);
    }
}
/* Corner-to-corner Bresenham lines, 2 pixels high in each column. Every photo
   area is wider than tall, so each column has one pair of each diagonal. The
   upper pixel goes from the top row to the row above the bottom one, rounded
   half up. */
static void guide_diagonals(u32 data, u32 stride, u32 color, const u16 *box)
{
    int run = box[2] - box[0] - 1, rise = box[3] - box[1] - 2, error = 0;
    u32 y = box[1], mapped = y;
    volatile u32 *top = GRKIT_PIXELS(data + y * stride * 4), *below = GRKIT_PIXELS(data + (y + 1) * stride * 4);
    for (u32 x = 0; x <= (u32)run; ++x) {
        if (mapped != y) {
            mapped = y;
            top = GRKIT_PIXELS(data + y * stride * 4);
            below = GRKIT_PIXELS(data + (y + 1) * stride * 4);
        }
        top[box[0] + x] = color;
        below[box[0] + x] = color;
        top[box[2] - 1 - x] = color;
        below[box[2] - 1 - x] = color;
        error += rise;
        if (2 * error >= run) { ++y; error -= run; }
    }
}
/* A 2-pixel cross at the center of the photo area. Each arm reaches
   GUIDE_ARM pixels past the 2 center pixels. */
static void guide_cross(u32 data, u32 stride, u32 color, const u16 *box)
{
    u32 x = guide_at(box[0], box[2], 32768), y = guide_at(box[1], box[3], 32768);
    guide_row(data, stride, color, y, x - 1 - GUIDE_ARM, x + 1 + GUIDE_ARM);
    guide_column(data, stride, color, x, y - 1 - GUIDE_ARM, y + 1 + GUIDE_ARM);
}
/* Dynamic symmetry: the two diagonals and their four reciprocals. A
   reciprocal starts at a corner, is perpendicular to the diagonal that does
   not pass through that corner, and ends on the opposite side: it steps
   height/width pixels across for each row. Each row has a 2-pixel pair of each
   reciprocal. */
static void guide_symmetry(u32 data, u32 stride, u32 color, const u16 *box)
{
    u32 width = box[2] - box[0], height = box[3] - box[1], x = 0, error = 0;
    guide_diagonals(data, stride, color, box);
    for (u32 r = 0; r < height; ++r) {
        volatile u32 *top = GRKIT_PIXELS(data + (box[1] + r) * stride * 4),
                     *bottom = GRKIT_PIXELS(data + (box[3] - 1 - r) * stride * 4);
        /* In a 1:1 area the reciprocals are the diagonals and reach the
           last column; the pair stays inside. */
        u32 across = x < width - 1 ? x : width - 2, left = box[0] + across, right = box[2] - 1 - across;
        top[left] = top[left + 1] = top[right] = top[right - 1] = color;
        bottom[left] = bottom[left + 1] = bottom[right] = bottom[right - 1] = color;
        error += height;
        if (2 * error >= width) { ++x; error -= width; }
    }
}
/* sin(i x 90 / 32 degrees) x 65536 for i = 0 to 32, rounded; 65535 at 90
   degrees. The cosine is the table read backwards. */
static const u16 guide_quarter[33] = {
    0, 3216, 6424, 9616, 12785, 15924, 19024, 22078, 25080, 28020, 30893,
    33692, 36410, 39040, 41576, 44011, 46341, 48559, 50660, 52639, 54491, 56212,
    57798, 59244, 60547, 61705, 62714, 63572, 64277, 64827, 65220, 65457, 65535,
};
/* A 2 x 2 dot on the pixels around the edge point (x, y), mirrored in the
   box: bit 0 of `flip` mirrors x, bit 1 mirrors y. Dots outside the box are
   left out. */
static void guide_dot(u32 data, u32 stride, u32 color, const u16 *box, u32 flip, int x, int y)
{
    if (flip & 1) x = box[0] + box[2] - x;
    if (flip & 2) y = box[1] + box[3] - y;
    for (int j = y - 1; j <= y; ++j) {
        if (j < box[1] || j >= box[3]) continue;
        volatile u32 *row = GRKIT_PIXELS(data + (u32)j * stride * 4);
        for (int i = x - 1; i <= x; ++i)
            if (i >= box[0] && i < box[2]) row[i] = color;
    }
}
/* A Bresenham line of dots from (x0, y0) to (x1, y1). */
static void guide_segment(u32 data, u32 stride, u32 color, const u16 *box, u32 flip, int x0, int y0, int x1, int y1)
{
    int dx = x1 > x0 ? x1 - x0 : x0 - x1, dy = y1 > y0 ? y0 - y1 : y1 - y0;
    int sx = x0 < x1 ? 1 : -1, sy = y0 < y1 ? 1 : -1, error = dx + dy;
    for (;;) {
        guide_dot(data, stride, color, box, flip, x0, y0);
        if (x0 == x1 && y0 == y1) return;
        int twice = 2 * error;
        if (twice >= dy) { error += dy; x0 += sx; }
        if (twice <= dx) { error += dx; y0 += sy; }
    }
}
/* A division line of the golden spiral: a 2-pixel line at edge `at`, from
   `from` to `to` on the other axis, mirrored in the box as guide_dot. */
static void guide_division(u32 data, u32 stride, u32 color, const u16 *box, u32 flip, u32 vertical, u32 at, u32 from, u32 to)
{
    u32 mirror_at = vertical ? flip & 1 : flip & 2, mirror_span = vertical ? flip & 2 : flip & 1;
    u32 low = vertical ? box[0] + box[2] : box[1] + box[3], high = vertical ? box[1] + box[3] : box[0] + box[2];
    if (mirror_at) at = low - at;
    if (mirror_span) {
        u32 start = high - to;
        to = high - from;
        from = start;
    }
    if (vertical) guide_column(data, stride, color, at, from, to);
    else guide_row(data, stride, color, at, from, to);
}
/* The golden spiral, stretched to the box, with its division lines. Each step
   cuts 0.618 (40503/65536) of the rest of the box, in turn on the left, the
   top, the right, and the bottom, draws the line of the cut, and draws a
   quarter ellipse in the part that it cuts, as polyline segments of the sine
   table. The steps stop before a part narrower than GUIDE_SPIRAL_END, where
   2-pixel lines would merge: 9 quarters in 3:2. Without a flip, the
   spiral starts at the bottom-left corner and ends at about 72% of the
   width and the height. */
#define GUIDE_SPIRAL_END 8U
static void guide_spiral(u32 data, u32 stride, u32 color, const u16 *box, u32 flip)
{
    u32 l = box[0], t = box[1], r = box[2], b = box[3];
    for (u32 k = 0; r - l >= GUIDE_SPIRAL_END && b - t >= GUIDE_SPIRAL_END; ++k) {
        /* The center of the ellipse, its radii, the signs of x and y from the
           center, and whether x takes the sine. */
        u32 cx, cy, rx, ry, sine_x = k & 1;
        int sign_x = (k & 3) == 0 || (k & 3) == 3 ? -1 : 1, sign_y = (k & 3) < 2 ? -1 : 1;
        switch (k & 3) {
        case 0: cx = guide_at(l, r, 40503); cy = b; rx = cx - l; ry = b - t; break;
        case 1: cx = l; cy = guide_at(t, b, 40503); rx = r - l; ry = cy - t; break;
        case 2: cx = l + r - guide_at(l, r, 40503); cy = t; rx = r - cx; ry = b - t; break;
        default: cx = r; cy = t + b - guide_at(t, b, 40503); rx = r - l; ry = b - cy; break;
        }
        if (sine_x) guide_division(data, stride, color, box, flip, 0, cy, l, r);
        else guide_division(data, stride, color, box, flip, 1, cx, t, b);
        if ((k & 3) == 0) l = cx;
        else if ((k & 3) == 1) t = cy;
        else if ((k & 3) == 2) r = cx;
        else b = cy;
        /* Fewer segments for a smaller arc. */
        u32 span = rx > ry ? rx : ry, step = span >= 128 ? 1 : span >= 32 ? 2 : span >= 8 ? 4 : 8;
        int x0 = 0, y0 = 0;
        for (u32 i = 0; i <= 32; i += step) {
            u32 cosine = guide_quarter[32 - i], sine = guide_quarter[i];
            int x = (int)cx + sign_x * (int)((rx * (sine_x ? sine : cosine) + 0x8000) >> 16);
            int y = (int)cy + sign_y * (int)((ry * (sine_x ? cosine : sine) + 0x8000) >> 16);
            if (i) guide_segment(data, stride, color, box, flip, x0, y0, x, y);
            x0 = x;
            y0 = y;
        }
    }
}
/* Styles: 1 thirds, 2 center, 3 golden ratio, 4 diagonals, 5 thirds and
   center, 6 golden spiral, 7 dynamic symmetry. The golden section is
   0.381966, 25033/65536. SPIRAL places the end of the spiral, clockwise:
   0 top left, 1 top right, 2 bottom right, the spiral without a flip, and
   3 bottom left. */
static const u8 guide_spiral_flip[4] = {3, 2, 0, 1};
static void guides_draw(const Surface *s, u32 style)
{
    const u16 *box = s->box;
    u32 data = s->data, stride = s->stride, color = setting_get(SETTING_COLOR), corner = setting_get(SETTING_SPIRAL);
    color = guide_rgb[color < 3 ? color : 0] | (setting_get(SETTING_OPACITY) ? 0xff000000U : 0x80000000U);
    if (style == 1 || style == 5) guide_thirds(data, stride, color, box);
    if (style == 3) guide_golden(data, stride, color, box);
    if (style == 4) guide_diagonals(data, stride, color, box);
    if (style == 2 || style == 5) guide_cross(data, stride, color, box);
    if (style == 6) guide_spiral(data, stride, color, box, guide_spiral_flip[corner < 4 ? corner : 2]);
    if (style == 7) guide_symmetry(data, stride, color, box);
}
/* The layer that follows DISP, for DISP 0, and the one that always shows,
   for DISP 1. */
void guides_paint(const Surface *s, u32 style)
{
    if (!setting_get(SETTING_DISP)) guides_draw(s, style);
}
void guides_paint_always(const Surface *s, u32 style)
{
    if (setting_get(SETTING_DISP)) guides_draw(s, style);
}
