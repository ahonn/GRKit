#ifndef GRKIT_INSTALL_RECIPES_H
#define GRKIT_INSTALL_RECIPES_H
/* A part of the installer: the recipes (runtime/recipes.h). installer.c
   includes it once, after the helpers, the state parser, and the effects.

   - The recipe file (GRKIT/RECIPES.TXT, or RECIPES.TMP that a save cut by
     the power left) holds the recipes, their names and IDs, and the hash of
     the last package library that INSTALL imported
     (grkit_core::runtime::recipe_key). INSTALL generates an ID
     (runtime/recipe_id.h) for each recipe without one, such as an added
     recipe.
     INSTALL restores them into the header, also without a module that uses
     CORE.RECIPE, so the loader saves them again. The undo steps are not
     saved.
   - The recipes that are on come first, at most GRKIT_RECIPE_ON_MAX
     (runtime/recipes.h). INSTALL keeps this order: it moves the recipes
     that are on before those that are off, each in their order, and turns
     off each recipe that is on past GRKIT_RECIPE_ON_MAX. Then the loader
     saves the file.
   - A package holds no file of the data folder, so it holds a recipe library
     (grkit_core::recipes, RECIPES.BIN) in the package folder. INSTALL adds
     the usable recipes of the camera line that the recipes do not have yet
     (the family of a record: GR III, GR IV, or GR IV Monochrome, which has
     its own types and no white balance),
     up to GRKIT_RECIPE_SLOTS, once for each library: it keeps the hash of
     the file, and sets the count of recipe changes so that the loader saves
     the file. An added recipe keeps its on or off; one that is on goes after
     the recipes that are on. A recipe that the list holds takes the on or
     off of the library, because the site turns recipes on and off. A recipe that a person deleted on the camera does not come
     back from the same package. A record is usable if its name is at most
     GRKIT_RCP_NAME_UNITS printable ASCII characters (none for the automatic
     name, as in the recipe file), its family is known,
     its first word is that of a recipe, and each word stays below 2^29. The
     core checks the values for the camera when a person applies the recipe.
   - REPORT.TXT tells the count, the import, and each recipe that cannot
     apply in full (check_recipes). The installer deletes no file.
   - The file of the user modes of the mode dial (GRKIT/USERMODE.TXT, or
     USERMODE.TMP) holds the snapshot of each user mode, what each user mode
     does, and whether GRKit is off at P/Av/Tv/M (runtime/mode_dial.h,
     grkit_core::runtime::dial_set_key). Only the camera writes it. INSTALL
     restores them into the header as the file has them
     (install_dial_sets); a set with more pairs than the header holds is
     damaged and stays out. The file also holds a copy of the switch, Enable
     at Start, and Distance Unit (runtime/manage.h: manage_copy), which
     decides them before the main state file (read_manage_copy). */
#include "recipe_id.h"
static const char recipes_state_path[] = RECIPES_STATE_PATH, recipes_state_tmp_path[] = RECIPES_STATE_TMP_PATH;
static const char dial_sets_path[] = DIAL_SETS_PATH, dial_sets_tmp_path[] = DIAL_SETS_TMP_PATH;
/* The text of the recipe file, then that of the file of the sets. */
static u8 recipes_text[STATE_TEXT_MAX];
static const char recipes_inbox_path[] = RECIPES_INBOX_PATH;
static u8 recipes_inbox[GRKIT_RCP_HEADER_SIZE + GRKIT_RCP_MAX * GRKIT_RCP_RECORD_SIZE];
static struct state saved_recipes;
/* The result for REPORT.TXT: the recipes after INSTALL, and the import of a
   new package library: RECIPES_IMPORT_NONE without one. */
#define RECIPES_IMPORT_NONE 0U
#define RECIPES_IMPORT_DAMAGED 1U
#define RECIPES_IMPORT_READ 2U
static u32 recipes_count, recipes_on, recipes_import, recipes_added, recipes_full, recipes_other, recipes_same,
    recipes_bad, recipes_turned_off, recipes_switched;
/* The recipes while INSTALL reads them, in the order of the file, then the
   added ones. Each part of a recipe that the header keeps by slot is here,
   so a recipe moves as one. */
