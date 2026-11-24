#ifndef GRKIT_CONTROL_MODULES_H
#define GRKIT_CONTROL_MODULES_H
/* The commands of the Wi-Fi service for the module settings (grkit_core::abi,
   CONTROL_MODULES_READ and after), answered in the poll of the mailbox
   (runtime/control_looks.h). They read the texts and values of the module
   pages, and change Enable, the values, Options, Order, and ADJ Order with
   the code of those MENU rows, so a change saves and applies as in MENU.
   Two more read and set the rows of GRKit Settings in the same way.

   Each command selects pages through GRKIT_HOST_MODULE_CHOICE, the page of
   the generic callbacks, and puts the selection back. A change checks each
   field before the first store, so an invalid request changes nothing. */

/* The settings of a module: at most four children of a group, each with
   GRKIT_SETTING_ROWS rows. */
#define CONTROL_MODULE_ROWS (4U*(SETTING_ROWS-1U))
/* The core builds with a frame limit, and the poll runs on the stack of the
   App task: each command keeps its own frame out of the frame of the poll. */
#define CONTROL_FRAME __attribute__((noinline))
/* The row of a page without settings rows: the page setting, named by the
   title and the labels of the page. */
#define CONTROL_ROW_PAGE 0xffU
#define CONTROL_NO_OPTIONS MODULE_TOTAL
#ifndef MODULE_TILE_WORD
#define MODULE_TILE_WORD(address,offset) WORD(address,offset)
#endif
typedef struct { u16 handle; u8 slot, row; } ControlModuleRow;
_Static_assert(GRKIT_CORE_SETTING_BASE+GRKIT_CORE_SETTING_COUNT<=0x10000U,"A handle fits its row");

/* The rows of page `slot` after the `n` rows in `rows`, as MENU lists them
   (runtime/menu_listed.h: listed_base). A page of actions has none. */
static u32 control_page_rows(volatile u32 *h,u32 slot,ControlModuleRow *rows,u32 n) {
    const MenuPage *page=menu_page(h,slot);
    u32 m=PAGE_WORD(CHOICE_ENTRY(h)[0],PAGE_SETTING_ROWS), k;
    if(m>SETTING_ROWS-1U) m=SETTING_ROWS-1U;
    for(k=0;k<m && n<CONTROL_MODULE_ROWS;++k,++n) {
        rows[n].handle=(u16)(k ? choice_row_handle(h,k) : CHOICE_ENTRY(h)[1]);
        rows[n].slot=(u8)slot;
        rows[n].row=(u8)k;
    }
    if(!m && !(page->flags&MENU_PAGE_ACTIONS) && page->rows(h)<=MENU_VALUES_MAX && n<CONTROL_MODULE_ROWS) {
        rows[n].handle=(u16)CHOICE_ENTRY(h)[1];
        rows[n].slot=(u8)slot;
        rows[n++].row=CONTROL_ROW_PAGE;
    }
    return n;
}
/* The settings of a listed module and their number. *managed is 1 for the
   page of CORE.TONE and 2 for that of CORE.RECIPE, whose rows the phone
   manages in its own tabs: they have no settings here. */
static u32 control_module_rows(volatile u32 *h,u32 module,ControlModuleRow *rows,u32 *managed) {
    u32 slot=listed_page(h,module), n=0, r, children, child;
    *managed=0;
    if(slot>=module_pages(h)) return 0;
    menu_page(h,slot);
    if(CHOICE_ENTRY(h)[1]==GRKIT_CORE_TONE) *managed=1;
    else if(CHOICE_ENTRY(h)[1]==GRKIT_CORE_RECIPE) *managed=2;
    if(*managed) return 0;
    if(!choice_group(h)) return control_page_rows(h,slot,rows,0);
    children=group_rows(h);
    if(children>4U) children=4U;
    for(r=0;r<children;++r) {
        h[GRKIT_HOST_MODULE_CHOICE/4]=slot;
        child=group_child(h,r);
        if(child<module_pages(h)) n=control_page_rows(h,child,rows,n);
    }
    return n;
}
/* The key of a setting: its state ID, stable across starts, or for a
   setting without one, such as a core setting, its handle with a mark. */
