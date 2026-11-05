#ifndef GRKIT_INSTALL_RECIPES_H
#define GRKIT_INSTALL_RECIPES_H
/* A part of the installer: the recipes (runtime/recipes.h). installer.c
   includes it once, after the helpers, the state parser, and the effects.

   - The recipe file (GRKIT/RECIPES.TXT, or RECIPES.TMP that a save cut by
     the power left) holds the recipes, their names, and the hash of the last
     package library that INSTALL imported (grkit_core::runtime::recipe_key).
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
     the usable recipes of the camera line that the recipes do not have yet,
     up to GRKIT_RECIPE_SLOTS, once for each library: it keeps the hash of
     the file, and sets the count of recipe changes so that the loader saves
     the file. An added recipe keeps its on or off; one that is on goes after
     the recipes that are on. A recipe that the list holds takes the on or
     off of the library, because the site turns recipes on and off. A recipe that a person deleted on the camera does not come
     back from the same package. A record is usable if its name is 1 to
     GRKIT_RCP_NAME_UNITS printable ASCII characters, its family is known,
     its first word is that of a recipe, and each word stays below 2^29. The
     core checks the values for the camera when a person applies the recipe.
   - REPORT.TXT tells the count, the import, and each recipe that cannot
     apply in full (check_recipes). The installer deletes no file. */
static const char recipes_state_path[] = RECIPES_STATE_PATH, recipes_state_tmp_path[] = RECIPES_STATE_TMP_PATH;
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
   added ones. */
static u32 recipes_words[GRKIT_RECIPE_SLOTS][GRKIT_RECIPE_WORDS], recipes_names[GRKIT_RECIPE_SLOTS][GRKIT_RECIPE_NAME_WORDS];

/* The bits of the recipe words that the installer reads (runtime/recipes.h):
   word 0 has the look in bits 0-1 (2 for an effect), Grain held in bit 2
   and its level in bits 3-5, the effect code in bits 8-22, the number in
   bits 23-26, the used bit 27, and the hidden bit 28; word 4 has 3 bits for
   each module setting, a held bit and then the value, and the off bit 28. */
#define RECIPE_WORD_USED 0x08000000U
#define RECIPE_WORD_NUMBER 0x07800000U
#define RECIPE_WORD_HIDDEN 0x10000000U
#define RECIPE_WORD_OFF 0x10000000U
static u32 recipe_slot_word(u32 base, u32 slot, u32 w)
{
    return base + GRKIT_HOST_RECIPES + 4U * (slot * GRKIT_RECIPE_WORDS + w);
}
static u32 recipe_name_word(u32 base, u32 slot, u32 w)
{
    return base + GRKIT_HOST_RECIPE_NAMES + 4U * (slot * GRKIT_RECIPE_NAME_WORDS + w);
}
/* Whether word 0 is that of a recipe. */
static int recipe_first_word(u32 w)
{
    return (w & RECIPE_WORD_USED) && !(w >> 29) && (w & 3U) != 3U && (w >> 3 & 7U) <= 4U;
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
    u32 units = 0, ended = 0;
    for (u32 i = 0; i < GRKIT_RCP_NAME_UNITS; ++i) {
        u32 unit = record[GRKIT_RCP_RECORD_NAME + 2 * i] | record[GRKIT_RCP_RECORD_NAME + 2 * i + 1] << 8;
        if (!unit) { ended = 1; continue; }
        if (ended || unit < 0x20 || unit > 0x7e) return 0;
        ++units;
    }
    u32 family = record[GRKIT_RCP_RECORD_FAMILY];
    if (!units || family < 1 || family > 3 || !recipe_first_word(get32(record + GRKIT_RCP_RECORD_WORDS))) return 0;
    for (u32 w = 0; w < GRKIT_RECIPE_WORDS; ++w)
        if (get32(record + GRKIT_RCP_RECORD_WORDS + 4 * w) >> 29) return 0;
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
            u32 ignored = !k ? RECIPE_WORD_NUMBER | RECIPE_WORD_HIDDEN : k == 4U ? RECIPE_WORD_OFF : 0U;
            same = (recipes_words[slot][k] & ~ignored) == (w[k] & ~ignored);
        }
        for (u32 k = 0; k < GRKIT_RECIPE_NAME_WORDS && same; ++k) same = recipes_names[slot][k] == name[k];
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
        if (recipes_words[i][4] & RECIPE_WORD_OFF) continue;
        if (recipes_on < GRKIT_RECIPE_ON_MAX) { ++recipes_on; continue; }
        recipes_words[i][4] |= RECIPE_WORD_OFF;
        ++recipes_turned_off;
        changed = 1;
    }
    /* The recipes that are on in pass 0, then those that are off. */
    for (u32 pass = 0; pass < 2; ++pass)
        for (u32 i = 0; i < recipes_count; ++i) {
            u32 on = !(recipes_words[i][4] & RECIPE_WORD_OFF);
            if (on != !pass) continue;
            if (slot != i) changed = 1;
            for (u32 k = 0; k < GRKIT_RECIPE_WORDS; ++k) store(recipe_slot_word(base, slot, k), recipes_words[i][k]);
            for (u32 k = 0; k < GRKIT_RECIPE_NAME_WORDS; ++k) store(recipe_name_word(base, slot, k), recipes_names[i][k]);
            ++slot;
        }
    for (; slot < GRKIT_RECIPE_SLOTS; ++slot) {
        for (u32 k = 0; k < GRKIT_RECIPE_WORDS; ++k) store(recipe_slot_word(base, slot, k), 0);
        for (u32 k = 0; k < GRKIT_RECIPE_NAME_WORDS; ++k) store(recipe_name_word(base, slot, k), 0);
    }
    return changed;
}
/* The hash that the recipe file keeps for a library: its CRC, below 2^29
   and never 0. */
