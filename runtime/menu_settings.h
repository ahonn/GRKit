#ifndef GRKIT_MENU_SETTINGS_H
#define GRKIT_MENU_SETTINGS_H
/* The native settings page: a borrowed MENU submenu and its value pages,
   which show the settings of a module page (runtime/menu.c). */
/* The native settings page of a root row (MenuPage::settings). GRKit opens the
   MENU submenu SETTINGS_SUBMENU from ADJ, as the Fn page opens. While
   GRKIT_HOST_SETTINGS holds the module + 1, the submenu reads a private copy
   of its kind table entry with the rows of the module (1 to SUBMENU_ROWS),
   and the value pages of the rows show and set the settings of the module.
   Row i opens the borrowed value page SETTINGS_PAGES[i], whose title is the
   row label SETTINGS_ROW_TEXTS[i]. The text hooks show the
   module title, the row labels, and the values. A value view, and the label
   of a row past the labels of the entry, shows the text ID
   SETTINGS_TITLE_TEXT, so that the text hooks find the view.

   The settings block (GRKIT_TEXT_SETTINGS): the entry and the value methods
   of its rows, the descriptors of the title and of the row labels, then for
   each view the view and its descriptor: the submenu value of each row, the
   value page rows, then the labels past those of the entry.

   The return to ADJ is a controller change, which ends the page. ADJ
   refreshes also while GRKit opens the page (leave), so its refresh must not
   end the page. */
#ifndef SETTINGS_MAP
#define SETTINGS_MAP(address) ((volatile u32 *)MENU_TEXT(address))
#endif
/* The rows whose labels the kind table entry has room for. */
#define SETTINGS_LABEL_ROWS ((SUBMENU_KIND_CONTROLLERS-SUBMENU_KIND_LABELS)/2U)
#define SETTINGS_LABELS ((SUBMENU_KIND_VALUES+8*SUBMENU_ROWS+7)&~7U)
#define SETTINGS_VIEWS (SETTINGS_LABELS+(1+SUBMENU_ROWS)*8)
/* The view slots: the value of each row, the value page rows, then the
   labels past SETTINGS_LABEL_ROWS. */
#define SETTINGS_LABEL_SLOT(row) (SUBMENU_ROWS+SETTING_VALUES+(row)-SETTINGS_LABEL_ROWS)
#define SETTINGS_VIEW_SLOTS SETTINGS_LABEL_SLOT(SUBMENU_ROWS)
_Static_assert(SUBMENU_KIND_SIZE<=SETTINGS_LABELS &&
               SETTINGS_VIEWS+SETTINGS_VIEW_SLOTS*12<=GRKIT_TEXT_PARAMETER_TITLE-GRKIT_TEXT_SETTINGS,
               "The settings page fits its block");
/* The kind table entry has room for the labels of SETTINGS_LABEL_ROWS rows
   and the controllers of SETTINGS_BORROWED rows: a later label shows in a
   view slot (menu_settings_row_label), and a later row opens the value page
   of row SETTINGS_ALIAS (menu_settings_row_controller), which then shows
   that row (SETTINGS_OPEN_ROW). The value methods of all rows follow the
   entry in the private copy. */
#define SETTINGS_BORROWED 5U
#define SETTINGS_ALIAS 0U
_Static_assert(SETTINGS_LABEL_ROWS==SETTING_ROWS &&
               SUBMENU_KIND_CONTROLLERS+4*SETTINGS_BORROWED<=SUBMENU_KIND_ROW_KINDS,
               "The entry holds the labels and controllers");
static const u32 settings_pages[SETTINGS_BORROWED]=SETTINGS_PAGES;
static const u16 settings_row_texts[SETTINGS_LABEL_ROWS]=SETTINGS_ROW_TEXTS;
/* Whether controller `current` is the borrowed submenu or one of its value
   pages, whose texts a settings page gives (menu_text_override). */
static u32 settings_shows(u32 current) {
    u32 i;
    if(current==SETTINGS_SUBMENU) return 1;
    for(i=0;i<SETTINGS_BORROWED;++i) if(current==settings_pages[i]) return 1;
    return 0;
}
static volatile u32 *settings_block(volatile u32 *h) { return SETTINGS_MAP(GRKIT_TEXT(h)+GRKIT_TEXT_SETTINGS); }
/* GRKIT_HOST_SETTINGS: the module + 1, SETTINGS_ONE for the one value page
   of a page setting (choice_one), and SETTINGS_FN while the Fn adjustments
   that a row of the page opened show (menu_settings_controller). */
