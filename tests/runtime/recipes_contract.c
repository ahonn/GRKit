#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef unsigned char u8;
typedef unsigned short u16;
typedef unsigned int u32;
#include "abi.h"

/* Three effect records at a camera address, mapped to `records`; the
   native settings object and its record (GR III: the record is the word at
   NATIVE_RECORD of the object). */
#define TEST_RECORDS 0x59005000U
#define TEST_COUNT 3U
#define TEST_SETTINGS 0x5a000000U
#define TEST_NATIVE 0x5a100000U
static u8 records[TEST_COUNT * GRKIT_RECORD_SIZE];
static u32 settings_object[2];
static u8 native_record_bytes[4096];
static u8 *mapped(u32 address)
{
    if (address >= TEST_SETTINGS && address < TEST_SETTINGS + sizeof(settings_object))
        return (u8 *)settings_object + (address - TEST_SETTINGS);
    if (address >= TEST_NATIVE && address < TEST_NATIVE + sizeof(native_record_bytes))
        return native_record_bytes + (address - TEST_NATIVE);
    assert(address >= TEST_RECORDS && address < TEST_RECORDS + sizeof(records));
    return records + (address - TEST_RECORDS);
}
#define WORD(a,o) (*(volatile u32 *)mapped((a)+(o)))
#define HALF(a,o) (*(volatile u16 *)mapped((a)+(o)))
#define BYTE(a,o) (*(volatile u8 *)mapped((a)+(o)))
/* The build header of the target gives the native tables. Each UserData
   Set is kept as "name=label". */
#include "firmware.h"
static u32 settings_present;
static char native_calls[64][96];
static u32 native_call_count;
static void native_record_call(const char *const *argv)
{
    assert(native_call_count < 64);
    assert(!strcmp(argv[0], "UserData") && !strcmp(argv[1], "Set"));
    snprintf(native_calls[native_call_count++], 96, "%s=%s", argv[2], argv[3]);
}
#ifdef NATIVE_RECORD
#define NATIVE_SETTINGS() (settings_present ? TEST_SETTINGS : 0U)
#else
#define NATIVE_SETTINGS() (settings_present ? TEST_NATIVE : 0U)
#endif
#define NATIVE_SET(argv) native_record_call(argv)
/* The camera is GR IV Monochrome; the type setter is kept as "TYPE=code". */
static u32 camera_monochrome;
static void native_record_type(u32 code)
{
    assert(native_call_count < 64);
    snprintf(native_calls[native_call_count++], 96, "TYPE=%u", code);
}
#define NATIVE_MONOCHROME() (camera_monochrome)
#define NATIVE_SET_TYPE(code) native_record_type(code)

static volatile u32 header[GRKIT_HOST_HEADER_SIZE / 4];
#include "tone.h"

/* The module settings table (runtime/setting_table.h): another module's
   setting, then Glow's Bloom, Halation, and Halation color, as INSTALL
   writes them when the Glow module loads. They are the first settings of
   GRKIT_RECIPE_SETTINGS. */
#include "setting_table.h"
static const u32 module_ids[] = {GRKIT_RECIPE_SETTING_IDS};
#define TEST_SETTINGS_COUNT 4U
static u32 module_table[SETTINGS_ENTRY(TEST_SETTINGS_COUNT)];
static u32 glow_loaded;
static volatile u32 *settings_table(volatile u32 *h)
{
    (void)h;
    return glow_loaded ? module_table : 0;
}
static u32 setting_limit(volatile u32 *h, u32 handle)
{
    volatile u32 *t = settings_table(h);
    assert(t && handle < t[0]);
    return t[SETTINGS_ENTRY(handle) + 2];
}
/* Load Glow, with or without its color setting. */
static void load_glow(u32 with_color)
{
    glow_loaded = 1;
    memset(module_table, 0, sizeof module_table);
    module_table[0] = with_color ? 4U : 3U;
    /* An unrelated setting with more values comes first. */
    module_table[SETTINGS_ENTRY(0)] = 12345U;
    module_table[SETTINGS_ENTRY(0) + 2] = 9U;
    for (u32 i = 0; i + 1U < module_table[0]; ++i) {
        module_table[SETTINGS_ENTRY(i + 1U)] = module_ids[i];
        module_table[SETTINGS_ENTRY(i + 1U) + 2] = 4U;
    }
}
/* Glow's setting i (0 Bloom, 1 Halation, 2 color). */
static u32 glow_value(u32 i) { return module_table[SETTINGS_ENTRY(i + 1U) + 1]; }
static void set_glow(u32 bloom, u32 halation, u32 color)
{
    module_table[SETTINGS_ENTRY(1) + 1] = bloom;
    module_table[SETTINGS_ENTRY(2) + 1] = halation;
    module_table[SETTINGS_ENTRY(3) + 1] = color;
}

/* The capabilities as the menu sets them (runtime/setting_store.h): the
   effect is the active record, Grain and its size are header words, and a
   module setting is its table entry. */
static u32 stores;
static u32 setting_value(volatile u32 *h, u32 handle)
{
    if (handle == GRKIT_CORE_TONE) return tone_value(h);
    if (handle == GRKIT_CORE_GRAIN) return h[GRKIT_HOST_GRAIN / 4];
    if (handle == GRKIT_CORE_GRAIN_SIZE) return h[GRKIT_HOST_GRAIN_SIZE / 4];
    if (handle == GRKIT_CORE_BORDER) return h[GRKIT_HOST_BORDER / 4];
    if (handle == GRKIT_CORE_BORDER_COLOR) return h[GRKIT_HOST_BORDER_COLOR / 4];
    assert(settings_table(h) && handle < module_table[0]);
    return module_table[SETTINGS_ENTRY(handle) + 1];
}
static void setting_store(volatile u32 *h, u32 handle, u32 value)
{
    ++stores;
    if (handle == GRKIT_CORE_TONE) h[GRKIT_HOST_ACTIVE / 4] = tone_record(h, value);
    else if (handle == GRKIT_CORE_GRAIN) h[GRKIT_HOST_GRAIN / 4] = value;
    else if (handle == GRKIT_CORE_GRAIN_SIZE) h[GRKIT_HOST_GRAIN_SIZE / 4] = value;
    else if (handle == GRKIT_CORE_BORDER) { assert(value < GRKIT_CORE_BORDER_VALUES); h[GRKIT_HOST_BORDER / 4] = value; }
    else if (handle == GRKIT_CORE_BORDER_COLOR) {
        assert(value < GRKIT_CORE_BORDER_COLOR_VALUES);
        h[GRKIT_HOST_BORDER_COLOR / 4] = value;
    }
    else {
        assert(settings_table(h) && handle < module_table[0] && value < module_table[SETTINGS_ENTRY(handle) + 2]);
        module_table[SETTINGS_ENTRY(handle) + 1] = value;
    }
}
/* The value behind generated recipe IDs (runtime/recipe_id.h), which a test
   sets so that it knows each candidate. */
static u32 test_id_source;
#define RECIPE_ID_SOURCE(high) (*(high) = 0U, test_id_source)
#include "native_look.h"
#include "recipes.h"
#include "services.h"
#include "recipe_details.h"

#define RUNNING (GRKIT_CORE_TONE_BIT | GRKIT_CORE_GRAIN_BIT | GRKIT_CORE_GRAIN_SIZE_BIT | GRKIT_CORE_RECIPE_BIT)
/* The capabilities of the Border module. */
#define BORDER_BITS (GRKIT_CORE_BORDER_BIT | GRKIT_CORE_BORDER_COLOR_BIT)
/* The settings of GRKIT_RECIPE_SETTINGS. */
#define BLOOM 0U
#define HALATION 1U
#define HALATION_COLOR 2U
#define BORDER 3U
#define BORDER_COLOR 4U
/* Whether recipe `slot` holds setting i of GRKIT_RECIPE_SETTINGS, and its
   value. */
static u32 slot_words[GRKIT_RECIPE_WORDS];
static const u32 *words_of(u32 slot)
{
    for (u32 k = 0; k < GRKIT_RECIPE_WORDS; ++k) slot_words[k] = recipe_at(header, slot)[k];
    return slot_words;
}
#define HELD(slot, i) recipe_setting_held(words_of(slot), i)
#define VALUE(slot, i) recipe_setting_value(words_of(slot), i)
static void set_border(u32 style, u32 color)
{
    header[GRKIT_HOST_BORDER / 4] = style;
    header[GRKIT_HOST_BORDER_COLOR / 4] = color;
}

static u32 record_at(u32 i) { return TEST_RECORDS + i * GRKIT_RECORD_SIZE; }
/* Portable indexes of runtime/native.h. */
#define TYPE_STANDARD 0U
#define TYPE_MONOTONE 2U
#define TYPE_POSITIVE 7U
#define MODE_DAYLIGHT 4U
#define MODE_TEMPERATURE 14U
/* A type and a mode that the family does not have: Cinema (Yellow) and
   AWB Warm on GR III, a type after the table and Multi-pattern Auto on
   GR IV. */
