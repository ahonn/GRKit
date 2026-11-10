/* -std=c11 hides POSIX declarations on glibc, and the test needs
   pthread_attr_setstack. */
#define _POSIX_C_SOURCE 200809L
#include <assert.h>
#include <pthread.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* The module plan and link of the installer (runtime/install_plan.h and
   runtime/install_modules.h) on the host. The harness gives the parts what
   installer.c and the generated module_payload.h give them on the camera: a
   card in memory behind syscall6, camera memory behind at(), the values of a
   target, and stubs for the cache sync and the icons. */
#include "abi.h"
typedef unsigned char u8;
typedef unsigned short u16;
typedef unsigned int u32;

/* The values of module_payload.h that the parts use
   (crates/grkit-build/src/loader.rs). The target has no Image Control, so
   discover lists no effect. */
#define GRKIT_NO_IMAGE_CONTROL 1
#define MAX_COUNT 40U
#define ICONS_PER_CHUNK 40U
#define ICON_STRIDE 0x1908U
#define HOST_ICON_COUNT 4U
#define CARD "/card"
#define INBOX_PATH CARD "/SCRIPT/GRKIT/EFFECTS"
#define PACKAGE_PATH CARD "/SCRIPT/GRKIT/PACKAGE.TXT"
#define PACKAGE_MODULES_PATH CARD "/SCRIPT/GRKIT/MODULES"
#define DATA_PATH CARD "/GRKIT"
#define IMPORTED_PATH CARD "/GRKIT/IMPORTED.TXT"
#define LIBRARY_PATH CARD "/GRKIT/EFFECTS"
#define STATE_PATH CARD "/GRKIT/STATE.TXT"
#define MODULES_PATH CARD "/GRKIT/MODULES"
#define LEGACY_EFFECTS_PATH CARD "/SCRIPT/PLUGINS/EFFECTS"
#define CORE_STATE_IDS 861965461, 484706023, 790270076, 977751003, 826089556, 458876999, 697053196
#define PARAMETER_KEY_BASE 990000000U
#define STATE_TEXT_MAX 16384U
#define POINT_VIEWFINDER_OVERLAY 0x4U
#define POINT_MENU_PAGE 0x1U
#define POINT_PICTURE_ENCODE 0x10U
static const char *const core_setting_names[] = {
    "CORE.TONE", "CORE.ASPECT", "CORE.GRAIN", "CORE.GRAIN_SIZE", "CORE.ASPECT_MODE", "CORE.ADJ",
    "CORE.ADJ_MODULE"};
static const u32 core_setting_points[] = {0x12, 0xc, 0x20, 0x40, 0xc, 0x0, 0x0};
/* The tail of the core, and its one export, overlay_label. */
#define TAIL 0x54000000U
#define TAIL_SYMBOL 0U
static u32 relocated[] = {TAIL};
#define CORE_EXPORT_COUNT 1U
#define OVERLAY_LABEL (TAIL + 0x40U)
static const u32 core_exports[] = {0xc0e27352U, 0x40U};

#include "grf.h"
#include "module_link.h"
#include "setting_table.h"
#include "effects.h"
#include "services.h"
#include "install_status.h"
/* The records of a folder listing, which the card writes. */
#include "directory.h"

/* The parts pass each address to syscall6 as a u32, which keeps only the
   low half of a host address. They run on a thread whose stack is in the
   static data, so each such address is in the image of the harness, and
   host() takes the high half that puts it nearest to that stack. */
#define TEST_STACK 0x200000U
static u8 test_stack[TEST_STACK] __attribute__((aligned(0x4000)));
static void *host(u32 address)
{
    uintptr_t near = (uintptr_t)test_stack, at = (near & ~(uintptr_t)0xffffffffU) | address;
    if (at > near + 0x80000000U) at -= (uintptr_t)1 << 32;
    else if (at + 0x80000000U < near) at += (uintptr_t)1 << 32;
    return (void *)at;
}
/* AddressSanitizer can move the locals of the parts to a fake stack that it
   maps far from the image (Linux), where host() cannot find them again. Keep
   the locals on the stack of the thread. */
#if defined(__has_feature)
#if __has_feature(address_sanitizer)
#define TEST_ADDRESS_SANITIZER 1
#endif
#endif
#if defined(__SANITIZE_ADDRESS__) || defined(TEST_ADDRESS_SANITIZER)
const char *__asan_default_options(void);
const char *__asan_default_options(void) { return "detect_stack_use_after_return=0"; }
#endif

/* The card: each file with its path and bytes. A folder exists while it
   holds a file, and it lists its files in the order that they were added,
   after "." and "..". */
#define CARD_FILES 32U
#define PATH_SIZE 96U
struct card_file { char path[PATH_SIZE]; u8 *bytes; u32 size; };
static struct card_file card[CARD_FILES];
static u32 card_count;
/* An open file or folder: the file index or the folder path, and the read
   position (the entry index of a folder). */
struct open_file { int used, folder; u32 file; char path[PATH_SIZE]; long position; };
#define OPEN_FILES 8U
#define FIRST_FD 3U
static struct open_file open_files[OPEN_FILES];
#define ENOENT_ERROR (-2L)
#define EBADF_ERROR (-9L)

static void add_file(const char *path, const void *bytes, u32 size)
{
    assert(card_count < CARD_FILES && strlen(path) < PATH_SIZE);
    struct card_file *f = &card[card_count++];
    strcpy(f->path, path);
    f->bytes = malloc(size ? size : 1);
    memcpy(f->bytes, bytes, size);
    f->size = size;
}
static void add_text(const char *path, const char *text) { add_file(path, text, (u32)strlen(text)); }
/* The name of `path` inside `folder`, or 0 when it is not a file of it. */
static const char *inside(const char *path, const char *folder)
{
    size_t n = strlen(folder);
    if (strncmp(path, folder, n) || path[n] != '/' || strchr(path + n + 1, '/')) return 0;
    return path + n + 1;
}
static int folder_exists(const char *folder)
{
    for (u32 i = 0; i < card_count; ++i)
        if (inside(card[i].path, folder)) return 1;
    return 0;
}
static long open_path(const char *path, u32 flags)
{
    u32 fd = 0;
    while (fd < OPEN_FILES && open_files[fd].used) ++fd;
    assert(fd < OPEN_FILES);
    struct open_file *o = &open_files[fd];
    memset(o, 0, sizeof *o);
    if (flags == 0xc000) {
        if (!folder_exists(path)) return ENOENT_ERROR;
        o->folder = 1;
        strcpy(o->path, path);
    } else {
        assert(flags == 0x8800);
        while (o->file < card_count && strcmp(card[o->file].path, path)) ++o->file;
        if (o->file == card_count) return ENOENT_ERROR;
    }
    o->used = 1;
    return (long)(fd + FIRST_FD);
}
static struct open_file *opened(u32 fd)
{
    if (fd < FIRST_FD || fd - FIRST_FD >= OPEN_FILES || !open_files[fd - FIRST_FD].used) return 0;
    return &open_files[fd - FIRST_FD];
}
/* getdents64: the linux_dirent64 records (runtime/directory.h) that fit. */
static long list_entries(struct open_file *o, u8 *buffer, u32 size)
{
    u32 used = 0;
    for (;;) {
        const char *name = o->position == 0 ? "." : o->position == 1 ? ".." : 0;
        u8 type = 4;
        if (!name) {
            long k = 1;
            for (u32 i = 0; i < card_count && !name; ++i)
                if (inside(card[i].path, o->path) && ++k == o->position) name = inside(card[i].path, o->path);
            if (!name) return (long)used;
            type = 8;
        }
        u32 n = (u32)strlen(name), record = (DIRECTORY_NAME + n + 1 + 7) & ~7U;
        if (used + record > size) return (long)used;
        u8 *entry = buffer + used;
        memset(entry, 0, record);
        entry[16] = (u8)record; entry[17] = (u8)(record >> 8); entry[18] = type;
        memcpy(entry + DIRECTORY_NAME, name, n);
        used += record;
        ++o->position;
    }
}
static long syscall6(u32 number, u32 a, u32 b, u32 c, u32 d, u32 e, u32 f)
{
    (void)d; (void)e; (void)f;
    struct open_file *o = opened(a);
    switch (number) {
    case 3: {
        if (!o || o->folder) return EBADF_ERROR;
        const struct card_file *file = &card[o->file];
        u32 n = file->size - (u32)o->position < c ? file->size - (u32)o->position : c;
        memcpy(host(b), file->bytes + o->position, n);
        o->position += n;
        return (long)n;
    }
    /* The parts print their progress to standard output. */
    case 4: assert(a == 1); return (long)c;
    case 5: return open_path(host(a), b);
    case 6: if (!o) return EBADF_ERROR; o->used = 0; return 0;
    case 19:
        if (!o || !o->folder || c > 1) return EBADF_ERROR;
        o->position = (c ? o->position : 0) + (long)(int)b;
        return o->position;
    case 217: return o && o->folder ? list_entries(o, host(b), c) : EBADF_ERROR;
    default: assert(!"a system call that the parts do not make"); return -1;
    }
}

/* Camera memory: the code of the modules, the module array, and the icon
   chunks. */
#define MEMORY_BASE 0x56000000U
#define MEMORY_SIZE 0x100000U
#define CODE_AT MEMORY_BASE
#define ARRAY_AT (MEMORY_BASE + 0x40000U)
#define ICONS_AT (MEMORY_BASE + 0x80000U)
#define PIXELS 0x52000000U
static u8 memory[MEMORY_SIZE] __attribute__((aligned(4)));
static volatile u8 *at(u32 physical)
{
    assert(physical >= MEMORY_BASE && physical - MEMORY_BASE < MEMORY_SIZE);
    return memory + (physical - MEMORY_BASE);
}
static u32 word(u32 p) { return *(volatile u32 *)at(p); }
static void store(u32 p, u32 v) { *(volatile u32 *)at(p) = v; }
#include "install_io.h"

