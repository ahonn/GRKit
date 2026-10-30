#ifndef GRKIT_UI_H
#define GRKIT_UI_H
/* The timer and UI services (runtime/sdk/grkit.h), part of the menu, which
   runs in the App task. The state is in the services block
   (runtime/services.h). The includer is runtime/menu.c.

   The timers are firmware App timers: a one-shot timer calls vtable slot 2
   of its handler with the handler and the timer id, in the App task.

   The UI shows the native message box (view type 0x0e) on the ADJ canvas
   of the GRKit page. Firmware view calls are valid only in the App task, so
   each service refuses a caller that is not module code in the App task
   (services_in_app).
   The view lifetime rules of the firmware:
   1. A new view starts with its layout and redraw flags set, so AddChild
      alone never reaches the canvas: the parent must get RequestLayout
      (vtable +0x20) and ChildDirty (vtable +0x44).
   2. Other views keep raw pointers to a view in their overlap lists. Before
      a delete, each view of the subtree is hidden and purged from the lists
      (canvas vtable +0x38), children first.
   The box text is the text ID SETTINGS_TITLE_TEXT, which the text hook
   (menu_text_override) gives while the box shows, in the menu language. */
#define UI_NONE 0U
#define UI_TOAST 1U
#define UI_MESSAGE 2U
#define UI_CONFIRM 3U
#define UI_PROGRESS 4U
/* The text after ui_progress_close, until its timer. */
#define UI_DONE_TEXT 5U
#define UI_DONE_MS 1500U
#define UI_VIEW_MESSAGE 0x0eU
#define UI_VIEW_PROGRESS 0x0fU
/* A message with a native slider (ui_slider). */
#define UI_SLIDER 6U
#define UI_VIEW_SLIDER 0x16U
/* A message with two buttons of other texts (ui_choice). */
#define UI_CHOICE 7U

static volatile u32 *ui_word(volatile u32 *h,u32 offset) { return services_word(h,SERVICES_UI+offset); }

/* ---- Timers --------------------------------------------------------------- */
A32_ENTRY void ui_timer_fired(u32 handler,u32 id);
/* Pointer tables, so a host build also takes them; on the camera each entry
   is one word. */
static void *const ui_timer_vtable[3]={0,0,(void *)ui_timer_fired};
static const void *const ui_timer_handler[1]={ui_timer_vtable};
static u32 ui_timer_start(u32 ms) {
    return CALL3(FW_TIMER_START)(CALL0(FW_TIMER_SERVICE)(),ms,(u32)ui_timer_handler);
}
static void ui_timer_cancel(u32 id) {
    if(id) CALL2(FW_TIMER_CANCEL)(CALL0(FW_TIMER_SERVICE)(),id);
}
__attribute__((used)) u32 grkit_export_timer_start(u32 ms,void (*fire)(u32),u32 arg) {
    volatile u32 *h=menu_header();
    u32 i;
    if(!services_in_app(h) || !fire) return 0;
    for(i=0;i<SERVICES_TIMER_SLOTS;++i) {
        volatile u32 *slot=services_word(h,SERVICES_TIMERS+i*SERVICES_TIMER_SIZE);
        if(slot[0]) continue;
        slot[1]=(u32)fire; slot[2]=arg;
        slot[0]=ui_timer_start(ms);
        return slot[0] ? i+1 : 0;
    }
    return 0;
}
__attribute__((used)) void grkit_export_timer_cancel(u32 timer) {
    volatile u32 *h=menu_header();
    volatile u32 *slot;
    if(!services_in_app(h) || timer-1>=SERVICES_TIMER_SLOTS) return;
    slot=services_word(h,SERVICES_TIMERS+(timer-1)*SERVICES_TIMER_SIZE);
    ui_timer_cancel(slot[0]);
    slot[0]=0;
}

