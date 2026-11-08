#include <assert.h>
#include <string.h>
/* The build's label table, which the Rust test writes for this harness. */
#include "menu_text.h"
/* The family header sends the aspect request; the harness counts requests. */
#include "firmware.h"
#undef ADJ_MOVIE
static unsigned test_movie;
#define ADJ_MOVIE(controller) ((void)(controller), test_movie)
#undef MENU_FRAMING_REQUEST
static unsigned framing_requests, requested_crop;
static unsigned record_framing_request(void);
#define MENU_FRAMING_REQUEST() record_framing_request()
/* The native aspect that the camera applies, and the native setter calls. */
#undef MENU_NATIVE_ASPECT
#undef MENU_SET_NATIVE_ASPECT
#undef MENU_STOP_LIVE_VIEW
static unsigned native_aspect_value, native_sets, native_requested_crop, live_view_stops;
static void record_native_aspect(unsigned id);
#define MENU_NATIVE_ASPECT() (native_aspect_value)
#define MENU_SET_NATIVE_ASPECT(id) record_native_aspect(id)
#define MENU_STOP_LIVE_VIEW() (++live_view_stops)
/* The native Image Control type plus 1 (runtime/menu.c). */
static unsigned native_ic_value;
#define MENU_NATIVE_IC_TYPE() (native_ic_value)
/* GR IIIx: the lens of 40 mm that the GR III family runtime asks for. */
#undef CROP_LENS_40MM
static unsigned test_lens_40mm;
#define CROP_LENS_40MM() (test_lens_40mm)
/* The camera's crop factor that the crop button sets (runtime/crop.h). */
static unsigned test_crop_bits = 0x3f800000U, test_crop_sets;
#define CROP_NATIVE_BITS() (test_crop_bits)
#define CROP_NATIVE_SET(bits) (test_crop_bits = (bits), ++test_crop_sets)
/* The controller factory gives the TopMenu instance, which the GRKit section
   takes out of it (runtime/menu_section.h). */
#undef FW_CONTROLLER_FACTORY
static unsigned factory_calls, factory_top;
static unsigned controller_factory(void)
{
    ++factory_calls;
    return factory_top;
}
#define FW_CONTROLLER_FACTORY controller_factory
/* Camera addresses of the menu text map to text_arena, relative to the text
   start of the header, so the count of records can change during the test. */
static const unsigned short *text_at(unsigned int address);
#define MENU_TEXT(address) text_at(address)
static unsigned test_language;
#define MENU_LANGUAGE() test_language
/* Module pages in mapped module memory: the page of slot n at TEST_PAGE +
   n * TEST_PAGE_SIZE, then its text lists. Text n of all pages is at
   TEST_PAGE_TEXT + n * 128. */
#define TEST_PAGE 0x5d000000U
#define TEST_PAGE_SIZE 0x400U
#define TEST_PAGE_SLOTS 10U
#define TEST_PAGE_TEXT 0x5d800000U
static unsigned page_memory[TEST_PAGE_SLOTS * TEST_PAGE_SIZE / 4];
static unsigned short page_texts[1024][64];
static unsigned *page_at(unsigned address)
{
    assert(address >= TEST_PAGE && address < TEST_PAGE + sizeof(page_memory) && !(address & 3));
    return &page_memory[(address - TEST_PAGE) / 4];
}
#define PAGE_WORD(p,o) (*page_at((p)+(o)))
#define TEST_SETTINGS 0x5f000000U
/* Three module settings, the Off and Position settings of three listed
   modules (runtime/manage.h), the choices of three listed effects
   (runtime/effects.h), then COLOR, OPACITY, and DISP of Guides for the MENU
   page tests. A list of ten modules has its settings after the three
   module settings instead. */
static unsigned settings_memory[2 + 3 * (3 + 3 * 10)];
static volatile unsigned *mapped_settings(unsigned address)
{
    assert(address == TEST_SETTINGS);
    return settings_memory;
}
#define MODULE_SETTINGS_MAP(address) mapped_settings(address)
/* The module list of the Modules page, then at TEST_EFFECTS the effect list
   of the Effects page. */
#define TEST_MANAGE 0x5e000000U
#define TEST_EFFECTS (TEST_MANAGE + 0x300U)
static unsigned manage_memory[(TEST_EFFECTS - TEST_MANAGE + 12 + 3 * 44) / 4];
/* The module list: the header, then module i at word MANAGE_ENTRY(i). */
#define MANAGE_ENTRY(i) (MANAGE_HEADER / 4 + (i) * 14)
static volatile unsigned *mapped_manage(unsigned address)
{
    assert(address >= TEST_MANAGE && address < TEST_MANAGE + sizeof(manage_memory) && !(address & 3));
    return &manage_memory[(address - TEST_MANAGE) / 4];
}
#define MANAGE_MAP(address) mapped_manage(address)
/* The settings of the modules: Guides STYLE (handle 0, 6 values), then Date
   Imprint STYLE (handle 1, 6 values) and LOOK (handle 2, 3 values). */
#define GUIDE_STYLE settings_memory[3]
#define DATE_STYLE settings_memory[6]
#define DATE_LOOK settings_memory[9]
/* The services block of the core (runtime/services.h). */
#define TEST_SERVICES 0x5c000000U
static unsigned services_memory[0x4000];
static volatile unsigned *mapped_services(unsigned address)
{
    assert(address >= TEST_SERVICES && address < TEST_SERVICES + sizeof(services_memory) && !(address & 3));
    return &services_memory[(address - TEST_SERVICES) / 4];
}
#define SERVICES_MAP(address) mapped_services(address)
/* The firmware objects that the MENU pages read: rows and controllers at
   TEST_OBJECTS, and the stock submenu kind table and the MENU tab table at
   their firmware addresses. */
#define TEST_OBJECTS 0x5b000000U
static unsigned object_memory[0x1000];
static unsigned stock_kinds[17 * 0x64 / 4];
static unsigned menu_tabs[8];
static unsigned *object_at(unsigned address);
#define MENU_OBJECT_WORD(a,o) (*object_at((a)+(o)))
#define MENU_OBJECT_BYTE(a,o) (((unsigned char *)object_at(((a)+(o)) & ~3U))[((a)+(o)) & 3U])
#define MENU_OBJECT_HALF(a,o) (((unsigned short *)object_at(((a)+(o)) & ~3U))[(((a)+(o)) & 3U) >> 1])
/* The memory of WORD, HALF, and BYTE: the services block by the address that
   services_word gives, or a firmware object. */
static unsigned char *mapped(unsigned address);
#define WORD(a,o) (*(volatile unsigned *)mapped((a)+(o)))
#define HALF(a,o) (*(volatile unsigned short *)mapped((a)+(o)))
#define BYTE(a,o) (*(volatile unsigned char *)mapped((a)+(o)))
/* The firmware view calls of the MENU pages: the function and the arguments
   of each, in order. A string view measures 0 wide. */
static unsigned view_calls, view_functions[16], view_args[16][4];
static unsigned view_call(unsigned fn, unsigned a, unsigned b, unsigned c, unsigned d);
static unsigned view_record(unsigned fn, unsigned a, unsigned b, unsigned c, unsigned d)
{
    if (view_calls < 16) {
        view_functions[view_calls] = fn;
        view_args[view_calls][0] = a; view_args[view_calls][1] = b;
        view_args[view_calls][2] = c; view_args[view_calls][3] = d;
    }
    ++view_calls;
    return view_call(fn, a, b, c, d);
}
/* The index of the first call of `fn`, or -1. */
static int view_index(unsigned fn)
{
    for (unsigned i = 0; i < view_calls && i < 16; ++i) if (view_functions[i] == fn) return (int)i;
    return -1;
}
static int view_called(unsigned fn) { return view_index(fn) >= 0; }
#define MENU_VIEW1(fn,a) view_record((fn), (a), 0, 0, 0)
#define MENU_VIEW2(fn,a,b) view_record((fn), (a), (b), 0, 0)
#define MENU_VIEW3(fn,a,b,c) view_record((fn), (a), (b), (c), 0)
#define MENU_VIEW4(fn,a,b,c,d) view_record((fn), (a), (b), (c), (d))
static unsigned level_calls, level_screen, level_mode;
#define MENU_LEVEL_ALARM(screen,mode) (++level_calls, level_screen = (screen), level_mode = (mode))
static unsigned preview_invalidations, preview_invalidated;
#define MENU_PREVIEW_INVALIDATE(view) (++preview_invalidations, preview_invalidated = (view))
/* The native settings of recipes (runtime/native_look.h): no settings
   record, and each UserData Set counted. */
static unsigned native_sets_count;
#define NATIVE_SETTINGS() 0U
#define NATIVE_SET(argv) ((void)(argv), ++native_sets_count)
#define NATIVE_MONOCHROME() 0U
#define NATIVE_SET_TYPE(code) ((void)(code), ++native_sets_count)
/* On GR IV, a model holds the use of the keyboard: an object of the test
   holds it. */
#ifdef FW_KEYBOARD_MODEL
#undef FW_KEYBOARD_MODEL
#undef FW_KEYBOARD_SET_FUNCTION
#define TEST_KEYBOARD_MODEL (TEST_OBJECTS + 0x600U)
static unsigned keyboard_model(void) { return TEST_KEYBOARD_MODEL; }
static unsigned keyboard_set_function(unsigned model, unsigned function) { BYTE(model, KEYBOARD_MODEL_FUNCTION) = (unsigned char)function; return 0; }
#define FW_KEYBOARD_MODEL keyboard_model
#define FW_KEYBOARD_SET_FUNCTION keyboard_set_function
#endif
/* The mode dial (runtime/mode_dial.h): the dial position and whether the
   firmware has loaded its settings, which a test sets, and the timers of the
   dial, which a test fires by their ID. */
#undef DIAL_POSITION
#undef DIAL_NATIVE_SETTLED
static unsigned dial_code, dial_loaded, dial_timer_ids, dial_timer_ms, dial_timer_cancels;
#define DIAL_POSITION() (dial_code)
#define DIAL_NATIVE_SETTLED(position) ((void)(position), dial_loaded)
static unsigned dial_timer_start(unsigned ms)
{
    dial_timer_ms = ms;
    return ++dial_timer_ids;
}
#define DIAL_TIMER_START(ms) dial_timer_start(ms)
#define DIAL_TIMER_CANCEL(id) ((void)((id) && ++dial_timer_cancels))
#include "menu.c"
_Static_assert(SERVICES_SIZE <= sizeof(services_memory), "The test maps the services block");
/* The firmware side of the checkbox page (runtime/menu_settings.h): the
   allocation gives TEST_CHECK_PAGE, and the SelectMenuController constructor
   initializes its list of three rows, whose row kinds are at
   TEST_CHECK_KINDS. */
#define TEST_CHECK_PAGE (TEST_OBJECTS + 0x3000U)
#define TEST_CHECK_KINDS (TEST_OBJECTS + 0x3800U)
static unsigned view_call(unsigned fn, unsigned a, unsigned b, unsigned c, unsigned d)
{
    (void)b; (void)c; (void)d;
#if defined(FW_CHECK_ITEM) && defined(FW_MENU_TABS)
    if (fn == FW_ALLOCATE) return TEST_CHECK_PAGE;
    if (fn == FW_SELECT_MENU_NEW) menu_check_lines(a + 0x100, TEST_CHECK_KINDS, 3, 50);
#else
    (void)fn; (void)a;
#endif
    return 0;
}
static unsigned *object_at(unsigned address)
{
    if (address >= FW_SUBMENU_KINDS && address < FW_SUBMENU_KINDS + sizeof(stock_kinds))
        return &stock_kinds[(address - FW_SUBMENU_KINDS) / 4];
#ifdef FW_MENU_TABS
    if (address >= FW_MENU_TABS && address < FW_MENU_TABS + sizeof(menu_tabs))
        return &menu_tabs[(address - FW_MENU_TABS) / 4];
#endif
    assert(address >= TEST_OBJECTS && address < TEST_OBJECTS + sizeof(object_memory) && !(address & 3));
    return &object_memory[(address - TEST_OBJECTS) / 4];
}
static unsigned char *mapped(unsigned address)
{
    unsigned services = (unsigned)(unsigned long)services_memory;
    if (address - services < sizeof(services_memory)) return (unsigned char *)services_memory + (address - services);
    return (unsigned char *)object_at(address & ~3U) + (address & 3U);
}
/* The page of module i (runtime/modules.h). */
#define MODULE(i) (*menu_page(test_header, (i)))
/* XPan, a crop on every camera. */
#define CROP_RATIO 1U
/* A page icon as its global ID. */
#define PAGE_ICON(module, row) module_icon((module), MODULE((module)).icon(test_header, (row)))
#define CURRENT_ICON(row) module_icon(current_index(test_header), current_module(test_header)->icon(test_header, (row)))

static volatile u32 test_header[GRKIT_HOST_HEADER_SIZE / 4];
static void record_native_aspect(unsigned id)
{
    native_requested_crop = test_header[GRKIT_HOST_FRAME_CROP / 4];
    ++native_sets;
    native_aspect_value = id;
}
static unsigned record_framing_request(void)
{
    requested_crop = test_header[GRKIT_HOST_FRAME_CROP / 4];
    return ++framing_requests;
}
static u16 text_arena[GRKIT_TEXT_SIZE / 2] __attribute__((aligned(4)));
static const u16 *text_at(u32 address)
{
    if (address >= TEST_PAGE_TEXT && address < TEST_PAGE_TEXT + sizeof(page_texts) / 64 * 128)
        return page_texts[(address - TEST_PAGE_TEXT) / 128];
    u32 offset = address - GRKIT_TEXT(test_header);
    assert(offset < GRKIT_TEXT_SIZE && !(offset & 1));
    return text_arena + offset / 2;
}
/* A menu string equals an ASCII string. */
static int same(const u16 *actual, const char *expected)
{
    u32 i = 0;
    for (; expected[i]; ++i) if (actual[i] != (u8)expected[i]) return 0;
    return actual[i] == 0;
}
/* Framing labels and summaries by ratio value, and the ratio value of each
   row of the target. */
static const char *const ratio_names[] = {"Off", "XPan 65:24", "16:9", "4:3", "2.35:1", "1:1", "5:4", "7:6"};
static const char *const ratio_summaries[] = {"Framing: Off", "Framing: XPan", "Framing: 16:9", "Framing: 4:3",
                                              "Framing: 2.35:1", "Framing: 1:1", "Framing: 5:4", "Framing: 7:6"};
/* A value that no target offers. */
#define UNKNOWN_RATIO 8
static const u32 ratio_rows[] = {FRAMING_RATIO_ROWS};
#define ROW_COUNT ((u32)(sizeof(ratio_rows) / sizeof(ratio_rows[0])))
/* XPan: row 1, and a crop on every camera. */
#define CROP_ROW 1U
static const char *const guide_names[] = {"Off", "Thirds", "Center", "Golden Ratio", "Diagonals", "Thirds + Center"};
static void ascii(unsigned short *to, const char *from)
{
    while ((*to++ = (u8)*from++)) {}
}
static u32 text_count;
/* The bytes of one text list: a pointer for each language. */
#define TEXT_LIST (GRKIT_TEXT_LANGUAGES * 4U)
/* The camera address of a new page text. */
static u32 page_text(const char *text)
{
    assert(text_count < sizeof(page_texts) / sizeof(page_texts[0]));
    ascii(page_texts[text_count], text);
    return TEST_PAGE_TEXT + text_count++ * 128;
}
/* Register the choice page of module page slot `slot`, as INSTALL does: the
   page with its text in each language, its setting handle, and the global
   index of its tile 0. A page without rows (`names` 0) gets its rows from
   the core. */
/* The title list of a test page, after the 56 bytes of a page of ABI 2.5. */
#define PAGE_TITLES 0x38U
static void install_page(u32 slot, const char *title, const char *const *names, u32 rows, u32 handle, u32 tile)
{
    u32 page = TEST_PAGE + slot * TEST_PAGE_SIZE, titles = page + PAGE_TITLES, summaries = titles + TEXT_LIST,
        labels = summaries + rows * TEXT_LIST;
    char summary[64];
    assert(slot < TEST_PAGE_SLOTS && labels + rows * TEXT_LIST <= page + 0x200);
    const unsigned fields[] = {40, 1, 0, names ? 1 : 2, titles, names ? summaries : 0, names ? labels : 0, rows, 0, 0};
    for (u32 i = 0; i < 10; ++i) *page_at(page + i * 4) = fields[i];
    for (u32 language = 0; language < GRKIT_TEXT_LANGUAGES; ++language) {
        *page_at(titles + language * 4) = page_text(title);
        for (u32 value = 0; value < rows; ++value) {
            strcpy(summary, title);
            strcat(summary, ": ");
            strcat(summary, names[value]);
            *page_at(summaries + (value * GRKIT_TEXT_LANGUAGES + language) * 4) = page_text(summary);
            *page_at(labels + (value * GRKIT_TEXT_LANGUAGES + language) * 4) = page_text(names[value]);
        }
    }
    if (test_header[GRKIT_HOST_MODULE_PAGE_COUNT / 4] <= slot) test_header[GRKIT_HOST_MODULE_PAGE_COUNT / 4] = slot + 1;
    test_header[GRKIT_HOST_MODULE_PAGES / 4 + slot * 3] = page;
    test_header[GRKIT_HOST_MODULE_PAGES / 4 + slot * 3 + 1] = handle;
    test_header[GRKIT_HOST_MODULE_PAGES / 4 + slot * 3 + 2] = tile;
}
static const char *const grain_names[] = {"Off", "Weak", "Medium", "Strong", "Extra Strong"};
/* The summaries of Framing, which name the ratios shorter. */
static const char *const ratio_values[] = {"Off", "XPan", "16:9", "4:3", "2.35:1", "1:1", "5:4", "7:6"};
static const char *const stamp_names[] = {"Off", "'YY M D", "'YY M D H:MM", "M D 'YY", "D M 'YY", "D M 'YY H:MM"};
static const char *const look_names[] = {"Classic", "GR1s", "GR II"};
/* The rows of the native settings page of Date Imprint: STYLE, then LOOK. */
static void install_setting_rows(u32 slot)
{
    u32 page = TEST_PAGE + slot * TEST_PAGE_SIZE, rows = page + 0x200, texts = rows + 32;
    const char *const labels[2] = {"Date Style", "Date Look"};
    const char *const *values[2] = {stamp_names, look_names};
    const u32 counts[2] = {6, 3};
    *page_at(page + PAGE_SETTING_ROWS) = 2;
    *page_at(page + PAGE_SETTING_LIST) = rows;
    for (u32 r = 0; r < 2; ++r) {
        *page_at(rows + r * 16) = 16;
        /* INSTALL writes the handle of each row: STYLE is handle 1. */
        *page_at(rows + r * 16 + ROW_SETTING) = r + 1;
        *page_at(rows + r * 16 + ROW_LABEL) = texts;
        for (u32 language = 0; language < GRKIT_TEXT_LANGUAGES; ++language) *page_at(texts + language * 4) = page_text(labels[r]);
        texts += TEXT_LIST;
        *page_at(rows + r * 16 + ROW_VALUES) = texts;
        for (u32 value = 0; value < counts[r]; ++value)
            for (u32 language = 0; language < GRKIT_TEXT_LANGUAGES; ++language)
                *page_at(texts + (value * GRKIT_TEXT_LANGUAGES + language) * 4) = page_text(values[r][value]);
        texts += counts[r] * TEXT_LIST;
    }
    assert(texts <= page + TEST_PAGE_SIZE);
}
/* The settings rows of a Guides page modeled on modules/guides: STYLE (handle 0),
   COLOR (15), OPACITY (16), and DISP (17). */
static void install_guide_rows(u32 slot)
{
    u32 page = TEST_PAGE + slot * TEST_PAGE_SIZE, rows = page + 0x200, texts = rows + 64;
    static const char *const colors[] = {"Black", "White", "Yellow"}, *const opacities[] = {"50%", "100%"},
                             *const disp[] = {"On", "Off"};
    const char *const labels[4] = {"Style", "Color", "Opacity", "Hide with DISP"};
    const char *const *values[4] = {guide_names, colors, opacities, disp};
    const u32 counts[4] = {6, 3, 2, 2}, handles[4] = {0, 15, 16, 17};
    *page_at(page + PAGE_SETTING_ROWS) = 4;
    *page_at(page + PAGE_SETTING_LIST) = rows;
    for (u32 r = 0; r < 4; ++r) {
        *page_at(rows + r * 16) = 16;
        *page_at(rows + r * 16 + ROW_SETTING) = handles[r];
        *page_at(rows + r * 16 + ROW_LABEL) = texts;
        for (u32 language = 0; language < GRKIT_TEXT_LANGUAGES; ++language) *page_at(texts + language * 4) = page_text(labels[r]);
        texts += TEXT_LIST;
        *page_at(rows + r * 16 + ROW_VALUES) = texts;
        for (u32 value = 0; value < counts[r]; ++value)
            for (u32 language = 0; language < GRKIT_TEXT_LANGUAGES; ++language)
                *page_at(texts + (value * GRKIT_TEXT_LANGUAGES + language) * 4) = page_text(values[r][value]);
        texts += counts[r] * TEXT_LIST;
    }
    assert(texts <= page + TEST_PAGE_SIZE);
}
/* The official modules (modules/), as INSTALL registers them in their root
   order: Film Look, a page of CORE.TONE with rows from the core;
   Framing, a page and layers of CORE.ASPECT; Guides, its STYLE setting and
   a layer; Date Imprint, its STYLE and LOOK settings, a page with native
   settings rows, and a filter; Grain, a page of CORE.GRAIN. */
static void install_modules(void)
{
    install_page(0, "Film Look", 0, 0, GRKIT_CORE_TONE, 0);
    install_page(1, "Framing", ratio_values, 8, GRKIT_CORE_ASPECT, 0);
    install_page(2, "Guides", guide_names, 6, 0, 7);
    install_page(3, "Date Imprint", stamp_names, 6, 1, 13);
    install_setting_rows(3);
    install_page(4, "Grain", grain_names, 5, GRKIT_CORE_GRAIN, 19);
    /* The labels of Framing name the ratios in full. */
    for (u32 value = 0; value < 8; ++value)
        for (u32 language = 0; language < GRKIT_TEXT_LANGUAGES; ++language)
            *page_at(PAGE_WORD(TEST_PAGE + TEST_PAGE_SIZE, PAGE_LABELS) + (value * GRKIT_TEXT_LANGUAGES + language) * 4) =
                page_text(ratio_names[value]);
    memset(settings_memory, 0, sizeof settings_memory);
    settings_memory[0] = 3; settings_memory[4] = 6; settings_memory[7] = 6; settings_memory[10] = 3;
    test_header[GRKIT_HOST_MODULE_SETTINGS / 4] = TEST_SETTINGS;
    test_header[GRKIT_HOST_CAPABILITIES / 4] = 1U << (GRKIT_CORE_TONE - GRKIT_CORE_SETTING_BASE) |
                                               1U << (GRKIT_CORE_ASPECT - GRKIT_CORE_SETTING_BASE) |
                                               1U << (GRKIT_CORE_GRAIN - GRKIT_CORE_SETTING_BASE);
    test_header[GRKIT_HOST_MODULE_LAYER_COUNT / 4] = 1;
    test_header[GRKIT_HOST_MODULE_LAYERS / 4] = 0x5e000001;
    test_header[GRKIT_HOST_MODULE_LAYERS / 4 + 1] = 0;
    test_header[GRKIT_HOST_MODULE_LAYERS / 4 + 2] = 20U << 16 | 1;
}
/* The root rows of the module pages, then About. */
#define IMAGE_ROW 0U
#define IMAGE_MODULE MODULE_PAGE(0)
#define FRAMING_ROW 1U
#define FRAMING_MODULE MODULE_PAGE(1)
#define FRAMING_TILE(n) GRKIT_ICON_MODULE_TILE(n)
#define GUIDES_ROW 2U
#define GUIDES_MODULE MODULE_PAGE(2)
#define GUIDE_TILE(n) GRKIT_ICON_MODULE_TILE(7 + (n))
#define DATE_ROW 3U
#define DATE_MODULE MODULE_PAGE(3)
#define DATE_TILE(n) GRKIT_ICON_MODULE_TILE(13 + (n))
#define GRAIN_ROW 4U
#define GRAIN_MODULE MODULE_PAGE(4)
#define GRAIN_TILE(n) GRKIT_ICON_MODULE_TILE(19 + (n))
#define ALL_PAGES 5U
static u32 native_changes;
static u32 native_applies;
static u32 native_events;
volatile u32 *menu_header(void) { return test_header; }
u32 menu_event_original(u32 controller, const u16 *event)
{ (void)controller; (void)event; ++native_events; return 42; }
void menu_exit_original(u32 controller) { (void)controller; }
u32 menu_destroy_original(u32 controller) { return controller; }
void menu_refresh_original(u32 controller) { (void)controller; }
void menu_changed_original(u32 controller) { (void)controller; ++native_changes; }
void menu_apply_original(u32 controller) { (void)controller; ++native_applies; }

/* Each block starts from this state: the label table, the official modules,
   five effect records with their icons, and no other memory, setting, call,
   or header field. */
static void reset(void)
{
    memset((void *)test_header, 0, sizeof test_header);
    memset(text_arena, 0, sizeof text_arena);
    memset(services_memory, 0, sizeof services_memory);
    memset(object_memory, 0, sizeof object_memory);
    memset(stock_kinds, 0, sizeof stock_kinds);
    memset(menu_tabs, 0, sizeof menu_tabs);
    memset(settings_memory, 0, sizeof settings_memory);
    memset(manage_memory, 0, sizeof manage_memory);
    memset(page_memory, 0, sizeof page_memory);
    memset(page_texts, 0, sizeof page_texts);
    text_count = 0;
    view_calls = 0;
    framing_requests = requested_crop = native_aspect_value = native_sets = native_requested_crop = live_view_stops = 0;
    native_changes = native_applies = native_events = 0;
    factory_calls = factory_top = 0;
    test_language = test_movie = 0;
    assert(sizeof(menu_text_table) <= GRKIT_TEXT_SUMMARY);
    memcpy(text_arena, menu_text_table, sizeof(menu_text_table));
    test_header[GRKIT_HOST_COUNT / 4] = 5;
    /* Older modules keep their implicit tone slot. Explicit registrations
       are exercised separately below. */
    test_header[GRKIT_HOST_LEGACY_TONE_OWNER / 4] = TEST_PAGE;
    /* The official modules are loaded. */
    install_modules();
    test_header[GRKIT_HOST_OFF_ICON / 4] = 0x59001000;
    test_header[GRKIT_HOST_GRKIT_ICON / 4] = 0x59003000;
    test_header[GRKIT_HOST_RECORDS / 4] = 0x59005000;
}

static void the_root_reads_off_without_a_committed_effect(void)
{
    volatile u32 slot[2];
    reset();
    descriptor(slot, grkit_title);
    assert(slot[0] == 0x106);
    assert(selected_icon(test_header) == 0x7fff);
    descriptor(slot, MODULE(0).summary(test_header));
    assert(same(MODULE(0).summary(test_header), "Film Look: Off") && slot[0] == 0x10f);
    assert(same(MODULE(0).title(test_header), "Film Look"));
    /* The root row shows tile 0 of the module for Off; the Off row of the
       page keeps the Off icon. A module without tiles shows Off. */
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_ROOT;
    assert(MODULE(0).icon(test_header, 0) == GRKIT_ICON_MODULE_TILE(0));
    test_header[GRKIT_HOST_MODULE_PAGES / 4 + 2] = MANAGE_NO_TILE;
    assert(MODULE(0).icon(test_header, 0) == GRKIT_ICON_OFF);
    test_header[GRKIT_HOST_MODULE_PAGES / 4 + 2] = 0;
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_MODULE;
    assert(MODULE(0).icon(test_header, 0) == GRKIT_ICON_OFF);
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = 0;
}

static void each_language_reads_its_own_texts_and_font(void)
{
    reset();
    /* Each language reads its own strings, and an unknown language reads English. */
    for (u32 language = 0; language < GRKIT_TEXT_LANGUAGES + 2; ++language) {
        u32 table = language < GRKIT_TEXT_LANGUAGES ? language : 0;
        test_language = language;
        for (u32 id = 0; id < GRKIT_TEXT_STRINGS; ++id)
            assert(text(test_header, id) == text_arena + text_arena[table * GRKIT_TEXT_STRINGS + id] / 2);
    }
    test_language = 2;
    assert(!same(text(test_header, TEXT_IMAGE_OFF_SUMMARY), "Film Look: Off"));
    /* The text hooks select the font of the text language. */
    assert(menu_table_index() == 18);
    test_language = 1;
    assert(menu_table_index() == 19);
    test_language = 3;
    assert(menu_table_index() == 0);
    test_language = 4;
    assert(menu_table_index() == 17);
    test_language = 7;
    assert(menu_table_index() == 1);
    test_language = 0;
    assert(menu_table_index() == 1);
}

static void the_viewfinder_shows_the_committed_record(void)
{
    reset();
    assert(menu_viewfinder_icon_select(0x166, 0) == 0x166);
    assert(menu_viewfinder_icon_select(0, 0) == 0);
    test_header[GRKIT_HOST_ACTIVE / 4] = 0x59005000 + 3 * GRKIT_RECORD_SIZE;
    assert(selected_icon(test_header) == 0x8003);
    /* The summary of record 3 is its slot in the menu language. */
    for (u32 language = 0; language < GRKIT_TEXT_LANGUAGES; ++language) {
        test_language = language;
        assert(MODULE(0).summary(test_header) ==
               text_arena + (GRKIT_TEXT_SUMMARY + (language * GRKIT_TEXT_RECORDS + 3) * GRKIT_SUMMARY_UNITS * 2) / 2);
    }
    test_language = 0;
    assert(menu_viewfinder_icon_select(0x166, 0) == 0x8003);
    assert(menu_viewfinder_icon_select(0x166, 1) == 0x166);
    test_header[GRKIT_HOST_ACTIVE / 4] += 4;
    assert(menu_viewfinder_icon_select(0x167, 0) == 0x167);
    test_header[GRKIT_HOST_ACTIVE / 4] = 0;
}

static void fn_applies_to_effect_rows_of_film_look(void)
{
    reset();
    /* Fn and its hint apply to effect rows of Film Look only. */
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_MODULE;
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 0;
    assert(!parameters_row(test_header));
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 2;
    assert(parameters_row(test_header));
    test_header[GRKIT_HOST_MODULE / 4] = 1;
    assert(!parameters_row(test_header));
    test_header[GRKIT_HOST_MODULE / 4] = 0;
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_ROOT;
    assert(!parameters_row(test_header));
    /* On the root, Fn applies to the Film Look row while an effect is
       committed. Off and the rows of other modules have no parameters. */
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 0;
    assert(!parameters_row(test_header));
    test_header[GRKIT_HOST_ACTIVE / 4] = 0x59005000 + 2 * GRKIT_RECORD_SIZE;
    assert(parameters_row(test_header));
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 1;
    assert(!parameters_row(test_header));
    test_header[GRKIT_HOST_ACTIVE / 4] = 0;
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = 0;
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 0;
    assert(!parameters_row(test_header));
}

static void switch_presses_and_back_events_are_filtered(void)
{
    reset();
    assert(menu_press(2, 0));
    assert(!menu_press(1, 0));
    assert(!menu_press(2, 1));
    assert(!menu_press(2, 100));
    assert(back_event(test_header, EV_SWITCH, 1) == 1);
    assert(back_event(test_header, EV_SWITCH, 1) == 2);
    assert(back_event(test_header, 0x19d, 1) == 0);
    assert(back_event(test_header, EV_CONFIRM, 0) == 2);
    assert(back_event(test_header, EV_CONFIRM, 0) == 0);
    assert(back_event(test_header, EV_SWITCH, 0) == 0);
    assert(back_event(test_header, EV_SWITCH, 1) == 1);
    assert(back_event(test_header, EV_SWITCH_END, 1) == 0);
    assert(back_event(test_header, EV_CONFIRM, 1) == 0);
}