/* The rest of what installer.c gives the parts, in its order. */
#define GRKIT_GRF_EMBEDDED
#include "grf.c"
#include "catalog.h"
#include "state.h"
#include "manage.h"
static const char library_path[] = LIBRARY_PATH, inbox_path[] = INBOX_PATH, package_path[] = PACKAGE_PATH;
static const char imported_path[] = IMPORTED_PATH, data_path[] = DATA_PATH;
static const char legacy_effects_path[] = LEGACY_EFFECTS_PATH, state_path[] = STATE_PATH;
static u8 profiles[MAX_COUNT + 1][GRKIT_GRF_MAX_SIZE], temporary[GRKIT_GRF_MAX_SIZE];
static u8 directory_entries[4096];
static u32 count;
#define LISTED_HEAD 64U
static u8 listed[EFFECTS_LISTED + 1][LISTED_HEAD];
static char names[EFFECTS_LISTED + 1][CATALOG_NAME_SIZE];
static u32 listed_count, listed_status[EFFECTS_LISTED], listed_choice[EFFECTS_LISTED];
static u32 listed_record[EFFECTS_LISTED];
#define EFFECT_PATH_SIZE (sizeof(legacy_effects_path) + CATALOG_NAME_SIZE)
#include "install_library.h"
#include "install_effects.h"
static u32 module_tiles;
#include "install_plan.h"
static u32 needed_points;
static int sync_alias(u32 address, u32 n)
{
    if (n) (void)at(address + n - 1);
    return 1;
}
#define ICON_PIXELS 8
static void icon_header(u32 icon, u32 pixels) { store(icon, pixels); }
/* installer.c reads the model record of the camera. */
static int test_monochrome;
static int monochrome_camera(void) { return test_monochrome; }
#include "install_modules.h"
/* installer.c also reads STATE.TMP and the state of GRKit 1.2. */
static void read_state(void)
{
    static u8 text[STATE_TEXT_MAX];
    state_parse(&saved_state, text, read_text(state_path, text, sizeof(text)));
}

/* A module file built by hand (crates/grkit-core/src/grm.rs; see
   tests/runtime/module_link_contract.c). Each layer and filter has its own
   Thumb function. */
#define SPEC_ITEMS 4U
struct spec {
    const char *id;
    u32 order, data_bytes, bss_bytes;
    /* A file with another magic, or with another major ABI. */
    int bad_magic, new_abi, legacy_abi;
    struct { const char *key; u32 values, flags, default_value; } settings[SPEC_ITEMS];
    const char *uses[SPEC_ITEMS], *imports[SPEC_ITEMS];
    u32 layer_count, filter_count, layer_conditions;
    struct { u32 order, flags, setting; } layers[SPEC_ITEMS];
    struct { u32 stage, setting; } filters[SPEC_ITEMS];
    /* A choice page of own setting 0 with these native settings rows: row 0
       is setting 0, the others an own setting n or GRM_USED | used n. */
    u32 page_rows, rows[SPEC_ITEMS], settings_page, group_page, bad_group;
    u32 indicator_count, indicator_slot, condition_setting, condition_value;
    u32 icon_kind, icon_setting, icon_first, indicator_tiles, page_icon;
    /* The flags of a 44-byte descriptor (ABI 2.4), or 0 for a 40-byte one. */
    u32 module_flags;
    /* The Fn value and setting of a 56-byte page (ABI 2.5), or 0. */
    u32 fn_value, fn_setting;
};
static u32 code_words[GRM_MAX_SEGMENT / 4], code_count, relocations[256], relocation_count;
static u32 emit(u32 value)
{
    assert(code_count < GRM_MAX_SEGMENT / 4);
    code_words[code_count] = value;
    return code_count++;
}
/* A word that holds the link address of code word `target`, plus `extra`. */
static u32 emit_pointer(u32 target, u32 extra)
{
    relocations[relocation_count++] = code_count * 4;
    return emit(target * 4 + extra);
}
/* A list address and its count, or 0 for an empty list. */
static void emit_list(u32 first, u32 n)
{
    if (n) emit_pointer(first, 0);
    else emit(0);
    emit(n);
}
static u32 emit_text(const char *text)
{
    u32 first = code_count, n = (u32)strlen(text) + 1;
    for (u32 i = 0; i < (n + 3) / 4; ++i) emit(0);
    memcpy(&code_words[first], text, n);
    return first;
}
static u32 items(const char *const *list)
{
    u32 n = 0;
    while (n < SPEC_ITEMS && list[n]) ++n;
    return n;
}
/* Add the module file `folder`/`file`. Return its code size. */
static u32 add_module(const char *folder, const char *file, const struct spec *s)
{
    static u8 bytes[MODULE_FILE_MAX];
    u32 settings = 0, uses = items(s->uses), imports = items(s->imports);
    u32 paint[SPEC_ITEMS], apply[SPEC_ITEMS], import_at[SPEC_ITEMS], keys[SPEC_ITEMS], use_names[SPEC_ITEMS];
    u32 layer_at[SPEC_ITEMS], filter_at[SPEC_ITEMS], indicator_at[SPEC_ITEMS];
    u32 abi = (GRM_ABI_MAJOR + (s->new_abi ? 1U : 0U)) << 16 | (s->legacy_abi ? 1U : GRM_ABI_MINOR);
    while (settings < SPEC_ITEMS && s->settings[settings].key) ++settings;
    memset(code_words, 0, sizeof code_words);
    code_count = relocation_count = 0;
    for (u32 k = 0; k < s->layer_count; ++k) paint[k] = emit(0x4770bf00);
    for (u32 k = 0; k < s->filter_count; ++k) apply[k] = emit(0x4770bf00);
    for (u32 k = 0; k < imports; ++k) import_at[k] = emit(0);
    for (u32 k = 0; k < settings; ++k) keys[k] = emit_text(s->settings[k].key);
    for (u32 k = 0; k < uses; ++k) use_names[k] = emit_text(s->uses[k]);
    u32 setting_list = code_count;
    for (u32 k = 0; k < settings; ++k) {
        emit(GRM_SETTING_DEFAULT); emit_pointer(keys[k], 0);
        emit(s->settings[k].values); emit(s->settings[k].flags); emit(s->settings[k].default_value);
    }
    u32 use_list = code_count;
    for (u32 k = 0; k < uses; ++k) emit_pointer(use_names[k], 0);
    u32 layer_conditions = code_count;
    if (s->layer_conditions) { emit(s->condition_setting); emit(GRM_CONDITION_EQUALS); emit(s->condition_value); }
    for (u32 k = 0; k < s->layer_count; ++k) {
        layer_at[k] = emit(s->layer_conditions ? GRM_LAYER_CONDITIONAL : GRM_LAYER);
        emit(s->layers[k].flags); emit_pointer(paint[k], 1);
        emit(s->layers[k].setting); emit(s->layers[k].order);
        if (s->layer_conditions) { emit(1); emit_pointer(layer_conditions, 0); }
    }
    for (u32 k = 0; k < s->filter_count; ++k) {
        filter_at[k] = emit(GRM_FILTER); emit(s->filters[k].stage); emit(s->filters[k].setting);
        emit_pointer(apply[k], 1);
    }
    u32 icon_at = 0;
    if (s->indicator_count || s->page_icon) {
        icon_at = emit(GRM_ICON); emit(s->icon_kind ? s->icon_kind : GRM_ICON_TILE);
        emit(s->icon_setting); emit(s->icon_first);
        u32 conditions = emit(s->condition_setting); emit(GRM_CONDITION_EQUALS); emit(s->condition_value);
        for (u32 k = 0; k < s->indicator_count; ++k) {
            indicator_at[k] = emit(GRM_INDICATOR);
            emit(s->indicator_slot ? GRM_INDICATOR_NATIVE_SLOT : GRM_INDICATOR_STATUS_ROW);
            emit(s->indicator_slot); emit(1); emit_pointer(conditions, 0); emit_pointer(icon_at, 0);
        }
    }
    /* The page: one text for each language and value, 32 at most, and a
       blank tile for each value of setting 0. */
    u32 page_at = 0, child_at[2] = {0, 0}, tiles_at = 0;
    u32 tile_count = s->group_page ? 3 : s->page_rows ? (s->settings_page ? 1 : s->settings[0].values) : 0;
    if (s->indicator_tiles) tile_count = s->indicator_tiles;
    if (s->page_rows) {
        static const u16 hi[] = {'H', 'i', 0};
        u32 text = emit(0), texts, row_list;
        emit(0);
        memcpy(&code_words[text], hi, sizeof hi);
        texts = code_count;
        for (u32 k = 0; k < 32; ++k) emit_pointer(text, 0);
        row_list = code_count;
        for (u32 r = 0; r < s->page_rows; ++r) {
            emit(GRM_ROW); emit(r ? s->rows[r] : 0); emit_pointer(texts, 0); emit_pointer(texts, 0);
        }
        page_at = emit(GRM_PAGE); emit(s->settings_page ? GRM_PAGE_SETTINGS : GRM_PAGE_CHOICE); emit(0); emit(0);
        emit_pointer(texts, 0); emit_pointer(texts, 0); emit_pointer(texts, 0);
        emit(s->settings[0].values); emit(s->page_rows); emit_pointer(row_list, 0);
        if (s->page_icon || s->fn_value) {
            code_words[page_at] = GRM_PAGE_ICON;
            emit(0);
            if (s->page_icon) emit_pointer(icon_at, 0);
            else emit(0);
        }
        if (s->fn_value) {
            code_words[page_at] = GRM_PAGE_FN;
            emit(s->fn_value); emit(s->fn_setting);
        }
        if (s->group_page) {
            child_at[0] = page_at;
            code_words[page_at + GRM_PAGE_KIND_AT / 4] = GRM_PAGE_SETTINGS;
            code_words[page_at + GRM_PAGE_FLAGS_AT / 4] = GRM_PAGE_MENU_ONLY;
            row_list = code_count;
            for (u32 r = 0; r < 2; ++r) {
                emit(GRM_ROW); emit(r + 2); emit_pointer(texts, 0); emit_pointer(texts, 0);
            }
            child_at[1] = emit(GRM_PAGE); emit(GRM_PAGE_SETTINGS); emit(2); emit(GRM_PAGE_MENU_ONLY);
            emit_pointer(texts, 0); emit(0); emit(0); emit(s->settings[2].values); emit(2); emit_pointer(row_list, 0);
            u32 children = code_count;
            emit_pointer(child_at[0], 0); emit_pointer(child_at[1], 0);
            page_at = emit(GRM_GROUP_PAGE); emit(GRM_PAGE_GROUP); emit(GRM_NO_SETTING); emit(0);
            emit_pointer(texts, 0); emit(0); emit(0); emit(2); emit(0); emit(0); emit_pointer(children, 0);
            if (s->bad_group == 1) code_words[child_at[1] + GRM_PAGE_FLAGS_AT / 4] = 0;
            if (s->bad_group == 2) code_words[children + 1] = page_at * 4;
            if (s->bad_group == 3) code_words[page_at] = GRM_PAGE;
            if (s->bad_group == 4) code_words[page_at + GRM_PAGE_ROWS_AT / 4] = GRM_PAGE_CHILDREN + 1;
        }
    }
    tiles_at = code_count;
    for (u32 k = 0; k < tile_count * GRM_TILE_BYTES / 4; ++k) emit(0);
    u32 registrations = code_count;
    for (u32 k = 0; k < s->layer_count; ++k) { emit(GRM_POINT_OVERLAY_LAYER); emit_pointer(layer_at[k], 0); }
    for (u32 k = 0; k < s->filter_count; ++k) { emit(GRM_POINT_PICTURE_PLANE); emit_pointer(filter_at[k], 0); }
    if (s->page_rows) { emit(GRM_POINT_MENU_PAGE); emit_pointer(page_at, 0); }
    if (s->group_page) for (u32 k = 0; k < 2; ++k) { emit(GRM_POINT_MENU_PAGE); emit_pointer(child_at[k], 0); }
    for (u32 k = 0; k < s->indicator_count; ++k) { emit(GRM_POINT_VIEWFINDER_INDICATOR); emit_pointer(indicator_at[k], 0); }
    u32 descriptor = emit(s->module_flags ? GRM_DESCRIPTOR_FLAGS : GRM_DESCRIPTOR);
    emit(abi);
    emit_list(registrations, s->layer_count + s->filter_count + s->indicator_count + (s->page_rows ? 1 : 0) + (s->group_page ? 2 : 0));
    emit_list(setting_list, settings);
    emit_list(use_list, uses);
    emit_list(tiles_at, tile_count);
    if (s->module_flags) emit(s->module_flags);

    u32 names = 0;
    for (u32 k = 0; k < imports; ++k) names += (u32)strlen(s->imports[k]) + 1;
    names = (names + 3) & ~3U;
    u8 *p = bytes;
    memset(bytes, 0, sizeof bytes);
    grm_put(p, s->bad_magic ? 0x12345678U : GRM_MAGIC); grm_put(p + 4, GRM_FORMAT); grm_put(p + 8, abi);
    memcpy(p + 12, s->id, strlen(s->id));
    grm_put(p + 20, 1); grm_put(p + 24, code_count * 4); grm_put(p + 28, s->data_bytes);
    grm_put(p + 32, s->bss_bytes); grm_put(p + 36, descriptor * 4);
    grm_put(p + 40, relocation_count); grm_put(p + 44, 0);
    grm_put(p + 48, imports); grm_put(p + 52, names); grm_put(p + GRM_ORDER, s->order);
    p += GRM_HEADER;
    memcpy(p, code_words, code_count * 4); p += code_count * 4;
    p += s->data_bytes;
    for (u32 k = 0; k < relocation_count; ++k, p += 4) grm_put(p, relocations[k]);
    for (u32 k = 0; k < imports; ++k, p += 8) {
        grm_put(p, fnv((const u8 *)s->imports[k], (u32)strlen(s->imports[k])));
        grm_put(p + 4, import_at[k] * 4);
    }
    for (u32 k = 0; k < imports; ++k) {
        u32 n = (u32)strlen(s->imports[k]) + 1;
        memcpy(p, s->imports[k], n); p += n;
    }
    p = bytes + ((u32)(p - bytes) + 3) / 4 * 4;
    char path[PATH_SIZE];
    snprintf(path, sizeof path, "%s/%s", folder, file);
    add_file(path, bytes, (u32)(p - bytes));
    return code_count * 4;
}
/* The code word of import `k` of a module that the spec builds. */
static u32 import_word(const struct spec *s, u32 k) { return s->layer_count + s->filter_count + k; }

