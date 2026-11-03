#ifndef GRKIT_MODULE_LINK_H
#define GRKIT_MODULE_LINK_H
/* The installer side of the module format (crates/grkit-core/src/grm.rs).
   These functions read and link a module file in the memory of the installer.
   They do not access the RTOS, so host tests can run them. The includer
   defines u8 and u32.

   A module has a code segment and a data segment. The installer chooses the
   address of each segment, links a copy of each in its own memory, then
   copies the linked bytes to those addresses. */

#define GRM_MAGIC 0x4d4b5247U
#define GRM_FORMAT 1U
#define GRM_HEADER 64U
/* The header word of the order of the module (grkit_core::grm): its default
   place in ADJ and in MENU, at most GRM_MAX_ORDER. */
#define GRM_ORDER 56U
#define GRM_MAX_ORDER 0xffffU
#define GRM_DATA_LINK 0x10000000U
#define GRM_DATA_LOCATION 0x80000000U
#define GRM_MAX_SEGMENT 0x10000U
/* The ABI of the core (runtime/sdk/grkit.h). */
#define GRM_ABI_MAJOR 2U
#define GRM_ABI_MINOR 1U
#define GRM_POINT_OVERLAY_LAYER 1U
#define GRM_POINT_MENU_PAGE 2U
#define GRM_POINT_PICTURE_PLANE 3U
#define GRM_POINT_EVENT 4U
/* The events of a subscriber (GRKIT_EVENT_*). */
#define GRM_EVENTS 3U
#define GRM_PAGE_CHOICE 1U
#define GRM_CHOICE_OFF_ROW 1U
#define GRM_CHOICE_CORE_ROWS 2U
#define GRM_CHOICE_OPTIONS 4U
#define GRM_LAYER_FOLLOW_DISP 1U
#define GRM_LAYER_FULL_DISP 2U
#define GRM_LAYER_LIVE_ONLY 4U
/* The largest layer order: the core sorts layers by order << 16. */
#define GRM_MAX_LAYER_ORDER 0xffffU
/* The menu languages, the tile size, the most tiles of one module, and the
   most rows of a choice page and UTF-16 units of one text. */
#define GRM_LANGUAGES 5U
#define GRM_TILE_BYTES 6400U
#define GRM_TILES 64U
#define GRM_PAGE_ROWS 64U
#define GRM_TEXT_UNITS 64U
/* The most rows of a native settings page (a page has 0 to this many; with
   1, Fn opens no settings page, runtime/menu.c), and the most values of each
   row. */
#define GRM_SETTING_ROWS 5U
#define GRM_SETTING_ROW_VALUES 8U
/* The stages of plane filters (runtime/sdk/grkit.h). */
#define GRM_STAGE_FRAME 20U
#define GRM_STAGE_MARK 30U
/* The most registrations that one descriptor gives. */
#define GRM_REGISTRATIONS 16U
/* The descriptor: size, ABI, registrations and their count, settings and
   their count, used settings and their count, tiles and their count
   (runtime/sdk/grkit.h). */
#define GRM_DESCRIPTOR 40U
/* The layout of each descriptor struct of runtime/sdk/grkit.h, as 32-bit ARM
   code has it (tests/runtime/layout_contract.c checks each against the
   struct). Word 0 of each is its size, which is at least GRM_<STRUCT>. */