/* ---- Views ---------------------------------------------------------------- */
static u32 ui_create(u32 type,u32 x,u32 y,u32 w,u32 hh) {
    u32 desc[5]={type,x,y,w,hh};
    return CALL1(FW_VIEW_FACTORY)((u32)desc);
}
static void ui_attach(u32 parent,u32 view) {
    CALL2(WORD(WORD(parent,0),0x30))(parent,view);
    CALL2(FW_VIEW_SET_VISIBLE)(view,1);
    CALL2(WORD(WORD(parent,0),0x20))(parent,view);
    CALL1(WORD(WORD(parent,0),0x44))(parent);
}
static void ui_purge(u32 root,u32 view,u32 depth) {
    u32 child=WORD(view,0xc), end=WORD(view,0x10);
    if(depth<4) for(;child!=end;child+=4) if(WORD(child,0)) ui_purge(root,WORD(child,0),depth+1);
    CALL2(FW_VIEW_SET_VISIBLE)(view,0);
    CALL2(WORD(WORD(root,0),0x38))(root,view);
}
static void ui_remove(u32 parent,u32 view) {
    u32 position, end, root=parent;
    if(!view || !parent) return;
    while(WORD(root,4)) root=WORD(root,4);
    ui_purge(root,view,0);
    position=WORD(parent,0xc); end=WORD(parent,0x10);
    while(position!=end && WORD(position,0)!=view) position+=4;
    if(position!=end) CALL2(FW_VECTOR_ERASE)(parent+8,position);
    CALL2(FW_VIEW_SET_PARENT)(view,0);
    CALL2(WORD(WORD(parent,0),0x20))(parent,parent);
    CALL1(FW_VIEW_INVALIDATE)(parent);
    CALL1(WORD(WORD(view,0),4))(view);
}

/* ---- The box -------------------------------------------------------------- */
/* Close the box without a callback: ADJ exit, or a box that ends. */
static void ui_close(volatile u32 *h) {
    if(!h[GRKIT_HOST_SERVICES/4]) return;
    ui_timer_cancel(ui_word(h,UI_TIMER)[0]);
    ui_word(h,UI_TIMER)[0]=0;
    ui_remove(ui_word(h,UI_PARENT)[0],ui_word(h,UI_BAR)[0]);
    ui_remove(ui_word(h,UI_PARENT)[0],ui_word(h,UI_VIEW)[0]);
    ui_word(h,UI_BAR)[0]=ui_word(h,UI_VIEW)[0]=ui_word(h,UI_PARENT)[0]=0;
    ui_word(h,UI_MODE)[0]=UI_NONE;
}
/* The text of the box for the text hook, or 0. The buttons of a choice box
   keep the IDs of Execute and Cancel and show the texts of the choice. */
static u32 ui_text_override(volatile u32 *h,u32 id) {
    u32 mode;
    if(!h[GRKIT_HOST_SERVICES/4]) return 0;
    mode=ui_word(h,UI_MODE)[0];
    if(mode==UI_NONE) return 0;
    if(id==SETTINGS_TITLE_TEXT) return (u32)ui_word(h,UI_DESCRIPTOR)|1U;
    if(mode==UI_CHOICE && id==UI_EXECUTE_TEXT) return (u32)ui_word(h,UI_CHOICE_TEXTS)|1U;
    if(mode==UI_CHOICE && id==UI_CANCEL_TEXT) return (u32)ui_word(h,UI_CHOICE_TEXTS+8U)|1U;
    return 0;
}
static void ui_set_text(volatile u32 *h,const u16 *const *text) {
    ui_word(h,UI_TEXT)[0]=(u32)text;
    descriptor(ui_word(h,UI_DESCRIPTOR),text[menu_language()]);
    CALL2(FW_MESSAGE_SET)(ui_word(h,UI_VIEW)[0],SETTINGS_TITLE_TEXT);
}
/* Open a box of `mode` on the ADJ canvas of the GRKit page: 1, or 0 when
   the caller is not module code in the App task, GRKit is not open, or
   another box shows. */
