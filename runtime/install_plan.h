#ifndef GRKIT_INSTALL_PLAN_H
#define GRKIT_INSTALL_PLAN_H
/* A part of the installer: the module list and the memory plan. installer.c
   includes it once, in its order, after the helpers and the state that this
   part uses.

   Each GRKIT/MODULES/<ID>.GRM is one module. SCAN reserves script
   memory for the modules, and INSTALL links them there and registers their
   layers. Both steps skip the same unusable modules.

   The code of each module goes after the tail, up to RTOS_EXECUTABLE_END.
   The RTOS maps that area read-only and executable, and no firmware code uses
   it, so the code stays valid even if the resident script ends. The data of
   each module goes into the module script array, which the RTOS can write. */
#define MODULE_FILE_MAX (GRM_HEADER + 4U * GRM_MAX_SEGMENT)
/* The most code bytes of all modules after the tail. The area is larger on
   each target. */
#define MODULE_CODE_LIMIT 0x100000U
static const char modules_path[] = MODULES_PATH;
static const char package_modules_path[] = PACKAGE_MODULES_PATH;
_Static_assert(sizeof(package_modules_path) >= sizeof(modules_path), "The path buffer fits both folders");
static char module_names[GRKIT_MODULES_MAX][CATALOG_NAME_SIZE];
/* The listed modules, the usable ones (bit i), and what they need: script
   words for their data, and code bytes after the tail. */
static u32 module_count, module_usable, module_words, module_code_bytes;
static u8 module_file[MODULE_FILE_MAX];
static struct grm_module module;
/* The settings of the usable modules in module order (runtime/setting_table.h):
   the full name, values, and flags of each, the first setting and the count
   of each module, and the used settings of each module. */
static char setting_names[GRKIT_MODULE_SETTINGS_MAX][GRM_NAME];
static u32 setting_values[GRKIT_MODULE_SETTINGS_MAX], setting_flags[GRKIT_MODULE_SETTINGS_MAX], setting_total;
static u32 module_first[GRKIT_MODULES_MAX], module_settings[GRKIT_MODULES_MAX], module_use_count[GRKIT_MODULES_MAX];
/* The global index of tile 0 of each module (GRKIT_ICON_MODULE_TILE). */
static u32 module_tile_first[GRKIT_MODULES_MAX];
static char module_uses[GRKIT_MODULES_MAX][GRM_USES][GRM_NAME + 1];
/* The result of each listed module for REPORT.TXT and the Modules page: 0
   when it loads, also when it is Off, MANAGE_SAFE in safe mode, else a
   status (runtime/module_link.h), with the name of the missing import or
   setting of GRM_MISSING_IMPORT and GRM_MISSING_SETTING. */
_Static_assert(GRKIT_MODULES_MAX == MANAGE_MODULES, "The Modules page lists each module");
static u32 module_status[GRKIT_MODULES_MAX];
static char module_detail[GRKIT_MODULES_MAX][GRM_NAME + 1];
/* The choices of the GRKit section of MENU (runtime/manage.h): Off, the
   position + 1 or 0, and the values that are off in ADJ of each module.
   INSTALL also notes the title texts and tile 0 of each module. */
static u32 module_off[GRKIT_MODULES_MAX], module_position[GRKIT_MODULES_MAX], module_rows[GRKIT_MODULES_MAX];
static u32 module_title[GRKIT_MODULES_MAX], module_tile[GRKIT_MODULES_MAX];
/* The GRKit switch: 1 while it is Off, which stops each module as its own
   Off does, and keeps the Enable of each. */
static u32 all_off;
/* Whether module i loads stopped: its Enable or the switch is Off. */
static u32 module_stopped(u32 i) { return module_off[i] || all_off; }
/* The code range and the used capabilities of each loaded module, for the
   soft Off of runtime/manage.h. */
static u32 module_code_from[GRKIT_MODULES_MAX], module_code_to[GRKIT_MODULES_MAX], module_capabilities[GRKIT_MODULES_MAX];
/* The capabilities of the modules that run: not those of a module that is
   Off, which loads stopped (runtime/manage.h). */
static u32 capabilities_running;
/* The addresses of the module list, of the effect list, and of the state of
   the core services, or 0 (install_modules). */