/* STATE.TXT with `n` pairs of a key and a value. */
static void save_state(const u32 *pairs, u32 n)
{
    char text[1024];
    u32 used = 0;
    for (u32 i = 0; i < n; ++i)
        used += (u32)snprintf(text + used, sizeof text - used, "%u\r\n%u\r\n", pairs[2 * i], pairs[2 * i + 1]);
    add_text(STATE_PATH, text);
}
static u32 id_of(const char *name) { return grm_state_id(name); }

static void reset(void)
{
    for (u32 i = 0; i < card_count; ++i) free(card[i].bytes);
    card_count = 0;
    memset(open_files, 0, sizeof open_files);
    memset(memory, 0, sizeof memory);
    safe_mode = 0;
    listed_count = count = needed_points = 0;
}
/* SCAN: the modules, the effects that discover lists, then the array. */
static void scan(void)
{
    plan_modules();
    plan_array();
}
/* INSTALL after SCAN. */
static int install_with_core(u32 tail, u32 core_bytes, u32 executable_end)
{
    static const u32 icon_bases[1] = {ICONS_AT};
    u32 code;
    scan();
    int status = plan_code(tail, core_bytes, executable_end, &code);
    return status ? status : install_modules(code, ARRAY_AT, PIXELS, icon_bases);
}
static int install(void) { return install_with_core(CODE_AT, 0, ARRAY_AT); }
static void expect_modules(const char *const *files, const u32 *sources, u32 n)
{
    assert(module_count == n);
    for (u32 i = 0; i < n; ++i) assert(!strcmp(module_names[i], files[i]) && module_source[i] == sources[i]);
}
/* Setting `handle` of the settings table: its ID, value, and values. */
static void expect_setting(u32 handle, u32 id, u32 value, u32 values)
{
    u32 entry = ARRAY_AT + 4 * SETTINGS_ENTRY(handle);
    assert(word(entry) == id && word(entry + 4) == value && word(entry + 8) == values);
}
/* Entry i of the module list: its status, code range, and UTF-16 name. */
static u32 list_entry(u32 i) { return module_list + MANAGE_HEADER + i * MANAGE_STRIDE; }
static void expect_listed(u32 i, u32 status, const char *name)
{
    u32 entry = list_entry(i), n = (u32)strlen(name);
    assert(word(entry + MANAGE_STATUS) == status);
    for (u32 k = 0; k < MANAGE_NAME_UNITS; ++k)
        assert(((volatile u16 *)at(entry + MANAGE_NAME))[k] == (k < n ? (u16)name[k] : 0));
}

static void modules_come_from_both_folders_in_header_order(void)
{
    reset();
    add_module(PACKAGE_MODULES_PATH, "C.GRM", &(struct spec){.id = "C", .order = 10});
    add_module(PACKAGE_MODULES_PATH, "A.GRM", &(struct spec){.id = "A", .order = 20});
    add_text(PACKAGE_MODULES_PATH "/NOTES.TXT", "not a module");
    add_module(PACKAGE_MODULES_PATH, "B.GRM", &(struct spec){.id = "B", .order = 10});
    /* A of GRKIT/MODULES replaces the package A, with its own order. */
    add_module(MODULES_PATH, "A.GRM", &(struct spec){.id = "A", .order = 5});
    add_module(MODULES_PATH, "D.GRM", &(struct spec){.id = "D", .order = 10});
    scan();
    static const char *const all[] = {"A.GRM", "B.GRM", "C.GRM", "D.GRM"};
    static const u32 all_sources[] = {1, 0, 0, 1};
    expect_modules(all, all_sources, 4);
    assert(module_usable == 0xf && module_overflow == 0 && module_order[0] == 5);
    assert(module_words > 0 && module_words <= GRKIT_MODULE_ARRAY_WORDS);

    /* With a package list, a package module loads only if the list names it. */
    add_text(PACKAGE_PATH, "B.GRM\r\nA.GRM\r\n");
    scan();
    static const char *const listed_files[] = {"A.GRM", "B.GRM", "D.GRM"};
    static const u32 listed_sources[] = {1, 0, 1};
    expect_modules(listed_files, listed_sources, 3);
}
static void more_modules_than_the_list_holds_are_counted(void)
{
    reset();
    /* The folder lists the files in the reverse of their order, so the list
       keeps the first files of the order, not of the folder. */
    for (u32 i = GRKIT_MODULES_MAX + 2; i-- > 0;) {
        char id[8], file[16];
        snprintf(id, sizeof id, "M%u", i);
        snprintf(file, sizeof file, "M%u.GRM", i);
        add_module(PACKAGE_MODULES_PATH, file, &(struct spec){.id = id, .order = i});
    }
    scan();
    assert(module_count == GRKIT_MODULES_MAX && module_overflow == 2);
    for (u32 i = 0; i < GRKIT_MODULES_MAX; ++i) {
        char file[16];
        snprintf(file, sizeof file, "M%u.GRM", i);
        assert(!strcmp(module_names[i], file) && module_status[i] == 0);
    }
}
/* A full list keeps the package modules: a module of GRKIT/MODULES takes
   only the place of another one of GRKIT/MODULES that comes after it. */