static u32 ui_open(volatile u32 *h,u32 mode,const u16 *const *text,u32 background) {
    u32 controller=h[GRKIT_HOST_MENU_CONTROLLER/4], parent, view;
    if(!services_in_app(h) || !text || !controller || !h[GRKIT_HOST_MENU_LEVEL/4]) return 0;
    if(ui_word(h,UI_MODE)[0]==UI_TOAST || ui_word(h,UI_MODE)[0]==UI_DONE_TEXT) ui_close(h);
    if(ui_word(h,UI_MODE)[0]!=UI_NONE) return 0;
    parent=WORD(controller,ADJ_CANVAS);
    view=parent ? ui_create(UI_VIEW_MESSAGE,20,20,680,400) : 0;
    if(!view) return 0;
    ui_word(h,UI_VIEW)[0]=view;
    ui_word(h,UI_PARENT)[0]=parent;
    ui_word(h,UI_MODE)[0]=mode;
    CALL2(FW_MESSAGE_BACKGROUND)(view,background);
    ui_set_text(h,text);
    CALL3(FW_MESSAGE_ENABLE_BUTTONS)(view,0,0);
    ui_attach(parent,view);
    return 1;
}
__attribute__((used)) u32 grkit_export_ui_toast(const u16 *const *text,u32 ms) {
    volatile u32 *h=menu_header();
    if(!ui_open(h,UI_TOAST,text,1)) return 0;
    ui_word(h,UI_TIMER)[0]=ui_timer_start(ms ? ms : UI_DONE_MS);
    return 1;
}
__attribute__((used)) u32 grkit_export_ui_message(const u16 *const *text,void (*done)(u32),u32 arg) {
    volatile u32 *h=menu_header();
    if(!ui_open(h,UI_MESSAGE,text,0)) return 0;
    ui_word(h,UI_DONE)[0]=(u32)done; ui_word(h,UI_ARG)[0]=arg;
    return 1;
}
__attribute__((used)) u32 grkit_export_ui_confirm(const u16 *const *text,void (*done)(u32,u32),u32 arg) {
    volatile u32 *h=menu_header();
    u32 view;
    if(!ui_open(h,UI_CONFIRM,text,0)) return 0;
    ui_word(h,UI_DONE)[0]=(u32)done; ui_word(h,UI_ARG)[0]=arg;
    view=ui_word(h,UI_VIEW)[0];
    CALL3(FW_MESSAGE_BUTTONS)(view,UI_EXECUTE_TEXT,UI_CANCEL_TEXT);
    /* Both buttons, the cursor on Cancel. */
    CALL3(FW_MESSAGE_ENABLE_BUTTONS)(view,1,1);
    return 1;
}
/* A box of the core with two buttons, `first` and `second`, in the menu
   language, and the cursor on `first`. OK calls done(0 or 1, arg) for the
   button under the cursor, and MENU done(2, arg). The native message box
   has two buttons at most, so a third choice is MENU. */
