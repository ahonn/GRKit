#ifndef GRKIT_MENU_DIAL_H
#define GRKIT_MENU_DIAL_H
/* The Mode Dial row of GRKit Settings in MENU (runtime/menu_settings.h:
   CORE_PAGE): what each position of the mode dial does
   (runtime/mode_dial.h). The row always shows: the dial belongs to the
   core.

   - The row opens a list with a row for each position and its value: P,
     Av, Tv, and M (and Sn on GR IV), On or Off, then U1, U2, and U3: On,
     Off, or the name of its snapshot.
   - OK on a row shows its values in place of the list, with the row as the
     title, and the mark and the cursor on its value. P, Av, Tv, M, and Sn:
     On and Off. A user mode: Save Current Settings, On, Off, then the name
     of its snapshot while it has one whose native slot holds what the core
     saved. OK on a value sets it and returns to the list, with the cursor
     on the row. MENU returns to the list and changes nothing.
   - Save Current Settings overwrites the native slot of the user mode, so
     it asks first with the confirm box of the core, Execute or Cancel, the
     cursor on Cancel. The box names each user mode that has the slot that
     the save writes (dial_save_slot), such as "Overwrite U1/U2?". Execute
     opens the keyboard of the camera with the
     automatic name (dial_automatic_name), and OK there saves the current
     settings as the snapshot of the user mode with the typed name
     (dial_snapshot_save), which the user mode then has. Cancel, or a way out
     of the keyboard without OK, saves nothing. The keyboard returns to the
     values of the user mode.
   - A new value applies at once where the dial is (dial_land_again). The
     file of the user modes keeps the values (GRKIT_HOST_DIAL_SET_CHANGES).
   - A half-press sets nothing and saves nothing: the native confirm of the
     page sets a value with Setting::set, which does nothing here.
   - When the page opens, the file of an older build upgrades
     (dial_upgrade), and the core checks the slots (dial_page_opens), so a
     user mode whose slot the person changed in the native menu no longer
     shows its snapshot.

   The list and the values of a row are one own value page, which shows the
   values while MENU_ROWS_PAGE_STATE holds the row. The includer is
   runtime/menu.c, after runtime/mode_dial.h and runtime/menu_settings.h. */
/* The texts of the page (runtime/abi.h: the DIAL run). On and Off are those
   of the switch. */
#define DIAL_TEXT_ROW (TEXT_DIAL+0U)
#define DIAL_TEXT_SAVE (TEXT_DIAL+1U)
#define DIAL_TEXT_OVERWRITE (TEXT_DIAL+2U)
#define DIAL_TEXT_OVERWRITE_AFTER (TEXT_DIAL+3U)
/* The rows of the list: the positions that share their settings, then the
   user modes. */
#define DIAL_ROWS (DIAL_SHARED_COUNT+GRKIT_DIAL_USER_MODES)
/* MENU_ROWS_PAGE_STATE: DIAL_PAGE_SHOWN with the row whose values show, or
   the row that the list puts its cursor on + 1, or 0 for the cursor on the
   first row. */
#define DIAL_PAGE_SHOWN 0x10000U
/* The values of a position that shares its settings: On, then Off. Those of
   a user mode: Save Current Settings, On, Off, then its snapshot. The list
   has no mark: no value is DIAL_NO_MARK. */
#define DIAL_VALUE_SAVE 0U
#define DIAL_VALUE_ON 1U
#define DIAL_VALUE_OFF 2U
#define DIAL_VALUE_SNAPSHOT 3U
#define DIAL_NO_MARK 0xffffffffU
_Static_assert(DIAL_ROWS<RECIPE_ROWS_MAX,"The page has a text buffer for each row");
/* The names of the positions, as the dial shows them and the same in each
   language. */
static const u16 dial_shared_names[][3]={DIAL_SHARED_NAMES};
static const u16 dial_names[GRKIT_DIAL_USER_MODES][3]={{'U','1',0},{'U','2',0},{'U','3',0}};
_Static_assert(sizeof dial_shared_names/sizeof *dial_shared_names==DIAL_SHARED_COUNT,
               "Each position that shares its settings has a name");
static volatile u32 *dial_page_state(volatile u32 *h) { return services_word(h,SERVICES_MENU+MENU_ROWS_PAGE_STATE); }
/* The user mode of row `row`, 1 to 3, or 0 for a position that shares its
   settings. */
