#ifndef GRKIT_NATIVE_LOOK_H
#define GRKIT_NATIVE_LOOK_H
/* The native picture settings of camera recipes (runtime/recipes.h): the
   Image Control type of the main set with its parameters and its native
   grain, the white balance, the correction and noise reduction settings,
   and the exposure settings: the compensation, the Auto ISO upper limit,
   metering, and the ND filter. The model is runtime/native.h, and the build header of each
   target gives its stored values and field offsets
   (runtime/targets/<target>-native.h).

   - A read takes each field from the settings record of FW_SETTINGS.
   - A write calls the Set dispatcher of the RTOS shell command UserData
     (FW_USERDATA_SET) with the setting name and the label of the value, as
     the native menu sets them. The dispatcher refuses a label that it does
     not know, and each value comes from the label tables.

   A read takes the type and its parameters from the active set: the main
   set or a Custom set. A write selects the main set first, so a recipe
   saved from a Custom set sets its type in the main set.

   GR IV Monochrome (the model record of FW_MODEL_RECORD) has its own types
   and no white balance. StillImageControlNo0 has no label for its types, so
   a write calls the type setter (FW_IMAGE_CONTROL_SET) with the stored
   value, as the native menu does.

   The includer defines u8, u16, u32, BYTE, HALF, and WORD. A host test
   defines NATIVE_SETTINGS, NATIVE_SET, NATIVE_SET_TYPE, and
   NATIVE_MONOCHROME. */
#include "native.h"

#ifndef NATIVE_SETTINGS
#define NATIVE_SETTINGS() CALL0(FW_SETTINGS)()
#endif
#ifndef NATIVE_SET
#define NATIVE_SET(argv) CALL2(FW_USERDATA_SET)(4,(u32)(argv))
#endif
#if !defined(NATIVE_SET_TYPE) && defined(FW_IMAGE_CONTROL_SET)
#define NATIVE_SET_TYPE(code) CALL2(FW_IMAGE_CONTROL_SET)(NATIVE_SETTINGS(),(code))
#endif

#if defined(NATIVE_FAMILY_GR3)
#define NATIVE_OF(name) NATIVE_GR3_##name
#elif defined(NATIVE_FAMILY_GR4)
#define NATIVE_OF(name) NATIVE_GR4_##name
#else
#error "The native header of the target names its family"
#endif

/* The record words of the native fields (runtime/recipes.h). The layout is
   GRKIT_RECIPE_* of runtime/abi.h (grkit_core::recipes::layout).
   Word 2 (NATIVE_LOOK_WORD): the type + 1 (0: no type), its tone, the HDR
   intensity, the B&W filter, the white balance mode + 1 (0: no mode), its
   A-B and G-M shifts.
   Word 3 (NATIVE_EXTRA_WORD): the color temperature, (kelvin - 2500) / 10;
   each correction setting, its stored value + 1 (0: Unset, which keeps the
   camera value); the native grain of the type; and its Solarization type. */
#define NATIVE_LEVELS_WORD GRKIT_RECIPE_FN_WORD
#define NATIVE_LOOK_WORD GRKIT_RECIPE_NATIVE_WORD
#define NATIVE_EXTRA_WORD GRKIT_RECIPE_EXTRA_WORD
#define NATIVE_TYPE_SHIFT GRKIT_RECIPE_TYPE_SHIFT
#define NATIVE_TONE_SHIFT GRKIT_RECIPE_TONE_SHIFT
#define NATIVE_INTENSITY_SHIFT GRKIT_RECIPE_INTENSITY_SHIFT
#define NATIVE_FILTER_SHIFT GRKIT_RECIPE_FILTER_SHIFT
#define NATIVE_MODE_SHIFT GRKIT_RECIPE_MODE_SHIFT
#define NATIVE_BA_SHIFT GRKIT_RECIPE_BA_SHIFT
#define NATIVE_GM_SHIFT GRKIT_RECIPE_GM_SHIFT
#define NATIVE_FIELD(w,shift,mask) ((w)>>(shift)&(mask))
#define NATIVE_TYPE(w) NATIVE_FIELD(w,NATIVE_TYPE_SHIFT,GRKIT_RECIPE_TYPE_MASK)
#define NATIVE_TONE(w) NATIVE_FIELD(w,NATIVE_TONE_SHIFT,GRKIT_RECIPE_TONE_MASK)
#define NATIVE_INTENSITY(w) NATIVE_FIELD(w,NATIVE_INTENSITY_SHIFT,GRKIT_RECIPE_INTENSITY_MASK)
#define NATIVE_FILTER(w) NATIVE_FIELD(w,NATIVE_FILTER_SHIFT,GRKIT_RECIPE_FILTER_MASK)
#define NATIVE_MODE(w) NATIVE_FIELD(w,NATIVE_MODE_SHIFT,GRKIT_RECIPE_MODE_MASK)
#define NATIVE_BA(w) NATIVE_FIELD(w,NATIVE_BA_SHIFT,GRKIT_RECIPE_SHIFT_MASK)
#define NATIVE_GM(w) NATIVE_FIELD(w,NATIVE_GM_SHIFT,GRKIT_RECIPE_SHIFT_MASK)
#define NATIVE_TEMPERATURE(w) ((w)&GRKIT_RECIPE_TEMPERATURE_MASK)
#define NATIVE_CORRECTION_SHIFT GRKIT_RECIPE_CORRECTION_SHIFT
#define NATIVE_CORRECTION(w,c) \
    NATIVE_FIELD(w,NATIVE_CORRECTION_SHIFT+GRKIT_RECIPE_CORRECTION_BITS*(c),GRKIT_RECIPE_CORRECTION_MASK)
/* The native grain in portable values: the strength, 0 Off and 1 to 3 Low
   to High, and the size, 0 to 2 Small to Large. GR III has one size, 0. */
