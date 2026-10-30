#ifndef GRKIT_SDK_H
#define GRKIT_SDK_H
/* The interface between a GRKit module and the core. A module includes only
   this file. It runs on the RTOS of the camera, in Thumb code, and calls the
   core only through the functions that this file declares. The installer
   links each call at boot (crates/grkit-core/src/grm.rs).

   A module describes itself with one constant descriptor, `grkit_module`
   (GRKIT_MODULE). It has no init function: the installer reads the
   descriptor and registers the module, but it cannot run module code.

   Module code runs on the stacks of firmware tasks: a function may use at
   most 1536 bytes of stack, which `grkit module build` checks. */

typedef unsigned int u32;
typedef unsigned short u16;
typedef unsigned char u8;

/* The ABI of this file. The installer loads a module whose major version
   equals that of the core and whose minor version is not larger. A minor
   version only adds functions and registration types. */
#define GRKIT_ABI_MAJOR 2U
#define GRKIT_ABI_MINOR 0U
#define GRKIT_ABI (GRKIT_ABI_MAJOR << 16 | GRKIT_ABI_MINOR)

/* ---- overlay.layer -------------------------------------------------------- */

/* A 32-bit drawing surface of 720 x 480 pixels over the still live view. A
   pixel holds red in its low byte, then green, blue, and alpha in its high
   byte (bytes R, G, B, A in memory), as the native graphics layer reads it.
   `area` is the capture area that the surface shows: 0 for the full 3:2
   area, else the ratio value of the crop (CORE.ASPECT). `box` is its photo
   area {left, top, right, bottom}. */
#ifndef GRKIT_CORE_SURFACE
typedef struct {
    u32 data;
    u32 stride;
    u32 area;
    const u16 *box;
} Surface;
#endif

/* The pixels at an address of the surface. A host test maps the addresses
   to its own buffer. */
#ifndef GRKIT_PIXELS
#define GRKIT_PIXELS(address) ((volatile u32 *)(address))
#endif
/* The pixel at (x, y). */
#define SURFACE_PIXEL(s,x,y) (GRKIT_PIXELS((s)->data+((y)*(s)->stride+(x))*4)[0])

/* A drawing layer. The core calls `paint` for each draw of the live view
   with the value of `setting` (see GRKIT_USE), and does not call
   it while the value is 0, so a layer that is off costs nothing. With
   GRKIT_NO_SETTING, the value is always 1. While the page of the setting is
   open in ADJ, the core paints the highlighted value on the ADJ canvas.
   Layers paint in ascending `order`. The official layers: the Framing mask
   at 10, Guides at 20, the Framing ratio label at 30. */
#define GRKIT_LAYER_FOLLOW_DISP 1U /* off while DISP hides the native information */
#define GRKIT_LAYER_FULL_DISP 2U   /* on only while DISP shows the full information */
#define GRKIT_LAYER_LIVE_ONLY 4U   /* not on the ADJ canvas */
typedef struct {
    u32 size;
    u32 flags;
    void (*paint)(const Surface *s, u32 value);
    u32 setting;
    u32 order;
} GrkitLayer;

/* Live-view labels of the core (overlay_label): the GRKit cue, the name of
   each capture area (Surface.area), such as "XPan", and the sign that Date
   Imprint is on. */
#define LABEL_GRKIT 0U
#define LABEL_RATIO(area) (area)
#define LABEL_DATE 8U
/* The top of a label in the second row of the native live-view text, and
   the left edge of a label that lines up with the native icons. A capture
   area label is at most 80 pixels wide, so a second label of the row starts
   at LABEL_NEXT_LEFT and never covers it. */
#define LABEL_TOP 45U
#define LABEL_LEFT 7U
#define LABEL_NEXT_LEFT (LABEL_LEFT + 88U)
/* Draw a label of the core with its top left at (left, top), in the style of
   the native live-view text. A label that does not fit draws nothing. */
#ifndef GRKIT_CORE_SURFACE
void overlay_label(const Surface *s, u32 label, u32 left, u32 top);
#endif

