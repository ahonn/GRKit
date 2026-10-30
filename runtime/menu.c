/* The target header, then the shared layout. */
#include "firmware.h"
#include "abi.h"
/* Private ADJ menu. All state belongs to the module host. */
typedef unsigned int u32;
typedef unsigned short u16;
typedef unsigned char u8;
extern volatile u32 *menu_header(void);
extern u32 menu_event_original(u32, const u16 *);
extern void menu_exit_original(u32);
extern void menu_changed_original(u32);
extern void menu_refresh_original(u32);
extern u32 menu_destroy_original(u32);
#define CALL0(a) ((u32 (*)(void))(a))
#define CALL1(a) ((u32 (*)(u32))(a))
#define CALL2(a) ((u32 (*)(u32,u32))(a))
#define CALL4(a) ((u32 (*)(u32,u32,u32,u32))(a))
#define CALL5(a) ((u32 (*)(u32,u32,u32,u32,u32))(a))
#define CALL3(a) ((u32 (*)(u32,u32,u32))(a))
/* A firmware call on a view or a controller of a MENU page: a settings page,
   an options page, the GRKit section, or About GRKit. A host test defines
   them to record the calls. */
#ifndef MENU_VIEW1
#define MENU_VIEW1(fn,a) CALL1(fn)(a)
#define MENU_VIEW2(fn,a,b) CALL2(fn)(a,b)
#define MENU_VIEW3(fn,a,b,c) CALL3(fn)(a,b,c)
#define MENU_VIEW4(fn,a,b,c,d) CALL4(fn)(a,b,c,d)
#endif
/* A word of a firmware object that a MENU page reads: a row, a controller,
   or the stock submenu entry. A host test maps the objects. */
#ifndef MENU_OBJECT_WORD
#define MENU_OBJECT_WORD(a,o) WORD(a,o)
#define MENU_OBJECT_BYTE(a,o) BYTE(a,o)
#endif
/* Memory access. A host test maps it to its own buffers. */
#ifndef WORD
#define WORD(a,o) (*(u32 *)((a)+(o)))
#endif
#ifndef BYTE
#define BYTE(a,o) (*(u8 *)((a)+(o)))
#endif
/* The shared types of the core, before the services that it declares. */
#include "core.h"
#include "profile.h"
#define PAGE 6U
/* GRKIT_HOST_RETURN: a native page (the parameter page or a settings page)
   is opening (1) or open (2). ADJ exited while GRKit was open (3), so the next
   ADJ opens GRKit. MENU or OK on GRKit passes the event that closes ADJ to the
   native list (4); the exit then sets 3. Only an exit sets 3, because leaving
   GRKit refreshes ADJ. RETURN_ROOT marks a page that opened from the root, so
   the page returns to the root instead of the page of the module. */
#define RETURN_OPENING 1U
#define RETURN_PARAMETERS 2U
#define RETURN_GRKIT 3U
#define RETURN_CLOSING 4U
#define RETURN_ROOT 0x10U
#define RETURN_STATE(value) ((value)&~RETURN_ROOT)
/* The return state after an ADJ exit. `in_grkit` is set for an exit while
   GRKit was open. */
static u32 exit_return(u32 value,u32 in_grkit) {
    if(RETURN_STATE(value)==RETURN_OPENING) return RETURN_PARAMETERS|(value&RETURN_ROOT);
    return in_grkit || value==RETURN_CLOSING ? RETURN_GRKIT : 0;
}
/* Menu C is Thumb-2 for size. Firmware hooks and thunks enter with A32 B/BL,
   which cannot change state, so their targets stay A32. */
#ifdef __arm__
#define A32_ENTRY __attribute__((target("arm")))
#else
#define A32_ENTRY
#endif
static void grkit_touch(u32 controller);
static void render(u32 controller);
static void render_step(u32 controller,u32 step);
static void enter(u32 controller,u32 edge);
static void ui_native_returned(volatile u32 *h);
static const u16 grkit_title[] = {'G','R','K','i','t',0};
#ifndef MENU_TEXT
/* The menu text at a camera address. Host tests map the address. */
#define MENU_TEXT(address) ((const u16 *)(address))
#endif
#ifndef MENU_LANGUAGE
/* The camera's UI language byte (runtime/targets): 0 is English, 17
   Korean, 18 Traditional Chinese, 19 Simplified Chinese, and 20 Japanese. */
static u32 camera_language(void) {
#if defined(UI_SETTINGS_SLOT)
    u32 settings=WORD(UI_SETTINGS_SLOT,0);
    return settings ? BYTE(settings,LANGUAGE_SETTING) : 0;
#elif defined(LANGUAGE_SETTING_BYTE)
    return BYTE(LANGUAGE_SETTING_BYTE,0);
#else
#error "The build header defines LANGUAGE_SETTING_BYTE, or UI_SETTINGS_SLOT and LANGUAGE_SETTING"
#endif
}
/* The camera language as a menu language (runtime/abi.h), or English. */
static u32 camera_menu_language(void) {
    switch (camera_language()) {
    case 19: return 1;
    case 18: return 2;
    case 20: return 3;
    case 17: return 4;
    default: return 0;
    }
}
#define MENU_LANGUAGE() camera_menu_language()
#endif
static u32 menu_language(void) {
    u32 language=MENU_LANGUAGE();
    return language<GRKIT_TEXT_LANGUAGES ? language : 0;
}
/* Whether the menu language puts spaces between words and uses ASCII
   parentheses, as English and Korean do. Chinese and Japanese do not. */
static u32 menu_spaced(void) {
    u32 language=menu_language();
    return language==0 || language==4;
}
/* The language table index that the text hooks of menu.S pass on for GRKit
   text. It selects the font: a Chinese, Japanese, or Korean font draws that
   text, and the Latin table index 1 draws English. Such text with index 1
   shows "?" glyphs. The firmware maps the UI language byte to this index:
   Korean and Chinese keep their bytes 17, 18, and 19, and Japanese (byte 20)
   uses index 0.
   The Korean font draws ASCII and the Hangul of KS X 1001 only
   (Language::undrawable). */
A32_ENTRY u32 menu_table_index(void) {
    switch (menu_language()) {
    case 1: return 19;
    case 2: return 18;
    case 3: return 0;
    case 4: return 17;
    default: return 1;
    }
}
/* One string of the label table in the menu language. */
static const u16 *text(volatile u32 *h,u32 id) {
    u32 base=GRKIT_TEXT(h);
    return MENU_TEXT(base+MENU_TEXT(base)[menu_language()*GRKIT_TEXT_STRINGS+id]);
}