static u32 ui_choice(volatile u32 *h,const u16 *const *text,const u16 *first,const u16 *second,
                     void (*done)(u32,u32),u32 arg) {
    u32 view;
    if(!ui_open(h,UI_CHOICE,text,0)) return 0;
    descriptor(ui_word(h,UI_CHOICE_TEXTS),first);
    descriptor(ui_word(h,UI_CHOICE_TEXTS+8U),second);
    ui_word(h,UI_DONE)[0]=(u32)done; ui_word(h,UI_ARG)[0]=arg;
    view=ui_word(h,UI_VIEW)[0];
    CALL3(FW_MESSAGE_BUTTONS)(view,UI_EXECUTE_TEXT,UI_CANCEL_TEXT);
    CALL3(FW_MESSAGE_ENABLE_BUTTONS)(view,1,0);
    return 1;
}
__attribute__((used)) u32 grkit_export_ui_progress_open(const u16 *const *text) {
    volatile u32 *h=menu_header();
    u32 bar;
    if(!ui_open(h,UI_PROGRESS,text,0)) return 0;
    bar=ui_create(UI_VIEW_PROGRESS,82,270,556,30);
    if(bar) {
        ui_attach(ui_word(h,UI_PARENT)[0],bar);
        CALL2(FW_PROGRESS_PERCENT)(bar,0);
    }
    ui_word(h,UI_BAR)[0]=bar;
    return 1;
}
__attribute__((used)) void grkit_export_ui_progress_set(u32 percent) {
    volatile u32 *h=menu_header();
    if(!services_in_app(h) || ui_word(h,UI_MODE)[0]!=UI_PROGRESS || !ui_word(h,UI_BAR)[0]) return;
    CALL2(FW_PROGRESS_PERCENT)(ui_word(h,UI_BAR)[0],percent<100 ? percent : 100);
}
__attribute__((used)) void grkit_export_ui_progress_close(const u16 *const *text) {
    volatile u32 *h=menu_header();
    if(!services_in_app(h) || ui_word(h,UI_MODE)[0]!=UI_PROGRESS) return;
    if(!text) { ui_close(h); return; }
    ui_remove(ui_word(h,UI_PARENT)[0],ui_word(h,UI_BAR)[0]);
    ui_word(h,UI_BAR)[0]=0;
    ui_set_text(h,text);
    ui_word(h,UI_MODE)[0]=UI_DONE_TEXT;
    ui_word(h,UI_TIMER)[0]=ui_timer_start(UI_DONE_MS);
}
static void ui_native_open(volatile u32 *h);
A32_ENTRY void ui_timer_fired(u32 handler,u32 id) {
    volatile u32 *h=menu_header();
    u32 i;
    (void)handler;
    if(!h[GRKIT_HOST_SERVICES/4] || !id) return;
    if(id==ui_word(h,UI_NATIVE_TIMER)[0]) {
        ui_word(h,UI_NATIVE_TIMER)[0]=0;
        ui_native_open(h);
        return;
    }
    if(id==ui_word(h,UI_TIMER)[0]) {
        ui_word(h,UI_TIMER)[0]=0;
        ui_close(h);
        return;
    }
    for(i=0;i<SERVICES_TIMER_SLOTS;++i) {
        volatile u32 *slot=services_word(h,SERVICES_TIMERS+i*SERVICES_TIMER_SIZE);
        if(slot[0]!=id) continue;
        slot[0]=0;
        services_call1(h,slot[1],slot[2]);
        return;
    }
}
/* The slider of a slider box. The native slider places its knob icon at
   x = base + step * value in the view, so the core gives it a step of one
   pixel and the knob position: the value on a track of UI_SLIDER_TRACK
   pixels from UI_SLIDER_BASE, as on the native sliders of 400 pixels. The
   firmware fills a slider configuration (0x34 bytes) and applies it to the
   view, which redraws when the configuration changes. */
#define UI_SLIDER_WIDTH 400U
#define UI_SLIDER_BASE 15U
#define UI_SLIDER_TRACK 360U
/* n / d for d > 0, by shift and subtract: the menu code has no division
   helper. */
static u32 ui_divide(u32 n,u32 d) {
    u32 q=0, r=0, i;
    for(i=32;i--;) {
        r=r<<1|(n>>i&1U);
        if(r>=d) { r-=d; q|=1U<<i; }
    }
    return q;
}
static void ui_slider_show(volatile u32 *h) {
    static const u16 icons[4]=UI_SLIDER_ICONS;
    u32 config[13], bar=ui_word(h,UI_BAR)[0], min=ui_word(h,UI_MIN)[0];
    u32 knob=ui_divide((ui_word(h,UI_VALUE)[0]-min)*UI_SLIDER_TRACK,ui_word(h,UI_MAX)[0]-min);
    if(!bar) return;
    ((void (*)(u32,u32,u32,u32,u32,u32,u32,u32,u32,u32,u32))FW_SLIDER_INIT)(
        (u32)config,0,8,1,UI_SLIDER_BASE,0,0,0x14,icons[0],icons[1],icons[2]);
    ((void (*)(u32,u32,u32,u32,u32))FW_SLIDER_RANGE)((u32)config,knob,0,0x1e,icons[3]);
    CALL2(FW_SLIDER_FLAG)((u32)config,0);
    CALL2(FW_SLIDER_APPLY)(bar,(u32)config);
}
__attribute__((used)) u32 grkit_export_ui_slider(const u16 *const *text,u32 min,u32 max,u32 value,
                                                 void (*changed)(u32,u32),void (*done)(u32,u32,u32),u32 arg) {
    volatile u32 *h=menu_header();
    u32 bar;
    if(max<=min || max-min>0xffffU || value<min || value>max || !ui_open(h,UI_SLIDER,text,0)) return 0;
    bar=ui_create(UI_VIEW_SLIDER,(720-UI_SLIDER_WIDTH)/2,250,UI_SLIDER_WIDTH,60);
    if(bar) ui_attach(ui_word(h,UI_PARENT)[0],bar);
    ui_word(h,UI_BAR)[0]=bar;
    ui_word(h,UI_MIN)[0]=min; ui_word(h,UI_MAX)[0]=max;
    ui_word(h,UI_VALUE)[0]=ui_word(h,UI_START)[0]=value;
    ui_word(h,UI_CHANGED)[0]=(u32)changed;
    ui_word(h,UI_DONE)[0]=(u32)done; ui_word(h,UI_ARG)[0]=arg;
    ui_slider_show(h);
    return 1;
}
/* Up and Down move the slider by one, Left and Right by a tenth of the
   range. OK keeps the value, MENU the value when the box opened. */
