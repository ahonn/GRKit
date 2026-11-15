#ifndef GRKIT_MENU_LISTED_H
#define GRKIT_MENU_LISTED_H
/* The settings of a module page in the GRKit section of MENU (runtime/menu.c):
   Enable, Options, Order, and the settings of the page, and the option list. The
   settings page code (runtime/menu_settings.h) reads them, so the includer
   includes this file inside that one. */
/* The page of a listed module in the GRKit section of MENU
   (menu_rows_install, LISTED_PAGE), which
   shows an Enable row, then the settings of its module page. SERVICES_MENU +
   MENU_ROWS_LISTED holds the listed module, and MENU_ROWS_PAGE its page
   slot, or GRKIT_MODULE_PAGES_MAX for a module without a loaded page. */
#define LISTED_PAGE (MODULE_TOTAL+1U)

/* The row of the Fn adjustments in the page of Image Control targets this
   ID, which no controller has (menu_rows_controller). */
#define MENU_FN_TARGET 0x2000f40U
static const u16 *on_off_text(volatile u32 *h,u32 value) { return text(h,value ? TEXT_OFF : TEXT_ON); }
static u32 listed(volatile u32 *h) { return services_word(h,SERVICES_MENU+MENU_ROWS_LISTED)[0]; }
static u32 listed_slot(volatile u32 *h) { return services_word(h,SERVICES_MENU+MENU_ROWS_PAGE)[0]; }
/* The page slot of listed module `module`, or GRKIT_MODULE_PAGES_MAX. */
static u32 listed_page(volatile u32 *h,u32 module) {
    u32 slot;
    for(slot=0;slot<module_pages(h);++slot) if(page_listed(h,slot)==module) return slot;
    return GRKIT_MODULE_PAGES_MAX;
}
/* The Fn adjustments of the committed effect: the row shows the effect. */
static u32 one_value(volatile u32 *h) { (void)h; return 1; }
static u32 no_value(volatile u32 *h) { (void)h; return 0; }
static void no_set(volatile u32 *h,u32 value) { (void)h; (void)value; }
static const u16 *adjustments_label(volatile u32 *h) { return text(h,TEXT_ADJUSTMENTS); }
static const u16 no_value_text[1]={0};
/* The rows of a page of CORE.TONE: Film Stock, the effect, whose value page
   lists Off and the shown effects in their ADJ order, then Parameters, the
   Fn adjustments of the committed effect, which open the native parameter
   page and show no value. */
static const u16 *film_label(volatile u32 *h) { return text(h,TEXT_FILM); }
static const u16 *film_text(volatile u32 *h,u32 value) { return tone_label(h,value); }
static void film_set(volatile u32 *h,u32 value) { tone_apply(h,value); }
static const u16 *adjustments_text(volatile u32 *h,u32 value) { (void)h; (void)value; return no_value_text; }
static const Setting tone_settings[2]={
    {film_label,tone_rows,film_text,tone_committed,film_set},
    {adjustments_label,one_value,adjustments_text,no_value,no_set,0,SETTING_FN},
};
static const Setting *tone_listed(volatile u32 *h,u32 *n) {
    /* Without a committed effect, there are no Fn adjustments. */
    *n=h[GRKIT_HOST_ACTIVE/4] ? 2U : 1U;
    return tone_settings;
}
/* The options of a page of CORE.TONE are the listed effects in the order of
   the Effects choices. */
static const u16 *tone_option_label(volatile u32 *h,u32 k) { return effects_name(h,effects_at(h,k)); }
static u32 tone_option_off(volatile u32 *h,u32 k) { return effects_off(h,effects_at(h,k)); }
static void tone_option_set(volatile u32 *h,u32 k,u32 off) { effects_turn(h,effects_at(h,k),off); }
static const MenuListing tone_listing={tone_listed,0,effects_count,tone_option_label,tone_option_off,tone_option_set,
    effects_move};
/* Group rows are links that show the value of the child's first setting.
   The controller dispatcher opens the child's native settings page. */