struct install_recipe {
    u32 words[GRKIT_RECIPE_WORDS], name[GRKIT_RECIPE_NAME_WORDS], id;
};
static struct install_recipe recipes_list[GRKIT_RECIPE_SLOTS];

/* The installer reads the recipe words by the layout of runtime/abi.h
   (GRKIT_RECIPE_*, grkit_core::recipes::layout): the look, the Grain, the
   effect code, the number, and the used and hidden bits of word 0, and the
   module settings and the off bit of word 4. */
#define RECIPE_WORD_NUMBER (GRKIT_RECIPE_NUMBER_MAX << GRKIT_RECIPE_NUMBER_SHIFT)
static u32 recipe_slot_word(u32 base, u32 slot, u32 w)
{
    return base + GRKIT_HOST_RECIPES + 4U * (slot * GRKIT_RECIPE_WORDS + w);
}
static u32 recipe_name_word(u32 base, u32 slot, u32 w)
{
    return base + GRKIT_HOST_RECIPE_NAMES + 4U * (slot * GRKIT_RECIPE_NAME_WORDS + w);
}
static u32 recipe_id_word(u32 base, u32 slot) { return base + GRKIT_HOST_RECIPE_IDS + 4U * slot; }
/* Whether word 0 is that of a recipe. */
static int recipe_first_word(u32 w)
{
    return (w & GRKIT_RECIPE_USED) && !(w >> GRKIT_RECIPE_VALUE_BITS) &&
           (w & GRKIT_RECIPE_LOOK_MASK) <= GRKIT_RECIPE_LOOK_EFFECT &&
           (w >> GRKIT_RECIPE_GRAIN_LEVEL_SHIFT & GRKIT_RECIPE_GRAIN_LEVEL_MASK) < GRKIT_CORE_GRAIN_VALUES;
}
/* The CRC-32 of a library file, which reads the CRC field as zero. */
static u32 recipes_crc(const u8 *data, u32 size)
{
    u32 crc = 0xffffffffU;
    for (u32 i = 0; i < size; ++i) {
        u8 byte = i >= GRKIT_RCP_CRC && i < GRKIT_RCP_CRC + 4U ? 0 : data[i];
        crc ^= byte;
        for (u32 bit = 0; bit < 8; ++bit) crc = crc >> 1 ^ (0xedb88320U & (0U - (crc & 1)));
    }
    return ~crc;
}
/* Whether the `size` bytes are a whole library file: the magic, the size
   of its count of records, and its CRC. */
static int recipes_whole(const u8 *file, u32 size)
{
    u32 count;
    if (size < GRKIT_RCP_HEADER_SIZE || !equal(file + GRKIT_RCP_MAGIC, (const u8 *)"GRKR0001", 8)) return 0;
    count = get32(file + GRKIT_RCP_COUNT);
    return count <= GRKIT_RCP_MAX && get32(file + GRKIT_RCP_BYTES) == size &&
           size == GRKIT_RCP_HEADER_SIZE + count * GRKIT_RCP_RECORD_SIZE &&
           get32(file + GRKIT_RCP_CRC) == recipes_crc(file, size);
}
/* Whether a record is usable. */
static int recipe_record_usable(const u8 *record)
{
    u32 ended = 0;
    for (u32 i = 0; i < GRKIT_RCP_NAME_UNITS; ++i) {
        u32 unit = record[GRKIT_RCP_RECORD_NAME + 2 * i] | record[GRKIT_RCP_RECORD_NAME + 2 * i + 1] << 8;
        if (!unit) { ended = 1; continue; }
        if (ended || unit < 0x20 || unit > 0x7e) return 0;
    }
    u32 family = record[GRKIT_RCP_RECORD_FAMILY];
    if (family < GRKIT_RCP_FAMILY_GR3 || family > GRKIT_RCP_FAMILY_GR4_MONOCHROME ||
        !recipe_first_word(get32(record + GRKIT_RCP_RECORD_WORDS)))
        return 0;
    for (u32 w = 0; w < GRKIT_RECIPE_WORDS; ++w)
        if (get32(record + GRKIT_RCP_RECORD_WORDS + 4 * w) >> GRKIT_RECIPE_VALUE_BITS) return 0;
    return 1;
}
/* The name words of a record: four 7-bit characters in each word. */
static void recipe_record_name(const u8 *record, u32 *name)
{
    for (u32 w = 0; w < GRKIT_RECIPE_NAME_WORDS; ++w) name[w] = 0;
    for (u32 i = 0; i < GRKIT_RECIPE_NAME_CHARS && i < GRKIT_RCP_NAME_UNITS; ++i)
        name[i / 4] |= (u32)(record[GRKIT_RCP_RECORD_NAME + 2 * i] & 0x7fU) << (7U * (i & 3U));
}
/* The first of the first `count` recipes of the buffer with these words and
   this name, or `count`. The number, the hidden bit, and the off bit do not
   count. */