#ifdef NATIVE_FAMILY_GR3
#define TYPE_MISSING 12U
#define MODE_MISSING 2U
#else
#define TYPE_MISSING NATIVE_TYPES
#define MODE_MISSING 1U
#endif
/* The ID of record i, and the code of its effect in a recipe. */
static void set_id(u32 i, const char *id)
{
    u32 n = 0;
    for (; id[n]; ++n) BYTE(TEST_RECORDS + i * GRKIT_RECORD_SIZE, GRKIT_RECORD_ID + n) = (u8)id[n];
    BYTE(TEST_RECORDS + i * GRKIT_RECORD_SIZE, GRKIT_RECORD_ID + n) = 0;
}
static u32 code_of(u32 i) { return recipe_effect_code(TEST_RECORDS + i * GRKIT_RECORD_SIZE); }
static void reset(void)
{
    memset((void *)header, 0, sizeof header);
    memset(records, 0, sizeof records);
    memset(native_record_bytes, 0, sizeof native_record_bytes);
    native_call_count = 0;
    settings_present = 0;
    camera_monochrome = 0;
    glow_loaded = 0;
    settings_object[1] = TEST_NATIVE;
    header[GRKIT_HOST_COUNT / 4] = TEST_COUNT;
    header[GRKIT_HOST_RECORDS / 4] = TEST_RECORDS;
    header[GRKIT_HOST_CAPABILITIES / 4] = RUNNING;
    /* Effect IDs, and ImageTone codes that INSTALL moved: a recipe finds its
       effect by the hash of the ID, not by the code. */
    for (u32 i = 0; i < TEST_COUNT; ++i) {
        set_id(i, i == 0 ? "effect-0" : i == 1 ? "effect-1" : "effect-2");
        HALF(record_at(i), GRKIT_RECORD_CODE) = (u16)(0x8000U | (0x100U + i));
    }
    stores = 0;
}
static void select_effect(u32 i) { header[GRKIT_HOST_ACTIVE / 4] = i < TEST_COUNT ? record_at(i) : 0; }
static void set_fn(u32 i, u32 index, u8 stored) { BYTE(record_at(i), GRKIT_RECORD_PARAMETERS + index) = stored; }
static u8 fn_of(u32 i, u32 index) { return BYTE(record_at(i), GRKIT_RECORD_PARAMETERS + index); }
static void set_grain(u32 level, u32 size)
{
    header[GRKIT_HOST_GRAIN / 4] = level;
    header[GRKIT_HOST_GRAIN_SIZE / 4] = size;
}
/* The changes that the loader saves: those of the state file and of the
   recipe file. */
static u32 changes(void) { return header[GRKIT_HOST_STATE_CHANGES / 4] + header[GRKIT_HOST_RECIPE_CHANGES / 4]; }

/* A save holds the effect, its Fn adjustments, and the grain. The second
   recipe of an effect gets number 2, the next 3; another look starts again. */
static void save_numbers_each_look(void)
{
    reset();
    select_effect(1); set_fn(1, 0, 3); set_fn(1, 8, 7); set_grain(2, 3);
    u32 before = changes();
    assert(recipe_save(header) == 0);
    assert(changes() == before + 1);
    u32 w = recipe_at(header, 0)[0];
    assert(RECIPE_LOOK(w) == RECIPE_LOOK_EFFECT && RECIPE_CODE(w) == code_of(1));
    /* The hash of "effect-1" (grkit_core::recipes::effect_code). */
    assert(code_of(1) == 0x637eU);
    assert((w & RECIPE_GRAIN) && RECIPE_GRAIN_LEVEL(w) == 2 && RECIPE_GRAIN_SIZE(w) == 3);
    assert(RECIPE_NUMBER(w) == 0);
    /* Base-9 digits, the first adjustment lowest: 3 + 7 * 9^8. */
    assert(recipe_at(header, 0)[RECIPE_FN_WORD] == 3U + 7U * 43046721U);
    assert(recipe_save(header) == 1 && RECIPE_NUMBER(recipe_at(header, 1)[0]) == 2);
    assert(recipe_save(header) == 2 && RECIPE_NUMBER(recipe_at(header, 2)[0]) == 3);
    select_effect(TEST_COUNT);
    assert(recipe_save(header) == 3);
    w = recipe_at(header, 3)[0];
    assert(RECIPE_LOOK(w) == RECIPE_LOOK_OFF && RECIPE_NUMBER(w) == 0);
    /* Each word stays below 2^29, so the state file keeps it. */
    for (u32 i = 0; i < RECIPE_ALL * GRKIT_RECIPE_WORDS; ++i) assert(header[GRKIT_HOST_RECIPES / 4 + i] < 1U << 29);
    assert(recipe_count(header) == 4);
}

/* A capability that does not run is not held, and its setting stays. */
static void save_holds_only_running_capabilities(void)
{
    reset();
    header[GRKIT_HOST_CAPABILITIES / 4] = GRKIT_CORE_RECIPE_BIT | GRKIT_CORE_GRAIN_BIT;
    select_effect(0); set_grain(4, 2);
    assert(recipe_save(header) == 0);
    u32 w = recipe_at(header, 0)[0];
    assert(RECIPE_LOOK(w) == RECIPE_LOOK_KEPT);
    assert((w & RECIPE_GRAIN) && RECIPE_GRAIN_LEVEL(w) == 4 && RECIPE_GRAIN_SIZE(w) == 0);
    /* Apply keeps the effect: Film Look does not run. */
    select_effect(2); set_grain(0, 0);
    assert(recipe_apply(header, 0) == RECIPE_READY);
    assert(header[GRKIT_HOST_ACTIVE / 4] == record_at(2));
    assert(header[GRKIT_HOST_GRAIN / 4] == 4);
    /* The size does not run, so it stays. */
    assert(header[GRKIT_HOST_GRAIN_SIZE / 4] == 0);
}

/* The camera holds GRKIT_RECIPE_ON_MAX recipes that are on. */
static void save_stops_when_full(void)
{
    reset();
    select_effect(0);
    for (u32 i = 0; i < GRKIT_RECIPE_ON_MAX; ++i) assert(recipe_save(header) == i);
    u32 before = changes();
    assert(recipe_save(header) == RECIPE_FULL);
    assert(changes() == before && recipe_count(header) == GRKIT_RECIPE_ON_MAX);
    assert(recipe_entries(header) == GRKIT_RECIPE_ON_MAX);
    /* The numbers stop at the largest one. */
    assert(RECIPE_NUMBER(recipe_at(header, GRKIT_RECIPE_ON_MAX - 1)[0]) == RECIPE_NUMBER_MAX);
}

/* The recipes that are off follow those that are on, and the camera does not
   list them. A save goes before them, and they move down with their names;
   with all slots taken, Save is full. Delete takes only a recipe that is on. */
static void recipes_that_are_off_stay_for_the_site(void)
{
    reset();
    select_effect(0);
    assert(recipe_save(header) == 0);
    recipe_name_set(header, 0, (const u8 *)"On");
    for (u32 slot = 1; slot < 4; ++slot) {
        recipe_at(header, slot)[0] = RECIPE_USED | RECIPE_LOOK_OFF;
        recipe_at(header, slot)[RECIPE_MODULE_WORD] = RECIPE_OFF;
    }
    recipe_name_set(header, 1, (const u8 *)"Off");
    assert(recipe_count(header) == 4 && recipe_entries(header) == 1);
    assert(recipe_shown_count(header) == 1 && recipe_shown_at(header, 1) == RECIPE_NONE);
    /* A save goes after the recipe that is on. */
    select_effect(1);
    assert(recipe_save(header) == 1);
    assert(recipe_count(header) == 5 && recipe_entries(header) == 2);
    assert(recipe_on(header, 1) && !recipe_on(header, 2) && recipe_name_length(header, 2) == 3);
    assert(recipe_name_char(header, 2, 0) == 'O' && recipe_name_char(header, 2, 1) == 'f');
    assert(!recipe_name_length(header, 1));
    /* Delete and Show in ADJ take only an entry. */
    recipe_delete(header, 2);
    assert(recipe_count(header) == 5);
    recipe_show_in_adj(header, 2, 0);
    assert(!(recipe_at(header, 2)[0] & RECIPE_HIDDEN));
    /* All slots taken: Save is full, also with fewer than GRKIT_RECIPE_ON_MAX on. */
    while (recipe_count(header) < GRKIT_RECIPE_SLOTS) {
        u32 n = recipe_count(header);
        recipe_at(header, n)[0] = RECIPE_USED | RECIPE_LOOK_OFF;
        recipe_at(header, n)[RECIPE_MODULE_WORD] = RECIPE_OFF;
    }
    assert(recipe_save(header) == RECIPE_FULL && recipe_entries(header) == 2);
}

/* Each recipe keeps its ID and its name when Save and Delete move the
   recipes. A new recipe takes the first generated candidate that no recipe
   has. */
static void each_recipe_keeps_its_id(void)
{
    reset();
    test_id_source = 0x1234U;
    const u32 c0 = recipe_id_candidate(0), c1 = recipe_id_candidate(1);
    assert(c0 && c1 && c0 != c1 && c0 <= GRKIT_RECIPE_ID_MAX && c1 <= GRKIT_RECIPE_ID_MAX);
    select_effect(0);
    assert(recipe_save(header) == 0 && *recipe_id_at(header, 0) == c0);
    /* Two recipes that are off, from the site, with IDs 40 and 41. */
    for (u32 slot = 1; slot < 3; ++slot) {
        recipe_at(header, slot)[0] = RECIPE_USED | RECIPE_LOOK_OFF;
        recipe_at(header, slot)[RECIPE_MODULE_WORD] = RECIPE_OFF;
        *recipe_id_at(header, slot) = 39 + slot;
    }
    recipe_name_set(header, 2, (const u8 *)"B");
    /* The same source: recipe 0 has the first candidate, so the save takes
       the second. It goes after the recipe that is on, and the others move
       down with their IDs and names. */
    select_effect(1);
    assert(recipe_save(header) == 1 && *recipe_id_at(header, 1) == c1);
    assert(*recipe_id_at(header, 2) == 40 && *recipe_id_at(header, 3) == 41 && recipe_name_char(header, 3, 0) == 'B');
    /* Delete moves them up again and clears the last slot. */
    recipe_delete(header, 0);
    assert(*recipe_id_at(header, 0) == c1 && *recipe_id_at(header, 1) == 40 && *recipe_id_at(header, 2) == 41);
    assert(recipe_name_char(header, 2, 0) == 'B' && !*recipe_id_at(header, 3) && !recipe_name_length(header, 3));
    /* The ID of the deleted recipe is free again. */
    assert(recipe_save(header) == 1 && *recipe_id_at(header, 1) == c0);
}

/* Apply writes the Fn adjustments before it selects the effect, counts the
   change for the live view, and stores the settings before it as an undo
   step. */
