#ifndef GRKIT_SERVICES_H
#define GRKIT_SERVICES_H
#include "profile.h"
/* The services of the core for modules (runtime/sdk/grkit.h): events, the
   timer, and the UI. Their state is a block of the module array, which
   INSTALL writes, and GRKIT_HOST_SERVICES holds its address. The includer
   defines u8, u16, u32, and the header fields.

   The block: the number of subscribers, then for each its handler and its
   events; the ready flag, the event depth, and the App call depth; a slot
   for each timer; then the UI state.

   The timer and UI services are valid only in the App task. The core calls
   module code in the App task only through services_call1, 2, or 3: an
   event handler, a timer callback, or a UI callback. The App call depth is
   above 0 only in such a call, so the services refuse a call from module
   code in another task, such as a layer or a plane filter. */
#define SERVICES_SUBSCRIBER_COUNT 0U
#define SERVICES_SUBSCRIBERS 4U
#define SERVICES_SUBSCRIBERS_MAX GRKIT_MODULES_MAX
#define SERVICES_READY (SERVICES_SUBSCRIBERS + 8U * SERVICES_SUBSCRIBERS_MAX)
#define SERVICES_DEPTH (SERVICES_READY + 4U)
#define SERVICES_APP (SERVICES_DEPTH + 4U)
/* A timer slot: the id of the firmware timer (0 when free), the callback,
   and its argument. */
#define SERVICES_TIMERS (SERVICES_APP + 4U)
#define SERVICES_TIMER_SLOTS 8U
#define SERVICES_TIMER_SIZE 12U
#define SERVICES_UI (SERVICES_TIMERS + SERVICES_TIMER_SLOTS * SERVICES_TIMER_SIZE)
/* The UI: its mode, the message box and the progress bar, their parent, the
   firmware timer of the box, the callback and its argument, the texts, the
   text descriptor of the box (two words); then the native screen that waits
   to open and its firmware timer, and the callback for the return from a
   native screen and its argument. */
#define UI_MODE 0U
#define UI_VIEW 4U
#define UI_BAR 8U
#define UI_PARENT 12U
#define UI_TIMER 16U
#define UI_DONE 20U
#define UI_ARG 24U
#define UI_TEXT 28U
#define UI_DESCRIPTOR 32U
#define UI_NATIVE 40U
#define UI_NATIVE_TIMER 44U
#define UI_BACK 48U
#define UI_BACK_ARG 52U
/* A slider box (ui_slider): its range, its value and the value when it
   opened, and the callback for each change. The slider view is UI_BAR. */
#define UI_MIN 56U
#define UI_MAX 60U
#define UI_VALUE 64U
#define UI_START 68U
#define UI_CHANGED 72U
#define UI_SIZE 76U
/* The GRKit section of the native MENU (runtime/menu_section.h): whether the Customize
   tab has been patched, the number of rows, the controller of the last
   controller change,
   the listed module of the open module page and its page slot, whether ADJ
   has refreshed after INSTALL, the text descriptor of the section title,
   and whether an options value page stays open; for each row its module,
   its label and value views, their text descriptors, and what it
   opens; then the private copy of the Customize
   tab descriptor. */
#define SERVICES_MENU (SERVICES_UI + UI_SIZE)
#define MENU_ROWS_DONE 0U
#define MENU_ROWS_COUNT 4U
#define MENU_ROWS_CURRENT 12U
#define MENU_ROWS_LISTED 16U
#define MENU_ROWS_PAGE 20U
#define MENU_ROWS_ADJ 24U
#define MENU_ROWS_TITLE_AT 28U
/* Set by OK on an option: the value page stays open (runtime/menu_settings.h); the
   row of that option, where the page puts its cursor; then for each value of
   a long value page, an options page or Film Stock, which draws each row at
   once: its row and text view, the text descriptor and the text of the view
   (MENU_VALUE_UNITS units). */