#define SETTINGS_ONE 0x10000U
#define SETTINGS_FN 0x20000U
/* The row past SETTINGS_BORROWED whose value page SETTINGS_ALIAS shows, or
   0: bits 24 to 31. */
#define SETTINGS_OPEN_ROW 24U
#define SETTINGS_OPEN_ROWS 0xff000000U
#include "menu_listed.h"
/* The settings of page `module`, or 0. */
static const Setting *settings_of(volatile u32 *h,u32 module,u32 one) {
    if(module==LISTED_PAGE) return listed_settings;
    if(module>=MODULE_TOTAL) return 0;
    if(one) { menu_page(h,module); return choice_one; }
    return menu_page(h,module)->settings;
}
/* The settings of the open page, or 0. */
static const Setting *settings_page(volatile u32 *h) {
    return settings_of(h,(h[GRKIT_HOST_SETTINGS/4]&0xffffU)-1,h[GRKIT_HOST_SETTINGS/4]&SETTINGS_ONE);
}
/* The rows of the open page: the row count of the private entry. */
static u32 settings_rows(volatile u32 *h) { return ((volatile u8 *)settings_block(h))[SUBMENU_KIND_ROWS]; }
/* The settings of the open page for row `setting`, or 0 for a row that the
   page does not have: the native code also asks the value pages of rows
   past the count, and a short page has no entry for them. */
static const Setting *settings_row_page(volatile u32 *h,u32 setting) {
    const Setting *page=settings_page(h);
    return page && setting<settings_rows(h) ? page : 0;
}
static u32 menu_rows_controller(volatile u32 *h,u32 id);
static void settings_open(volatile u32 *h,u32 module,u32 one);
/* The row of the value page of borrowed page `page`: a later row after it
   opened the page of SETTINGS_ALIAS. */
static u32 settings_alias(volatile u32 *h,u32 page) {
    u32 open=h[GRKIT_HOST_SETTINGS/4]>>SETTINGS_OPEN_ROW;
    return page==SETTINGS_ALIAS && open ? open : page;
}
/* The borrowed page that shows the value page of row `row`. */
static u32 settings_borrowed(u32 row) { return row<SETTINGS_BORROWED ? row : SETTINGS_ALIAS; }
/* Whether row `setting` of the open page is long: its value page keeps each
   value in an entry (value_entry), up to MENU_VALUES_MAX values. An Options
   row is long, and a row with more values than the value slots, such as Film
   Stock. A long value page shows on an own controller with a row view for
   each value (menu_check_controller). */
_Static_assert(GRKIT_TEXT_RECORDS+1<=MENU_VALUES_MAX,"Film Stock shows Off and each effect");
static u32 settings_options(volatile u32 *h,const Setting *page,u32 setting) {
    const Setting *b;
    return page==listed_settings && listed_kind(h,setting,&b)==LISTED_OPTIONS;
}
static u32 settings_long(volatile u32 *h,const Setting *page,u32 setting) {
    return settings_options(h,page,setting) || page[setting].values(h)>SETTING_VALUES;
}
/* Each SelectMenu of the camera shares one SelectMenuController. Its
   constructor builds a radio row view for each row of the longest native
   kind (SELECT_MENU_ROWS), maybe before INSTALL. A long value page shows on
   an own controller instead: the controller factory gives the open long
   value page (CHECK_ID) the checkbox page for an options page, else the
   radio page. Each is built once, with checkbox rows (CheckBoxListLineView)
   or radio rows, and the list init of the constructor appends rows: the
   factory appends more until the page has a row view for each value. So
   each native list keeps its own rows. */
static volatile u32 *check_word(volatile u32 *h,u32 at) { return services_word(h,SERVICES_CHECK+at); }
static volatile u32 *check_list(volatile u32 *h,u32 which) { return check_word(h,CHECK_LISTS+which*8); }
static u32 check_row(u32 line) { return line && MENU_OBJECT_WORD(line,0)==CHECK_LINE_VTABLE; }
/* The row views that the value page of row `setting` has: those of the own
   page that shows it, or the rows of the shared controller. */