static void list_restoration_keeps_the_committed_effect(void)
{
    reset();
    /* List restoration changes level to native before rebuilding the rows. */
    test_header[GRKIT_HOST_ACTIVE / 4] = 0x59005000;
    test_header[GRKIT_HOST_MENU_CONTROLLER / 4] = 0x1000;
    test_header[GRKIT_HOST_MENU_REBUILDING / 4] = 1;
    menu_changed(0x1000);
    assert(native_changes == 0);
    assert(test_header[GRKIT_HOST_ACTIVE / 4] == 0x59005000);
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = 1;
    menu_changed(0x1000);
    assert(native_changes == 0);
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = 0;
    test_header[GRKIT_HOST_MENU_REBUILDING / 4] = 0;
    menu_changed(0x1000);
    assert(native_changes == 1);
    {
        const u16 confirm[] = {EV_CONFIRM};
        test_header[GRKIT_HOST_MENU_LEVEL / 4] = 0;
        test_header[GRKIT_HOST_BACK_PENDING / 4] = 1;
        assert(menu_event(0x1000, confirm) == 0);
        assert(native_events == 0);
        assert(test_header[GRKIT_HOST_BACK_PENDING / 4] == 0);
        assert(test_header[GRKIT_HOST_ACTIVE / 4] == 0x59005000);
    }
}

static void a_touch_on_a_grkit_row_keeps_the_native_slot_value(void)
{
    reset();
    /* A touch on a row of the ADJ list applies its row to the slot setting.
       On a GRKit page it selects the GRKit row instead; while a UI box is
       open, it does nothing. Neither writes the native slot. */
    test_header[GRKIT_HOST_SERVICES / 4] = TEST_SERVICES;
    test_header[GRKIT_HOST_MENU_CONTROLLER / 4] = 0x1000;
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_ROOT;
    ui_word(test_header, UI_MODE)[0] = UI_MESSAGE;
    menu_apply(0x1000);
    assert(native_applies == 0);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT);
    /* Another controller, and the native rows, apply as before. */
    menu_apply(0x2000);
    assert(native_applies == 1);
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = 0;
    menu_apply(0x1000);
    assert(native_applies == 2);
}

static void the_adj_update_shows_no_slot_type_on_a_grkit_page(void)
{
    reset();
    /* The periodic ADJ update reads the slot type to show the exposure box.
       On a GRKit page it reads type 0: no box. */
    test_header[GRKIT_HOST_MENU_CONTROLLER / 4] = 0x1000;
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_MODULE;
    assert(menu_tick_type(0x1000, 4) == 0);
}

static void the_event_ring_tells_menu_from_ok_and_adj(void)
{
    reset();
    /* The event ring distinguishes MENU's confirm from OK and ADJ press confirms. */
    test_header[GRKIT_HOST_MENU_EVENTS / 4] = 18;
    test_header[GRKIT_HOST_MENU_EVENT_RING / 4] = EV_SWITCH | (2 << 16);
    test_header[GRKIT_HOST_MENU_EVENT_RING / 4 + 1] = EV_CONFIRM;
    assert(after_menu_switch(test_header));
    test_header[GRKIT_HOST_MENU_EVENT_RING / 4] = EV_SWITCH_END;
    assert(!after_menu_switch(test_header));
    test_header[GRKIT_HOST_MENU_EVENT_RING / 4] = EV_ADJ_PRESS;
    assert(!after_menu_switch(test_header));
    /* An ADJ press is EV_ADJ_PRESS, then the confirm. OK and MENU are not. */
    assert(after_adj_press(test_header));
    test_header[GRKIT_HOST_MENU_EVENT_RING / 4] = EV_SWITCH_END | (2 << 16);
    assert(!after_adj_press(test_header));
    test_header[GRKIT_HOST_MENU_EVENT_RING / 4] = EV_SWITCH | (2 << 16);
    assert(!after_adj_press(test_header));
    test_header[GRKIT_HOST_MENU_EVENTS / 4] = 0;
    test_header[GRKIT_HOST_MENU_EVENT_RING / 4] = test_header[GRKIT_HOST_MENU_EVENT_RING / 4 + 1] = 0;
#ifdef EV_FN_ACTION
    /* GR IV: the Fn press that opens a GRKit parameter page also sends the
       native Fn action, which must not open the native page. */
    {
        const u16 action[] = {EV_FN_ACTION};
        u32 events = native_events;
        test_header[GRKIT_HOST_RETURN / 4] = RETURN_OPENING;
        assert(menu_event(0x1000, action) == 0);
        test_header[GRKIT_HOST_RETURN / 4] = RETURN_PARAMETERS | RETURN_ROOT;
        assert(menu_event(0x1000, action) == 0);
        assert(native_events == events);
        test_header[GRKIT_HOST_RETURN / 4] = 0;
        test_header[GRKIT_HOST_MENU_EVENTS / 4] = 0;
        test_header[GRKIT_HOST_MENU_EVENT_RING / 4] = test_header[GRKIT_HOST_MENU_EVENT_RING / 4 + 1] = 0;
    }
#endif
}

static void playback_matches_private_tags_only(void)
{
    reset();
    /* Playback matches only extracted non-native tags with the private code bit. */
    u32 count = test_header[GRKIT_HOST_COUNT / 4];
    assert(playback_icon(test_header, 0) == 0);
    assert(playback_icon(test_header, 5) == 0);
    assert(playback_icon(test_header, 0x10000 | 0x7eff) == 0);
    assert(playback_icon(test_header, 0x20000 | 0x7f00) == 0);
    test_header[GRKIT_HOST_COUNT / 4] = 0;
    assert(playback_icon(test_header, 0x10000 | 0x7f00) == 0);
    test_header[GRKIT_HOST_COUNT / 4] = count;
}

static void icons_toolbar_and_edges_map_their_ids(void)
{
    reset();
    assert(!native_action_release(10, 0));
    assert(!native_action_release(0, 1));
    assert(native_action_release(10, 1));
    assert(menu_icon(0x7ffe) == 0x59003000);
    /* The Version icon follows the GRKit icon in chunk 0. */
    assert(menu_icon(GRKIT_ICON_VERSION) == 0x59003000 + GRKIT_ICON_STRIDE);
    assert(toolbar_x(0) == 415 && toolbar_x(5) == 665);
    assert(menu_icon(0x7fff) == 0x59001000);
    assert(menu_icon(0x166) == 0);
    assert(menu_icon(0x8005) == 0);
    assert(menu_icon(0xffff) == 0);
    assert(wrapped_edge(EV_RIGHT, 3, 0, 0) == 2);
    assert(wrapped_edge(EV_LEFT, 1, 4, 0) == 1);
    assert(wrapped_edge(EV_RIGHT, 1, 2, 0) == 0);
    assert(wrapped_edge(EV_LEFT, 3, 2, 0) == 0);
    assert(wrapped_edge(EV_RIGHT, 4, 0, 1) == 0);
    assert(wrapped_edge(0x35, 4, 0, 0) == 0);
    assert(advance_from_private(EV_RIGHT, 1) == 1);
    assert(advance_from_private(EV_LEFT, 1) == 0);
    assert(advance_from_private(EV_RIGHT, 2) == 0);
    assert(advance_from_private(EV_LEFT, 2) == 1);
    /* ADJ that reopens GRKit after a power-on has no edge: Left and Right
       find the neighbor slots as after a touch. A known edge stays. */
    assert(reopen_edge(0) == 3);
    assert(reopen_edge(1) == 1 && reopen_edge(2) == 2 && reopen_edge(3) == 3);
}

/* A half-press on the Fn adjustments page changes to the shooting screen,
   and the release changes back and starts the page again. The page must
   still edit the effect, not the native Image Control settings. Any other
   screen ends the session, so a later native page edits the native ones. */
static void a_half_press_keeps_the_parameter_session(void)
{
    const u32 record = 0x59005000;
    reset();
    test_header[GRKIT_HOST_PARAMETERS / 4] = record | 1U;
    menu_parameter_controller(test_header, IC_PARAMETERS_CONTROLLER);
    menu_parameter_started(test_header);
    assert(menu_parameter_session() == record);
    menu_parameter_controller(test_header, SHOOTING_CONTROLLER);
    assert(menu_parameter_session() == record);
    menu_parameter_controller(test_header, IC_PARAMETERS_CONTROLLER);
    menu_parameter_started(test_header);
    assert(menu_parameter_session() == record);
    /* The page ends at ADJ (menu_settings_controller calls this on each
       controller change): a native page that ADJ opens later has no
       session. */
    menu_parameter_controller(test_header, 0x2000000);
    assert(menu_parameter_session() == 0 && test_header[GRKIT_HOST_PARAMETERS / 4] == 0);
    menu_parameter_started(test_header);
    assert(menu_parameter_session() == 0);
    /* A native page with no session stays native through a half-press. */
    menu_parameter_controller(test_header, SHOOTING_CONTROLLER);
    menu_parameter_controller(test_header, IC_PARAMETERS_CONTROLLER);
    menu_parameter_started(test_header);
    assert(menu_parameter_session() == 0);
    /* A record that waits for its page stays through the change to it. */
    test_header[GRKIT_HOST_PARAMETERS / 4] = record | 1U;
    menu_parameter_controller(test_header, IC_PARAMETERS_CONTROLLER);
    assert(test_header[GRKIT_HOST_PARAMETERS / 4] == (record | 1U));
    test_header[GRKIT_HOST_PARAMETERS / 4] = 0;
}

/* A native look keeps its Fn defaults in the record: the page shows the
   default plus the stored change, and a stored zero is the default. */
static void fn_values_add_the_defaults_of_a_native_look(void)
{
    const u32 record = TEST_OBJECTS + 0x2000U;
    reset();
    for (u32 k = 0; k < 9; ++k) {
        BYTE(record, GRKIT_RECORD_PARAMETERS + k) = 0;
        BYTE(record, GRKIT_RECORD_FN_DEFAULTS + k) = 0;
    }
    /* Contrast +3 and shadow -2, each its shown value mod 9. */
    BYTE(record, GRKIT_RECORD_FN_DEFAULTS + 3) = 3;
    BYTE(record, GRKIT_RECORD_FN_DEFAULTS + 5) = 7;
    test_header[GRKIT_HOST_PARAMETERS / 4] = record;
    /* The getter gives the native value 0..8 plus 1. */
    assert(menu_parameter_get(0) == 4 + 1);
    assert(menu_parameter_get(3) == 7 + 1);
    assert(menu_parameter_get(5) == 2 + 1);
    /* The default stores zero; the ends of the native range store their change. */
    assert(menu_parameter_set(3, 7) && BYTE(record, GRKIT_RECORD_PARAMETERS + 3) == 0);
    assert(menu_parameter_set(3, 8) && BYTE(record, GRKIT_RECORD_PARAMETERS + 3) == 1);
    assert(menu_parameter_get(3) == 8 + 1);
    assert(menu_parameter_set(3, 0) && BYTE(record, GRKIT_RECORD_PARAMETERS + 3) == 2);
    assert(menu_parameter_get(3) == 0 + 1);
    assert(menu_parameter_set(5, 8) && BYTE(record, GRKIT_RECORD_PARAMETERS + 5) == 6);
    assert(menu_parameter_get(5) == 8 + 1);
    /* A stored byte above 8 counts as no change. */
    BYTE(record, GRKIT_RECORD_PARAMETERS + 5) = 200;
    assert(menu_parameter_get(5) == 2 + 1);
    test_header[GRKIT_HOST_PARAMETERS / 4] = 0;
}

/* Toning (runtime/toning.h): adjustment 9 of a native look with its own red and
   blue curves in a tail slot. A change makes the record curves from the slot,
   green + (own - green) x (2 + t) / 2, and clears the gamma key. */
static void toning_moves_red_and_blue_from_green(void)
{
#ifdef PARAMETER_TONING_BASE
    const u32 record = TEST_OBJECTS + 0x2000U, slot = TEST_OBJECTS + 0x3400U;
    reset();
    for (u32 k = 0; k < 9; ++k) {
        BYTE(record, GRKIT_RECORD_PARAMETERS + k) = 0;
        BYTE(record, GRKIT_RECORD_FN_DEFAULTS + k) = 0;
    }
    test_header[GRKIT_HOST_TAIL / 4] = slot - GRKIT_TONE_CURVES;
    BYTE(record, GRKIT_RECORD_TONE_SLOT) = 1;
    BYTE(record, GRKIT_RECORD_TONING) = 0;
    for (u32 i = 0; i < 256; ++i) {
        u32 green = 1000 + i * 40;
        HALF(record, GRKIT_RECORD_CURVES + 512 + i * 2) = (u16)green;
        HALF(slot, i * 2) = (u16)(green + 100);
        HALF(slot, 512 + i * 2) = (u16)(green - 200);
    }
    test_header[GRKIT_HOST_PARAMETERS / 4] = record;
    /* Its own toning reads as the neutral native value. */
    assert(menu_parameter_get(PARAMETER_TONING) == 4 + 1);
    const struct { u32 native; u8 stored; int red, blue; } steps[] = {
        {5, 1, 150, -300},  /* +1, the GR IV level 3 */
        {3, 8, 50, -100},   /* -1, the GR IV level 1 */
        {2, 7, 0, 0},       /* -2, gray */
        {0, 5, -100, 200},  /* -4, the other way */
        {8, 4, 300, -600},  /* +4 */
        {4, 0, 100, -200},  /* its own toning again */
    };
    for (u32 s = 0; s < sizeof(steps) / sizeof(steps[0]); ++s) {
        test_header[GRKIT_HOST_GAMMA_KEY / 4] = 0x1234567U;
        assert(menu_parameter_set(PARAMETER_TONING, steps[s].native));
        assert(BYTE(record, GRKIT_RECORD_TONING) == steps[s].stored);
        assert(menu_parameter_get(PARAMETER_TONING) == steps[s].native + 1);
        assert(test_header[GRKIT_HOST_GAMMA_KEY / 4] == 0);
        for (u32 i = 0; i < 256; i += 51) {
            int green = 1000 + (int)i * 40;
            assert(HALF(record, GRKIT_RECORD_CURVES + i * 2) == green + steps[s].red);
            assert(HALF(record, GRKIT_RECORD_CURVES + 1024 + i * 2) == green + steps[s].blue);
            assert(HALF(record, GRKIT_RECORD_CURVES + 512 + i * 2) == green);
        }
    }
    /* The snapshot holds Toning in its row kind. */
    const u32 model = TEST_OBJECTS + 0x3800U;
    BYTE(record, GRKIT_RECORD_TONING) = 1;
    menu_parameter_page_snapshot(model);
    assert(BYTE(model, MODEL_SNAPSHOT + PARAMETER_TONING_KIND) == 5);
    /* Without a slot, the native getter and setter run. */
    BYTE(record, GRKIT_RECORD_TONE_SLOT) = 0;
    assert(menu_parameter_get(PARAMETER_TONING) == 0);
    assert(!menu_parameter_set(PARAMETER_TONING, 6));
    assert(BYTE(record, GRKIT_RECORD_TONING) == 1);
    test_header[GRKIT_HOST_PARAMETERS / 4] = 0;
#endif
}

static void adj_exits_return_once_for_the_parameter_page(void)
{
    reset();
    /* ADJ exits once for the parameter page. An exit from GRKit, or MENU or OK
       that closed ADJ from GRKit, opens GRKit at the next ADJ. Any other exit
       ends the return. */
    assert(exit_return(RETURN_OPENING, 0) == RETURN_PARAMETERS);
    assert(exit_return(RETURN_CLOSING, 0) == RETURN_GRKIT);
    assert(exit_return(0, 1) == RETURN_GRKIT);
    assert(exit_return(0, 0) == 0);
    assert(exit_return(RETURN_GRKIT, 0) == 0);
    assert(exit_return(RETURN_PARAMETERS, 0) == 0);
    /* A page that Fn opened from the root keeps its mark until it returns. */
    assert(exit_return(RETURN_OPENING | RETURN_ROOT, 0) == (RETURN_PARAMETERS | RETURN_ROOT));
    assert(exit_return(RETURN_PARAMETERS | RETURN_ROOT, 0) == 0);
    assert(RETURN_STATE(RETURN_PARAMETERS | RETURN_ROOT) == RETURN_PARAMETERS);
    /* The page, the shooting screen of a half-press, and ADJ keep the return
       of an open page. Any other screen, such as playback, ends it. */
    test_header[GRKIT_HOST_RETURN / 4] = RETURN_PARAMETERS | RETURN_ROOT;
    menu_settings_controller(IC_PARAMETERS_CONTROLLER);
    menu_settings_controller(SHOOTING_CONTROLLER);
    menu_settings_controller(IC_PARAMETERS_CONTROLLER);
    assert(test_header[GRKIT_HOST_RETURN / 4] == (RETURN_PARAMETERS | RETURN_ROOT));
    menu_settings_controller(ADJ_CONTROLLER);
    assert(test_header[GRKIT_HOST_RETURN / 4] == (RETURN_PARAMETERS | RETURN_ROOT));
    menu_settings_controller(0x4000000);
    assert(!test_header[GRKIT_HOST_RETURN / 4]);
    /* ADJ that opens GRKit after an exit from GRKit keeps its return. */
    test_header[GRKIT_HOST_RETURN / 4] = RETURN_GRKIT;
    menu_settings_controller(0x4000000);
    assert(test_header[GRKIT_HOST_RETURN / 4] == RETURN_GRKIT);
    test_header[GRKIT_HOST_RETURN / 4] = 0;
    /* CORE.ADJ mirrors the sticky preference that menu_exit writes: 1 when
       the next ADJ opens GRKit, 0 when it opens as a native entry. A
       transient return to the parameter page does not change it. */
    assert(exit_return(0, 1) == RETURN_GRKIT);
    assert(exit_return(0, 0) == 0);
    assert(RETURN_STATE(exit_return(RETURN_OPENING, 0)) == RETURN_PARAMETERS);
    /* INSTALL restored CORE.ADJ = 1 (RETURN_POWERON). An ADJ that opened
       before INSTALL stays native, and its native exit keeps the return. An
       exit from GRKit sets RETURN_GRKIT. */
    assert(exit_return(RETURN_POWERON, 0) == RETURN_POWERON);
    assert(exit_return(RETURN_POWERON, 1) == RETURN_GRKIT);
    /* Only a change that opens ADJ turns it into RETURN_GRKIT, so that the
       ADJ refresh opens GRKit. The return to an open ADJ after a half-press
       (CONTROLLER_BACK) and other screens keep it. */
    test_header[GRKIT_HOST_RETURN / 4] = RETURN_POWERON;
    menu_settings_transition(0, SHOOTING_CONTROLLER, CONTROLLER_BACK);
    menu_settings_transition(0, ADJ_CONTROLLER, CONTROLLER_BACK);
    menu_settings_transition(0, 0x4000000, SHOOTING_CONTROLLER);
    assert(test_header[GRKIT_HOST_RETURN / 4] == RETURN_POWERON);
    menu_settings_transition(0, ADJ_CONTROLLER, SHOOTING_CONTROLLER);
    assert(test_header[GRKIT_HOST_RETURN / 4] == RETURN_GRKIT);
    assert(controller_return(RETURN_GRKIT, ADJ_CONTROLLER, SHOOTING_CONTROLLER) == RETURN_GRKIT);
    assert(controller_return(RETURN_PARAMETERS, ADJ_CONTROLLER, SHOOTING_CONTROLLER) == RETURN_PARAMETERS);
    test_header[GRKIT_HOST_RETURN / 4] = 0;
}

static void a_half_press_on_the_native_slot_selects_its_type(void)
{
    reset();
    /* ADJ keeps the native type when it opens, also when it returns after
       a half-press. Other screens do not change it. */
    native_ic_value = 12;
    menu_settings_transition(0, ADJ_CONTROLLER, SHOOTING_CONTROLLER);
    assert(test_header[GRKIT_HOST_ADJ_IC_TYPE / 4] == 12);
    native_ic_value = 3;
    menu_settings_transition(0, SHOOTING_CONTROLLER, CONTROLLER_BACK);
    assert(test_header[GRKIT_HOST_ADJ_IC_TYPE / 4] == 12);
    menu_settings_transition(0, ADJ_CONTROLLER, CONTROLLER_BACK);
    assert(test_header[GRKIT_HOST_ADJ_IC_TYPE / 4] == 3);
    native_ic_value = 0;
    /* A half-press on the native Image Control slot with another type
       selects it, as the native slot keeps the type under its cursor. */
    assert(native_ic_chosen(12, 3, 1, 1));
    /* The same type, which the slot writes again when it is selected, keeps
       the GRKit effect. */
    assert(!native_ic_chosen(12, 12, 1, 1));
    /* Another slot, a screen other than the shooting screen (playback, a
       mode change), or a type that was not readable keeps it. */
    assert(!native_ic_chosen(12, 3, 0, 1));
    assert(!native_ic_chosen(12, 3, 1, 0));
    assert(!native_ic_chosen(0, 3, 1, 1));
    assert(!native_ic_chosen(12, 0, 1, 1));
}

static void the_root_lists_the_official_modules(void)
{
    reset();
    /* The root lists the five official modules. Module management and the
       version are in the native MENU. Without effects, the page of Image
       Control does not show. */
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = 1;
    assert(count(test_header) == 5);
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = 2;
    assert(count(test_header) == 6);
    test_header[GRKIT_HOST_COUNT / 4] = 0;
    assert(count(test_header) == 1);
}

static void module_tiles_follow_the_effect_icons(void)
{
    reset();
    /* Registry routing leaves the existing effect record untouched. */
    test_header[GRKIT_HOST_ACTIVE / 4] = 0x59005000;
    test_header[GRKIT_HOST_COUNT / 4] = 5;
    /* Module tiles follow the effect icons in the chunks: slot
       GRKIT_HOST_ICON_COUNT + effects + n. */
    test_header[GRKIT_HOST_ICON_CHUNKS / 4] = 0x59100000;
    test_header[GRKIT_HOST_ICON_CHUNKS / 4 + 1] = 0x59200000;
    {
        u32 first = GRKIT_HOST_ICON_COUNT + 5;
        assert(menu_icon(GRKIT_ICON_MODULE_TILE(0)) == 0x59100000 + first * GRKIT_ICON_STRIDE);
        assert(menu_icon(GRKIT_ICON_MODULE_TILE(40 - first)) == 0x59200000);
        assert(menu_icon(GRKIT_ICON_MODULE_TILE(41 - first)) == 0x59200000 + GRKIT_ICON_STRIDE);
        /* A chunk that the installer did not allocate. */
        assert(menu_icon(GRKIT_ICON_MODULE_TILE(80 - first)) == 0);
        assert(menu_icon(GRKIT_ICON_MODULE_TILE(GRKIT_MODULE_TILES_MAX)) == 0);
    }
    /* The Modules tile follows the Version tile, and the icon IDs below it
       are not tiles. */
    assert(menu_icon(GRKIT_ICON_MODULES) == test_header[GRKIT_HOST_GRKIT_ICON / 4] + 2 * GRKIT_ICON_STRIDE);
    assert(menu_icon(GRKIT_ICON_MODULES_OFF) == test_header[GRKIT_HOST_GRKIT_ICON / 4] + 3 * GRKIT_ICON_STRIDE);
    assert(menu_icon(GRKIT_ICON_ROTATE) == test_header[GRKIT_HOST_GRKIT_ICON / 4] + 4 * GRKIT_ICON_STRIDE);
    assert(menu_icon(GRKIT_ICON_ROTATE - 1) == 0);
}

/* The ADJ page blocks start with record 0 committed, as ADJ has it, and
   Framing at `ratio`. */
/* ADJ controllers of the preview tests: their ADJ_CANVAS words name the
   canvases 0x5a000000, 0x5b000000, and 0x5c000000. */
#define ADJ_A (TEST_OBJECTS + 0x3800U)
#define ADJ_B (TEST_OBJECTS + 0x3900U)
#define ADJ_C (TEST_OBJECTS + 0x3a00U)
static void adj_controllers(void)
{
    object_memory[(0x3800U + ADJ_CANVAS) / 4] = 0x5a000000U;
    object_memory[(0x3900U + ADJ_CANVAS) / 4] = 0x5b000000U;
    object_memory[(0x3a00U + ADJ_CANVAS) / 4] = 0x5c000000U;
}
static void adj_pages_start(u32 ratio)
{
    reset();
    adj_controllers();
    test_header[GRKIT_HOST_ACTIVE / 4] = 0x59005000;
    test_header[GRKIT_HOST_FRAME_RATIO / 4] = ratio;
}

static void the_framing_page_sets_the_ratios_of_the_target(void)
{
    adj_pages_start(0);
    /* The Framing page of CORE.ASPECT: its root row shows the ratio that the
       camera applies. */
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_ROOT;
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = FRAMING_ROW;
    assert(root_module(test_header, FRAMING_ROW) == FRAMING_MODULE);
    assert(same(current_module(test_header)->summary(test_header), "Framing: Off"));
    assert(PAGE_ICON(FRAMING_MODULE, FRAMING_ROW) == FRAMING_TILE(0));
    test_header[GRKIT_HOST_FRAME_RATIO / 4] = CROP_RATIO;
    assert(PAGE_ICON(FRAMING_MODULE, FRAMING_ROW) == FRAMING_TILE(CROP_RATIO));
    /* A ratio that the target does not offer reads as Off. */
    test_header[GRKIT_HOST_FRAME_RATIO / 4] = UNKNOWN_RATIO;
    assert(PAGE_ICON(FRAMING_MODULE, FRAMING_ROW) == FRAMING_TILE(0));
    /* Without a module that uses CORE.ASPECT, a stored crop is no crop. */
    test_header[GRKIT_HOST_FRAME_RATIO / 4] = CROP_RATIO;
    test_header[GRKIT_HOST_CAPABILITIES / 4] &= ~(1U << (GRKIT_CORE_ASPECT - GRKIT_CORE_SETTING_BASE));
    assert(setting_value(test_header, GRKIT_CORE_ASPECT) == 0 && framing_crop(test_header) == 0);
    test_header[GRKIT_HOST_CAPABILITIES / 4] |= 1U << (GRKIT_CORE_ASPECT - GRKIT_CORE_SETTING_BASE);
    /* Framing lists the ratios of the target in its row order and opens at
       the row of the selected one. */
    menu_confirm(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_MODULE && test_header[GRKIT_HOST_MODULE / 4] == FRAMING_MODULE);
    assert(test_header[GRKIT_HOST_MENU_CURSOR / 4] == CROP_ROW);
    assert(count(test_header) == ROW_COUNT && ROW_COUNT == 8);
    assert(same(current_module(test_header)->title(test_header), "Framing"));
    for (u32 row = 0; row < ROW_COUNT; ++row) {
        u32 value = ratio_rows[row];
        u32 requests = framing_requests, sets = native_sets, stops = live_view_stops;
        test_header[GRKIT_HOST_MENU_CURSOR / 4] = row;
        menu_confirm(test_header);
        assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_MODULE);
        /* Each ratio, also one that the camera has, is a crop of native 3:2.
           Off clears the crop. Each choice sends one request. */
        assert(test_header[GRKIT_HOST_FRAME_RATIO / 4] == value && native_aspect_value == 0);
        assert(native_sets == sets && framing_requests == requests + 1);
        /* Each native aspect change stops the live view first. */
        assert(live_view_stops - stops == native_sets - sets);
        assert(setting_value(test_header, GRKIT_CORE_ASPECT) == value);
        assert(current_module(test_header)->checked(test_header, row));
        assert(!current_module(test_header)->checked(test_header, (row + 1) % ROW_COUNT));
        assert(same(current_module(test_header)->label(test_header, row), ratio_names[value]));
        assert(CURRENT_ICON(row) == (row ? FRAMING_TILE(value) : 0x7fff));
        assert(same(current_module(test_header)->summary(test_header), ratio_summaries[value]));
        assert(test_header[GRKIT_HOST_ACTIVE / 4] == 0x59005000);
    }
    /* 1:1 is the last row, and a crop. */
    assert(ratio_rows[ROW_COUNT - 1] == 5 && test_header[GRKIT_HOST_FRAME_RATIO / 4] == 5);
    /* A native aspect from the camera menu is not a Framing ratio, so the
       page shows Off. A crop applies only in native 3:2, except XPan and
       2.35:1 in GR IV native 16:9. */
    test_header[GRKIT_HOST_FRAME_RATIO / 4] = CROP_RATIO;
    native_aspect_value = 3;
    assert(current_module(test_header)->committed(test_header) == 0);
#ifdef FRAMING_XPAN_NATIVE
    native_aspect_value = FRAMING_XPAN_NATIVE;
    assert(current_module(test_header)->committed(test_header) == CROP_ROW);
    test_header[GRKIT_HOST_FRAME_RATIO / 4] = 6;
    assert(current_module(test_header)->committed(test_header) == 0);
    native_aspect_value = 1;
    assert(current_module(test_header)->committed(test_header) == 0);
#endif
    native_aspect_value = 0;
    /* Off clears the GRKit ratio without changing the native aspect. */
    {
        u32 sets = native_sets;
        native_aspect_value = 3;
        test_header[GRKIT_HOST_MENU_CURSOR / 4] = 0;
        menu_confirm(test_header);
        assert(native_sets == sets && native_aspect_value == 3);
        assert(test_header[GRKIT_HOST_FRAME_RATIO / 4] == 0);
    }
    native_aspect_value = 0;
    /* A saved value that the target does not offer reads as Off. */
    test_header[GRKIT_HOST_FRAME_RATIO / 4] = UNKNOWN_RATIO;
    assert(current_module(test_header)->committed(test_header) == 0);
    /* A saved 1:1 is a crop. */
    test_header[GRKIT_HOST_FRAME_RATIO / 4] = 5;
    assert(current_module(test_header)->committed(test_header) == framing_row(5));
    /* A module sets only a value that the target offers. */
    setting_store(test_header, GRKIT_CORE_ASPECT, UNKNOWN_RATIO);
    assert(test_header[GRKIT_HOST_FRAME_RATIO / 4] == 5);
    test_header[GRKIT_HOST_FRAME_RATIO / 4] = CROP_RATIO;
    /* The Framing page shows its highlighted row on the ADJ canvas: the area
       of the ratio, and the ratio as the value of the CORE.ASPECT layers. */
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 1;
    preview_invalidations = 0;
    frame_preview(test_header, ADJ_A, 0);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW / 4] == 2);
    assert(test_header[GRKIT_HOST_MODULE_PREVIEW / 4] == GRKIT_CORE_ASPECT + 1 && test_header[GRKIT_HOST_MODULE_PREVIEW / 4 + 1] == 1);
    /* A new preview redraws the ADJ canvas once. The same preview does not. */
    assert(preview_invalidations == 1 && preview_invalidated == 0x5a000000);
    frame_preview(test_header, ADJ_A, 0);
    assert(preview_invalidations == 1);
    /* The preview holds the value of the row: row 2 is 2.35:1 (value 4). */
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 2;
    frame_preview(test_header, ADJ_A, 0);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW / 4] == 5 && test_header[GRKIT_HOST_MODULE_PREVIEW / 4 + 1] == 4);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW_ADJ / 4] == ADJ_A);
    assert(preview_invalidations == 2);
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 0;
    frame_preview(test_header, ADJ_A, 0);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW / 4] == 1);
    assert(preview_invalidations == 3);
    /* The movie ADJ has no preview and no redraw for it. */
    frame_preview(test_header, ADJ_A, 1);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW / 4] == 0);
    assert(preview_invalidations == 3);
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = CROP_ROW;
    assert(menu_back(test_header));
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT && test_header[GRKIT_HOST_MENU_CURSOR / 4] == FRAMING_ROW);
    /* Other ADJ pages show the committed crop. */
    frame_preview(test_header, ADJ_B, 0);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW / 4] == CROP_RATIO + 1);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW_ADJ / 4] == ADJ_B);
    assert(preview_invalidated == 0x5b000000);
    /* An ADJ that opens again, also after a native settings page, has drawn
       its canvas before the preview: the exit cleared the preview, so the
       canvas redraws with the crop. */
    {
        unsigned before = preview_invalidations;
        test_header[GRKIT_HOST_FRAME_PREVIEW / 4] = 0;
        frame_preview(test_header, ADJ_B, 0);
        assert(preview_invalidations == before + 1 && test_header[GRKIT_HOST_FRAME_PREVIEW / 4] == CROP_RATIO + 1);
    }
    /* A committed Off ends the mask but keeps the canvas for one erasing
       draw. The Guides layer is off too. */
    test_header[GRKIT_HOST_FRAME_RATIO / 4] = 0;
    frame_preview(test_header, ADJ_B, 0);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW / 4] == 0);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW_ADJ / 4] == ADJ_B);
    test_header[GRKIT_HOST_FRAME_RATIO / 4] = CROP_RATIO;
    test_header[GRKIT_HOST_FRAME_PREVIEW_ADJ / 4] = 0;
    assert(!menu_back(test_header));
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = FRAMING_ROW;
    assert(same(current_module(test_header)->summary(test_header), ratio_summaries[CROP_RATIO]));
}

