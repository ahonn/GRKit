#ifndef GRKIT_TEST_H
#define GRKIT_TEST_H
/* The host harness of `grkit module test`. The command builds each source of
   the module, its generated descriptor, its test.c, and grkit_test.c for the
   host computer, with this header first in each file. A test calls the paint
   functions of the module on a surface of the harness, and checks the pixels.
   The harness stands in for the core: settings, overlay_label, and
   grkit_self. */

/* The surface address of the harness, and its stride in pixels. */
#define GRKIT_TEST_PIXELS 0x60000000U
#define GRKIT_TEST_STRIDE 736U
/* Each pixel address of a surface maps to the pixel buffer of the harness. */
#define GRKIT_PIXELS(address) grkit_test_pixels(address)
/* The plane addresses of the harness: GRKIT_TEST_PLANE_BYTES bytes from
   GRKIT_TEST_PLANES. */
#define GRKIT_TEST_PLANES 0x70000000U
#define GRKIT_TEST_PLANE_BYTES (8U << 20)
#define GRKIT_BYTES(address) grkit_test_bytes(address)
#include "grkit.h"

volatile u32 *grkit_test_pixels(u32 address);
volatile u8 *grkit_test_bytes(u32 address);
/* The time that local_time gives. */
extern GrkitDateTime grkit_test_time;
/* The white that picture_white gives, 255 each at start. */
extern u8 grkit_test_white[3];
/* The data that picture_shot gives: none known at start. Its `size` is
   ignored; picture_shot fills the caller's size. */
extern GrkitShot grkit_test_shot;
/* The scratch of a PictureSet, GRKIT_PLANE_SCRATCH bytes. */
extern u8 grkit_test_scratch[];
/* The pixel at (x, y), and the value of each pixel after grkit_test_clear. */
u32 grkit_test_pixel(u32 x, u32 y);
#define GRKIT_TEST_CLEAR 0x12345678U
void grkit_test_clear(void);
/* A 720 x 480 surface over the photo area `box` {left, top, right, bottom}. */
Surface grkit_test_surface(const u16 *box);
/* The values of the settings: handle n is entry n, and setting k of the
   module is handle GRKIT_TEST_SETTING_BASE + k, so its entry is
   grkit_test_settings[SETTING_<KEY>]. setting_get and setting_set of the
   harness use them. */
#define GRKIT_TEST_SETTINGS 64U
#define GRKIT_TEST_SETTING_BASE 16U
extern u32 grkit_test_settings[GRKIT_TEST_SETTINGS];
/* The calls of overlay_label, and the label, left, and top of the last one. */
extern u32 grkit_test_labels;
extern u32 grkit_test_label[3];
/* The timer and UI services record their calls. grkit_test_fire_timers
   calls each started timer once, as the App task would. The UI calls return
   grkit_test_ui_shows (1 at start). */
extern u32 grkit_test_timers;
void grkit_test_fire_timers(void);
extern u32 grkit_test_ui_shows, grkit_test_ui_calls;
extern GrkitText grkit_test_ui_text;
/* The callbacks of the last ui_message or ui_confirm, which a test calls. */
extern void (*grkit_test_ui_message_done)(u32 arg);
extern void (*grkit_test_ui_confirm_done)(u32 executed, u32 arg);
extern u32 grkit_test_ui_arg, grkit_test_ui_percent;
/* The callbacks of the last ui_slider, which a test calls. */
extern void (*grkit_test_slider_changed)(u32 value, u32 arg);
extern void (*grkit_test_slider_done)(u32 committed, u32 value, u32 arg);
/* The screen and the callback of the last ui_open_native. A test calls the
   callback as the return from the screen would. */
extern u32 grkit_test_native;
extern void (*grkit_test_native_back)(u32 arg);
#endif