static u32 dial_row_user(u32 row) { return row<DIAL_SHARED_COUNT ? 0U : row-DIAL_SHARED_COUNT+1U; }
static const u16 *dial_row_name(u32 row) {
    return row<DIAL_SHARED_COUNT ? dial_shared_names[row] : dial_names[row-DIAL_SHARED_COUNT];
}
/* The row (0 to DIAL_ROWS - 1) whose values show, or DIAL_ROWS while the
   list shows. */
static u32 dial_page_shown(volatile u32 *h) {
    u32 state=dial_page_state(h)[0];
    return state&DIAL_PAGE_SHOWN && (state&0xffU)<DIAL_ROWS ? state&0xffU : DIAL_ROWS;
}
/* The value of row `row`: Off (1) or On (0) for a position that shares its
   settings, and the value of a user mode, its snapshot only while the slot
   holds it. */
static u32 dial_row_value(volatile u32 *h,u32 row) {
    u32 u=dial_row_user(row), value;
    if(!u) return dial_shared_off(h,dial_shared_codes[row]);
    value=dial_value(h,u);
    if(value==GRKIT_DIAL_VALUE_SNAPSHOT && dial_slot_intact(h,u)) return DIAL_VALUE_SNAPSHOT;
    return value==GRKIT_DIAL_VALUE_OFF ? DIAL_VALUE_OFF : DIAL_VALUE_ON;
}
static u32 dial_row_count(volatile u32 *h,u32 row) {
    u32 u=dial_row_user(row);
    return u ? DIAL_VALUE_SNAPSHOT+dial_slot_intact(h,u) : 2U;
}
/* Append the name of the slot of user mode u. */
static u32 dial_append_slot(volatile u16 *to,u32 at,u32 u) {
    u32 name=DIAL_SLOT_NAME_AT(DIAL_SLOT_OF(u)), i, c;
    for(i=0;i<DIAL_NAME_CHARS && (c=BYTE(name,i)) && at+1U<RECIPE_UNITS;++i) to[at++]=(u16)c;
    to[at]=0;
    return at;
}
/* Append the text of value `value` of row `row`. */
static u32 dial_append_value(volatile u32 *h,u32 row,volatile u16 *to,u32 at,u32 value) {
    u32 u=dial_row_user(row);
    if(!u) return recipe_append(to,at,text(h,value ? TEXT_OFF : TEXT_ON));
    if(value==DIAL_VALUE_SAVE) return recipe_append(to,at,text(h,DIAL_TEXT_SAVE));
    if(value==DIAL_VALUE_SNAPSHOT) return dial_append_slot(to,at,u);
    return recipe_append(to,at,text(h,value==DIAL_VALUE_OFF ? TEXT_OFF : TEXT_ON));
}
/* Set row `row` to value `value`, which is not Save Current Settings: the
   file of the user modes saves, and the position acts at once. */
static void dial_row_set(volatile u32 *h,u32 row,u32 value) {
    u32 u=dial_row_user(row), next;
    volatile u32 *at;
    if(u) {
        at=dial_slot_record(h,u)+GRKIT_DIAL_SLOT_VALUE;
        next=value==DIAL_VALUE_OFF ? GRKIT_DIAL_VALUE_OFF : value==DIAL_VALUE_SNAPSHOT ? GRKIT_DIAL_VALUE_SNAPSHOT : GRKIT_DIAL_VALUE_ON;
    } else {
        at=h+GRKIT_HOST_DIAL_SHARED_OFF/4;
        next=value ? *at|dial_shared_bit(row) : *at&~dial_shared_bit(row);
    }
    if(*at==next) return;
    *at=next;
    ++h[GRKIT_HOST_DIAL_SET_CHANGES/4];
    dial_land_again(h);
}
/* The Mode Dial page opens: the core upgrades the file of an older build
   and checks the slots first (runtime/menu_settings.h). */
static void dial_page_opens(volatile u32 *h) {
    dial_upgrade(h);
    dial_slots_check(h);
}

/* ---- The row ---------------------------------------------------------------- */
static const u16 *dial_row_label(volatile u32 *h) { return text(h,DIAL_TEXT_ROW); }
static u32 dial_row_values(volatile u32 *h) {
    u32 row=dial_page_shown(h);
    return row<DIAL_ROWS ? dial_row_count(h,row) : DIAL_ROWS;
}
/* The list shows each row with its value, such as "U1: Street", and the
   values of a row their texts, each in a text buffer of a recipe row, which
   this page does not show: the page copies the text of each value as it
   draws it (entry_text). */