static void apply_sets_the_look_and_stores_an_undo_step(void)
{
    reset();
    select_effect(1); set_fn(1, 2, 5); set_grain(3, 1);
    assert(recipe_save(header) == 0);
    select_effect(0); set_fn(1, 2, 0); set_grain(1, 2);
    u32 parameters = header[GRKIT_HOST_PARAMETER_CHANGES / 4];
    assert(!recipe_matches(header, 0) && recipe_current(header) == RECIPE_NONE);
    assert(recipe_apply(header, 0) == RECIPE_READY);
    assert(header[GRKIT_HOST_ACTIVE / 4] == record_at(1));
    assert(fn_of(1, 2) == 5);
    assert(header[GRKIT_HOST_PARAMETER_CHANGES / 4] == parameters + 1);
    assert(header[GRKIT_HOST_GRAIN / 4] == 3 && header[GRKIT_HOST_GRAIN_SIZE / 4] == 1);
    assert(recipe_matches(header, 0) && recipe_current(header) == 0);
    assert(recipe_undo_count(header) == 1);
    u32 step = recipe_at(header, GRKIT_RECIPE_SLOTS)[0];
    assert(RECIPE_LOOK(step) == RECIPE_LOOK_EFFECT && RECIPE_CODE(step) == code_of(0));
    assert(RECIPE_GRAIN_LEVEL(step) == 1 && RECIPE_GRAIN_SIZE(step) == 2);
    /* Equal Fn adjustments count no change. */
    assert(recipe_apply(header, 0) == RECIPE_READY);
    assert(header[GRKIT_HOST_PARAMETER_CHANGES / 4] == parameters + 1);
    /* Undo goes back one apply at a time. */
    recipe_undo(header);
    assert(recipe_undo_count(header) == 1 && header[GRKIT_HOST_ACTIVE / 4] == record_at(1));
    recipe_undo(header);
    assert(recipe_undo_count(header) == 0);
    assert(header[GRKIT_HOST_ACTIVE / 4] == record_at(0));
    assert(header[GRKIT_HOST_GRAIN / 4] == 1 && header[GRKIT_HOST_GRAIN_SIZE / 4] == 2);
    /* Undo without a step does nothing. */
    u32 before = changes();
    recipe_undo(header);
    assert(changes() == before);
}

/* The undo steps keep the newest GRKIT_RECIPE_UNDO applies. */
static void undo_keeps_the_newest_steps(void)
{
    reset();
    for (u32 level = 0; level < 2; ++level) { set_grain(level, 0); recipe_save(header); }
    /* Grain levels before each apply: 0, 1, 0, 1, ... */
    for (u32 i = 0; i < GRKIT_RECIPE_UNDO + 2; ++i) {
        set_grain(i & 1U ? 1 : 0, 0);
        assert(recipe_apply(header, i & 1U ? 0 : 1) == RECIPE_READY);
    }
    assert(recipe_undo_count(header) == GRKIT_RECIPE_UNDO);
    /* The oldest steps went; the newest holds the level before the last
       apply. */
    for (u32 i = GRKIT_RECIPE_UNDO + 2; i-- > 2;) {
        recipe_undo(header);
        assert(header[GRKIT_HOST_GRAIN / 4] == (i & 1U ? 1U : 0U));
    }
    assert(recipe_undo_count(header) == 0);
}

/* A recipe of an effect that is not installed, or without Film Look,
   changes nothing. */
static void missing_effect_changes_nothing(void)
{
    reset();
    select_effect(2); set_grain(2, 0);
    recipe_save(header);
    set_id(2, "gone");
    select_effect(0); set_grain(0, 0);
    u32 before = changes();
    assert(recipe_apply(header, 0) == RECIPE_MISSING);
    assert(header[GRKIT_HOST_ACTIVE / 4] == record_at(0) && header[GRKIT_HOST_GRAIN / 4] == 0);
    assert(changes() == before && recipe_undo_count(header) == 0 && !stores);
    set_id(2, "effect-2");
    header[GRKIT_HOST_CAPABILITIES / 4] &= ~(u32)GRKIT_CORE_TONE_BIT;
    assert(recipe_apply(header, 0) == RECIPE_MISSING);
    /* A slot past the recipes. */
    assert(recipe_apply(header, 1) == RECIPE_MISSING);
}

/* An undo step whose effect went restores the rest. */
static void undo_skips_a_missing_effect(void)
{
    reset();
    select_effect(0); set_grain(0, 0);
    recipe_save(header);
    select_effect(2); set_grain(3, 0);
    assert(recipe_apply(header, 0) == RECIPE_READY);
    set_id(2, "gone");
    recipe_undo(header);
    assert(header[GRKIT_HOST_ACTIVE / 4] == record_at(0));
    assert(header[GRKIT_HOST_GRAIN / 4] == 3);
}

/* Delete moves the later recipes up. A word that is not a recipe ends the
   list, as after a damaged state file. */
static void delete_and_damaged_words(void)
{
    reset();
    for (u32 level = 0; level < 3; ++level) { set_grain(level, 0); recipe_save(header); }
    recipe_delete(header, 0);
    assert(recipe_count(header) == 2);
    assert(RECIPE_GRAIN_LEVEL(recipe_at(header, 0)[0]) == 1 && RECIPE_GRAIN_LEVEL(recipe_at(header, 1)[0]) == 2);
    assert(!recipe_at(header, 2)[0]);
    recipe_delete(header, 5);
    assert(recipe_count(header) == 2);
    recipe_at(header, 1)[0] = RECIPE_USED | 7U << RECIPE_GRAIN_LEVEL_SHIFT;
    assert(recipe_count(header) == 1);
    /* The hidden bit is the highest bit of a recipe word. */
    recipe_at(header, 1)[0] = 0x20000000U | RECIPE_USED;
    assert(recipe_count(header) == 1);
    recipe_at(header, 1)[0] = RECIPE_USED | 3U;
    assert(recipe_count(header) == 1);
}

/* A recipe that holds nothing that runs matches nothing. */
static void empty_recipe_matches_nothing(void)
{
    reset();
    header[GRKIT_HOST_CAPABILITIES / 4] = GRKIT_CORE_RECIPE_BIT;
    /* A settings record with a type and a mode that the camera does not
       have: nothing native is held. */
    settings_present = 1;
    native_record_bytes[NATIVE_TYPE_FIELD] = 0xee;
    native_record_bytes[NATIVE_MODE_FIELD] = 0xee;
    for (u32 c = 0; c < NATIVE_CORRECTIONS; ++c) native_record_bytes[native_correction_fields[c]] = 0xee;
    /* Exposure settings out of range: 2^30 stops, and unknown labels. */
    *(u32 *)(native_record_bytes + NATIVE_EXPOSURE_FIELD) = 0x4e800000U;
    *(u32 *)(native_record_bytes + NATIVE_ISO_UPPER_FIELD) = 0x4e800000U;
    native_record_bytes[NATIVE_METERING_FIELD] = 0xee;
    native_record_bytes[NATIVE_ND_FIELD] = 0xee;
    recipe_save(header);
    assert(!recipe_matches(header, 0));
    assert(recipe_apply(header, 0) == RECIPE_READY && !stores);
}

/* The settings record of a native look: Positive Film with saturation P2
   and contrast M1, Daylight with A3 and M2, and 5500 K. */
static void set_native_look(void)
{
    settings_present = 1;
    native_record_bytes[NATIVE_TYPE_FIELD] = native_type_codes[TYPE_POSITIVE];
    for (u32 p = 0; p < NATIVE_LEVELS; ++p) native_record_bytes[native_field(TYPE_POSITIVE, p)] = 4;
    native_record_bytes[native_field(TYPE_POSITIVE, NATIVE_PARAMETER_SATURATION)] = 6;
    native_record_bytes[native_field(TYPE_POSITIVE, NATIVE_PARAMETER_CONTRAST)] = 3;
    native_record_bytes[NATIVE_MODE_FIELD] = (u8)native_mode_codes[MODE_DAYLIGHT];
    native_record_bytes[native_shift_fields[2 * MODE_DAYLIGHT]] = 17;
    native_record_bytes[native_shift_fields[2 * MODE_DAYLIGHT + 1]] = 16;
    native_record_bytes[NATIVE_TEMPERATURE_FIELD] = 5500 & 0xff;
    native_record_bytes[NATIVE_TEMPERATURE_FIELD + 1] = 5500 >> 8;
}
static int called(const char *call)
{
    for (u32 i = 0; i < native_call_count; ++i) if (!strcmp(native_calls[i], call)) return 1;
    return 0;
}

/* Without an effect, a save holds the native type, its levels, and the
   white balance; apply writes them through UserData Set by name and
   label, the type before its parameters. */
static void native_looks_save_and_apply(void)
{
    reset();
    set_native_look();
    select_effect(TEST_COUNT);
    assert(recipe_save(header) == 0);
    volatile u32 *w = recipe_at(header, 0);
    assert(RECIPE_LOOK(w[0]) == RECIPE_LOOK_OFF);
    assert(NATIVE_TYPE(w[RECIPE_NATIVE_WORD]) == TYPE_POSITIVE + 1);
    assert(NATIVE_MODE(w[RECIPE_NATIVE_WORD]) == MODE_DAYLIGHT + 1);
    assert(NATIVE_BA(w[RECIPE_NATIVE_WORD]) == 17 && NATIVE_GM(w[RECIPE_NATIVE_WORD]) == 16);
    assert(NATIVE_TEMPERATURE(w[RECIPE_EXTRA_WORD]) == 300);
    /* Digits of (native - 4) mod 9, the first lowest: saturation 2,
       contrast 8 (M1). */
    assert(w[RECIPE_FN_WORD] == 2U + 8U * 729U);
    assert(recipe_matches(header, 0));
    for (u32 i = 0; i < RECIPE_ALL * GRKIT_RECIPE_WORDS; ++i) assert(header[GRKIT_HOST_RECIPES / 4 + i] < 1U << 29);
    /* Another look, then apply. */
    native_record_bytes[NATIVE_TYPE_FIELD] = native_type_codes[TYPE_STANDARD];
    native_record_bytes[NATIVE_MODE_FIELD] = (u8)native_mode_codes[MODE_TEMPERATURE];
    assert(!recipe_matches(header, 0));
    assert(recipe_apply(header, 0) == RECIPE_READY);
    assert(!strcmp(native_calls[0], "StillImageControlNo0=PositiveFilm"));
    assert(called("StillImageControlNo0ParamPositiveFilmSaturation=P2"));
    assert(called("StillImageControlNo0ParamPositiveFilmContrast=M1"));
    assert(called("StillImageControlNo0ParamPositiveFilmClarityControl=OFF"));
    assert(called("StillWhiteBalance=Daylight"));
    assert(called("StillWhiteBalanceDaylightAdjustBA=A3"));
    assert(called("StillWhiteBalanceDaylightAdjustGM=M2"));
    /* The temperature only goes with ColorTemp1. */
    assert(!called("StillWhiteBalanceColorTemp=5500"));
    /* The undo step holds the look before: Standard and ColorTemp1, with
       its temperature. The main set is active, so no set change. */
    native_call_count = 0;
    recipe_undo(header);
    assert(!strcmp(native_calls[0], "StillImageControlNo0=Standard"));
    assert(called("StillWhiteBalance=ColorTemp1"));
    assert(called("StillWhiteBalanceColorTemp=5500"));
    assert(!called("StillActiveImageControlSet=No0"));
}

