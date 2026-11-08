#include <assert.h>
#include <string.h>

typedef unsigned char u8;
typedef unsigned int u32;
#include "module_link.h"

/* A module file built by hand (crates/grkit-core/src/grm.rs), with code
   words at these indexes: */
enum {
    PAINT = 0,       /* Thumb code */
    DATA_WORD = 1,   /* a word to the data */
    IMPORT = 2,      /* an import word */
    LAYER = 3,       /* {size, flags, paint, setting, order} */
    REGISTRATIONS = 8, /* {point, impl} x 3: the layer, the page, the filter */
    DESCRIPTOR = 14, /* 10 words */
    SETTING = 24,    /* {size, key, values, flags} x 2 */
    KEY = 32,        /* "MODE" */
    KEY2 = 34,       /* "LOOK" */
    USES = 36,       /* one name pointer */
    USE_NAME = 37,   /* "?CORE.ASPECT" */
    PAGE = 41,       /* {size, kind, setting, flags, title, summaries, labels, rows, setting rows, settings,
                        children, icon, fn value, fn setting} */
    /* Each text list has one pointer for each language (L). */
    TITLE = 55,                     /* L text pointers */
    SUMMARIES = TITLE + GRM_LANGUAGES,   /* 2 values x L */
    LABELS = SUMMARIES + 2 * GRM_LANGUAGES, /* 2 values x L */
    ROWS = LABELS + 2 * GRM_LANGUAGES,   /* {size, setting, label, values} x 2 */
    /* Row 0: L labels, 2 values x L; row 1: L labels, 3 values x L. */
    ROW_TEXTS = ROWS + 8,
    ROW1_TEXTS = ROW_TEXTS + 3 * GRM_LANGUAGES,
    FILTER = ROW1_TEXTS + 4 * GRM_LANGUAGES, /* {size, stage, setting, apply} */
    TEXT = FILTER + 4,              /* u"Hi" */
    TILES = TEXT + 1,               /* 2 tiles */
    CODE_WORDS = TILES + 2 * GRM_TILE_BYTES / 4
};
static u32 code_words[CODE_WORDS], data_words[2];
static u32 relocations[128], relocation_count;
static u32 import_hash, abi;
static const char *id;
static u8 file[GRM_HEADER + CODE_WORDS * 4 + 1024 + 3 * GRM_TILE_BYTES];
/* The tiles after the import names (ABI 2.3), filled with 0xa5. */
static u32 file_tiles;
static u32 file_size;