/* A wrapped stock navigation result places the private entry across that edge. */
static u32 wrapped_edge(u32 kind,u32 old,u32 next,u32 movie) {
    if(movie==1) return 0;
    if(kind==EV_RIGHT && next<=old) return 2;
    if(kind==EV_LEFT && next>=old) return 1;
    return 0;
}
static u32 advance_from_private(u32 kind,u32 edge) {
    return (kind==EV_RIGHT && edge==1) || (kind==EV_LEFT && edge==2);
}
/* Match the native ActionModel: only the initial press starts an action. */
static u32 menu_press(u32 phase,u32 repeats) { return phase==2 && repeats==0; }
/* MENU emits a switch event followed by the same confirm used by OK. */
static u32 back_event(volatile u32 *h,u32 kind,u32 enabled) {
    if(kind==EV_SWITCH_END) h[GRKIT_HOST_BACK_PENDING/4]=0;
    if(kind==EV_SWITCH && enabled) {
        if(h[GRKIT_HOST_BACK_PENDING/4]) return 2;
        h[GRKIT_HOST_BACK_PENDING/4]=1;
        return 1;
    }
    if(kind==EV_CONFIRM && h[GRKIT_HOST_BACK_PENDING/4]) {
        h[GRKIT_HOST_BACK_PENDING/4]=0;
        return 2;
    }
    return 0;
}
/* MENU closes native ADJ with its switch event (5) followed by the OK confirm. */
static u32 after_menu_switch(volatile u32 *h) {
    return (h[GRKIT_HOST_MENU_EVENT_RING/4+((h[GRKIT_HOST_MENU_EVENTS/4]-2)&(GRKIT_HOST_MENU_EVENT_RING_WORDS-1))]&0xffff)==EV_SWITCH;
}
/* An ADJ press emits EV_ADJ_PRESS followed by the confirm that OK also sends. */
static u32 after_adj_press(volatile u32 *h) {
    return (h[GRKIT_HOST_MENU_EVENT_RING/4+((h[GRKIT_HOST_MENU_EVENTS/4]-2)&(GRKIT_HOST_MENU_EVENT_RING_WORDS-1))]&0xffff)==EV_ADJ_PRESS;
}
static u32 native_action_release(u32 action,u32 active) {
    return action==10 && active;
}
#ifndef MENU_LEVEL_ALARM
#define MENU_LEVEL_ALARM(screen,mode) CALL2(FW_LEVEL_ALARM)(screen,mode)
#endif
/* The live-view highlight warning takes its threshold from the effect when
   the camera sets the warning (runtime/level.h: framing_level_value). A
   change of the effect sets the warning again with its last screen and mode,
   while the warning is on. */
static void level_reapply(volatile u32 *h) {
    if(h[GRKIT_HOST_LEVEL_SCREEN/4] && h[GRKIT_HOST_LEVEL_MODE/4])
        MENU_LEVEL_ALARM(h[GRKIT_HOST_LEVEL_SCREEN/4],h[GRKIT_HOST_LEVEL_MODE/4]);
}
#include "tone.h"
static u32 selected_icon(volatile u32 *h) {
    u32 i=record_index(h,h[GRKIT_HOST_ACTIVE/4]);
    return i<GRKIT_TEXT_RECORDS ? 0x8000+i : 0x7fff;
}
/* Override only the native still-viewfinder icon result. Off retains stock UI. */
A32_ENTRY u32 menu_viewfinder_icon_select(u32 native_icon,u32 movie) {
    volatile u32 *h=menu_header();
    u32 start=profile_begin(h), icon=selected_icon(h);
    profile_end(h,start,GRKIT_PROFILE_MENU_ICON);
    return !movie && icon!=0x7fff ? icon : native_icon;
}
#ifndef MENU_FRAMING_REQUEST
/* The family header sends the native aspect request again after a Framing
   change, so the capture hook maps it to the new ratio. */
#error "The family header defines MENU_FRAMING_REQUEST"
#endif
#if !defined(MENU_NATIVE_ASPECT) || !defined(MENU_SET_NATIVE_ASPECT) || !defined(MENU_STOP_LIVE_VIEW)
/* Framing sets the native aspect for a ratio that the camera has natively. */
#error "The family header defines MENU_NATIVE_ASPECT, MENU_SET_NATIVE_ASPECT, and MENU_STOP_LIVE_VIEW"
#endif
#ifndef TOOLBAR_CREATED
/* The first full ADJ draw follows the toolbar creation. */
#define TOOLBAR_CREATED(widget,icon) ((void)0)
#endif
/* Menu services (runtime/core.h). */
/* The Image Control summary of effect n. The installer writes one for each
   record in each language (runtime/abi.h). */
static const u16 *effect_summary(volatile u32 *h,u32 n) {
    return MENU_TEXT(GRKIT_TEXT(h)+GRKIT_TEXT_SUMMARY+
        (menu_language()*GRKIT_TEXT_RECORDS+n)*GRKIT_SUMMARY_UNITS*2);
}
/* The capture aspect (CORE.ASPECT). A native aspect change stops the live
   view first where the family needs that, as the native MENU does. */
static u32 aspect_native(void) { return MENU_NATIVE_ASPECT(); }
static void aspect_set_native(u32 id) { MENU_STOP_LIVE_VIEW(); MENU_SET_NATIVE_ASPECT(id); }
static void aspect_request(void) { MENU_FRAMING_REQUEST(); }
/* A new effect sets the highlight warning again. */
static void tone_stored(volatile u32 *h) { level_reapply(h); }
#include "modules.h"
/* The core exports this service to modules (runtime/sdk/grkit.h). It runs
   in the context of its caller. */
__attribute__((used)) void grkit_export_setting_set(u32 handle,u32 value) { setting_store(menu_header(),handle,value); }
static u32 private_mode(u32 controller) {
    volatile u32 *h=menu_header();
    return h[GRKIT_HOST_MENU_CONTROLLER/4] == controller && h[GRKIT_HOST_MENU_LEVEL/4] != 0;
}
/* A native Image Control preview lasts while its ADJ slot has focus. */
static void native_focus(u32 controller) {
    if(CALL2(FW_ADJ_SLOT_TYPE)(controller,BYTE(controller,ADJ_SLOT))!=ADJ_IC_SLOT_TYPE)
        menu_header()[GRKIT_HOST_PREVIEW/4]=0;
}
/* Fn opens the native parameter page from an effect row in the still
   context. The native Fn hint shows only where this works. */
