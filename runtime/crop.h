#ifndef GRKIT_CROP_H
#define GRKIT_CROP_H
/* The capability CORE.CROP (grkit_core::capabilities): a focal-length crop
   of the lens. Its value is 0 for Off, then 35, 40, 50, and 70 mm of a
   28 mm lens; GR IIIx has the same factors of its 40 mm lens. The live view
   shows the crop, or the frame of the crop inside the layer area
   (runtime/viewfinder.h), and other layers, such as the guides, draw inside
   that frame. A longer crop has no value: even 70 mm shows only its frame
   (crop_view_value).

   CORE.CROP_MODE selects cropping (0, the default) or the frame only (1).
   The resident loader saves both settings.

   A crop is a crop of the camera in each native aspect and with each
   Framing ratio: the core sets the camera's own crop factor (FW_CROP_SET)
   to the factor of 35 or 50 mm, so that the live view, the focus, and the
   exposure follow it, and the window and the size of the photo follow the
   factor (runtime/framing.c: crop_resolution_size). The camera keeps 35 mm
   for 40 mm, and 50 mm for 70 mm; their shots, and the RAW developments of
   their DNGs, take their own factor (runtime/framing.c: crop_shot_copy).
   The live view zooms to 40 mm, and shows 70 mm as a frame in
   the view of 50 mm (crop_view_value). The camera saves its factor at
   power-off and knows only its own factors, so the core never sets another
   one.

   The includer defines u16, u32, and the header fields (runtime/abi.h). */
#ifndef CAPABILITY_USED
/* Whether a loaded module uses the capability `handle`. */
#define CAPABILITY_USED(h,handle) ((h)[GRKIT_HOST_CAPABILITIES/4]>>((handle)-GRKIT_CORE_SETTING_BASE)&1U)
#endif

/* The factor of each value as float bits (runtime/framing_ratios.h), and
   the side of its frame as a part of the uncropped view in 1/65536
   (runtime/targets: about 28/35, 28/40, 28/50, and 28/70). */
static const u32 crop_factors[GRKIT_CORE_CROP_VALUES]={CROP_FACTORS};
static const u32 crop_scales[GRKIT_CORE_CROP_VALUES]={CROP_SCALES};
#define CROP_SCALE_ONE 65536U
/* A frame within this part of the view is the view itself: the live view
   already shows the crop. */
#define CROP_SCALE_SAME (CROP_SCALE_ONE-256U)
/* The value of the live-view factor of each crop, a factor of the camera:
   35 and 50 mm their own, 40 mm that of 35 mm, and 70 mm that of 50 mm. */
static const u8 crop_live_values[GRKIT_CORE_CROP_VALUES]={0,1,1,3,3};
/* The value of the factor that the live view shows for each crop, where it
   takes another factor than the camera's (CROP_LIVE_FACTOR,
   runtime/framing.S: crop_live): 40 mm its own, and the camera keeps the
   factor of crop_live_values. The live view of the camera only reduces its
   picture, and the factor of 50 mm already takes the full height, so a
   larger factor gives a broken picture on a camera: 70 mm shows the view
   of 50 mm. Elsewhere the live view shows the factor of the camera. Each
   view of another factor than the crop's has the frame of the crop. */
#ifndef CROP_LIVE_FACTOR
#define CROP_LIVE_FACTOR 0
#endif
static const u8 crop_view_values[GRKIT_CORE_CROP_VALUES]={0,1,2,3,3};
#define crop_view_value(crop) (CROP_LIVE_FACTOR ? crop_view_values[crop] : crop_live_values[crop])
/* GRKIT_HOST_CROP_LIVE_RETRY after a try of crop_sync (crop_live_try). */
#define CROP_LIVE_TRIED 0xffffffU
/* The factor `bits` (float bits of a factor of 1 or more) in 1/65536. */
static __attribute__((unused)) u32 crop_bits_q16(u32 bits) {
    u32 exponent=(bits>>23)&0xffU, mantissa=(bits&0x7fffffU)|0x800000U;
    return exponent>=134U ? mantissa<<(exponent-134U) : mantissa>>(134U-exponent);
}
/* The crop step of the factor `bits`, not 1: the size class of the native
   crop whose window a crop takes, 1 below the factor of 50 mm, else 2.
   Positive floats compare as their bits. */