#define NATIVE_GRAIN_SHIFT GRKIT_RECIPE_NATIVE_GRAIN_SHIFT
#define NATIVE_GRAIN_SIZE_SHIFT GRKIT_RECIPE_NATIVE_GRAIN_SIZE_SHIFT
#define NATIVE_GRAIN(w) NATIVE_FIELD(w,NATIVE_GRAIN_SHIFT,GRKIT_RECIPE_NATIVE_GRAIN_MASK)
#define NATIVE_GRAIN_SIZE(w) NATIVE_FIELD(w,NATIVE_GRAIN_SIZE_SHIFT,GRKIT_RECIPE_NATIVE_GRAIN_SIZE_MASK)
#define NATIVE_GRAIN_BITS GRKIT_RECIPE_NATIVE_GRAIN_BITS
/* The stored SolarizationType of a Monochrome Solarized Effect, else 0. */
#define NATIVE_SOLARIZATION_SHIFT GRKIT_RECIPE_SOLARIZATION_SHIFT
#define NATIVE_SOLARIZATION(w) NATIVE_FIELD(w,NATIVE_SOLARIZATION_SHIFT,GRKIT_RECIPE_SOLARIZATION_MASK)
/* Word 4 after the module settings of runtime/recipes.h: the exposure
   settings, each 0 when the recipe does not hold it. The compensation in
   sixths of a stop + NATIVE_EV_ZERO (1 to 61), so a step of 1/3 or 1/2 is
   exact; the third stop of the Auto ISO upper limit from ISO 100 + 1; the
   stored metering + 1; and the stored ND filter + 1. */
#define NATIVE_EXPOSURE_WORD GRKIT_RECIPE_MODULE_WORD
#define NATIVE_EV_SHIFT GRKIT_RECIPE_EV_SHIFT
#define NATIVE_EV_ZERO ((int)GRKIT_RECIPE_EV_ZERO)
#define NATIVE_ISO_SHIFT GRKIT_RECIPE_ISO_SHIFT
#define NATIVE_METERING_SHIFT GRKIT_RECIPE_METERING_SHIFT
#define NATIVE_ND_SHIFT GRKIT_RECIPE_ND_SHIFT
#define NATIVE_EV(w) NATIVE_FIELD(w,NATIVE_EV_SHIFT,GRKIT_RECIPE_EV_MASK)
#define NATIVE_ISO(w) NATIVE_FIELD(w,NATIVE_ISO_SHIFT,GRKIT_RECIPE_ISO_MASK)
#define NATIVE_METERING(w) NATIVE_FIELD(w,NATIVE_METERING_SHIFT,GRKIT_RECIPE_METERING_MASK)
#define NATIVE_ND(w) NATIVE_FIELD(w,NATIVE_ND_SHIFT,GRKIT_RECIPE_ND_MASK)
#define NATIVE_EXPOSURE_BITS GRKIT_RECIPE_EXPOSURE_BITS
/* The third stops of the Auto ISO upper limit, from ISO 100. */
#define NATIVE_ISO_STEPS NATIVE_OF(ISO_STEPS)
/* The bits of word 3 that belong to the type, and the bits of word 2 of the
   type and of the white balance. */
#define NATIVE_TYPE_EXTRA_BITS GRKIT_RECIPE_TYPE_EXTRA_BITS
#define NATIVE_LOOK_BITS GRKIT_RECIPE_LOOK_BITS
#define NATIVE_MODE_BITS GRKIT_RECIPE_MODE_BITS
_Static_assert(NATIVE_TYPES<GRKIT_RECIPE_TYPE_MASK,"word 2 holds the type + 1");
/* The portable indexes of the parameters after the levels that a word
   holds: the tone of a type is the one tone parameter that it has. */
#define NATIVE_FIRST_TONE NATIVE_PARAMETER_COLORTONEMONOTONE
#define NATIVE_LAST_TONE NATIVE_PARAMETER_COLORTONECINEMA
#define NATIVE_NAME_BYTES 80U

static const u8 native_type_codes[NATIVE_TYPES]={NATIVE_TYPE_CODES};
static const u16 native_mode_codes[NATIVE_MODES]={NATIVE_MODE_CODES};
static const u16 native_parameter_fields[NATIVE_TYPES*NATIVE_PARAMETERS]={NATIVE_PARAMETER_FIELDS};
static const u8 native_parameter_strides[NATIVE_TYPES*NATIVE_PARAMETERS]={NATIVE_PARAMETER_SET_STRIDES};
static const u16 native_shift_fields[NATIVE_MODES*2U]={NATIVE_SHIFT_FIELDS};
static const char *const native_prefixes[]={NATIVE_PREFIXES};
static const char *const native_keys[NATIVE_PARAMETERS]={NATIVE_PARAMETER_KEYS};
static const char *const native_type_labels[NATIVE_TYPES]={NATIVE_OF(TYPE_LABELS)};
static const u8 native_type_prefixes[NATIVE_TYPES]={NATIVE_OF(TYPE_PREFIXES)};
static const char *const native_mode_labels[NATIVE_MODES]={NATIVE_OF(MODE_LABELS)};
static const u8 native_mode_shifts[NATIVE_MODES]={NATIVE_OF(MODE_SHIFTS)};
static const char *const native_labels[]={NATIVE_OF(LABELS)};
static const u8 native_label_starts[NATIVE_PARAMETERS]={NATIVE_OF(LABEL_STARTS)};
static const u8 native_label_counts[NATIVE_PARAMETERS]={NATIVE_OF(LABEL_COUNTS)};
static const char *const native_ba_labels[]={NATIVE_SHIFT_BA_LABELS};
static const char *const native_gm_labels[]={NATIVE_SHIFT_GM_LABELS};
#define NATIVE_SHIFTS ((u32)(sizeof(native_ba_labels)/sizeof(native_ba_labels[0])))
static const char *const native_metering_labels[]={NATIVE_METERING_LABELS};
static const char *const native_nd_labels[]={NATIVE_ND_LABELS};
static const u16 native_correction_fields[NATIVE_CORRECTIONS]={NATIVE_CORRECTION_FIELDS};
static const char *const native_correction_names[NATIVE_CORRECTIONS]={NATIVE_CORRECTION_NAMES};
static const char *const native_correction_0[]={NATIVE_CORRECTION_0_LABELS};
static const char *const native_correction_1[]={NATIVE_CORRECTION_1_LABELS};
static const char *const native_correction_2[]={NATIVE_CORRECTION_2_LABELS};
static const char *const native_correction_3[]={NATIVE_CORRECTION_3_LABELS};
#define NATIVE_COUNT(a) ((u8)(sizeof(a)/sizeof((a)[0])))
static const char *const *const native_correction_labels[NATIVE_CORRECTIONS]={
    native_correction_0,native_correction_1,native_correction_2,native_correction_3};