static u32 control_module_key(volatile u32 *h,u32 handle) {
    volatile u32 *t=settings_table(h);
    if(!IS_CORE_SETTING(handle) && t && handle<t[0] && t[SETTINGS_ENTRY(handle)]) return t[SETTINGS_ENTRY(handle)];
    return GRKIT_CONTROL_MODULE_KEY_HANDLE|handle;
}
/* Whether the camera offers `value` for a row: the page setting also needs
   a text of its page (choice_named). */
static u32 control_row_offers(volatile u32 *h,const ControlModuleRow *row,u32 value) {
    h[GRKIT_HOST_MODULE_CHOICE/4]=row->slot;
    if(!setting_offered(h,row->handle,value)) return 0;
    return row->handle!=CHOICE_ENTRY(h)[1] || value<choice_named(h);
}
/* The page of a module with options, selected, or CONTROL_NO_OPTIONS. */
static u32 control_options_page(volatile u32 *h,u32 module) {
    u32 slot=listed_page(h,module);
    if(slot>=module_pages(h)) return CONTROL_NO_OPTIONS;
    menu_page(h,slot);
    if(CHOICE_ENTRY(h)[1]==GRKIT_CORE_TONE || CHOICE_ENTRY(h)[1]==GRKIT_CORE_RECIPE ||
       choice_group(h) || !choice_options(h)) return CONTROL_NO_OPTIONS;
    return slot;
}
/* The values of the selected page in the order of ADJ, first row included,
   without a value that the page does not name, and their number. */
static u32 control_sequence(volatile u32 *h,u8 *values) {
    u32 n=choice_sequence(h,values), i, k=0;
    for(i=0;i<n;++i) if(values[i]<choice_named(h)) values[k++]=values[i];
    return k;
}
/* Whether the selected page has an Order row (runtime/menu_listed.h:
   order_count), whose values fit the nibbles of CORE.ORDER. */
static u32 control_page_orders(volatile u32 *h,const u8 *values,u32 n) {
    u32 i;
    if(!choice_orders(h) || !manage_word(h,MANAGE_ORDERS) || n<3U || n>MANAGE_ORDER_VALUES+2U) return 0;
    for(i=1;i<n;++i) if(!values[i] || values[i]>15U) return 0;
    return 1;
}

/* The entry of a module at `at`, or 0 when it does not fit: the new `at`.
   `flags` has GRKIT_CONTROL_MODULE_ORDERABLE and GRKIT_CONTROL_MODULE_PLACEABLE. */
