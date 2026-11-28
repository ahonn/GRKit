#ifndef GRKIT_EFFECTS_H
#define GRKIT_EFFECTS_H
/* The effects of GRKIT/EFFECTS: the list of each effect file, also one that
   did not install. The options rows of a page of CORE.TONE in the GRKit
   section of MENU turn each effect on or off (runtime/menu_listed.h). A change of
   an installed effect applies at once: the page of CORE.TONE shows the
   installed effects that are On, in their order (runtime/tone.h). On of an
   effect that did not install applies at the next start, and the About
   GRKit row says so until then.

   One setting of the settings table keeps the choice of each listed effect
   (runtime/settings.h): CORE.EFFECT.<effect ID>, with Off in bit 0, and its
   position + 1, or 0 for the default place, above. The Order row of the page
   in the GRKit section of MENU sets the positions (effects_move). The resident loader saves
   it as other settings, and SCAN reads it to select and order the effects
   (runtime/install_effects.h). GRKit installs at most MAX_COUNT effects: those
   that are On, in that order, then those that are Off while there is room.

   INSTALL writes the effect list (GRKIT_HOST_EFFECTS): EFFECTS_HEADER bytes,
   then EFFECTS_STRIDE bytes for each listed effect, in the order of the last
   start. The includer defines u16, u32, and the header fields. */
/* The most effect files that GRKit lists, those with the smallest IDs. */
#define EFFECTS_LISTED GRKIT_EFFECTS_LISTED
#define EFFECTS_COUNT 0U
/* The handle of the choice of effect 0. Effect i has handle base + i. */
#define EFFECTS_SETTINGS 4U
#define EFFECTS_HEADER 8U
/* Each effect: its status at this start, its record index or EFFECTS_NO_RECORD,
   then its name, NUL-terminated. */
#define EFFECTS_STATUS 0U
#define EFFECTS_RECORD 4U
#define EFFECTS_NAME 8U
#define EFFECTS_NAME_UNITS 18U
#define EFFECTS_STRIDE (EFFECTS_NAME + 2U * EFFECTS_NAME_UNITS)
#define EFFECTS_NO_RECORD 0xffffffffU
/* The status of an effect that installed, of one that is Off, of one past
   MAX_COUNT, of one whose file changed during the start, and of one whose ID
   has the code of an effect with a smaller ID (runtime/install_effects.h). */
#define EFFECT_LOADED 0U
#define EFFECT_OFF 2U
#define EFFECT_OVER 3U
#define EFFECT_UNREADABLE 4U
#define EFFECT_CODE_TAKEN 5U
/* A choice: Off in bit 0, the position + 1 above. */
#define EFFECT_CHOICE_VALUES (2U * (EFFECTS_LISTED + 1U))
#define EFFECTS_DEFAULT_KEY 1000U

#ifdef MENU_TEXT
#ifndef MANAGE_MAP
#define MANAGE_MAP(address) ((volatile u32 *)(address))
#endif
static u32 effects_count(volatile u32 *h) {
    u32 list=h[GRKIT_HOST_EFFECTS/4];
    u32 n=list ? MANAGE_MAP(list+EFFECTS_COUNT)[0] : 0;
    return n<=EFFECTS_LISTED ? n : 0;
}
static volatile u32 *effects_word(volatile u32 *h,u32 offset) { return MANAGE_MAP(h[GRKIT_HOST_EFFECTS/4]+offset); }
static u32 effects_entry(volatile u32 *h,u32 effect) { return h[GRKIT_HOST_EFFECTS/4]+EFFECTS_HEADER+effect*EFFECTS_STRIDE; }
static u32 effects_handle(volatile u32 *h,u32 effect) { return effects_word(h,EFFECTS_SETTINGS)[0]+effect; }
static u32 effects_choice(volatile u32 *h,u32 effect) { return setting_value(h,effects_handle(h,effect)); }
static u32 effects_off(volatile u32 *h,u32 effect) { return effects_choice(h,effect)&1U; }
static u32 effects_key(volatile u32 *h,u32 effect) {
    u32 position=effects_choice(h,effect)>>1;
    return position ? position : EFFECTS_DEFAULT_KEY+effect;
}
/* The effects in the order of their keys. Equal keys keep the order of this
   start. An insertion sort: the list is short, and the menu sorts only on a
   key press. */
/* u8 and u16 keep the frames of the menu small. */
static void effects_order(volatile u32 *h,u8 *order,u32 n) {
    u16 keys[EFFECTS_LISTED];
    u32 i, j;
    for(i=0;i<n;++i) {
        u32 key=effects_key(h,i);
        for(j=i;j && keys[j-1]>key;--j) { keys[j]=keys[j-1]; order[j]=order[j-1]; }
        keys[j]=(u16)key; order[j]=(u8)i;
    }
}
/* The effect of a row. */
static u32 effects_at(volatile u32 *h,u32 row) {
    u8 order[EFFECTS_LISTED];
    u32 n=effects_count(h);
    effects_order(h,order,n);
    return row<n ? order[row] : 0;
}
/* The record index of an effect, or EFFECTS_NO_RECORD. */
static u32 effects_record(volatile u32 *h,u32 effect) {
    u32 record=MANAGE_MAP(effects_entry(h,effect)+EFFECTS_RECORD)[0];
    return record<h[GRKIT_HOST_COUNT/4] ? record : EFFECTS_NO_RECORD;
}
/* Whether a choice waits for the next start: an effect that is On did not
   install, and its file can. */
static u32 effects_changed(volatile u32 *h) {
    u32 n=effects_count(h), i, status;
    for(i=0;i<n;++i) {
        status=MANAGE_MAP(effects_entry(h,i)+EFFECTS_STATUS)[0];
        if(!effects_off(h,i) && effects_record(h,i)==EFFECTS_NO_RECORD &&
           status!=EFFECT_UNREADABLE && status!=EFFECT_CODE_TAKEN) return 1;
    }
    return 0;
}
/* Turn an effect on or off. Off of the committed effect commits Off. */
static void effects_turn(volatile u32 *h,u32 effect,u32 off) {
    u32 record=effects_record(h,effect);
    setting_store(h,effects_handle(h,effect),(effects_choice(h,effect)&~1U)|(off&1U));
    if(off && record!=EFFECTS_NO_RECORD && h[GRKIT_HOST_ACTIVE/4]==tone_record(h,record+1))
        setting_store(h,GRKIT_CORE_TONE,0);
}
/* Move the effect of row `from` to row `to` of the order of the effects
   (effects_at): the effects between them move by one row. Each effect then
   gets its row as its position, so the page of CORE.TONE shows the order at
   once, and the next start installs the effects in it. */
static void effects_move(volatile u32 *h,u32 from,u32 to) {
    u8 order[EFFECTS_LISTED];
    u32 n=effects_count(h), i, effect;
    if(from>=n || to>=n || from==to) return;
    effects_order(h,order,n);
    effect=order[from];
    for(i=from;i<to;++i) order[i]=order[i+1];
    for(i=from;i>to;--i) order[i]=order[i-1];
    order[to]=(u8)effect;
    for(i=0;i<n;++i) setting_store(h,effects_handle(h,order[i]),(i+1U)<<1|effects_off(h,order[i]));
}
static const u16 *effects_name(volatile u32 *h,u32 effect) {
    return (const u16 *)MANAGE_MAP(effects_entry(h,effect)+EFFECTS_NAME);
}
#endif
#endif
