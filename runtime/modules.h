#ifndef GRKIT_MODULES_H
#define GRKIT_MODULES_H

/* Modules own their settings. Native Image Control stores no private values. */
#define MENU_ROOT 1U
#define MENU_MODULE 2U
#define FRAMING_MODULE 1U
/* Framing artwork is contiguous: the module tile, then the three ratios. */
#define FRAMING_ICON 0x7ff0U
#define FRAMING_RATIO_ICON(ratio) (FRAMING_ICON+(ratio))
#define FRAMING_ICON_COUNT 4U
#define GUIDES_MODULE 2U
/* Guides artwork follows the Framing artwork: the module tile, then the four styles. */
#define GUIDES_ICON (FRAMING_ICON+FRAMING_ICON_COUNT)
#define GUIDE_STYLE_ICON(style) (GUIDES_ICON+(style))
#define GUIDES_ICON_COUNT 5U
#define STAMP_MODULE 3U
/* Date stamp artwork follows the Guides artwork: the module tile, then four styles. */
#define STAMP_ICON (GUIDES_ICON+GUIDES_ICON_COUNT)
#define STAMP_STYLE_ICON(style) (STAMP_ICON+(style))
#define STAMP_ICON_COUNT 5U
#define MODULE_ICON_COUNT (FRAMING_ICON_COUNT+GUIDES_ICON_COUNT+STAMP_ICON_COUNT)
#define ARRAY_COUNT(a) (sizeof(a)/sizeof((a)[0]))

typedef struct {
    const u16 *(*title)(volatile u32 *);
    const u16 *(*summary)(volatile u32 *);
    u32 (*rows)(volatile u32 *);
    const u16 *(*label)(volatile u32 *,u32);
    u32 (*icon)(volatile u32 *,u32);
    u32 (*checked)(volatile u32 *,u32);
    void (*apply)(volatile u32 *,u32);
    /* The committed row, where the module page opens. */
    u32 (*committed)(volatile u32 *);
} Module;

static const u16 *image_title(volatile u32 *h) { (void)h; return image_control; }
static const u16 *image_summary(volatile u32 *h) {
    return selected_icon(h)==0x7fff ? off_summary :
        (const u16 *)(h[GRKIT_HOST_ACTIVE/4]+GRKIT_RECORD_SUMMARY);
}
static u32 image_rows(volatile u32 *h) { return h[GRKIT_HOST_COUNT/4]+1; }
/* Row 0 is Off and has no record. */
static u32 image_record(volatile u32 *h,u32 row) {
    return row ? h[GRKIT_HOST_RECORDS/4]+(row-1)*GRKIT_RECORD_SIZE : 0;
}
static const u16 *image_label(volatile u32 *h,u32 row) {
    return row ? (const u16 *)(h[GRKIT_HOST_RECORDS/4]+(row-1)*GRKIT_RECORD_SIZE+GRKIT_RECORD_NAME) : off;
}
static u32 image_icon(volatile u32 *h,u32 row) {
    return h[9]==MENU_ROOT ? selected_icon(h) : row ? 0x8000+row-1 : 0x7fff;
}
static u32 image_checked(volatile u32 *h,u32 row) {
    return h[GRKIT_HOST_ACTIVE/4]==image_record(h,row);
}
static u32 image_committed(volatile u32 *h) {
    u32 icon=selected_icon(h);
    return icon==0x7fff ? 0 : icon-0x7fff;
}
/* OK commits the highlighted row and ends its live-view preview. */
static void image_apply(volatile u32 *h,u32 row) {
    h[GRKIT_HOST_ACTIVE/4]=image_record(h,row);
    h[GRKIT_HOST_PREVIEW/4]=0;
    ++h[20];
}
/* Highlighting a row previews it in the live view. Off previews native processing. */
static void image_preview(volatile u32 *h) {
    u32 record;
    if(h[9]!=MENU_MODULE || h[GRKIT_HOST_MODULE/4]) return;
    record=image_record(h,h[10]);
    h[GRKIT_HOST_PREVIEW/4]=record ? record : 1;
}