static void framing_modes_publish_capture_geometry_before_the_next_frame(void)
{
    volatile u32 *h = test_header;
    reset();
    h[GRKIT_HOST_CAPABILITIES / 4] |= GRKIT_CORE_ASPECT_MODE_BIT;
    for (u32 row = 1; row < ROW_COUNT; ++row) {
        u32 ratio = ratio_rows[row], requests;
        setting_store(h, GRKIT_CORE_ASPECT, ratio);
        assert(requested_crop == ratio && h[GRKIT_HOST_FRAME_CROP / 4] == ratio);
        requests = framing_requests;
        setting_store(h, GRKIT_CORE_ASPECT_MODE, 1);
        assert(framing_requests == requests + 1 && requested_crop == 0);
        assert(h[GRKIT_HOST_FRAME_CROP / 4] == 0 && framing_crop(h) == 0);
        assert(h[GRKIT_HOST_FRAME_RATIO / 4] == ratio && setting_value(h, GRKIT_CORE_ASPECT) == ratio);
        /* No live-view bind occurs between the setting and these assertions. */
        setting_store(h, GRKIT_CORE_ASPECT_MODE, 0);
        assert(framing_requests == requests + 2 && requested_crop == ratio);
        assert(h[GRKIT_HOST_FRAME_CROP / 4] == ratio);
    }
#ifdef FRAMING_XPAN_NATIVE
    /* GR IV keeps capture id 2 while its geometry changes between XPan and 16:9. */
    setting_store(h, GRKIT_CORE_ASPECT, 1);
    native_aspect_value = FRAMING_XPAN_NATIVE;
    setting_store(h, GRKIT_CORE_ASPECT_MODE, 0);
    assert(native_aspect_value == FRAMING_XPAN_NATIVE && requested_crop == 1);
    setting_store(h, GRKIT_CORE_ASPECT_MODE, 1);
    assert(native_aspect_value == FRAMING_XPAN_NATIVE && requested_crop == 0);
#endif
}

static void selected_framing_ratios_start_from_native_three_two(void)
{
    volatile u32 *h = test_header;
    for (u32 mode = 0; mode < 2; ++mode) for (u32 native = 0; native < 4; ++native)
        for (u32 row = 1; row < ROW_COUNT; ++row) {
            u32 ratio = ratio_rows[row], crop = mode ? 0 : ratio;
            reset();
            h[GRKIT_HOST_CAPABILITIES / 4] |= GRKIT_CORE_ASPECT_MODE_BIT;
            native_aspect_value = native;
            /* Changing Mode alone preserves the native aspect. */
            setting_store(h, GRKIT_CORE_ASPECT_MODE, mode);
            assert(native_aspect_value == native && native_sets == 0 && live_view_stops == 0);
            h[GRKIT_HOST_FRAME_CROP / 4] = 999;
            setting_store(h, GRKIT_CORE_ASPECT, ratio);
            assert(native_aspect_value == 0 && h[GRKIT_HOST_FRAME_RATIO / 4] == ratio);
            assert(h[GRKIT_HOST_FRAME_CROP / 4] == crop && requested_crop == crop);
            assert(native_sets == (native != 0) && live_view_stops == native_sets);
            /* Geometry is ready before the native setter can request a
               capture change, not only before the following refresh. */
            if (native) assert(native_requested_crop == crop);
            assert(setting_value(h, GRKIT_CORE_ASPECT) == ratio);
            /* A later native choice survives Off in either mode. */
            native_aspect_value = native;
            u32 sets = native_sets, stops = live_view_stops, changes = h[GRKIT_HOST_STATE_CHANGES / 4];
            setting_store(h, GRKIT_CORE_ASPECT, 0);
            assert(native_aspect_value == native && native_sets == sets && live_view_stops == stops);
            assert(h[GRKIT_HOST_FRAME_RATIO / 4] == 0 && h[GRKIT_HOST_FRAME_CROP / 4] == 0 && requested_crop == 0);
            /* Clearing a selection hidden by the native aspect is saved. */
            assert(h[GRKIT_HOST_STATE_CHANGES / 4] == changes + 1);
        }
}

/* An ADJ exit that GRKit did not start, such as a half-press of the
   shutter, commits the highlighted row of a page, as native ADJ keeps the
   value under its cursor. An exit from the root commits nothing. */
static void an_exit_from_a_page_commits_the_highlighted_row(void)
{
    u32 value;
    adj_pages_start(0);
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_ROOT;
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = GUIDES_ROW;
    GUIDE_STYLE = 1;
    menu_confirm(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_MODULE && test_header[GRKIT_HOST_MODULE / 4] == GUIDES_MODULE);
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 3;
    value = choice_row_value(test_header, 3);
    assert(value != 1);
    exit_commit(test_header);
    assert(GUIDE_STYLE == value);
    /* From the root, the exit keeps the committed values. */
    GUIDE_STYLE = 1;
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_ROOT;
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = GUIDES_ROW;
    exit_commit(test_header);
    assert(GUIDE_STYLE == 1);
}

static void the_guides_page_commits_only_its_style(void)
{
    adj_pages_start(CROP_RATIO);
    /* The Guides module page opens at the committed style and commits only
       its own setting. */
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_ROOT;
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = GUIDES_ROW;
    assert(root_module(test_header, GUIDES_ROW) == GUIDES_MODULE);
    assert(same(current_module(test_header)->summary(test_header), "Guides: Off"));
    assert(PAGE_ICON(GUIDES_MODULE, GUIDES_ROW) == GUIDE_TILE(0));
    GUIDE_STYLE = 3;
    assert(PAGE_ICON(GUIDES_MODULE, GUIDES_ROW) == GUIDE_TILE(3));
    {
        u32 requests = framing_requests, changes = settings_memory[1];
        menu_confirm(test_header);
        assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_MODULE && test_header[GRKIT_HOST_MODULE / 4] == GUIDES_MODULE);
        assert(test_header[GRKIT_HOST_MENU_CURSOR / 4] == 3);
        assert(count(test_header) == 6);
        assert(same(current_module(test_header)->title(test_header), "Guides"));
        for (u32 row = 0; row < 6; ++row) {
            test_header[GRKIT_HOST_MENU_CURSOR / 4] = row;
            menu_confirm(test_header);
            assert(GUIDE_STYLE == row);
            assert(current_module(test_header)->checked(test_header, row));
            assert(!current_module(test_header)->checked(test_header, (row + 1) % 6));
            assert(same(current_module(test_header)->label(test_header, row), guide_names[row]));
            assert(CURRENT_ICON(row) == (row ? GUIDE_TILE(row) : 0x7fff));
            assert(test_header[GRKIT_HOST_FRAME_RATIO / 4] == CROP_RATIO);
            assert(test_header[GRKIT_HOST_ACTIVE / 4] == 0x59005000);
        }
        assert(framing_requests == requests);
        /* Each change counts once, for the resident loader. */
        assert(settings_memory[1] == changes + 6);
        /* OK on the committed row changes nothing. */
        menu_confirm(test_header);
        assert(settings_memory[1] == changes + 6);
    }
    /* The page previews its highlighted style inside the committed ratio. */
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 2;
    frame_preview(test_header, ADJ_A, 0);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW / 4] == CROP_RATIO + 1);
    assert(test_header[GRKIT_HOST_MODULE_PREVIEW / 4] == 1 && test_header[GRKIT_HOST_MODULE_PREVIEW / 4 + 1] == 2);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW_ADJ / 4] == ADJ_A);
    /* A highlighted Off with no ratio keeps the canvas for the next row. */
    test_header[GRKIT_HOST_FRAME_RATIO / 4] = 0;
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 0;
    GUIDE_STYLE = 0;
    frame_preview(test_header, ADJ_A, 0);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW / 4] == 1);
    assert(test_header[GRKIT_HOST_MODULE_PREVIEW / 4] == 1 && test_header[GRKIT_HOST_MODULE_PREVIEW / 4 + 1] == 0);
    /* The movie live view previews nothing. */
    frame_preview(test_header, ADJ_A, 1);
    assert(!test_header[GRKIT_HOST_FRAME_PREVIEW / 4] && !test_header[GRKIT_HOST_MODULE_PREVIEW / 4]);
    assert(menu_back(test_header));
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT && test_header[GRKIT_HOST_MENU_CURSOR / 4] == GUIDES_ROW);
    /* Other ADJ pages keep the committed style on the canvas, even without a
       ratio, and preview no module value. */
    GUIDE_STYLE = 4;
    frame_preview(test_header, ADJ_B, 0);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW / 4] == 1 && !test_header[GRKIT_HOST_MODULE_PREVIEW / 4]);
    /* The Framing page keeps the committed style with its highlighted ratio
       (row 1, XPan). */
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = FRAMING_ROW;
    menu_confirm(test_header);
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 1;
    frame_preview(test_header, ADJ_B, 0);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW / 4] == 2 && test_header[GRKIT_HOST_MODULE_PREVIEW / 4] == GRKIT_CORE_ASPECT + 1);
    /* Committed Off for both ends the drawing. */
    assert(menu_back(test_header));
    GUIDE_STYLE = 0;
    frame_preview(test_header, ADJ_B, 0);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW / 4] == 0);
    /* A value out of range of its setting reads as 0. */
    GUIDE_STYLE = 999;
    assert(setting_value(test_header, 0) == 0);
}

static void the_date_imprint_page_lists_its_styles(void)
{
    adj_pages_start(CROP_RATIO);
    /* Date Imprint lists its styles, opens at the committed one, and changes
       nothing else. */
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_ROOT;
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = DATE_ROW;
    assert(root_module(test_header, DATE_ROW) == DATE_MODULE);
    assert(same(current_module(test_header)->summary(test_header), "Date Imprint: Off"));
    assert(PAGE_ICON(DATE_MODULE, DATE_ROW) == DATE_TILE(0));
    DATE_STYLE = 2;
    assert(PAGE_ICON(DATE_MODULE, DATE_ROW) == DATE_TILE(2));
    menu_confirm(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_MODULE && test_header[GRKIT_HOST_MODULE / 4] == DATE_MODULE);
    assert(test_header[GRKIT_HOST_MENU_CURSOR / 4] == 2 && count(test_header) == 6);
    assert(same(current_module(test_header)->title(test_header), "Date Imprint"));
    for (u32 row = 0; row < 6; ++row) {
        test_header[GRKIT_HOST_MENU_CURSOR / 4] = row;
        menu_confirm(test_header);
        assert(DATE_STYLE == row);
        assert(current_module(test_header)->checked(test_header, row));
        assert(same(current_module(test_header)->label(test_header, row), stamp_names[row]));
        assert(CURRENT_ICON(row) == (row ? DATE_TILE(row) : 0x7fff));
        assert(test_header[GRKIT_HOST_FRAME_RATIO / 4] == CROP_RATIO && GUIDE_STYLE == 0 && DATE_LOOK == 0);
        assert(test_header[GRKIT_HOST_ACTIVE / 4] == 0x59005000);
    }
    /* The Date Imprint page keeps the committed mask on the ADJ canvas, and
       previews its highlighted style for its layers, which it has none of. */
    frame_preview(test_header, ADJ_C, 0);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW / 4] == CROP_RATIO + 1 && test_header[GRKIT_HOST_MODULE_PREVIEW / 4] == 2);
    assert(menu_back(test_header) && test_header[GRKIT_HOST_MENU_CURSOR / 4] == DATE_ROW);
    assert(same(current_module(test_header)->summary(test_header), "Date Imprint: D M 'YY H:MM"));
    DATE_STYLE = 999;
    assert(setting_value(test_header, 1) == 0);
}

static void the_grain_page_changes_only_the_grain_word(void)
{
    adj_pages_start(CROP_RATIO);
    /* The Grain page of the capability CORE.GRAIN lists its levels, opens at
       the committed one, and changes only the Grain word of the core. */
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_ROOT;
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = GRAIN_ROW;
    assert(root_module(test_header, GRAIN_ROW) == GRAIN_MODULE);
    assert(same(current_module(test_header)->summary(test_header), "Grain: Off"));
    assert(PAGE_ICON(GRAIN_MODULE, GRAIN_ROW) == GRAIN_TILE(0));
    test_header[GRKIT_HOST_GRAIN / 4] = 2;
    assert(PAGE_ICON(GRAIN_MODULE, GRAIN_ROW) == GRAIN_TILE(2));
    menu_confirm(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_MODULE && test_header[GRKIT_HOST_MODULE / 4] == GRAIN_MODULE);
    assert(test_header[GRKIT_HOST_MENU_CURSOR / 4] == 2 && count(test_header) == 5);
    assert(same(current_module(test_header)->title(test_header), "Grain"));
    const u32 guide_style = GUIDE_STYLE, module_changes = settings_memory[1];
    for (u32 row = 0; row < 5; ++row) {
        test_header[GRKIT_HOST_MENU_CURSOR / 4] = row;
        menu_confirm(test_header);
        assert(test_header[GRKIT_HOST_GRAIN / 4] == row);
        assert(current_module(test_header)->checked(test_header, row));
        assert(same(current_module(test_header)->label(test_header, row), grain_names[row]));
        assert(CURRENT_ICON(row) == (row ? GRAIN_TILE(row) : 0x7fff));
        assert(test_header[GRKIT_HOST_FRAME_RATIO / 4] == CROP_RATIO && DATE_STYLE == 0);
        assert(test_header[GRKIT_HOST_ACTIVE / 4] == 0x59005000);
        assert(GUIDE_STYLE == guide_style && settings_memory[1] == module_changes);
    }
    assert(menu_back(test_header) && test_header[GRKIT_HOST_MENU_CURSOR / 4] == GRAIN_ROW);
    assert(same(current_module(test_header)->summary(test_header), "Grain: Extra Strong"));
    /* A value out of range reads as Off, and a store out of range does nothing. */
    test_header[GRKIT_HOST_GRAIN / 4] = 999;
    assert(setting_value(test_header, GRKIT_CORE_GRAIN) == 0 && setting_limit(test_header, GRKIT_CORE_GRAIN) == 5);
    setting_store(test_header, GRKIT_CORE_GRAIN, 5);
    assert(test_header[GRKIT_HOST_GRAIN / 4] == 999);
    setting_store(test_header, GRKIT_CORE_GRAIN, 1);
    assert(test_header[GRKIT_HOST_GRAIN / 4] == 1);
    /* A handle after the capabilities is no setting. */
    assert(setting_limit(test_header, GRKIT_CORE_SETTING_BASE + GRKIT_CORE_SETTING_COUNT) == 0);
}

/* The page of CORE.RECIPE in place of Grain: Undo while an undo step
   exists, the camera recipes, then Save. The rows are actions: an ADJ exit
   applies none, and the page shows with Save alone. The recipes here hold
   Film Look Off and the grain, so no record is read. */
static void the_recipe_page_saves_applies_and_deletes(void)
{
    reset();
    install_page(4, "Recipes", 0, 0, GRKIT_CORE_RECIPE, 19);
    test_header[GRKIT_HOST_CAPABILITIES / 4] |= GRKIT_CORE_RECIPE_BIT;
    test_header[GRKIT_HOST_SERVICES / 4] = TEST_SERVICES;
    test_header[GRKIT_HOST_ACTIVE / 4] = 0;
    test_header[GRKIT_HOST_GRAIN / 4] = 0;
    assert(MODULE(GRAIN_MODULE).rows(test_header) == 1);
    assert(visible_modules(test_header) >> GRAIN_MODULE & 1);
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_ROOT;
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = GRAIN_ROW;
    assert(same(current_module(test_header)->summary(test_header), "Recipes"));
    assert(PAGE_ICON(GRAIN_MODULE, GRAIN_ROW) == GRAIN_TILE(0));
    menu_confirm(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_MODULE && test_header[GRKIT_HOST_MENU_CURSOR / 4] == 0);
    assert(same(current_module(test_header)->label(test_header, 0), "Save Settings"));
    assert(CURRENT_ICON(0) == GRAIN_TILE(2));
    /* OK on Save adds a recipe and puts the cursor on it. The second of a
       look gets a number. */
    menu_confirm(test_header);
    assert(recipe_count(test_header) == 1 && test_header[GRKIT_HOST_MENU_CURSOR / 4] == 0);
    test_header[GRKIT_HOST_GRAIN / 4] = 3;
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 1;
    menu_confirm(test_header);
    assert(recipe_count(test_header) == 2 && test_header[GRKIT_HOST_MENU_CURSOR / 4] == 1);
    assert(count(test_header) == 3);
    assert(same(current_module(test_header)->label(test_header, 0), "Film Look: Off"));
    assert(same(current_module(test_header)->label(test_header, 1), "Film Look: Off 2"));
    assert(CURRENT_ICON(0) == GRAIN_TILE(3));
    assert(!current_module(test_header)->checked(test_header, 0) && current_module(test_header)->checked(test_header, 1));
    assert(same(current_module(test_header)->summary(test_header), "Recipes: Film Look: Off 2"));
    /* OK on a recipe applies it and stores an undo step, which adds the Undo
       row first. */
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 0;
    menu_confirm(test_header);
    assert(test_header[GRKIT_HOST_GRAIN / 4] == 0 && recipe_undo_count(test_header) == 1);
    assert(count(test_header) == 4);
    assert(same(current_module(test_header)->label(test_header, 0), "Undo (1)"));
    assert(CURRENT_ICON(0) == GRAIN_TILE(1));
    assert(same(current_module(test_header)->label(test_header, 2), "Film Look: Off 2"));
    assert(current_module(test_header)->committed(test_header) == 1);
    /* An ADJ exit on a recipe applies nothing. */
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 2;
    exit_commit(test_header);
    assert(test_header[GRKIT_HOST_GRAIN / 4] == 0 && recipe_undo_count(test_header) == 1);
    /* Fn on a recipe shows its name with Rename and Delete; Fn on Undo or
       Save does nothing. Delete asks to delete it. */
    assert(current_module(test_header)->fn(test_header, 2));
    assert(same((const u16 *)recipe_text_at(test_header, RECIPE_BOX_AT), "Film Look: Off 2"));
    assert(same(text(test_header, RECIPE_TEXT_RENAME), "Rename") && same(text(test_header, RECIPE_TEXT_REMOVE), "Delete"));
    assert(!current_module(test_header)->fn(test_header, 0) && !current_module(test_header)->fn(test_header, 3));
    recipe_action_done(2, 1);
    assert(same((const u16 *)recipe_text_at(test_header, RECIPE_BOX_AT), "Film Look: Off 2"));
    recipe_action_done(1, 1);
    assert(same((const u16 *)recipe_text_at(test_header, RECIPE_BOX_AT), "Delete Film Look: Off 2?"));
    /* A typed name replaces the automatic name in the label, the summary,
       and the texts of the boxes. */
    recipe_name_set(test_header, 1, (const volatile u8 *)"  Street  Nega  ");
    assert(same(current_module(test_header)->label(test_header, 2), "Street  Nega"));
    recipe_action_done(1, 1);
    assert(same((const u16 *)recipe_text_at(test_header, RECIPE_BOX_AT), "Delete Street  Nega?"));
    recipe_name_set(test_header, 1, (const volatile u8 *)"   ");
    assert(same(current_module(test_header)->label(test_header, 2), "Film Look: Off 2"));
    recipe_name_set(test_header, 1, (const volatile u8 *)"Street Nega");
    recipe_delete_done(0, 1);
    assert(recipe_count(test_header) == 2);
    /* The name of a deleted recipe goes with it, and the names after it
       move up. */
    recipe_name_set(test_header, 0, (const volatile u8 *)"First");
    recipe_delete_done(1, 1);
    assert(recipe_count(test_header) == 1 && count(test_header) == 3);
    assert(test_header[GRKIT_HOST_MENU_CURSOR / 4] == 2);
    assert(same(current_module(test_header)->label(test_header, 1), "First"));
    assert(!recipe_name_length(test_header, 1) && !test_header[GRKIT_HOST_RECIPE_NAMES / 4 + GRKIT_RECIPE_NAME_WORDS]);
    recipe_delete_done(1, 0);
    assert(!recipe_count(test_header) && !recipe_name_length(test_header, 0));
    menu_confirm(test_header);
    assert(recipe_count(test_header) == 1 && !recipe_name_length(test_header, 0));
    /* OK on Undo keeps ADJ open, and on a recipe closes it. */
    assert(current_module(test_header)->stays(test_header, 0));
    assert(!current_module(test_header)->stays(test_header, 1) && !current_module(test_header)->stays(test_header, 2));
    /* OK on Undo restores the grain before the apply. The last undo step
       takes the Undo row away, and the cursor moves to the page's row. */
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 0;
    menu_confirm(test_header);
    assert(test_header[GRKIT_HOST_GRAIN / 4] == 3 && recipe_undo_count(test_header) == 0);
    assert(count(test_header) == 2);
    assert(test_header[GRKIT_HOST_MENU_CURSOR / 4] == current_module(test_header)->committed(test_header));
    /* With undo steps left, the cursor stays on Undo for the next one. */
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 0;
    menu_confirm(test_header);
    assert(recipe_undo_count(test_header) == 1 && test_header[GRKIT_HOST_MENU_CURSOR / 4] == 1);
    menu_confirm(test_header);
    assert(recipe_undo_count(test_header) == 2);
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 0;
    menu_confirm(test_header);
    assert(recipe_undo_count(test_header) == 1 && test_header[GRKIT_HOST_MENU_CURSOR / 4] == 0);
    menu_confirm(test_header);
    assert(recipe_undo_count(test_header) == 0);
    assert(test_header[GRKIT_HOST_MENU_CURSOR / 4] == current_module(test_header)->committed(test_header));
    /* CORE.RECIPE has no value, and a store does nothing. */
    assert(setting_value(test_header, GRKIT_CORE_RECIPE) == 0 && setting_limit(test_header, GRKIT_CORE_RECIPE) == 0);
    setting_store(test_header, GRKIT_CORE_RECIPE, 0);
    assert(recipe_count(test_header) == 1);
}

/* Recipe i in the header (runtime/recipes.h), with its name. */
static void header_recipe(u32 i, const char *name, u32 first)
{
    volatile u32 *w = test_header + GRKIT_HOST_RECIPES / 4 + i * GRKIT_RECIPE_WORDS;
    volatile u32 *n = test_header + GRKIT_HOST_RECIPE_NAMES / 4 + i * GRKIT_RECIPE_NAME_WORDS;
    for (u32 k = 0; k < GRKIT_RECIPE_WORDS; ++k) w[k] = 0;
    for (u32 k = 0; k < GRKIT_RECIPE_NAME_WORDS; ++k) n[k] = 0;
    w[0] = first;
    for (u32 c = 0; name[c] && c < GRKIT_RECIPE_NAME_CHARS; ++c) n[c / 4] |= (u32)name[c] << (7U * (c % 4));
}

/* The keyboard of the camera (runtime/ui_keyboard.h): its hooks serve the GRKit text
   only while GRKit borrows the keyboard, so the copyright text never meets
   a GRKit text. OK gives the typed text to the call after the return, which
   names the recipe. */
static void the_keyboard_serves_the_grkit_text_only_while_borrowed(void)
{
    const u32 keyboard = TEST_OBJECTS + 0x100U, buffer = TEST_OBJECTS + 0x200U;
    volatile u8 *grkit;
    reset();
    install_page(4, "Recipes", 0, 0, GRKIT_CORE_RECIPE, 19);
    test_header[GRKIT_HOST_CAPABILITIES / 4] |= GRKIT_CORE_RECIPE_BIT;
    test_header[GRKIT_HOST_SERVICES / 4] = TEST_SERVICES;
    grkit = (volatile u8 *)ui_word(test_header, UI_KEYBOARD_TEXT);
    BYTE(keyboard, KEYBOARD_FUNCTION) = KEYBOARD_COPYRIGHT;
    for (u32 i = 0; i < 6; ++i) BYTE(buffer, i) = (u8)"Owner"[i];
    /* Not borrowed: the native getter, setter, and length run. */
    assert(!menu_keyboard_text_in(0, buffer) && !menu_keyboard_text_out(0, buffer) && !menu_keyboard_limit(keyboard));
    assert(BYTE(buffer, 0) == 'O' && !ui_keyboard_result(test_header));
    /* Borrowed: the keyboard shows the GRKit text, with at most 16
       characters, and only for its copyright use. */
    ui_word(test_header, UI_KEYBOARD)[0] = UI_KEYBOARD_ARMED;
    strcpy((char *)grkit, "Nega");
    assert(menu_keyboard_text_in(0, buffer) && BYTE(buffer, 0) == 'N' && BYTE(buffer, 4) == 0);
    assert(ui_word(test_header, UI_KEYBOARD)[0] == UI_KEYBOARD_SHOWN);
    assert(menu_keyboard_limit(keyboard) == GRKIT_RECIPE_NAME_CHARS);
    BYTE(keyboard, KEYBOARD_FUNCTION) = KEYBOARD_COPYRIGHT + 1U;
    assert(!menu_keyboard_limit(keyboard));
    BYTE(keyboard, KEYBOARD_FUNCTION) = KEYBOARD_COPYRIGHT;
    /* OK: GRKit takes the text, and the borrow ends. */
    for (u32 i = 0; i < 9; ++i) BYTE(buffer, i) = (u8)"Nega Two"[i];
    assert(menu_keyboard_text_out(0, buffer));
    assert(!ui_word(test_header, UI_KEYBOARD)[0] && ui_keyboard_result(test_header));
    assert(!strcmp((const char *)ui_keyboard_result(test_header), "Nega Two"));
    assert(!menu_keyboard_text_out(0, buffer) && !menu_keyboard_text_in(0, buffer) && !menu_keyboard_limit(keyboard));
    /* The call after the return names the new recipe. */
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_ROOT;
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = GRAIN_ROW;
    menu_confirm(test_header);
    menu_confirm(test_header);
    assert(recipe_count(test_header) == 1);
    recipe_saved_back(0);
    assert(same(current_module(test_header)->label(test_header, 0), "Nega Two"));
    /* MENU on the keyboard keeps the name. */
    ui_word(test_header, UI_KEYBOARD_RESULT)[0] = 0;
    recipe_renamed_back(0);
    assert(same(current_module(test_header)->label(test_header, 0), "Nega Two"));
    strcpy((char *)grkit, "Rio");
    ui_word(test_header, UI_KEYBOARD_RESULT)[0] = 1;
    recipe_renamed_back(0);
    assert(same(current_module(test_header)->label(test_header, 0), "Rio"));
    /* A change to another controller ends a borrow; the change to the
       keyboard keeps it. */
    ui_word(test_header, UI_KEYBOARD)[0] = UI_KEYBOARD_SHOWN;
    ui_keyboard_change(test_header, KEYBOARD_CONTROLLER);
    assert(ui_word(test_header, UI_KEYBOARD)[0] == UI_KEYBOARD_SHOWN);
    ui_keyboard_change(test_header, KEYBOARD_CONTROLLER + 1U);
    assert(!ui_word(test_header, UI_KEYBOARD)[0] && !menu_keyboard_text_in(0, buffer));
}


/* OK on the keyboard with the text that it started with keeps the automatic
   name of a recipe, so the name follows its look and is not cut to the
   keyboard length. A changed text names the recipe, in ADJ and in MENU. */
static void the_keyboard_keeps_an_unchanged_automatic_name(void)
{
    volatile u8 *grkit;
    reset();
    install_page(4, "Recipes", 0, 0, GRKIT_CORE_RECIPE, 19);
    test_header[GRKIT_HOST_CAPABILITIES / 4] |= GRKIT_CORE_RECIPE_BIT;
    test_header[GRKIT_HOST_SERVICES / 4] = TEST_SERVICES;
    grkit = (volatile u8 *)ui_word(test_header, UI_KEYBOARD_TEXT);
    header_recipe(0, "", RECIPE_USED | RECIPE_LOOK_OFF | 12U << RECIPE_NUMBER_SHIFT);
    header_recipe(1, "", RECIPE_USED | RECIPE_LOOK_OFF | 2U << RECIPE_NUMBER_SHIFT);
    /* The keyboard starts with the automatic name, cut to its length. */
    ui_keyboard_text(test_header, recipe_keyboard_text(test_header, 0));
    assert(!strcmp((const char *)grkit, "Film Look: Off 1"));
    /* OK without a change: after Save and Rename in ADJ, and in MENU. */
    ui_word(test_header, UI_KEYBOARD_RESULT)[0] = 1;
    recipe_saved_back(0);
    recipe_renamed_back(0);
    recipe_named(0);
    assert(!recipe_name_length(test_header, 0));
    assert(same(menu_recipe_name(test_header, 0), "Film Look: Off 12"));
    ui_keyboard_text(test_header, recipe_keyboard_text(test_header, 1));
    assert(!strcmp((const char *)grkit, "Film Look: Off 2"));
    ui_word(test_header, UI_KEYBOARD_RESULT)[0] = 1;
    recipe_named(1);
    assert(!recipe_name_length(test_header, 1));
    /* MENU on the keyboard gives no text. */
    ui_word(test_header, UI_KEYBOARD_RESULT)[0] = 0;
    strcpy((char *)grkit, "Other");
    recipe_named(1);
    assert(!recipe_name_length(test_header, 1));
    /* A changed text names the recipe; a shorter one too. */
    ui_word(test_header, UI_KEYBOARD_RESULT)[0] = 1;
    recipe_named(1);
    assert(same(menu_recipe_name(test_header, 1), "Other"));
    strcpy((char *)grkit, "Film Look: Off");
    recipe_saved_back(0);
    assert(same(menu_recipe_name(test_header, 0), "Film Look: Off") && recipe_name_length(test_header, 0) == 14);
    /* A recipe with a typed name keeps the text as its name. */
    ui_keyboard_text(test_header, recipe_keyboard_text(test_header, 1));
    ui_word(test_header, UI_KEYBOARD_RESULT)[0] = 1;
    recipe_renamed_back(1);
    assert(same(menu_recipe_name(test_header, 1), "Other") && recipe_name_length(test_header, 1) == 5);
}

/* The page lists the recipes that show in ADJ, by their names. OK on one
   applies it, and Fn offers Rename and Delete. */
static void the_recipe_page_lists_the_shown_recipes(void)
{
    reset();
    install_page(4, "Recipes", 0, 0, GRKIT_CORE_RECIPE, 19);
    test_header[GRKIT_HOST_CAPABILITIES / 4] |= GRKIT_CORE_RECIPE_BIT;
    test_header[GRKIT_HOST_SERVICES / 4] = TEST_SERVICES;
    test_header[GRKIT_HOST_ACTIVE / 4] = 0;
    test_header[GRKIT_HOST_GRAIN / 4] = 3;
    /* Film Look Off and Grain 0; a hidden one. */
    header_recipe(0, "Far Away", RECIPE_USED | RECIPE_HIDDEN | RECIPE_LOOK_OFF);
    header_recipe(1, "Street", RECIPE_USED | RECIPE_LOOK_OFF | RECIPE_GRAIN);
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_ROOT;
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = GRAIN_ROW;
    menu_confirm(test_header);
    assert(count(test_header) == 2);
    assert(same(current_module(test_header)->label(test_header, 0), "Street"));
    assert(same(current_module(test_header)->label(test_header, 1), "Save Settings"));
    /* Save goes after the recipes, and the cursor shows it. */
    u32 recipes = test_header[GRKIT_HOST_RECIPE_CHANGES / 4];
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 1;
    menu_confirm(test_header);
    assert(recipe_count(test_header) == 3 && test_header[GRKIT_HOST_MENU_CURSOR / 4] == 1);
    assert(test_header[GRKIT_HOST_RECIPE_CHANGES / 4] == recipes + 1);
    assert(current_module(test_header)->checked(test_header, 1) && !current_module(test_header)->checked(test_header, 0));
    /* Street: applied. An apply saves no recipe. */
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 0;
    menu_confirm(test_header);
    assert(test_header[GRKIT_HOST_GRAIN / 4] == 0 && recipe_undo_count(test_header) == 1);
    assert(test_header[GRKIT_HOST_RECIPE_CHANGES / 4] == recipes + 1);
    /* The Undo row came first, and the cursor stays on the recipe. */
    assert(test_header[GRKIT_HOST_MENU_CURSOR / 4] == 1);
    assert(same(current_module(test_header)->summary(test_header), "Recipes: Street"));
    /* Undo, Street, the saved recipe, Save. */
    assert(count(test_header) == 4 && current_module(test_header)->committed(test_header) == 1);
}

/* A recipe with a name whose effect is not installed shows a mark after its
   name, before OK tells why it does not apply. */