static u32 module_list, effect_list, services_block;
/* The entries of the settings table after the module settings: the
   choices of each module, the choices of `effects` effects, then the
   switch. */
#define CHOICE_ENTRIES(modules, effects) (MANAGE_PER_MODULE * (modules) + 1U + (effects))
/* The words of the module list and of the effect list. */
#define MANAGE_WORDS(n) ((MANAGE_HEADER + (n) * MANAGE_STRIDE) / 4U)
#define EFFECT_LIST_WORDS(n) ((EFFECTS_HEADER + (n) * EFFECTS_STRIDE) / 4U)
/* The words of the contexts and of the module data (plan_modules). */
static u32 module_contexts, module_data_words;
/* Saved values that no setting of this start takes, such as the settings of
   a module that is Off. The settings table keeps them, so the next save of
   the loader keeps them too. */
static u32 kept_ids[GRKIT_MODULE_SETTINGS_MAX], kept_values[GRKIT_MODULE_SETTINGS_MAX], kept_count;
/* Safe mode: 1 while the file GRKIT/SAFE or SAFE.TXT exists. No module
   loads then (scan). */
static u32 safe_mode;
/* The development profile: 1 while the file GRKIT/PROFILE or PROFILE.TXT
   exists (runtime/profile.h). */
static u32 profile_mode;
/* The capabilities that the loaded modules use (GRKIT_HOST_CAPABILITIES). */
static u32 capabilities_used;
static void read_state(void);
/* The words of the settings table and of the contexts (GrkitContext). */
#define SETTINGS_WORDS(n) (2U + 3U * (n))
#define CONTEXT_WORDS(uses) (3U + (uses))
static void name_copy(char *to, const char *from, u32 n)
{
    for (u32 i = 0; i < n; ++i) to[i] = from[i];
    to[n] = 0;
}
/* The order of each listed module, from the header of its file: its default
   place in ADJ and in MENU. A file that is not a module sorts last. */
static u32 module_order[GRKIT_MODULES_MAX];
#define ORDER_UNKNOWN (GRM_MAX_ORDER + 1U)
/* Whether a module of `order_a` in file `a` goes after one of `order_b` in
   file `b`: by order, then by name. */
static int module_after(u32 order_a, const char *a, u32 order_b, const char *b)
{
    return order_a != order_b ? order_a > order_b : compare((const u8 *)a, (const u8 *)b) > 0;
}
/* The order in the header of module file `name` in the package folder or in
   GRKIT/MODULES (`source`). */
static u32 module_file_order(const char *name, u32 source)
{
    char path[sizeof(package_modules_path) + CATALOG_NAME_SIZE];
    u8 header[GRM_HEADER];
    u32 have = 0, order;
    long got;
    if (!join(path, sizeof(path), source ? modules_path : package_modules_path, name)) return ORDER_UNKNOWN;
    long fd = syscall6(5, (u32)path, 0x8800, 0, 0, 0, 0);
    if (fd < 0) return ORDER_UNKNOWN;
    while (have < GRM_HEADER && (got = syscall6(3, fd, (u32)(header + have), GRM_HEADER - have, 0, 0, 0)) > 0)
        have += (u32)got;
    syscall6(6, fd, 0, 0, 0, 0, 0);
    if (have < GRM_HEADER || grm_word(header) != GRM_MAGIC) return ORDER_UNKNOWN;
    order = grm_word(header + GRM_ORDER);
    return order <= GRM_MAX_ORDER ? order : ORDER_UNKNOWN;
}
/* A module file is <ID>.GRM. Keep the names in their default order, so SCAN
   and INSTALL take the modules in the same order. */
/* The module files past GRKIT_MODULES_MAX, for REPORT.TXT. A full list keeps the
   package modules, then the first other files in the default order, so the
   result does not depend on the directory order. */
static u32 module_overflow;
/* The folder of each listed module: 0 for the package folder, 1 for
   GRKIT/MODULES. A module file of GRKIT/MODULES replaces the package module
   with the same name. */
