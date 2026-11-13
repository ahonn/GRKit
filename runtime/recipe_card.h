#ifndef GRKIT_RECIPE_CARD_H
#define GRKIT_RECIPE_CARD_H
/* The tile of a recipe in ADJ: the stacked cards of the Recipes module and,
   below them, the abbreviation of the name of the recipe, so that the rows
   of recipes on the same look differ (docs/icon-guidelines.md: a symbol
   with a caption). The name is the one that the person gave, else the
   automatic name in English.

   The abbreviation follows runtime/recipe_abbreviation.h.

   The tile of recipe e has the icon ID GRKIT_ICON_RECIPE(e). The installer
   keeps the tile without a caption (grkit_core::artwork::
   recipe_card_pixels) in the host icon RECIPE_CARD_BASE and
   RECIPE_CARD_SLOTS more host icons after it, which the core writes: when
   the camera draws the icon of a recipe, the core finds the slot with its
   abbreviation, or writes the abbreviation into the next slot in turn
   (runtime/services.h: SERVICES_CARDS). A list shows fewer rows than the
   slots, so the slots of the rows on screen stay as they are while the
   camera draws them. The caption glyphs come from the label table
   (GRKIT_CAPTION_FIRST): their pixels are the coverage of the text, which
   blends GRKIT_CAPTION_INK over the fill of the tile.

   The includer defines u8, u16, u32, HALF, and WORD, and includes
   runtime/services.h, runtime/recipes.h, and runtime/recipe_page.h. */
#include "recipe_abbreviation.h"
#define GRKIT_ICON_RECIPE(e) (0x7e00U+(e))
/* The host icon of the tile without a caption: after the icons that the
   installer writes from its artwork (runtime/installer.c:
   write_host_icons). */
#define RECIPE_CARD_BASE 8U
_Static_assert(RECIPE_CARD_BASE+1U+RECIPE_CARD_SLOTS==GRKIT_HOST_ICON_COUNT,
               "the host icons end with the base tile of recipes and their slots");
_Static_assert(GRKIT_RECIPE_SLOTS<=0x100U,"the icon IDs of recipes stay below the core labels");

/* The abbreviation of recipe e: of the name that the person gave, else of
   the automatic name in English. */
static __attribute__((noinline)) u32 recipe_card_key(volatile u32 *h,u32 e) {
    u16 name[RECIPE_UNITS];
    u32 n=recipe_name_length(h,e), i;
    if(n) {
        for(i=0;i<n && i+1U<RECIPE_UNITS;++i) name[i]=(u16)recipe_name_char(h,e,i);
        name[i]=0;
    }
    else recipe_append_automatic(h,(volatile u16 *)name,0,e,1);
    return recipe_abbreviation(name,RECIPE_UNITS);
}
/* A field of the label table, little-endian. */
static u32 card_field(const volatile u8 *table,u32 at,u32 bytes) {
    u32 value=0, i;
    for(i=0;i<bytes;++i) value|=(u32)table[at+i]<<(8U*i);
    return value;
}
/* The caption glyph of character c, or 0 for a character without one. */
static u32 card_glyph(u32 c) {
    static const char characters[]=GRKIT_CAPTION_CHARACTERS;
    u32 i;
    for(i=0;characters[i];++i) if((u32)(u8)characters[i]==c) return GRKIT_CAPTION_FIRST+i;
    return 0;
}
/* Write the tile of abbreviation `key` at the pixels `to`: the base tile,
   then the caption centered on the tile, without the characters that pass
   the inside of the frame. */
static void recipe_card_write(volatile u32 *h,volatile u32 *to,const volatile u32 *from,u32 key) {
    const volatile u8 *table=(const volatile u8 *)GRKIT_LABELS(h), *at;
    u32 glyph[RECIPE_CARD_UNITS], n=0, width=0, i, k, x, y, w, rows, spans, first, length, gray;
    for(i=0;i<40U*40U;++i) to[i]=from[i];
    for(k=0;k<RECIPE_CARD_UNITS;++k) {
        u32 g=card_glyph(key>>(8U*k)&0xffU);
        if(!g) continue;
        w=card_field(table,g*8U,2);
        if(width+w>36U) break;
        glyph[n++]=g;
        width+=w;
    }
    x=20U-width/2U;
    for(k=0;k<n;++k) {
        w=card_field(table,glyph[k]*8U,2);
        rows=card_field(table,glyph[k]*8U+2U,2);
        at=table+card_field(table,glyph[k]*8U+4U,4);
        for(y=0;y<rows;++y) {
            spans=*at++;
            while(spans--) {
                first=card_field(at,0,2); length=card_field(at,2,2); at+=4;
                for(i=0;i<length;++i) {
                    /* (ink - fill) * coverage / 255, rounded, without a division. */
                    gray=GRKIT_TILE_FILL+(((GRKIT_CAPTION_INK-GRKIT_TILE_FILL)*at[i]*257U+32768U)>>16);
                    to[(GRKIT_CAPTION_TOP+y)*40U+x+first+i]=0xff000000U|gray*0x010101U;
                }
                at+=length;
            }
        }
        x+=w;
    }
}
/* The descriptor of the tile of recipe e, or 0. */
static u32 recipe_card_address(volatile u32 *h,u32 e) {
    volatile u32 *cards;
    u32 grkit=h[GRKIT_HOST_GRKIT_ICON/4], key, k, slot, next;
    if(!grkit || !h[GRKIT_HOST_SERVICES/4] || e>=GRKIT_RECIPE_SLOTS || !(recipe_words(h,e)[0]&RECIPE_USED)) return 0;
    cards=services_word(h,SERVICES_CARDS);
    key=recipe_card_key(h,e);
    /* The GRKit icon is host icon 1. */
    for(k=0;k<RECIPE_CARD_SLOTS;++k)
        if(cards[CARD_KEYS/4+k]==key) return grkit+(RECIPE_CARD_BASE+k)*GRKIT_ICON_STRIDE;
    next=cards[CARD_NEXT/4];
    slot=next<RECIPE_CARD_SLOTS ? next : 0;
    cards[CARD_NEXT/4]=slot+1U<RECIPE_CARD_SLOTS ? slot+1U : 0;
    cards[CARD_KEYS/4+slot]=key;
    recipe_card_write(h,(volatile u32 *)(grkit+(RECIPE_CARD_BASE+slot)*GRKIT_ICON_STRIDE+GRKIT_ICON_STRIDE-GRKIT_GRF_ICON_SIZE),
                      (const volatile u32 *)(grkit+(RECIPE_CARD_BASE-1U)*GRKIT_ICON_STRIDE+GRKIT_ICON_STRIDE-GRKIT_GRF_ICON_SIZE),key);
    return grkit+(RECIPE_CARD_BASE+slot)*GRKIT_ICON_STRIDE;
}
#endif
