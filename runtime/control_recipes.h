#ifndef GRKIT_CONTROL_RECIPES_H
#define GRKIT_CONTROL_RECIPES_H
/* The commands of the Wi-Fi service for the recipes (grkit_core::abi,
   CONTROL_RECIPES_READ and after), answered in the poll of the mailbox
   (runtime/control_looks.h). The recipes live in the header
   (runtime/recipes.h), so each change applies at once, and the resident
   loader saves the recipe file after GRKIT_RECIPES_CHANGED, as after a
   change in MENU. A recipe is found by its ID (runtime/recipe_id.h), which
   stays with it when it moves.

   A change of the list waits while a page of GRKit shows, as a change of
   the effects does (control_looks_idle): the pages of the recipes keep
   their rows while they show. */

/* The slot of the recipe with ID `id`, or RECIPE_NONE. */
static u32 control_recipe_slot(volatile u32 *h,u32 id) {
    u32 n=recipe_count(h), i;
    for(i=0;id && i<n;++i) if(*recipe_id_at(h,i)==id) return i;
    return RECIPE_NONE;
}
static u32 control_recipes_read(volatile u32 *h,const u8 *payload,u32 length,u8 *out,u32 *size) {
    u32 n=recipe_count(h), first, k, i, w, at=GRKIT_CONTROL_RECIPES_HEADER, current=RECIPE_NONE, now[GRKIT_RECIPE_WORDS];
    if(length!=4) return CONTROL_BAD_LENGTH;
    first=control_looks_word(payload);
    if(first>n) return CONTROL_INVALID;
    k=n-first;
    if(k>GRKIT_CONTROL_RECIPES_PAGE) k=GRKIT_CONTROL_RECIPES_PAGE;
    /* The phone lists the recipes that are off too, and applies them. */
    if(n) recipe_capture(h,now);
    for(i=0;i<n && current==RECIPE_NONE;++i) if(recipe_matches_now(h,i,now)) current=i;
    control_looks_put(out,n);
    control_looks_put(out+4,recipe_entries(h));
    control_looks_put(out+8,current==RECIPE_NONE ? GRKIT_CONTROL_RECIPE_NONE : current);
    control_looks_put(out+12,h[GRKIT_HOST_RECIPE_CHANGES/4]);
    control_looks_put(out+16,recipe_undo_count(h));
    control_looks_put(out+20,first);
    control_looks_put(out+24,k);
    for(i=first;i<first+k;++i) {
        control_looks_put(out+at,*recipe_id_at(h,i)); at+=4;
        for(w=0;w<GRKIT_RECIPE_WORDS;++w,at+=4) control_looks_put(out+at,recipe_at(h,i)[w]);
        for(w=0;w<GRKIT_RECIPE_NAME_WORDS;++w,at+=4) control_looks_put(out+at,recipe_name_at(h,i)[w]);
    }
    *size=at;
    return 0;
}
/* Move slot `from` to slot `to` with its name and ID; the slots between
   them move by one. */
static void control_recipe_shift(volatile u32 *h,u32 from,u32 to) {
    u32 w[GRKIT_RECIPE_WORDS], name[GRKIT_RECIPE_NAME_WORDS], id, i;
    for(i=0;i<GRKIT_RECIPE_WORDS;++i) w[i]=recipe_at(h,from)[i];
    for(i=0;i<GRKIT_RECIPE_NAME_WORDS;++i) name[i]=recipe_name_at(h,from)[i];
    id=*recipe_id_at(h,from);
    for(i=from;i<to;++i) recipe_slot_move(h,i,i+1U);
    for(i=from;i>to;--i) recipe_slot_move(h,i,i-1U);
    for(i=0;i<GRKIT_RECIPE_WORDS;++i) recipe_at(h,to)[i]=w[i];
    for(i=0;i<GRKIT_RECIPE_NAME_WORDS;++i) recipe_name_at(h,to)[i]=name[i];
    *recipe_id_at(h,to)=id;
}
/* A recipe that turns off goes before the others that are off; one that
   turns on, after the others that are on, while fewer than
   GRKIT_RECIPE_ON_MAX are on. */
