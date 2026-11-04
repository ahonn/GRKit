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

/* RAW development of a DNG to a Framing ratio (GRKIT_HOST_DEVELOP_RATIO).
   The development of a ratio sends a capture aspect id to the pipeline
   (0 3:2, 1 4:3, 2 16:9, 3 1:1) and sets GRKIT_HOST_FRAME_FILE to the
   geometry of the ratio: 1 for the stock sizes of that id, 2 XPan, 3 2.35:1,
   4 5:4, 5 7:6 (runtime/framing.S). 1:1 is the native 1:1 of the camera. */
static const u8 develop_ratio_capture[GRKIT_CORE_ASPECT_VALUES]={0,2,2,1,2,3,1,1};
static const u8 develop_ratio_file[GRKIT_CORE_ASPECT_VALUES]={1,2,1,1,3,1,4,5};
static __attribute__((unused)) u32 develop_capture(u32 ratio) {
    return ratio<GRKIT_CORE_ASPECT_VALUES ? develop_ratio_capture[ratio] : 0;
}
static __attribute__((unused)) u32 develop_file_of(u32 ratio) {
    return ratio<GRKIT_CORE_ASPECT_VALUES ? develop_ratio_file[ratio] : 1;
}
/* The ratio of GRKit row `row` of the aspect list of a development, or 0
   past the last row. `captures` has bit n set when a native row (not the row
   of the recorded aspect) develops capture aspect n. A ratio follows the
   native rows when no native row has its stock geometry, and when the
   development can crop it: the camera offers 3:2, which keeps the whole
   sensor, or the capture aspect of the ratio. The rows keep the order of
   the Framing menu. */
static __attribute__((unused)) u32 develop_ratio_row(u32 captures,u32 row) {
    u32 i, ratio, capture;
    for(i=1;i<sizeof(framing_rows);++i) {
        ratio=framing_rows[i];
        capture=develop_capture(ratio);
        if(develop_file_of(ratio)==1 && captures>>capture&1U) continue;
        if(!(captures&1U) && !(captures>>capture&1U)) continue;
        if(!row--) return ratio;
    }
    return 0;
}
static __attribute__((unused)) u32 develop_ratio_rows(u32 captures) {
    u32 rows=0;
    while(develop_ratio_row(captures,rows)) ++rows;
    return rows;
}

#endif