static u32 module_source[GRKIT_MODULES_MAX];
static void offer_module(const char *name, u32 n, void *context)
{
    u32 source = context != 0;
    if (!module_file_name(name, n)) return;
    char file[CATALOG_NAME_SIZE];
    name_copy(file, name, n);
    if (!source && !package_module(file)) return;
    /* A file of GRKIT/MODULES replaces the package module of its name, with
       its own order. */
    for (u32 k = 0; k < module_count; ++k)
        if (same_name(module_names[k], file)) {
            if (!source) return;
            for (u32 j = k; j + 1 < module_count; ++j) {
                name_copy(module_names[j], module_names[j + 1], length(module_names[j + 1]));
                module_source[j] = module_source[j + 1];
                module_order[j] = module_order[j + 1];
            }
            --module_count;
            break;
        }
    u32 order = module_file_order(file, source);
    if (module_count == GRKIT_MODULES_MAX) {
        /* The file that leaves: the last one of the person, else the last
           package file. A file of the person never takes the place of a
           package file. */
        u32 out = GRKIT_MODULES_MAX - 1;
        for (u32 k = GRKIT_MODULES_MAX; k-- > 0;)
            if (module_source[k]) { out = k; break; }
        ++module_overflow;
        if (module_source[out] < source ||
            (module_source[out] == source && !module_after(module_order[out], module_names[out], order, file))) return;
        for (u32 j = out; j + 1 < module_count; ++j) {
            name_copy(module_names[j], module_names[j + 1], length(module_names[j + 1]));
            module_source[j] = module_source[j + 1];
            module_order[j] = module_order[j + 1];
        }
        --module_count;
    }
    u32 at = module_count++;
    while (at && module_after(module_order[at - 1], module_names[at - 1], order, file)) {
        name_copy(module_names[at], module_names[at - 1], length(module_names[at - 1]));
        module_source[at] = module_source[at - 1];
        module_order[at] = module_order[at - 1];
        --at;
    }
    name_copy(module_names[at], name, n);
    module_source[at] = source;
    module_order[at] = order;
}
/* The ID of module i: its file name before the dot, in capitals, at most 8
   characters (module_file_name). */
static void module_id(u32 i, char id[9])
{
    u32 n = 0;
    for (const char *p = module_names[i]; *p && *p != '.' && n < 8; ++p)
        id[n++] = *p >= 'a' && *p <= 'z' ? (char)(*p - 32) : *p;
    id[n] = 0;
}
/* Read and check module i into `module`. Return 0 or a status. */
static int read_module(u32 i)
{
    char path[sizeof(package_modules_path) + CATALOG_NAME_SIZE];
    if (!join(path, sizeof(path), module_source[i] ? modules_path : package_modules_path, module_names[i])) return GRM_BAD_FILE;
    long fd = syscall6(5, (u32)path, 0x8800, 0, 0, 0, 0);
    if (fd < 0) return GRM_BAD_FILE;
    u32 have = 0; long got = 0; u8 extra;
    while (have < sizeof(module_file)) {
        got = syscall6(3, fd, (u32)(module_file + have), sizeof(module_file) - have, 0, 0, 0);
        if (got <= 0) break;
        have += (u32)got;
    }
    if (got >= 0 && have == sizeof(module_file)) got = syscall6(3, fd, (u32)&extra, 1, 0, 0, 0) ? -1 : 0;
    syscall6(6, fd, 0, 0, 0, 0, 0);
    if (got < 0) return GRM_BAD_FILE;
    int status = grm_parse(module_file, have, &module);
    if (status) return status;
    /* The file name before the dot is the module ID. */
    char id[9];
    module_id(i, id);
    return text_equal(id, module.id) ? 0 : GRM_BAD_FILE;
}
/* The state file ID of CORE.<key>.<ID> for module i. */
static u32 manage_id(u32 i, const char *key)
{
    char name[GRM_NAME + 1];
    u32 n = 0;
    for (const char *p = "CORE."; *p; ++p) name[n++] = *p;
    for (const char *p = key; *p; ++p) name[n++] = *p;
    name[n++] = '.';
    char id[9];
    module_id(i, id);
    for (const char *p = id; *p; ++p) name[n++] = *p;
    name[n] = 0;
    return grm_state_id(name);
}
/* Read the choices of the GRKit section. The list keeps the default order,
   which the GRKit section of MENU shows. The core orders the GRKit root of
   ADJ by the positions (runtime/manage.h: manage_key). */
