#ifndef GRKIT_DEVELOP_SETTINGS_H
#define GRKIT_DEVELOP_SETTINGS_H
/* The settings of a RAW development. At its start, a development takes a
   copy of the settings that change a picture and that it does not take from
   the DNG: each module setting, CORE.GRAIN, CORE.GRAIN_SIZE, CORE.BORDER,
   and CORE.BORDER_COLOR (SERVICES_DEVELOP). While it runs, setting_value and
   setting_store use the copy, and the picture stages read it: a change in
   the development changes only that development, and the shooting settings
   stay as they are. Film Look, the Framing ratio, and the crop have values
   of their own in a development (runtime/develop.h, runtime/framing.c).
   The includer includes runtime/setting_table.h. */
#include "services.h"
/* Whether a RAW development runs. A host test defines it. */
#ifndef DEVELOP_COPY_RUNNING
#ifdef FW_DEVELOP_MODEL
#define DEVELOP_COPY_RUNNING() ((BYTE(FW_DEVELOP_GUARD,0)&1U) && BYTE(FW_DEVELOP_MODEL,DEVELOP_STATE))
#else
#define DEVELOP_COPY_RUNNING() 0
#endif
#endif
/* The header words of the core settings in the copy. */
static __attribute__((unused)) const u16 develop_copy_words[DEVELOP_COPY_HEADER_WORDS]={
    GRKIT_HOST_GRAIN,GRKIT_HOST_GRAIN_SIZE,GRKIT_HOST_BORDER,GRKIT_HOST_BORDER_COLOR};
/* The copy while a development runs, else 0. A copy is valid from the first
   development start after INSTALL. */
static __attribute__((unused)) volatile u32 *develop_copy(volatile u32 *h) {
    volatile u32 *c;
    if(!h[GRKIT_HOST_SERVICES/4]) return 0;
    c=services_word(h,SERVICES_DEVELOP);
    return c[DEVELOP_COPY_MARK/4]==DEVELOP_COPY_VALID && DEVELOP_COPY_RUNNING() ? c : 0;
}
/* The word of the copy for header word `offset`, or 0. */
static __attribute__((unused)) volatile u32 *develop_copy_header(volatile u32 *h,u32 offset) {
    volatile u32 *c=develop_copy(h);
    u32 i;
    if(!c) return 0;
    for(i=0;i<DEVELOP_COPY_HEADER_WORDS;++i)
        if(develop_copy_words[i]==offset) return c+DEVELOP_COPY_HEADER/4+i;
    return 0;
}
/* The word of the copy for setting `handle`, or 0 for a setting that a
   development does not copy. */
static __attribute__((unused)) volatile u32 *develop_copy_setting(volatile u32 *h,u32 handle) {
    volatile u32 *c;
    if(IS_CORE_SETTING(handle)) {
        u32 word=core_setting_words[CORE_SETTING(handle)];
        return word ? develop_copy_header(h,word) : 0;
    }
    if(handle>=GRKIT_MODULE_SETTINGS_MAX || !(c=develop_copy(h))) return 0;
    return c+DEVELOP_COPY_MODULE/4+handle;
}
/* The start of a development: the copy of the current settings. The table
   is that of runtime/settings.h (`t`, or 0 without one). */
static __attribute__((unused)) void develop_copy_start(volatile u32 *h,volatile u32 *t) {
    volatile u32 *c;
    u32 i;
    if(!h[GRKIT_HOST_SERVICES/4]) return;
    c=services_word(h,SERVICES_DEVELOP);
    c[DEVELOP_COPY_ASPECT/4]=0;
    for(i=0;i<DEVELOP_COPY_HEADER_WORDS;++i) c[DEVELOP_COPY_HEADER/4+i]=h[develop_copy_words[i]/4];
    for(i=0;i<GRKIT_MODULE_SETTINGS_MAX;++i) c[DEVELOP_COPY_MODULE/4+i]=t && i<t[0] ? t[SETTINGS_ENTRY(i)+1] : 0;
    c[DEVELOP_COPY_MARK/4]=DEVELOP_COPY_VALID;
}
#endif
