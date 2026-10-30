#ifndef GRKIT_MANAGE_H
#define GRKIT_MANAGE_H
/* Module management: the GRKit section of the native MENU lists each module
   file of the package and of GRKIT/MODULES, also one that did not load. Its
   settings page turns the module on or off (Enable), and the ADJ Order row of
   the section orders the modules, and so their pages in the GRKit root of
   ADJ, at once. Off applies at once too, as a soft Off: the page of the
   module leaves ADJ, and the core calls no code of the module and treats the
   capabilities that only it uses as unused (manage_suspend). On applies at
   once too. A module that is Off at a start loads stopped, so that On can
   apply at once (runtime/install_plan.h).

   Three settings of the settings table keep the choices of each listed
   module (runtime/settings.h): CORE.OFF.<ID>, 1 for Off; CORE.POS.<ID>, its
   position + 1, or 0 for the default place; and CORE.ROWS.<ID>, a bit for
   each value of its page that is off in ADJ (the options of the page,
   runtime/modules.h).
   The resident loader saves them as other settings, and SCAN reads them to
   select and order the modules (runtime/install_plan.h).

   INSTALL writes the module list (GRKIT_HOST_MANAGE): MANAGE_HEADER words,
   then MANAGE_STRIDE bytes for each listed module, in the order of the last
   start. The includer defines u16, u32, and the header fields. */
#define MANAGE_COUNT 0U
/* The handle of CORE.OFF of module 0. Module i has CORE.OFF at handle
   base + 3i, CORE.POS at base + 3i + 1, and CORE.ROWS at base + 3i + 2. */
#define MANAGE_SETTINGS 4U
#define MANAGE_PER_MODULE 3U
/* CORE.ROWS has a bit for each of the first MANAGE_OPTION_VALUES values. */
#define MANAGE_OPTION_VALUES 16U
#define MANAGE_FLAGS 8U
#define MANAGE_SAFE_MODE 1U
/* The listed module whose native settings page the Modules page opened. */
#define MANAGE_OPEN 12U
#define MANAGE_HEADER 16U
/* Each module: its status at this start, the address of the title texts of
   its page or 0, the global index of its tile 0 or MANAGE_NO_TILE, the start
   and the end of its code and the capabilities that it uses (a loaded module;
   0 otherwise), then its name: the file name before .GRM, with the error
   status after a module that did not link. */
#define MANAGE_STATUS 0U
#define MANAGE_TITLE 4U
#define MANAGE_TILE 8U
#define MANAGE_CODE 12U
#define MANAGE_CODE_END 16U
#define MANAGE_USES 20U
#define MANAGE_NAME 24U
#define MANAGE_NAME_UNITS 16U
#define MANAGE_STRIDE (MANAGE_NAME + 2U * MANAGE_NAME_UNITS)
#define MANAGE_NO_TILE 0xffffffffU
/* The status of a module that loaded, also stopped, and of one that safe
   mode kept out. Other values are the errors of runtime/module_link.h. */
#define MANAGE_LOADED 0U
#define MANAGE_SAFE 1U
/* The most listed modules (MODULES_MAX of the installer). */
#define MANAGE_MODULES GRKIT_MODULES_MAX
/* A position without a CORE.POS value sorts after each position. */
#define MANAGE_DEFAULT_KEY 1000U

#ifdef MENU_TEXT
/* The module list at a camera address. A host test maps it. */
#ifndef MANAGE_MAP
#define MANAGE_MAP(address) ((volatile u32 *)(address))
#endif

static u32 manage_count(volatile u32 *h) {
    u32 list=h[GRKIT_HOST_MANAGE/4];
    u32 n=list ? MANAGE_MAP(list+MANAGE_COUNT)[0] : 0;
    return n<=MANAGE_MODULES ? n : 0;
}
static u32 manage_word(volatile u32 *h,u32 offset) { return MANAGE_MAP(h[GRKIT_HOST_MANAGE/4]+offset)[0]; }
static u32 manage_entry(volatile u32 *h,u32 module) { return h[GRKIT_HOST_MANAGE/4]+MANAGE_HEADER+module*MANAGE_STRIDE; }
static u32 manage_off_handle(volatile u32 *h,u32 module) {
    return manage_word(h,MANAGE_SETTINGS)+MANAGE_PER_MODULE*module;
}
static u32 manage_off(volatile u32 *h,u32 module) { return setting_value(h,manage_off_handle(h,module)); }
/* The values of the page of a module that are off in ADJ, a bit each. */
static u32 manage_rows_handle(volatile u32 *h,u32 module) { return manage_off_handle(h,module)+2; }
static u32 manage_hidden_values(volatile u32 *h,u32 module) { return setting_value(h,manage_rows_handle(h,module)); }
/* The sort key of a module: its CORE.POS, or the default place after them. */
static u32 manage_key(volatile u32 *h,u32 module) {
    u32 position=setting_value(h,manage_off_handle(h,module)+1);
    return position ? position : MANAGE_DEFAULT_KEY+module;
}
/* The module of a row: the modules in the order of their keys. Equal keys
   keep the order of this start. */