static __attribute__((unused)) u32 crop_step(u32 bits) { return bits>=CROP_NATIVE_50 ? 2U : 1U; }
/* The camera's factor of that crop step: that of 35 or 50 mm. */
static __attribute__((unused)) u32 crop_step_factor(u32 bits) { return crop_step(bits)==2U ? CROP_NATIVE_50 : CROP_NATIVE_35; }
/* A side of an L size at the crop factor `bits`: the side divided by the
   factor, truncated, then to the nearest multiple of 16, which the JPEG
   codec takes (grkit_core::aspect::geometry::crop_side). For each side and
   factor that a size takes, the factor in 1/65536 gives the result of the
   camera's single-precision division (tests); not for every side. A
   shift-subtract division: the freestanding link has no division helper.
   `bits` is a finite factor of 1 or more (crop_resolution_size). */
static __attribute__((unused)) u32 crop_side(u32 side,u32 bits) {
    u32 n=side<<16, d=crop_bits_q16(bits), q=0, bit=1;
    while(d<n && !(d&0x80000000U)) { d<<=1; bit<<=1; }
    while(bit) { if(n>=d) { n-=d; q|=bit; } d>>=1; bit>>=1; }
    return (q+8U)&~15U;
}
/* The zoom of the factor `bits`: the factor times 8192, as the live view
   takes it (runtime/framing.S: crop_window). */
static __attribute__((unused)) u32 crop_zoom(u32 bits) { return (crop_bits_q16(bits)+4U)>>3; }
static __attribute__((unused)) u32 crop_zoom_near(u32 a,u32 b) { return a+2U>=b && b+2U>=a; }
/* The factor that the live view shows in 1/65536: the factor of the crop
   that it zooms to, else the camera's (CROP_NATIVE_Q16). */
#define CROP_VIEW_Q16(h) ((h)[GRKIT_HOST_CROP_LIVE/4] ? crop_bits_q16((h)[GRKIT_HOST_CROP_LIVE/4]) : CROP_NATIVE_Q16())

/* GR IIIx shares the GR III runtime and has a 40 mm lens (runtime/targets).
   Its crops have the same factors, so only their texts differ. */
#ifndef CROP_LENS_40MM
#define CROP_LENS_40MM() 0
#endif
/* The text of a crop (CORE.CROP value) on GR IIIx, whose module texts name
   the focal lengths of a 28 mm lens, or 0 for the module text. */
static const u16 crop_texts_40mm[GRKIT_CORE_CROP_VALUES-1][6]={
    {'5','0','m','m',0},{'5','7','m','m',0},{'7','1','m','m',0},{'1','0','0','m','m',0}};
static __attribute__((unused)) const u16 *crop_lens_text(u32 value) {
    return value && value<GRKIT_CORE_CROP_VALUES && CROP_LENS_40MM() ? crop_texts_40mm[value-1] : 0;
}
#ifdef CROP_FOCALS
#ifndef CROP_FOCALS_LENS_40MM
#define CROP_FOCALS_LENS_40MM CROP_FOCALS
#endif
/* The crop of a photo that the camera does not have, 40 or 70 mm, or 0: the
   crop whose Exif 35 mm focal length the photo records (`focal`), where the
   camera reads the crop factor `bits` of no crop or of the crop step from
   the photo. The camera knows only its own factors, so it reads such a DNG
   as one without a crop. */