static CONTROL_FRAME u32 control_module_entry(volatile u32 *h,u32 module,u32 place,u32 flags,u8 *out,u32 at) {
    ControlModuleRow rows[CONTROL_MODULE_ROWS];
    u8 values[CHOICE_SEQUENCE];
    u32 managed, settings=control_module_rows(h,module,rows,&managed);
    u32 slot=control_options_page(h,module), ordered=slot!=CONTROL_NO_OPTIONS ? control_sequence(h,values) : 0;
    u32 status=manage_status(h,module), word, s, name;
    const volatile u16 *units=(const volatile u16 *)MANAGE_MAP(manage_entry(h,module)+MANAGE_NAME);
    if(at+GRKIT_CONTROL_MODULE_ENTRY+8U*settings+((ordered+3U)&~3U)>CONTROL_PAYLOAD_BYTES) return 0;
    word=module|place<<GRKIT_CONTROL_MODULE_PLACE_SHIFT;
    word|=(status==MANAGE_LOADED ? (manage_halted(h,module) ? GRKIT_CONTROL_MODULE_STOPPED : GRKIT_CONTROL_MODULE_RUNNING) :
           status==MANAGE_SAFE ? GRKIT_CONTROL_MODULE_SAFE : GRKIT_CONTROL_MODULE_FAILED)<<GRKIT_CONTROL_MODULE_STATUS_SHIFT;
    if(!manage_off(h,module)) word|=GRKIT_CONTROL_MODULE_ENABLED;
    word|=flags;
    control_looks_put(out+at,word);
    /* The file name, without the error status after it. */
    for(s=0,name=1;s<8U;++s) {
        u32 unit=name ? units[s] : 0;
        if(!unit || unit==' ') { name=0; unit=0; }
        out[at+4U+s]=(u8)unit;
    }
    control_looks_put(out+at+12,slot!=CONTROL_NO_OPTIONS ? manage_hidden_values(h,module)&~1U : 0);
    control_looks_put(out+at+16,settings|ordered<<8);
    at+=GRKIT_CONTROL_MODULE_ENTRY;
    for(s=0;s<settings;++s,at+=8) {
        control_looks_put(out+at,control_module_key(h,rows[s].handle));
        control_looks_put(out+at+4,setting_value(h,rows[s].handle));
    }
    for(s=0;s<ordered;++s) out[at+s]=values[s];
    for(;s&3U;++s) out[at+s]=0;
    return at+s;
}
/* The state of the modules from row `first`, as many as fit. */
static CONTROL_FRAME u32 control_modules_read(volatile u32 *h,const u8 *payload,u32 length,u8 *out,u32 *size) {
    ManagePlaces p;
    u8 list[MANAGE_MODULES], places[MANAGE_MODULES];
    u32 count=manage_count(h), first, n=0, seen=0, roots=0, placed, i, k=0, at=GRKIT_CONTROL_MODULES_HEADER, next;
    if(length!=4) return CONTROL_BAD_LENGTH;
    first=control_looks_word(payload);
    manage_places(h,&p);
    for(i=0;i<MANAGE_PINS;++i) if(p.pins[i]<MANAGE_MODULES) { places[n]=GRKIT_CONTROL_MODULE_TOOLBAR; list[n++]=(u8)p.pins[i]; }
    for(i=0;i<p.roots;++i) { roots|=1U<<p.root[i]; places[n]=GRKIT_CONTROL_MODULE_ROOT; list[n++]=(u8)p.root[i]; }
    for(i=0;i<p.hiddens;++i) { places[n]=GRKIT_CONTROL_MODULE_HIDDEN; list[n++]=(u8)p.hidden[i]; }
    for(i=0;i<n;++i) seen|=1U<<list[i];
    placed=seen;
    /* A module that ADJ cannot show keeps the place of its CORE.POS. */
    for(i=0;i<count;++i) {
        u32 place;
        if(seen>>i&1) continue;
        place=manage_place(h,i);
        places[n]=(u8)(place>=MANAGE_PLACE_PIN ? GRKIT_CONTROL_MODULE_TOOLBAR :
                       place==MANAGE_PLACE_HIDDEN ? GRKIT_CONTROL_MODULE_HIDDEN : GRKIT_CONTROL_MODULE_ROOT);
        list[n++]=(u8)i;
    }
    /* A color-only module on a monochrome camera is not a module of MENU. */
    for(i=0,next=0;i<n;++i) if(manage_status(h,list[i])!=MANAGE_COLOR_ONLY) {
        list[next]=list[i];
        places[next++]=places[i];
    }
    n=next;
    if(first>n) return CONTROL_INVALID;
    for(i=first;i<n;++i,++k) {
        next=control_module_entry(h,list[i],places[i],(roots>>list[i]&1U ? GRKIT_CONTROL_MODULE_ORDERABLE : 0)|
                                  (placed>>list[i]&1U ? GRKIT_CONTROL_MODULE_PLACEABLE : 0),out,at);
        if(!next) break;
        at=next;
    }
    control_looks_put(out,n);
    control_looks_put(out+4,h[GRKIT_HOST_STATE_CHANGES/4]);
    control_looks_put(out+8,!manage_grkit_off(h));
    control_looks_put(out+12,first);
    control_looks_put(out+16,k);
    control_looks_put(out+20,h[GRKIT_HOST_CAPABILITIES/4]);
    *size=at;
    return 0;
}

