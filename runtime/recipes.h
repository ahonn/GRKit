#ifndef GRKIT_RECIPES_H
#define GRKIT_RECIPES_H
/* The recipes of CORE.RECIPE: named snapshots of the picture settings that
   OK applies in one step. The header holds GRKIT_RECIPE_SLOTS recipes
   (GRKIT_HOST_RECIPES) and GRKIT_RECIPE_UNDO undo steps, the oldest first
   (GRKIT_HOST_RECIPE_UNDO), GRKIT_RECIPE_WORDS words each. A recipe whose
   word 0 is not a recipe word ends the list.

   A recipe is on or off (RECIPE_OFF). The recipes that are on come first,
   at most GRKIT_RECIPE_ON_MAX: they are the entries of the lists of ADJ and
   MENU. The camera keeps the recipes that are off for the site, which turns
   recipes on and off, and does not list them. INSTALL puts the recipes in
   this order (runtime/install_recipes.h). The resident loader saves the
   recipes in the recipe file after GRKIT_RECIPES_CHANGED, and INSTALL
   restores them (grkit_core::runtime::recipe_key). The undo steps are not
   saved: a start begins without them. The site reads and writes the recipe
   file, and INSTALL adds the recipes of a package
   (runtime/install_recipes.h).

   A recipe is a whole look: it holds each setting of the picture, so it
   gives the same picture after any other recipe. Those are the look of
   CORE.TONE (Off, or an effect and its Fn adjustments), the grain of
   CORE.GRAIN and CORE.GRAIN_SIZE, the module settings of
   GRKIT_RECIPE_MODULE_SETTINGS (Glow), and the native settings
   (runtime/native_look.h): the native type of the active set, its
   parameters, and its native grain while no effect is active, the white
   balance, and the correction and noise reduction settings. A recipe with
   a native type has the look Off also without CORE.TONE. A save holds
   Grain and each module setting at Off when its module does not run, as
   the picture has none; a color-only setting has no Off and stays out.
   Apply sets each held setting whose capability runs, so a recipe never
   turns on a capability.

   The exposure settings are not part of the look: they belong to one
   scene. A save on the camera holds none, an undo step holds them, and a
   library recipe can. Apply keeps each one that a recipe does not hold.

   Without a running Grain module, CORE.GRAIN has no effect on pictures
   (grain_level in runtime/picture.S), so apply skips the Grain of a recipe
   and says so.

   A recipe finds its effect by the hash of the effect ID
   (grkit_core::recipes::effect_code), not by the ImageTone code of the
   record: INSTALL moves the code of an effect whose hash another effect
   already has.

   A recipe can have a name that the person typed on the camera or on the
   site (GRKIT_HOST_RECIPE_NAMES): GRKIT_RECIPE_NAME_WORDS words of four 7-bit
   ASCII characters, the first in the low bits, at most
   GRKIT_RECIPE_NAME_CHARS. A recipe without one has the automatic name
   (runtime/recipe_page.h). The names move with the recipes; the undo steps
   have none. A hidden recipe (RECIPE_HIDDEN) stays out of the rows of ADJ.

   The includer defines u8, u16, u32, the header fields, HALF and BYTE,
   record_index (runtime/tone.h), settings_table (runtime/settings.h),
   setting_store (runtime/setting_store.h), and the native settings
   (runtime/native_look.h). */
#include "recipe_id.h"

/* The bit layout of the words is GRKIT_RECIPE_* of runtime/abi.h
   (grkit_core::recipes::layout); these are the short names of the core.

   Word 0. RECIPE_USED is set in each recipe, so word 0 of a recipe is never
   0, and each word stays below 2^GRKIT_RECIPE_VALUE_BITS (RECIPE_VALUE_MAX).
   RECIPE_HIDDEN keeps the recipe out of ADJ. */
#define RECIPE_USED GRKIT_RECIPE_USED
#define RECIPE_HIDDEN GRKIT_RECIPE_HIDDEN
#define RECIPE_LOOK_MASK GRKIT_RECIPE_LOOK_MASK
/* The look: kept, Film Look Off, or a GRKit effect. */
#define RECIPE_LOOK_KEPT GRKIT_RECIPE_LOOK_KEPT
#define RECIPE_LOOK_OFF GRKIT_RECIPE_LOOK_OFF
#define RECIPE_LOOK_EFFECT GRKIT_RECIPE_LOOK_EFFECT
/* The recipe holds Grain: its level (0 to 4) and its size (0 to 3). */
#define RECIPE_GRAIN GRKIT_RECIPE_GRAIN_HELD
#define RECIPE_GRAIN_LEVEL_SHIFT GRKIT_RECIPE_GRAIN_LEVEL_SHIFT
#define RECIPE_GRAIN_SIZE_SHIFT GRKIT_RECIPE_GRAIN_SIZE_SHIFT
/* The low 15 bits of the ImageTone code of the effect. */
#define RECIPE_CODE_SHIFT GRKIT_RECIPE_CODE_SHIFT
#define RECIPE_CODE_MASK GRKIT_RECIPE_CODE_MASK
/* The number after the name, 0 for none: the second recipe of a look is
   "Name 2". */