static u32 recipes_hash(const u8 *file, u32 size) { return (recipes_crc(file, size) & 0x0fffffffU) | 0x10000000U; }

/* Restore the recipes, then add those of a new package library. `gr3`:
   the camera is of the GR III line, whose recipes have family 1. */
static void install_recipes(u32 base, int gr3)
{
    static u8 text[STATE_TEXT_MAX];
    u32 value, imported = 0, hash = 0;
    recipes_import = recipes_added = recipes_full = recipes_other = recipes_same = recipes_bad = recipes_switched = 0;
    long n = read_text(recipes_state_path, text, sizeof(text));
    if (n < 0) n = read_text(recipes_state_tmp_path, text, sizeof(text));
    state_parse(&saved_recipes, text, n);
    for (u32 slot = 0; slot < GRKIT_RECIPE_SLOTS; ++slot) {
        for (u32 w = 0; w < GRKIT_RECIPE_WORDS; ++w)
            recipes_words[slot][w] =
                state_value(&saved_recipes, GRKIT_RECIPE_KEY_BASE + w * GRKIT_RECIPE_KEY_STRIDE + slot, &value) &&
                        value <= RECIPE_VALUE_MAX ? value : 0;
        for (u32 w = 0; w < GRKIT_RECIPE_NAME_WORDS; ++w)
            recipes_names[slot][w] =
                state_value(&saved_recipes, GRKIT_RECIPE_NAME_KEY_BASE + w * GRKIT_RECIPE_KEY_STRIDE + slot, &value) &&
                        value <= RECIPE_VALUE_MAX ? value : 0;
    }
    if (state_value(&saved_recipes, GRKIT_RECIPE_IMPORTED_KEY, &value) && value <= RECIPE_VALUE_MAX) imported = value;
    /* The recipes: the run of recipe words. */
    recipes_count = 0;
    while (recipes_count < GRKIT_RECIPE_SLOTS && recipe_first_word(recipes_words[recipes_count][0])) ++recipes_count;

    long size = read_text(recipes_inbox_path, recipes_inbox, sizeof(recipes_inbox));
    if (size >= 0 && !recipes_whole(recipes_inbox, (u32)size)) recipes_import = RECIPES_IMPORT_DAMAGED;
    else if (size >= 0 && (hash = recipes_hash(recipes_inbox, (u32)size)) != imported) {
        recipes_import = RECIPES_IMPORT_READ;
        u32 records = get32(recipes_inbox + GRKIT_RCP_COUNT);
        for (u32 i = 0; i < records; ++i) {
            const u8 *record = recipes_inbox + GRKIT_RCP_HEADER_SIZE + i * GRKIT_RCP_RECORD_SIZE;
            u32 w[GRKIT_RECIPE_WORDS], name[GRKIT_RECIPE_NAME_WORDS], family;
            if (!recipe_record_usable(record)) { ++recipes_bad; continue; }
            family = record[GRKIT_RCP_RECORD_FAMILY];
            if (gr3 ? family != 1U : family == 1U) { ++recipes_other; continue; }
            for (u32 k = 0; k < GRKIT_RECIPE_WORDS; ++k) w[k] = get32(record + GRKIT_RCP_RECORD_WORDS + 4 * k);
            recipe_record_name(record, name);
            /* A recipe that the list holds takes the on or off of the library:
               the site turns recipes on and off, and the camera does not. */
            u32 known = recipe_known(recipes_count, w, name);
            if (known < recipes_count) {
                u32 off = w[4] & RECIPE_WORD_OFF;
                if ((recipes_words[known][4] & RECIPE_WORD_OFF) != off) {
                    recipes_words[known][4] = (recipes_words[known][4] & ~RECIPE_WORD_OFF) | off;
                    ++recipes_switched;
                }
                ++recipes_same;
                continue;
            }
            if (recipes_count == GRKIT_RECIPE_SLOTS) { ++recipes_full; continue; }
            for (u32 k = 0; k < GRKIT_RECIPE_WORDS; ++k) recipes_words[recipes_count][k] = w[k];
            for (u32 k = 0; k < GRKIT_RECIPE_NAME_WORDS; ++k) recipes_names[recipes_count][k] = name[k];
            ++recipes_count;
            ++recipes_added;
        }
        imported = hash;
    }
    /* The loader saves the recipe file with the hash (StateFile::Recipes)
       after an import or a change of the order. */
    store(base + GRKIT_HOST_RECIPE_IMPORTED, imported);
    if (recipes_store(base) || recipes_import == RECIPES_IMPORT_READ) store(base + GRKIT_HOST_RECIPE_CHANGES, 1);
}