/* ---- settings -------------------------------------------------------------

   The core stores the value of each setting, and the resident loader saves
   the values of settings with GRKIT_SETTING_SAVED in the state file. A value
   is 0 to values - 1, and 0 until the user changes it. INSTALL restores
   the saved values before the core starts, and the user can change a value
   at any time, so read a setting each time that you use it, and do not keep
   a copy. A handle names one
   setting: GRKIT_SETTING(n) is setting n of this module, and GRKIT_USED(n)
   is used setting n (GrkitModule.uses).

   The core has public settings of its own, its capabilities. A module uses
   one by its name, and the core applies its value:

     CORE.TONE    the effect of still pictures and the live view: 0 Off,
                  then each installed effect in the order of Image Control.
                  The core installs the effects only for a module that uses
                  it. A page of CORE.TONE gets its rows from the core.
     CORE.GRAIN   5 values: the grain of recorded pictures, 0 Off, 1 weak,
                  2 medium, 3 strong, 4 extra strong.
     CORE.GRAIN_SIZE  4 values: the grain size, 0 Auto (by the ISO of the
                  shot), 1 small, 2 medium, 3 large.
     CORE.RECIPE  no value: the camera recipes, which hold and set the
                  other capabilities that loaded modules use. A page of
                  CORE.RECIPE gets its rows from the core.
     CORE.ASPECT  8 values: the aspect of still pictures, 0 native 3:2, 1
                  XPan 65:24, 2 16:9, 3 4:3, 4 2.35:1, 5 1:1, 6 5:4, 7 7:6. Each
                  camera offers some of them, and a choice page shows only
                  those. A ratio that the camera has natively sets the native
                  aspect. The core crops the others (Surface.area). */

#define GRKIT_SETTING_PUBLIC 1U /* other modules can use it */
#define GRKIT_SETTING_SAVED 2U  /* the value survives a restart */
/* The handle of an optional used setting that no module has. */
#define GRKIT_NO_SETTING 0xffffffffU
/* The setting of a page or a layer: n for setting n of
   this module, or GRKIT_USE(n) for used setting n. The installer does not
   register a page or a layer of an optional used setting that no module
   has. */
#define GRKIT_USE(n) (0x80000000U | (n))

typedef struct {
    u32 size;
    /* 1 to 15 characters A-Z, 0-9, or _. The full name is <module ID>.<key>. */
    const char *key;
    /* 1 to 65536. */
    u32 values;
    u32 flags;
} GrkitSetting;

/* What the installer gives this module (the import grkit_self). */
typedef struct {
    u32 settings;      /* the handle of setting 0 of this module */
    u32 setting_count;
    u32 tiles;         /* reserved */
    u32 uses[];        /* the handle of each used setting */
} GrkitContext;
extern const GrkitContext grkit_self;

/* The value of a setting, or 0 for GRKIT_NO_SETTING. */
u32 setting_get(u32 handle);
/* Set a setting. A value out of range does nothing. */
void setting_set(u32 handle, u32 value);

/* The asm keeps the compiler from folding the field offset into the address
   of the import, which the installer fills as one whole word. */
static inline const GrkitContext *grkit_context(void)
{
    const GrkitContext *c = &grkit_self;
    __asm__("" : "+r"(c));
    return c;
}
#define GRKIT_SETTING(n) (grkit_context()->settings + (n))
#define GRKIT_USED(n) (grkit_context()->uses[n])

/* ---- menu.page ------------------------------------------------------------ */

/* The menu languages, in this order: English, Simplified Chinese,
   Traditional Chinese, Japanese, Korean. */
#define GRKIT_LANGUAGES 5U
/* A 40 x 40 tile in RGBA order. */
#define GRKIT_TILE_BYTES 6400U

/* A row of a native settings page (GrkitPage.settings): a setting of this
   module (an index) or a used setting (GRKIT_USE), its label, and the text
   of each of its values. A setting has at most GRKIT_SETTING_ROW_VALUES
   values. The installer leaves out the row of an optional used setting that
   no module has. */
#define GRKIT_SETTING_ROW_VALUES 8U
typedef struct {
    u32 size;
    u32 setting;
    const u16 *const *label;  /* GRKIT_LANGUAGES */
    const u16 *const *values; /* values x GRKIT_LANGUAGES */
} GrkitSettingRow;

/* A choice page: one row for each value of `setting` (see GRKIT_USE),
   `rows` rows. OK on a row sets the value. The root row shows the summary and
   the tile of the value; row n shows tile n, or the Off icon for row 0 with
   GRKIT_CHOICE_OFF_ROW. Each text is one UTF-16 string for each language.
   The installer rejects the module if `rows` is not the number of values.

   A page of a setting of this module can also have 1 to GRKIT_SETTING_ROWS
   rows of a native settings page, the first the page setting. Fn on the root row or on the page opens
   it when it has 2 rows or more. Each module also has a row in the GRKit
   section of the Customize tab of the native MENU, which opens the native
   settings page with an Enable row before its rows, or Enable and the row
   of the page setting (at most 8 values) for a page without settings rows;
   so MENU shows Enable and at most GRKIT_SETTING_ROWS rows of a page. The row of a page of CORE.TONE
   opens Enable and the Fn adjustments of the committed effect.

   A page of CORE.TONE has GRKIT_CHOICE_CORE_ROWS and only its title: the
   core gives its rows, the effects, with their names, icons, live-view
   preview, and Fn adjustments. The page shows only with effects. */