static u32 control_recipe_turn(volatile u32 *h,const u8 *payload,u32 length) {
    u32 slot, off, e;
    if(length!=8) return CONTROL_BAD_LENGTH;
    slot=control_recipe_slot(h,control_looks_word(payload));
    off=control_looks_word(payload+4);
    if(off>1) return CONTROL_INVALID;
    if(slot==RECIPE_NONE) return CONTROL_CHANGED;
    if(!control_looks_idle(h)) return CONTROL_BUSY;
    e=recipe_entries(h);
    if(off && slot<e) {
        control_recipe_shift(h,slot,e-1U);
        recipe_at(h,e-1U)[RECIPE_MODULE_WORD]|=RECIPE_OFF;
    } else if(!off && slot>=e) {
        if(e>=GRKIT_RECIPE_ON_MAX) return CONTROL_INVALID;
        control_recipe_shift(h,slot,e);
        recipe_at(h,e)[RECIPE_MODULE_WORD]&=~RECIPE_OFF;
    } else return 0;
    GRKIT_RECIPES_CHANGED(h);
    return 0;
}
/* The ID of each recipe that is on, once, in the new order. */
static u32 control_recipes_order(volatile u32 *h,const u8 *payload,u32 length) {
    u32 e=recipe_entries(h), i, j, slot;
    if(length!=4*e) return CONTROL_CHANGED;
    for(i=0;i<e;++i) {
        slot=control_recipe_slot(h,control_looks_word(payload+4*i));
        if(slot==RECIPE_NONE || slot>=e) return CONTROL_CHANGED;
        for(j=0;j<i;++j) if(control_looks_word(payload+4*j)==control_looks_word(payload+4*i)) return CONTROL_INVALID;
    }
    if(!control_looks_idle(h)) return CONTROL_BUSY;
    for(i=0;i<e;++i) {
        slot=control_recipe_slot(h,control_looks_word(payload+4*i));
        if(slot!=i) recipe_move(h,slot,i);
    }
    return 0;
}
/* The ID, then the name in printable ASCII, NUL-padded to whole words as
   the mailbox carries them; no name gives the automatic name. */
static u32 control_recipe_rename(volatile u32 *h,const u8 *payload,u32 length) {
    u8 text[GRKIT_RECIPE_NAME_CHARS+1U];
    u32 slot, i, n=0;
    if(length<4 || length>4U+GRKIT_RECIPE_NAME_CHARS || length&3U) return CONTROL_BAD_LENGTH;
    slot=control_recipe_slot(h,control_looks_word(payload));
    if(slot==RECIPE_NONE) return CONTROL_CHANGED;
    while(4U+n<length && payload[4U+n]) ++n;
    for(i=0;i<n;++i) if(payload[4U+i]<0x20U || payload[4U+i]>0x7eU) return CONTROL_INVALID;
    for(i=4U+n;i<length;++i) if(payload[i]) return CONTROL_INVALID;
    if(!control_looks_idle(h)) return CONTROL_BUSY;
    for(i=0;i<n;++i) text[i]=payload[4+i];
    text[n]=0;
    recipe_name_set(h,slot,text);
    return 0;
}
static u32 control_recipe_delete(volatile u32 *h,const u8 *payload,u32 length) {
    u32 slot;
    if(length!=4) return CONTROL_BAD_LENGTH;
    slot=control_recipe_slot(h,control_looks_word(payload));
    if(slot==RECIPE_NONE) return CONTROL_CHANGED;
    if(!control_looks_idle(h)) return CONTROL_BUSY;
    recipe_close_gap(h,slot,recipe_count(h));
    GRKIT_RECIPES_CHANGED(h);
    return 0;
}
/* Apply as OK in ADJ does: the settings before it become an undo step. A
   recipe that is off applies too: the phone lists it. */
