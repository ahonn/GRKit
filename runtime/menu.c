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
extern void menu_apply_original(u32);
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
#define MENU_OBJECT_HALF(a,o) HALF(a,o)
#endif
/* Memory access. A host test maps it to its own buffers. */
#ifndef WORD
#define WORD(a,o) (*(u32 *)((a)+(o)))
#endif
#ifndef BYTE
#define BYTE(a,o) (*(u8 *)((a)+(o)))
#endif
#ifndef HALF
#define HALF(a,o) (*(volatile u16 *)((a)+(o)))
#endif
/* The shared types of the core, before the services that it declares. */
#include "core.h"
#include "profile.h"
#define PAGE 6U
/* GRKIT_HOST_RETURN: a native page (the parameter page or a settings page)
   is opening (1) or open (2). ADJ exited while GRKit was open (3), so the next
   ADJ opens GRKit. MENU or OK on GRKit passes the event that closes ADJ to the
   native list (4); the exit then sets 3. Only an exit sets 3, because leaving
   GRKit refreshes ADJ. INSTALL sets 5 when ADJ closed on GRKit before the last
   power-off (CORE.ADJ). An ADJ that opened before INSTALL keeps 5 and stays
   native, also after a half-press. The change that opens ADJ turns 5 into 3,
   so only an ADJ that opens after INSTALL opens GRKit (controller_return).
   RETURN_ROOT marks a page that opened from the root, so the page returns to
   the root instead of the page of the module. */
#define RETURN_OPENING 1U
#define RETURN_PARAMETERS 2U
#define RETURN_GRKIT 3U
#define RETURN_CLOSING 4U
#define RETURN_POWERON 5U
#define RETURN_ROOT 0x10U
#define RETURN_STATE(value) ((value)&~RETURN_ROOT)
/* The controller of ADJ, where a native page that GRKit opened returns. */
#define ADJ_CONTROLLER 0x2000000U
/* The return state after an ADJ exit. `in_grkit` is set for an exit while
   GRKit was open. */
static u32 exit_return(u32 value,u32 in_grkit) {
    if(RETURN_STATE(value)==RETURN_OPENING) return RETURN_PARAMETERS|(value&RETURN_ROOT);
    if(in_grkit || value==RETURN_CLOSING) return RETURN_GRKIT;
    /* A native exit of an ADJ that opened before INSTALL keeps CORE.ADJ. */
    return value==RETURN_POWERON ? RETURN_POWERON : 0;
}
/* The third argument of ChangeController for a return to the previous
   controller. The release of a half-press returns to ADJ with it; the ADJ
   button opens ADJ with the shooting screen as the argument. */
#define CONTROLLER_BACK 0x10000000U
/* The return state when the screen changes to the controller `id` with the
   argument `arg`. Only an ADJ that opens, not a return to an open ADJ after
   a half-press, takes RETURN_POWERON. */
static u32 controller_return(u32 value,u32 id,u32 arg) {
    return value==RETURN_POWERON && id==ADJ_CONTROLLER && arg!=CONTROLLER_BACK ? RETURN_GRKIT : value;
}
#ifndef MENU_NATIVE_IC_TYPE
/* The native Image Control type plus 1, or 0 before the settings exist:
   settings domain +0x518 -> control -> payload (runtime/targets). */
static u32 native_ic_type(void) {
    u32 domain=WORD(FW_SETTINGS_DOMAIN,0), control, payload;
    if(!domain) return 0;
    control=WORD(domain,0x518);
    if(!control) return 0;
    payload=WORD(control,0);
    return payload ? BYTE(payload,SETTINGS_IC_TYPE)+1U : 0;
}
#define MENU_NATIVE_IC_TYPE() native_ic_type()
#endif
/* Whether a native ADJ exit selects the native Image Control type under the
   cursor. The native slot writes the type under its cursor at each cursor
   change, and a half-press keeps it. So an exit to the shooting screen on
   that slot, with another type than when ADJ opened, selects the type, as OK
   does. `opened` and `current` are types plus 1 (MENU_NATIVE_IC_TYPE). */
