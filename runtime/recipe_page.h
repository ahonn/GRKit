#ifndef GRKIT_RECIPE_PAGE_H
#define GRKIT_RECIPE_PAGE_H
/* The rows of a module page of CORE.RECIPE (runtime/recipes.h), for the
   menu. The rows are Undo while an undo step exists, each library recipe in
   site order, each camera recipe, then Save Current Settings. The rows are
   actions, so an ADJ exit applies none of them (MENU_PAGE_ACTIONS).

   - OK on a recipe applies it and closes ADJ, as OK on a native value does.
     A recipe that cannot apply shows a message with the reason instead. A
     recipe whose Grain has no effect (RECIPE_GRAIN_SKIPPED) or a library
     recipe with exposure hints applies and shows a message, which keeps
     ADJ open.
   - OK on Undo restores the newest undo step and closes ADJ.
   - OK on Save saves the current settings as a new recipe with the
     automatic name, then opens the keyboard of the camera with that name
     (runtime/ui.h). OK there names the recipe; MENU keeps the automatic
     name. Back on ADJ, a toast shows the name, and ADJ stays on the new
     row. Where the keyboard cannot open, the toast shows at once. With all
     recipes taken, Save shows a message.
   - Fn on a camera recipe shows a box with Rename and Delete, and MENU
     cancels it. Rename opens the keyboard with the name; Delete asks to
     delete the recipe. The site owns the library.
   - A camera recipe shows the name that the person typed, or the automatic
     name: the name of its effect or native type, and a number.

   The tiles of the module: 0 the module, 1 Undo, 2 Save, 3 a recipe
   without an effect icon or a native type icon. The includer is runtime/modules.h: it defines the
   menu text, the choice functions, and the services; runtime/menu.c
   defines render. The UI services follow in runtime/ui.h. */
#include "native_look.h"
/* The library of the services block (runtime/services.h). */
#define RECIPE_LIBRARY(h) services_word(h,SERVICES_LIBRARY)
#include "recipes.h"

u32 grkit_export_ui_toast(const u16 *const *text,u32 ms);
u32 grkit_export_ui_message(const u16 *const *text,void (*done)(u32),u32 arg);
u32 grkit_export_ui_confirm(const u16 *const *text,void (*done)(u32,u32),u32 arg);
static u32 ui_choice(volatile u32 *h,const u16 *const *text,const u16 *first,const u16 *second,
                     void (*done)(u32,u32),u32 arg);
static u32 ui_keyboard_open(volatile u32 *h,const volatile u16 *text,void (*back)(u32),u32 arg);
static const volatile u8 *ui_keyboard_result(volatile u32 *h);

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
#define RECIPE_TEXT_OTHER_CAMERA (TEXT_RECIPE+11U)
/* The hint texts: the ISO limit, the four meterings, ND Off, On, Auto. */
#define RECIPE_TEXT_ISO_LIMIT (TEXT_RECIPE+12U)
#define RECIPE_TEXT_METERING (TEXT_RECIPE+13U)
#define RECIPE_TEXT_ND (TEXT_RECIPE+17U)
/* The buttons of Fn on a camera recipe. */
#define RECIPE_TEXT_RENAME (TEXT_RECIPE+20U)
#define RECIPE_TEXT_REMOVE (TEXT_RECIPE+21U)
/* The row kinds besides a recipe slot. */
#define RECIPE_ROW_UNDO 0xfffffffeU
#define RECIPE_ROW_SAVE 0xfffffffdU
/* The units of the effect name of a record, with its NUL. */
#define RECIPE_NAME_UNITS 17U
_Static_assert(RECIPE_ROWS_MAX>=GRKIT_RECIPE_SLOTS+1U,"a text buffer for Undo and each camera recipe");

/* The row of entry 0: after Undo, if it shows. */
static u32 recipe_first_row(volatile u32 *h) { return recipe_undo_count(h) ? 1U : 0U; }
static u32 recipe_rows(volatile u32 *h) { return recipe_first_row(h)+recipe_entries(h)+1U; }
/* The entry of a row (runtime/recipes.h), RECIPE_ROW_UNDO, or
   RECIPE_ROW_SAVE. */