static u32 settings_long_rows(volatile u32 *h,u32 setting) {
    u32 which;
    if(!h[GRKIT_HOST_SERVICES/4] || !check_word(h,CHECK_ID)[0] || check_word(h,CHECK_SETTING)[0]!=setting)
        return SELECT_MENU_ROWS;
    which=settings_options(h,settings_page(h),setting) ? CHECK_PAGE_CHECKBOX : CHECK_PAGE_RADIO;
    return check_word(h,which==CHECK_PAGE_RADIO ? CHECK_RADIO : CHECK_PAGE)[0] ? check_list(h,which)[1] : SELECT_MENU_ROWS;
}
static u32 settings_values(volatile u32 *h,const Setting *page,u32 setting) {
    u32 n=page[setting].values(h), most=SETTING_VALUES;
    if(settings_long(h,page,setting)) {
        most=settings_long_rows(h,setting);
        if(most>MENU_VALUES_MAX) most=MENU_VALUES_MAX;
    }
    return n<most ? n : most;
}
static void settings_forget_all(volatile u32 *h);
/* Append rows to page `which` until it has `need` row views. */
static void check_rows(volatile u32 *h,u32 which,u32 need) {
    volatile u32 *list=check_list(h,which);
    u8 types[MENU_VALUES_MAX];
    u32 i,add;
    if(need>MENU_VALUES_MAX) need=MENU_VALUES_MAX;
    if(!list[0] || list[1]>=need) return;
    add=need-list[1];
    for(i=0;i<add;++i)
        types[i]=(u8)(which==CHECK_PAGE_RADIO ? check_word(h,CHECK_RADIO_TYPE)[0] : CHECK_LINE_TYPE);
    MENU_VIEW4(FW_LIST_INIT,list[0],(u32)types,add,check_word(h,CHECK_HEIGHT)[0]);
    list[1]=need;
}
/* The controller of `id` (menu.S: menu_check_factory): the own page of the
   open long value page, else 0 for the native one. */
A32_ENTRY u32 menu_check_controller(u32 factory,u32 id) {
    volatile u32 *h=menu_header();
    const Setting *page;
    u32 which,setting,controller;
    volatile u32 *slot;
    (void)factory;
    if(!h[GRKIT_HOST_SERVICES/4] || !id || id!=check_word(h,CHECK_ID)[0]) return 0;
    page=settings_page(h);
    setting=check_word(h,CHECK_SETTING)[0];
    if(!page || setting>=settings_rows(h)) return 0;
    which=settings_options(h,page,setting) ? CHECK_PAGE_CHECKBOX : CHECK_PAGE_RADIO;
    slot=check_word(h,which==CHECK_PAGE_RADIO ? CHECK_RADIO : CHECK_PAGE);
    controller=slot[0];
    if(!controller) {
        controller=MENU_VIEW1(FW_ALLOCATE,SELECT_MENU_SIZE);
        if(!controller) return 0;
        check_word(h,CHECK_BUILD)[0]=1+which;
        MENU_VIEW1(FW_SELECT_MENU_NEW,controller);
        check_word(h,CHECK_BUILD)[0]=0;
        slot[0]=controller;
    }
    check_rows(h,which,page[setting].values(h));
    return controller;
}
/* The rows of a SelectMenuController list: while an own page is built, its
   list, its rows, and the row height, with checkbox rows for the checkbox
   page. */
A32_ENTRY void menu_check_lines(u32 list,u32 types,u32 count,u32 height) {
    volatile u32 *h=menu_header();
    u32 i,build=h[GRKIT_HOST_SERVICES/4] ? check_word(h,CHECK_BUILD)[0] : 0;
    if(build) {
        volatile u32 *at=check_list(h,build-1);
        if(build-1==CHECK_PAGE_CHECKBOX)
            for(i=0;i<count;++i) MENU_OBJECT_BYTE(types,i)=CHECK_LINE_TYPE;
        else if(count) check_word(h,CHECK_RADIO_TYPE)[0]=MENU_OBJECT_BYTE(types,0);
        check_word(h,CHECK_HEIGHT)[0]=height;
        at[0]=list;
        at[1]=count;
    }
    MENU_VIEW4(FW_LIST_INIT,list,types,count,height);
}
/* The start shows the radio dot of the current value only: a checkbox row
   keeps its checkbox. */
A32_ENTRY void menu_check_dot(u32 line,u32 shown) {
    if(check_row(line)) return;
    MENU_VIEW2(FW_ROW_DOT,line,shown);
}
/* A controller change to `id`: returns the controller to change to. A MENU
   row of a module opens its settings page (menu_rows_controller). The page
   ends at any controller outside it. */