static const u16 *dial_row_text(volatile u32 *h,u32 value) {
    u32 row=dial_page_shown(h), at;
    volatile u16 *to=recipe_text_at(h,(value+1U)*RECIPE_TEXT_SIZE);
    if(row<DIAL_ROWS) {
        if(value>=dial_row_count(h,row)) return no_value_text;
        dial_append_value(h,row,to,0,value);
        return (const u16 *)to;
    }
    if(value>=DIAL_ROWS) return no_value_text;
    at=recipe_append(to,0,dial_row_name(value));
    at=recipe_append(to,at,text(h,TEXT_SEPARATOR));
    dial_append_value(h,value,to,at,dial_row_value(h,value));
    return (const u16 *)to;
}
/* The mark is on the value of the row; the list has none. */
static u32 dial_row_get(volatile u32 *h) {
    u32 row=dial_page_shown(h);
    return row<DIAL_ROWS ? dial_row_value(h,row) : DIAL_NO_MARK;
}
static u32 dial_row_cursor(volatile u32 *h) {
    u32 state=dial_page_state(h)[0];
    if(state&DIAL_PAGE_SHOWN) return dial_row_get(h);
    return state && state-1U<DIAL_ROWS ? state-1U : 0;
}
/* The values of a row have its name as the title; the list has the row
   label. */
static const u16 *dial_row_title(volatile u32 *h) {
    u32 row=dial_page_shown(h);
    return row<DIAL_ROWS ? dial_row_name(row) : 0;
}
/* Append user mode u (1 to 3), and each other user mode that has the slot
   that a save for u writes, in order, with a slash between them. */
static u32 dial_append_users(volatile u16 *to,u32 at,u32 u) {
    static const u16 slash[2]={'/',0};
    u32 slot=dial_save_slot(u), v, first=1;
    for(v=1;v<=GRKIT_DIAL_USER_MODES;++v) {
        if(v!=u && (!slot || DIAL_SLOT_OF(v)!=slot)) continue;
        if(!first) at=recipe_append(to,at,slash);
        at=recipe_append(to,at,dial_names[v-1U]);
        first=0;
    }
    return at;
}
/* The return from the keyboard of Save Current Settings on user mode u (1
   to 3): OK saves with the typed text; the values of u show again. */
static void dial_menu_named(u32 u) {
    volatile u32 *h=menu_header();
    const volatile u8 *typed=ui_keyboard_result(h);
    if(!typed || u-1U>=GRKIT_DIAL_USER_MODES) return;
    dial_snapshot_save(h,u,typed);
}
/* Execute or Cancel of the confirm of Save Current Settings on user mode u:
   Execute opens the keyboard with the automatic name. */
static void dial_menu_confirmed(u32 executed,u32 u) {
    volatile u32 *h=menu_header();
    if(executed!=1U || u-1U>=GRKIT_DIAL_USER_MODES) return;
    ui_keyboard_on(h,dial_word(h,DIAL_PAGE)[0],dial_automatic_name(h),DIAL_NAME_CHARS,dial_menu_named,u);
}
/* OK on a row shows its values. OK on Save Current Settings asks before
   it overwrites the slot; OK on another value sets it and returns to the
   list, as MENU does, which sets nothing. MENU on the list returns to GRKit
   Settings as MENU does. */
static u32 dial_row_select(volatile u32 *h,u32 controller,u32 value) {
    u32 row=dial_page_shown(h), u=dial_row_user(row), at;
    volatile u16 *to;
    if(row>=DIAL_ROWS) {
        if(value>=DIAL_ROWS) return 0;
        dial_page_state(h)[0]=DIAL_PAGE_SHOWN|value;
        settings_value_again(h,0);
        return 1;
    }
    if(u && value==DIAL_VALUE_SAVE) {
        dial_word(h,DIAL_PAGE)[0]=controller;
        to=recipe_text_at(h,RECIPE_BOX_AT);
        at=dial_append_users(to,recipe_append(to,0,text(h,DIAL_TEXT_OVERWRITE)),u);
        recipe_append(to,at,text(h,DIAL_TEXT_OVERWRITE_AFTER));
        ui_confirm_on(h,controller,recipe_box(h),dial_menu_confirmed,u);
        return 1;
    }
    if(value<dial_row_count(h,row)) dial_row_set(h,row,value);
    dial_page_state(h)[0]=row+1U;
    settings_value_again(h,0);
    return 1;
}
static const Setting dial_menu_settings[1]={
    {dial_row_label,dial_row_values,dial_row_text,dial_row_get,no_set,dial_row_select,SETTING_OWN_PAGE,
     dial_row_cursor,dial_row_title},
};
static const Setting *dial_menu_setting(void) { return dial_menu_settings; }
#endif /* GRKIT_MENU_DIAL_H */
