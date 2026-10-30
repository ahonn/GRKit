#ifndef GRKIT_RECIPE_PAGE_H
#define GRKIT_RECIPE_PAGE_H
/* The rows of a module page of CORE.RECIPE (runtime/recipes.h), for the
   menu. The rows are Undo while an undo step exists, each camera recipe,
   then Save Current Settings. The rows are actions, so an ADJ exit applies
   none of them (MENU_PAGE_ACTIONS).

   - OK on a recipe applies it and closes ADJ, as OK on a native value does.
     A recipe whose effect is not installed shows a message instead. A
     recipe whose Grain has no effect (RECIPE_GRAIN_SKIPPED) applies and
     shows a message.
   - OK on Undo restores the newest undo step and closes ADJ.
   - OK on Save saves the current settings as a new recipe, shows a toast,
     and keeps ADJ open on the new row. With all recipes taken, it shows a
     message.
   - Fn on a recipe asks to delete it.

   The tiles of the module: 0 the module, 1 Undo, 2 Save, 3 a recipe
   without an effect icon. The includer is runtime/modules.h: it defines the
   menu text, the choice functions, and the services; runtime/menu.c
   defines render. The UI services follow in runtime/ui.h. */
#include "native_look.h"
#include "recipes.h"

u32 grkit_export_ui_toast(const u16 *const *text,u32 ms);
u32 grkit_export_ui_message(const u16 *const *text,void (*done)(u32),u32 arg);
u32 grkit_export_ui_confirm(const u16 *const *text,void (*done)(u32,u32),u32 arg);

#define RECIPE_TILE_MODULE 0U
#define RECIPE_TILE_UNDO 1U
#define RECIPE_TILE_SAVE 2U
#define RECIPE_TILE_RECIPE 3U
/* The strings of the RECIPE run of the menu text (runtime/abi.h). */
#define RECIPE_TEXT_UNDO (TEXT_RECIPE+0U)
#define RECIPE_TEXT_SAVE (TEXT_RECIPE+1U)
#define RECIPE_TEXT_SAVED (TEXT_RECIPE+2U)
#define RECIPE_TEXT_DELETE (TEXT_RECIPE+3U)
#define RECIPE_TEXT_DELETE_AFTER (TEXT_RECIPE+4U)
#define RECIPE_TEXT_FULL (TEXT_RECIPE+5U)
#define RECIPE_TEXT_MISSING (TEXT_RECIPE+6U)
#define RECIPE_TEXT_NAME (TEXT_RECIPE+7U)
#define RECIPE_TEXT_SEPARATOR (TEXT_RECIPE+8U)
#define RECIPE_TEXT_UNSUPPORTED (TEXT_RECIPE+9U)
#define RECIPE_TEXT_NO_GRAIN (TEXT_RECIPE+10U)
/* The row kinds besides a recipe slot. */
#define RECIPE_ROW_UNDO 0xfffffffeU
#define RECIPE_ROW_SAVE 0xfffffffdU
/* The units of the effect name of a record, with its NUL. */
#define RECIPE_NAME_UNITS 17U
_Static_assert(RECIPE_ROWS_MAX==GRKIT_RECIPE_SLOTS+2U,"a label for Undo, each recipe, and Save");

/* The row of recipe 0: after Undo, if it shows. */
static u32 recipe_first_row(volatile u32 *h) { return recipe_undo_count(h) ? 1U : 0U; }
static u32 recipe_rows(volatile u32 *h) { return recipe_first_row(h)+recipe_count(h)+1U; }
/* The recipe slot of a row, RECIPE_ROW_UNDO, or RECIPE_ROW_SAVE. */
static u32 recipe_row_slot(volatile u32 *h,u32 row) {
    u32 first=recipe_first_row(h);
    if(row<first) return RECIPE_ROW_UNDO;
    return row-first<recipe_count(h) ? row-first : RECIPE_ROW_SAVE;
}

/* ---- Texts ---------------------------------------------------------------- */
static volatile u16 *recipe_text_at(volatile u32 *h,u32 offset) {
    return (volatile u16 *)services_word(h,SERVICES_RECIPES+offset);
}
/* Append a NUL-terminated text at unit `at` of a text buffer. The result
   stays NUL-terminated and within RECIPE_UNITS units. */
static u32 recipe_append(volatile u16 *to,u32 at,const u16 *from) {
    while(at+1U<RECIPE_UNITS && *from) to[at++]=*from++;
    to[at]=0;
    return at;
}
/* Append a number from 1 to 99. n * 205 >> 11 is n / 10 for n below 1024,
   without a division helper. */
