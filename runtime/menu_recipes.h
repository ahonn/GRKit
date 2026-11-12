#ifndef GRKIT_MENU_RECIPES_H
#define GRKIT_MENU_RECIPES_H
/* The page of the Recipes module in the GRKit section of MENU
   (runtime/menu_listed.h). The person chooses a recipe first, then what to
   do with it.

   - Enable, then Options, which chooses the recipes that ADJ shows, as the
     Options of other modules choose their values (runtime/menu_listed.h).
   - Recipes shows the recipe that the current settings match. Its value
     page lists each recipe that is on (runtime/recipes.h), also those that
     ADJ does not show, with the mark on that recipe and the cursor on it,
     else on the first recipe. The camera keeps the recipes that are off for
     the site and does not list them.
   - OK on a recipe shows the page of the recipe in place of the list, with
     the name of the recipe as its title: Apply, Rename, and Delete. MENU
     there returns to the list, with the cursor on the recipe. A list that
     has the mark starts with the cursor on the mark (recipes_row_cursor).
     - Apply applies the recipe and returns to the Recipes page. MENU shows
       no message after it: a recipe that cannot apply changes nothing.
     - Rename opens the keyboard of the camera with the name, and the
       keyboard returns to the page of the recipe.
     - Delete asks with the confirm box of the core, Execute or Cancel, the
       cursor on Cancel. Execute deletes the recipe and returns to the list
       with the cursor on the recipe after it, or on the last recipe, unless
       the list has the mark. After the last recipe, it returns to the
       Recipes page.
   - Mode Dial sets what the user modes of the mode dial do
     (runtime/menu_dial.h).
   - Save Settings adds a recipe that is on, then opens the keyboard with
     its name. Its row shows the number of recipes that are on and the most
     of them, and while no recipe can be on, OK does nothing.
   - Without a recipe, the page has Enable, Mode Dial, and Save Settings.
     The undo steps show only in ADJ.

   The list and the page of a recipe are one own value page of the Recipes
   row, which shows the page of a recipe while MENU_ROWS_PAGE_STATE holds
   it. So MENU, OK, the boxes, and a half-press work as on any own value
   page (core.h: Setting, runtime/menu_settings.h, runtime/ui.h,
   runtime/ui_keyboard.h). The includer is runtime/menu.c, after
   runtime/menu_settings.h and runtime/menu_dial.h. */
/* The texts of the page (runtime/abi.h: the RECIPE run). */
#define RECIPE_TEXT_APPLY (TEXT_RECIPE+13U)

/* MENU_ROWS_PAGE_STATE: RECIPE_PAGE_SHOWN with the recipe whose page shows,
   or the recipe of the last action + 1, where the list puts its cursor
   without a recipe that the settings match, or 0. */
#define RECIPE_PAGE_SHOWN 0x10000U
#define RECIPE_PAGE_ACTIONS 3U
static volatile u32 *recipe_page_state(volatile u32 *h) { return services_word(h,SERVICES_MENU+MENU_ROWS_PAGE_STATE); }
/* The recipe whose page shows, or RECIPE_NONE while the list shows. */
static u32 recipe_page_shown(volatile u32 *h) {
    u32 state=recipe_page_state(h)[0];
    return state&RECIPE_PAGE_SHOWN ? state&0xffffU : RECIPE_NONE;
}

/* Whether the listed module has the Recipes page. menu_page selects it. */
static u32 recipe_listed(volatile u32 *h) {
    return listed_slot(h)<module_pages(h) && menu_page(h,listed_slot(h))==&recipe_module;
}
/* The name of recipe e in the text buffer of its ADJ row: ADJ does not show
   while MENU does. A value row of MENU has no room for the missing mark
   (entry_text). */
