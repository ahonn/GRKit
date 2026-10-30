#ifndef GRKIT_MENU_SECTION_H
#define GRKIT_MENU_SECTION_H
/* The GRKit section of the Customize tab of the native MENU (runtime/menu.c). */
/* The GRKit section of the native MENU. MENU builds its tabs at the first
   MENU of a start, after a controller change. So the first controller
   change after INSTALL copies the Customize tab descriptor into the services
   block (SERVICES_MENU), adds a section with the title GRKit and its rows,
   and points the RAM tab table at the copy. A MENU that was built before
   INSTALL gives up its instance in the controller factory, so the next MENU
   builds its tabs again.

   The rows: a row for each listed module (runtime/manage.h), which opens
   its Enable row, its Options row, and the settings of its module page; ADJ
   Order, which orders the module pages in ADJ; then About GRKit, which
   shows the GRKit version and the project address.

   A row targets MENU_ROW_TARGET + its slot, which no controller has: the
   controller change becomes a change to the borrowed settings submenu with
   the settings of the row. MENU on the submenu returns to MENU. A filler
   gives the row the text IDs MENU_ROW_LABEL_TEXT and MENU_ROW_VALUE_TEXT,
   and the text hook gives the label and the value of the row for its
   views. The section title has the text ID MENU_ROW_LABEL_TEXT too, which no
   native MENU row or title has: the text hook shows GRKit for it while no
   submenu or value page that borrows the ID shows. */
#define MENU_ROW_TARGET 0x2000f00U
#define MENU_ROW_GRKIT 1U
#define MENU_ROW_LISTED 2U
#define MENU_ROW_ADJ 3U
/* The model snapshot that a cancel of the parameter page restores, as
   open_parameters takes it from ADJ. FW_ADJ_SNAPSHOT reads only the movie
   flag of its controller: GR III the byte ADJ_MOVIE_FIELD, GR IV a global
   setting. So MENU passes a stand-in whose flag reads still: a constant 0
   byte at the field. */
static const u8 adj_still[1]={0};
static void menu_still_snapshot(void) {
#ifdef ADJ_MOVIE_FIELD
    CALL1(FW_ADJ_SNAPSHOT)((u32)adj_still-ADJ_MOVIE_FIELD);
#else
    CALL1(FW_ADJ_SNAPSHOT)((u32)adj_still);
#endif
}
/* The text IDs of the label and the value of a MENU row, and of the section
   title: those of the first two rows of the borrowed submenu. No view of the
   Customize tab shows them, so the text hook can claim them there; it gives
   the texts of one screen at a time (menu_text_override). */
#define MENU_ROW_LABEL_TEXT (settings_row_texts[0])
#define MENU_ROW_VALUE_TEXT (settings_row_texts[1])
_Static_assert(MENU_TAB_SIZE==MENU_ROWS_TAB_SIZE,"The services block holds a MENU tab");
static volatile u32 *menu_row(volatile u32 *h,u32 slot) {
    return services_word(h,SERVICES_MENU+MENU_ROWS_FIRST+slot*MENU_ROW_SIZE);
}
/* The page slot of listed module `module`, or GRKIT_MODULE_PAGES_MAX. */
static u32 listed_page(volatile u32 *h,u32 module) {
    u32 slot;
    for(slot=0;slot<module_pages(h);++slot) if(page_listed(h,slot)==module) return slot;
    return GRKIT_MODULE_PAGES_MAX;
}
/* The fillers of the rows (below). */
static void (*const fillers[MENU_ROWS_SLOTS])(u32,u32);
static u32 menu_rows_add(volatile u32 *h,u32 copy,u32 *rows,u32 slots,u32 kind,u32 module) {
    if(slots>=MENU_ROWS_SLOTS || *rows>=MENU_TAB_MAX_ROWS) return slots;
    WORD(copy,MENU_TAB_FILLERS+8**rows)=(u32)fillers[slots];
    WORD(copy,MENU_TAB_FILLERS+8**rows+4)=0;
    WORD(copy,MENU_TAB_TARGETS+4**rows)=MENU_ROW_TARGET+slots;
    WORD(copy,MENU_TAB_PREDICATES+8**rows)=0;
    WORD(copy,MENU_TAB_PREDICATES+8**rows+4)=0;
    menu_row(h,slots)[MENU_ROW_MODULE/4]=module;
    menu_row(h,slots)[MENU_ROW_KIND/4]=kind;
    ++*rows;
    return slots+1;
}
/* The GRKit rows fit the Customize tab of each build: its stock rows
   (MENU_TAB_STOCK_ROWS of the build header), then a row for each listed
   module, ADJ Order, and About GRKit. */