#define GRM_DESCRIPTOR_ABI_AT 4U
#define GRM_DESCRIPTOR_REGISTRATIONS_AT 8U
#define GRM_DESCRIPTOR_REGISTRATION_COUNT_AT 12U
#define GRM_DESCRIPTOR_SETTINGS_AT 16U
#define GRM_DESCRIPTOR_SETTING_COUNT_AT 20U
#define GRM_DESCRIPTOR_USES_AT 24U
#define GRM_DESCRIPTOR_USE_COUNT_AT 28U
#define GRM_DESCRIPTOR_TILES_AT 32U
#define GRM_DESCRIPTOR_TILE_COUNT_AT 36U
/* GrkitRegistration (no size word). */
#define GRM_REGISTRATION 8U
#define GRM_REGISTRATION_IMPL_AT 4U
/* GrkitSetting. */
#define GRM_SETTING 16U
#define GRM_SETTING_KEY_AT 4U
#define GRM_SETTING_VALUES_AT 8U
#define GRM_SETTING_FLAGS_AT 12U
/* GrkitLayer. */
#define GRM_LAYER 20U
#define GRM_LAYER_FLAGS_AT 4U
#define GRM_LAYER_PAINT_AT 8U
#define GRM_LAYER_SETTING_AT 12U
#define GRM_LAYER_ORDER_AT 16U
/* GrkitPage. */
#define GRM_PAGE 40U
#define GRM_PAGE_KIND_AT 4U
#define GRM_PAGE_SETTING_AT 8U
#define GRM_PAGE_FLAGS_AT 12U
#define GRM_PAGE_TITLE_AT 16U
#define GRM_PAGE_SUMMARIES_AT 20U
#define GRM_PAGE_LABELS_AT 24U
#define GRM_PAGE_ROWS_AT 28U
#define GRM_PAGE_SETTING_ROWS_AT 32U
#define GRM_PAGE_SETTINGS_AT 36U
/* GrkitSettingRow. */
#define GRM_ROW 16U
#define GRM_ROW_SETTING_AT 4U
#define GRM_ROW_LABEL_AT 8U
#define GRM_ROW_VALUES_AT 12U
/* GrkitPlaneFilter. */
#define GRM_FILTER 16U
#define GRM_FILTER_STAGE_AT 4U
#define GRM_FILTER_SETTING_AT 8U
#define GRM_FILTER_APPLY_AT 12U
/* GrkitSubscriber. */
#define GRM_SUBSCRIBER 12U
#define GRM_SUBSCRIBER_EVENTS_AT 4U
#define GRM_SUBSCRIBER_HANDLE_AT 8U
/* The most settings and used settings of one module, the longest setting key
   with its NUL, and the most values of a setting. */
#define GRM_SETTINGS 16U
#define GRM_USES 8U
#define GRM_KEY 16U
#define GRM_SETTING_VALUES 65536U
/* A full setting name: module ID, '.', key, NUL. */
#define GRM_NAME (8U + 1U + GRM_KEY)
#define GRM_SETTING_PUBLIC 1U
#define GRM_SETTING_SAVED 2U
/* A used setting that is optional: its name starts with '?'. Its handle is
   GRM_NO_SETTING when no module has it. */
#define GRM_NO_SETTING 0xffffffffU
/* A page or a layer names setting n of the module, or GRM_USED | n for used
   setting n. */
#define GRM_USED 0x80000000U

/* Status codes of INSTALL for a module (runtime/install_status.h). */
#define GRM_BAD_FILE 50
#define GRM_BAD_ABI 51
#define GRM_MISSING_IMPORT 52
#define GRM_BAD_DESCRIPTOR 53
#define GRM_NO_ROOM 54
#define GRM_MISSING_SETTING 55

struct grm_module {
    char id[9];
    u32 version, abi;
    u32 code_size, data_size, bss_size, descriptor;
    u32 code_relocations, data_relocations, imports;
    const u8 *code, *data, *relocations, *import_entries, *names;
};

/* A symbol that an import can name: the FNV-1a hash of its name and its
   value, which is an address of the core or a value of the installer. */
struct grm_symbol { u32 hash, value; };

static u32 grm_word(const u8 *p) { return p[0] | p[1] << 8 | p[2] << 16 | (u32)p[3] << 24; }
static void grm_put(u8 *p, u32 v) { for (u32 i = 0; i < 4; ++i) p[i] = (u8)(v >> (i * 8)); }
/* Whether `bytes` bytes at `offset` lie in `size` bytes. A file can give any
   offset, so the check must not wrap. */