static void a_full_list_keeps_the_package_modules(void)
{
    reset();
    for (u32 i = 0; i < GRKIT_MODULES_MAX - 1; ++i) {
        char id[8], file[16];
        snprintf(id, sizeof id, "P%u", i);
        snprintf(file, sizeof file, "P%u.GRM", i);
        add_module(PACKAGE_MODULES_PATH, file, &(struct spec){.id = id, .order = 50 + i});
    }
    add_module(MODULES_PATH, "LATE.GRM", &(struct spec){.id = "LATE", .order = 90});
    add_module(MODULES_PATH, "EARLY.GRM", &(struct spec){.id = "EARLY", .order = 0});
    add_module(MODULES_PATH, "NEXT.GRM", &(struct spec){.id = "NEXT", .order = 95});
    scan();
    assert(module_count == GRKIT_MODULES_MAX && module_overflow == 2);
    /* EARLY takes the place of LATE, and NEXT gets none. */
    assert(!strcmp(module_names[0], "EARLY.GRM") && module_source[0] == 1);
    for (u32 i = 1; i < GRKIT_MODULES_MAX; ++i) assert(module_source[i] == 0);
    /* With a full list of package modules, EARLY gets no place. */
    add_module(PACKAGE_MODULES_PATH, "PLAST.GRM", &(struct spec){.id = "PLAST", .order = 99});
    scan();
    assert(module_count == GRKIT_MODULES_MAX && module_overflow == 3);
    for (u32 i = 0; i < GRKIT_MODULES_MAX; ++i) assert(module_source[i] == 0);
    assert(!strcmp(module_names[GRKIT_MODULES_MAX - 1], "PLAST.GRM"));
}
static void safe_mode_loads_no_module(void)
{
    reset();
    add_module(PACKAGE_MODULES_PATH, "A.GRM", &(struct spec){.id = "A"});
    add_module(MODULES_PATH, "B.GRM", &(struct spec){.id = "B"});
    safe_mode = 1;
    assert(install() == 0);
    assert(module_count == 2 && module_usable == 0);
    assert(module_status[0] == MANAGE_SAFE && module_status[1] == MANAGE_SAFE);
    assert(word(module_list + MANAGE_FLAGS) == MANAGE_SAFE_MODE);
    expect_listed(0, MANAGE_SAFE, "A");
    expect_listed(1, MANAGE_SAFE, "B");
}
static void a_bad_module_keeps_the_others(void)
{
    reset();
    add_module(PACKAGE_MODULES_PATH, "GOOD.GRM", &(struct spec){.id = "GOOD", .order = 1});
    /* A file that is not a module sorts last. */
    add_module(PACKAGE_MODULES_PATH, "BADMAGIC.GRM", &(struct spec){.id = "BADMAGIC", .bad_magic = 1});
    add_module(PACKAGE_MODULES_PATH, "NEWABI.GRM", &(struct spec){.id = "NEWABI", .order = 2, .new_abi = 1});
    /* The file name is the module ID. */
    add_module(PACKAGE_MODULES_PATH, "OTHER.GRM", &(struct spec){.id = "NAME", .order = 3});
    add_module(PACKAGE_MODULES_PATH, "BADDESC.GRM",
               &(struct spec){.id = "BADDESC", .order = 4, .settings = {{"MODE", 0, 0}}});
    add_module(MODULES_PATH, "LAST.GRM", &(struct spec){.id = "LAST", .order = 5});
    assert(install() == 0);
    static const char *const files[] = {"GOOD.GRM", "NEWABI.GRM", "OTHER.GRM", "BADDESC.GRM", "LAST.GRM", "BADMAGIC.GRM"};
    static const u32 sources[] = {0, 0, 0, 0, 1, 0};
    static const u32 statuses[] = {0, GRM_BAD_ABI, GRM_BAD_FILE, GRM_BAD_DESCRIPTOR, 0, GRM_BAD_FILE};
    expect_modules(files, sources, 6);
    for (u32 i = 0; i < 6; ++i) assert(module_status[i] == statuses[i]);
    assert(module_usable == (1U << 0 | 1U << 4));
    assert(module_code_from[0] == CODE_AT && module_code_from[4] == module_code_to[0]);
    expect_listed(1, GRM_BAD_ABI, "NEWABI (51)");
    expect_listed(5, GRM_BAD_FILE, "BADMAGIC (50)");
}
/* A larger core moves both module code and its linked entry points. The
   last module may end exactly at the executable boundary. */
static void module_code_follows_the_actual_core(void)
{
    for (u32 core = 0x10000U; core <= 0x18000U; core += 0x8000U) {
        reset();
        u32 first = add_module(PACKAGE_MODULES_PATH, "FIRST.GRM",
            &(struct spec){.id = "FIRST", .order = 1, .layer_count = 1, .layers = {{0, 0, GRM_NO_SETTING}}});
        u32 second = add_module(PACKAGE_MODULES_PATH, "SECOND.GRM",
            &(struct spec){.id = "SECOND", .order = 2, .layer_count = 1, .layers = {{0, 0, GRM_NO_SETTING}}});
        u32 start = CODE_AT + core;
        assert(install_with_core(CODE_AT, core, start + first + second) == 0);
        assert(module_code_from[0] == start && module_code_to[0] == start + first);
        assert(module_code_from[1] == start + first && module_code_to[1] == start + first + second);
        assert(module_layers[0] == start + 1 && module_layers[4] == start + first + 1);
    }
}
/* A core or module overflow must fail before code, data, icons, or settings
   are written. Include inputs whose sums would wrap a 32-bit address. */
static void code_overflow_leaves_camera_memory_unchanged(void)
{
    reset();
    u32 bytes = add_module(PACKAGE_MODULES_PATH, "ONLY.GRM", &(struct spec){.id = "ONLY"});
    const u32 attempts[][3] = {
        {CODE_AT, 0x10000U, CODE_AT + 0x10000U + bytes - 1},
        {CODE_AT, 0x10000U, CODE_AT + 0xffffU},
        {0x55000004U, 0, 0x55000000U},
        {0x54ff0000U, 0xffffffffU, 0x55000000U},
        {0xfffffff0U, 0x20U, 0xffffffffU},
    };
    memset(memory, 0xa5, sizeof memory);
    for (u32 i = 0; i < sizeof attempts / sizeof attempts[0]; ++i) {
        assert(install_with_core(attempts[i][0], attempts[i][1], attempts[i][2]) == GRM_NO_ROOM);
        for (u32 j = 0; j < sizeof memory; ++j) assert(memory[j] == 0xa5);
    }
    /* The RX end itself is a valid start only when no module code remains. */
    reset();
    scan();
    u32 start = 0;
    assert(plan_code(0x54ff0000U, 0x10000U, 0x55000000U, &start) == 0);
    assert(start == 0x55000000U);
}
/* SCAN reserves the module array for the most effects. BIG fills half of
   it, and FILL the rest, so that one word more of FILL leaves no room for
   the services block. */
static void the_budget_counts_the_services_block(void)
{
    const u32 big = 2 * GRM_MAX_SEGMENT / 4;
    const u32 others = SETTINGS_WORDS(CHOICE_ENTRIES(2, EFFECTS_LISTED)) + 2 * CONTEXT_WORDS(0) +
                       MANAGE_WORDS(2) + EFFECT_LIST_WORDS(EFFECTS_LISTED) + SERVICES_WORDS + big;
    const u32 room = GRKIT_MODULE_ARRAY_WORDS - others;
    assert(room * 4 > GRM_MAX_SEGMENT && room * 4 < 2 * GRM_MAX_SEGMENT);
    for (u32 extra = 0; extra < 2; ++extra) {
        reset();
        add_module(PACKAGE_MODULES_PATH, "BIG.GRM",
                   &(struct spec){.id = "BIG", .order = 1, .data_bytes = GRM_MAX_SEGMENT, .bss_bytes = GRM_MAX_SEGMENT});
        add_module(PACKAGE_MODULES_PATH, "FILL.GRM",
                   &(struct spec){.id = "FILL", .order = 2, .data_bytes = (room + extra) * 4 - GRM_MAX_SEGMENT,
                                  .bss_bytes = GRM_MAX_SEGMENT});
        plan_modules();
        assert(module_status[0] == 0 && module_status[1] == (extra ? GRM_NO_ROOM : 0));
        assert(module_usable == (extra ? 1U : 3U));
        /* discover lists the most effects. */
        listed_count = EFFECTS_LISTED;
        plan_array();
        assert(module_words <= GRKIT_MODULE_ARRAY_WORDS);
        assert(extra || module_words == GRKIT_MODULE_ARRAY_WORDS);
    }
}
static const struct spec guides = {
    .id = "GUIDES", .order = 10, .data_bytes = 8, .bss_bytes = 4,
    .settings = {{"STYLE", 6, GRM_SETTING_SAVED | GRM_SETTING_PUBLIC}, {"COLOR", 3, GRM_SETTING_SAVED}, {"TEMP", 2, 0}},
    .uses = {"CORE.ASPECT", "?CORE.MISSING"},
    .imports = {"grkit_self", "overlay_label"},
};
static const struct spec date = {
    .id = "DATE", .order = 20,
    .settings = {{"LOOK", 3, GRM_SETTING_SAVED}},
    .uses = {"GUIDES.STYLE"},
};
static void install_writes_the_settings_and_the_contexts(void)
{
    reset();
    u32 guides_code = add_module(PACKAGE_MODULES_PATH, "GUIDES.GRM", &guides);
    add_module(PACKAGE_MODULES_PATH, "DATE.GRM", &date);
    const u32 pairs[] = {
        id_of("GUIDES.STYLE"), 4, id_of("CORE.OFF.DATE"), 1, id_of("CORE.POS.GUIDES"), 2,
        id_of("CORE.ROWS.DATE"), 5, 12345, 7, 23456, 0, id_of("CORE.ASPECT"), 3, id_of("CORE.GRAIN"), 1,
    };
    static const u32 core_ids[] = {CORE_STATE_IDS};
    for (u32 k = 0; k < GRKIT_CORE_SETTING_COUNT; ++k) assert(core_ids[k] == id_of(core_setting_names[k]));
    save_state(pairs, sizeof pairs / sizeof pairs[0] / 2);
    assert(install() == 0);
    /* The settings of the modules, the choices of the GRKit section, the
       switch, then the saved values that no setting takes: CORE.GRAIN,
       which no module uses, and unknown keys, including a value 0. */
    assert(word(ARRAY_AT) == 14 && word(ARRAY_AT + 4) == 0);
    expect_setting(0, id_of("GUIDES.STYLE"), 0, 6);
    expect_setting(1, id_of("GUIDES.COLOR"), 0, 3);
    expect_setting(2, 0, 0, 2);
    expect_setting(3, id_of("DATE.LOOK"), 0, 3);
    expect_setting(4, id_of("CORE.OFF.GUIDES"), 0, 2);
    expect_setting(5, id_of("CORE.POS.GUIDES"), 2, MANAGE_MODULES + 1);
    expect_setting(6, id_of("CORE.ROWS.GUIDES"), 0, 1U << MANAGE_OPTION_VALUES);
    expect_setting(7, id_of("CORE.OFF.DATE"), 1, 2);
    expect_setting(8, id_of("CORE.POS.DATE"), 0, MANAGE_MODULES + 1);
    expect_setting(9, id_of("CORE.ROWS.DATE"), 5, 1U << MANAGE_OPTION_VALUES);
    expect_setting(10, id_of("CORE.OFF"), 0, 2);
    expect_setting(11, 12345, 7, 8);
    expect_setting(12, 23456, 0, 1);
    expect_setting(13, id_of("CORE.GRAIN"), 1, 2);
    /* The contexts: the first setting, the count, reserved, then the handle
       of each used setting. */
    u32 context = ARRAY_AT + 4 * SETTINGS_WORDS(14);
    assert(word(context) == 0 && word(context + 4) == 3 && word(context + 8) == 0);
    assert(word(context + 12) == GRKIT_CORE_ASPECT && word(context + 16) == GRM_NO_SETTING);
    u32 date_context = context + 4 * CONTEXT_WORDS(2);
    assert(word(date_context) == 3 && word(date_context + 4) == 1 && word(date_context + 12) == 0);
    /* The imports of GUIDES: its context and the export of the core. */
    assert(word(CODE_AT + 4 * import_word(&guides, 0)) == context);
    assert(word(CODE_AT + 4 * import_word(&guides, 1)) == OVERLAY_LABEL);
    assert(module_code_from[1] == CODE_AT + guides_code);
    /* The capability of a module that is Off counts as used, not as running. */
    assert(capabilities_used == 1U << (GRKIT_CORE_ASPECT - GRKIT_CORE_SETTING_BASE));
    assert(capabilities_running == capabilities_used);
    assert((needed_points & core_setting_points[1]) == core_setting_points[1]);
    assert(word(module_list + MANAGE_SETTINGS) == 4);
    assert(word(module_list + MANAGE_SWITCH) == 10 + 1);
}
/* Defaults initialize both saved and transient settings. Explicit saved zero
   survives a restart and a period without the module installed. */