static void ui_slider_event(volatile u32 *h,u32 kind,u32 back) {
    u32 value=ui_word(h,UI_VALUE)[0], min=ui_word(h,UI_MIN)[0], max=ui_word(h,UI_MAX)[0];
    /* A tenth of the range, without a division helper. */
    u32 tenth=(u32)(((unsigned long long)(max-min)*0xCCCCCCCDULL)>>35);
    u32 step=tenth ? tenth : 1, changed=ui_word(h,UI_CHANGED)[0];
    u32 done=ui_word(h,UI_DONE)[0], arg=ui_word(h,UI_ARG)[0];
    if(kind==EV_UP || kind==EV_DIAL_UP) value=value<max ? value+1 : max;
    else if(kind==EV_DOWN || kind==EV_DIAL_DOWN) value=value>min ? value-1 : min;
    else if(kind==EV_RIGHT) value=max-value>step ? value+step : max;
    else if(kind==EV_LEFT) value=value-min>step ? value-step : min;
    else if(kind==EV_CONFIRM || back) {
        u32 committed=!back;
        if(back) value=ui_word(h,UI_START)[0];
        ui_close(h);
        if(done) services_call3(h,done,committed,value,arg);
        return;
    }
    else return;
    if(value==ui_word(h,UI_VALUE)[0]) return;
    ui_word(h,UI_VALUE)[0]=value;
    ui_slider_show(h);
    if(changed) services_call2(h,changed,value,arg);
}
/* The keys while a message, a confirm, a progress, or a slider box shows.
   The box takes each key. `back` is 1 for MENU (back_event). */
static u32 ui_modal(volatile u32 *h) {
    u32 mode=h[GRKIT_HOST_SERVICES/4] ? ui_word(h,UI_MODE)[0] : UI_NONE;
    return mode==UI_MESSAGE || mode==UI_CONFIRM || mode==UI_PROGRESS || mode==UI_SLIDER || mode==UI_CHOICE;
}
static void ui_event(volatile u32 *h,u32 kind,u32 back) {
    u32 mode=ui_word(h,UI_MODE)[0], view=ui_word(h,UI_VIEW)[0], done=ui_word(h,UI_DONE)[0], arg=ui_word(h,UI_ARG)[0], executed;
    if(mode==UI_PROGRESS) return;
    if(mode==UI_SLIDER) { ui_slider_event(h,kind,back); return; }
    if((mode==UI_CONFIRM || mode==UI_CHOICE) && (kind==EV_UP || kind==EV_DOWN || kind==EV_DIAL_UP || kind==EV_DIAL_DOWN ||
                                                  kind==EV_LEFT || kind==EV_RIGHT)) {
        CALL3(FW_MESSAGE_CURSOR)(view,CALL1(FW_MESSAGE_GET_CURSOR)(view)&0xff ? 0 : 1,1);
        return;
    }
    if(kind!=EV_CONFIRM && !back) return;
    if(mode==UI_CHOICE) {
        /* Cursor 0 is the first button. MENU is the third choice. */
        executed=back ? 2U : CALL1(FW_MESSAGE_GET_CURSOR)(view)&0xff ? 1U : 0U;
        ui_close(h);
        if(done) services_call2(h,done,executed,arg);
        return;
    }
    /* Cursor 0 is Execute. MENU cancels. */
    executed=mode==UI_CONFIRM && !back && !(CALL1(FW_MESSAGE_GET_CURSOR)(view)&0xff);
    ui_close(h);
    if(!done) return;
    if(mode==UI_CONFIRM) services_call2(h,done,executed,arg);
    else services_call1(h,done,arg);
}

