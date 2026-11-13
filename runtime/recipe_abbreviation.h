#ifndef GRKIT_RECIPE_ABBREVIATION_H
#define GRKIT_RECIPE_ABBREVIATION_H
/* The abbreviation of the name of a recipe, which its tile in ADJ shows
   (runtime/recipe_card.h). A word (a run of ASCII letters and digits) that
   starts with a letter gives that letter in upper case, and a word that
   starts with a digit gives its leading digits, at most RECIPE_CARD_UNITS
   characters in all. A result of one letter takes the first three letters
   and digits of the name instead: Street Neg SN, CineStill 800T C800,
   Negative Film 2 NF2, Kodachrome KOD. The includer defines u16 and u32. */
#define RECIPE_CARD_UNITS 4U
static u32 card_alnum(u32 c) { return (c>='0' && c<='9') || (c>='A' && c<='Z') || (c>='a' && c<='z'); }
static u32 card_upper(u32 c) { return c>='a' && c<='z' ? c-'a'+'A' : c; }
/* The abbreviation of `name` (NUL-terminated, at most `units`): its
   characters, the first in the low byte, or a space for a name without a
   letter or a digit. */
static u32 recipe_abbreviation(const u16 *name,u32 units) {
    u32 out=0, n=0, i=0, c;
    while(i<units && name[i] && n<RECIPE_CARD_UNITS) {
        c=name[i];
        if(!card_alnum(c)) { ++i; continue; }
        if(c>='0' && c<='9') {
            while(i<units && name[i]>='0' && name[i]<='9' && n<RECIPE_CARD_UNITS) out|=(u32)name[i++]<<(8U*n++);
        }
        else out|=card_upper(c)<<(8U*n++);
        while(i<units && card_alnum(name[i])) ++i;
    }
    if(n==1U && (out<'0' || out>'9')) {
        out=0; n=0;
        for(i=0;i<units && name[i] && n<3U;++i)
            if(card_alnum(name[i])) out|=card_upper(name[i])<<(8U*n++);
    }
    return n ? out : ' ';
}
#endif