static const u8 native_correction_counts[NATIVE_CORRECTIONS]={NATIVE_COUNT(native_correction_0),
    NATIVE_COUNT(native_correction_1),NATIVE_COUNT(native_correction_2),NATIVE_COUNT(native_correction_3)};
_Static_assert(NATIVE_CORRECTIONS==4U,"a label table for each correction setting");

/* n / 9 and n / 10 for each u32 n, without a division helper, which the
   core does not link: the products with ceil(2^33 / 9) and ceil(2^35 / 10). */
static u32 native_div9(u32 n) { return (u32)((unsigned long long)n*0x38e38e39ULL>>33); }
static u32 native_div10(u32 n) { return (u32)((unsigned long long)n*0xcccccccdULL>>35); }

/* The exposure settings are floats, and the core has no float code: an
   integer reads them, and a write passes the number as text, which the
   dispatcher parses.

   round(v * k) for the float v of the bits f and a small k (at most 6), or
   NATIVE_NO_STEP for |v| of 2^20 or more. Zero and the subnormals give 0. */
#define NATIVE_NO_STEP 0x7fffffff
static int native_float_steps(u32 f,u32 k) {
    u32 e=f>>23&0xffU, m=((f&0x7fffffU)|0x800000U)*k, shift, r;
    if(!e) return 0;
    if(e>=147U) return NATIVE_NO_STEP;
    shift=150U-e;
    r=shift>=32U ? 0U : (m+(1U<<(shift-1U)))>>shift;
    return f>>31 ? -(int)r : (int)r;
}
/* n rounded to a multiple of `grid`, a tie toward 0. */
static int native_snap(int n,u32 grid) {
    u32 a=n<0 ? (u32)-n : (u32)n, q=0;
    while(a>=grid) { a-=grid; ++q; }
    if(2U*a>grid) ++q;
    return n<0 ? -(int)(q*grid) : (int)(q*grid);
}
/* The text of n / q (q at most 9) with ten decimals, which parses to the
   float nearest to it, in `to` of at least 16 bytes. */
static void native_decimal(char *to,int n,u32 q) {
    u32 a=n<0 ? (u32)-n : (u32)n, whole=0, at=0, i, d;
    char digits[4];
    if(n<0) to[at++]='-';
    while(a>=q) { a-=q; ++whole; }
    i=0;
    do { u32 rest=native_div10(whole); digits[i++]=(char)('0'+whole-10U*rest); whole=rest; } while(whole && i<4U);
    while(i) to[at++]=digits[--i];
    to[at++]='.';
    for(i=0;i<10U;++i) {
        a*=10U; d=0;
        while(a>=q) { a-=q; ++d; }
        to[at++]=(char)('0'+d);
    }
    to[at]=0;
}
/* The exposure compensation in sixths of a stop, and the third stop of an
   Auto ISO limit from ISO 100, of the fields of the record. */
static int native_ev_read(u32 record) { return native_float_steps(WORD(record,NATIVE_EXPOSURE_FIELD),6U); }
static int native_iso_read(u32 record,u32 field) {
    int n=native_float_steps(WORD(record,field),3U);
    return n==NATIVE_NO_STEP ? n : n-3*(int)NATIVE_ISO_SV_BASE;
}
/* The exposure bits of word 4 as the camera takes them: the compensation
   rounded to the exposure step of the camera (1/3 or 1/2), and the ISO limit
   rounded to its ISO step (1/3 or 1) and not below its lower limit. */
static u32 native_exposure_fit(u32 record,u32 w) {
    u32 out=w&NATIVE_EXPOSURE_BITS;
    if(!record) return out;
    if(NATIVE_EV(w)) {
        u32 grid=native_float_steps(WORD(record,NATIVE_EXPOSURE_STEP_FIELD),6U)==3 ? 3U : 2U;
        int ev=native_snap((int)NATIVE_EV(w)-NATIVE_EV_ZERO,grid);
        out=(out&~(GRKIT_RECIPE_EV_MASK<<NATIVE_EV_SHIFT))|(u32)(ev+NATIVE_EV_ZERO)<<NATIVE_EV_SHIFT;
    }
    if(NATIVE_ISO(w)) {
        int n=(int)NATIVE_ISO(w)-1, lower=native_iso_read(record,NATIVE_ISO_LOWER_FIELD);
        if(native_float_steps(WORD(record,NATIVE_ISO_STEP_FIELD),3U)==3) n=native_snap(n,3U);
        if(lower>n && lower<(int)NATIVE_ISO_STEPS) n=lower;
        if(n>=(int)NATIVE_ISO_STEPS) n-=3;
        out=(out&~(GRKIT_RECIPE_ISO_MASK<<NATIVE_ISO_SHIFT))|(u32)(n+1)<<NATIVE_ISO_SHIFT;
    }
    return out;
}

/* Whether the camera is GR IV Monochrome: 1 or 0. */
#ifndef NATIVE_MONOCHROME
#ifdef FW_MODEL_RECORD
static u32 native_model_monochrome(void) {
    u32 record=WORD(FW_MODEL_RECORD,0);
    return record && BYTE(record,NATIVE_SENSOR_COLOR_FIELD)==1U;
}
#define NATIVE_MONOCHROME() native_model_monochrome()
#else
#define NATIVE_MONOCHROME() 0U
#endif
#endif
/* Whether type t belongs to this camera: a Monochrome type on GR IV
   Monochrome, any other type elsewhere. */