static const u16 *menu_recipe_name(volatile u32 *h,u32 e) {
    volatile u16 *to=recipe_text_at(h,(e+1U)*RECIPE_TEXT_SIZE);
    recipe_append_name(h,to,0,e);
    return (const u16 *)to;
}
static const u16 *recipe_list_text(volatile u32 *h,u32 value) {
    return value<recipe_entries(h) ? menu_recipe_name(h,value) : no_value_text;
}

/* ---- The page of a recipe --------------------------------------------------- */
/* Show the list again, with the cursor on recipe e, or return to the
   Recipes page without a recipe. */
static void recipe_list_again(volatile u32 *h,u32 e) {
    u32 n=recipe_entries(h);
    recipe_page_state(h)[0]=n ? (e<n ? e : n-1U)+1U : 0;
    settings_value_again(h,!n);
}
/* The return from the keyboard that renames recipe e: the text of an OK,
   then the page of the recipe with its new name as the title. */
static void recipe_menu_renamed(u32 e) {
    volatile u32 *h=menu_header();
    recipe_keyboard_name(h,e);
    if(e<recipe_entries(h)) menu_recipe_name(h,e);
}
/* Execute or Cancel of the box of Delete on recipe e. */
static void recipe_menu_deleted(u32 executed,u32 e) {
    volatile u32 *h=menu_header();
    if(!executed || e>=recipe_entries(h)) return;
    recipe_delete(h,e);
    recipe_list_again(h,e);
}
/* An action on recipe e: 0 Apply, 1 Rename, 2 Delete. */
static void recipe_action(volatile u32 *h,u32 controller,u32 e,u32 action) {
    volatile u16 *to;
    u32 at;
    if(e>=recipe_entries(h)) return;
    if(action==0U) {
        recipe_apply(h,e);
        recipe_page_state(h)[0]=0;
        settings_value_again(h,1);
    }
    else if(action==1U) ui_keyboard_on(h,controller,recipe_keyboard_text(h,e),recipe_menu_renamed,e);
    else {
        to=recipe_text_at(h,RECIPE_BOX_AT);
        at=recipe_append(to,0,text(h,RECIPE_TEXT_DELETE));
        at=recipe_append_name(h,to,at,e);
        recipe_append(to,at,text(h,RECIPE_TEXT_DELETE_AFTER));
        ui_confirm_on(h,controller,recipe_box(h),recipe_menu_deleted,e);
    }
}

/* ---- The rows --------------------------------------------------------------- */
/* Recipes: the list, or the actions on the recipe whose page shows. */
static const u16 *recipes_row_label(volatile u32 *h) { return recipe_title(h); }
static u32 recipes_row_values(volatile u32 *h) {
    return recipe_page_shown(h)!=RECIPE_NONE ? RECIPE_PAGE_ACTIONS : recipe_entries(h);
}
static const u16 *recipes_row_text(volatile u32 *h,u32 value) {
    static const u32 actions[RECIPE_PAGE_ACTIONS]={RECIPE_TEXT_APPLY,RECIPE_TEXT_RENAME,RECIPE_TEXT_REMOVE};
    if(recipe_page_shown(h)==RECIPE_NONE) return recipe_list_text(h,value);
    return value<RECIPE_PAGE_ACTIONS ? text(h,actions[value]) : no_value_text;
}
/* The mark is on the recipe that the settings match; the actions have
   none. */
static u32 recipes_row_get(volatile u32 *h) {
    return recipe_page_shown(h)!=RECIPE_NONE ? RECIPE_NONE : recipe_matching(h,0);
}
/* The camera shows the mark only on the row where the cursor starts
   (menu_check_dot), so the cursor starts on the recipe that the settings
   match. Without a match, it starts on the recipe of the last action. */
static u32 recipes_row_cursor(volatile u32 *h) {
    u32 state=recipe_page_state(h)[0], e;
    if(state&RECIPE_PAGE_SHOWN) return 0;
    e=recipe_matching(h,0);
    if(e!=RECIPE_NONE) return e;
    return state ? state-1U : 0;
}
/* The page of a recipe has the name of the recipe as its title, which its
   text buffer holds since OK on the recipe or the return from the
   keyboard. */