static __attribute__((unused)) u32 crop_of_photo(u32 focal,u32 bits) {
    static const u8 focals[2][GRKIT_CORE_CROP_VALUES]={{CROP_FOCALS},{CROP_FOCALS_LENS_40MM}};
    u32 lens=CROP_LENS_40MM() ? 1U : 0U, crop;
    for(crop=1;crop<GRKIT_CORE_CROP_VALUES;++crop)
        if(focals[lens][crop] && focals[lens][crop]==focal &&
           (bits==crop_factors[0] || bits==crop_factors[crop_live_values[crop]])) return crop;
    return 0;
}
#endif
/* The selected crop, or 0 without a module that uses it. */
static __attribute__((unused)) u32 crop_selection(volatile u32 *h) {
    u32 value=h[GRKIT_HOST_CROP/4];
    if(!CAPABILITY_USED(h,GRKIT_CORE_CROP)) return 0;
    return value<GRKIT_CORE_CROP_VALUES ? value : 0;
}
/* The crop whose frame the live view paints: the highlighted value while
   ADJ previews the page of CORE.CROP (runtime/modules.h: frame_preview_set),
   else the selection. Only the layers follow it (runtime/viewfinder.h:
   crop_painted): the camera's factor and the shot keep the selection
   (crop_capture), so that a highlight does not change the factor of the
   capture while the camera may still be making the last photo. ADJ keeps
   the highlighted value when it closes, and the next bind of the live view
   applies it (runtime/framing.c: bind_canvas). */
static __attribute__((unused)) u32 crop_shown(volatile u32 *h) {
    u32 handle=h[GRKIT_HOST_MODULE_PREVIEW/4], value;
    if(!h[GRKIT_HOST_FRAME_PREVIEW/4] || !handle || handle-1U!=GRKIT_CORE_CROP || !CAPABILITY_USED(h,GRKIT_CORE_CROP))
        return crop_selection(h);
    value=h[GRKIT_HOST_MODULE_PREVIEW/4+1];
    return value<GRKIT_CORE_CROP_VALUES ? value : 0;
}
/* A module that does not use CROP_MODE crops. */
static __attribute__((unused)) u32 crop_frame_only(volatile u32 *h) {
    return CAPABILITY_USED(h,GRKIT_CORE_CROP_MODE) && h[GRKIT_HOST_CROP_MODE/4]==1;
}
/* The crop that the camera applies, or 0: the selected crop in Crop mode,
   in each native aspect and with each Framing ratio. */
static __attribute__((unused)) u32 crop_capture(volatile u32 *h) {
    u32 crop=crop_selection(h);
    if(!crop || crop_frame_only(h)) return 0;
    return crop;
}
/* The camera's crop factor in 1/65536 (runtime/framing.S), for the frame
   of a crop in a live view that the camera crops already. */
#if defined(FW_CROP_GET) && !defined(CROP_NATIVE_Q16)
u32 crop_native_q16(void);
#define CROP_NATIVE_Q16() crop_native_q16()
#endif
#ifndef CROP_NATIVE_Q16
#define CROP_NATIVE_Q16() CROP_SCALE_ONE
#endif
/* The side of the frame of `crop` as a part of a live view that shows the
   native factor `native` (in 1/65536, 65536 without a crop of the camera). */
static __attribute__((unused)) u32 crop_scale(u32 crop,u32 native) {
    return (u32)(((unsigned long long)crop_scales[crop]*native)>>16);
}

/* The crop badge of the live view: the camera has its own for 35 and 50 mm
   (GR IIIx 50 and 71 mm) and none for the other crops, and its text differs
   from the text that the core can draw. So for each crop, the core hides the
   camera's badge (runtime/framing.S: crop_badge) and draws a label in its
   place, so that the badges have one look
   (grkit_core::artwork::OVERLAY_LABELS): LABEL_CROP + n for 40 and 70 mm (n
   is 0 and 1) and for 57 and 100 mm of GR IIIx (2 and 3), and
   LABEL_CROP_CAMERA + n for 35, 50, and 71 mm (0 to 2). Returns
   GRKIT_LABEL_COUNT without a crop. */
#define LABEL_CROP GRKIT_MODULE_LABEL_COUNT
#define LABEL_CROP_CAMERA GRKIT_LABEL_CROP_CAMERA
#define CROP_BADGE_LEFT 662U
#define CROP_BADGE_TOP 80U
/* The badge of CORE.CROP value `value`. Playback shows it for the crop of a
   photo too (runtime/viewfinder.h: playback_paint). */
