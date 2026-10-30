#ifndef GRKIT_MODULES_H
#define GRKIT_MODULES_H
/* The menu modules, in root order: GRKIT_MODULE_PAGES_MAX slots for the pages of
   modules (GRKIT_HOST_MODULE_PAGES), then the core Modules row
   (runtime/manage.h) and the core About row (runtime/about.h). */
#include "about.h"
#include "setting_store.h"
#include "effects.h"
#include "tone_page.h"

#define MODULE_PAGE(slot) (slot)
#define MANAGE_MODULE MODULE_PAGE(GRKIT_MODULE_PAGES_MAX)
#define ABOUT_MODULE (MANAGE_MODULE+1U)
#define MODULE_TOTAL (ABOUT_MODULE+1U)

static const MenuPage about_module = ABOUT_PAGE;

/* ---- Module pages ----------------------------------------------------------
   One generic choice page serves each module page (runtime/sdk/grkit.h).
   menu_page() selects the slot in GRKIT_HOST_MODULE_CHOICE before it returns
   the page, so get a page again after you get another one. */
#define CHOICE_ENTRY(h) ((h)+GRKIT_HOST_MODULE_PAGES/4+(h)[GRKIT_HOST_MODULE_CHOICE/4]*3)
/* GrkitPage fields, and GRKIT_CHOICE_OFF_ROW. */
#define PAGE_SETTING 8U
#define PAGE_FLAGS 12U
#define PAGE_TITLE 16U
#define PAGE_SUMMARIES 20U
#define PAGE_LABELS 24U
#define PAGE_SETTING_ROWS 32U
#define PAGE_SETTING_LIST 36U
#define CHOICE_OFF_ROW 1U
/* GrkitSettingRow fields. */
#define ROW_SETTING 4U
#define ROW_LABEL 8U
#define ROW_VALUES 12U
#define ROW_SIZE 16U
/* A word of a module page in the module code. A host test maps it. */
#ifndef PAGE_WORD
#define PAGE_WORD(p,o) WORD(p,o)
#endif
#include "manage.h"
static const MenuPage manage_module_page = MANAGE_PAGE;
static u32 module_pages(volatile u32 *h) {
    u32 n=h[GRKIT_HOST_MODULE_PAGE_COUNT/4];
    return n<=GRKIT_MODULE_PAGES_MAX ? n : 0;
}
static u32 choice_value(volatile u32 *h) { return setting_value(h,CHOICE_ENTRY(h)[1]); }
/* Whether the page has options (GRKIT_CHOICE_OPTIONS). */
#define CHOICE_OPTIONS 4U
static u32 choice_options(volatile u32 *h) { return PAGE_WORD(CHOICE_ENTRY(h)[0],PAGE_FLAGS)&CHOICE_OPTIONS; }
static u32 page_listed(volatile u32 *h,u32 slot);
/* The values of the page that are off in ADJ: a bit each (CORE.ROWS of its
   module, runtime/manage.h). Off stays on. */
static u32 choice_hidden(volatile u32 *h) {
    u32 listed;
    if(!choice_options(h)) return 0;
    listed=page_listed(h,h[GRKIT_HOST_MODULE_CHOICE/4]);
    return listed<MANAGE_MODULES ? manage_hidden_values(h,listed)&~1U : 0;
}
/* The rows of a choice page are the values that the camera offers, in the
   order of the setting (runtime/settings.h: setting_rows), without the
   values that are off in ADJ. */