#define MENU_ROWS_STAY 36U
#define MENU_ROWS_STAY_ROW 40U
#define MENU_ROWS_VALUES 44U
#define MENU_VALUES_MAX 64U
#define MENU_VALUE_UNITS 32U
#define MENU_VALUE_ROW 0U
#define MENU_VALUE_VIEW 4U
#define MENU_VALUE_TEXT_AT 8U
#define MENU_VALUE_TEXT 16U
#define MENU_VALUE_SIZE (MENU_VALUE_TEXT + 2U * MENU_VALUE_UNITS)
/* A row for each listed module, ADJ Order, and About GRKit. */
#define MENU_ROWS_SLOTS 12U
#define MENU_ROW_MODULE 0U
#define MENU_ROW_LABEL 4U
#define MENU_ROW_VALUE 8U
#define MENU_ROW_LABEL_TEXT_AT 12U
#define MENU_ROW_VALUE_TEXT_AT 20U
#define MENU_ROW_KIND 28U
#define MENU_ROW_SIZE 32U
/* Whether the camera information page shows About GRKit, the text
   descriptors of its title and labels, and its ASCII lines. */
#define MENU_ROWS_ABOUT (MENU_ROWS_VALUES + MENU_VALUES_MAX * MENU_VALUE_SIZE)
#define MENU_ROWS_ABOUT_TEXTS (MENU_ROWS_ABOUT + 4U)
#define MENU_ROWS_ABOUT_LINES (MENU_ROWS_ABOUT_TEXTS + 3U * 8U)
#define MENU_ABOUT_LINE 40U
/* The text descriptors of the title and of the OK and MENU guides of an
   options value page, then the text of its title. */
#define MENU_ROWS_PAGE_TEXTS (MENU_ROWS_ABOUT_LINES + 2U * MENU_ABOUT_LINE)
#define MENU_PAGE_TITLE_UNITS 48U
#define MENU_ROWS_PAGE_TITLE (MENU_ROWS_PAGE_TEXTS + 3U * 8U)
#define MENU_ROWS_FIRST (MENU_ROWS_PAGE_TITLE + 2U * MENU_PAGE_TITLE_UNITS)
#define MENU_ROWS_TAB (MENU_ROWS_FIRST + MENU_ROWS_SLOTS * MENU_ROW_SIZE)
#define MENU_ROWS_TAB_SIZE 0x384U
/* The code of the modules that a soft Off stopped at this start
   (runtime/manage.h): their count, then the start and the end of the code of
   each. The core calls no module code in these ranges. */
#define SERVICES_SUSPENDED (SERVICES_MENU + MENU_ROWS_TAB + MENU_ROWS_TAB_SIZE)
#define SERVICES_SUSPENDED_MAX GRKIT_MODULES_MAX
/* The own SelectMenuControllers of the long value pages
   (runtime/menu_settings.h): the checkbox page of options pages, or 0 before
   the first one; the controller ID of the open long value page, or 0; while a
   controller is built, 1 + its page (CHECK_PAGE_CHECKBOX or CHECK_PAGE_RADIO),
   else 0; the radio page of the other long value pages, or 0; the setting
   that the open long value page shows; the row view type of a radio row and
   the row height of the native constructor; then for each page its list and
   its row count. */
#define SERVICES_CHECK (SERVICES_SUSPENDED + 4U + 8U * SERVICES_SUSPENDED_MAX)
#define CHECK_PAGE 0U
#define CHECK_ID 4U
#define CHECK_BUILD 8U
#define CHECK_RADIO 12U
#define CHECK_SETTING 16U
#define CHECK_RADIO_TYPE 20U
#define CHECK_HEIGHT 24U
#define CHECK_LISTS 28U
#define CHECK_PAGE_CHECKBOX 0U
#define CHECK_PAGE_RADIO 1U
#define SERVICES_SIZE (SERVICES_CHECK + CHECK_LISTS + 2U * 8U)
#define SERVICES_WORDS (SERVICES_SIZE / 4U)
/* The events (GRKIT_EVENT_*). */
#define SERVICE_EVENT_SYSTEM_READY 1U
#define SERVICE_EVENT_SETTING_CHANGED 2U
/* The native screens of ui_open_native (GRKIT_NATIVE_*). */
#define SERVICE_NATIVE_WHITE_BALANCE 1U
#define SERVICE_NATIVE_LCD 2U