static u32 parameters_available(u32 controller) {
    /* The page edits the adjustments of the effect (menu_parameter_*), so the
       native Image Control type does not matter. */
    return parameters_row(menu_header()) && ADJ_MOVIE(controller)!=1;
}
/* Native slot indices stay in 0..4. Only the transient toolbar has six cells. */
static u32 toolbar_x(u32 slot) { return 415 + 50 * slot; }
static void toolbar_destroy(u32 widget) {
    /* Both sibling objects share an allocation; only LineView releases it. */
    CALL1(FW_ICON_VIEW_DESTROY)(widget+0xf0);
    CALL1(FW_LINE_VIEW_DESTROY)(widget);
}
static void toolbar_visible(u32 widget,u32 visible) {
    CALL2(FW_VIEW_SET_VISIBLE)(widget,visible);
    CALL2(FW_VIEW_SET_VISIBLE)(widget+0xf0,visible);
}
static void toolbar_layout(u32 controller,u32 expanded) {
    u32 i, origin=expanded ? 415 : 425, pitch=expanded ? 50 : 60;
    for(i=0;i<5;++i) {
        CALL2(FW_VIEW_SET_X)(WORD(controller,ADJ_CELLS+i*4),origin+pitch*i);
        CALL2(FW_VIEW_SET_X)(WORD(controller,ADJ_ICONS+i*4),origin+pitch*i+5);
    }
    CALL2(FW_VIEW_SET_X)(WORD(controller,ADJ_FRAME),expanded ?
        toolbar_x(private_mode(controller) ? 5 : BYTE(controller,ADJ_SLOT)) :
        425+60*BYTE(controller,ADJ_SLOT));
}
static void toolbar(u32 controller) {
    volatile u32 *h=menu_header();
    u32 widget=h[GRKIT_HOST_TOOLBAR_WIDGET/4];
    if(widget && h[GRKIT_HOST_TOOLBAR_OWNER/4]!=controller) return;
    if(ADJ_MOVIE(controller)==1) {
        if(widget) toolbar_visible(widget,0);
        toolbar_layout(controller,0);
        return;
    }
    if(!widget) {
        u32 parent=WORD(WORD(controller,ADJ_ICONS),4);
        u32 position=WORD(parent,0xc), end=WORD(parent,0x10), icon, offset;
        u32 binding[3]={(u32)grkit_touch,0,controller};
        u32 callback[4]={(u32)binding,0,FW_ADJ_TOUCH_ENTER,FW_ADJ_TOUCH_LEAVE};
        /* Draw the native background before the shared selection frame. */
        while(position!=end && WORD(position,0)!=WORD(controller,ADJ_FRAME)) position+=4;
        if(position==end) return;
        offset=position-WORD(parent,0xc);
        widget=CALL1(FW_ALLOCATE)(0x1e8);
        if(!widget) return;
        icon=widget+0xf0;
        CALL5(FW_LINE_VIEW_INIT)(widget,toolbar_x(5),5,50,50);
        CALL2(FW_LINE_VIEW_SET_STYLE)(widget,0x10);
        CALL5(FW_ICON_VIEW_INIT)(icon,toolbar_x(5)+5,10,40,40);
        CALL2(FW_ICON_VIEW_SET_ICON)(icon,GRKIT_ICON_GRKIT);
        CALL2(FW_VIEW_SET_TOUCH)(icon,(u32)callback);
        CALL2(FW_VIEW_SET_TOUCHABLE)(icon,1);
        /* Use the same vector insertion as View::AddChild, at the marker. */
        if(!CALL3(FW_VECTOR_INSERT)(parent+8,position,(u32)&widget)) {
            toolbar_destroy(widget);
            return;
        }
        /* The icon must draw after the frame, which fills the entire cell. */
        if(!CALL3(FW_VECTOR_INSERT)(parent+8,WORD(parent,0x10),(u32)&icon)) {
            CALL2(FW_VECTOR_ERASE)(parent+8,WORD(parent,0xc)+offset);
            toolbar_destroy(widget);
            return;
        }
        CALL2(FW_VIEW_SET_PARENT)(widget,parent);
        CALL2(FW_VIEW_SET_PARENT)(icon,parent);
        TOOLBAR_CREATED(widget,icon);
        h[GRKIT_HOST_TOOLBAR_WIDGET/4]=widget;
        h[GRKIT_HOST_TOOLBAR_OWNER/4]=controller;
    }
    toolbar_visible(widget,1);
    toolbar_layout(controller,1);
    CALL2(FW_VIEW_SET_VISIBLE)(WORD(controller,ADJ_FRAME),1);
}
/* The parameter page title of a GRKit session (GRKIT_TEXT_PARAMETER_TITLE). */
static volatile u32 *parameter_title(volatile u32 *h) {
    return (volatile u32 *)(GRKIT_TEXT(h)+GRKIT_TEXT_PARAMETER_TITLE);
}
A32_ENTRY void menu_refresh(u32 controller) {
    volatile u32 *h=menu_header();
    /* ADJ runs, so no parameter page shows its title. */
    parameter_title(h)[0]=0;
    if(private_mode(controller)) { render(controller); return; }
    if(RETURN_STATE(h[GRKIT_HOST_RETURN/4])==RETURN_PARAMETERS && ADJ_MOVIE(controller)!=1) {
        u32 root=h[GRKIT_HOST_RETURN/4]&RETURN_ROOT;
        h[GRKIT_HOST_RETURN/4]=0;
        /* The page has ended. A record that no page started stays unused. */
        if(h[GRKIT_HOST_PARAMETERS/4]&1U) h[GRKIT_HOST_PARAMETERS/4]=0;
        /* GRKit reopens at the root row of the module (menu_reopen). A page
           that opened from a page returns to that page: Fn on a module
           page. The page is visible, because it opened the other page. */
        enter(controller,3);
        if(!root) {
            menu_open(h,h[GRKIT_HOST_MODULE/4]);
            render(controller);
        }
        ui_native_returned(h);
        return;
    }
    if(h[GRKIT_HOST_RETURN/4]==RETURN_GRKIT) {
        h[GRKIT_HOST_RETURN/4]=0;
        /* ADJ opens on GRKit, as a native ADJ opens on its last entry. The
           native refresh first sets the list of the native slot, so that
           leaving GRKit restores the value of that slot. */
        if(ADJ_MOVIE(controller)!=1) {
            menu_refresh_original(controller);
            enter(controller,h[GRKIT_HOST_MENU_EDGE/4]);
            return;
        }
    }
    menu_refresh_original(controller);
    toolbar(controller);
    frame_preview(h,controller,ADJ_MOVIE(controller)==1);
}
/* First row of the cursor's page. The reciprocal equals row/6 for rows below 2^15
   and avoids a division helper, which the freestanding link does not provide. */
static u32 page_start(u32 row) { return (row*0xaaabU>>18)*PAGE; }
/* The list has as many rows as the longest native slot of the controller
   (32 on GR III). It holds all rows of the root or page when they fit, and
   scrolls them as a native list does. Otherwise it holds the page of six rows
   around the cursor. Each native ADJ list holds more than six rows. */