static u32 choice_rows(volatile u32 *h) {
    u32 handle=CHOICE_ENTRY(h)[1], hidden=choice_hidden(h), n=setting_rows(h,handle), row, rows=0, value;
    for(row=0;row<n;++row) {
        value=setting_row_value(h,handle,row);
        rows+=value>=32 || !(hidden>>value&1);
    }
    return rows;
}
static u32 choice_row_value(volatile u32 *h,u32 row) {
    u32 handle=CHOICE_ENTRY(h)[1], hidden=choice_hidden(h), n=setting_rows(h,handle), base, value;
    for(base=0;base<n;++base) {
        value=setting_row_value(h,handle,base);
        if(value<32 && hidden>>value&1) continue;
        if(!row--) return value;
    }
    return 0;
}
/* The row of a value, or row 0 for a value that does not show. */
static u32 choice_value_row(volatile u32 *h,u32 value) {
    u32 handle=CHOICE_ENTRY(h)[1], hidden=choice_hidden(h), n=setting_rows(h,handle), base, row=0, v;
    if(value<32 && hidden>>value&1) return 0;
    for(base=0;base<n;++base) {
        v=setting_row_value(h,handle,base);
        if(v==value) return row;
        row+=v>=32 || !(hidden>>v&1);
    }
    return 0;
}
/* Text `index` of a text list of the page, in the menu language. */
static const u16 *choice_text(volatile u32 *h,u32 field,u32 index) {
    return MENU_TEXT(PAGE_WORD(PAGE_WORD(CHOICE_ENTRY(h)[0],field),(index*GRKIT_TEXT_LANGUAGES+menu_language())*4));
}
static u32 choice_tile(volatile u32 *h,u32 n) { return GRKIT_ICON_MODULE_TILE(CHOICE_ENTRY(h)[2]+n); }
static const u16 *choice_title(volatile u32 *h) { return choice_text(h,PAGE_TITLE,0); }
static const u16 *choice_summary(volatile u32 *h) { return choice_text(h,PAGE_SUMMARIES,choice_value(h)); }
static const u16 *choice_label(volatile u32 *h,u32 row) { return choice_text(h,PAGE_LABELS,choice_row_value(h,row)); }
/* The root row shows the tile of the value; a row shows the tile of its
   value, or Off for value 0. */
static u32 choice_icon(volatile u32 *h,u32 row) {
    u32 value;
    if(h[GRKIT_HOST_MENU_LEVEL/4]==MENU_ROOT) return choice_tile(h,choice_value(h));
    value=choice_row_value(h,row);
    return !value && PAGE_WORD(CHOICE_ENTRY(h)[0],PAGE_FLAGS)&CHOICE_OFF_ROW ? GRKIT_ICON_OFF : choice_tile(h,value);
}
static u32 choice_checked(volatile u32 *h,u32 row) { return choice_row_value(h,row)==choice_value(h); }
static void choice_apply(volatile u32 *h,u32 row) { setting_store(h,CHOICE_ENTRY(h)[1],choice_row_value(h,row)); }
static u32 choice_committed(volatile u32 *h) { return choice_value_row(h,choice_value(h)); }
static const MenuPage choice_module={choice_title,choice_summary,choice_rows,choice_label,choice_icon,
    choice_checked,choice_apply,choice_committed,0,0,0};

/* The rows of the native settings page of a module page. INSTALL writes
   the setting handle of each row into the row (runtime/install_modules.h). Row 0
   is the page setting, which can be a used setting such as CORE.ASPECT: its
   values are the rows of the page. The other rows are settings of the
   module. */
static u32 choice_row(volatile u32 *h,u32 row) {
    return PAGE_WORD(CHOICE_ENTRY(h)[0],PAGE_SETTING_LIST)+row*ROW_SIZE;
}
static u32 choice_row_handle(volatile u32 *h,u32 row) {
    return PAGE_WORD(choice_row(h,row),ROW_SETTING);
}
static const u16 *choice_row_text(volatile u32 *h,u32 row,u32 field,u32 index) {
    return MENU_TEXT(PAGE_WORD(PAGE_WORD(choice_row(h,row),field),(index*GRKIT_TEXT_LANGUAGES+menu_language())*4));
}
#define CHOICE_SETTING(row) \
    static const u16 *choice_label_##row(volatile u32 *h) { return choice_row_text(h,row,ROW_LABEL,0); } \
    static u32 choice_values_##row(volatile u32 *h) { return setting_limit(h,choice_row_handle(h,row)); } \
    static const u16 *choice_value_##row(volatile u32 *h,u32 value) { return choice_row_text(h,row,ROW_VALUES,value); } \
    static u32 choice_get_##row(volatile u32 *h) { return setting_value(h,choice_row_handle(h,row)); } \
    static void choice_set_##row(volatile u32 *h,u32 value) { setting_store(h,choice_row_handle(h,row),value); }