static u32 fnv(const char *s)
{
    u32 h = 0x811c9dc5;
    while (*s) { h ^= (u8)*s++; h *= 0x01000193; }
    return h;
}
/* A word that holds the link address of code word `target`, plus `extra`. */
static void pointer(u32 word, u32 target, u32 extra)
{
    code_words[word] = target * 4 + extra;
    relocations[relocation_count++] = word * 4;
}
static void bytes(u32 word, const void *s, u32 n) { memcpy(&code_words[word], s, n); }
static void reset(void)
{
    memset(code_words, 0, sizeof code_words);
    relocation_count = 0;
    file_tiles = 0;
    abi = GRM_ABI_MAJOR << 16 | GRM_ABI_MINOR;
    id = "PROBE";
    import_hash = fnv("overlay_label");
    code_words[PAINT] = 0x4770bf00;
    code_words[DATA_WORD] = GRM_DATA_LINK + 4;
    relocations[relocation_count++] = DATA_WORD * 4;
    code_words[LAYER] = 20; code_words[LAYER + 1] = GRM_LAYER_FOLLOW_DISP;
    pointer(LAYER + 2, PAINT, 1);
    code_words[LAYER + 3] = 0; code_words[LAYER + 4] = 20;
    code_words[REGISTRATIONS] = GRM_POINT_OVERLAY_LAYER; pointer(REGISTRATIONS + 1, LAYER, 0);
    code_words[REGISTRATIONS + 2] = GRM_POINT_MENU_PAGE; pointer(REGISTRATIONS + 3, PAGE, 0);
    code_words[REGISTRATIONS + 4] = GRM_POINT_PICTURE_PLANE; pointer(REGISTRATIONS + 5, FILTER, 0);
    code_words[DESCRIPTOR] = GRM_DESCRIPTOR; code_words[DESCRIPTOR + 1] = abi;
    pointer(DESCRIPTOR + 2, REGISTRATIONS, 0); code_words[DESCRIPTOR + 3] = 3;
    pointer(DESCRIPTOR + 4, SETTING, 0); code_words[DESCRIPTOR + 5] = 2;
    pointer(DESCRIPTOR + 6, USES, 0); code_words[DESCRIPTOR + 7] = 1;
    pointer(DESCRIPTOR + 8, TILES, 0); code_words[DESCRIPTOR + 9] = 2;
    code_words[SETTING] = 16; pointer(SETTING + 1, KEY, 0);
    code_words[SETTING + 2] = 2; code_words[SETTING + 3] = GRM_SETTING_SAVED;
    bytes(KEY, "MODE", 5);
    code_words[SETTING + 4] = 16; pointer(SETTING + 5, KEY2, 0);
    code_words[SETTING + 6] = 3; code_words[SETTING + 7] = 0;
    bytes(KEY2, "LOOK", 5);
    pointer(USES, USE_NAME, 0);
    bytes(USE_NAME, "?CORE.ASPECT", 13);
    code_words[PAGE] = 40; code_words[PAGE + 1] = GRM_PAGE_CHOICE; code_words[PAGE + 7] = 2;
    code_words[PAGE + 2] = 0; code_words[PAGE + 3] = GRM_CHOICE_OFF_ROW;
    pointer(PAGE + 4, TITLE, 0); pointer(PAGE + 5, SUMMARIES, 0); pointer(PAGE + 6, LABELS, 0);
    for (u32 i = TITLE; i < ROWS; ++i) pointer(i, TEXT, 0);
    /* The rows of a native settings page, which a page has only with
       setting_rows (PAGE + 8): MODE, then LOOK. */
    code_words[ROWS] = 16; code_words[ROWS + 1] = 0;
    pointer(ROWS + 2, ROW_TEXTS, 0); pointer(ROWS + 3, ROW_TEXTS + GRM_LANGUAGES, 0);
    code_words[ROWS + 4] = 16; code_words[ROWS + 5] = 1;
    pointer(ROWS + 6, ROW1_TEXTS, 0); pointer(ROWS + 7, ROW1_TEXTS + GRM_LANGUAGES, 0);
    for (u32 i = ROW_TEXTS; i < FILTER; ++i) pointer(i, TEXT, 0);
    pointer(PAGE + 9, ROWS, 0);
    code_words[FILTER] = 16; code_words[FILTER + 1] = GRM_STAGE_MARK; code_words[FILTER + 2] = 0;
    pointer(FILTER + 3, PAINT, 1);
    static const unsigned short hi[] = {'H', 'i', 0};
    bytes(TEXT, hi, sizeof hi);
}
static void build(void)
{
    u8 *p = file;
    /* The relocations in ascending order, as the module format has them. */
    for (u32 i = 1; i < relocation_count; ++i)
        for (u32 j = i; j && relocations[j - 1] > relocations[j]; --j) {
            u32 t = relocations[j]; relocations[j] = relocations[j - 1]; relocations[j - 1] = t;
        }
    code_words[DESCRIPTOR + 1] = abi;
    memset(file, 0, sizeof file);
    grm_put(p, GRM_MAGIC); grm_put(p + 4, GRM_FORMAT); grm_put(p + 8, abi);
    memcpy(p + 12, id, strlen(id));
    grm_put(p + 20, 1); grm_put(p + 24, CODE_WORDS * 4); grm_put(p + 28, sizeof data_words);
    grm_put(p + 32, 8); grm_put(p + 36, DESCRIPTOR * 4);
    grm_put(p + 40, relocation_count); grm_put(p + 44, 2);
    grm_put(p + 48, 1); grm_put(p + 52, 16);
    p += GRM_HEADER;
    memcpy(p, code_words, CODE_WORDS * 4); p += CODE_WORDS * 4;
    data_words[0] = 1; data_words[1] = GRM_DATA_LINK + 8;
    memcpy(p, data_words, 8); p += 8;
    for (u32 i = 0; i < relocation_count; ++i, p += 4) grm_put(p, relocations[i]);
    grm_put(p, 0); grm_put(p + 4, 4); p += 8;
    grm_put(p, import_hash); grm_put(p + 4, IMPORT * 4); p += 8;
    memcpy(p, "overlay_label\0\0\0", 16); p += 16;
    grm_put(file + 60, file_tiles);
    memset(p, 0xa5, file_tiles * GRM_TILE_BYTES); p += file_tiles * GRM_TILE_BYTES;
    file_size = (u32)(p - file);
}
/* Move the tiles out of the code, as grkit module build does since ABI 2.3:
   the descriptor has no tile address, and n tiles follow the file. */
static void tiles_after_file(u32 n)
{
    code_words[DESCRIPTOR + 8] = 0;
    for (u32 i = 0; i < relocation_count; ++i)
        if (relocations[i] == (DESCRIPTOR + 8) * 4) relocations[i--] = relocations[--relocation_count];
    code_words[DESCRIPTOR + 9] = n;
    file_tiles = n;
}

