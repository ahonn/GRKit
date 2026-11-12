#ifndef GRKIT_MENU_DIAL_H
#define GRKIT_MENU_DIAL_H
/* The Mode Dial row of GRKit Settings in MENU (runtime/menu_settings.h:
   CORE_PAGE): what the user modes U1 to U3 of the mode dial do
   (runtime/mode_dial.h). The row shows while the Recipes page is loaded.

   - The row opens a list of the user modes. Each shows its value: None (the
     user mode follows), GRKit Off, Saved Settings, or the name of its
     recipe. A recipe that the camera does not list, as a deleted recipe or
     one that is off, shows None: the user mode then follows
     (dial_assignment). So does Saved Settings without a set.
   - OK on a user mode shows its values in place of the list, with the user
     mode as the title: Save Settings, None, GRKit Off, Saved Settings while
     the user mode has a set, then each recipe that is on, with the mark and
     the cursor on its value. OK on a value sets it and returns to the list,
     with the cursor on the user mode. MENU returns to the list and changes
     nothing.
   - OK on Save Settings saves the current GRKit settings in the set of the
     user mode, in place of the set that it had (dial_save), and gives the
     user mode Saved Settings. Another value keeps the set, so Saved Settings
     can come back.
   - The recipe file keeps the values (GRKIT_RECIPES_CHANGED), and the file
     of the sets the sets. While the dial is on that user mode, a new value
     or a new set applies at once (dial_assigned).
   - A half-press sets nothing and saves nothing: the native confirm of the
     page sets a value with Setting::set, which does nothing here.

   As the Recipes row, the list and the values of a user mode are one own
   value page, which shows the values while MENU_ROWS_PAGE_STATE holds the
   user mode. The includer is runtime/menu.c, after runtime/mode_dial.h and
   runtime/menu_settings.h. */
/* The texts of the page (runtime/abi.h: the DIAL run). GRKit Off and Saved
   Settings are DIAL_TEXT_OFF and DIAL_TEXT_SAVED, which the label of the dial
   shows too (runtime/mode_dial.h). Save Settings is the row of the Recipes
   page. */
#define DIAL_TEXT_ROW (TEXT_DIAL+0U)
#define DIAL_TEXT_NONE (TEXT_DIAL+1U)
/* MENU_ROWS_PAGE_STATE: DIAL_PAGE_SHOWN with the user mode (0 to 2) whose
   values show, or the user mode that the list puts its cursor on + 1, or 0
   for the cursor on U1. */
#define DIAL_PAGE_SHOWN 0x10000U
/* The values of a user mode: Save Settings, None, GRKit Off, Saved Settings
   while the user mode has a set, then the recipes from dial_first_recipe.
   The list has no mark: no value is DIAL_NO_MARK. */
#define DIAL_VALUE_SAVE 0U
#define DIAL_VALUE_NONE 1U
#define DIAL_VALUE_OFF 2U
#define DIAL_VALUE_SAVED 3U
#define DIAL_NO_MARK 0xffffffffU
_Static_assert(DIAL_VALUE_SAVED+1U+GRKIT_RECIPE_ON_MAX<=MENU_VALUES_MAX,"The page lists each recipe that is on");
static volatile u32 *dial_page_state(volatile u32 *h) { return services_word(h,SERVICES_MENU+MENU_ROWS_PAGE_STATE); }
/* The user mode (0 to 2) whose values show, or GRKIT_RECIPE_DIAL_POSITIONS
   while the list shows. */
static u32 dial_page_shown(volatile u32 *h) {
    u32 state=dial_page_state(h)[0];
    return state&DIAL_PAGE_SHOWN ? state&0xffffU : GRKIT_RECIPE_DIAL_POSITIONS;
}
/* The value of recipe 0 for user mode u (0 to 2): after Saved Settings
   while the user mode has a set. */
static u32 dial_first_recipe(volatile u32 *h,u32 u) { return DIAL_VALUE_SAVED+dial_has_set(h,u+1U); }
/* The value of user mode u (0 to 2), as the dial applies it. */
static u32 dial_value(volatile u32 *h,u32 u) {
    u32 entry=0, kind=dial_assignment(h,u+1U,&entry);
    if(kind==DIAL_RECIPE) return dial_first_recipe(h,u)+entry;
    return kind==DIAL_SAVED ? DIAL_VALUE_SAVED : kind==DIAL_OFF ? DIAL_VALUE_OFF : DIAL_VALUE_NONE;
}
static u32 dial_values(volatile u32 *h,u32 u) { return dial_first_recipe(h,u)+recipe_entries(h); }
/* The text of value `value` of user mode u besides a recipe. */
static const u16 *dial_value_text(volatile u32 *h,u32 value) {
    static const u32 texts[DIAL_VALUE_SAVED+1U]={RECIPE_TEXT_SAVE,DIAL_TEXT_NONE,DIAL_TEXT_OFF,DIAL_TEXT_SAVED};
    return text(h,texts[value<=DIAL_VALUE_SAVED ? value : DIAL_VALUE_NONE]);
}
/* Append the text of value `value` of user mode u. A recipe has its name. */
static u32 dial_append_value(volatile u32 *h,u32 u,volatile u16 *to,u32 at,u32 value) {
    u32 first=dial_first_recipe(h,u);
    if(value>=first) return recipe_append_name(h,to,at,value-first);
    return recipe_append(to,at,dial_value_text(h,value));
}
/* Set what user mode u (0 to 2) does to value `value`. A recipe that has no
   ID yet, which INSTALL gives at the next start, gets one now. Save Settings
   saves the set and gives Saved Settings; a new set applies at once also
   when the user mode had Saved Settings. */