#define GRKIT_PAGE_CHOICE 1U
#define GRKIT_CHOICE_OFF_ROW 1U
#define GRKIT_CHOICE_CORE_ROWS 2U
/* The settings page of the module in the GRKit section of MENU has an
   Options row: a list that turns each value of the page but Off on or off in
   ADJ, or each effect for a page of CORE.TONE. A value that is off does not
   show in ADJ, and Off of the value of the setting sets the setting to 0. A
   page of a setting with options has GRKIT_CHOICE_OFF_ROW and at most 16
   values. */
#define GRKIT_CHOICE_OPTIONS 4U
#define GRKIT_SETTING_ROWS 5U
typedef struct {
    u32 size;
    u32 kind;
    u32 setting;
    u32 flags;
    const u16 *const *title;     /* GRKIT_LANGUAGES */
    const u16 *const *summaries; /* values x GRKIT_LANGUAGES */
    const u16 *const *labels;    /* values x GRKIT_LANGUAGES */
    u32 rows;
    u32 setting_rows;            /* 0 to GRKIT_SETTING_ROWS */
    const GrkitSettingRow *settings;
} GrkitPage;

/* ---- picture.plane -------------------------------------------------------- */

#ifndef GRKIT_CORE_SURFACE
/* One YCbCr plane of a recorded picture, with its visible area. Luma rows are
   `pitch` bytes apart. Interleaved CbCr rows follow, one for every second
   luma row when `half` is set. */
typedef struct { u32 y, c, width, height, pitch, half, left, top, right, bottom; } Plane;
/* The planes of one encode: the main picture, the screennail, and the
   thumbnail, without duplicates. `mono` is set for a monochrome picture.
   `scratch` holds GRKIT_PLANE_SCRATCH bytes for this call. The planes keep
   the landscape layout of the sensor in each posture of the camera; `turns`
   is the quarter turns clockwise that show the picture upright, as its EXIF
   Orientation does: 0 (Orientation 1), 1 (6), 2 (3), or 3 (8). */
typedef struct {
    Plane planes[4];
    u32 count;
    u32 reserved;
    u32 mono;
    u8 *scratch;
    u32 turns;
} PictureSet;
/* The address of the chroma row of luma row y. */
static inline u32 chroma_row(const Plane *p, u32 y) { return p->c + (p->half ? y >> 1 : y) * p->pitch; }
#endif
#define GRKIT_PLANE_SCRATCH 141376U
/* The bytes at an address of a plane. A host test maps the addresses to its
   own buffers. */
#ifndef GRKIT_BYTES
#define GRKIT_BYTES(address) ((volatile u8 *)(address))
#endif

/* A filter of the planes of each recorded picture. The core calls `apply`
   with the value of `setting` (see GRKIT_USE) for each recorded picture, and
   does not call it while the value is 0. Filters run in ascending `stage`,
   after the color of the effect: */
#define GRKIT_STAGE_FRAME 20U /* frames and borders */
#define GRKIT_STAGE_MARK 30U  /* marks over the picture, such as the date */
typedef struct {
    u32 size;
    u32 stage;
    u32 setting;
    void (*apply)(PictureSet *set, u32 value);
} GrkitPlaneFilter;

/* The local time: u16 year, then u8 month, day, hour, minute, and second. */
typedef struct { u16 year; u8 month, day, hour, minute, second, reserved; } GrkitDateTime;
void local_time(GrkitDateTime *out);
/* n / d, rounded down, for d > 0. The core exports no compiler helper for
   division, so a module divides by a variable only through this function. */
u32 divide(u32 n, u32 d);

/* ---- events --------------------------------------------------------------

   A module subscribes to events with one GrkitSubscriber (GRKIT_POINT_EVENT,
   `[events]` in module.toml). The core calls `handle` in the App task, the
   task of the camera UI, so the handler can call the timer and UI services:

     GRKIT_EVENT_SYSTEM_READY     once after each start, when the shooting
                                  screen first shows GRKit. a and b are 0.
     GRKIT_EVENT_SETTING_CHANGED  after a setting or a capability changes in
                                  the GRKit menu or through setting_set: a is
                                  the handle, b the new value.

   A handler returns at once. A setting_set in a handler does not send a
   second SETTING_CHANGED. */
#define GRKIT_EVENT_SYSTEM_READY 1U
#define GRKIT_EVENT_SETTING_CHANGED 2U
typedef struct {
    u32 size;
    u32 events; /* GRKIT_EVENT_* bits */
    void (*handle)(u32 event, u32 a, u32 b);
} GrkitSubscriber;