static u32 recipe_known(u32 count, const u32 *w, const u32 *name)
{
    for (u32 slot = 0; slot < count; ++slot) {
        int same = 1;
        for (u32 k = 0; k < GRKIT_RECIPE_WORDS && same; ++k) {
            u32 ignored = !k ? RECIPE_WORD_NUMBER | GRKIT_RECIPE_HIDDEN : k == GRKIT_RECIPE_MODULE_WORD ? GRKIT_RECIPE_OFF : 0U;
            same = (recipes_list[slot].words[k] & ~ignored) == (w[k] & ~ignored);
        }
        for (u32 k = 0; k < GRKIT_RECIPE_NAME_WORDS && same; ++k) same = recipes_list[slot].name[k] == name[k];
        if (same) return slot;
    }
    return count;
}
/* Write the `recipes_count` recipes of the buffer to the header in the order
   of the camera: those that are on, at most GRKIT_RECIPE_ON_MAX, then the
   others, each in their order. The other slots get zeros. 1 when the order
   or an off bit changed. */
static int recipes_store(u32 base)
{
    u32 slot = 0, changed = 0;
    recipes_on = recipes_turned_off = 0;
    /* Past GRKIT_RECIPE_ON_MAX, a recipe that is on turns off. */
    for (u32 i = 0; i < recipes_count; ++i) {
        if (recipes_list[i].words[GRKIT_RECIPE_MODULE_WORD] & GRKIT_RECIPE_OFF) continue;
        if (recipes_on < GRKIT_RECIPE_ON_MAX) { ++recipes_on; continue; }
        recipes_list[i].words[GRKIT_RECIPE_MODULE_WORD] |= GRKIT_RECIPE_OFF;
        ++recipes_turned_off;
        changed = 1;
    }
    /* The recipes that are on in pass 0, then those that are off. */
    for (u32 pass = 0; pass < 2; ++pass)
        for (u32 i = 0; i < recipes_count; ++i) {
            u32 on = !(recipes_list[i].words[GRKIT_RECIPE_MODULE_WORD] & GRKIT_RECIPE_OFF);
            if (on != !pass) continue;
            if (slot != i) changed = 1;
            for (u32 k = 0; k < GRKIT_RECIPE_WORDS; ++k) store(recipe_slot_word(base, slot, k), recipes_list[i].words[k]);
            for (u32 k = 0; k < GRKIT_RECIPE_NAME_WORDS; ++k) store(recipe_name_word(base, slot, k), recipes_list[i].name[k]);
            store(recipe_id_word(base, slot), recipes_list[i].id);
            ++slot;
        }
    for (; slot < GRKIT_RECIPE_SLOTS; ++slot) {
        for (u32 k = 0; k < GRKIT_RECIPE_WORDS; ++k) store(recipe_slot_word(base, slot, k), 0);
        for (u32 k = 0; k < GRKIT_RECIPE_NAME_WORDS; ++k) store(recipe_name_word(base, slot, k), 0);
        store(recipe_id_word(base, slot), 0);
    }
    return changed;
}
/* A generated ID (runtime/recipe_id.h) that no recipe of the list has, or 0
   when each candidate is taken. */
