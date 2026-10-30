#ifndef GRKIT_RECIPES_H
#define GRKIT_RECIPES_H
/* The camera recipes of CORE.RECIPE: named snapshots of the picture settings
   that OK applies in one step. The header holds them (GRKIT_HOST_RECIPES):
   GRKIT_RECIPE_SLOTS recipes, then GRKIT_RECIPE_UNDO undo steps, the oldest
   first, GRKIT_RECIPE_WORDS words each. A recipe whose word 0 is not a
   recipe word ends the list. The resident loader saves each word that is
   not 0 in the state file, and INSTALL restores the words
   (grkit_core::runtime::RECIPE_KEY_BASE).

   A recipe holds the settings of the capabilities that run when it is
   saved: the look of CORE.TONE (Off, or an effect and its Fn adjustments)
   and the grain of CORE.GRAIN and CORE.GRAIN_SIZE. Apply sets each held
   setting whose capability runs, so a recipe never turns on a capability.
   It also holds the native settings (runtime/native_look.h): the native
   type, its parameters, and its native grain while no effect is active,
   the white balance, and the correction and noise reduction settings.

   Without a running Grain module, CORE.GRAIN has no effect on pictures
   (grain_level in runtime/picture.S). Then apply gives the Grain of a
   recipe to the native grain of its native type where the values match
   (recipe_native_grain), and else skips it.

   The includer defines u8, u16, u32, the header fields, HALF and BYTE,
   record_index (runtime/tone.h), setting_store (runtime/setting_store.h),
   and the native settings (runtime/native_look.h). */

/* Word 0. RECIPE_USED is set in each recipe, so word 0 of a recipe is never
   0, and each word stays below 2^29 (RECIPE_VALUE_MAX). */
#define RECIPE_USED 0x08000000U
#define RECIPE_LOOK_MASK 3U
/* The look: kept, Film Look Off, or a GRKit effect. */
#define RECIPE_LOOK_KEPT 0U
#define RECIPE_LOOK_OFF 1U
#define RECIPE_LOOK_EFFECT 2U
/* The recipe holds Grain: its level (0 to 4) and its size (0 to 3). */
#define RECIPE_GRAIN 4U
#define RECIPE_GRAIN_LEVEL_SHIFT 3U
#define RECIPE_GRAIN_SIZE_SHIFT 6U
/* The low 15 bits of the ImageTone code of the effect. */
#define RECIPE_CODE_SHIFT 8U
#define RECIPE_CODE_MASK 0x7fffU
/* The number after the name, 0 for none: the second recipe of a look is
   "Name 2". */
#define RECIPE_NUMBER_SHIFT 23U
#define RECIPE_NUMBER_MAX 15U
/* Word 1: the Fn adjustments of the effect, the stored bytes
   (GRKIT_RECORD_PARAMETERS) as base-9 digits, the first lowest, as in the
   state file; or the levels of the native type in the same form. Words 2
   and 3 hold the native type and the white balance (runtime/native_look.h). */
#define RECIPE_FN_WORD 1U
#define RECIPE_NATIVE_WORD 2U
#define RECIPE_EXTRA_WORD 3U

#define RECIPE_LOOK(w) ((w)&RECIPE_LOOK_MASK)
#define RECIPE_GRAIN_LEVEL(w) ((w)>>RECIPE_GRAIN_LEVEL_SHIFT&7U)
#define RECIPE_GRAIN_SIZE(w) ((w)>>RECIPE_GRAIN_SIZE_SHIFT&3U)
#define RECIPE_CODE(w) ((w)>>RECIPE_CODE_SHIFT&RECIPE_CODE_MASK)
#define RECIPE_NUMBER(w) ((w)>>RECIPE_NUMBER_SHIFT&RECIPE_NUMBER_MAX)

/* Why a recipe cannot apply: its effect is not installed, or Film Look does
   not run. */
#define RECIPE_READY 0U
#define RECIPE_MISSING 1U
/* A native value that this camera does not have. */
#define RECIPE_UNSUPPORTED 2U
/* Applied without its Grain: no Grain module runs, and the native grain
   has no match. */
#define RECIPE_GRAIN_SKIPPED 3U
/* The result of a save: the slot of the new recipe, or RECIPE_FULL. */
#define RECIPE_FULL 0xffffffffU

#define RECIPE_CAPABILITY(h,key) ((h)[GRKIT_HOST_CAPABILITIES/4]&GRKIT_CORE_##key##_BIT)
#define RECIPE_ALL (GRKIT_RECIPE_SLOTS+GRKIT_RECIPE_UNDO)
_Static_assert(GRKIT_RECIPE_WORDS>=4U,"a recipe holds its look, its levels, and its native settings");

/* The native settings (runtime/native_look.h). */
static void native_read(u32 *w,u32 look);
static u32 native_supported(const volatile u32 *w);
static void native_write(const volatile u32 *w);
static u32 native_grain_sized(u32 t);
static u32 native_grain_fits(u32 t,u32 s,u32 size);