static const u16 *group_value(volatile u32 *h,u32 row) {
    u32 child=group_child_address(h,row), value=setting_value(h,group_setting(h,row));
    return MENU_TEXT(PAGE_WORD(PAGE_WORD(PAGE_WORD(child,PAGE_SETTING_LIST),ROW_VALUES),
                               (value*GRKIT_TEXT_LANGUAGES+menu_language())*4));
}
#define GROUP_SETTING(n) \
    static const u16 *group_label_##n(volatile u32 *h) { return group_label(h,n); } \
    static const u16 *group_text_##n(volatile u32 *h,u32 value) { (void)value; return group_value(h,n); }
GROUP_SETTING(0) GROUP_SETTING(1) GROUP_SETTING(2) GROUP_SETTING(3)
static const Setting group_settings[SETTING_ROWS]={
    {group_label_0,one_value,group_text_0,no_value,no_set},
    {group_label_1,one_value,group_text_1,no_value,no_set},
    {group_label_2,one_value,group_text_2,no_value,no_set},
    {group_label_3,one_value,group_text_3,no_value,no_set},
};
static const Setting *group_listed(volatile u32 *h,u32 *n) { *n=group_rows(h); return group_settings; }
/* A group shows the children that are on. */
static const MenuListing group_listing={group_listed,group_value_text};
/* The MENU parts of the listed page, or 0. menu_page selects the page. */
static const MenuListing *listed_menu(volatile u32 *h) { return menu_page(h,listed_slot(h))->menu; }
/* The settings of the module page of the listed module after its Enable
   row, with menu_page selecting the page, and their count in *n; or 0. */
static const Setting *listed_base(volatile u32 *h,u32 *n) {
    const MenuPage *page;
    *n=0;
    if(listed_slot(h)>=module_pages(h)) return 0;
    page=menu_page(h,listed_slot(h));
    if(page->menu && page->menu->settings) return page->menu->settings(h,n);
    if(page->settings) {
        *n=PAGE_WORD(CHOICE_ENTRY(h)[0],PAGE_SETTING_ROWS);
        if(*n>SETTING_ROWS-1) *n=SETTING_ROWS-1;
        return page->settings;
    }
    /* The rows of another page of actions are no setting: MENU shows only
       Enable. */
    if(page->flags&MENU_PAGE_ACTIONS || page->rows(h)>MENU_VALUES_MAX) return 0;
    *n=1;
    return choice_one;
}
/* The options of the listed module: menu_page selects its page. A page
   gives its options (MenuListing), and the page of another module setting
   has its values but Off. */
/* The number of options of page `slot`, or 0 for a page without options. */
static u32 options_of(volatile u32 *h,u32 slot) {
    const MenuPage *page;
    if(slot>=module_pages(h)) return 0;
    page=menu_page(h,slot);
    if(!choice_options(h)) return 0;
    if(page->menu && page->menu->options) return page->menu->options(h);
    return choice_setting_rows(h)-1;
}
static u32 options_count(volatile u32 *h) { return options_of(h,listed_slot(h)); }
/* The value of option `k` of a module setting. */
static u32 option_item(volatile u32 *h,u32 k) { return choice_setting_row_value(h,k+1); }
static const u16 *option_label(volatile u32 *h,u32 k) {
    const MenuListing *menu=listed_menu(h);
    if(menu && menu->options) return menu->option_label(h,k);
    return choice_text(h,PAGE_LABELS,option_item(h,k));
}
static u32 option_off(volatile u32 *h,u32 k) {
    const MenuListing *menu=listed_menu(h);
    u32 item;
    if(menu && menu->options) return menu->option_off(h,k);
    item=option_item(h,k);
    return item<MANAGE_OPTION_VALUES && manage_hidden_values(h,listed(h))>>item&1;
}
/* Off of the value that the setting has sets the setting to Off. */
static void option_set(volatile u32 *h,u32 k,u32 off) {
    const MenuListing *menu=listed_menu(h);
    u32 item, handle, mask;
    if(off>=2) return;
    if(menu && menu->options) { menu->option_set(h,k,off); return; }
    item=option_item(h,k);
    if(item>=MANAGE_OPTION_VALUES) return;
    handle=CHOICE_ENTRY(h)[1];
    mask=manage_hidden_values(h,listed(h));
    setting_store(h,manage_rows_handle(h,listed(h)),off ? mask|1U<<item : mask&~(1U<<item));
    if(off && setting_value(h,handle)==item) setting_store(h,handle,0);
}
/* The options that the Order row orders, or 0 for a page without the row:
   the options of a page that can move them (MenuListing::option_move), or
   those of a module setting whose values have an order (choice_orders),
   while the settings table keeps it (runtime/manage.h: MANAGE_ORDERS). One
   option has no order. */