static u32 manage_module(volatile u32 *h,u32 row) {
    u32 n=manage_count(h), i, j, before;
    for(i=0;i<n;++i) {
        before=0;
        for(j=0;j<n;++j) {
            u32 a=manage_key(h,j), b=manage_key(h,i);
            if(a<b || (a==b && j<i)) ++before;
        }
        if(before==row) return i;
    }
    return 0;
}
/* The listed modules that the root of ADJ can show, a bit each: those with
   a module page of two rows or more, as the menu last found them
   (runtime/modules.h: manage_adj_refresh). A soft Off hides a page in ADJ,
   but its module keeps its place in ADJ Order. */
static u32 manage_adj_modules(volatile u32 *h);
/* ADJ Order lists these modules in the order of their keys: the number of
   its rows, the module of a row, and the row of a module. */
static u32 manage_adj_count(volatile u32 *h) {
    u32 mask=manage_adj_modules(h), n=0;
    for(;mask;mask&=mask-1) ++n;
    return n;
}
static u32 manage_adj_module(volatile u32 *h,u32 row) {
    u32 mask=manage_adj_modules(h), n=manage_count(h), i, module;
    for(i=0;i<n;++i) {
        module=manage_module(h,i);
        if(mask>>module&1 && !row--) return module;
    }
    return 0;
}
static u32 manage_adj_row(volatile u32 *h,u32 module) {
    u32 mask=manage_adj_modules(h), n=manage_count(h), i, row=0, m;
    for(i=0;i<n;++i) {
        m=manage_module(h,i);
        if(m==module) return row;
        row+=mask>>m&1;
    }
    return 0;
}
static u32 manage_status(volatile u32 *h,u32 module) { return MANAGE_MAP(manage_entry(h,module)+MANAGE_STATUS)[0]; }
/* Whether a soft Off stops a module that loaded at this start. */
static u32 manage_stopped(volatile u32 *h,u32 module) {
    return manage_status(h,module)==MANAGE_LOADED && manage_off(h,module);
}
/* Apply the soft Off after a change of Enable: the code ranges of the
   stopped modules (SERVICES_SUSPENDED), and the capabilities of the modules
   that run (GRKIT_HOST_CAPABILITIES). */
static void manage_suspend(volatile u32 *h) {
    u32 i, n=0, uses=0;
    if(!h[GRKIT_HOST_SERVICES/4]) return;
    for(i=0;i<manage_count(h);++i) {
        u32 entry=manage_entry(h,i);
        if(manage_status(h,i)!=MANAGE_LOADED) continue;
        if(!manage_off(h,i)) { uses|=MANAGE_MAP(entry+MANAGE_USES)[0]; continue; }
        if(n>=SERVICES_SUSPENDED_MAX) continue;
        services_word(h,SERVICES_SUSPENDED+4+8*n)[0]=MANAGE_MAP(entry+MANAGE_CODE)[0];
        services_word(h,SERVICES_SUSPENDED+8+8*n)[0]=MANAGE_MAP(entry+MANAGE_CODE_END)[0];
        ++n;
    }
    services_word(h,SERVICES_SUSPENDED)[0]=n;
    h[GRKIT_HOST_CAPABILITIES/4]=uses;
}
/* The name of a module: the title of its page in the menu language, else
   its file name. */
static const u16 *manage_name(volatile u32 *h,u32 module) {
    u32 entry=manage_entry(h,module), title=MANAGE_MAP(entry+MANAGE_TITLE)[0];
    if(title) return MENU_TEXT(PAGE_WORD(title,menu_language()*4));
    return (const u16 *)MANAGE_MAP(entry+MANAGE_NAME);
}

/* The listed module of a loaded module page, by the address of its title
   texts, or MANAGE_MODULES. */
static u32 manage_of_title(volatile u32 *h,u32 title) {
    u32 i;
    for(i=0;i<manage_count(h);++i)
        if(title && MANAGE_MAP(manage_entry(h,i)+MANAGE_TITLE)[0]==title) return i;
    return MANAGE_MODULES;
}
static const u16 *manage_title(volatile u32 *h) { return text(h,TEXT_ADJ); }
static const u16 *manage_summary(volatile u32 *h) {
    return text(h,TEXT_MODULES_SUMMARY+(manage_word(h,MANAGE_FLAGS)&MANAGE_SAFE_MODE ? 1 : 0));
}
static u32 manage_rows(volatile u32 *h) { return manage_adj_count(h); }
static const u16 *manage_label(volatile u32 *h,u32 row) { return manage_name(h,manage_adj_module(h,row)); }
/* A module that is Off shows the Off icon, and a loaded module its tile. */
static u32 manage_icon(volatile u32 *h,u32 row) {
    u32 module, tile;
    if(h[GRKIT_HOST_MENU_LEVEL/4]==MENU_ROOT) return GRKIT_ICON_MODULES;
    module=manage_adj_module(h,row);
    if(manage_off(h,module)) return GRKIT_ICON_OFF;
    tile=MANAGE_MAP(manage_entry(h,module)+MANAGE_TILE)[0];
    return tile!=MANAGE_NO_TILE ? GRKIT_ICON_MODULE_TILE(tile) : GRKIT_ICON_MODULES;
}
static u32 manage_checked(volatile u32 *h,u32 row) { return !manage_off(h,manage_adj_module(h,row)); }
/* OK opens the settings of the row (runtime/menu.c: open_settings). */
static void manage_apply(volatile u32 *h,u32 row) { (void)h; (void)row; }
/* The page opens at the module of the last settings page. */
static u32 manage_committed(volatile u32 *h) {
    u32 module=manage_word(h,MANAGE_OPEN);
    return module<manage_count(h) ? manage_adj_row(h,module) : 0;
}