static void order_modules(void)
{
    u32 value;
    all_off = state_value(&saved_state, grm_state_id(MANAGE_SWITCH_KEY), &value) && value == 1;
    for (u32 i = 0; i < module_count; ++i) {
        module_off[i] = state_value(&saved_state, manage_id(i, "OFF"), &value) && value == 1;
        module_position[i] = state_value(&saved_state, manage_id(i, "POS"), &value) && value <= MANAGE_MODULES ? value : 0;
        module_rows[i] = state_value(&saved_state, manage_id(i, "ROWS"), &value) && value < 1U << MANAGE_OPTION_VALUES ? value : 0;
    }
}
/* Whether a setting of this start takes the saved value of `id`: a
   capability that a usable module uses, a saved setting of a usable module,
   or a choice of the GRKit section. The Fn adjustments are not kept. */
static int state_taken(u32 id)
{
    static const u32 core_ids[] = {CORE_STATE_IDS};
    if (id >= PARAMETER_KEY_BASE) return 1;
    for (u32 k = 0; k < GRKIT_CORE_SETTING_COUNT; ++k) {
        if (core_ids[k] != id) continue;
        for (u32 i = 0; i < module_count; ++i) {
            if (!(module_usable >> i & 1)) continue;
            for (u32 u = 0; u < module_use_count[i]; ++u) {
                const char *name = module_uses[i][u];
                if (text_equal(name + (name[0] == '?'), core_setting_names[k])) return 1;
            }
        }
    }
    for (u32 k = 0; k < setting_total; ++k)
        if ((setting_flags[k] & GRM_SETTING_SAVED) && grm_state_id(setting_names[k]) == id) return 1;
    for (u32 i = 0; i < module_count; ++i)
        if (manage_id(i, "OFF") == id || manage_id(i, "POS") == id || manage_id(i, "ROWS") == id) return 1;
    if (grm_state_id(MANAGE_SWITCH_KEY) == id) return 1;
    for (u32 i = 0; i < listed_count; ++i)
        if (effect_choice_id(i) == id) return 1;
    return 0;
}
/* The saved values that the table keeps, at most `room`. A later pair of a
   key wins. */
