#ifndef GRKIT_MENU_DIAL_H
#define GRKIT_MENU_DIAL_H
/* The Mode Dial row of the Recipes page of MENU (runtime/menu_recipes.h): what
   the user modes U1 to U3 of the mode dial do (runtime/mode_dial.h).

   - The row opens a list of the user modes. Each shows its value: None (the
     user mode follows), GRKit Off, or the name of its recipe. A recipe that
     the camera does not list, as a deleted recipe or one that is off, shows
     None: the user mode then follows (dial_assignment).
   - OK on a user mode shows its values in place of the list, with the user
     mode as the title: None, GRKit Off, then each recipe that is on, with
     the mark and the cursor on its value. OK on a value sets it and returns
     to the list, with the cursor on the user mode. MENU returns to the list
     and changes nothing.
   - The recipe file keeps the values (GRKIT_RECIPES_CHANGED). While the dial
     is on that user mode, a new value applies at once (dial_assigned).
   - A half-press sets nothing: the native confirm of the page sets a value
     with Setting::set, which does nothing here.

   As the Recipes row, the list and the values of a user mode are one own
   value page, which shows the values while MENU_ROWS_PAGE_STATE holds the
   user mode. The includer is runtime/menu.c, after runtime/mode_dial.h and
   before runtime/menu_recipes.h. */
/* The texts of the page (runtime/abi.h: the DIAL run). GRKit Off is
   DIAL_TEXT_OFF, which the label of the dial shows too (runtime/mode_dial.h). */
#define DIAL_TEXT_ROW (TEXT_DIAL+0U)
#define DIAL_TEXT_NONE (TEXT_DIAL+1U)
/* MENU_ROWS_PAGE_STATE: DIAL_PAGE_SHOWN with the user mode (0 to 2) whose
   values show, or the user mode that the list puts its cursor on + 1, or 0
   for the cursor on U1. */
#define DIAL_PAGE_SHOWN 0x10000U
/* The values of a user mode: None, GRKit Off, then recipe e at
   DIAL_VALUE_RECIPE + e. The list has no mark: no value is DIAL_NO_MARK. */
#define DIAL_VALUE_NONE 0U
#define DIAL_VALUE_OFF 1U
#define DIAL_VALUE_RECIPE 2U
#define DIAL_NO_MARK 0xffffffffU
_Static_assert(DIAL_VALUE_RECIPE+GRKIT_RECIPE_ON_MAX<=MENU_VALUES_MAX,"The page lists each recipe that is on");
static volatile u32 *dial_page_state(volatile u32 *h) { return services_word(h,SERVICES_MENU+MENU_ROWS_PAGE_STATE); }
/* The user mode (0 to 2) whose values show, or GRKIT_RECIPE_DIAL_POSITIONS
   while the list shows. */
static u32 dial_page_shown(volatile u32 *h) {
    u32 state=dial_page_state(h)[0];
    return state&DIAL_PAGE_SHOWN ? state&0xffffU : GRKIT_RECIPE_DIAL_POSITIONS;
}
/* The value of user mode u (0 to 2), as the dial applies it. */
static u32 dial_value(volatile u32 *h,u32 u) {
    u32 entry=0, kind=dial_assignment(h,u+1U,&entry);
    return kind==DIAL_RECIPE ? DIAL_VALUE_RECIPE+entry : kind==DIAL_OFF ? DIAL_VALUE_OFF : DIAL_VALUE_NONE;
}
static u32 dial_values(volatile u32 *h) { return DIAL_VALUE_RECIPE+recipe_entries(h); }
/* Append the text of value `value` of a user mode. A recipe has its name. */
static u32 dial_append_value(volatile u32 *h,volatile u16 *to,u32 at,u32 value) {
    if(value>=DIAL_VALUE_RECIPE) return recipe_append_name(h,to,at,value-DIAL_VALUE_RECIPE);
    return recipe_append(to,at,text(h,value==DIAL_VALUE_OFF ? DIAL_TEXT_OFF : DIAL_TEXT_NONE));
}
/* Set what user mode u (0 to 2) does to value `value`. A recipe that has no
   ID yet, which INSTALL gives at the next start, gets one now. */
static void dial_assign(volatile u32 *h,u32 u,u32 value) {
    volatile u32 *at=h+GRKIT_HOST_RECIPE_DIAL/4+u, *id;
    u32 next=value==DIAL_VALUE_OFF ? GRKIT_RECIPE_DIAL_OFF : GRKIT_RECIPE_DIAL_FOLLOW;
    if(value>=DIAL_VALUE_RECIPE) {
        id=recipe_id_at(h,value-DIAL_VALUE_RECIPE);
        if(!*id) *id=recipe_new_id(h);
        next=*id;
    }
    if(*at==next) return;
    *at=next;
    GRKIT_RECIPES_CHANGED(h);
    dial_assigned(h,u+1U);
}

/* ---- The row ---------------------------------------------------------------- */
static const u16 *dial_row_label(volatile u32 *h) { return text(h,DIAL_TEXT_ROW); }
static u32 dial_row_values(volatile u32 *h) {
    return dial_page_shown(h)<GRKIT_RECIPE_DIAL_POSITIONS ? dial_values(h) : GRKIT_RECIPE_DIAL_POSITIONS;
}
/* The list shows each user mode with its value, such as "U1: Street", in
   the text buffer of a box, which this page does not show: the page copies
   the text of each value as it draws it (entry_text). */
static const u16 *dial_row_text(volatile u32 *h,u32 value) {
    u32 u=dial_page_shown(h), at;
    volatile u16 *to;
    if(u<GRKIT_RECIPE_DIAL_POSITIONS) {
        if(value>=dial_values(h)) return no_value_text;
        if(value<DIAL_VALUE_RECIPE) return text(h,value==DIAL_VALUE_OFF ? DIAL_TEXT_OFF : DIAL_TEXT_NONE);
        to=recipe_text_at(h,(value-DIAL_VALUE_RECIPE+1U)*RECIPE_TEXT_SIZE);
        dial_append_value(h,to,0,value);
        return (const u16 *)to;
    }
    if(value>=GRKIT_RECIPE_DIAL_POSITIONS) return no_value_text;
    to=recipe_text_at(h,RECIPE_BOX_AT);
    at=recipe_append(to,recipe_append(to,0,dial_names[value]),text(h,TEXT_SEPARATOR));
    dial_append_value(h,to,at,dial_value(h,value));
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
/* OK on a user mode shows its values. OK on a value sets it, and MENU there
   sets nothing; both return to the list. MENU on the list returns to the
   Recipes page as MENU does. */
static u32 dial_row_select(volatile u32 *h,u32 controller,u32 value) {
    u32 u=dial_page_shown(h);
    (void)controller;
    if(u>=GRKIT_RECIPE_DIAL_POSITIONS) {
        if(value>=GRKIT_RECIPE_DIAL_POSITIONS) return 0;
        dial_page_state(h)[0]=DIAL_PAGE_SHOWN|value;
        settings_value_again(h,0);
        return 1;
    }
    if(value<dial_values(h)) dial_assign(h,u,value);
    dial_page_state(h)[0]=u+1U;
    settings_value_again(h,0);
    return 1;
}
#define DIAL_MENU_SETTING \
    {dial_row_label,dial_row_values,dial_row_text,dial_row_get,no_set,dial_row_select,SETTING_OWN_PAGE, \
     dial_row_cursor,dial_row_title}
#endif /* GRKIT_MENU_DIAL_H */