static u32 recipes_new_id(void)
{
    for (u32 attempt = 0; attempt < RECIPE_ID_ATTEMPTS; ++attempt) {
        u32 id = recipe_id_candidate(attempt);
        int taken = !id || id > GRKIT_RECIPE_ID_MAX;
        for (u32 i = 0; i < recipes_count && !taken; ++i) taken = recipes_list[i].id == id;
        if (!taken) return id;
    }
    return 0;
}
/* Generate an ID for each recipe that has none, or one that an earlier
   recipe has: a recipe of a package, or of a recipe file from before the
   IDs. 1 when a recipe got an ID. */
static int recipes_give_ids(void)
{
    int changed = 0;
    for (u32 i = 0; i < recipes_count; ++i) {
        u32 id = recipes_list[i].id;
        int taken = !id || id > GRKIT_RECIPE_ID_MAX;
        for (u32 j = 0; j < i && !taken; ++j) taken = recipes_list[j].id == id;
        if (!taken) continue;
        recipes_list[i].id = recipes_new_id();
        changed = 1;
    }
    return changed;
}
/* The hash that the recipe file keeps for a library: its CRC, below 2^29
   and never 0. */
static u32 recipes_hash(const u8 *file, u32 size) { return (recipes_crc(file, size) & 0x0fffffffU) | 0x10000000U; }

/* Restore the recipes, then add those of a new package library whose
   family is `family`, the GRKIT_RCP_FAMILY of this camera. A record of
   another family counts in recipes_other. */
static void install_recipes(u32 base, u32 family)
{
    u8 *text = recipes_text;
    u32 value, imported = 0, hash = 0;
    recipes_import = recipes_added = recipes_full = recipes_other = recipes_same = recipes_bad = recipes_switched = 0;
    long n = read_text(recipes_state_path, text, sizeof(recipes_text));
    if (n < 0) n = read_text(recipes_state_tmp_path, text, sizeof(recipes_text));
    state_parse(&saved_recipes, text, n);
    for (u32 slot = 0; slot < GRKIT_RECIPE_SLOTS; ++slot) {
        for (u32 w = 0; w < GRKIT_RECIPE_WORDS; ++w)
            recipes_list[slot].words[w] =
                state_value(&saved_recipes, GRKIT_RECIPE_KEY_BASE + w * GRKIT_RECIPE_KEY_STRIDE + slot, &value) &&
                        value <= RECIPE_VALUE_MAX ? value : 0;
        for (u32 w = 0; w < GRKIT_RECIPE_NAME_WORDS; ++w)
            recipes_list[slot].name[w] =
                state_value(&saved_recipes, GRKIT_RECIPE_NAME_KEY_BASE + w * GRKIT_RECIPE_KEY_STRIDE + slot, &value) &&
                        value <= RECIPE_VALUE_MAX ? value : 0;
        recipes_list[slot].id =
            state_value(&saved_recipes, GRKIT_RECIPE_ID_KEY_BASE + slot, &value) && value <= RECIPE_VALUE_MAX ? value : 0;
    }
    if (state_value(&saved_recipes, GRKIT_RECIPE_IMPORTED_KEY, &value) && value <= RECIPE_VALUE_MAX) imported = value;
    /* The recipes: the run of recipe words. */
    recipes_count = 0;
    while (recipes_count < GRKIT_RECIPE_SLOTS && recipe_first_word(recipes_list[recipes_count].words[0])) ++recipes_count;

    long size = read_text(recipes_inbox_path, recipes_inbox, sizeof(recipes_inbox));
    if (size >= 0 && !recipes_whole(recipes_inbox, (u32)size)) recipes_import = RECIPES_IMPORT_DAMAGED;
    else if (size >= 0 && (hash = recipes_hash(recipes_inbox, (u32)size)) != imported) {
        recipes_import = RECIPES_IMPORT_READ;
        u32 records = get32(recipes_inbox + GRKIT_RCP_COUNT);
        for (u32 i = 0; i < records; ++i) {
            const u8 *record = recipes_inbox + GRKIT_RCP_HEADER_SIZE + i * GRKIT_RCP_RECORD_SIZE;
            u32 w[GRKIT_RECIPE_WORDS], name[GRKIT_RECIPE_NAME_WORDS];
            if (!recipe_record_usable(record)) { ++recipes_bad; continue; }
            if (record[GRKIT_RCP_RECORD_FAMILY] != family) { ++recipes_other; continue; }
            for (u32 k = 0; k < GRKIT_RECIPE_WORDS; ++k) w[k] = get32(record + GRKIT_RCP_RECORD_WORDS + 4 * k);
            recipe_record_name(record, name);
            /* A recipe that the list holds takes the on or off of the library:
               the site turns recipes on and off, and the camera does not. */
            u32 known = recipe_known(recipes_count, w, name);
            if (known < recipes_count) {
                u32 off = w[GRKIT_RECIPE_MODULE_WORD] & GRKIT_RECIPE_OFF;
                if ((recipes_list[known].words[GRKIT_RECIPE_MODULE_WORD] & GRKIT_RECIPE_OFF) != off) {
                    recipes_list[known].words[GRKIT_RECIPE_MODULE_WORD] = (recipes_list[known].words[GRKIT_RECIPE_MODULE_WORD] & ~GRKIT_RECIPE_OFF) | off;
                    ++recipes_switched;
                }
                ++recipes_same;
                continue;
            }
            if (recipes_count == GRKIT_RECIPE_SLOTS) { ++recipes_full; continue; }
            for (u32 k = 0; k < GRKIT_RECIPE_WORDS; ++k) recipes_list[recipes_count].words[k] = w[k];
            for (u32 k = 0; k < GRKIT_RECIPE_NAME_WORDS; ++k) recipes_list[recipes_count].name[k] = name[k];
            recipes_list[recipes_count].id = 0;
            ++recipes_count;
            ++recipes_added;
        }
        imported = hash;
    }
    int given = recipes_give_ids(), changed = recipes_store(base);
    /* The loader saves the recipe file with the hash (StateFile::Recipes)
       after an import, a change of the order, or new IDs. */
    store(base + GRKIT_HOST_RECIPE_IMPORTED, imported);
    if (changed || given || recipes_import == RECIPES_IMPORT_READ) store(base + GRKIT_HOST_RECIPE_CHANGES, 1);
}