static void dial_assign(volatile u32 *h,u32 u,u32 value) {
    volatile u32 *at=h+GRKIT_HOST_RECIPE_DIAL/4+u, *id;
    u32 first=dial_first_recipe(h,u), next=GRKIT_RECIPE_DIAL_FOLLOW;
    if(value>=first) {
        id=recipe_id_at(h,value-first);
        if(!*id) *id=recipe_new_id(h);
        next=*id;
    }
    else if(value==DIAL_VALUE_OFF) next=GRKIT_RECIPE_DIAL_OFF;
    else if(value==DIAL_VALUE_SAVED) next=GRKIT_RECIPE_DIAL_SAVED;
    else if(value==DIAL_VALUE_SAVE) {
        dial_save(h,u+1U);
        next=GRKIT_RECIPE_DIAL_SAVED;
    }
    if(*at!=next) {
        *at=next;
        GRKIT_RECIPES_CHANGED(h);
    }
    else if(value!=DIAL_VALUE_SAVE) return;
    dial_assigned(h,u+1U);
}

/* ---- The row ---------------------------------------------------------------- */
static const u16 *dial_row_label(volatile u32 *h) { return text(h,DIAL_TEXT_ROW); }
static u32 dial_row_values(volatile u32 *h) {
    u32 u=dial_page_shown(h);
    return u<GRKIT_RECIPE_DIAL_POSITIONS ? dial_values(h,u) : GRKIT_RECIPE_DIAL_POSITIONS;
}
/* The list shows each user mode with its value, such as "U1: Street", in
   the text buffer of a box, which this page does not show: the page copies
   the text of each value as it draws it (entry_text). */
static const u16 *dial_row_text(volatile u32 *h,u32 value) {
    u32 u=dial_page_shown(h), at;
    volatile u16 *to;
    if(u<GRKIT_RECIPE_DIAL_POSITIONS) {
        if(value>=dial_values(h,u)) return no_value_text;
        if(value<dial_first_recipe(h,u)) return dial_value_text(h,value);
        to=recipe_text_at(h,(value-dial_first_recipe(h,u)+1U)*RECIPE_TEXT_SIZE);
        dial_append_value(h,u,to,0,value);
        return (const u16 *)to;
    }
    if(value>=GRKIT_RECIPE_DIAL_POSITIONS) return no_value_text;
    to=recipe_text_at(h,RECIPE_BOX_AT);
    at=recipe_append(to,recipe_append(to,0,dial_names[value]),text(h,TEXT_SEPARATOR));
    dial_append_value(h,value,to,at,dial_value(h,value));
    return (const u16 *)to;
}
/* The mark is on the value of the user mode; the list has none. */
static u32 dial_row_get(volatile u32 *h) {
    u32 u=dial_page_shown(h);
    return u<GRKIT_RECIPE_DIAL_POSITIONS ? dial_value(h,u) : DIAL_NO_MARK;
}
static u32 dial_row_cursor(volatile u32 *h) {
    u32 state=dial_page_state(h)[0];
    if(state&DIAL_PAGE_SHOWN) return dial_row_get(h);
    return state && state-1U<GRKIT_RECIPE_DIAL_POSITIONS ? state-1U : 0;
}
/* The values of a user mode have its name as the title; the list has the
   row label. */
static const u16 *dial_row_title(volatile u32 *h) {
    u32 u=dial_page_shown(h);
    return u<GRKIT_RECIPE_DIAL_POSITIONS ? dial_names[u] : 0;
}
/* OK on a user mode shows its values. OK on a value sets it, OK on Save
   Settings saves, and MENU there sets nothing; each returns to the list.
   MENU on the list returns to the Recipes page as MENU does. */
static u32 dial_row_select(volatile u32 *h,u32 controller,u32 value) {
    u32 u=dial_page_shown(h);
    (void)controller;
    if(u>=GRKIT_RECIPE_DIAL_POSITIONS) {
        if(value>=GRKIT_RECIPE_DIAL_POSITIONS) return 0;
        dial_page_state(h)[0]=DIAL_PAGE_SHOWN|value;
        settings_value_again(h,0);
        return 1;
    }
    if(value<dial_values(h,u)) dial_assign(h,u,value);
    dial_page_state(h)[0]=u+1U;
    settings_value_again(h,0);
    return 1;
}
static const Setting dial_menu_settings[1]={
    {dial_row_label,dial_row_values,dial_row_text,dial_row_get,no_set,dial_row_select,SETTING_OWN_PAGE,
     dial_row_cursor,dial_row_title},
};
static const Setting *dial_menu_setting(void) { return dial_menu_settings; }
#endif /* GRKIT_MENU_DIAL_H */