static u32 order_count(volatile u32 *h) {
    const MenuListing *menu;
    u32 n;
    if(listed_slot(h)>=module_pages(h)) return 0;
    menu=listed_menu(h);
    if(!choice_options(h)) return 0;
    if(menu && menu->options) n=menu->option_move ? menu->options(h) : 0;
    else n=choice_orders(h) && manage_word(h,MANAGE_ORDERS) ? choice_setting_rows(h)-1 : 0;
    return n>=2 && n<=MENU_VALUES_MAX ? n : 0;
}
/* Move option `from` to the place of option `to`: the options between them
   move by one place. The options of a module setting get their places as
   the nibbles of CORE.ORDER of the module (runtime/manage.h). */
static void option_move(volatile u32 *h,u32 from,u32 to) {
    const MenuListing *menu=listed_menu(h);
    u8 items[MANAGE_ORDER_VALUES+1U];
    u32 n=order_count(h), words[MANAGE_ORDER_WORDS], i, item;
    if(from>=n || to>=n || from==to) return;
    if(menu && menu->options) { menu->option_move(h,from,to); return; }
    if(n>MANAGE_ORDER_VALUES+1U) return;
    for(i=0;i<n;++i) {
        item=option_item(h,i);
        if(!item || item>15U) return;
        items[i]=(u8)item;
    }
    item=items[from];
    for(i=from;i<to;++i) items[i]=items[i+1];
    for(i=from;i>to;--i) items[i]=items[i-1];
    items[to]=(u8)item;
    for(i=0;i<MANAGE_ORDER_WORDS;++i) words[i]=0;
    /* The last of the most options has no nibble: it follows the others. */
    for(i=0;i<n && i<MANAGE_ORDER_VALUES;++i) {
        u32 word=i>=MANAGE_ORDER_NIBBLES;
        words[word]|=(u32)items[i]<<(4U*(i-MANAGE_ORDER_NIBBLES*word));
    }
    for(i=0;i<MANAGE_ORDER_WORDS;++i) setting_store(h,manage_order_handle(h,listed(h),i)-1U,words[i]);
}
/* The rows of the page of the listed module: Enable, then Options for a page
   with options, then Order for a page that orders them, then the settings
   of the module page. Without room for each, the first settings, such as
   the page setting, leave. LISTED_ENABLE, LISTED_OPTIONS, and LISTED_ORDER
   are kinds; another kind n is setting n of the page. */
#define LISTED_ENABLE 0x100U
#define LISTED_OPTIONS 0x101U
#define LISTED_NONE 0x102U
#define LISTED_ORDER 0x103U
/* The Order row takes a row of its own, so the page shows as many settings
   with it as without it. */
#define LISTED_ROWS (SETTING_ROWS+1U)
_Static_assert(LISTED_ROWS<=SUBMENU_ROWS,"The submenu shows each row of the listed page");
/* The entry of value `k` of a long value page (MENU_ROWS_VALUES). */
static volatile u32 *value_entry(volatile u32 *h,u32 k) {
    return services_word(h,SERVICES_MENU+MENU_ROWS_VALUES+k*MENU_VALUE_SIZE);
}
/* The text of option `k` in its entry: its label, then Off for an option
   that is off, in parentheses: "16:9 (Off)", "16:9 (끔)", "16:9（关闭）".
   Only a label with Off leaves room for it. */