static void setting_defaults_and_saved_zero(void)
{
    const struct spec module = {
        .id = "DEFAULT", .settings = {{"LEVEL", 3, GRM_SETTING_SAVED, 1}, {"TEMP", 3, 0, 2}},
    };
    const u32 pairs[] = {id_of("DEFAULT.LEVEL"), 0};
    reset();
    add_module(PACKAGE_MODULES_PATH, "DEFAULT.GRM", &module);
    assert(install() == 0);
    expect_setting(0, pairs[0], 1, 3);
    expect_setting(1, 0, 2, 3);
    save_state(pairs, 1);
    assert(install() == 0);
    u32 entry = ARRAY_AT + 4 * SETTINGS_ENTRY(0);
    assert(state_setting_value(&saved_state, word(entry), word(entry + 4), word(entry + 8)) == 0);

    reset();
    add_module(PACKAGE_MODULES_PATH, "GUIDES.GRM", &guides);
    save_state(pairs, 1);
    assert(install() == 0);
    assert(kept_count == 1 && kept_ids[0] == pairs[0] && kept_values[0] == 0);
}
/* SCAN and INSTALL keep the framing mode available for the state restore,
   including when Framing loads stopped. This harness does not run the
   apply_state function that writes the restored values into the header. */
static void framing_mode_state_survives_install_planning(void)
{
    const struct spec framing = {.id = "FRAMING", .uses = {"CORE.ASPECT", "CORE.ASPECT_MODE"}};
    const u32 capabilities = GRKIT_CORE_ASPECT_BIT | GRKIT_CORE_ASPECT_MODE_BIT;
    for (u32 scenario = 0; scenario < 3; ++scenario) {
        reset();
        add_module(PACKAGE_MODULES_PATH, "FRAMING.GRM", &framing);
        const u32 pairs[] = {id_of("CORE.ASPECT"), 1, id_of("CORE.ASPECT_MODE"), 1,
                             id_of("CORE.OFF.FRAMING"), 1};
        /* An existing state has the selected XPan ratio but no mode key.
           A newer state saves guide-only, optionally with Framing off. */
        save_state(pairs, scenario + 1);
        assert(install() == 0 && module_status[0] == 0);
        assert(capabilities_used == capabilities);
        assert(capabilities_running == (scenario == 2 ? 0 : capabilities));
        u32 context = ARRAY_AT + 4 * SETTINGS_WORDS(word(ARRAY_AT));
        assert(word(context + 12) == GRKIT_CORE_ASPECT);
        assert(word(context + 16) == GRKIT_CORE_ASPECT_MODE);
        u32 value = 99;
        assert(state_value(&saved_state, pairs[0], &value) && value == 1);
        assert(state_value(&saved_state, pairs[2], &value) == (scenario != 0));
        if (scenario) assert(value == 1);
        assert(state_setting_value(&saved_state, pairs[2], 0, GRKIT_CORE_ASPECT_MODE_VALUES) == (scenario != 0));
        /* A stopped module still owns its core state. Its saved mode is
           not reclassified as an unknown setting or removed. */
        assert(state_taken(pairs[2]) && kept_count == 0);
        assert(module_off[0] == (scenario == 2));
        expect_setting(word(module_list + MANAGE_SETTINGS), pairs[4], scenario == 2, 2);
    }
}
/* A settings-only entry links its independent rows with only a module tile. */
static void install_settings_page(void)
{
    const struct spec module = {
        .id = "SETTINGS", .settings = {{"FIRST", 4, GRM_SETTING_SAVED}, {"SECOND", 4, GRM_SETTING_SAVED}},
        .page_rows = 2, .rows = {0, 1}, .settings_page = 1,
    };
    reset();
    add_module(PACKAGE_MODULES_PATH, "SETTINGS.GRM", &module);
    assert(install() == 0 && module_page_count == 1 && module_tiles == 1);
    u32 page = module_pages[0], rows = word(page + GRM_PAGE_SETTINGS_AT);
    assert(word(page + GRM_PAGE_KIND_AT) == GRM_PAGE_SETTINGS && word(page + GRM_PAGE_SETTING_ROWS_AT) == 2);
    assert(word(rows + GRM_ROW_SETTING_AT) == 0 && word(rows + GRM_ROW + GRM_ROW_SETTING_AT) == 1);
}
/* On a monochrome camera a row of a color-only setting is left out, and the
   rows after it move up. A color camera keeps every row. */
static void install_color_only_rows(void)
{
    const struct spec module = {
        .id = "COLOR", .settings = {{"FIRST", 4, GRM_SETTING_SAVED}, {"TINT", 4, GRM_SETTING_SAVED | GRM_SETTING_COLOR_ONLY},
                                    {"LAST", 2, GRM_SETTING_SAVED}},
        .page_rows = 3, .rows = {0, 1, 2}, .settings_page = 1,
    };
    for (int monochrome = 0; monochrome < 2; ++monochrome) {
        reset();
        test_monochrome = monochrome;
        add_module(PACKAGE_MODULES_PATH, "COLOR.GRM", &module);
        assert(install() == 0 && module_status[0] == 0 && module_page_count == 1);
        u32 page = module_pages[0], rows = word(page + GRM_PAGE_SETTINGS_AT);
        assert(word(page + GRM_PAGE_SETTING_ROWS_AT) == (monochrome ? 2U : 3U));
        assert(word(rows + GRM_ROW_SETTING_AT) == 0);
        assert(word(rows + GRM_ROW + GRM_ROW_SETTING_AT) == (monochrome ? 2U : 1U));
    }
    test_monochrome = 0;
}
/* On a monochrome camera a color-only module does not load (GRM_COLOR_ONLY),
   and the other modules load. MENU does not list it (menu_contract.c). A
   color camera loads it. */
static void a_color_only_module_needs_a_color_camera(void)
{
    for (int monochrome = 0; monochrome < 2; ++monochrome) {
        reset();
        test_monochrome = monochrome;
        add_module(PACKAGE_MODULES_PATH, "FILM.GRM", &(struct spec){.id = "FILM", .order = 1, .module_flags = GRM_MODULE_COLOR_ONLY});
        add_module(PACKAGE_MODULES_PATH, "OTHER.GRM", &(struct spec){.id = "OTHER", .order = 2});
        assert(install() == 0 && module_count == 2);
        assert(module_status[0] == (monochrome ? (u32)GRM_COLOR_ONLY : 0) && module_status[1] == 0);
        expect_listed(0, monochrome ? MANAGE_COLOR_ONLY : MANAGE_LOADED, monochrome ? "FILM (56)" : "FILM");
    }
    /* A flag that this GRKit does not know. */
    reset();
    add_module(PACKAGE_MODULES_PATH, "NEW.GRM", &(struct spec){.id = "NEW", .module_flags = 2});
    assert(install() == 0 && module_status[0] == GRM_BAD_DESCRIPTOR);
    test_monochrome = 0;
}
/* A group and both children install atomically; children keep independent
   handles, and descriptors remain inside the owning module's code. */
