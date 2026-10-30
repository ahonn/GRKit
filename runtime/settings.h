#ifndef GRKIT_SETTINGS_H
#define GRKIT_SETTINGS_H
/* The settings of the modules (runtime/sdk/grkit.h). INSTALL writes the table
   at the start of the module script array, and GRKIT_HOST_MODULE_SETTINGS
   holds its address (runtime/setting_table.h has the layout). The resident
   loader restores the saved values at boot and saves them after the count
   of changes changes (crates/grkit-core/src/runtime/loader.rs).
   runtime/setting_store.h sets the settings. The includer
   defines u8, u16, u32, and the header fields (runtime/abi.h), and the
   functions that runtime/aspect.h names. */
#include "setting_table.h"
/* CORE.TONE: its value is the active record (runtime/tone.h). CORE.ASPECT:
   its value comes from the native aspect and the crop (runtime/aspect.h). */
#include "tone.h"
#include "aspect.h"
#ifndef MODULE_SETTINGS_MAP
#define MODULE_SETTINGS_MAP(address) ((volatile u32 *)(address))
#endif
/* The table, or 0 without one or with a count out of range. */
static __attribute__((unused)) volatile u32 *settings_table(volatile u32 *h) {
    u32 address=h[GRKIT_HOST_MODULE_SETTINGS/4];
    volatile u32 *t=address ? MODULE_SETTINGS_MAP(address) : 0;
    return t && t[0]<=GRKIT_MODULE_SETTINGS_MAX ? t : 0;
}
/* The number of values of a setting, or 0 for an unknown handle. */
static __attribute__((unused)) u32 setting_limit(volatile u32 *h,u32 handle) {
    volatile u32 *t;
    if(handle==GRKIT_CORE_TONE) return tone_values(h);
    if(IS_CORE_SETTING(handle)) return core_setting_values[CORE_SETTING(handle)];
    t=settings_table(h);
    return t && handle<t[0] ? t[SETTINGS_ENTRY(handle)+2] : 0;
}
/* Whether the camera offers a value of a setting: each value of a module
   setting, and the ratios of the target for CORE.ASPECT. */
static __attribute__((unused)) u32 setting_offered(volatile u32 *h,u32 handle,u32 value) {
    if(value>=setting_limit(h,handle)) return 0;
    if(handle==GRKIT_CORE_ASPECT) return aspect_offered(value);
    return 1;
}
/* The rows of a choice page of a setting: the values that the camera
   offers, in the order of the page. A module setting lists its values in
   order, and CORE.ASPECT the rows of the target (FRAMING_RATIO_ROWS). */
static __attribute__((unused)) u32 setting_rows(volatile u32 *h,u32 handle) {
    return handle==GRKIT_CORE_ASPECT ? framing_ratios() : setting_limit(h,handle);
}
/* The value of a row. */
static __attribute__((unused)) u32 setting_row_value(volatile u32 *h,u32 handle,u32 row) {
    (void)h;
    return handle==GRKIT_CORE_ASPECT ? framing_ratio(row) : row;
}
/* The row of a value, or row 0 for a value that the camera does not offer. */
static __attribute__((unused)) u32 setting_value_row(volatile u32 *h,u32 handle,u32 value) {
    if(handle==GRKIT_CORE_ASPECT) return framing_row(value);
    return value<setting_limit(h,handle) ? value : 0;
}
/* The value of a setting, or 0 for an unknown handle or a value out of its
   range. */
static __attribute__((unused)) u32 setting_value(volatile u32 *h,u32 handle) {
    volatile u32 *t;
    u32 value;
    if(handle==GRKIT_CORE_TONE) return tone_value(h);
    /* CORE.RECIPE has no value: its rows are the recipes (runtime/recipes.h). */
    if(handle==GRKIT_CORE_RECIPE) return 0;
    if(IS_CORE_SETTING(handle)) {
        if(handle==GRKIT_CORE_ASPECT) return aspect_value(h);
        value=h[core_setting_words[CORE_SETTING(handle)]/4];
        return value<core_setting_values[CORE_SETTING(handle)] ? value : 0;
    }
    t=settings_table(h);
    if(!t || handle>=t[0]) return 0;
    value=t[SETTINGS_ENTRY(handle)+1];
    return value<t[SETTINGS_ENTRY(handle)+2] ? value : 0;
}
#endif