/* Restore the settings of the user modes from their file. The loader saves
   the file only after a change on the camera (GRKIT_HOST_DIAL_SET_CHANGES),
   so a set that stays out stays in the file until then. */
static void install_dial_sets(u32 base)
{
    long n = read_text(dial_sets_path, recipes_text, sizeof(recipes_text));
    if (n < 0) n = read_text(dial_sets_tmp_path, recipes_text, sizeof(recipes_text));
    state_parse(&saved_recipes, recipes_text, n);
    for (u32 u = 0; u < GRKIT_DIAL_USER_MODES; ++u) {
        u32 key = GRKIT_DIAL_SET_KEY_BASE + u * GRKIT_DIAL_SET_KEY_STRIDE, value, count = 0;
        u32 set = base + GRKIT_HOST_DIAL_SETS + 4U * u * GRKIT_DIAL_SET_WORDS;
        if (state_value(&saved_recipes, key + GRKIT_DIAL_SET_COUNT, &value)) count = value;
        int whole = count <= GRKIT_DIAL_SET_PAIRS + 1U;
        /* A value has at most 9 digits (runtime/state.h), as the loader
           writes each word up to GRKIT_DIAL_SET_VALUE_MAX. */
        for (u32 w = 0; w < GRKIT_DIAL_SET_WORDS; ++w)
            store(set + 4U * w, whole && state_value(&saved_recipes, key + w, &value) ? value : 0);
    }
    /* What each user mode does, then whether GRKit is off at P/Av/Tv/M,
       which follows it in the header and in the keys. */
    _Static_assert(GRKIT_HOST_DIAL_SHARED_OFF == GRKIT_HOST_DIAL_SLOTS + 4U * GRKIT_DIAL_SLOT_WORDS * GRKIT_DIAL_USER_MODES,
                   "the row of P/Av/Tv/M follows the user modes");
    for (u32 w = 0; w <= GRKIT_DIAL_SLOT_WORDS * GRKIT_DIAL_USER_MODES; ++w) {
        u32 value;
        store(base + GRKIT_HOST_DIAL_SLOTS + 4U * w,
              state_value(&saved_recipes, GRKIT_DIAL_SLOT_KEY_BASE + w, &value) ? value : 0);
    }
}

