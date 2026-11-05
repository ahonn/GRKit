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

   The settings of a Custom set are not read: while a Custom set is active,
   a recipe holds no type. A write selects the main set first.

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

/* The record words of the native fields (runtime/recipes.h). */
#define NATIVE_TYPE_SHIFT 0U
#define NATIVE_TONE_SHIFT 5U
#define NATIVE_INTENSITY_SHIFT 9U
#define NATIVE_FILTER_SHIFT 11U
#define NATIVE_MODE_SHIFT 14U
#define NATIVE_BA_SHIFT 19U
#define NATIVE_GM_SHIFT 24U
#define NATIVE_FIELD(w,shift,mask) ((w)>>(shift)&(mask))
/* Word 2: the type + 1 (0: no type), its tone, the HDR intensity, the B&W
   filter, the white balance mode + 1 (0: no mode), its A-B and G-M shifts.
   Word 3: the color temperature, (kelvin - 2500) / 10; each correction
   setting, its stored value + 1 (0: Unset, which keeps the camera value);
   the native grain of the type; and its Solarization type. */
#define NATIVE_TYPE(w) NATIVE_FIELD(w,NATIVE_TYPE_SHIFT,31U)
#define NATIVE_TONE(w) NATIVE_FIELD(w,NATIVE_TONE_SHIFT,15U)
#define NATIVE_INTENSITY(w) NATIVE_FIELD(w,NATIVE_INTENSITY_SHIFT,3U)
#define NATIVE_FILTER(w) NATIVE_FIELD(w,NATIVE_FILTER_SHIFT,7U)
#define NATIVE_MODE(w) NATIVE_FIELD(w,NATIVE_MODE_SHIFT,31U)
#define NATIVE_BA(w) NATIVE_FIELD(w,NATIVE_BA_SHIFT,31U)
#define NATIVE_GM(w) NATIVE_FIELD(w,NATIVE_GM_SHIFT,31U)
#define NATIVE_TEMPERATURE(w) ((w)&1023U)
#define NATIVE_CORRECTION_SHIFT 10U
#define NATIVE_CORRECTION(w,c) NATIVE_FIELD(w,NATIVE_CORRECTION_SHIFT+3U*(c),7U)
/* The native grain in portable values: the strength, 0 Off and 1 to 3 Low
   to High, and the size, 0 to 2 Small to Large. GR III has one size, 0. */
#define NATIVE_GRAIN_SHIFT 22U
#define NATIVE_GRAIN_SIZE_SHIFT 24U
#define NATIVE_GRAIN(w) NATIVE_FIELD(w,NATIVE_GRAIN_SHIFT,3U)
#define NATIVE_GRAIN_SIZE(w) NATIVE_FIELD(w,NATIVE_GRAIN_SIZE_SHIFT,3U)
#define NATIVE_GRAIN_BITS 0x03c00000U
/* The stored SolarizationType of a Monochrome Solarized Effect, else 0. */
#define NATIVE_SOLARIZATION_SHIFT 26U
#define NATIVE_SOLARIZATION(w) NATIVE_FIELD(w,NATIVE_SOLARIZATION_SHIFT,7U)
/* Word 4 from bit 9, after the module settings of runtime/recipes.h: the
   exposure settings, each 0 when the recipe does not hold it. The
   compensation in sixths of a stop + 31 (1 to 61), so a step of 1/3 or 1/2
   is exact; the third stop of the Auto ISO upper limit from ISO 100 + 1;
   the stored metering + 1; and the stored ND filter + 1. */
#define NATIVE_EXPOSURE_WORD 4U
#define NATIVE_EV_SHIFT 9U
#define NATIVE_EV_ZERO 31
#define NATIVE_ISO_SHIFT 15U
#define NATIVE_METERING_SHIFT 21U
#define NATIVE_ND_SHIFT 24U
#define NATIVE_EV(w) NATIVE_FIELD(w,NATIVE_EV_SHIFT,63U)
#define NATIVE_ISO(w) NATIVE_FIELD(w,NATIVE_ISO_SHIFT,63U)
#define NATIVE_METERING(w) NATIVE_FIELD(w,NATIVE_METERING_SHIFT,7U)
#define NATIVE_ND(w) NATIVE_FIELD(w,NATIVE_ND_SHIFT,3U)
#define NATIVE_EXPOSURE_BITS 0x03fffe00U
/* The third stops of the Auto ISO upper limit, from ISO 100. */
#define NATIVE_ISO_STEPS NATIVE_OF(ISO_STEPS)
/* The bits of word 3 that belong to the type. */
#define NATIVE_TYPE_EXTRA_BITS 0x1fc00000U
#define NATIVE_LOOK_BITS 0x3fffU
#define NATIVE_MODE_BITS 0x1fffc000U
_Static_assert(NATIVE_TYPES<31U,"word 2 holds the type + 1 in 5 bits");
/* The portable indexes of the parameters after the levels that a word
   holds: the tone of a type is the one tone parameter that it has. */