static u32 list_capacity(u32 view) { return (WORD(view,LIST_ROWS+4)-WORD(view,LIST_ROWS))>>2; }
static u32 list_start(volatile u32 *h,u32 view) {
    return count(h)>list_capacity(view) ? page_start(h[GRKIT_HOST_MENU_CURSOR/4]) : 0;
}
static void descriptor(volatile u32 *at, const u16 *name) {
    u32 length=0;
    while (length<GRKIT_SUMMARY_UNITS-1 && name[length]) ++length;
    at[0]=(length+1)|0x100; at[1]=(u32)name;
}
static void label(u32 object) {
    CALL3(FW_STRING_VIEW_RESET)(object,0,0);
    BYTE(object,0xf3)=1;
    CALL1(FW_VIEW_INVALIDATE)(object);
    CALL2(FW_VIEW_SET_VISIBLE)(object,1);
}
static __attribute__((noinline)) void detail_width(u32 controller,u32 width) {
    u32 object=WORD(controller,ADJ_DETAIL), rectangle[4]={0,50,width,40};
    CALL2(WORD(WORD(object,0),0x18))(object,(u32)rectangle);
}
/* Size the list for `rows` rows, as the native refresh does for the values of
   a slot: rows are 60 pixels high, a short list ends at y=420, and six or more
   rows fill y=60..420 and scroll. The private menu replaces the native
   refresh, so without this the list keeps the size of the last native slot. */
static void list_frame(u32 view,u32 rows) {
    u32 width=CALL1(FW_LIST_WIDTH)(view), rectangle[4];
    if(rows>PAGE) rows=PAGE;
    rectangle[0]=720-width; rectangle[1]=60+60*(PAGE-rows);
    rectangle[2]=width; rectangle[3]=60*rows;
    CALL2(WORD(WORD(view,0),0x18))(view,(u32)rectangle);
}
/* Show the Fn hint where Fn opens the parameter page or the settings of a
   module. The native refresh fills the hint only on a native slot that
   offers Fn, so a hint that GRKit shows could be empty. Fill it as the
   native Image Control slot does. */
static u32 settings_target(volatile u32 *h,u32 controller,u32 fn);
static void fn_guide(u32 controller) {
    volatile u32 *h=menu_header();
    u32 guide=WORD(controller,ADJ_FN_GUIDE), shown=parameters_row(h) ? parameters_available(controller) :
        settings_target(h,controller,1)!=MODULE_TOTAL;
    if(shown) {
        CALL4(FW_GUIDE_SET_CONTENT)(guide,FN_GUIDE_TEXT,10,FN_GUIDE_ICON);
#ifdef FN_GUIDE_X
        CALL2(FW_VIEW_SET_X)(guide,FN_GUIDE_X);
#endif
    }
    CALL2(FW_VIEW_SET_VISIBLE)(guide,shown);
}
/* Put the list cursor on the cursor row. After one Up or Down step (`step`
   is 1 or -1), move it as the native list does: the list keeps one row of
   context, animates the scroll, and wraps at the ends. If the list keeps its
   rows and is already on the row, as after a touch or a refresh, it keeps its
   scroll. Otherwise a jump scrolls the row into view without animation. */
static void list_cursor(u32 view,u32 row,u32 rows,u32 step,u32 kept) {
    u32 at=CALL1(FW_LIST_CURSOR)(view);
    u32 before=step==1 ? (row ? row-1 : rows-1) : (row+1==rows ? 0 : row+1);
    if(kept && step && at==before) CALL3(FW_LIST_MOVE)(view,step,1);
    else if(!kept || at!=row) CALL2(FW_LIST_SET_CURSOR)(view,row);
}
static void render(u32 controller) { render_step(controller,0); }
static void render_step(u32 controller,u32 step) {
    volatile u32 *h=menu_header();
    u32 cursor, start, n, i, total, kept;
    u32 view=WORD(controller,ADJ_LIST);
    const MenuPage *module;
    menu_normalize(h);
    cursor=h[GRKIT_HOST_MENU_CURSOR/4]; total=count(h);
    start=list_start(h,view); n=total-start;
    module=current_module(h);
    if(total>list_capacity(view)) {
        /* Paged rows move one page at a time. */
        step=0;
        if(n>PAGE) n=PAGE;
    }
    kept=BYTE(view,LIST_COUNT)==n;
    /* Suppress the native list callback while replacing private rows. */
    h[GRKIT_HOST_MENU_REBUILDING/4]=1;
    CALL2(FW_LIST_SET_COUNT)(view,n);
    for(i=0;i<n;++i) {
        u32 row=CALL2(FW_LIST_ROW)(view,i);
        u32 index=start+i;
        /* Private icon IDs are resolved by the host's guarded icon adapter. */
        u32 owner=h[GRKIT_HOST_MENU_LEVEL/4]==MENU_ROOT ? root_module(h,index) : h[GRKIT_HOST_MODULE/4];
        u32 icon=module_icon(owner,menu_page(h,owner)->icon(h,index));
        CALL2(FW_ROW_SET_ICON)(row,icon);
        CALL2(FW_ROW_SET_CHECK)(row,h[GRKIT_HOST_MENU_LEVEL/4]!=MENU_ROOT && module->checked(h,index));
    }
    /* The rows selected other module pages (menu_page). */
    module=current_module(h);
    /* The size comes from all rows, so that a short last page keeps the
       position of the full pages. The cursor needs the new size. */
    list_frame(view,total);
    list_cursor(view,cursor-start,n,step,kept);
    descriptor(h+GRKIT_HOST_MENU_TITLE/4,h[GRKIT_HOST_MENU_LEVEL/4]==MENU_ROOT ? grkit_title : module->title(h));
    descriptor(h+GRKIT_HOST_MENU_DETAIL/4,h[GRKIT_HOST_MENU_LEVEL/4]==MENU_ROOT ? module->summary(h) : module->label(h,cursor));
    label(WORD(controller,ADJ_TITLE));
    /* The summary ends before the list at x=620; native labels use 400 pixels. */
    detail_width(controller,h[GRKIT_HOST_MENU_LEVEL/4]==MENU_ROOT ? 620 : 400);
    label(WORD(controller,ADJ_DETAIL));
    /* Native detail controls do not describe private modules, except the Fn
       hint (+0x1ac) where Fn opens the native parameters. */
    CALL2(FW_VIEW_SET_VISIBLE)(WORD(controller,ADJ_DETAIL_VALUE),0);
    fn_guide(controller);
    toolbar(controller);
    /* Slot zero's canvas is at +4, as in the live view. */
    frame_preview(h,controller,ADJ_MOVIE(controller)==1);
    h[GRKIT_HOST_MENU_REBUILDING/4]=0;
}
static void leave(u32 controller) {
    volatile u32 *h=menu_header();
    if(!private_mode(controller)) return;
    /* The next ADJ session opens at the row of this module (menu_reopen). */
    if(h[GRKIT_HOST_MENU_LEVEL/4]==MENU_ROOT) h[GRKIT_HOST_MODULE/4]=root_module(h,h[GRKIT_HOST_MENU_CURSOR/4]);
    h[GRKIT_HOST_MENU_LEVEL/4]=0; h[GRKIT_HOST_MENU_REBUILDING/4]=1;
    h[GRKIT_HOST_PREVIEW/4]=0;
    detail_width(controller,400);
    /* Rebuild the original list before its exit callback saves a cursor. */
    CALL1(FW_ADJ_REFRESH)(controller);
    CALL2(FW_VIEW_SET_VISIBLE)(WORD(controller,ADJ_FRAME),1);
    CALL3(FW_LIST_RESTORE)(WORD(controller,ADJ_LIST),h[GRKIT_HOST_MENU_LIST/4],h[GRKIT_HOST_MENU_LIST/4+1]);
    h[GRKIT_HOST_MENU_CONTROLLER/4]=0; h[GRKIT_HOST_MENU_REBUILDING/4]=0;
}
static void enter(u32 controller,u32 edge) {
    volatile u32 *h=menu_header();
    u32 view=WORD(controller,ADJ_LIST);
    h[GRKIT_HOST_MENU_CONTROLLER/4]=controller; menu_reopen(h);
    h[GRKIT_HOST_MENU_EDGE/4]=edge;
    h[GRKIT_HOST_PREVIEW/4]=0;
    CALL3(FW_LIST_SAVE)(view,(u32)(h+GRKIT_HOST_MENU_LIST/4),(u32)(h+GRKIT_HOST_MENU_LIST/4+1));
    render(controller);
}
static void grkit_touch(u32 controller) {
    if(!private_mode(controller) && ADJ_MOVIE(controller)!=1) enter(controller,3);
}
/* Fn adjustments. While GRKIT_HOST_PARAMETERS holds a record, the native
   parameter page reads and writes the adjustments of that record
   (GRKIT_RECORD_PARAMETERS) instead of the native settings, and shows the
   Standard rows for any native type. The page snapshot holds the record's
   values, so a cancel writes them back to the record. Native settings do not
   change. Only the page calls the hooked model getters and setters. */