static void the_recipe_page_marks_a_missing_effect(void)
{
    reset();
    install_page(4, "Recipes", 0, 0, GRKIT_CORE_RECIPE, 19);
    test_header[GRKIT_HOST_CAPABILITIES / 4] |= GRKIT_CORE_RECIPE_BIT;
    test_header[GRKIT_HOST_SERVICES / 4] = TEST_SERVICES;
    /* Film Look does not run, so the effect is missing. */
    test_header[GRKIT_HOST_CAPABILITIES / 4] &= ~(u32)GRKIT_CORE_TONE_BIT;
    header_recipe(0, "Street", RECIPE_USED | RECIPE_LOOK_OFF);
    header_recipe(1, "Gone", RECIPE_USED | RECIPE_LOOK_EFFECT | 0x7777U << RECIPE_CODE_SHIFT);
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_ROOT;
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = GRAIN_ROW;
    menu_confirm(test_header);
    assert(same(current_module(test_header)->label(test_header, 0), "Street"));
    assert(same(current_module(test_header)->label(test_header, 1), "Gone (effect missing)"));
}

/* A recipe of a native type shows the native Image Control icon of the type
   (NATIVE_TYPE_ICONS). A type without an icon on the camera, or a type
   after the table, and Film Look Off show the recipe tile. */
static void the_recipe_page_shows_native_type_icons(void)
{
    static const u16 icons[NATIVE_TYPES] = {NATIVE_TYPE_ICONS};
    u32 missing = NATIVE_TYPES;
    reset();
    install_page(4, "Recipes", 0, 0, GRKIT_CORE_RECIPE, 19);
    test_header[GRKIT_HOST_CAPABILITIES / 4] |= GRKIT_CORE_RECIPE_BIT;
    test_header[GRKIT_HOST_SERVICES / 4] = TEST_SERVICES;
    test_header[GRKIT_HOST_ACTIVE / 4] = 0;
    /* GR III has no Cinema types, GR IV 1.11 no Solarization; GR IV 1.20
       has an icon for each type. */
    for (u32 t = 0; t < NATIVE_TYPES; ++t)
        if (!icons[t] && missing == NATIVE_TYPES) missing = t;
    header_recipe(0, "Standard", RECIPE_USED);
    header_recipe(1, "Missing", RECIPE_USED);
    header_recipe(2, "Off", RECIPE_USED | RECIPE_LOOK_OFF);
    for (u32 i = 0; i < 2; ++i)
        test_header[GRKIT_HOST_RECIPES / 4 + i * GRKIT_RECIPE_WORDS + RECIPE_NATIVE_WORD] =
            ((i ? missing : 0U) + 1U) << NATIVE_TYPE_SHIFT;
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_ROOT;
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = GRAIN_ROW;
    menu_confirm(test_header);
    assert(count(test_header) == 4);
    assert(icons[0] && icons[0] < 0x7000U);
    assert(CURRENT_ICON(0) == icons[0]);
    assert(CURRENT_ICON(1) == GRAIN_TILE(3));
    assert(CURRENT_ICON(2) == GRAIN_TILE(3));
    assert(CURRENT_ICON(3) == GRAIN_TILE(2));
}

static void invalid_cursors_form_no_effect_pointer(void)
{
    adj_pages_start(CROP_RATIO);
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_ROOT;
    /* Invalid public navigation cursors cannot form an effect pointer. */
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 999;
    menu_confirm(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_MODULE && test_header[GRKIT_HOST_MODULE / 4] == 0);
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 999;
    menu_confirm(test_header);
    assert(test_header[GRKIT_HOST_ACTIVE / 4] == 0);
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 5;
    menu_confirm(test_header);
    assert(test_header[GRKIT_HOST_ACTIVE / 4] == 0x59005000 + 4 * GRKIT_RECORD_SIZE);
    assert(test_header[GRKIT_HOST_FRAME_RATIO / 4] == CROP_RATIO);
    assert(menu_back(test_header));
    assert(test_header[GRKIT_HOST_MENU_CURSOR / 4] == 0);
    test_header[GRKIT_HOST_MODULE / 4] = 999;
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_MODULE + 1;
    menu_normalize(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT && test_header[GRKIT_HOST_MODULE / 4] == 0);
    test_header[GRKIT_HOST_FRAME_RATIO / 4] = 999;
    assert(setting_value(test_header, GRKIT_CORE_ASPECT) == 0);
    /* Pages start at multiples of PAGE for every supported catalog row. */
    for (u32 row = 0; row <= 146; ++row) assert(page_start(row) == row / PAGE * PAGE);
}

static void film_look_previews_and_commits_the_highlighted_effect(void)
{
    reset();
    /* Film Look opens at the committed row. */
    test_header[GRKIT_HOST_PREVIEW / 4] = 0;
    test_header[GRKIT_HOST_ACTIVE / 4] = 0x59005000 + 3 * GRKIT_RECORD_SIZE;
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_ROOT;
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 0;
    menu_confirm(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_MODULE && test_header[GRKIT_HOST_MODULE / 4] == 0);
    assert(test_header[GRKIT_HOST_MENU_CURSOR / 4] == 4);
    assert(test_header[GRKIT_HOST_PREVIEW / 4] == 0);
    /* Highlighting previews in the live view without committing. Off previews native. */
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 2;
    highlight(test_header);
    assert(test_header[GRKIT_HOST_PREVIEW / 4] == 0x59005000 + GRKIT_RECORD_SIZE);
    assert(test_header[GRKIT_HOST_ACTIVE / 4] == 0x59005000 + 3 * GRKIT_RECORD_SIZE);
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 0;
    highlight(test_header);
    assert(test_header[GRKIT_HOST_PREVIEW / 4] == 1);
    assert(test_header[GRKIT_HOST_ACTIVE / 4] == 0x59005000 + 3 * GRKIT_RECORD_SIZE);
    /* MENU restores the committed effect. */
    assert(menu_back(test_header));
    assert(test_header[GRKIT_HOST_PREVIEW / 4] == 0);
    assert(test_header[GRKIT_HOST_ACTIVE / 4] == 0x59005000 + 3 * GRKIT_RECORD_SIZE);
    /* OK commits the highlighted row and ends the preview. */
    menu_confirm(test_header);
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 1;
    highlight(test_header);
    assert(test_header[GRKIT_HOST_PREVIEW / 4] == 0x59005000);
    menu_confirm(test_header);
    assert(test_header[GRKIT_HOST_ACTIVE / 4] == 0x59005000);
    assert(test_header[GRKIT_HOST_PREVIEW / 4] == 0);
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 0;
    highlight(test_header);
    menu_confirm(test_header);
    assert(test_header[GRKIT_HOST_ACTIVE / 4] == 0);
    assert(test_header[GRKIT_HOST_PREVIEW / 4] == 0);
    /* Off opens Film Look at the Off row. */
    assert(menu_back(test_header));
    menu_confirm(test_header);
    assert(test_header[GRKIT_HOST_MENU_CURSOR / 4] == 0);
    /* Other levels and modules do not preview effects. */
    assert(menu_back(test_header));
    highlight(test_header);
    assert(test_header[GRKIT_HOST_PREVIEW / 4] == 0);
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 1;
    menu_confirm(test_header);
    highlight(test_header);
    assert(test_header[GRKIT_HOST_PREVIEW / 4] == 0);
}

static void the_root_has_no_about_row(void)
{
    reset();
    /* With module pages, the root has no About row: the version is in the
       native MENU. */
    test_header[GRKIT_HOST_COUNT / 4] = 5;
    test_header[GRKIT_HOST_PREVIEW / 4] = 0;
    menu_start(test_header);
    assert(count(test_header) == 5 && !(visible_modules(test_header) >> ABOUT_MODULE & 1));
    assert(same(MODULE(ABOUT_MODULE).summary(test_header), "Version 0.0.0"));
    /* An About page cannot stay open: it has no rows. */
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_MODULE;
    test_header[GRKIT_HOST_MODULE / 4] = ABOUT_MODULE;
    menu_normalize(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT && test_header[GRKIT_HOST_MENU_CURSOR / 4] == 0);
}

static void a_page_past_the_page_count_has_no_root_row(void)
{
    reset();
    /* A module page after GRKIT_HOST_MODULE_PAGE_COUNT has no root row. The
       others keep their order. */
    test_header[GRKIT_HOST_PREVIEW / 4] = 0;
    test_header[GRKIT_HOST_FRAME_RATIO / 4] = 0;
    test_header[GRKIT_HOST_MODULE_PAGE_COUNT / 4] = 3;
    menu_start(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT && test_header[GRKIT_HOST_MENU_CURSOR / 4] == 0 && count(test_header) == 3);
    assert(current_module(test_header) == &MODULE(IMAGE_MODULE));
    assert(root_module(test_header, 1) == FRAMING_MODULE);
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 1;
    assert(same(current_module(test_header)->summary(test_header), "Framing: Off"));
    menu_confirm(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_MODULE && test_header[GRKIT_HOST_MODULE / 4] == FRAMING_MODULE);
    assert(menu_back(test_header));
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT && test_header[GRKIT_HOST_MENU_CURSOR / 4] == 1);
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 2;
    menu_confirm(test_header);
    assert(test_header[GRKIT_HOST_MODULE / 4] == GUIDES_MODULE);
    assert(menu_back(test_header) && test_header[GRKIT_HOST_MENU_CURSOR / 4] == 2);
    /* The page of a module page that is gone does not stay open. */
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_MODULE;
    test_header[GRKIT_HOST_MODULE / 4] = DATE_MODULE;
    menu_normalize(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT && test_header[GRKIT_HOST_MENU_CURSOR / 4] == 0);
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_MODULE;
    test_header[GRKIT_HOST_MODULE / 4] = GRAIN_MODULE;
    menu_normalize(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT && test_header[GRKIT_HOST_MENU_CURSOR / 4] == 0);
    test_header[GRKIT_HOST_MODULE_PAGE_COUNT / 4] = ALL_PAGES;
}

static void the_root_shows_only_visible_module_pages(void)
{
    reset();
    /* Without effects, the page of Film Look does not show. */
    test_header[GRKIT_HOST_COUNT / 4] = 0;
    assert(visible_modules(test_header) == (1U << FRAMING_MODULE | 1U << GUIDES_MODULE | 1U << DATE_MODULE |
                                            1U << GRAIN_MODULE));
    menu_start(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT && count(test_header) == 4);
    assert(root_module(test_header, 0) == FRAMING_MODULE);
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 2;
    menu_confirm(test_header);
    assert(test_header[GRKIT_HOST_MODULE / 4] == DATE_MODULE);
    assert(menu_back(test_header) && test_header[GRKIT_HOST_MENU_CURSOR / 4] == 2);
    /* visible_modules keeps the page that the caller selected. */
    menu_page(test_header, GUIDES_MODULE);
    visible_modules(test_header);
    assert(test_header[GRKIT_HOST_MODULE_CHOICE / 4] == GUIDES_MODULE);
    /* A module page without its slot in GRKIT_HOST_MODULE_PAGE_COUNT does not
       show. Without a module page, the root shows About alone. The installer
       installs no menu hook then, so only an inconsistent header gets here. */
    test_header[GRKIT_HOST_MODULE_PAGE_COUNT / 4] = 0;
    assert(!(visible_modules(test_header) >> GUIDES_MODULE & 1));
    menu_start(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT && count(test_header) == 1);
    assert(current_module(test_header) == &MODULE(ABOUT_MODULE));
    menu_confirm(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT && !menu_back(test_header));
    /* Film Look alone keeps the root level. Its page opens at the committed
       effect, where Fn applies. */
    test_header[GRKIT_HOST_MODULE_PAGE_COUNT / 4] = 1;
    test_header[GRKIT_HOST_COUNT / 4] = 5;
    test_header[GRKIT_HOST_ACTIVE / 4] = 0x59005000 + 2 * GRKIT_RECORD_SIZE;
    menu_start(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT && count(test_header) == 1);
    menu_confirm(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_MODULE && test_header[GRKIT_HOST_MODULE / 4] == IMAGE_MODULE);
    assert(test_header[GRKIT_HOST_MENU_CURSOR / 4] == 3 && parameters_row(test_header));
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 1;
    highlight(test_header);
    assert(test_header[GRKIT_HOST_PREVIEW / 4] == 0x59005000);
    assert(menu_back(test_header));
    assert(test_header[GRKIT_HOST_PREVIEW / 4] == 0 && test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT);
    test_header[GRKIT_HOST_MODULE_PAGE_COUNT / 4] = ALL_PAGES;
}

static void adj_reopens_on_the_row_of_the_last_module(void)
{
    reset();
    /* ADJ reopens at the root, on the row of the last module. A module without
       a row, or a module that is not valid, reopens at the first row. */
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_MODULE;
    test_header[GRKIT_HOST_MODULE / 4] = GRAIN_MODULE;
    menu_reopen(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT && test_header[GRKIT_HOST_MENU_CURSOR / 4] == GRAIN_ROW);
    test_header[GRKIT_HOST_MODULE / 4] = ABOUT_MODULE;
    menu_reopen(test_header);
    assert(test_header[GRKIT_HOST_MENU_CURSOR / 4] == 0);
    test_header[GRKIT_HOST_MODULE / 4] = GUIDES_MODULE;
    menu_reopen(test_header);
    assert(test_header[GRKIT_HOST_MENU_CURSOR / 4] == GUIDES_ROW);
    test_header[GRKIT_HOST_MODULE_PAGE_COUNT / 4] = 3;
    test_header[GRKIT_HOST_MODULE / 4] = DATE_MODULE;
    menu_reopen(test_header);
    assert(test_header[GRKIT_HOST_MENU_CURSOR / 4] == 0);
    test_header[GRKIT_HOST_MODULE_PAGE_COUNT / 4] = ALL_PAGES;
    test_header[GRKIT_HOST_MODULE / 4] = 999;
    menu_reopen(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT && test_header[GRKIT_HOST_MENU_CURSOR / 4] == 0);
}

/* A settings entry opens the native rows from the ADJ root. Each row changes
   only its setting; the root title and tile do not follow the first value. */
