#ifndef GRKIT_CORE_H
#define GRKIT_CORE_H
/* The types and services that the components of the core share: the menu
   pages (runtime/modules.h), the overlay surface, and the planes of recorded
   pictures. It holds no firmware address: the components and the target
   headers keep each firmware detail. */
#include "abi.h"

typedef unsigned int u32;
typedef unsigned short u16;
typedef unsigned char u8;
typedef short s16;
/* A service or handler that a component or target may not use. */
#define GRKIT_MAYBE_UNUSED __attribute__((unused))

/* Memory access. Host tests map these to their own buffers. */
#ifndef WORD
#define WORD(p,o) (*(volatile u32 *)((p)+(o)))
#endif
#ifndef HALF
#define HALF(p,o) (*(volatile u16 *)((p)+(o)))
#endif
#ifndef BYTE
#define BYTE(p,o) (*(volatile u8 *)((p)+(o)))
#endif

/* ---- menu.page ------------------------------------------------------------ */

/* Private menu levels (GRKIT_HOST_MENU_LEVEL). */
#define MENU_ROOT 1U
#define MENU_MODULE 2U
/* Icon results of a page: Off, the GRKit tile of the ADJ toolbar, the icon of
   effect n, or a module tile (GRKIT_ICON_MODULE_TILE). */
#define GRKIT_ICON_OFF 0x7fffU
#define GRKIT_ICON_GRKIT 0x7ffeU
#define GRKIT_ICON_EFFECT(n) (0x8000U+(n))
/* The Modules tile. The installer puts it after the Version tile. */
#define GRKIT_ICON_MODULES 0x7ffcU

/* One setting of a native settings page: its row label, its values (at most
   SETTING_VALUES, grkit_core::runtime::NATIVE_SETTING_VALUES), the text of a
   value, and the stored value. */
#define SETTING_VALUES 8U
typedef struct {
    const u16 *(*label)(volatile u32 *);
    u32 (*values)(volatile u32 *);
    const u16 *(*value_text)(volatile u32 *,u32);
    u32 (*get)(volatile u32 *);
    void (*set)(volatile u32 *,u32);
} Setting;
/* The most rows of the settings page of a module: its settings
   (GRM_SETTING_ROWS) and the Enable row before them in MENU. */
#define SETTING_ROWS 6U
/* The most rows of a native settings page. The borrowed submenu builds
   SUBMENU_ROWS row views instead of the native five (runtime/menu.S:
   menu_settings_build) and scrolls past six. The ADJ Order page has a row
   for each listed module (runtime/manage.h). Its table entry has room for
   the controllers of five rows: a later row borrows the value page of
   another row (runtime/menu_settings.h: SETTINGS_ALIAS). */
#define SUBMENU_ROWS GRKIT_SUBMENU_ROWS
_Static_assert(SETTING_ROWS<=SUBMENU_ROWS,"The submenu shows each row of a module");

typedef struct {
    const u16 *(*title)(volatile u32 *);
    /* The root row text. */
    const u16 *(*summary)(volatile u32 *);
    u32 (*rows)(volatile u32 *);
    const u16 *(*label)(volatile u32 *,u32);
    u32 (*icon)(volatile u32 *,u32);
    u32 (*checked)(volatile u32 *,u32);
    /* OK on a row. */
    void (*apply)(volatile u32 *,u32);
    /* The committed row, where the page opens. */
    u32 (*committed)(volatile u32 *);
    /* Optional: the highlighted row changed. */
    void (*highlight)(volatile u32 *);
    /* Optional: whether a row has native parameters. Fn on such a row of the
       page applies it and opens the native parameter page. Fn on the root row
       opens the page for the committed row. */
    u32 (*parameters)(volatile u32 *,u32);
    /* Optional: the settings of a native settings page, up to SETTING_ROWS;
       the first entry without a label ends the list. Where the target has a
       native settings page, OK on the root row opens that page with these
       settings instead of the page of the module. The page title is the
       module title. */
    const Setting *settings;
    /* Optional: Fn on a row of the page. It returns 1 when it took the key. */
    u32 (*fn)(volatile u32 *,u32);
    /* MENU_PAGE_* flags. */
    u32 flags;
} MenuPage;
/* The rows are actions, not values: an ADJ exit, such as a half-press of
   the shutter, applies no row, and the page shows with one row. */