static u32 native_type_fits(u32 t,u32 monochrome) {
    return t<NATIVE_TYPES && native_type_codes[t]!=0xffU && (t>=NATIVE_FIRST_MONOCHROME)==monochrome;
}

/* The settings record. */
static u32 native_record(void) {
    u32 settings=NATIVE_SETTINGS();
#ifdef NATIVE_RECORD
    return settings ? WORD(settings,NATIVE_RECORD) : 0;
#else
    return settings;
#endif
}
/* The field offset of parameter p of type t in the main set, or 0. */
static u32 native_field(u32 t,u32 p) { return native_parameter_fields[t*NATIVE_PARAMETERS+p]; }
/* The stored value of parameter p of type t in set `set` of the record. */
static u32 native_value(u32 record,u32 set,u32 t,u32 p) {
    return BYTE(record,native_field(t,p)+set*native_parameter_strides[t*NATIVE_PARAMETERS+p]);
}
/* The label of stored value v of parameter p, or 0. */
static const char *native_label(u32 p,u32 v) {
    return v<native_label_counts[p] ? native_labels[native_label_starts[p]+v] : 0;
}
/* A level as a digit of a record word: (native - 4) mod 9, so 0 is neutral
   (runtime/recipes.h), and back. */
static u32 native_digit(u32 v) { return v>=4U ? v-4U : v+5U; }
static u32 native_level(u32 digit) { return digit>=5U ? digit-5U : digit+4U; }
/* The tone parameter of a type, or NATIVE_PARAMETERS. */
static u32 native_tone(u32 t) {
    u32 p;
    for(p=NATIVE_FIRST_TONE;p<=NATIVE_LAST_TONE;++p)
        if(p!=NATIVE_PARAMETER_HDRINTENSE && native_field(t,p)) return p;
    return NATIVE_PARAMETERS;
}

/* The native grain of type t. Each family keeps it in its own form:
   - GR IV: Particle Off or On, ParticleStrength Low to High, ParticleSize
     Small to Large.
   - GR III: only Particle, with the strength as Off, High, Mid, or Low.
   A type without Particle has no native grain. */
static u32 native_grain_sized(u32 t) { return native_field(t,NATIVE_PARAMETER_PARTICLESTRENGTH)!=0; }
/* The stored GR III Particle of strength s, and back: the same mapping. */
static u32 native_grain_particle(u32 s) { return s ? 4U-s : 0U; }
/* The native grain of type t in set `set` of the record as word-3 bits, or
   0xffffffff for a value out of the tables. */
static u32 native_grain_read(u32 record,u32 set,u32 t) {
    u32 particle, strength, size;
    if(!native_field(t,NATIVE_PARAMETER_PARTICLE)) return 0;
    particle=native_value(record,set,t,NATIVE_PARAMETER_PARTICLE);
    if(!native_label(NATIVE_PARAMETER_PARTICLE,particle)) return 0xffffffffU;
    if(!native_grain_sized(t)) return native_grain_particle(particle)<<NATIVE_GRAIN_SHIFT;
    if(!particle) return 0;
    strength=native_value(record,set,t,NATIVE_PARAMETER_PARTICLESTRENGTH);
    size=native_value(record,set,t,NATIVE_PARAMETER_PARTICLESIZE);
    if(!native_label(NATIVE_PARAMETER_PARTICLESTRENGTH,strength) ||
       !native_label(NATIVE_PARAMETER_PARTICLESIZE,size)) return 0xffffffffU;
    return (strength+1U)<<NATIVE_GRAIN_SHIFT|size<<NATIVE_GRAIN_SIZE_SHIFT;
}
/* Whether type t has native grain of strength s and size `size`: 1 or 0.
   Off fits each type. */
static u32 native_grain_fits(u32 t,u32 s,u32 size) {
    if(!s) return !size;
    if(!native_field(t,NATIVE_PARAMETER_PARTICLE)) return 0;
    if(!native_grain_sized(t)) return !size;
    return native_label(NATIVE_PARAMETER_PARTICLESIZE,size)!=0;
}

/* Read the native settings into the record words: the type of the active
   set, its parameters, and its native grain in words 1 to 3 when `look` is
   set, the
   white balance in words 2 and 3, each correction setting in word 3, and
   the exposure settings in word 4, from the settings record `record`, or
   from a copy of it (runtime/recipe_preview.h).
   A value out of the tables is not held, and GR IV Monochrome holds no
   white balance. */