/* Parameter p of type t in set `set` of the settings record. */
static void set_parameter(u32 set, u32 t, u32 p, u8 v)
{
    native_record_bytes[native_field(t, p) + set * native_parameter_strides[t * NATIVE_PARAMETERS + p]] = v;
}

/* An effect replaces the native type: the recipe holds the white balance
   only. */
static void effects_hold_no_type(void)
{
    reset();
    set_native_look();
    select_effect(1);
    assert(recipe_save(header) == 0);
    assert(NATIVE_TYPE(recipe_at(header, 0)[RECIPE_NATIVE_WORD]) == 0);
    assert(NATIVE_MODE(recipe_at(header, 0)[RECIPE_NATIVE_WORD]) == MODE_DAYLIGHT + 1);
}

/* A Custom set is active: a save holds its type and parameters, from the
   fields of that set, so the recipe is the look that the camera shows.
   Apply selects the main set and writes them there. */
static void custom_sets_hold_their_type(void)
{
    reset();
    set_native_look();
    select_effect(TEST_COUNT);
    u32 set = NATIVE_SETS - 1;
    native_record_bytes[NATIVE_ACTIVE_SET_FIELD] = (u8)set;
    native_record_bytes[NATIVE_TYPE_FIELD + set * NATIVE_TYPE_SET_STRIDE] = native_type_codes[TYPE_STANDARD];
    for (u32 p = 0; p < NATIVE_LEVELS; ++p)
        if (native_field(TYPE_STANDARD, p)) set_parameter(set, TYPE_STANDARD, p, 4);
    set_parameter(set, TYPE_STANDARD, NATIVE_PARAMETER_SATURATION, 7);
    assert(recipe_save(header) == 0);
    volatile u32 *w = recipe_at(header, 0);
    assert(RECIPE_LOOK(w[0]) == RECIPE_LOOK_OFF);
    assert(NATIVE_TYPE(w[RECIPE_NATIVE_WORD]) == TYPE_STANDARD + 1);
    /* Saturation 3, the first digit. */
    assert(w[RECIPE_FN_WORD] == 3U);
    assert(recipe_matches(header, 0));
    /* The main set is Positive Film: apply selects it, then writes the
       Custom look to it. */
    native_record_bytes[NATIVE_ACTIVE_SET_FIELD] = 0;
    assert(!recipe_matches(header, 0));
    native_record_bytes[NATIVE_ACTIVE_SET_FIELD] = (u8)set;
    native_call_count = 0;
    assert(recipe_apply(header, 0) == RECIPE_READY);
    assert(!strcmp(native_calls[0], "StillActiveImageControlSet=No0"));
    assert(called("StillImageControlNo0=Standard"));
    assert(called("StillImageControlNo0ParamStandardSaturation=P3"));
    /* A set after the sets of the build holds no type. */
    native_record_bytes[NATIVE_ACTIVE_SET_FIELD] = (u8)NATIVE_SETS;
    assert(recipe_save(header) == 1);
    assert(NATIVE_TYPE(recipe_at(header, 1)[RECIPE_NATIVE_WORD]) == 0);
}

/* A save without the Grain and Glow modules holds them at Off, as the
   picture had none: after another recipe, apply turns them off once the
   modules run, so the recipe gives the picture that it was saved from. */
static void saves_hold_modules_that_do_not_run_at_off(void)
{
    reset();
    header[GRKIT_HOST_CAPABILITIES / 4] = GRKIT_CORE_RECIPE_BIT | GRKIT_CORE_TONE_BIT;
    select_effect(1); set_grain(3, 0);
    assert(recipe_save(header) == 0);
    volatile u32 *w = recipe_at(header, 0);
    assert((w[0] & RECIPE_GRAIN) && RECIPE_GRAIN_LEVEL(w[0]) == 0 && RECIPE_GRAIN_SIZE(w[0]) == 0);
    assert(HELD(0, BLOOM) && VALUE(0, BLOOM) == 0);
    assert(HELD(0, HALATION) && VALUE(0, HALATION) == 0);
    assert(!HELD(0, HALATION_COLOR));
    /* The border is not part of the look: without the module, a save
       holds none, and apply keeps the border of the camera. */
    assert(!HELD(0, BORDER) && !HELD(0, BORDER_COLOR));
    /* The modules that do not run decide no match. */
    assert(recipe_matches(header, 0));
    /* The modules run, with Grain and Glow of another recipe. */
    header[GRKIT_HOST_CAPABILITIES / 4] = RUNNING;
    load_glow(1);
    set_grain(3, 2); set_glow(2, 1, 1);
    assert(!recipe_matches(header, 0));
    assert(recipe_apply(header, 0) == RECIPE_READY);
    assert(header[GRKIT_HOST_GRAIN / 4] == 0 && header[GRKIT_HOST_GRAIN_SIZE / 4] == 0);
    assert(glow_value(0) == 0 && glow_value(1) == 0);
    /* Halation is off, so its color does not change the picture. */
    assert(glow_value(2) == 1);
    assert(recipe_matches(header, 0));
}

/* A native value that the camera does not have: a type or a mode of the
   other family, a shift out of range, a correction value after the
   labels, or native grain on a type without it. Nothing changes. */
static void unsupported_native_values_change_nothing(void)
{
    reset();
    set_native_look();
    select_effect(TEST_COUNT);
    recipe_save(header);
    volatile u32 *w = recipe_at(header, 0);
    const u32 saved = w[RECIPE_NATIVE_WORD];
    const u32 extra = w[RECIPE_EXTRA_WORD];
    w[RECIPE_NATIVE_WORD] = (saved & ~31U) | (TYPE_MISSING + 1);
    native_call_count = 0;
    assert(recipe_apply(header, 0) == RECIPE_UNSUPPORTED && !native_call_count && !recipe_undo_count(header));
    w[RECIPE_NATIVE_WORD] = (saved & ~(31U << NATIVE_MODE_SHIFT)) | (MODE_MISSING + 1) << NATIVE_MODE_SHIFT;
    assert(recipe_apply(header, 0) == RECIPE_UNSUPPORTED);
    w[RECIPE_NATIVE_WORD] = (saved & ~(31U << NATIVE_BA_SHIFT)) | 29U << NATIVE_BA_SHIFT;
    assert(recipe_apply(header, 0) == RECIPE_UNSUPPORTED);
    w[RECIPE_NATIVE_WORD] = saved;
    /* High-ISO NR has 6 labels, so its value + 1 stops at 6. */
    w[RECIPE_EXTRA_WORD] = extra | 7U << (NATIVE_CORRECTION_SHIFT + 9);
    assert(recipe_apply(header, 0) == RECIPE_UNSUPPORTED);
    /* Standard has no native grain. */
    w[RECIPE_NATIVE_WORD] = (saved & ~31U) | (TYPE_STANDARD + 1);
    w[RECIPE_EXTRA_WORD] = extra | 1U << NATIVE_GRAIN_SHIFT;
    assert(recipe_apply(header, 0) == RECIPE_UNSUPPORTED);
    w[RECIPE_NATIVE_WORD] = saved;
    w[RECIPE_EXTRA_WORD] = extra;
    assert(recipe_apply(header, 0) == RECIPE_READY);
}

/* A save holds the current value of each correction setting, never Unset.
   Apply writes each one that is set; Unset keeps the camera value and is
   not compared. */
static void corrections_save_apply_and_unset(void)
{
    reset();
    header[GRKIT_HOST_CAPABILITIES / 4] = GRKIT_CORE_RECIPE_BIT;
    settings_present = 1;
    native_record_bytes[NATIVE_TYPE_FIELD] = 0xee;
    native_record_bytes[NATIVE_MODE_FIELD] = 0xee;
    /* Highlight Off, shadow Low, peripheral illumination On, NR Custom. */
    const u8 values[NATIVE_CORRECTIONS] = {2, 1, 1, 4};
    for (u32 c = 0; c < NATIVE_CORRECTIONS; ++c) native_record_bytes[native_correction_fields[c]] = values[c];
    assert(recipe_save(header) == 0);
    volatile u32 *w = recipe_at(header, 0);
    for (u32 c = 0; c < NATIVE_CORRECTIONS; ++c) assert(NATIVE_CORRECTION(w[RECIPE_EXTRA_WORD], c) == values[c] + 1U);
    assert(w[RECIPE_EXTRA_WORD] < 1U << 29);
    assert(recipe_matches(header, 0));
    /* Shadow and NR change on the camera. */
    native_record_bytes[native_correction_fields[1]] = 4;
    native_record_bytes[native_correction_fields[3]] = 0;
    assert(!recipe_matches(header, 0));
    assert(recipe_apply(header, 0) == RECIPE_READY);
    assert(called("StillHighlightCorrect=Off"));
    assert(called("StillShadowCorrect=Low"));
    assert(called("StillLensShadingCorrection=On"));
    assert(called("StillHighIsoNR=Custom"));
    /* The undo step holds the values before: shadow Off, NR Auto. */
    native_call_count = 0;
    recipe_undo(header);
    assert(called("StillShadowCorrect=Off") && called("StillHighIsoNR=Auto"));
    /* Shadow and NR Unset: apply leaves them, and they do not count for a
       match. */
    w[RECIPE_EXTRA_WORD] &= ~(7U << (NATIVE_CORRECTION_SHIFT + 3) | 7U << (NATIVE_CORRECTION_SHIFT + 9));
    assert(recipe_matches(header, 0));
    /* Without the exposure settings too, only the two set corrections. */
    w[RECIPE_MODULE_WORD] &= ~NATIVE_EXPOSURE_BITS;
    native_call_count = 0;
    assert(recipe_apply(header, 0) == RECIPE_READY);
    assert(native_call_count == 2);
    assert(called("StillHighlightCorrect=Off") && called("StillLensShadingCorrection=On"));
}