static const u16 *recipes_row_title(volatile u32 *h) {
    u32 e=recipe_page_shown(h);
    return e<GRKIT_RECIPE_ON_MAX ? (const u16 *)recipe_text_at(h,(e+1U)*RECIPE_TEXT_SIZE) : 0;
}
/* OK on a recipe shows its page; OK on an action does it. MENU on the page
   of a recipe returns to the list. */
static u32 recipes_row_select(volatile u32 *h,u32 controller,u32 value) {
    u32 e=recipe_page_shown(h);
    if(e==RECIPE_NONE) {
        if(value>=recipe_entries(h)) return 0;
        menu_recipe_name(h,value);
        recipe_page_state(h)[0]=RECIPE_PAGE_SHOWN|value;
        settings_value_again(h,0);
        return 1;
    }
    if(value==SETTING_BACK) recipe_list_again(h,e);
    else if(value<RECIPE_PAGE_ACTIONS) recipe_action(h,controller,e,value);
    return 1;
}
static const u16 *save_row_label(volatile u32 *h) { return text(h,RECIPE_TEXT_SAVE); }
/* The number of recipes that are on and the most of them, such as 12/40, in
   the text buffer of Undo, which shows only in ADJ. */
static const u16 *save_row_text(volatile u32 *h,u32 value) {
    static const u16 slash[2]={'/',0};
    volatile u16 *to=recipe_text_at(h,0);
    (void)value;
    recipe_append_number(to,recipe_append(to,recipe_append_number(to,0,recipe_entries(h)),slash),GRKIT_RECIPE_ON_MAX);
    return (const u16 *)to;
}
/* The return from the keyboard that names the new recipe e: the text of an
   OK. MENU keeps the automatic name. */
static void recipe_named(u32 e) { recipe_keyboard_name(menu_header(),e); }
/* OK on Save Settings: the keyboard with the name of the new recipe, which
   returns to the page, or 0 when the page stays: no recipe can be on
   (recipe_save), or another use holds the keyboard. */
static u32 save_row_select(volatile u32 *h,u32 controller,u32 value) {
    u32 e;
    (void)controller; (void)value;
    if(!recipe_listed(h)) return 0;
    e=recipe_save(h);
    if(e==RECIPE_FULL || !ui_keyboard_prepare(h,recipe_keyboard_text(h,e),recipe_named,e)) return 0;
    return KEYBOARD_CONTROLLER;
}
/* Recipes, Mode Dial, then Save Settings. Without a recipe, Recipes does
   not show. */
static const Setting recipe_menu_settings[3]={
    {recipes_row_label,recipes_row_values,recipes_row_text,recipes_row_get,no_set,recipes_row_select,
     SETTING_OWN_PAGE,recipes_row_cursor,recipes_row_title},
    DIAL_MENU_SETTING,
    {save_row_label,one_value,save_row_text,no_value,no_set,save_row_select,SETTING_ACTION},
};
static const Setting *recipe_menu_base(volatile u32 *h,u32 *n) {
    *n=recipe_entries(h) ? 3U : 2U;
    return recipe_menu_settings+(recipe_entries(h) ? 0U : 1U);
}
/* The row of the module shows the recipe that the settings match. */
static const u16 *recipe_menu_value(volatile u32 *h) { return recipe_list_text(h,recipe_matching(h,0)); }
/* The options are the recipes that are on: an option is on while ADJ shows
   the recipe. */
static u32 recipe_option_off(volatile u32 *h,u32 e) { return !recipe_shown(h,e); }
static void recipe_option_set(volatile u32 *h,u32 e,u32 off) { recipe_show_in_adj(h,e,!off); }
static const MenuListing recipe_listing={recipe_menu_base,recipe_menu_value,recipe_entries,menu_recipe_name,
    recipe_option_off,recipe_option_set};
#endif /* GRKIT_MENU_RECIPES_H */