static int grm_inside(u32 offset, u32 bytes, u32 size) { return bytes <= size && offset <= size - bytes; }

/* Read the header of a module file and check its layout. Return 0, or
   GRM_BAD_FILE or GRM_BAD_ABI. */
static int grm_parse(const u8 *file, u32 size, struct grm_module *m)
{
    if (size < GRM_HEADER || grm_word(file) != GRM_MAGIC || grm_word(file + 4) != GRM_FORMAT) return GRM_BAD_FILE;
    m->abi = grm_word(file + 8);
    for (u32 i = 0; i < 8; ++i) {
        u8 c = file[12 + i];
        int valid = (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') || c == '_';
        if (!c) { for (u32 j = i; j < 8; ++j) if (file[12 + j]) return GRM_BAD_FILE; m->id[i] = 0; break; }
        if (!valid) return GRM_BAD_FILE;
        m->id[i] = (char)c;
    }
    m->id[8] = 0;
    if (!m->id[0]) return GRM_BAD_FILE;
    m->version = grm_word(file + 20);
    m->code_size = grm_word(file + 24);
    m->data_size = grm_word(file + 28);
    m->bss_size = grm_word(file + 32);
    m->descriptor = grm_word(file + 36);
    m->code_relocations = grm_word(file + 40);
    m->data_relocations = grm_word(file + 44);
    m->imports = grm_word(file + 48);
    u32 names = grm_word(file + 52);
    if (m->code_size > GRM_MAX_SEGMENT || m->data_size > GRM_MAX_SEGMENT || m->bss_size > GRM_MAX_SEGMENT ||
        (m->code_size | m->data_size | m->bss_size | names) & 3 ||
        m->code_relocations > GRM_MAX_SEGMENT / 4 || m->data_relocations > GRM_MAX_SEGMENT / 4 ||
        m->imports > GRM_MAX_SEGMENT / 4 || names > GRM_MAX_SEGMENT) return GRM_BAD_FILE;
    u32 total = GRM_HEADER + m->code_size + m->data_size +
                (m->code_relocations + m->data_relocations) * 4 + m->imports * 8 + names;
    if (total != size) return GRM_BAD_FILE;
    if ((m->descriptor & 3) || !grm_inside(m->descriptor, GRM_DESCRIPTOR, m->code_size)) return GRM_BAD_FILE;
    m->code = file + GRM_HEADER;
    m->data = m->code + m->code_size;
    m->relocations = m->data + m->data_size;
    m->import_entries = m->relocations + (m->code_relocations + m->data_relocations) * 4;
    m->names = m->import_entries + m->imports * 8;
    if (m->abi >> 16 != GRM_ABI_MAJOR || (m->abi & 0xffff) > GRM_ABI_MINOR) return GRM_BAD_ABI;
    return 0;
}

/* The load address of the word `value`, which holds a link address of the
   module, or 1 if it lies outside the module. */
static u32 grm_relocate(const struct grm_module *m, u32 value, u32 code_base, u32 data_base)
{
    if (value >= GRM_DATA_LINK) {
        u32 offset = value - GRM_DATA_LINK;
        return offset <= m->data_size + m->bss_size ? data_base + offset : 1;
    }
    return (value & ~1U) <= m->code_size ? code_base + value : 1;
}

/* Link the module for the load addresses `code_base` and `data_base` (each a
   multiple of 4): copy its segments into `code` and `data`, move each
   relocated word, and fill each import from `symbols`. Return 0, or
   GRM_BAD_FILE, or GRM_MISSING_IMPORT with the import index in `*failed`. */
static int grm_link(const struct grm_module *m, u8 *code, u8 *data, u32 code_base, u32 data_base,
                    const struct grm_symbol *symbols, u32 symbol_count, u32 *failed)
{
    for (u32 i = 0; i < m->code_size; ++i) code[i] = m->code[i];
    for (u32 i = 0; i < m->data_size; ++i) data[i] = m->data[i];
    u32 previous = 0;
    for (u32 i = 0; i < m->code_relocations + m->data_relocations; ++i) {
        int in_code = i < m->code_relocations;
        u32 offset = grm_word(m->relocations + i * 4);
        u32 size = in_code ? m->code_size : m->data_size;
        if (i == m->code_relocations) previous = 0;
        else if (i && offset <= previous) return GRM_BAD_FILE;
        previous = offset;
        if ((offset & 3) || !grm_inside(offset, 4, size)) return GRM_BAD_FILE;
        u8 *p = (in_code ? code : data) + offset;
        u32 value = grm_relocate(m, grm_word(p), code_base, data_base);
        if (value == 1) return GRM_BAD_FILE;
        grm_put(p, value);
    }
    for (u32 i = 0; i < m->imports; ++i) {
        u32 hash = grm_word(m->import_entries + i * 8), location = grm_word(m->import_entries + i * 8 + 4);
        int in_data = (location & GRM_DATA_LOCATION) != 0;
        u32 offset = location & ~GRM_DATA_LOCATION;
        if ((offset & 3) || !grm_inside(offset, 4, in_data ? m->data_size : m->code_size)) return GRM_BAD_FILE;
        u8 *p = (in_data ? data : code) + offset;
        if (grm_word(p)) return GRM_BAD_FILE;
        u32 j = 0;
        while (j < symbol_count && symbols[j].hash != hash) ++j;
        if (j == symbol_count) { *failed = i; return GRM_MISSING_IMPORT; }
        grm_put(p, symbols[j].value);
    }
    return 0;
}

/* The local copy of `size` bytes at load address `address` in the linked
   code, or 0 if they are not all in the code. */
static const u8 *grm_code_at(const struct grm_module *m, const u8 *code, u32 code_base, u32 address, u32 size)
{
    if (address < code_base || address - code_base > m->code_size || size > m->code_size - (address - code_base)) return 0;
    return code + (address - code_base);
}

/* Copy the NUL-terminated string at `address` of the code into `out` (at
   most size - 1 characters, each A-Z, 0-9, or _, after an optional '?' when
   `optional` allows it). Return its length, or 0 for an invalid string. */
static u32 grm_string(const struct grm_module *m, const u8 *code, u32 base, u32 address,
                      char *out, u32 size, int optional)
{
    for (u32 n = 0; n < size; ++n) {
        const u8 *p = grm_code_at(m, code, base, address + n, 1);
        if (!p) return 0;
        u8 c = *p;
        out[n] = (char)c;
        if (!c) return n;
        int valid = (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') || c == '_' ||
                    (c == '.' && optional == 2) || (c == '?' && n == 0 && optional);
        if (!valid) return 0;
    }
    return 0;
}

/* A setting of a module: its key, its number of values, and its flags. */
struct grm_setting { char key[GRM_KEY]; u32 values, flags; };

/* Read the settings and the used settings of the descriptor of a module whose
   code was linked for `base` (0 for the file itself). A used setting is a
   full name, MODULE.KEY, with '?' first when it is optional. Return 0 or
   GRM_BAD_DESCRIPTOR. */
static int grm_settings(const struct grm_module *m, const u8 *code, u32 base,
                        struct grm_setting *settings, u32 *setting_count,
                        char uses[][GRM_NAME + 1], u32 *use_count)
{
    const u8 *d = code + m->descriptor;
    u32 n = grm_word(d + GRM_DESCRIPTOR_SETTING_COUNT_AT), u = grm_word(d + GRM_DESCRIPTOR_USE_COUNT_AT);
    if (grm_word(d) < GRM_DESCRIPTOR || n > GRM_SETTINGS || u > GRM_USES) return GRM_BAD_DESCRIPTOR;
    const u8 *list = n ? grm_code_at(m, code, base, grm_word(d + GRM_DESCRIPTOR_SETTINGS_AT), n * GRM_SETTING) : d;
    const u8 *names = u ? grm_code_at(m, code, base, grm_word(d + GRM_DESCRIPTOR_USES_AT), u * 4) : d;
    if (!list || !names) return GRM_BAD_DESCRIPTOR;
    for (u32 i = 0; i < n; ++i) {
        const u8 *e = list + i * GRM_SETTING;
        struct grm_setting *s = settings + i;
        s->values = grm_word(e + GRM_SETTING_VALUES_AT);
        s->flags = grm_word(e + GRM_SETTING_FLAGS_AT);
        if (grm_word(e) < GRM_SETTING || !s->values || s->values > GRM_SETTING_VALUES ||
            (s->flags & ~(GRM_SETTING_PUBLIC | GRM_SETTING_SAVED)) ||
            !grm_string(m, code, base, grm_word(e + GRM_SETTING_KEY_AT), s->key, GRM_KEY, 0)) return GRM_BAD_DESCRIPTOR;
        for (u32 j = 0; j < i; ++j) {
            u32 k = 0;
            while (settings[j].key[k] && settings[j].key[k] == s->key[k]) ++k;
            if (settings[j].key[k] == s->key[k]) return GRM_BAD_DESCRIPTOR;
        }
    }
    for (u32 i = 0; i < u; ++i) {
        u32 length = grm_string(m, code, base, grm_word(names + i * 4), uses[i], GRM_NAME + 1, 2);
        if (!length) return GRM_BAD_DESCRIPTOR;
    }
    *setting_count = n;
    *use_count = u;
    return 0;
}

/* A layer and a page of a module. Their setting is an index into the
   settings of the module, GRM_USED | n for used setting n, or
   GRM_NO_SETTING for a layer without one. A page has one row for each
   value of its setting. */
struct grm_layer { u32 paint, setting, flags, order; };
struct grm_page { u32 address, setting, flags, rows; };
/* A plane filter: its apply function, its setting, and its stage. */
struct grm_filter { u32 apply, setting, stage; };
/* An event subscriber: its handler and its events. */
struct grm_subscriber { u32 handle, events; };

/* The name of import `index` into `out` (`size` bytes, NUL included), for a
   report. A name that does not fit is cut. */
static void grm_import_name(const struct grm_module *m, u32 index, char *out, u32 size)
{
    u32 total = grm_word(m->code - GRM_HEADER + 52), at = 0, n = 0;
    for (u32 seen = 0; at < total && seen < index; ++at) if (!m->names[at]) ++seen;
    while (at + n < total && m->names[at + n] && n + 1 < size) { out[n] = (char)m->names[at + n]; ++n; }
    if (size) out[n] = 0;
}

/* Whether a page or a layer names a setting that the module has. */
static int grm_setting_reference(u32 setting, u32 setting_count, u32 use_count)
{
    return setting & GRM_USED ? (setting & ~GRM_USED) < use_count : setting < setting_count;
}

/* Whether `address` holds a NUL-terminated UTF-16 text of at most
   GRM_TEXT_UNITS units in the code. */
static int grm_text(const struct grm_module *m, const u8 *code, u32 base, u32 address)
{
    if (address & 1) return 0;
    for (u32 n = 0; n <= GRM_TEXT_UNITS; ++n) {
        const u8 *p = grm_code_at(m, code, base, address + n * 2, 2);
        if (!p) return 0;
        if (!p[0] && !p[1]) return 1;
    }
    return 0;
}
/* Whether the `count` text pointers at `address` each point to a text. */
static int grm_texts(const struct grm_module *m, const u8 *code, u32 base, u32 address, u32 count)
{
    const u8 *list = grm_code_at(m, code, base, address, count * 4);
    if (!list) return 0;
    for (u32 i = 0; i < count; ++i)
        if (!grm_text(m, code, base, grm_word(list + i * 4))) return 0;
    return 1;
}

/* The tiles of the module: their number in `*count`, and the local address of
   the first one, or 0 without tiles. Return 0 or GRM_BAD_DESCRIPTOR. */
static int grm_tiles(const struct grm_module *m, const u8 *code, u32 base, const u8 **tiles, u32 *count)
{
    const u8 *d = code + m->descriptor;
    u32 n = grm_word(d + GRM_DESCRIPTOR_TILE_COUNT_AT);
    *count = n;
    *tiles = 0;
    if (!n) return 0;
    if (n > GRM_TILES) return GRM_BAD_DESCRIPTOR;
    *tiles = grm_code_at(m, code, base, grm_word(d + GRM_DESCRIPTOR_TILES_AT), n * GRM_TILE_BYTES);
    return *tiles ? 0 : GRM_BAD_DESCRIPTOR;
}

/* Read the registrations of the module, whose code was linked for `base`.
   `settings` holds its `setting_count` settings. Each layer and page goes
   after the earlier ones, at most `max` of each in all. Return 0,
   GRM_BAD_ABI, GRM_BAD_DESCRIPTOR, or GRM_NO_ROOM. */
static int grm_registrations(const struct grm_module *m, const u8 *code, u32 base,
                             const struct grm_setting *settings, u32 setting_count, u32 use_count,
                             struct grm_layer *layers, u32 *layer_count,
                             struct grm_page *pages, u32 *page_count,
                             struct grm_filter *filters, u32 *filter_count, u32 max)
{
    const u8 *d = code + m->descriptor, *tiles;
    u32 abi = grm_word(d + GRM_DESCRIPTOR_ABI_AT), n = grm_word(d + GRM_DESCRIPTOR_REGISTRATION_COUNT_AT), tile_count;
    if (grm_word(d) < GRM_DESCRIPTOR) return GRM_BAD_DESCRIPTOR;
    if (abi >> 16 != GRM_ABI_MAJOR || (abi & 0xffff) > GRM_ABI_MINOR) return GRM_BAD_ABI;
    if (n > GRM_REGISTRATIONS || grm_tiles(m, code, base, &tiles, &tile_count)) return GRM_BAD_DESCRIPTOR;
    const u8 *list = n ? grm_code_at(m, code, base, grm_word(d + GRM_DESCRIPTOR_REGISTRATIONS_AT), n * GRM_REGISTRATION) : d;
    if (!list) return GRM_BAD_DESCRIPTOR;
    for (u32 i = 0; i < n; ++i) {
        u32 point = grm_word(list + i * GRM_REGISTRATION), impl = grm_word(list + i * GRM_REGISTRATION + GRM_REGISTRATION_IMPL_AT);
        if (point == GRM_POINT_OVERLAY_LAYER) {
            const u8 *layer = grm_code_at(m, code, base, impl, GRM_LAYER);
            if (!layer || grm_word(layer) < GRM_LAYER) return GRM_BAD_DESCRIPTOR;
            u32 flags = grm_word(layer + GRM_LAYER_FLAGS_AT), paint = grm_word(layer + GRM_LAYER_PAINT_AT);
            u32 setting = grm_word(layer + GRM_LAYER_SETTING_AT);
            /* A Thumb function, or an ARM one at a word address. */
            if ((flags & ~(GRM_LAYER_FOLLOW_DISP | GRM_LAYER_FULL_DISP | GRM_LAYER_LIVE_ONLY)) ||
                (flags & (GRM_LAYER_FOLLOW_DISP | GRM_LAYER_FULL_DISP)) == (GRM_LAYER_FOLLOW_DISP | GRM_LAYER_FULL_DISP) ||
                (setting != GRM_NO_SETTING && !grm_setting_reference(setting, setting_count, use_count)) ||
                grm_word(layer + GRM_LAYER_ORDER_AT) > GRM_MAX_LAYER_ORDER ||
                !grm_code_at(m, code, base, paint & ~1U, 2) || (!(paint & 1) && (paint & 3)))
                return GRM_BAD_DESCRIPTOR;
            if (*layer_count >= max) return GRM_NO_ROOM;
            struct grm_layer *l = layers + (*layer_count)++;
            l->paint = paint; l->setting = setting; l->flags = flags; l->order = grm_word(layer + GRM_LAYER_ORDER_AT);
        } else if (point == GRM_POINT_MENU_PAGE) {
            const u8 *page = grm_code_at(m, code, base, impl, GRM_PAGE);
            if (!page || grm_word(page) < GRM_PAGE || grm_word(page + GRM_PAGE_KIND_AT) != GRM_PAGE_CHOICE) return GRM_BAD_DESCRIPTOR;
            u32 setting = grm_word(page + GRM_PAGE_SETTING_AT), flags = grm_word(page + GRM_PAGE_FLAGS_AT);
            u32 rows = grm_word(page + GRM_PAGE_ROWS_AT);
            if (!grm_setting_reference(setting, setting_count, use_count) ||
                (flags & ~(GRM_CHOICE_OFF_ROW | GRM_CHOICE_CORE_ROWS | GRM_CHOICE_OPTIONS))) return GRM_BAD_DESCRIPTOR;
            /* The core gives the rows of a page of a used capability: the
               page has only its title. The installer checks the setting. */
            if (flags & GRM_CHOICE_CORE_ROWS) {
                if (!(setting & GRM_USED) || rows || grm_word(page + GRM_PAGE_SETTING_ROWS_AT) ||
                    !grm_texts(m, code, base, grm_word(page + GRM_PAGE_TITLE_AT), GRM_LANGUAGES)) return GRM_BAD_DESCRIPTOR;
                if (*page_count >= max) return GRM_NO_ROOM;
                struct grm_page *p = pages + (*page_count)++;
                p->address = impl; p->setting = setting; p->flags = flags; p->rows = 0;
                continue;
            }
            /* One row and one tile for each value. The installer checks the
               values of a used setting. */
            if (!(setting & GRM_USED) && rows != settings[setting].values) return GRM_BAD_DESCRIPTOR;
            if (!rows || rows > GRM_PAGE_ROWS || rows > tile_count ||
                !grm_texts(m, code, base, grm_word(page + GRM_PAGE_TITLE_AT), GRM_LANGUAGES) ||
                !grm_texts(m, code, base, grm_word(page + GRM_PAGE_SUMMARIES_AT), rows * GRM_LANGUAGES) ||
                !grm_texts(m, code, base, grm_word(page + GRM_PAGE_LABELS_AT), rows * GRM_LANGUAGES)) return GRM_BAD_DESCRIPTOR;
            /* The rows of a native settings page: settings of the module, the
               first the page setting, each with its label and value texts. */
            u32 setting_rows = grm_word(page + GRM_PAGE_SETTING_ROWS_AT);
            if (setting_rows) {
                const u8 *list = grm_code_at(m, code, base, grm_word(page + GRM_PAGE_SETTINGS_AT), setting_rows * GRM_ROW);
                if (setting_rows > GRM_SETTING_ROWS || !list || rows > GRM_SETTING_ROW_VALUES)
                    return GRM_BAD_DESCRIPTOR;
                /* Row 0 is the page setting, also a used one, with a text for
                   each row of the page. The others are settings of the
                   module or used settings. The installer checks the value
                   texts of a used row, whose values it knows. */
                for (u32 r = 0; r < setting_rows; ++r) {
                    const u8 *row = list + r * GRM_ROW;
                    u32 named = grm_word(row + GRM_ROW_SETTING_AT), used = r && (named & GRM_USED);
                    u32 values = !r ? rows : used ? 1 : named < setting_count ? settings[named].values : 0;
                    if (grm_word(row) < GRM_ROW ||
                        (r ? !grm_setting_reference(named, setting_count, use_count) : named != setting) ||
                        !values || values > GRM_SETTING_ROW_VALUES ||
                        !grm_texts(m, code, base, grm_word(row + GRM_ROW_LABEL_AT), GRM_LANGUAGES) ||
                        !grm_texts(m, code, base, grm_word(row + GRM_ROW_VALUES_AT), values * GRM_LANGUAGES))
                        return GRM_BAD_DESCRIPTOR;
                }
            }
            if (*page_count >= max) return GRM_NO_ROOM;
            struct grm_page *p = pages + (*page_count)++;
            p->address = impl; p->setting = setting; p->flags = flags; p->rows = rows;
        } else if (point == GRM_POINT_PICTURE_PLANE) {
            const u8 *filter = grm_code_at(m, code, base, impl, GRM_FILTER);
            if (!filter || grm_word(filter) < GRM_FILTER) return GRM_BAD_DESCRIPTOR;
            u32 stage = grm_word(filter + GRM_FILTER_STAGE_AT), setting = grm_word(filter + GRM_FILTER_SETTING_AT);
            u32 apply = grm_word(filter + GRM_FILTER_APPLY_AT);
            if ((stage != GRM_STAGE_FRAME && stage != GRM_STAGE_MARK) ||
                !grm_setting_reference(setting, setting_count, use_count) ||
                !grm_code_at(m, code, base, apply & ~1U, 2) || (!(apply & 1) && (apply & 3)))
                return GRM_BAD_DESCRIPTOR;
            if (*filter_count >= max) return GRM_NO_ROOM;
            struct grm_filter *f = filters + (*filter_count)++;
            f->apply = apply; f->setting = setting; f->stage = stage;
        } else if (point != GRM_POINT_EVENT) return GRM_BAD_DESCRIPTOR;
    }
    return 0;
}

/* Read the event subscriber of the module into `out`: at most one. Return
   1 with a subscriber, 0 without one, or -1 for a descriptor that is not
   valid. */
static int grm_subscriber(const struct grm_module *m, const u8 *code, u32 base, struct grm_subscriber *out)
{
    const u8 *d = code + m->descriptor;
    u32 n = grm_word(d + GRM_DESCRIPTOR_REGISTRATION_COUNT_AT), found = 0;
    const u8 *list = n ? grm_code_at(m, code, base, grm_word(d + GRM_DESCRIPTOR_REGISTRATIONS_AT), n * GRM_REGISTRATION) : d;
    if (n > GRM_REGISTRATIONS || !list) return -1;
    for (u32 i = 0; i < n; ++i) {
        if (grm_word(list + i * GRM_REGISTRATION) != GRM_POINT_EVENT) continue;
        const u8 *sub = grm_code_at(m, code, base, grm_word(list + i * GRM_REGISTRATION + GRM_REGISTRATION_IMPL_AT), GRM_SUBSCRIBER);
        if (found || !sub || grm_word(sub) < GRM_SUBSCRIBER) return -1;
        u32 events = grm_word(sub + GRM_SUBSCRIBER_EVENTS_AT), handle = grm_word(sub + GRM_SUBSCRIBER_HANDLE_AT);
        if (!events || (events & ~GRM_EVENTS) ||
            !grm_code_at(m, code, base, handle & ~1U, 2) || (!(handle & 1) && (handle & 3))) return -1;
        out->handle = handle; out->events = events;
        found = 1;
    }
    return (int)found;
}

/* The state file ID of the setting `name`, MODULE.KEY: the CRC-32 of the
   name, modulo 10^9 (crates/grkit-core/src/grm.rs, state_id). An ID in the
   range of the Fn adjustment entries (990000000 and up) moves below it, and
   0 becomes 1. */
static u32 grm_state_id(const char *name)
{
    u32 crc = 0xffffffffU;
    for (; *name; ++name) {
        crc ^= (u8)*name;
        for (u32 bit = 0; bit < 8; ++bit) crc = crc >> 1 ^ (0xedb88320U & (0U - (crc & 1)));
    }
    crc = ~crc;
    /* No division helper: subtract multiples of 10^9. */
    while (crc >= 1000000000U) crc -= 1000000000U;
    if (crc >= 990000000U) crc -= 10000000U;
    if (!crc) crc = 1;
    return crc;
}
#endif
