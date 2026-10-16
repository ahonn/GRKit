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
#define ARRAY_COUNT(a) (sizeof(a)/sizeof((a)[0]))

typedef struct {
    const u16 *(*title)(volatile u32 *);
    const u16 *(*summary)(volatile u32 *);
    u32 (*rows)(volatile u32 *);
    const u16 *(*label)(volatile u32 *,u32);
    u32 (*icon)(volatile u32 *,u32);
    u32 (*checked)(volatile u32 *,u32);
    void (*apply)(volatile u32 *,u32);
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
    return value<ARRAY_COUNT(ratio_labels) ? value : 0;
}
static const u16 *frame_title(volatile u32 *h) { (void)h; return framing; }
static const u16 *frame_summary(volatile u32 *h) { return frame_summaries[frame_ratio(h)]; }
static u32 frame_rows(volatile u32 *h) { (void)h; return ARRAY_COUNT(ratio_labels); }
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
/* ADJ covers the live-view mask, so ADJ draws a mask on its own canvas: the
   highlighted ratio on the Framing page, else the committed ratio. Off ends
   the mask, and the canvas stays until a full draw has erased it. */
static void frame_preview(volatile u32 *h,u32 canvas,u32 movie) {
    u32 page=h[9]==MENU_MODULE && h[GRKIT_HOST_MODULE/4]==FRAMING_MODULE;
    u32 ratio=page ? h[10] : frame_ratio(h);
    if(movie) { h[GRKIT_HOST_FRAME_PREVIEW/4]=0; return; }
    /* A highlighted Off keeps the canvas for the next row. */
    h[GRKIT_HOST_FRAME_PREVIEW/4]=ratio || page ? ratio+1 : 0;
    h[GRKIT_HOST_FRAME_PREVIEW_CANVAS/4]=canvas;
}

static const Module modules[] = {
    {image_title,image_summary,image_rows,image_label,image_icon,image_checked,image_apply},
    {frame_title,frame_summary,frame_rows,frame_label,frame_icon,frame_checked,frame_apply}
};
static const Module *current_module(volatile u32 *h) {
    u32 index=h[9]==MENU_ROOT ? h[10] : h[GRKIT_HOST_MODULE/4];
    return &modules[index<ARRAY_COUNT(modules) ? index : 0];
}
static u32 count(volatile u32 *h) {
    return h[9]==MENU_ROOT ? ARRAY_COUNT(modules) : current_module(h)->rows(h);
}
static void menu_normalize(volatile u32 *h) {
    if(h[9]<MENU_ROOT || h[9]>MENU_MODULE || h[GRKIT_HOST_MODULE/4]>=ARRAY_COUNT(modules)) {
        h[9]=MENU_ROOT;
        h[GRKIT_HOST_MODULE/4]=0;
    }
    if(h[10]>=count(h)) h[10]=0;
}
static void menu_confirm(volatile u32 *h) {
    menu_normalize(h);
    if(h[9]==MENU_ROOT) {
        u32 icon=selected_icon(h), module=h[10];
        h[GRKIT_HOST_MODULE/4]=module;
        h[9]=MENU_MODULE;
        /* Each module opens at its committed row. Image Control previews from there. */
        h[10]=module ? frame_ratio(h) : icon==0x7fff ? 0 : icon-0x7fff;
    } else current_module(h)->apply(h,h[10]);
}
/* Return false at the root so the caller can restore the stock ADJ page. */
static u32 menu_back(volatile u32 *h) {
    /* Leaving a level without OK restores the committed effect in the live view. */
    h[GRKIT_HOST_PREVIEW/4]=0;
    menu_normalize(h);
    if(h[9]==MENU_ROOT) return 0;
    h[9]=MENU_ROOT;
    h[10]=h[GRKIT_HOST_MODULE/4];
    return 1;
}
#endif