/* Particle of type t in the settings record: GR III the stored strength
   Off, High, Mid, Low; GR IV Off or On, the size, and the strength. */
static void set_particle(u32 t, u32 strength, u32 size)
{
    if (native_grain_sized(t)) {
        native_record_bytes[native_field(t, NATIVE_PARAMETER_PARTICLE)] = strength != 0;
        native_record_bytes[native_field(t, NATIVE_PARAMETER_PARTICLESIZE)] = (u8)size;
        native_record_bytes[native_field(t, NATIVE_PARAMETER_PARTICLESTRENGTH)] = (u8)(strength ? strength - 1 : 0);
    }
    else native_record_bytes[native_field(t, NATIVE_PARAMETER_PARTICLE)] = (u8)native_grain_particle(strength);
}
static void set_monotone(u32 strength, u32 size)
{
    settings_present = 1;
    native_record_bytes[NATIVE_TYPE_FIELD] = native_type_codes[TYPE_MONOTONE];
    for (u32 p = 0; p < NATIVE_LEVELS; ++p)
        if (native_field(TYPE_MONOTONE, p)) native_record_bytes[native_field(TYPE_MONOTONE, p)] = 4;
    native_record_bytes[native_field(TYPE_MONOTONE, NATIVE_PARAMETER_FILTEREFFECTCUSTOM)] = 0;
    native_record_bytes[NATIVE_MODE_FIELD] = 0xee;
    set_particle(TYPE_MONOTONE, strength, size);
}

/* The native grain of a type is part of its look, in portable values. */
static void native_grain_saves_and_applies(void)
{
    reset();
    header[GRKIT_HOST_CAPABILITIES / 4] = GRKIT_CORE_RECIPE_BIT;
#ifdef NATIVE_FAMILY_GR3
    const u32 size = 0;
#else
    const u32 size = 2;
#endif
    set_monotone(2, size);
    assert(recipe_save(header) == 0);
    volatile u32 *w = recipe_at(header, 0);
    assert(NATIVE_TYPE(w[RECIPE_NATIVE_WORD]) == TYPE_MONOTONE + 1);
    assert(NATIVE_GRAIN(w[RECIPE_EXTRA_WORD]) == 2 && NATIVE_GRAIN_SIZE(w[RECIPE_EXTRA_WORD]) == size);
    assert(recipe_matches(header, 0));
    set_particle(TYPE_MONOTONE, 3, size);
    assert(!recipe_matches(header, 0));
    assert(recipe_apply(header, 0) == RECIPE_READY);
#ifdef NATIVE_FAMILY_GR3
    assert(called("StillImageControlNo0ParamMonotoneGRParticle=Mid"));
#else
    assert(called("StillImageControlNo0ParamMonotoneGRParticle=On"));
    assert(called("StillImageControlNo0ParamMonotoneGRParticleSize=Large"));
    assert(called("StillImageControlNo0ParamMonotoneGRParticleStrength=Mid"));
#endif
    /* Off writes only Off. */
    set_particle(TYPE_MONOTONE, 0, 0);
    assert(recipe_save(header) == 1);
    assert(!(recipe_at(header, 1)[RECIPE_EXTRA_WORD] & NATIVE_GRAIN_BITS));
    native_call_count = 0;
    assert(recipe_apply(header, 1) == RECIPE_READY);
    assert(called("StillImageControlNo0ParamMonotoneGRParticle=Off"));
    assert(!called("StillImageControlNo0ParamMonotoneGRParticleSize=Small"));
}

/* Without a running Grain module, apply skips a Grain that is not Off,
   sets the rest with the native grain of the look, and reports
   RECIPE_SKIPPED_GRAIN. Grain Off changes nothing, so nothing is skipped. */
static void grain_needs_the_grain_module(void)
{
    reset();
    set_monotone(0, 0);
    select_effect(TEST_COUNT);
    set_grain(2, 0);
    assert(recipe_save(header) == 0);
    volatile u32 *w = recipe_at(header, 0);
    assert((w[0] & RECIPE_GRAIN) && RECIPE_GRAIN_LEVEL(w[0]) == 2);
    /* The Grain module stops. */
    header[GRKIT_HOST_CAPABILITIES / 4] = GRKIT_CORE_RECIPE_BIT | GRKIT_CORE_TONE_BIT;
    set_grain(0, 0);
    native_call_count = 0;
    assert(recipe_apply(header, 0) == (RECIPE_READY | RECIPE_SKIPPED_GRAIN));
    assert(called("StillImageControlNo0=MonotoneGR"));
    assert(called("StillImageControlNo0ParamMonotoneGRParticle=Off"));
    assert(header[GRKIT_HOST_GRAIN / 4] == 0);
    /* Grain does not run, so it does not decide the match. */
    assert(recipe_matches(header, 0));
    /* Grain Off: nothing is skipped. */
    w[0] &= ~(7U << RECIPE_GRAIN_LEVEL_SHIFT);
    assert(recipe_apply(header, 0) == RECIPE_READY);

    /* With the Grain module, CORE.GRAIN takes the Grain, and the native
       grain stays that of the look. */
    header[GRKIT_HOST_CAPABILITIES / 4] = RUNNING;
    w[0] = (w[0] & ~(7U << RECIPE_GRAIN_LEVEL_SHIFT)) | 3U << RECIPE_GRAIN_LEVEL_SHIFT;
    native_call_count = 0;
    assert(recipe_apply(header, 0) == RECIPE_READY);
    assert(header[GRKIT_HOST_GRAIN / 4] == 3);
    assert(called("StillImageControlNo0ParamMonotoneGRParticle=Off"));
}

#ifdef FW_IMAGE_CONTROL_SET
/* GR IV Monochrome: its types have no label, so apply calls the type
   setter with the stored value. It has no white balance, and a recipe of
   one sensor does not apply on the other. */
static void monochrome_types(void)
{
    const u32 standard = NATIVE_FIRST_MONOCHROME, solarized = NATIVE_TYPES - 1;
    reset();
    camera_monochrome = 1;
    settings_present = 1;
    select_effect(TEST_COUNT);
    native_record_bytes[NATIVE_TYPE_FIELD] = native_type_codes[standard];
    for (u32 p = 0; p < NATIVE_LEVELS; ++p)
        if (native_field(standard, p)) native_record_bytes[native_field(standard, p)] = 4;
    native_record_bytes[native_field(standard, NATIVE_PARAMETER_CONTRAST)] = 7;
    native_record_bytes[NATIVE_MODE_FIELD] = (u8)native_mode_codes[MODE_DAYLIGHT];
    assert(recipe_save(header) == 0);
    volatile u32 *w = recipe_at(header, 0);
    assert(NATIVE_TYPE(w[RECIPE_NATIVE_WORD]) == standard + 1);
    assert(NATIVE_MODE(w[RECIPE_NATIVE_WORD]) == 0);
    assert(recipe_matches(header, 0));
    for (u32 i = 0; i < RECIPE_ALL * GRKIT_RECIPE_WORDS; ++i) assert(header[GRKIT_HOST_RECIPES / 4 + i] < 1U << 29);
    native_record_bytes[NATIVE_TYPE_FIELD] = native_type_codes[standard + 1];
    assert(!recipe_matches(header, 0));
    assert(recipe_apply(header, 0) == RECIPE_READY);
    char type[16];
    snprintf(type, sizeof type, "TYPE=%u", native_type_codes[standard]);
    assert(!strcmp(native_calls[0], type));
    assert(called("AfterReleaseStillImageControlNo0ParamMonochromeStandardContrast=P3"));
    for (u32 i = 0; i < native_call_count; ++i) {
        assert(strncmp(native_calls[i], "StillImageControlNo0=", 21));
        assert(strncmp(native_calls[i], "StillWhiteBalance", 17));
    }
    /* Not on a color GR IV. */
    camera_monochrome = 0;
    assert(recipe_apply(header, 0) == RECIPE_UNSUPPORTED);
    /* A color type or a white balance does not apply on Monochrome. */
    camera_monochrome = 1;
    const u32 saved = w[RECIPE_NATIVE_WORD];
    w[RECIPE_NATIVE_WORD] = (saved & ~31U) | (TYPE_STANDARD + 1);
    assert(recipe_apply(header, 0) == RECIPE_UNSUPPORTED);
    w[RECIPE_NATIVE_WORD] = saved | (MODE_DAYLIGHT + 1) << NATIVE_MODE_SHIFT;
    assert(recipe_apply(header, 0) == RECIPE_UNSUPPORTED);
    w[RECIPE_NATIVE_WORD] = saved;
    /* The Solarized Effect and its type, where the build has it. */
    if (native_type_codes[solarized] != 0xff) {
        native_record_bytes[NATIVE_TYPE_FIELD] = native_type_codes[solarized];
        native_record_bytes[native_field(solarized, NATIVE_PARAMETER_SOLARIZATIONTYPE)] = 2;
        assert(recipe_save(header) == 1);
        w = recipe_at(header, 1);
        assert(NATIVE_TYPE(w[RECIPE_NATIVE_WORD]) == solarized + 1 && NATIVE_SOLARIZATION(w[RECIPE_EXTRA_WORD]) == 2);
        assert(recipe_matches(header, 1));
        native_record_bytes[native_field(solarized, NATIVE_PARAMETER_SOLARIZATIONTYPE)] = 0;
        assert(!recipe_matches(header, 1));
        native_call_count = 0;
        assert(recipe_apply(header, 1) == RECIPE_READY);
        assert(called("AfterReleaseStillImageControlNo0ParamMonochromeSolarizationSolarizationType=TYPE3"));
        /* A Solarization type on another type does not apply. */
        recipe_at(header, 0)[RECIPE_EXTRA_WORD] |= 1U << NATIVE_SOLARIZATION_SHIFT;
        assert(recipe_apply(header, 0) == RECIPE_UNSUPPORTED);
    }
}
#endif

