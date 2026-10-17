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
#define CALL5(a) ((u32 (*)(u32,u32,u32,u32,u32))(a))
#define CALL3(a) ((u32 (*)(u32,u32,u32))(a))
#define WORD(a,o) (*(u32 *)((a)+(o)))
#define BYTE(a,o) (*(u8 *)((a)+(o)))
#define PAGE 6U
/* Menu C is Thumb-2 for size. Firmware hooks and thunks enter with A32 B/BL,
   which cannot change state, so their targets stay A32. */
#ifdef __arm__
#define A32_ENTRY __attribute__((target("arm")))
#else
#define A32_ENTRY
#endif
static void plugin_touch(u32 controller);
static void render(u32 controller);
static void enter(u32 controller,u32 edge);
static const u16 plugins[] = {'G','R','K','i','t',0};
static const u16 image_control[] = {'I','m','a','g','e',' ','C','o','n','t','r','o','l',0};
static const u16 off[] = {'O','f','f',0};
static const u16 off_summary[] = {'I','m','a','g','e',' ','C','o','n','t','r','o','l',':',' ','O','f','f',0};

/* A wrapped stock navigation result places the private entry across that edge. */
static u32 wrapped_edge(u32 kind,u32 old,u32 next,u32 movie) {
    if(movie==1) return 0;
    if(kind==0xcf && next<=old) return 2;
    if(kind==0xce && next>=old) return 1;
    return 0;
}
static u32 advance_from_private(u32 kind,u32 edge) {
    return (kind==0xcf && edge==1) || (kind==0xce && edge==2);
}
/* Match the native ActionModel: only the initial press starts an action. */
static u32 menu_press(u32 phase,u32 repeats) { return phase==2 && repeats==0; }
/* MENU emits a switch event followed by the same confirm used by OK. */
static u32 back_event(volatile u32 *h,u32 kind,u32 enabled) {
    if(kind==8) h[GRKIT_HOST_BACK_PENDING/4]=0;
    if(kind==5 && enabled) {
        if(h[GRKIT_HOST_BACK_PENDING/4]) return 2;
        h[GRKIT_HOST_BACK_PENDING/4]=1;
        return 1;
    }
    if(kind==0xd8 && h[GRKIT_HOST_BACK_PENDING/4]) {
        h[GRKIT_HOST_BACK_PENDING/4]=0;
        return 2;
    }
    return 0;
}
/* MENU closes native ADJ with its switch event (5) followed by the OK confirm. */
static u32 after_menu_switch(volatile u32 *h) {
    return (h[28+((h[19]-2)&15)]&0xffff)==5;
}
static u32 native_action_release(u32 action,u32 active) {
    return action==10 && active;
}
static u32 selected_icon(volatile u32 *h) {
    u32 i;
    for(i=0;i<h[GRKIT_HOST_COUNT/4];++i)
        if(h[GRKIT_HOST_ACTIVE/4]==h[GRKIT_HOST_RECORDS/4]+i*GRKIT_RECORD_SIZE)
            return 0x8000+i;
    return 0x7fff;
}
/* Override only the native still-viewfinder icon result. Off retains stock UI. */
A32_ENTRY u32 menu_viewfinder_icon_select(u32 native_icon,u32 movie) {
    u32 icon=selected_icon(menu_header());
    return !movie && icon!=0x7fff ? icon : native_icon;
}
#ifndef MENU_FRAMING_REQUEST
/* Resend the native StillAspectRatio value, as a native menu change does.
   The Framing capture hook maps a 3:2 request to the Framing ratio. */
#define MENU_FRAMING_REQUEST() do { u32 s=CALL0(0x5320b31c)(); CALL2(0x532525a4)(s,CALL1(0x53245260)(s)); } while(0)
#endif
#include "modules.h"
static u32 private_mode(u32 controller) {
    volatile u32 *h=menu_header();
    return h[8] == controller && h[9] != 0;
}
/* A native Image Control preview lasts while its ADJ slot has focus. */
static void native_focus(u32 controller) {
    if(CALL2(0x531a2820)(controller,BYTE(controller,0x1f4))!=12)
        menu_header()[GRKIT_HOST_PREVIEW/4]=0;
}
/* Fn applies to effect rows of GRKit Image Control. Off has no adjustments. */
static u32 parameters_row(volatile u32 *h) {
    return h[9]==MENU_MODULE && !h[GRKIT_HOST_MODULE/4] && h[10];
}
/* Fn opens the native Standard parameters from an effect row in the still
   context. The native Fn hint shows only where this works. */