/* The ADJ Order row of the GRKit section of MENU (runtime/menu_section.h) opens a
   native settings page with a row for each module that ADJ can show
   (manage_adj_modules), in their order. The values of a row are the
   positions 1 to the row count; the row shows the position of the module. A
   choice moves the module to that position and gives each of these modules
   its row + 1 as CORE.POS, so the page and the root of ADJ
   (runtime/modules.h: root_order) show the new order at once. The other
   modules keep their CORE.POS. */
_Static_assert(MANAGE_MODULES<=SUBMENU_ROWS,"The ADJ Order page has a row for each module");
/* More positions than SETTING_VALUES make a long value page
   (runtime/menu_settings.h: settings_long). */
_Static_assert(MANAGE_MODULES<=MENU_VALUES_MAX,"A value page holds each position");
static u32 manage_values(volatile u32 *h) { return manage_adj_count(h); }
static const u16 manage_positions[][3]={{'1',0},{'2',0},{'3',0},{'4',0},{'5',0},{'6',0},{'7',0},{'8',0},{'9',0},
    {'1','0',0}};
_Static_assert(sizeof(manage_positions)/sizeof(manage_positions[0])==MANAGE_MODULES,"A text for each position");
static const u16 *manage_value_text(volatile u32 *h,u32 value) {
    (void)h;
    return manage_positions[value<MANAGE_MODULES ? value : MANAGE_MODULES-1];
}
/* Move the module at ADJ Order row `row` to row `to`, the others keep their
   order: each module of the page gets its row + 1. */
static void manage_move(volatile u32 *h,u32 row,u32 to) {
    u32 n=manage_adj_count(h), order[MANAGE_MODULES], i, moved;
    if(row>=n || to>=n) return;
    for(i=0;i<n;++i) order[i]=manage_adj_module(h,i);
    moved=order[row];
    for(i=row;i<to;++i) order[i]=order[i+1];
    for(i=row;i>to;--i) order[i]=order[i-1];
    order[to]=moved;
    for(i=0;i<n;++i) setting_store(h,manage_off_handle(h,order[i])+1,i+1);
}
#define MANAGE_ROW(n) \
    static const u16 *manage_label_##n(volatile u32 *h) { \
        return manage_name(h,manage_adj_module(h,n)); } \
    static u32 manage_get_##n(volatile u32 *h) { (void)h; return n; } \
    static void manage_set_##n(volatile u32 *h,u32 value) { manage_move(h,n,value); }
MANAGE_ROW(0) MANAGE_ROW(1) MANAGE_ROW(2) MANAGE_ROW(3) MANAGE_ROW(4) MANAGE_ROW(5) MANAGE_ROW(6) MANAGE_ROW(7)
MANAGE_ROW(8) MANAGE_ROW(9)
static const Setting manage_settings[SUBMENU_ROWS]={
    {manage_label_0,manage_values,manage_value_text,manage_get_0,manage_set_0},
    {manage_label_1,manage_values,manage_value_text,manage_get_1,manage_set_1},
    {manage_label_2,manage_values,manage_value_text,manage_get_2,manage_set_2},
    {manage_label_3,manage_values,manage_value_text,manage_get_3,manage_set_3},
    {manage_label_4,manage_values,manage_value_text,manage_get_4,manage_set_4},
    {manage_label_5,manage_values,manage_value_text,manage_get_5,manage_set_5},
    {manage_label_6,manage_values,manage_value_text,manage_get_6,manage_set_6},
    {manage_label_7,manage_values,manage_value_text,manage_get_7,manage_set_7},
    {manage_label_8,manage_values,manage_value_text,manage_get_8,manage_set_8},
    {manage_label_9,manage_values,manage_value_text,manage_get_9,manage_set_9},
};
#define MANAGE_PAGE {manage_title,manage_summary,manage_rows,manage_label,manage_icon, \
    manage_checked,manage_apply,manage_committed,0,0,manage_settings}
#endif
#endif