static void native_read_at(u32 *w,u32 look,u32 record) {
    u32 t, p, v, digits=0, code, mode, kelvin, grain, c, monochrome, set;
    if(!record) return;
    monochrome=NATIVE_MONOCHROME();
    set=BYTE(record,NATIVE_ACTIVE_SET_FIELD);
    if(look && set<NATIVE_SETS) {
        code=BYTE(record,NATIVE_TYPE_FIELD+set*NATIVE_TYPE_SET_STRIDE);
        for(t=0;t<NATIVE_TYPES && native_type_codes[t]!=code;++t) {}
        if(code!=0xffU && native_type_fits(t,monochrome)) {
            u32 word=(t+1U)<<NATIVE_TYPE_SHIFT, ok=1, tone=native_tone(t);
            for(p=NATIVE_LEVELS;p--;) {
                v=native_field(t,p) ? native_value(record,set,t,p) : 4U;
                if(v>8U) ok=0;
                digits=digits*9U+native_digit(v);
            }
            if(tone<NATIVE_PARAMETERS) {
                v=native_value(record,set,t,tone);
                if(!native_label(tone,v)) ok=0;
                word|=v<<NATIVE_TONE_SHIFT;
            }
            if(native_field(t,NATIVE_PARAMETER_HDRINTENSE)) {
                v=native_value(record,set,t,NATIVE_PARAMETER_HDRINTENSE);
                if(!native_label(NATIVE_PARAMETER_HDRINTENSE,v)) ok=0;
                word|=v<<NATIVE_INTENSITY_SHIFT;
            }
            if(native_field(t,NATIVE_PARAMETER_FILTEREFFECTCUSTOM)) {
                v=native_value(record,set,t,NATIVE_PARAMETER_FILTEREFFECTCUSTOM);
                if(!native_label(NATIVE_PARAMETER_FILTEREFFECTCUSTOM,v)) ok=0;
                word|=v<<NATIVE_FILTER_SHIFT;
            }
            grain=native_grain_read(record,set,t);
            if(grain==0xffffffffU) ok=0;
            if(native_field(t,NATIVE_PARAMETER_SOLARIZATIONTYPE)) {
                v=native_value(record,set,t,NATIVE_PARAMETER_SOLARIZATIONTYPE);
                if(!native_label(NATIVE_PARAMETER_SOLARIZATIONTYPE,v)) ok=0;
                else grain|=v<<NATIVE_SOLARIZATION_SHIFT;
            }
            if(ok) { w[NATIVE_LEVELS_WORD]=digits; w[NATIVE_LOOK_WORD]|=word; w[NATIVE_EXTRA_WORD]|=grain; }
        }
    }
    for(c=0;c<NATIVE_CORRECTIONS;++c) {
        v=BYTE(record,native_correction_fields[c]);
        if(v<native_correction_counts[c])
            w[NATIVE_EXTRA_WORD]|=(v+1U)<<(NATIVE_CORRECTION_SHIFT+GRKIT_RECIPE_CORRECTION_BITS*c);
    }
    code=BYTE(record,NATIVE_MODE_FIELD);
    for(mode=0;mode<NATIVE_MODES && native_mode_codes[mode]!=code;++mode) {}
    if(mode<NATIVE_MODES && !monochrome) {
        u32 ba=14U, gm=14U;
        if(native_mode_shifts[mode]) {
            ba=BYTE(record,native_shift_fields[2U*mode]);
            gm=BYTE(record,native_shift_fields[2U*mode+1U]);
        }
        if(ba<NATIVE_SHIFTS && gm<NATIVE_SHIFTS)
            w[NATIVE_LOOK_WORD]|=(mode+1U)<<NATIVE_MODE_SHIFT|ba<<NATIVE_BA_SHIFT|gm<<NATIVE_GM_SHIFT;
        kelvin=HALF(record,NATIVE_TEMPERATURE_FIELD);
        if(kelvin>=NATIVE_TEMPERATURE_MIN && kelvin<=NATIVE_TEMPERATURE_MAX)
            w[NATIVE_EXTRA_WORD]|=native_div10(kelvin-NATIVE_TEMPERATURE_MIN);
    }
    {
        int ev=native_ev_read(record), iso=native_iso_read(record,NATIVE_ISO_UPPER_FIELD);
        if(ev>=-2*(int)NATIVE_EXPOSURE_THIRDS_MAX && ev<=2*(int)NATIVE_EXPOSURE_THIRDS_MAX)
            w[NATIVE_EXPOSURE_WORD]|=(u32)(ev+NATIVE_EV_ZERO)<<NATIVE_EV_SHIFT;
        if(iso>=0 && iso<(int)NATIVE_ISO_STEPS) w[NATIVE_EXPOSURE_WORD]|=(u32)(iso+1)<<NATIVE_ISO_SHIFT;
        v=BYTE(record,NATIVE_METERING_FIELD);
        if(v<NATIVE_COUNT(native_metering_labels)) w[NATIVE_EXPOSURE_WORD]|=(v+1U)<<NATIVE_METERING_SHIFT;
        v=BYTE(record,NATIVE_ND_FIELD);
        if(v<NATIVE_COUNT(native_nd_labels)) w[NATIVE_EXPOSURE_WORD]|=(v+1U)<<NATIVE_ND_SHIFT;
    }
}

/* Whether this camera has each native value of the words: 1 or 0. */
static u32 native_supported(const volatile u32 *w) {
    u32 t=NATIVE_TYPE(w[NATIVE_LOOK_WORD]), mode=NATIVE_MODE(w[NATIVE_LOOK_WORD]), p, digits=w[NATIVE_LEVELS_WORD];
    u32 monochrome=NATIVE_MONOCHROME();
    if(t) {
        --t;
        if(!native_type_fits(t,monochrome)) return 0;
        for(p=0;p<NATIVE_LEVELS;++p) {
            u32 rest=native_div9(digits);
            if(native_field(t,p) && !native_label(p,native_level(digits-9U*rest))) return 0;
            digits=rest;
        }
        if(native_tone(t)<NATIVE_PARAMETERS && !native_label(native_tone(t),NATIVE_TONE(w[NATIVE_LOOK_WORD]))) return 0;
        if(native_field(t,NATIVE_PARAMETER_HDRINTENSE) &&
           !native_label(NATIVE_PARAMETER_HDRINTENSE,NATIVE_INTENSITY(w[NATIVE_LOOK_WORD]))) return 0;
        if(native_field(t,NATIVE_PARAMETER_FILTEREFFECTCUSTOM) &&
           !native_label(NATIVE_PARAMETER_FILTEREFFECTCUSTOM,NATIVE_FILTER(w[NATIVE_LOOK_WORD]))) return 0;
        if(!native_grain_fits(t,NATIVE_GRAIN(w[NATIVE_EXTRA_WORD]),NATIVE_GRAIN_SIZE(w[NATIVE_EXTRA_WORD]))) return 0;
        if(native_field(t,NATIVE_PARAMETER_SOLARIZATIONTYPE) ?
           !native_label(NATIVE_PARAMETER_SOLARIZATIONTYPE,NATIVE_SOLARIZATION(w[NATIVE_EXTRA_WORD])) : NATIVE_SOLARIZATION(w[NATIVE_EXTRA_WORD])!=0U)
            return 0;
    }
    else if(w[NATIVE_EXTRA_WORD]&NATIVE_TYPE_EXTRA_BITS) return 0;
    for(p=0;p<NATIVE_CORRECTIONS;++p)
        if(NATIVE_CORRECTION(w[NATIVE_EXTRA_WORD],p)>native_correction_counts[p]) return 0;
    {
        u32 x=w[NATIVE_EXPOSURE_WORD];
        if(NATIVE_EV(x) && (NATIVE_EV(x)<(u32)NATIVE_EV_ZERO-2U*NATIVE_EXPOSURE_THIRDS_MAX ||
                            NATIVE_EV(x)>(u32)NATIVE_EV_ZERO+2U*NATIVE_EXPOSURE_THIRDS_MAX)) return 0;
        if(NATIVE_ISO(x)>NATIVE_ISO_STEPS || NATIVE_METERING(x)>NATIVE_COUNT(native_metering_labels) ||
           NATIVE_ND(x)>NATIVE_COUNT(native_nd_labels)) return 0;
    }
    if(mode) {
        --mode;
        if(monochrome || mode>=NATIVE_MODES || native_mode_codes[mode]==0xffffU) return 0;
        if(native_mode_shifts[mode] && (NATIVE_BA(w[NATIVE_LOOK_WORD])>=NATIVE_SHIFTS || NATIVE_GM(w[NATIVE_LOOK_WORD])>=NATIVE_SHIFTS))
            return 0;
        if(mode==NATIVE_MODE_TEMPERATURE && NATIVE_TEMPERATURE(w[NATIVE_EXTRA_WORD])>
           (NATIVE_TEMPERATURE_MAX-NATIVE_TEMPERATURE_MIN)/NATIVE_TEMPERATURE_STEP) return 0;
        _Static_assert(NATIVE_TEMPERATURE_STEP==10U,"native_div10 gives the step of the temperature");
    }
    return 1;
}