static const u8 parameter_kinds[]=PARAMETER_KINDS;
_Static_assert(sizeof(parameter_kinds)==9, "GRKIT_RECORD_PARAMETERS holds nine adjustments");
/* The record of a started page, or 0. */
A32_ENTRY u32 menu_parameter_session(void) {
    u32 record=menu_header()[GRKIT_HOST_PARAMETERS/4];
    return record&1U ? 0 : record;
}
/* The native value 0..8 of a stored byte: (native - 4) mod 9. */
static u32 parameter_native(u32 record,u32 index) {
    u32 stored=BYTE(record,GRKIT_RECORD_PARAMETERS+index);
    return stored>8 ? 4 : stored<5 ? stored+4 : stored-5;
}
/* The native value plus 1, or 0 to run the native getter. */
A32_ENTRY u32 menu_parameter_get(u32 index) {
    u32 record=menu_parameter_session();
    return record ? parameter_native(record,index)+1 : 0;
}
/* Nonzero when the value went to the record. */
A32_ENTRY u32 menu_parameter_set(u32 index,u32 value) {
    volatile u32 *h=menu_header();
    u32 record=menu_parameter_session();
    if(!record) return 0;
    if(value<=8) {
        BYTE(record,GRKIT_RECORD_PARAMETERS+index)=(u8)(value>=4 ? value-4 : value+5);
        ++h[GRKIT_HOST_PARAMETER_CHANGES/4];
        GRKIT_STATE_CHANGED(h);
    }
    return 1;
}
/* A page that GRKit opened starts its session. A start while the session
   runs is the same page again: a half-press changes from the page to the
   shooting screen and back (menu_parameter_controller). A controller change
   to any other screen ends the session, so a native page starts without
   one. */
static void menu_parameter_started(volatile u32 *h) {
    u32 record=h[GRKIT_HOST_PARAMETERS/4];
    if(record&1U) h[GRKIT_HOST_PARAMETERS/4]=record&~1U;
}
A32_ENTRY void menu_parameter_page_start(void) {
    volatile u32 *h=menu_header();
    menu_parameter_started(h);
    parameter_title(h)[0]=0;
}
/* A controller change to `id` while a session runs: it stays for the page
   and for the shooting screen that a half-press shows over the page, and
   ends for any other controller. A record that waits for its page to start
   (bit 0) stays: ADJ opens the page with it. */
static void menu_parameter_controller(volatile u32 *h,u32 id) {
    u32 record=h[GRKIT_HOST_PARAMETERS/4];
    if(!record || record&1U || id==IC_PARAMETERS_CONTROLLER || id==SHOOTING_CONTROLLER) return;
    h[GRKIT_HOST_PARAMETERS/4]=0;
}
/* Show the Standard rows of the base set. The type is Standard, so the
   native title would read Standard. Keep the page and the effect name, so
   that the text hooks draw the name in the title (menu_parameter_title). */
A32_ENTRY void menu_parameter_page_kinds(u32 page) {
    u32 record=menu_parameter_session();
    volatile u32 *title;
    /* The title view keeps its measured width while its text ID stays the
       same, which clipped a longer effect name. Clear the text, so that the
       title measures the text that the page sets next. */
    CALL3(FW_STRING_VIEW_RESET)(WORD(page,DETAIL_TITLE),0,0);
    if(!record) return;
    BYTE(page,DETAIL_TYPE)=11;
    BYTE(page,DETAIL_SET_KIND)=0;
    BYTE(page,DETAIL_BASE_KIND)=0;
    title=parameter_title(menu_header());
    descriptor(title+2,(const u16 *)(record+GRKIT_RECORD_NAME));
    title[1]=WORD(page,0);
    title[0]=page;
}
/* Replace the snapshot of the Standard adjustments with the record's values. */
A32_ENTRY void menu_parameter_page_snapshot(u32 model) {
    u32 record=menu_parameter_session(),i;
    if(!record) return;
    for(i=0;i<sizeof(parameter_kinds);++i)
        BYTE(model,MODEL_SNAPSHOT+parameter_kinds[i])=(u8)parameter_native(record,i);
}
/* The text descriptor for `view`: the effect name for the title of a page
   that GRKit started, or 0 for native text. A page whose first word is no
   longer its vtable has ended, and its memory may hold another object. */
static u32 menu_parameter_title(u32 view) {
    volatile u32 *title;
    u32 page;
    if(!menu_parameter_session()) return 0;
    title=parameter_title(menu_header());
    page=title[0];
    if(page && WORD(page,0)==title[1] && WORD(page,DETAIL_TITLE)==view) return (u32)(title+2);
    return 0;
}
/* The value view of a MENU or submenu row starts its column of VALUE_COLUMN
   pixels: a MENU row view is that wide, and a submenu row view as wide as
   its text, which the row clips. Call after the native value setter, which
   sets the frame. A longer text moves the view left, so that the text ends
   VALUE_GAP pixels before the column ends, which leaves room for the arrow
   of a submenu row. It moves at most VALUE_SHIFT pixels: the labels of
   GRKit rows end before that. */