static u32 parameters_available(u32 controller) {
    u32 settings=WORD(WORD(WORD(0x553458b0,0),0x518),0);
    return parameters_row(menu_header()) &&
           BYTE(controller,0x270)!=1 && BYTE(settings,0x77c)==11;
}
/* Native slot indices stay in 0..4. Only the transient toolbar has six cells. */
static u32 toolbar_x(u32 slot) { return 415 + 50 * slot; }
static void toolbar_destroy(u32 widget) {
    /* Both sibling objects share an allocation; only LineView releases it. */
    CALL1(0x5339a874)(widget+0xf0);
    CALL1(0x53393d54)(widget);
}
static void toolbar_visible(u32 widget,u32 visible) {
    CALL2(0x533ad3c0)(widget,visible);
    CALL2(0x533ad3c0)(widget+0xf0,visible);
}
static void toolbar_layout(u32 controller,u32 expanded) {
    u32 i, origin=expanded ? 415 : 425, pitch=expanded ? 50 : 60;
    for(i=0;i<5;++i) {
        CALL2(0x533ac568)(WORD(controller,0x16c+i*4),origin+pitch*i);
        CALL2(0x533ac568)(WORD(controller,0x184+i*4),origin+pitch*i+5);
    }
    CALL2(0x533ac568)(WORD(controller,0x180),expanded ?
        toolbar_x(private_mode(controller) ? 5 : BYTE(controller,0x1f4)) :
        425+60*BYTE(controller,0x1f4));
}
static void toolbar(u32 controller) {
    volatile u32 *h=menu_header();
    u32 widget=h[GRKIT_HOST_TOOLBAR_WIDGET/4];
    if(widget && h[GRKIT_HOST_TOOLBAR_OWNER/4]!=controller) return;
    if(BYTE(controller,0x270)==1) {
        if(widget) toolbar_visible(widget,0);
        toolbar_layout(controller,0);
        return;
    }
    if(!widget) {
        u32 parent=WORD(WORD(controller,0x184),4);
        u32 position=WORD(parent,0xc), end=WORD(parent,0x10), icon, offset;
        u32 binding[3]={(u32)plugin_touch,0,controller};
        u32 callback[4]={(u32)binding,0,0x531a16c4,0x531a1874};
        /* Draw the native background before the shared selection frame. */
        while(position!=end && WORD(position,0)!=WORD(controller,0x180)) position+=4;
        if(position==end) return;
        offset=position-WORD(parent,0xc);
        widget=CALL1(0x536af2a8)(0x1e8);
        if(!widget) return;
        icon=widget+0xf0;
        CALL5(0x53393d84)(widget,toolbar_x(5),5,50,50);
        CALL2(0x53393e00)(widget,0x10);
        CALL5(0x5339a908)(icon,toolbar_x(5)+5,10,40,40);
        CALL2(0x5339a9a0)(icon,0x7ffe);
        CALL2(0x533abeb0)(icon,(u32)callback);
        CALL2(0x533ac1b4)(icon,1);
        /* Use the same vector insertion as View::AddChild, at the marker. */
        if(!CALL3(0x536b6de4)(parent+8,position,(u32)&widget)) {
            toolbar_destroy(widget);
            return;
        }
        /* The icon must draw after the frame, which fills the entire cell. */
        if(!CALL3(0x536b6de4)(parent+8,WORD(parent,0x10),(u32)&icon)) {
            CALL2(0x536b6f68)(parent+8,WORD(parent,0xc)+offset);
            toolbar_destroy(widget);
            return;
        }
        CALL2(0x533ac120)(widget,parent);
        CALL2(0x533ac120)(icon,parent);
        h[GRKIT_HOST_TOOLBAR_WIDGET/4]=widget;
        h[GRKIT_HOST_TOOLBAR_OWNER/4]=controller;
    }
    toolbar_visible(widget,1);
    toolbar_layout(controller,1);
    CALL2(0x533ad3c0)(WORD(controller,0x180),1);
}
A32_ENTRY void menu_refresh(u32 controller) {
    volatile u32 *h=menu_header();
    if(private_mode(controller)) { render(controller); return; }
    if(h[GRKIT_HOST_RETURN/4]==2 && BYTE(controller,0x270)!=1) {
        h[GRKIT_HOST_RETURN/4]=0;
        enter(controller,3);
        /* Fn starts only from Image Control, so it is visible. */
        menu_open(h,0);
        render(controller);
        return;
    }
    menu_refresh_original(controller);
    toolbar(controller);
    frame_preview(h,WORD(controller,4),BYTE(controller,0x270)==1);
}
/* First row of the cursor's page. The reciprocal equals row/6 for rows below 2^15
   and avoids a division helper, which the freestanding link does not provide. */