/* Append a NUL-terminated text to `to`, which holds NATIVE_NAME_BYTES. */
static u32 native_cat(char *to,u32 at,const char *text) {
    while(*text && at+1U<NATIVE_NAME_BYTES) to[at++]=*text++;
    to[at]=0;
    return at;
}
static void native_set(const char *name,const char *label) {
    const char *argv[4]={"UserData","Set",name,label};
    if(label) NATIVE_SET(argv);
}
/* Set parameter p of type t to stored value v. */
static void native_set_parameter(u32 t,u32 p,u32 v) {
    char name[NATIVE_NAME_BYTES];
    u32 at=native_cat(name,0,native_prefixes[native_type_prefixes[t]]);
    at=native_cat(name,at,native_type_labels[t]);
    native_cat(name,at,native_keys[p]);
    native_set(name,native_label(p,v));
}

/* ---- Writes ----------------------------------------------------------------
   A write goes field by field. Each native value of the words has a field
   in the settings record: the active set, the type of the main set, each
   parameter of a type in the main set, the white balance mode, the shifts
   of a mode, the color temperature, each correction setting, and each
   exposure setting. native_sync gives each field the value that the words
   `to` set, or for a field that `to` does not hold, its value in `base`, a
   settings record or a copy of one (runtime/recipe_preview.h):
   - With `force`, as an apply, it writes each field that `to` holds, also
     when the field has that value already.
   - Else it writes only a field whose value differs, and only while the
     field shows its value in `base` or the value that the words `from` set:
     a field that the camera changed meanwhile, as when the mode dial loads
     the settings of another position, stays.
   `live` is the settings record, or 0 when it is not readable: then each
   field that `to` holds is written. The order is that of the native menu:
   the active set, the type, its parameters, the white balance mode, its
   shifts, the color temperature, the corrections, then the exposure. */
#define NATIVE_UNHELD 0xffffffffU
typedef struct {
    u32 live, base, force;
    const volatile u32 *from, *to;
} NativeSync;
/* Whether to write a field that shows `now` and holds `base` in the base,
   which `to` sets to `want` (`held`) and `from` to `was` (`had`). `known`
   is 0 for a field that cannot be read. */
static u32 native_due(const NativeSync *s,u32 known,u32 now,u32 base,u32 held,u32 want,u32 had,u32 was) {
    if(!known) return held;
    if(s->force && held) return 1;
    return now!=(held ? want : base) && (now==base || (had && now==was));
}
/* The same for a field of `size` bytes (1 or 2) at `at` of the records, or
   at 0 for a field that the build does not give. `want` and `was` are the
   values of `to` and `from`, NATIVE_UNHELD for a field that they do not
   hold. The value to write goes to *value. */
static u32 native_due_field(const NativeSync *s,u32 at,u32 size,u32 want,u32 was,u32 *value) {
    u32 known=s->live && at, now=0, base=0;
    if(known) {
        now=size==1U ? BYTE(s->live,at) : HALF(s->live,at);
        base=size==1U ? BYTE(s->base,at) : HALF(s->base,at);
    }
    *value=want!=NATIVE_UNHELD ? want : base;
    return native_due(s,known,now,base,want!=NATIVE_UNHELD,want,was!=NATIVE_UNHELD,was);
}
/* The stored value of parameter p of type t that the words w set, or
   NATIVE_UNHELD: the levels, the tone of the type, the HDR intensity, the
   B&W filter, the native grain, and the Solarization type of the type of
   w. A sized native grain (GR IV) sets its size and strength only while it
   is on. */
