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

   The switch Enable GRKit, the first row of the GRKit section and a root
   row of ADJ after the module pages, stops each module at once as a soft
   Off does, and On starts again each module whose own Enable is On. Its
   setting, CORE.OFF, 1 for Off, follows the settings of the effects in the
   table. A module that is Off at a start, by its Enable or by the switch,
   loads stopped.

   The mode dial can turn GRKit off at a position (runtime/mode_dial.h): a
   flag of the session, not the switch, which stops the modules as the
   switch Off does (manage_grkit_off). The Enable row of ADJ shows whether
   GRKit runs; the MENU row shows the switch.

   Enable at Start, the row after the switch in the GRKit section (no ADJ
   row), decides the switch at each start: On, Off, or Keep Last, which
   keeps the value of the last session. Its setting, CORE.START, follows
   the switch in the table. INSTALL applies it before it selects the modules
   (runtime/install_plan.h: start_switch), so a start with Off loads each
   module stopped. A change of the switch during a session applies at once,
   and On or Off overrides it at the next start.

   Distance Unit, the second setting of the GRKit Settings row, shows the
   focus distances of the camera in meters or in feet: the Snap focus
   distances (runtime/distance.h). Its setting, CORE.UNIT, follows Enable at
   Start in the table, and a change applies at once.

   The header keeps a copy of these three settings (GRKIT_HOST_MANAGE_COPY,
   manage_copy), which the resident loader saves in the file of the user
   modes of the mode dial. On a user mode with a snapshot, the core holds
   the main state file (runtime/mode_dial.h), but not that file, so a change
   of these settings there saves. INSTALL takes them from the copy before the
   main state file (runtime/install_recipes.h: read_manage_copy).

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
/* The handle + 1 of CORE.OFF, the GRKit switch, or 0 without it. */
#define MANAGE_SWITCH 16U
/* The handle + 1 of CORE.START, Enable at Start, or 0 without it. */
#define MANAGE_START 20U
/* The handle + 1 of CORE.UNIT, Distance Unit, or 0 without it. */
#define MANAGE_UNIT 24U
#define MANAGE_HEADER 28U
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
/* A color-only module on a monochrome camera (GRM_COLOR_ONLY): MENU does not
   list it. */
#define MANAGE_COLOR_ONLY 56U
/* The most listed modules (MODULES_MAX of the installer). */
#define MANAGE_MODULES GRKIT_MODULES_MAX
/* A position without a CORE.POS value sorts after each position. */
#define MANAGE_DEFAULT_KEY 1000U
/* CORE.POS of a module: 0 for its default place in the GRKit root of ADJ,
   or its position there, 1 to MANAGE_MODULES; MANAGE_POS_HIDDEN while ADJ
   does not show it; or MANAGE_POS_PIN + k while it has entry k of the ADJ
   toolbar, before the GRKit entry (k < MANAGE_PINS, runtime/menu.c). The
   module works in each place. */
#define MANAGE_PINS (GRKIT_CORE_ADJ_VALUES-2U)
#define MANAGE_POS_HIDDEN (MANAGE_MODULES+1U)
#define MANAGE_POS_PIN (MANAGE_MODULES+2U)
#define MANAGE_POS_VALUES (MANAGE_POS_PIN+MANAGE_PINS)
/* The place of a module (manage_place): the GRKit root, hidden, or entry
   MANAGE_PLACE_PIN + k of the toolbar. */
#define MANAGE_PLACE_ROOT 0U
#define MANAGE_PLACE_HIDDEN 1U
#define MANAGE_PLACE_PIN 2U
/* The setting of the GRKit switch: the CORE.OFF of each module without the
   module ID. */
#define MANAGE_SWITCH_KEY "CORE.OFF"
/* The setting of Enable at Start and its values, in the order of its value
   page. */
#define MANAGE_START_KEY "CORE.START"
#define MANAGE_START_ON 0U
#define MANAGE_START_OFF 1U
#define MANAGE_START_KEEP 2U
#define MANAGE_START_VALUES 3U
/* The setting of Distance Unit and its values, in the order of its value
   page. */
