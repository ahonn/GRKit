#ifndef GRKIT_INSTALL_STATUS_H
#define GRKIT_INSTALL_STATUS_H
/* The exit status of CORE.BIN. A person sees it in INSTALL.ERR, in
   GRKIT/REPORT.TXT, and after a module name on the Modules page, and reports
   it, so a value never changes meaning. 0 is success. The scripts of a
   package test STATUS_RELOCATION and STATUS_OTHER_BUILD
   (grkit_core::runtime::package_runtime_files). */
enum install_status {
    STATUS_OK = 0,
    /* /dev/mem does not open, or does not map. */
    STATUS_NO_MEMORY = 1,
    STATUS_NO_MAPPING = 2,
    /* Another SCAN or INSTALL attempt of this power-on runs or ran. */
    STATUS_BUSY = 3,
    /* The script arrays are not the ones that the plan asked for, or their
       memory is not valid. */
    STATUS_ARRAYS = 10,
    STATUS_ARRAY_MEMORY = 11,
    /* The camera has not set its language yet. INSTALL tries again. */
    STATUS_LANGUAGE_NOT_READY = 12,
    /* The Standard record of Image Control changed since startup. */
    STATUS_STOCK_RECORD = 13,
    /* The Standard record is not initialized yet. INSTALL tries again. */
    STATUS_RECORD_NOT_READY = 14,
    /* The firmware at a hook site is not the stock code. */
    STATUS_STOCK_BYTES = 16,
    /* The Image Control tables changed since startup. */
    STATUS_TONE_TABLES = 17,
    /* A cache sync of written memory failed. */
    STATUS_SYNC = 21,
    /* A patch did not publish; the earlier patches were rolled back, or the
       rollback failed too. */
    STATUS_ROLLED_BACK = 23,
    STATUS_ROLLBACK_FAILED = 24,
    /* The icon drawer does not exist yet. INSTALL tries again. */
    STATUS_ICONS_NOT_READY = 26,
    /* The effects library does not list (DIRECTORY_UNREADABLE and
       DIRECTORY_BAD_RECORD of runtime/directory.h). */
    STATUS_LIBRARY_UNREADABLE = 30,
    STATUS_LIBRARY_BAD_RECORD = 31,
    /* SCAN could not write the plan, or INSTALL found no valid plan or one
       that does not match the modules and effects on the card. */
    STATUS_PLAN_WRITE = 36,
    STATUS_PLAN = 37,
    /* The native settings are not initialized yet. INSTALL tries again. */
    STATUS_SETTINGS_NOT_READY = 39,
    /* The command line names no known mode. */
    STATUS_USAGE = 40,
    /* Diagnostic builds: no GRKit header, or no effect with the ID. */
    STATUS_NO_HEADER = 41,
    STATUS_NO_EFFECT = 42,
    /* The firmware keeps data where the core goes. */
    STATUS_TAIL_USED = 45,
    /* The core does not relocate to the running firmware build, or (--known)
       the core does not name that build. */
    STATUS_RELOCATION = 47,
    STATUS_OTHER_BUILD = 48,
    /* 50 to 56: the module statuses of runtime/module_link.h (GRM_*). */
    /* The loader stopped or waits: SCAN wrote no plan, the plan header is not
       valid, or INSTALL does not answer. The loader writes this status to
       INSTALL.ERR itself, with its attempts on the next line
       (grkit_core::runtime::loader). */
    STATUS_LOADER_NO_PLAN = 60,
    STATUS_LOADER_BAD_PLAN = 61,
    STATUS_LOADER_NO_INSTALL = 62,
};
#endif
