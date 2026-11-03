/* Framing: the mask of the selected framing area in the live view. The core
   gives this CORE.ASPECT layer the composition area in Surface.area and
   Surface.box, including in Guides Only mode. Off or an unavailable framing
   area has area 0, so the layer draws nothing. */
#include "grkit.h"
#include "grkit_module.h"

/* MASK: black at 50% alpha (0) or at 100% (1) outside the photo area, or
   frame lines only (2). Each full draw paints the mask, so the loops write
   only the bars: the rows above and below the photo area, and the columns
   at its sides. */
static const u32 mask_colors[2] = {0x80000000U, 0xff000000U};
/* The frame lines: 2 white pixels inside each edge of the photo area. */
#define FRAME_COLOR 0xffffffffU
#define FRAME_WIDTH 2U
/* Fill pixels `from` to `to` of a row with `color`. A module is Thumb code
   without NEON stores, so on the camera a loop of two STM of eight words
   writes sixteen pixels for each test, then one STM of eight words may
   follow. r7 can be the frame pointer of Thumb code, so the words go in r4
   to r6 and r8 to r12. The rest, and a host test, use one store for each
   pixel. */
__attribute__((noinline)) static void mask_span(volatile u32 *row, u32 from, u32 to, u32 color)
{
    u32 *pixel = (u32 *)row + from, *end = (u32 *)row + to;
#if defined(__arm__)
    if (end - pixel >= 8) {
        register u32 a __asm__("r4") = color, b __asm__("r5") = color, c __asm__("r6") = color;
        register u32 d __asm__("r8") = color, e __asm__("r9") = color, f __asm__("r10") = color;
        register u32 g __asm__("r11") = color, k __asm__("r12") = color;
        u32 *last = end - 16;
        __asm__ volatile("cmp %0, %1\n\t"
                         "bhi 2f\n"
                         "1:\n\t"
                         "stmia %0!, {%3, %4, %5, %6, %7, %8, %9, %10}\n\t"
                         "stmia %0!, {%3, %4, %5, %6, %7, %8, %9, %10}\n\t"
                         "cmp %0, %1\n\t"
                         "bls 1b\n"
                         "2:\n\t"
                         "cmp %0, %2\n\t"
                         "it ls\n\t"
                         "stmials %0!, {%3, %4, %5, %6, %7, %8, %9, %10}"
                         : "+r"(pixel)
                         : "r"(last), "r"(end - 8), "r"(a), "r"(b), "r"(c), "r"(d), "r"(e), "r"(f), "r"(g), "r"(k)
                         : "memory", "cc");
    }
#endif
    for (; pixel < end; ++pixel) *pixel = color;
}

static void frame_lines(const Surface *s)
{
    const u16 *box = s->box;
    for (u32 y = box[1]; y < box[3]; ++y) {
        volatile u32 *row = &SURFACE_PIXEL(s, 0, y);
        if (y < box[1] + FRAME_WIDTH || y + FRAME_WIDTH >= box[3]) { mask_span(row, box[0], box[2], FRAME_COLOR); continue; }
        mask_span(row, box[0], box[0] + FRAME_WIDTH, FRAME_COLOR);
        mask_span(row, box[2] - FRAME_WIDTH, box[2], FRAME_COLOR);
    }
}

void framing_mask(const Surface *s, u32 value)
{
    u32 style = setting_get(SETTING_MASK), color;
    (void)value;
    if (!s->area) return;
    if (style == 2) { frame_lines(s); return; }
    color = mask_colors[style == 1];
    for (u32 y = 0; y < 480; ++y) {
        volatile u32 *row = &SURFACE_PIXEL(s, 0, y);
        if (y < s->box[1] || y >= s->box[3]) { mask_span(row, 0, 720, color); continue; }
        /* A crop as wide as the view, such as XPan, has no side bars. */
        if (s->box[0]) mask_span(row, 0, s->box[0], color);
        if (s->box[2] < 720) mask_span(row, s->box[2], 720, color);
    }
}