static u32 recipe_append_number(volatile u16 *to,u32 at,u32 n) {
    u16 digits[3]={0,0,0};
    u32 tens=n*205U>>11;
    if(tens) { digits[0]=(u16)('0'+tens); digits[1]=(u16)('0'+n-10U*tens); }
    else digits[0]=(u16)('0'+n);
    return recipe_append(to,at,digits);
}
/* Append the name of an effect from its record. */
static u32 recipe_append_effect(volatile u16 *to,u32 at,u32 record) {
    u32 i;
    u16 unit;
    for(i=0;i<RECIPE_NAME_UNITS;++i) {
        unit=HALF(record,GRKIT_RECORD_NAME+2U*i);
        if(!unit || at+1U>=RECIPE_UNITS) break;
        to[at++]=unit;
    }
    to[at]=0;
    return at;
}
/* Append the name of a recipe: the name of its effect, of its native type,
   Film Look Off, or Recipe, then its number. A recipe whose effect is not
   installed shows that instead. */
static u32 recipe_append_name(volatile u32 *h,volatile u16 *to,u32 at,const volatile u32 *w) {
    static const u16 space[2]={' ',0};
    u32 index, type=NATIVE_TYPE(w[RECIPE_NATIVE_WORD]);
    if(RECIPE_LOOK(w[0])==RECIPE_LOOK_EFFECT) {
        index=recipe_effect(h,RECIPE_CODE(w[0]));
        if(index>=GRKIT_TEXT_RECORDS) return recipe_append(to,at,text(h,RECIPE_TEXT_MISSING));
        at=recipe_append_effect(to,at,recipe_record(h,index));
    }
    else if(type && type<=NATIVE_TYPES) at=recipe_append(to,at,text(h,TEXT_NATIVE_TYPE+type-1U));
    else at=recipe_append(to,at,text(h,RECIPE_LOOK(w[0])==RECIPE_LOOK_OFF ? TEXT_IMAGE_OFF_SUMMARY : RECIPE_TEXT_NAME));
    if(RECIPE_NUMBER(w[0])) at=recipe_append_number(to,recipe_append(to,at,space),RECIPE_NUMBER(w[0]));
    return at;
}
/* The text of a box, for each language the same: only the menu language
   shows (runtime/ui.h). */
static const u16 *const *recipe_box(volatile u32 *h) {
    volatile u32 *table=services_word(h,SERVICES_RECIPES+RECIPE_BOX_TABLE_AT);
    u32 i, at=(u32)recipe_text_at(h,RECIPE_BOX_AT);
    for(i=0;i<GRKIT_TEXT_LANGUAGES;++i) table[i]=at;
    return (const u16 *const *)table;
}

/* ---- Boxes ---------------------------------------------------------------- */
/* The core calls the UI services as module code does: in the App task, with
   the App call depth above 0 (runtime/services.h). */
static void recipe_toast(volatile u32 *h) {
    ++services_word(h,SERVICES_APP)[0];
    grkit_export_ui_toast(recipe_box(h),0);
    --services_word(h,SERVICES_APP)[0];
}
static void recipe_message(volatile u32 *h,u32 id) {
    recipe_append(recipe_text_at(h,RECIPE_BOX_AT),0,text(h,id));
    ++services_word(h,SERVICES_APP)[0];
    grkit_export_ui_message(recipe_box(h),0,0);
    --services_word(h,SERVICES_APP)[0];
}
/* Execute of the delete confirm: delete the recipe and draw the page
   again. The cursor stays on the row, or on the last row. */
static void recipe_delete_done(u32 executed,u32 slot) {
    volatile u32 *h=menu_header();
    u32 controller=h[GRKIT_HOST_MENU_CONTROLLER/4];
    if(!executed) return;
    recipe_delete(h,slot);
    if(h[GRKIT_HOST_MENU_CURSOR/4]>=recipe_rows(h)) h[GRKIT_HOST_MENU_CURSOR/4]=recipe_rows(h)-1U;
    if(controller && h[GRKIT_HOST_MENU_LEVEL/4]) render(controller);
}

/* ---- The page --------------------------------------------------------------- */
static const u16 *recipe_title(volatile u32 *h) { return choice_title(h); }
/* The root row: the title, then the name of the recipe that the current
   settings match. */
