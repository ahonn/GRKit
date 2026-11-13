#ifndef GRKIT_PERSIST_H
#define GRKIT_PERSIST_H
/* All producers of saved values run in the App task. This timer uses the
   same task, including while MENU or a native keyboard is open. Capture
   contains no firmware calls, so a dial transition cannot interleave it. */
#define PERSIST_POLL_MS 100U
#ifndef PERSIST_TIMER_START
#define PERSIST_TIMER_START(ms) ui_timer_start(ms)
#endif
#ifndef PERSIST_BARRIER
#define PERSIST_BARRIER() __asm__ volatile("dmb ish" ::: "memory")
#endif
static u32 persist_core_value(volatile u32 *h,u32 k);
static u32 persist_fn_value(volatile u32 *h,u32 record,u32 i);
static u32 persist_toning_value(volatile u32 *h,u32 record,u32 i);
static u32 persist_module_value(volatile u32 *h,volatile u32 *t,u32 i);
#define PERSIST_HALF(p,o) HALF(p,o)
#define PERSIST_BYTE(p,o) BYTE(p,o)
#define PERSIST_SETTINGS(h) settings_table(h)
#define PERSIST_CORE_VALUE(h,k) persist_core_value(h,k)
#define PERSIST_FN_VALUE(h,r,i) persist_fn_value(h,r,i)
#define PERSIST_TONING_VALUE(h,r,i) persist_toning_value(h,r,i)
#define PERSIST_MODULE_VALUE(h,t,i) persist_module_value(h,t,i)
#include "persist_view.h"
static u32 persist_core_value(volatile u32 *h,u32 k) {
    u32 value;
    if(!dial_word(h,DIAL_ACTIVE)[0] || !dial_core_holds(k)) return persist_live_core(h,k);
    value=dial_word(h,DIAL_BASE_CORE+4U*k)[0];
    if(GRKIT_CORE_SETTING_BASE+k==GRKIT_CORE_TONE) {
        if(!value || value>h[GRKIT_HOST_COUNT/4]) return 0;
        return HALF(tone_record(h,value),GRKIT_RECORD_CODE);
    }
    return value;
}
static u32 persist_fn_value(volatile u32 *h,u32 record,u32 i) {
    return dial_word(h,DIAL_ACTIVE)[0] ? dial_word(h,DIAL_BASE_FN+4U*i)[0] : persist_live_fn(record);
}
static u32 persist_toning_value(volatile u32 *h,u32 record,u32 i) {
    return dial_word(h,DIAL_ACTIVE)[0] ? ((volatile u8 *)dial_word(h,DIAL_BASE_TONING))[i] : BYTE(record,GRKIT_RECORD_TONING);
}
static u32 persist_module_value(volatile u32 *h,volatile u32 *t,u32 i) {
    return dial_word(h,DIAL_ACTIVE)[0] && dial_set_holds(h,t,i) ? dial_word(h,DIAL_BASE_SETTINGS+4U*i)[0] : t[SETTINGS_ENTRY(i)+1];
}
static void persist_poll(volatile u32 *h) {
    volatile u32 *out=services_word(h,SERVICES_PERSIST);
    volatile u32 *t=settings_table(h);
    u32 request=out[GRKIT_PERSIST_REQUEST/4], ready=out[GRKIT_PERSIST_READY/4];
    if(!request || request>0x7fffffffU || request==ready || ready!=out[GRKIT_PERSIST_RELEASE/4]) return;
    /* Acquire the release before reusing the buffer, then publish READY
       only after every payload word and the revision are visible. */
    PERSIST_BARRIER();
    /* Installed table sizes never change. Refuse a damaged view instead
       of publishing zeros that would erase previously saved values. */
    if(h[GRKIT_HOST_COUNT/4]>GRKIT_TEXT_RECORDS || !t) return;
    if(dial_word(h,DIAL_ACTIVE)[0] &&
       (dial_word(h,DIAL_BASE_RECORDS)[0]!=h[GRKIT_HOST_COUNT/4] ||
        dial_word(h,DIAL_BASE_COUNT)[0]!=(t ? t[0] : 0))) return;
    if(persist_capture(h,out)) {
        u32 revision=out[GRKIT_PERSIST_REVISION/4];
        out[GRKIT_PERSIST_REVISION/4]=revision>=0x7fffffffU ? 1U : revision+1U;
    }
    PERSIST_BARRIER();
    out[GRKIT_PERSIST_READY/4]=request;
}
static void persist_ready(volatile u32 *h) {
    volatile u32 *timer=services_word(h,SERVICES_PERSIST_TIMER);
    if(!h[GRKIT_HOST_PERSIST_WORD/4] || timer[0]) return;
    persist_poll(h);
    timer[0]=PERSIST_TIMER_START(PERSIST_POLL_MS);
}
static u32 persist_fired(volatile u32 *h,u32 id) {
    volatile u32 *timer=services_word(h,SERVICES_PERSIST_TIMER);
    if(id!=timer[0]) return 0;
    timer[0]=0;
    persist_ready(h);
    return 1;
}
#endif