static const u16 framing[] = {'F','r','a','m','i','n','g',0};
/* 65:24 is named after the XPan panoramic format. */
static const u16 ratio_labels[][11] = {
    {'O','f','f',0}, {'X','P','a','n',' ','6','5',':','2','4',0},
    {'1','6',':','9',0}, {'4',':','3',0}
};
static const u16 frame_summaries[][15] = {
    {'F','r','a','m','i','n','g',':',' ','O','f','f',0},
    {'F','r','a','m','i','n','g',':',' ','X','P','a','n',0},
    {'F','r','a','m','i','n','g',':',' ','1','6',':','9',0},
    {'F','r','a','m','i','n','g',':',' ','4',':','3',0}
};
static u32 frame_ratio(volatile u32 *h) {
    u32 value=h[GRKIT_HOST_FRAME_RATIO/4];
    return value<FRAMING_RATIO_COUNT ? value : 0;
}
static const u16 *frame_title(volatile u32 *h) { (void)h; return framing; }
static const u16 *frame_summary(volatile u32 *h) { return frame_summaries[frame_ratio(h)]; }
static u32 frame_rows(volatile u32 *h) { (void)h; return FRAMING_RATIO_COUNT; }
static const u16 *frame_label(volatile u32 *h,u32 row) { (void)h; return ratio_labels[row]; }
/* The root tile shows the selected ratio, like Image Control shows its effect. */
static u32 frame_icon(volatile u32 *h,u32 row) {
    if(h[9]==MENU_ROOT) return frame_ratio(h) ? FRAMING_RATIO_ICON(frame_ratio(h)) : FRAMING_ICON;
    return row ? FRAMING_RATIO_ICON(row) : 0x7fff;
}
static u32 frame_checked(volatile u32 *h,u32 row) { return row==frame_ratio(h); }
static void frame_apply(volatile u32 *h,u32 row) {
    h[GRKIT_HOST_FRAME_RATIO/4]=row;
    MENU_FRAMING_REQUEST();
}

static const u16 guides[] = {'G','u','i','d','e','s',0};
static const u16 guide_labels[][13] = {
    {'O','f','f',0}, {'T','h','i','r','d','s',0}, {'C','e','n','t','e','r',0},
    {'G','o','l','d','e','n',' ','R','a','t','i','o',0}, {'D','i','a','g','o','n','a','l','s',0}
};
static const u16 guide_summaries[][21] = {
    {'G','u','i','d','e','s',':',' ','O','f','f',0},
    {'G','u','i','d','e','s',':',' ','T','h','i','r','d','s',0},
    {'G','u','i','d','e','s',':',' ','C','e','n','t','e','r',0},
    {'G','u','i','d','e','s',':',' ','G','o','l','d','e','n',' ','R','a','t','i','o',0},
    {'G','u','i','d','e','s',':',' ','D','i','a','g','o','n','a','l','s',0}
};
static u32 guide_style(volatile u32 *h) {
    u32 value=h[GRKIT_HOST_GUIDE/4];
    return value<ARRAY_COUNT(guide_labels) ? value : 0;
}
static const u16 *guide_title(volatile u32 *h) { (void)h; return guides; }
static const u16 *guide_summary(volatile u32 *h) { return guide_summaries[guide_style(h)]; }
static u32 guide_rows(volatile u32 *h) { (void)h; return ARRAY_COUNT(guide_labels); }
static const u16 *guide_label(volatile u32 *h,u32 row) { (void)h; return guide_labels[row]; }
/* The root tile shows the selected style, like Framing shows its ratio. */
static u32 guide_icon(volatile u32 *h,u32 row) {
    if(h[9]==MENU_ROOT) return guide_style(h) ? GUIDE_STYLE_ICON(guide_style(h)) : GUIDES_ICON;
    return row ? GUIDE_STYLE_ICON(row) : 0x7fff;
}
static u32 guide_checked(volatile u32 *h,u32 row) { return row==guide_style(h); }
static void guide_apply(volatile u32 *h,u32 row) { h[GRKIT_HOST_GUIDE/4]=row; }