typedef struct { u8 *out; u32 at, index, cursor, count, full; } ControlTexts;
/* One record of the walk, from `cursor` on, while it fits. */
static void control_text_record(ControlTexts *w,u32 kind,u32 a,u32 b,const u16 *text) {
    u32 units=0, i, size;
    if(w->full) return;
    if(w->index<w->cursor) { ++w->index; return; }
    while(text && units<GRKIT_CONTROL_MODULE_TEXT_UNITS && text[units]) ++units;
    size=12U+((2U*units+3U)&~3U);
    if(w->at+size>CONTROL_PAYLOAD_BYTES) { w->full=1; return; }
    control_looks_put(w->out+w->at,kind|units<<8);
    control_looks_put(w->out+w->at+4,a);
    control_looks_put(w->out+w->at+8,b);
    for(i=0;i<units;++i) {
        w->out[w->at+12U+2U*i]=(u8)text[i];
        w->out[w->at+13U+2U*i]=(u8)(text[i]>>8);
    }
    for(i=12U+2U*units;i<size;++i) w->out[w->at+i]=0;
    w->at+=size;
    ++w->index;
    ++w->count;
}
/* Text `index` of a text list in `language`, or 0 without the list. */
static const u16 *control_list_text(u32 list,u32 index,u32 language) {
    u32 address=list ? PAGE_WORD(list,(index*GRKIT_TEXT_LANGUAGES+language)*4) : 0;
    return address ? MENU_TEXT(address) : 0;
}
/* The label of a row of the selected page. */
static const u16 *control_row_label(volatile u32 *h,const ControlModuleRow *row,u32 language) {
    u32 list=row->row==CONTROL_ROW_PAGE ? PAGE_WORD(CHOICE_ENTRY(h)[0],PAGE_TITLE) : PAGE_WORD(choice_row(h,row->row),ROW_LABEL);
    return control_list_text(list,0,language);
}
/* The text of a value of a row of the selected page. */
static const u16 *control_value_text(volatile u32 *h,const ControlModuleRow *row,u32 value,u32 language) {
    const u16 *crop=crop_text(row->handle,value);
    u32 list;
    if(crop) return crop;
    list=row->row==CONTROL_ROW_PAGE ? PAGE_WORD(CHOICE_ENTRY(h)[0],PAGE_LABELS) : PAGE_WORD(choice_row(h,row->row),ROW_VALUES);
    return control_list_text(list,value,language);
}
/* The title of a module: the title of its page, or for a module without a
   page, its file name without the error status after it. */