#define MANAGE_UNIT_KEY "CORE.UNIT"
#define MANAGE_UNIT_METERS 0U
#define MANAGE_UNIT_FEET 1U
#define MANAGE_UNIT_VALUES 2U

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
/* The GRKit switch: the handle + 1 of CORE.OFF, or 0 without a module list,
   and whether it is Off. */
static u32 manage_switch(volatile u32 *h) { return h[GRKIT_HOST_MANAGE/4] ? manage_word(h,MANAGE_SWITCH) : 0; }
static u32 manage_all_off(volatile u32 *h) { u32 at=manage_switch(h); return at && setting_value(h,at-1); }
/* Whether GRKit is off: by the switch, or at this position of the mode dial
   (runtime/mode_dial.h). */
static u32 manage_grkit_off(volatile u32 *h) {
    return manage_all_off(h) || (h[GRKIT_HOST_SERVICES/4] && services_word(h,SERVICES_DIAL+DIAL_SUSPENDED)[0]);
}
/* Whether a module is Off by its Enable, by the switch, or by the dial. */
static u32 manage_halted(volatile u32 *h,u32 module) { return manage_off(h,module) || manage_grkit_off(h); }
/* The values of the page of a module that are off in ADJ, a bit each. */
static u32 manage_rows_handle(volatile u32 *h,u32 module) { return manage_off_handle(h,module)+2; }
static u32 manage_hidden_values(volatile u32 *h,u32 module) { return setting_value(h,manage_rows_handle(h,module)); }
static u32 manage_position(volatile u32 *h,u32 module) { return setting_value(h,manage_off_handle(h,module)+1); }
static u32 manage_place(volatile u32 *h,u32 module) {
    u32 position=manage_position(h,module);
    if(position>=MANAGE_POS_PIN && position<MANAGE_POS_VALUES) return MANAGE_PLACE_PIN+position-MANAGE_POS_PIN;
    return position==MANAGE_POS_HIDDEN ? MANAGE_PLACE_HIDDEN : MANAGE_PLACE_ROOT;
}
/* The sort key of a module: an entry of the toolbar first, then its
   position in the root, the default place after them, and a module that
   ADJ does not show last. */
static u32 manage_key(volatile u32 *h,u32 module) {
    u32 position=manage_position(h,module);
    if(position>=MANAGE_POS_PIN && position<MANAGE_POS_VALUES) return position-MANAGE_POS_PIN;
    if(position==MANAGE_POS_HIDDEN) return 2U*MANAGE_DEFAULT_KEY+module;
    return position ? MANAGE_PINS+position : MANAGE_DEFAULT_KEY+module;
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
    return manage_status(h,module)==MANAGE_LOADED && manage_halted(h,module);
}
/* The switch shows while a module loaded at this start, and not in safe
   mode, where no module loads. */
static u32 manage_switch_shown(volatile u32 *h) {
    u32 i;
    if(!manage_switch(h)) return 0;
    for(i=0;i<manage_count(h);++i) if(manage_status(h,i)==MANAGE_LOADED) return 1;
    return 0;
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
        if(!manage_halted(h,i)) { uses|=MANAGE_MAP(entry+MANAGE_USES)[0]; continue; }
        if(n>=SERVICES_SUSPENDED_MAX) continue;
        services_word(h,SERVICES_SUSPENDED+4+8*n)[0]=MANAGE_MAP(entry+MANAGE_CODE)[0];
        services_word(h,SERVICES_SUSPENDED+8+8*n)[0]=MANAGE_MAP(entry+MANAGE_CODE_END)[0];
        ++n;
    }
    services_word(h,SERVICES_SUSPENDED)[0]=n;
    h[GRKIT_HOST_CAPABILITIES/4]=uses;
}
/* Apply a change of an Enable or of the switch: the soft Off, then the
   capabilities that start or stop. Framing sends the current native request
   again, so the capture hook applies the crop or the full area at once, and
   Film Look sets the highlight warning again. */
