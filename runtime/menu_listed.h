#ifndef GRKIT_MENU_LISTED_H
#define GRKIT_MENU_LISTED_H
/* The settings of a module page in the GRKit section of MENU (runtime/menu.c):
   Enable, Options, and the settings of the page, and the option list. The
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
    {adjustments_label,one_value,adjustments_text,no_value,no_set},
};
/* The settings of the module page of the listed module after its Enable
   row, with menu_page selecting the page, and their count in *n; or 0. */
static const Setting *listed_base(volatile u32 *h,u32 *n) {
    const MenuPage *page;
    *n=0;
    if(listed_slot(h)>=module_pages(h)) return 0;
    page=menu_page(h,listed_slot(h));
    if(page==&tone_module) {
        /* Without a committed effect, there are no Fn adjustments. */
        *n=1;
        if(h[GRKIT_HOST_ACTIVE/4]) *n=2;
        return tone_settings;
    }
    if(page->settings) {
        *n=PAGE_WORD(CHOICE_ENTRY(h)[0],PAGE_SETTING_ROWS);
        if(*n>SETTING_ROWS-1) *n=SETTING_ROWS-1;
        return page->settings;
    }
    /* The rows of a page of actions, such as the recipes, are no setting:
       MENU shows only Enable. */
    if(page->flags&MENU_PAGE_ACTIONS || page->rows(h)>MENU_VALUES_MAX) return 0;
    *n=1;
    return choice_one;
}
/* The options of the listed module: menu_page selects its page. A page of
   CORE.TONE has the listed effects in the order of the Effects choices,
   another page its values but Off. */
static u32 option_tone(volatile u32 *h) { return menu_page(h,listed_slot(h))==&tone_module; }
/* The number of options of page `slot`, or 0 for a page without options. */
static u32 options_of(volatile u32 *h,u32 slot) {
    const MenuPage *page;
    if(slot>=module_pages(h)) return 0;
    page=menu_page(h,slot);
    if(!choice_options(h)) return 0;
    if(page==&tone_module) return effects_count(h);
    return setting_rows(h,CHOICE_ENTRY(h)[1])-1;
}
static u32 options_count(volatile u32 *h) { return options_of(h,listed_slot(h)); }
/* The effect or the value of option `k`. */
static u32 option_item(volatile u32 *h,u32 k) {
    if(option_tone(h)) return effects_at(h,k);
    return setting_row_value(h,CHOICE_ENTRY(h)[1],k+1);
}
static const u16 *option_label(volatile u32 *h,u32 k) {
    u32 item=option_item(h,k);
    return option_tone(h) ? effects_name(h,item) : choice_text(h,PAGE_LABELS,item);
}
static u32 option_off(volatile u32 *h,u32 k) {
    u32 item=option_item(h,k);
    if(option_tone(h)) return effects_off(h,item);
    return item<MANAGE_OPTION_VALUES && manage_hidden_values(h,listed(h))>>item&1;
}
/* Off of the value that the setting has sets the setting to Off. */
static void option_set(volatile u32 *h,u32 k,u32 off) {
    u32 item=option_item(h,k), handle, mask;
    if(off>=2) return;
    if(option_tone(h)) { effects_turn(h,item,off); return; }
    if(item>=MANAGE_OPTION_VALUES) return;
    handle=CHOICE_ENTRY(h)[1];
    mask=manage_hidden_values(h,listed(h));
    setting_store(h,manage_rows_handle(h,listed(h)),off ? mask|1U<<item : mask&~(1U<<item));
    if(off && setting_value(h,handle)==item) setting_store(h,handle,0);
}
/* The rows of the page of the listed module: Enable, then Options for a page
   with options, then the settings of the module page. Without room for
   each, the first settings, such as the page setting, leave. LISTED_ENABLE
   and LISTED_OPTIONS are kinds; another kind n is setting n of the page. */
#define LISTED_ENABLE 0x100U
#define LISTED_OPTIONS 0x101U
#define LISTED_NONE 0x102U
/* The entry of value `k` of a long value page (MENU_ROWS_VALUES). */
static volatile u32 *value_entry(volatile u32 *h,u32 k) {
    return services_word(h,SERVICES_MENU+MENU_ROWS_VALUES+k*MENU_VALUE_SIZE);
}
/* The text of option `k` in its entry: its label, then Off for an option
   that is off, in parentheses: "16:9 (Off)", "16:9 (끔)", "16:9（关闭）". */