#define VALUE_GAP 8
#define VALUE_SHIFT 190
static void fit_value(u32 view) {
    volatile int *frame=(volatile int *)(view+VIEW_FRAME);
    int width=(int)MENU_VIEW1(FW_STRING_VIEW_WIDTH,view), room=VALUE_COLUMN-VALUE_GAP, rect[4];
    if(width<=room) return;
    if(width>room+VALUE_SHIFT) width=room+VALUE_SHIFT;
    rect[0]=frame[0]+room-width; rect[1]=frame[1]; rect[2]=width; rect[3]=frame[3];
    CALL2(WORD(WORD(view,0),VIEW_SET_FRAME))(view,(u32)rect);
}
#include "menu_settings.h"
#include "menu_section.h"
#include "menu_about.h"
/* The timer and UI services for modules. */
#include "ui.h"
/* The text descriptor for `view` and text ID `id`, or 0 for the native text.
   Bit 0 selects the font of the menu language. GRKit gives the texts of one
   screen, which the last controller change selects (MENU_ROWS_CURRENT): the
   camera information page shows About GRKit, the borrowed submenu and its
   value pages show a settings page, and another screen can show the rows of
   the GRKit section. So the screens can use the same text IDs: the rows of
   the GRKit section use those of the borrowed rows (MENU_ROW_LABEL_TEXT). */
A32_ENTRY u32 menu_text_override(u32 view,u32 id) {
    volatile u32 *h=menu_header();
    u32 title=menu_parameter_title(view);
    if(title) return title;
    title=ui_text_override(h,id);
    if(title) return title;
    if(h[GRKIT_HOST_SERVICES/4]) {
        u32 current=services_word(h,SERVICES_MENU+MENU_ROWS_CURRENT)[0];
        if(current==INFO_CONTROLLER) return info_text(h,id);
        if(settings_shows(current)) return settings_text(h,view,id);
        return menu_rows_text(h,view,id);
    }
    return settings_text(h,view,id);
}
/* Fn on a root row or a module page with settings opens the native
   settings page where the target has one. The page returns to where it
   opened (menu_refresh). */
/* The module whose settings Fn opens at the cursor, or MODULE_TOTAL: Fn on
   a root row or on a module page. OK opens none. */
static u32 settings_target(volatile u32 *h,u32 controller,u32 fn) {
    u32 module, level=h[GRKIT_HOST_MENU_LEVEL/4];
    const MenuPage *page;
    if(ADJ_MOVIE(controller)==1) return MODULE_TOTAL;
    if(level==MENU_ROOT) {
        if(!fn) return MODULE_TOTAL;
        module=root_module(h,h[GRKIT_HOST_MENU_CURSOR/4]);
    }
    else if(fn && level==MENU_MODULE) module=h[GRKIT_HOST_MODULE/4];
    else return MODULE_TOTAL;
    page=menu_page(h,module);
    if(!page->settings) return MODULE_TOTAL;
    /* A page with one settings row has it in MENU only: it is the page. */
    if(page->settings==choice_settings && PAGE_WORD(CHOICE_ENTRY(h)[0],PAGE_SETTING_ROWS)<2) return MODULE_TOTAL;
    return module;
}
static u32 open_settings(u32 controller,u32 fn) {
    volatile u32 *h=menu_header();
    u32 module=settings_target(h,controller,fn), root=h[GRKIT_HOST_MENU_LEVEL/4]==MENU_ROOT;
    if(module==MODULE_TOTAL) return 0;
    leave(controller);
    settings_start(h,module);
    h[GRKIT_HOST_RETURN/4]=RETURN_OPENING|(root ? RETURN_ROOT : 0);
    CALL3(FW_CHANGE_CONTROLLER)(controller,SETTINGS_SUBMENU,0x2000000);
    return 1;
}
/* Open the native parameter page, as the native ADJ Image Control slot does
   for event 0x1b with argument 2. Refresh the model snapshot first: a cancel
   restores its type persistently. */
static u32 open_parameters(u32 controller) {
    if(!parameters_available(controller)) return 0;
    /* leave() clears the level, so read it first. */
    u32 root=menu_header()[GRKIT_HOST_MENU_LEVEL/4]==MENU_ROOT ? RETURN_ROOT : 0;
    CALL1(FW_ADJ_SNAPSHOT)(controller);
    /* The page edits the adjustments of the committed effect. Bit 0 marks a
       page that has not started yet. */
    menu_header()[GRKIT_HOST_PARAMETERS/4]=menu_header()[GRKIT_HOST_ACTIVE/4]|1U;
    leave(controller);
    /* The page returns to ADJ (0x2000000) with the native Image Control slot
       focused, where OK would confirm Standard. Come back to GRKit instead. */
    menu_header()[GRKIT_HOST_RETURN/4]=RETURN_OPENING|root;
    CALL3(FW_CHANGE_CONTROLLER)(controller,IC_PARAMETERS_CONTROLLER,0x2000000);
    return 1;
}
A32_ENTRY void menu_native_touch(u32 controller,u32 slot) {
    leave(controller);
    if(BYTE(controller,ADJ_SLOT)!=slot) CALL2(FW_ADJ_SELECT_SLOT)(controller,slot);
    native_focus(controller);
}
/* GRKit leaves a page itself on OK, MENU, Fn, Left, Right, and touch. An
   ADJ exit that GRKit did not start, such as a half-press of the shutter,
   keeps the highlighted row of a page, as a native ADJ keeps the value under
   its cursor. The Modules page opens settings on OK, so it commits nothing. */