/* ---- Native screens -------------------------------------------------------
   ui_open_native leaves the GRKit page and changes the controller to the
   native screen, with ADJ as the screen to return to, as the parameter page
   does (open_parameters). The return state brings ADJ back to the GRKit
   page that was open (menu_refresh), and then the core calls back(arg).

   The call only records the screen: a 1 ms App timer opens it after the
   current event, so the menu code that called the module does not run on a
   page that it left. In the same way, the return starts the timer, and the
   timer calls back(arg) after ADJ has built the page: a box that back
   shows then stays on the canvas. UI_NATIVE holds the screen to open, or
   UI_NATIVE_BACK while the call of back waits. */
#define UI_NATIVE_BACK 1U
static u32 ui_native_screen(u32 screen) {
    switch(screen) {
    case SERVICE_NATIVE_WHITE_BALANCE: return UI_NATIVE_WHITE_BALANCE;
    case SERVICE_NATIVE_LCD: return UI_NATIVE_LCD;
    default: return 0;
    }
}
/* Record the controller `id` to open, for the core and for modules. */
static u32 ui_native_request(volatile u32 *h,u32 id,void (*back)(u32),u32 arg) {
    if(!services_in_app(h) || !id || !h[GRKIT_HOST_MENU_CONTROLLER/4] || !h[GRKIT_HOST_MENU_LEVEL/4] ||
       ui_word(h,UI_NATIVE)[0] || ui_modal(h)) return 0;
    ui_word(h,UI_NATIVE_TIMER)[0]=ui_timer_start(1);
    if(!ui_word(h,UI_NATIVE_TIMER)[0]) return 0;
    ui_word(h,UI_NATIVE)[0]=id;
    ui_word(h,UI_BACK)[0]=(u32)back; ui_word(h,UI_BACK_ARG)[0]=arg;
    return 1;
}
__attribute__((used)) u32 grkit_export_ui_open_native(u32 screen,void (*back)(u32),u32 arg) {
    return ui_native_request(menu_header(),ui_native_screen(screen),back,arg);
}
static u32 ui_keyboard_arm(volatile u32 *h);
static void ui_native_open(volatile u32 *h) {
    u32 id=ui_word(h,UI_NATIVE)[0], controller=h[GRKIT_HOST_MENU_CONTROLLER/4], root, back=ui_word(h,UI_BACK)[0];
    ui_word(h,UI_NATIVE)[0]=0;
    if(id==UI_NATIVE_BACK) {
        ui_word(h,UI_BACK)[0]=0;
        if(back) services_call1(h,back,ui_word(h,UI_BACK_ARG)[0]);
        /* A keyboard text is for this call only. */
        ui_word(h,UI_KEYBOARD_RESULT)[0]=0;
        return;
    }
    /* ADJ closed before the timer: drop the request. */
    if(!id || !controller || !h[GRKIT_HOST_MENU_LEVEL/4]) { ui_word(h,UI_BACK)[0]=0; return; }
    /* The keyboard is busy: GRKit stays, and back runs without a text. */
    if(id==KEYBOARD_CONTROLLER && !ui_keyboard_arm(h)) {
        ui_word(h,UI_BACK)[0]=0;
        if(back) services_call1(h,back,ui_word(h,UI_BACK_ARG)[0]);
        return;
    }
    ui_close(h);
    /* leave() clears the level, so read it first. */
    root=h[GRKIT_HOST_MENU_LEVEL/4]==MENU_ROOT ? RETURN_ROOT : 0;
    leave(controller);
    h[GRKIT_HOST_RETURN/4]=RETURN_OPENING|root;
    CALL3(FW_CHANGE_CONTROLLER)(controller,id,0x2000000);
}
/* Whether the UI needs the GRKit page to stay open: a box shows, or a
   native screen waits to open. OK on a page row then keeps ADJ open, so
   the module that the row changed can show its feedback (menu_event). */