static void install_group_page(void)
{
    struct spec group = {
        .id = "GROUP", .settings = {{"FIRST_ON", 2, GRM_SETTING_SAVED}, {"FIRST_POWER", 3, GRM_SETTING_SAVED, 1},
                                 {"SECOND_ON", 2, GRM_SETTING_SAVED}, {"SECOND_POWER", 3, GRM_SETTING_SAVED, 1}},
        .page_rows = 2, .rows = {0, 1}, .group_page = 1,
    };
    reset();
    add_module(PACKAGE_MODULES_PATH, "GROUP.GRM", &group);
    assert(install() == 0 && module_status[0] == 0 && module_page_count == 3 && module_tiles == 3);
    u32 page = module_pages[0], children = word(page + GRM_PAGE_CHILDREN_AT);
    assert(word(page + GRM_PAGE_KIND_AT) == GRM_PAGE_GROUP && module_pages[1] == GRM_NO_SETTING);
    for (u32 c = 0; c < 2; ++c) {
        u32 child = module_pages[(c + 1) * 3], rows = word(child + GRM_PAGE_SETTINGS_AT);
        assert(word(children + c * 4) == child && module_pages[(c + 1) * 3 + 1] == c * 2);
        assert(word(rows + GRM_ROW_SETTING_AT) == c * 2 && word(rows + GRM_ROW + GRM_ROW_SETTING_AT) == c * 2 + 1);
        assert(word(child + GRM_PAGE_FLAGS_AT) == GRM_PAGE_MENU_ONLY);
    }
    for (u32 bad = 1; bad <= 4; ++bad) {
        reset(); group.bad_group = bad;
        add_module(PACKAGE_MODULES_PATH, "GROUP.GRM", &group);
        assert(install() == 0 && module_status[0] == GRM_BAD_DESCRIPTOR && !module_page_count);
    }
    group.bad_group = 0;
    /* A child lists the values of its first setting: Off, Weak, Medium,
       and Strong install. A setting with one value has no Off to list. */
    reset();
    group.settings[0].values = 4;
    add_module(PACKAGE_MODULES_PATH, "GROUP.GRM", &group);
    assert(install() == 0 && module_status[0] == 0 && module_page_count == 3);
    reset();
    group.settings[0].values = 1;
    add_module(PACKAGE_MODULES_PATH, "GROUP.GRM", &group);
    assert(install() == 0 && module_status[0] == GRM_BAD_DESCRIPTOR && !module_page_count);
    group.settings[0].values = 2;
    reset();
    for (u32 i = 0; i < 4; ++i) {
        char id[] = "GROUP0", file[] = "GROUP0.GRM";
        id[5] += (char)i; file[5] += (char)i;
        group.id = id; group.order = i;
        add_module(PACKAGE_MODULES_PATH, file, &group);
    }
    assert(install() == 0 && module_page_count == 9 && module_status[3] == GRM_NO_ROOM);
}
/* The GRKit switch Off stops each module that loads, as its own Off does,
   and keeps the Enable of each. */
static void the_switch_stops_each_module(void)
{
    reset();
    u32 first = add_module(PACKAGE_MODULES_PATH, "FIRST.GRM", &(struct spec){.id = "FIRST", .order = 1, .uses = {"CORE.ASPECT"}});
    u32 second = add_module(PACKAGE_MODULES_PATH, "SECOND.GRM", &(struct spec){.id = "SECOND", .order = 2, .uses = {"CORE.GRAIN"}});
    const u32 pairs[] = {id_of("CORE.OFF"), 1, id_of("CORE.OFF.SECOND"), 1};
    save_state(pairs, 2);
    assert(install() == 0);
    assert(all_off && module_status[0] == 0 && module_status[1] == 0);
    /* The switch and the Enable of each module keep their values. */
    u32 handle = word(module_list + MANAGE_SWITCH) - 1;
    expect_setting(handle, id_of("CORE.OFF"), 1, 2);
    expect_setting(word(module_list + MANAGE_SETTINGS), id_of("CORE.OFF.FIRST"), 0, 2);
    expect_setting(word(module_list + MANAGE_SETTINGS) + MANAGE_PER_MODULE, id_of("CORE.OFF.SECOND"), 1, 2);
    /* Each module loads stopped, and no capability runs. */
    assert(word(services_block + SERVICES_SUSPENDED) == 2);
    assert(word(services_block + SERVICES_SUSPENDED + 4) == CODE_AT);
    assert(word(services_block + SERVICES_SUSPENDED + 16) == CODE_AT + first + second);
    assert(capabilities_running == 0);
    assert(capabilities_used == (1U << (GRKIT_CORE_ASPECT - GRKIT_CORE_SETTING_BASE) |
                                 1U << (GRKIT_CORE_GRAIN - GRKIT_CORE_SETTING_BASE)));
    /* A switch that is On runs the modules whose Enable is On. */
    reset();
    add_module(PACKAGE_MODULES_PATH, "FIRST.GRM", &(struct spec){.id = "FIRST", .order = 1, .uses = {"CORE.ASPECT"}});
    add_module(PACKAGE_MODULES_PATH, "SECOND.GRM", &(struct spec){.id = "SECOND", .order = 2, .uses = {"CORE.GRAIN"}});
    const u32 on[] = {id_of("CORE.OFF"), 0, id_of("CORE.OFF.SECOND"), 1};
    save_state(on, 2);
    assert(install() == 0);
    assert(!all_off && word(services_block + SERVICES_SUSPENDED) == 1);
    assert(capabilities_running == 1U << (GRKIT_CORE_ASPECT - GRKIT_CORE_SETTING_BASE));
}
/* Conditions in two descriptors can share one array. Relocation must read
   original local handles each time, not relocate an already global handle. */
static void layer_conditions_resolve_shared_and_optional_settings(void)
{
    reset();
    add_module(PACKAGE_MODULES_PATH, "FIRST.GRM", &(struct spec){
        .id="FIRST", .order=1, .settings={{"ON",2,0}}});
    add_module(PACKAGE_MODULES_PATH, "GUIDES.GRM", &(struct spec){
        .id="GUIDES", .order=2, .settings={{"STYLE",6,0},{"DISP",2,0}},
        .layer_count=2, .layer_conditions=1, .condition_setting=1, .condition_value=1,
        .layers={{20,GRM_LAYER_FOLLOW_DISP,0},{20,0,0}}});
    assert(install()==0 && module_layer_count==2 && module_status[1]==0);
    u32 first=module_layers[3], second=module_layers[7];
    assert(first && second && first!=second);
    u32 conditions=word(first+GRM_LAYER_CONDITIONS_AT);
    assert(conditions==word(second+GRM_LAYER_CONDITIONS_AT));
    assert(word(conditions)==2 && word(conditions+GRM_CONDITION_VALUE_AT)==1);
    assert(module_layers[1]==1 && module_layers[5]==1);

    reset();
    add_module(PACKAGE_MODULES_PATH, "OPTIONAL.GRM", &(struct spec){
        .id="OPTIONAL", .uses={"?MISSING.ON"}, .layer_count=1, .layer_conditions=1,
        .condition_setting=GRM_USED, .layers={{20,0,GRM_NO_SETTING}}});
    assert(install()==0 && module_status[0]==0 && module_layer_count==1);
    assert(word(word(module_layers[3]+GRM_LAYER_CONDITIONS_AT))==GRM_NO_SETTING);

    reset();
    add_module(PACKAGE_MODULES_PATH, "INVALID.GRM", &(struct spec){
        .id="INVALID", .uses={"CORE.GRAIN"}, .layer_count=1, .layer_conditions=1,
        .condition_setting=GRM_USED, .condition_value=5, .layers={{20,0,GRM_NO_SETTING}}});
    assert(install()==0 && module_status[0]==GRM_BAD_DESCRIPTOR && !module_layer_count);
}
static void layers_follow_their_order_and_filters_their_stage(void)
{
    reset();
    u32 first_code = add_module(PACKAGE_MODULES_PATH, "FIRST.GRM", &(struct spec){
        .id = "FIRST", .order = 1, .settings = {{"ON", 2, 0}},
        .layer_count = 2, .layers = {{30, GRM_LAYER_FOLLOW_DISP, 0}, {10, GRM_LAYER_LIVE_ONLY, GRM_NO_SETTING}},
        .filter_count = 1, .filters = {{GRM_STAGE_MARK, 0}}});
    add_module(PACKAGE_MODULES_PATH, "SECOND.GRM", &(struct spec){
        .id = "SECOND", .order = 2, .settings = {{"ON", 2, 0}},
        .layer_count = 2, .layers = {{10, GRM_LAYER_FULL_DISP, GRM_NO_SETTING}, {20, 0, 0}},
        .filter_count = 1, .filters = {{GRM_STAGE_FRAME, 0}}});
    assert(install() == 0);
    const u32 second = CODE_AT + first_code;
    /* Equal orders keep the load order. A layer setting is a handle. */
    static const u32 keys[] = {10 << 16 | GRM_LAYER_LIVE_ONLY, 10 << 16 | GRM_LAYER_FULL_DISP, 20 << 16,
                               30 << 16 | GRM_LAYER_FOLLOW_DISP};
    static const u32 settings[] = {GRM_NO_SETTING, GRM_NO_SETTING, 1, 0};
    const u32 paints[] = {CODE_AT + 4 + 1, second + 1, second + 4 + 1, CODE_AT + 1};
    assert(module_layer_count == 4);
    for (u32 k = 0; k < 4; ++k)
        assert(module_layers[k * 4] == paints[k] && module_layers[k * 4 + 1] == settings[k] &&
               module_layers[k * 4 + 2] == keys[k] && module_layers[k * 4 + 3] == 0);
    assert(module_filter_count == 2);
    assert(module_filters[0] == second + 8 + 1 && module_filters[1] == (GRM_STAGE_FRAME << 16 | 1));
    assert(module_filters[2] == CODE_AT + 8 + 1 && module_filters[3] == (GRM_STAGE_MARK << 16 | 0));
    assert((needed_points & (POINT_VIEWFINDER_OVERLAY | POINT_PICTURE_ENCODE)) ==
           (POINT_VIEWFINDER_OVERLAY | POINT_PICTURE_ENCODE));
}
/* The header holds GRKIT_MODULE_LAYERS_MAX layers and GRKIT_MODULE_FILTERS_MAX
   filters of all modules. A module that needs more gets error 54. */