/* The words of slot `slot`: recipe n, or undo step GRKIT_RECIPE_SLOTS + n. */
static volatile u32 *recipe_at(volatile u32 *h,u32 slot) {
    return h+GRKIT_HOST_RECIPES/4+slot*GRKIT_RECIPE_WORDS;
}
/* Whether word 0 is that of a recipe: the used bit, a known look, a grain
   level of CORE.GRAIN, and no bit above the number. */
static u32 recipe_word_valid(u32 w) {
    return (w&RECIPE_USED) && !(w>>28) && RECIPE_LOOK(w)<=RECIPE_LOOK_EFFECT &&
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

/* The record of record index i. */
static u32 recipe_record(volatile u32 *h,u32 i) {
    return h[GRKIT_HOST_RECORDS/4]+i*GRKIT_RECORD_SIZE;
}
/* The record index of the installed effect with this code, or
   GRKIT_TEXT_RECORDS. */
static u32 recipe_effect(volatile u32 *h,u32 code) {
    u32 n=h[GRKIT_HOST_COUNT/4], i;
    if(n>GRKIT_TEXT_RECORDS) return GRKIT_TEXT_RECORDS;
    for(i=0;i<n;++i)
        if((HALF(recipe_record(h,i),GRKIT_RECORD_CODE)&RECIPE_CODE_MASK)==code) return i;
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

/* Word 3 with the native grain of the Grain of a recipe, or
   RECIPE_NO_GRAIN. Grain levels 1 to 3 are the native strengths Low to
   High, and Extra Strong has no match. Size Auto keeps the size of the
   native look, and a fixed size needs a type with native sizes (GR IV). */
#define RECIPE_NO_GRAIN 0xffffffffU
static u32 recipe_native_grain(const volatile u32 *w) {
    u32 t=NATIVE_TYPE(w[RECIPE_NATIVE_WORD]), level=RECIPE_GRAIN_LEVEL(w[0]), size=RECIPE_GRAIN_SIZE(w[0]), native;
    if(!t || t>NATIVE_TYPES || level>3U) return RECIPE_NO_GRAIN;
    --t;
    if(size && !native_grain_sized(t)) return RECIPE_NO_GRAIN;
    native=size ? size-1U : NATIVE_GRAIN_SIZE(w[RECIPE_EXTRA_WORD]);
    if(!native_grain_fits(t,level,native)) return RECIPE_NO_GRAIN;
    return (w[RECIPE_EXTRA_WORD]&~NATIVE_GRAIN_BITS)|level<<NATIVE_GRAIN_SHIFT|native<<NATIVE_GRAIN_SIZE_SHIFT;
}
/* The words of a recipe as apply writes them: while no Grain module runs,
   the native grain takes the Grain of the recipe (recipe_native_grain).
   Grain Off keeps the native grain of the look, as CORE.GRAIN Off does.
   1 when the Grain of the recipe has no effect and no native match. */
static u32 recipe_native_words(volatile u32 *h,const volatile u32 *w,u32 *out) {
    u32 i, grain;
    for(i=0;i<GRKIT_RECIPE_WORDS;++i) out[i]=w[i];
    if(!(w[0]&RECIPE_GRAIN) || RECIPE_CAPABILITY(h,GRAIN) || !RECIPE_GRAIN_LEVEL(w[0])) return 0;
    grain=recipe_native_grain(w);
    if(grain==RECIPE_NO_GRAIN) return 1;
    out[RECIPE_EXTRA_WORD]=grain;
    return 0;
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
            out[0]|=RECIPE_LOOK_EFFECT|(HALF(record,GRKIT_RECORD_CODE)&RECIPE_CODE_MASK)<<RECIPE_CODE_SHIFT;
            out[RECIPE_FN_WORD]=recipe_fn_read(record);
        }
        else out[0]|=RECIPE_LOOK_OFF;
    }
    if(RECIPE_CAPABILITY(h,GRAIN)) {
        out[0]|=RECIPE_GRAIN|setting_value(h,GRKIT_CORE_GRAIN)<<RECIPE_GRAIN_LEVEL_SHIFT;
        if(RECIPE_CAPABILITY(h,GRAIN_SIZE)) out[0]|=setting_value(h,GRKIT_CORE_GRAIN_SIZE)<<RECIPE_GRAIN_SIZE_SHIFT;
    }
    /* The native type only without an effect: an effect replaces it. */
    native_read(out,RECIPE_LOOK(out[0])!=RECIPE_LOOK_EFFECT);
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
   1 when the Grain of the recipe has no effect (recipe_native_words). */
static u32 recipe_set(volatile u32 *h,const u32 *w) {
    u32 look=RECIPE_LOOK(w[0]), index, record, native[GRKIT_RECIPE_WORDS], skipped;
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
    /* The native settings after the effect: Film Look Off first, then the
       native type that it shows. */
    skipped=recipe_native_words(h,w,native);
    native_write(native);
    return skipped;
}
/* Whether the current settings equal each setting that a recipe holds and
   that runs. A recipe that holds nothing that runs matches nothing. */
static u32 recipe_matches(volatile u32 *h,u32 slot) {
    u32 w[GRKIT_RECIPE_WORDS], now[GRKIT_RECIPE_WORDS], look, held=0, c;
    recipe_native_words(h,recipe_at(h,slot),w);
    look=RECIPE_LOOK(w[0]);
    recipe_capture(h,now);
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
    if(NATIVE_MODE(w[RECIPE_NATIVE_WORD])) {
        if((now[RECIPE_NATIVE_WORD]^w[RECIPE_NATIVE_WORD])&NATIVE_MODE_BITS) return 0;
        if(NATIVE_MODE(w[RECIPE_NATIVE_WORD])==NATIVE_MODE_TEMPERATURE+1U &&
           NATIVE_TEMPERATURE(now[RECIPE_EXTRA_WORD])!=NATIVE_TEMPERATURE(w[RECIPE_EXTRA_WORD])) return 0;
        held=1;
    }
    return held;
}
/* The first recipe that matches the current settings, or GRKIT_RECIPE_SLOTS. */
static u32 recipe_current(volatile u32 *h) {
    u32 n=recipe_count(h), i;
    for(i=0;i<n;++i) if(recipe_matches(h,i)) return i;
    return GRKIT_RECIPE_SLOTS;
}

static void recipe_copy(volatile u32 *to,const volatile u32 *from) {
    u32 i;
    for(i=0;i<GRKIT_RECIPE_WORDS;++i) to[i]=from[i];
}
static void recipe_clear(volatile u32 *w) {
    u32 i;
    for(i=0;i<GRKIT_RECIPE_WORDS;++i) w[i]=0;
}
/* Move slots first + 1 .. first + n - 1 down by one and clear the last. */
static void recipe_close_gap(volatile u32 *h,u32 first,u32 n) {
    u32 i;
    for(i=first;i+1<n;++i) recipe_copy(recipe_at(h,i),recipe_at(h,i+1));
    if(n) recipe_clear(recipe_at(h,n-1));
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
/* Save the current settings as a new recipe after the others: its slot, or
   RECIPE_FULL. The first recipe of a look has no number, the next one the
   largest number of that look plus 1 (2 after the first). */
static u32 recipe_save(volatile u32 *h) {
    u32 n=recipe_count(h), w[GRKIT_RECIPE_WORDS], i, number=0, other;
    if(n>=GRKIT_RECIPE_SLOTS) return RECIPE_FULL;
    recipe_capture(h,w);
    for(i=0;i<n;++i) {
        if(!recipe_same_base(recipe_at(h,i),w)) continue;
        other=recipe_at(h,i)[0];
        if(number<RECIPE_NUMBER(other)) number=RECIPE_NUMBER(other);
        if(number<1U) number=1U;
    }
    if(number) number=number<RECIPE_NUMBER_MAX ? number+1U : RECIPE_NUMBER_MAX;
    w[0]|=number<<RECIPE_NUMBER_SHIFT;
    for(i=0;i<GRKIT_RECIPE_WORDS;++i) recipe_at(h,n)[i]=w[i];
    GRKIT_STATE_CHANGED(h);
    return n;
}
/* Delete recipe `slot`. The recipes after it move up. */
static void recipe_delete(volatile u32 *h,u32 slot) {
    u32 n=recipe_count(h);
    if(slot>=n) return;
    recipe_close_gap(h,slot,n);
    GRKIT_STATE_CHANGED(h);
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
    GRKIT_STATE_CHANGED(h);
}
/* Apply recipe `slot`: RECIPE_READY or RECIPE_GRAIN_SKIPPED, or the
   reason, and then nothing changes. The settings before it become an undo
   step. */
static u32 recipe_apply(volatile u32 *h,u32 slot) {
    u32 w[GRKIT_RECIPE_WORDS], reason, i;
    if(slot>=recipe_count(h)) return RECIPE_MISSING;
    reason=recipe_reason(h,recipe_at(h,slot));
    if(reason!=RECIPE_READY) return reason;
    for(i=0;i<GRKIT_RECIPE_WORDS;++i) w[i]=recipe_at(h,slot)[i];
    recipe_undo_push(h);
    return recipe_set(h,w) ? RECIPE_GRAIN_SKIPPED : RECIPE_READY;
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
    GRKIT_STATE_CHANGED(h);
    if(recipe_reason(h,w)==RECIPE_MISSING) w[0]&=~RECIPE_LOOK_MASK;
    if(!native_supported(w)) w[RECIPE_NATIVE_WORD]=w[RECIPE_EXTRA_WORD]=0;
    recipe_set(h,w);
}
#endif
