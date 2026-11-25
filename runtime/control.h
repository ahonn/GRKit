#ifndef GRKIT_CONTROL_H
#define GRKIT_CONTROL_H
/* The commands of the Wi-Fi service (grkit_core::abi). The App task answers
   each one in its poll of the mailbox (runtime/control_mailbox.h), from the
   timer of the saved state (runtime/persist.h): the effects
   (runtime/control_looks.h), the recipes (runtime/control_recipes.h), and
   the module settings and GRKit Settings (runtime/control_modules.h). */
#include "control_mailbox.h"
/* A page of GRKit that lists the effects or the recipes keeps its rows
   while it shows, so they change while none shows: not the GRKit menu of
   ADJ, a preview, the adjustments of an effect, or a development, nor in
   MENU a page of GRKit (the settings submenu and its value pages,
   settings_shows) or the camera information page with a GRKit view. The
   shooting screen, playback, and the other pages of MENU take changes:
   each GRKit row of MENU reads its value when it draws. */
static u32 control_idle(volatile u32 *h) {
    u32 current=services_word(h,SERVICES_MENU+MENU_ROWS_CURRENT)[0];
    return !h[GRKIT_HOST_MENU_CONTROLLER/4] && !h[GRKIT_HOST_PREVIEW/4] && !h[GRKIT_HOST_DEVELOP/4] &&
           !(h[GRKIT_HOST_PARAMETERS/4]&~1U) && !settings_shows(current) &&
           !(current==INFO_CONTROLLER && services_word(h,SERVICES_MENU+MENU_ROWS_ABOUT)[0]);
}
#include "control_looks.h"
#include "control_recipes.h"
#include "control_modules.h"
/* Each command to the commands of its kind, when the camera has them. */
static u32 control_dispatch(void *context,struct control_request *request,u32 *length) {
    volatile u32 *h=(volatile u32 *)context;
    u32 capabilities=h[GRKIT_HOST_CAPABILITIES/4];
    switch(request->command) {
    case GRKIT_CONTROL_LOOKS_READ:
    case GRKIT_CONTROL_LOOK_TURN:
    case GRKIT_CONTROL_LOOKS_ORDER:
        return capabilities&GRKIT_CORE_TONE_BIT ? control_looks(h,request,length) : GRKIT_CONTROL_UNSUPPORTED;
    case GRKIT_CONTROL_RECIPES_READ:
    case GRKIT_CONTROL_RECIPE_TURN:
    case GRKIT_CONTROL_RECIPES_ORDER:
    case GRKIT_CONTROL_RECIPE_RENAME:
    case GRKIT_CONTROL_RECIPE_DELETE:
    case GRKIT_CONTROL_RECIPE_APPLY:
    case GRKIT_CONTROL_RECIPE_UNDO:
    case GRKIT_CONTROL_RECIPE_ADD:
        return capabilities&GRKIT_CORE_RECIPE_BIT ? control_recipes(h,request,length) : GRKIT_CONTROL_UNSUPPORTED;
    /* The modules and the settings need no capability of Looks or Recipes. */
    case GRKIT_CONTROL_MODULES_READ:
    case GRKIT_CONTROL_MODULE_TEXTS:
    case GRKIT_CONTROL_MODULE_TILE:
    case GRKIT_CONTROL_MODULE_SET:
    case GRKIT_CONTROL_MODULES_ORDER:
    case GRKIT_CONTROL_MODULES_LAYOUT:
    case GRKIT_CONTROL_SETTINGS_READ:
    case GRKIT_CONTROL_SETTINGS_SET:
        return control_modules(h,request,length);
    default: return GRKIT_CONTROL_UNSUPPORTED;
    }
}
/* Whether the Wi-Fi service waits for an answer. The request line belongs
   to Linux, so it is read from memory as control_acquire reads it. */
static u32 control_pending(volatile u32 *h) {
    if(!h[GRKIT_HOST_CONTROL_WORD/4]) return 0;
    volatile struct control_mailbox *box=(volatile struct control_mailbox *)SERVICES_MAP(SERVICES_CONTROL_ADDRESS(h[GRKIT_HOST_SERVICES/4]));
    CONTROL_INVALIDATE(&box->request,64);
    return box->request!=box->ready;
}
/* The poll of the mailbox, from the timer of the saved state
   (runtime/persist.h). */
static void control_poll(volatile u32 *h) {
    if(!h[GRKIT_HOST_CONTROL_WORD/4]) return;
    volatile struct control_mailbox *box=(volatile struct control_mailbox *)SERVICES_MAP(SERVICES_CONTROL_ADDRESS(h[GRKIT_HOST_SERVICES/4]));
    /* The service watches this count to tell the phone of a change that the
       user made on the camera. No request and no card read. */
    u32 changes=h[GRKIT_HOST_STATE_CHANGES/4];
    if(box->state_changes!=changes) { box->state_changes=changes; CONTROL_CLEAN(&box->magic,64); }
    control_poll_once(box,(struct control_request *)services_word(h,SERVICES_CONTROL_CANDIDATE),control_dispatch,(void *)h);
}
#endif