static void manage_off_changed(volatile u32 *h) {
    u32 before=h[GRKIT_HOST_CAPABILITIES/4], changed;
    manage_suspend(h);
    changed=before^h[GRKIT_HOST_CAPABILITIES/4];
    if(changed&(GRKIT_CORE_ASPECT_BIT|GRKIT_CORE_ASPECT_MODE_BIT)) aspect_refresh(h);
    if(changed&GRKIT_CORE_TONE_BIT) tone_stored(h);
}
/* Keep the switch, Enable at Start, and Distance Unit in their copy, each
   as its value + 1, after a change of one of them. A copy that changes
   counts as a change of the file of the user modes, which the loader saves
   also while the core holds the main state file. Off of the mode dial is not
   the switch and changes no copy. */
_Static_assert(MANAGE_START==MANAGE_SWITCH+4U*GRKIT_MANAGE_COPY_START &&
               MANAGE_UNIT==MANAGE_SWITCH+4U*GRKIT_MANAGE_COPY_UNIT && GRKIT_MANAGE_COPY_SWITCH==0U &&
               GRKIT_MANAGE_COPY_WORDS==3U, "The copy follows the handles of the module list");
static void manage_copy(volatile u32 *h) {
    u32 i, at, value, changed=0;
    if(!h[GRKIT_HOST_MANAGE/4]) return;
    for(i=0;i<GRKIT_MANAGE_COPY_WORDS;++i) {
        at=manage_word(h,MANAGE_SWITCH+4U*i);
        if(!at) continue;
        value=setting_value(h,at-1)+1U;
        if(h[GRKIT_HOST_MANAGE_COPY/4+i]==value) continue;
        h[GRKIT_HOST_MANAGE_COPY/4+i]=value;
        changed=1;
    }
    if(changed) ++h[GRKIT_HOST_DIAL_SET_CHANGES/4];
}
/* The mode dial follows a change of the switch (runtime/mode_dial.h). */
static void dial_master_changed(volatile u32 *h);
static void manage_set_all_off(volatile u32 *h,u32 off) {
    u32 at=manage_switch(h), was;
    if(!at) return;
    was=manage_all_off(h);
    setting_store(h,at-1,off);
    manage_copy(h);
    manage_off_changed(h);
    if(was!=manage_all_off(h)) dial_master_changed(h);
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

/* The ADJ Order row of GRKit Settings in MENU (runtime/menu_settings.h:
   CORE_PAGE) opens a native settings page with a row for each module that
   ADJ can show (manage_adj_modules), in the order of their keys: the
   entries of the toolbar, the GRKit root, then the modules that ADJ does
   not show. The value page of a row lists the places in that order: Top 1
   to Top MANAGE_PINS, the positions 1 to n of the root, and Hidden; the row
   shows the place of its module. A choice moves the module there. A module
   that had that entry of the toolbar takes the place that the moved module
   left. Each module of the page then gets its place as CORE.POS, so the page,
   the toolbar, and the root of ADJ (runtime/modules.h: root_order) show the
   change at once. The other modules keep their CORE.POS. */
_Static_assert(MANAGE_MODULES<=SUBMENU_ROWS,"The ADJ Order page has a row for each module");
/* More places than SETTING_VALUES make a long value page
   (runtime/menu_settings.h: settings_long). */
_Static_assert(MANAGE_PINS+MANAGE_MODULES+1U<=MENU_VALUES_MAX,"A value page holds each place");
static const u16 manage_positions[][3]={{'1',0},{'2',0},{'3',0},{'4',0},{'5',0},{'6',0},{'7',0},{'8',0},{'9',0},
    {'1','0',0}};
_Static_assert(sizeof(manage_positions)/sizeof(manage_positions[0])==MANAGE_MODULES,"A text for each position");
/* The modules of the page by place: the module of each entry of the
   toolbar (or MANAGE_MODULES), the root in order, and the hidden ones. */
typedef struct {
    u32 pins[MANAGE_PINS], root[MANAGE_MODULES], hidden[MANAGE_MODULES], roots, hiddens;
} ManagePlaces;
static void manage_places(volatile u32 *h,ManagePlaces *p) {
    u32 n=manage_adj_count(h), i, module, place;
    for(i=0;i<MANAGE_PINS;++i) p->pins[i]=MANAGE_MODULES;
    p->roots=p->hiddens=0;
    for(i=0;i<n;++i) {
        module=manage_adj_module(h,i);
        place=manage_place(h,module);
        if(place>=MANAGE_PLACE_PIN && p->pins[place-MANAGE_PLACE_PIN]==MANAGE_MODULES) p->pins[place-MANAGE_PLACE_PIN]=module;
        else if(place==MANAGE_PLACE_HIDDEN) p->hidden[p->hiddens++]=module;
        else p->root[p->roots++]=module;
    }
}
/* The places of the module of row `row`: the entries of the toolbar, the
   positions of the root with room for the module, and Hidden. */
static u32 manage_root_room(volatile u32 *h,u32 row) {
    ManagePlaces p;
    manage_places(h,&p);
    return p.roots+(manage_place(h,manage_adj_module(h,row))!=MANAGE_PLACE_ROOT);
}
static u32 manage_place_values(volatile u32 *h,u32 row) { return MANAGE_PINS+manage_root_room(h,row)+1U; }
static const u16 *manage_place_text(volatile u32 *h,u32 row,u32 value) {
    u32 room;
    if(value<MANAGE_PINS) return text(h,TEXT_ADJ+1U+value);
    room=manage_root_room(h,row);
    if(value<MANAGE_PINS+room) return manage_positions[value-MANAGE_PINS<MANAGE_MODULES ? value-MANAGE_PINS : MANAGE_MODULES-1];
    return text(h,TEXT_ADJ+1U+MANAGE_PINS);
}
/* The value of the place of the module of row `row`. */
static u32 manage_place_value(volatile u32 *h,u32 row) {
    u32 module=manage_adj_module(h,row), place=manage_place(h,module), i;
    ManagePlaces p;
    if(place>=MANAGE_PLACE_PIN) return place-MANAGE_PLACE_PIN;
    manage_places(h,&p);
    /* Hidden follows each position of the root and the one for this module. */
    if(place==MANAGE_PLACE_HIDDEN) return MANAGE_PINS+p.roots+1U;
    for(i=0;i<p.roots && p.root[i]!=module;++i) {}
    return MANAGE_PINS+i;
}
/* Insert `module` at index `at` of the root. */
static void manage_root_insert(ManagePlaces *p,u32 at,u32 module) {
    u32 i;
    if(at>p->roots) at=p->roots;
    for(i=p->roots;i>at;--i) p->root[i]=p->root[i-1];
    p->root[at]=module;
    ++p->roots;
}
/* Move the module of row `row` to the place of value `value`. */
static void manage_place_set(volatile u32 *h,u32 row,u32 value) {
    u32 module=manage_adj_module(h,row), old=manage_place(h,module), i, at=0, other=MANAGE_MODULES;
    ManagePlaces p;
    if(row>=manage_adj_count(h) || value>=manage_place_values(h,row)) return;
    manage_places(h,&p);
    /* Take the module out of its place. */
    if(old>=MANAGE_PLACE_PIN) p.pins[old-MANAGE_PLACE_PIN]=MANAGE_MODULES;
    else if(old==MANAGE_PLACE_HIDDEN) {
        for(i=0;i<p.hiddens && p.hidden[i]!=module;++i) {}
        for(;i+1<p.hiddens;++i) p.hidden[i]=p.hidden[i+1];
        --p.hiddens;
    }
    else {
        for(at=0;at<p.roots && p.root[at]!=module;++at) {}
        for(i=at;i+1<p.roots;++i) p.root[i]=p.root[i+1];
        --p.roots;
    }
    if(value<MANAGE_PINS) {
        other=p.pins[value];
        p.pins[value]=module;
        /* A module that had the entry takes the place that the module left. */
        if(other<MANAGE_MODULES) {
            if(old>=MANAGE_PLACE_PIN) p.pins[old-MANAGE_PLACE_PIN]=other;
            else if(old==MANAGE_PLACE_HIDDEN) p.hidden[p.hiddens++]=other;
            else manage_root_insert(&p,at,other);
        }
    }
    else if(value<MANAGE_PINS+p.roots+1U) manage_root_insert(&p,value-MANAGE_PINS,module);
    else p.hidden[p.hiddens++]=module;
    for(i=0;i<MANAGE_PINS;++i)
        if(p.pins[i]<MANAGE_MODULES) setting_store(h,manage_off_handle(h,p.pins[i])+1,MANAGE_POS_PIN+i);
    for(i=0;i<p.roots;++i) setting_store(h,manage_off_handle(h,p.root[i])+1,i+1);
    for(i=0;i<p.hiddens;++i) setting_store(h,manage_off_handle(h,p.hidden[i])+1,MANAGE_POS_HIDDEN);
    /* The page shows again with the cursor on the module at its new row. */
    if(h[GRKIT_HOST_SERVICES/4]) services_word(h,SERVICES_MENU+MENU_ROWS_MOVED)[0]=module+1;
}
#define MANAGE_ROW(n) \
    static const u16 *manage_label_##n(volatile u32 *h) { \
        return manage_name(h,manage_adj_module(h,n)); } \
    static u32 manage_values_##n(volatile u32 *h) { return manage_place_values(h,n); } \
    static const u16 *manage_text_##n(volatile u32 *h,u32 value) { return manage_place_text(h,n,value); } \
    static u32 manage_get_##n(volatile u32 *h) { return manage_place_value(h,n); } \
    static void manage_set_##n(volatile u32 *h,u32 value) { manage_place_set(h,n,value); }