#define RECIPE_NUMBER_SHIFT GRKIT_RECIPE_NUMBER_SHIFT
#define RECIPE_NUMBER_MAX GRKIT_RECIPE_NUMBER_MAX
/* Word 1: the Fn adjustments of the effect, the stored bytes
   (GRKIT_RECORD_PARAMETERS) as base-9 digits, the first lowest, as in the
   state file; or the levels of the native type in the same form. Words 2
   and 3 hold the native type and the white balance (runtime/native_look.h). */
#define RECIPE_FN_WORD GRKIT_RECIPE_FN_WORD
#define RECIPE_NATIVE_WORD GRKIT_RECIPE_NATIVE_WORD
#define RECIPE_EXTRA_WORD GRKIT_RECIPE_EXTRA_WORD
/* Word 4: the module settings, RECIPE_MODULE_BITS bits for setting i of
   GRKIT_RECIPE_MODULE_SETTINGS: bit 0 holds it, then its value (0 to 3).
   The exposure settings follow (NATIVE_EXPOSURE_BITS, runtime/native_look.h),
   and RECIPE_OFF marks a recipe that is off. */
#define RECIPE_MODULE_WORD GRKIT_RECIPE_MODULE_WORD
#define RECIPE_OFF GRKIT_RECIPE_OFF
#define RECIPE_MODULE_BITS GRKIT_RECIPE_MODULE_SETTING_BITS
#define RECIPE_MODULE_VALUES (GRKIT_RECIPE_MODULE_VALUE_MASK+1U)
#define RECIPE_MODULE_HELD(w,i) ((w)>>(RECIPE_MODULE_BITS*(i))&1U)
#define RECIPE_MODULE_VALUE(w,i) ((w)>>(RECIPE_MODULE_BITS*(i)+1U)&GRKIT_RECIPE_MODULE_VALUE_MASK)

#define RECIPE_LOOK(w) ((w)&RECIPE_LOOK_MASK)
#define RECIPE_GRAIN_LEVEL(w) ((w)>>RECIPE_GRAIN_LEVEL_SHIFT&GRKIT_RECIPE_GRAIN_LEVEL_MASK)
#define RECIPE_GRAIN_SIZE(w) ((w)>>RECIPE_GRAIN_SIZE_SHIFT&GRKIT_RECIPE_GRAIN_SIZE_MASK)
#define RECIPE_CODE(w) ((w)>>RECIPE_CODE_SHIFT&RECIPE_CODE_MASK)
#define RECIPE_NUMBER(w) ((w)>>RECIPE_NUMBER_SHIFT&RECIPE_NUMBER_MAX)

/* Why a recipe cannot apply: its effect is not installed, or Film Look does
   not run. */
#define RECIPE_READY 0U
#define RECIPE_MISSING 1U
/* A native value that this camera does not have. */
#define RECIPE_UNSUPPORTED 2U
/* The bits of an apply that left settings out: a Grain that is not Off,
   because no Grain module runs, and a module setting that is not Off and
   whose module is not loaded. */
#define RECIPE_SKIPPED_GRAIN 0x100U
#define RECIPE_SKIPPED_MODULE 0x200U
/* No entry. */
#define RECIPE_NONE 0xffffffffU
/* The result of a save: the slot of the new recipe, or RECIPE_FULL. */
#define RECIPE_FULL 0xffffffffU

#define RECIPE_CAPABILITY(h,key) ((h)[GRKIT_HOST_CAPABILITIES/4]&GRKIT_CORE_##key##_BIT)
#define RECIPE_ALL (GRKIT_RECIPE_SLOTS+GRKIT_RECIPE_UNDO)
_Static_assert(GRKIT_RECIPE_WORDS>=5U,"a recipe holds its look, its levels, its native settings, and its module settings");
_Static_assert(RECIPE_MODULE_BITS*GRKIT_RECIPE_MODULE_SETTINGS<=NATIVE_EV_SHIFT,"the module settings end where the exposure settings start");

/* The native settings (runtime/native_look.h). */
static void native_read(u32 *w,u32 look);
static u32 native_supported(const volatile u32 *w);
static void native_write(const volatile u32 *w);