static void exit_commit(volatile u32 *h) {
    if(h[GRKIT_HOST_MENU_LEVEL/4]==MENU_MODULE && h[GRKIT_HOST_MODULE/4]!=MANAGE_MODULE &&
       !(current_module(h)->flags&MENU_PAGE_ACTIONS)) menu_confirm(h);
}
A32_ENTRY void menu_exit(u32 controller) {
    volatile u32 *h=menu_header();
    u32 in_grkit=private_mode(controller);
    if(in_grkit) exit_commit(h);
    /* A box of the UI service closes with ADJ, and a native screen that
       waits to open does not open. */
    ui_close(h);
    ui_native_cancel(h);
    leave(controller);
    h[GRKIT_HOST_BACK_PENDING/4]=0;
    h[GRKIT_HOST_PREVIEW/4]=0;
    h[GRKIT_HOST_FRAME_PREVIEW/4]=0;
    /* The next ADJ returns to the parameter page's GRKit page, opens on
       GRKit, or opens as native ADJ does (exit_return). */
    h[GRKIT_HOST_RETURN/4]=exit_return(h[GRKIT_HOST_RETURN/4],in_grkit);
    if(RETURN_STATE(h[GRKIT_HOST_RETURN/4])!=RETURN_PARAMETERS && (h[GRKIT_HOST_PARAMETERS/4]&1U)) h[GRKIT_HOST_PARAMETERS/4]=0;
    parameter_title(h)[0]=0;
    menu_exit_original(controller);
    if(menu_header()[GRKIT_HOST_TOOLBAR_OWNER/4]==controller) {
        toolbar_visible(menu_header()[GRKIT_HOST_TOOLBAR_WIDGET/4],0);
        toolbar_layout(controller,0);
    }
}
A32_ENTRY u32 menu_destroy(u32 controller) {
    volatile u32 *h=menu_header();
    u32 widget=0, result;
    ui_close(h);
    ui_native_cancel(h);
    if(h[GRKIT_HOST_TOOLBAR_OWNER/4]==controller) {
        widget=h[GRKIT_HOST_TOOLBAR_WIDGET/4];
        h[GRKIT_HOST_TOOLBAR_WIDGET/4]=0;
        h[GRKIT_HOST_TOOLBAR_OWNER/4]=0;
        /* The root clears its child vector without deleting its children. */
        CALL2(WORD(WORD(widget,0),0x2c))(widget,0);
        CALL2(FW_VIEW_SET_PARENT)(widget+0xf0,0);
    }
    if(h[GRKIT_HOST_MENU_CONTROLLER/4]==controller) {
        /* A destroy without an exit keeps the highlighted row too. */
        exit_commit(h);
        h[GRKIT_HOST_MENU_CONTROLLER/4]=0; h[GRKIT_HOST_MENU_LEVEL/4]=0;
    }
    h[GRKIT_HOST_PREVIEW/4]=0;
    h[GRKIT_HOST_FRAME_PREVIEW/4]=0;
    h[GRKIT_HOST_FRAME_PREVIEW_ADJ/4]=0;
    result=menu_destroy_original(controller);
    if(widget) toolbar_destroy(widget);
    return result;
}
A32_ENTRY void menu_changed(u32 controller) {
    volatile u32 *h=menu_header();
    /* Restoration sets the level to native before rebuilding the list. */
    if(h[GRKIT_HOST_MENU_REBUILDING/4]) return;
    if(h[GRKIT_HOST_MENU_CONTROLLER/4]==controller && h[GRKIT_HOST_MENU_LEVEL/4]) {
        u32 view=WORD(controller,ADJ_LIST);
        u32 next=list_start(h,view)+CALL1(FW_LIST_CURSOR)(view);
        if(next<count(h) && next!=h[GRKIT_HOST_MENU_CURSOR/4]) { h[GRKIT_HOST_MENU_CURSOR/4]=next; highlight(h); render(controller); }
        return;
    }
    menu_changed_original(controller);
}
A32_ENTRY u32 menu_event(u32 controller,const u16 *event) {
    volatile u32 *h=menu_header();
    u32 kind=*event, back;
    h[GRKIT_HOST_MENU_EVENT_RING/4+(h[GRKIT_HOST_MENU_EVENTS/4]&(GRKIT_HOST_MENU_EVENT_RING_WORDS-1))]=kind | ((kind==EV_FN || kind==EV_SWITCH || kind==EV_SWITCH_END) ? ((u32)*(u8 *)WORD((u32)event,4)<<16) :
        kind==EV_TOUCH ? (u32)BYTE(WORD((u32)event,4),0x50)<<16 : 0);
    ++h[GRKIT_HOST_MENU_EVENTS/4];
    /* An ADJ that opened before INSTALL drew its slots without GRKit: the
       first event after INSTALL refreshes it. */
    if(h[GRKIT_HOST_SERVICES/4] && !services_word(h,SERVICES_MENU+MENU_ROWS_ADJ)[0]) {
        services_word(h,SERVICES_MENU+MENU_ROWS_ADJ)[0]=1;
        if(!private_mode(controller)) CALL1(FW_ADJ_REFRESH)(controller);
    }
    if(kind==EV_SWITCH && !menu_press(BYTE(WORD((u32)event,4),0),WORD(WORD((u32)event,4),4))) return 0;
    back=back_event(h,kind,private_mode(controller));
    if(back==2) return 0;
    /* A message, a confirm, or a progress box takes each key. */
    if(private_mode(controller) && ui_modal(h)) {
        ui_event(h,kind,back==1);
        return 0;
    }
    if(back==1) {
        if(menu_back(h)) { render(controller); return 0; }
        /* MENU on the root closes ADJ, as on a native entry: the restored
           native list gets this switch event and the confirm after it. */
        h[GRKIT_HOST_BACK_PENDING/4]=0;
        leave(controller);
        h[GRKIT_HOST_RETURN/4]=RETURN_CLOSING;
        return menu_event_original(controller,event);
    }
#ifdef EV_FN_ACTION
    /* The Fn press that opened a GRKit parameter page also sends its native
       action. The native page must not open for it. */
    if(kind==EV_FN_ACTION && (RETURN_STATE(h[GRKIT_HOST_RETURN/4])==RETURN_OPENING ||
                              RETURN_STATE(h[GRKIT_HOST_RETURN/4])==RETURN_PARAMETERS)) return 0;
#endif
    if(!private_mode(controller)) {
        /* OK or ADJ press on the native Image Control slot confirms a native value.
           Confirming the existing native type can omit its setter entirely. */
        if(kind==EV_CONFIRM && !after_menu_switch(h) &&
           CALL2(FW_ADJ_SLOT_TYPE)(controller,BYTE(controller,ADJ_SLOT))==ADJ_IC_SLOT_TYPE) {
            h[GRKIT_HOST_ACTIVE/4]=0;
            GRKIT_STATE_CHANGED(h);
            level_reapply(h);
        }
        if(ADJ_MOVIE(controller)!=1 &&
           ((kind==EV_RIGHT && BYTE(controller,ADJ_SLOT)==4) ||
            (kind==EV_LEFT && BYTE(controller,ADJ_SLOT)==0))) {
            enter(controller,kind==EV_RIGHT ? 1 : 2); return 0;
        }
        {
            u32 old=BYTE(controller,ADJ_SLOT);
            u32 result=menu_event_original(controller,event);
            u32 next=BYTE(controller,ADJ_SLOT);
            u32 edge=wrapped_edge(kind,old,next,ADJ_MOVIE(controller));
            if(edge) enter(controller,edge);
            else if(next!=old) native_focus(controller);
            return result;
        }
    }
    menu_normalize(h);
    /* Fn on a row of a page that takes it, such as a delete on the page of
       recipes. */
    if(kind==EV_FN && *(u8 *)WORD((u32)event,4)==FN_PRESS_ARG && h[GRKIT_HOST_MENU_LEVEL/4]==MENU_MODULE &&
       current_module(h)->fn && current_module(h)->fn(h,h[GRKIT_HOST_MENU_CURSOR/4])) {
        render(controller);
        return 0;
    }
    /* On an effect row, commit the highlighted row, then adjust its parameters.
       On the root row, adjust the committed effect. Fn on Off does nothing. */
    if(kind==EV_FN && *(u8 *)WORD((u32)event,4)==FN_PRESS_ARG && parameters_row(h)) {
        if(h[GRKIT_HOST_MENU_LEVEL/4]==MENU_MODULE) current_module(h)->apply(h,h[GRKIT_HOST_MENU_CURSOR/4]);
        if(!open_parameters(controller)) render(controller);
        return 0;
    }
    /* Fn elsewhere opens the detail settings of the module, if it has any. */
    if(kind==EV_FN && *(u8 *)WORD((u32)event,4)==FN_PRESS_ARG && open_settings(controller,1)) return 0;
    /* Left and Right move to the next ADJ entry from the root and from a
       page, as on a native entry. MENU returns from a page to the root. */
    if(kind==EV_LEFT || kind==EV_RIGHT) {
        u32 edge=h[GRKIT_HOST_MENU_EDGE/4], advance=advance_from_private(kind,edge);
        leave(controller);
        if(edge==3) {
            u32 i;
            for(i=0;i<5;++i) {
                u32 slot=kind==EV_LEFT ? 4-i : i;
                if(!CALL2(FW_ADJ_SLOT_TYPE)(controller,slot)) continue;
                menu_native_touch(controller,slot);
                if(BYTE(controller,ADJ_SLOT)==slot) break;
            }
            return 0;
        }
        return advance ? menu_event_original(controller,event) : 0;
    }
    if(kind==EV_UP || kind==EV_DIAL_UP || kind==EV_DOWN || kind==EV_DIAL_DOWN) {
        u32 n=count(h);
        if(kind==EV_UP || kind==EV_DIAL_UP) h[GRKIT_HOST_MENU_CURSOR/4]=h[GRKIT_HOST_MENU_CURSOR/4] ? h[GRKIT_HOST_MENU_CURSOR/4]-1 : n-1;
        else { ++h[GRKIT_HOST_MENU_CURSOR/4]; if(h[GRKIT_HOST_MENU_CURSOR/4]==n) h[GRKIT_HOST_MENU_CURSOR/4]=0; }
        highlight(h);
        render_step(controller,kind==EV_UP || kind==EV_DIAL_UP ? (u32)-1 : 1); return 0;
    }
    /* Confirm is 0xd8. Press and release events do not activate a profile.
       OK on a root row opens its page. OK on a page row commits it and closes
       ADJ, as OK on a native value does: the restored native list gets the
       confirm, as after MENU on the root. An ADJ press closes ADJ from the
       root too, as on a native entry; on a page it commits the row first.
       If the commit made a module show a UI box or ask for a native screen,
       ADJ stays open on the page instead (ui_holds_page). */
    if(kind==EV_CONFIRM) {
        u32 page=h[GRKIT_HOST_MENU_LEVEL/4]==MENU_MODULE, close=page;
        /* OK on the Modules page opens the settings of the row. */
        if(page && h[GRKIT_HOST_MODULE/4]==MANAGE_MODULE) close=0;
        if(after_adj_press(h)) close=1;
        if(!close && open_settings(controller,0)) return 0;
        if(page || !close) menu_confirm(h);
        if(close && ui_holds_page(h)) close=0;
        if(close) {
            leave(controller);
            h[GRKIT_HOST_RETURN/4]=RETURN_CLOSING;
            return menu_event_original(controller,event);
        }
        render(controller); return 0;
    }
    /* Ordinary touch notifications must not dismiss a private page. */
    if(kind==EV_TOUCH && native_action_release(BYTE(WORD((u32)event,4),0x50),BYTE(controller,ADJ_ACTION))) {
        leave(controller);
        return menu_event_original(controller,event);
    }
    return 0;
}
/* Playback keeps a non-native Image Control tag as 0x10000 | (tag - 0x100) in
   metadata key 0x25. Return the private icon for a recorded effect code, or 0. */