MANAGE_ROW(0) MANAGE_ROW(1) MANAGE_ROW(2) MANAGE_ROW(3) MANAGE_ROW(4) MANAGE_ROW(5) MANAGE_ROW(6) MANAGE_ROW(7)
MANAGE_ROW(8) MANAGE_ROW(9)
#define MANAGE_SETTING(n) {manage_label_##n,manage_values_##n,manage_text_##n,manage_get_##n,manage_set_##n}
static const Setting manage_settings[SUBMENU_ROWS]={
    MANAGE_SETTING(0),MANAGE_SETTING(1),MANAGE_SETTING(2),MANAGE_SETTING(3),MANAGE_SETTING(4),
    MANAGE_SETTING(5),MANAGE_SETTING(6),MANAGE_SETTING(7),MANAGE_SETTING(8),MANAGE_SETTING(9),
};
#define MANAGE_PAGE {manage_title,manage_summary,manage_rows,manage_label,manage_icon, \
    manage_checked,manage_apply,manage_committed,0,0,manage_settings}

/* The switch in ADJ: a root row after the module pages, under the GRKit
   title, so its page is Enable and its summary Enable: On or Enable: Off.
   The page has two rows, Off first as on each module page, then On: row r
   gives CORE.OFF !r. The root row shows the Modules tile, with a line
   through it while GRKit is off. The page shows whether GRKit runs, also
   where the mode dial turned it off; On there turns it on until the dial
   goes to another position, and the switch keeps its value
   (runtime/mode_dial.h: dial_suspend). */