#define NATIVE_FIRST_TONE NATIVE_PARAMETER_COLORTONEMONOTONE
#define NATIVE_LAST_TONE NATIVE_PARAMETER_COLORTONECINEMA
/* ColorTemp1, the mode that uses the color temperature. */
#define NATIVE_MODE_TEMPERATURE 14U
#define NATIVE_NAME_BYTES 80U

static const u8 native_type_codes[NATIVE_TYPES]={NATIVE_TYPE_CODES};
static const u16 native_mode_codes[NATIVE_MODES]={NATIVE_MODE_CODES};
static const u16 native_parameter_fields[NATIVE_TYPES*NATIVE_PARAMETERS]={NATIVE_PARAMETER_FIELDS};
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
        out=(out&~(63U<<NATIVE_EV_SHIFT))|(u32)(ev+NATIVE_EV_ZERO)<<NATIVE_EV_SHIFT;
    }
    if(NATIVE_ISO(w)) {
        int n=(int)NATIVE_ISO(w)-1, lower=native_iso_read(record,NATIVE_ISO_LOWER_FIELD);
        if(native_float_steps(WORD(record,NATIVE_ISO_STEP_FIELD),3U)==3) n=native_snap(n,3U);
        if(lower>n && lower<(int)NATIVE_ISO_STEPS) n=lower;
        if(n>=(int)NATIVE_ISO_STEPS) n-=3;
        out=(out&~(63U<<NATIVE_ISO_SHIFT))|(u32)(n+1)<<NATIVE_ISO_SHIFT;
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
/* The field offset of parameter p of type t, or 0. */
static u32 native_field(u32 t,u32 p) { return native_parameter_fields[t*NATIVE_PARAMETERS+p]; }
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
/* The native grain of type t in the record as word-3 bits, or 0xffffffff
   for a value out of the tables. */
static u32 native_grain_read(u32 record,u32 t) {
    u32 particle, strength, size;
    if(!native_field(t,NATIVE_PARAMETER_PARTICLE)) return 0;
    particle=BYTE(record,native_field(t,NATIVE_PARAMETER_PARTICLE));
    if(!native_label(NATIVE_PARAMETER_PARTICLE,particle)) return 0xffffffffU;
    if(!native_grain_sized(t)) return native_grain_particle(particle)<<NATIVE_GRAIN_SHIFT;
    if(!particle) return 0;
    strength=BYTE(record,native_field(t,NATIVE_PARAMETER_PARTICLESTRENGTH));
    size=BYTE(record,native_field(t,NATIVE_PARAMETER_PARTICLESIZE));
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

/* Read the native settings into the record words: the type, its
   parameters, and its native grain in words 1 to 3 when `look` is set, the
   white balance in words 2 and 3, each correction setting in word 3, and
   the exposure settings in word 4.
   A value out of the tables is not held, and GR IV Monochrome holds no
   white balance. */
static void native_read(u32 *w,u32 look) {
    u32 record=native_record(), t, p, v, digits=0, code, mode, kelvin, grain, c, monochrome;
    if(!record) return;
    monochrome=NATIVE_MONOCHROME();
    if(look && !BYTE(record,NATIVE_ACTIVE_SET_FIELD)) {
        code=BYTE(record,NATIVE_TYPE_FIELD);
        for(t=0;t<NATIVE_TYPES && native_type_codes[t]!=code;++t) {}
        if(code!=0xffU && native_type_fits(t,monochrome)) {
            u32 word=(t+1U)<<NATIVE_TYPE_SHIFT, ok=1, tone=native_tone(t);
            for(p=NATIVE_LEVELS;p--;) {
                v=native_field(t,p) ? BYTE(record,native_field(t,p)) : 4U;
                if(v>8U) ok=0;
                digits=digits*9U+native_digit(v);
            }
            if(tone<NATIVE_PARAMETERS) {
                v=BYTE(record,native_field(t,tone));
                if(!native_label(tone,v)) ok=0;
                word|=v<<NATIVE_TONE_SHIFT;
            }
            if(native_field(t,NATIVE_PARAMETER_HDRINTENSE)) {
                v=BYTE(record,native_field(t,NATIVE_PARAMETER_HDRINTENSE));
                if(!native_label(NATIVE_PARAMETER_HDRINTENSE,v)) ok=0;
                word|=v<<NATIVE_INTENSITY_SHIFT;
            }
            if(native_field(t,NATIVE_PARAMETER_FILTEREFFECTCUSTOM)) {
                v=BYTE(record,native_field(t,NATIVE_PARAMETER_FILTEREFFECTCUSTOM));
                if(!native_label(NATIVE_PARAMETER_FILTEREFFECTCUSTOM,v)) ok=0;
                word|=v<<NATIVE_FILTER_SHIFT;
            }
            grain=native_grain_read(record,t);
            if(grain==0xffffffffU) ok=0;
            if(native_field(t,NATIVE_PARAMETER_SOLARIZATIONTYPE)) {
                v=BYTE(record,native_field(t,NATIVE_PARAMETER_SOLARIZATIONTYPE));
                if(!native_label(NATIVE_PARAMETER_SOLARIZATIONTYPE,v)) ok=0;
                else grain|=v<<NATIVE_SOLARIZATION_SHIFT;
            }
            if(ok) { w[1]=digits; w[2]|=word; w[3]|=grain; }
        }
    }
    for(c=0;c<NATIVE_CORRECTIONS;++c) {
        v=BYTE(record,native_correction_fields[c]);
        if(v<native_correction_counts[c]) w[3]|=(v+1U)<<(NATIVE_CORRECTION_SHIFT+3U*c);
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
            w[2]|=(mode+1U)<<NATIVE_MODE_SHIFT|ba<<NATIVE_BA_SHIFT|gm<<NATIVE_GM_SHIFT;
        kelvin=HALF(record,NATIVE_TEMPERATURE_FIELD);
        if(kelvin>=NATIVE_TEMPERATURE_MIN && kelvin<=NATIVE_TEMPERATURE_MAX)
            w[3]|=native_div10(kelvin-NATIVE_TEMPERATURE_MIN);
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
    u32 t=NATIVE_TYPE(w[2]), mode=NATIVE_MODE(w[2]), p, digits=w[1], monochrome=NATIVE_MONOCHROME();
    if(t) {
        --t;
        if(!native_type_fits(t,monochrome)) return 0;
        for(p=0;p<NATIVE_LEVELS;++p) {
            u32 rest=native_div9(digits);
            if(native_field(t,p) && !native_label(p,native_level(digits-9U*rest))) return 0;
            digits=rest;
        }
        if(native_tone(t)<NATIVE_PARAMETERS && !native_label(native_tone(t),NATIVE_TONE(w[2]))) return 0;
        if(native_field(t,NATIVE_PARAMETER_HDRINTENSE) &&
           !native_label(NATIVE_PARAMETER_HDRINTENSE,NATIVE_INTENSITY(w[2]))) return 0;
        if(native_field(t,NATIVE_PARAMETER_FILTEREFFECTCUSTOM) &&
           !native_label(NATIVE_PARAMETER_FILTEREFFECTCUSTOM,NATIVE_FILTER(w[2]))) return 0;
        if(!native_grain_fits(t,NATIVE_GRAIN(w[3]),NATIVE_GRAIN_SIZE(w[3]))) return 0;
        if(native_field(t,NATIVE_PARAMETER_SOLARIZATIONTYPE) ?
           !native_label(NATIVE_PARAMETER_SOLARIZATIONTYPE,NATIVE_SOLARIZATION(w[3])) : NATIVE_SOLARIZATION(w[3])!=0U)
            return 0;
    }
    else if(w[3]&NATIVE_TYPE_EXTRA_BITS) return 0;
    for(p=0;p<NATIVE_CORRECTIONS;++p)
        if(NATIVE_CORRECTION(w[3],p)>native_correction_counts[p]) return 0;
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
        if(native_mode_shifts[mode] && (NATIVE_BA(w[2])>=NATIVE_SHIFTS || NATIVE_GM(w[2])>=NATIVE_SHIFTS))
            return 0;
        if(mode==NATIVE_MODE_TEMPERATURE && NATIVE_TEMPERATURE(w[3])>
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

/* Write the native settings of the words that the camera supports
   (native_supported): the type, its parameters, its native grain, and its
   Solarization type, then the white balance mode, its shifts, and the color
   temperature of ColorTemp1, then each correction setting that is not
   Unset, then each exposure setting that the recipe holds, as the camera
   takes it (native_exposure_fit). */
static void native_write(const volatile u32 *w) {
    u32 t=NATIVE_TYPE(w[2]), mode=NATIVE_MODE(w[2]), p, digits=w[1], record=native_record();
    char name[NATIVE_NAME_BYTES], number[6];
    if(t) {
        --t;
        if(record && BYTE(record,NATIVE_ACTIVE_SET_FIELD)) native_set(NATIVE_ACTIVE_SET_SETTING,NATIVE_MAIN_SET);
#ifdef NATIVE_SET_TYPE
        if(t>=NATIVE_FIRST_MONOCHROME) NATIVE_SET_TYPE(native_type_codes[t]);
        else
#endif
        native_set(NATIVE_TYPE_SETTING,native_type_labels[t]);
        for(p=0;p<NATIVE_LEVELS;++p) {
            u32 rest=native_div9(digits);
            if(native_field(t,p)) native_set_parameter(t,p,native_level(digits-9U*rest));
            digits=rest;
        }
        if(native_tone(t)<NATIVE_PARAMETERS) native_set_parameter(t,native_tone(t),NATIVE_TONE(w[2]));
        if(native_field(t,NATIVE_PARAMETER_HDRINTENSE))
            native_set_parameter(t,NATIVE_PARAMETER_HDRINTENSE,NATIVE_INTENSITY(w[2]));
        if(native_field(t,NATIVE_PARAMETER_FILTEREFFECTCUSTOM))
            native_set_parameter(t,NATIVE_PARAMETER_FILTEREFFECTCUSTOM,NATIVE_FILTER(w[2]));
        if(native_field(t,NATIVE_PARAMETER_PARTICLE)) {
            u32 s=NATIVE_GRAIN(w[3]);
            if(!native_grain_sized(t)) native_set_parameter(t,NATIVE_PARAMETER_PARTICLE,native_grain_particle(s));
            else {
                native_set_parameter(t,NATIVE_PARAMETER_PARTICLE,s!=0);
                if(s) {
                    native_set_parameter(t,NATIVE_PARAMETER_PARTICLESIZE,NATIVE_GRAIN_SIZE(w[3]));
                    native_set_parameter(t,NATIVE_PARAMETER_PARTICLESTRENGTH,s-1U);
                }
            }
        }
        if(native_field(t,NATIVE_PARAMETER_SOLARIZATIONTYPE))
            native_set_parameter(t,NATIVE_PARAMETER_SOLARIZATIONTYPE,NATIVE_SOLARIZATION(w[3]));
    }
    if(mode) {
        --mode;
        native_set(NATIVE_WHITE_BALANCE_SETTING,native_mode_labels[mode]);
        if(native_mode_shifts[mode]) {
            u32 at=native_cat(name,0,NATIVE_WHITE_BALANCE_SETTING);
            at=native_cat(name,at,native_mode_labels[mode]);
            native_cat(name,at,"AdjustBA");
            native_set(name,native_ba_labels[NATIVE_BA(w[2])]);
            name[at+6]='G'; name[at+7]='M';
            native_set(name,native_gm_labels[NATIVE_GM(w[2])]);
        }
        if(mode==NATIVE_MODE_TEMPERATURE) {
            u32 kelvin=NATIVE_TEMPERATURE_MIN+NATIVE_TEMPERATURE(w[3])*NATIVE_TEMPERATURE_STEP, i=5;
            number[5]=0;
            while(i--) { u32 rest=native_div10(kelvin); number[i]=(char)('0'+kelvin-10U*rest); kelvin=rest; }
            native_set(NATIVE_COLOR_TEMPERATURE_SETTING,number[0]=='0' ? number+1 : number);
        }
    }
    for(p=0;p<NATIVE_CORRECTIONS;++p)
        if(NATIVE_CORRECTION(w[3],p))
            native_set(native_correction_names[p],native_correction_labels[p][NATIVE_CORRECTION(w[3],p)-1U]);
    {
        u32 x=native_exposure_fit(record,w[NATIVE_EXPOSURE_WORD]);
        if(NATIVE_EV(x)) {
            native_decimal(name,(int)NATIVE_EV(x)-NATIVE_EV_ZERO,6U);
            native_set(NATIVE_EXPOSURE_SETTING,name);
        }
        if(NATIVE_ISO(x)) {
            native_decimal(name,3*(int)NATIVE_ISO_SV_BASE+(int)NATIVE_ISO(x)-1,3U);
            native_set(NATIVE_ISO_UPPER_SETTING,name);
        }
        if(NATIVE_METERING(x)) native_set(NATIVE_METERING_SETTING,native_metering_labels[NATIVE_METERING(x)-1U]);
        if(NATIVE_ND(x)) native_set(NATIVE_ND_SETTING,native_nd_labels[NATIVE_ND(x)-1U]);
    }
}
#endif