/* The recipes that cannot apply in full on this camera, which REPORT.TXT
   names. A recipe has the first of these problems: its effect is not
   installed or Film Look does not run; it holds a Grain that is not Off
   without a running Grain module; or it holds a Glow that is not Off
   without the Glow module (runtime/recipes.h applies the same rules). */
#define RECIPE_PROBLEMS_MAX 8U
#define RECIPE_PROBLEM_EFFECT 1U
#define RECIPE_PROBLEM_GRAIN 2U
#define RECIPE_PROBLEM_GLOW 3U
struct recipe_problem {
    char name[GRKIT_RECIPE_NAME_CHARS + 1];
    /* The number of the recipe, from 1. */
    u32 recipe, problem;
};
static struct recipe_problem recipe_problems[RECIPE_PROBLEMS_MAX];
static u32 recipe_problem_count, recipe_problems_more;
static u32 recipe_problem_of(const u32 *w, u32 base, u32 records, u32 table, u32 capabilities)
{
    static const u32 ids[GRKIT_RECIPE_MODULE_SETTINGS] = {GRKIT_RECIPE_MODULE_SETTING_IDS};
    if ((w[0] & 3U) == 2U) {
        u32 code = w[0] >> 8 & 0x7fffU, i = 0;
        if (!(capabilities & GRKIT_CORE_TONE_BIT)) return RECIPE_PROBLEM_EFFECT;
        for (; i < records; ++i) {
            u32 id = base + GRKIT_HOST_HEADER_SIZE + i * GRKIT_RECORD_SIZE + GRKIT_RECORD_ID;
            if ((effect_code((const u8 *)at(id)) & 0x7fffU) == code) break;
        }
        if (i == records) return RECIPE_PROBLEM_EFFECT;
    }
    if ((w[0] & 4U) && (w[0] >> 3 & 7U) && !(capabilities & GRKIT_CORE_GRAIN_BIT)) return RECIPE_PROBLEM_GRAIN;
    for (u32 m = 0; m < GRKIT_RECIPE_MODULE_SETTINGS; ++m) {
        u32 bits = w[4] >> (3U * m), found = 0;
        if (!(bits & 1U) || !(bits >> 1 & 3U)) continue;
        for (u32 k = 0; table && k < word(table); ++k)
            if (word(table + 4U * SETTINGS_ENTRY(k)) == ids[m]) found = 1;
        if (!found) return RECIPE_PROBLEM_GLOW;
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
           !(word(recipe_slot_word(base, count, 4)) & RECIPE_WORD_OFF)) ++count;
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
