#ifndef GRKIT_CONTROL_LOOKS_H
#define GRKIT_CONTROL_LOOKS_H
/* The commands of the Wi-Fi service for the effects (grkit_core::abi,
   CONTROL_LOOKS_READ and after). The App task answers them in its poll of
   the mailbox (runtime/control_mailbox.h). They read the effect list and set
   the choice of an effect as its checkbox and Order in MENU do
   (runtime/effects.h): no command reads the card, loads an effect, or
   removes one. A new, changed, or removed effect file applies at the next
   start (runtime/install_effects.h).

   The key of an effect is the ID of its choice in the settings table: the
   state ID of CORE.EFFECT.<its ID>. */
static u32 control_looks_word(const u8 *p) { return (u32)p[0]|((u32)p[1]<<8)|((u32)p[2]<<16)|((u32)p[3]<<24); }
static void control_looks_put(u8 *p,u32 value) { p[0]=(u8)value; p[1]=(u8)(value>>8); p[2]=(u8)(value>>16); p[3]=(u8)(value>>24); }
static u32 control_looks_key(volatile u32 *h,u32 effect) {
    volatile u32 *t=settings_table(h);
    u32 handle=effects_handle(h,effect);
    return t && handle<t[0] ? t[SETTINGS_ENTRY(handle)] : 0;
}
/* The effect of a key, or EFFECTS_LISTED without one. */
static u32 control_looks_effect(volatile u32 *h,u32 key) {
    u32 n=effects_count(h), i;
    for(i=0;key && i<n;++i) if(control_looks_key(h,i)==key) return i;
    return EFFECTS_LISTED;
}
/* A page of GRKit that lists the effects or the recipes keeps its rows
   while it shows, so they change while none shows: not the GRKit menu of
   ADJ, a preview, the adjustments of an effect, or a development, nor in
   MENU a page of GRKit (the settings submenu and its value pages,
   settings_shows) or the camera information page with a GRKit view. The
   shooting screen, playback, and the other pages of MENU take changes:
   each GRKit row of MENU reads its value when it draws. */
static u32 control_looks_idle(volatile u32 *h) {
    u32 current=services_word(h,SERVICES_MENU+MENU_ROWS_CURRENT)[0];
    return !h[GRKIT_HOST_MENU_CONTROLLER/4] && !h[GRKIT_HOST_PREVIEW/4] && !h[GRKIT_HOST_DEVELOP/4] &&
           !(h[GRKIT_HOST_PARAMETERS/4]&~1U) && !settings_shows(current) &&
           !(current==INFO_CONTROLLER && services_word(h,SERVICES_MENU+MENU_ROWS_ABOUT)[0]);
}
static u32 control_looks_read(volatile u32 *h,u8 *out,u32 *length) {
    u32 n=effects_count(h), i, at=GRKIT_CONTROL_LOOKS_HEADER, selected=0, codes=0, names;
    u32 active=h[GRKIT_HOST_CAPABILITIES/4]&GRKIT_CORE_TONE_BIT ? h[GRKIT_HOST_ACTIVE/4] : 0;
    if(n>EFFECTS_LISTED) return CONTROL_INVALID;
    for(i=0;i<n;++i,at+=GRKIT_CONTROL_LOOK_ENTRY) {
        u32 record=effects_record(h,i), key=control_looks_key(h,i);
        if(active && record!=EFFECTS_NO_RECORD && active==tone_record(h,record+1)) selected=key;
        control_looks_put(out+at,key);
        control_looks_put(out+at+4,effects_choice(h,i));
        control_looks_put(out+at+8,record!=EFFECTS_NO_RECORD);
    }
    /* One word for each recipe and dial set that names an effect, and the
       name words of a recipe while the payload has room for them after the
       words of the uses still to come. */
    for(i=0;i<GRKIT_RECIPE_SLOTS;++i) {
        u32 word=h[GRKIT_HOST_RECIPES/4+i*GRKIT_RECIPE_WORDS];
        if((word&RECIPE_USED) && RECIPE_LOOK(word)==RECIPE_LOOK_EFFECT) ++codes;
    }
    for(i=1;i<=GRKIT_DIAL_USER_MODES;++i)
        if(dial_has_set(h,i) && dial_set(h,i)[GRKIT_DIAL_SET_CORE+CORE_SETTING(GRKIT_CORE_TONE)]&GRKIT_DIAL_SET_EFFECT) ++codes;
    names=CONTROL_PAYLOAD_BYTES-at-4*codes;
    for(i=0;i<GRKIT_RECIPE_SLOTS;++i) {
        u32 word=h[GRKIT_HOST_RECIPES/4+i*GRKIT_RECIPE_WORDS], use, w;
        if(!(word&RECIPE_USED) || RECIPE_LOOK(word)!=RECIPE_LOOK_EFFECT) continue;
        use=RECIPE_CODE(word)|(i+1U)<<GRKIT_CONTROL_USE_ORIGIN_SHIFT|RECIPE_NUMBER(word)<<GRKIT_CONTROL_USE_NUMBER_SHIFT;
        if(names>=4U*GRKIT_RECIPE_NAME_WORDS) use|=GRKIT_CONTROL_USE_NAMED;
        control_looks_put(out+at,use); at+=4;
        if(use&GRKIT_CONTROL_USE_NAMED) {
            for(w=0;w<GRKIT_RECIPE_NAME_WORDS;++w,at+=4) control_looks_put(out+at,recipe_name_at(h,i)[w]);
            names-=4U*GRKIT_RECIPE_NAME_WORDS;
        }
    }
    for(i=1;i<=GRKIT_DIAL_USER_MODES;++i) {
        u32 effect=dial_has_set(h,i) ? dial_set(h,i)[GRKIT_DIAL_SET_CORE+CORE_SETTING(GRKIT_CORE_TONE)] : 0;
        if(effect&GRKIT_DIAL_SET_EFFECT) {
            control_looks_put(out+at,(effect&~GRKIT_DIAL_SET_EFFECT)|i<<GRKIT_CONTROL_USE_ORIGIN_SHIFT|GRKIT_CONTROL_USE_DIAL);
            at+=4;
        }
    }
    control_looks_put(out,n);
    control_looks_put(out+4,selected);
    control_looks_put(out+8,h[GRKIT_HOST_STATE_CHANGES/4]);
    control_looks_put(out+12,codes);
    *length=at;
    return 0;
}
static u32 control_looks_turn(volatile u32 *h,const u8 *payload,u32 length) {
    u32 effect, off;
    if(length!=8) return CONTROL_BAD_LENGTH;
    effect=control_looks_effect(h,control_looks_word(payload));
    off=control_looks_word(payload+4);
    if(off>1) return CONTROL_INVALID;
    if(effect==EFFECTS_LISTED) return CONTROL_CHANGED;
    if(!control_looks_idle(h)) return CONTROL_BUSY;
    if(effects_off(h,effect)!=off) effects_turn(h,effect,off);
    return 0;
}
/* Each listed effect once, in the new order: its row is its position, as
   effects_move gives it. */