static const u16 *entry_text(volatile u32 *h,u32 k,const u16 *label,u32 off_text) {
    volatile u16 *to=(volatile u16 *)(value_entry(h,k)+MENU_VALUE_TEXT/4);
    const u16 *off=text(h,TEXT_OFF);
    u32 n=0, wide=!menu_spaced();
    while(n<MENU_VALUE_UNITS-12 && label[n]) { to[n]=label[n]; ++n; }
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
    u32 n, options, first;
    *base=listed_base(h,&n);
    options=options_count(h)!=0;
    first=1+options+n>SETTING_ROWS ? 1+options+n-SETTING_ROWS : 0;
    if(!row) return LISTED_ENABLE;
    if(options && row==1) return LISTED_OPTIONS;
    row=row-1-options+first;
    return row<n ? row : LISTED_NONE;
}
static u32 listed_rows(volatile u32 *h) {
    u32 n, options;
    if(listed(h)>=manage_count(h)) return 0;
    listed_base(h,&n);
    options=options_count(h)!=0;
    return 1+options+n<SETTING_ROWS ? 1+options+n : SETTING_ROWS;
}
static const u16 *enable_label(volatile u32 *h) { return text(h,TEXT_ENABLE); }
/* The Options row shows no value. Its value page lists the options: a value
   for each, its label, with Off after it while it is off. OK turns it on or
   off, and the page stays (menu_settings_controller) with its cursor on the
   option. The native mark of the current value follows the cursor. */
#define OPTIONS_SHOWN 0xffffU
static const u16 *listed_label(volatile u32 *h,u32 row) {
    const Setting *b; u32 k=listed_kind(h,row,&b);
    return k==LISTED_ENABLE ? enable_label(h) : k==LISTED_OPTIONS ? text(h,TEXT_OPTIONS) : k<LISTED_ENABLE ? b[k].label(h) : no_value_text;
}
static u32 listed_values(volatile u32 *h,u32 row) {
    const Setting *b; u32 k=listed_kind(h,row,&b);
    return k==LISTED_ENABLE ? 2 : k==LISTED_OPTIONS ? options_count(h) : k<LISTED_ENABLE ? b[k].values(h) : 0;
}
static const u16 *listed_text(volatile u32 *h,u32 row,u32 value) {
    const Setting *b; u32 k=listed_kind(h,row,&b);
    if(k==LISTED_ENABLE) return on_off_text(h,value);
    if(k==LISTED_OPTIONS) return value==OPTIONS_SHOWN ? no_value_text : option_text(h,value);
    return k<LISTED_ENABLE ? b[k].value_text(h,value) : no_value_text;
}
static u32 listed_get(volatile u32 *h,u32 row) {
    const Setting *b; u32 k=listed_kind(h,row,&b);
    return k==LISTED_ENABLE ? manage_off(h,listed(h)) : k==LISTED_OPTIONS ? OPTIONS_SHOWN : k<LISTED_ENABLE ? b[k].get(h) : 0;
}
static void listed_set(volatile u32 *h,u32 row,u32 value) {
    const Setting *b; u32 k=listed_kind(h,row,&b);
    if(k==LISTED_ENABLE) { setting_store(h,manage_off_handle(h,listed(h)),value); manage_suspend(h); }
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
LISTED_ROW(0) LISTED_ROW(1) LISTED_ROW(2) LISTED_ROW(3) LISTED_ROW(4) LISTED_ROW(5)
static const Setting listed_settings[SETTING_ROWS]={
    {listed_label_0,listed_values_0,listed_text_0,listed_get_0,listed_set_0},
    {listed_label_1,listed_values_1,listed_text_1,listed_get_1,listed_set_1},
    {listed_label_2,listed_values_2,listed_text_2,listed_get_2,listed_set_2},
    {listed_label_3,listed_values_3,listed_text_3,listed_get_3,listed_set_3},
    {listed_label_4,listed_values_4,listed_text_4,listed_get_4,listed_set_4},
    {listed_label_5,listed_values_5,listed_text_5,listed_get_5,listed_set_5},
};
/* Whether row `row` of a page opens the Fn adjustments instead of a value
   page. */
static u32 settings_fn_row(volatile u32 *h,const Setting *page,u32 row) {
    const Setting *b;
    return page==listed_settings && listed_kind(h,row,&b)==1 && b==tone_settings;
}
#endif /* GRKIT_MENU_LISTED_H */
