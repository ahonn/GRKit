#include <assert.h>
#include <stdio.h>
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
static u8 library_bytes[2048] __attribute__((aligned(4)));
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

/* The capabilities as the menu sets them (runtime/setting_store.h): the
   effect is the active record, Grain and its size are header words. */
static u32 stores;
static u32 setting_value(volatile u32 *h, u32 handle)
{
    if (handle == GRKIT_CORE_TONE) return tone_value(h);
    if (handle == GRKIT_CORE_GRAIN) return h[GRKIT_HOST_GRAIN / 4];
    if (handle == GRKIT_CORE_GRAIN_SIZE) return h[GRKIT_HOST_GRAIN_SIZE / 4];
    assert(0);
    return 0;
}
static void setting_store(volatile u32 *h, u32 handle, u32 value)
{
    ++stores;
    if (handle == GRKIT_CORE_TONE) h[GRKIT_HOST_ACTIVE / 4] = tone_record(h, value);
    else if (handle == GRKIT_CORE_GRAIN) h[GRKIT_HOST_GRAIN / 4] = value;
    else if (handle == GRKIT_CORE_GRAIN_SIZE) h[GRKIT_HOST_GRAIN_SIZE / 4] = value;
    else assert(0);
}
#include "native_look.h"
/* The library of the services block (runtime/services.h). */
#include "services.h"
_Static_assert(LIBRARY_SIZE <= sizeof(library_bytes), "the test library holds a full library");
#define RECIPE_LIBRARY(h) ((void)(h), (volatile u32 *)library_bytes)
#include "recipes.h"

