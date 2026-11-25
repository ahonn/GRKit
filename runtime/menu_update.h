#ifndef GRKIT_MENU_UPDATE_H
#define GRKIT_MENU_UPDATE_H
/* The state of an update over Wi-Fi on the camera (GRKIT_HOST_UPDATE): the
   resident loader notices the journal of the service (WAITING), and a
   confirm on the camera makes the update run at the next power-on
   (CONFIRMED). The page of the Update row of MENU (update_ask_menu) and the
   Update row of the MOD page of ADJ (update_ask_adj) ask with the same
   confirm. */
static u32 update_waiting(volatile u32 *h) { return h[GRKIT_HOST_UPDATE/4]==GRKIT_UPDATE_WAITING; }
static u32 update_shown(volatile u32 *h) { return h[GRKIT_HOST_UPDATE/4]-GRKIT_UPDATE_WAITING<=GRKIT_UPDATE_READY-GRKIT_UPDATE_WAITING; }
static u32 update_confirmed(volatile u32 *h) { return update_shown(h) && !update_waiting(h); }
/* Append the version of the update, such as 1.7.0, or nothing when the
   package does not name it. */
static u32 update_append_version(volatile u32 *h,volatile u16 *to,u32 at) {
    static const u16 dot[]={'.',0};
    u32 version=h[GRKIT_HOST_UPDATE_VERSION/4];
    if(!version) return at;
    at=recipe_append(to,recipe_append_number(to,at,version>>16&255U),dot);
    at=recipe_append(to,recipe_append_number(to,at,version>>8&255U),dot);
    return recipe_append_number(to,at,version&255U);
}
/* Update and the version, in the text of About GRKit and of the page. */
static const u16 *update_title(volatile u32 *h) {
    static const u16 space[]={' ',0};
    volatile u16 *to=recipe_text_at(h,RECIPE_UPDATE_AT);
    u32 at=recipe_append(to,0,text(h,TEXT_UPDATE));
    if(h[GRKIT_HOST_UPDATE_VERSION/4]) update_append_version(h,to,recipe_append(to,at,space));
    return (const u16 *)to;
}
/* Confirms the update that waits when `executed` is 1, the Execute of its
   confirm. Returns whether it did. */
static u32 update_confirm(volatile u32 *h,u32 executed) {
    if(executed!=1U || !update_waiting(h)) return 0;
    h[GRKIT_HOST_UPDATE/4]=GRKIT_UPDATE_CONFIRMED;
    return 1;
}
/* The confirm of the update that waits, on the canvas of `controller`, the
   cursor on Cancel: OK calls done(1 for Execute, arg). */
static u32 update_ask(volatile u32 *h,u32 controller,void (*done)(u32,u32),u32 arg) {
    volatile u16 *to=recipe_text_at(h,RECIPE_BOX_AT);
    recipe_append(to,update_append_version(h,to,recipe_append(to,0,text(h,TEXT_UPDATE+2))),text(h,TEXT_UPDATE+3));
    return ui_confirm_on(h,controller,recipe_box(h),done,arg);
}
/* Execute or Cancel of the confirm in MENU (runtime/menu_settings.h:
   update_setting): the page has no other choice, so both leave it. */
static void update_menu_done(u32 executed,u32 arg) {
    volatile u32 *h=menu_header();
    (void)arg;
    update_confirm(h,executed);
    settings_value_leave(h);
}
/* The confirm on the page of the Update row of MENU. */
static void update_ask_menu(volatile u32 *h,u32 controller) {
    update_ask(h,controller,update_menu_done,0);
}
/* The Update row of the MOD page of ADJ under the cursor, while an update
   waits (runtime/manage.h: SWITCH_ROW_UPDATE). */
static u32 update_row(volatile u32 *h) {
    return update_waiting(h) && h[GRKIT_HOST_MENU_LEVEL/4]==MENU_MODULE &&
           h[GRKIT_HOST_MODULE/4]==SWITCH_MODULE && h[GRKIT_HOST_MENU_CURSOR/4]==SWITCH_ROW_UPDATE;
}
/* Execute or Cancel of the confirm in ADJ, whose controller is `controller`.
   Cancel keeps the page. Execute confirms the update: ADJ then closes
   (runtime/menu.c: menu_event), and the live view tells to restart, as the
   load cue does. */
static void update_adj_done(u32 executed,u32 controller) {
    volatile u32 *h=menu_header();
    if(update_confirm(h,executed)) {
        if(h[GRKIT_HOST_MENU_CURSOR/4]>=count(h)) h[GRKIT_HOST_MENU_CURSOR/4]=count(h)-1;
        h[GRKIT_HOST_CUE_LABEL/4]=GRKIT_CUE_UPDATE_RESTART;
        h[GRKIT_HOST_LOAD_CUE/4]=GRKIT_CUE_PENDING;
        return;
    }
    render(controller);
}
/* OK on the Update row of the MOD page asks with the confirm: whether it
   did. */
static u32 update_ask_adj(volatile u32 *h,u32 controller) {
    if(!update_row(h)) return 0;
    update_ask(h,controller,update_adj_done,controller);
    return 1;
}
#endif
