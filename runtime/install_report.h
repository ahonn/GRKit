#ifndef GRKIT_INSTALL_REPORT_H
#define GRKIT_INSTALL_REPORT_H
/* A part of the installer: GRKIT/REPORT.TXT. installer.c includes it once, in
   its order, after the helpers and the state that this part uses.

   The result of each boot for a person: the modules and why one did not
   load, the effects, and the safe mode. INSTALL writes it only when its text
   changes, in place: on GR IV exFAT a rename that replaces a file made the
   later requests of the camera fail. */
#define REPORT_MAX 8192U
static char report_text[REPORT_MAX + 1], report_old[REPORT_MAX + 1];
static u32 report_length;
static void say(const char *text)
{
    while (*text && report_length < REPORT_MAX) report_text[report_length++] = *text++;
}
static void say_number(u32 value)
{
    char digits[12]; u32 n = sizeof(digits) - 1;
    digits[n] = 0;
    do { digits[--n] = (char)('0' + value % 10); value /= 10; } while (value);
    say(digits + n);
}
/* Why a module did not load (runtime/module_link.h). */
static const char *module_reason(u32 status)
{
    switch (status) {
    case GRM_BAD_FILE: return "it is not a module file that this GRKit reads";
    case GRM_BAD_ABI: return "it needs another version of GRKit";
    case GRM_MISSING_IMPORT: return "it calls a function that GRKit does not have: ";
    case GRM_BAD_DESCRIPTOR: return "its description is not valid";
    case GRM_NO_ROOM: return "the modules need more memory than GRKit has";
    case GRM_MISSING_SETTING: return "it uses a setting that no module has: ";
    default: return "INSTALL stopped";
    }
}
static void write_report(int status)
{
    report_length = 0;
    say("GRKit "); say(GRKIT_VERSION); say(" ("); say(target_marker + sizeof("GRKIT_TARGET=") - 1); say(")\r\n");
    if (status) { say("\r\nGRKit did not install: error "); say_number((u32)status); say(".\r\n"); }
    if (safe_mode) say("\r\nSafe mode: GRKIT\\SAFE exists, so no module loads. Delete it to load the modules.\r\n");
    if (profile_mode) say("\r\nProfile: GRKIT\\PROFILE exists, so GRKit writes its CPU use to GRKIT\\PROFILE.LOG. Delete it to stop.\r\n");
    say("\r\nModules (the package, and GRKIT\\MODULES):\r\n");
    if (!module_count) say("none\r\n");
    for (u32 i = 0; i < module_count; ++i) {
        say(module_names[i]);
        if (module_source[i]) say(" (GRKIT\\MODULES)");
        u32 result = module_status[i];
        if (!result && (module_usable >> i & 1))
            say(module_off[i] ? "  loaded, stopped: Enable is Off in the GRKit section of MENU\r\n" :
                all_off ? "  loaded, stopped: Enable GRKit is Off in the GRKit section of MENU\r\n" : "  loaded\r\n");
        else if (result == MANAGE_SAFE) say("  not loaded: safe mode\r\n");
        else {
            say("  not loaded: error "); say_number(result); say(", "); say(module_reason(result));
            say(module_detail[i]); say("\r\n");
        }
    }
    if (module_overflow) {
        say_number(module_overflow);
        say(" more module files are not loaded. GRKit loads at most "); say_number(GRKIT_MODULES_MAX); say(" modules.\r\n");
    }
    say("\r\nEffects (GRKIT\\EFFECTS):\r\n");
    if (!listed_count) say(tone_used() ? "none\r\n" : "none: no module uses them\r\n");
    for (u32 i = 0; i < listed_count; ++i) {
        char name[CATALOG_EFFECT_NAME_MAX + 1]; u32 k = 0;
        while (k < CATALOG_EFFECT_NAME_MAX && listed[i][CATALOG_EFFECT_NAME + k]) { name[k] = (char)listed[i][CATALOG_EFFECT_NAME + k]; ++k; }
        name[k] = 0;
        say(name); say(" ("); say(names[i]); say(")");
        if (listed_status[i] == EFFECT_LOADED) say("  loaded\r\n");
        else if (listed_status[i] == EFFECT_OFF && listed_record[i] != EFFECTS_NO_RECORD)
            say("  loaded, hidden: Off in the GRKit section of MENU\r\n");
        else if (listed_status[i] == EFFECT_OFF) say("  not loaded: Off in the GRKit section of MENU\r\n");
        else if (listed_status[i] == EFFECT_OVER) { say("  not loaded: GRKit installs at most "); say_number(MAX_COUNT); say(" effects\r\n"); }
        else say("  not loaded: the file changed during the start\r\n");
    }
    if (overflow_count) {
        say_number(overflow_count);
        say(" more effect files are not listed. GRKit lists at most "); say_number(EFFECTS_LISTED); say(" effects.\r\n");
    }
    /* A new card has no GRKIT folder until this start. */
    long made = syscall6(39, (u32)data_path, 0755, 0, 0, 0, 0);
    if (made < 0 && made != -17) return;
    long had = read_text(report_path, (u8 *)report_old, sizeof(report_old));
    if (had == (long)report_length && equal((const u8 *)report_old, (const u8 *)report_text, report_length)) return;
    write_file(report_path, (const u8 *)report_text, report_length);
}
#endif