static void control_module_title(volatile u32 *h,u32 module,u32 language,u16 *name,const u16 **title) {
    u32 entry=manage_entry(h,module), list=MANAGE_MAP(entry+MANAGE_TITLE)[0], i;
    const volatile u16 *units=(const volatile u16 *)MANAGE_MAP(entry+MANAGE_NAME);
    *title=list ? control_list_text(list,0,language) : 0;
    if(*title) return;
    for(i=0;i<MANAGE_NAME_UNITS && units[i] && units[i]!=' ';++i) name[i]=units[i];
    name[i]=0;
    *title=name;
}
/* The options bits of the module record, and the key of the page setting. */
static CONTROL_FRAME u32 control_module_flags(volatile u32 *h,u32 module,u32 *key) {
    u8 sequence[CHOICE_SEQUENCE];
    u32 flags, n;
    *key=0;
    if(control_options_page(h,module)==CONTROL_NO_OPTIONS) return 0;
    flags=GRKIT_CONTROL_MODULE_HAS_OPTIONS;
    n=control_sequence(h,sequence);
    if(control_page_orders(h,sequence,n)) flags|=GRKIT_CONTROL_MODULE_ORDERED;
    *key=control_module_key(h,CHOICE_ENTRY(h)[1]);
    return flags;
}
static CONTROL_FRAME u32 control_module_texts(volatile u32 *h,const u8 *payload,u32 length,u8 *out,u32 *size) {
    ControlModuleRow rows[CONTROL_MODULE_ROWS];
    ControlTexts w;
    u16 name[MANAGE_NAME_UNITS+1U];
    const u16 *title;
    u32 module, language, n, managed, flags, key, group, slot, i, v, values;
    if(length!=12) return CONTROL_BAD_LENGTH;
    module=control_looks_word(payload);
    language=control_looks_word(payload+4);
    w.out=out; w.at=GRKIT_CONTROL_MODULE_TEXTS_HEADER; w.index=0;
    w.cursor=control_looks_word(payload+8); w.count=0; w.full=0;
    if(language>=GRKIT_TEXT_LANGUAGES) return CONTROL_INVALID;
    if(module>=manage_count(h)) return CONTROL_CHANGED;
    n=control_module_rows(h,module,rows,&managed);
    flags=managed|control_module_flags(h,module,&key);
    control_module_title(h,module,language,name,&title);
    control_text_record(&w,GRKIT_CONTROL_MODULE_RECORD_MODULE,flags,key,title);
    slot=listed_page(h,module);
    group=0;
    if(slot<module_pages(h)) { menu_page(h,slot); group=choice_group(h); }
    for(i=0;i<n;++i) {
        h[GRKIT_HOST_MODULE_CHOICE/4]=rows[i].slot;
        if(group && (!i || rows[i].slot!=rows[i-1].slot))
            control_text_record(&w,GRKIT_CONTROL_MODULE_RECORD_GROUP,0,0,
                                control_list_text(PAGE_WORD(CHOICE_ENTRY(h)[0],PAGE_TITLE),0,language));
        control_text_record(&w,GRKIT_CONTROL_MODULE_RECORD_SETTING,control_module_key(h,rows[i].handle),0,
                            control_row_label(h,&rows[i],language));
        values=setting_rows(h,rows[i].handle);
        if(values>MENU_VALUES_MAX) values=MENU_VALUES_MAX;
        for(v=0;v<values;++v) {
            u32 value=setting_row_value(h,rows[i].handle,v);
            if(!control_row_offers(h,&rows[i],value)) continue;
            control_text_record(&w,GRKIT_CONTROL_MODULE_RECORD_VALUE,value,0,control_value_text(h,&rows[i],value,language));
        }
    }
    control_looks_put(out,w.full ? w.index : GRKIT_CONTROL_MODULE_TEXTS_END);
    control_looks_put(out+4,w.count);
    *size=w.at;
    return 0;
}
/* The next bytes of tile 0 of a module. */
static CONTROL_FRAME u32 control_module_tile(volatile u32 *h,const u8 *payload,u32 length,u8 *out,u32 *size) {
    u32 module, offset, tile, address, n, i;
    if(length!=8) return CONTROL_BAD_LENGTH;
    module=control_looks_word(payload);
    offset=control_looks_word(payload+4);
    if(module>=manage_count(h)) return CONTROL_CHANGED;
    if(offset>=GRM_TILE_BYTES || offset&3U) return CONTROL_INVALID;
    tile=MANAGE_MAP(manage_entry(h,module)+MANAGE_TILE)[0];
    if(tile==MANAGE_NO_TILE) return CONTROL_INVALID;
    address=module_tile_address(h,GRKIT_ICON_MODULE_TILE(tile));
    if(!address) return CONTROL_INVALID;
    address+=GRKIT_ICON_STRIDE-GRM_TILE_BYTES;
    n=GRM_TILE_BYTES-offset;
    if(n>CONTROL_PAYLOAD_BYTES) n=CONTROL_PAYLOAD_BYTES;
    for(i=0;i<n;i+=4) control_looks_put(out+i,MODULE_TILE_WORD(address,offset+i));
    *size=n;
    return 0;
}
/* Check the whole request, then store it as the rows of MENU do: Enable
   (listed_set), the values, Order (option_move), and Options (option_set),
   where a value that goes off in ADJ sets its setting to Off. */