#define CODE_BASE 0x543dc000U
#define DATA_BASE 0x59200000U
static const struct grm_symbol symbols[] = {{0x12345678, 1}, {0xc0e27352, 0x543d6801}};
static u8 code[GRM_MAX_SEGMENT], data[GRM_MAX_SEGMENT];
static struct grm_module m;
static struct grm_setting settings[GRM_SETTINGS];
static char uses[GRM_USES][GRM_NAME + 1];
static u32 setting_count, use_count;
static struct grm_layer layers[2];
static struct grm_page pages[2];
static struct grm_filter filters[2];
static u32 layer_count, page_count, filter_count;

static int link(u32 *failed)
{
    build();
    int status = grm_parse(file, file_size, &m);
    if (status) return status;
    return grm_link(&m, code, data, CODE_BASE, DATA_BASE, symbols, 2, failed);
}
/* Link, then read the settings and the registrations. */
static int registrations(void)
{
    u32 failed;
    int status = link(&failed);
    if (!status) status = grm_settings(&m, code, CODE_BASE, settings, &setting_count, uses, &use_count);
    layer_count = page_count = filter_count = 0;
    if (!status) status = grm_registrations(&m, code, CODE_BASE, settings, setting_count, use_count,
                                            layers, &layer_count, pages, &page_count, filters, &filter_count, 2);
    return status;
}

/* Reuse a tile's bytes for descriptor storage in this synthetic module.
   The linker does not interpret the artwork pixels. */
#define INDICATOR_AT TILES
#define CONDITIONS_AT (TILES + 6)
#define ICON_AT (TILES + 9)
static void indicator_reset(void)
{
    reset();
    code_words[REGISTRATIONS + 4] = GRM_POINT_VIEWFINDER_INDICATOR;
    code_words[REGISTRATIONS + 5] = INDICATOR_AT * 4;
    code_words[INDICATOR_AT] = GRM_INDICATOR;
    code_words[INDICATOR_AT + 1] = GRM_INDICATOR_STATUS_ROW;
    code_words[INDICATOR_AT + 3] = 1;
    pointer(INDICATOR_AT + 4, CONDITIONS_AT, 0);
    pointer(INDICATOR_AT + 5, ICON_AT, 0);
    code_words[CONDITIONS_AT] = 0;
    code_words[CONDITIONS_AT + 1] = GRM_CONDITION_EQUALS;
    code_words[CONDITIONS_AT + 2] = 1;
    code_words[ICON_AT] = GRM_ICON;
    code_words[ICON_AT + 1] = GRM_ICON_TILE;
    code_words[ICON_AT + 2] = 0;
}
/* Keep the original 20-byte layer fixture intact for compatibility checks. */
#define CONDITIONAL_LAYER_AT TILES
#define LAYER_CONDITIONS_AT (TILES + 7)
static void conditional_layer_reset(void)
{
    reset();
    code_words[REGISTRATIONS + 1] = CONDITIONAL_LAYER_AT * 4;
    code_words[CONDITIONAL_LAYER_AT] = GRM_LAYER_CONDITIONAL;
    pointer(CONDITIONAL_LAYER_AT + 2, PAINT, 1);
    code_words[CONDITIONAL_LAYER_AT + 4] = 20;
    code_words[CONDITIONAL_LAYER_AT + 5] = 1;
    pointer(CONDITIONAL_LAYER_AT + 6, LAYER_CONDITIONS_AT, 0);
    code_words[LAYER_CONDITIONS_AT] = 1;
    code_words[LAYER_CONDITIONS_AT + 1] = GRM_CONDITION_EQUALS;
    code_words[LAYER_CONDITIONS_AT + 2] = 0;
}
static void layers_validate_optional_conditions(void)
{
    reset();
    abi = GRM_ABI_MAJOR << 16 | 2;
    assert(registrations() == 0 && layers[0].descriptor == 0);
    conditional_layer_reset();
    assert(registrations() == 0 && layers[0].descriptor == CODE_BASE + CONDITIONAL_LAYER_AT * 4);
    conditional_layer_reset(); code_words[LAYER_CONDITIONS_AT + 1] = GRM_CONDITION_NOT_EQUALS;
    assert(registrations() == 0);
    conditional_layer_reset(); code_words[LAYER_CONDITIONS_AT] = GRM_USED;
    assert(registrations() == 0); /* Resolve the used setting in INSTALL. */
    conditional_layer_reset(); code_words[CONDITIONAL_LAYER_AT + 5] = 0;
    assert(registrations() == GRM_BAD_DESCRIPTOR); /* No conditions needs a null pointer. */
    conditional_layer_reset(); code_words[CONDITIONAL_LAYER_AT + 5] = 0;
    code_words[CONDITIONAL_LAYER_AT + 6] = 0;
    /* A null pointer is not a code relocation. */
    --relocation_count;
    assert(registrations() == 0 && layers[0].descriptor == 0);
    for (u32 size = GRM_LAYER + 1; size < GRM_LAYER_CONDITIONAL; ++size) {
        conditional_layer_reset(); code_words[CONDITIONAL_LAYER_AT] = size;
        assert(registrations() == GRM_BAD_DESCRIPTOR);
    }
    conditional_layer_reset(); code_words[CONDITIONAL_LAYER_AT + 5] = GRM_CONDITIONS_MAX + 1;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    conditional_layer_reset(); code_words[LAYER_CONDITIONS_AT] = 2;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    conditional_layer_reset(); code_words[LAYER_CONDITIONS_AT + 1] = 3;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    conditional_layer_reset(); code_words[LAYER_CONDITIONS_AT + 2] = 3;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    conditional_layer_reset(); ++code_words[CONDITIONAL_LAYER_AT + 6];
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    conditional_layer_reset(); code_words[CONDITIONAL_LAYER_AT + 6] = CODE_WORDS * 4 - 4;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
}