static const u16 date_stamp[] = {'D','a','t','e',' ','S','t','a','m','p',0};
static const u16 stamp_labels[][13] = {
    {'O','f','f',0}, {'\'','Y','Y',' ','M',' ','D',0}, {'\'','Y','Y',' ','M',' ','D',' ','H',':','M','M',0},
    {'M',' ','D',' ','\'','Y','Y',0}, {'D',' ','M',' ','\'','Y','Y',0}
};
static const u16 stamp_summaries[][25] = {
    {'D','a','t','e',' ','S','t','a','m','p',':',' ','O','f','f',0},
    {'D','a','t','e',' ','S','t','a','m','p',':',' ','\'','Y','Y',' ','M',' ','D',0},
    {'D','a','t','e',' ','S','t','a','m','p',':',' ','\'','Y','Y',' ','M',' ','D',' ','H',':','M','M',0},
    {'D','a','t','e',' ','S','t','a','m','p',':',' ','M',' ','D',' ','\'','Y','Y',0},
    {'D','a','t','e',' ','S','t','a','m','p',':',' ','D',' ','M',' ','\'','Y','Y',0}
};
static u32 stamp_style(volatile u32 *h) {
    u32 value=h[GRKIT_HOST_STAMP/4];
    return value<ARRAY_COUNT(stamp_labels) ? value : 0;
}
static const u16 *stamp_title(volatile u32 *h) { (void)h; return date_stamp; }
static const u16 *stamp_summary(volatile u32 *h) { return stamp_summaries[stamp_style(h)]; }
static u32 stamp_rows(volatile u32 *h) { (void)h; return ARRAY_COUNT(stamp_labels); }
static const u16 *stamp_label(volatile u32 *h,u32 row) { (void)h; return stamp_labels[row]; }
static u32 stamp_icon(volatile u32 *h,u32 row) {
    if(h[9]==MENU_ROOT) return stamp_style(h) ? STAMP_STYLE_ICON(stamp_style(h)) : STAMP_ICON;
    return row ? STAMP_STYLE_ICON(row) : 0x7fff;
}
static u32 stamp_checked(volatile u32 *h,u32 row) { return row==stamp_style(h); }
/* Recorded pictures only: the encode hook prints the capture date. */
static void stamp_apply(volatile u32 *h,u32 row) { h[GRKIT_HOST_STAMP/4]=row; }

/* ADJ covers the live-view drawing, so ADJ draws the mask and guides on its
   own canvas. The Framing page shows its highlighted ratio and the Guides page
   its highlighted style, each with the other module's committed value. Off
   ends the drawing, and the canvas stays until a full draw has erased it. */
static void frame_preview(volatile u32 *h,u32 canvas,u32 movie) {
    u32 module=h[9]==MENU_MODULE ? h[GRKIT_HOST_MODULE/4] : 0;
    u32 page=module==FRAMING_MODULE || module==GUIDES_MODULE;
    u32 ratio=module==FRAMING_MODULE ? h[10] : frame_ratio(h);
    u32 style=module==GUIDES_MODULE ? h[10] : guide_style(h);
    if(movie) { h[GRKIT_HOST_FRAME_PREVIEW/4]=0; return; }
    h[GRKIT_HOST_GUIDE_PREVIEW/4]=style;
    /* A highlighted Off keeps the canvas for the next row. */
    h[GRKIT_HOST_FRAME_PREVIEW/4]=ratio || style || page ? ratio+1 : 0;
    h[GRKIT_HOST_FRAME_PREVIEW_CANVAS/4]=canvas;
}