static CONTROL_FRAME u32 control_module_set(volatile u32 *h,const u8 *payload,u32 length) {
    ControlModuleRow rows[CONTROL_MODULE_ROWS];
    u8 target[CONTROL_MODULE_ROWS], current[CHOICE_SEQUENCE];
    u32 module, fields, off, hidden, settings, ordered, n, managed, slot, handle=0, c=0, mask=0, i, j, moved=0;
    const u8 *pairs=payload+24, *list;
    if(length<24 || length&3U) return CONTROL_BAD_LENGTH;
    module=control_looks_word(payload);
    fields=control_looks_word(payload+4);
    off=control_looks_word(payload+8);
    hidden=control_looks_word(payload+12);
    settings=control_looks_word(payload+16);
    ordered=control_looks_word(payload+20);
    if(settings>CONTROL_MODULE_ROWS || ordered>CHOICE_SEQUENCE || length!=24U+8U*settings+4U*ordered) return CONTROL_BAD_LENGTH;
    list=pairs+8U*settings;
    if(module>=manage_count(h)) return CONTROL_CHANGED;
    if(fields&~(GRKIT_CONTROL_MODULE_SET_ENABLED|GRKIT_CONTROL_MODULE_SET_HIDDEN|GRKIT_CONTROL_MODULE_SET_ORDER) || off>1U)
        return CONTROL_INVALID;
    n=control_module_rows(h,module,rows,&managed);
    for(i=0;i<settings;++i) {
        u32 key=control_looks_word(pairs+8U*i), value=control_looks_word(pairs+8U*i+4U);
        for(j=0;j<n && control_module_key(h,rows[j].handle)!=key;++j) {}
        if(j>=n || !control_row_offers(h,&rows[j],value)) return CONTROL_INVALID;
        target[i]=(u8)j;
        for(j=0;j<i;++j) if(target[j]==target[i]) return CONTROL_INVALID;
    }
    slot=control_options_page(h,module);
    if(slot==CONTROL_NO_OPTIONS) {
        if(fields&(GRKIT_CONTROL_MODULE_SET_HIDDEN|GRKIT_CONTROL_MODULE_SET_ORDER)) return CONTROL_INVALID;
    } else {
        u32 allowed=0, left=0;
        handle=CHOICE_ENTRY(h)[1];
        c=control_sequence(h,current);
        for(i=1;i<c;++i) {
            if(current[i]<MANAGE_OPTION_VALUES) allowed|=1U<<current[i];
            else left=1;
        }
        mask=manage_hidden_values(h,module)&~1U;
        if(fields&GRKIT_CONTROL_MODULE_SET_HIDDEN) {
            if(hidden&~allowed || (c>1U && !left && !(allowed&~hidden))) return CONTROL_INVALID;
            mask=hidden;
        }
        /* A value that ADJ does not show is not a value to choose. */
        for(i=0;i<settings;++i) {
            u32 value=control_looks_word(pairs+8U*i+4U);
            if(rows[target[i]].handle==handle && value<32U && mask>>value&1U) return CONTROL_INVALID;
        }
        if(fields&GRKIT_CONTROL_MODULE_SET_ORDER) {
            if(ordered!=c) return CONTROL_INVALID;
            for(i=0;i<c;++i) {
                u32 value=control_looks_word(list+4U*i);
                for(j=0;j<c && current[j]!=value;++j) {}
                if(j>=c) return CONTROL_INVALID;
                for(j=0;j<i;++j) if(control_looks_word(list+4U*j)==value) return CONTROL_INVALID;
                moved|=value!=current[i];
            }
            /* The first row stays first. */
            if(c && control_looks_word(list)!=current[0]) return CONTROL_INVALID;
            /* The new order takes the place of the old one. */
            for(i=0;i<c;++i) current[i]=(u8)control_looks_word(list+4U*i);
            h[GRKIT_HOST_MODULE_CHOICE/4]=slot;
            if(moved && !control_page_orders(h,current,c)) return CONTROL_INVALID;
        }
    }
    if(!control_looks_idle(h)) return CONTROL_BUSY;
    if(fields&GRKIT_CONTROL_MODULE_SET_ENABLED) {
        setting_store(h,manage_off_handle(h,module),off);
        manage_off_changed(h);
    }
    for(i=0;i<settings;++i) setting_store(h,rows[target[i]].handle,control_looks_word(pairs+8U*i+4U));
    if(moved) option_store_order(h,module,current+1,c-1U);
    if(fields&GRKIT_CONTROL_MODULE_SET_HIDDEN) {
        u32 value;
        setting_store(h,manage_rows_handle(h,module),mask);
        value=setting_value(h,handle);
        if(value<32U && mask>>value&1U) setting_store(h,handle,0);
    }
    return 0;
}
/* The modules of the ADJ root in their new order, each once. Toolbar and
   hidden modules keep their places (runtime/manage.h: manage_place_set). */