/* The block at a camera address. A host test maps it. */
#ifndef SERVICES_MAP
#define SERVICES_MAP(address) ((volatile u32 *)(address))
#endif
static __attribute__((unused)) volatile u32 *services_word(volatile u32 *h,u32 offset) {
    return SERVICES_MAP(h[GRKIT_HOST_SERVICES/4]+offset);
}

/* Whether module code that the core called in the App task runs now. */
static __attribute__((unused)) u32 services_in_app(volatile u32 *h) {
    return h[GRKIT_HOST_SERVICES/4] && services_word(h,SERVICES_APP)[0];
}
/* Whether `fn` is code of a module that a soft Off stopped. */
static __attribute__((unused)) u32 services_suspended(volatile u32 *h,u32 fn) {
    u32 n, i;
    if(!h[GRKIT_HOST_SERVICES/4]) return 0;
    n=services_word(h,SERVICES_SUSPENDED)[0];
    for(i=0;i<n && i<SERVICES_SUSPENDED_MAX;++i)
        if(fn>=services_word(h,SERVICES_SUSPENDED+4+8*i)[0] && fn<services_word(h,SERVICES_SUSPENDED+8+8*i)[0]) return 1;
    return 0;
}
/* Call module code in the App task with one, two, or three arguments. Code
   of a stopped module does not run. */
static __attribute__((unused)) void services_call1(volatile u32 *h,u32 fn,u32 a) {
    u32 start;
    if(services_suspended(h,fn)) return;
    ++services_word(h,SERVICES_APP)[0];
    start=profile_begin(h);
    ((void (*)(u32))fn)(a);
    profile_end_module(h,start,fn,GRKIT_PROFILE_CALL);
    --services_word(h,SERVICES_APP)[0];
}
static __attribute__((unused)) void services_call2(volatile u32 *h,u32 fn,u32 a,u32 b) {
    u32 start;
    if(services_suspended(h,fn)) return;
    ++services_word(h,SERVICES_APP)[0];
    start=profile_begin(h);
    ((void (*)(u32,u32))fn)(a,b);
    profile_end_module(h,start,fn,GRKIT_PROFILE_CALL);
    --services_word(h,SERVICES_APP)[0];
}
static __attribute__((unused)) void services_call3(volatile u32 *h,u32 fn,u32 a,u32 b,u32 c) {
    u32 start;
    if(services_suspended(h,fn)) return;
    ++services_word(h,SERVICES_APP)[0];
    start=profile_begin(h);
    ((void (*)(u32,u32,u32))fn)(a,b,c);
    profile_end_module(h,start,fn,GRKIT_PROFILE_CALL);
    --services_word(h,SERVICES_APP)[0];
}

/* Call the handler of each subscriber to `event`. A handler that changes a
   setting sends no second event, so a handler cannot loop. */
static __attribute__((unused)) void services_emit(volatile u32 *h,u32 event,u32 a,u32 b) {
    u32 n, i;
    if(!h[GRKIT_HOST_SERVICES/4] || services_word(h,SERVICES_DEPTH)[0]) return;
    n=services_word(h,SERVICES_SUBSCRIBER_COUNT)[0];
    if(n>SERVICES_SUBSCRIBERS_MAX) return;
    services_word(h,SERVICES_DEPTH)[0]=1;
    for(i=0;i<n;++i) {
        u32 handler=services_word(h,SERVICES_SUBSCRIBERS+8*i)[0];
        if(handler && services_word(h,SERVICES_SUBSCRIBERS+8*i+4)[0]&event)
            services_call3(h,handler,event,a,b);
    }
    services_word(h,SERVICES_DEPTH)[0]=0;
}
/* SYSTEM_READY, once after INSTALL: the core calls it when the shooting
   screen first shows (runtime/framing.c). */
static __attribute__((unused)) void services_ready(volatile u32 *h) {
    if(!h[GRKIT_HOST_SERVICES/4] || services_word(h,SERVICES_READY)[0]) return;
    services_word(h,SERVICES_READY)[0]=1;
    services_emit(h,SERVICE_EVENT_SYSTEM_READY,0,0);
}
#endif