static void dial_suspend(volatile u32 *h,u32 off);
static const u16 *switch_title(volatile u32 *h) { return text(h,TEXT_ENABLE); }
static const u16 *switch_summary(volatile u32 *h) { return text(h,TEXT_SWITCH+manage_grkit_off(h)); }
/* The MENU row: Enable GRKit. */
static const u16 *switch_menu_label(volatile u32 *h) { return text(h,TEXT_SWITCH+2); }
static u32 switch_rows(volatile u32 *h) { (void)h; return 2; }
static const u16 *switch_value_text(volatile u32 *h,u32 off) { return text(h,off ? TEXT_OFF : TEXT_ON); }
static const u16 *switch_label(volatile u32 *h,u32 row) { return switch_value_text(h,!row); }
static u32 switch_icon(volatile u32 *h,u32 row) {
    if(h[GRKIT_HOST_MENU_LEVEL/4]==MENU_ROOT) return manage_grkit_off(h) ? GRKIT_ICON_MODULES_OFF : GRKIT_ICON_MODULES;
    return row ? GRKIT_ICON_MODULES : GRKIT_ICON_OFF;
}
static u32 switch_checked(volatile u32 *h,u32 row) { return (!row)==manage_grkit_off(h); }
static void switch_apply(volatile u32 *h,u32 row) {
    manage_set_all_off(h,!row);
    if(row && h[GRKIT_HOST_SERVICES/4]) dial_suspend(h,0);
}
static u32 switch_committed(volatile u32 *h) { return !manage_grkit_off(h); }
#define SWITCH_PAGE {switch_title,switch_summary,switch_rows,switch_label,switch_icon, \
    switch_checked,switch_apply,switch_committed,0,0,0}