/* The copy of the switch, Enable at Start, and Distance Unit in the file
   of the user modes decides them instead of the main state file: the core
   writes the copy with each change of them, and the loader never holds that
   file, so the copy is never older. The main state file can be: the core
   holds it on a user mode with a snapshot (runtime/mode_dial.h). A value of
   the copy goes into the pairs of the main state file, so start_switch,
   order_modules, and apply_state take it as a saved value, and Keep Last
   keeps the switch of the copy. A card without a copy, or a value out of
   range, keeps the value of the main state file. plan_modules calls this
   after read_state, so SCAN and INSTALL agree. */
static void read_manage_copy(void)
{
    static const char *const keys[GRKIT_MANAGE_COPY_WORDS] = {MANAGE_SWITCH_KEY, MANAGE_START_KEY, MANAGE_UNIT_KEY};
    static const u32 values[GRKIT_MANAGE_COPY_WORDS] = {2U, MANAGE_START_VALUES, MANAGE_UNIT_VALUES};
    _Static_assert(GRKIT_MANAGE_COPY_SWITCH == 0U && GRKIT_MANAGE_COPY_START == 1U && GRKIT_MANAGE_COPY_UNIT == 2U,
                   "the keys follow the words of the copy");
    long n = read_text(dial_sets_path, recipes_text, sizeof(recipes_text));
    if (n < 0) n = read_text(dial_sets_tmp_path, recipes_text, sizeof(recipes_text));
    state_parse(&saved_recipes, recipes_text, n);
    for (u32 i = 0; i < GRKIT_MANAGE_COPY_WORDS; ++i) {
        u32 value;
        if (state_value(&saved_recipes, GRKIT_MANAGE_COPY_KEY + i, &value) && value && value - 1U < values[i])
            state_set(&saved_state, grm_state_id(keys[i]), value - 1U);
    }
}
/* Write the copy of the switch, Enable at Start, and Distance Unit into the
   header: the values of this start in `table`, which apply_state restored,
   for the handles of the module list. No count of changes moves, so the
   file of the user modes saves only after a change on the camera. Without a
   module list, no copy. */
static void install_manage_copy(u32 base, u32 table)
{
    _Static_assert(GRKIT_HOST_MANAGE_COPY == GRKIT_HOST_DIAL_SHARED_OFF + 4U, "the copy follows the row of P/Av/Tv/M");
    for (u32 i = 0; i < GRKIT_MANAGE_COPY_WORDS; ++i) {
        u32 at = table && module_list ? word(module_list + MANAGE_SWITCH + 4U * i) : 0;
        store(base + GRKIT_HOST_MANAGE_COPY + 4U * i, at ? word(table + 4U * SETTINGS_ENTRY(at - 1U) + 4U) + 1U : 0);
    }
}

/* The recipes that cannot apply in full on this camera, which REPORT.TXT
   names. A recipe has the first of these problems: its effect is not
   installed or Film Look does not run; it holds a Grain that is not Off
   without a running Grain module; or it holds a setting of module m of
   GRKIT_RECIPE_MODULES that is not Off without that module
   (runtime/recipes.h applies the same rules). */