/* ---- timer ---------------------------------------------------------------

   timer_start calls `fire(arg)` once, `ms` milliseconds later, in the App
   task. It returns a timer, or 0 when no timer is free (GRKIT_TIMERS for all
   modules). timer_cancel stops a timer that has not fired. Call both only in
   the App task: in an event handler, a timer callback, or a UI callback. */
#define GRKIT_TIMERS 8U
u32 timer_start(u32 ms, void (*fire)(u32 arg), u32 arg);
void timer_cancel(u32 timer);

/* ---- UI -----------------------------------------------------------------

   The core shows the native message box of the camera on the GRKit page of
   ADJ, so a module looks like the native camera. A text is one string for
   each menu language (GRKIT_LANGUAGES), as the texts of a page. The UI
   shows only while the GRKit page of ADJ is open, and one box at a time: a
   call returns 1 when the box shows, else 0. ADJ exit closes the box
   without a callback. Call these in the App task.

     ui_toast           a panel that closes after `ms` milliseconds.
     ui_message         a message. OK or MENU closes it and calls done(arg).
     ui_confirm         Execute and Cancel, the cursor on Cancel. OK calls
                        done(1, arg) for Execute, else done(0, arg). MENU
                        calls done(0, arg).
     ui_progress_open   a message with a progress bar at 0 %. The box takes
                        the keys until ui_progress_close.
     ui_progress_set    the bar, 0 to 100 %.
     ui_progress_close  closes the bar. With a text, it shows the text for
                        1.5 s, else it closes the box. */
typedef const u16 *const *GrkitText;
u32 ui_toast(GrkitText text, u32 ms);
u32 ui_message(GrkitText text, void (*done)(u32 arg), u32 arg);
u32 ui_confirm(GrkitText text, void (*done)(u32 executed, u32 arg), u32 arg);
u32 ui_progress_open(GrkitText text);
void ui_progress_set(u32 percent);
void ui_progress_close(GrkitText text);
/* ui_slider shows a message with the native slider for a value from min to
   max. Up and Down move it by one, Left and Right by a tenth of the range,
   and each move calls changed(value, arg), for example to preview the value.
   OK calls done(1, value, arg), MENU done(0, the value at the start, arg).
   changed and done can be 0. It returns 0 when max <= min, max - min is
   more than 65535, or value is out of the range, and as the other boxes. */
u32 ui_slider(GrkitText text, u32 min, u32 max, u32 value, void (*changed)(u32 value, u32 arg),
              void (*done)(u32 committed, u32 value, u32 arg), u32 arg);

/* ui_open_native opens a native screen of the camera from the GRKit page,
   after the current event. MENU on the screen returns to ADJ, which opens
   the GRKit page that was open, and then the core calls back(arg) (back
   can be 0). It returns 1 when the screen will open, else 0: an unknown
   screen, the GRKit page is not open, a confirm or a message shows, or a
   screen already waits to open. The same rules as for the boxes apply. */
#define GRKIT_NATIVE_WHITE_BALANCE 1U /* the white balance list (still) */
#define GRKIT_NATIVE_LCD 2U           /* the LCD settings */
u32 ui_open_native(u32 screen, void (*back)(u32 arg), u32 arg);

/* ---- descriptor ----------------------------------------------------------- */

#define GRKIT_POINT_OVERLAY_LAYER 1U
#define GRKIT_POINT_MENU_PAGE 2U
#define GRKIT_POINT_PICTURE_PLANE 3U
#define GRKIT_POINT_EVENT 4U

typedef struct {
    u32 point;
    const void *impl;
} GrkitRegistration;

typedef struct {
    u32 size;
    u32 abi;
    const GrkitRegistration *registrations;
    u32 registration_count;
    const GrkitSetting *settings;
    u32 setting_count;
    /* Full names of settings of other modules or of the core, MODULE.KEY.
       A name that starts with '?' is optional. Only public settings. */
    const char *const *uses;
    u32 use_count;
    /* The artwork tiles of the module, GRKIT_TILE_BYTES each. */
    const u8 *tiles;
    u32 tile_count;
} GrkitModule;

/* The descriptor of this module. */
extern const GrkitModule grkit_module;

#define GRKIT_COUNT(array) (sizeof(array) / sizeof((array)[0]))

/* Define the descriptor of the module by hand. A module with a module.toml
   gets it from `grkit module build`. Pass 0 and 0 for no settings, no used
   settings, or no tiles. */
#define GRKIT_MODULE(registrations, settings, setting_count, uses, use_count, tiles, tile_count) \
    __attribute__((used)) const GrkitModule grkit_module = { \
        sizeof(GrkitModule), GRKIT_ABI, (registrations), GRKIT_COUNT(registrations), \
        (settings), (setting_count), (uses), (use_count), (tiles), (tile_count) }

#endif