static CONTROL_FRAME u32 control_modules_order(volatile u32 *h,const u8 *payload,u32 length) {
    ManagePlaces p;
    u32 i, j, seen=0;
    manage_places(h,&p);
    if(length!=4U*p.roots) return CONTROL_CHANGED;
    for(i=0;i<p.roots;++i) {
        u32 module=control_looks_word(payload+4U*i);
        for(j=0;j<p.roots && p.root[j]!=module;++j) {}
        if(j>=p.roots) return CONTROL_CHANGED;
        if(seen>>module&1U) return CONTROL_INVALID;
        seen|=1U<<module;
    }
    if(!control_looks_idle(h)) return CONTROL_BUSY;
    for(i=0;i<p.roots;++i) setting_store(h,manage_off_handle(h,control_looks_word(payload+4U*i))+1U,i+1U);
    return 0;
}
/* The places of each module that ADJ can show, as MENU > ADJ Order sets
   them: the toolbar in its order, the root in its order, then the hidden
   modules, each once. Each module of the page gets its place as CORE.POS
   (runtime/manage.h: manage_place_set), so ADJ shows the change at once. The
   toolbar keeps no empty entry between its modules. */
static CONTROL_FRAME u32 control_modules_layout(volatile u32 *h,const u8 *payload,u32 length) {
    ManagePlaces p;
    u32 i, j, n, seen=0, last=GRKIT_CONTROL_MODULE_TOOLBAR, pins=0, roots=0, placeable=0;
    manage_places(h,&p);
    for(i=0;i<MANAGE_PINS;++i) if(p.pins[i]<MANAGE_MODULES) { placeable|=1U<<p.pins[i]; ++pins; }
    for(i=0;i<p.roots;++i) placeable|=1U<<p.root[i];
    for(i=0;i<p.hiddens;++i) placeable|=1U<<p.hidden[i];
    n=pins+p.roots+p.hiddens;
    if(length&3U || length>4U*MANAGE_MODULES) return CONTROL_BAD_LENGTH;
    if(length!=4U*n) return CONTROL_CHANGED;
    for(i=0,pins=0;i<n;++i) {
        u32 word=control_looks_word(payload+4U*i), module=word&0xffU;
        u32 place=word>>GRKIT_CONTROL_MODULE_PLACE_SHIFT;
        if(word&~(0xffU|GRKIT_CONTROL_MODULE_PLACE_MASK<<GRKIT_CONTROL_MODULE_PLACE_SHIFT) || place>GRKIT_CONTROL_MODULE_HIDDEN ||
           place<last) return CONTROL_INVALID;
        last=place;
        if(place==GRKIT_CONTROL_MODULE_TOOLBAR && ++pins>MANAGE_PINS) return CONTROL_INVALID;
        if(module>=MANAGE_MODULES || !(placeable>>module&1U)) return CONTROL_CHANGED;
        if(seen>>module&1U) return CONTROL_INVALID;
        seen|=1U<<module;
    }
    if(!control_looks_idle(h)) return CONTROL_BUSY;
    for(i=0,j=0;i<n;++i) {
        u32 word=control_looks_word(payload+4U*i), module=word&0xffU, place=word>>GRKIT_CONTROL_MODULE_PLACE_SHIFT, position;
        if(place==GRKIT_CONTROL_MODULE_TOOLBAR) position=MANAGE_POS_PIN+j++;
        else if(place==GRKIT_CONTROL_MODULE_ROOT) position=++roots;
        else position=MANAGE_POS_HIDDEN;
        setting_store(h,manage_off_handle(h,module)+1U,position);
    }
    return 0;
}