/* The words of slot `slot`: recipe n, or undo step GRKIT_RECIPE_SLOTS + n. */
static volatile u32 *recipe_at(volatile u32 *h,u32 slot) {
    return slot<GRKIT_RECIPE_SLOTS ? h+GRKIT_HOST_RECIPES/4+slot*GRKIT_RECIPE_WORDS :
                                     h+GRKIT_HOST_RECIPE_UNDO/4+(slot-GRKIT_RECIPE_SLOTS)*GRKIT_RECIPE_WORDS;
}
/* The name words of recipe `slot`. */
static volatile u32 *recipe_name_at(volatile u32 *h,u32 slot) {
    return h+GRKIT_HOST_RECIPE_NAMES/4+slot*GRKIT_RECIPE_NAME_WORDS;
}
/* The ID of recipe `slot` (GRKIT_HOST_RECIPE_IDS, runtime/recipe_id.h): it
   stays with the recipe when the recipe moves, so a reference to the recipe
   holds it. */
static volatile u32 *recipe_id_at(volatile u32 *h,u32 slot) {
    return h+GRKIT_HOST_RECIPE_IDS/4+slot;
}
_Static_assert(GRKIT_RECIPE_NAME_CHARS==4U*GRKIT_RECIPE_NAME_WORDS,"four characters in each name word");
/* The length of the name of recipe `slot`, 0 for the automatic name: the
   characters up to the first 0. A name with a character that is not
   printable ASCII, or with bits above its characters, counts as none. */
static u32 recipe_name_length(volatile u32 *h,u32 slot) {
    const volatile u32 *w=recipe_name_at(h,slot);
    u32 n=0, i, c, ended=0;
    for(i=0;i<GRKIT_RECIPE_NAME_CHARS;++i) {
        if(!(i&3U) && w[i>>2]>>28) return 0;
        c=w[i>>2]>>(7U*(i&3U))&0x7fU;
        if(!c) { ended=1; continue; }
        if(ended || c<0x20U || c>0x7eU) return 0;
        ++n;
    }
    return n;
}
/* Character i of the name of recipe `slot`. */
static u32 recipe_name_char(volatile u32 *h,u32 slot,u32 i) {
    return recipe_name_at(h,slot)[i>>2]>>(7U*(i&3U))&0x7fU;
}
/* Set the name of recipe `slot` from NUL-terminated ASCII: its printable
   characters without the spaces at both ends, at most
   GRKIT_RECIPE_NAME_CHARS. A text without one gives the automatic name. */
static void recipe_name_set(volatile u32 *h,u32 slot,const volatile u8 *text) {
    u32 w[GRKIT_RECIPE_NAME_WORDS]={0}, n=0, spaces=0, i, c;
    for(i=0;text[i] && n<GRKIT_RECIPE_NAME_CHARS;++i) {
        c=text[i];
        if(c<0x20U || c>0x7eU) continue;
        if(c==' ') {
            /* Spaces count only before a later character. */
            if(n) ++spaces;
            continue;
        }
        while(spaces && n<GRKIT_RECIPE_NAME_CHARS-1U) { w[n>>2]|=(u32)' '<<(7U*(n&3U)); ++n; --spaces; }
        spaces=0;
        w[n>>2]|=c<<(7U*(n&3U));
        ++n;
    }
    for(i=0;i<GRKIT_RECIPE_NAME_WORDS;++i) recipe_name_at(h,slot)[i]=w[i];
    GRKIT_RECIPES_CHANGED(h);
}
/* Whether word 0 is that of a recipe: the used bit, a known look, a grain
   level of CORE.GRAIN, and no bit above the hidden bit. */
static u32 recipe_word_valid(u32 w) {
    return (w&RECIPE_USED) && !(w>>GRKIT_RECIPE_VALUE_BITS) && RECIPE_LOOK(w)<=RECIPE_LOOK_EFFECT &&
           RECIPE_GRAIN_LEVEL(w)<GRKIT_CORE_GRAIN_VALUES;
}
/* The number of slots in a row from `first` that hold a recipe, at most `n`. */
static u32 recipe_run(volatile u32 *h,u32 first,u32 n) {
    u32 k=0;
    while(k<n && recipe_word_valid(recipe_at(h,first+k)[0])) ++k;
    return k;
}
static u32 recipe_count(volatile u32 *h) { return recipe_run(h,0,GRKIT_RECIPE_SLOTS); }
static u32 recipe_undo_count(volatile u32 *h) { return recipe_run(h,GRKIT_RECIPE_SLOTS,GRKIT_RECIPE_UNDO); }

/* The entries of the lists are the recipes that are on: the run from slot
   0, at most GRKIT_RECIPE_ON_MAX. Entry e is slot e. */
