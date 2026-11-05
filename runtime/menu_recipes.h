#ifndef GRKIT_MENU_RECIPES_H
#define GRKIT_MENU_RECIPES_H
/* The page of the Recipes module in the GRKit section of MENU
   (runtime/menu_listed.h), in the form of the native User Mode page: each
   action is a row whose value page lists the recipes. The includer is
   runtime/menu_listed.h.

   - Enable, then Options, which chooses the recipes that ADJ shows, as the
     Options of other modules choose their values (runtime/menu_listed.h).
   - The lists have the recipes that are on (runtime/recipes.h). The camera
     keeps the recipes that are off for the site and does not list them.
   - Apply lists each recipe that is on, also those that ADJ does not show,
     with the mark on the recipe that the current settings match. OK applies
     the recipe and returns to the page.
   - Rename lists the recipes. OK opens the keyboard of the camera with the
     name, and the keyboard returns to the list.
   - Delete lists the recipes. OK asks with the confirm box of the core,
     Execute or Cancel, the cursor on Cancel. Execute deletes the recipe and
     starts the list again, or returns to the page after the last recipe.
   - Save Current Settings adds a recipe that is on, then opens the keyboard
     with its name. Its row shows the number of recipes that are on and the
     most of them, and while no recipe can be on, OK does nothing.
   - Without a recipe, the page has Enable and Save Current Settings.
   - The lists of Rename and Delete keep the row of the cursor and have no
     mark (SETTING_CURSOR). The undo steps show only in ADJ. MENU shows no
     message after Apply: a recipe that cannot apply changes nothing.

   The page uses the parts of the settings page and the UI of the core:
   own value pages, the confirm box, and the keyboard (core.h: Setting,
   runtime/ui.h). */
/* SERVICES_MENU + MENU_ROWS_RECIPE: the row of the cursor of the lists of
   Rename and Delete. */
/* The texts of the page (runtime/abi.h: the RECIPE run). */
#define RECIPE_TEXT_APPLY (TEXT_RECIPE+14U)
static u32 ui_keyboard_prepare(volatile u32 *h,const volatile u16 *text,void (*done)(u32),u32 arg);
static u32 ui_keyboard_on(volatile u32 *h,u32 controller,const volatile u16 *text,void (*done)(u32),u32 arg);
static const volatile u8 *ui_keyboard_result(volatile u32 *h);
static u32 ui_confirm_on(volatile u32 *h,u32 controller,const u16 *const *text,void (*done)(u32,u32),u32 arg);
static void settings_value_again(volatile u32 *h,u32 back);
static const volatile u16 *recipe_keyboard_text(volatile u32 *h,u32 slot);

static volatile u32 *menu_recipe_word(volatile u32 *h) { return services_word(h,SERVICES_MENU+MENU_ROWS_RECIPE); }

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

/* ---- The rows --------------------------------------------------------------- */
static const u16 *apply_row_label(volatile u32 *h) { return text(h,RECIPE_TEXT_APPLY); }
static const u16 *rename_row_label(volatile u32 *h) { return text(h,RECIPE_TEXT_RENAME); }
static const u16 *delete_row_label(volatile u32 *h) { return text(h,RECIPE_TEXT_REMOVE); }
static u32 recipe_list_values(volatile u32 *h) { return recipe_entries(h); }
static const u16 *recipe_list_text(volatile u32 *h,u32 value) {
    return value<recipe_entries(h) ? menu_recipe_name(h,value) : no_value_text;
}
/* Apply: the mark is on the recipe that the current settings match, and OK
   applies a recipe. */
static u32 apply_row_get(volatile u32 *h) { return recipe_current(h); }
static void apply_row_set(volatile u32 *h,u32 value) {
    if(value<recipe_entries(h)) recipe_apply(h,value);
}
/* Rename and Delete: the value is the row of the cursor. */
static u32 cursor_row_get(volatile u32 *h) {
    u32 e=menu_recipe_word(h)[0];
    return e<recipe_entries(h) ? e : 0;
}
/* The return from the keyboard that names recipe e: the text of an OK.
   MENU keeps the name. */
static void recipe_named(u32 e) {
    volatile u32 *h=menu_header();
    const volatile u8 *name=ui_keyboard_result(h);
    if(name && e<recipe_entries(h)) recipe_name_set(h,e,name);
}
/* OK on a recipe of Rename: the keyboard with its name, which returns to
   the list. A keyboard that another use holds does not open, and the list
   stays. */
static u32 rename_row_select(volatile u32 *h,u32 controller,u32 e) {
    if(e>=recipe_entries(h)) return 0;
    menu_recipe_word(h)[0]=e;
    ui_keyboard_on(h,controller,recipe_keyboard_text(h,e),recipe_named,e);
    return 1;
}
/* Execute or Cancel of the box of Delete on recipe e. Execute deletes it and
   starts the list again with the cursor on the same row, or returns to the
   page after the last recipe. */
static void delete_row_done(u32 executed,u32 e) {
    volatile u32 *h=menu_header();
    if(!executed || e>=recipe_entries(h)) return;
    recipe_delete(h,e);
    menu_recipe_word(h)[0]=e<recipe_entries(h) ? e : e-1U;
    settings_value_again(h,!recipe_entries(h));
}
/* OK on a recipe of Delete asks first. */
static u32 delete_row_select(volatile u32 *h,u32 controller,u32 e) {
    volatile u16 *to=recipe_text_at(h,RECIPE_BOX_AT);
    u32 at;
    if(e>=recipe_entries(h)) return 0;
    at=recipe_append(to,0,text(h,RECIPE_TEXT_DELETE));
    at=recipe_append_name(h,to,at,e);
    recipe_append(to,at,text(h,RECIPE_TEXT_DELETE_AFTER));
    menu_recipe_word(h)[0]=e;
    ui_confirm_on(h,controller,recipe_box(h),delete_row_done,e);
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
/* OK on Save Current Settings: the keyboard with the name of the new
   recipe, which returns to the page, or 0 when the page stays: no recipe
   can be on (recipe_save), or another use holds the keyboard. */
static u32 save_row_select(volatile u32 *h,u32 controller,u32 value) {
    u32 e;
    (void)controller; (void)value;
    if(!recipe_listed(h)) return 0;
    e=recipe_save(h);
    if(e==RECIPE_FULL || !ui_keyboard_prepare(h,recipe_keyboard_text(h,e),recipe_named,e)) return 0;
    return KEYBOARD_CONTROLLER;
}
/* Apply, Rename, Delete, then Save. Without a recipe, only Save shows. */
static const Setting recipe_menu_settings[4]={
    {apply_row_label,recipe_list_values,recipe_list_text,apply_row_get,apply_row_set,0,SETTING_OWN_PAGE},
    {rename_row_label,recipe_list_values,recipe_list_text,cursor_row_get,no_set,rename_row_select,
     SETTING_OWN_PAGE|SETTING_CURSOR},
    {delete_row_label,recipe_list_values,recipe_list_text,cursor_row_get,no_set,delete_row_select,
     SETTING_OWN_PAGE|SETTING_CURSOR},
    {save_row_label,one_value,save_row_text,no_value,no_set,save_row_select,SETTING_ACTION},
};
static const Setting *recipe_menu_base(volatile u32 *h,u32 *n) {
    *n=recipe_entries(h) ? 4U : 1U;
    return recipe_menu_settings+(recipe_entries(h) ? 0U : 3U);
}
#endif /* GRKIT_MENU_RECIPES_H */
