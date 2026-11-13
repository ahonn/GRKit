#ifndef GRKIT_MENU_ABOUT_H
#define GRKIT_MENU_ABOUT_H
/* About GRKit on the native camera information page (INFO_CONTROLLER,
   runtime/menu.c). The build header gives the lines of the page that show
   the version and the address (INFO_LINES), or the text IDs of its labels
   (INFO_LABEL_TEXTS). */
#if !defined(INFO_LINES) && !defined(INFO_LABEL_TEXTS)
#error "The build header defines INFO_LINES or INFO_LABEL_TEXTS"
#endif
static u32 about_mode(volatile u32 *h) {
    return h[GRKIT_HOST_SERVICES/4] ? services_word(h,SERVICES_MENU+MENU_ROWS_ABOUT)[0] : 0;
}
static u32 about_shows(volatile u32 *h) { return about_mode(h)==MENU_ABOUT_GRKIT; }
/* The View of a recipe: the page keeps its title, with the name of the
   recipe, and hides its lines; GRKit paints the settings on its canvas
   (runtime/viewfinder.h: view_paint). */
#define INFO_VIEWS_FIRST 0x150U
#define INFO_VIEWS_END 0x180U
/* The event method of the information page while it shows the View of a
   recipe: Up and Down, and the dial, turn the pages of the View; the native
   method takes each other event. The page gets the event method in the
   slot of the own value pages (SELECT_EVENT_SLOT), as the controllers of a
   family share the layout of the vtable. A taken event returns 1. */
#define VIEW_EVENT_SLOT SELECT_EVENT_SLOT
_Static_assert(VIEW_EVENT_SLOT<DETAIL_VIEW_VTABLE_SLOTS,"The copy holds the event method");
A32_ENTRY u32 menu_view_event(u32 controller,const u16 *event) {
    volatile u32 *h=menu_header();
    volatile u32 *details=services_word(h,SERVICES_DETAILS);
    u32 kind=*event, page;
    if(about_mode(h)==MENU_ABOUT_RECIPE && details[DETAIL_VIEW_CONTROLLER/4]==controller &&
       (kind==EV_UP || kind==EV_DOWN || kind==EV_DIAL_UP || kind==EV_DIAL_DOWN)) {
        page=detail_page_turn(details[DETAIL_VIEW_PAGE/4],details[DETAIL_COUNT/4],kind==EV_DOWN || kind==EV_DIAL_DOWN);
        if(page!=details[DETAIL_VIEW_PAGE/4]) {
            details[DETAIL_VIEW_PAGE/4]=page;
            MENU_VIEW1(FW_VIEW_INVALIDATE,MENU_OBJECT_WORD(controller,ADJ_CANVAS));
        }
        return 1;
    }
    return CALL2(WORD(details[DETAIL_VIEW_VTABLE/4],4U*VIEW_EVENT_SLOT))(controller,(u32)event);
}
/* Give the information page `controller` the event method of the View: its
   vtable becomes the copy in the details block. */
static void view_events(volatile u32 *h,u32 controller) {
    volatile u32 *at=services_word(h,SERVICES_DETAILS+DETAIL_VIEW_VTABLE);
    u32 vtable=MENU_OBJECT_WORD(controller,0), i;
    if(!vtable || vtable==(u32)(at+2)) return;
    at[0]=vtable;
    for(i=0;i<1U+DETAIL_VIEW_VTABLE_SLOTS;++i) at[1+i]=WORD(vtable-4U,4U*i);
    at[2+VIEW_EVENT_SLOT]=(u32)menu_view_event;
    MENU_OBJECT_WORD(controller,0)=(u32)(at+2);
}
/* A controller change ends the View: the information page gets its native
   vtable again. */
static void view_events_end(volatile u32 *h) {
    volatile u32 *details=services_word(h,SERVICES_DETAILS);
    u32 controller=details[DETAIL_VIEW_CONTROLLER/4];
    if(controller && MENU_OBJECT_WORD(controller,0)==(u32)(details+DETAIL_VIEW_VTABLE/4+2U))
        MENU_OBJECT_WORD(controller,0)=details[DETAIL_VIEW_VTABLE/4];
    details[DETAIL_VIEW_CONTROLLER/4]=0;
}
/* The title view measures its text only when its text ID changes, so the
   name of a recipe kept the width of About GRKit, and About GRKit the width
   of the name of the last recipe. Set the ID again. */