static u32 native_parameter_held(const volatile u32 *w,u32 t,u32 p) {
    u32 x, s, digits, i;
    if(!w || NATIVE_TYPE(w[NATIVE_LOOK_WORD])!=t+1U || !native_field(t,p)) return NATIVE_UNHELD;
    x=w[NATIVE_EXTRA_WORD];
    s=NATIVE_GRAIN(x);
    if(p<NATIVE_LEVELS) {
        digits=w[NATIVE_LEVELS_WORD];
        for(i=0;i<p;++i) digits=native_div9(digits);
        return native_level(digits-9U*native_div9(digits));
    }
    if(p==native_tone(t)) return NATIVE_TONE(w[NATIVE_LOOK_WORD]);
    switch(p) {
    case NATIVE_PARAMETER_HDRINTENSE: return NATIVE_INTENSITY(w[NATIVE_LOOK_WORD]);
    case NATIVE_PARAMETER_FILTEREFFECTCUSTOM: return NATIVE_FILTER(w[NATIVE_LOOK_WORD]);
    case NATIVE_PARAMETER_PARTICLE: return native_grain_sized(t) ? s!=0U : native_grain_particle(s);
    case NATIVE_PARAMETER_PARTICLESIZE: return native_grain_sized(t) && s ? NATIVE_GRAIN_SIZE(x) : NATIVE_UNHELD;
    case NATIVE_PARAMETER_PARTICLESTRENGTH: return native_grain_sized(t) && s ? s-1U : NATIVE_UNHELD;
    case NATIVE_PARAMETER_SOLARIZATIONTYPE: return NATIVE_SOLARIZATION(x);
    default: return NATIVE_UNHELD;
    }
}
/* The stored code of the type that the words w set, or NATIVE_UNHELD; the
   same for the white balance mode. */
static u32 native_type_held(const volatile u32 *w) {
    u32 t=w ? NATIVE_TYPE(w[NATIVE_LOOK_WORD]) : 0;
    return t ? native_type_codes[t-1U] : NATIVE_UNHELD;
}
static u32 native_mode_held(const volatile u32 *w) {
    u32 m=w ? NATIVE_MODE(w[NATIVE_LOOK_WORD]) : 0;
    return m ? native_mode_codes[m-1U] : NATIVE_UNHELD;
}
/* The shift (`gm` 0 for A-B, 1 for G-M) of mode m that the words w set,
   or NATIVE_UNHELD. */
static u32 native_shift_held(const volatile u32 *w,u32 m,u32 gm) {
    if(!w || NATIVE_MODE(w[NATIVE_LOOK_WORD])!=m+1U || !native_mode_shifts[m]) return NATIVE_UNHELD;
    return gm ? NATIVE_GM(w[NATIVE_LOOK_WORD]) : NATIVE_BA(w[NATIVE_LOOK_WORD]);
}
/* The color temperature in kelvin that the words w set, or NATIVE_UNHELD. */
static u32 native_temperature_held(const volatile u32 *w) {
    if(!w || NATIVE_MODE(w[NATIVE_LOOK_WORD])!=NATIVE_MODE_TEMPERATURE+1U) return NATIVE_UNHELD;
    return NATIVE_TEMPERATURE_MIN+NATIVE_TEMPERATURE(w[NATIVE_EXTRA_WORD])*NATIVE_TEMPERATURE_STEP;
}
/* The stored value of correction c that the words w set, or NATIVE_UNHELD. */
static u32 native_correction_held(const volatile u32 *w,u32 c) {
    u32 v=w ? NATIVE_CORRECTION(w[NATIVE_EXTRA_WORD],c) : 0;
    return v ? v-1U : NATIVE_UNHELD;
}
/* The exposure bits of the words w as the camera takes them, or 0. */
static u32 native_exposure_held(const NativeSync *s,const volatile u32 *w) {
    return w ? native_exposure_fit(s->live,w[NATIVE_EXPOSURE_WORD]) : 0;
}

/* Set the active set to set v, whose label is No and its number. */
static void native_set_active(u32 v) {
    char label[4]={'N','o',(char)('0'+v),0};
    if(v<NATIVE_SETS) native_set(NATIVE_ACTIVE_SET_SETTING,label);
}
/* Set the type of the main set to stored code `code`. GR IV Monochrome
   sets its types with the type setter. */
static void native_set_type(u32 code) {
    u32 t;
    for(t=0;t<NATIVE_TYPES && (native_type_codes[t]!=code || code==0xffU);++t) {}
    if(t>=NATIVE_TYPES) return;
#ifdef NATIVE_SET_TYPE
    if(t>=NATIVE_FIRST_MONOCHROME) { NATIVE_SET_TYPE(code); return; }
#endif
    native_set(NATIVE_TYPE_SETTING,native_type_labels[t]);
}
/* Set the white balance mode to stored code `code`. */
static void native_set_mode(u32 code) {
    u32 m;
    for(m=0;m<NATIVE_MODES && native_mode_codes[m]!=code;++m) {}
    if(m<NATIVE_MODES) native_set(NATIVE_WHITE_BALANCE_SETTING,native_mode_labels[m]);
}
/* Set the A-B (`gm` 0) or the G-M shift of mode m to stored value v. */
static void native_set_shift(u32 m,u32 gm,u32 v) {
    char name[NATIVE_NAME_BYTES];
    u32 at;
    if(!native_mode_labels[m] || v>=NATIVE_SHIFTS) return;
    at=native_cat(name,0,NATIVE_WHITE_BALANCE_SETTING);
    at=native_cat(name,at,native_mode_labels[m]);
    native_cat(name,at,gm ? "AdjustGM" : "AdjustBA");
    native_set(name,gm ? native_gm_labels[v] : native_ba_labels[v]);
}
static void native_set_temperature(u32 kelvin) {
    char number[6];
    u32 i=5;
    if(kelvin<NATIVE_TEMPERATURE_MIN || kelvin>NATIVE_TEMPERATURE_MAX) return;
    number[5]=0;
    while(i--) { u32 rest=native_div10(kelvin); number[i]=(char)('0'+kelvin-10U*rest); kelvin=rest; }
    native_set(NATIVE_COLOR_TEMPERATURE_SETTING,number[0]=='0' ? number+1 : number);
}
static void native_set_correction(u32 c,u32 v) {
    if(v<native_correction_counts[c]) native_set(native_correction_names[c],native_correction_labels[c][v]);
}