static u32 page_start(u32 row) { return (row*0xaaabU>>18)*PAGE; }
static void descriptor(volatile u32 *at, const u16 *name) {
    u32 length=0;
    while (length<GRKIT_SUMMARY_UNITS-1 && name[length]) ++length;
    at[0]=(length+1)|0x100; at[1]=(u32)name;
}
static void label(u32 object) {
    CALL3(0x53396f80)(object,0,0);
    BYTE(object,0xf3)=1;
    CALL1(0x533ad308)(object);
    CALL2(0x533ad3c0)(object,1);
}
static __attribute__((noinline)) void detail_width(u32 controller,u32 width) {
    u32 object=WORD(controller,0x164), rectangle[4]={0,50,width,40};
    CALL2(WORD(WORD(object,0),0x18))(object,(u32)rectangle);
}
static void render(u32 controller) {
    volatile u32 *h=menu_header();
    u32 cursor, start, n, i;
    u32 view=WORD(controller,0x1f0);
    const Module *module;
    menu_normalize(h);
    cursor=h[10]; start=page_start(cursor); n=count(h)-start;
    module=current_module(h);
    if(n>PAGE)n=PAGE;
    /* Suppress the native list callback while replacing private rows. */
    h[15]=1;
    CALL2(0x53394e8c)(view,n);
    for(i=0;i<n;++i) {
        u32 row=CALL2(0x53394758)(view,i);
        u32 index=start+i;
        /* Private icon IDs are resolved by the host's guarded icon adapter. */
        u32 icon=h[9]==MENU_ROOT ? modules[root_module(h,index)].icon(h,index) : module->icon(h,index);
        CALL2(0x533a1734)(row,icon);
        CALL2(0x533a19b4)(row,h[9]!=MENU_ROOT && module->checked(h,index));
    }
    CALL2(0x53395430)(view,cursor-start);
    descriptor(h+24,h[9]==MENU_ROOT ? plugins : module->title(h));
    descriptor(h+26,h[9]==MENU_ROOT ? module->summary(h) : module->label(h,cursor));
    label(WORD(controller,0x160));
    /* The summary ends before the list at x=620; native labels use 400 pixels. */
    detail_width(controller,h[9]==1 ? 620 : 400);
    label(WORD(controller,0x164));
    /* Native detail controls do not describe private modules, except the Fn
       hint (+0x1ac) where Fn opens the native parameters. */
    CALL2(0x533ad3c0)(WORD(controller,0x168),0);
    CALL2(0x533ad3c0)(WORD(controller,0x1ac),parameters_available(controller));
    toolbar(controller);
    /* Slot zero's canvas is at +4, as in the live view. */
    frame_preview(h,WORD(controller,4),BYTE(controller,0x270)==1);
    h[15]=0;
}
static void leave(u32 controller) {
    volatile u32 *h=menu_header();
    if(!private_mode(controller)) return;
    h[9]=0; h[15]=1;
    h[GRKIT_HOST_PREVIEW/4]=0;
    detail_width(controller,400);
    /* Rebuild the original list before its exit callback saves a cursor. */
    CALL1(0x531a4114)(controller);
    CALL2(0x533ad3c0)(WORD(controller,0x180),1);
    CALL3(0x53395224)(WORD(controller,0x1f0),h[16],h[17]);
    h[8]=0; h[15]=0;
}
static void enter(u32 controller,u32 edge) {
    volatile u32 *h=menu_header();
    u32 view=WORD(controller,0x1f0);
    h[8]=controller; menu_start(h);
    h[11]=BYTE(controller,0x1f4); h[22]=edge;
    h[GRKIT_HOST_PREVIEW/4]=0;
    CALL3(0x533947f0)(view,(u32)(h+16),(u32)(h+17));
    render(controller);
}
static void plugin_touch(u32 controller) {
    if(!private_mode(controller) && BYTE(controller,0x270)!=1) enter(controller,3);
}
/* Open the native Standard parameter page, as the native ADJ Image Control slot
   does for event 0x1b with argument 2. GRKit effects apply these adjustments.
   Refresh the model snapshot first: a cancel restores its type persistently. */