#define MENU_PAGE_ACTIONS 1U

/* Core services for pages. */
/* One string of the menu text table in the menu language (runtime/abi.h). */
GRKIT_MAYBE_UNUSED static const u16 *text(volatile u32 *h,u32 id);
/* The Image Control summary of effect n in the menu language. */
GRKIT_MAYBE_UNUSED static const u16 *effect_summary(volatile u32 *h,u32 n);
/* ---- viewfinder.overlay --------------------------------------------------- */

/* A 32-bit drawing surface of 720 x 480 pixels, red in the low byte and
   alpha in the high byte (runtime/sdk/grkit.h). `area` is the capture
   area that the surface shows: 0 for the full 3:2 area, else the crop.
   `box` is its photo area {left, top, right, bottom}. */
typedef struct {
    u32 data;
    u32 stride;
    u32 area;
    const u16 *box;
} Surface;
/* The module SDK (runtime/sdk/grkit.h) has the same Surface. A host test
   that links module code into the core uses this one. */
#define GRKIT_CORE_SURFACE 1
/* The native live-view display that DISP selects. Full shows the top row
   of settings (effect and others), simple only the bottom row, no
   information neither, and display off turns the live view black. */
#define DISPLAY_FULL 0U
#define DISPLAY_SIMPLE 1U
#define DISPLAY_NO_INFO 2U
#define DISPLAY_OFF 3U
GRKIT_MAYBE_UNUSED static u32 native_display(void);
/* Live-view labels (GRKIT_LABELS): the load cue, then capture area n, then
   the Date Imprint sign (LABEL_DATE in runtime/sdk/grkit.h). */
#define LABEL_GRKIT 0U
/* The top of a label in the second row of the native live-view text. Its
   capitals then share the baseline of the focus distance, such as "2.5m". */
#define LABEL_TOP 45U
/* The left edge of a label whose text lines up with the native icons of the
   top row. The outline lies left of the text. */
#define LABEL_LEFT 7U
/* Draw a label with its top left at (left, top) in the style of the native
   live-view text: white Roboto Condensed Light with a black outline. A label
   that does not fit the surface draws nothing. */
GRKIT_MAYBE_UNUSED static void overlay_label(const Surface *s,u32 label,u32 left,u32 top);
#ifndef OVERLAY_PIXELS
#define OVERLAY_PIXELS(address) ((volatile u32 *)(address))
#endif

/* ---- picture.encode ------------------------------------------------------- */

/* One YCbCr plane of a recorded picture, with its visible area. Luma rows are
   `pitch` bytes apart. Interleaved CbCr rows follow, one for every second luma
   row when `half` is set. */
typedef struct { u32 y,c,width,height,pitch,half,left,top,right,bottom; } Plane;
/* The address of the chroma row of luma row y. */
static inline u32 chroma_row(const Plane *p,u32 y) { return p->c+(p->half ? y>>1 : y)*p->pitch; }
/* The deduplicated planes of one encode. `record` is the processing record of
   the shot, or 0 for native processing, and `mono` is set for a monochrome
   record or a native monochrome Image Control type. `scratch` holds GRKIT_PICTURE_MASKS bytes for this call.
   `turns` is the quarter turns clockwise that show the picture upright. */
typedef struct {
    Plane planes[4];
    u32 count;
    u32 record;
    u32 mono;
    u8 *scratch;
    u32 turns;
} PictureSet;
/* Core services for the planes. */
/* Unsigned division: the freestanding link has no division helper. */
GRKIT_MAYBE_UNUSED static u32 core_divide(u32 n,u32 d);
/* The address of the scratch for direct access. Host tests remap it. */
#ifndef PICTURE_MAP
#define PICTURE_MAP(address) ((u8 *)(address))
#endif
#endif
