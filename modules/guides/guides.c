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
/* Styles: 1 thirds, 2 center, 3 golden ratio, 4 diagonals, 5 thirds and
   center. The golden section is 0.381966, 25033/65536. */
static void guides_draw(const Surface *s, u32 style)
{
    const u16 *box = s->box;
    u32 data = s->data, stride = s->stride, color = setting_get(SETTING_COLOR);
    color = guide_rgb[color < 3 ? color : 0] | (setting_get(SETTING_OPACITY) ? 0xff000000U : 0x80000000U);
    if (style == 1 || style == 5) guide_thirds(data, stride, color, box);
    if (style == 3) guide_golden(data, stride, color, box);
    if (style == 4) guide_diagonals(data, stride, color, box);
    if (style == 2 || style == 5) guide_cross(data, stride, color, box);
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