static const u16 *recipe_summary(volatile u32 *h) {
    volatile u16 *to=recipe_text_at(h,RECIPE_SUMMARY_AT);
    u32 slot=recipe_current(h), at=recipe_append(to,0,choice_title(h));
    if(slot<GRKIT_RECIPE_SLOTS)
        recipe_append_name(h,to,recipe_append(to,at,text(h,RECIPE_TEXT_SEPARATOR)),recipe_at(h,slot));
    return (const u16 *)to;
}
static const u16 *recipe_label(volatile u32 *h,u32 row) {
    static const u16 open[3]={' ','(',0}, close[2]={')',0};
    u32 slot=recipe_row_slot(h,row), at;
    volatile u16 *to;
    if(slot==RECIPE_ROW_SAVE) return text(h,RECIPE_TEXT_SAVE);
    to=recipe_text_at(h,row*RECIPE_TEXT_SIZE);
    if(slot==RECIPE_ROW_UNDO) {
        at=recipe_append(to,0,text(h,RECIPE_TEXT_UNDO));
        at=recipe_append_number(to,recipe_append(to,at,open),recipe_undo_count(h));
        recipe_append(to,at,close);
    }
    else recipe_append_name(h,to,0,recipe_at(h,slot));
    return (const u16 *)to;
}
static u32 recipe_icon(volatile u32 *h,u32 row) {
    u32 slot, w, index;
    if(h[GRKIT_HOST_MENU_LEVEL/4]==MENU_ROOT) return choice_tile(h,RECIPE_TILE_MODULE);
    slot=recipe_row_slot(h,row);
    if(slot==RECIPE_ROW_UNDO) return choice_tile(h,RECIPE_TILE_UNDO);
    if(slot==RECIPE_ROW_SAVE) return choice_tile(h,RECIPE_TILE_SAVE);
    w=recipe_at(h,slot)[0];
    index=RECIPE_LOOK(w)==RECIPE_LOOK_EFFECT ? recipe_effect(h,RECIPE_CODE(w)) : GRKIT_TEXT_RECORDS;
    return index<GRKIT_TEXT_RECORDS ? GRKIT_ICON_EFFECT(index) : choice_tile(h,RECIPE_TILE_RECIPE);
}
static u32 recipe_checked(volatile u32 *h,u32 row) {
    u32 slot=recipe_row_slot(h,row);
    return slot<GRKIT_RECIPE_SLOTS && recipe_matches(h,slot);
}
/* The page opens on the recipe that the current settings match, else on
   the first recipe, or on Save without one. */
static u32 recipe_committed(volatile u32 *h) {
    u32 slot=recipe_current(h);
    return recipe_first_row(h)+(slot<GRKIT_RECIPE_SLOTS ? slot : 0U);
}
static void recipe_ok(volatile u32 *h,u32 row) {
    u32 slot=recipe_row_slot(h,row), saved;
    volatile u16 *to;
    if(slot==RECIPE_ROW_UNDO) { recipe_undo(h); return; }
    if(slot==RECIPE_ROW_SAVE) {
        saved=recipe_save(h);
        if(saved==RECIPE_FULL) { recipe_message(h,RECIPE_TEXT_FULL); return; }
        h[GRKIT_HOST_MENU_CURSOR/4]=recipe_first_row(h)+saved;
        to=recipe_text_at(h,RECIPE_BOX_AT);
        recipe_append_name(h,to,recipe_append(to,0,text(h,RECIPE_TEXT_SAVED)),recipe_at(h,saved));
        recipe_toast(h);
        return;
    }
    switch(recipe_apply(h,slot)) {
    case RECIPE_MISSING: recipe_message(h,RECIPE_TEXT_MISSING); break;
    case RECIPE_UNSUPPORTED: recipe_message(h,RECIPE_TEXT_UNSUPPORTED); break;
    /* Applied: the message keeps ADJ open, so it shows. */
    case RECIPE_GRAIN_SKIPPED: recipe_message(h,RECIPE_TEXT_NO_GRAIN); break;
    default: break;
    }
}
/* Fn on a recipe asks to delete it: 1 when the page took the key. */
static u32 recipe_fn(volatile u32 *h,u32 row) {
    u32 slot=recipe_row_slot(h,row), at;
    volatile u16 *to=recipe_text_at(h,RECIPE_BOX_AT);
    if(slot>=GRKIT_RECIPE_SLOTS) return 0;
    at=recipe_append(to,0,text(h,RECIPE_TEXT_DELETE));
    at=recipe_append_name(h,to,at,recipe_at(h,slot));
    recipe_append(to,at,text(h,RECIPE_TEXT_DELETE_AFTER));
    ++services_word(h,SERVICES_APP)[0];
    grkit_export_ui_confirm(recipe_box(h),recipe_delete_done,slot);
    --services_word(h,SERVICES_APP)[0];
    return 1;
}
static const MenuPage recipe_module={recipe_title,recipe_summary,recipe_rows,recipe_label,recipe_icon,
    recipe_checked,recipe_ok,recipe_committed,0,0,0,recipe_fn,MENU_PAGE_ACTIONS};
#endif