_Static_assert(MENU_ROWS_SLOTS==MANAGE_MODULES+2U,"A row slot for each GRKit row");
_Static_assert(MENU_TAB_STOCK_ROWS+MENU_ROWS_SLOTS<=MENU_TAB_MAX_ROWS,"The Customize tab holds each GRKit row");
static void menu_rows_install(volatile u32 *h) {
    volatile u32 *state=services_word(h,SERVICES_MENU);
    u32 copy=(u32)services_word(h,SERVICES_MENU+MENU_ROWS_TAB), stock, rows, sections, slots=0, i, top;
    u32 listed=manage_count(h), adj;
    if(state[MENU_ROWS_DONE/4]) return;
    state[MENU_ROWS_DONE/4]=1;
    manage_adj_refresh(h);
    adj=manage_adj_count(h)>0;
    stock=WORD(FW_MENU_TABS+4*MENU_TAB_GRKIT,0);
    if(!stock) return;
    for(i=0;i<MENU_TAB_SIZE/4;++i) WORD(copy,4*i)=WORD(stock,4*i);
    rows=BYTE(copy,MENU_TAB_ROWS);
    if(rows>=MENU_TAB_MAX_ROWS) return;
    /* The section starts at the first GRKit row. A tab with all its sections
       gets the rows in its last section. */
    sections=BYTE(copy,MENU_TAB_SECTIONS);
    if(sections<MENU_TAB_MAX_SECTIONS) {
        HALF(copy,MENU_TAB_TITLES+2*sections)=MENU_ROW_LABEL_TEXT;
        BYTE(copy,MENU_TAB_STARTS+sections)=(u8)rows;
        BYTE(copy,MENU_TAB_SECTIONS)=(u8)(sections+1);
    }
    descriptor(services_word(h,SERVICES_MENU+MENU_ROWS_TITLE_AT),grkit_title);
    /* A build with more stock rows than its header gives, such as a build
       that INSTALL relocates, keeps ADJ Order and About GRKit: the last
       modules of the default order get no row. */
    if(rows+listed+adj+1U>MENU_TAB_MAX_ROWS)
        listed=MENU_TAB_MAX_ROWS>rows+adj+1U ? MENU_TAB_MAX_ROWS-rows-adj-1U : 0;
    /* The modules in their default order, which the ADJ Order row changes
       only in ADJ. */
    for(i=0;i<listed;++i) slots=menu_rows_add(h,copy,&rows,slots,MENU_ROW_LISTED,i);
    if(adj) slots=menu_rows_add(h,copy,&rows,slots,MENU_ROW_ADJ,0);
    slots=menu_rows_add(h,copy,&rows,slots,MENU_ROW_GRKIT,0);
    state[MENU_ROWS_COUNT/4]=slots;
    BYTE(copy,MENU_TAB_ROWS)=(u8)rows;
    WORD(FW_MENU_TABS+4*MENU_TAB_GRKIT,0)=copy;
    /* A MENU that was built before this keeps its tabs. Without it in the
       factory, the next MENU builds a new one with the copy. */
    top=((u32 (*)(void))FW_CONTROLLER_FACTORY)();
    if(top) WORD(top,MENU_TOP_SLOT)=0;
}
static u32 menu_rows_controller(volatile u32 *h,u32 id) {
    u32 slot=id-MENU_ROW_TARGET, kind, module;
    if(!h[GRKIT_HOST_SERVICES/4]) return id;
    menu_rows_install(h);
    /* The Fn adjustments row of the page of Image Control. Without a
       committed effect, it opens nothing. */
    if(id==MENU_FN_TARGET) {
        if(!h[GRKIT_HOST_ACTIVE/4]) return SETTINGS_SUBMENU;
        menu_still_snapshot();
        h[GRKIT_HOST_PARAMETERS/4]=h[GRKIT_HOST_ACTIVE/4]|1U;
        h[GRKIT_HOST_SETTINGS/4]|=SETTINGS_FN;
        return IC_PARAMETERS_CONTROLLER;
    }
    if(slot>=services_word(h,SERVICES_MENU)[MENU_ROWS_COUNT/4]) return id;
    kind=menu_row(h,slot)[MENU_ROW_KIND/4];
    module=menu_row(h,slot)[MENU_ROW_MODULE/4];
    if(kind==MENU_ROW_ADJ) {
        settings_start(h,MANAGE_MODULE);
        return SETTINGS_SUBMENU;
    }
    if(kind==MENU_ROW_LISTED) {
        services_word(h,SERVICES_MENU+MENU_ROWS_LISTED)[0]=module;
        services_word(h,SERVICES_MENU+MENU_ROWS_PAGE)[0]=listed_page(h,module);
        settings_start(h,LISTED_PAGE);
        return SETTINGS_SUBMENU;
    }
    /* About GRKit opens the camera information page (menu_info_fill). */
    services_word(h,SERVICES_MENU+MENU_ROWS_ABOUT)[0]=1;
    return INFO_CONTROLLER;
}
static const u16 no_text[]={0};
/* The value of the About GRKit row: safe mode, an effects choice for the
   next start, or the version. */
static const u16 *grkit_value(volatile u32 *h) {
    if(manage_word(h,MANAGE_FLAGS)&MANAGE_SAFE_MODE) return text(h,TEXT_SAFE_MODE);
    if(effects_changed(h)) return text(h,TEXT_RESTART);
    return text(h,TEXT_ABOUT+2);
}
/* A filler: the label and the value of row view `row`. The views are known
   first, because a setter asks the text hook at once. A view measures its
   text only when its ID changes, so each gets ID 0 first. */