#define RECIPE_PROBLEMS_MAX 8U
#define RECIPE_PROBLEM_EFFECT 1U
#define RECIPE_PROBLEM_GRAIN 2U
#define RECIPE_PROBLEM_MODULE(m) (3U + (m))
struct recipe_problem {
    char name[GRKIT_RECIPE_NAME_CHARS + 1];
    /* The number of the recipe, from 1. */
    u32 recipe, problem;
};
static struct recipe_problem recipe_problems[RECIPE_PROBLEMS_MAX];
static u32 recipe_problem_count, recipe_problems_more;
static u32 recipe_problem_of(const u32 *w, u32 base, u32 records, u32 table, u32 capabilities)
{
    static const u32 ids[GRKIT_RECIPE_SETTINGS] = {GRKIT_RECIPE_SETTING_IDS};
    static const u8 words[GRKIT_RECIPE_SETTINGS] = {GRKIT_RECIPE_SETTING_WORDS};
    static const u8 shifts[GRKIT_RECIPE_SETTINGS] = {GRKIT_RECIPE_SETTING_SHIFTS};
    static const u8 masks[GRKIT_RECIPE_SETTINGS] = {GRKIT_RECIPE_SETTING_MASKS};
    static const u8 modules[GRKIT_RECIPE_SETTINGS] = {GRKIT_RECIPE_SETTING_MODULES};
    if ((w[0] & GRKIT_RECIPE_LOOK_MASK) == GRKIT_RECIPE_LOOK_EFFECT) {
        u32 code = w[0] >> GRKIT_RECIPE_CODE_SHIFT & GRKIT_RECIPE_CODE_MASK, i = 0;
        if (!(capabilities & GRKIT_CORE_TONE_BIT)) return RECIPE_PROBLEM_EFFECT;
        for (; i < records; ++i) {
            u32 id = base + GRKIT_HOST_HEADER_SIZE + i * GRKIT_RECORD_SIZE + GRKIT_RECORD_ID;
            if ((effect_code((const u8 *)at(id)) & GRKIT_RECIPE_CODE_MASK) == code) break;
        }
        if (i == records) return RECIPE_PROBLEM_EFFECT;
    }
    if ((w[0] & GRKIT_RECIPE_GRAIN_HELD) && (w[0] >> GRKIT_RECIPE_GRAIN_LEVEL_SHIFT & GRKIT_RECIPE_GRAIN_LEVEL_MASK) &&
        !(capabilities & GRKIT_CORE_GRAIN_BIT))
        return RECIPE_PROBLEM_GRAIN;
    for (u32 s = 0; s < GRKIT_RECIPE_SETTINGS; ++s) {
        u32 bits = w[words[s]] >> shifts[s], found = 0;
        if (!(bits & 1U) || !(bits >> 1 & masks[s])) continue;
        if (GRKIT_RECIPE_SETTING_CORE >> s & 1U)
            found = capabilities >> (ids[s] - GRKIT_CORE_SETTING_BASE) & 1U;
        else
            for (u32 k = 0; table && k < word(table); ++k)
                if (word(table + 4U * SETTINGS_ENTRY(k)) == ids[s]) found = 1;
        if (!found) return RECIPE_PROBLEM_MODULE(modules[s]);
    }
    return 0;
}
/* Check the recipes that are on after INSTALL wrote the records, the
   modules, and the recipes: `base` is the header, `records` the installed
   effects, and `table` the settings table or 0. A recipe that is off does
   not apply on the camera. */
static void check_recipes(u32 base, u32 records, u32 table, u32 capabilities)
{
    u32 w[GRKIT_RECIPE_WORDS], count = 0;
    while (count < GRKIT_RECIPE_ON_MAX && recipe_first_word(word(recipe_slot_word(base, count, 0))) &&
           !(word(recipe_slot_word(base, count, GRKIT_RECIPE_MODULE_WORD)) & GRKIT_RECIPE_OFF)) ++count;
    recipe_problem_count = recipe_problems_more = 0;
    for (u32 slot = 0; slot < count; ++slot) {
        u32 problem, n = 0;
        for (u32 k = 0; k < GRKIT_RECIPE_WORDS; ++k) w[k] = word(recipe_slot_word(base, slot, k));
        problem = recipe_problem_of(w, base, records, table, capabilities);
        if (!problem) continue;
        if (recipe_problem_count == RECIPE_PROBLEMS_MAX) { ++recipe_problems_more; continue; }
        struct recipe_problem *p = &recipe_problems[recipe_problem_count++];
        for (; n < GRKIT_RECIPE_NAME_CHARS; ++n) {
            u32 c = word(recipe_name_word(base, slot, n / 4U)) >> (7U * (n & 3U)) & 0x7fU;
            if (c < 0x20U || c > 0x7eU) break;
            p->name[n] = (char)c;
        }
        p->name[n] = 0;
        p->recipe = slot + 1U;
        p->problem = problem;
    }
}
#endif
