#ifndef GRKIT_DISTANCE_H
#define GRKIT_DISTANCE_H
/* Distance Unit (runtime/manage.h) in feet: the Snap focus distances and
   the normal manual focus scale. */
/* The texts of the Snap focus
   distances. The camera has a text ID for each distance, SNAP_DISTANCE_TEXTS
   IDs from SNAP_DISTANCE_TEXT of the build header: 0.3, 0.4, 1, 1.5, 2, 2.5,
   3.5, and 5 m, the same in each language. The text hooks show the feet
   label of each instead, so a change of the unit applies at once. Infinity
   has no number and keeps its text. The labels round to whole feet; 0.4 m,
   which no Snap setting offers, shows as 0.3 m does.

   A string view lays out its text when its text ID changes, and keeps that
   layout while the ID stays: a list of the Snap distances would show the
   texts of one unit in the widths of the other. So the string view entry
   (menu_string_text) lays out a Snap distance again when the view showed it
   in the other unit, or in a unit that the core did not note. */
#define SNAP_DISTANCE_TEXTS 8U
static const u16 snap_feet[SNAP_DISTANCE_TEXTS][4]={
    {'1','\'',0},{'1','\'',0},{'3','\'',0},{'5','\'',0},
    {'7','\'',0},{'8','\'',0},{'1','1','\'',0},{'1','6','\'',0}};
/* The text descriptor of text ID `id` in feet, or 0 for the native text. The
   descriptor is in the services block (SERVICES_DISTANCE). */
static u32 distance_text(volatile u32 *h,u32 id) {
    u32 i=id-SNAP_DISTANCE_TEXT;
    volatile u32 *at;
    if(i>=SNAP_DISTANCE_TEXTS || !h[GRKIT_HOST_SERVICES/4] || !unit_feet(h)) return 0;
    at=services_word(h,SERVICES_DISTANCE+DISTANCE_TEXTS+8U*i);
    descriptor(at,snap_feet[i]);
    return (u32)at;
}
/* The normal manual focus scale labels its marks at 0.3, 1, 3, and 5 m and
   shows the unit m. In feet, the same marks show 1, 3, 10, and 16, and the
   unit ft: the camera icons of 1, 3, and 10 (SCALE_ICON_* of the build
   header), and the GRKit icons of 16 and ft. Infinity and the macro scale in
   cm keep their icons. The hooks give the site of the label: the unit, then
   5, 3, 1, and 0.3 m (runtime/menu.S). */
#define SCALE_SITES 5U
/* Whether `view` shows its unit in `unit` now (bit 0), as the list of views
   at `list`, with `slots` slots and the next slot at `next`, notes it. The
   list notes the unit of the view, and a view that it did not note takes
   the next slot. */
static u32 distance_view_current(volatile u32 *h,u32 list,u32 slots,u32 next,u32 view,u32 unit) {
    volatile u32 *at=services_word(h,SERVICES_DISTANCE+list), *after=services_word(h,SERVICES_DISTANCE+next);
    u32 i;
    for(i=0;i<slots;++i) {
        if((at[i]&~1U)!=view) continue;
        if((at[i]&1U)==unit) return 1;
        at[i]=view|unit;
        return 0;
    }
    i=after[0]<slots ? after[0] : 0;
    at[i]=view|unit;
    after[0]=i+1<slots ? i+1 : 0;
    return 0;
}
/* The entry of the string view text setter (view, text ID): nonzero to lay
   out a Snap distance again. */
A32_ENTRY u32 menu_string_text(u32 view,u32 id) {
    volatile u32 *h=menu_header();
    if(id-SNAP_DISTANCE_TEXT>=SNAP_DISTANCE_TEXTS || !h[GRKIT_HOST_SERVICES/4]) return 0;
    return !distance_view_current(h,DISTANCE_VIEWS,DISTANCE_VIEW_SLOTS,DISTANCE_VIEW_NEXT,view,unit_feet(h));
}
/* The entry of the update of a manual focus scale: a scale that shows
   another unit, or one that the core did not note, such as one that the
   camera built before INSTALL, sets its icons again (FW_FOCUS_SCALE_ICONS),
   so it shows a change of the unit at its next update. */
A32_ENTRY void menu_scale_update(u32 scale) {
    volatile u32 *h=menu_header();
    if(!h[GRKIT_HOST_SERVICES/4] ||
       distance_view_current(h,DISTANCE_SCALES,DISTANCE_SCALE_SLOTS,DISTANCE_SCALE_NEXT,scale,unit_feet(h))) return;
    CALL1(FW_FOCUS_SCALE_ICONS)(scale);
}
A32_ENTRY u32 menu_scale_icon(u32 site,u32 id) {
    static const u16 feet[SCALE_SITES]={GRKIT_ICON_SCALE_FEET,GRKIT_ICON_SCALE_SIXTEEN,SCALE_ICON_TEN,
                                        SCALE_ICON_THREE,SCALE_ICON_ONE};
    volatile u32 *h=menu_header();
    return site<SCALE_SITES && unit_feet(h) ? feet[site] : id;
}
#endif /* GRKIT_DISTANCE_H */
