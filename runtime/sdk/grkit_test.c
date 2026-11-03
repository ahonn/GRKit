/* The core of `grkit module test` (grkit_test.h). */
#include <assert.h>
#include <stdlib.h>
#include <string.h>

static u32 pixels[GRKIT_TEST_STRIDE * 480];
static u8 planes[GRKIT_TEST_PLANE_BYTES];
u8 grkit_test_scratch[GRKIT_PLANE_SCRATCH];
GrkitDateTime grkit_test_time = {2026, 10, 25, 9, 5, 0, 0};
u8 grkit_test_white[3] = {255, 255, 255};
GrkitShot grkit_test_shot;
u32 grkit_test_settings[GRKIT_TEST_SETTINGS];
u32 grkit_test_labels;
u32 grkit_test_label[3];
/* The module settings start at handle GRKIT_TEST_SETTING_BASE, as they
   start at a handle above 0 on the camera, so a test finds a SETTING_<KEY>
   that the code wraps in GRKIT_SETTING again. No setting is used. */
const GrkitContext grkit_self = {GRKIT_TEST_SETTING_BASE, GRKIT_TEST_SETTINGS - GRKIT_TEST_SETTING_BASE, 0};

volatile u32 *grkit_test_pixels(u32 address)
{
    u32 offset = address - GRKIT_TEST_PIXELS;
    /* Every write stays inside the surface and on a pixel. */
    assert(address >= GRKIT_TEST_PIXELS && !(offset & 3) && offset / 4 < GRKIT_TEST_STRIDE * 480);
    return pixels + offset / 4;
}
u32 grkit_test_pixel(u32 x, u32 y) { return pixels[y * GRKIT_TEST_STRIDE + x]; }
volatile u8 *grkit_test_bytes(u32 address)
{
    /* Every access stays inside the plane memory. */
    assert(address >= GRKIT_TEST_PLANES && address - GRKIT_TEST_PLANES < GRKIT_TEST_PLANE_BYTES);
    return planes + (address - GRKIT_TEST_PLANES);
}
void local_time(GrkitDateTime *out) { *out = grkit_test_time; }
u32 divide(u32 n, u32 d) { assert(d); return n / d; }
void picture_white(const PictureSet *set, u8 white[3])
{
    (void)set;
    white[0] = grkit_test_white[0];
    white[1] = grkit_test_white[1];
    white[2] = grkit_test_white[2];
}
void picture_shot(const PictureSet *set, GrkitShot *shot)
{
    u32 size = shot->size < sizeof(GrkitShot) ? shot->size : sizeof(GrkitShot);
    GrkitShot copy = grkit_test_shot;
    (void)set;
    /* As the core: a size below 8 gets nothing. */
    if (size < 8) return;
    copy.size = size;
    memcpy(shot, &copy, size);
}
void grkit_test_clear(void)
{
    for (u32 i = 0; i < GRKIT_TEST_STRIDE * 480; ++i) pixels[i] = GRKIT_TEST_CLEAR;
}
Surface grkit_test_surface(const u16 *box)
{
    Surface s = {GRKIT_TEST_PIXELS, GRKIT_TEST_STRIDE, 0, box};
    return s;
}
u32 setting_get(u32 handle) { return handle < GRKIT_TEST_SETTINGS ? grkit_test_settings[handle] : 0; }
void setting_set(u32 handle, u32 value)
{
    if (handle < GRKIT_TEST_SETTINGS) grkit_test_settings[handle] = value;
}
void overlay_label(const Surface *s, u32 label, u32 left, u32 top)
{
    (void)s;
    grkit_test_label[0] = label; grkit_test_label[1] = left; grkit_test_label[2] = top;
    ++grkit_test_labels;
}

/* ---- timer and UI ------------------------------------------------------- */
static struct { void (*fire)(u32); u32 arg; } test_timers[GRKIT_TIMERS];
u32 grkit_test_timers;
u32 timer_start(u32 ms, void (*fire)(u32 arg), u32 arg)
{
    (void)ms;
    for (u32 i = 0; i < GRKIT_TIMERS; ++i)
        if (!test_timers[i].fire) {
            test_timers[i].fire = fire;
            test_timers[i].arg = arg;
            ++grkit_test_timers;
            return i + 1;
        }
    return 0;
}
void timer_cancel(u32 timer)
{
    if (timer - 1 < GRKIT_TIMERS && test_timers[timer - 1].fire) {
        test_timers[timer - 1].fire = 0;
        --grkit_test_timers;
    }
}
void grkit_test_fire_timers(void)
{
    for (u32 i = 0; i < GRKIT_TIMERS; ++i) {
        void (*fire)(u32) = test_timers[i].fire;
        if (!fire) continue;
        test_timers[i].fire = 0;
        --grkit_test_timers;
        fire(test_timers[i].arg);
    }
}
u32 grkit_test_ui_shows = 1, grkit_test_ui_calls, grkit_test_ui_arg, grkit_test_ui_percent;
GrkitText grkit_test_ui_text;
void (*grkit_test_ui_message_done)(u32 arg);
void (*grkit_test_ui_confirm_done)(u32 executed, u32 arg);
static u32 test_ui(GrkitText text)
{
    ++grkit_test_ui_calls;
    grkit_test_ui_text = text;
    return grkit_test_ui_shows;
}
u32 ui_toast(GrkitText text, u32 ms) { (void)ms; return test_ui(text); }
u32 ui_message(GrkitText text, void (*done)(u32 arg), u32 arg)
{
    grkit_test_ui_message_done = done;
    grkit_test_ui_arg = arg;
    return test_ui(text);
}
u32 ui_confirm(GrkitText text, void (*done)(u32 executed, u32 arg), u32 arg)
{
    grkit_test_ui_confirm_done = done;
    grkit_test_ui_arg = arg;
    return test_ui(text);
}
u32 ui_progress_open(GrkitText text) { grkit_test_ui_percent = 0; return test_ui(text); }
void ui_progress_set(u32 percent) { grkit_test_ui_percent = percent; }
void ui_progress_close(GrkitText text) { if (text) test_ui(text); }
void (*grkit_test_slider_changed)(u32 value, u32 arg);
void (*grkit_test_slider_done)(u32 committed, u32 value, u32 arg);
u32 ui_slider(GrkitText text, u32 min, u32 max, u32 value, void (*changed)(u32 value, u32 arg),
              void (*done)(u32 committed, u32 value, u32 arg), u32 arg)
{
    if (max <= min || value < min || value > max) return 0;
    grkit_test_slider_changed = changed;
    grkit_test_slider_done = done;
    grkit_test_ui_arg = arg;
    return test_ui(text);
}
u32 grkit_test_native;
void (*grkit_test_native_back)(u32 arg);
u32 ui_open_native(u32 screen, void (*back)(u32 arg), u32 arg)
{
    if (screen != GRKIT_NATIVE_WHITE_BALANCE && screen != GRKIT_NATIVE_LCD) return 0;
    grkit_test_native = screen;
    grkit_test_native_back = back;
    grkit_test_ui_arg = arg;
    ++grkit_test_ui_calls;
    return grkit_test_ui_shows;
}