static u32 ui_holds_page(volatile u32 *h) {
    return h[GRKIT_HOST_SERVICES/4] && (ui_word(h,UI_MODE)[0]!=UI_NONE || ui_word(h,UI_NATIVE)[0]);
}
/* ADJ exit or destroy before the timer fires drops the request. */
static void ui_native_cancel(volatile u32 *h) {
    if(!h[GRKIT_HOST_SERVICES/4] || !ui_word(h,UI_NATIVE)[0]) return;
    ui_timer_cancel(ui_word(h,UI_NATIVE_TIMER)[0]);
    ui_word(h,UI_NATIVE_TIMER)[0]=ui_word(h,UI_NATIVE)[0]=ui_word(h,UI_BACK)[0]=0;
}
/* ---- The keyboard of the camera ------------------------------------------
   The camera has one text keyboard, SoftwareKeyboardController, with a few
   fixed uses: each reads its text from a camera setting when it starts and
   writes the setting on OK. GRKit borrows its copyright use while
   UI_KEYBOARD is set. Three hooks (runtime/menu.S) then give the keyboard
   the GRKit text instead of the copyright holder, take the text on OK
   instead of the copyright setter, and limit the text to UI_KEYBOARD_CHARS.
   So the copyright setting never sees a GRKit text, and the keyboard never
   shows the copyright holder for GRKit.

   UI_KEYBOARD is UI_KEYBOARD_ARMED from the change to the keyboard until it
   loads the text, then UI_KEYBOARD_SHOWN. OK clears it. Each change to
   another controller clears it too (ui_keyboard_change), as does INSTALL,
   which clears the services block: the return to ADJ after OK or MENU, and
   any other way out of the keyboard, end the borrow. The native Copyright
   page opens the keyboard from its own controller, so its keyboard shows
   the copyright holder. On GR IV, a model holds the use of the keyboard: the
   copyright use is set before the change, as the Copyright page does, and
   the keyboard clears it when it stops. */
#define UI_KEYBOARD_ARMED 1U
#define UI_KEYBOARD_SHOWN 2U
/* Open the keyboard with `text`, its printable ASCII without the spaces at
   both ends and at most UI_KEYBOARD_CHARS characters: 1, or 0 as
   ui_open_native refuses. After the return to the GRKit page the core calls
   back(arg), where ui_keyboard_result gives the text of an OK. If the
   keyboard is busy, back runs at once on the page. */
static u32 ui_keyboard_open(volatile u32 *h,const volatile u16 *text,void (*back)(u32),u32 arg) {
    volatile u8 *to=(volatile u8 *)ui_word(h,UI_KEYBOARD_TEXT);
    u32 n=0, i;
    if(!ui_native_request(h,KEYBOARD_CONTROLLER,back,arg)) return 0;
    for(i=0;text[i] && n<UI_KEYBOARD_CHARS;++i)
        if(text[i]>=0x20U && text[i]<=0x7eU && (n || text[i]!=' ')) to[n++]=(u8)text[i];
    while(n && to[n-1]==' ') --n;
    to[n]=0;
    ui_word(h,UI_KEYBOARD_RESULT)[0]=0;
    return 1;
}
/* The text of an OK on the keyboard, during the call of back, else 0. */
static const volatile u8 *ui_keyboard_result(volatile u32 *h) {
    if(!h[GRKIT_HOST_SERVICES/4] || !ui_word(h,UI_KEYBOARD_RESULT)[0]) return 0;
    return (const volatile u8 *)ui_word(h,UI_KEYBOARD_TEXT);
}
/* Before the change to the keyboard: the borrow starts. 0 when another use
   holds the keyboard model (GR IV). */
static u32 ui_keyboard_arm(volatile u32 *h) {
#ifdef FW_KEYBOARD_MODEL
    u32 model=CALL0(FW_KEYBOARD_MODEL)();
    /* SetFunction asserts that no use is set. */
    if(BYTE(model,KEYBOARD_MODEL_FUNCTION)) return 0;
    CALL2(FW_KEYBOARD_SET_FUNCTION)(model,KEYBOARD_COPYRIGHT);
#endif
    ui_word(h,UI_KEYBOARD)[0]=UI_KEYBOARD_ARMED;
    ui_word(h,UI_KEYBOARD_RESULT)[0]=0;
    return 1;
}
/* A controller change (menu_settings_controller): a change to any other
   controller than the keyboard ends the borrow. */