/* Library recipe i of family `family` with the words w0 to w3 and hints. */
/* A hidden recipe stays out of ADJ, and its change saves the recipes. The
   current recipe is one that shows. */
static void hidden_recipes_leave_adj(void)
{
    reset();
    select_effect(0);
    for (u32 i = 0; i < 3; ++i) assert(recipe_save(header) == i);
    assert(recipe_shown_count(header) == 3 && recipe_current(header) == 0);
    u32 before = header[GRKIT_HOST_RECIPE_CHANGES / 4];
    recipe_show_in_adj(header, 0, 0);
    assert(header[GRKIT_HOST_RECIPE_CHANGES / 4] == before + 1);
    assert(recipe_at(header, 0)[0] & RECIPE_HIDDEN);
    assert(recipe_word_valid(recipe_at(header, 0)[0]) && recipe_count(header) == 3);
    assert(recipe_shown_count(header) == 2 && recipe_shown_at(header, 0) == 1);
    assert(recipe_shown_index(header, 0) == RECIPE_NONE && recipe_shown_index(header, 2) == 1);
    assert(recipe_current(header) == 1);
    recipe_show_in_adj(header, 0, 1);
    assert(recipe_shown_count(header) == 3 && recipe_current(header) == 0);
}

/* An apply and an undo save no recipe: the undo steps are not saved. */
static void undo_steps_are_not_saved(void)
{
    reset();
    select_effect(1);
    assert(recipe_save(header) == 0);
    select_effect(0);
    u32 before = header[GRKIT_HOST_RECIPE_CHANGES / 4];
    assert(recipe_apply(header, 0) == RECIPE_READY && recipe_undo_count(header) == 1);
    recipe_undo(header);
    assert(header[GRKIT_HOST_RECIPE_CHANGES / 4] == before && !recipe_undo_count(header));
    /* The undo steps are their own field, after the recipes. */
    assert(recipe_at(header, GRKIT_RECIPE_SLOTS) == header + GRKIT_HOST_RECIPE_UNDO / 4);
}

/* With the Glow module loaded, a save holds Bloom, Halation, and its
   color, apply sets them, and an undo step restores them. A recipe that
   holds Glow without the module applies the rest and reports it. */
static void glow_saves_applies_and_undoes(void)
{
    reset();
    load_glow(1);
    select_effect(1); set_grain(0, 0); set_glow(2, 3, 0);
    assert(recipe_save(header) == 0);
    assert(HELD(0, BLOOM) && VALUE(0, BLOOM) == 2);
    assert(HELD(0, HALATION) && VALUE(0, HALATION) == 3);
    assert(HELD(0, HALATION_COLOR) && VALUE(0, HALATION_COLOR) == 0);
    assert(recipe_matches(header, 0));
    /* Another Glow: no match, and apply sets the recipe's Glow. */
    set_glow(0, 1, 3);
    assert(!recipe_matches(header, 0));
    assert(recipe_apply(header, 0) == RECIPE_READY);
    assert(glow_value(0) == 2 && glow_value(1) == 3 && glow_value(2) == 0);
    assert(recipe_matches(header, 0));
    recipe_undo(header);
    assert(glow_value(0) == 0 && glow_value(1) == 1 && glow_value(2) == 3);

    /* A module without the color setting: the color is not held, and a
       recipe without it keeps the color of the camera. */
    reset();
    load_glow(0);
    set_glow(1, 1, 0);
    assert(recipe_save(header) == 0);
    assert(HELD(0, HALATION) && !HELD(0, HALATION_COLOR));

    /* Without the Glow module the picture has no Glow: a save holds Bloom
       and Halation at Off, and no color, which has no Off. A recipe with
       Glow applies the rest and reports the module setting that it left
       out. */
    reset();
    select_effect(2); set_grain(2, 0);
    assert(recipe_save(header) == 0);
    assert(recipe_at(header, 0)[RECIPE_MODULE_WORD] == (1U | 1U << recipe_setting_shifts[HALATION]));
    recipe_at(header, 0)[RECIPE_MODULE_WORD] = 1U | 3U << recipe_setting_shifts[HALATION];
    select_effect(0); set_grain(0, 0);
    assert(recipe_apply(header, 0) == (RECIPE_READY | RECIPE_SKIPPED_MODULE(0)));
    assert(header[GRKIT_HOST_ACTIVE / 4] == record_at(2) && header[GRKIT_HOST_GRAIN / 4] == 2);
    /* The Glow that does not run does not decide the match. */
    assert(recipe_matches(header, 0));
    /* Glow Off changes nothing without the module, so apply reports none. */
    recipe_at(header, 0)[RECIPE_MODULE_WORD] = 1U | 1U << recipe_setting_shifts[HALATION];
    assert(recipe_apply(header, 0) == RECIPE_READY);
}

/* With the Border module, a save holds the border, and the color of a
   border that is on. Apply sets them, and an undo step restores them. The
   border is not part of the look: it does not decide a match, a recipe
   without it keeps the border of the camera, and a save without the module
   holds none. A recipe with a border applies the rest without the module
   and reports it. */
static void border_saves_applies_and_does_not_match(void)
{
    reset();
    header[GRKIT_HOST_CAPABILITIES / 4] = RUNNING | BORDER_BITS;
    select_effect(1); set_border(2, 1);
    assert(recipe_save(header) == 0);
    assert(HELD(0, BORDER) && VALUE(0, BORDER) == 2);
    assert(HELD(0, BORDER_COLOR) && VALUE(0, BORDER_COLOR) == 1);
    /* Border Off: no color. */
    set_border(0, 1);
    assert(recipe_save(header) == 1);
    assert(HELD(1, BORDER) && VALUE(1, BORDER) == 0 && !HELD(1, BORDER_COLOR));
    /* Each matches whatever the border. */
    set_border(3, 0);
    assert(recipe_matches(header, 0) && recipe_matches(header, 1) && recipe_current(header) == 0);
    assert(recipe_apply(header, 0) == RECIPE_READY);
    assert(header[GRKIT_HOST_BORDER / 4] == 2 && header[GRKIT_HOST_BORDER_COLOR / 4] == 1);
    recipe_undo(header);
    assert(header[GRKIT_HOST_BORDER / 4] == 3 && header[GRKIT_HOST_BORDER_COLOR / 4] == 0);
    /* Off keeps the color for the next border. */
    assert(recipe_apply(header, 1) == RECIPE_READY);
    assert(header[GRKIT_HOST_BORDER / 4] == 0 && header[GRKIT_HOST_BORDER_COLOR / 4] == 0);
    /* A recipe without a border, as a preset, keeps the border. */
    set_border(1, 1);
    recipe_at(header, 0)[GRKIT_RECIPE_SETTINGS_WORD] = 0;
    assert(recipe_apply(header, 0) == RECIPE_READY);
    assert(header[GRKIT_HOST_BORDER / 4] == 1 && header[GRKIT_HOST_BORDER_COLOR / 4] == 1);

    /* Without the module, a save holds no border, a border that is on is
       left out and reported, and a border that is off changes nothing. */
    header[GRKIT_HOST_CAPABILITIES / 4] = RUNNING;
    assert(recipe_save(header) == 2);
    assert(!HELD(2, BORDER) && !HELD(2, BORDER_COLOR));
    assert(recipe_apply(header, 1) == RECIPE_READY);
    recipe_at(header, 1)[GRKIT_RECIPE_SETTINGS_WORD] |= 1U | 2U << 1;
    assert(recipe_apply(header, 1) == (RECIPE_READY | RECIPE_SKIPPED_MODULE(1)));
    assert(header[GRKIT_HOST_BORDER / 4] == 1 && header[GRKIT_HOST_BORDER_COLOR / 4] == 1);
}

/* Without Film Look, a save that reads a native type holds Film Look Off
   with it, as with Film Look: the type shows only then. Apply sets the
   type and keeps the effect, which Film Look does not change. */
static void native_type_without_film_look(void)
{
    reset();
    header[GRKIT_HOST_CAPABILITIES / 4] = GRKIT_CORE_RECIPE_BIT;
    set_native_look();
    select_effect(1);
    assert(recipe_save(header) == 0);
    volatile u32 *w = recipe_at(header, 0);
    assert(RECIPE_LOOK(w[0]) == RECIPE_LOOK_OFF && NATIVE_TYPE(w[RECIPE_NATIVE_WORD]) == TYPE_POSITIVE + 1);
    assert(recipe_matches(header, 0) && recipe_current(header) == 0);
    select_effect(2);
    native_record_bytes[NATIVE_TYPE_FIELD] = native_type_codes[TYPE_STANDARD];
    assert(!recipe_matches(header, 0) && recipe_current(header) == RECIPE_NONE);
    assert(recipe_apply(header, 0) == RECIPE_READY);
    assert(header[GRKIT_HOST_ACTIVE / 4] == record_at(2));
    assert(!strcmp(native_calls[0], "StillImageControlNo0=PositiveFilm"));
}

/* GR IV Monochrome hides the rows of the Halation color: a save does not
   hold it, and apply leaves it as it is. */