A32_ENTRY u32 menu_settings_controller(u32 id) {
    volatile u32 *h=menu_header();
    u32 i;
    /* OK on an option: the value page stays, so the change does not happen. */
    if(h[GRKIT_HOST_SERVICES/4] && services_word(h,SERVICES_MENU+MENU_ROWS_STAY)[0]) {
        services_word(h,SERVICES_MENU+MENU_ROWS_STAY)[0]=0;
        return 0;
    }
    id=menu_rows_controller(h,id);
    menu_parameter_controller(h,id);
    /* An own page serves the long value page that opens now. The return to
       the settings page keeps it, as the submenu finds the row to put its
       cursor on from the page that returns. Any other controller gets the
       native one. */
    if(h[GRKIT_HOST_SERVICES/4] && id!=SETTINGS_SUBMENU) {
        check_word(h,CHECK_ID)[0]=0;
        for(i=0;i<SETTINGS_BORROWED;++i)
            if(id==settings_pages[i] && settings_page(h) && settings_alias(h,i)<settings_rows(h) &&
               settings_long(h,settings_page(h),settings_alias(h,i))) {
                check_word(h,CHECK_ID)[0]=id;
                check_word(h,CHECK_SETTING)[0]=settings_alias(h,i);
            }
    }
    /* The section title of MENU needs the controller that shows. The camera
       information page shows About GRKit only after the About GRKit row. */
    if(h[GRKIT_HOST_SERVICES/4]) {
        services_word(h,SERVICES_MENU+MENU_ROWS_CURRENT)[0]=id;
        if(id!=INFO_CONTROLLER) services_word(h,SERVICES_MENU+MENU_ROWS_ABOUT)[0]=0;
    }
    if(id==SETTINGS_SUBMENU) {
        h[GRKIT_HOST_SETTINGS/4]&=~(SETTINGS_FN|SETTINGS_OPEN_ROWS);
        if(settings_page(h)) settings_forget_all(h);
        /* The page shows its rows and labels again on each start, as a value
           page can change them: Film Stock adds Parameters once an effect is
           committed, and an ADJ Order choice moves the modules. */
        if(settings_page(h))
            settings_open(h,(h[GRKIT_HOST_SETTINGS/4]&0xffffU)-1,h[GRKIT_HOST_SETTINGS/4]&SETTINGS_ONE);
        return id;
    }
    for(i=0;i<SETTINGS_BORROWED;++i) if(id==settings_pages[i]) {
        if(settings_page(h)) settings_forget_all(h);
        /* An options page starts with its cursor on the first option. */
        if(h[GRKIT_HOST_SERVICES/4] && settings_page(h) && settings_alias(h,i)<settings_rows(h) &&
           settings_long(h,settings_page(h),settings_alias(h,i)))
            services_word(h,SERVICES_MENU+MENU_ROWS_STAY_ROW)[0]=0;
        return id;
    }
    /* The Fn adjustments that a row of the page opened return to the page,
       also after a half-press, which shows the shooting screen over them. */
    if((id==IC_PARAMETERS_CONTROLLER || (id==SHOOTING_CONTROLLER && menu_parameter_session())) &&
       h[GRKIT_HOST_SETTINGS/4]&SETTINGS_FN) return id;
    h[GRKIT_HOST_SETTINGS/4]=0;
    /* A native page that GRKit opened from ADJ returns to GRKit at the next
       ADJ (RETURN_PARAMETERS). The page, the shooting screen of a half-press,
       and ADJ keep that return. Any other screen, such as playback or MENU,
       ends it, so the next ADJ opens as a native ADJ does and its Fn works. */
    if(RETURN_STATE(h[GRKIT_HOST_RETURN/4])==RETURN_PARAMETERS && id!=ADJ_CONTROLLER &&
       id!=IC_PARAMETERS_CONTROLLER && id!=SHOOTING_CONTROLLER) h[GRKIT_HOST_RETURN/4]=0;
    return id;
}
/* The kind table that the submenu reads: the private entry at the index of
   the borrowed entry, or the native table. */
A32_ENTRY u32 menu_settings_kinds(void) {
    volatile u32 *h=menu_header();
    if(!settings_page(h)) return FW_SUBMENU_KINDS;
    return (u32)settings_block(h)-SETTINGS_SUBMENU_KIND*SUBMENU_KIND_SIZE;
}
/* The title view of a submenu page, before the page sets its title
   (menu.S: menu_settings_kinds_r3_1). The title setter sets the text ID of
   its string view (SUBMENU_TITLE_STRING), which measures its text only when
   the ID changes. Each GRKit page has the title ID of the borrowed entry
   (SETTINGS_TITLE_TEXT), so a longer title was cut to the width of the title
   before it. Reset the string view, as the Fn page does with its title
   (runtime/menu.c: menu_parameter_page_kinds). */
