#ifndef GRKIT_RECIPE_UNDO_H
#define GRKIT_RECIPE_UNDO_H
/* The native settings of the undo steps of the recipes (runtime/recipes.h).
   An undo step holds the settings before an apply as a recipe. That alone
   does not give the native settings back as they were: a recipe holds the
   type of the active set but not the set, and it holds the parameters of
   one type and the shifts of one mode, while an apply writes those of the
   type and the mode of its recipe.

   So an apply also keeps, in the services block (SERVICES_UNDO), the words
   that it wrote and the value before it of each byte of the settings record
   that it wrote (runtime/native_look.h: native_keep). Undo builds the
   settings record from before the apply: a copy of the record now, in the
   copy of the ADJ preview (runtime/recipe_preview.h), with the kept bytes.
   Then it writes back each field that the apply wrote and that differs
   (native_sync, NATIVE_RESTORE): the active set, a Custom set included,
   the type of the main set and the parameters of the type of the recipe,
   the white balance mode and the shifts of the mode of the recipe, the
   color temperature, the corrections, and the exposure settings.

   Without the services block or the settings record, a step keeps nothing,
   and undo writes the native settings of its words (runtime/recipes.h).

   The includer defines the services (runtime/services.h), the recipes, the
   native settings, and the preview. */

_Static_assert(UNDO_KEPT_MAX>=NATIVE_KEEP_MAX,"an undo step keeps each byte that an apply writes");
_Static_assert(PREVIEW_IMAGE_BYTES<=0x10000U,"a kept offset fits in the low half");

/* The native settings of undo step `slot`, or 0 without the services
   block. */
static volatile u32 *undo_native(volatile u32 *h,u32 slot) {
    if(!h[GRKIT_HOST_SERVICES/4] || slot<GRKIT_RECIPE_SLOTS || slot>=RECIPE_ALL) return 0;
    return services_word(h,SERVICES_UNDO+(slot-GRKIT_RECIPE_SLOTS)*UNDO_STEP_SIZE);
}
/* Write the native settings of the words w, which undo step `slot` keeps.
   During the ADJ preview of the recipe, the settings before it are the
   copy of the preview. */
static void recipe_native_apply(volatile u32 *h,const u32 *w,u32 slot) {
    volatile u32 *step=undo_native(h,slot);
    u32 i, before=h[GRKIT_HOST_RECIPE_PREVIEW/4];
    if(!step) { native_write(w,0,0); return; }
    for(i=0;i<GRKIT_RECIPE_WORDS;++i) step[UNDO_WORDS/4U+i]=w[i];
    native_write(w,step+UNDO_KEPT/4U,before ? before : native_record());
}
/* Write back the native settings that undo step `slot` kept: 1, or 0 when
   it kept none. A preview ends first: undo uses its copy. */
static u32 recipe_native_undo(volatile u32 *h,u32 slot) {
    volatile u32 *step=undo_native(h,slot), *kept;
    u32 live, image, i, at;
    if(!step || !step[UNDO_KEPT/4U]) return 0;
    live=native_record();
    if(!live) return 0;
    recipe_preview_show(h,RECIPE_NONE);
    image=preview_image(h);
    for(i=0;i<PREVIEW_IMAGE_BYTES;i+=4U) WORD(image,i)=WORD(live,i);
    kept=step+UNDO_KEPT/4U;
    for(i=1;i<kept[0];++i) {
        at=kept[i]&0xffffU;
        if(at<PREVIEW_IMAGE_BYTES) BYTE(image,at)=(u8)(kept[i]>>16);
    }
    native_sync(live,image,step+UNDO_WORDS/4U,0,NATIVE_RESTORE);
    return 1;
}
static void recipe_native_move(volatile u32 *h,u32 to,u32 from) {
    volatile u32 *a=undo_native(h,to), *b=undo_native(h,from);
    u32 i;
    if(a && b) for(i=0;i<UNDO_STEP_SIZE/4U;++i) a[i]=b[i];
}
static void recipe_native_clear(volatile u32 *h,u32 slot) {
    volatile u32 *a=undo_native(h,slot);
    u32 i;
    if(a) for(i=0;i<UNDO_STEP_SIZE/4U;++i) a[i]=0;
}
#endif
