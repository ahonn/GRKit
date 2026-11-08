#ifndef GRKIT_CROP_H
#define GRKIT_CROP_H
/* The capability CORE.CROP (grkit_core::capabilities): a focal-length crop
   of the lens. Its value is 0 for Off, then 35, 40, 50, 70, and 90 mm of a
   28 mm lens; GR IIIx has the same factors of its 40 mm lens. The live view
   shows the frame of the crop inside the layer area (runtime/viewfinder.h),
   and other layers, such as the guides, draw inside that frame.

   CORE.CROP_MODE selects cropping where the camera can (0, the default) or
   the frame only (1). The resident loader saves both settings.

   The includer defines u16, u32, and the header fields (runtime/abi.h). */
#ifndef CAPABILITY_USED
/* Whether a loaded module uses the capability `handle`. */
#define CAPABILITY_USED(h,handle) ((h)[GRKIT_HOST_CAPABILITIES/4]>>((handle)-GRKIT_CORE_SETTING_BASE)&1U)
#endif

/* The side of the frame of each value as a part of the uncropped view, in
   1/65536: 28/35, 28/40, 28/50, 28/70, and 28/90. */
static const u32 crop_scales[GRKIT_CORE_CROP_VALUES]={65536,52429,45875,36700,26214,20389};
#define CROP_SCALE_ONE 65536U

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
/* The frame of a crop in `outer` {left, top, right, bottom}: its center,
   with each side at `scale`/65536 of the side of `outer`. Returns 0, and
   leaves `box` alone, when the frame would not be smaller than `outer`. */
static __attribute__((unused)) u32 crop_box(const u16 *outer,u32 scale,u16 *box) {
    u32 width=outer[2]-outer[0], height=outer[3]-outer[1], w, v;
    if(scale>=CROP_SCALE_ONE) return 0;
    w=(width*scale+0x8000U)>>16;
    v=(height*scale+0x8000U)>>16;
    box[0]=(u16)(outer[0]+((width-w)>>1));
    box[1]=(u16)(outer[1]+((height-v)>>1));
    box[2]=(u16)(box[0]+w);
    box[3]=(u16)(box[1]+v);
    return 1;
}

#endif