static void the_layers_and_filters_fill_their_slots(void)
{
    reset();
    for (u32 i = 0; i < GRKIT_MODULES_MAX; ++i) {
        char id[8], file[16];
        snprintf(id, sizeof id, "M%u", i);
        snprintf(file, sizeof file, "M%u.GRM", i);
        add_module(PACKAGE_MODULES_PATH, file, &(struct spec){
            .id = id, .order = i, .settings = {{"ON", 2, 0}},
            .layer_count = 2, .layers = {{10, 0, 0}, {20, 0, 0}},
            .filter_count = 1, .filters = {{GRM_STAGE_MARK, 0}}});
    }
    assert(install() == 0);
    _Static_assert(GRKIT_MODULE_LAYERS_MAX == 16 && GRKIT_MODULE_FILTERS_MAX == 8, "Eight modules fill the slots");
    for (u32 i = 0; i < GRKIT_MODULES_MAX; ++i) assert(module_status[i] == (i < 8 ? 0 : GRM_NO_ROOM));
    assert(module_layer_count == GRKIT_MODULE_LAYERS_MAX && module_filter_count == GRKIT_MODULE_FILTERS_MAX);
}
static void the_module_list_shows_each_result(void)
{
    reset();
    u32 good = add_module(PACKAGE_MODULES_PATH, "good.grm", &(struct spec){.id = "GOOD", .order = 1, .uses = {"CORE.ASPECT"}});
    u32 missing = add_module(PACKAGE_MODULES_PATH, "MISSING.GRM",
                             &(struct spec){.id = "MISSING", .order = 2, .imports = {"no_such_function"}});
    u32 off = add_module(MODULES_PATH, "OFFMOD.GRM", &(struct spec){.id = "OFFMOD", .order = 3, .uses = {"CORE.GRAIN"}});
    add_module(MODULES_PATH, "BROKEN.GRM", &(struct spec){.id = "BROKEN", .bad_magic = 1});
    const u32 pairs[] = {id_of("CORE.OFF.OFFMOD"), 1};
    save_state(pairs, 1);
    assert(install() == 0);
    assert(word(module_list + MANAGE_COUNT) == 4 && word(module_list + MANAGE_FLAGS) == 0);
    /* The name is the ID in capitals, and an error follows it. */
    expect_listed(0, 0, "GOOD");
    expect_listed(1, GRM_MISSING_IMPORT, "MISSING (52)");
    expect_listed(2, 0, "OFFMOD");
    expect_listed(3, GRM_BAD_FILE, "BROKEN (50)");
    assert(!strcmp(module_detail[1], "no_such_function"));
    /* A module that did not link keeps its code memory. */
    const u32 off_from = CODE_AT + good + missing;
    assert(word(list_entry(0) + MANAGE_CODE) == CODE_AT && word(list_entry(0) + MANAGE_CODE_END) == CODE_AT + good);
    assert(word(list_entry(1) + MANAGE_CODE) == 0 && word(list_entry(1) + MANAGE_TILE) == MANAGE_NO_TILE);
    assert(word(list_entry(2) + MANAGE_CODE) == off_from && word(list_entry(2) + MANAGE_CODE_END) == off_from + off);
    assert(word(list_entry(2) + MANAGE_USES) == 1U << (GRKIT_CORE_GRAIN - GRKIT_CORE_SETTING_BASE));
    /* The module that is Off loads stopped: the core calls no code in its
       range, and its capability does not run. */
    assert(word(services_block + SERVICES_SUSPENDED) == 1);
    assert(word(services_block + SERVICES_SUSPENDED + 4) == off_from);
    assert(word(services_block + SERVICES_SUSPENDED + 8) == off_from + off);
    assert(capabilities_running == 1U << (GRKIT_CORE_ASPECT - GRKIT_CORE_SETTING_BASE));
    assert(capabilities_used == (capabilities_running | 1U << (GRKIT_CORE_GRAIN - GRKIT_CORE_SETTING_BASE)));
}

/* A settings row of a used setting gets the handle of that setting. A row of
   an optional used setting that no module has is left out. */
static void settings_rows_of_used_settings(void)
{
    reset();
    const struct spec grain = {.id = "GRAIN", .order = 1,
                               .settings = {{"LEVEL", 2, GRM_SETTING_SAVED}, {"TONE", 3, GRM_SETTING_SAVED}},
                               .uses = {"?OTHER.SIZE", "CORE.GRAIN"},
                               .page_rows = 4, .rows = {0, GRM_USED | 0, GRM_USED | 1, 1}};
    add_module(PACKAGE_MODULES_PATH, "GRAIN.GRM", &grain);
    assert(install() == 0 && module_status[0] == 0 && module_page_count == 1);
    u32 page = module_pages[0], list = word(page + GRM_PAGE_SETTINGS_AT);
    assert(module_pages[1] == 0 && word(page + GRM_PAGE_SETTING_ROWS_AT) == 3);
    static const u32 handles[] = {0, GRKIT_CORE_GRAIN, 1};
    for (u32 r = 0; r < 3; ++r) {
        assert(word(list + r * GRM_ROW) == GRM_ROW && word(list + r * GRM_ROW + GRM_ROW_SETTING_AT) == handles[r]);
        assert(word(list + r * GRM_ROW + GRM_ROW_LABEL_AT) == word(page + GRM_PAGE_TITLE_AT));
    }
    assert(capabilities_used == 1U << (GRKIT_CORE_GRAIN - GRKIT_CORE_SETTING_BASE));
    /* A row of a used setting that the module does not use is not valid. */
    reset();
    struct spec bad = grain;
    bad.rows[1] = GRM_USED | 2;
    add_module(PACKAGE_MODULES_PATH, "GRAIN.GRM", &bad);
    assert(install() == 0 && module_status[0] == GRM_BAD_DESCRIPTOR);
}

/* The Fn setting of a page row (ABI 2.5) gets its handle: an own setting or
   a used one. Without the setting of an optional use, the page has no Fn
   value. */
static void the_fn_setting_of_a_page_gets_its_handle(void)
{
    static const u32 named[] = {1, GRM_USED | 1, GRM_USED | 0}, handles[] = {1, GRKIT_CORE_GRAIN, 0};
    for (u32 k = 0; k < 3; ++k) {
        reset();
        const struct spec grain = {.id = "GRAIN", .order = 1,
                                   .settings = {{"LEVEL", 2, GRM_SETTING_SAVED}, {"TONE", 3, GRM_SETTING_SAVED}},
                                   .uses = {"?OTHER.SIZE", "CORE.GRAIN"},
                                   .page_rows = 1, .rows = {0}, .fn_value = 1, .fn_setting = named[k]};
        add_module(PACKAGE_MODULES_PATH, "GRAIN.GRM", &grain);
        assert(install() == 0 && module_status[0] == 0 && module_page_count == 1);
        u32 page = module_pages[0];
        assert(word(page) == GRM_PAGE_FN && word(page + GRM_PAGE_FN_VALUE_AT) == (k < 2 ? 1U : 0U));
        if (k < 2) assert(word(page + GRM_PAGE_FN_SETTING_AT) == handles[k]);
    }
    /* A value past the rows is not valid. */
    reset();
    add_module(PACKAGE_MODULES_PATH, "GRAIN.GRM", &(struct spec){
        .id = "GRAIN", .settings = {{"LEVEL", 2, 0}, {"TONE", 3, 0}}, .page_rows = 1, .rows = {0},
        .fn_value = 2, .fn_setting = 1});
    assert(install() == 0 && module_status[0] == GRM_BAD_DESCRIPTOR);
}

/* A setting of a module that does not link is missing, also for a module
   that links before it: a module that requires the setting gets error 55,
   and the layer of an optional one is not registered. */
static void a_setting_of_a_module_that_did_not_link_is_missing(void)
{
    const struct spec provider = {.id = "PROV", .settings = {{"MODE", 2, GRM_SETTING_PUBLIC}},
                                  .imports = {"no_such_function"}};
    for (u32 first = 0; first < 2; ++first) {
        reset();
        struct spec p = provider;
        p.order = first ? 1 : 3;
        add_module(PACKAGE_MODULES_PATH, "PROV.GRM", &p);
        add_module(PACKAGE_MODULES_PATH, "NEEDS.GRM", &(struct spec){
            .id = "NEEDS", .order = 2, .uses = {"PROV.MODE"},
            .layer_count = 1, .layers = {{10, 0, GRM_USED | 0}}});
        add_module(PACKAGE_MODULES_PATH, "MAYUSE.GRM", &(struct spec){
            .id = "MAYUSE", .order = 2, .uses = {"?PROV.MODE"}, .settings = {{"ON", 2, 0}},
            .layer_count = 2, .layers = {{10, 0, GRM_USED | 0}, {20, 0, 0}}});
        assert(install() == 0);
        /* The order: PROV, MAYUSE, NEEDS, or MAYUSE, NEEDS, PROV. */
        u32 prov = first ? 0 : 2, mayuse = first ? 1 : 0, needs = first ? 2 : 1;
        assert(module_status[prov] == GRM_MISSING_IMPORT);
        assert(module_status[needs] == GRM_MISSING_SETTING && !strcmp(module_detail[needs], "PROV.MODE"));
        assert(module_status[mayuse] == 0);
        /* Only the layer of the own setting of MAYUSE paints. */
        assert(module_layer_count == 1 && module_layers[1] != GRM_NO_SETTING && module_layers[2] == 20U << 16);
        u32 context = ARRAY_AT + 4 * SETTINGS_WORDS(word(ARRAY_AT));
        for (u32 i = 0; i < mayuse; ++i)
            if (module_usable >> i & 1) context += 4 * CONTEXT_WORDS(module_use_count[i]);
        assert(word(context + 12) == GRM_NO_SETTING);
        expect_listed(needs, GRM_MISSING_SETTING, "NEEDS (55)");
    }
}