static const u16 *choice_label_0(volatile u32 *h) { return choice_row_text(h,0,ROW_LABEL,0); }
static const u16 *choice_value_0(volatile u32 *h,u32 row) {
    return choice_row_text(h,0,ROW_VALUES,choice_row_value(h,row));
}
CHOICE_SETTING(1)
CHOICE_SETTING(2)
CHOICE_SETTING(3)
CHOICE_SETTING(4)
static const Setting choice_settings[SETTING_ROWS]={
    {choice_label_0,choice_rows,choice_value_0,choice_committed,choice_apply},
    {choice_label_1,choice_values_1,choice_value_1,choice_get_1,choice_set_1},
    {choice_label_2,choice_values_2,choice_value_2,choice_get_2,choice_set_2},
    {choice_label_3,choice_values_3,choice_value_3,choice_get_3,choice_set_3},
    {choice_label_4,choice_values_4,choice_value_4,choice_get_4,choice_set_4},
};
/* The page setting as the one row of a native value page, for the MENU row
   of a page without settings rows. Its values are the rows of the page. */
static const u16 *choice_one_label(volatile u32 *h) { return choice_title(h); }
static const u16 *choice_one_text(volatile u32 *h,u32 row) { return choice_label(h,row); }
static const Setting choice_one[SETTING_ROWS]={
    {choice_one_label,choice_rows,choice_one_text,choice_committed,choice_apply},
};
static const MenuPage choice_settings_module={choice_title,choice_summary,choice_rows,choice_label,choice_icon,
    choice_checked,choice_apply,choice_committed,0,0,choice_settings};

/* A page of CORE.TONE gets its rows from the core (runtime/tone.h), and its
   title from the module. */
static const MenuPage tone_module={choice_title,tone_summary,tone_rows,tone_label,tone_icon,
    tone_checked,tone_apply,tone_committed,tone_preview,tone_parameters,0};

/* The page of module `index`. */
static const MenuPage *menu_page(volatile u32 *h,u32 index) {
    if(index>=ABOUT_MODULE) return &about_module;
    if(index==MANAGE_MODULE) return &manage_module_page;
    h[GRKIT_HOST_MODULE_CHOICE/4]=index;
    /* Only a registered page has a GrkitPage to read. */
    if(index>=module_pages(h)) return &choice_module;
    if(CHOICE_ENTRY(h)[1]==GRKIT_CORE_TONE) return &tone_module;
    if(PAGE_WORD(CHOICE_ENTRY(h)[0],PAGE_SETTING_ROWS)) return &choice_settings_module;
    return &choice_module;
}

/* The global icon ID of a page's icon result. Module pages give global IDs. */
static u32 module_icon(u32 module,u32 icon) { (void)module; return icon; }
/* Whether a module layer paints its committed value on the ADJ canvas. */
static u32 module_layers_on(volatile u32 *h) {
    u32 i, n=h[GRKIT_HOST_MODULE_LAYER_COUNT/4];
    for(i=0;i<n && i<GRKIT_MODULE_LAYERS_MAX;++i) {
        u32 handle=h[GRKIT_HOST_MODULE_LAYERS/4+i*3+1];
        if(services_suspended(h,h[GRKIT_HOST_MODULE_LAYERS/4+i*3])) continue;
        if(handle==0xffffffffU || setting_value(h,handle)) return 1;
    }
    return 0;
}

/* ADJ covers the live-view drawing, so ADJ draws the module layers on its
   own canvas. A module page shows its highlighted value with the committed
   values of the others. The page of CORE.ASPECT shows the area of its
   highlighted ratio, and other pages show the crop that the live view draws.
   Off ends the drawing, and the canvas stays until a full draw has erased
   it. The preview names the ADJ controller, not its canvas: when ADJ opens
   on GRKit, its only refresh runs before ADJ makes the canvas. A change of
   the preview redraws the canvas, if it exists: a return from a native
   settings page draws nothing more. */