/* Write the parameters of type t - 1 (none for 0). */
static void native_sync_type(const NativeSync *s,u32 t) {
    u32 p, v;
    if(!t--) return;
    for(p=0;p<NATIVE_PARAMETERS;++p)
        if(native_field(t,p) &&
           native_due_field(s,native_field(t,p),1,native_parameter_held(s->to,t,p),native_parameter_held(s->from,t,p),&v))
            native_set_parameter(t,p,v);
}
/* Write the shifts of mode m - 1 (none for 0). */
static void native_sync_shifts(const NativeSync *s,u32 m) {
    u32 gm, v;
    if(!m-- || !native_mode_shifts[m]) return;
    for(gm=0;gm<2U;++gm)
        if(native_due_field(s,native_shift_fields[2U*m+gm],1,native_shift_held(s->to,m,gm),native_shift_held(s->from,m,gm),&v))
            native_set_shift(m,gm,v);
}
/* Write the exposure compensation (`iso` 0) or the Auto ISO upper limit in
   their steps: sixths of a stop, and third stops from ISO 100. x and y are
   the exposure bits of `to` and `from` (native_exposure_held). */
static void native_sync_steps(const NativeSync *s,u32 iso,u32 x,u32 y) {
    char name[NATIVE_NAME_BYTES];
    u32 held=iso ? NATIVE_ISO(x) : NATIVE_EV(x), had=iso ? NATIVE_ISO(y) : NATIVE_EV(y);
    int now=0, base=0, want, was;
    if(s->live) {
        now=iso ? native_iso_read(s->live,NATIVE_ISO_UPPER_FIELD) : native_ev_read(s->live);
        base=iso ? native_iso_read(s->base,NATIVE_ISO_UPPER_FIELD) : native_ev_read(s->base);
    }
    want=iso ? (int)held-1 : (int)held-NATIVE_EV_ZERO;
    was=iso ? (int)had-1 : (int)had-NATIVE_EV_ZERO;
    if(!native_due(s,s->live!=0,(u32)now,(u32)base,held!=0,(u32)want,had!=0,(u32)was)) return;
    if(!held) want=base;
    if(want==NATIVE_NO_STEP) return;
    if(iso) {
        native_decimal(name,3*(int)NATIVE_ISO_SV_BASE+want,3U);
        native_set(NATIVE_ISO_UPPER_SETTING,name);
    }
    else {
        native_decimal(name,want,6U);
        native_set(NATIVE_EXPOSURE_SETTING,name);
    }
}
static void native_sync(u32 live,u32 base,const volatile u32 *from,const volatile u32 *to,u32 force) {
    NativeSync s, once;
    u32 tt=to ? NATIVE_TYPE(to[NATIVE_LOOK_WORD]) : 0, ft=from ? NATIVE_TYPE(from[NATIVE_LOOK_WORD]) : 0;
    u32 tm=to ? NATIVE_MODE(to[NATIVE_LOOK_WORD]) : 0, fm=from ? NATIVE_MODE(from[NATIVE_LOOK_WORD]) : 0;
    u32 v, c, x, y;
    s.live=live; s.base=live ? base : 0; s.force=force; s.from=from; s.to=to;
    /* A type is one of the main set, so it makes the main set active. Only
       a change of the set writes it. */
    once=s;
    once.force=0;
    if(live && native_due_field(&once,NATIVE_ACTIVE_SET_FIELD,1,tt ? 0U : NATIVE_UNHELD,ft ? 0U : NATIVE_UNHELD,&v))
        native_set_active(v);
    if(native_due_field(&s,NATIVE_TYPE_FIELD,1,native_type_held(to),native_type_held(from),&v)) native_set_type(v);
    native_sync_type(&s,tt);
    if(ft!=tt) native_sync_type(&s,ft);
    if(native_due_field(&s,NATIVE_MODE_FIELD,1,native_mode_held(to),native_mode_held(from),&v)) native_set_mode(v);
    native_sync_shifts(&s,tm);
    if(fm!=tm) native_sync_shifts(&s,fm);
    if(native_due_field(&s,NATIVE_TEMPERATURE_FIELD,2,native_temperature_held(to),native_temperature_held(from),&v))
        native_set_temperature(v);
    for(c=0;c<NATIVE_CORRECTIONS;++c)
        if(native_due_field(&s,native_correction_fields[c],1,native_correction_held(to,c),native_correction_held(from,c),&v))
            native_set_correction(c,v);
    x=native_exposure_held(&s,to);
    y=native_exposure_held(&s,from);
    native_sync_steps(&s,0,x,y);
    native_sync_steps(&s,1,x,y);
    if(native_due_field(&s,NATIVE_METERING_FIELD,1,NATIVE_METERING(x) ? NATIVE_METERING(x)-1U : NATIVE_UNHELD,
                        NATIVE_METERING(y) ? NATIVE_METERING(y)-1U : NATIVE_UNHELD,&v) &&
       v<NATIVE_COUNT(native_metering_labels))
        native_set(NATIVE_METERING_SETTING,native_metering_labels[v]);
    if(native_due_field(&s,NATIVE_ND_FIELD,1,NATIVE_ND(x) ? NATIVE_ND(x)-1U : NATIVE_UNHELD,
                        NATIVE_ND(y) ? NATIVE_ND(y)-1U : NATIVE_UNHELD,&v) &&
       v<NATIVE_COUNT(native_nd_labels))
        native_set(NATIVE_ND_SETTING,native_nd_labels[v]);
}

/* Write the native settings of the words that the camera supports
   (native_supported): the type, its parameters, its native grain, and its
   Solarization type, then the white balance mode, its shifts, and the color
   temperature of ColorTemp1, then each correction setting that is not
   Unset, then each exposure setting that the recipe holds, as the camera
   takes it (native_exposure_fit). Each one is written, also when it has
   that value already. */
static void native_write(const volatile u32 *w) {
    u32 record=native_record();
    native_sync(record,record,0,w,1);
}
#endif