static const u16 *entry_text(volatile u32 *h,u32 k,const u16 *label,u32 off_text) {
    volatile u16 *to=(volatile u16 *)(value_entry(h,k)+MENU_VALUE_TEXT/4);
    const u16 *off=text(h,TEXT_OFF);
    u32 n=0, wide=!menu_spaced(), most=off_text ? MENU_VALUE_UNITS-12 : MENU_VALUE_UNITS-1;
    while(n<most && label[n]) { to[n]=label[n]; ++n; }
    if(off_text) {
        if(!wide) to[n++]=' ';
        to[n++]=wide ? 0xff08 : '(';
        while(n<MENU_VALUE_UNITS-2 && *off) to[n++]=*off++;
        to[n++]=wide ? 0xff09 : ')';
    }
    to[n]=0;
    return (const u16 *)to;
}
static const u16 *option_text(volatile u32 *h,u32 k) { return entry_text(h,k,option_label(h,k),option_off(h,k)); }
static u32 settings_draw(volatile u32 *h,u32 row,u32 value,u32 setting);
/* Draw the value row of option `k` again after OK, as the page does not:
   settings_draw notes the row of each option. */
static void option_redraw(volatile u32 *h,u32 setting,u32 k) {
    if(k<MENU_VALUES_MAX && value_entry(h,k)[MENU_VALUE_ROW/4])
        settings_draw(h,value_entry(h,k)[MENU_VALUE_ROW/4],k,setting);
}
static u32 listed_kind(volatile u32 *h,u32 row,const Setting **base) {
    u32 n, options, order, most, first;
    *base=listed_base(h,&n);
    options=options_count(h)!=0;
    order=order_count(h)!=0;
    most=SETTING_ROWS+order;
    first=1+options+order+n>most ? 1+options+order+n-most : 0;
    if(!row) return LISTED_ENABLE;
    if(options && row==1) return LISTED_OPTIONS;
    if(order && row==1+options) return LISTED_ORDER;
    row=row-1-options-order+first;
    return row<n ? row : LISTED_NONE;
}
static u32 listed_child(volatile u32 *h,u32 row) {
    const Setting *base;
    u32 kind=listed_kind(h,row,&base);
    return base==group_settings && kind<LISTED_ENABLE ? group_child(h,kind) : MODULE_TOTAL;
}
static u32 listed_rows(volatile u32 *h) {
    u32 n, options, order;
    if(listed(h)>=manage_count(h)) return 0;
    listed_base(h,&n);
    options=options_count(h)!=0;
    order=order_count(h)!=0;
    return 1+options+order+n<SETTING_ROWS+order ? 1+options+order+n : SETTING_ROWS+order;
}
static const u16 *enable_label(volatile u32 *h) { return text(h,TEXT_ENABLE); }
/* The Options row shows no value. Its value page lists the options: a value
   for each, its label, with Off after it while it is off. OK turns it on or
   off, and the page stays (menu_settings_controller) with its cursor on the
   option. The native mark of the current value follows the cursor. */
#define OPTIONS_SHOWN 0xffffU
/* The Order row shows no value. Its own value page lists the options in
   their order, each after its place, as "2. 16:9". OK on an option shows
   the places in place of the list, 1 to the number of options, with the
   name of the option as the title and the mark on its place. OK on a place
   moves the option there, and the list shows again with the cursor on the
   option; MENU there shows the list again without a move. The order
   applies at once. MENU_ROWS_PAGE_STATE holds ORDER_PICKED with the option
   whose places show, else the option + 1 where the list puts its cursor,
   or 0. */