static u32 recipe_row_slot(volatile u32 *h,u32 row) {
    u32 first=recipe_first_row(h);
    if(row<first) return RECIPE_ROW_UNDO;
    return row-first<recipe_entries(h) ? row-first : RECIPE_ROW_SAVE;
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
/* One string of the label table, in the menu language or in English
   (menu language 0). */
static const u16 *recipe_string(volatile u32 *h,u32 id,u32 english) {
    u32 base=GRKIT_TEXT(h);
    return english ? MENU_TEXT(base+MENU_TEXT(base)[id]) : text(h,id);
}
/* Append the automatic name of camera recipe entry e: the name of its
   effect, of its native type, Film Look Off, or Recipe, then its number. A
   camera recipe whose effect is not installed shows that instead. */
static u32 recipe_append_automatic(volatile u32 *h,volatile u16 *to,u32 at,u32 e,u32 english) {
    static const u16 space[2]={' ',0};
    const volatile u32 *w=recipe_words(h,e);
    u32 index, type=NATIVE_TYPE(w[RECIPE_NATIVE_WORD]);
    if(RECIPE_LOOK(w[0])==RECIPE_LOOK_EFFECT) {
        index=recipe_effect(h,RECIPE_CODE(w[0]));
        if(index>=GRKIT_TEXT_RECORDS) return recipe_append(to,at,recipe_string(h,RECIPE_TEXT_MISSING,english));
        at=recipe_append_effect(to,at,recipe_record(h,index));
    }
    else if(type && type<=NATIVE_TYPES) at=recipe_append(to,at,recipe_string(h,TEXT_NATIVE_TYPE+type-1U,english));
    else at=recipe_append(to,at,recipe_string(h,RECIPE_LOOK(w[0])==RECIPE_LOOK_OFF ? TEXT_IMAGE_OFF_SUMMARY : RECIPE_TEXT_NAME,english));
    if(RECIPE_NUMBER(w[0])) at=recipe_append_number(to,recipe_append(to,at,space),RECIPE_NUMBER(w[0]));
    return at;
}
/* Append the name of entry e: the name of a library recipe; for a camera
   recipe, the name that the person typed, or the automatic name. */
static u32 recipe_append_name(volatile u32 *h,volatile u16 *to,u32 at,u32 e) {
    u32 slot, n, i;
    if(recipe_in_library(h,e)) return recipe_append(to,at,(const u16 *)(recipe_library_entry(h,e)+LIBRARY_NAME));
    slot=e-recipe_library_count(h);
    n=recipe_name_length(h,slot);
    if(!n) return recipe_append_automatic(h,to,at,e,0);
    for(i=0;i<n && at+1U<RECIPE_UNITS;++i) to[at++]=(u16)recipe_name_char(h,slot,i);
    to[at]=0;
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
/* The page shows camera recipe `slot` under the cursor. */
static void recipe_show(volatile u32 *h,u32 slot) {
    u32 controller=h[GRKIT_HOST_MENU_CONTROLLER/4];
    if(slot<recipe_count(h)) h[GRKIT_HOST_MENU_CURSOR/4]=recipe_first_row(h)+recipe_library_count(h)+slot;
    if(controller && h[GRKIT_HOST_MENU_LEVEL/4]) render(controller);
}
/* The toast after a save, with the name of camera recipe `slot`. */
static void recipe_saved_toast(volatile u32 *h,u32 slot) {
    volatile u16 *to=recipe_text_at(h,RECIPE_BOX_AT);
    recipe_append_name(h,to,recipe_append(to,0,text(h,RECIPE_TEXT_SAVED)),recipe_library_count(h)+slot);
    recipe_toast(h);
}
/* Open the keyboard of the camera with the name of camera recipe `slot`:
   1, or 0 when it cannot open. The keyboard takes printable ASCII, so an
   automatic name with other characters starts from its English name. Then
   back(slot) runs on the page. */
static u32 recipe_keyboard(volatile u32 *h,u32 slot,void (*back)(u32)) {
    volatile u16 *to=recipe_text_at(h,RECIPE_BOX_AT);
    u32 e=recipe_library_count(h)+slot, i, opened;
    recipe_append_name(h,to,0,e);
    for(i=0;to[i];++i) if(to[i]<0x20U || to[i]>0x7eU) { recipe_append_automatic(h,to,0,e,1); break; }
    ++services_word(h,SERVICES_APP)[0];
    opened=ui_keyboard_open(h,to,back,slot);
    --services_word(h,SERVICES_APP)[0];
    return opened;
}
/* The return from the keyboard after Save: the typed name, if any, then
   the toast. */
static void recipe_saved_back(u32 slot) {
    volatile u32 *h=menu_header();
    const volatile u8 *name=ui_keyboard_result(h);
    if(slot>=recipe_count(h)) return;
    if(name) recipe_name_set(h,slot,name);
    recipe_show(h,slot);
    recipe_saved_toast(h,slot);
}
/* The return from the keyboard after Rename. MENU keeps the name. */
static void recipe_renamed_back(u32 slot) {
    volatile u32 *h=menu_header();
    const volatile u8 *name=ui_keyboard_result(h);
    if(slot>=recipe_count(h)) return;
    if(name) recipe_name_set(h,slot,name);
    recipe_show(h,slot);
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
    u32 e=recipe_current(h), at=recipe_append(to,0,choice_title(h));
    if(e!=RECIPE_NONE) recipe_append_name(h,to,recipe_append(to,at,text(h,RECIPE_TEXT_SEPARATOR)),e);
    return (const u16 *)to;
}
/* A library row shows the name of its entry. Undo and each camera recipe
   have a text buffer: Undo the first, camera recipe n buffer n + 1. */
static const u16 *recipe_label(volatile u32 *h,u32 row) {
    static const u16 open[3]={' ','(',0}, close[2]={')',0};
    u32 e=recipe_row_slot(h,row), at;
    volatile u16 *to;
    if(e==RECIPE_ROW_SAVE) return text(h,RECIPE_TEXT_SAVE);
    if(e==RECIPE_ROW_UNDO) {
        to=recipe_text_at(h,0);
        at=recipe_append(to,0,text(h,RECIPE_TEXT_UNDO));
        at=recipe_append_number(to,recipe_append(to,at,open),recipe_undo_count(h));
        recipe_append(to,at,close);
        return (const u16 *)to;
    }
    if(recipe_in_library(h,e)) return (const u16 *)(recipe_library_entry(h,e)+LIBRARY_NAME);
    to=recipe_text_at(h,(e-recipe_library_count(h)+1U)*RECIPE_TEXT_SIZE);
    recipe_append_name(h,to,0,e);
    return (const u16 *)to;
}
/* A recipe shows the icon of its effect, else the native Image Control icon
   of its native type, as recipe_append_name names it. A type without an
   icon on this camera, Film Look Off, and an effect that is not installed
   show the recipe tile. */
static u32 recipe_icon(volatile u32 *h,u32 row) {
    static const u16 type_icons[NATIVE_TYPES]={NATIVE_TYPE_ICONS};
    const volatile u32 *w;
    u32 slot, index, type;
    if(h[GRKIT_HOST_MENU_LEVEL/4]==MENU_ROOT) return choice_tile(h,RECIPE_TILE_MODULE);
    slot=recipe_row_slot(h,row);
    if(slot==RECIPE_ROW_UNDO) return choice_tile(h,RECIPE_TILE_UNDO);
    if(slot==RECIPE_ROW_SAVE) return choice_tile(h,RECIPE_TILE_SAVE);
    w=recipe_words(h,slot);
    if(RECIPE_LOOK(w[0])==RECIPE_LOOK_EFFECT) {
        index=recipe_effect(h,RECIPE_CODE(w[0]));
        return index<GRKIT_TEXT_RECORDS ? GRKIT_ICON_EFFECT(index) : choice_tile(h,RECIPE_TILE_RECIPE);
    }
    type=NATIVE_TYPE(w[RECIPE_NATIVE_WORD]);
    return type && type<=NATIVE_TYPES && type_icons[type-1U] ? type_icons[type-1U] : choice_tile(h,RECIPE_TILE_RECIPE);
}
static u32 recipe_checked(volatile u32 *h,u32 row) {
    u32 e=recipe_row_slot(h,row);
    return e<recipe_entries(h) && recipe_matches(h,e);
}
/* The page opens on the recipe that the current settings match, else on
   the first recipe, or on Save without one. */
static u32 recipe_committed(volatile u32 *h) {
    u32 e=recipe_current(h);
    return recipe_first_row(h)+(e!=RECIPE_NONE ? e : 0U);
}

/* ---- Hints ---------------------------------------------------------------- */
/* Append a number of up to 6 digits. */
static u32 recipe_append_decimal(volatile u16 *to,u32 at,u32 n) {
    u16 digits[7];
    u32 i=6;
    digits[6]=0;
    do { u32 rest=native_div10(n); digits[--i]=(u16)('0'+n-10U*rest); n=rest; } while(n && i);
    return recipe_append(to,at,digits+i);
}
/* The exposure hints of a library recipe (grkit_core::recipes::Hints) show
   in two messages, because a box shows at most GRKIT_SUMMARY_UNITS - 1
   units: the compensation and the ISO limit, then the metering and the ND
   filter. Each one that the recipe does not give is left out. The argument
   of a message holds the hints and, from bit 29, the part to show next. */
#define RECIPE_HINTS_PART 29U
#define RECIPE_HINTS_MASK ((1U<<RECIPE_HINTS_PART)-1U)
static u32 recipe_append_hints(volatile u32 *h,volatile u16 *to,u32 hints,u32 part) {
    static const u16 ev[4]={'E','V',' ',0}, gap[3]={' ',' ',0}, thirds[3]={'/','3',0}, space[2]={' ',0};
    u32 exposure=hints&0x3fU, iso=hints>>6&0x3ffffU, metering=hints>>24&7U, nd=hints>>27&3U, at=0;
    to[0]=0;
    if(part==1U) {
        if(exposure) {
            /* n / 3 is n * 43 >> 7 for n below 128, without a division helper. */
            u32 value=exposure>=16U ? exposure-16U : 16U-exposure, whole=value*43U>>7;
            u16 sign[2]={exposure>=16U ? (u16)'+' : (u16)'-',0};
            value-=3U*whole;
            at=recipe_append(to,at,ev);
            if(exposure!=16U) at=recipe_append(to,at,sign);
            if(whole || !value) at=recipe_append_decimal(to,at,whole);
            if(whole && value) at=recipe_append(to,at,space);
            if(value) at=recipe_append(to,recipe_append_decimal(to,at,value),thirds);
        }
        if(iso) {
            if(at) at=recipe_append(to,at,gap);
            at=recipe_append_decimal(to,recipe_append(to,at,text(h,RECIPE_TEXT_ISO_LIMIT)),iso);
        }
    }
    else {
        if(metering && metering<=4U) at=recipe_append(to,at,text(h,RECIPE_TEXT_METERING+metering-1U));
        if(nd) {
            if(at) at=recipe_append(to,at,gap);
            at=recipe_append(to,at,text(h,RECIPE_TEXT_ND+nd-1U));
        }
    }
    return at;
}
/* Show the next message of the hints from part `arg >> RECIPE_HINTS_PART`.
   OK on it shows the part after it. */
static void recipe_hints_next(u32 arg) {
    volatile u32 *h=menu_header();
    u32 hints=arg&RECIPE_HINTS_MASK, part;
    volatile u16 *to=recipe_text_at(h,RECIPE_BOX_AT);
    for(part=arg>>RECIPE_HINTS_PART;part<=2U;++part) {
        if(!recipe_append_hints(h,to,hints,part)) continue;
        ++services_word(h,SERVICES_APP)[0];
        grkit_export_ui_message(recipe_box(h),part<2U ? recipe_hints_next : 0,hints|(part+1U)<<RECIPE_HINTS_PART);
        --services_word(h,SERVICES_APP)[0];
        return;
    }
}

static void recipe_ok(volatile u32 *h,u32 row) {
    u32 slot=recipe_row_slot(h,row), saved, result, hints;
    if(slot==RECIPE_ROW_UNDO) { recipe_undo(h); return; }
    if(slot==RECIPE_ROW_SAVE) {
        saved=recipe_save(h);
        if(saved==RECIPE_FULL) { recipe_message(h,RECIPE_TEXT_FULL); return; }
        h[GRKIT_HOST_MENU_CURSOR/4]=recipe_first_row(h)+recipe_library_count(h)+saved;
        if(!recipe_keyboard(h,saved,recipe_saved_back)) recipe_saved_toast(h,saved);
        return;
    }
    hints=recipe_hints(h,slot);
    saved=recipe_first_row(h);
    result=recipe_apply(h,slot);
    switch(result) {
    case RECIPE_MISSING: recipe_message(h,RECIPE_TEXT_MISSING); return;
    case RECIPE_UNSUPPORTED: recipe_message(h,RECIPE_TEXT_UNSUPPORTED); return;
    case RECIPE_OTHER_CAMERA: recipe_message(h,RECIPE_TEXT_OTHER_CAMERA); return;
    default: break;
    }
    /* Applied. The first undo step adds the Undo row, so the cursor moves
       with the recipe. A message keeps ADJ open, so it shows: the Grain that
       has no effect, then the exposure hints. */
    h[GRKIT_HOST_MENU_CURSOR/4]+=recipe_first_row(h)-saved;
    if(result==RECIPE_GRAIN_SKIPPED) {
        recipe_append(recipe_text_at(h,RECIPE_BOX_AT),0,text(h,RECIPE_TEXT_NO_GRAIN));
        ++services_word(h,SERVICES_APP)[0];
        grkit_export_ui_message(recipe_box(h),hints ? recipe_hints_next : 0,hints|1U<<RECIPE_HINTS_PART);
        --services_word(h,SERVICES_APP)[0];
    }
    else if(hints) recipe_hints_next(hints|1U<<RECIPE_HINTS_PART);
}
/* Delete camera recipe `slot` after a confirm, the cursor on Cancel. */
static void recipe_delete_confirm(volatile u32 *h,u32 slot) {
    volatile u16 *to=recipe_text_at(h,RECIPE_BOX_AT);
    u32 at=recipe_append(to,0,text(h,RECIPE_TEXT_DELETE));
    at=recipe_append_name(h,to,at,recipe_library_count(h)+slot);
    recipe_append(to,at,text(h,RECIPE_TEXT_DELETE_AFTER));
    ++services_word(h,SERVICES_APP)[0];
    grkit_export_ui_confirm(recipe_box(h),recipe_delete_done,slot);
    --services_word(h,SERVICES_APP)[0];
}
/* A button of the box of Fn: 0 Rename, 1 Delete, 2 MENU. */
static void recipe_action_done(u32 choice,u32 slot) {
    volatile u32 *h=menu_header();
    if(slot>=recipe_count(h)) return;
    if(choice==0U) recipe_keyboard(h,slot,recipe_renamed_back);
    else if(choice==1U) recipe_delete_confirm(h,slot);
}
/* Fn on a camera recipe shows its name with Rename and Delete: 1 when the
   page took the key. Fn on a library recipe does nothing. */
static u32 recipe_fn(volatile u32 *h,u32 row) {
    u32 e=recipe_row_slot(h,row), library=recipe_library_count(h);
    if(e==RECIPE_ROW_UNDO || e==RECIPE_ROW_SAVE || e<library) return 0;
    recipe_append_name(h,recipe_text_at(h,RECIPE_BOX_AT),0,e);
    ++services_word(h,SERVICES_APP)[0];
    ui_choice(h,recipe_box(h),text(h,RECIPE_TEXT_RENAME),text(h,RECIPE_TEXT_REMOVE),recipe_action_done,e-library);
    --services_word(h,SERVICES_APP)[0];
    return 1;
}
static const MenuPage recipe_module={recipe_title,recipe_summary,recipe_rows,recipe_label,recipe_icon,
    recipe_checked,recipe_ok,recipe_committed,0,0,0,recipe_fn,MENU_PAGE_ACTIONS};
#endif