static u32 playback_icon(volatile u32 *h,u32 value) {
    u32 code=(value+0x100)&0xffff, i;
    if(value>>16!=1 || !(code&0x8000)) return 0;
    for(i=0;i<h[GRKIT_HOST_COUNT/4];++i)
        if(*(volatile u16 *)(h[GRKIT_HOST_RECORDS/4]+i*GRKIT_RECORD_SIZE+GRKIT_RECORD_CODE)==code)
            return 0x8000+i;
    return 0;
}
/* The playback metadata singleton holds key 0x25 at METADATA_TONE. */
u32 menu_playback_private(void) {
    return playback_icon(menu_header(),WORD(CALL0(FW_PLAYBACK_METADATA)(),METADATA_TONE));
}
_Static_assert(GRKIT_ICONS_PER_CHUNK==40 && (GRKIT_HOST_ICON_CHUNKS_WORDS*40)*1639U>>16==GRKIT_HOST_ICON_CHUNKS_WORDS*40/40,
               "The module tile chunk uses slot / 40");
/* IDs stay in transient private widgets and never enter native settings. */
static u32 icon_address(volatile u32 *h,u32 id) {
    u32 index=id-0x8000;
    /* Module tiles follow the effect icons in the chunks. slot * 1639 >> 16
       is slot / 40 for each slot of the chunks. */
    if(id-GRKIT_ICON_MODULE_TILE(0)<GRKIT_MODULE_TILES_MAX) {
        u32 slot=GRKIT_HOST_ICON_COUNT+h[GRKIT_HOST_COUNT/4]+id-GRKIT_ICON_MODULE_TILE(0), chunk=slot*1639U>>16;
        if(chunk>=GRKIT_HOST_ICON_CHUNKS_WORDS || !h[GRKIT_HOST_ICON_CHUNKS/4+chunk]) return 0;
        return h[GRKIT_HOST_ICON_CHUNKS/4+chunk]+(slot-chunk*GRKIT_ICONS_PER_CHUNK)*GRKIT_ICON_STRIDE;
    }
    if(id==GRKIT_ICON_GRKIT) return h[GRKIT_HOST_GRKIT_ICON/4];
    /* The installer puts the Version icon after the GRKit icon. */
    if(id==GRKIT_ICON_VERSION) return h[GRKIT_HOST_GRKIT_ICON/4] ? h[GRKIT_HOST_GRKIT_ICON/4]+GRKIT_ICON_STRIDE : 0;
    /* The Modules icon follows the Version icon. */
    if(id==GRKIT_ICON_MODULES) return h[GRKIT_HOST_GRKIT_ICON/4] ? h[GRKIT_HOST_GRKIT_ICON/4]+2*GRKIT_ICON_STRIDE : 0;
    if(id==0x7fff) return h[GRKIT_HOST_OFF_ICON/4] ? h[GRKIT_HOST_OFF_ICON/4] : NATIVE_OFF_ICON();
    if(index>=h[GRKIT_HOST_COUNT/4]) return 0;
    return WORD(h[GRKIT_HOST_RECORDS/4]+index*GRKIT_RECORD_SIZE,GRKIT_RECORD_ICON);
}
A32_ENTRY u32 menu_icon(u32 id) {
    volatile u32 *h=menu_header();
    u32 start=profile_begin(h), icon=icon_address(h,id);
    profile_end(h,start,GRKIT_PROFILE_MENU_ICON);
    return icon;
}
