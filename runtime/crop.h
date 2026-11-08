#ifndef GRKIT_CROP_H
#define GRKIT_CROP_H
/* The capability CORE.CROP (grkit_core::capabilities): a focal-length crop
   of the lens. Its value is 0 for Off, then 35, 40, 50, 70, and 90 mm of a
   28 mm lens; GR IIIx has the same factors of its 40 mm lens. The live view
   shows the frame of the crop inside the layer area (runtime/viewfinder.h),
   and other layers, such as the guides, draw inside that frame.

   CORE.CROP_MODE selects cropping where the camera can (0, the default) or
   the frame only (1). The resident loader saves both settings.

   A crop of 3:2 photos without a Framing ratio is a crop of the camera:
   the core sets the camera's own crop factor (FW_CROP_SET) to the factor of
   35, 40, or 50 mm, so that the live view, the focus, and the exposure
   follow it, and the window and the size of the photo follow the factor
   (runtime/framing.S: crop_resolution). 70 and 90 mm keep the live view at
   50 mm with the frame of the crop, and their shots take their own factor
   (crop_shot). With a Framing ratio or another native aspect, the live
   view shows the frame only.

   The includer defines u16, u32, and the header fields (runtime/abi.h). */
#ifndef CAPABILITY_USED
/* Whether a loaded module uses the capability `handle`. */
#define CAPABILITY_USED(h,handle) ((h)[GRKIT_HOST_CAPABILITIES/4]>>((handle)-GRKIT_CORE_SETTING_BASE)&1U)
#endif

/* The factor of each value as float bits, and the side of its frame as a
   part of the uncropped view in 1/65536 (runtime/targets: about 28/35,
   28/40, 28/50, 28/70, and 28/90). */
#ifndef CROP_FACTORS
#define CROP_FACTORS 0x3f800000,0x3fa00000,0x3fb6db6e,0x3fe49249,0x40200000,0x404db6db
#define CROP_SCALES 65536,52429,45875,36700,26214,20389
#endif
static const u32 crop_factors[GRKIT_CORE_CROP_VALUES]={CROP_FACTORS};
static const u32 crop_scales[GRKIT_CORE_CROP_VALUES]={CROP_SCALES};
#define CROP_SCALE_ONE 65536U
/* A frame within this part of the view is the view itself: the live view
   already shows the crop. */
#define CROP_SCALE_SAME (CROP_SCALE_ONE-256U)
/* The value of the live-view factor of each crop: 35 and 40 mm their own,
   and 70 and 90 mm that of 50 mm. */
static const u8 crop_live_values[GRKIT_CORE_CROP_VALUES]={0,1,2,3,3,3};
#define CROP_LONGEST_LIVE 3U

/* GR IIIx shares the GR III runtime and has a 40 mm lens (runtime/targets).
   Its crops have the same factors, so only their texts differ. */
#ifndef CROP_LENS_40MM
#define CROP_LENS_40MM() 0
#endif
/* The selected crop, or 0 without a module that uses it. */
static __attribute__((unused)) u32 crop_selection(volatile u32 *h) {
    u32 value=h[GRKIT_HOST_CROP/4];
    if(!CAPABILITY_USED(h,GRKIT_CORE_CROP)) return 0;
    return value<GRKIT_CORE_CROP_VALUES ? value : 0;
}
/* A module that does not use CROP_MODE crops where the camera can. */
static __attribute__((unused)) u32 crop_frame_only(volatile u32 *h) {
    return CAPABILITY_USED(h,GRKIT_CORE_CROP_MODE) && h[GRKIT_HOST_CROP_MODE/4]==1;
}
/* The crop that the camera applies to photos, or 0: a selected crop in
   Crop mode, in native 3:2 (`native_aspect` 0) without a Framing ratio. */
static __attribute__((unused)) u32 crop_capture(volatile u32 *h,u32 native_aspect) {
    u32 crop=crop_selection(h);
    if(!crop || crop_frame_only(h) || native_aspect || framing_selection(h)) return 0;
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

/* The camera's own crop factor: its float bits, and its setter as the
   native menu calls it (runtime/framing.S, runtime/menu.S). */
#if defined(FW_CROP_SET) && !defined(CROP_NATIVE_BITS)
u32 crop_native_bits(void);
void crop_native_set(u32 bits);
#define CROP_NATIVE_BITS() crop_native_bits()
#define CROP_NATIVE_SET(bits) crop_native_set(bits)
#endif
#ifdef CROP_NATIVE_BITS
/* Set the camera's crop factor to that of the crop that the camera applies,
   and arm the factor of the next shot. When no crop applies, a factor that
   the core set and that the camera still has returns to Off; a crop from
   the camera menu stays. The live view follows the factor. */
static __attribute__((unused)) void crop_sync(volatile u32 *h,u32 native_aspect) {
    u32 crop=crop_capture(h,native_aspect), want=crop ? crop_factors[crop_live_values[crop]] : 0;
    u32 owned=h[GRKIT_HOST_CROP_NATIVE/4], now;
    h[GRKIT_HOST_CROP_SHOT/4]=crop>CROP_LONGEST_LIVE ? crop_factors[crop] : 0;
    if(!want && !owned) return;
    now=CROP_NATIVE_BITS();
    if(!want) {
        if(now==owned) CROP_NATIVE_SET(crop_factors[0]);
        h[GRKIT_HOST_CROP_NATIVE/4]=0;
        return;
    }
    if(now!=want) CROP_NATIVE_SET(want);
    h[GRKIT_HOST_CROP_NATIVE/4]=want;
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