static __attribute__((unused)) u32 crop_value_label(u32 value) {
    static const u8 badges[2][GRKIT_CORE_CROP_VALUES]={
        {GRKIT_LABEL_COUNT,LABEL_CROP_CAMERA,LABEL_CROP,LABEL_CROP_CAMERA+1U,LABEL_CROP+1U},
        {GRKIT_LABEL_COUNT,LABEL_CROP_CAMERA+1U,LABEL_CROP+2U,LABEL_CROP_CAMERA+2U,LABEL_CROP+3U},
    };
    return value<GRKIT_CORE_CROP_VALUES ? badges[CROP_LENS_40MM() ? 1 : 0][value] : GRKIT_LABEL_COUNT;
}
static __attribute__((unused)) u32 crop_badge_label(volatile u32 *h) { return crop_value_label(crop_capture(h)); }
_Static_assert(LABEL_CROP+4U<=GRKIT_LABEL_COUNT && LABEL_CROP_CAMERA+3U<=GRKIT_LABEL_COUNT,
               "The labels hold each crop badge");

/* The camera's own crop factor: its float bits, and its setter as the
   native menu calls it (runtime/framing.S, runtime/menu.S). */
#if defined(FW_CROP_SET) && !defined(CROP_NATIVE_BITS)
u32 crop_native_bits(void);
void crop_native_set(u32 bits);
#define CROP_NATIVE_BITS() crop_native_bits()
#define CROP_NATIVE_SET(bits) crop_native_set(bits)
#endif
#ifdef CROP_NATIVE_BITS
/* Whether the camera knows the crop factor `bits`: Off, 35 mm, or 50 mm. */
static __attribute__((unused)) u32 crop_native_known(u32 bits) {
    return bits==crop_factors[0] || bits==CROP_NATIVE_35 || bits==CROP_NATIVE_50;
}
/* Set the camera's crop factor to the live-view factor of the crop that the
   camera applies, the factor that the live view shows in its place
   (GRKIT_HOST_CROP_LIVE), and arm the factor of the next shot. When no crop
   applies, a factor that the core set and that the camera still has returns
   to Off; a crop from the camera menu stays. A factor that the camera does
   not know returns to Off first: an earlier GRKit saved it at power-off, or
   the camera's crop button stepped from one to 0. The live view reads its
   factor again only when the camera's factor changes, so a new live factor
   with the same camera factor sets another factor of the camera first. */
static __attribute__((unused)) void crop_sync(volatile u32 *h) {
    u32 crop=crop_capture(h), want=crop ? crop_factors[crop_live_values[crop]] : 0;
    u32 owned=h[GRKIT_HOST_CROP_NATIVE/4], now=CROP_NATIVE_BITS(), target;
    u32 view=crop_view_value(crop), live=crop && view!=crop_live_values[crop] ? crop_factors[view] : 0;
    u32 before=h[GRKIT_HOST_CROP_LIVE/4];
    h[GRKIT_HOST_CROP_SHOT/4]=crop && crop_live_values[crop]!=crop ? crop_factors[crop] : 0;
    h[GRKIT_HOST_CROP_LIVE/4]=live;
    h[GRKIT_HOST_CROP_LIVE_ZOOM/4]=live ? crop_zoom(live) : 0;
    /* A new live factor: the change of the camera's factor below makes the
       live view read it, so a try waits a second first. */
    if(live!=before) h[GRKIT_HOST_CROP_LIVE_RETRY/4]=live ? CROP_LIVE_TRIED : 0;
    if(!crop_native_known(now)) {
        now=crop_factors[0];
        CROP_NATIVE_SET(now);
    }
    target=want ? want : owned && now==owned ? crop_factors[0] : now;
    h[GRKIT_HOST_CROP_NATIVE/4]=want;
    h[GRKIT_HOST_CROP_CAMERA_ZOOM/4]=crop_zoom(target);
    if(now!=target) CROP_NATIVE_SET(target);
    else if(live!=before) {
        CROP_NATIVE_SET(target==crop_factors[0] ? CROP_NATIVE_35 : crop_factors[0]);
        CROP_NATIVE_SET(target);
    }
}
#endif
#if CROP_LIVE_FACTOR && defined(CROP_NATIVE_BITS)
/* Make the live view read its factor again where it did not read the one
   that crop_sync set: at power-on, the camera can still be loading its
   settings when the core first changes its factor, and the live view then
   keeps the old zoom. A movie live view takes the camera's factor.
   crop_live_due says whether to try, so the caller reads the clock only
   then; crop_live_try tries at most CROP_LIVE_TRIES times for each new
   factor, one in a second (`second`, the second of the day). A try that
   crop_sync made starts the wait of a second (CROP_LIVE_TRIED). */