/* The GRKit settings of the phone, in the order of the GRKIT_CONTROL_SETTING_*
   indexes: the handle + 1 of each (0 without it), its value, its store, and
   its number of values. The getters and stores are those of the MENU rows
   (runtime/manage.h), so a change saves and keeps its copy as there. */
typedef struct {
    u32 (*handle)(volatile u32 *h);
    u32 (*value)(volatile u32 *h);
    void (*set)(volatile u32 *h,u32 value);
    u32 values;
} ControlSetting;
static const ControlSetting control_settings[GRKIT_CONTROL_SETTINGS]={
    {start_handle,start_value,start_set,MANAGE_START_VALUES},
    {raw_handle,raw_value,raw_set,MANAGE_RAW_VALUES},
    {unit_handle,unit_value,unit_set,MANAGE_UNIT_VALUES},
    {battery_handle,battery_simple,battery_set,MANAGE_BATTERY_VALUES},
    {battery_percent_handle,battery_percent_on,battery_percent_set,MANAGE_BATTERY_PERCENT_VALUES},
    {playback_info_handle,playback_info_on,playback_info_set,MANAGE_PLAYBACK_INFO_VALUES},
    {goodbye_handle,goodbye_custom,goodbye_set,MANAGE_GOODBYE_VALUES},
    {goodbye_text_handle,goodbye_text_shown,goodbye_text_set,MANAGE_GOODBYE_TEXT_VALUES},
};
static CONTROL_FRAME u32 control_settings_read(volatile u32 *h,u32 length,u8 *out,u32 *size) {
    u32 i;
    if(length) return CONTROL_BAD_LENGTH;
    for(i=0;i<GRKIT_CONTROL_SETTINGS;++i)
        control_looks_put(out+4U*i,control_settings[i].handle(h) ? control_settings[i].value(h) : GRKIT_CONTROL_SETTING_NONE);
    *size=4U*GRKIT_CONTROL_SETTINGS;
    return 0;
}
static CONTROL_FRAME u32 control_settings_set(volatile u32 *h,const u8 *payload,u32 length) {
    u32 index, value;
    if(length!=8) return CONTROL_BAD_LENGTH;
    index=control_looks_word(payload);
    value=control_looks_word(payload+4);
    if(index>=GRKIT_CONTROL_SETTINGS || !control_settings[index].handle(h) || value>=control_settings[index].values)
        return CONTROL_INVALID;
    if(!control_looks_idle(h)) return CONTROL_BUSY;
    control_settings[index].set(h,value);
    return 0;
}
static CONTROL_FRAME u32 control_modules(volatile u32 *h,struct control_request *request,u32 *size) {
    u32 choice=h[GRKIT_HOST_MODULE_CHOICE/4], result;
    /* ADJ Order reads a mask that only the menu refreshes, so it can be
       unset before MENU first opens. While a GRKit page shows, the page keeps
       the mask that it read. */
    if(control_looks_idle(h)) manage_adj_refresh(h);
    switch(request->command) {
    case GRKIT_CONTROL_MODULES_READ: result=control_modules_read(h,request->payload,request->length,request->payload,size); break;
    case GRKIT_CONTROL_MODULE_TEXTS: result=control_module_texts(h,request->payload,request->length,request->payload,size); break;
    case GRKIT_CONTROL_MODULE_TILE: result=control_module_tile(h,request->payload,request->length,request->payload,size); break;
    case GRKIT_CONTROL_MODULE_SET: result=control_module_set(h,request->payload,request->length); break;
    case GRKIT_CONTROL_MODULES_ORDER: result=control_modules_order(h,request->payload,request->length); break;
    case GRKIT_CONTROL_MODULES_LAYOUT: result=control_modules_layout(h,request->payload,request->length); break;
    case GRKIT_CONTROL_SETTINGS_READ: result=control_settings_read(h,request->length,request->payload,size); break;
    case GRKIT_CONTROL_SETTINGS_SET: result=control_settings_set(h,request->payload,request->length); break;
    default: result=CONTROL_UNSUPPORTED; break;
    }
    h[GRKIT_HOST_MODULE_CHOICE/4]=choice;
    return result;
}
#endif
