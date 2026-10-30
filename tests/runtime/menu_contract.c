#include <assert.h>
#include <string.h>
/* The build's label table, which the Rust test writes for this harness. */
#include "menu_text.h"
/* The family header sends the aspect request; the harness counts requests. */
#include "firmware.h"
#undef MENU_FRAMING_REQUEST
static unsigned framing_requests;
#define MENU_FRAMING_REQUEST() (++framing_requests)
/* The native aspect that the camera applies, and the native setter calls. */
#undef MENU_NATIVE_ASPECT
#undef MENU_SET_NATIVE_ASPECT
#undef MENU_STOP_LIVE_VIEW
static unsigned native_aspect_value, native_sets, live_view_stops;
#define MENU_NATIVE_ASPECT() (native_aspect_value)
#define MENU_SET_NATIVE_ASPECT(id) (++native_sets, native_aspect_value = (id))
#define MENU_STOP_LIVE_VIEW() (++live_view_stops)
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
#define TEST_PAGE_SLOTS 5U
#define TEST_PAGE_TEXT 0x5d800000U
static unsigned page_memory[TEST_PAGE_SLOTS * TEST_PAGE_SIZE / 4];
static unsigned short page_texts[512][64];
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
#define MANAGE_ENTRY(i) (4 + (i) * 14)
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
static void install_page(u32 slot, const char *title, const char *const *names, u32 rows, u32 handle, u32 tile)
{
    u32 page = TEST_PAGE + slot * TEST_PAGE_SIZE, titles = page + 0x28, summaries = titles + TEXT_LIST,
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
/* The settings rows of Guides, as modules/guides has them: STYLE (handle 0),
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
static u32 native_events;
volatile u32 *menu_header(void) { return test_header; }
u32 menu_event_original(u32 controller, const u16 *event)
{ (void)controller; (void)event; ++native_events; return 42; }
void menu_exit_original(u32 controller) { (void)controller; }
u32 menu_destroy_original(u32 controller) { return controller; }
void menu_refresh_original(u32 controller) { (void)controller; }
void menu_changed_original(u32 controller) { (void)controller; ++native_changes; }

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
    framing_requests = native_aspect_value = native_sets = live_view_stops = 0;
    native_changes = native_events = 0;
    factory_calls = factory_top = 0;
    test_language = 0;
    assert(sizeof(menu_text_table) <= GRKIT_TEXT_SUMMARY);
    memcpy(text_arena, menu_text_table, sizeof(menu_text_table));
    test_header[GRKIT_HOST_COUNT / 4] = 5;
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
    assert(menu_icon(GRKIT_ICON_MODULES - 1) == 0);
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
    /* Off from a native aspect sets the native 3:2 again. */
    {
        u32 sets = native_sets;
        native_aspect_value = 3;
        test_header[GRKIT_HOST_MENU_CURSOR / 4] = 0;
        menu_confirm(test_header);
        assert(native_sets == sets + 1 && native_aspect_value == 0);
        assert(test_header[GRKIT_HOST_FRAME_RATIO / 4] == 0);
    }
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
    _Static_assert(MANAGE_STRIDE == 56 && MANAGE_HEADER == 16, "The test list matches manage.h");
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
    manage_memory[entry + 1] = TEST_PAGE + 2 * TEST_PAGE_SIZE + 0x28;
    manage_memory[entry + 2] = 7;
    entry += 14;
    manage_memory[entry] = MANAGE_LOADED;
    manage_memory[entry + 1] = TEST_PAGE + 3 * TEST_PAGE_SIZE + 0x28;
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
    /* The ADJ Order page lists the title of a loaded module and the name
       of another one. */
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_MODULE;
    test_header[GRKIT_HOST_MODULE / 4] = MANAGE_MODULE;
    assert(same(current_module(test_header)->title(test_header), "ADJ Order"));
    assert(count(test_header) == 3);
    assert(same(MODULE(MANAGE_MODULE).label(test_header, 0), "Guides"));
    assert(same(MODULE(MANAGE_MODULE).label(test_header, 1), "Date Imprint"));
    assert(same(MODULE(MANAGE_MODULE).label(test_header, 2), "BROKEN (52)"));
    /* The settings page of the first group: a row for each module, in
       order, whose values are the positions 1 to 3. */
    test_header[GRKIT_HOST_SETTINGS / 4] = MANAGE_MODULE + 1;
    page = settings_page(test_header);
    assert(page == MODULE(MANAGE_MODULE).settings && settings_count(test_header, page) == 3);
    /* settings_start writes the row count; a row past it gets no hook. */
    ((volatile u8 *)settings_block(test_header))[SUBMENU_KIND_ROWS] = 3;
    assert(!menu_settings_count(0, 0, 3) && !menu_settings_apply(0, 0, 4));
    assert(same(page[0].label(test_header), "Guides") && same(page[1].label(test_header), "Date Imprint"));
    assert(same(page[2].label(test_header), "BROKEN (52)"));
    assert(menu_settings_count(0, 0, 1) == 4);
    assert(same(page[1].value_text(test_header, 0), "1") && same(page[1].value_text(test_header, 2), "3"));
    assert(menu_settings_current(1, 0, 1) == 2 && menu_settings_current(0, 0, 1) == 1);
    /* Position 1 moves Date Imprint first. Each module gets its row + 1. */
    assert(menu_settings_apply(0, 0, 1) == 1);
    assert(settings_memory[2 + 7 * 3 + 1] == 1 && settings_memory[2 + 4 * 3 + 1] == 2 && settings_memory[2 + 10 * 3 + 1] == 3);
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
    /* Position 3 moves it last; the others keep their order. */
    assert(menu_settings_apply(2, 0, 0) == 1);
    assert(same(page[0].label(test_header), "Guides") && same(page[1].label(test_header), "BROKEN (52)"));
    assert(same(page[2].label(test_header), "Date Imprint"));
    /* Position 2 restores the order of this start. */
    assert(menu_settings_apply(1, 0, 2) == 1 && same(page[1].label(test_header), "Date Imprint"));
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
        /* The return reads the rows of the page again: its three modules.
           The six rows of this case are set again. */
        assert(((volatile u8 *)block)[SUBMENU_KIND_ROWS] == 3);
        ((volatile u8 *)block)[SUBMENU_KIND_ROWS] = SETTING_ROWS;
        assert(menu_text_override(view, row_texts[SETTINGS_ALIAS]) == ((u32)(block + SETTINGS_LABELS / 4 + 2) | 1U));
        assert(menu_text_override(view, row_texts[SETTING_ROWS - 1]) ==
               ((u32)(block + SETTINGS_LABELS / 4 + SETTING_ROWS * 2) | 1U));
        /* The value page of the first row is its own. */
        assert(menu_settings_row_controller(pages[SETTINGS_ALIAS], 0) == pages[SETTINGS_ALIAS]);
        assert(menu_settings_controller(pages[SETTINGS_ALIAS]) == pages[SETTINGS_ALIAS]);
        assert(!(test_header[GRKIT_HOST_SETTINGS / 4] & SETTINGS_OPEN_ROWS));
        ((volatile u8 *)block)[SUBMENU_KIND_ROWS] = 3;
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

/* ADJ Order has a row for each listed module. The kind table entry has
   room for the labels of six rows and the controllers of five: a later
   label shows in a view slot, and OK on a later row opens the value page
   of SETTINGS_ALIAS for that row. */
static void the_adj_order_page_has_a_row_for_each_module(void)
{
    static const u32 pages[] = SETTINGS_PAGES;
    volatile u32 *h = test_header, *block;
    const u32 last = MANAGE_MODULES - 1, native = 0x1234, view = 0x5b000000;
    _Static_assert(MANAGE_HEADER + MANAGE_MODULES * MANAGE_STRIDE <= TEST_EFFECTS - TEST_MANAGE &&
                   sizeof(settings_memory) >= 4 * (2 + 3 * (3 + 3 * MANAGE_MODULES)), "The test memory holds the list");
    reset();
    settings_memory[0] = 3 + 3 * MANAGE_MODULES;
    for (u32 i = 0; i < MANAGE_MODULES; ++i) {
        u32 entry = MANAGE_ENTRY(i);
        char name[] = "Module 0";
        settings_memory[2 + (3 + 3 * i) * 3 + 2] = 2;
        settings_memory[2 + (4 + 3 * i) * 3 + 2] = MANAGE_MODULES + 1;
        settings_memory[2 + (5 + 3 * i) * 3 + 2] = 1U << MANAGE_OPTION_VALUES;
        manage_memory[entry] = 52;
        manage_memory[entry + 2] = MANAGE_NO_TILE;
        name[7] = (char)('0' + i);
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
        manage_memory[entry + 1] = TEST_PAGE + i * TEST_PAGE_SIZE + 0x28;
        manage_memory[entry + 2] = MANAGE_NO_TILE;
    }
    h[GRKIT_HOST_MANAGE / 4] = TEST_MANAGE;
    h[GRKIT_HOST_EFFECTS / 4] = TEST_EFFECTS;
    install_guide_rows(2);
    *page_at(guides + PAGE_FLAGS) |= CHOICE_OPTIONS;
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

int main(void)
{
    the_root_reads_off_without_a_committed_effect();
    each_language_reads_its_own_texts_and_font();
    the_viewfinder_shows_the_committed_record();
    fn_applies_to_effect_rows_of_film_look();
    switch_presses_and_back_events_are_filtered();
    list_restoration_keeps_the_committed_effect();
    the_event_ring_tells_menu_from_ok_and_adj();
    playback_matches_private_tags_only();
    icons_toolbar_and_edges_map_their_ids();
    adj_exits_return_once_for_the_parameter_page();
    a_half_press_keeps_the_parameter_session();
    the_root_lists_the_official_modules();
    module_tiles_follow_the_effect_icons();
    the_framing_page_sets_the_ratios_of_the_target();
    the_guides_page_commits_only_its_style();
    an_exit_from_a_page_commits_the_highlighted_row();
    the_date_imprint_page_lists_its_styles();
    the_grain_page_changes_only_the_grain_word();
    invalid_cursors_form_no_effect_pointer();
    film_look_previews_and_commits_the_highlighted_effect();
    the_root_has_no_about_row();
    a_page_past_the_page_count_has_no_root_row();
    the_root_shows_only_visible_module_pages();
    adj_reopens_on_the_row_of_the_last_module();
    the_date_imprint_settings_page_reads_and_stores_values();
    the_adj_order_page_moves_modules();
    the_adj_order_page_has_a_row_for_each_module();
    options_hide_values_that_are_off_in_adj();
    enable_applies_at_once();
    film_look_lists_the_installed_effects_that_are_on();
    /* The MENU page blocks after the first continue the screen that the block
       before them leaves. */
    the_guides_menu_page_lists_enable_options_and_settings();
    the_options_page_toggles_its_values();
#if defined(FW_CHECK_ITEM) && defined(FW_MENU_TABS)
    the_options_page_shows_checkboxes();
#endif
    reused_row_views_show_the_page_that_reuses_them();
    the_film_look_menu_page_lists_film_stocks();
    a_section_row_shows_its_module_and_value();
    about_grkit_shows_the_version_and_address();
#ifdef FW_MENU_TABS
    the_section_rows_follow_the_listed_modules();
    a_full_tab_gets_no_new_section();
#endif
    each_selection_change_counts_once();
    record_index_finds_each_record_start();
    return 0;
}