static void monochrome_holds_no_color_setting(void)
{
    reset();
    camera_monochrome = 1;
    load_glow(1);
    set_glow(1, 2, 3);
    assert(recipe_save(header) == 0);
    assert(HELD(0, BLOOM) && HELD(0, HALATION) && !HELD(0, HALATION_COLOR));
    set_glow(0, 0, 1);
    assert(recipe_apply(header, 0) == RECIPE_READY);
    assert(glow_value(0) == 1 && glow_value(1) == 2 && glow_value(2) == 1);
}

/* The camera line of this target as the Rust codec names it. */
static const char *family_name(void)
{
#ifdef NATIVE_FAMILY_GR3
    return "gr3";
#else
    return camera_monochrome ? "gr4Monochrome" : "gr4";
#endif
}
/* Save the current settings and print the words of the recipe for the
   Rust codec (grkit_core::recipes::Recipe::from_words), which the site
   reads the recipe file with: "save FAMILY W0 ... W5". */
static void print_save(void)
{
    assert(recipe_save(header) == 0);
    printf("save %s", family_name());
    for (u32 k = 0; k < GRKIT_RECIPE_WORDS; ++k) printf(" %u", recipe_at(header, 0)[k]);
    printf("\n");
}
/* The saves of the camera for the Rust codec: an effect with Fn, Grain,
   Glow, the border, and the native settings; a native look with and
   without Film Look; and GR IV Monochrome with Glow. */
static void print_saves(void)
{
    reset();
    header[GRKIT_HOST_CAPABILITIES / 4] = RUNNING | BORDER_BITS;
    load_glow(1);
    set_native_look();
    select_effect(1); set_fn(1, 0, 3); set_fn(1, 8, 7); set_grain(2, 3); set_glow(2, 3, 1); set_border(3, 1);
    print_save();
    reset();
    set_native_look();
    select_effect(TEST_COUNT);
    print_save();
    reset();
    header[GRKIT_HOST_CAPABILITIES / 4] = GRKIT_CORE_RECIPE_BIT | GRKIT_CORE_GRAIN_BIT;
    set_native_look();
    select_effect(1); set_grain(1, 0);
    print_save();
#ifdef FW_IMAGE_CONTROL_SET
    const u32 standard = NATIVE_FIRST_MONOCHROME;
    reset();
    header[GRKIT_HOST_CAPABILITIES / 4] = GRKIT_CORE_RECIPE_BIT | GRKIT_CORE_GRAIN_BIT;
    camera_monochrome = 1;
    settings_present = 1;
    load_glow(1);
    set_glow(1, 2, 3);
    native_record_bytes[NATIVE_TYPE_FIELD] = native_type_codes[standard];
    for (u32 p = 0; p < NATIVE_LEVELS; ++p)
        if (native_field(standard, p)) native_record_bytes[native_field(standard, p)] = 4;
    native_record_bytes[native_field(standard, NATIVE_PARAMETER_CONTRAST)] = 7;
    print_save();
#endif
}

/* The recipes of the Rust codec in the file $GRKIT_TEST_RECIPE_WORDS, one
   "FAMILY W0 ... W5" line each: each recipe of a camera line of this
   target is a recipe that the camera holds, and the camera has each of its
   native values. */
static void rust_recipes_apply(void)
{
    const char *path = getenv("GRKIT_TEST_RECIPE_WORDS");
    char family[32];
    u32 w[GRKIT_RECIPE_WORDS], checked = 0;
    if (!path) return;
    FILE *file = fopen(path, "r");
    assert(file);
    _Static_assert(GRKIT_RECIPE_WORDS == 6U, "a line has six words");
    while (fscanf(file, "%31s %u %u %u %u %u %u", family, &w[0], &w[1], &w[2], &w[3], &w[4], &w[5]) == 7) {
        reset();
#ifdef NATIVE_FAMILY_GR3
        if (strcmp(family, "gr3")) continue;
#else
        if (strcmp(family, "gr4") && strcmp(family, "gr4Monochrome")) continue;
        camera_monochrome = !strcmp(family, "gr4Monochrome");
#endif
        assert(recipe_word_valid(w[0]));
        assert(native_supported(w));
        ++checked;
    }
    fclose(file);
    assert(checked);
}

/* The float fields of the exposure settings in the settings record. */
static void set_float(u32 field, u32 bits) { memcpy(native_record_bytes + field, &bits, 4); }
#define F_THIRD 0x3eaaaaabU   /* 1/3 */
#define F_HALF 0x3f000000U    /* 1/2 */
#define F_ONE 0x3f800000U     /* 1 */
#define F_MINUS_TWO_THIRDS 0xbf2aaaabU
#define F_SV_125 0x40aaaaabU  /* 5 1/3: ISO 125 */
#define F_SV_6400 0x41300000U /* 11: ISO 6400 */
#define F_SV_12800 0x41400000U /* 12 */
static void exposure_camera(u32 ev, u32 upper, u32 lower, u32 metering, u32 nd)
{
    set_float(NATIVE_EXPOSURE_FIELD, ev);
    set_float(NATIVE_ISO_UPPER_FIELD, upper);
    set_float(NATIVE_ISO_LOWER_FIELD, lower);
    native_record_bytes[NATIVE_METERING_FIELD] = (u8)metering;
    native_record_bytes[NATIVE_ND_FIELD] = (u8)nd;
}

/* A save holds the exposure compensation, the Auto ISO upper limit,
   metering, and the ND filter of the camera. Apply writes them as text,
   rounded to the steps of the camera and not below its lower ISO limit,
   and undo writes the ones before. They do not decide the match. */
static void exposure_saves_applies_and_fits_the_camera(void)
{
    reset();
    header[GRKIT_HOST_CAPABILITIES / 4] = GRKIT_CORE_RECIPE_BIT;
    settings_present = 1;
    native_record_bytes[NATIVE_TYPE_FIELD] = 0xee;
    native_record_bytes[NATIVE_MODE_FIELD] = 0xee;
    set_float(NATIVE_EXPOSURE_STEP_FIELD, F_THIRD);
    set_float(NATIVE_ISO_STEP_FIELD, F_THIRD);
    exposure_camera(F_THIRD, F_SV_6400, F_SV_125, 2, 1);
    assert(recipe_save(header) == 0);
    volatile u32 *w = recipe_at(header, 0);
    /* The recipe holds +1/3, ISO 6400, Spot, and ND On. */
    assert((w[RECIPE_MODULE_WORD] & NATIVE_EXPOSURE_BITS) ==
           ((31U + 2U) << NATIVE_EV_SHIFT | (18U + 1U) << NATIVE_ISO_SHIFT | 3U << NATIVE_METERING_SHIFT |
            2U << NATIVE_ND_SHIFT));
    /* The camera changes: -2/3, ISO 125, Multi, ND Off. The look is the
       same, so the recipe still matches. */
    exposure_camera(F_MINUS_TWO_THIRDS, F_SV_125, F_SV_125, 0, 2);
    assert(recipe_matches(header, 0));
    native_call_count = 0;
    assert(recipe_apply(header, 0) == RECIPE_READY);
    assert(called("CommonXv=0.3333333333"));
    assert(called("StillSvAutoUpper=11.0000000000"));
    assert(called("StillAeMode=Spot") && called("StillNdFilter=On"));
    /* Undo writes the settings before. */
    native_call_count = 0;
    recipe_undo(header);
    assert(called("CommonXv=-0.6666666666"));
    assert(called("StillSvAutoUpper=5.3333333333"));
    assert(called("StillAeMode=Multi") && called("StillNdFilter=Off"));

    /* An exposure step of 1/2: +1/3 becomes +1/2, and the camera at +1/2
       matches. */
    set_float(NATIVE_EXPOSURE_STEP_FIELD, F_HALF);
    native_call_count = 0;
    assert(recipe_apply(header, 0) == RECIPE_READY);
    assert(called("CommonXv=0.5000000000"));
    exposure_camera(F_HALF, F_SV_6400, F_SV_125, 2, 1);

    /* A lower limit above the recipe's upper limit: the upper limit takes
       the lower one. An ISO step of 1 stop rounds to whole stops. */
    set_float(NATIVE_ISO_LOWER_FIELD, F_SV_12800);
    native_call_count = 0;
    assert(recipe_apply(header, 0) == RECIPE_READY);
    assert(called("StillSvAutoUpper=12.0000000000"));
    set_float(NATIVE_ISO_LOWER_FIELD, F_SV_125);
    set_float(NATIVE_ISO_STEP_FIELD, F_ONE);
    w[RECIPE_MODULE_WORD] = (w[RECIPE_MODULE_WORD] & ~(63U << NATIVE_ISO_SHIFT)) | 17U << NATIVE_ISO_SHIFT;
    native_call_count = 0;
    assert(recipe_apply(header, 0) == RECIPE_READY);
    /* ISO 5000 (third stop 16) rounds to ISO 3200 (15), Sv 10. */
    assert(called("StillSvAutoUpper=10.0000000000"));

    /* Each value out of the tables makes the recipe unsupported. */
    w[RECIPE_MODULE_WORD] = (w[RECIPE_MODULE_WORD] & ~(63U << NATIVE_EV_SHIFT)) | 62U << NATIVE_EV_SHIFT;
    assert(recipe_apply(header, 0) == RECIPE_UNSUPPORTED);
    w[RECIPE_MODULE_WORD] = (w[RECIPE_MODULE_WORD] & ~(63U << NATIVE_EV_SHIFT)) | 31U << NATIVE_EV_SHIFT;
    w[RECIPE_MODULE_WORD] = (w[RECIPE_MODULE_WORD] & ~(63U << NATIVE_ISO_SHIFT)) | (NATIVE_ISO_STEPS + 1U) << NATIVE_ISO_SHIFT;
    assert(recipe_apply(header, 0) == RECIPE_UNSUPPORTED);
    w[RECIPE_MODULE_WORD] = (w[RECIPE_MODULE_WORD] & ~(63U << NATIVE_ISO_SHIFT)) | 1U << NATIVE_ISO_SHIFT;
    w[RECIPE_MODULE_WORD] |= 7U << NATIVE_METERING_SHIFT;
    assert(recipe_apply(header, 0) == RECIPE_UNSUPPORTED);
}