/* The MENU row opens the value page of this one setting at once
   (menu_rows_controller), titled Enable GRKit: On, then Off, as the Enable
   of a module. */
static const Setting switch_settings[SETTING_ROWS]={
    {switch_menu_label,switch_rows,switch_value_text,manage_all_off,manage_set_all_off},
};

/* The GRKit Settings row of MENU opens a settings page of the core, titled
   GRKit Settings (runtime/menu_settings.h: CORE_PAGE). Its first rows are
   two settings of the core. Enable at Start: On, Off, then Keep Last. On
   and Off are the texts of the switch. A choice applies at the next start;
   the switch keeps its value in this session. Distance Unit: Meters, then
   Feet, at once. */
static u32 start_handle(volatile u32 *h) { return h[GRKIT_HOST_MANAGE/4] ? manage_word(h,MANAGE_START) : 0; }
static u32 start_value(volatile u32 *h) {
    u32 at=start_handle(h);
    return at ? setting_value(h,at-1) : MANAGE_START_KEEP;
}
static void start_set(volatile u32 *h,u32 value) {
    u32 at=start_handle(h);
    if(at && value<MANAGE_START_VALUES) {
        setting_store(h,at-1,value);
        manage_copy(h);
    }
}
static const u16 *start_label(volatile u32 *h) { return text(h,TEXT_START); }
static u32 start_values(volatile u32 *h) { (void)h; return MANAGE_START_VALUES; }
static const u16 *start_value_text(volatile u32 *h,u32 value) {
    if(value==MANAGE_START_KEEP) return text(h,TEXT_START+1);
    return switch_value_text(h,value==MANAGE_START_OFF);
}
static u32 unit_handle(volatile u32 *h) { return h[GRKIT_HOST_MANAGE/4] ? manage_word(h,MANAGE_UNIT) : 0; }
/* Whether the camera shows its focus distances in feet. */
static u32 unit_feet(volatile u32 *h) {
    u32 at=unit_handle(h);
    return at && setting_value(h,at-1)==MANAGE_UNIT_FEET;
}
static u32 unit_value(volatile u32 *h) { return unit_feet(h) ? MANAGE_UNIT_FEET : MANAGE_UNIT_METERS; }
static void unit_set(volatile u32 *h,u32 value) {
    u32 at=unit_handle(h);
    if(at && value<MANAGE_UNIT_VALUES) {
        setting_store(h,at-1,value);
        manage_copy(h);
    }
}
static const u16 *unit_label(volatile u32 *h) { return text(h,TEXT_UNIT+1); }
static u32 unit_values(volatile u32 *h) { (void)h; return MANAGE_UNIT_VALUES; }
static const u16 *unit_value_text(volatile u32 *h,u32 value) { return text(h,TEXT_UNIT+2+(value==MANAGE_UNIT_FEET)); }
static const u16 *core_settings_title(volatile u32 *h) { return text(h,TEXT_UNIT); }
#define START_SETTING {start_label,start_values,start_value_text,start_value,start_set}
#define UNIT_SETTING {unit_label,unit_values,unit_value_text,unit_value,unit_set}
static const Setting core_settings[2]={START_SETTING,UNIT_SETTING};
#endif
#endif