A32_ENTRY void menu_settings_title(u32 view) {
    u32 string=view ? MENU_OBJECT_WORD(view,SUBMENU_TITLE_STRING) : 0;
    if(string) MENU_VIEW3(FW_STRING_VIEW_RESET,string,0,0);
}
/* The kind table of the submenu draw, whose row list is `list`. The firmware
   builds one submenu and keeps it until power-off. A submenu that it built
   before INSTALL has the five row views of the stock constructor, not
   SUBMENU_ROWS (menu.S: menu_settings_build). So a page shows at most the
   rows that the list has views for: the draw, Down, and OK read the row count
   of the entry, and no row past the views of the list. */
A32_ENTRY u32 menu_settings_kinds_draw(u32 list) {
    volatile u32 *h=menu_header();
    if(settings_page(h)) {
        volatile u8 *rows=(volatile u8 *)settings_block(h)+SUBMENU_KIND_ROWS;
        u32 views=list_capacity(list);
        if(*rows>views) *rows=(u8)views;
    }
    return menu_settings_kinds();
}
/* The text hook finds the text of a view by its address. The native menu
   frees its views at each page change and reuses their addresses, so a view
   is in one slot or entry at most, and a page change forgets each view. */
static void settings_forget(volatile u32 *h,u32 view) {
    volatile u32 *block=settings_block(h);
    u32 i;
    for(i=0;i<SETTINGS_VIEW_SLOTS;++i) if(block[SETTINGS_VIEWS/4+i*3]==view) block[SETTINGS_VIEWS/4+i*3]=0;
    if(h[GRKIT_HOST_SERVICES/4])
        for(i=0;i<MENU_VALUES_MAX;++i)
            if(value_entry(h,i)[MENU_VALUE_VIEW/4]==view) value_entry(h,i)[MENU_VALUE_ROW/4]=value_entry(h,i)[MENU_VALUE_VIEW/4]=0;
}
static void settings_forget_all(volatile u32 *h) {
    volatile u32 *block=settings_block(h);
    u32 i;
    for(i=0;i<SETTINGS_VIEW_SLOTS;++i) block[SETTINGS_VIEWS/4+i*3]=0;
    if(h[GRKIT_HOST_SERVICES/4])
        for(i=0;i<MENU_VALUES_MAX;++i) value_entry(h,i)[MENU_VALUE_ROW/4]=value_entry(h,i)[MENU_VALUE_VIEW/4]=0;
}
/* Show `name` in `view` of value slot `slot`. Reset the text ID first: the
   view measures its text only when its ID changes. */
static void settings_view(volatile u32 *h,u32 slot,u32 view,const u16 *name) {
    volatile u32 *at=settings_block(h)+SETTINGS_VIEWS/4+slot*3;
    settings_forget(h,view);
    at[0]=view;
    descriptor(at+1,name);
    MENU_VIEW3(FW_STRING_VIEW_RESET,view,0,0);
}
/* OK on row `row` of the submenu: the controller that the row opens, which
   the entry gives (`native`) for a row up to SETTINGS_BORROWED. A later row
   opens its Fn adjustments, or the value page of SETTINGS_ALIAS for that
   row. */
A32_ENTRY u32 menu_settings_row_controller(u32 native,u32 row) {
    volatile u32 *h=menu_header();
    const Setting *page=settings_page(h);
    if(!page) return native;
    h[GRKIT_HOST_SETTINGS/4]&=~SETTINGS_OPEN_ROWS;
    if(row<SETTINGS_BORROWED || row>=settings_rows(h)) return native;
    h[GRKIT_HOST_SETTINGS/4]|=row<<SETTINGS_OPEN_ROW;
    return settings_fn_row(h,page,row) ? MENU_FN_TARGET : settings_pages[SETTINGS_ALIAS];
}
/* The label text ID of row `row` of the submenu, whose row view is `line`:
   the entry gives it (`native`) for a row up to SETTINGS_LABEL_ROWS. Reset
   the label view of each row of the page, as its text can change while its
   ID stays: ADJ Order moves the modules. */