static void indicators_validate_resources_and_conditions(void)
{
    u32 indicators[GRM_REGISTRATIONS];
    indicator_reset();
    assert(registrations() == 0);
    assert(grm_indicators(&m, code, CODE_BASE, indicators) == 1 && indicators[0] == CODE_BASE + INDICATOR_AT * 4);
    assert(layer_count == 1 && page_count == 1 && filter_count == 0);
    indicator_reset(); code_words[CONDITIONS_AT + 2] = 0;
    assert(registrations() == 0); /* Zero is a valid condition operand. */
    indicator_reset(); code_words[INDICATOR_AT + 1] = GRM_INDICATOR_NATIVE_SLOT;
    code_words[INDICATOR_AT + 2] = GRM_SLOT_IMAGE_CONTROL;
    assert(registrations() == 0);
    code_words[INDICATOR_AT + 2] = 2;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    indicator_reset(); code_words[INDICATOR_AT + 2] = GRM_SLOT_IMAGE_CONTROL;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    indicator_reset(); code_words[INDICATOR_AT + 3] = 0;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    indicator_reset(); code_words[INDICATOR_AT + 3] = GRM_CONDITIONS_MAX + 1;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    indicator_reset(); code_words[CONDITIONS_AT] = 2;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    indicator_reset(); code_words[CONDITIONS_AT + 1] = 3;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    indicator_reset(); code_words[CONDITIONS_AT + 2] = 2;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    indicator_reset(); code_words[ICON_AT + 3] = 1;
    assert(registrations() == GRM_BAD_DESCRIPTOR); /* Two values need two tiles. */
    code_words[ICON_AT + 2] = GRM_NO_SETTING;
    assert(registrations() == 0); /* A fixed tile needs only one. */
    indicator_reset(); code_words[ICON_AT + 1] = GRM_ICON_TONE;
    assert(registrations() == GRM_BAD_DESCRIPTOR); /* Tone requires a used setting. */
    indicator_reset(); code_words[ICON_AT + 2] = GRM_USED;
    assert(registrations() == 0); /* Full range checked when uses resolve. */
    indicator_reset(); code_words[INDICATOR_AT + 4] = CODE_WORDS * 4 - 4;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    /* A status-row indicator can show a core label: label first + value, or
       the label first without a setting. Every value needs a label. */
    indicator_reset(); code_words[ICON_AT + 1] = GRM_ICON_LABEL; code_words[ICON_AT + 3] = GRM_LABELS - 2;
    assert(registrations() == 0); /* MODE has two values: labels 7 and 8. */
    code_words[ICON_AT + 3] = GRM_LABELS - 1;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    code_words[ICON_AT + 2] = GRM_NO_SETTING;
    assert(registrations() == 0);
    code_words[ICON_AT + 3] = GRM_LABELS;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    indicator_reset(); code_words[ICON_AT + 1] = GRM_ICON_LABEL; code_words[ICON_AT + 2] = GRM_USED;
    assert(registrations() == 0); /* Checked when uses resolve. */
    /* The Image Control slot cannot show a label. */
    indicator_reset(); code_words[ICON_AT + 1] = GRM_ICON_LABEL; code_words[ICON_AT + 2] = GRM_NO_SETTING;
    code_words[INDICATOR_AT + 1] = GRM_INDICATOR_NATIVE_SLOT; code_words[INDICATOR_AT + 2] = GRM_SLOT_IMAGE_CONTROL;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
}