static void menu_row_fill(u32 slot,u32 row) {
    volatile u32 *h=menu_header(), *at=menu_row(h,slot);
    u32 kind, module, page;
    const u16 *label, *value;
    if(!h[GRKIT_HOST_SERVICES/4] || slot>=services_word(h,SERVICES_MENU)[MENU_ROWS_COUNT/4]) return;
    /* The text hook finds a row by the address of its view, and a row view
       can move to another slot when the list scrolls: a view is in one slot. */
    {
        u32 label_view=MENU_OBJECT_WORD(row,MENU_ROW_LABEL_VIEW), value_view=MENU_OBJECT_WORD(row,MENU_ROW_VALUE_VIEW), s;
        for(s=0;s<MENU_ROWS_SLOTS;++s) {
            volatile u32 *other=menu_row(h,s);
            if(other[MENU_ROW_LABEL/4]==label_view || other[MENU_ROW_LABEL/4]==value_view) other[MENU_ROW_LABEL/4]=0;
            if(other[MENU_ROW_VALUE/4]==label_view || other[MENU_ROW_VALUE/4]==value_view) other[MENU_ROW_VALUE/4]=0;
        }
        at[MENU_ROW_LABEL/4]=label_view;
        at[MENU_ROW_VALUE/4]=value_view;
    }
    kind=at[MENU_ROW_KIND/4];
    module=at[MENU_ROW_MODULE/4];
    if(kind==MENU_ROW_ADJ) { label=text(h,TEXT_ADJ); value=no_text; }
    else if(kind==MENU_ROW_LISTED) {
        /* A loaded module shows the value of its page, as a native row shows
           its value, or Off while it is Off; another module its Enable
           value. */
        page=listed_page(h,module);
        if(page<module_pages(h)) {
            const MenuPage *m=menu_page(h,page);
            label=m->title(h);
            value=manage_off(h,module) ? on_off_text(h,1) : m->label(h,m->committed(h));
        }
        else { label=manage_name(h,module); value=on_off_text(h,manage_off(h,module)); }
    }
    else { label=text(h,TEXT_ABOUT); value=grkit_value(h); }
    descriptor(at+MENU_ROW_LABEL_TEXT_AT/4,label);
    descriptor(at+MENU_ROW_VALUE_TEXT_AT/4,value);
    MENU_VIEW3(FW_STRING_VIEW_RESET,at[MENU_ROW_LABEL/4],0,0);
    MENU_VIEW3(FW_MENU_ROW_LABEL,row,MENU_ROW_LABEL_TEXT,MENU_ROW_LABEL_TEXT);
    MENU_VIEW3(FW_STRING_VIEW_RESET,at[MENU_ROW_VALUE/4],0,0);
    MENU_VIEW2(FW_MENU_ROW_VALUE,row,MENU_ROW_VALUE_TEXT);
    fit_value(at[MENU_ROW_VALUE/4]);
}
#define MENU_ROW_FILLER(n) \
    A32_ENTRY void menu_row_fill_##n(u32 menu,u32 row) { (void)menu; menu_row_fill(n,row); }
MENU_ROW_FILLER(0) MENU_ROW_FILLER(1) MENU_ROW_FILLER(2) MENU_ROW_FILLER(3)
MENU_ROW_FILLER(4) MENU_ROW_FILLER(5) MENU_ROW_FILLER(6) MENU_ROW_FILLER(7)
MENU_ROW_FILLER(8) MENU_ROW_FILLER(9) MENU_ROW_FILLER(10) MENU_ROW_FILLER(11)
_Static_assert(MENU_ROWS_SLOTS==12,"A filler for each row");
static void (*const fillers[MENU_ROWS_SLOTS])(u32,u32)={
    menu_row_fill_0,menu_row_fill_1,menu_row_fill_2,menu_row_fill_3,
    menu_row_fill_4,menu_row_fill_5,menu_row_fill_6,menu_row_fill_7,
    menu_row_fill_8,menu_row_fill_9,menu_row_fill_10,menu_row_fill_11};
/* The text of a view of a MENU row or of the section title, or 0. */
static u32 menu_rows_text(volatile u32 *h,u32 view,u32 id) {
    u32 slot, n;
    if(!h[GRKIT_HOST_SERVICES/4]) return 0;
    n=services_word(h,SERVICES_MENU)[MENU_ROWS_COUNT/4];
    for(slot=0;slot<n && slot<MENU_ROWS_SLOTS;++slot) {
        volatile u32 *at=menu_row(h,slot);
        if(id==MENU_ROW_LABEL_TEXT && view==at[MENU_ROW_LABEL/4]) return (u32)(at+MENU_ROW_LABEL_TEXT_AT/4)|1U;
        if(id==MENU_ROW_VALUE_TEXT && view==at[MENU_ROW_VALUE/4]) return (u32)(at+MENU_ROW_VALUE_TEXT_AT/4)|1U;
    }
    if(n && id==MENU_ROW_LABEL_TEXT)
        return (u32)services_word(h,SERVICES_MENU+MENU_ROWS_TITLE_AT)|1U;
    return 0;
}
#endif /* GRKIT_MENU_SECTION_H */