A32_ENTRY u32 menu_settings_row_label(u32 native,u32 line,u32 row) {
    volatile u32 *h=menu_header();
    const Setting *page=settings_page(h);
    u32 view=MENU_OBJECT_WORD(line,ROW_LABEL_VIEW);
    if(!page || row>=settings_rows(h)) return native;
    if(row<SETTINGS_LABEL_ROWS) {
        settings_forget(h,view);
        MENU_VIEW3(FW_STRING_VIEW_RESET,view,0,0);
        return native;
    }
    settings_view(h,SETTINGS_LABEL_SLOT(row),view,page[row].label(h));
    return SETTINGS_TITLE_TEXT;
}
/* The row of the open value page of the settings page, or SUBMENU_ROWS. */
static u32 settings_value_row(volatile u32 *h) {
    u32 current=services_word(h,SERVICES_MENU+MENU_ROWS_CURRENT)[0], i;
    for(i=0;i<SETTINGS_BORROWED;++i) if(current==settings_pages[i]) return settings_alias(h,i);
    return SUBMENU_ROWS;
}
/* The descriptor `n` of the texts of an options page with `name`. */
static u32 options_page_text(volatile u32 *h,u32 n,const u16 *name) {
    volatile u32 *at=services_word(h,SERVICES_MENU+MENU_ROWS_PAGE_TEXTS+n*8);
    descriptor(at,name);
    return (u32)at|1U;
}
/* The title of an options page: the page title, then Options, as
   "Framing Options", "프레이밍 옵션", or "画幅选项". */
static const u16 *options_title(volatile u32 *h) {
    volatile u16 *to=(volatile u16 *)services_word(h,SERVICES_MENU+MENU_ROWS_PAGE_TITLE);
    const u16 *title=listed_slot(h)<module_pages(h) ? menu_page(h,listed_slot(h))->title(h) : manage_name(h,listed(h));
    const u16 *options=text(h,TEXT_OPTIONS);
    u32 n=0;
    while(n<MENU_PAGE_TITLE_UNITS-20 && *title) to[n++]=*title++;
    if(menu_spaced()) to[n++]=' ';
    while(n<MENU_PAGE_TITLE_UNITS-1 && *options) to[n++]=*options++;
    to[n]=0;
    return (const u16 *)to;
}
/* The text descriptor of a settings page text, with bit 0 set (the text hooks
   then select the font of the menu language), or 0. */
static u32 settings_text(volatile u32 *h,u32 view,u32 id) {
    volatile u32 *block=settings_block(h);
    const Setting *page=settings_page(h);
    u32 i;
    if(!page) return 0;
    if(h[GRKIT_HOST_SERVICES/4]) {
        u32 row=settings_value_row(h);
        /* The values of an open long value page. */
        if(id==SETTINGS_TITLE_TEXT && row<settings_rows(h) && settings_long(h,page,row))
            for(i=0;i<MENU_VALUES_MAX;++i)
                if(value_entry(h,i)[MENU_VALUE_VIEW/4]==view) return (u32)(value_entry(h,i)+MENU_VALUE_TEXT_AT/4)|1U;
        /* An options page: its title names the module, OK turns an option
           on or off, and MENU returns with the changes. */
        if(row<settings_rows(h) && settings_options(h,page,row)) {
            if(id==settings_row_texts[settings_borrowed(row)]) return options_page_text(h,0,options_title(h));
            /* The OK guide keeps the width of the native word. */
            if(id==SELECT_GUIDE_OK_TEXT && *text(h,TEXT_GUIDE)) return options_page_text(h,1,text(h,TEXT_GUIDE));
            if(id==SELECT_GUIDE_BACK_TEXT) return options_page_text(h,2,text(h,TEXT_GUIDE+1));
        }
    }
    if(id==SETTINGS_TITLE_TEXT)
        for(i=0;i<SETTINGS_VIEW_SLOTS;++i)
            if(block[SETTINGS_VIEWS/4+i*3]==view) return (u32)(block+SETTINGS_VIEWS/4+i*3+1)|1U;
    if(id==SETTINGS_TITLE_TEXT) return (u32)(block+SETTINGS_LABELS/4)|1U;
    /* A row label, or the title of its value page. */
    for(i=0;i<SETTINGS_LABEL_ROWS && i<settings_rows(h);++i)
        if(id==settings_row_texts[i]) {
            u32 row=i<SETTINGS_BORROWED ? settings_alias(h,i) : i;
            return (u32)(block+SETTINGS_LABELS/4+(row+1)*2)|1U;
        }
    return 0;
}
/* The value method of submenu row `setting`, from the private entry. */
static void settings_row(u32 row,u32 setting) {
    volatile u32 *h=menu_header();
    const Setting *page=settings_row_page(h,setting);
    if(!page) return;
    settings_view(h,setting,MENU_OBJECT_WORD(row,ROW_VALUE_VIEW),page[setting].value_text(h,page[setting].get(h)));
    MENU_VIEW2(FW_ROW_VALUE,row,SETTINGS_TITLE_TEXT);
    fit_value(MENU_OBJECT_WORD(row,ROW_VALUE_VIEW));
}
#define SETTINGS_VALUE(n) \
    A32_ENTRY void menu_settings_value_##n(u32 submenu,u32 row) { (void)submenu; settings_row(row,n); }