#define ORDER_PICKED 0x10000U
static const u16 *page_title_with(volatile u32 *h,u32 word);
static void settings_value_again(volatile u32 *h,u32 back);
static volatile u32 *order_state(volatile u32 *h) { return services_word(h,SERVICES_MENU+MENU_ROWS_PAGE_STATE); }
static u32 order_picked(volatile u32 *h) {
    u32 state=order_state(h)[0];
    return state&ORDER_PICKED ? state&0xffffU : OPTIONS_SHOWN;
}
/* Write the number `n` below 100 at `to`: the number of units. A multiply
   and a shift divide by 10, exactly below 1029, as the core has no division
   routine (__aeabi_uidiv). */
static u32 order_number(volatile u16 *to,u32 n) {
    u32 tens=n*205U>>11, at=0;
    if(tens) to[at++]=(u16)('0'+tens);
    to[at++]=(u16)('0'+n-10U*tens);
    return at;
}
_Static_assert(MENU_VALUES_MAX<100U,"A place has two digits at most");
static const u16 *order_label(volatile u32 *h) { return text(h,TEXT_ORDER); }
/* The text of a value in its entry of the value page, which the page then
   shows from there (entry_text). */
static const u16 *order_text(volatile u32 *h,u32 value) {
    volatile u16 *to;
    const u16 *label;
    u32 n;
    if(value>=MENU_VALUES_MAX || !h[GRKIT_HOST_SERVICES/4]) return no_value_text;
    to=(volatile u16 *)(value_entry(h,value)+MENU_VALUE_TEXT/4);
    n=order_number(to,value+1U);
    if(order_picked(h)==OPTIONS_SHOWN) {
        label=option_label(h,value);
        to[n++]='.';
        to[n++]=' ';
        while(n<MENU_VALUE_UNITS-1U && *label) to[n++]=*label++;
    }
    to[n]=0;
    return (const u16 *)to;
}
static u32 order_get(volatile u32 *h) { return order_picked(h); }
static u32 order_cursor(volatile u32 *h) {
    u32 state=order_state(h)[0];
    return state&ORDER_PICKED ? state&0xffffU : state ? state-1U : 0;
}
/* The list has the title of the module, then Order, as "Framing Order". The
   places have the name of their option, which the title text holds since
   OK on the option. */
static const u16 *order_title(volatile u32 *h) {
    if(order_picked(h)!=OPTIONS_SHOWN)
        return (const u16 *)services_word(h,SERVICES_MENU+MENU_ROWS_PAGE_TITLE);
    return page_title_with(h,TEXT_ORDER);
}
static u32 order_select(volatile u32 *h,u32 controller,u32 value) {
    u32 picked=order_picked(h), n=order_count(h), i;
    (void)controller;
    if(picked==OPTIONS_SHOWN) {
        volatile u16 *to=(volatile u16 *)services_word(h,SERVICES_MENU+MENU_ROWS_PAGE_TITLE);
        const u16 *label;
        /* MENU on the list returns to the settings page. */
        if(value>=n) return 0;
        label=option_label(h,value);
        for(i=0;i<MENU_PAGE_TITLE_UNITS-1U && label[i];++i) to[i]=label[i];
        to[i]=0;
        order_state(h)[0]=ORDER_PICKED|value;
    }
    else {
        if(value<n) { option_move(h,picked,value); picked=value; }
        order_state(h)[0]=picked+1U;
    }
    settings_value_again(h,0);
    return 1;
}
static const Setting order_setting={order_label,order_count,order_text,order_get,no_set,order_select,
    SETTING_OWN_PAGE,order_cursor,order_title};