int main(void)
{
    indicators_validate_resources_and_conditions();
    layers_validate_optional_conditions();
    u32 failed = 99;
    const u8 *tiles;
    u32 tile_count;

    /* A valid module links, and gives its settings, layer, page, and tiles. */
    reset();
    assert(fnv("overlay_label") == 0xc0e27352);
    assert(registrations() == 0);
    assert(strcmp(m.id, "PROBE") == 0 && m.bss_size == 8);
    assert(grm_word(code + 4) == DATA_BASE + 4);
    assert(grm_word(code + IMPORT * 4) == 0x543d6801);
    assert(grm_word(code + (LAYER + 2) * 4) == CODE_BASE + 1);
    assert(grm_word(data) == CODE_BASE + 1 && grm_word(data + 4) == DATA_BASE + 8);
    assert(setting_count == 2 && strcmp(settings[0].key, "MODE") == 0 && strcmp(settings[1].key, "LOOK") == 0);
    assert(settings[0].values == 2 && settings[0].flags == GRM_SETTING_SAVED);
    assert(settings[0].default_value == 0 && settings[1].default_value == 0);
    assert(use_count == 1 && strcmp(uses[0], "?CORE.ASPECT") == 0);
    assert(layer_count == 1 && layers[0].paint == CODE_BASE + 1 && layers[0].setting == 0);
    assert(layers[0].flags == GRM_LAYER_FOLLOW_DISP && layers[0].order == 20);
    assert(page_count == 1 && pages[0].address == CODE_BASE + PAGE * 4 && pages[0].flags == GRM_CHOICE_OFF_ROW);
    assert(pages[0].setting == 0 && pages[0].rows == 2);
    assert(filter_count == 1 && filters[0].apply == CODE_BASE + 1 && filters[0].setting == 0 &&
           filters[0].stage == GRM_STAGE_MARK);
    assert(grm_tiles(&m, code, CODE_BASE, &tiles, &tile_count) == 0 && tile_count == 2 && tiles == code + TILES * 4);
    /* The same from the file itself, before the link. */
    assert(grm_settings(&m, m.code, 0, settings, &setting_count, uses, &use_count) == 0 && setting_count == 2);
    assert(grm_tiles(&m, m.code, 0, &tiles, &tile_count) == 0 && tile_count == 2);
    /* The state ID is the one of grkit_core::grm::state_id. */
    assert(grm_state_id("GUIDES.STYLE") == 604351797U);
    assert(grm_state_id("PROBE.MODE") == 221689958U);

    /* ABI 2.0 uses 16-byte settings and defaults to zero. */
    reset(); abi = GRM_ABI_MAJOR << 16;
    assert(registrations() == 0 && setting_count == 2);
    assert(settings[0].default_value == 0 && settings[1].default_value == 0);

    /* Extended settings use their size as their stride. Read two defaults,
       accept a future trailing word, and reject invalid or truncated entries. */
    reset();
    code_words[DESCRIPTOR + 4] = TILES * 4;
    code_words[TILES] = 24; code_words[TILES + 1] = KEY * 4;
    code_words[TILES + 2] = 2; code_words[TILES + 3] = GRM_SETTING_SAVED;
    code_words[TILES + 4] = 1;
    code_words[TILES + 6] = 20; code_words[TILES + 7] = KEY2 * 4;
    code_words[TILES + 8] = 3; code_words[TILES + 9] = 0; code_words[TILES + 10] = 2;
    relocations[relocation_count++] = (TILES + 1) * 4;
    relocations[relocation_count++] = (TILES + 7) * 4;
    assert(registrations() == 0 && setting_count == 2);
    assert(settings[0].default_value == 1 && settings[1].default_value == 2);
    code_words[TILES + 10] = 3;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    code_words[TILES + 10] = 2;
    code_words[TILES] = 18;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    code_words[TILES] = 0xfffffffcU;
    assert(registrations() == GRM_BAD_DESCRIPTOR);

    /* A settings entry has one module tile and no choice texts. Its first
       native row still owns the setting and the count of its values. */
    reset(); code_words[PAGE + 1] = GRM_PAGE_SETTINGS; code_words[PAGE + 3] = 0; code_words[PAGE + 8] = 2;
    code_words[DESCRIPTOR + 9] = 1;
    code_words[PAGE + 5] = 0; code_words[PAGE + 6] = 0;
    assert(registrations() == 0 && page_count == 1);
    code_words[PAGE + 8] = 0;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    code_words[PAGE + 8] = 2; code_words[PAGE + 3] = GRM_CHOICE_OPTIONS;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    code_words[PAGE + 3] = 0; code_words[DESCRIPTOR + 9] = 0;
    assert(registrations() == GRM_BAD_DESCRIPTOR);

    /* Settings that the installer does not accept. */
    reset(); bytes(KEY, "Mode", 5);
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    reset(); code_words[SETTING + 2] = GRM_SETTING_VALUES + 1;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    reset(); code_words[SETTING + 3] = 8;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    /* ABI 2.4: a color-only setting. */
    reset(); code_words[SETTING + 3] = GRM_SETTING_SAVED | GRM_SETTING_COLOR_ONLY;
    assert(registrations() == 0);

    /* ABI 2.4: the flags of a module after the 40 bytes of the descriptor. A
       40-byte descriptor has none. The word after it (here the size of the
       first setting) is the flags of a 44-byte one. */
    {
        u32 flags = 99;
        reset(); build();
        assert(grm_parse(file, file_size, &m) == 0 && grm_module_flags(&m, m.code, &flags) == 0 && flags == 0);
        reset(); code_words[DESCRIPTOR] = GRM_DESCRIPTOR_FLAGS; code_words[SETTING] = GRM_MODULE_COLOR_ONLY; build();
        assert(grm_parse(file, file_size, &m) == 0 && grm_module_flags(&m, m.code, &flags) == 0);
        assert(flags == GRM_MODULE_COLOR_ONLY);
        reset(); code_words[DESCRIPTOR] = GRM_DESCRIPTOR_FLAGS; code_words[SETTING] = 2; build();
        assert(grm_parse(file, file_size, &m) == 0 && grm_module_flags(&m, m.code, &flags) == GRM_BAD_DESCRIPTOR);
    }

    /* Layers: an unknown flag, a setting that the module does not have. A
       layer without a setting is valid. */
    reset(); code_words[LAYER + 1] = 8;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    /* A layer follows DISP, or shows only with the full display, not both. */
    reset(); code_words[LAYER + 1] = GRM_LAYER_FOLLOW_DISP | GRM_LAYER_FULL_DISP;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    reset(); code_words[LAYER + 1] = GRM_LAYER_FULL_DISP | GRM_LAYER_LIVE_ONLY;
    assert(registrations() == 0 && layers[0].flags == (GRM_LAYER_FULL_DISP | GRM_LAYER_LIVE_ONLY));
    reset(); code_words[LAYER + 3] = 2;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    /* The core sorts layers by a 16-bit order. */
    reset(); code_words[LAYER + 4] = GRM_MAX_LAYER_ORDER;
    assert(registrations() == 0 && layers[0].order == GRM_MAX_LAYER_ORDER);
    reset(); code_words[LAYER + 4] = GRM_MAX_LAYER_ORDER + 1;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    reset(); code_words[LAYER + 3] = GRM_NO_SETTING;
    assert(registrations() == 0 && layers[0].setting == GRM_NO_SETTING);
    /* A layer of a used setting: the installer gives its handle. */
    reset(); code_words[LAYER + 3] = GRM_USED;
    assert(registrations() == 0 && layers[0].setting == GRM_USED);
    reset(); code_words[LAYER + 3] = GRM_USED | 1;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    /* A paint function at the end of the code, or past it. */
    reset(); code_words[LAYER + 2] = CODE_WORDS * 4 + 1;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    reset(); code_words[LAYER + 2] = CODE_WORDS * 4 + 3;
    assert(registrations() == GRM_BAD_FILE);

    /* Pages: another kind, a size without the rows, rows that are not the
       values of its own setting, fewer tiles than values, a text without its
       NUL. */
    reset(); code_words[PAGE + 1] = 2;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    reset(); code_words[PAGE] = 36;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    /* Native settings rows: settings of the module, the first the page
       setting, each with its texts. The page setting alone is a row too. */
    reset(); code_words[PAGE + 8] = 2;
    assert(registrations() == 0 && page_count == 1);
    reset(); code_words[PAGE + 8] = 1;
    assert(registrations() == 0 && page_count == 1);
    reset(); code_words[PAGE + 8] = GRM_SETTING_ROWS + 1;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    reset(); code_words[PAGE + 8] = 2; code_words[ROWS + 1] = 1;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    reset(); code_words[PAGE + 8] = 2; code_words[ROWS + 5] = 2;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    reset(); code_words[PAGE + 8] = 2; code_words[SETTING + 6] = GRM_SETTING_ROW_VALUES + 1;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    reset(); code_words[PAGE + 8] = 2; code_words[ROWS + 7] = TEXT * 4 + 1;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    reset(); code_words[PAGE + 8] = 2; code_words[PAGE + 2] = GRM_USED; code_words[PAGE + 7] = 1;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    /* A page whose rows the core gives: a used setting, and only its title. */
    reset(); code_words[PAGE + 2] = GRM_USED; code_words[PAGE + 3] = GRM_CHOICE_CORE_ROWS; code_words[PAGE + 7] = 0;
    assert(registrations() == 0 && pages[0].flags == GRM_CHOICE_CORE_ROWS && pages[0].rows == 0);
    reset(); code_words[PAGE + 3] = GRM_CHOICE_CORE_ROWS; code_words[PAGE + 7] = 0;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    reset(); code_words[PAGE + 2] = GRM_USED; code_words[PAGE + 3] = GRM_CHOICE_CORE_ROWS;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    /* A page can have options; another flag is an error. */
    reset(); code_words[PAGE + 3] = GRM_CHOICE_OPTIONS;
    assert(registrations() == 0 && page_count == 1);
    reset(); code_words[PAGE + 3] = GRM_PAGE_MENU_ONLY;
    assert(registrations() == 0 && pages[0].flags == GRM_PAGE_MENU_ONLY);
    reset(); code_words[PAGE + 3] = 16;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    /* ABI 2.5: Fn on the row of a value other than Off steps another setting
       with two values or more, of the module or used. Value 0 ignores the
       setting. A page that the module does not give rows has none. */
    reset(); code_words[PAGE] = GRM_PAGE_FN; code_words[PAGE + 12] = 1; code_words[PAGE + 13] = 1;
    assert(registrations() == 0 && page_count == 1);
    reset(); code_words[PAGE] = GRM_PAGE_FN; code_words[PAGE + 12] = 1; code_words[PAGE + 13] = GRM_USED;
    assert(registrations() == 0);
    reset(); code_words[PAGE] = GRM_PAGE_FN; code_words[PAGE + 13] = 9;
    assert(registrations() == 0);
    reset(); code_words[PAGE] = GRM_PAGE_FN; code_words[PAGE + 12] = 1; code_words[PAGE + 13] = 0;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    reset(); code_words[PAGE] = GRM_PAGE_FN; code_words[PAGE + 12] = 2; code_words[PAGE + 13] = 1;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    reset(); code_words[PAGE] = GRM_PAGE_FN; code_words[PAGE + 12] = 1; code_words[PAGE + 13] = 2;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    reset(); code_words[PAGE] = GRM_PAGE_FN; code_words[PAGE + 12] = 1; code_words[PAGE + 13] = 1;
    code_words[SETTING + 6] = 1;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    reset(); code_words[PAGE] = GRM_PAGE_FN; code_words[PAGE + 12] = 1; code_words[PAGE + 13] = 1;
    code_words[PAGE + 1] = GRM_PAGE_SETTINGS; code_words[PAGE + 3] = 0; code_words[PAGE + 8] = 2;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    reset(); code_words[PAGE] = GRM_PAGE_FN; code_words[PAGE + 12] = 1; code_words[PAGE + 13] = 1;
    code_words[PAGE + 2] = GRM_USED; code_words[PAGE + 3] = GRM_CHOICE_CORE_ROWS; code_words[PAGE + 7] = 0;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    /* Filters: a stage of the core, a setting that the module has, and
       apply code. */
    reset(); code_words[FILTER + 1] = GRM_STAGE_FRAME;
    assert(registrations() == 0 && filters[0].stage == GRM_STAGE_FRAME);
    reset(); code_words[FILTER + 1] = 10;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    reset(); code_words[FILTER + 2] = 2;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    reset(); code_words[FILTER + 2] = GRM_USED;
    assert(registrations() == 0 && filters[0].setting == GRM_USED);
    reset(); code_words[FILTER] = 12;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    reset(); code_words[FILTER + 3] = CODE_WORDS * 4 + 1;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    reset(); code_words[PAGE + 7] = 1;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    /* A page of a used setting has its own rows, which the installer checks
       against the setting. It needs a tile and the text of each row, and a
       used setting that the module names. */
    reset(); code_words[PAGE + 2] = GRM_USED; code_words[PAGE + 7] = 1;
    assert(registrations() == 0 && pages[0].setting == GRM_USED && pages[0].rows == 1);
    reset(); code_words[PAGE + 2] = GRM_USED; code_words[PAGE + 7] = 3;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    reset(); code_words[PAGE + 2] = GRM_USED; code_words[PAGE + 7] = 0;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    reset(); code_words[PAGE + 2] = GRM_USED | 1;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    reset(); code_words[DESCRIPTOR + 9] = 1;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    reset(); for (u32 i = 0; i <= GRM_TEXT_UNITS / 2; ++i) code_words[TILES + i] = 0x00410041;
    for (u32 i = TITLE; i < ROWS; ++i) code_words[i] = TILES * 4;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    /* An unknown point. More registrations than room. */
    reset(); code_words[REGISTRATIONS] = 7;
    assert(registrations() == GRM_BAD_DESCRIPTOR);
    reset();
    {
        u32 status = (u32)link(&failed);
        assert(!status && grm_settings(&m, code, CODE_BASE, settings, &setting_count, uses, &use_count) == 0);
        layer_count = 0; page_count = 0; filter_count = 0;
        assert(grm_registrations(&m, code, CODE_BASE, settings, setting_count, use_count, layers, &layer_count,
                                 pages, &page_count, filters, &filter_count, 0) == GRM_NO_ROOM);
    }

    /* An import that the core does not export. An import word that is not 0. */
    reset(); import_hash = fnv("missing");
    assert(link(&failed) == GRM_MISSING_IMPORT && failed == 0);
    reset(); code_words[IMPORT] = 4;
    assert(link(&failed) == GRM_BAD_FILE);

    /* Another major ABI, or a newer minor one. */
    reset(); abi = (GRM_ABI_MAJOR + 1) << 16; build();
    assert(grm_parse(file, file_size, &m) == GRM_BAD_ABI);
    reset(); abi = GRM_ABI_MAJOR << 16 | (GRM_ABI_MINOR + 1); build();
    assert(grm_parse(file, file_size, &m) == GRM_BAD_ABI);

    /* Invalid files. */
    reset(); id = "probe"; build();
    assert(grm_parse(file, file_size, &m) == GRM_BAD_FILE);
    reset(); build();
    assert(grm_parse(file, file_size - 4, &m) == GRM_BAD_FILE);
    file[0] ^= 1;
    assert(grm_parse(file, file_size, &m) == GRM_BAD_FILE);
    /* Unsorted or unaligned relocations. */
    reset(); relocations[1] = relocations[0];
    assert(link(&failed) == GRM_BAD_FILE);
    reset(); relocations[0] = 6;
    assert(link(&failed) == GRM_BAD_FILE);
    /* Offsets near 2^32 must not wrap past the bounds checks: a relocation
       and the descriptor. */
    reset(); relocations[relocation_count++] = 0xfffffffcU;
    assert(link(&failed) == GRM_BAD_FILE);
    reset(); build(); grm_put(file + 36, 0xffffffdcU);
    assert(grm_parse(file, file_size, &m) == GRM_BAD_FILE);

    /* Since ABI 2.3 the tiles follow the file: the descriptor gives their
       count and no address, and INSTALL copies them from the file. */
    reset(); tiles_after_file(2);
    assert(registrations() == 0);
    assert(m.tile_count == 2 && m.tiles == file + file_size - 2 * GRM_TILE_BYTES);
    assert(grm_tiles(&m, code, CODE_BASE, &tiles, &tile_count) == 0 && tile_count == 2 && tiles == m.tiles);
    assert(tiles[0] == 0xa5 && tiles[2 * GRM_TILE_BYTES - 1] == 0xa5);
    assert(grm_tiles(&m, m.code, 0, &tiles, &tile_count) == 0 && tiles == m.tiles);
    /* The descriptor count must match the tiles of the file. */
    reset(); tiles_after_file(2); code_words[DESCRIPTOR + 9] = 1;
    assert(link(&failed) == 0 && grm_tiles(&m, code, CODE_BASE, &tiles, &tile_count) == GRM_BAD_DESCRIPTOR);
    reset(); tiles_after_file(2); file_tiles = 0;
    assert(link(&failed) == 0 && grm_tiles(&m, code, CODE_BASE, &tiles, &tile_count) == GRM_BAD_DESCRIPTOR);
    /* Tiles in the code and after the file. */
    reset(); file_tiles = 2;
    assert(link(&failed) == 0 && grm_tiles(&m, code, CODE_BASE, &tiles, &tile_count) == GRM_BAD_DESCRIPTOR);
    /* Tiles after the file of an older ABI, too many tiles, or a short file. */
    reset(); tiles_after_file(2); abi = GRM_ABI_MAJOR << 16 | 2; build();
    assert(grm_parse(file, file_size, &m) == GRM_BAD_FILE);
    reset(); tiles_after_file(2); build(); grm_put(file + 60, GRM_TILES + 1);
    assert(grm_parse(file, file_size, &m) == GRM_BAD_FILE);
    reset(); tiles_after_file(2); build();
    assert(grm_parse(file, file_size - 1, &m) == GRM_BAD_FILE);
    return 0;
}