static u32 control_recipe_apply(volatile u32 *h,u8 *payload,u32 length,u32 *size) {
    u32 slot, w[GRKIT_RECIPE_WORDS], reason, i, undo, result;
    if(length!=4) return CONTROL_BAD_LENGTH;
    slot=control_recipe_slot(h,control_looks_word(payload));
    if(slot==RECIPE_NONE) return CONTROL_CHANGED;
    if(!control_looks_idle(h)) return CONTROL_BUSY;
    for(i=0;i<GRKIT_RECIPE_WORDS;++i) w[i]=recipe_at(h,slot)[i];
    reason=recipe_reason(h,w);
    if(reason!=RECIPE_READY) result=reason;
    else {
        undo=recipe_undo_push(h);
        result=recipe_set(h,w);
        recipe_native_apply(h,w,undo);
    }
    control_looks_put(payload,result);
    *size=4;
    return 0;
}
static u32 control_recipe_undo(volatile u32 *h,u8 *payload,u32 length,u32 *size) {
    if(length) return CONTROL_BAD_LENGTH;
    if(!control_looks_idle(h)) return CONTROL_BUSY;
    if(!recipe_undo_count(h)) return CONTROL_INVALID;
    recipe_undo(h);
    control_looks_put(payload,recipe_undo_count(h));
    *size=4;
    return 0;
}
/* A recipe of the phone, such as a preset: on after the others that are on
   while fewer than GRKIT_RECIPE_ON_MAX are on, else off after the others. */
static u32 control_recipe_add(volatile u32 *h,u8 *payload,u32 length,u32 *size) {
    u32 n=recipe_count(h), e=recipe_entries(h), w[GRKIT_RECIPE_WORDS], name[GRKIT_RECIPE_NAME_WORDS], i, slot, id;
    if(length!=4U*(GRKIT_RECIPE_WORDS+GRKIT_RECIPE_NAME_WORDS)) return CONTROL_BAD_LENGTH;
    for(i=0;i<GRKIT_RECIPE_WORDS;++i) w[i]=control_looks_word(payload+4*i);
    for(i=0;i<GRKIT_RECIPE_NAME_WORDS;++i) name[i]=control_looks_word(payload+4*(GRKIT_RECIPE_WORDS+i));
    if(!recipe_word_valid(w[0])) return CONTROL_INVALID;
    for(i=1;i<GRKIT_RECIPE_WORDS;++i) if(w[i]>>GRKIT_RECIPE_VALUE_BITS) return CONTROL_INVALID;
    if(n>=GRKIT_RECIPE_SLOTS) return CONTROL_INVALID;
    if(!control_looks_idle(h)) return CONTROL_BUSY;
    id=recipe_new_id(h);
    if(!id) return CONTROL_INVALID;
    slot=e<GRKIT_RECIPE_ON_MAX ? e : n;
    for(i=n;i>slot;--i) recipe_slot_move(h,i,i-1U);
    recipe_slot_clear(h,slot);
    if(slot==e) w[RECIPE_MODULE_WORD]&=~RECIPE_OFF;
    else w[RECIPE_MODULE_WORD]|=RECIPE_OFF;
    for(i=0;i<GRKIT_RECIPE_WORDS;++i) recipe_at(h,slot)[i]=w[i];
    for(i=0;i<GRKIT_RECIPE_NAME_WORDS;++i) recipe_name_at(h,slot)[i]=name[i];
    *recipe_id_at(h,slot)=id;
    GRKIT_RECIPES_CHANGED(h);
    control_looks_put(payload,id);
    *size=4;
    return 0;
}
static u32 control_recipes(volatile u32 *h,struct control_request *request,u32 *size) {
    switch(request->command) {
    case GRKIT_CONTROL_RECIPES_READ: return control_recipes_read(h,request->payload,request->length,request->payload,size);
    case GRKIT_CONTROL_RECIPE_TURN: return control_recipe_turn(h,request->payload,request->length);
    case GRKIT_CONTROL_RECIPES_ORDER: return control_recipes_order(h,request->payload,request->length);
    case GRKIT_CONTROL_RECIPE_RENAME: return control_recipe_rename(h,request->payload,request->length);
    case GRKIT_CONTROL_RECIPE_DELETE: return control_recipe_delete(h,request->payload,request->length);
    case GRKIT_CONTROL_RECIPE_APPLY: return control_recipe_apply(h,request->payload,request->length,size);
    case GRKIT_CONTROL_RECIPE_UNDO: return control_recipe_undo(h,request->payload,request->length,size);
    case GRKIT_CONTROL_RECIPE_ADD: return control_recipe_add(h,request->payload,request->length,size);
    default: return CONTROL_UNSUPPORTED;
    }
}
#endif