#define CROP_LIVE_TRIES 5U
static __attribute__((unused)) u32 crop_live_due(volatile u32 *h,u32 movie) {
    u32 live=h[GRKIT_HOST_CROP_LIVE/4], seen=h[GRKIT_HOST_CROP_LIVE_SEEN/4], ok;
    /* A movie window does not pass the window function: nothing to check. */
    if(movie) {
        h[GRKIT_HOST_CROP_LIVE/4]=0;
        h[GRKIT_HOST_CROP_LIVE_ZOOM/4]=0;
        h[GRKIT_HOST_CROP_LIVE_RETRY/4]=0;
        return 0;
    }
    /* The window shows the live factor. Without one, a window of the
       camera, or none since INSTALL, needs nothing; a window of an earlier
       live factor needs the camera's again. At power-on the camera can make
       its windows before INSTALL and not again, so a live factor without a
       window since INSTALL (`seen` 0) also tries. */
    if(live) ok=crop_zoom_near(seen,crop_zoom(live));
    else ok=!seen || crop_zoom_near(seen,crop_zoom(crop_factors[0])) || crop_zoom_near(seen,crop_zoom(CROP_NATIVE_35)) ||
            crop_zoom_near(seen,crop_zoom(CROP_NATIVE_50));
    if(ok) {
        h[GRKIT_HOST_CROP_LIVE_RETRY/4]=0;
        return 0;
    }
    return h[GRKIT_HOST_CROP_LIVE_RETRY/4]>>24<CROP_LIVE_TRIES;
}
static __attribute__((unused)) void crop_live_try(volatile u32 *h,u32 second) {
    u32 retry=h[GRKIT_HOST_CROP_LIVE_RETRY/4], at=retry&CROP_LIVE_TRIED, now=CROP_NATIVE_BITS();
    if(at==CROP_LIVE_TRIED) {
        h[GRKIT_HOST_CROP_LIVE_RETRY/4]=(retry&~CROP_LIVE_TRIED)|(second+1U);
        return;
    }
    if(at==second+1U) return;
    h[GRKIT_HOST_CROP_LIVE_RETRY/4]=((retry>>24)+1U)<<24|(second+1U);
    CROP_NATIVE_SET(now==crop_factors[0] ? CROP_NATIVE_35 : crop_factors[0]);
    CROP_NATIVE_SET(now);
}
#endif
/* The frame of a crop in `outer` {left, top, right, bottom}: its center,
   with each side at `scale`/65536 of the side of `outer`. Returns 0, and
   leaves `box` alone, when the frame would be about as large as `outer`. */
static __attribute__((unused)) u32 crop_box(const u16 *outer,u32 scale,u16 *box) {
    u32 width=outer[2]-outer[0], height=outer[3]-outer[1], w, v;
    if(scale>=CROP_SCALE_SAME) return 0;
    w=(width*scale+0x8000U)>>16;
    v=(height*scale+0x8000U)>>16;
    box[0]=(u16)(outer[0]+((width-w)>>1));
    box[1]=(u16)(outer[1]+((height-v)>>1));
    box[2]=(u16)(box[0]+w);
    box[3]=(u16)(box[1]+v);
    return 1;
}

#endif