#define RUNNING (GRKIT_CORE_TONE_BIT | GRKIT_CORE_GRAIN_BIT | GRKIT_CORE_GRAIN_SIZE_BIT | GRKIT_CORE_RECIPE_BIT)

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
static void reset(void)
{
    memset((void *)header, 0, sizeof header);
    memset(records, 0, sizeof records);
    memset(native_record_bytes, 0, sizeof native_record_bytes);
    native_call_count = 0;
    settings_present = 0;
    camera_monochrome = 0;
    memset(library_bytes, 0, sizeof library_bytes);
    settings_object[1] = TEST_NATIVE;
    header[GRKIT_HOST_COUNT / 4] = TEST_COUNT;
    header[GRKIT_HOST_RECORDS / 4] = TEST_RECORDS;
    header[GRKIT_HOST_CAPABILITIES / 4] = RUNNING;
    /* Codes with the top bit, as the installer writes them. */
    for (u32 i = 0; i < TEST_COUNT; ++i) HALF(record_at(i), GRKIT_RECORD_CODE) = (u16)(0x8000U | (0x100U + i));
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
static u32 changes(void) { return header[GRKIT_HOST_STATE_CHANGES / 4]; }

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
    assert(RECIPE_LOOK(w) == RECIPE_LOOK_EFFECT && RECIPE_CODE(w) == 0x101U);
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

/* The camera holds GRKIT_RECIPE_SLOTS recipes. */
static void save_stops_when_full(void)
{
    reset();
    select_effect(0);
    for (u32 i = 0; i < GRKIT_RECIPE_SLOTS; ++i) assert(recipe_save(header) == i);
    u32 before = changes();
    assert(recipe_save(header) == RECIPE_FULL);
    assert(changes() == before && recipe_count(header) == GRKIT_RECIPE_SLOTS);
    /* The numbers stop at the largest one. */
    assert(RECIPE_NUMBER(recipe_at(header, GRKIT_RECIPE_SLOTS - 1)[0]) == GRKIT_RECIPE_SLOTS);
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
    assert(RECIPE_LOOK(step) == RECIPE_LOOK_EFFECT && RECIPE_CODE(step) == 0x100U);
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
    HALF(record_at(2), GRKIT_RECORD_CODE) = 0x8222U;
    select_effect(0); set_grain(0, 0);
    u32 before = changes();
    assert(recipe_apply(header, 0) == RECIPE_MISSING);
    assert(header[GRKIT_HOST_ACTIVE / 4] == record_at(0) && header[GRKIT_HOST_GRAIN / 4] == 0);
    assert(changes() == before && recipe_undo_count(header) == 0 && !stores);
    HALF(record_at(2), GRKIT_RECORD_CODE) = 0x8102U;
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
    HALF(record_at(2), GRKIT_RECORD_CODE) = 0x8222U;
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
    recipe_at(header, 1)[0] = 0x10000000U | RECIPE_USED;
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

/* An effect replaces the native type: the recipe holds the white balance
   only, and a Custom set holds no type. */
static void effects_and_custom_sets_hold_no_type(void)
{
    reset();
    set_native_look();
    select_effect(1);
    assert(recipe_save(header) == 0);
    assert(NATIVE_TYPE(recipe_at(header, 0)[RECIPE_NATIVE_WORD]) == 0);
    assert(NATIVE_MODE(recipe_at(header, 0)[RECIPE_NATIVE_WORD]) == MODE_DAYLIGHT + 1);
    select_effect(TEST_COUNT);
    native_record_bytes[NATIVE_ACTIVE_SET_FIELD] = 1;
    assert(recipe_save(header) == 1);
    assert(NATIVE_TYPE(recipe_at(header, 1)[RECIPE_NATIVE_WORD]) == 0);
    /* A write of a type selects the main set first. */
    native_record_bytes[NATIVE_ACTIVE_SET_FIELD] = 0;
    assert(recipe_save(header) == 2);
    native_record_bytes[NATIVE_ACTIVE_SET_FIELD] = 2;
    native_call_count = 0;
    assert(recipe_apply(header, 2) == RECIPE_READY);
    assert(!strcmp(native_calls[0], "StillActiveImageControlSet=No0"));
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

/* Without a running Grain module, apply gives Grain levels 1 to 3 to the
   native grain of the native type. Extra Strong, a GRKit effect, and a
   fixed size on GR III have no match: apply sets the rest and reports
   RECIPE_GRAIN_SKIPPED. */
static void grain_falls_back_to_native_grain(void)
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
    native_call_count = 0;
    assert(recipe_apply(header, 0) == RECIPE_READY);
#ifdef NATIVE_FAMILY_GR3
    assert(called("StillImageControlNo0ParamMonotoneGRParticle=Mid"));
#else
    /* Size Auto keeps the size of the look: Small. */
    assert(called("StillImageControlNo0ParamMonotoneGRParticle=On"));
    assert(called("StillImageControlNo0ParamMonotoneGRParticleSize=Small"));
    assert(called("StillImageControlNo0ParamMonotoneGRParticleStrength=Mid"));
#endif
    /* The camera now has that native grain, so the recipe matches. */
    set_particle(TYPE_MONOTONE, 2, 0);
    assert(recipe_matches(header, 0));
    set_particle(TYPE_MONOTONE, 0, 0);
    assert(!recipe_matches(header, 0));

    /* A fixed size: GR IV has it, GR III does not. */
    w[0] = (w[0] & ~(3U << RECIPE_GRAIN_SIZE_SHIFT)) | 3U << RECIPE_GRAIN_SIZE_SHIFT;
    native_call_count = 0;
#ifdef NATIVE_FAMILY_GR3
    assert(recipe_apply(header, 0) == RECIPE_GRAIN_SKIPPED);
    assert(called("StillImageControlNo0ParamMonotoneGRParticle=Off"));
#else
    assert(recipe_apply(header, 0) == RECIPE_READY);
    assert(called("StillImageControlNo0ParamMonotoneGRParticleSize=Large"));
#endif

    /* Extra Strong: skipped, and the rest applies. */
    w[0] = (w[0] & ~(7U << RECIPE_GRAIN_LEVEL_SHIFT | 3U << RECIPE_GRAIN_SIZE_SHIFT)) | 4U << RECIPE_GRAIN_LEVEL_SHIFT;
    native_call_count = 0;
    assert(recipe_apply(header, 0) == RECIPE_GRAIN_SKIPPED);
    assert(called("StillImageControlNo0=MonotoneGR"));
    assert(called("StillImageControlNo0ParamMonotoneGRParticle=Off"));

    /* Off keeps the native grain of the look, so nothing is skipped. */
    w[0] &= ~(7U << RECIPE_GRAIN_LEVEL_SHIFT);
    assert(recipe_apply(header, 0) == RECIPE_READY);

    /* A GRKit effect has no native type. */
    select_effect(1);
    set_grain(1, 0);
    header[GRKIT_HOST_CAPABILITIES / 4] = RUNNING;
    assert(recipe_save(header) == 1);
    header[GRKIT_HOST_CAPABILITIES / 4] = GRKIT_CORE_RECIPE_BIT | GRKIT_CORE_TONE_BIT;
    assert(recipe_apply(header, 1) == RECIPE_GRAIN_SKIPPED);
    assert(header[GRKIT_HOST_ACTIVE / 4] == record_at(1));

    /* With the Grain module, CORE.GRAIN takes the Grain, and the native
       grain stays that of the look. */
    header[GRKIT_HOST_CAPABILITIES / 4] = RUNNING;
    select_effect(TEST_COUNT);
    set_grain(0, 0);
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
static void set_library(u32 i, const char *name, u32 family, const u32 *w, u32 hints)
{
    u8 *entry = library_bytes + LIBRARY_ENTRIES + i * LIBRARY_ENTRY_SIZE;
    for (u32 n = 0; name[n]; ++n) ((u16 *)(entry + LIBRARY_NAME))[n] = (u16)name[n];
    entry[LIBRARY_FAMILY] = (u8)family;
    for (u32 k = 0; k < GRKIT_RECIPE_WORDS; ++k) ((u32 *)(entry + LIBRARY_WORDS))[k] = w[k];
    *(u32 *)(entry + LIBRARY_HINTS) = hints;
    u32 *count = (u32 *)(library_bytes + LIBRARY_COUNT);
    if (*count <= i) *count = i + 1;
}

/* The library recipes come before the camera recipes. A recipe of another
   camera line does not apply and matches nothing. A save takes the next
   camera slot, and its number counts only the camera recipes. */
static void library_recipes_come_first(void)
{
    reset();
    select_effect(0);
#ifdef NATIVE_FAMILY_GR3
    const u32 own = 1, other = 2;
#else
    const u32 own = 2, other = 1;
#endif
    const u32 effect[4] = {RECIPE_USED | RECIPE_LOOK_EFFECT | 0x102U << RECIPE_CODE_SHIFT, 2U, 0, 0};
    const u32 off[4] = {RECIPE_USED | RECIPE_LOOK_OFF, 0, 0, 0};
    set_library(0, "Mine", own, effect, 0x0000c8c3U);
    set_library(1, "Other", other, off, 0);
    assert(recipe_library_count(header) == 2 && recipe_entries(header) == 2);
    assert(recipe_save(header) == 0);
    assert(recipe_entries(header) == 3 && !recipe_in_library(header, 2));
    /* The camera recipe holds effect 0, entry 2. */
    assert(recipe_current(header) == 2);
    assert(recipe_entry_reason(header, 1) == RECIPE_OTHER_CAMERA);
    assert(recipe_apply(header, 1) == RECIPE_OTHER_CAMERA && !recipe_undo_count(header));
    assert(!recipe_matches(header, 1));
    assert(recipe_hints(header, 0) == 0x0000c8c3U && recipe_hints(header, 2) == 0);
    assert(recipe_apply(header, 0) == RECIPE_READY);
    assert(header[GRKIT_HOST_ACTIVE / 4] == record_at(2) && fn_of(2, 0) == 2);
    assert(recipe_current(header) == 0 && recipe_matches(header, 0));
    assert(recipe_undo_count(header) == 1);
    /* Undo restores effect 0. */
    recipe_undo(header);
    assert(header[GRKIT_HOST_ACTIVE / 4] == record_at(0));
    /* A save of effect 2 is the first of its look on the camera: no number. */
    select_effect(2);
    assert(recipe_save(header) == 1 && RECIPE_NUMBER(recipe_at(header, 1)[0]) == 0);
    /* An entry after the list. */
    assert(recipe_apply(header, 9) == RECIPE_MISSING);
}

int main(void)
{
    save_numbers_each_look();
    save_holds_only_running_capabilities();
    save_stops_when_full();
    apply_sets_the_look_and_stores_an_undo_step();
    undo_keeps_the_newest_steps();
    missing_effect_changes_nothing();
    undo_skips_a_missing_effect();
    delete_and_damaged_words();
    empty_recipe_matches_nothing();
    native_looks_save_and_apply();
    effects_and_custom_sets_hold_no_type();
    unsupported_native_values_change_nothing();
    corrections_save_apply_and_unset();
    native_grain_saves_and_applies();
    grain_falls_back_to_native_grain();
    library_recipes_come_first();
#ifdef FW_IMAGE_CONTROL_SET
    monochrome_types();
#endif
    return 0;
}
