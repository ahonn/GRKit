#ifndef GRKIT_ASPECT_H
#define GRKIT_ASPECT_H
/* The capability CORE.ASPECT (grkit_core::capabilities): the selected
   composition ratio. Its value is 0 for Off, then XPan
   65:24, 16:9, 4:3, 2.35:1, 1:1, 5:4, and 7:6 (GRKIT_CORE_ASPECT_VALUES). Each
   target offers the ratios of FRAMING_RATIO_ROWS.

   CORE.ASPECT_MODE selects cropping (0, the default) or guides only (1).
   GRKIT_HOST_FRAME_RATIO keeps the selection in both modes. Capture paths
   use the effective crop, which is zero in guides-only mode. The resident
   loader saves both settings.

   The includer defines u8, u32, the header fields (runtime/abi.h), and
   aspect_native(). runtime/setting_store.h sets the value. */
/* The native still aspect id. Each component that includes this file
   defines it. */
static u32 aspect_native(void);
#ifndef CAPABILITY_USED
/* Whether a loaded module uses the capability `handle`. */
#define CAPABILITY_USED(h,handle) ((h)[GRKIT_HOST_CAPABILITIES/4]>>((handle)-GRKIT_CORE_SETTING_BASE)&1U)
#endif

static const u8 framing_rows[]={FRAMING_RATIO_ROWS};
/* The rows of the target, Off included, and the ratio value of a row. */
static __attribute__((unused)) u32 framing_ratios(void) { return sizeof(framing_rows); }
static __attribute__((unused)) u32 framing_ratio(u32 row) { return row<sizeof(framing_rows) ? framing_rows[row] : 0; }
/* The row of a ratio value, or 0 when the target does not offer it. */
static __attribute__((unused)) u32 framing_row(u32 value) {
    u32 row;
    for(row=1;row<sizeof(framing_rows);++row) if(framing_rows[row]==value) return row;
    return 0;
}
/* The selected ratio, independent of whether it crops recorded pictures. */
static __attribute__((unused)) u32 framing_selection(volatile u32 *h) {
    u32 ratio=h[GRKIT_HOST_FRAME_RATIO/4];
    if(!CAPABILITY_USED(h,GRKIT_CORE_ASPECT)) return 0;
    return ratio && framing_row(ratio) ? ratio : 0;
}
/* A module that does not use ASPECT_MODE keeps the existing crop behavior. */
static __attribute__((unused)) u32 framing_guides_only(volatile u32 *h) {
    return CAPABILITY_USED(h,GRKIT_CORE_ASPECT_MODE) && h[GRKIT_HOST_ASPECT_MODE/4]==1;
}
/* Only capture paths use this ratio. A composition frame never crops. */
static __attribute__((unused)) u32 framing_crop(volatile u32 *h) {
    return framing_guides_only(h) ? 0 : framing_selection(h);
}
/* Whether the target offers a ratio value. */
static __attribute__((unused)) u32 aspect_offered(u32 value) { return !value || framing_row(value); }

#ifndef FRAMING_CROP_KEEPS
/* A crop that applies in a native aspect other than 3:2 (GR IV: XPan and
   2.35:1 also crop native 16:9 photos). */
#ifdef FRAMING_XPAN_NATIVE
#define FRAMING_CROP_KEEPS(native,crop) ((native)==FRAMING_XPAN_NATIVE && ((crop)==1 || (crop)==4))
#else
#define FRAMING_CROP_KEEPS(native,crop) 0
#endif
#endif

/* CORE.ASPECT shows the selected composition in native 3:2. Crop mode also
   permits the target's supported native-aspect crops. Other native aspects
   read as Off without discarding the saved selection. */
static __attribute__((unused)) u32 aspect_value(volatile u32 *h) {
    u32 native=aspect_native(), ratio=framing_selection(h);
    return !native || (!framing_guides_only(h) && ratio && FRAMING_CROP_KEEPS(native,ratio)) ? ratio : 0;
}

/* The selected crop that applies to the current native capture geometry. */
static __attribute__((unused)) u32 capture_aspect_value(volatile u32 *h) {
    return framing_guides_only(h) ? 0 : aspect_value(h);
}

#endif