static u32 recipe_on(volatile u32 *h,u32 slot) { return !(recipe_at(h,slot)[RECIPE_MODULE_WORD]&RECIPE_OFF); }
static u32 recipe_entries(volatile u32 *h) {
    u32 n=recipe_count(h), e=0;
    while(e<n && e<GRKIT_RECIPE_ON_MAX && recipe_on(h,e)) ++e;
    return e;
}
static volatile u32 *recipe_words(volatile u32 *h,u32 e) { return recipe_at(h,e); }
/* Whether entry e shows in ADJ, and its change (the Options of the module). */
static u32 recipe_shown(volatile u32 *h,u32 e) { return !(recipe_at(h,e)[0]&RECIPE_HIDDEN); }
static void recipe_show_in_adj(volatile u32 *h,u32 e,u32 shown) {
    volatile u32 *w;
    if(e>=recipe_entries(h)) return;
    w=recipe_at(h,e);
    w[0]=shown ? w[0]&~RECIPE_HIDDEN : w[0]|RECIPE_HIDDEN;
    GRKIT_RECIPES_CHANGED(h);
}
/* The recipes that show in ADJ: their count, and the recipe of shown
   position i, or RECIPE_NONE. */
static u32 recipe_shown_count(volatile u32 *h) {
    u32 n=recipe_entries(h), e, k=0;
    for(e=0;e<n;++e) k+=recipe_shown(h,e);
    return k;
}
static u32 recipe_shown_at(volatile u32 *h,u32 i) {
    u32 n=recipe_entries(h), e;
    for(e=0;e<n;++e) if(recipe_shown(h,e) && !i--) return e;
    return RECIPE_NONE;
}
/* The shown position of recipe e, or RECIPE_NONE. */
static u32 recipe_shown_index(volatile u32 *h,u32 e) {
    u32 i, k=0;
    if(e>=recipe_entries(h) || !recipe_shown(h,e)) return RECIPE_NONE;
    for(i=0;i<e;++i) k+=recipe_shown(h,i);
    return k;
}

/* The record of record index i. */
static u32 recipe_record(volatile u32 *h,u32 i) {
    return h[GRKIT_HOST_RECORDS/4]+i*GRKIT_RECORD_SIZE;
}
/* The code of the effect of a record: the low 15 bits of the FNV-1a hash of
   its ID, which ends at a NUL before GRKIT_RECORD_CODE. */
static u32 recipe_effect_code(u32 record) {
    u32 hash=0x811c9dc5U, i, c;
    for(i=GRKIT_RECORD_ID;i<GRKIT_RECORD_CODE && (c=BYTE(record,i));++i) hash=(hash^c)*0x01000193U;
    return hash&RECIPE_CODE_MASK;
}
/* The record index of the first installed effect with this code, or
   GRKIT_TEXT_RECORDS. */
static u32 recipe_effect(volatile u32 *h,u32 code) {
    u32 n=h[GRKIT_HOST_COUNT/4], i;
    if(n>GRKIT_TEXT_RECORDS) return GRKIT_TEXT_RECORDS;
    for(i=0;i<n;++i)
        if(recipe_effect_code(recipe_record(h,i))==code) return i;
    return GRKIT_TEXT_RECORDS;
}
/* The stored Fn adjustments of a record as base-9 digits, the first lowest.
   A stored byte out of range counts as neutral. */
static u32 recipe_fn_read(u32 record) {
    u32 value=0, d=9, digit;
    while(d--) {
        digit=BYTE(record,GRKIT_RECORD_PARAMETERS+d);
        value=value*9U+(digit<=8U ? digit : 0U);
    }
    return value;
}
/* n / 9 for each u32 n, without a division helper, which the core does
   not link: the product with ceil(2^33 / 9). */
static u32 recipe_div9(u32 n) { return (u32)((unsigned long long)n*0x38e38e39ULL>>33); }
/* Write the adjustments of `value` to a record: 1 when one changed. */
static u32 recipe_fn_write(u32 record,u32 value) {
    u32 d, digit, changed=0, rest;
    for(d=0;d<9U;++d) {
        rest=recipe_div9(value);
        digit=value-9U*rest;
        value=rest;
        if(BYTE(record,GRKIT_RECORD_PARAMETERS+d)!=digit) {
            BYTE(record,GRKIT_RECORD_PARAMETERS+d)=(u8)digit;
            changed=1;
        }
    }
    return changed;
}

/* The state file IDs of the module settings that a recipe holds. */
static const u32 recipe_module_ids[GRKIT_RECIPE_MODULE_SETTINGS]={GRKIT_RECIPE_MODULE_SETTING_IDS};
#define RECIPE_NO_SETTING 0xffffffffU
/* The handle of module setting i of a recipe, or RECIPE_NO_SETTING when no
   loaded module has it, when it has more values than word 4 holds, or when
   it is color-only on GR IV Monochrome, which hides its rows. */