static void direct_settings_have_independent_rows(void)
{
    reset();
    volatile u32 *h = test_header;
    const u32 address = TEST_PAGE + DATE_MODULE * TEST_PAGE_SIZE;
    *page_at(address + PAGE_KIND) = PAGE_SETTINGS;
    *page_at(address + PAGE_FLAGS) = 0;
    *page_at(address + PAGE_SUMMARIES) = 0;
    *page_at(address + PAGE_LABELS) = 0;
    menu_start(h);
    h[GRKIT_HOST_MENU_CURSOR / 4] = DATE_ROW;
    assert(settings_target(h, TEST_OBJECTS) == DATE_MODULE);
    assert(direct_settings_event(h, EV_CONFIRM) && direct_settings_event(h, EV_RIGHT));
    assert(!direct_settings_event(h, EV_LEFT) && !direct_settings_event(h, EV_UP));
    h[GRKIT_HOST_MENU_EVENTS / 4] = 2;
    h[GRKIT_HOST_MENU_EVENT_RING / 4] = EV_ADJ_PRESS;
    assert(!direct_settings_event(h, EV_CONFIRM));
    h[GRKIT_HOST_MENU_EVENT_RING / 4] = 0;
    test_movie = 1;
    assert(settings_target(h, TEST_OBJECTS) == MODULE_TOTAL);
    test_movie = 0;
    const MenuPage *module = menu_page(h, DATE_MODULE);
    assert(same(module->summary(h), "Date Imprint"));
    DATE_STYLE = 4;
    assert(module->icon(h, 0) == DATE_TILE(0));
    menu_confirm(h);
    assert(h[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT);
    const Setting *rows = menu_page(h, DATE_MODULE)->settings;
    rows[0].set(h, 2);
    rows[1].set(h, 1);
    assert(DATE_STYLE == 2 && DATE_LOOK == 1);
    rows[0].set(h, 0);
    assert(DATE_STYLE == 0 && DATE_LOOK == 1);
    h[GRKIT_HOST_MODULE / 4] = DATE_MODULE;
    menu_reopen(h);
    assert(h[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT && h[GRKIT_HOST_MENU_CURSOR / 4] == DATE_ROW);
    assert(DATE_STYLE == 0 && DATE_LOOK == 1);
    menu_page(h, DATE_MODULE);
    rows[1].set(h, 0);
    assert(DATE_STYLE == 0 && DATE_LOOK == 0);
    /* A single row with a single value remains an ADJ settings entry. */
    settings_memory[2 + 1 * 3 + 2] = 1;
    *page_at(address + PAGE_SETTING_ROWS) = 1;
    assert(visible_modules(h) & (1U << DATE_MODULE));
    assert(settings_target(h, TEST_OBJECTS) == DATE_MODULE);
}

static void the_date_imprint_settings_page_reads_and_stores_values(void)
{
    reset();
    /* The settings page of Date Imprint: the date style, then the look. Only
       Date Imprint has one. */
    for (u32 module = 0; module < MODULE_TOTAL; ++module)
        assert(!MODULE(module).settings == (module != DATE_MODULE && module != MANAGE_MODULE));
    const Setting *page = MODULE(DATE_MODULE).settings;
    volatile u32 *block = settings_block(test_header);
    const u32 view = 0x59100000;
    static const u32 row_texts[] = SETTINGS_ROW_TEXTS, pages[] = SETTINGS_PAGES;
    assert(same(page[0].label(test_header), "Date Style") && same(page[1].label(test_header), "Date Look"));
    assert(page[0].values(test_header) == 6 && page[1].values(test_header) == 3);
    for (u32 value = 0; value < 6; ++value)
        assert(same(page[0].value_text(test_header, value), stamp_names[value]));
    assert(same(page[1].value_text(test_header, 0), "Classic") && same(page[1].value_text(test_header, 2), "GR II"));
    /* The title view of each submenu page is reset before the page sets its
       title ID, so the title measures its own text: GRKit pages share one
       title ID. */
    MENU_OBJECT_WORD(TEST_OBJECTS + 0x2c00, SUBMENU_TITLE_STRING) = view;
    view_calls = 0;
    menu_settings_title(TEST_OBJECTS + 0x2c00);
    assert(view_calls == 1 && view_functions[0] == FW_STRING_VIEW_RESET && view_args[0][0] == view &&
           !view_args[0][1] && !view_args[0][2]);
    MENU_OBJECT_WORD(TEST_OBJECTS + 0x2c00, SUBMENU_TITLE_STRING) = 0;
    view_calls = 0;
    menu_settings_title(TEST_OBJECTS + 0x2c00);
    menu_settings_title(0);
    assert(!view_calls);
    /* Without a page the native table, methods, and text stay. */
    test_header[GRKIT_HOST_SETTINGS / 4] = 0;
    assert(menu_settings_kinds() == FW_SUBMENU_KINDS);
    assert(!menu_settings_count(0, 0, 1) && !menu_settings_apply(1, 0, 0) && !menu_settings_current(0, 0, 0));
    assert(!menu_text_override(view, SETTINGS_TITLE_TEXT));
    /* A page reads the private entry at the index of the borrowed one.
       Date Imprint has two rows, which settings_start writes as the row
       count of the entry. */
    test_header[GRKIT_HOST_SETTINGS / 4] = DATE_MODULE + 1;
    assert(settings_count(test_header, page) == 2);
    ((volatile u8 *)block)[SUBMENU_KIND_ROWS] = 2;
    assert(settings_page(test_header) == page);
    assert(menu_settings_kinds() + SETTINGS_SUBMENU_KIND * SUBMENU_KIND_SIZE == (u32)block);
    assert(menu_settings_count(0, 0, 0) == 7 && menu_settings_count(0, 0, 1) == 4);
    /* OK stores a value of the row's setting, and the page marks it. */
    DATE_STYLE = 0;
    DATE_LOOK = 0;
    assert(menu_settings_apply(4, 0, 0) == 1 && DATE_STYLE == 4);
    assert(menu_settings_apply(2, 0, 1) == 1 && DATE_LOOK == 2);
    assert(menu_settings_apply(3, 0, 1) == 1 && DATE_LOOK == 2);
    assert(menu_settings_current(4, 0, 0) == 2 && menu_settings_current(3, 0, 0) == 1);
    assert(menu_settings_current(2, 0, 1) == 2 && menu_settings_current(0, 0, 1) == 1);
    /* A stored look out of range reads as classic. */
    DATE_LOOK = 7;
    assert(menu_settings_current(0, 0, 1) == 2);
    /* The title and the row labels replace their text IDs, in the font of
       the menu language. A value view shows its value for the title ID. */
    for (u32 i = 0; i < SETTINGS_VIEW_SLOTS * 3; ++i) block[SETTINGS_VIEWS / 4 + i] = 0;
    block[SETTINGS_VIEWS / 4 + 3] = view;
    block[SETTINGS_VIEWS / 4 + 4] = 0x104;
    assert(menu_text_override(view, SETTINGS_TITLE_TEXT) == ((u32)(block + SETTINGS_VIEWS / 4 + 4) | 1U));
    assert(menu_text_override(view + 4, SETTINGS_TITLE_TEXT) == ((u32)(block + SETTINGS_LABELS / 4) | 1U));
    assert(menu_text_override(view, row_texts[0]) == ((u32)(block + SETTINGS_LABELS / 4 + 2) | 1U));
    assert(menu_text_override(view, row_texts[1]) == ((u32)(block + SETTINGS_LABELS / 4 + 4) | 1U));
    /* The label of a row past the rows of the page keeps its text. */
    assert(!menu_text_override(view, row_texts[2]));
    /* A change to the submenu or to its value pages keeps the page. Any
       other controller, such as ADJ (0x2000000), ends it. */
    menu_settings_controller(SETTINGS_SUBMENU);
    menu_settings_controller(pages[0]);
    menu_settings_controller(pages[1]);
    assert(test_header[GRKIT_HOST_SETTINGS / 4] == DATE_MODULE + 1);
    menu_settings_controller(0x2000000);
    assert(!test_header[GRKIT_HOST_SETTINGS / 4] && !menu_text_override(view, SETTINGS_TITLE_TEXT));
    /* A stored module without settings, or out of range, has no page. */
    test_header[GRKIT_HOST_SETTINGS / 4] = FRAMING_MODULE + 1;
    assert(!settings_page(test_header));
    test_header[GRKIT_HOST_SETTINGS / 4] = 99;
    assert(!settings_page(test_header) && menu_settings_kinds() == FW_SUBMENU_KINDS);
}

/* The module list holds Guides and Date Imprint, which loaded, and a
   module that did not link. Its settings follow the three module
   settings: CORE.OFF, CORE.POS, and CORE.ROWS of each. */
static void install_module_list(void)
{
    u32 entry;
    _Static_assert(MANAGE_STRIDE == 56 && MANAGE_HEADER == 24, "The test list matches manage.h");
    settings_memory[0] = 12;
    for (u32 i = 0; i < 3; ++i) {
        settings_memory[2 + (3 + 3 * i) * 3 + 1] = 0;
        settings_memory[2 + (3 + 3 * i) * 3 + 2] = 2;
        settings_memory[2 + (4 + 3 * i) * 3 + 1] = 0;
        settings_memory[2 + (4 + 3 * i) * 3 + 2] = MANAGE_MODULES + 1;
        settings_memory[2 + (5 + 3 * i) * 3 + 1] = 0;
        settings_memory[2 + (5 + 3 * i) * 3 + 2] = 1U << MANAGE_OPTION_VALUES;
    }
    manage_memory[0] = 3;
    manage_memory[1] = 3;
    entry = MANAGE_ENTRY(0);
    manage_memory[entry] = MANAGE_LOADED;
    manage_memory[entry + 1] = TEST_PAGE + 2 * TEST_PAGE_SIZE + PAGE_TITLES;
    manage_memory[entry + 2] = 7;
    entry += 14;
    manage_memory[entry] = MANAGE_LOADED;
    manage_memory[entry + 1] = TEST_PAGE + 3 * TEST_PAGE_SIZE + PAGE_TITLES;
    manage_memory[entry + 2] = MANAGE_NO_TILE;
    entry += 14;
    manage_memory[entry] = 52;
    manage_memory[entry + 2] = MANAGE_NO_TILE;
    ascii((unsigned short *)&manage_memory[entry + 6], "BROKEN (52)");
    test_header[GRKIT_HOST_MANAGE / 4] = TEST_MANAGE;
}

static void the_adj_order_page_moves_modules(void)
{
    const Setting *page;
    u32 roots;
    reset();
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_ROOT;
    roots = count(test_header);
    assert(!(visible_modules(test_header) >> MANAGE_MODULE & 1));
    install_module_list();
    /* The root has no ADJ Order row: it is in the GRKit section of MENU.
       A loaded page finds its listed module by its title, and each loaded
       page shows. */
    assert(count(test_header) == roots && !(visible_modules(test_header) >> MANAGE_MODULE & 1));
    assert(page_listed(test_header, 2) == 0 && page_listed(test_header, 3) == 1 && page_listed(test_header, 0) == MANAGE_MODULES);
    assert(visible_modules(test_header) >> 3 & 1);
    /* ADJ Order lists the modules that ADJ can show, by their page titles.
       BROKEN (52) did not link, so it has no page and no row. */
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_MODULE;
    test_header[GRKIT_HOST_MODULE / 4] = MANAGE_MODULE;
    assert(same(current_module(test_header)->title(test_header), "ADJ Order"));
    /* The menu finds these modules when ADJ Order opens or the section is
       added. The text hooks read only the result. */
    assert(!manage_adj_modules(test_header) && !count(test_header));
    manage_adj_refresh(test_header);
    assert(test_header[GRKIT_HOST_ADJ_MODULES / 4] == 3);
    assert(manage_adj_modules(test_header) == 3 && count(test_header) == 2);
    assert(same(MODULE(MANAGE_MODULE).label(test_header, 0), "Guides"));
    assert(same(MODULE(MANAGE_MODULE).label(test_header, 1), "Date Imprint"));
    /* The settings page: a row for each of these modules, in order, whose
       values are the positions 1 and 2. */
    test_header[GRKIT_HOST_SETTINGS / 4] = MANAGE_MODULE + 1;
    page = settings_page(test_header);
    assert(page == MODULE(MANAGE_MODULE).settings && settings_count(test_header, page) == 2);
    /* settings_start writes the row count; a row past it gets no hook. */
    ((volatile u8 *)settings_block(test_header))[SUBMENU_KIND_ROWS] = 2;
    assert(!menu_settings_count(0, 0, 2) && !menu_settings_apply(0, 0, 4));
    assert(same(page[0].label(test_header), "Guides") && same(page[1].label(test_header), "Date Imprint"));
    assert(menu_settings_count(0, 0, 1) == 3);
    assert(same(page[1].value_text(test_header, 0), "1") && same(page[1].value_text(test_header, 1), "2"));
    assert(menu_settings_current(1, 0, 1) == 2 && menu_settings_current(0, 0, 1) == 1);
    /* Position 1 moves Date Imprint first. Each module of the page gets its
       row + 1, and BROKEN (52) keeps its position. */
    assert(menu_settings_apply(0, 0, 1) == 1);
    assert(settings_memory[2 + 7 * 3 + 1] == 1 && settings_memory[2 + 4 * 3 + 1] == 2 && settings_memory[2 + 10 * 3 + 1] == 0);
    assert(same(page[0].label(test_header), "Date Imprint") && same(page[1].label(test_header), "Guides"));
    assert(menu_settings_current(0, 0, 0) == 2);
    /* The return from the value page labels the rows in the new order:
       "Date Imprint", then "Guides". */
    assert(menu_settings_controller(SETTINGS_SUBMENU) == SETTINGS_SUBMENU);
    assert(test_header[GRKIT_HOST_SETTINGS / 4] == MANAGE_MODULE + 1);
    assert(settings_block(test_header)[SETTINGS_LABELS / 4 + 2] == ((12 + 1) | 0x100));
    assert(settings_block(test_header)[SETTINGS_LABELS / 4 + 4] == ((6 + 1) | 0x100));
    /* The order applies at once: the page of Date Imprint comes first in
       the root of ADJ, and no restart is due. */
    assert(root_row(test_header, 3) + 1 == root_row(test_header, 2) && root_module(test_header, root_row(test_header, 3)) == 3);
    assert(same(MODULE(MANAGE_MODULE).summary(test_header), "Modules"));
    /* Position 2 moves it last again: the order of this start. */
    assert(menu_settings_apply(1, 0, 0) == 1);
    assert(same(page[0].label(test_header), "Guides") && same(page[1].label(test_header), "Date Imprint"));
    assert(settings_memory[2 + 10 * 3 + 1] == 0);
    assert(same(MODULE(MANAGE_MODULE).summary(test_header), "Modules"));
    assert(root_row(test_header, 2) + 1 == root_row(test_header, 3));
    /* A page of six rows: the entry has no room for the controller of the
       last row, so OK on it opens the value page of SETTINGS_ALIAS for the
       last row: its title is the label of the last row, and its methods
       are those of the last row. The submenu shows its own labels again. */
    {
        volatile u32 *block = settings_block(test_header);
        static const u32 row_texts[] = SETTINGS_ROW_TEXTS, pages[] = SETTINGS_PAGES;
        const u32 kind = 0x21c, view = 0x59200000;
        _Static_assert(sizeof(row_texts) / sizeof(row_texts[0]) == SETTING_ROWS, "A label for each row");
        ((volatile u8 *)block)[SUBMENU_KIND_ROWS] = SETTING_ROWS;
        block[SUBMENU_KIND_ROW_KINDS / 4] = kind;
        assert(menu_settings_row_controller(kind, SETTING_ROWS - 1) == pages[SETTINGS_ALIAS]);
        assert(menu_settings_controller(pages[SETTINGS_ALIAS]) == pages[SETTINGS_ALIAS]);
        assert(test_header[GRKIT_HOST_SETTINGS / 4] == (MANAGE_MODULE + 1 | (SETTING_ROWS - 1) << SETTINGS_OPEN_ROW));
        assert(settings_alias(test_header, SETTINGS_ALIAS) == SETTING_ROWS - 1 && settings_alias(test_header, 1) == 1);
        assert(menu_text_override(view, row_texts[SETTINGS_ALIAS]) ==
               ((u32)(block + SETTINGS_LABELS / 4 + SETTING_ROWS * 2) | 1U));
        /* The last row is past the modules of the group: its value page
           moves no module, where that of the first row would. */
        assert(menu_settings_apply(2, 0, SETTINGS_ALIAS) == 1);
        assert(same(page[0].label(test_header), "Guides") && same(page[1].label(test_header), "Date Imprint"));
        assert(menu_settings_controller(SETTINGS_SUBMENU) == SETTINGS_SUBMENU);
        assert(test_header[GRKIT_HOST_SETTINGS / 4] == MANAGE_MODULE + 1);
        /* The return reads the rows of the page again: its two modules.
           The six rows of this case are set again. */
        assert(((volatile u8 *)block)[SUBMENU_KIND_ROWS] == 2);
        ((volatile u8 *)block)[SUBMENU_KIND_ROWS] = SETTING_ROWS;
        assert(menu_text_override(view, row_texts[SETTINGS_ALIAS]) == ((u32)(block + SETTINGS_LABELS / 4 + 2) | 1U));
        assert(menu_text_override(view, row_texts[SETTING_ROWS - 1]) ==
               ((u32)(block + SETTINGS_LABELS / 4 + SETTING_ROWS * 2) | 1U));
        /* The value page of the first row is its own. */
        assert(menu_settings_row_controller(pages[SETTINGS_ALIAS], 0) == pages[SETTINGS_ALIAS]);
        assert(menu_settings_controller(pages[SETTINGS_ALIAS]) == pages[SETTINGS_ALIAS]);
        assert(!(test_header[GRKIT_HOST_SETTINGS / 4] & SETTINGS_OPEN_ROWS));
        ((volatile u8 *)block)[SUBMENU_KIND_ROWS] = 2;
    }
}

static void options_hide_values_that_are_off_in_adj(void)
{
    reset();
    install_module_list();
    /* A page with options hides the values that are off in ADJ: the bits
       of CORE.ROWS of its module. Off stays. A hidden value of the
       setting shows no checked row. */
    *page_at(TEST_PAGE + 2 * TEST_PAGE_SIZE + PAGE_FLAGS) |= CHOICE_OPTIONS;
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_MODULE;
    test_header[GRKIT_HOST_MODULE / 4] = GUIDES_MODULE;
    settings_memory[2 + 1] = 1;
    assert(count(test_header) == 6 && MODULE(GUIDES_MODULE).committed(test_header) == 1);
    settings_memory[2 + 5 * 3 + 1] = 1U << 1 | 1U;
    assert(count(test_header) == 5 && same(MODULE(GUIDES_MODULE).label(test_header, 0), "Off"));
    assert(same(MODULE(GUIDES_MODULE).label(test_header, 1), "Center") && !MODULE(GUIDES_MODULE).committed(test_header));
    assert(!MODULE(GUIDES_MODULE).checked(test_header, 0) && !MODULE(GUIDES_MODULE).checked(test_header, 1));
    settings_memory[2 + 5 * 3 + 1] = 0;
    settings_memory[2 + 1] = 0;
    *page_at(TEST_PAGE + 2 * TEST_PAGE_SIZE + PAGE_FLAGS) &= ~CHOICE_OPTIONS;
}

static void enable_applies_at_once(void)
{
    reset();
    install_module_list();
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_MODULE;
    test_header[GRKIT_HOST_MODULE / 4] = MANAGE_MODULE;
    /* Enable Off of a loaded module applies at once, as a soft Off: its
       page leaves the root of ADJ, and no restart is due. */
    settings_memory[2 + 6 * 3 + 1] = 1;
    assert(manage_stopped(test_header, 1));
    assert(!(visible_modules(test_header) >> 3 & 1));
    assert(same(MODULE(MANAGE_MODULE).summary(test_header), "Modules"));
    /* On applies at once too. */
    settings_memory[2 + 6 * 3 + 1] = 0;
    assert(!manage_stopped(test_header, 1) && visible_modules(test_header) >> 3 & 1);
    /* Safe mode shows in the summary. */
    manage_memory[2] = MANAGE_SAFE_MODE;
    assert(same(MODULE(MANAGE_MODULE).summary(test_header), "Modules: Safe mode"));
    /* A count past the list lists nothing. */
    manage_memory[0] = MANAGE_MODULES + 1;
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_ROOT;
    assert(!manage_count(test_header));
}

/* The GRKit switch stops each module at once, as a soft Off does, and keeps
   the Enable of each: On starts again each module whose Enable is On. In
   ADJ, it is a root row after the module pages, with a page of On and Off. */
static void the_switch_stops_each_module_at_once(void)
{
    volatile u32 *h = test_header;
    const u32 tone = 1U << (GRKIT_CORE_TONE - GRKIT_CORE_SETTING_BASE),
              aspect = 1U << (GRKIT_CORE_ASPECT - GRKIT_CORE_SETTING_BASE);
    u32 roots, n;
    reset();
    install_module_list();
    h[GRKIT_HOST_SERVICES / 4] = TEST_SERVICES;
    h[GRKIT_HOST_MENU_LEVEL / 4] = MENU_ROOT;
    /* Without a switch in the module list, the root has no switch row. */
    roots = count(h);
    assert(!manage_switch(h) && !manage_all_off(h) && !(visible_modules(h) >> SWITCH_MODULE & 1));
    /* The switch, CORE.OFF, follows the choices of the modules. Guides
       uses Film Look and Framing here. */
    settings_memory[0] = 13;
    settings_memory[2 + 12 * 3 + 2] = 2;
    manage_memory[MANAGE_SWITCH / 4] = 12 + 1;
    manage_memory[MANAGE_ENTRY(0) + MANAGE_USES / 4] = tone | aspect;
    assert(count(h) == roots + 1 && root_module(h, roots) == SWITCH_MODULE);
    /* Under the GRKit title of the root, the row is Enable. */
    assert(same(MODULE(SWITCH_MODULE).summary(h), "Enable: On"));
    assert(MODULE(SWITCH_MODULE).icon(h, roots) == GRKIT_ICON_MODULES);
    assert(!MODULE(SWITCH_MODULE).settings);
    /* The page: Off first, as on each module page, then On; it opens at the
       value. */
    menu_open(h, SWITCH_MODULE);
    assert(count(h) == 2 && h[GRKIT_HOST_MENU_CURSOR / 4] == 1 && same(current_module(h)->title(h), "Enable"));
    assert(same(MODULE(SWITCH_MODULE).label(h, 0), "Off") && same(MODULE(SWITCH_MODULE).label(h, 1), "On"));
    assert(!MODULE(SWITCH_MODULE).checked(h, 0) && MODULE(SWITCH_MODULE).checked(h, 1));
    assert(MODULE(SWITCH_MODULE).icon(h, 0) == GRKIT_ICON_OFF && MODULE(SWITCH_MODULE).icon(h, 1) == GRKIT_ICON_MODULES);
    /* An ADJ exit on the page, such as a half-press with Off highlighted,
       keeps the switch On. */
    h[GRKIT_HOST_MENU_CURSOR / 4] = 0;
    exit_commit(h);
    assert(!manage_all_off(h) && h[GRKIT_HOST_MODULE / 4] == SWITCH_MODULE);
    /* Off stops each loaded module at once and counts as one change. The
       Enable of each keeps its value. No capability runs, and Framing
       sends the native 3:2 request again for the full area. */
    n = h[GRKIT_HOST_STATE_CHANGES / 4];
    h[GRKIT_HOST_MENU_CURSOR / 4] = 0;
    menu_confirm(h);
    assert(manage_all_off(h) && settings_memory[2 + 12 * 3 + 1] == 1 && h[GRKIT_HOST_STATE_CHANGES / 4] == n + 1);
    assert(manage_stopped(h, 0) && manage_stopped(h, 1) && !manage_off(h, 0) && !manage_off(h, 1));
    assert(services_word(h, SERVICES_SUSPENDED)[0] == 2);
    assert(!h[GRKIT_HOST_CAPABILITIES / 4] && framing_requests == 1);
    /* The pages of the modules leave the root, and the switch stays. */
    h[GRKIT_HOST_MENU_LEVEL / 4] = MENU_ROOT;
    assert(count(h) == roots - 1 && !(visible_modules(h) >> 2 & 1) && !(visible_modules(h) >> 3 & 1));
    assert(root_module(h, roots - 2) == SWITCH_MODULE);
    /* The root row shows the Modules tile with a line through it. */
    assert(same(MODULE(SWITCH_MODULE).summary(h), "Enable: Off") &&
           MODULE(SWITCH_MODULE).icon(h, 0) == GRKIT_ICON_MODULES_OFF);
    /* Film Look keeps its effect, and the viewfinder shows the native
       icon. */
    h[GRKIT_HOST_ACTIVE / 4] = 0x59005000;
    assert(selected_icon(h) == 0x7fff);
    /* On starts the modules whose Enable is On: Date Imprint stays Off. */
    settings_memory[2 + 6 * 3 + 1] = 1;
    manage_set_all_off(h, 0);
    assert(!manage_all_off(h) && !manage_stopped(h, 0) && manage_stopped(h, 1));
    assert(h[GRKIT_HOST_CAPABILITIES / 4] == (tone | aspect) && framing_requests == 2);
    assert(selected_icon(h) == 0x8000);
    assert(visible_modules(h) >> 2 & 1 && !(visible_modules(h) >> 3 & 1));
    settings_memory[2 + 6 * 3 + 1] = 0;
    /* Refresh geometry in other native aspects too, without changing the native choice. */
    native_aspect_value = 1;
    manage_set_all_off(h, 1);
    assert(framing_requests == 3 && native_aspect_value == 1 && requested_crop == 0);
    manage_set_all_off(h, 0);
    assert(framing_requests == 4 && native_aspect_value == 1);
    native_aspect_value = 0;
    /* In safe mode no module loads, and the switch does not show. */
    for (u32 i = 0; i < 3; ++i) manage_memory[MANAGE_ENTRY(i)] = MANAGE_SAFE;
    assert(!manage_switch_shown(h) && !(visible_modules(h) >> SWITCH_MODULE & 1));
    h[GRKIT_HOST_ACTIVE / 4] = 0;
}

static void stopping_the_mode_provider_restores_legacy_capture_geometry(void)
{
    volatile u32 *h = test_header;
    reset();
    install_module_list();
    h[GRKIT_HOST_SERVICES / 4] = TEST_SERVICES;
    manage_memory[MANAGE_ENTRY(0) + MANAGE_USES / 4] = GRKIT_CORE_ASPECT_BIT;
    manage_memory[MANAGE_ENTRY(1) + MANAGE_USES / 4] = GRKIT_CORE_ASPECT_MODE_BIT;
    manage_suspend(h);
    setting_store(h, GRKIT_CORE_ASPECT, 1);
    setting_store(h, GRKIT_CORE_ASPECT_MODE, 1);
    assert(requested_crop == 0);
    for (u32 native = 0; native < 4; ++native) {
        u32 requests = framing_requests;
        u32 crop = !native || FRAMING_CROP_KEEPS(native, 1) ? 1 : 0;
        native_aspect_value = native;
        setting_store(h, manage_off_handle(h, 1), 1);
        manage_off_changed(h);
        assert(h[GRKIT_HOST_CAPABILITIES / 4] == GRKIT_CORE_ASPECT_BIT);
        assert(h[GRKIT_HOST_ASPECT_MODE / 4] == 1 && framing_crop(h) == 1);
        assert(framing_requests == requests + 1 && requested_crop == crop);
        assert(h[GRKIT_HOST_FRAME_CROP / 4] == crop && native_aspect_value == native);
        setting_store(h, manage_off_handle(h, 1), 0);
        manage_off_changed(h);
        assert(framing_requests == requests + 2 && requested_crop == 0);
        assert(h[GRKIT_HOST_FRAME_CROP / 4] == 0 && framing_crop(h) == 0);
    }
    /* Stopping Framing clears XPan geometry even when GR IV keeps native capture id 2. */
#ifdef FRAMING_XPAN_NATIVE
    native_aspect_value = FRAMING_XPAN_NATIVE;
#else
    native_aspect_value = 0;
#endif
    setting_store(h, GRKIT_CORE_ASPECT_MODE, 0);
    assert(requested_crop == 1);
    setting_store(h, manage_off_handle(h, 0), 1);
    manage_off_changed(h);
    assert(h[GRKIT_HOST_CAPABILITIES / 4] == GRKIT_CORE_ASPECT_MODE_BIT);
    assert(requested_crop == 0 && h[GRKIT_HOST_FRAME_CROP / 4] == 0 && framing_selection(h) == 0);
    setting_store(h, manage_off_handle(h, 0), 0);
    manage_off_changed(h);
    assert(requested_crop == 1 && h[GRKIT_HOST_FRAME_CROP / 4] == 1 && framing_selection(h) == 1);
}

/* ADJ Order has a row for each module that ADJ can show: here each listed
   module has a page of two rows. The kind table entry has room for the
   labels of six rows and the controllers of five: a later label shows in a
   view slot, and OK on a later row opens the value page of SETTINGS_ALIAS
   for that row. */
static void the_adj_order_page_has_a_row_for_each_module(void)
{
    static const u32 pages[] = SETTINGS_PAGES;
    volatile u32 *h = test_header, *block;
    const u32 last = MANAGE_MODULES - 1, native = 0x1234, view = 0x5b000000;
    _Static_assert(MANAGE_HEADER + MANAGE_MODULES * MANAGE_STRIDE <= TEST_EFFECTS - TEST_MANAGE &&
                   sizeof(settings_memory) >= 4 * (2 + 3 * (3 + 3 * MANAGE_MODULES)), "The test memory holds the list");
    _Static_assert(TEST_PAGE_SLOTS >= MANAGE_MODULES && GRKIT_MODULE_PAGES_MAX >= MANAGE_MODULES, "A page for each module");
    reset();
    settings_memory[0] = 3 + 3 * MANAGE_MODULES;
    for (u32 i = 0; i < MANAGE_MODULES; ++i) {
        u32 entry = MANAGE_ENTRY(i);
        char name[] = "Module 0";
        settings_memory[2 + (3 + 3 * i) * 3 + 2] = 2;
        settings_memory[2 + (4 + 3 * i) * 3 + 2] = MANAGE_MODULES + 1;
        settings_memory[2 + (5 + 3 * i) * 3 + 2] = 1U << MANAGE_OPTION_VALUES;
        static const char *const two_rows[] = {"Off", "On"};
        name[7] = (char)('0' + i);
        install_page(i, name, two_rows, 2, 3 + 3 * i, 0);
        manage_memory[entry] = MANAGE_LOADED;
        manage_memory[entry + 1] = TEST_PAGE + i * TEST_PAGE_SIZE + PAGE_TITLES;
        manage_memory[entry + 2] = MANAGE_NO_TILE;
        ascii((unsigned short *)&manage_memory[entry + 6], name);
    }
    manage_memory[0] = MANAGE_MODULES;
    manage_memory[1] = 3;
    h[GRKIT_HOST_MANAGE / 4] = TEST_MANAGE;
    settings_start(h, MANAGE_MODULE);
    block = settings_block(h);
    assert(settings_rows(h) == MANAGE_MODULES);
    /* The entry holds the labels of the first rows. Their views measure
       the label again, as a move changes the label and keeps its ID. A
       later row shows its label in a view slot. */
    object_memory[ROW_LABEL_VIEW / 4] = view;
    for (u32 i = 0; i < SETTINGS_LABEL_ROWS; ++i) {
        view_calls = 0;
        assert(menu_settings_row_label(native, TEST_OBJECTS, i) == native && view_called(FW_STRING_VIEW_RESET));
    }
    assert(menu_settings_row_label(native, TEST_OBJECTS, last) == SETTINGS_TITLE_TEXT);
    assert(menu_text_override(view, SETTINGS_TITLE_TEXT) ==
           ((u32)(block + SETTINGS_VIEWS / 4 + SETTINGS_LABEL_SLOT(last) * 3 + 1) | 1U));
    assert(block[SUBMENU_KIND_VALUES / 4 + 2 * last] == (u32)(unsigned long)menu_settings_value_9);
    /* OK on a row of the entry opens its own value page. */
    assert(menu_settings_row_controller(pages[1], 1) == pages[1]);
    assert(!(h[GRKIT_HOST_SETTINGS / 4] & SETTINGS_OPEN_ROWS));
    /* OK on the last row opens the value page of SETTINGS_ALIAS, which
       shows each position. Position 1 moves the last module first. */
    assert(menu_settings_row_controller(native, last) == pages[SETTINGS_ALIAS]);
    assert(menu_settings_controller(pages[SETTINGS_ALIAS]) == pages[SETTINGS_ALIAS]);
    assert(settings_alias(h, SETTINGS_ALIAS) == last);
    assert(menu_settings_count(0, 0, SETTINGS_ALIAS) == MANAGE_MODULES + 1);
    assert(menu_settings_current(last, 0, SETTINGS_ALIAS) == 2);
    assert(menu_settings_apply(0, 0, SETTINGS_ALIAS) == 1);
    assert(same(manage_settings[0].label(h), "Module 9") && same(manage_settings[1].label(h), "Module 0"));
    /* The return to the page forgets the row. */
    assert(menu_settings_controller(SETTINGS_SUBMENU) == SETTINGS_SUBMENU);
    assert(settings_alias(h, SETTINGS_ALIAS) == SETTINGS_ALIAS);
    /* The draw of a submenu with SUBMENU_ROWS row views shows each row. The
       firmware keeps a submenu that it built before INSTALL, with the five
       views of the stock constructor: the page then shows five rows, so the
       draw, Down, and OK read no view past the list. */
    const u32 list = TEST_OBJECTS + 0x2800;
    WORD(list, LIST_ROWS) = 0x59200000;
    WORD(list, LIST_ROWS + 4) = 0x59200000 + 4 * SUBMENU_ROWS;
    assert(menu_settings_kinds_draw(list) + SETTINGS_SUBMENU_KIND * SUBMENU_KIND_SIZE == (u32)block);
    assert(settings_rows(h) == MANAGE_MODULES);
    WORD(list, LIST_ROWS + 4) = 0x59200000 + 4 * 5;
    assert(menu_settings_kinds_draw(list) + SETTINGS_SUBMENU_KIND * SUBMENU_KIND_SIZE == (u32)block);
    assert(settings_rows(h) == 5 && !settings_row_page(h, 5) && settings_row_page(h, 4));
    /* A native page keeps its entry. */
    h[GRKIT_HOST_SETTINGS / 4] = 0;
    assert(menu_settings_kinds_draw(list) == FW_SUBMENU_KINDS && settings_rows(h) == 5);
    WORD(list, LIST_ROWS) = WORD(list, LIST_ROWS + 4) = 0;
}

/* The effect list: Portra 400 and Velvia 100 installed as records 0 and
   1, and Gold 200 Off, not installed. Their choices follow the settings
   of the GRKit section. The options rows of MENU (runtime/menu.c) turn
   them on or off with effects_turn. */
static void install_effect_list(void)
{
    _Static_assert(EFFECTS_STRIDE == 44 && EFFECTS_HEADER == 8, "The test list matches effects.h");
    static const char *const effect_names[] = {"Portra 400", "Velvia 100", "Gold 200"};
    volatile unsigned *list = &manage_memory[(TEST_EFFECTS - TEST_MANAGE) / 4];
    settings_memory[0] = 15;
    for (u32 i = 0; i < 3; ++i) {
        settings_memory[2 + (12 + i) * 3 + 1] = i == 2;
        settings_memory[2 + (12 + i) * 3 + 2] = EFFECT_CHOICE_VALUES;
        list[2 + i * 11] = i == 2 ? EFFECT_OFF : EFFECT_LOADED;
        list[2 + i * 11 + 1] = i == 2 ? EFFECTS_NO_RECORD : i;
        memset((void *)&list[2 + i * 11 + 2], 0, 36);
        ascii((unsigned short *)&list[2 + i * 11 + 2], effect_names[i]);
    }
    list[0] = 3; list[1] = 12;
}

static void film_look_lists_the_installed_effects_that_are_on(void)
{
    reset();
    install_effect_list();
    test_header[GRKIT_HOST_COUNT / 4] = 5;
    /* A module reads and sets CORE.TONE as a setting: n + 1 for record n. */
    assert(setting_limit(test_header, GRKIT_CORE_TONE) == 6);
    setting_store(test_header, GRKIT_CORE_TONE, 2);
    assert(setting_value(test_header, GRKIT_CORE_TONE) == 2 && tone_value(test_header) == 2);
    setting_store(test_header, GRKIT_CORE_TONE, 6);
    assert(setting_value(test_header, GRKIT_CORE_TONE) == 2);
    setting_store(test_header, GRKIT_CORE_TONE, 0);
    assert(!setting_value(test_header, GRKIT_CORE_TONE) && !test_header[GRKIT_HOST_ACTIVE / 4]);
    /* Without a list, Film Look shows Off and each record. */
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_MODULE;
    test_header[GRKIT_HOST_MODULE / 4] = IMAGE_MODULE;
    assert(count(test_header) == 6);
    /* With the list, Film Look shows Off and the installed effects that
       are On, in the order of their choices, and no other row. */
    test_header[GRKIT_HOST_EFFECTS / 4] = TEST_EFFECTS;
    assert(count(test_header) == 3 && tone_page_values(test_header) == 3);
    assert(tone_row_index(test_header, 1) == 0 && tone_row_index(test_header, 2) == 1);
    assert(MODULE(IMAGE_MODULE).parameters(test_header, 2) && !MODULE(IMAGE_MODULE).parameters(test_header, 0));
    assert(PAGE_ICON(IMAGE_MODULE, 2) == GRKIT_ICON_EFFECT(1));
    /* A new order shows at once: Velvia 100 first. */
    settings_memory[2 + 13 * 3 + 1] = 1 << 1;
    settings_memory[2 + 12 * 3 + 1] = 2 << 1;
    assert(tone_row_index(test_header, 1) == 1 && tone_row_index(test_header, 2) == 0 && !effects_changed(test_header));
    MODULE(IMAGE_MODULE).apply(test_header, 2);
    assert(tone_value(test_header) == 1 && MODULE(IMAGE_MODULE).committed(test_header) == 2);
    /* Off of the committed Portra 400 hides it at once and commits Off. On
       shows it again. */
    effects_turn(test_header, 0, 1);
    assert(effects_off(test_header, 0) && !tone_value(test_header) && count(test_header) == 2);
    assert(tone_row_index(test_header, 1) == 1 && MODULE(IMAGE_MODULE).committed(test_header) == 0);
    effects_turn(test_header, 0, 0);
    assert(!effects_off(test_header, 0) && count(test_header) == 3);
    /* Gold 200, which did not install, waits for the next start when it
       turns On. */
    effects_turn(test_header, 2, 0);
    assert(!effects_off(test_header, 2) && effects_changed(test_header) && count(test_header) == 3);
    effects_turn(test_header, 2, 1);
    assert(!effects_changed(test_header));
}

/* The MENU pages: the settings page of a module in the GRKit section,
   its options page, the GRKit section rows, and About GRKit. The listed
   modules are Film Look, Framing, and Guides, with their pages in slots
   0 to 2; their CORE.OFF, CORE.POS, and CORE.ROWS are handles 3 to 11.
   Guides has the settings rows of modules/guides. */
static void install_menu_pages(void)
{
    const u32 guides = TEST_PAGE + 2 * TEST_PAGE_SIZE, kind = 0x21c;
    volatile u32 *h = test_header, *menu;
    u32 entry;
    reset();
    install_effect_list();
    h[GRKIT_HOST_SERVICES / 4] = TEST_SERVICES;
    menu = services_word(h, SERVICES_MENU);
    /* The section is in place: no copy of the native tab. */
    menu[MENU_ROWS_DONE / 4] = 1;
    stock_kinds[(SETTINGS_SUBMENU_KIND * SUBMENU_KIND_SIZE + SUBMENU_KIND_ROW_KINDS) / 4] = kind;
    settings_memory[0] = 18;
    for (u32 i = 0; i < 3; ++i) {
        settings_memory[2 + (3 + 3 * i) * 3 + 1] = 0;
        settings_memory[2 + (3 + 3 * i) * 3 + 2] = 2;
        settings_memory[2 + (4 + 3 * i) * 3 + 1] = 0;
        settings_memory[2 + (4 + 3 * i) * 3 + 2] = MANAGE_MODULES + 1;
        settings_memory[2 + (5 + 3 * i) * 3 + 1] = 0;
        settings_memory[2 + (5 + 3 * i) * 3 + 2] = 1U << MANAGE_OPTION_VALUES;
        settings_memory[2 + (12 + i) * 3 + 1] = i == 2;
        settings_memory[2 + (12 + i) * 3 + 2] = EFFECT_CHOICE_VALUES;
    }
    const u32 limits[3] = {3, 2, 2};
    for (u32 i = 0; i < 3; ++i) {
        settings_memory[2 + (15 + i) * 3 + 1] = 0;
        settings_memory[2 + (15 + i) * 3 + 2] = limits[i];
    }
    manage_memory[0] = 3;
    manage_memory[1] = 3;
    manage_memory[2] = 0;
    for (u32 i = 0; i < 3; ++i) {
        entry = MANAGE_ENTRY(i);
        memset(&manage_memory[entry], 0, MANAGE_STRIDE);
        manage_memory[entry] = MANAGE_LOADED;
        manage_memory[entry + 1] = TEST_PAGE + i * TEST_PAGE_SIZE + PAGE_TITLES;
        manage_memory[entry + 2] = MANAGE_NO_TILE;
    }
    h[GRKIT_HOST_MANAGE / 4] = TEST_MANAGE;
    h[GRKIT_HOST_EFFECTS / 4] = TEST_EFFECTS;
    install_guide_rows(2);
    *page_at(guides + PAGE_FLAGS) |= CHOICE_OPTIONS;
}

static void settings_entry_enable_preserves_each_value(void)
{
    install_menu_pages();
    volatile u32 *h = test_header, *menu = services_word(h, SERVICES_MENU);
    const u32 address = TEST_PAGE + GUIDES_MODULE * TEST_PAGE_SIZE;
    *page_at(address + PAGE_KIND) = PAGE_SETTINGS;
    *page_at(address + PAGE_FLAGS) = 0;
    *page_at(address + PAGE_SETTING_ROWS) = 2;
    menu[MENU_ROWS_LISTED / 4] = 2;
    menu[MENU_ROWS_PAGE / 4] = GUIDES_MODULE;
    assert(listed_rows(h) == 3);
    assert(same(listed_label(h, 0), "Enable"));
    assert(same(listed_label(h, 1), "Style") && same(listed_label(h, 2), "Color"));
    listed_set(h, 1, 2);
    listed_set(h, 2, 1);
    assert(listed_get(h, 1) == 2 && listed_get(h, 2) == 1);
    listed_set(h, 0, 1);
    assert(listed_get(h, 1) == 2 && listed_get(h, 2) == 1);
    listed_set(h, 0, 0);
    assert(listed_get(h, 1) == 2 && listed_get(h, 2) == 1);
}

/* A settings-only page has no choice labels. MENU still fills its actual
   section row: a title with an empty value, or Off while disabled. */
static void a_settings_section_row_has_no_choice_value(void)
{
    install_menu_pages();
    volatile u32 *h = test_header, *menu = services_word(h, SERVICES_MENU), *row = menu_row(h, 0);
    const u32 address = TEST_PAGE + GUIDES_MODULE * TEST_PAGE_SIZE;
    *page_at(address + PAGE_KIND) = PAGE_SETTINGS;
    *page_at(address + PAGE_FLAGS) = 0;
    *page_at(address + PAGE_LABELS) = 0;
    *page_at(address + PAGE_SUMMARIES) = 0;
    *page_at(address + PAGE_SETTING_ROWS) = 2;
    menu[MENU_ROWS_COUNT / 4] = 1;
    row[MENU_ROW_KIND / 4] = MENU_ROW_LISTED;
    row[MENU_ROW_MODULE / 4] = 2;
    object_memory[(0x800 + MENU_ROW_LABEL_VIEW) / 4] = 0x5a000030;
    object_memory[(0x800 + MENU_ROW_VALUE_VIEW) / 4] = 0x5a000031;
    GUIDE_STYLE = 2;
    for (u32 disabled = 0; disabled < 2; ++disabled) {
        settings_memory[2 + 9 * 3 + 1] = disabled;
        menu_row_fill(0, TEST_OBJECTS + 0x800);
        assert(row[MENU_ROW_LABEL_TEXT_AT / 4] == ((6 + 1) | 0x100));
        assert(row[MENU_ROW_LABEL_TEXT_AT / 4 + 1] == (u32)menu_page(h, GUIDES_MODULE)->title(h));
        assert(row[MENU_ROW_VALUE_TEXT_AT / 4] == ((disabled ? 4U : 1U) | 0x100));
        assert(row[MENU_ROW_VALUE_TEXT_AT / 4 + 1] == (u32)(disabled ? on_off_text(h, 1) : no_text));
        assert(GUIDE_STYLE == 2);
    }
}

/* Three registered pages form one ADJ group and one module in MENU.
   Child settings remain editable when their first (Enable) value is zero. */
static void install_group_pages(void)
{
    install_menu_pages();
    volatile u32 *h = test_header;
    const u32 main = TEST_PAGE + GUIDES_MODULE * TEST_PAGE_SIZE, titles = main + 0x380;
    for (u32 l = 0; l < GRKIT_TEXT_LANGUAGES; ++l)
        *page_at(titles + l * 4) = *page_at(*page_at(main + PAGE_TITLE) + l * 4);
    *page_at(main) = 44;
    *page_at(main + PAGE_KIND) = PAGE_GROUP;
    *page_at(main + PAGE_FLAGS) = 0;
    *page_at(main + PAGE_TITLE) = titles;
    *page_at(main + PAGE_LABELS) = *page_at(main + PAGE_SUMMARIES) = 0;
    *page_at(main + PAGE_ROWS) = 2;
    *page_at(main + PAGE_SETTING_ROWS) = 0;
    *page_at(main + PAGE_CHILDREN) = main + 0x3c0;
    manage_memory[MANAGE_ENTRY(2) + 1] = titles;
    h[GRKIT_HOST_MODULE_PAGES / 4 + GUIDES_MODULE * 3 + 1] = 0xffffffffU;
    static const char *const enabled[] = {"Off", "On"}, *const strength[] = {"Weak", "Medium", "Strong"};
    for (u32 child = 0; child < 2; ++child) {
        u32 slot = 5 + child, handle = child ? 16 : 0, power = child ? 17 : 15;
        u32 address = TEST_PAGE + slot * TEST_PAGE_SIZE, rows = address + 0x200, texts = rows + 32;
        install_page(slot, child ? "Second" : "First", enabled, 2, handle, 7);
        *page_at(main + 0x3c0 + child * 4) = address;
        *page_at(address + PAGE_KIND) = PAGE_SETTINGS;
        *page_at(address + PAGE_FLAGS) = PAGE_MENU_ONLY;
        *page_at(address + PAGE_LABELS) = *page_at(address + PAGE_SUMMARIES) = 0;
        *page_at(address + PAGE_SETTING_ROWS) = 2;
        *page_at(address + PAGE_SETTING_LIST) = rows;
        settings_memory[2 + handle * 3 + 1] = 0;
        settings_memory[2 + handle * 3 + 2] = 2;
        settings_memory[2 + power * 3 + 1] = 1;
        settings_memory[2 + power * 3 + 2] = 3;
        for (u32 row = 0; row < 2; ++row) {
            *page_at(rows + row * ROW_SIZE) = ROW_SIZE;
            *page_at(rows + row * ROW_SIZE + ROW_SETTING) = row ? power : handle;
            *page_at(rows + row * ROW_SIZE + ROW_LABEL) = texts;
            for (u32 l = 0; l < GRKIT_TEXT_LANGUAGES; ++l)
                *page_at(texts + l * 4) = page_text(row ? "Strength" : "Enable");
            texts += TEXT_LIST;
            *page_at(rows + row * ROW_SIZE + ROW_VALUES) = texts;
            for (u32 v = 0; v < (row ? 3U : 2U); ++v)
                for (u32 l = 0; l < GRKIT_TEXT_LANGUAGES; ++l)
                    *page_at(texts + (v * GRKIT_TEXT_LANGUAGES + l) * 4) = page_text(row ? strength[v] : enabled[v]);
            texts += (row ? 3U : 2U) * TEXT_LIST;
        }
    }
}

/* A child row shows its icon for the value of its setting, and the list
   that OK opens shows each value with the icon of that value. The root
   keeps tile 0. */
static void group_icons_follow_child_values(void)
{
    install_group_pages();
    volatile u32 *h=test_header;
    u32 child=TEST_PAGE+5*TEST_PAGE_SIZE,icon=child+0x40;
    PAGE_WORD(child,0)=48; PAGE_WORD(child,44)=icon;
    PAGE_WORD(icon,0)=16; PAGE_WORD(icon,4)=1; PAGE_WORD(icon,8)=0; PAGE_WORD(icon,12)=1;
    h[GRKIT_HOST_MODULE_CHOICE/4]=GUIDES_MODULE;
    h[GRKIT_HOST_MENU_LEVEL/4]=MENU_MODULE;
    for(u32 value=0;value<2;++value) {
        settings_memory[SETTINGS_ENTRY(0)+1]=value;
        assert(group_icon(h,0)==GRKIT_ICON_MODULE_TILE(7+1+value));
        assert(group_checked(h,0)==value);
    }
    /* The value list of the child: each row the icon of its own value. */
    settings_memory[SETTINGS_ENTRY(0)+1]=0;
    assert(menu_page(h,5)==&child_module);
    assert(child_icon(h,0)==GRKIT_ICON_MODULE_TILE(8) && child_icon(h,1)==GRKIT_ICON_MODULE_TILE(9));
    /* Without an icon, Off shows the core Off icon and a value tile 0. */
    assert(menu_page(h,6)==&child_module);
    assert(child_icon(h,0)==GRKIT_ICON_OFF && child_icon(h,1)==GRKIT_ICON_MODULE_TILE(7));
    h[GRKIT_HOST_MODULE_CHOICE/4]=GUIDES_MODULE;
    h[GRKIT_HOST_MENU_LEVEL/4]=MENU_ROOT;
    assert(group_icon(h,0)==GRKIT_ICON_MODULE_TILE(7));
}

/* A choice page without an Off row shows its Off icon on row 0, as Grain
   shows its struck symbol. An Off row keeps the core Off icon, and the
   root keeps tile 0 while the setting is Off. */
static void choice_off_row_shows_the_page_off_icon(void)
{
    install_group_pages();
    volatile u32 *h=test_header;
    u32 page=TEST_PAGE+7*TEST_PAGE_SIZE,off=page+0x3e0;
    install_page(7,"Grain",grain_names,3,15,7);
    settings_memory[SETTINGS_ENTRY(15)+1]=0;
    PAGE_WORD(page,0)=48; PAGE_WORD(page,PAGE_FLAGS)=0; PAGE_WORD(page,44)=off;
    PAGE_WORD(off,0)=16; PAGE_WORD(off,4)=1; PAGE_WORD(off,8)=0xffffffffU; PAGE_WORD(off,12)=9;
    h[GRKIT_HOST_MENU_LEVEL/4]=MENU_MODULE;
    assert(menu_page(h,7)==&choice_module);
    assert(choice_icon(h,0)==GRKIT_ICON_MODULE_TILE(7+9) && choice_icon(h,1)==GRKIT_ICON_MODULE_TILE(8));
    PAGE_WORD(page,PAGE_FLAGS)=CHOICE_OFF_ROW;
    assert(choice_icon(h,0)==GRKIT_ICON_OFF);
    PAGE_WORD(page,44)=0;
    PAGE_WORD(page,PAGE_FLAGS)=0;
    assert(choice_icon(h,0)==GRKIT_ICON_MODULE_TILE(7));
    PAGE_WORD(page,44)=off;
    h[GRKIT_HOST_MENU_LEVEL/4]=MENU_ROOT;
    assert(choice_icon(h,0)==GRKIT_ICON_MODULE_TILE(7));
    settings_memory[SETTINGS_ENTRY(15)+1]=2;
    assert(choice_icon(h,0)==GRKIT_ICON_MODULE_TILE(9));
}

/* OK on a child opens the values of its first setting. OK on a value sets
   it and closes ADJ, as on a choice page; MENU returns to the child's row
   of the group without a change. */
static void group_ok_opens_each_child_and_returns_to_its_row(void)
{
    install_group_pages();
    volatile u32 *h = test_header;
    menu_start(h);
    h[GRKIT_HOST_MENU_CURSOR / 4] = root_row(h, GUIDES_MODULE);
    /* On the ADJ root, a group names itself before its value, as a choice
       page does: Off while no child is on. */
    assert(same(current_module(h)->summary(h), "Guides: Off"));
    assert(!confirm_selection(h));
    assert(group_settings_row(h) && h[GRKIT_HOST_MENU_CURSOR / 4] == 0);
    assert(!current_module(h)->checked(h, 0) && !current_module(h)->checked(h, 1));
    u32 changes = h[GRKIT_HOST_STATE_CHANGES / 4];
    assert(!confirm_selection(h));
    assert(h[GRKIT_HOST_STATE_CHANGES / 4] == changes);
    assert(h[GRKIT_HOST_MENU_LEVEL / 4] == MENU_MODULE && h[GRKIT_HOST_MODULE / 4] == 5);
    assert(current_module(h) == &child_module && count(h) == 2);
    assert(h[GRKIT_HOST_MENU_CURSOR / 4] == 0);
    assert(same(current_module(h)->title(h), "First"));
    assert(same(current_module(h)->label(h, 0), "Off") && same(current_module(h)->label(h, 1), "On"));
    assert(current_module(h)->checked(h, 0) && !current_module(h)->checked(h, 1));
    /* Up and Down keep a child open: menu_normalize finds its group. */
    h[GRKIT_HOST_MENU_CURSOR / 4] = 1;
    menu_normalize(h);
    assert(h[GRKIT_HOST_MENU_LEVEL / 4] == MENU_MODULE && h[GRKIT_HOST_MODULE / 4] == 5);
    assert(menu_back(h));
    assert(h[GRKIT_HOST_STATE_CHANGES / 4] == changes);
    assert(group_settings_row(h) && h[GRKIT_HOST_MENU_CURSOR / 4] == 0);
    /* The second child: OK on On sets it and closes ADJ. */
    h[GRKIT_HOST_MENU_CURSOR / 4] = 1;
    assert(!confirm_selection(h));
    assert(h[GRKIT_HOST_MODULE / 4] == 6 && same(current_module(h)->title(h), "Second"));
    h[GRKIT_HOST_MENU_CURSOR / 4] = 1;
    assert(confirm_selection(h));
    assert(h[GRKIT_HOST_STATE_CHANGES / 4] == changes + 1);
    assert(settings_memory[SETTINGS_ENTRY(16) + 1] == 1 && settings_memory[SETTINGS_ENTRY(0) + 1] == 0);
    /* The next ADJ opens on the row of the group. */
    menu_reopen(h);
    assert(h[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT && h[GRKIT_HOST_MENU_CURSOR / 4] == root_row(h, GUIDES_MODULE));
    /* The value of the group: the titles of the children that are on. */
    assert(same(current_module(h)->summary(h), "Guides: Second"));
    assert(same(group_value_text(h), "Second"));
    settings_memory[SETTINGS_ENTRY(0) + 1] = 1;
    assert(same(current_module(h)->summary(h), "Guides: First/Second"));
    assert(same(group_value_text(h), "First/Second"));
    /* A half press on the group list changes nothing. Fn on a child row
       opens that child's settings. */
    menu_open(h, GUIDES_MODULE);
    h[GRKIT_HOST_MENU_CURSOR / 4] = 1;
    changes = h[GRKIT_HOST_STATE_CHANGES / 4];
    exit_commit(h);
    assert(h[GRKIT_HOST_STATE_CHANGES / 4] == changes);
    assert(settings_target(h, TEST_OBJECTS) == 6);
    /* Fn on the open child also opens its settings, and the settings page
       returns to the child. */
    assert(!confirm_selection(h));
    assert(settings_target(h, TEST_OBJECTS) == 6);
    settings_start(h, 6);
    assert(settings_rows(h) == 2);
    menu_settings_controller(ADJ_CONTROLLER);
    menu_return_page(h, 1);
    assert(h[GRKIT_HOST_MENU_LEVEL / 4] == MENU_MODULE && h[GRKIT_HOST_MODULE / 4] == 6);
    assert(h[GRKIT_HOST_MENU_CURSOR / 4] == 1);
    /* The ADJ dial press closes the group list without opening a child. */
    menu_open(h, GUIDES_MODULE);
    h[GRKIT_HOST_MENU_EVENTS / 4] = 2;
    h[GRKIT_HOST_MENU_EVENT_RING / 4] = EV_ADJ_PRESS;
    assert(confirm_selection(h));
    assert(h[GRKIT_HOST_MODULE / 4] == GUIDES_MODULE && h[GRKIT_HOST_STATE_CHANGES / 4] == changes);
    h[GRKIT_HOST_MENU_EVENT_RING / 4] = 0;
    assert(menu_back(h));
    assert(h[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT);
}

static void group_pages_open_children_and_return_without_committing(void)
{
    install_group_pages();
    volatile u32 *h = test_header, *menu = services_word(h, SERVICES_MENU), *block = settings_block(h);
    manage_adj_refresh(h);
    assert(manage_adj_count(h) == 3 && listed_page(h, 2) == GUIDES_MODULE);
    assert(!(visible_modules(h) & ((1U << 5) | (1U << 6))));
    menu_start(h);
    h[GRKIT_HOST_MENU_CURSOR / 4] = root_row(h, GUIDES_MODULE);
    assert(current_module(h)->icon(h, 0) == GUIDE_TILE(0));
    menu_confirm(h);
    assert(group_settings_row(h) && count(h) == 2);
    assert(same(current_module(h)->label(h, 0), "First") && same(current_module(h)->label(h, 1), "Second"));
    assert(current_module(h)->icon(h, 0) == GUIDE_TILE(1) && current_module(h)->icon(h, 1) == GUIDE_TILE(2));
    /* A navigation group has no setting to preview, but other layers still
       paint on the ADJ canvas and retain the current controller. */
    h[GRKIT_HOST_FRAME_RATIO / 4] = 0;
    h[GRKIT_HOST_MODULE_LAYERS / 4 + 1] = 0xffffffffU;
    frame_preview_set(h, TEST_OBJECTS, 0);
    assert(!h[GRKIT_HOST_MODULE_PREVIEW / 4] && h[GRKIT_HOST_FRAME_PREVIEW / 4] == 1);
    assert(h[GRKIT_HOST_FRAME_PREVIEW_ADJ / 4] == TEST_OBJECTS);
    h[GRKIT_HOST_MODULE_LAYERS / 4 + 1] = 0;

    h[GRKIT_HOST_MENU_CURSOR / 4] = 1;
    assert(settings_target(h, TEST_OBJECTS) == 6);
    exit_commit(h);
    assert(settings_memory[2 + 16 * 3 + 1] == 0 && settings_memory[2 + 17 * 3 + 1] == 1);
    settings_start(h, 6);
    assert(settings_rows(h) == 2 && same(settings_page(h)[0].label(h), "Enable"));
    assert(menu_settings_apply(2, 0, 1) && settings_memory[2 + 17 * 3 + 1] == 2);
    assert(settings_memory[2 + 16 * 3 + 1] == 0);
    menu_settings_controller(ADJ_CONTROLLER);
    menu_reopen(h);
    menu_return_page(h, 1);
    assert(h[GRKIT_HOST_MENU_LEVEL / 4] == MENU_MODULE && h[GRKIT_HOST_MENU_CURSOR / 4] == 1);
    assert(menu_back(h) && h[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT);

    menu[MENU_ROWS_COUNT / 4] = 1;
    menu_row(h, 0)[MENU_ROW_KIND / 4] = MENU_ROW_LISTED;
    menu_row(h, 0)[MENU_ROW_MODULE / 4] = 2;
    object_memory[(0x800 + MENU_ROW_LABEL_VIEW) / 4] = 0x5a000030;
    object_memory[(0x800 + MENU_ROW_VALUE_VIEW) / 4] = 0x5a000031;
    menu_row_fill(0, TEST_OBJECTS + 0x800);
    /* The row of a group shows the children that are on: none, so Off. */
    {
        const u16 *value;
        (void)menu_page(h, GUIDES_MODULE);
        value = group_value_text(h);
        u32 length = 0;
        while (value[length]) ++length;
        assert(same(value, "Off"));
        assert(menu_row(h, 0)[MENU_ROW_VALUE_TEXT_AT / 4] == ((length + 1) | 0x100));
    }
    assert(menu_settings_controller(MENU_ROW_TARGET) == SETTINGS_SUBMENU);
    assert(settings_rows(h) == 3 && same(listed_label(h, 0), "Enable"));
    assert(same(listed_label(h, 1), "First") && same(listed_label(h, 2), "Second"));
    for (u32 row = 1; row <= 2; ++row) {
        assert(block[SUBMENU_KIND_CONTROLLERS / 4 + row] == MENU_GROUP_TARGET + row);
        assert(menu_settings_controller(MENU_GROUP_TARGET + row) == SETTINGS_SUBMENU);
        assert((h[GRKIT_HOST_SETTINGS / 4] & 0xffffU) == 5 + row);
        assert(h[GRKIT_HOST_SETTINGS / 4] & SETTINGS_PARENT);
        assert(settings_rows(h) == 2);
        assert(menu_settings_controller(settings_pages[1]) == settings_pages[1]);
        assert(menu_settings_apply(0, 0, 1));
        assert(menu_settings_controller(SETTINGS_SUBMENU) == SETTINGS_SUBMENU);
        assert((h[GRKIT_HOST_SETTINGS / 4] & 0xffffU) == 5 + row);
        assert(h[GRKIT_HOST_SETTINGS / 4] & SETTINGS_PARENT);
        /* The firmware remembers the shared submenu as the return target.
           Unlike the value-page return above, MENU comes from the submenu
           itself: child -> parent, then parent -> the top-level MENU. */
        assert(menu[MENU_ROWS_CURRENT / 4] == SETTINGS_SUBMENU);
        assert(menu_settings_controller(SETTINGS_SUBMENU) == SETTINGS_SUBMENU);
        assert(h[GRKIT_HOST_SETTINGS / 4] == LISTED_PAGE + 1 && settings_rows(h) == 3);
    }
    assert(menu_settings_controller(SETTINGS_SUBMENU) == MENU_RETURN_CONTROLLER);
    assert(!h[GRKIT_HOST_SETTINGS / 4]);
    assert(menu[MENU_ROWS_CURRENT / 4] == MENU_RETURN_CONTROLLER);
    /* Targets that return directly to MENU still unwind the child first. */
    assert(menu_settings_controller(MENU_ROW_TARGET) == SETTINGS_SUBMENU);
    assert(menu_settings_controller(MENU_GROUP_TARGET + 1) == SETTINGS_SUBMENU);
    assert(menu_settings_controller(MENU_RETURN_CONTROLLER) == SETTINGS_SUBMENU);
    assert(h[GRKIT_HOST_SETTINGS / 4] == LISTED_PAGE + 1);
    assert(menu_settings_controller(MENU_RETURN_CONTROLLER) == MENU_RETURN_CONTROLLER);
    assert(!h[GRKIT_HOST_SETTINGS / 4]);
}

/* BaseController::Back marks the source before queuing ChangeController.
   Completion runs after Start, and invalidates the source's return target.
   For a self transition, that source is also the newly opened parent. */
static u32 native_settings_back(u32 controller)
{
    u32 target = MENU_OBJECT_WORD(controller, SETTINGS_RETURN_TARGET);
    assert(target != 0x10000000U);
    MENU_OBJECT_BYTE(controller, SETTINGS_BACK_PENDING) = 1;
    target = menu_settings_transition(controller, target, 0);
    if (MENU_OBJECT_BYTE(controller, SETTINGS_BACK_PENDING))
        MENU_OBJECT_WORD(controller, SETTINGS_RETURN_TARGET) = 0x10000000U;
    MENU_OBJECT_BYTE(controller, SETTINGS_BACK_PENDING) = 0;
    return target;
}

static void group_back_keeps_the_shared_native_parent_return_target(void)
{
    install_group_pages();
    volatile u32 *h = test_header, *menu = services_word(h, SERVICES_MENU);
    const u32 controller = TEST_OBJECTS + 0x3000;
    menu[MENU_ROWS_COUNT / 4] = 1;
    menu_row(h, 0)[MENU_ROW_KIND / 4] = MENU_ROW_LISTED;
    menu_row(h, 0)[MENU_ROW_MODULE / 4] = 2;
    for (u32 child = 1; child <= 2; ++child) {
        assert(menu_settings_transition(controller, MENU_ROW_TARGET, 0) == SETTINGS_SUBMENU);
        MENU_OBJECT_WORD(controller, SETTINGS_RETURN_TARGET) = MENU_RETURN_CONTROLLER;
        assert(menu_settings_transition(controller, MENU_GROUP_TARGET + child, 0) == SETTINGS_SUBMENU);
        assert(!MENU_OBJECT_BYTE(controller, SETTINGS_BACK_PENDING));
        /* A native forward self transition stores its own id as the child
           return target. Also cover a direct MENU return from the child. */
        MENU_OBJECT_WORD(controller, SETTINGS_RETURN_TARGET) = child == 1 ? SETTINGS_SUBMENU : MENU_RETURN_CONTROLLER;
        assert(native_settings_back(controller) == SETTINGS_SUBMENU);
        assert(h[GRKIT_HOST_SETTINGS / 4] == LISTED_PAGE + 1);
        assert(MENU_OBJECT_WORD(controller, SETTINGS_RETURN_TARGET) == MENU_RETURN_CONTROLLER);
        assert(native_settings_back(controller) == MENU_RETURN_CONTROLLER);
        assert(!h[GRKIT_HOST_SETTINGS / 4]);
        assert(MENU_OBJECT_WORD(controller, SETTINGS_RETURN_TARGET) == 0x10000000U);
    }
    /* A value page returning to a child keeps the parent marker, but must
       still release that value controller's native Back target. */
    assert(menu_settings_transition(controller, MENU_ROW_TARGET, 0) == SETTINGS_SUBMENU);
    assert(menu_settings_transition(controller, MENU_GROUP_TARGET + 1, 0) == SETTINGS_SUBMENU);
    assert(menu_settings_transition(controller, settings_pages[1], 0) == settings_pages[1]);
    MENU_OBJECT_WORD(controller, SETTINGS_RETURN_TARGET) = SETTINGS_SUBMENU;
    assert(native_settings_back(controller) == SETTINGS_SUBMENU);
    assert(h[GRKIT_HOST_SETTINGS / 4] & SETTINGS_PARENT);
    assert(MENU_OBJECT_WORD(controller, SETTINGS_RETURN_TARGET) == 0x10000000U);
    /* ADJ opens the same child without a MENU parent. Its return is native. */
    settings_start(h, 5);
    menu[MENU_ROWS_CURRENT / 4] = SETTINGS_SUBMENU;
    MENU_OBJECT_WORD(controller, SETTINGS_RETURN_TARGET) = ADJ_CONTROLLER;
    assert(native_settings_back(controller) == ADJ_CONTROLLER);
    assert(MENU_OBJECT_WORD(controller, SETTINGS_RETURN_TARGET) == 0x10000000U);
}

/* The MENU page of Recipes (runtime/menu_recipes.h): Enable, Options,
   Recipes, whose value page lists the recipes and shows the page of the
   recipe that OK chooses, then Save Settings. */
static void the_recipes_menu_page_chooses_a_recipe_then_an_action(void)
{
    install_menu_pages();
    volatile u32 *h = test_header, *menu = services_word(h, SERVICES_MENU), *block = settings_block(h);
    volatile u32 *state = menu + MENU_ROWS_PAGE_STATE / 4;
    const u32 buffer = TEST_OBJECTS + 0x200U, controller = TEST_OBJECTS + 0x800U;
    install_page(4, "Recipes", 0, 0, GRKIT_CORE_RECIPE, 19);
    h[GRKIT_HOST_CAPABILITIES / 4] |= GRKIT_CORE_RECIPE_BIT;
    h[GRKIT_HOST_ACTIVE / 4] = 0;
    h[GRKIT_HOST_GRAIN / 4] = 3;
    /* The own page without a canvas shows no box. */
    MENU_OBJECT_WORD(controller, ADJ_CANVAS) = 0;
    header_recipe(0, "Far Away", RECIPE_USED | RECIPE_HIDDEN | RECIPE_LOOK_OFF | RECIPE_GRAIN | 4U << RECIPE_GRAIN_LEVEL_SHIFT);
    header_recipe(1, "Street", RECIPE_USED | RECIPE_LOOK_OFF | RECIPE_GRAIN);
    header_recipe(2, "Corner", RECIPE_USED | RECIPE_LOOK_OFF | RECIPE_GRAIN | 2U << RECIPE_GRAIN_LEVEL_SHIFT);
    /* A recipe that is off stays for the site: no list of the camera has it. */
    header_recipe(3, "Archive", RECIPE_USED | RECIPE_LOOK_OFF);
    h[GRKIT_HOST_RECIPES / 4 + 3 * GRKIT_RECIPE_WORDS + RECIPE_MODULE_WORD] = RECIPE_OFF;
    menu[MENU_ROWS_LISTED / 4] = 0;
    menu[MENU_ROWS_PAGE / 4] = 4;
    /* The Options row shows as the manifest declares it (options = true). */
    settings_open(h, LISTED_PAGE, 0);
    assert(settings_rows(h) == 3 && same(listed_label(h, 1), "Recipes"));
    menu[MENU_ROWS_CURRENT / 4] = SETTINGS_SUBMENU;
    *page_at(TEST_PAGE + 4 * TEST_PAGE_SIZE + PAGE_FLAGS) |= CHOICE_OPTIONS;
    *state = 7;
    assert(menu_settings_controller(SETTINGS_SUBMENU) == SETTINGS_SUBMENU);
    /* A start of the settings page clears the state of the value page. */
    assert(!*state);
    const Setting *page = settings_page(h);
    assert(settings_rows(h) == 4 && same(listed_label(h, 1), "Options"));
    assert(same(listed_label(h, 2), "Recipes") && same(listed_label(h, 3), "Save Settings"));
    assert(same(listed_text(h, 3, 0), "3/40"));
    /* Options: each recipe, on while ADJ shows it. OK turns it on or off. */
    assert(options_count(h) == 3 && option_off(h, 0) && !option_off(h, 1));
    assert(same(option_label(h, 1), "Street"));
    listed_set(h, 1, 0);
    assert(recipe_shown(h, 0));
    listed_set(h, 1, 0);
    assert(!recipe_shown(h, 0));
    menu[MENU_ROWS_STAY / 4] = 0;
    /* Recipes shows on the own value page; Save acts at OK. */
    for (u32 row = 0; row < 4; ++row) {
        u32 flags = settings_flags(h, page, row);
        assert(!!(flags & SETTING_OWN_PAGE) == (row == 2));
        assert(settings_long(h, page, row) == (row == 1 || row == 2));
        assert(!!(flags & SETTING_ACTION) == (row == 3));
    }
    assert(listed_setting(h, 0) == 0 && listed_setting(h, 1) == 0);
    assert(menu_settings_row_controller(settings_pages[0], 3) == MENU_ACTION_TARGET + 3);
    assert(block[SUBMENU_KIND_CONTROLLERS / 4 + 2] == settings_pages[2]);
    const Setting *recipes = listed_setting(h, 2);
    /* The list has each recipe, also one that ADJ does not show. Without a
       recipe that the settings match, the row shows no value, the page has
       no mark, and the cursor is on the first recipe. */
    assert(listed_values(h, 2) == 3 && same(listed_text(h, 2, 0), "Far Away") && same(listed_text(h, 2, 2), "Corner"));
    assert(listed_get(h, 2) == RECIPE_NONE && same(listed_text(h, 2, listed_get(h, 2)), ""));
    assert(recipes->cursor(h) == 0 && !recipes->title(h));
    /* Grain 2 matches Corner: the row shows it, and the list has the mark
       and the cursor on it. */
    h[GRKIT_HOST_GRAIN / 4] = 2;
    assert(listed_get(h, 2) == 2 && recipes->cursor(h) == 2 && same(listed_text(h, 2, 2), "Corner"));
    assert(menu_settings_current(2, 0, 2) == 2 && menu_settings_current(1, 0, 2) == 1);
    /* MENU lists the recipes that ADJ does not show, so its mark can be on
       one of them, while ADJ names the first recipe that it shows. */
    volatile u32 *far = h + GRKIT_HOST_RECIPES / 4;
    far[0] = (far[0] & ~(7U << RECIPE_GRAIN_LEVEL_SHIFT)) | 2U << RECIPE_GRAIN_LEVEL_SHIFT;
    assert(listed_get(h, 2) == 0 && recipes->cursor(h) == 0 && same(recipe_menu_value(h), "Far Away"));
    assert(recipe_current(h) == 2);
    far[0] = (far[0] & ~(7U << RECIPE_GRAIN_LEVEL_SHIFT)) | 4U << RECIPE_GRAIN_LEVEL_SHIFT;
    assert(listed_get(h, 2) == 2);
    /* MENU on the list returns to the page as MENU does. */
    assert(!recipes->select(h, 0, SETTING_BACK) && !recipes->select(h, 0, 3));
    /* OK on Street shows its page: its name, Apply, Rename, and Delete,
       the cursor on Apply and no mark. */
    assert(recipes->select(h, 0, 1));
    assert(*state == (RECIPE_PAGE_SHOWN | 1U) && same(recipes->title(h), "Street"));
    assert(listed_values(h, 2) == 3 && same(listed_text(h, 2, 0), "Apply"));
    assert(same(listed_text(h, 2, 1), "Rename") && same(listed_text(h, 2, 2), "Delete"));
    assert(recipes->cursor(h) == 0 && recipes->get(h) == RECIPE_NONE);
    menu[MENU_ROWS_CURRENT / 4] = settings_pages[2];
    assert(settings_text(h, 0, settings_row_texts[2]) == (u32)services_word(h, SERVICES_MENU + MENU_ROWS_PAGE_TEXTS) + 1U);
    menu[MENU_ROWS_CURRENT / 4] = SETTINGS_SUBMENU;
    /* MENU returns to the list with the cursor on Street, which the
       settings do not match. */
    assert(recipes->select(h, 0, SETTING_BACK));
    assert(*state == 2 && recipes->cursor(h) == 1 && listed_get(h, 2) == 2 && same(listed_text(h, 2, 1), "Street"));
    /* Rename: the return from the keyboard names the recipe after an OK,
       keeps the name after MENU, and the page shows the new name. The
       change from the keyboard calls the callback (ui_keyboard_change); a
       host pointer does not fit the services word, so the test calls it. */
    assert(recipes->select(h, 0, 1));
    ui_word(h, UI_KEYBOARD)[0] = UI_KEYBOARD_ARMED;
    assert(menu_keyboard_text_in(0, buffer));
    for (u32 i = 0; i < 7; ++i) BYTE(buffer, i) = (u8)"Night"[i];
    assert(menu_keyboard_text_out(0, buffer));
    recipe_menu_renamed(1);
    assert(same(recipes->title(h), "Night"));
    ui_word(h, UI_KEYBOARD_RESULT)[0] = 0;
    recipe_menu_renamed(1);
    assert(same(recipes->title(h), "Night") && *state == (RECIPE_PAGE_SHOWN | 1U));
    ui_word(h, UI_KEYBOARD)[0] = UI_KEYBOARD_ARMED;
    ui_keyboard_change(h, KEYBOARD_CONTROLLER);
    assert(ui_keyboard_borrowed(h));
    ui_keyboard_change(h, settings_pages[2]);
    assert(!ui_keyboard_borrowed(h));
    /* The page keeps the settings while the keyboard that it opened shows. */
    ui_word(h, UI_KEYBOARD)[0] = UI_KEYBOARD_ARMED;
    assert(menu_settings_controller(KEYBOARD_CONTROLLER) == KEYBOARD_CONTROLLER);
    assert(h[GRKIT_HOST_SETTINGS / 4] == LISTED_PAGE + 1);
    ui_word(h, UI_KEYBOARD)[0] = 0;
    menu[MENU_ROWS_CURRENT / 4] = SETTINGS_SUBMENU;
    /* Apply applies the recipe and returns to the Recipes page, whose row
       shows it. */
    assert(recipes->select(h, 0, 0));
    assert(h[GRKIT_HOST_GRAIN / 4] == 0 && recipe_undo_count(h) == 1 && !*state);
    assert(listed_get(h, 2) == 1 && same(listed_text(h, 2, listed_get(h, 2)), "Night"));
    /* Delete asks first: Cancel keeps the recipe, and Execute returns to
       the list with the cursor on the recipe after it. */
    assert(recipes->select(h, 0, 1) && recipes->select(h, controller, 2));
    recipe_menu_deleted(0, 1);
    assert(recipe_entries(h) == 3 && *state == (RECIPE_PAGE_SHOWN | 1U));
    recipe_menu_deleted(1, 1);
    assert(recipe_entries(h) == 2 && *state == 2 && recipes->cursor(h) == 1 && same(listed_text(h, 2, 1), "Corner"));
    /* The last recipe: the cursor goes to the one before it. */
    assert(recipes->select(h, 0, 1) && recipes->select(h, controller, 2));
    recipe_menu_deleted(1, 1);
    assert(recipe_entries(h) == 1 && *state == 1 && recipes->cursor(h) == 0);
    /* After the last recipe, the page returns to the Recipes page, which
       has Enable and Save Settings. */
    assert(recipes->select(h, 0, 0) && recipes->select(h, controller, 2));
    recipe_menu_deleted(1, 0);
    assert(!recipe_entries(h) && !*state && recipe_count(h) == 1);
    settings_open(h, LISTED_PAGE, 0);
    assert(settings_rows(h) == 2 && same(listed_label(h, 1), "Save Settings"));
    assert(settings_flags(h, listed_settings, 1) == SETTING_ACTION);
    menu[MENU_ROWS_CURRENT / 4] = SETTINGS_SUBMENU;
}

/* A camera whose MENU has another controller ID, as GR IV: the parent page
   returns to the controller that the submenu holds, not to the one of GR III,
   which is another screen there. */
static void group_back_returns_to_the_menu_of_the_camera(void)
{
    install_group_pages();
    volatile u32 *h = test_header, *menu = services_word(h, SERVICES_MENU);
    const u32 controller = TEST_OBJECTS + 0x3000, top = 0x2000e00U;
    menu[MENU_ROWS_COUNT / 4] = 1;
    menu_row(h, 0)[MENU_ROW_KIND / 4] = MENU_ROW_LISTED;
    menu_row(h, 0)[MENU_ROW_MODULE / 4] = 2;
    assert(menu_settings_transition(controller, MENU_ROW_TARGET, 0) == SETTINGS_SUBMENU);
    MENU_OBJECT_WORD(controller, SETTINGS_RETURN_TARGET) = top;
    assert(menu_return(h) == MENU_RETURN_CONTROLLER);
    /* The change to the child notes the controller. */
    assert(menu_settings_transition(controller, MENU_GROUP_TARGET + 1, 0) == SETTINGS_SUBMENU);
    assert(menu_return(h) == top);
    MENU_OBJECT_WORD(controller, SETTINGS_RETURN_TARGET) = SETTINGS_SUBMENU;
    assert(native_settings_back(controller) == SETTINGS_SUBMENU);
    assert(h[GRKIT_HOST_SETTINGS / 4] == LISTED_PAGE + 1);
    assert(MENU_OBJECT_WORD(controller, SETTINGS_RETURN_TARGET) == top);
    assert(native_settings_back(controller) == top);
    assert(!h[GRKIT_HOST_SETTINGS / 4]);
}

static void the_guides_menu_page_lists_enable_options_and_settings(void)
{
    install_menu_pages();
    static const u32 pages[] = SETTINGS_PAGES;
    static const u16 row_texts[] = SETTINGS_ROW_TEXTS;
    volatile u32 *h = test_header, *block = settings_block(test_header), *menu = services_word(test_header, SERVICES_MENU);
    const u32 guides = TEST_PAGE + 2 * TEST_PAGE_SIZE, view = 0x5a000000, kind = 0x21c;
    const Setting *page;
    /* The page of Guides has six rows: Enable, Options, then the four
       settings of the module. */
    menu[MENU_ROWS_LISTED / 4] = 2;
    menu[MENU_ROWS_PAGE / 4] = 2;
    settings_open(h, LISTED_PAGE, 0);
    /* The page shows: the change to the borrowed submenu. */
    assert(menu_settings_controller(SETTINGS_SUBMENU) == SETTINGS_SUBMENU);
    page = settings_page(h);
    assert(page == listed_settings && settings_rows(h) == SETTING_ROWS);
    assert(same(page[0].label(h), "Enable") && same(page[1].label(h), "Options"));
    assert(same(page[2].label(h), "Style") && same(page[3].label(h), "Color"));
    assert(same(page[4].label(h), "Opacity") && same(page[5].label(h), "Hide with DISP"));
    /* Without options, the page has five rows. */
    *page_at(guides + PAGE_FLAGS) &= ~CHOICE_OPTIONS;
    assert(listed_rows(h) == 5 && same(page[1].label(h), "Style"));
    *page_at(guides + PAGE_FLAGS) |= CHOICE_OPTIONS;
    /* The entry has room for the controllers and kinds of five rows. The
       last row keeps its label in the entry, and its value method after
       it. */
    for (u32 i = 0; i < SETTING_ROWS - 1; ++i) assert(block[SUBMENU_KIND_CONTROLLERS / 4 + i] == pages[i]);
    assert(((volatile u16 *)((volatile u8 *)block + SUBMENU_KIND_LABELS))[SETTING_ROWS - 1] == row_texts[SETTING_ROWS - 1]);
    assert(block[SUBMENU_KIND_VALUES / 4 + 2 * (SETTING_ROWS - 1)] == (u32)(unsigned long)menu_settings_value_5);
    assert(block[SUBMENU_KIND_ROW_KINDS / 4] == kind);
    /* The value method of the last row shows its value. */
    object_memory[ROW_VALUE_VIEW / 4] = view + 5;
    view_calls = 0;
    menu_settings_value_5(0, TEST_OBJECTS);
    assert(block[SETTINGS_VIEWS / 4 + 5 * 3] == view + 5 && view_called(FW_ROW_VALUE));
    assert(menu_text_override(view + 5, SETTINGS_TITLE_TEXT) == ((u32)(block + SETTINGS_VIEWS / 4 + 5 * 3 + 1) | 1U));
    /* OK on the last row opens the value page of SETTINGS_ALIAS for the
       last row: its title, its values, and its setting. */
    assert(menu_settings_row_controller(kind, SETTING_ROWS - 1) == pages[SETTINGS_ALIAS]);
    assert(menu_settings_controller(pages[SETTINGS_ALIAS]) == pages[SETTINGS_ALIAS]);
    assert(h[GRKIT_HOST_SETTINGS / 4] >> SETTINGS_OPEN_ROW == SETTING_ROWS - 1);
    assert(menu_settings_count(0, 0, SETTINGS_ALIAS) == 3);
    /* A value page shows at most the rows that each SelectMenu has views for. */
    assert(settings_memory[2 + 17 * 3 + 2] == 2);
    settings_memory[2 + 17 * 3 + 2] = SELECT_MENU_ROWS + 9;
    assert(menu_settings_count(0, 0, SETTINGS_ALIAS) == SELECT_MENU_ROWS + 1);
#if defined(FW_CHECK_ITEM) && defined(FW_MENU_TABS)
    {
        /* A long value page shows on the own radio page: the factory builds
           it once with the rows of the constructor, then appends a radio
           row view for each value, so the page shows every value. */
        volatile u32 *check = services_word(h, SERVICES_CHECK);
        const u32 values = SELECT_MENU_ROWS + 9;
        assert(menu_settings_controller(pages[SETTINGS_ALIAS]) == pages[SETTINGS_ALIAS]);
        assert(check[CHECK_ID / 4] == pages[SETTINGS_ALIAS] && !check[CHECK_RADIO / 4]);
        MENU_OBJECT_BYTE(TEST_CHECK_KINDS, 0) = 0x15;
        view_calls = 0;
        assert(menu_check_controller(0, pages[SETTINGS_ALIAS]) == TEST_CHECK_PAGE);
        assert(check[CHECK_RADIO / 4] == TEST_CHECK_PAGE && !check[CHECK_PAGE / 4] && !check[CHECK_BUILD / 4]);
        assert(MENU_OBJECT_BYTE(TEST_CHECK_KINDS, 0) == 0x15 && check[CHECK_RADIO_TYPE / 4] == 0x15);
        int at = view_index(FW_LIST_INIT);
        assert(at >= 0 && view_args[at + 1][0] == TEST_CHECK_PAGE + 0x100);
        assert(view_args[at + 1][2] == values - 3 && view_args[at + 1][3] == 50);
        assert(menu_settings_count(0, 0, SETTINGS_ALIAS) == values + 1);
        /* The next open reuses the page and its rows. */
        view_calls = 0;
        assert(menu_check_controller(0, pages[SETTINGS_ALIAS]) == TEST_CHECK_PAGE && !view_calls);
        /* A short page gets the native controller again. */
        settings_memory[2 + 17 * 3 + 2] = 2;
        assert(menu_settings_controller(pages[SETTINGS_ALIAS]) == pages[SETTINGS_ALIAS] && !check[CHECK_ID / 4]);
        assert(menu_check_controller(0, pages[SETTINGS_ALIAS]) == 0);
        check[CHECK_RADIO / 4] = 0;
        check[CHECK_LISTS / 4 + 2 * CHECK_PAGE_RADIO] = check[CHECK_LISTS / 4 + 2 * CHECK_PAGE_RADIO + 1] = 0;
    }
#endif
    settings_memory[2 + 17 * 3 + 2] = 2;
    assert(menu_text_override(view, row_texts[SETTINGS_ALIAS]) ==
           ((u32)(block + SETTINGS_LABELS / 4 + SETTING_ROWS * 2) | 1U));
    assert(menu_settings_apply(1, 0, SETTINGS_ALIAS) == 1 && settings_memory[2 + 17 * 3 + 1] == 1);
    assert(menu_settings_current(1, 0, SETTINGS_ALIAS) == 2 && menu_settings_current(0, 0, SETTINGS_ALIAS) == 1);
    settings_memory[2 + 17 * 3 + 1] = 0;
    /* The return to the page ends it, and the page reads its rows again. */
    assert(menu_settings_controller(SETTINGS_SUBMENU) == SETTINGS_SUBMENU);
    assert(!(h[GRKIT_HOST_SETTINGS / 4] & SETTINGS_OPEN_ROWS) && settings_rows(h) == SETTING_ROWS);
}

static void the_options_page_toggles_its_values(void)
{
    static const u32 pages[] = SETTINGS_PAGES;
    static const u16 row_texts[] = SETTINGS_ROW_TEXTS;
    volatile u32 *h = test_header, *menu = services_word(test_header, SERVICES_MENU);
    const u32 view = 0x5a000000;
    /* The options page lists the guide styles but Off. Its title names
       the module; MENU reads Back; OK keeps the native word in English
       and reads Toggle in Chinese. */
    assert(menu_settings_controller(pages[1]) == pages[1] && !menu[MENU_ROWS_STAY_ROW / 4]);
    assert(menu_settings_count(0, 0, 1) == 6);
    assert(menu_text_override(view, row_texts[1]) ==
           ((u32)services_word(h, SERVICES_MENU + MENU_ROWS_PAGE_TEXTS) | 1U));
    assert(same((const u16 *)services_word(h, SERVICES_MENU + MENU_ROWS_PAGE_TITLE), "Guides Options"));
    assert(!menu_text_override(view, SELECT_GUIDE_OK_TEXT));
    assert(menu_text_override(view, SELECT_GUIDE_BACK_TEXT) ==
           ((u32)services_word(h, SERVICES_MENU + MENU_ROWS_PAGE_TEXTS + 16) | 1U));
    test_language = 1;
    assert(menu_text_override(view, SELECT_GUIDE_OK_TEXT) ==
           ((u32)services_word(h, SERVICES_MENU + MENU_ROWS_PAGE_TEXTS + 8) | 1U));
    menu_text_override(view, row_texts[1]);
    assert(((const u16 *)services_word(h, SERVICES_MENU + MENU_ROWS_PAGE_TITLE))[6] == text(h, TEXT_OPTIONS)[0]);
    /* Korean puts a space between the words, as English does. */
    test_language = 4;
    menu_text_override(view, row_texts[1]);
    assert(((const u16 *)services_word(h, SERVICES_MENU + MENU_ROWS_PAGE_TITLE))[6] == ' ');
    assert(((const u16 *)services_word(h, SERVICES_MENU + MENU_ROWS_PAGE_TITLE))[7] == text(h, TEXT_OPTIONS)[0]);
    test_language = 0;
    /* Each option keeps the view and the text of its row. */
    for (u32 k = 0; k < 5; ++k) {
        object_memory[(0x100 + k * 0x40 + ROW_TEXT_VIEW) / 4] = view + 0x10 + k;
        assert(menu_settings_draw(TEST_OBJECTS + 0x100 + k * 0x40, k, 1) == 1);
        assert(menu_text_override(view + 0x10 + k, SETTINGS_TITLE_TEXT) ==
               ((u32)(value_entry(h, k) + MENU_VALUE_TEXT_AT / 4) | 1U));
    }
    assert(same((const u16 *)(value_entry(h, 1) + MENU_VALUE_TEXT / 4), "Center"));
    /* OK turns Center off: the bit of its value in CORE.ROWS of Guides,
       the text of its row, and the page stays with the cursor on it. */
    assert(menu_settings_apply(1, 0, 1) == 1);
    assert(settings_memory[2 + 11 * 3 + 1] == 1U << 2);
    assert(same((const u16 *)(value_entry(h, 1) + MENU_VALUE_TEXT / 4), "Center (Off)"));
    /* Korean writes Off as English does: a space and ASCII parentheses. */
    test_language = 4;
    {
        const u16 *korean = option_text(h, 1), *off = text(h, TEXT_OFF);
        assert(korean[6] == ' ' && korean[7] == '(' && korean[8] == off[0] && korean[9] == off[1]);
        assert(!off[2] && korean[10] == ')' && !korean[11]);
    }
    test_language = 0;
    assert(same(option_text(h, 1), "Center (Off)"));
    assert(menu[MENU_ROWS_STAY / 4] && menu[MENU_ROWS_STAY_ROW / 4] == 1);
    assert(menu_settings_current(1, 0, 1) == 2 && menu_settings_current(0, 0, 1) == 1);
    assert(menu_settings_controller(pages[1]) == 0 && !menu[MENU_ROWS_STAY / 4]);
    assert(menu_settings_apply(1, 0, 1) == 1 && !settings_memory[2 + 11 * 3 + 1]);
    assert(same((const u16 *)(value_entry(h, 1) + MENU_VALUE_TEXT / 4), "Center"));
    menu_settings_controller(pages[1]);
    assert(menu_settings_controller(SETTINGS_SUBMENU) == SETTINGS_SUBMENU);
}

#if defined(FW_CHECK_ITEM) && defined(FW_MENU_TABS)
static void the_options_page_shows_checkboxes(void)
{
    static const u32 pages[] = SETTINGS_PAGES;
    volatile u32 *h = test_header;
    const u32 view = 0x5a000000;
    /* The options page shows a checkbox on each option. The return to
       the settings page keeps the checkbox page for it, so the cursor
       finds its row. */
    volatile u32 *check = services_word(h, SERVICES_CHECK);
    const u32 rows = TEST_OBJECTS + 0x2000;
    assert(check[CHECK_ID / 4] == pages[1]);
    /* The factory gives the native controller to any other id. */
    assert(menu_check_controller(0, pages[2]) == 0 && menu_check_controller(0, 0) == 0);
    /* The first open builds the checkbox page once, with checkbox
       rows; the next open reuses it. */
    view_calls = 0;
    assert(menu_check_controller(0, pages[1]) == TEST_CHECK_PAGE);
    assert(view_index(FW_ALLOCATE) == 0 && view_args[0][0] == SELECT_MENU_SIZE);
    assert(view_index(FW_SELECT_MENU_NEW) == 1 && view_index(FW_LIST_INIT) == 2);
    for (u32 k = 0; k < 3; ++k) assert(MENU_OBJECT_BYTE(TEST_CHECK_KINDS, k) == CHECK_LINE_TYPE);
    assert(check[CHECK_PAGE / 4] == TEST_CHECK_PAGE && !check[CHECK_BUILD / 4]);
    view_calls = 0;
    assert(menu_check_controller(0, pages[1]) == TEST_CHECK_PAGE && !view_calls);
    /* Each open sets the title ID of the page again: own pages share one
       title ID, so the title measures the text of this page. */
    const u32 title = TEST_OBJECTS + 0x3c00U;
    MENU_OBJECT_WORD(TEST_CHECK_PAGE, SELECT_TITLE_STRING) = title;
    MENU_OBJECT_HALF(title, STRING_VIEW_TEXT) = 0x217;
    assert(menu_check_controller(0, pages[1]) == TEST_CHECK_PAGE);
    assert(view_calls == 2 && view_functions[0] == FW_STRING_VIEW_RESET && view_args[0][0] == title &&
           !view_args[0][1] && view_functions[1] == FW_STRING_VIEW_RESET && view_args[1][0] == title &&
           view_args[1][1] == 0x217 && !view_args[1][2]);
    MENU_OBJECT_WORD(TEST_CHECK_PAGE, SELECT_TITLE_STRING) = 0;
    MENU_OBJECT_HALF(title, STRING_VIEW_TEXT) = 0;
    /* A native list keeps its radio rows. */
    MENU_OBJECT_BYTE(TEST_CHECK_KINDS, 0) = 0x15;
    menu_check_lines(TEST_OBJECTS + 0x3400, TEST_CHECK_KINDS, 1, 50);
    assert(MENU_OBJECT_BYTE(TEST_CHECK_KINDS, 0) == 0x15);
    /* The radio dot shows on a native row, not on a checkbox row. */
    object_memory[(0x2000) / 4] = CHECK_LINE_VTABLE;
    object_memory[(0x2100) / 4] = 0;
    view_calls = 0;
    menu_check_dot(rows, 1);
    assert(!view_calls);
    menu_check_dot(rows + 0x100, 1);
    assert(view_index(FW_ROW_DOT) == 0 && view_args[0][0] == rows + 0x100);
    /* Each option row: the checkbox shows whether the option is on,
       and the text is its label without "(Off)". Center is off. */
    assert(menu_settings_controller(pages[1]) == pages[1]);
    assert(menu_settings_apply(1, 0, 1) == 1 && settings_memory[2 + 11 * 3 + 1] == 1U << 2);
    menu_settings_controller(pages[1]);
    for (u32 k = 0; k < 2; ++k) {
        const u32 row = rows + k * 0x100;
        object_memory[(row - TEST_OBJECTS) / 4] = CHECK_LINE_VTABLE;
        object_memory[(row - TEST_OBJECTS + CHECK_TEXT_VIEW) / 4] = view + 0x30 + k;
        view_calls = 0;
        assert(menu_settings_draw(row, k, 1) == 1);
        int at = view_index(FW_CHECK_ITEM);
        assert(at >= 0 && view_args[at][0] == row && view_args[at][3] == SETTINGS_TITLE_TEXT);
        assert(view_args[at][1] == (k == 1 ? CHECK_IMAGE_OFF : CHECK_IMAGE_ON));
        assert(view_args[at][2] == view_args[at][1]);
        assert(menu_text_override(view + 0x30 + k, SETTINGS_TITLE_TEXT) ==
               ((u32)(value_entry(h, k) + MENU_VALUE_TEXT_AT / 4) | 1U));
    }
    assert(same((const u16 *)(value_entry(h, 1) + MENU_VALUE_TEXT / 4), "Center"));
    /* A row past the values draws nothing. */
    view_calls = 0;
    assert(menu_settings_draw(rows, 9, 1) == 1 && !view_called(FW_CHECK_ITEM));
    assert(menu_settings_apply(1, 0, 1) == 1 && !settings_memory[2 + 11 * 3 + 1]);
    menu_settings_controller(pages[1]);
    object_memory[0x2000 / 4] = object_memory[0x2100 / 4] = 0;
    assert(menu_settings_controller(SETTINGS_SUBMENU) == SETTINGS_SUBMENU);
    assert(check[CHECK_ID / 4] == pages[1]);
    /* A page without options, and any page outside the settings
       page, get the native controller. */
    assert(menu_settings_controller(pages[2]) == pages[2] && !check[CHECK_ID / 4]);
    assert(menu_settings_controller(SETTINGS_SUBMENU) == SETTINGS_SUBMENU);
}
#endif

static void reused_row_views_show_the_page_that_reuses_them(void)
{
    static const u32 pages[] = SETTINGS_PAGES;
    volatile u32 *block = settings_block(test_header);
    const u32 view = 0x5a000000;
    /* The native rows reuse their views, so a view that showed an option
       shows the text of the page that reuses it: a row value of the
       settings page, and a value of the Style page. */
    object_memory[ROW_VALUE_VIEW / 4] = view + 0x10;
    menu_settings_value_2(0, TEST_OBJECTS);
    assert(menu_text_override(view + 0x10, SETTINGS_TITLE_TEXT) == ((u32)(block + SETTINGS_VIEWS / 4 + 2 * 3 + 1) | 1U));
    assert(menu_settings_controller(pages[2]) == pages[2]);
    object_memory[(0x100 + ROW_TEXT_VIEW) / 4] = view + 0x10;
    assert(menu_settings_draw(TEST_OBJECTS + 0x100, 0, 2) == 1);
    assert(menu_text_override(view + 0x10, SETTINGS_TITLE_TEXT) ==
           ((u32)(block + SETTINGS_VIEWS / 4 + SUBMENU_ROWS * 3 + 1) | 1U));
    assert(menu_settings_controller(SETTINGS_SUBMENU) == SETTINGS_SUBMENU);
}

static void the_film_look_menu_page_lists_film_stocks(void)
{
    volatile u32 *h = test_header, *block = settings_block(test_header), *menu = services_word(test_header, SERVICES_MENU);
    const u32 film = TEST_PAGE;
    const Setting *page;
    /* The page of Film Look: Enable, Options, and Film Stock, whose
       values are Off and the effects that are On. A committed effect adds
       Parameters, which opens the Fn adjustments and shows no value. */
    *page_at(film + PAGE_FLAGS) |= CHOICE_OPTIONS;
    h[GRKIT_HOST_ACTIVE / 4] = 0;
    menu[MENU_ROWS_LISTED / 4] = 0;
    menu[MENU_ROWS_PAGE / 4] = 0;
    settings_open(h, LISTED_PAGE, 0);
    page = settings_page(h);
    assert(settings_rows(h) == 3 && same(page[2].label(h), "Film Stock"));
    assert(page[1].values(h) == 3 && page[2].values(h) == 3 && same(page[2].value_text(h, 0), "Off"));
    assert(!settings_long(h, page, 2) && settings_options(h, page, 1));
    page[2].set(h, 1);
    assert(tone_value(h) && page[2].get(h) == 1);
    assert(menu_settings_controller(SETTINGS_SUBMENU) == SETTINGS_SUBMENU);
    assert(settings_rows(h) == 4 && same(page[3].label(h), "Parameters") && !page[3].value_text(h, 0)[0]);
    assert(settings_fn_row(h, page, 3) && block[SUBMENU_KIND_CONTROLLERS / 4 + 3] == MENU_FN_TARGET);
    page[2].set(h, 0);
    assert(!h[GRKIT_HOST_ACTIVE / 4]);
    *page_at(film + PAGE_FLAGS) &= ~CHOICE_OPTIONS;
}

static void a_section_row_shows_its_module_and_value(void)
{
    volatile u32 *h = test_header, *menu = services_word(test_header, SERVICES_MENU);
    const u32 view = 0x5a000000;
    /* A MENU row of a module shows its title and the value of its page,
       or Off while Enable is Off. The rows give their texts while a MENU
       tab shows, and not while the borrowed submenu does. */
    menu[MENU_ROWS_COUNT / 4] = 1;
    menu_row(h, 0)[MENU_ROW_KIND / 4] = MENU_ROW_LISTED;
    menu_row(h, 0)[MENU_ROW_MODULE / 4] = 2;
    object_memory[(0x800 + MENU_ROW_LABEL_VIEW) / 4] = view + 0x30;
    object_memory[(0x800 + MENU_ROW_VALUE_VIEW) / 4] = view + 0x31;
    GUIDE_STYLE = 1;
    menu_row_fill(0, TEST_OBJECTS + 0x800);
    assert(menu_text_override(view + 0x30, MENU_ROW_LABEL_TEXT) !=
           ((u32)(menu_row(h, 0) + MENU_ROW_LABEL_TEXT_AT / 4) | 1U));
    menu[MENU_ROWS_CURRENT / 4] = 0x200004a;
    assert(menu_text_override(view + 0x30, MENU_ROW_LABEL_TEXT) ==
           ((u32)(menu_row(h, 0) + MENU_ROW_LABEL_TEXT_AT / 4) | 1U));
    assert(menu_text_override(view + 0x40, MENU_ROW_LABEL_TEXT) ==
           ((u32)services_word(h, SERVICES_MENU + MENU_ROWS_TITLE_AT) | 1U));
    assert(menu_row(h, 0)[MENU_ROW_LABEL_TEXT_AT / 4] == ((6 + 1) | 0x100));
    assert(menu_row(h, 0)[MENU_ROW_VALUE_TEXT_AT / 4] == ((6 + 1) | 0x100));
    settings_memory[2 + 9 * 3 + 1] = 1;
    menu_row_fill(0, TEST_OBJECTS + 0x800);
    assert(menu_row(h, 0)[MENU_ROW_VALUE_TEXT_AT / 4] == ((3 + 1) | 0x100));
    settings_memory[2 + 9 * 3 + 1] = 0;
    GUIDE_STYLE = 0;
    /* A scroll moves a row view to another slot: the view shows the row
       of its new slot. */
    menu[MENU_ROWS_COUNT / 4] = 2;
    menu_row(h, 1)[MENU_ROW_KIND / 4] = MENU_ROW_ADJ;
    menu_row(h, 1)[MENU_ROW_MODULE / 4] = 0;
    menu_row_fill(1, TEST_OBJECTS + 0x800);
    assert(!menu_row(h, 0)[MENU_ROW_LABEL / 4] && !menu_row(h, 0)[MENU_ROW_VALUE / 4]);
    assert(menu_text_override(view + 0x30, MENU_ROW_LABEL_TEXT) ==
           ((u32)(menu_row(h, 1) + MENU_ROW_LABEL_TEXT_AT / 4) | 1U));
    menu[MENU_ROWS_COUNT / 4] = 1;
}

static void about_grkit_shows_the_version_and_address(void)
{
    volatile u32 *h = test_header, *menu = services_word(test_header, SERVICES_MENU);
    const u32 view = 0x5a000000;
    /* About GRKit: the camera information page shows the version and the
       project address, only after the About GRKit row opened it. */
    assert(!menu_text_override(view, INFO_TITLE_TEXT));
    menu[MENU_ROWS_ABOUT / 4] = 1;
    assert(!menu_text_override(view, INFO_TITLE_TEXT));
    assert(menu_settings_controller(INFO_CONTROLLER) == INFO_CONTROLLER);
    menu[MENU_ROWS_ABOUT / 4] = 1;
    assert(menu_text_override(view, INFO_TITLE_TEXT) ==
           ((u32)services_word(h, SERVICES_MENU + MENU_ROWS_ABOUT_TEXTS) | 1U));
    assert(services_word(h, SERVICES_MENU + MENU_ROWS_ABOUT_TEXTS)[0] == ((11 + 1) | 0x100));
    menu_info_fill(TEST_OBJECTS + 0xc00);
    assert(!strcmp((const char *)services_word(h, SERVICES_MENU + MENU_ROWS_ABOUT_LINES) + MENU_ABOUT_LINE,
                   "github.com/ahonn/grkit"));
    menu[MENU_ROWS_ABOUT / 4] = 0;
}

static void each_selection_change_counts_once(void)
{
    reset();
    /* Each change of a saved selection counts in GRKIT_HOST_STATE_CHANGES,
       the one word that the resident loader polls. A store of the value that
       the setting has counts nothing. */
    volatile u32 *h = test_header;
    const u32 grain = h[GRKIT_HOST_GRAIN / 4], style = GUIDE_STYLE;
    u32 n = h[GRKIT_HOST_STATE_CHANGES / 4];
    setting_store(h, GRKIT_CORE_GRAIN, grain == 2 ? 1 : 2);
    assert(h[GRKIT_HOST_STATE_CHANGES / 4] == ++n);
    setting_store(h, GRKIT_CORE_GRAIN, setting_value(h, GRKIT_CORE_GRAIN));
    assert(h[GRKIT_HOST_STATE_CHANGES / 4] == n);
    setting_store(h, GRKIT_CORE_GRAIN, grain);
    assert(h[GRKIT_HOST_STATE_CHANGES / 4] == ++n);
    tone_store(h, 1);
    assert(h[GRKIT_HOST_STATE_CHANGES / 4] == ++n);
    tone_store(h, 0);
    assert(h[GRKIT_HOST_STATE_CHANGES / 4] == ++n);
    /* While the highlight warning is on, a committed effect sets it again
       with its screen and mode, so its threshold follows the effect. */
    level_calls = 0;
    h[GRKIT_HOST_LEVEL_SCREEN / 4] = 0x5c001000; h[GRKIT_HOST_LEVEL_MODE / 4] = 0;
    tone_store(h, 1);
    assert(level_calls == 0);
    h[GRKIT_HOST_LEVEL_MODE / 4] = 2;
    tone_store(h, 0);
    assert(level_calls == 1 && level_screen == 0x5c001000 && level_mode == 2);
    h[GRKIT_HOST_LEVEL_SCREEN / 4] = h[GRKIT_HOST_LEVEL_MODE / 4] = 0;
    n = h[GRKIT_HOST_STATE_CHANGES / 4];
    setting_store(h, 0, style == 1 ? 2 : 1);
    assert(h[GRKIT_HOST_STATE_CHANGES / 4] == ++n);
    setting_store(h, 0, setting_value(h, 0));
    assert(h[GRKIT_HOST_STATE_CHANGES / 4] == n);
    setting_store(h, 0, style);
    assert(h[GRKIT_HOST_STATE_CHANGES / 4] == ++n);
}

/* The mode dial (runtime/mode_dial.h). Listed module 0 is the Recipes
   module, which uses CORE.RECIPE; listed module 1 runs the other
   capabilities; handle 12 is the switch Enable GRKit. Two effect records,
   "a" and "b", are in mapped memory. Recipe 0 (ID 1001) is effect b with Fn
   adjustments and Grain Strong, recipe 1 (ID 1002) holds only Grain Extra
   Strong, and recipe 2 (ID 1003) is an effect that is not installed. */
#define DIAL_P 3U
#define DIAL_U(u) (DIAL_CODE_U1 + (u) - 1U)
#define DIAL_SWITCH_HANDLE 12U
#define DIAL_RECIPES_OFF settings_memory[2 + 3 * 3 + 1]
#define DIAL_SWITCH_OFF settings_memory[2 + DIAL_SWITCH_HANDLE * 3 + 1]
#define DIAL_FN 19U
static u32 dial_record(u32 i) { return TEST_OBJECTS + i * GRKIT_RECORD_SIZE; }
static void dial_reset(void)
{
    volatile u32 *h = test_header;
    const u32 running = GRKIT_CORE_TONE_BIT | GRKIT_CORE_ASPECT_BIT | GRKIT_CORE_GRAIN_BIT | GRKIT_CORE_GRAIN_SIZE_BIT;
    reset();
    install_module_list();
    h[GRKIT_HOST_SERVICES / 4] = TEST_SERVICES;
    settings_memory[0] = DIAL_SWITCH_HANDLE + 1;
    settings_memory[2 + DIAL_SWITCH_HANDLE * 3 + 2] = 2;
    manage_memory[MANAGE_SWITCH / 4] = DIAL_SWITCH_HANDLE + 1;
    manage_memory[MANAGE_ENTRY(0) + MANAGE_USES / 4] = GRKIT_CORE_RECIPE_BIT;
    manage_memory[MANAGE_ENTRY(1) + MANAGE_USES / 4] = running;
    h[GRKIT_HOST_CAPABILITIES / 4] = running | GRKIT_CORE_RECIPE_BIT;
    h[GRKIT_HOST_RECORDS / 4] = dial_record(0);
    h[GRKIT_HOST_COUNT / 4] = 2;
    BYTE(dial_record(0), GRKIT_RECORD_ID) = 'a';
    BYTE(dial_record(1), GRKIT_RECORD_ID) = 'b';
    const u32 b = recipe_effect_code(dial_record(1));
    u32 missing = 1;
    while (missing == b || missing == recipe_effect_code(dial_record(0))) ++missing;
    header_recipe(0, "", RECIPE_USED | RECIPE_LOOK_EFFECT | b << RECIPE_CODE_SHIFT | RECIPE_GRAIN | 3U << RECIPE_GRAIN_LEVEL_SHIFT);
    h[GRKIT_HOST_RECIPES / 4 + RECIPE_FN_WORD] = DIAL_FN;
    header_recipe(1, "", RECIPE_USED | RECIPE_GRAIN | 4U << RECIPE_GRAIN_LEVEL_SHIFT);
    header_recipe(2, "", RECIPE_USED | RECIPE_LOOK_EFFECT | missing << RECIPE_CODE_SHIFT);
    for (u32 i = 0; i < 3; ++i) *recipe_id_at(h, i) = 1001 + i;
    h[GRKIT_HOST_RECIPE_DIAL / 4] = 1001;
    h[GRKIT_HOST_RECIPE_DIAL / 4 + 1] = 1002;
    h[GRKIT_HOST_RECIPE_DIAL / 4 + 2] = GRKIT_RECIPE_DIAL_OFF;
    /* The base state: Film Look Off, Grain Weak, and Guides Center. */
    h[GRKIT_HOST_ACTIVE / 4] = 0;
    h[GRKIT_HOST_GRAIN / 4] = 1;
    GUIDE_STYLE = 2;
    dial_code = DIAL_P;
    dial_loaded = 1;
    dial_timer_ids = dial_timer_ms = dial_timer_cancels = 0;
}
/* The firmware applies position `code`. */
static void dial_to(u32 code)
{
    dial_code = code;
    menu_dial_moved();
}
/* The last timer of the dial ends. */
static void dial_fire(void)
{
    assert(services_word(test_header, SERVICES_DIAL + DIAL_TIMER)[0] == dial_timer_ids);
    ui_timer_fired(0, dial_timer_ids);
}
static int dial_base(void)
{
    volatile u32 *h = test_header;
    return !h[GRKIT_HOST_ACTIVE / 4] && h[GRKIT_HOST_GRAIN / 4] == 1 && GUIDE_STYLE == 2 && !DIAL_SWITCH_OFF &&
           recipe_fn_read(dial_record(1)) == 0 && !h[GRKIT_HOST_STATE_HOLD / 4] &&
           !services_word(h, SERVICES_DIAL + DIAL_ACTIVE)[0];
}

static void the_mode_dial_applies_a_user_mode_and_restores_the_base(void)
{
    volatile u32 *h = test_header;
    u32 changes, sets;
    dial_reset();
    /* A move starts the timer of the dial; nothing applies before it ends. */
    dial_to(DIAL_U(1));
    assert(dial_timer_ids == 1 && dial_timer_ms == DIAL_SETTLE_MS && dial_base());
    /* U1 applies its recipe without an undo step and holds the state file. */
    changes = h[GRKIT_HOST_STATE_CHANGES / 4];
    dial_fire();
    assert(h[GRKIT_HOST_ACTIVE / 4] == dial_record(1) && h[GRKIT_HOST_GRAIN / 4] == 3);
    assert(recipe_fn_read(dial_record(1)) == DIAL_FN && !recipe_undo_count(h));
    assert(h[GRKIT_HOST_STATE_HOLD / 4] == 1 && h[GRKIT_HOST_STATE_CHANGES / 4] > changes);
    assert(!services_word(h, SERVICES_DIAL + DIAL_TIMER)[0]);
    /* Changes in the user mode: Grain Medium, Guides Thirds, and Fn. */
    setting_store(h, GRKIT_CORE_GRAIN, 2);
    setting_store(h, 0, 1);
    BYTE(dial_record(0), GRKIT_RECORD_PARAMETERS) = 3;
    /* The firmware applies U1 again: the core acted on it, so nothing
       changes. */
    dial_to(DIAL_U(1));
    dial_fire();
    assert(h[GRKIT_HOST_GRAIN / 4] == 2 && GUIDE_STYLE == 1);
    /* U1 to U2 applies U2 at once, on the settings of U1: the recipe holds
       only Grain. The copy of the base state stays. */
    dial_to(DIAL_U(2));
    dial_fire();
    assert(h[GRKIT_HOST_GRAIN / 4] == 4 && h[GRKIT_HOST_ACTIVE / 4] == dial_record(1) && GUIDE_STYLE == 1);
    assert(h[GRKIT_HOST_STATE_HOLD / 4] == 1 && services_word(h, SERVICES_DIAL + DIAL_ACTIVE)[0] == 2);
    /* P restores each setting of the base state, also the Fn adjustments of
       each effect, and releases the state file. A setting that did not
       change writes nothing: Framing keeps the native aspect. */
    sets = native_sets;
    changes = h[GRKIT_HOST_STATE_CHANGES / 4];
    dial_to(DIAL_P);
    dial_fire();
    assert(dial_base() && BYTE(dial_record(0), GRKIT_RECORD_PARAMETERS) == 0);
    assert(native_sets == sets && !framing_requests);
    /* The restore counts its changes, so the loader saves the base state. */
    assert(h[GRKIT_HOST_STATE_CHANGES / 4] > changes);
    assert(services_word(h, SERVICES_DIAL + DIAL_AT)[0] == 0);
    /* A position that is no user mode leaves the base state as it is. */
    dial_to(DIAL_P + 2);
    dial_fire();
    assert(dial_base());
}

static void the_mode_dial_acts_once_where_the_dial_stays(void)
{
    volatile u32 *h = test_header;
    u32 first;
    dial_reset();
    /* A fast pass P, U1, U2, U3: each move starts the timer again, and only
       the last one acts. U3 turns GRKit off. */
    dial_to(DIAL_U(1));
    first = dial_timer_ids;
    dial_to(DIAL_U(2));
    dial_to(DIAL_U(3));
    assert(dial_timer_cancels == 2 && dial_base());
    /* A timer that a move replaced does nothing. */
    ui_timer_fired(0, first);
    assert(dial_base());
    dial_fire();
    assert(DIAL_SWITCH_OFF == 1 && manage_all_off(h) && h[GRKIT_HOST_STATE_HOLD / 4] == 1);
    assert(h[GRKIT_HOST_GRAIN / 4] == 1 && !h[GRKIT_HOST_ACTIVE / 4]);
    /* P turns GRKit on again. */
    dial_to(DIAL_P);
    dial_fire();
    assert(dial_base() && !manage_all_off(h));
    /* While the firmware loads the settings of the user mode, the timer
       tries again, a bounded number of times, and then gives up. */
    dial_loaded = 0;
    dial_to(DIAL_U(1));
    dial_fire();
    assert(dial_timer_ms == DIAL_RETRY_MS && dial_base());
    for (u32 i = 2; i < DIAL_TRIES; ++i) dial_fire();
    assert(services_word(h, SERVICES_DIAL + DIAL_TIMER)[0] == dial_timer_ids);
    dial_fire();
    assert(!services_word(h, SERVICES_DIAL + DIAL_TIMER)[0] && dial_base());
    /* The next move tries again from the start. */
    dial_to(DIAL_U(1));
    dial_fire();
    dial_loaded = 1;
    dial_fire();
    assert(h[GRKIT_HOST_GRAIN / 4] == 3 && h[GRKIT_HOST_STATE_HOLD / 4] == 1);
}

static void the_mode_dial_follows_without_a_recipe_that_applies(void)
{
    volatile u32 *h = test_header;
    dial_reset();
    /* A user mode that follows changes nothing and holds nothing. */
    h[GRKIT_HOST_RECIPE_DIAL / 4] = GRKIT_RECIPE_DIAL_FOLLOW;
    dial_to(DIAL_U(1));
    dial_fire();
    assert(dial_base());
    /* So does an ID that no recipe has, as of a deleted recipe, and that of
       a recipe that is off. */
    h[GRKIT_HOST_RECIPE_DIAL / 4] = 4242;
    dial_to(DIAL_U(1));
    dial_to(DIAL_P);
    dial_fire();
    dial_to(DIAL_U(1));
    dial_fire();
    assert(dial_base());
    h[GRKIT_HOST_RECIPE_DIAL / 4] = 1002;
    recipe_at(h, 1)[RECIPE_MODULE_WORD] |= RECIPE_OFF;
    recipe_at(h, 2)[RECIPE_MODULE_WORD] |= RECIPE_OFF;
    dial_to(DIAL_P);
    dial_fire();
    dial_to(DIAL_U(1));
    dial_fire();
    assert(dial_base());
    recipe_at(h, 1)[RECIPE_MODULE_WORD] &= ~RECIPE_OFF;
    recipe_at(h, 2)[RECIPE_MODULE_WORD] &= ~RECIPE_OFF;
    /* A recipe whose effect is not installed follows too, also with the
       switch Off, which the core turned On to check the recipe. */
    DIAL_SWITCH_OFF = 1;
    manage_off_changed(h);
    h[GRKIT_HOST_RECIPE_DIAL / 4] = 1003;
    dial_to(DIAL_P);
    dial_fire();
    dial_to(DIAL_U(1));
    dial_fire();
    assert(DIAL_SWITCH_OFF == 1 && manage_all_off(h) && !h[GRKIT_HOST_STATE_HOLD / 4]);
    /* From a bound user mode, it restores the base state. */
    DIAL_SWITCH_OFF = 0;
    manage_off_changed(h);
    dial_to(DIAL_U(2));
    dial_fire();
    assert(h[GRKIT_HOST_GRAIN / 4] == 4 && h[GRKIT_HOST_STATE_HOLD / 4] == 1);
    dial_to(DIAL_U(1));
    dial_fire();
    assert(dial_base());
}

static void the_mode_dial_belongs_to_the_recipes_module(void)
{
    volatile u32 *h = test_header;
    dial_reset();
    /* With the Recipes module Off, the dial does nothing, also at a start. */
    DIAL_RECIPES_OFF = 1;
    manage_off_changed(h);
    dial_to(DIAL_U(1));
    dial_fire();
    assert(dial_base() && !manage_stopped(h, 1));
    dial_code = DIAL_P;
    services_word(h, SERVICES_DIAL + DIAL_AT)[0] = 0;
    dial_timer_ids = 0;
    dial_code = DIAL_U(1);
    menu_ready();
    assert(!dial_timer_ids);
    /* The switch Off does not stop it: U1 turns GRKit on and applies its
       recipe, and P turns GRKit off again. */
    DIAL_RECIPES_OFF = 0;
    DIAL_SWITCH_OFF = 1;
    manage_off_changed(h);
    assert(manage_stopped(h, 0) && manage_stopped(h, 1) && !h[GRKIT_HOST_CAPABILITIES / 4]);
    dial_to(DIAL_P);
    dial_fire();
    dial_to(DIAL_U(1));
    dial_fire();
    assert(!DIAL_SWITCH_OFF && !manage_stopped(h, 1));
    assert(h[GRKIT_HOST_ACTIVE / 4] == dial_record(1) && h[GRKIT_HOST_GRAIN / 4] == 3);
    dial_to(DIAL_P);
    dial_fire();
    assert(DIAL_SWITCH_OFF == 1 && manage_stopped(h, 1) && !h[GRKIT_HOST_ACTIVE / 4]);
    assert(!h[GRKIT_HOST_STATE_HOLD / 4]);
}

static void the_mode_dial_applies_a_user_mode_at_a_start(void)
{
    volatile u32 *h = test_header;
    dial_reset();
    /* At a start in U1 the firmware sends no dial event: the first
       shooting screen starts the timer, and its end applies U1. */
    dial_code = DIAL_U(1);
    menu_ready();
    assert(dial_timer_ids == 1 && dial_timer_ms == DIAL_RETRY_MS);
    dial_fire();
    assert(h[GRKIT_HOST_GRAIN / 4] == 3 && h[GRKIT_HOST_STATE_HOLD / 4] == 1);
    /* A start in P changes nothing. */
    dial_reset();
    menu_ready();
    dial_fire();
    assert(dial_base());
}

static void record_index_finds_each_record_start(void)
{
    reset();
    /* record_index finds a record by its address as the scan over the
       records did: each record start, and no other address. */
    volatile u32 *h = test_header;
    const u32 count = h[GRKIT_HOST_COUNT / 4], records = h[GRKIT_HOST_RECORDS / 4];
    h[GRKIT_HOST_COUNT / 4] = GRKIT_TEXT_RECORDS;
    h[GRKIT_HOST_RECORDS / 4] = 0x59005000;
    for (u32 at = 0x59005000 - 0x2000; at < 0x59005000 + (GRKIT_TEXT_RECORDS + 1) * GRKIT_RECORD_SIZE; at += 4) {
        u32 expected = GRKIT_TEXT_RECORDS;
        for (u32 i = 0; i < GRKIT_TEXT_RECORDS; ++i)
            if (at == 0x59005000 + i * GRKIT_RECORD_SIZE) expected = i;
        assert(record_index(h, at) == expected);
    }
    assert(record_index(h, 0) == GRKIT_TEXT_RECORDS);
    h[GRKIT_HOST_COUNT / 4] = 3;
    assert(record_index(h, 0x59005000 + 2 * GRKIT_RECORD_SIZE) == 2);
    assert(record_index(h, 0x59005000 + 3 * GRKIT_RECORD_SIZE) == GRKIT_TEXT_RECORDS);
    h[GRKIT_HOST_COUNT / 4] = count;
    h[GRKIT_HOST_RECORDS / 4] = records;
}

#ifdef FW_MENU_TABS
/* The Customize tab of the firmware at TEST_TAB: `rows` rows in `sections`
   sections, a pattern in each other byte, and the TopMenu at TEST_TOP in the
   controller factory. The module list has three modules, and the section is
   not in place. */
#define TEST_TAB (TEST_OBJECTS + 0x1000U)
#define TEST_TOP (TEST_OBJECTS + 0x1800U)
static void install_stock_tab(u32 rows, u32 sections)
{
    reset();
    test_header[GRKIT_HOST_SERVICES / 4] = TEST_SERVICES;
    test_header[GRKIT_HOST_MANAGE / 4] = TEST_MANAGE;
    manage_memory[0] = 3;
    /* Module 0 has the page of Guides, so ADJ can show it. */
    for (u32 i = 0; i < 3; ++i) {
        manage_memory[MANAGE_ENTRY(i)] = i ? 52 : MANAGE_LOADED;
        manage_memory[MANAGE_ENTRY(i) + 1] = i ? 0 : TEST_PAGE + 2 * TEST_PAGE_SIZE + PAGE_TITLES;
    }
    for (u32 i = 0; i < MENU_TAB_SIZE; i += 4) WORD(TEST_TAB, i) = 0xa5000000U | i;
    BYTE(TEST_TAB, MENU_TAB_ROWS) = (u8)rows;
    BYTE(TEST_TAB, MENU_TAB_SECTIONS) = (u8)sections;
    menu_tabs[MENU_TAB_GRKIT] = TEST_TAB;
    factory_top = TEST_TOP;
    WORD(TEST_TOP, MENU_TOP_SLOT) = TEST_OBJECTS + 0x1c00;
}
/* The copy of the Customize tab in the services block. */
static u32 tab_copy(void) { return (u32)services_word(test_header, SERVICES_MENU + MENU_ROWS_TAB); }
/* The first controller change after INSTALL copies the Customize tab and adds
   the GRKit section after its rows: a row for each listed module in default
   order, an ADJ Order row for the group, and About GRKit. The tab table
   points at the copy, and the factory gives up its TopMenu. */
static void the_section_rows_follow_the_listed_modules(void)
{
    static const u32 kinds[] = {MENU_ROW_LISTED, MENU_ROW_LISTED, MENU_ROW_LISTED, MENU_ROW_ADJ, MENU_ROW_GRKIT},
                     modules[] = {0, 1, 2, 0, 0};
    volatile u32 *h = test_header, *menu, title[2];
    u32 copy;
    install_stock_tab(10, 3);
    menu = services_word(h, SERVICES_MENU);
    copy = tab_copy();
    menu_rows_install(h);
    assert(menu[MENU_ROWS_DONE / 4] && menu[MENU_ROWS_COUNT / 4] == 5);
    assert(menu_tabs[MENU_TAB_GRKIT] == copy && BYTE(copy, MENU_TAB_ROWS) == 15);
    assert(factory_calls == 1 && !WORD(TEST_TOP, MENU_TOP_SLOT));
    /* The section: the title GRKit, from the first new row. */
    assert(BYTE(copy, MENU_TAB_SECTIONS) == 4);
    assert(HALF(copy, MENU_TAB_TITLES + 2 * 3) == MENU_ROW_LABEL_TEXT && BYTE(copy, MENU_TAB_STARTS + 3) == 10);
    descriptor(title, grkit_title);
    assert(menu[MENU_ROWS_TITLE_AT / 4] == title[0] && menu[MENU_ROWS_TITLE_AT / 4 + 1] == title[1]);
    /* Each new row: the filler and the target of its slot, no predicate, and
       its kind and module. */
    for (u32 slot = 0; slot < 5; ++slot) {
        const u32 row = 10 + slot;
        assert(WORD(copy, MENU_TAB_FILLERS + 8 * row) == (u32)(unsigned long)fillers[slot]);
        assert(!WORD(copy, MENU_TAB_FILLERS + 8 * row + 4));
        assert(WORD(copy, MENU_TAB_TARGETS + 4 * row) == MENU_ROW_TARGET + slot);
        assert(!WORD(copy, MENU_TAB_PREDICATES + 8 * row) && !WORD(copy, MENU_TAB_PREDICATES + 8 * row + 4));
        assert(menu_row(h, slot)[MENU_ROW_KIND / 4] == kinds[slot] && menu_row(h, slot)[MENU_ROW_MODULE / 4] == modules[slot]);
    }
    /* The stock sections and rows stay, the copy has no other new row, and
       the stock tab stays as it is. */
    for (u32 s = 0; s < 3; ++s)
        assert(HALF(copy, MENU_TAB_TITLES + 2 * s) == HALF(TEST_TAB, MENU_TAB_TITLES + 2 * s) &&
               BYTE(copy, MENU_TAB_STARTS + s) == BYTE(TEST_TAB, MENU_TAB_STARTS + s));
    for (u32 row = 0; row <= 15; row += row == 9 ? 6 : 1)
        assert(WORD(copy, MENU_TAB_FILLERS + 8 * row) == WORD(TEST_TAB, MENU_TAB_FILLERS + 8 * row) &&
               WORD(copy, MENU_TAB_TARGETS + 4 * row) == WORD(TEST_TAB, MENU_TAB_TARGETS + 4 * row) &&
               WORD(copy, MENU_TAB_PREDICATES + 8 * row) == WORD(TEST_TAB, MENU_TAB_PREDICATES + 8 * row));
    assert(BYTE(TEST_TAB, MENU_TAB_ROWS) == 10 && BYTE(TEST_TAB, MENU_TAB_SECTIONS) == 3);
    /* The section is added once. */
    menu_tabs[MENU_TAB_GRKIT] = TEST_TAB;
    menu_rows_install(h);
    assert(menu_tabs[MENU_TAB_GRKIT] == TEST_TAB && factory_calls == 1);
    /* Without a module that ADJ can show, the section has no ADJ Order row. */
    install_stock_tab(10, 3);
    manage_memory[MANAGE_ENTRY(0) + 1] = 0;
    menu_rows_install(h);
    assert(menu[MENU_ROWS_COUNT / 4] == 4 && menu_row(h, 3)[MENU_ROW_KIND / 4] == MENU_ROW_GRKIT);
}
/* A color-only module that a monochrome camera did not load gets no row; the
   rows of the other modules keep their module. */
static void a_color_only_module_gets_no_row(void)
{
    volatile u32 *h = test_header, *menu;
    install_stock_tab(10, 3);
    manage_memory[MANAGE_ENTRY(1)] = MANAGE_COLOR_ONLY;
    menu = services_word(h, SERVICES_MENU);
    menu_rows_install(h);
    assert(menu[MENU_ROWS_COUNT / 4] == 4);
    assert(menu_row(h, 0)[MENU_ROW_KIND / 4] == MENU_ROW_LISTED && menu_row(h, 0)[MENU_ROW_MODULE / 4] == 0);
    assert(menu_row(h, 1)[MENU_ROW_KIND / 4] == MENU_ROW_LISTED && menu_row(h, 1)[MENU_ROW_MODULE / 4] == 2);
    assert(menu_row(h, 2)[MENU_ROW_KIND / 4] == MENU_ROW_ADJ && menu_row(h, 3)[MENU_ROW_KIND / 4] == MENU_ROW_GRKIT);
    assert(BYTE(tab_copy(), MENU_TAB_ROWS) == 14);
}
/* While a module loaded, the section starts with the GRKit switch, then
   Enable at Start. OK on either opens its value page. A tab without room for
   each row keeps the switch, Enable at Start, ADJ Order, and About GRKit. */
static void the_section_starts_with_the_switch(void)
{
    static const u32 kinds[] = {MENU_ROW_SWITCH, MENU_ROW_START, MENU_ROW_LISTED, MENU_ROW_LISTED, MENU_ROW_LISTED,
                                MENU_ROW_ADJ, MENU_ROW_GRKIT},
                     short_kinds[] = {MENU_ROW_SWITCH, MENU_ROW_START, MENU_ROW_LISTED, MENU_ROW_ADJ, MENU_ROW_GRKIT};
    volatile u32 *h = test_header, *menu;
    u32 n;
    /* The switch is setting 12 and Enable at Start setting 13
       (runtime/install_modules.h). */
    install_stock_tab(10, 3);
    settings_memory[0] = 14;
    settings_memory[2 + 12 * 3 + 2] = 2;
    settings_memory[2 + 13 * 3 + 2] = MANAGE_START_VALUES;
    manage_memory[MANAGE_SWITCH / 4] = 12 + 1;
    manage_memory[MANAGE_START / 4] = 13 + 1;
    menu = services_word(h, SERVICES_MENU);
    menu_rows_install(h);
    assert(menu[MENU_ROWS_COUNT / 4] == 7 && BYTE(tab_copy(), MENU_TAB_ROWS) == 17);
    for (u32 slot = 0; slot < 7; ++slot) assert(menu_row(h, slot)[MENU_ROW_KIND / 4] == kinds[slot]);
    /* The row, Enable GRKit, opens its value page at once: On, then Off,
       as the Enable of a module. */
    assert(menu_rows_controller(h, MENU_ROW_TARGET) == settings_pages[0]);
    assert(h[GRKIT_HOST_SETTINGS / 4] == SWITCH_MODULE + 1 && settings_page(h) == switch_settings);
    assert(same(switch_menu_label(h), "Enable GRKit"));
    assert(settings_rows(h) == 1 && same(switch_settings[0].label(h), "Enable GRKit"));
    assert(switch_settings[0].values(h) == 2 && same(switch_settings[0].value_text(h, 0), "On") &&
           same(switch_settings[0].value_text(h, 1), "Off"));
    switch_settings[0].set(h, 1);
    assert(switch_settings[0].get(h) == 1 && settings_memory[2 + 12 * 3 + 1] == 1);
    switch_settings[0].set(h, 0);
    /* Enable at Start opens its value page at once too: On, Off, then Keep
       Last. A choice is a saved change for the next start, and the switch
       keeps its value in this session. */
    assert(menu_rows_controller(h, MENU_ROW_TARGET + 1) == settings_pages[0]);
    assert(h[GRKIT_HOST_SETTINGS / 4] == START_PAGE + 1 && settings_page(h) == start_settings);
    assert(settings_rows(h) == 1 && same(start_settings[0].label(h), "Enable at Start"));
    assert(start_settings[0].values(h) == 3 && same(start_settings[0].value_text(h, MANAGE_START_ON), "On") &&
           same(start_settings[0].value_text(h, MANAGE_START_OFF), "Off") &&
           same(start_settings[0].value_text(h, MANAGE_START_KEEP), "Keep Last"));
    assert(start_settings[0].get(h) == MANAGE_START_ON);
    n = h[GRKIT_HOST_STATE_CHANGES / 4];
    start_settings[0].set(h, MANAGE_START_OFF);
    assert(start_settings[0].get(h) == MANAGE_START_OFF && settings_memory[2 + 13 * 3 + 1] == MANAGE_START_OFF);
    assert(h[GRKIT_HOST_STATE_CHANGES / 4] == n + 1 && !manage_all_off(h));
    /* The row shows the value. */
    assert(same(start_value_text(h, start_value(h)), "Off"));
    start_settings[0].set(h, MANAGE_START_KEEP);
    assert(same(start_value_text(h, start_value(h)), "Keep Last"));
    start_settings[0].set(h, MANAGE_START_ON);
    h[GRKIT_HOST_SETTINGS / 4] = 0;
    install_stock_tab(MENU_TAB_MAX_ROWS - 5, 3);
    manage_memory[MANAGE_SWITCH / 4] = 12 + 1;
    manage_memory[MANAGE_START / 4] = 13 + 1;
    menu_rows_install(h);
    assert(menu[MENU_ROWS_COUNT / 4] == 5 && BYTE(tab_copy(), MENU_TAB_ROWS) == MENU_TAB_MAX_ROWS);
    for (u32 slot = 0; slot < 5; ++slot) assert(menu_row(h, slot)[MENU_ROW_KIND / 4] == short_kinds[slot]);
    /* Without Enable at Start in the module list, the switch shows alone. */
    install_stock_tab(10, 3);
    manage_memory[MANAGE_SWITCH / 4] = 12 + 1;
    menu_rows_install(h);
    assert(menu[MENU_ROWS_COUNT / 4] == 6 && menu_row(h, 1)[MENU_ROW_KIND / 4] == MENU_ROW_LISTED);
}
/* A tab with all its sections gets the rows in its last section. A tab
   without room for each row keeps ADJ Order and About GRKit, and gives rows
   to the first modules only. */
static void a_full_tab_gets_no_new_section(void)
{
    volatile u32 *h = test_header, *menu;
    u32 copy;
    install_stock_tab(10, MENU_TAB_MAX_SECTIONS);
    menu = services_word(h, SERVICES_MENU);
    copy = tab_copy();
    menu_rows_install(h);
    assert(menu[MENU_ROWS_COUNT / 4] == 5 && menu_tabs[MENU_TAB_GRKIT] == copy);
    assert(BYTE(copy, MENU_TAB_SECTIONS) == MENU_TAB_MAX_SECTIONS && BYTE(copy, MENU_TAB_ROWS) == 15);
    for (u32 s = 0; s < MENU_TAB_MAX_SECTIONS; ++s)
        assert(HALF(copy, MENU_TAB_TITLES + 2 * s) == HALF(TEST_TAB, MENU_TAB_TITLES + 2 * s) &&
               BYTE(copy, MENU_TAB_STARTS + s) == BYTE(TEST_TAB, MENU_TAB_STARTS + s));
    /* Room for four rows: the first two modules, ADJ Order, and About. */
    static const u32 kinds[] = {MENU_ROW_LISTED, MENU_ROW_LISTED, MENU_ROW_ADJ, MENU_ROW_GRKIT},
                     modules[] = {0, 1, 0, 0};
    install_stock_tab(MENU_TAB_MAX_ROWS - 4, 3);
    menu_rows_install(h);
    assert(menu[MENU_ROWS_COUNT / 4] == 4 && BYTE(copy, MENU_TAB_ROWS) == MENU_TAB_MAX_ROWS);
    assert(BYTE(copy, MENU_TAB_SECTIONS) == 4 && BYTE(copy, MENU_TAB_STARTS + 3) == MENU_TAB_MAX_ROWS - 4);
    for (u32 slot = 0; slot < 4; ++slot)
        assert(menu_row(h, slot)[MENU_ROW_KIND / 4] == kinds[slot] && menu_row(h, slot)[MENU_ROW_MODULE / 4] == modules[slot]);
    /* Room for two rows: ADJ Order and About, no module. */
    install_stock_tab(MENU_TAB_MAX_ROWS - 2, 3);
    menu_rows_install(h);
    assert(menu[MENU_ROWS_COUNT / 4] == 2 && BYTE(copy, MENU_TAB_ROWS) == MENU_TAB_MAX_ROWS);
    assert(menu_row(h, 0)[MENU_ROW_KIND / 4] == MENU_ROW_ADJ && menu_row(h, 1)[MENU_ROW_KIND / 4] == MENU_ROW_GRKIT);
    /* A tab without room keeps the stock tab and the TopMenu. */
    install_stock_tab(MENU_TAB_MAX_ROWS, 3);
    menu_rows_install(h);
    assert(menu[MENU_ROWS_DONE / 4] && !menu[MENU_ROWS_COUNT / 4]);
    assert(menu_tabs[MENU_TAB_GRKIT] == TEST_TAB && !factory_calls);
}
#endif

static void registered_icons_use_committed_values_and_restore_the_native_slot(void)
{
    reset();
    volatile u32 *h=test_header;
    u32 descriptor=TEST_PAGE+9*TEST_PAGE_SIZE, condition=descriptor+32, icon=descriptor+64;
    h[GRKIT_HOST_LEGACY_TONE_OWNER/4]=0;
    h[GRKIT_HOST_ACTIVE/4]=h[GRKIT_HOST_RECORDS/4]+GRKIT_RECORD_SIZE;
    assert(menu_viewfinder_icon_select(0x166,0)==0x166);
    PAGE_WORD(descriptor,0)=24; PAGE_WORD(descriptor,4)=1; PAGE_WORD(descriptor,8)=1;
    PAGE_WORD(descriptor,12)=1; PAGE_WORD(descriptor,16)=condition; PAGE_WORD(descriptor,20)=icon;
    PAGE_WORD(condition,0)=GRKIT_CORE_TONE; PAGE_WORD(condition,4)=2; PAGE_WORD(condition,8)=0;
    PAGE_WORD(icon,0)=16; PAGE_WORD(icon,4)=2; PAGE_WORD(icon,8)=GRKIT_CORE_TONE;
    h[GRKIT_HOST_INDICATOR_COUNT/4]=1;
    h[GRKIT_HOST_INDICATORS/4]=descriptor; h[GRKIT_HOST_INDICATORS/4+1]=0;
    assert(menu_viewfinder_icon_select(0x166,0)==0x8001);
    h[GRKIT_HOST_PREVIEW/4]=h[GRKIT_HOST_RECORDS/4]+3*GRKIT_RECORD_SIZE;
    assert(menu_viewfinder_icon_select(0x166,0)==0x8001);
    assert(menu_viewfinder_icon_select(0x166,1)==0x166);
    h[GRKIT_HOST_ACTIVE/4]=0;
    assert(menu_viewfinder_icon_select(0x167,0)==0x167);
    h[GRKIT_HOST_ACTIVE/4]=h[GRKIT_HOST_RECORDS/4];
    h[GRKIT_HOST_SERVICES/4]=TEST_SERVICES;
    services_memory[SERVICES_SUSPENDED/4]=1;
    services_memory[(SERVICES_SUSPENDED+4)/4]=descriptor;
    services_memory[(SERVICES_SUSPENDED+8)/4]=descriptor+128;
    assert(menu_viewfinder_icon_select(0x168,0)==0x168);
    services_memory[SERVICES_SUSPENDED/4]=0;
    /* A zero-valued strength still selects its weak tile, independent of
       the separate boolean setting used by its indicator. */
    PAGE_WORD(icon,4)=1; PAGE_WORD(icon,8)=0; PAGE_WORD(icon,12)=4;
    settings_memory[SETTINGS_ENTRY(0)+1]=0;
    assert(module_icon_value(h,icon,10)==GRKIT_ICON_MODULE_TILE(14));
    settings_memory[SETTINGS_ENTRY(0)+1]=2;
    assert(module_icon_value(h,icon,10)==GRKIT_ICON_MODULE_TILE(16));
}

/* Fn on the row of the value that names an Fn setting (ABI 2.5) steps
   that setting in place, wraps after its last value, and redraws the ADJ
   canvas. The page value stays uncommitted. Fn on another row, on the root,
   and on a page without the field opens the settings as before. */
static void fn_steps_the_setting_of_its_row(void)
{
    volatile u32 *h = test_header;
    const u32 page = TEST_PAGE + GUIDES_MODULE * TEST_PAGE_SIZE;
    reset();
    adj_controllers();
    GUIDE_STYLE = 1;
    menu_open(h, GUIDES_MODULE);
    h[GRKIT_HOST_MENU_CURSOR / 4] = 3;
    assert(!fn_step(h, ADJ_A));
    /* Golden Ratio (value 3) steps Date LOOK: handle 2, three values. */
    *page_at(page) = PAGE_FN; *page_at(page + PAGE_FN_VALUE) = 3; *page_at(page + PAGE_FN_SETTING) = 2;
    DATE_LOOK = 1;
    /* The Fn hint shows the rotate icon on the same rows. */
    assert(fn_setting(h, ADJ_A) == 2);
    test_movie = 1;
    assert(fn_setting(h, ADJ_A) == PAGE_NO_FN && !fn_step(h, ADJ_A) && DATE_LOOK == 1);
    test_movie = 0;
    preview_invalidations = 0;
    assert(fn_step(h, ADJ_A) && DATE_LOOK == 2);
    assert(preview_invalidations == 1 && preview_invalidated == 0x5a000000);
    assert(fn_step(h, ADJ_A) && DATE_LOOK == 0 && preview_invalidations == 2);
    assert(h[GRKIT_HOST_MENU_LEVEL / 4] == MENU_MODULE && h[GRKIT_HOST_MODULE / 4] == GUIDES_MODULE);
    assert(h[GRKIT_HOST_MENU_CURSOR / 4] == 3 && GUIDE_STYLE == 1);
    h[GRKIT_HOST_MENU_CURSOR / 4] = 2;
    assert(fn_setting(h, ADJ_A) == PAGE_NO_FN && !fn_step(h, ADJ_A) && DATE_LOOK == 0);
    assert(menu_back(h) && h[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT);
    assert(!fn_step(h, ADJ_A));
    /* INSTALL clears the value for a setting that the camera does not have. */
    menu_open(h, GUIDES_MODULE);
    h[GRKIT_HOST_MENU_CURSOR / 4] = 3;
    *page_at(page + PAGE_FN_VALUE) = 0;
    assert(!fn_step(h, ADJ_A) && DATE_LOOK == 0);
    *page_at(page) = 40;
}

/* GR IIIx names the crops by the focal lengths of its 40 mm lens. The
   other cameras keep the module texts, and so do other settings and Off. */
static void crops_name_the_focal_lengths_of_the_lens(void)
{
    static const unsigned short fifty[]={'5','0','m','m',0}, longest[]={'1','2','8','m','m',0};
    test_lens_40mm=0;
    assert(crop_text(GRKIT_CORE_CROP,1)==0);
    test_lens_40mm=1;
    assert(memcmp(crop_text(GRKIT_CORE_CROP,1),fifty,sizeof(fifty))==0);
    assert(memcmp(crop_text(GRKIT_CORE_CROP,5),longest,sizeof(longest))==0);
    assert(crop_text(GRKIT_CORE_CROP,0)==0 && crop_text(GRKIT_CORE_CROP,6)==0);
    assert(crop_text(GRKIT_CORE_ASPECT,1)==0 && crop_text(0,1)==0);
    test_lens_40mm=0;
}
int main(void)
{
    crops_name_the_focal_lengths_of_the_lens();
    fn_steps_the_setting_of_its_row();
    registered_icons_use_committed_values_and_restore_the_native_slot();
    the_root_reads_off_without_a_committed_effect();
    each_language_reads_its_own_texts_and_font();
    the_viewfinder_shows_the_committed_record();
    fn_applies_to_effect_rows_of_film_look();
    switch_presses_and_back_events_are_filtered();
    list_restoration_keeps_the_committed_effect();
    a_touch_on_a_grkit_row_keeps_the_native_slot_value();
    the_adj_update_shows_no_slot_type_on_a_grkit_page();
    the_event_ring_tells_menu_from_ok_and_adj();
    playback_matches_private_tags_only();
    icons_toolbar_and_edges_map_their_ids();
    adj_exits_return_once_for_the_parameter_page();
    a_half_press_on_the_native_slot_selects_its_type();
    a_half_press_keeps_the_parameter_session();
    fn_values_add_the_defaults_of_a_native_look();
    toning_moves_red_and_blue_from_green();
    the_root_lists_the_official_modules();
    module_tiles_follow_the_effect_icons();
    the_framing_page_sets_the_ratios_of_the_target();
    framing_modes_publish_capture_geometry_before_the_next_frame();
    selected_framing_ratios_start_from_native_three_two();
    stopping_the_mode_provider_restores_legacy_capture_geometry();
    the_guides_page_commits_only_its_style();
    an_exit_from_a_page_commits_the_highlighted_row();
    the_date_imprint_page_lists_its_styles();
    the_grain_page_changes_only_the_grain_word();
    the_recipe_page_saves_applies_and_deletes();
    the_keyboard_serves_the_grkit_text_only_while_borrowed();
    the_keyboard_keeps_an_unchanged_automatic_name();
    the_recipe_page_lists_the_shown_recipes();
    the_recipe_page_marks_a_missing_effect();
    the_recipe_page_shows_native_type_icons();
    invalid_cursors_form_no_effect_pointer();
    film_look_previews_and_commits_the_highlighted_effect();
    the_root_has_no_about_row();
    a_page_past_the_page_count_has_no_root_row();
    the_root_shows_only_visible_module_pages();
    adj_reopens_on_the_row_of_the_last_module();
    the_date_imprint_settings_page_reads_and_stores_values();
    direct_settings_have_independent_rows();
    the_adj_order_page_moves_modules();
    the_adj_order_page_has_a_row_for_each_module();
    options_hide_values_that_are_off_in_adj();
    enable_applies_at_once();
    the_switch_stops_each_module_at_once();
    film_look_lists_the_installed_effects_that_are_on();
    /* The MENU page blocks after the first continue the screen that the block
       before them leaves. */
    settings_entry_enable_preserves_each_value();
    a_settings_section_row_has_no_choice_value();
    group_icons_follow_child_values();
    choice_off_row_shows_the_page_off_icon();
    group_ok_opens_each_child_and_returns_to_its_row();
    group_pages_open_children_and_return_without_committing();
    group_back_keeps_the_shared_native_parent_return_target();
    group_back_returns_to_the_menu_of_the_camera();
    the_guides_menu_page_lists_enable_options_and_settings();
    the_options_page_toggles_its_values();
#if defined(FW_CHECK_ITEM) && defined(FW_MENU_TABS)
    the_options_page_shows_checkboxes();
#endif
    reused_row_views_show_the_page_that_reuses_them();
    the_film_look_menu_page_lists_film_stocks();
    the_recipes_menu_page_chooses_a_recipe_then_an_action();
    a_section_row_shows_its_module_and_value();
    about_grkit_shows_the_version_and_address();
#ifdef FW_MENU_TABS
    the_section_rows_follow_the_listed_modules();
    a_color_only_module_gets_no_row();
    a_full_tab_gets_no_new_section();
    the_section_starts_with_the_switch();
#endif
    each_selection_change_counts_once();
    record_index_finds_each_record_start();
    the_mode_dial_applies_a_user_mode_and_restores_the_base();
    the_mode_dial_acts_once_where_the_dial_stays();
    the_mode_dial_follows_without_a_recipe_that_applies();
    the_mode_dial_belongs_to_the_recipes_module();
    the_mode_dial_applies_a_user_mode_at_a_start();
    return 0;
}
