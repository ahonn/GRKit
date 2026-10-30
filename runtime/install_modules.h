#ifndef GRKIT_INSTALL_MODULES_H
#define GRKIT_INSTALL_MODULES_H
/* A part of the installer: the link of the modules and the module array.
   installer.c includes it once, in its order, after the helpers and the state
   that this part uses. */
/* The header entries of the module pages and layers (runtime/abi.h): three
   words each. Layers stay in painting order. */
static u32 module_pages[GRKIT_MODULE_PAGES_MAX * 3], module_page_count;
/* The plane filters in stage order (GRKIT_HOST_MODULE_FILTERS), two words
   each. */
static u32 module_filters[GRKIT_MODULE_FILTERS_MAX * 2], module_filter_count;
static u32 module_layers[GRKIT_MODULE_LAYERS_MAX * 3], module_layer_count;
static u8 module_code[GRM_MAX_SEGMENT], module_data[GRM_MAX_SEGMENT];
/* Insert the `width`-word `entry` into `table`, which holds `*count` entries
   in the order of the high half of their word `key`: after the entries whose
   high half is the same or lower. */
static void insert_sorted(u32 *table, u32 *count, u32 width, u32 key, const u32 *entry)
{
    u32 at_slot = (*count)++;
    for (; at_slot && table[(at_slot - 1) * width + key] >> 16 > entry[key] >> 16; --at_slot)
        for (u32 w = 0; w < width; ++w) table[at_slot * width + w] = table[(at_slot - 1) * width + w];
    for (u32 w = 0; w < width; ++w) table[at_slot * width + w] = entry[w];
}
/* The handle of a setting that a page or a layer of module i names
   (struct grm_layer): its own setting, or the handle of a used one. */
static u32 module_handle(u32 i, const u32 *use_handles, u32 setting)
{
    if (setting == GRM_NO_SETTING) return GRM_NO_SETTING;
    return setting & GRM_USED ? use_handles[setting & ~GRM_USED] : module_first[i] + setting;
}
/* The number of values of a setting handle. */
static u32 handle_values(u32 handle)
{
    if (IS_CORE_SETTING(handle)) return core_setting_values[CORE_SETTING(handle)];
    return handle < setting_total ? setting_values[handle] : 0;
}
/* The modules whose settings each loaded module uses, a bit each. */
static u32 module_needs[GRKIT_MODULES_MAX];
/* The module that has setting `k` of the table, or GRKIT_MODULES_MAX. */
static u32 setting_module(u32 k)
{
    for (u32 m = 0; m < module_count; ++m)
        if ((module_usable >> m & 1) && k - module_first[m] < module_settings[m]) return m;
    return GRKIT_MODULES_MAX;
}
/* One pass of install_modules. The modules of `broken` failed in an earlier
   pass: they keep their status and their memory, and their settings are
   missing. */