static u32 control_looks_order(volatile u32 *h,const u8 *payload,u32 length) {
    u8 order[EFFECTS_LISTED];
    u32 n=effects_count(h), i, j;
    if(n>EFFECTS_LISTED || length!=4*n) return n>EFFECTS_LISTED ? CONTROL_INVALID : CONTROL_CHANGED;
    for(i=0;i<n;++i) {
        u32 effect=control_looks_effect(h,control_looks_word(payload+4*i));
        if(effect==EFFECTS_LISTED) return CONTROL_CHANGED;
        for(j=0;j<i;++j) if(order[j]==effect) return CONTROL_INVALID;
        order[i]=(u8)effect;
    }
    if(!control_looks_idle(h)) return CONTROL_BUSY;
    for(i=0;i<n;++i) {
        u32 choice=(i+1U)<<1|effects_off(h,order[i]);
        if(effects_choice(h,order[i])!=choice) setting_store(h,effects_handle(h,order[i]),choice);
    }
    return 0;
}
#include "control_recipes.h"
static u32 control_looks(void *context,struct control_request *request,u32 *length) {
    volatile u32 *h=(volatile u32 *)context;
    if(request->command>=GRKIT_CONTROL_RECIPES_READ)
        return h[GRKIT_HOST_CAPABILITIES/4]&GRKIT_CORE_RECIPE_BIT ? control_recipes(h,request,length) : CONTROL_UNSUPPORTED;
    if(!(h[GRKIT_HOST_CAPABILITIES/4]&GRKIT_CORE_TONE_BIT)) return CONTROL_UNSUPPORTED;
    if(request->command==GRKIT_CONTROL_LOOKS_READ)
        return request->length ? CONTROL_BAD_LENGTH : control_looks_read(h,request->payload,length);
    if(request->command==GRKIT_CONTROL_LOOK_TURN) return control_looks_turn(h,request->payload,request->length);
    if(request->command==GRKIT_CONTROL_LOOKS_ORDER) return control_looks_order(h,request->payload,request->length);
    return CONTROL_UNSUPPORTED;
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
    control_poll_once(box,(struct control_request *)services_word(h,SERVICES_CONTROL_CANDIDATE),control_looks,(void *)h);
}
#endif