SETTINGS_VALUE(0) SETTINGS_VALUE(1) SETTINGS_VALUE(2) SETTINGS_VALUE(3) SETTINGS_VALUE(4)
SETTINGS_VALUE(5) SETTINGS_VALUE(6) SETTINGS_VALUE(7) SETTINGS_VALUE(8) SETTINGS_VALUE(9)
/* The methods of the value page of borrowed page `setting`, which shows the
   value page of row settings_alias(setting). Each returns 0 for the native
   method, else its result + 1. The row count: */
A32_ENTRY u32 menu_settings_count(u32 unused,u32 unused2,u32 setting) {
    volatile u32 *h=menu_header();
    const Setting *page;
    (void)unused; (void)unused2;
    setting=settings_alias(h,setting);
    page=settings_row_page(h,setting);
    return page ? settings_values(h,page,setting)+1 : 0;
}
/* The text of value row `row` of the value page of row `setting`, which
   shows value `value`. The page draws each row at once: a long page keeps
   the view and the text of each value in its entry (value_entry), another
   page in its value slots. */
static u32 settings_draw(volatile u32 *h,u32 row,u32 value,u32 setting) {
    const Setting *page=settings_row_page(h,setting);
    if(!page) return 0;
    /* A checkbox row of an options page: the checkbox shows whether the
       option is on, and the text is its label. */
    if(settings_options(h,page,setting) && check_row(row)) {
        if(value<settings_values(h,page,setting)) {
            volatile u32 *at=value_entry(h,value);
            u32 view=MENU_OBJECT_WORD(row,CHECK_TEXT_VIEW), image=option_off(h,value) ? CHECK_IMAGE_OFF : CHECK_IMAGE_ON;
            settings_forget(h,view);
            at[MENU_VALUE_ROW/4]=row;
            at[MENU_VALUE_VIEW/4]=view;
            descriptor(at+MENU_VALUE_TEXT_AT/4,entry_text(h,value,option_label(h,value),0));
            MENU_VIEW3(FW_STRING_VIEW_RESET,view,0,0);
            MENU_VIEW4(FW_CHECK_ITEM,row,image,image,SETTINGS_TITLE_TEXT);
        }
        return 1;
    }
    if(settings_long(h,page,setting)) {
        if(value<settings_values(h,page,setting)) {
            volatile u32 *at=value_entry(h,value);
            settings_forget(h,MENU_OBJECT_WORD(row,ROW_TEXT_VIEW));
            at[MENU_VALUE_ROW/4]=row;
            at[MENU_VALUE_VIEW/4]=MENU_OBJECT_WORD(row,ROW_TEXT_VIEW);
            descriptor(at+MENU_VALUE_TEXT_AT/4,settings_options(h,page,setting) ? option_text(h,value) :
                       entry_text(h,value,page[setting].value_text(h,value),0));
            MENU_VIEW3(FW_STRING_VIEW_RESET,MENU_OBJECT_WORD(row,ROW_TEXT_VIEW),0,0);
        }
        MENU_VIEW2(FW_ROW_TEXT,row,SETTINGS_TITLE_TEXT);
        return 1;
    }
    if(value<settings_values(h,page,setting))
        settings_view(h,SUBMENU_ROWS+value,MENU_OBJECT_WORD(row,ROW_TEXT_VIEW),page[setting].value_text(h,value));
    MENU_VIEW2(FW_ROW_TEXT,row,SETTINGS_TITLE_TEXT);
    return 1;
}
A32_ENTRY u32 menu_settings_draw(u32 row,u32 value,u32 setting) {
    volatile u32 *h=menu_header();
    return settings_draw(h,row,value,settings_alias(h,setting));
}
/* OK on value `value`: */
A32_ENTRY u32 menu_settings_apply(u32 value,u32 unused,u32 setting) {
    volatile u32 *h=menu_header();
    const Setting *page;
    (void)unused;
    setting=settings_alias(h,setting);
    page=settings_row_page(h,setting);
    if(!page) return 0;
    if(value<settings_values(h,page,setting)) page[setting].set(h,value);
    return 1;
}
/* Whether value `value` is the stored value, or for an option, whether it
   is On: */