static u32 recipe_module_handle(volatile u32 *h,u32 i) {
    volatile u32 *t=settings_table(h);
    u32 k;
    if(!t || (GRKIT_RECIPE_MODULE_COLOR_ONLY>>i&1U && NATIVE_MONOCHROME())) return RECIPE_NO_SETTING;
    for(k=0;k<t[0];++k)
        if(t[SETTINGS_ENTRY(k)]==recipe_module_ids[i])
            return setting_limit(h,k)<=RECIPE_MODULE_VALUES ? k : RECIPE_NO_SETTING;
    return RECIPE_NO_SETTING;
}

/* The current settings as a recipe without a number. */
static void recipe_capture(volatile u32 *h,u32 *out) {
    u32 i, record, index;
    for(i=0;i<GRKIT_RECIPE_WORDS;++i) out[i]=0;
    out[0]=RECIPE_USED;
    if(RECIPE_CAPABILITY(h,TONE)) {
        record=h[GRKIT_HOST_ACTIVE/4];
        index=record_index(h,record);
        if(index<GRKIT_TEXT_RECORDS) {
            out[0]|=RECIPE_LOOK_EFFECT|recipe_effect_code(record)<<RECIPE_CODE_SHIFT;
            out[RECIPE_FN_WORD]=recipe_fn_read(record);
        }
        else out[0]|=RECIPE_LOOK_OFF;
    }
    if(RECIPE_CAPABILITY(h,GRAIN)) {
        out[0]|=RECIPE_GRAIN|setting_value(h,GRKIT_CORE_GRAIN)<<RECIPE_GRAIN_LEVEL_SHIFT;
        if(RECIPE_CAPABILITY(h,GRAIN_SIZE)) out[0]|=setting_value(h,GRKIT_CORE_GRAIN_SIZE)<<RECIPE_GRAIN_SIZE_SHIFT;
    }
    for(i=0;i<GRKIT_RECIPE_MODULE_SETTINGS;++i) {
        u32 k=recipe_module_handle(h,i);
        if(k!=RECIPE_NO_SETTING)
            out[RECIPE_MODULE_WORD]|=(1U|setting_value(h,k)<<1)<<(RECIPE_MODULE_BITS*i);
    }
    /* The native type only without an effect: an effect replaces it. */
    native_read(out,RECIPE_LOOK(out[0])!=RECIPE_LOOK_EFFECT);
    /* A native type shows only with Film Look Off, so a recipe with one has
       that look also while Film Look does not run. Apply then sets the type
       and leaves Film Look as it is. */
    if(RECIPE_LOOK(out[0])==RECIPE_LOOK_KEPT && NATIVE_TYPE(out[RECIPE_NATIVE_WORD])) out[0]|=RECIPE_LOOK_OFF;
}
/* Hold Grain and each module setting that `w` does not hold at 0 (Off): the
   module does not run, so the picture has none. A color-only setting has no
   Off and stays out. */
static void recipe_hold_off(u32 *w) {
    u32 i;
    w[0]|=RECIPE_GRAIN;
    for(i=0;i<GRKIT_RECIPE_MODULE_SETTINGS;++i)
        if(!(GRKIT_RECIPE_MODULE_COLOR_ONLY>>i&1U)) w[RECIPE_MODULE_WORD]|=1U<<(RECIPE_MODULE_BITS*i);
}
/* Whether the recipe in `w` can apply: RECIPE_READY or the reason. */
static u32 recipe_reason(volatile u32 *h,const volatile u32 *w) {
    if(RECIPE_LOOK(w[0])==RECIPE_LOOK_EFFECT &&
       (!RECIPE_CAPABILITY(h,TONE) || recipe_effect(h,RECIPE_CODE(w[0]))==GRKIT_TEXT_RECORDS)) return RECIPE_MISSING;
    if(!native_supported(w)) return RECIPE_UNSUPPORTED;
    return RECIPE_READY;
}
/* Set the settings of a recipe that can apply. The Fn adjustments go to the
   record before the effect commits, so the live view builds once with them.
   The RECIPE_SKIPPED bits of the settings that it left out. */