static u32 native_ic_chosen(u32 opened,u32 current,u32 ic_slot,u32 shooting) {
    return opened && current && current!=opened && ic_slot && shooting;
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
/* The edge of an ADJ that reopens GRKit. After a power-on (CORE.ADJ) no edge
   is known, and the native slot can be any slot, so Left and Right find the
   neighbor slots as after a touch on the GRKit icon (3). */
static u32 reopen_edge(u32 edge) { return edge ? edge : 3; }
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
/* A native Image Control type is selected on the native ADJ slot: the
   GRKit effect ends. */
static void native_ic_selected(volatile u32 *h) {
    h[GRKIT_HOST_ACTIVE/4]=0;
    GRKIT_STATE_CHANGED(h);
    level_reapply(h);
}
#include "tone.h"
#include "develop.h"
#define TONING_HALF(address) HALF(address,0)
#include "toning.h"
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
/* The text descriptor of a text view: the length with its NUL, and the
   text. */
static void descriptor(volatile u32 *at, const u16 *name) {
    u32 length=0;
    while (length<GRKIT_SUMMARY_UNITS-1 && name[length]) ++length;
    at[0]=(length+1)|0x100; at[1]=(u32)name;
}
/* The timer and UI services for modules and the pages of the core. */
#include "ui.h"
#include "ui_keyboard.h"
#include "modules.h"
/* Only the still-viewfinder slot consults module state. Playback continues
   to resolve the icon recorded in the photo metadata. */
static u32 selected_icon(volatile u32 *h) {
    return viewfinder_slot_icon(h,INDICATOR_IMAGE_CONTROL,GRKIT_ICON_OFF);
}
A32_ENTRY u32 menu_viewfinder_icon_select(u32 native_icon,u32 movie) {
    volatile u32 *h=menu_header();
    u32 start=profile_begin(h),icon=movie ? native_icon :
        viewfinder_slot_icon(h,INDICATOR_IMAGE_CONTROL,native_icon);
    profile_end(h,start,GRKIT_PROFILE_MENU_ICON);
    return icon;
}
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
        u32 root=h[GRKIT_HOST_RETURN/4]&RETURN_ROOT, row=h[GRKIT_HOST_MENU_CURSOR/4];
        h[GRKIT_HOST_RETURN/4]=0;
        /* The page has ended. A record that no page started stays unused. */
        if(h[GRKIT_HOST_PARAMETERS/4]&1U) h[GRKIT_HOST_PARAMETERS/4]=0;
        /* GRKit reopens at the root row of the module (menu_reopen). A page
           that opened from a page returns to that page: Fn on a module
           page. The page is visible, because it opened the other page. */
        enter(controller,3);
        if(!root) {
            menu_return_page(h,row);
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
            enter(controller,reopen_edge(h[GRKIT_HOST_MENU_EDGE/4]));
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
/* The list has a row view for each row of the longest native slot of the
   controller (32 on GR III), and list_grow adds more for a longer page. It
   holds all rows of the root or page when they fit, and scrolls them as a
   native list does. Otherwise (more than ADJ_ROWS_MAX rows) it holds the
   page of six rows around the cursor. Each native ADJ list holds more than
   six rows. */
static u32 list_capacity(u32 view) { return (WORD(view,LIST_ROWS+4)-WORD(view,LIST_ROWS))>>2; }
/* Film Look can have more rows than the longest native slot: Off and up to
   40 effects. Append row views of the native kind (style 0x19, height 0x3a,
   as the native list init makes them) until the list has `rows`, so that it
   scrolls them as a native list does. The native fill and marks run over
   the native capacity only (controller byte +0x271 on GR III, the row count
   of the slot on GR IV), so the added rows stay with GRKit. The controller
   lives as long as the camera runs, so the rows are added once. */
#define ADJ_ROW_STYLE 0x19U
#define ADJ_ROW_HEIGHT 0x3aU
#define ADJ_ROWS_MAX 64U
/* Append row views of `style` and `height` until the list has `rows`. */
static void list_grow_rows(u32 view,u32 rows,u32 style,u32 height) {
    u8 styles[ADJ_ROWS_MAX];
    u32 have=list_capacity(view), i;
    if(rows>ADJ_ROWS_MAX) rows=ADJ_ROWS_MAX;
    if(!have || rows<=have) return;
    for(i=0;i<rows-have;++i) styles[i]=(u8)style;
    CALL4(FW_LIST_INIT)(view,(u32)styles,rows-have,height);
#ifdef FW_LIST_ROWS_VALUE
    /* GR IV gives the value of the native init to each row: the added rows
       need it too, else they draw with separators. */
    CALL2(FW_LIST_ROWS_VALUE)(view,0x10);
#endif
}
static void list_grow(u32 view,u32 rows) { list_grow_rows(view,rows,ADJ_ROW_STYLE,ADJ_ROW_HEIGHT); }
static u32 list_start(volatile u32 *h,u32 view) {
    return count(h)>list_capacity(view) ? page_start(h[GRKIT_HOST_MENU_CURSOR/4]) : 0;
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
   module, or turns the setting of the row. The native refresh fills the
   hint only on a native slot that offers Fn, so a hint that GRKit shows
   could be empty. Fill it as the native Image Control slot does, with the
   rotate icon where Fn turns a setting. */
static u32 settings_target(volatile u32 *h,u32 controller);
static u32 fn_setting(volatile u32 *h,u32 controller);
static void fn_guide(u32 controller) {
    volatile u32 *h=menu_header();
    u32 guide=WORD(controller,ADJ_FN_GUIDE), turns=!parameters_row(h) && fn_setting(h,controller)!=PAGE_NO_FN;
    u32 shown=parameters_row(h) ? parameters_available(controller) :
        turns || settings_target(h,controller)!=MODULE_TOTAL;
    if(shown) {
        CALL4(FW_GUIDE_SET_CONTENT)(guide,FN_GUIDE_TEXT,10,turns ? GRKIT_ICON_ROTATE : FN_GUIDE_ICON);
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
    list_grow(view,total);
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
    /* An Up or Down step keeps the rows: a row's icon depends on its index,
       and its check on committed values, which a step does not change. */
    if(!(kept && step)) {
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
    /* The next ADJ session opens at the row of this module (menu_reopen).
       STATE.TXT keeps the module across power-off (CORE.ADJ_MODULE). */
    if(h[GRKIT_HOST_MENU_LEVEL/4]==MENU_ROOT) {
        u32 module=root_module(h,h[GRKIT_HOST_MENU_CURSOR/4]);
        if(h[GRKIT_HOST_MODULE/4]!=module) {
            h[GRKIT_HOST_MODULE/4]=module;
            GRKIT_STATE_CHANGED(h);
        }
    }
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
/* value mod 9 for a value below 27, without a division, which core has no
   helper for. */
static u32 mod9(u32 value) {
    if(value>=18) value-=18;
    if(value>=9) value-=9;
    return value;
}
/* Toning (runtime/toning.h) is the adjustment after the nine of
   GRKIT_RECORD_PARAMETERS. The parameter page of a native look with its own
   curves (GRKIT_RECORD_TONE_SLOT) shows the rows of the native base kind
   PARAMETER_TONING_BASE: the nine adjustments and Toning, row kind
   PARAMETER_TONING_KIND. Its default is the look's own toning. */
#define PARAMETER_TONING 9
/* The record of a started page with Toning, or 0. */
static u32 toning_record(u32 record) {
#ifdef PARAMETER_TONING_BASE
    return record && BYTE(record,GRKIT_RECORD_TONE_SLOT) ? record : 0;
#else
    (void)record;
    return 0;
#endif
}
/* The record curves for Toning stored byte `stored`, from the look's own red
   and blue curves in its tail slot. The next live-view frame composes them
   again: a cleared key never matches (runtime/processing.h). */
static void toning_apply(volatile u32 *h,u32 record,u32 stored) {
    u32 slot=h[GRKIT_HOST_TAIL/4]+GRKIT_TONE_CURVES+(BYTE(record,GRKIT_RECORD_TONE_SLOT)-1U)*GRKIT_TONE_SLOT_SIZE;
    BYTE(record,GRKIT_RECORD_TONING)=(u8)stored;
    toning_curves(record+GRKIT_RECORD_CURVES,slot,toning_value(stored));
    h[GRKIT_HOST_GAMMA_KEY/4]=0;
}
/* The native value plus 1, or 0 to run the native getter. */
A32_ENTRY u32 menu_parameter_get(u32 index) {
    u32 record=menu_parameter_session();
    if(index==PARAMETER_TONING) {
        record=toning_record(record);
        return record ? (u32)(toning_value(BYTE(record,GRKIT_RECORD_TONING))+4)+1 : 0;
    }
    return record ? record_parameter(record,index)+1 : 0;
}
/* Nonzero when the value went to the record. */
A32_ENTRY u32 menu_parameter_set(u32 index,u32 value) {
    volatile u32 *h=menu_header();
    u32 record=menu_parameter_session();
    if(index==PARAMETER_TONING) record=toning_record(record);
    if(!record) return 0;
    if(value<=8) {
        if(index==PARAMETER_TONING) toning_apply(h,record,mod9(value+5));
        else BYTE(record,GRKIT_RECORD_PARAMETERS+index)=(u8)mod9(value+14-BYTE(record,GRKIT_RECORD_FN_DEFAULTS+index));
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
/* Show the Standard rows of the base set, or the rows with Toning
   (toning_record). The type is Standard, so the native title would read its
   name. Keep the page and the effect name, so
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
#ifdef PARAMETER_TONING_BASE
    if(toning_record(record)) BYTE(page,DETAIL_BASE_KIND)=PARAMETER_TONING_BASE;
#endif
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
        BYTE(model,MODEL_SNAPSHOT+parameter_kinds[i])=(u8)record_parameter(record,i);
#ifdef PARAMETER_TONING_KIND
    if(toning_record(record))
        BYTE(model,MODEL_SNAPSHOT+PARAMETER_TONING_KIND)=(u8)(toning_value(BYTE(record,GRKIT_RECORD_TONING))+4);
#endif
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
#include "menu_recipes.h"
#include "menu_section.h"
#include "menu_about.h"
/* The text descriptor for `view` and text ID `id`, or 0 for the native text.
   Bit 0 selects the font of the menu language. GRKit gives the texts of one
   screen, which the last controller change selects (MENU_ROWS_CURRENT): the
   camera information page shows About GRKit, the borrowed submenu and its
   value pages show a settings page, and another screen can show the rows of
   the GRKit section. So the screens can use the same text IDs: the rows of
   the GRKit section use those of the borrowed rows (MENU_ROW_LABEL_TEXT). */
#ifdef DEVELOP_TONE_LIST
static u32 develop_text(volatile u32 *h,u32 id);
#else
#define develop_text(h,id) 0U
#endif
A32_ENTRY u32 menu_text_override(u32 view,u32 id) {
    volatile u32 *h=menu_header();
    u32 title=menu_parameter_title(view);
    if(title) return title;
    title=develop_text(h,id);
    if(title) return title;
    title=ui_text_override(h,view,id);
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
   a root row or on a module page. */
static u32 settings_target(volatile u32 *h,u32 controller) {
    u32 module, level=h[GRKIT_HOST_MENU_LEVEL/4];
    const MenuPage *page;
    if(ADJ_MOVIE(controller)==1) return MODULE_TOTAL;
    if(level==MENU_ROOT) module=root_module(h,h[GRKIT_HOST_MENU_CURSOR/4]);
    else if(level==MENU_MODULE) module=h[GRKIT_HOST_MODULE/4];
    else return MODULE_TOTAL;
    page=menu_page(h,module);
    if(level==MENU_MODULE && page==&group_module) {
        module=group_child(h,h[GRKIT_HOST_MENU_CURSOR/4]);
        if(module>=module_pages(h)) return MODULE_TOTAL;
        page=menu_page(h,module);
    }
    if(!page->settings) return MODULE_TOTAL;
    /* A page with one settings row has it in MENU only: it is the page. */
    if(page->settings==choice_settings && !choice_direct(h) && PAGE_WORD(CHOICE_ENTRY(h)[0],PAGE_SETTING_ROWS)<2) return MODULE_TOTAL;
    return module;
}
/* The Fn setting of the highlighted row of a choice page (ABI 2.5), or
   PAGE_NO_FN. Fn turns it only in the still context, and only when it has
   two values or more. */
static u32 fn_setting(volatile u32 *h,u32 controller) {
    u32 module=h[GRKIT_HOST_MODULE/4], handle;
    if(h[GRKIT_HOST_MENU_LEVEL/4]!=MENU_MODULE || module>=module_pages(h) || ADJ_MOVIE(controller)==1) return PAGE_NO_FN;
    menu_page(h,module);
    handle=choice_fn_setting(h,h[GRKIT_HOST_MENU_CURSOR/4]);
    return handle!=PAGE_NO_FN && setting_limit(h,handle)>=2 ? handle : PAGE_NO_FN;
}
/* Fn on a row with an Fn setting: the setting takes its next value, and 0
   after the last one. ADJ stays on the row, and the live view redraws with
   the new value. */
static u32 fn_step(volatile u32 *h,u32 controller) {
    u32 handle=fn_setting(h,controller), values, value;
    if(handle==PAGE_NO_FN) return 0;
    values=setting_limit(h,handle);
    value=setting_value(h,handle)+1;
    setting_store(h,handle,value<values ? value : 0);
    MENU_PREVIEW_INVALIDATE(WORD(controller,ADJ_CANVAS));
    return 1;
}
static u32 open_settings(u32 controller) {
    volatile u32 *h=menu_header();
    u32 module=settings_target(h,controller), root=h[GRKIT_HOST_MENU_LEVEL/4]==MENU_ROOT;
    if(module==MODULE_TOTAL) return 0;
    leave(controller);
    settings_start(h,module);
    h[GRKIT_HOST_RETURN/4]=RETURN_OPENING|(root ? RETURN_ROOT : 0);
    CALL3(FW_CHANGE_CONTROLLER)(controller,SETTINGS_SUBMENU,0x2000000);
    return 1;
}
/* A settings entry has no intermediate choice page. */
static u32 direct_settings_root(volatile u32 *h) {
    u32 module;
    if(h[GRKIT_HOST_MENU_LEVEL/4]!=MENU_ROOT) return 0;
    module=current_index(h);
    if(module>=module_pages(h)) return 0;
    menu_page(h,module);
    return choice_direct(h);
}
static u32 group_settings_row(volatile u32 *h) {
    return h[GRKIT_HOST_MENU_LEVEL/4]==MENU_MODULE && current_module(h)==&group_module;
}
static u32 direct_settings_event(volatile u32 *h,u32 kind) {
    return (kind==EV_RIGHT || (kind==EV_CONFIRM && !after_adj_press(h))) && direct_settings_root(h);
}
/* Confirm the current selection and return whether ADJ should close.
   Group OK toggles a child in place; an ADJ press only closes its list. */
static u32 confirm_selection(volatile u32 *h) {
    u32 page=h[GRKIT_HOST_MENU_LEVEL/4]==MENU_MODULE, group=group_settings_row(h);
    u32 close=page && h[GRKIT_HOST_MODULE/4]!=MANAGE_MODULE && !group;
    if(after_adj_press(h)) close=1;
    if((page || !close) && !(group && close)) menu_confirm(h);
    return close;
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
   its cursor. The Modules page opens settings on OK, so it commits nothing.
   The GRKit switch and group toggles commit only on OK or a touch: leaving
   their pages must not change an Enable setting. A page of actions, such as
   the camera recipes, acts only on OK. */
static void exit_commit(volatile u32 *h) {
    u32 module=h[GRKIT_HOST_MODULE/4];
    const MenuPage *page;
    if(h[GRKIT_HOST_MENU_LEVEL/4]!=MENU_MODULE || module==MANAGE_MODULE || module==SWITCH_MODULE) return;
    page=menu_page(h,module);
    if(page!=&group_module && !(page->flags&MENU_PAGE_ACTIONS)) menu_confirm(h);
}
A32_ENTRY void menu_exit(u32 controller) {
    volatile u32 *h=menu_header();
    u32 in_grkit=private_mode(controller);
    if(in_grkit) exit_commit(h);
    else if(h[GRKIT_HOST_ACTIVE/4] && h[GRKIT_HOST_SERVICES/4] &&
            native_ic_chosen(h[GRKIT_HOST_ADJ_IC_TYPE/4],MENU_NATIVE_IC_TYPE(),
                             CALL2(FW_ADJ_SLOT_TYPE)(controller,BYTE(controller,ADJ_SLOT))==ADJ_IC_SLOT_TYPE,
                             services_word(h,SERVICES_MENU+MENU_ROWS_CURRENT)[0]==SHOOTING_CONTROLLER))
        native_ic_selected(h);
    /* A box of the UI service closes with ADJ, and a native screen that
       waits to open does not open. */
    ui_close(h);
    ui_native_cancel(h);
    leave(controller);
    h[GRKIT_HOST_BACK_PENDING/4]=0;
    h[GRKIT_HOST_PREVIEW/4]=0;
    h[GRKIT_HOST_FRAME_PREVIEW/4]=0;
    /* The next ADJ returns to the parameter page's GRKit page, opens on
       GRKit, or opens as native ADJ does (exit_return). CORE.ADJ keeps the
       sticky preference across power-off; a transient return to the
       parameter page does not change it. */
    h[GRKIT_HOST_RETURN/4]=exit_return(h[GRKIT_HOST_RETURN/4],in_grkit);
    if(RETURN_STATE(h[GRKIT_HOST_RETURN/4])!=RETURN_PARAMETERS &&
       h[GRKIT_HOST_RETURN/4]!=RETURN_POWERON) {
        u32 open=h[GRKIT_HOST_RETURN/4]==RETURN_GRKIT ? 1U : 0U;
        if(h[GRKIT_HOST_ADJ_OPEN/4]!=open) {
            h[GRKIT_HOST_ADJ_OPEN/4]=open;
            GRKIT_STATE_CHANGED(h);
        }
        if(h[GRKIT_HOST_PARAMETERS/4]&1U) h[GRKIT_HOST_PARAMETERS/4]=0;
    }
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
    if(private_mode(controller)) {
        u32 view=WORD(controller,ADJ_LIST);
        u32 next=list_start(h,view)+CALL1(FW_LIST_CURSOR)(view);
        if(next<count(h) && next!=h[GRKIT_HOST_MENU_CURSOR/4]) { h[GRKIT_HOST_MENU_CURSOR/4]=next; highlight(h); render(controller); }
        return;
    }
    menu_changed_original(controller);
}
/* The native ADJ writes the value under the list cursor to the setting of its
   slot: on a cursor change, a slot change, and a touch on a row. A GRKit page
   shows its own rows in that list, so it must not write the setting of the
   native slot that was selected before GRKit. On a GRKit page only a touch on
   a row reaches here: menu_changed does not run the native change, and a slot
   change leaves the page first. The touch selects the row as OK does, and ADJ
   stays open, as after a touch on a native row. */
A32_ENTRY void menu_apply(u32 controller) {
    volatile u32 *h=menu_header();
    if(!private_mode(controller)) { menu_apply_original(controller); return; }
    if(ui_modal(h)) return;
    if(direct_settings_root(h) && open_settings(controller)) return;
    menu_confirm(h);
    render(controller);
}
/* ADJ updates the exposure box of some slot types (ISO Setting) every 50 ms
   while their slot is selected. On a GRKit page the type of no slot shows,
   so the update hides the box and draws nothing. */
A32_ENTRY u32 menu_tick_type(u32 controller,u32 slot) {
    return private_mode(controller) ? 0 : CALL2(FW_ADJ_SLOT_TYPE)(controller,slot);
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
           CALL2(FW_ADJ_SLOT_TYPE)(controller,BYTE(controller,ADJ_SLOT))==ADJ_IC_SLOT_TYPE)
            native_ic_selected(h);
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
    /* Fn on the row of a page value with an Fn setting steps that setting. */
    if(kind==EV_FN && *(u8 *)WORD((u32)event,4)==FN_PRESS_ARG && fn_step(h,controller)) return 0;
    /* Fn elsewhere opens the detail settings of the module, if it has any. */
    if(kind==EV_FN && *(u8 *)WORD((u32)event,4)==FN_PRESS_ARG && open_settings(controller)) return 0;
    /* OK and Right on a settings entry open its native controls. */
    if(direct_settings_event(h,kind) && open_settings(controller)) return 0;
    /* Left and Right otherwise move to the next ADJ entry from the root
       and from a page. MENU returns from a page to the root. */
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
       confirm, as after MENU on the root. Group OK toggles a child and keeps
       the list open. An ADJ press closes that list without another toggle;
       on other pages it commits the row first.
       If the commit made a module show a UI box or ask for a native screen,
       ADJ stays open on the page instead (ui_holds_page). */
    if(kind==EV_CONFIRM) {
        u32 close=confirm_selection(h);
        if(close && ui_holds_page(h)) close=0;
        if(close) {
            leave(controller);
            h[GRKIT_HOST_RETURN/4]=RETURN_CLOSING;
            return menu_event_original(controller,event);
        }
        render(controller); return 0;
    }
    /* Ordinary touch notifications must not dismiss a private page. */
    if(kind==EV_TOUCH && native_action_release(BYTE(WORD((u32)event,4),0x50),BYTE(controller,ADJ_FN_HELD))) {
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
        return module_tile_address(h,id);
    }
    if(id==GRKIT_ICON_GRKIT) return h[GRKIT_HOST_GRKIT_ICON/4];
    /* The installer puts the Version icon after the GRKit icon. */
    if(id==GRKIT_ICON_VERSION) return h[GRKIT_HOST_GRKIT_ICON/4] ? h[GRKIT_HOST_GRKIT_ICON/4]+GRKIT_ICON_STRIDE : 0;
    /* The Modules icon follows the Version icon. */
    if(id==GRKIT_ICON_MODULES) return h[GRKIT_HOST_GRKIT_ICON/4] ? h[GRKIT_HOST_GRKIT_ICON/4]+2*GRKIT_ICON_STRIDE : 0;
    if(id==GRKIT_ICON_MODULES_OFF) return h[GRKIT_HOST_GRKIT_ICON/4] ? h[GRKIT_HOST_GRKIT_ICON/4]+3*GRKIT_ICON_STRIDE : 0;
    if(id==GRKIT_ICON_ROTATE) return h[GRKIT_HOST_GRKIT_ICON/4] ? h[GRKIT_HOST_GRKIT_ICON/4]+4*GRKIT_ICON_STRIDE : 0;
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

#ifdef DEVELOP_TONE_LIST
/* ---- RAW development: the effects in its Image Control list --------------
   The development applies GRKIT_HOST_DEVELOP (runtime/processing.h): the
   effect that its DNG records, or 1 for native processing. Its Image Control
   list keeps its native rows and lists the effects after them. An effect row
   is the native Standard to the model (the effects are built on Standard),
   with GRKIT_HOST_DEVELOP its record. A native row sets native processing.
   The list applies the highlighted row at once for the preview, and Cancel
   restores the value of the list start. */
#define DEVELOP_NONE 0xffffffffU
static u32 develop_running(void) {
    return (BYTE(FW_DEVELOP_GUARD,0)&1U) && BYTE(FW_DEVELOP_MODEL,DEVELOP_STATE);
}
static u32 develop_effects(volatile u32 *h) {
    return h[GRKIT_HOST_CAPABILITIES/4]&GRKIT_CORE_TONE_BIT ? h[GRKIT_HOST_COUNT/4] : 0;
}
static u32 develop_record(volatile u32 *h,u32 i) { return h[GRKIT_HOST_RECORDS/4]+i*GRKIT_RECORD_SIZE; }
/* The index of the effect of the development, or DEVELOP_NONE. */
static u32 develop_index(volatile u32 *h) {
    u32 record=h[GRKIT_HOST_DEVELOP/4], i;
    if(!develop_running() || record<=1) return DEVELOP_NONE;
    for(i=0;i<develop_effects(h);++i) if(develop_record(h,i)==record) return i;
    return DEVELOP_NONE;
}
static u32 develop_list(u32 controller) { return WORD(controller,CONTROLLER_ID)==DEVELOP_TONE_LIST; }
/* A GRKit row of a list of the development: `icon`, no text, no mark. */
static u32 develop_fill(u32 view,u32 icon) {
    static const u8 empty[1]={0};
    if(!CALL2(FW_ROW_CHILD)(view,2)) return DEVELOP_NONE;
    CALL2(FW_ICON_VIEW_SET_ICON)(CALL2(FW_ROW_CHILD)(view,0),icon);
    CALL2(FW_STRING_VIEW_SET_ASCII)(CALL2(FW_ROW_CHILD)(view,1),(u32)empty);
    CALL2(FW_ICON_VIEW_SET_ICON)(CALL2(FW_ROW_CHILD)(view,2),0);
    return 0;
}
#ifdef DEVELOP_ASPECT_LIST
/* ---- RAW development: the Framing ratios in its aspect list -------------
   The development develops GRKIT_HOST_DEVELOP_RATIO, a Framing ratio, or 0
   for its native aspect (runtime/framing.c). Its aspect list keeps its
   native rows and lists after them the ratios that the development can
   crop (develop_ratio_row, runtime/aspect.h). A ratio row is a native value
   to the model: the native row of the capture aspect of the ratio, else 3:2.
   So the native conversions accept it, and the development sends the
   capture aspect and the geometry of the ratio. The list applies the
   highlighted row at once for the preview, and Cancel restores the ratio of
   the list start. The rows show the tiles of the Framing page, and a label
   of the aspect shows the row label of the ratio on that page. */
#define ASPECT_MODEL FW_DEVELOP_MODEL
static u32 aspect_list(u32 controller) { return WORD(controller,CONTROLLER_ID)==DEVELOP_ASPECT_LIST; }
static u32 aspect_row_value(u32 row) { return CALL2(FW_DEVELOP_ASPECT_VALUE)(ASPECT_MODEL,row); }
static u32 aspect_row_capture(u32 value) { return CALL2(FW_DEVELOP_ASPECT_CAPTURE)(ASPECT_MODEL,value); }
/* The module page entry of CORE.ASPECT (the Framing page), or 0. */
static volatile u32 *aspect_page(volatile u32 *h) {
    volatile u32 *entry;
    u32 slot;
    if(!CAPABILITY_USED(h,GRKIT_CORE_ASPECT)) return 0;
    for(slot=0;slot<module_pages(h);++slot) {
        entry=h+GRKIT_HOST_MODULE_PAGES/4+slot*3;
        if(entry[1]==GRKIT_CORE_ASPECT && PAGE_WORD(entry[0],PAGE_KIND)!=PAGE_GROUP) return entry;
    }
    return 0;
}
/* Bit n for each native row that develops capture aspect n. The row of the
   recorded aspect (value 0) does not count. */
static u32 aspect_captures(volatile u32 *h) {
    u32 rows=h[GRKIT_HOST_DEVELOP_ASPECT_ROWS/4], row, value, captures=0;
    for(row=0;row<rows;++row)
        if((value=aspect_row_value(row))) captures|=1U<<(aspect_row_capture(value)&31U);
    return captures;
}
/* The ratio of a list row, or 0 for a native row. */
static u32 aspect_ratio(volatile u32 *h,u32 row) {
    u32 rows=h[GRKIT_HOST_DEVELOP_ASPECT_ROWS/4];
    if(!rows || row<rows || !aspect_page(h)) return 0;
    return develop_ratio_row(aspect_captures(h),row-rows);
}
/* The native value of a ratio row. */
static u32 aspect_native_value(volatile u32 *h,u32 ratio) {
    u32 rows=h[GRKIT_HOST_DEVELOP_ASPECT_ROWS/4], row, value, capture, stock=0;
    for(row=0;row<rows;++row) {
        if(!(value=aspect_row_value(row))) continue;
        capture=aspect_row_capture(value);
        if(capture==develop_capture(ratio)) return value;
        if(!capture && !stock) stock=value;
    }
    return stock;
}
/* The ratio that a label or an icon of native `value` shows, or 0. */
static u32 aspect_shown(volatile u32 *h,u32 value) {
    u32 ratio=h[GRKIT_HOST_DEVELOP_RATIO/4];
    return ratio && aspect_page(h) && value==aspect_native_value(h,ratio) ? ratio : 0;
}
static u32 aspect_icon(volatile u32 *h,u32 ratio) {
    volatile u32 *entry=aspect_page(h);
    return entry && entry[2]!=MANAGE_NO_TILE ? GRKIT_ICON_MODULE_TILE(entry[2]+ratio) : GRKIT_ICON_OFF;
}
/* The row label of a ratio on the Framing page, in the menu language. */
static const u16 *aspect_name(volatile u32 *h,u32 ratio) {
    return MENU_TEXT(PAGE_WORD(PAGE_WORD(aspect_page(h)[0],PAGE_LABELS),(ratio*GRKIT_TEXT_LANGUAGES+menu_language())*4));
}
/* The text of the label that shows a ratio. */
static u32 aspect_text(volatile u32 *h,u32 id) {
    return id && id==h[GRKIT_HOST_DEVELOP_RATIO_TEXT/4] && h[GRKIT_HOST_DEVELOP_RATIO/4] ? (u32)(h+GRKIT_HOST_DEVELOP_RATIO_NAME/4) : 0;
}
#else
#define aspect_list(controller) 0
#define aspect_text(h,id) 0U
#endif
/* The effect of a list row, or DEVELOP_NONE for a native row. */
static u32 develop_row(volatile u32 *h,u32 row) {
    u32 rows=h[GRKIT_HOST_DEVELOP_ROWS/4];
    return rows && row>=rows && row-rows<develop_effects(h) ? row-rows : DEVELOP_NONE;
}
/* Add row views until the list has `rows`. The list init of the controller
   gives each row view its children, an icon, a text, and a mark, as here. */
static void develop_grow(u32 list,u32 rows) {
    u32 have=list_capacity(list), i, row;
    list_grow_rows(list,rows,DEVELOP_ROW_STYLE,DEVELOP_ROW_HEIGHT);
    for(i=have;have && i<list_capacity(list);++i) {
        row=CALL2(FW_LIST_ROW)(list,i);
        CALL3(FW_ROW_CHILDREN)(row,3,DEVELOP_ROW_LAYOUT);
        CALL2(FW_ICON_VIEW_STYLE)(CALL2(FW_ROW_CHILD)(row,0),1);
        CALL2(FW_STRING_VIEW_STYLE)(CALL2(FW_ROW_CHILD)(row,1),0);
    }
}
/* The row count of a list of the development. The Image Control list gets
   a row view for each effect too, once: the list keeps its row views. */
A32_ENTRY u32 menu_develop_count_after(u32 count,u32 controller) {
    volatile u32 *h=menu_header();
#ifdef DEVELOP_ASPECT_LIST
    u32 ratios;
    if(aspect_list(controller)) {
        h[GRKIT_HOST_DEVELOP_ASPECT_ROWS/4]=count;
        ratios=aspect_page(h) ? develop_ratio_rows(aspect_captures(h)) : 0;
        develop_grow(WORD(controller,DEVELOP_LIST_VIEW),count+ratios);
        return count+ratios;
    }
#endif
    if(!develop_list(controller)) return count;
    h[GRKIT_HOST_DEVELOP_ROWS/4]=count;
    develop_grow(WORD(controller,DEVELOP_LIST_VIEW),count+develop_effects(h));
    return count+develop_effects(h);
}
/* The category and the value of an effect row: Standard. */
A32_ENTRY u32 menu_develop_category_answer(u32 model,u32 row) {
    (void)model;
    return develop_row(menu_header(),row)==DEVELOP_NONE ? DEVELOP_NONE : 0;
}
A32_ENTRY u32 menu_develop_value_answer(u32 model,u32 row) {
    (void)model;
    return develop_row(menu_header(),row)==DEVELOP_NONE ? DEVELOP_NONE : DEVELOP_STANDARD;
}
/* The fill of an effect row or a ratio row: its icon, no text, no mark. */
A32_ENTRY u32 menu_develop_row_answer(u32 controller,u32 view,u32 row) {
    volatile u32 *h=menu_header();
    u32 i;
#ifdef DEVELOP_ASPECT_LIST
    if(aspect_list(controller)) return (i=aspect_ratio(h,row)) ? develop_fill(view,aspect_icon(h,i)) : DEVELOP_NONE;
#endif
    if(!develop_list(controller) || (i=develop_row(h,row))==DEVELOP_NONE) return DEVELOP_NONE;
    return develop_fill(view,0x8000+i);
}
/* The current row: the row of the effect of the development, else the
   native row of the native value. */
A32_ENTRY u32 menu_develop_current_answer(u32 controller,u32 row) {
    volatile u32 *h=menu_header();
    u32 effect;
#ifdef DEVELOP_ASPECT_LIST
    /* The row of the ratio of the development, else the native row of the
       native value. */
    if(aspect_list(controller)) {
        u32 ratio=h[GRKIT_HOST_DEVELOP_RATIO/4];
        if(ratio) return aspect_ratio(h,row)==ratio;
        return aspect_ratio(h,row) ? 0 : DEVELOP_NONE;
    }
#endif
    if(!develop_list(controller)) return DEVELOP_NONE;
    effect=develop_index(h);
    if(effect!=DEVELOP_NONE) return develop_row(h,row)==effect;
    return develop_row(h,row)==DEVELOP_NONE ? DEVELOP_NONE : 0;
}
/* Fill the Standard set of the model for the effect of the development, or
   give it its own values back for native processing (runtime/develop.h). */
static void develop_set(volatile u32 *h) {
#ifdef DEVELOP_PARAMETERS
    develop_hold(h,h[GRKIT_HOST_DEVELOP/4]);
#else
    (void)h;
#endif
}
/* The choice of a row, also of each highlighted row for the preview. It runs
   before the native choice, which passes the parameter set of the row on. */
A32_ENTRY void menu_develop_choose(u32 controller,u32 row) {
    volatile u32 *h=menu_header();
    u32 i=develop_row(h,row);
    (void)controller;
    h[GRKIT_HOST_DEVELOP/4]=i==DEVELOP_NONE ? 1 : develop_record(h,i);
    develop_set(h);
}
A32_ENTRY void menu_develop_opened(u32 controller) {
    volatile u32 *h=menu_header();
    if(develop_list(controller)) h[GRKIT_HOST_DEVELOP_SAVED/4]=h[GRKIT_HOST_DEVELOP/4];
    if(aspect_list(controller)) h[GRKIT_HOST_DEVELOP_RATIO_SAVED/4]=h[GRKIT_HOST_DEVELOP_RATIO/4];
}
A32_ENTRY void menu_develop_restored(void) {
    volatile u32 *h=menu_header();
    h[GRKIT_HOST_DEVELOP/4]=h[GRKIT_HOST_DEVELOP_SAVED/4];
    develop_set(h);
}
/* Fn on the row of an effect opens the parameter page of the development for
   native Standard, whose set then holds the Fn adjustments of the effect
   (runtime/develop.h). The list may open on the effect that the DNG records
   without a native choice, so the model selects Standard first. The builds
   of an effect read the set, not the processing that a selection passes it
   to. A target without the set has no page for an effect: Fn does nothing on
   its row. */
A32_ENTRY u32 menu_develop_fn_answer(u32 controller) {
    volatile u32 *h=menu_header();
    if(!develop_list(controller) || develop_index(h)==DEVELOP_NONE) return DEVELOP_NONE;
#ifdef DEVELOP_PARAMETERS
    develop_set(h);
    BYTE(FW_DEVELOP_MODEL,DEVELOP_CATEGORY)=0;
    BYTE(FW_DEVELOP_MODEL,DEVELOP_VALUES)=DEVELOP_STANDARD;
    return 1;
#else
    return 0;
#endif
}
/* The label of an Image Control in a subtitle. For the effect of the
   development it is the Standard label, which menu_text_override shows as
   the effect name. A subtitle draws only a changed text ID, so the view
   forgets its text first. */
A32_ENTRY u32 menu_develop_label(u32 model,u32 category,u32 value,u32 view) {
    volatile u32 *h=menu_header();
    u32 id=CALL3(FW_DEVELOP_TONE_TEXT)(model,category,value), i=develop_index(h);
    if(i!=DEVELOP_NONE && !category && value==DEVELOP_STANDARD) {
        h[GRKIT_HOST_DEVELOP_TEXT/4]=id;
        descriptor(h+GRKIT_HOST_DEVELOP_NAME/4,(const u16 *)(develop_record(h,i)+GRKIT_RECORD_NAME));
        CALL3(FW_STRING_VIEW_RESET)(view,0,0);
    }
    return id;
}
/* The icon of an Image Control in the rows of the development. */
A32_ENTRY u32 menu_develop_top_icon(u32 model,u32 category,u32 value) {
    u32 i=develop_index(menu_header());
    if(i!=DEVELOP_NONE && !category && value==DEVELOP_STANDARD) return 0x8000+i;
    return CALL3(FW_DEVELOP_TONE_ICON)(model,category,value);
}
#ifdef DEVELOP_ASPECT_LIST
/* The native value of a ratio row. */
A32_ENTRY u32 menu_develop_aspect_value_answer(u32 model,u32 row) {
    volatile u32 *h=menu_header();
    u32 ratio=aspect_ratio(h,row);
    (void)model;
    return ratio ? aspect_native_value(h,ratio) : DEVELOP_NONE;
}
/* The choice of a row, also of each highlighted row for the preview: a
   ratio row selects its ratio, a native row the native aspect. */
A32_ENTRY void menu_develop_aspect_choose(u32 controller,u32 row) {
    volatile u32 *h=menu_header();
    (void)controller;
    h[GRKIT_HOST_DEVELOP_RATIO/4]=aspect_ratio(h,row);
}
A32_ENTRY void menu_develop_aspect_restored(void) {
    volatile u32 *h=menu_header();
    h[GRKIT_HOST_DEVELOP_RATIO/4]=h[GRKIT_HOST_DEVELOP_RATIO_SAVED/4];
}
/* The label of an aspect (model, value) in a subtitle. For the native value
   of the ratio of the development, menu_text_override shows the row label
   of the ratio. A subtitle draws only a changed text ID, so the view forgets
   its text whenever it shows that text ID. */
A32_ENTRY u32 menu_develop_aspect_label(u32 model,u32 value,u32 unused,u32 view) {
    volatile u32 *h=menu_header();
    u32 id=CALL2(FW_DEVELOP_ASPECT_TEXT)(model,value), ratio=aspect_shown(h,value);
    (void)unused;
    if(ratio) {
        h[GRKIT_HOST_DEVELOP_RATIO_TEXT/4]=id;
        descriptor(h+GRKIT_HOST_DEVELOP_RATIO_NAME/4,aspect_name(h,ratio));
    }
    if(id && id==h[GRKIT_HOST_DEVELOP_RATIO_TEXT/4]) CALL3(FW_STRING_VIEW_RESET)(view,0,0);
    return id;
}
/* The icon of an aspect in the rows of RawDevelopmentTopController. */
A32_ENTRY u32 menu_develop_aspect_top_icon(u32 model,u32 value) {
    volatile u32 *h=menu_header();
    u32 ratio=aspect_shown(h,value);
    return ratio ? aspect_icon(h,ratio) : CALL2(FW_DEVELOP_ASPECT_ICON)(model,value);
}
#endif
static u32 develop_text(volatile u32 *h,u32 id) {
    if(id && id==h[GRKIT_HOST_DEVELOP_TEXT/4] && develop_index(h)!=DEVELOP_NONE) return (u32)(h+GRKIT_HOST_DEVELOP_NAME/4);
    return aspect_text(h,id);
}
#endif
#ifdef CROP_NATIVE_BITS
/* The camera's crop button (runtime/menu.S: crop_button). With CORE.CROP
   in use, the button steps it to its next crop that ADJ shows, then to Off,
   and sets the camera's crop factor at once. Returns 0 without CORE.CROP,
   so that the camera steps its own crops. */
u32 crop_button_press(void) {
    volatile u32 *h=menu_header();
    u32 slot, hidden=0, value, i;
    if(!CAPABILITY_USED(h,GRKIT_CORE_CROP)) return 0;
    for(slot=0;slot<module_pages(h);++slot) {
        menu_page(h,slot);
        if(CHOICE_ENTRY(h)[1]==GRKIT_CORE_CROP && !choice_group(h)) { hidden=choice_hidden(h); break; }
    }
    value=crop_selection(h);
    for(i=0;i<GRKIT_CORE_CROP_VALUES;++i) {
        if(++value>=GRKIT_CORE_CROP_VALUES) value=0;
        if(!value || !(hidden>>value&1U)) break;
    }
    setting_store(h,GRKIT_CORE_CROP,value);
    crop_sync(h,aspect_native());
    return 1;
}
#endif