static u32 open_parameters(u32 controller) {
    if(!parameters_available(controller)) return 0;
    /* Like the native slot, edit the active parameters, including a custom set. */
    CALL1(0x531a0150)(controller);
    leave(controller);
    /* The page returns to ADJ (0x2000000) with the native Image Control slot
       focused, where OK would confirm Standard. Come back to GRKit instead. */
    menu_header()[GRKIT_HOST_RETURN/4]=1;
    CALL3(0x5316c9d8)(controller,0x2000014,0x2000000);
    return 1;
}
A32_ENTRY void menu_native_touch(u32 controller,u32 slot) {
    leave(controller);
    if(BYTE(controller,0x1f4)!=slot) CALL2(0x531a4864)(controller,slot);
    native_focus(controller);
}
A32_ENTRY void menu_exit(u32 controller) {
    volatile u32 *h=menu_header();
    leave(controller);
    h[GRKIT_HOST_BACK_PENDING/4]=0;
    h[GRKIT_HOST_PREVIEW/4]=0;
    h[GRKIT_HOST_FRAME_PREVIEW/4]=0;
    /* ADJ exits once for the parameter page itself. Any other exit cancels the return. */
    h[GRKIT_HOST_RETURN/4]=h[GRKIT_HOST_RETURN/4]==1 ? 2 : 0;
    menu_exit_original(controller);
    if(menu_header()[GRKIT_HOST_TOOLBAR_OWNER/4]==controller) {
        toolbar_visible(menu_header()[GRKIT_HOST_TOOLBAR_WIDGET/4],0);
        toolbar_layout(controller,0);
    }
}
A32_ENTRY u32 menu_destroy(u32 controller) {
    volatile u32 *h=menu_header();
    u32 widget=0, result;
    if(h[GRKIT_HOST_TOOLBAR_OWNER/4]==controller) {
        widget=h[GRKIT_HOST_TOOLBAR_WIDGET/4];
        h[GRKIT_HOST_TOOLBAR_WIDGET/4]=0;
        h[GRKIT_HOST_TOOLBAR_OWNER/4]=0;
        /* The root clears its child vector without deleting its children. */
        CALL2(WORD(WORD(widget,0),0x2c))(widget,0);
        CALL2(0x533ac120)(widget+0xf0,0);
    }
    if(h[8]==controller) { h[8]=0; h[9]=0; }
    h[GRKIT_HOST_PREVIEW/4]=0;
    h[GRKIT_HOST_FRAME_PREVIEW/4]=0;
    h[GRKIT_HOST_FRAME_PREVIEW_CANVAS/4]=0;
    result=menu_destroy_original(controller);
    if(widget) toolbar_destroy(widget);
    return result;
}
A32_ENTRY void menu_changed(u32 controller) {
    volatile u32 *h=menu_header();
    /* Restoration sets the level to native before rebuilding the list. */
    if(h[15]) return;
    if(h[8]==controller && h[9]) {
        u32 next=page_start(h[10])+CALL1(0x53394820)(WORD(controller,0x1f0));
        if(next<count(h)) { h[10]=next; image_preview(h); render(controller); }
        return;
    }
    menu_changed_original(controller);
}
A32_ENTRY u32 menu_event(u32 controller,const u16 *event) {
    volatile u32 *h=menu_header();
    u32 kind=*event, back;
    h[18]=kind;
    h[28+(h[19]&15)]=kind | ((kind==0x1b || kind==5 || kind==8) ? ((u32)*(u8 *)WORD((u32)event,4)<<16) :
        kind==0x30 ? (u32)BYTE(WORD((u32)event,4),0x50)<<16 : 0);
    ++h[19];
    if(kind==5 && !menu_press(BYTE(WORD((u32)event,4),0),WORD(WORD((u32)event,4),4))) return 0;
    back=back_event(h,kind,private_mode(controller));
    if(back==2) return 0;
    if(back==1) {
        if(menu_back(h)) render(controller);
        else leave(controller);
        return 0;
    }
    if(!private_mode(controller)) {
        /* OK or ADJ press on the native Image Control slot confirms a native value.
           Confirming the existing native type can omit its setter entirely. */
        if(kind==0xd8 && !after_menu_switch(h) &&
           CALL2(0x531a2820)(controller,BYTE(controller,0x1f4))==12)
            h[GRKIT_HOST_ACTIVE/4]=0;
        if(BYTE(controller,0x270)!=1 &&
           ((kind==0xcf && BYTE(controller,0x1f4)==4) ||
            (kind==0xce && BYTE(controller,0x1f4)==0))) {
            enter(controller,kind==0xcf ? 1 : 2); return 0;
        }
        {
            u32 old=BYTE(controller,0x1f4);
            u32 result=menu_event_original(controller,event);
            u32 next=BYTE(controller,0x1f4);
            u32 edge=wrapped_edge(kind,old,next,BYTE(controller,0x270));
            if(edge) enter(controller,edge);
            else if(next!=old) native_focus(controller);
            return result;
        }
    }
    menu_normalize(h);
    /* On an effect row, commit the highlighted row, then adjust its parameters.
       Fn on Off does nothing. */
    if(kind==0x1b && *(u8 *)WORD((u32)event,4)==2 &&
       h[9]==MENU_MODULE && !h[GRKIT_HOST_MODULE/4]) {
        if(!parameters_row(h)) return 0;
        image_apply(h,h[10]);
        if(!open_parameters(controller)) render(controller);
        return 0;
    }
    if(kind==0xce || kind==0xcf) {
        /* A single module has no root level: left leaves, as from the root. */
        if(h[9]>MENU_ROOT && kind==0xce && menu_back(h)) {
            render(controller); return 0;
        }
        {
            u32 edge=h[22], advance=advance_from_private(kind,edge);
            leave(controller);
            if(edge==3) {
                u32 i;
                for(i=0;i<5;++i) {
                    u32 slot=kind==0xce ? 4-i : i;
                    if(!CALL2(0x531a2820)(controller,slot)) continue;
                    menu_native_touch(controller,slot);
                    if(BYTE(controller,0x1f4)==slot) break;
                }
                return 0;
            }
            return advance ? menu_event_original(controller,event) : 0;
        }
    }
    if(kind==0xd0 || kind==0xd4 || kind==0xd1 || kind==0xd5) {
        u32 n=count(h);
        if(kind==0xd0 || kind==0xd4) h[10]=h[10] ? h[10]-1 : n-1;
        else { ++h[10]; if(h[10]==n) h[10]=0; }
        image_preview(h);
        render(controller); return 0;
    }
    /* Confirm is 0xd8. Press and release events do not activate a profile. */
    if(kind==0xd8) {
        menu_confirm(h);
        render(controller); return 0;
    }
    /* Ordinary touch notifications must not dismiss a private page. */
    if(kind==0x30 && native_action_release(BYTE(WORD((u32)event,4),0x50),BYTE(controller,0x279))) {
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
/* The playback metadata singleton holds key 0x25 at +0x220. */
u32 menu_playback_private(void) {
    return playback_icon(menu_header(),WORD(CALL0(0x5320b97c)(),0x220));
}
/* IDs stay in transient private widgets and never enter native settings. */
A32_ENTRY u32 menu_icon(u32 id) {
    volatile u32 *h=menu_header();
    u32 index=id-0x8000;
    /* Module tiles are contiguous from the first Framing tile. */
    if(id-FRAMING_ICON<MODULE_ICON_COUNT)
        return h[GRKIT_HOST_FRAMING_ICON/4]+(id-FRAMING_ICON)*GRKIT_ICON_STRIDE;
    if(id==0x7ffe) return h[GRKIT_HOST_PLUGINS_ICON/4];
    if(id==0x7fff) return h[GRKIT_HOST_OFF_ICON/4] ? h[GRKIT_HOST_OFF_ICON/4] : WORD(0x550020fc,0x166*4);
    if(index>=h[GRKIT_HOST_COUNT/4]) return 0;
    return WORD(h[GRKIT_HOST_RECORDS/4]+index*GRKIT_RECORD_SIZE,GRKIT_RECORD_ICON);
}