static void info_title_again(u32 controller) {
    u32 i, view;
    for(i=INFO_VIEWS_FIRST;i<INFO_VIEWS_END;i+=4) {
        view=MENU_OBJECT_WORD(controller,i);
        if(view<0x40000000U || view>=0x80000000U || (view&3U) || MENU_OBJECT_HALF(view,STRING_VIEW_TEXT)!=INFO_TITLE_TEXT) continue;
        MENU_VIEW3(FW_STRING_VIEW_RESET,view,0,0);
        MENU_VIEW3(FW_STRING_VIEW_RESET,view,INFO_TITLE_TEXT,0);
    }
}
static void view_fill(volatile u32 *h,u32 controller) {
#ifdef INFO_LINES
    static const u16 views[2]=INFO_LINES;
#else
    static const u16 views[6]=INFO_LABELS_VALUES;
#endif
    u32 i;
    for(i=0;i<sizeof(views)/sizeof(views[0]);++i) MENU_VIEW2(FW_VIEW_SET_VISIBLE,MENU_OBJECT_WORD(controller,views[i]),0);
    info_title_again(controller);
    services_word(h,SERVICES_DETAILS+DETAIL_VIEW_CONTROLLER)[0]=controller;
    view_events(h,controller);
    MENU_VIEW1(FW_VIEW_INVALIDATE,MENU_OBJECT_WORD(controller,ADJ_CANVAS));
}
/* Copy ASCII text, at most `room` - 1 units, after `at` units of `to`. */
static u32 ascii_add(volatile char *to,u32 at,const u16 *from,u32 room) {
    while(at<room-1 && *from) to[at++]=(char)*from++;
    to[at]=0;
    return at;
}
static const u16 about_prefix[]={'G','R','K','i','t',' ','V','e','r','.',0};
/* The update of the page ended (menu.S: menu_info_update): show the version
   and the project address in its lines. */
A32_ENTRY void menu_info_fill(u32 controller) {
    volatile u32 *h=menu_header();
    volatile char *version, *project;
    if(about_mode(h)==MENU_ABOUT_RECIPE) { view_fill(h,controller); return; }
    if(!about_shows(h)) return;
    info_title_again(controller);
    version=(volatile char *)services_word(h,SERVICES_MENU+MENU_ROWS_ABOUT_LINES);
    project=version+MENU_ABOUT_LINE;
    ascii_add(project,0,text(h,TEXT_ABOUT+4),MENU_ABOUT_LINE);
#ifdef INFO_LINES
    {
        static const u16 lines[2]=INFO_LINES;
        ascii_add(version,ascii_add(version,0,about_prefix,MENU_ABOUT_LINE),text(h,TEXT_ABOUT+2),MENU_ABOUT_LINE);
        MENU_VIEW2(FW_STRING_VIEW_SET_ASCII,MENU_OBJECT_WORD(controller,lines[0]),(u32)version);
        MENU_VIEW2(FW_STRING_VIEW_SET_ASCII,MENU_OBJECT_WORD(controller,lines[1]),(u32)project);
    }
#else
    {
        static const u16 labels[3]=INFO_LABELS, values[3]=INFO_VALUES;
        ascii_add(version,0,text(h,TEXT_ABOUT+2),MENU_ABOUT_LINE);
        MENU_VIEW2(FW_STRING_VIEW_SET_ASCII,MENU_OBJECT_WORD(controller,values[0]),(u32)version);
        MENU_VIEW2(FW_STRING_VIEW_SET_ASCII,MENU_OBJECT_WORD(controller,values[1]),(u32)project);
        MENU_VIEW2(FW_VIEW_SET_VISIBLE,MENU_OBJECT_WORD(controller,labels[2]),0);
        MENU_VIEW2(FW_VIEW_SET_VISIBLE,MENU_OBJECT_WORD(controller,values[2]),0);
    }
#endif
}
/* The title of the page reads About GRKit, and on GR IV its labels Version
   and Project. */
static u32 info_text(volatile u32 *h,u32 id) {
    volatile u32 *at;
    u32 n=3;
#ifdef INFO_LABEL_TEXTS
    static const u16 labels[3]=INFO_LABEL_TEXTS;
#endif
    if(about_mode(h)==MENU_ABOUT_RECIPE) {
        u32 e=recipe_page_shown(h);
        if(id!=INFO_TITLE_TEXT || e>=GRKIT_RECIPE_ON_MAX) return 0;
        at=services_word(h,SERVICES_MENU+MENU_ROWS_ABOUT_TEXTS);
        descriptor(at,(const u16 *)recipe_text_at(h,(e+1U)*RECIPE_TEXT_SIZE));
        return (u32)at|1U;
    }
    if(!about_shows(h)) return 0;
    at=services_word(h,SERVICES_MENU+MENU_ROWS_ABOUT_TEXTS);
    if(id==INFO_TITLE_TEXT) n=0;
#ifdef INFO_LABEL_TEXTS
    else if(id==labels[0]) n=1;
    else if(id==labels[1]) n=2;
#endif
    if(n==3) return 0;
    descriptor(at+n*2,text(h,TEXT_ABOUT+(n==0 ? 0 : n==1 ? 1 : 3)));
    return (u32)(at+n*2)|1U;
}
#endif /* GRKIT_MENU_ABOUT_H */