static int install_pass(u32 code, u32 array, u32 pixels, const u32 *icon_bases, u32 broken)
{
    struct grm_symbol symbols[CORE_EXPORT_COUNT + 1];
    u32 n = 0, code_at = code;
    for (; n < CORE_EXPORT_COUNT; ++n) {
        symbols[n].hash = core_exports[n * 2];
        symbols[n].value = relocated[TAIL_SYMBOL] + core_exports[n * 2 + 1];
    }
    /* grkit_self: the context of the module that links. */
    u32 self = n++;
    symbols[self].hash = fnv((const u8 *)"grkit_self", sizeof("grkit_self") - 1);
    /* The settings table. A saved setting has its state file ID; a second
       setting with the same ID is not saved. The choices of the GRKit
       section, those of the effects, and the kept values follow the settings
       of the modules. */
    u32 choices = CHOICE_ENTRIES(module_count, listed_count), total = setting_total + choices + kept_count;
    store(array, total);
    store(array + 4, 0);
    for (u32 k = 0; k < setting_total; ++k) {
        u32 entry = array + 4 * SETTINGS_ENTRY(k), id = 0;
        if (setting_flags[k] & GRM_SETTING_SAVED) {
            id = grm_state_id(setting_names[k]);
            for (u32 j = 0; j < k; ++j)
                if (word(array + 4 * SETTINGS_ENTRY(j)) == id) {
                    print("GR_MODULE_SETTING_UNSAVED "); print(setting_names[k]); print("\n");
                    id = 0;
                }
        }
        store(entry, id);
        store(entry + 4, 0);
        store(entry + 8, setting_values[k]);
    }
    for (u32 i = 0; i < module_count; ++i) {
        u32 entry = array + 4 * SETTINGS_ENTRY(setting_total + MANAGE_PER_MODULE * i);
        store(entry, manage_id(i, "OFF"));
        store(entry + 4, module_off[i]);
        store(entry + 8, 2);
        store(entry + 12, manage_id(i, "POS"));
        store(entry + 16, module_position[i]);
        store(entry + 20, MANAGE_MODULES + 1);
        store(entry + 24, manage_id(i, "ROWS"));
        store(entry + 28, module_rows[i]);
        store(entry + 32, 1U << MANAGE_OPTION_VALUES);
    }
    for (u32 i = 0; i < listed_count; ++i) {
        u32 entry = array + 4 * SETTINGS_ENTRY(setting_total + MANAGE_PER_MODULE * module_count + i);
        store(entry, effect_choice_id(i));
        store(entry + 4, listed_choice[i]);
        store(entry + 8, EFFECT_CHOICE_VALUES);
    }
    /* The GRKit switch follows the choices of the effects. */
    u32 switch_handle = setting_total + choices - 1;
    {
        u32 entry = array + 4 * SETTINGS_ENTRY(switch_handle);
        store(entry, grm_state_id(MANAGE_SWITCH_KEY));
        store(entry + 4, all_off);
        store(entry + 8, 2);
    }
    for (u32 k = 0; k < kept_count; ++k) {
        u32 entry = array + 4 * SETTINGS_ENTRY(setting_total + choices + k);
        store(entry, kept_ids[k]);
        store(entry + 4, kept_values[k]);
        store(entry + 8, kept_values[k] + 1);
    }
    u32 context_at = array + 4 * SETTINGS_WORDS(total), list_at = context_at;
    for (u32 i = 0; i < module_count; ++i)
        if (module_usable >> i & 1) list_at += 4 * CONTEXT_WORDS(module_use_count[i]);
    u32 effects_list = list_at + 4 * MANAGE_WORDS(module_count);
    u32 services = effects_list + 4 * EFFECT_LIST_WORDS(listed_count);
    u32 data_at = services + 4 * SERVICES_WORDS;
    u32 subscribers = 0;
    for (u32 k = 0; k < SERVICES_WORDS; ++k) store(services + 4 * k, 0);
    services_block = services;
    if (!sync_alias(array, list_at - array)) return STATUS_SYNC;
    module_page_count = module_layer_count = module_filter_count = capabilities_used = capabilities_running = 0;
    for (u32 i = 0; i < module_count; ++i) {
        if (!(module_usable >> i & 1) || read_module(i)) continue;
        u32 context = context_at, code_base = code_at, data_base = data_at, data_bytes = module.data_size + module.bss_size;
        context_at += 4 * CONTEXT_WORDS(module_use_count[i]);
        code_at += module.code_size;
        data_at += data_bytes;
        module_needs[i] = 0;
        if (broken >> i & 1) continue;
        u32 failed = 0, own_count = 0, own_uses = 0, layer_count = 0, page_count = 0, tile_count = 0;
        struct grm_setting own[GRM_SETTINGS];
        char own_names[GRM_USES][GRM_NAME + 1];
        struct grm_layer layers[GRM_REGISTRATIONS];
        struct grm_page pages[GRM_REGISTRATIONS];
        struct grm_filter filters[GRM_REGISTRATIONS];
        u32 filter_count = 0;
        const u8 *tiles = 0;
        int status = 0;
        /* The context: its first setting, their count, reserved, then the
           handle of each used setting: a capability of the core, or a
           setting of a module. A setting of another module must be public. */
        u32 use_handles[GRM_USES], points = 0, capabilities = 0;
        store(context, module_first[i]);
        store(context + 4, module_settings[i]);
        store(context + 8, 0);
        for (u32 u = 0; u < module_use_count[i]; ++u) {
            const char *name = module_uses[i][u];
            int optional = name[0] == '?';
            u32 handle = GRM_NO_SETTING;
            for (u32 k = 0; k < GRKIT_CORE_SETTING_COUNT && handle == GRM_NO_SETTING; ++k)
                if (text_equal(core_setting_names[k], name + optional)) {
                    handle = GRKIT_CORE_SETTING_BASE + k;
                    points |= core_setting_points[k];
                    capabilities |= 1U << k;
                }
            for (u32 k = 0; k < setting_total && handle == GRM_NO_SETTING; ++k) {
                u32 owner = setting_module(k);
                if (owner < GRKIT_MODULES_MAX && (broken >> owner & 1)) continue;
                if (text_equal(setting_names[k], name + optional) &&
                    ((setting_flags[k] & GRM_SETTING_PUBLIC) || owner == i)) {
                    handle = k;
                    if (owner != i && owner < GRKIT_MODULES_MAX) module_needs[i] |= 1U << owner;
                }
            }
            if (handle == GRM_NO_SETTING && !optional) { status = GRM_MISSING_SETTING; failed = u; }
            use_handles[u] = handle;
            store(context + 12 + u * 4, handle);
        }
        if (!sync_alias(context, 4 * CONTEXT_WORDS(module_use_count[i]))) return STATUS_SYNC;
        symbols[self].value = context;
        if (!status) status = grm_link(&module, module_code, module_data, code_base, data_base, symbols, n, &failed);
        if (!status) status = grm_settings(&module, module_code, code_base, own, &own_count, own_names, &own_uses);
        if (!status) status = grm_tiles(&module, module_code, code_base, &tiles, &tile_count);
        if (!status)
            status = grm_registrations(&module, module_code, code_base, own, own_count, own_uses,
                                       layers, &layer_count, pages, &page_count, filters, &filter_count,
                                       GRM_REGISTRATIONS);
        /* The handles of the pages and the layers. A page or a layer of an
           optional used setting that no module has is not registered. A
           page needs a row for each value of a used setting. */
        for (u32 k = 0; !status && k < page_count; ++k) {
            pages[k].setting = module_handle(i, use_handles, pages[k].setting);
            /* The core gives the rows of a page of CORE.TONE. */
            u32 core_rows = pages[k].flags & GRM_CHOICE_CORE_ROWS;
            /* The rows do not fit the setting: the page is not valid. */
            if (pages[k].setting != GRM_NO_SETTING &&
                (core_rows ? pages[k].setting != GRKIT_CORE_TONE : handle_values(pages[k].setting) != pages[k].rows))
                status = GRM_BAD_DESCRIPTOR;
            /* A settings row of a used setting needs a value text for each
               value of that setting. */
            const u8 *page = grm_code_at(&module, module_code, code_base, pages[k].address, GRM_PAGE);
            u32 rows = page ? grm_word(page + GRM_PAGE_SETTING_ROWS_AT) : 0;
            const u8 *list = rows ? grm_code_at(&module, module_code, code_base, grm_word(page + GRM_PAGE_SETTINGS_AT), rows * GRM_ROW) : 0;
            for (u32 r = 1; !status && list && r < rows; ++r) {
                const u8 *row = list + r * GRM_ROW;
                u32 named = grm_word(row + GRM_ROW_SETTING_AT), handle = module_handle(i, use_handles, named);
                if (!(named & GRM_USED) || handle == GRM_NO_SETTING) continue;
                u32 values = handle_values(handle);
                if (!values || values > GRM_SETTING_ROW_VALUES ||
                    !grm_texts(&module, module_code, code_base, grm_word(row + GRM_ROW_VALUES_AT), values * GRM_LANGUAGES))
                    status = GRM_BAD_DESCRIPTOR;
            }
        }
        for (u32 k = 0; !status && k < layer_count; ++k)
            if (layers[k].setting != GRM_NO_SETTING) {
                layers[k].setting = module_handle(i, use_handles, layers[k].setting);
                if (layers[k].setting == GRM_NO_SETTING) layers[k].paint = 0;
            }
        for (u32 k = 0; !status && k < filter_count; ++k) {
            filters[k].setting = module_handle(i, use_handles, filters[k].setting);
            if (filters[k].setting == GRM_NO_SETTING) filters[k].apply = 0;
        }
        if (!status && (module_layer_count + layer_count > GRKIT_MODULE_LAYERS_MAX ||
                        module_page_count + page_count > GRKIT_MODULE_PAGES_MAX ||
                        module_filter_count + filter_count > GRKIT_MODULE_FILTERS_MAX)) status = GRM_NO_ROOM;
        if (status) {
            module_status[i] = (u32)status;
            module_needs[i] = 0;
            if (status == GRM_MISSING_IMPORT) grm_import_name(&module, failed, module_detail[i], sizeof(module_detail[i]));
            if (status == GRM_MISSING_SETTING && failed < module_use_count[i])
                name_copy(module_detail[i], module_uses[i][failed], length(module_uses[i][failed]));
            print("GR_MODULE_SKIPPED "); print(module.id); print(" "); number((u32)status);
            if (status == GRM_MISSING_IMPORT || status == GRM_MISSING_SETTING) { print(" "); number(failed); }
            print("\n");
            continue;
        }
        copy(at(code_base), module_code, module.code_size);
        /* The native settings rows of each page get absolute setting handles
           (runtime/modules.h): row 0 the page setting, the others settings of
           the module or used settings. A row of an optional used setting
           that no module has is left out, and the rows after it move up. */
        for (u32 k = 0; k < page_count; ++k) {
            u32 rows = word(pages[k].address + GRM_PAGE_SETTING_ROWS_AT), list = word(pages[k].address + GRM_PAGE_SETTINGS_AT), kept = 0;
            if (pages[k].setting == GRM_NO_SETTING) continue;
            for (u32 r = 0; r < rows; ++r) {
                u32 row = list + r * GRM_ROW, to = list + kept * GRM_ROW;
                u32 handle = r ? module_handle(i, use_handles, word(row + GRM_ROW_SETTING_AT)) : pages[k].setting;
                if (handle == GRM_NO_SETTING) continue;
                for (u32 w = 0; to != row && w < GRM_ROW; w += 4) store(to + w, word(row + w));
                store(to + GRM_ROW_SETTING_AT, handle);
                ++kept;
            }
            if (kept != rows) store(pages[k].address + GRM_PAGE_SETTING_ROWS_AT, kept);
        }
        copy(at(data_base), module_data, module.data_size);
        zero(at(data_base + module.data_size), module.bss_size);
        if (!sync_alias(code_base, module.code_size)) return STATUS_SYNC;
        if (data_bytes && !sync_alias(data_base, data_bytes)) return STATUS_SYNC;
        /* The tiles, in the icon slots after the effect icons. */
        for (u32 k = 0; k < tile_count; ++k) {
            u32 slot = HOST_ICON_COUNT + count + module_tile_first[i] + k;
            u32 icon = icon_bases[slot / ICONS_PER_CHUNK] + (slot % ICONS_PER_CHUNK) * ICON_STRIDE;
            icon_header(icon, pixels);
            copy(at(icon + ICON_PIXELS), tiles + k * GRM_TILE_BYTES, GRM_TILE_BYTES);
            if (!sync_alias(icon, ICON_STRIDE)) return STATUS_SYNC;
        }
        /* Module management shows the title of the first page. */
        if (page_count) module_title[i] = word(pages[0].address + GRM_PAGE_TITLE_AT);
        if (tile_count) module_tile[i] = module_tile_first[i];
        /* The pages in load order, and the layers in painting order: a
           layer goes after the layers with the same or a lower order. */
        for (u32 k = 0; k < page_count; ++k) {
            if (pages[k].setting == GRM_NO_SETTING) continue;
            module_pages[module_page_count * 3] = pages[k].address;
            module_pages[module_page_count * 3 + 1] = pages[k].setting;
            module_pages[module_page_count * 3 + 2] = module_tile_first[i];
            ++module_page_count;
            points |= POINT_MENU_PAGE;
        }
        for (u32 k = 0; k < layer_count; ++k) {
            if (!layers[k].paint) continue;
            points |= POINT_VIEWFINDER_OVERLAY;
            const u32 entry[3] = {layers[k].paint, layers[k].setting, layers[k].order << 16 | layers[k].flags};
            insert_sorted(module_layers, &module_layer_count, 3, 2, entry);
        }
        /* The filters in stage order: a filter goes after the filters of the
           same or a lower stage. */
        for (u32 k = 0; k < filter_count; ++k) {
            if (!filters[k].apply) continue;
            points |= POINT_PICTURE_ENCODE;
            const u32 entry[2] = {filters[k].apply, filters[k].stage << 16 | filters[k].setting};
            insert_sorted(module_filters, &module_filter_count, 2, 1, entry);
        }
        /* The event subscriber: SYSTEM_READY needs the hook of the
           shooting screen. */
        struct grm_subscriber subscriber;
        if (grm_subscriber(&module, module_code, code_base, &subscriber) == 1 &&
            subscribers < SERVICES_SUBSCRIBERS_MAX) {
            store(services + SERVICES_SUBSCRIBERS + 8 * subscribers, subscriber.handle);
            store(services + SERVICES_SUBSCRIBERS + 8 * subscribers + 4, subscriber.events);
            store(services + SERVICES_SUBSCRIBER_COUNT, ++subscribers);
            if (subscriber.events & SERVICE_EVENT_SYSTEM_READY) points |= POINT_VIEWFINDER_OVERLAY;
        }
        /* The patches of the used capabilities and of the menu. */
        needed_points |= points;
        capabilities_used |= capabilities;
        module_code_from[i] = code_base;
        module_code_to[i] = code_base + module.code_size;
        module_capabilities[i] = capabilities;
        if (!module_stopped(i)) capabilities_running |= capabilities;
        print("GR_MODULE_LOADED "); print(module.id); print(" "); number(code_base); print("\n");
    }
    /* The module list of the Modules page. */
    module_list = list_at;
    store(list_at + MANAGE_COUNT, module_count);
    store(list_at + MANAGE_SETTINGS, setting_total);
    store(list_at + MANAGE_FLAGS, safe_mode ? MANAGE_SAFE_MODE : 0);
    store(list_at + MANAGE_OPEN, 0);
    store(list_at + MANAGE_SWITCH, switch_handle + 1);
    for (u32 i = 0; i < module_count; ++i) {
        u32 entry = list_at + MANAGE_HEADER + i * MANAGE_STRIDE, n = 0;
        volatile u16 *name = (volatile u16 *)at(entry + MANAGE_NAME);
        u32 status = module_status[i];
        store(entry + MANAGE_STATUS, status);
        store(entry + MANAGE_TITLE, status ? 0 : module_title[i]);
        store(entry + MANAGE_TILE, status ? MANAGE_NO_TILE : module_tile[i]);
        store(entry + MANAGE_CODE, status ? 0 : module_code_from[i]);
        store(entry + MANAGE_CODE_END, status ? 0 : module_code_to[i]);
        store(entry + MANAGE_USES, status ? 0 : module_capabilities[i]);
        char id[9];
        module_id(i, id);
        for (const char *p = id; *p; ++p) name[n++] = (u16)*p;
        /* An error shows after the name: "NAME (52)". */
        if (status > MANAGE_SAFE) {
            char digits[12]; u32 d = sizeof(digits);
            do { digits[--d] = (char)('0' + status % 10); status /= 10; } while (status);
            name[n++] = ' '; name[n++] = '(';
            while (d < sizeof(digits)) name[n++] = (u16)digits[d++];
            name[n++] = ')';
        }
        while (n < MANAGE_NAME_UNITS) name[n++] = 0;
    }
    if (!sync_alias(list_at, 4 * MANAGE_WORDS(module_count))) return STATUS_SYNC;
    /* The modules that are Off start stopped: the core calls none of their
       code (SERVICES_SUSPENDED). */
    {
        u32 stopped = 0;
        for (u32 i = 0; i < module_count && stopped < SERVICES_SUSPENDED_MAX; ++i) {
            if (module_status[i] || !module_stopped(i)) continue;
            store(services + SERVICES_SUSPENDED + 4 + 8 * stopped, module_code_from[i]);
            store(services + SERVICES_SUSPENDED + 8 + 8 * stopped, module_code_to[i]);
            ++stopped;
        }
        store(services + SERVICES_SUSPENDED, stopped);
    }
    /* The effect list of the GRKit section of MENU, with the record of each
       installed effect. */
    effect_list = listed_count ? effects_list : 0;
    store(effects_list + EFFECTS_COUNT, listed_count);
    store(effects_list + EFFECTS_SETTINGS, setting_total + MANAGE_PER_MODULE * module_count);
    for (u32 i = 0; i < listed_count; ++i) {
        u32 entry = effects_list + EFFECTS_HEADER + i * EFFECTS_STRIDE, n = 0;
        volatile u16 *name = (volatile u16 *)at(entry + EFFECTS_NAME);
        store(entry + EFFECTS_STATUS, listed_status[i]);
        store(entry + EFFECTS_RECORD, listed_record[i]);
        for (; n < CATALOG_EFFECT_NAME_MAX && listed[i][CATALOG_EFFECT_NAME + n]; ++n) name[n] = listed[i][CATALOG_EFFECT_NAME + n];
        while (n < EFFECTS_NAME_UNITS) name[n++] = 0;
    }
    if (listed_count && !sync_alias(effects_list, 4 * EFFECT_LIST_WORDS(listed_count))) return STATUS_SYNC;
    if (!sync_alias(services, 4 * SERVICES_WORDS)) return STATUS_SYNC;
    return 0;
}
/* Link and copy each usable module: its code at `code` and after, its data
   in the script array `array`. A module that does not link is skipped; its
   memory stays reserved. A setting of a module that does not link is
   missing, also for a module that linked before it. So if a loaded module
   uses a setting of a module that failed, the pass runs again with the
   failed modules kept out. Each new pass has more failed modules, so the
   passes end. The addresses of each module stay the same in each pass. */
static int install_modules(u32 code, u32 array, u32 pixels, const u32 *icon_bases)
{
    u32 points = needed_points, broken = 0;
    for (;;) {
        needed_points = points;
        int status = install_pass(code, array, pixels, icon_bases, broken);
        if (status) return status;
        u32 failed = 0, stale = 0;
        for (u32 i = 0; i < module_count; ++i)
            if ((module_usable >> i & 1) && module_status[i]) failed |= 1U << i;
        for (u32 i = 0; i < module_count; ++i)
            if ((module_usable >> i & 1) && !module_status[i] && (module_needs[i] & failed)) stale = 1;
        if (!stale) return 0;
        broken = failed;
    }
}
#endif