static u32 recipe_set(volatile u32 *h,const u32 *w) {
    u32 look=RECIPE_LOOK(w[0]), index, record, skipped=0, i, k;
    if(look==RECIPE_LOOK_EFFECT && RECIPE_CAPABILITY(h,TONE)) {
        index=recipe_effect(h,RECIPE_CODE(w[0]));
        if(index<GRKIT_TEXT_RECORDS) {
            record=recipe_record(h,index);
            if(recipe_fn_write(record,w[RECIPE_FN_WORD])) {
                ++h[GRKIT_HOST_PARAMETER_CHANGES/4];
                GRKIT_STATE_CHANGED(h);
            }
            setting_store(h,GRKIT_CORE_TONE,index+1);
        }
    }
    else if(look==RECIPE_LOOK_OFF && RECIPE_CAPABILITY(h,TONE)) setting_store(h,GRKIT_CORE_TONE,0);
    if((w[0]&RECIPE_GRAIN) && RECIPE_CAPABILITY(h,GRAIN)) {
        setting_store(h,GRKIT_CORE_GRAIN,RECIPE_GRAIN_LEVEL(w[0]));
        if(RECIPE_CAPABILITY(h,GRAIN_SIZE)) setting_store(h,GRKIT_CORE_GRAIN_SIZE,RECIPE_GRAIN_SIZE(w[0]));
    }
    else if((w[0]&RECIPE_GRAIN) && RECIPE_GRAIN_LEVEL(w[0])) skipped|=RECIPE_SKIPPED_GRAIN;
    for(i=0;i<GRKIT_RECIPE_MODULE_SETTINGS;++i) {
        if(!RECIPE_MODULE_HELD(w[RECIPE_MODULE_WORD],i)) continue;
        k=recipe_module_handle(h,i);
        /* Without its module, a setting at 0 (Off) changes nothing. */
        if(k==RECIPE_NO_SETTING) { if(RECIPE_MODULE_VALUE(w[RECIPE_MODULE_WORD],i)) skipped|=RECIPE_SKIPPED_MODULE; }
        else setting_store(h,k,RECIPE_MODULE_VALUE(w[RECIPE_MODULE_WORD],i));
    }
    /* The native settings after the effect: Film Look Off first, then the
       native type that it shows. */
    native_write(w);
    return skipped;
}
/* Whether the current settings `now` (recipe_capture) equal each setting
   that entry e (below recipe_entries) holds and that runs, besides its
   exposure settings. A recipe that holds nothing else that runs matches
   nothing. */
static u32 recipe_matches_now(volatile u32 *h,u32 e,const u32 *now) {
    u32 w[GRKIT_RECIPE_WORDS], look, held=0, c;
    for(c=0;c<GRKIT_RECIPE_WORDS;++c) w[c]=recipe_words(h,e)[c];
    look=RECIPE_LOOK(w[0]);
    if(look!=RECIPE_LOOK_KEPT && RECIPE_CAPABILITY(h,TONE)) {
        if(RECIPE_LOOK(now[0])!=look) return 0;
        if(look==RECIPE_LOOK_EFFECT &&
           (RECIPE_CODE(now[0])!=RECIPE_CODE(w[0]) || now[RECIPE_FN_WORD]!=w[RECIPE_FN_WORD])) return 0;
        held=1;
    }
    if((w[0]&RECIPE_GRAIN) && RECIPE_CAPABILITY(h,GRAIN)) {
        if(RECIPE_GRAIN_LEVEL(now[0])!=RECIPE_GRAIN_LEVEL(w[0])) return 0;
        if(RECIPE_CAPABILITY(h,GRAIN_SIZE) && RECIPE_GRAIN_SIZE(now[0])!=RECIPE_GRAIN_SIZE(w[0])) return 0;
        held=1;
    }
    for(c=0;c<GRKIT_RECIPE_MODULE_SETTINGS;++c) {
        if(!RECIPE_MODULE_HELD(w[RECIPE_MODULE_WORD],c) || !RECIPE_MODULE_HELD(now[RECIPE_MODULE_WORD],c)) continue;
        if(RECIPE_MODULE_VALUE(now[RECIPE_MODULE_WORD],c)!=RECIPE_MODULE_VALUE(w[RECIPE_MODULE_WORD],c)) return 0;
        held=1;
    }
    if(NATIVE_TYPE(w[RECIPE_NATIVE_WORD])) {
        if((now[RECIPE_NATIVE_WORD]^w[RECIPE_NATIVE_WORD])&NATIVE_LOOK_BITS || now[RECIPE_FN_WORD]!=w[RECIPE_FN_WORD] ||
           (now[RECIPE_EXTRA_WORD]^w[RECIPE_EXTRA_WORD])&NATIVE_TYPE_EXTRA_BITS) return 0;
        held=1;
    }
    for(c=0;c<NATIVE_CORRECTIONS;++c) {
        if(!NATIVE_CORRECTION(w[RECIPE_EXTRA_WORD],c)) continue;
        if(NATIVE_CORRECTION(now[RECIPE_EXTRA_WORD],c)!=NATIVE_CORRECTION(w[RECIPE_EXTRA_WORD],c)) return 0;
        held=1;
    }
    /* The exposure settings change from shot to shot, so they do not
       decide the match. */
    if(NATIVE_MODE(w[RECIPE_NATIVE_WORD])) {
        if((now[RECIPE_NATIVE_WORD]^w[RECIPE_NATIVE_WORD])&NATIVE_MODE_BITS) return 0;
        if(NATIVE_MODE(w[RECIPE_NATIVE_WORD])==NATIVE_MODE_TEMPERATURE+1U &&
           NATIVE_TEMPERATURE(now[RECIPE_EXTRA_WORD])!=NATIVE_TEMPERATURE(w[RECIPE_EXTRA_WORD])) return 0;
        held=1;
    }
    return held;
}
static u32 recipe_matches(volatile u32 *h,u32 e) {
    u32 now[GRKIT_RECIPE_WORDS];
    if(e>=recipe_entries(h)) return 0;
    recipe_capture(h,now);
    return recipe_matches_now(h,e,now);
}
/* The first recipe that matches the current settings, of those that show
   in ADJ when `shown` is set, else of all that are on; or RECIPE_NONE. One
   capture serves each recipe: a draw of a page can ask for it on each row. */