static void ui_keyboard_change(volatile u32 *h,u32 id) {
    u32 state;
    if(!h[GRKIT_HOST_SERVICES/4] || id==KEYBOARD_CONTROLLER) return;
    state=ui_word(h,UI_KEYBOARD)[0];
    ui_word(h,UI_KEYBOARD)[0]=0;
#ifdef FW_KEYBOARD_MODEL
    /* A keyboard that never started does not stop, so the copyright use
       that GRKit set stays: clear it. */
    if(state==UI_KEYBOARD_ARMED) {
        u32 model=CALL0(FW_KEYBOARD_MODEL)();
        if(BYTE(model,KEYBOARD_MODEL_FUNCTION)==KEYBOARD_COPYRIGHT) BYTE(model,KEYBOARD_MODEL_FUNCTION)=0;
    }
#else
    (void)state;
#endif
}
/* The hooks. The keyboard loads its copyright text: 1 when GRKit gave its
   text (r1, the text of the keyboard), else 0 for the copyright getter. */
A32_ENTRY u32 menu_keyboard_text_in(u32 settings,u32 text) {
    volatile u32 *h=menu_header();
    const volatile u8 *from;
    u32 i;
    (void)settings;
    if(!h[GRKIT_HOST_SERVICES/4] || !ui_word(h,UI_KEYBOARD)[0]) return 0;
    from=(const volatile u8 *)ui_word(h,UI_KEYBOARD_TEXT);
    for(i=0;i<UI_KEYBOARD_CHARS && from[i];++i) BYTE(text,i)=from[i];
    BYTE(text,i)=0;
    ui_word(h,UI_KEYBOARD)[0]=UI_KEYBOARD_SHOWN;
    return 1;
}
/* OK on the keyboard stores its copyright text: 1 when GRKit took the text,
   else 0 for the copyright setter. */
A32_ENTRY u32 menu_keyboard_text_out(u32 settings,u32 text) {
    volatile u32 *h=menu_header();
    volatile u8 *to;
    u32 i;
    (void)settings;
    if(!h[GRKIT_HOST_SERVICES/4] || ui_word(h,UI_KEYBOARD)[0]!=UI_KEYBOARD_SHOWN) return 0;
    to=(volatile u8 *)ui_word(h,UI_KEYBOARD_TEXT);
    for(i=0;i<UI_KEYBOARD_CHARS && BYTE(text,i);++i) to[i]=BYTE(text,i);
    to[i]=0;
    ui_word(h,UI_KEYBOARD_RESULT)[0]=1;
    ui_word(h,UI_KEYBOARD)[0]=0;
    return 1;
}
/* The text length of the keyboard (r0): UI_KEYBOARD_CHARS for the GRKit
   text, else 0 for the native length. */
A32_ENTRY u32 menu_keyboard_limit(u32 keyboard) {
    volatile u32 *h=menu_header();
    if(!h[GRKIT_HOST_SERVICES/4] || !ui_word(h,UI_KEYBOARD)[0] ||
       BYTE(keyboard,KEYBOARD_FUNCTION)!=KEYBOARD_COPYRIGHT) return 0;
    return UI_KEYBOARD_CHARS;
}
/* The GRKit page shows again after a native screen (menu_refresh). */
static void ui_native_returned(volatile u32 *h) {
    if(!h[GRKIT_HOST_SERVICES/4] || !ui_word(h,UI_BACK)[0] || ui_word(h,UI_NATIVE)[0]) return;
    ui_word(h,UI_NATIVE_TIMER)[0]=ui_timer_start(1);
    if(ui_word(h,UI_NATIVE_TIMER)[0]) ui_word(h,UI_NATIVE)[0]=UI_NATIVE_BACK;
    else ui_word(h,UI_BACK)[0]=0;
}
#endif