#ifndef MENU_PREVIEW_INVALIDATE
#define MENU_PREVIEW_INVALIDATE(view) CALL1(FW_VIEW_INVALIDATE)(view)
#endif
static void frame_preview_set(volatile u32 *h,u32 controller,u32 movie) {
    u32 module=h[GRKIT_HOST_MENU_LEVEL/4]==MENU_MODULE ? h[GRKIT_HOST_MODULE/4] : 0;
    u32 slot=module, choice=h[GRKIT_HOST_MENU_LEVEL/4]==MENU_MODULE && slot<module_pages(h);
    u32 ratio=framing_crop(h), handle, value;
    h[GRKIT_HOST_MODULE_PREVIEW/4]=0;
    if(movie) { h[GRKIT_HOST_FRAME_PREVIEW/4]=0; return; }
    if(choice) {
        menu_page(h,module);
        handle=CHOICE_ENTRY(h)[1];
        value=choice_row_value(h,h[GRKIT_HOST_MENU_CURSOR/4]);
        h[GRKIT_HOST_MODULE_PREVIEW/4]=handle+1;
        h[GRKIT_HOST_MODULE_PREVIEW/4+1]=value;
        if(handle==GRKIT_CORE_ASPECT) ratio=value;
    }
    /* A highlighted Off keeps the canvas for the next row. */
    h[GRKIT_HOST_FRAME_PREVIEW/4]=ratio || choice || module_layers_on(h) ? ratio+1 : 0;
    h[GRKIT_HOST_FRAME_PREVIEW_ADJ/4]=controller;
}
static void frame_preview(volatile u32 *h,u32 controller,u32 movie) {
    u32 frame=h[GRKIT_HOST_FRAME_PREVIEW/4], at=h[GRKIT_HOST_FRAME_PREVIEW_ADJ/4];
    u32 handle=h[GRKIT_HOST_MODULE_PREVIEW/4], value=h[GRKIT_HOST_MODULE_PREVIEW/4+1];
    u32 canvas=controller ? WORD(controller,ADJ_CANVAS) : 0;
    frame_preview_set(h,controller,movie);
    if(canvas && !movie && (frame!=h[GRKIT_HOST_FRAME_PREVIEW/4] || at!=controller ||
                  handle!=h[GRKIT_HOST_MODULE_PREVIEW/4] || value!=h[GRKIT_HOST_MODULE_PREVIEW/4+1]))
        MENU_PREVIEW_INVALIDATE(canvas);
}

/* The listed module of page `slot` (runtime/manage.h), or MANAGE_MODULES. */
static u32 page_listed(volatile u32 *h,u32 slot) {
    u32 choice=h[GRKIT_HOST_MODULE_CHOICE/4], listed=MANAGE_MODULES;
    if(slot<module_pages(h)) {
        h[GRKIT_HOST_MODULE_CHOICE/4]=slot;
        listed=manage_of_title(h,PAGE_WORD(CHOICE_ENTRY(h)[0],PAGE_TITLE));
    }
    h[GRKIT_HOST_MODULE_CHOICE/4]=choice;
    return listed;
}
/* The listed modules that the root of ADJ can show (runtime/manage.h): each
   with a module page of two rows or more, also while a soft Off stops it.
   The search reads each module page, so only the menu runs it, when the
   GRKit section is added and when ADJ Order opens. The text hooks of the
   page read the result (GRKIT_HOST_ADJ_MODULES). */
static void manage_adj_refresh(volatile u32 *h) {
    u32 choice=h[GRKIT_HOST_MODULE_CHOICE/4], mask=0, slot, listed;
    for(slot=0;slot<module_pages(h);++slot) {
        listed=page_listed(h,slot);
        if(listed<MANAGE_MODULES && menu_page(h,MODULE_PAGE(slot))->rows(h)>=2) mask|=1U<<listed;
    }
    /* menu_page selects a page: keep the page of the caller. */
    h[GRKIT_HOST_MODULE_CHOICE/4]=choice;
    h[GRKIT_HOST_ADJ_MODULES/4]=mask;
}
static u32 manage_adj_modules(volatile u32 *h) { return h[GRKIT_HOST_ADJ_MODULES/4]; }
/* Visible modules as a bit mask: each module page with two rows or more,
   unless a soft Off stopped its module. A page of CORE.TONE has
   rows only with effects. Without such a page, About shows alone, so that
   the root has a row. */