static const u16 *listed_label(volatile u32 *h,u32 row) {
    const Setting *b; u32 k=listed_kind(h,row,&b);
    if(k==LISTED_ORDER) return order_label(h);
    return k==LISTED_ENABLE ? enable_label(h) : k==LISTED_OPTIONS ? text(h,TEXT_OPTIONS) : k<LISTED_ENABLE ? b[k].label(h) : no_value_text;
}
static u32 listed_values(volatile u32 *h,u32 row) {
    const Setting *b; u32 k=listed_kind(h,row,&b);
    if(k==LISTED_ORDER) return order_count(h);
    return k==LISTED_ENABLE ? 2 : k==LISTED_OPTIONS ? options_count(h) : k<LISTED_ENABLE ? b[k].values(h) : 0;
}
static const u16 *listed_text(volatile u32 *h,u32 row,u32 value) {
    const Setting *b; u32 k=listed_kind(h,row,&b);
    if(k==LISTED_ENABLE) return on_off_text(h,value);
    if(k==LISTED_OPTIONS) return value==OPTIONS_SHOWN ? no_value_text : option_text(h,value);
    if(k==LISTED_ORDER) return order_text(h,value);
    return k<LISTED_ENABLE ? b[k].value_text(h,value) : no_value_text;
}
static u32 listed_get(volatile u32 *h,u32 row) {
    const Setting *b; u32 k=listed_kind(h,row,&b);
    if(k==LISTED_ORDER) return order_get(h);
    return k==LISTED_ENABLE ? manage_off(h,listed(h)) : k==LISTED_OPTIONS ? OPTIONS_SHOWN : k<LISTED_ENABLE ? b[k].get(h) : 0;
}
static void listed_set(volatile u32 *h,u32 row,u32 value) {
    const Setting *b; u32 k=listed_kind(h,row,&b);
    if(k==LISTED_ENABLE) { setting_store(h,manage_off_handle(h,listed(h)),value); manage_off_changed(h); }
    else if(k==LISTED_OPTIONS) {
        if(value<options_count(h)) option_set(h,value,!option_off(h,value));
        option_redraw(h,row,value);
        /* The value page stays open (menu_settings_controller). */
        services_word(h,SERVICES_MENU+MENU_ROWS_STAY)[0]=1;
        services_word(h,SERVICES_MENU+MENU_ROWS_STAY_ROW)[0]=value;
    }
    else if(k<LISTED_ENABLE) b[k].set(h,value);
}
#define LISTED_ROW(n) \
    static const u16 *listed_label_##n(volatile u32 *h) { return listed_label(h,n); } \
    static u32 listed_values_##n(volatile u32 *h) { return listed_values(h,n); } \
    static const u16 *listed_text_##n(volatile u32 *h,u32 value) { return listed_text(h,n,value); } \
    static u32 listed_get_##n(volatile u32 *h) { return listed_get(h,n); } \
    static void listed_set_##n(volatile u32 *h,u32 value) { listed_set(h,n,value); }
LISTED_ROW(0) LISTED_ROW(1) LISTED_ROW(2) LISTED_ROW(3) LISTED_ROW(4) LISTED_ROW(5) LISTED_ROW(6)
static const Setting listed_settings[LISTED_ROWS]={
    {listed_label_0,listed_values_0,listed_text_0,listed_get_0,listed_set_0},
    {listed_label_1,listed_values_1,listed_text_1,listed_get_1,listed_set_1},
    {listed_label_2,listed_values_2,listed_text_2,listed_get_2,listed_set_2},
    {listed_label_3,listed_values_3,listed_text_3,listed_get_3,listed_set_3},
    {listed_label_4,listed_values_4,listed_text_4,listed_get_4,listed_set_4},
    {listed_label_5,listed_values_5,listed_text_5,listed_get_5,listed_set_5},
    {listed_label_6,listed_values_6,listed_text_6,listed_get_6,listed_set_6},
};
/* Whether row `row` of a page opens the Fn adjustments instead of a value
   page. */
static u32 settings_fn_row(volatile u32 *h,const Setting *page,u32 row) {
    const Setting *b;
    u32 k;
    if(page!=listed_settings) return 0;
    k=listed_kind(h,row,&b);
    return k<LISTED_ENABLE && b && b[k].flags&SETTING_FN;
}
/* The setting that row `row` of the listed page shows, or 0 for Enable,
   Options, and a row past the settings. */
static const Setting *listed_setting(volatile u32 *h,u32 row) {
    const Setting *b;
    u32 k=listed_kind(h,row,&b);
    if(k==LISTED_ORDER) return &order_setting;
    return k<LISTED_ENABLE && b ? b+k : 0;
}
#endif /* GRKIT_MENU_LISTED_H */