static u32 recipe_matching(volatile u32 *h,u32 shown) {
    u32 n=recipe_entries(h), e, now[GRKIT_RECIPE_WORDS];
    if(!n) return RECIPE_NONE;
    recipe_capture(h,now);
    for(e=0;e<n;++e) if((!shown || recipe_shown(h,e)) && recipe_matches_now(h,e,now)) return e;
    return RECIPE_NONE;
}
/* The current recipe of ADJ, which lists only the recipes that it shows. */
static u32 recipe_current(volatile u32 *h) { return recipe_matching(h,1); }

static void recipe_copy(volatile u32 *to,const volatile u32 *from) {
    u32 i;
    for(i=0;i<GRKIT_RECIPE_WORDS;++i) to[i]=from[i];
}
static void recipe_clear(volatile u32 *w) {
    u32 i;
    for(i=0;i<GRKIT_RECIPE_WORDS;++i) w[i]=0;
}
/* Move slot `from` to slot `to`, both recipes or both undo steps. A recipe
   takes its name and its ID with it: each part of a recipe that the header
   keeps by slot moves here. */
static void recipe_slot_move(volatile u32 *h,u32 to,u32 from) {
    u32 w;
    recipe_copy(recipe_at(h,to),recipe_at(h,from));
    if(to>=GRKIT_RECIPE_SLOTS) return;
    for(w=0;w<GRKIT_RECIPE_NAME_WORDS;++w) recipe_name_at(h,to)[w]=recipe_name_at(h,from)[w];
    *recipe_id_at(h,to)=*recipe_id_at(h,from);
}
/* Empty slot `slot`: its words, and for a recipe its name and ID. */
static void recipe_slot_clear(volatile u32 *h,u32 slot) {
    u32 w;
    recipe_clear(recipe_at(h,slot));
    if(slot>=GRKIT_RECIPE_SLOTS) return;
    for(w=0;w<GRKIT_RECIPE_NAME_WORDS;++w) recipe_name_at(h,slot)[w]=0;
    *recipe_id_at(h,slot)=0;
}
/* Move slots first + 1 .. first + n - 1 down by one and clear the last. */
static void recipe_close_gap(volatile u32 *h,u32 first,u32 n) {
    u32 i;
    for(i=first;i+1<n;++i) recipe_slot_move(h,i,i+1);
    if(n) recipe_slot_clear(h,n-1);
}

/* A generated ID for a new recipe (runtime/recipe_id.h): none of the IDs of
   the recipes and none of the user modes of the mode dial. 0 when each
   candidate is taken, and INSTALL generates one at the next start. */
static u32 recipe_new_id(volatile u32 *h) {
    u32 attempt, id, i, taken;
    for(attempt=0;attempt<RECIPE_ID_ATTEMPTS;++attempt) {
        id=recipe_id_candidate(attempt);
        taken=!id || id>GRKIT_RECIPE_ID_MAX;
        for(i=0;i<GRKIT_RECIPE_SLOTS && !taken;++i) taken=*recipe_id_at(h,i)==id;
        for(i=0;i<GRKIT_RECIPE_DIAL_POSITIONS && !taken;++i) taken=h[GRKIT_HOST_RECIPE_DIAL/4+i]==id;
        if(!taken) return id;
    }
    return 0;
}
/* Two recipes have the same name base: the same look and effect, or the
   same native type (runtime/recipe_page.h names them). */
static u32 recipe_same_base(const volatile u32 *a,const u32 *b) {
    if(RECIPE_LOOK(a[0])==RECIPE_LOOK_EFFECT || RECIPE_LOOK(b[0])==RECIPE_LOOK_EFFECT)
        return RECIPE_LOOK(a[0])==RECIPE_LOOK(b[0]) && RECIPE_CODE(a[0])==RECIPE_CODE(b[0]);
    if(NATIVE_TYPE(a[RECIPE_NATIVE_WORD]) || NATIVE_TYPE(b[RECIPE_NATIVE_WORD]))
        return NATIVE_TYPE(a[RECIPE_NATIVE_WORD])==NATIVE_TYPE(b[RECIPE_NATIVE_WORD]);
    return RECIPE_LOOK(a[0])==RECIPE_LOOK(b[0]);
}
/* Save the current settings as a new recipe that is on, after the other
   recipes that are on: its entry, or RECIPE_FULL while GRKIT_RECIPE_ON_MAX
   recipes are on or all slots are taken. The recipes that are off move down
   by one. The first recipe of a look has no number, the next one the
   largest number of that look plus 1 (2 after the first). */
