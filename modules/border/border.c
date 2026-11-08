/* Border: a white or black edge over each recorded picture.

   A print border, painted over the edge of the JPEG: the picture keeps its
   size, and the edge covers 3% (thin) or 6% (wide) of the picture width on
   each side. Every plane of the shot (the picture, the screennail, the
   thumbnail) gets the edge in its own scale, so the thumbnail matches. The
   edge follows the visible area of each plane, which a Framing crop sets.
   DNG files and the live view stay as they are. */
#include "grkit.h"
#include "grkit_module.h"

#define BYTE(p, o) (GRKIT_BYTES((p) + (o))[0])

#define STYLE_WHITE_THIN 1U
#define STYLE_WHITE_WIDE 2U
#define STYLE_BLACK_THIN 3U
#define STYLE_BLACK_WIDE 4U

/* The thickness of the edge of a plane: 3% or 6% of its visible width, at
   least one pixel. */
u32 border_thickness(const Plane *p, u32 style)
{
    u32 width = p->right - p->left;
    u32 percent = (style == STYLE_WHITE_WIDE || style == STYLE_BLACK_WIDE) ? 6U : 3U;
    u32 t = divide(width * percent, 100U);
    return t ? t : 1U;
}

/* Paint luma `l` and neutral chroma on the pixels x0 to x1 (exclusive) of
   row y. */
static void paint_row(const Plane *p, u32 mono, u32 y, u32 x0, u32 x1, u8 l)
{
    u32 luma = p->y + y * p->pitch, x;
    for (x = x0; x < x1; ++x) BYTE(luma, x) = l;
    if (mono || (p->half && (y & 1))) return;
    {
        u32 c = chroma_row(p, y), cx0 = x0 & ~1U;
        for (x = cx0; x < x1; x += 2) { BYTE(c, x) = 128; BYTE(c, x + 1) = 128; }
    }
}

static void border_plane(const Plane *p, u32 mono, u32 style)
{
    u32 t = border_thickness(p, style), y;
    u8 l = (style == STYLE_BLACK_THIN || style == STYLE_BLACK_WIDE) ? 0 : 255;
    u32 left = p->left, right = p->right, top = p->top, bottom = p->bottom;
    if (right <= left || bottom <= top) return;
    if (2 * t >= right - left || 2 * t >= bottom - top) {
        /* A tiny plane: the edge covers all of it. */
        for (y = top; y < bottom; ++y) paint_row(p, mono, y, left, right, l);
        return;
    }
    for (y = top; y < bottom; ++y) {
        if (y < top + t || y >= bottom - t) {
            paint_row(p, mono, y, left, right, l);
        } else {
            paint_row(p, mono, y, left, left + t, l);
            paint_row(p, mono, y, right - t, right, l);
        }
    }
}

/* The filter: the edge on every plane of the shot. */
void border_apply(PictureSet *set, u32 style)
{
    u32 i;
    if (style == 0 || style > STYLE_BLACK_WIDE) return;
    for (i = 0; i < set->count; ++i) border_plane(&set->planes[i], set->mono, style);
}