static void indicators_resolve_settings_and_preserve_shared_descriptors(void)
{
    reset();
    add_module(PACKAGE_MODULES_PATH, "FIRST.GRM", &(struct spec){.id = "FIRST", .order = 1,
        .settings = {{"ON", 2, GRM_SETTING_PUBLIC}}});
    add_module(PACKAGE_MODULES_PATH, "ICONS.GRM", &(struct spec){.id = "ICONS", .order = 2,
        .settings = {{"ON", 2, 0}, {"STRENGTH", 3, 0}}, .uses = {"FIRST.ON"},
        .indicator_count = 2, .condition_setting = GRM_USED, .condition_value = 1,
        .icon_setting = 1, .indicator_tiles = 3, .page_rows = 1, .page_icon = 1});
    assert(install() == 0 && module_status[0] == 0 && module_status[1] == 0);
    assert(module_indicator_count == 2 && module_status_icons == 2 && !module_native_slots);
    u32 indicator = module_indicators[0], conditions = word(indicator + GRM_INDICATOR_CONDITIONS_AT);
    u32 icon = word(indicator + GRM_INDICATOR_ICON_AT);
    assert(word(conditions) == 0); /* FIRST.ON, not ICONS.ON. */
    assert(word(icon + GRM_ICON_SETTING_AT) == 2); /* The strength after FIRST.ON and ICONS.ON. */
    assert(word(module_indicators[2] + GRM_INDICATOR_ICON_AT) == icon);
    assert(word(module_pages[0] + GRM_PAGE_ICON_AT) == icon);
    assert((needed_points & POINT_VIEWFINDER_OVERLAY) != 0);

    /* Missing optional settings hide the icon, not the whole module. */
    reset();
    add_module(PACKAGE_MODULES_PATH, "OPTIONAL.GRM", &(struct spec){.id = "OPTIONAL",
        .uses = {"?MISSING.ON"}, .indicator_count = 1, .condition_setting = GRM_USED,
        .icon_setting = GRM_USED, .indicator_tiles = 1});
    assert(install() == 0 && module_status[0] == 0 && module_indicator_count == 1);
    indicator = module_indicators[0];
    assert(word(word(indicator + GRM_INDICATOR_CONDITIONS_AT)) == GRM_NO_SETTING);
    assert(word(word(indicator + GRM_INDICATOR_ICON_AT) + GRM_ICON_KIND_AT) == 0);
}
static void indicators_reject_conflicts_overflow_and_used_value_ranges(void)
{
    reset();
    const struct spec native = {.id = "FIRST", .order = 1, .settings = {{"ON", 2, 0}},
        .indicator_count = 1, .indicator_slot = GRM_SLOT_IMAGE_CONTROL, .indicator_tiles = 2};
    add_module(PACKAGE_MODULES_PATH, "FIRST.GRM", &native);
    struct spec duplicate = native; duplicate.id = "SECOND"; duplicate.order = 2;
    add_module(PACKAGE_MODULES_PATH, "SECOND.GRM", &duplicate);
    assert(install() == 0 && module_indicator_count == 1);
    assert(module_status[0] == 0 && module_status[1] == GRM_BAD_DESCRIPTOR);
    assert(module_native_slots == 1U << GRM_SLOT_IMAGE_CONTROL);

    reset();
    struct spec row = {.id = "FIRST", .order = 1, .settings = {{"ON", 2, 0}},
        .indicator_count = 4, .indicator_tiles = 2};
    add_module(PACKAGE_MODULES_PATH, "FIRST.GRM", &row);
    row.id = "SECOND"; row.order = 2; row.indicator_count = 3;
    add_module(PACKAGE_MODULES_PATH, "SECOND.GRM", &row);
    assert(install() == 0 && module_indicator_count == 4 && module_status[1] == GRM_NO_ROOM);

    for (u32 bad = 0; bad < 3; ++bad) {
        reset();
        add_module(PACKAGE_MODULES_PATH, "INVALID.GRM", &(struct spec){.id = "INVALID",
            .uses = {"CORE.GRAIN"}, .indicator_count = 1, .condition_setting = GRM_USED,
            .condition_value = bad == 0 ? 5 : 0, .icon_setting = GRM_USED,
            .icon_kind = bad == 2 ? GRM_ICON_TONE : GRM_ICON_TILE,
            .indicator_tiles = bad == 1 ? 4 : 5});
        assert(install() == 0 && !module_indicator_count && module_status[0] == GRM_BAD_DESCRIPTOR);
    }
    /* A label of CORE.ASPECT needs a core label for each of its 8 values:
       labels 0 to 7 exist, and the last 7 labels and one more do not. */
    for (u32 first = 0; first <= GRM_LABELS - 7; first += GRM_LABELS - 7) {
        reset();
        add_module(PACKAGE_MODULES_PATH, "LABEL.GRM", &(struct spec){.id = "LABEL",
            .uses = {"CORE.ASPECT"}, .indicator_count = 1, .condition_setting = GRM_USED,
            .icon_kind = GRM_ICON_LABEL, .icon_setting = GRM_USED, .icon_first = first});
        assert(install() == 0 && module_status[0] == (first ? GRM_BAD_DESCRIPTOR : 0));
        assert(module_indicator_count == (first ? 0U : 1U));
    }
}
static void legacy_tone_icon_is_only_for_old_module_abi(void)
{
    for (u32 legacy = 0; legacy < 2; ++legacy) {
        reset();
        add_module(PACKAGE_MODULES_PATH, "TONE.GRM", &(struct spec){.id = "TONE",
            .uses = {"CORE.TONE"}, .legacy_abi = (int)legacy});
        assert(install() == 0 && module_status[0] == 0);
        assert(module_legacy_tone_owner == (legacy ? CODE_AT : 0));
    }
    reset();
    add_module(PACKAGE_MODULES_PATH, "TONE.GRM", &(struct spec){.id = "TONE",
        .uses = {"CORE.TONE"}, .indicator_count = 1, .indicator_slot = GRM_SLOT_IMAGE_CONTROL,
        .condition_setting = GRM_USED, .icon_kind = GRM_ICON_TONE, .icon_setting = GRM_USED});
    assert(install() == 0 && module_status[0] == 0 && module_indicator_count == 1);
    assert(word(word(module_indicators[0] + GRM_INDICATOR_ICON_AT) + GRM_ICON_SETTING_AT) == GRKIT_CORE_TONE);
}

/* IMPORTED.TXT: a line with the size and the stamp of an inbox file gives its
   hash while both stay the same, so the RTOS side does not read the file. A
   line without them, or another size or stamp, needs the file. */
static void imported_lines_give_the_hash_of_an_unchanged_file(void)
{
    const char *text = "1315C678.GRF FDEB34F1 00002200 1234ABCD\r\n13A530FC.GRF 19B4C184\r\n";
    long n = (long)strlen(text);
    memcpy(recorded, text, (size_t)n);
    u32 hash = 0;
    assert(recorded_hash(n, "1315c678.grf", 0x2200, 0x1234abcd, &hash) && hash == 0xfdeb34f1);
    assert(!recorded_hash(n, "1315C678.GRF", 0x2200, 0x1234abce, &hash));
    assert(!recorded_hash(n, "1315C678.GRF", 0x2000, 0x1234abcd, &hash));
    assert(!recorded_hash(n, "13A530FC.GRF", 0, 0, &hash));
    assert(!recorded_hash(n, "1315C678.GR", 0x2200, 0x1234abcd, &hash));
    assert(!recorded_hash(-1, "1315C678.GRF", 0x2200, 0x1234abcd, &hash));
    /* The lines that record_line writes read back the same. */
    u32 m = record_line(0, "1315C678.GRF", 0xfdeb34f1, 1, 0x2200, 0x1234abcd);
    m = record_line(m, "13A530FC.GRF", 0x19b4c184, 0, 0, 0);
    assert(m == (u32)n && !memcmp(record, text, (size_t)n));
}

static void *run(void *unused)
{
    (void)unused;
    imported_lines_give_the_hash_of_an_unchanged_file();
    indicators_resolve_settings_and_preserve_shared_descriptors();
    indicators_reject_conflicts_overflow_and_used_value_ranges();
    legacy_tone_icon_is_only_for_old_module_abi();
    modules_come_from_both_folders_in_header_order();
    more_modules_than_the_list_holds_are_counted();
    a_full_list_keeps_the_package_modules();
    safe_mode_loads_no_module();
    a_bad_module_keeps_the_others();
    module_code_follows_the_actual_core();
    code_overflow_leaves_camera_memory_unchanged();
    the_budget_counts_the_services_block();
    install_writes_the_settings_and_the_contexts();
    setting_defaults_and_saved_zero();
    framing_mode_state_survives_install_planning();
    install_settings_page();
    install_color_only_rows();
    a_color_only_module_needs_a_color_camera();
    install_group_page();
    the_switch_stops_each_module();
    layer_conditions_resolve_shared_and_optional_settings();
    layers_follow_their_order_and_filters_their_stage();
    the_module_list_shows_each_result();
    the_layers_and_filters_fill_their_slots();
    settings_rows_of_used_settings();
    the_fn_setting_of_a_page_gets_its_handle();
    a_setting_of_a_module_that_did_not_link_is_missing();
    reset();
    return 0;
}
int main(void)
{
    pthread_attr_t attributes;
    pthread_t thread;
    assert(!pthread_attr_init(&attributes) && !pthread_attr_setstack(&attributes, test_stack, sizeof test_stack));
    assert(!pthread_create(&thread, &attributes, run, 0) && !pthread_join(thread, 0));
    return 0;
}