/* The rows of the details of words `w` in `mode`, as ASCII, and their
   number. */
static u16 detail_rows[DETAIL_ROWS_MAX * DETAIL_UNITS];
static char detail_ascii[DETAIL_ROWS_MAX][DETAIL_UNITS];
static u32 details_of(const u32 *w, u32 mode)
{
    memset(detail_rows, 0xff, sizeof detail_rows);
    u32 n = recipe_details(header, w, mode, detail_rows);
    assert(n >= 1 && n <= DETAIL_ROWS_MAX);
    for (u32 r = 0; r < n; ++r) {
        u32 i = 0;
        for (; detail_rows[r * DETAIL_UNITS + i]; ++i) {
            assert(i + 1 < DETAIL_UNITS && detail_rows[r * DETAIL_UNITS + i] < 0x80);
            detail_ascii[r][i] = (char)detail_rows[r * DETAIL_UNITS + i];
        }
        /* Each row has text, at most DETAIL_WRAP characters. */
        assert(i && i <= DETAIL_WRAP);
        detail_ascii[r][i] = 0;
    }
    return n;
}
static void set_name(u32 i, const char *name)
{
    u32 n = 0;
    for (; name[n]; ++n) HALF(record_at(i), GRKIT_RECORD_NAME + 2 * n) = (u16)name[n];
    HALF(record_at(i), GRKIT_RECORD_NAME + 2 * n) = 0;
}
/* The details name each setting that a recipe holds, in groups, with the
   same English abbreviations in each menu language. The panel leaves out
   Grain, Glow, and Border, which it shows as their status icons. */
static void details_name_what_the_recipe_holds(void)
{
    reset();
    set_name(1, "Portra 400");
    u32 w[GRKIT_RECIPE_WORDS] = {0};
    w[0] = RECIPE_USED | RECIPE_LOOK_EFFECT | code_of(1) << RECIPE_CODE_SHIFT | RECIPE_GRAIN |
           2U << RECIPE_GRAIN_LEVEL_SHIFT | 3U << RECIPE_GRAIN_SIZE_SHIFT;
    /* Fn: saturation +1 (digit 1), contrast -1 (digit 8). */
    w[RECIPE_FN_WORD] = 1U + 8U * 9U * 9U * 9U;
    /* Daylight, A2 and G1. */
    w[RECIPE_NATIVE_WORD] = (MODE_DAYLIGHT + 1U) << NATIVE_MODE_SHIFT | 16U << NATIVE_BA_SHIFT | 13U << NATIVE_GM_SHIFT;
    /* Highlight Auto, shadow Off, peripheral Off, NR Low: stored + 1. */
    w[RECIPE_EXTRA_WORD] = 1U << NATIVE_CORRECTION_SHIFT | 5U << (NATIVE_CORRECTION_SHIFT + 3U) |
                           1U << (NATIVE_CORRECTION_SHIFT + 6U) | 2U << (NATIVE_CORRECTION_SHIFT + 9U);
    /* +4/6 EV, ISO 3200 (third stop 15), Spot, ND On. */
    w[RECIPE_MODULE_WORD] = 35U << NATIVE_EV_SHIFT | 16U << NATIVE_ISO_SHIFT | 3U << NATIVE_METERING_SHIFT |
                            2U << NATIVE_ND_SHIFT;
    recipe_setting_hold(w, BLOOM, 1);
    recipe_setting_hold(w, HALATION, 2);
    recipe_setting_hold(w, HALATION_COLOR, 0);
    recipe_setting_hold(w, BORDER, 1);
    recipe_setting_hold(w, BORDER_COLOR, 1);
    static const char *const menu[] = {
        "Portra 400", "Sat +1  Con -1", "Grain Medium Large", "Bloom Weak", "Halation Medium Red",
        "Border Thin Black", "WB Daylight A2 G1", "HL Auto  SH Off", "Periph Off  NR Low",
        "EV +0.7  ISO max 3200", "Meter Spot  ND On"};
    u32 n = details_of(w, DETAIL_MENU);
    assert(n == sizeof menu / sizeof menu[0]);
    for (u32 r = 0; r < n; ++r) assert(!strcmp(detail_ascii[r], menu[r]));
    static const char *const panel[] = {"Portra 400", "Sat +1  Con -1", "WB Daylight A2 G1", "HL Auto  SH Off",
                                        "Periph Off  NR Low", "EV +0.7  ISO max 3200", "Meter Spot  ND On"};
    n = details_of(w, DETAIL_PANEL);
    assert(n == sizeof panel / sizeof panel[0]);
    for (u32 r = 0; r < n; ++r) assert(!strcmp(detail_ascii[r], panel[r]));

    /* The name of an effect keeps its own characters. */
    HALF(record_at(1), GRKIT_RECORD_NAME + 2) = 0x4e2d;
    assert(recipe_details(header, w, DETAIL_PANEL, detail_rows) == 7);
    assert(detail_rows[0] == 'P' && detail_rows[1] == 0x4e2d && detail_rows[2] == 'r');
    /* A row holds exactly DETAIL_WRAP characters, and the levels reach +4
       and -4: digits 4 and 5. */
    u32 edge[GRKIT_RECIPE_WORDS] = {0};
    edge[0] = RECIPE_USED | RECIPE_LOOK_EFFECT | code_of(0) << RECIPE_CODE_SHIFT;
    set_name(0, "Edge");
    edge[RECIPE_FN_WORD] = 1U + 1U * 9U + 1U * 531441U + 4U * 4782969U + 5U * 43046721U;
    assert(details_of(edge, DETAIL_MENU) == 3 && !strcmp(detail_ascii[1], "Sat +1  Hue +1  Sharp +1"));
    assert(strlen(detail_ascii[1]) == DETAIL_WRAP && !strcmp(detail_ascii[2], "Shade +4  Clarity -4"));
    /* An effect that is not installed. */
    w[0] = (w[0] & ~(RECIPE_CODE_MASK << RECIPE_CODE_SHIFT)) | ((code_of(1) + 1U) & RECIPE_CODE_MASK) << RECIPE_CODE_SHIFT;
    details_of(w, DETAIL_PANEL);
    assert(!strcmp(detail_ascii[0], "Effect missing"));

    /* A native type: its English name, only its own levels, its tone; the
       color temperature; a negative and a zero compensation. */
    memset(w, 0, sizeof w);
    w[0] = RECIPE_USED | RECIPE_LOOK_OFF;
    u32 own = NATIVE_LEVELS, other = NATIVE_LEVELS;
    for (u32 p = 0; p < NATIVE_LEVELS; ++p) {
        if (native_field(TYPE_MONOTONE, p) && own == NATIVE_LEVELS) own = p;
        if (!native_field(TYPE_MONOTONE, p) && other == NATIVE_LEVELS) other = p;
    }
    assert(own < NATIVE_LEVELS && other < NATIVE_LEVELS);
    u32 digits = 0;
    for (u32 p = NATIVE_LEVELS; p--;) digits = digits * 9U + (p == own ? 2U : p == other ? 3U : 0U);
    w[RECIPE_FN_WORD] = digits;
    w[RECIPE_NATIVE_WORD] = (TYPE_MONOTONE + 1U) << NATIVE_TYPE_SHIFT | 1U << NATIVE_TONE_SHIFT |
                            (MODE_TEMPERATURE + 1U) << NATIVE_MODE_SHIFT | 14U << NATIVE_BA_SHIFT | 14U << NATIVE_GM_SHIFT;
    w[RECIPE_EXTRA_WORD] = 270U;
    w[RECIPE_MODULE_WORD] = 29U << NATIVE_EV_SHIFT;
    n = details_of(w, DETAIL_MENU);
    char level[DETAIL_UNITS];
    snprintf(level, sizeof level, "%s +2", detail_levels[own]);
    assert(n == 5 && !strcmp(detail_ascii[0], "Monotone") && !strcmp(detail_ascii[1], level));
    assert(!strcmp(detail_ascii[2], "Tone Sepia") && !strcmp(detail_ascii[3], "WB 5200K") && !strcmp(detail_ascii[4], "EV -0.3"));
    w[RECIPE_MODULE_WORD] = 31U << NATIVE_EV_SHIFT;
    n = details_of(w, DETAIL_MENU);
    assert(!strcmp(detail_ascii[n - 1], "EV 0.0"));

    /* A recipe that keeps each setting, and Film Look Off. */
    memset(w, 0, sizeof w);
    w[0] = RECIPE_USED;
    assert(details_of(w, DETAIL_MENU) == 1 && !strcmp(detail_ascii[0], "No settings"));
    w[0] |= RECIPE_LOOK_OFF;
    assert(details_of(w, DETAIL_PANEL) == 1 && !strcmp(detail_ascii[0], "Film Look Off"));
}

int main(void)
{
    save_numbers_each_look();
    save_holds_only_running_capabilities();
    save_stops_when_full();
    recipes_that_are_off_stay_for_the_site();
    each_recipe_keeps_its_id();
    apply_sets_the_look_and_stores_an_undo_step();
    undo_keeps_the_newest_steps();
    missing_effect_changes_nothing();
    undo_skips_a_missing_effect();
    delete_and_damaged_words();
    empty_recipe_matches_nothing();
    native_looks_save_and_apply();
    effects_hold_no_type();
    custom_sets_hold_their_type();
    saves_hold_modules_that_do_not_run_at_off();
    unsupported_native_values_change_nothing();
    corrections_save_apply_and_unset();
    native_grain_saves_and_applies();
    grain_needs_the_grain_module();
    hidden_recipes_leave_adj();
    undo_steps_are_not_saved();
    glow_saves_applies_and_undoes();
    border_saves_applies_and_does_not_match();
    exposure_saves_applies_and_fits_the_camera();
#ifdef FW_IMAGE_CONTROL_SET
    monochrome_types();
#endif
    native_type_without_film_look();
    monochrome_holds_no_color_setting();
    rust_recipes_apply();
    details_name_what_the_recipe_holds();
    print_saves();
    return 0;
}