static u32 recipe_save(volatile u32 *h) {
    u32 n=recipe_count(h), e=recipe_entries(h), w[GRKIT_RECIPE_WORDS], i, number=0, other;
    if(n>=GRKIT_RECIPE_SLOTS || e>=GRKIT_RECIPE_ON_MAX) return RECIPE_FULL;
    /* A whole look: without the exposure settings, and with Off for each
       module that does not run. */
    recipe_capture(h,w);
    w[RECIPE_MODULE_WORD]&=~NATIVE_EXPOSURE_BITS;
    recipe_hold_off(w);
    for(i=0;i<n;++i) {
        if(!recipe_same_base(recipe_at(h,i),w)) continue;
        other=recipe_at(h,i)[0];
        if(number<RECIPE_NUMBER(other)) number=RECIPE_NUMBER(other);
        if(number<1U) number=1U;
    }
    if(number) number=number<RECIPE_NUMBER_MAX ? number+1U : RECIPE_NUMBER_MAX;
    w[0]|=number<<RECIPE_NUMBER_SHIFT;
    for(i=n;i>e;--i) recipe_slot_move(h,i,i-1U);
    /* A new recipe has the automatic name and a new ID. */
    recipe_slot_clear(h,e);
    for(i=0;i<GRKIT_RECIPE_WORDS;++i) recipe_at(h,e)[i]=w[i];
    *recipe_id_at(h,e)=recipe_new_id(h);
    GRKIT_RECIPES_CHANGED(h);
    return e;
}
/* Delete entry `slot`. The recipes after it move up. */
static void recipe_delete(volatile u32 *h,u32 slot) {
    u32 n=recipe_count(h);
    if(slot>=recipe_entries(h)) return;
    recipe_close_gap(h,slot,n);
    GRKIT_RECIPES_CHANGED(h);
}
/* Store the current settings as the newest undo step. With all steps
   taken, the oldest goes. */
static void recipe_undo_push(volatile u32 *h) {
    u32 n=recipe_undo_count(h), w[GRKIT_RECIPE_WORDS], i;
    if(n==GRKIT_RECIPE_UNDO) {
        recipe_close_gap(h,GRKIT_RECIPE_SLOTS,GRKIT_RECIPE_SLOTS+n);
        --n;
    }
    recipe_capture(h,w);
    for(i=0;i<GRKIT_RECIPE_WORDS;++i) recipe_at(h,GRKIT_RECIPE_SLOTS+n)[i]=w[i];
}
/* Whether entry e can apply: RECIPE_READY or the reason. */
static u32 recipe_entry_reason(volatile u32 *h,u32 e) {
    if(e>=recipe_entries(h)) return RECIPE_MISSING;
    return recipe_reason(h,recipe_words(h,e));
}
/* Apply entry e: RECIPE_READY with the RECIPE_SKIPPED bits of the settings
   that it left out, or the reason, and then nothing changes. The settings
   before it become an undo step. */
static u32 recipe_apply(volatile u32 *h,u32 e) {
    u32 w[GRKIT_RECIPE_WORDS], reason, i;
    reason=recipe_entry_reason(h,e);
    if(reason!=RECIPE_READY) return reason;
    for(i=0;i<GRKIT_RECIPE_WORDS;++i) w[i]=recipe_words(h,e)[i];
    recipe_undo_push(h);
    return RECIPE_READY|recipe_set(h,w);
}
/* Restore the newest undo step and remove it. A step whose effect is no
   longer installed restores the settings that it can. */
static void recipe_undo(volatile u32 *h) {
    u32 n=recipe_undo_count(h), w[GRKIT_RECIPE_WORDS], i;
    volatile u32 *step;
    if(!n) return;
    step=recipe_at(h,GRKIT_RECIPE_SLOTS+n-1);
    for(i=0;i<GRKIT_RECIPE_WORDS;++i) w[i]=step[i];
    recipe_clear(step);
    if(recipe_reason(h,w)==RECIPE_MISSING) w[0]&=~RECIPE_LOOK_MASK;
    if(!native_supported(w)) {
        w[RECIPE_NATIVE_WORD]=w[RECIPE_EXTRA_WORD]=0;
        w[RECIPE_MODULE_WORD]&=~NATIVE_EXPOSURE_BITS;
    }
    recipe_set(h,w);
}
#endif