A32_ENTRY u32 menu_settings_current(u32 value,u32 unused,u32 setting) {
    volatile u32 *h=menu_header();
    const Setting *page;
    (void)unused;
    setting=settings_alias(h,setting);
    page=settings_row_page(h,setting);
    if(!page) return 0;
    if(settings_options(h,page,setting)) return (services_word(h,SERVICES_MENU+MENU_ROWS_STAY_ROW)[0]==value)+1;
    return (page[setting].get(h)==value)+1;
}
/* The settings of a page: those of a module page give their count, the
   others end at the first entry without a label. */
static u32 settings_count(volatile u32 *h,const Setting *page) {
    u32 n=0;
    if(page==choice_settings) n=PAGE_WORD(CHOICE_ENTRY(h)[0],PAGE_SETTING_ROWS);
    else if(page==manage_settings) n=manage_adj_count(h);
    else if(page==listed_settings) n=listed_rows(h);
    else while(n<SETTING_ROWS && page[n].label) ++n;
    return n<SUBMENU_ROWS ? n : SUBMENU_ROWS;
}
/* Start a page for `module`: copy the borrowed entry with the rows of the
   module, their labels, value pages, and private value methods, and fill
   the descriptors of the labels. The menu language does not change while
   the page is open. */
static void settings_open(volatile u32 *h,u32 module,u32 one) {
    static void (*const values[])(u32,u32)={menu_settings_value_0,menu_settings_value_1,
        menu_settings_value_2,menu_settings_value_3,menu_settings_value_4,menu_settings_value_5,
        menu_settings_value_6,menu_settings_value_7,menu_settings_value_8,menu_settings_value_9};
    volatile u32 *block=settings_block(h);
    const Setting *page;
    u32 stock=FW_SUBMENU_KINDS+SETTINGS_SUBMENU_KIND*SUBMENU_KIND_SIZE, i, rows;
    /* ADJ Order lists the modules that ADJ shows now. */
    if(module==MANAGE_MODULE) manage_adj_refresh(h);
    page=settings_of(h,module,one);
    rows=page ? settings_count(h,page) : 0;
    for(i=0;i<SUBMENU_KIND_SIZE/4;++i) block[i]=MENU_OBJECT_WORD(stock,i*4);
    ((volatile u8 *)block)[SUBMENU_KIND_ROWS]=(u8)rows;
    _Static_assert(sizeof(values)/sizeof(values[0])==SUBMENU_ROWS,"A value method for each row");
    for(i=0;i<rows;++i) {
        if(i<SETTINGS_LABEL_ROWS)
            ((volatile u16 *)((volatile u8 *)block+SUBMENU_KIND_LABELS))[i]=settings_row_texts[i];
        /* A later row has no room for its controller and kind
           (menu_settings_row_controller, menu.S: menu_settings_row_kind). */
        if(i<SETTINGS_BORROWED) {
            block[SUBMENU_KIND_CONTROLLERS/4+i]=settings_pages[i];
            if(settings_fn_row(h,page,i)) block[SUBMENU_KIND_CONTROLLERS/4+i]=MENU_FN_TARGET;
            /* The stock entry has three rows: the others take the kind of the first. */
            block[SUBMENU_KIND_ROW_KINDS/4+i]=block[SUBMENU_KIND_ROW_KINDS/4];
        }
        /* A C++ member pointer: the function, then the adjustment of `this`. */
        block[SUBMENU_KIND_VALUES/4+i*2]=(u32)values[i];
        block[SUBMENU_KIND_VALUES/4+i*2+1]=0;
        descriptor(block+SETTINGS_LABELS/4+(i+1)*2,page[i].label(h));
    }
    /* The settings of a module or an effect have its name. */
    if(module==LISTED_PAGE)
        descriptor(block+SETTINGS_LABELS/4,listed_slot(h)<module_pages(h) ? menu_page(h,listed_slot(h))->title(h) :
                   manage_name(h,listed(h)));
    else
    descriptor(block+SETTINGS_LABELS/4,module==MANAGE_MODULE ? manage_title(h) :
               menu_page(h,module)->title(h));
    for(i=0;i<SETTINGS_VIEW_SLOTS*3;++i) block[SETTINGS_VIEWS/4+i]=0;
    h[GRKIT_HOST_SETTINGS/4]=(module+1)|(one ? SETTINGS_ONE : 0);
}
static void settings_start(volatile u32 *h,u32 module) { settings_open(h,module,0); }
#endif /* GRKIT_MENU_SETTINGS_H */