static u32 visible_modules(volatile u32 *h) {
    u32 choice=h[GRKIT_HOST_MODULE_CHOICE/4], mask=0, slot, listed;
    for(slot=0;slot<module_pages(h);++slot) {
        listed=page_listed(h,slot);
        if(listed<MANAGE_MODULES && manage_stopped(h,listed)) continue;
        if(menu_page(h,MODULE_PAGE(slot))->rows(h)>=2) mask|=1U<<MODULE_PAGE(slot);
    }
    /* Module management and the version are in the native MENU (runtime/menu.c).
       About shows only so that the root has a row. */
    if(!mask) mask=1U<<ABOUT_MODULE;
    /* menu_page selects a page: keep the page of the caller. */
    h[GRKIT_HOST_MODULE_CHOICE/4]=choice;
    return mask;
}
/* The root order in `order`, and its length: the module pages in the order
   of their modules (CORE.POS, runtime/manage.h), which applies at once, then
   the core rows. */
static u32 root_order(volatile u32 *h,u32 *order) {
    u32 n=module_pages(h), keys[GRKIT_MODULE_PAGES_MAX], i, j, t;
    for(i=0;i<n;++i) {
        u32 listed=page_listed(h,i);
        keys[i]=listed<MANAGE_MODULES ? manage_key(h,listed) : 2*MANAGE_DEFAULT_KEY+i;
        order[i]=MODULE_PAGE(i);
        for(j=i;j && keys[j-1]>keys[j];--j) {
            t=keys[j]; keys[j]=keys[j-1]; keys[j-1]=t;
            t=order[j]; order[j]=order[j-1]; order[j-1]=t;
        }
    }
    order[n]=MANAGE_MODULE; order[n+1]=ABOUT_MODULE;
    return n+2;
}
/* The module of a root row, or of the last row for a row past the end. */
static u32 root_module(volatile u32 *h,u32 row) {
    u32 mask=visible_modules(h), order[GRKIT_MODULE_PAGES_MAX+3], n=root_order(h,order), i, last=0;
    for(i=0;i<n;++i) {
        if(!(mask>>order[i]&1)) continue;
        if(!row--) return order[i];
        last=order[i];
    }
    return last;
}
/* The root row of a module, or the number of root rows for MODULE_TOTAL. */
static u32 root_row(volatile u32 *h,u32 module) {
    u32 mask=visible_modules(h), order[GRKIT_MODULE_PAGES_MAX+3], n=root_order(h,order), row=0, i;
    for(i=0;i<n && order[i]!=module;++i) row+=mask>>order[i]&1;
    return row;
}
static u32 root_count(volatile u32 *h) { return root_row(h,MODULE_TOTAL); }
/* The module of the current row: the row's module at the root, else the open page. */
static u32 current_index(volatile u32 *h) {
    return h[GRKIT_HOST_MENU_LEVEL/4]==MENU_ROOT ? root_module(h,h[GRKIT_HOST_MENU_CURSOR/4]) : h[GRKIT_HOST_MODULE/4];
}
static const MenuPage *current_module(volatile u32 *h) { return menu_page(h,current_index(h)); }
static u32 count(volatile u32 *h) {
    return h[GRKIT_HOST_MENU_LEVEL/4]==MENU_ROOT ? root_count(h) : current_module(h)->rows(h);
}
/* The page learns that its highlighted row changed, so the page of CORE.TONE
   can preview the row in the live view. */
static void highlight(volatile u32 *h) {
    const MenuPage *module;
    if(h[GRKIT_HOST_MENU_LEVEL/4]!=MENU_MODULE || h[GRKIT_HOST_MODULE/4]>=MODULE_TOTAL) return;
    module=menu_page(h,h[GRKIT_HOST_MODULE/4]);
    if(module->highlight) module->highlight(h);
}
/* Fn applies to the highlighted row of a page that has native parameters,
   and to the root row of such a page for its committed row. */
