#ifndef GRKIT_UI_KEYBOARD_H
#define GRKIT_UI_KEYBOARD_H
/* The keyboard of the camera, for the UI services (runtime/ui.h).

   The camera has one text keyboard, SoftwareKeyboardController, with a few
   fixed uses: each reads its text from a camera setting when it starts and
   writes the setting on OK. GRKit borrows its copyright use while
   UI_KEYBOARD is set. Three hooks (runtime/menu.S) then give the keyboard
   the GRKit text instead of the copyright holder, take the text on OK
   instead of the copyright setter, and limit the text to the most
   characters of the use: UI_KEYBOARD_SHORT, such as for a recipe name, or
   up to UI_KEYBOARD_CHARS, as for the name of a user mode of the mode dial.
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
   the keyboard clears it when it stops.

   The includer is runtime/menu.c, after runtime/ui.h. */
#define UI_KEYBOARD_ARMED 1U
#define UI_KEYBOARD_SHOWN 2U
/* The text that the keyboard starts with for `text`, in `to`: its
   printable ASCII without the spaces at both ends, at most `limit`
   characters, then a NUL. */
static void ui_keyboard_start(volatile u8 *to,const volatile u16 *text,u32 limit) {
    u32 n=0, i;
    for(i=0;text[i] && n<limit;++i)
        if(text[i]>=0x20U && text[i]<=0x7eU && (n || text[i]!=' ')) to[n++]=(u8)text[i];
    while(n && to[n-1]==' ') --n;
    to[n]=0;
}
/* The most characters of the keyboard now. */
static u32 ui_keyboard_limit(volatile u32 *h) {
    u32 limit=ui_word(h,UI_KEYBOARD_LIMIT)[0];
    return limit && limit<=UI_KEYBOARD_CHARS ? limit : UI_KEYBOARD_SHORT;
}
/* Start the keyboard with `text`, of at most `limit` characters. A page of
   MENU gives it, then changes to the keyboard itself (ui_keyboard_on). */
static void ui_keyboard_text(volatile u32 *h,const volatile u16 *text,u32 limit) {
    ui_word(h,UI_KEYBOARD_LIMIT)[0]=limit;
    ui_keyboard_start((volatile u8 *)ui_word(h,UI_KEYBOARD_TEXT),text,ui_keyboard_limit(h));
    ui_word(h,UI_KEYBOARD_RESULT)[0]=0;
}
/* Prepare the keyboard for a page of MENU, which changes to it next, with
   `text` of at most `limit` characters: 1, or 0 when another use holds the
   keyboard. The change from the keyboard calls done(arg)
   (ui_keyboard_change), where ui_keyboard_result gives the text of an OK. */
static u32 ui_keyboard_prepare(volatile u32 *h,const volatile u16 *text,u32 limit,void (*done)(u32),u32 arg) {
    ui_keyboard_text(h,text,limit);
    if(!ui_keyboard_arm(h)) return 0;
    ui_word(h,UI_KEYBOARD_DONE)[0]=(u32)done;
    ui_word(h,UI_KEYBOARD_ARG)[0]=arg;
    return 1;
}
/* Open the keyboard from ADJ with `text`: 1, or 0 as ui_open_native
   refuses. After the return to the GRKit page the core calls back(arg),
   where ui_keyboard_result gives the text of an OK. If the keyboard is busy,
   back runs at once on the page. */
static u32 ui_keyboard_open(volatile u32 *h,const volatile u16 *text,void (*back)(u32),u32 arg) {
    if(!ui_native_request(h,KEYBOARD_CONTROLLER,back,arg)) return 0;
    ui_keyboard_text(h,text,UI_KEYBOARD_SHORT);
    return 1;
}
/* Whether GRKit holds the keyboard. */
static u32 ui_keyboard_borrowed(volatile u32 *h) { return ui_word(h,UI_KEYBOARD)[0]!=0; }
/* The text of an OK on the keyboard, during the call of back or the return
   to the MENU page, else 0. */
static const volatile u8 *ui_keyboard_result(volatile u32 *h) {
    if(!h[GRKIT_HOST_SERVICES/4] || !ui_word(h,UI_KEYBOARD_RESULT)[0]) return 0;
    return (const volatile u8 *)ui_word(h,UI_KEYBOARD_TEXT);
}
/* Whether OK on the keyboard gave the text that it started with for
   `text`: the person did not change it. */
static u32 ui_keyboard_kept(volatile u32 *h,const volatile u16 *text) {
    u8 start[UI_KEYBOARD_CHARS+1U];
    const volatile u8 *typed=ui_keyboard_result(h);
    u32 i;
    if(!typed) return 0;
    ui_keyboard_start(start,text,ui_keyboard_limit(h));
    for(i=0;i<=UI_KEYBOARD_CHARS && start[i]==typed[i];++i) if(!start[i]) return 1;
    return 0;
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
    u32 state, done;
    if(!h[GRKIT_HOST_SERVICES/4] || id==KEYBOARD_CONTROLLER) return;
    state=ui_word(h,UI_KEYBOARD)[0];
    ui_word(h,UI_KEYBOARD)[0]=0;
    /* The return from a keyboard that a page of MENU opened. */
    done=ui_word(h,UI_KEYBOARD_DONE)[0];
    ui_word(h,UI_KEYBOARD_DONE)[0]=0;
    if(done) services_call1(h,done,ui_word(h,UI_KEYBOARD_ARG)[0]);
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
    for(i=0;i<ui_keyboard_limit(h) && from[i];++i) BYTE(text,i)=from[i];
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
    for(i=0;i<ui_keyboard_limit(h) && BYTE(text,i);++i) to[i]=BYTE(text,i);
    to[i]=0;
    ui_word(h,UI_KEYBOARD_RESULT)[0]=1;
    ui_word(h,UI_KEYBOARD)[0]=0;
    return 1;
}
/* The text length of the keyboard (r0): the most characters of the use for
   the GRKit text, else 0 for the native length. */
A32_ENTRY u32 menu_keyboard_limit(u32 keyboard) {
    volatile u32 *h=menu_header();
    if(!h[GRKIT_HOST_SERVICES/4] || !ui_word(h,UI_KEYBOARD)[0] ||
       BYTE(keyboard,KEYBOARD_FUNCTION)!=KEYBOARD_COPYRIGHT) return 0;
    return ui_keyboard_limit(h);
}
#endif