static const Module modules[] = {
    {image_title,image_summary,image_rows,image_label,image_icon,image_checked,image_apply,image_committed},
    {frame_title,frame_summary,frame_rows,frame_label,frame_icon,frame_checked,frame_apply,frame_ratio},
    {guide_title,guide_summary,guide_rows,guide_label,guide_icon,guide_checked,guide_apply,guide_style},
    {stamp_title,stamp_summary,stamp_rows,stamp_label,stamp_icon,stamp_checked,stamp_apply,stamp_style}
};
/* Visible modules as a bit mask in registry order. Image Control shows exactly
   when effects are installed, and the others unless MODULES.TXT hides them.
   With nothing visible, Image Control shows with Off only. */
static u32 visible_modules(volatile u32 *h) {
    u32 all=(1U<<ARRAY_COUNT(modules))-1;
    u32 mask=~h[GRKIT_HOST_HIDDEN/4]&all&~1U;
    if(h[GRKIT_HOST_COUNT/4]) mask|=1;
    return mask ? mask : 1;
}
/* The module of a root row, or of the last row for a row past the end. */
static u32 root_module(volatile u32 *h,u32 row) {
    u32 mask=visible_modules(h), module=0, last=0;
    for(;module<ARRAY_COUNT(modules);++module) {
        if(!(mask>>module&1)) continue;
        if(!row--) return module;
        last=module;
    }
    return last;
}
static u32 root_row(volatile u32 *h,u32 module) {
    u32 mask=visible_modules(h), row=0, i;
    for(i=0;i<module && i<ARRAY_COUNT(modules);++i) row+=mask>>i&1;
    return row;
}
static u32 root_count(volatile u32 *h) { return root_row(h,ARRAY_COUNT(modules)); }
/* With one visible module, ADJ opens its page directly and has no root level. */
static u32 single_module(volatile u32 *h) { return root_count(h)==1; }
static const Module *current_module(volatile u32 *h) {
    return &modules[h[9]==MENU_ROOT ? root_module(h,h[10]) : h[GRKIT_HOST_MODULE/4]];
}
static u32 count(volatile u32 *h) {
    return h[9]==MENU_ROOT ? root_count(h) : current_module(h)->rows(h);
}
/* Each module opens at its committed row. Image Control previews from there. */
static void menu_open(volatile u32 *h,u32 module) {
    h[GRKIT_HOST_MODULE/4]=module;
    h[9]=MENU_MODULE;
    h[10]=modules[module].committed(h);
}
static void menu_start(volatile u32 *h) {
    h[9]=MENU_ROOT;
    h[10]=0;
    h[GRKIT_HOST_MODULE/4]=0;
    if(single_module(h)) menu_open(h,root_module(h,0));
}
static void menu_normalize(volatile u32 *h) {
    u32 module=h[GRKIT_HOST_MODULE/4];
    if(h[9]<MENU_ROOT || h[9]>MENU_MODULE || module>=ARRAY_COUNT(modules) ||
       (h[9]==MENU_MODULE && !(visible_modules(h)>>module&1)) ||
       (h[9]==MENU_ROOT && single_module(h)))
        menu_start(h);
    if(h[10]>=count(h)) h[10]=0;
}
static void menu_confirm(volatile u32 *h) {
    menu_normalize(h);
    if(h[9]==MENU_ROOT) menu_open(h,root_module(h,h[10]));
    else current_module(h)->apply(h,h[10]);
}
/* Return false at the top level so the caller can restore the stock ADJ page. */
static u32 menu_back(volatile u32 *h) {
    /* Leaving a level without OK restores the committed effect in the live view. */
    h[GRKIT_HOST_PREVIEW/4]=0;
    menu_normalize(h);
    if(h[9]==MENU_ROOT || single_module(h)) return 0;
    h[9]=MENU_ROOT;
    h[10]=root_row(h,h[GRKIT_HOST_MODULE/4]);
    return 1;
}
#endif
