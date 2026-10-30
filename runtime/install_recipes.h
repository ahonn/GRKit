#ifndef GRKIT_INSTALL_RECIPES_H
#define GRKIT_INSTALL_RECIPES_H
/* A part of the installer: the recipe library of GRKIT/RECIPES.BIN, which the
   site writes (grkit_core::recipes). installer.c includes it once, after the
   helpers and the services block.

   INSTALL copies each usable record into the library of the services block
   (runtime/services.h) in file order, and REPORT.TXT tells the result. A
   file with a wrong size or CRC holds no usable record. A record is usable if
   its name is 1 to GRKIT_RCP_NAME_UNITS printable ASCII characters, its
   family is known, its first word is that of a recipe without a number, and
   each word stays below 2^29. The core checks the values for the camera
   when a person applies the recipe (runtime/recipes.h). */
static const char recipes_path[] = RECIPES_PATH;
static u8 recipes_file[GRKIT_RCP_HEADER_SIZE + GRKIT_RCP_MAX * GRKIT_RCP_RECORD_SIZE];
/* The result for REPORT.TXT: RECIPES_NONE without a file, RECIPES_DAMAGED,
   or the number of records and of the usable ones. */
#define RECIPES_NONE 0U
#define RECIPES_DAMAGED 1U
#define RECIPES_READ 2U
static u32 recipes_result, recipes_records, recipes_loaded;
static u32 recipes_skipped_first;

/* The CRC-32 of the file, which reads the CRC field as zero. */
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
    u32 family = record[GRKIT_RCP_RECORD_FAMILY], first = get32(record + GRKIT_RCP_RECORD_WORDS);
    if (!units || family < 1 || family > 3) return 0;
    /* The used bit, a known look, a Grain level below 5, no number. */
    if (!(first & 0x08000000U) || (first & 3U) == 3U || (first >> 3 & 7U) > 4U || (first >> 23 & 15U)) return 0;
    for (u32 w = 0; w < GRKIT_RECIPE_WORDS; ++w)
        if (get32(record + GRKIT_RCP_RECORD_WORDS + 4 * w) >> 29) return 0;
    return !(get32(record + GRKIT_RCP_RECORD_HINTS) >> 29);
}
/* Read the library into the services block at `services`. */
static void install_recipes(u32 services)
{
    u32 library = services + SERVICES_LIBRARY, count;
    recipes_result = RECIPES_NONE;
    recipes_records = recipes_loaded = 0;
    store(library + LIBRARY_COUNT, 0);
    long size = read_text(recipes_path, recipes_file, sizeof(recipes_file));
    if (size < 0) {
        if (present(recipes_path)) recipes_result = RECIPES_DAMAGED;
        return;
    }
    recipes_result = RECIPES_DAMAGED;
    if ((u32)size < GRKIT_RCP_HEADER_SIZE || !equal(recipes_file + GRKIT_RCP_MAGIC, (const u8 *)"GRKR0001", 8)) return;
    count = get32(recipes_file + GRKIT_RCP_COUNT);
    if (count > GRKIT_RCP_MAX || get32(recipes_file + GRKIT_RCP_BYTES) != (u32)size ||
        (u32)size != GRKIT_RCP_HEADER_SIZE + count * GRKIT_RCP_RECORD_SIZE ||
        get32(recipes_file + GRKIT_RCP_CRC) != recipes_crc(recipes_file, (u32)size)) return;
    recipes_result = RECIPES_READ;
    recipes_records = count;
    recipes_skipped_first = 0;
    for (u32 i = 0; i < count; ++i) {
        const u8 *record = recipes_file + GRKIT_RCP_HEADER_SIZE + i * GRKIT_RCP_RECORD_SIZE;
        if (!recipe_record_usable(record)) {
            if (!recipes_skipped_first) recipes_skipped_first = i + 1;
            continue;
        }
        u32 entry = library + LIBRARY_ENTRIES + recipes_loaded * LIBRARY_ENTRY_SIZE;
        u8 bytes[LIBRARY_ENTRY_SIZE];
        zero(bytes, sizeof(bytes));
        copy(bytes + LIBRARY_NAME, record + GRKIT_RCP_RECORD_NAME, 2 * GRKIT_RCP_NAME_UNITS);
        bytes[LIBRARY_FAMILY] = record[GRKIT_RCP_RECORD_FAMILY];
        copy(bytes + LIBRARY_WORDS, record + GRKIT_RCP_RECORD_WORDS, 4 * GRKIT_RECIPE_WORDS);
        copy(bytes + LIBRARY_HINTS, record + GRKIT_RCP_RECORD_HINTS, 4);
        for (u32 k = 0; k < LIBRARY_ENTRY_SIZE; k += 4) store(entry + k, get32(bytes + k));
        ++recipes_loaded;
    }
    store(library + LIBRARY_COUNT, recipes_loaded);
}
#endif
