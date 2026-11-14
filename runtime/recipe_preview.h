#ifndef GRKIT_RECIPE_PREVIEW_H
#define GRKIT_RECIPE_PREVIEW_H
/* The ADJ preview of a recipe (runtime/recipes.h): while the cursor of the
   Recipes page is on a recipe that can apply, the live view shows it, as
   the native ADJ shows a highlighted Image Control. A move of the cursor
   starts the preview, as on the page of CORE.TONE: the page opens without
   one, while the panel of ADJ already shows the row under the cursor.

   - The look shows only in the live view (runtime/processing.h): an effect
     with the Fn adjustments of the recipe (GRKIT_HOST_PREVIEW and
     GRKIT_HOST_PREVIEW_FN), Film Look Off as native processing, and a look
     that the recipe keeps as it is. Pictures and the records keep the
     committed effect.
   - The native settings of the recipe go to the camera, as the native ADJ
     sets a highlighted value (runtime/native_look.h: native_sync). The
     start keeps a copy of the settings record (GRKIT_HOST_RECIPE_PREVIEW,
     SERVICES_PREVIEW), and each change writes only the fields that change.
     The end writes back each field that the preview changed, unless the
     camera changed it meanwhile.
   - Grain, Glow, and the border have no live-view layer: they show at OK.
   - A capture of the current settings, as for an undo step, the check mark
     of a row, or a save, reads the native settings from the copy, so the
     preview changes no current setting (runtime/recipes.h: recipe_capture).

   The preview ends when the cursor leaves the recipes, and when the page or
   ADJ closes without OK: MENU, a half-press, playback, a change of the
   mode or of the controller (runtime/menu.c). OK on the recipe applies it
   from the settings before the preview, so its undo step holds them, and
   then the preview ends without a write (recipe_preview_drop).

   The includer defines the services (runtime/services.h), the recipes, and
   the native settings. */

static volatile u32 *preview_words(volatile u32 *h) { return services_word(h,SERVICES_PREVIEW+PREVIEW_WORDS); }
/* The copy of the settings record, a camera address. */
static u32 preview_image(volatile u32 *h) { return h[GRKIT_HOST_SERVICES/4]+SERVICES_PREVIEW+PREVIEW_IMAGE; }
/* Whether a preview shows: then its recipe words hold the used bit. */
static u32 recipe_preview_on(volatile u32 *h) { return h[GRKIT_HOST_SERVICES/4] && preview_words(h)[0]; }

/* Show the look of the words w in the live view, or the committed look for
   w = 0. A change of the Fn adjustments of the live view counts as a
   change of adjustments, so that the live view builds its stages again. */
static void recipe_preview_look(volatile u32 *h,const u32 *w) {
    u32 record=0, preview=0, bytes[GRKIT_HOST_PREVIEW_FN_BYTES_WORDS]={0}, digits, rest, i, changed;
    _Static_assert(GRKIT_HOST_PREVIEW_FN_BYTES_WORDS*4U>=9U,"a byte for each Fn adjustment");
    if(w && RECIPE_CAPABILITY(h,TONE)) {
        if(RECIPE_LOOK(w[0])==RECIPE_LOOK_EFFECT) {
            i=recipe_effect(h,RECIPE_CODE(w[0]));
            if(i<GRKIT_TEXT_RECORDS) preview=record=recipe_record(h,i);
        }
        else if(RECIPE_LOOK(w[0])==RECIPE_LOOK_OFF) preview=1;
    }
    if(record) {
        digits=w[RECIPE_FN_WORD];
        for(i=0;i<9U;++i) {
            rest=recipe_div9(digits);
            bytes[i>>2]|=(digits-9U*rest)<<(8U*(i&3U));
            digits=rest;
        }
    }
    changed=h[GRKIT_HOST_PREVIEW_FN/4]!=record;
    for(i=0;i<GRKIT_HOST_PREVIEW_FN_BYTES_WORDS;++i) {
        changed|=h[GRKIT_HOST_PREVIEW_FN_BYTES/4+i]!=bytes[i];
        h[GRKIT_HOST_PREVIEW_FN_BYTES/4+i]=bytes[i];
    }
    h[GRKIT_HOST_PREVIEW_FN/4]=record;
    h[GRKIT_HOST_PREVIEW/4]=preview;
    if(changed) ++h[GRKIT_HOST_PARAMETER_CHANGES/4];
}

/* Whether the preview shows recipe e. */
static u32 recipe_preview_shows(volatile u32 *h,u32 e) {
    u32 i;
    if(!recipe_preview_on(h) || e>=recipe_entries(h)) return 0;
    for(i=0;i<GRKIT_RECIPE_WORDS;++i) if(preview_words(h)[i]!=recipe_words(h,e)[i]) return 0;
    return 1;
}
/* Show recipe e, or end the preview for RECIPE_NONE or a recipe that
   cannot apply. Showing the recipe that shows changes nothing besides the
   look, which another page may have cleared. */
static void recipe_preview_show(volatile u32 *h,u32 e) {
    u32 to[GRKIT_RECIPE_WORDS], from[GRKIT_RECIPE_WORDS], on, want, i, live, image;
    if(!h[GRKIT_HOST_SERVICES/4]) return;
    on=recipe_preview_on(h);
    want=e<recipe_entries(h) && recipe_entry_reason(h,e)==RECIPE_READY;
    if(!on && !want) return;
    if(want && recipe_preview_shows(h,e)) {
        for(i=0;i<GRKIT_RECIPE_WORDS;++i) to[i]=recipe_words(h,e)[i];
        recipe_preview_look(h,to);
        return;
    }
    for(i=0;i<GRKIT_RECIPE_WORDS;++i) {
        from[i]=preview_words(h)[i];
        to[i]=want ? recipe_words(h,e)[i] : 0U;
    }
    live=native_record();
    image=preview_image(h);
    /* The start copies the settings record. Without one, the preview shows
       only the look. */
    if(!on) {
        h[GRKIT_HOST_RECIPE_PREVIEW/4]=0;
        if(live) {
            for(i=0;i<PREVIEW_IMAGE_BYTES;i+=4U) WORD(image,i)=WORD(live,i);
            h[GRKIT_HOST_RECIPE_PREVIEW/4]=image;
        }
    }
    if(live && h[GRKIT_HOST_RECIPE_PREVIEW/4]==image) native_sync(live,image,on ? from : 0,want ? to : 0,0);
    recipe_preview_look(h,want ? to : 0);
    for(i=0;i<GRKIT_RECIPE_WORDS;++i) preview_words(h)[i]=to[i];
    if(!want) h[GRKIT_HOST_RECIPE_PREVIEW/4]=0;
}
/* End the preview and keep the native settings that it wrote: OK applied
   its recipe. */
static void recipe_preview_drop(volatile u32 *h) {
    u32 i;
    if(!recipe_preview_on(h)) return;
    for(i=0;i<GRKIT_RECIPE_WORDS;++i) preview_words(h)[i]=0;
    h[GRKIT_HOST_RECIPE_PREVIEW/4]=0;
    recipe_preview_look(h,0);
}
#endif