static void plan_kept(u32 room)
{
    kept_count = 0;
    for (u32 p = 0; p < saved_state.pairs; ++p) {
        u32 id = saved_state.ids[p], value = saved_state.values[p], k = 0;
        if (!id || state_taken(id)) continue;
        while (k < kept_count && kept_ids[k] != id) ++k;
        if (k == kept_count) {
            if (kept_count == room) continue;
            kept_ids[kept_count++] = id;
        }
        kept_values[k] = value;
    }
    /* A key with the value 0 has no entry in the file. */
    for (u32 k = 0; k < kept_count;)
        if (kept_values[k]) ++k;
        else { kept_ids[k] = kept_ids[--kept_count]; kept_values[k] = kept_values[kept_count]; }
}
/* List the modules and find the usable ones and the memory that they need. */
static void plan_modules(void)
{
    module_count = module_usable = module_words = module_code_bytes = setting_total = module_tiles = kept_count = 0;
    all_off = 0;
    module_overflow = 0;
    module_contexts = module_data_words = 0;
    for (u32 i = 0; i < GRKIT_MODULES_MAX; ++i) {
        module_status[i] = module_off[i] = module_position[i] = module_rows[i] = module_title[i] = 0;
        module_code_from[i] = module_code_to[i] = module_capabilities[i] = 0;
        module_detail[i][0] = 0;
        module_tile[i] = MANAGE_NO_TILE;
    }
    read_state();
    read_package_list();
    /* The package modules, then the modules of the person. */
    for (u32 source = 0; source < 2; ++source) {
        long fd = syscall6(5, (u32)(source ? modules_path : package_modules_path), 0xc000, 0, 0, 0, 0);
        if (fd < 0) continue;
        static const struct directory_calls calls = {directory_read, directory_seek};
        int listed = directory_list(fd, directory_entries, sizeof(directory_entries), &calls, offer_module, (void *)source);
        syscall6(6, fd, 0, 0, 0, 0, 0);
        if (listed) { module_count = 0; return; }
    }
    if (!module_count) return;
    order_modules();
    /* The module array: the settings table with the choices of the GRKit
       section of MENU and the kept values, the contexts, the module list,
       the effect list, the services, then the data of each module. The
       effects are listed later (discover), so the modules leave room for
       each of them. plan_array counts the same parts. */
    u32 contexts = 0, data = 0, choices = CHOICE_ENTRIES(module_count, EFFECTS_LISTED);
    u32 list = MANAGE_WORDS(module_count) + EFFECT_LIST_WORDS(EFFECTS_LISTED) + SERVICES_WORDS;
    for (u32 i = 0; i < module_count; ++i) {
        struct grm_setting settings[GRM_SETTINGS];
        u32 n = 0, uses = 0, tiles = 0;
        const u8 *tile_data;
        /* A module that is Off loads too, stopped (runtime/manage.h), so that
           Enable or the switch turns it on at once. Its code does not run
           while it is Off. */
        int status = safe_mode ? (int)MANAGE_SAFE : read_module(i);
        if (!status) status = grm_settings(&module, module.code, 0, settings, &n, module_uses[i], &uses);
        if (!status) status = grm_tiles(&module, module.code, 0, &tile_data, &tiles);
        if (!status && module_tiles + tiles > GRKIT_MODULE_TILES_MAX) status = GRM_NO_ROOM;
        u32 words = (module.data_size + module.bss_size) / 4;
        if (!status && (setting_total + n + choices > GRKIT_MODULE_SETTINGS_MAX ||
                        SETTINGS_WORDS(setting_total + n + choices) + contexts + CONTEXT_WORDS(uses) + list + data + words > GRKIT_MODULE_ARRAY_WORDS ||
                        module_code_bytes + module.code_size > MODULE_CODE_LIMIT)) status = GRM_NO_ROOM;
        if (status) {
            module_status[i] = (u32)status;
            print("GR_MODULE_SKIPPED "); print(module_names[i]); print(" "); number((u32)status); print("\n");
            continue;
        }
        module_usable |= 1U << i;
        module_tile_first[i] = module_tiles;
        module_tiles += tiles;
        module_first[i] = setting_total;
        module_settings[i] = n;
        module_use_count[i] = uses;
        for (u32 k = 0; k < n; ++k, ++setting_total) {
            u32 a = length(module.id), b = length(settings[k].key);
            name_copy(setting_names[setting_total], module.id, a);
            setting_names[setting_total][a] = '.';
            name_copy(setting_names[setting_total] + a + 1, settings[k].key, b);
            setting_values[setting_total] = settings[k].values;
            setting_flags[setting_total] = settings[k].flags;
        }
        contexts += CONTEXT_WORDS(uses);
        data += words;
        module_code_bytes += module.code_size;
    }
    module_contexts = contexts;
    module_data_words = data;
}
/* The words of the module array, after the effects are listed: the kept
   values fill the rest of the table and of the array. */
static void plan_array(void)
{
    module_words = kept_count = 0;
    if (!module_count) return;
    u32 entries = setting_total + CHOICE_ENTRIES(module_count, listed_count);
    u32 fixed = SETTINGS_WORDS(entries) + module_contexts + MANAGE_WORDS(module_count) +
                EFFECT_LIST_WORDS(listed_count) + SERVICES_WORDS + module_data_words;
    u32 room = GRKIT_MODULE_SETTINGS_MAX - entries;
    if (room > (GRKIT_MODULE_ARRAY_WORDS - fixed) / 3) room = (GRKIT_MODULE_ARRAY_WORDS - fixed) / 3;
    plan_kept(room);
    module_words = fixed + 3 * kept_count;
}
/* Whether a usable module uses CORE.TONE (plan_modules). */
static int tone_used(void)
{
    for (u32 i = 0; i < module_count; ++i) {
        if (!(module_usable >> i & 1)) continue;
        for (u32 u = 0; u < module_use_count[i]; ++u) {
            const char *name = module_uses[i][u];
            if (text_equal(name + (name[0] == '?'), "CORE.TONE")) return 1;
        }
    }
    return 0;
}
#endif