static u32 parameters_row(volatile u32 *h) {
    const MenuPage *module;
    u32 row=h[GRKIT_HOST_MENU_CURSOR/4];
    if(h[GRKIT_HOST_MENU_LEVEL/4]==MENU_ROOT) {
        module=menu_page(h,root_module(h,row));
        row=module->committed(h);
    }
    else if(h[GRKIT_HOST_MENU_LEVEL/4]!=MENU_MODULE || h[GRKIT_HOST_MODULE/4]>=MODULE_TOTAL) return 0;
    else module=menu_page(h,h[GRKIT_HOST_MODULE/4]);
    return module->parameters && module->parameters(h,row);
}
/* Each module opens at its committed row. Image Control previews from there. */
static void menu_open(volatile u32 *h,u32 module) {
    h[GRKIT_HOST_MODULE/4]=module;
    h[GRKIT_HOST_MENU_LEVEL/4]=MENU_MODULE;
    h[GRKIT_HOST_MENU_CURSOR/4]=menu_page(h,module)->committed(h);
}
/* ADJ opens at the root. About always shows there, so the root has a row even
   when no module page is visible. */
static void menu_start(volatile u32 *h) {
    h[GRKIT_HOST_MENU_LEVEL/4]=MENU_ROOT;
    h[GRKIT_HOST_MENU_CURSOR/4]=0;
    h[GRKIT_HOST_MODULE/4]=0;
}
/* Each ADJ session opens at the root, on the row of the module that was
   highlighted or open when the last session ended. */
static void menu_normalize(volatile u32 *h);
static void menu_reopen(volatile u32 *h) {
    u32 module=h[GRKIT_HOST_MODULE/4];
    h[GRKIT_HOST_MENU_LEVEL/4]=MENU_ROOT;
    h[GRKIT_HOST_MENU_CURSOR/4]=module<MODULE_TOTAL && visible_modules(h)>>module&1 ? root_row(h,module) : 0;
    menu_normalize(h);
}
/* Whether a page can be open: a visible module. */
static u32 page_reachable(volatile u32 *h,u32 module) { return visible_modules(h)>>module&1; }
static void menu_normalize(volatile u32 *h) {
    u32 module=h[GRKIT_HOST_MODULE/4];
    if(h[GRKIT_HOST_MENU_LEVEL/4]<MENU_ROOT || h[GRKIT_HOST_MENU_LEVEL/4]>MENU_MODULE || module>=MODULE_TOTAL ||
       (h[GRKIT_HOST_MENU_LEVEL/4]==MENU_MODULE && (!page_reachable(h,module) || !menu_page(h,module)->rows(h))))
        menu_start(h);
    if(h[GRKIT_HOST_MENU_CURSOR/4]>=count(h)) h[GRKIT_HOST_MENU_CURSOR/4]=0;
}
/* OK on a root row opens its page. A row without a page, such as About, does nothing. */
static void menu_confirm(volatile u32 *h) {
    menu_normalize(h);
    if(h[GRKIT_HOST_MENU_LEVEL/4]==MENU_ROOT) {
        u32 module=root_module(h,h[GRKIT_HOST_MENU_CURSOR/4]);
        if(menu_page(h,module)->rows(h)) menu_open(h,module);
    }
    else current_module(h)->apply(h,h[GRKIT_HOST_MENU_CURSOR/4]);
}
/* Return false at the top level so the caller can restore the stock ADJ page. */
static u32 menu_back(volatile u32 *h) {
    /* Leaving a level without OK restores the committed effect in the live view. */
    h[GRKIT_HOST_PREVIEW/4]=0;
    menu_normalize(h);
    if(h[GRKIT_HOST_MENU_LEVEL/4]==MENU_ROOT) return 0;
    h[GRKIT_HOST_MENU_LEVEL/4]=MENU_ROOT;
    h[GRKIT_HOST_MENU_CURSOR/4]=root_row(h,h[GRKIT_HOST_MODULE/4]);
    return 1;
}
#endif
