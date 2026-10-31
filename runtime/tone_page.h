#ifndef GRKIT_TONE_PAGE_H
#define GRKIT_TONE_PAGE_H
/* The rows of a module page of CORE.TONE (runtime/tone.h), for the menu.
   The includer defines MENU_TEXT and includes runtime/setting_store.h and
   the menu functions of runtime/effects.h first. */
/* The page shows Off, then the installed effects that are On, in the order
   of their choices. The order and the choices apply at once
   (runtime/effects.h). The record index of each shown effect goes
   to `records`; the count returns. Without a listed effect, each record
   shows in its order. */
static u32 tone_shown(volatile u32 *h,u8 *records) {
    u8 order[EFFECTS_LISTED];
    u32 n=effects_count(h), i, k=0, record;
    if(!n) {
        for(i=0;i<h[GRKIT_HOST_COUNT/4] && i<GRKIT_TEXT_RECORDS;++i) records[k++]=(u8)i;
        return k;
    }
    effects_order(h,order,n);
    for(i=0;i<n;++i) {
        if(effects_off(h,order[i])) continue;
        record=effects_record(h,order[i]);
        if(record!=EFFECTS_NO_RECORD) records[k++]=(u8)record;
    }
    return k;
}
/* The record index of row `row` (1 or more), or EFFECTS_NO_RECORD. */
static u32 tone_row_index(volatile u32 *h,u32 row) {
    u8 records[GRKIT_TEXT_RECORDS];
    u32 k=tone_shown(h,records);
    return row && row<=k ? records[row-1] : EFFECTS_NO_RECORD;
}
/* Off and the shown effects. */
static u32 tone_page_values(volatile u32 *h) {
    u8 records[GRKIT_TEXT_RECORDS];
    return tone_shown(h,records)+1;
}
/* The record of row `row`, or 0 for Off. */
static u32 tone_row_record(volatile u32 *h,u32 row) {
    u32 index=tone_row_index(h,row);
    return index!=EFFECTS_NO_RECORD ? tone_record(h,index+1) : 0;
}
/* The row of the committed effect, or 0 for Off or a hidden effect. */
static u32 tone_committed(volatile u32 *h) {
    u8 records[GRKIT_TEXT_RECORDS];
    u32 k=tone_shown(h,records), value=tone_value(h), i;
    for(i=0;value && i<k;++i) if(records[i]==value-1) return i+1;
    return 0;
}
static const u16 *tone_summary(volatile u32 *h) {
    u32 row=tone_value(h);
    return row ? effect_summary(h,row-1) : text(h,TEXT_IMAGE_OFF_SUMMARY);
}
static u32 tone_rows(volatile u32 *h) { return tone_page_values(h); }
static const u16 *tone_label(volatile u32 *h,u32 row) {
    u32 record=tone_row_record(h,row);
    return record ? (const u16 *)(record+GRKIT_RECORD_NAME) : text(h,TEXT_OFF);
}
/* The root tile shows the selected effect, or the tile 0 of the module
   for Off (runtime/modules.h). */
static u32 choice_off_tile(volatile u32 *h);
static u32 tone_icon(volatile u32 *h,u32 row) {
    u32 index;
    if(h[GRKIT_HOST_MENU_LEVEL/4]==MENU_ROOT) {
        row=tone_value(h);
        return row ? GRKIT_ICON_EFFECT(row-1) : choice_off_tile(h);
    }
    index=tone_row_index(h,row);
    return index!=EFFECTS_NO_RECORD ? GRKIT_ICON_EFFECT(index) : GRKIT_ICON_OFF;
}
static u32 tone_checked(volatile u32 *h,u32 row) {
    return h[GRKIT_HOST_ACTIVE/4]==tone_row_record(h,row);
}
static void tone_apply(volatile u32 *h,u32 row) {
    u32 index=tone_row_index(h,row);
    tone_store(h,index!=EFFECTS_NO_RECORD ? index+1 : 0);
}
/* Highlighting a row previews it in the live view. Off previews native
   processing. */
static void tone_preview(volatile u32 *h) {
    u32 record=tone_row_record(h,h[GRKIT_HOST_MENU_CURSOR/4]);
    h[GRKIT_HOST_PREVIEW/4]=record ? record : 1;
}
/* Fn adjusts the native Standard parameters of an effect row. Off has
   none. */
static u32 tone_parameters(volatile u32 *h,u32 row) { return row!=0 && row<tone_page_values(h); }
#endif
