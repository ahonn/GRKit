#ifndef GRKIT_RECIPE_DETAILS_H
#define GRKIT_RECIPE_DETAILS_H
/* The settings of a recipe as items, for the View page of MENU
   (runtime/menu_recipes.h, runtime/viewfinder.h: view_paint) and the panel
   that ADJ shows at the left while the cursor is on a recipe
   (runtime/modules.h: recipe_details_build, runtime/viewfinder.h:
   panel_paint). Each item is one setting that the recipe holds: its icon,
   its English name, the short value that the panel writes after the icon
   where the icon does not show the value, and the value that the View page
   writes below the name (runtime/services.h: SERVICES_DETAILS). The items
   come in groups, in this order:
   - the look: the effect or the native type, with its name;
   - the levels: the Fn adjustments of the effect, or the levels of the
     native type, that are not 0, such as Saturation +1, then the tone,
     HDR, filter, grain, and solarization of the native type that are not
     Off;
   - the modules: Grain, Bloom, Halation, and Border that are not Off, whose
     icons are their status icons for these values (runtime/modules.h
     resolves them);
   - the white balance: the mode, with its shifts or its color temperature;
   - the corrections: highlight, shadow, peripheral illumination, and
     high-ISO noise reduction, whose icons show their values;
   - the exposure settings: the compensation, the Auto ISO upper limit, the
     metering, and the ND filter.
   The words are English, the same in each menu language: the live view
   writes only the glyphs of printable ASCII. The native icons are those of
   the camera's own menus, from the firmware icon table
   (NATIVE_DETAIL_*_ICONS of the build header), 0 where the camera has none.

   The includer defines u8, u16, u32, BYTE, HALF, and WORD, and includes
   runtime/services.h, runtime/native_look.h, and runtime/recipes.h. */
#include "recipe_detail_names.h"
/* The icon ID of effect record n, as runtime/core.h has it. */
#ifndef GRKIT_ICON_EFFECT
#define GRKIT_ICON_EFFECT(n) (0x8000U+(n))
#endif

#if !defined(NATIVE_DETAIL_MODE_ICONS) || !defined(NATIVE_DETAIL_PARAMETER_ICONS) || \
    !defined(NATIVE_DETAIL_CORRECTION_ICONS) || !defined(NATIVE_DETAIL_METERING_ICONS) || \
    !defined(NATIVE_DETAIL_ND_ICONS) || !defined(NATIVE_DETAIL_COMPENSATION_ICON) || !defined(NATIVE_DETAIL_ISO_ICON)
#error "The build header gives the native icons of the settings of a recipe"
#endif
static const u16 detail_mode_icons[NATIVE_MODES]={NATIVE_DETAIL_MODE_ICONS};
static const u16 detail_parameter_icons[NATIVE_PARAMETERS]={NATIVE_DETAIL_PARAMETER_ICONS};
/* The icon of each stored value of each correction, the corrections one
   after the other. */
static const u16 detail_correction_icons[]={NATIVE_DETAIL_CORRECTION_ICONS};
static const u16 detail_metering_icons[]={NATIVE_DETAIL_METERING_ICONS};
static const u16 detail_nd_icons[]={NATIVE_DETAIL_ND_ICONS};
static const u16 detail_type_icons[NATIVE_TYPES]={NATIVE_TYPE_ICONS};

static const char *const detail_strengths[4]={"Off","Weak","Medium","Strong"};
static const char *const detail_halation_colors[4]={"Red","Orange","Gold","White"};
static const char *const detail_borders[4]={"Off","Thin","Wide","Instant"};
static const char *const detail_border_colors[2]={"White","Black"};
/* Grain (modules/grain/text/en.toml): its levels and its sizes. */
static const char *const detail_grains[5]={"Off","Weak","Medium","Strong","Extra Strong"};
static const char *const detail_grain_sizes[4]={"Auto","Small","Medium","Large"};
/* The white balance modes, in the portable order of runtime/native.h. */
static const char *const detail_modes[NATIVE_MODES]={
    "Auto","Multi Auto","Auto Warm","Auto White","Daylight","Shade","Cloudy","Fluorescent D","Fluorescent N",
    "Fluorescent W","Fluorescent L","Tungsten","CTE","Manual",0,"Custom 1","Custom 2","Custom 3"};
static const char *const detail_meterings[4]={"Multi-segment","Center-weighted","Spot","Highlight-weighted"};
static const char *const detail_native_grains[4]={0,"Low","Mid","High"};
static const char *const detail_native_grain_sizes[3]={"Small","Medium","Large"};
static const char *const detail_type_names[NATIVE_TYPES]={NATIVE_TYPE_NAMES};
static const u32 detail_iso_values[]={NATIVE_ISO_VALUES};
_Static_assert(sizeof(detail_iso_values)/sizeof(detail_iso_values[0])>=NATIVE_GR3_ISO_STEPS &&
               sizeof(detail_iso_values)/sizeof(detail_iso_values[0])>=NATIVE_GR4_ISO_STEPS,
               "an ISO for each third stop of the Auto ISO limit");
_Static_assert(GRKIT_RECIPE_SETTINGS==5U,"the details name each module setting of a recipe");
_Static_assert(DETAIL_SHORT_UNITS<=DETAIL_VALUE_UNITS,"a short value fits the value");

/* An item while it is written. */
typedef struct {
    u32 icon, meta;
    u16 value[DETAIL_VALUE_UNITS];
    u32 n;
    u16 shorter[DETAIL_SHORT_UNITS];
    u32 m;
} DetailItem;
/* The items of a recipe while they are written into `base`, the details
   block of the services. */
typedef struct {
    volatile u32 *base;
    u32 n;
} DetailList;

static void detail_start(DetailItem *item,u32 group,u32 name,u32 icon) {
    item->icon=icon; item->meta=group|name<<8; item->n=0; item->m=0;
}
/* Add ASCII text to the value, or with `both` to the short value too. */
static void detail_add(DetailItem *item,const char *text) {
    while(*text && item->n+1U<DETAIL_VALUE_UNITS) item->value[item->n++]=(u16)(u8)*text++;
}
static void detail_add_short(DetailItem *item,const char *text) {
    while(*text && item->m+1U<DETAIL_SHORT_UNITS) item->shorter[item->m++]=(u16)(u8)*text++;
}
static void detail_both(DetailItem *item,const char *text) { detail_add(item,text); detail_add_short(item,text); }
/* The decimal digits of n. */
static void detail_digits(char *to,u32 n) {
    char digits[10];
    u32 i=0, rest, k=0;
    do { rest=native_div10(n); digits[i++]=(char)('0'+n-10U*rest); n=rest; } while(n && i<10U);
    while(i) to[k++]=digits[--i];
    to[k]=0;
}
/* A signed level, such as +1, -2, or 0. */
static void detail_signed(char *to,int n) {
    if(!n) { to[0]='0'; to[1]=0; return; }
    to[0]=n<0 ? '-' : '+';
    detail_digits(to+1,(u32)(n<0 ? -n : n));
}
/* A native label: P2 and M2 as +2 and -2, OFF as Off, other labels as
   they are. 0 for no label, and with `off` 0 for Off too. */
static u32 detail_label(char *to,const char *label,u32 off) {
    u32 i=0;
    if(!label || !label[0]) return 0;
    if(label[0]=='O' && (label[1]=='F' || label[1]=='f') && (label[2]=='F' || label[2]=='f') && !label[3]) {
        if(!off) return 0;
        label="Off";
    }
    if((label[0]=='P' || label[0]=='M') && label[1]>='0' && label[1]<='9') { to[i++]=label[0]=='P' ? '+' : '-'; ++label; }
    while(*label && i<15U) to[i++]=*label++;
    to[i]=0;
    return 1;
}
/* Write the item into the list. Items after DETAIL_ITEMS_MAX are left out. */
static void detail_put(DetailList *list,const DetailItem *item) {
    volatile u32 *at;
    volatile u16 *text;
    u32 i;
    if(list->n>=DETAIL_ITEMS_MAX) return;
    at=list->base+(DETAIL_ITEMS+list->n*DETAIL_ITEM_SIZE)/4U;
    at[DETAIL_ITEM_ICON/4]=item->icon;
    at[DETAIL_ITEM_META/4]=item->meta;
    text=(volatile u16 *)(at+DETAIL_ITEM_SHORT/4);
    for(i=0;i<item->m;++i) text[i]=item->shorter[i];
    text[i]=0;
    text=(volatile u16 *)(at+DETAIL_ITEM_VALUE/4);
    for(i=0;i<item->n;++i) text[i]=item->value[i];
    text[i]=0;
    ++list->n;
}

/* The look: the effect with its name as its record has it, the native type
   with its English name, or Film Look Off; nothing while the recipe keeps
   the look. */
static void detail_look(volatile u32 *h,DetailList *list,const u32 *w) {
    DetailItem item;
    u32 type=NATIVE_TYPE(w[RECIPE_NATIVE_WORD]), index, record, i, unit;
    if(RECIPE_LOOK(w[0])==RECIPE_LOOK_EFFECT) {
        index=recipe_effect(h,RECIPE_CODE(w[0]));
        detail_start(&item,DETAIL_GROUP_LOOK,DETAIL_NAME_FILM_LOOK,0);
        if(index>=GRKIT_TEXT_RECORDS) detail_add(&item,"Effect missing");
        else {
            item.icon=GRKIT_ICON_EFFECT(index);
            record=recipe_record(h,index);
            for(i=0;i+1U<DETAIL_VALUE_UNITS && (unit=HALF(record,GRKIT_RECORD_NAME+2U*i));++i) item.value[item.n++]=(u16)unit;
        }
    }
    else if(type && type<=NATIVE_TYPES) {
        detail_start(&item,DETAIL_GROUP_LOOK,DETAIL_NAME_IMAGE_CONTROL,detail_type_icons[type-1U]);
        detail_add(&item,detail_type_names[type-1U]);
    }
    else if(RECIPE_LOOK(w[0])==RECIPE_LOOK_OFF) {
        detail_start(&item,DETAIL_GROUP_LOOK,DETAIL_NAME_FILM_LOOK,0);
        detail_add(&item,"Off");
    }
    else return;
    detail_put(list,&item);
}
/* A level or a native parameter with a word value: its icon, its name, and
   its value as the short value too, as the icon does not show it. */
static void detail_parameter(DetailList *list,u32 p,u32 name,const char *value) {
    DetailItem item;
    detail_start(&item,DETAIL_GROUP_LEVELS,name,detail_parameter_icons[p]);
    detail_both(&item,value);
    detail_put(list,&item);
}
/* The Fn adjustments or the levels that are not 0: base-9 digits, the
   first lowest, each (value - 4) mod 9 (runtime/recipes.h). An effect shows
   each adjustment that differs from its Fn default, as the Fn page shows it:
   the default plus the adjustment. A native type shows only its own
   parameters. */
static void detail_levels_of(volatile u32 *h,DetailList *list,const u32 *w) {
    u32 digits=w[RECIPE_FN_WORD], p, rest, digit, type=NATIVE_TYPE(w[RECIPE_NATIVE_WORD]), record=0, index, shown;
    u32 effect=RECIPE_LOOK(w[0])==RECIPE_LOOK_EFFECT;
    char value[12];
    if(effect) {
        index=recipe_effect(h,RECIPE_CODE(w[0]));
        if(index<GRKIT_TEXT_RECORDS) record=recipe_record(h,index);
    }
    else if(!(type && type<=NATIVE_TYPES)) return;
    for(p=0;p<NATIVE_LEVELS;++p) {
        rest=native_div9(digits);
        digit=digits-9U*rest;
        digits=rest;
        if(!digit || (!effect && !native_field(type-1U,p))) continue;
        shown=digit;
        if(record) {
            shown+=BYTE(record,GRKIT_RECORD_FN_DEFAULTS+p);
            if(shown>=9U) shown-=9U;
        }
        detail_signed(value,shown<=4U ? (int)shown : (int)shown-9);
        detail_parameter(list,p,DETAIL_NAME_LEVEL(p),value);
    }
}
/* The tone, HDR, filter, grain, and solarization of the native type. */
static void detail_native(DetailList *list,const u32 *w) {
    u32 type=NATIVE_TYPE(w[RECIPE_NATIVE_WORD]), t, p, s, k;
    char value[16];
    if(RECIPE_LOOK(w[0])==RECIPE_LOOK_EFFECT || !type || type>NATIVE_TYPES) return;
    t=type-1U;
    p=native_tone(t);
    if(p<NATIVE_PARAMETERS && detail_label(value,native_label(p,NATIVE_TONE(w[RECIPE_NATIVE_WORD])),0))
        detail_parameter(list,p,DETAIL_NAME_TONING,value);
    if(native_field(t,NATIVE_PARAMETER_HDRINTENSE) &&
       detail_label(value,native_label(NATIVE_PARAMETER_HDRINTENSE,NATIVE_INTENSITY(w[RECIPE_NATIVE_WORD])),0))
        detail_parameter(list,NATIVE_PARAMETER_HDRINTENSE,DETAIL_NAME_HDR,value);
    if(native_field(t,NATIVE_PARAMETER_FILTEREFFECTCUSTOM) &&
       detail_label(value,native_label(NATIVE_PARAMETER_FILTEREFFECTCUSTOM,NATIVE_FILTER(w[RECIPE_NATIVE_WORD])),0))
        detail_parameter(list,NATIVE_PARAMETER_FILTEREFFECTCUSTOM,DETAIL_NAME_FILTER,value);
    s=NATIVE_GRAIN(w[RECIPE_EXTRA_WORD]);
    if(s && s<4U) {
        for(k=0;detail_native_grains[s][k];++k) value[k]=detail_native_grains[s][k];
        value[k]=0;
        detail_parameter(list,NATIVE_PARAMETER_PARTICLE,DETAIL_NAME_GRAIN_EFFECT,value);
    }
    if(native_field(t,NATIVE_PARAMETER_SOLARIZATIONTYPE) &&
       detail_label(value,native_label(NATIVE_PARAMETER_SOLARIZATIONTYPE,NATIVE_SOLARIZATION(w[RECIPE_EXTRA_WORD])),0))
        detail_parameter(list,NATIVE_PARAMETER_SOLARIZATIONTYPE,DETAIL_NAME_SOLARIZATION,value);
}
/* A module setting that is not Off: its name and value, and in the meta
   word what its status icon needs. */
static void detail_module(DetailList *list,u32 name,u32 setting,u32 value,const char *text,const char *more) {
    DetailItem item;
    detail_start(&item,DETAIL_GROUP_MODULES,name,0);
    item.meta|=setting<<16|value<<24;
    detail_add(&item,text);
    if(more) { detail_add(&item,", "); detail_add(&item,more); }
    detail_put(list,&item);
}
/* Grain, Bloom, Halation, and Border, as the recipe holds them. */
static void detail_modules(DetailList *list,const u32 *w) {
    u32 level, v;
    char size[16];
    if(w[0]&RECIPE_GRAIN) {
        level=RECIPE_GRAIN_LEVEL(w[0]);
        if(level && level<5U) {
            const char *s="size ", *n=detail_grain_sizes[RECIPE_GRAIN_SIZE(w[0])&3U];
            u32 k=0;
            while(*s) size[k++]=*s++;
            while(*n) size[k++]=*n++;
            size[k]=0;
            detail_module(list,DETAIL_NAME_GRAIN,DETAIL_GRAIN,level,detail_grains[level],size);
        }
    }
    if(recipe_setting_held(w,0) && (v=recipe_setting_value(w,0)) && v<4U)
        detail_module(list,DETAIL_NAME_BLOOM,1U,v,detail_strengths[v],0);
    if(recipe_setting_held(w,1) && (v=recipe_setting_value(w,1)) && v<4U)
        detail_module(list,DETAIL_NAME_HALATION,2U,v,detail_strengths[v],
                      recipe_setting_held(w,2) && recipe_setting_value(w,2)<4U ? detail_halation_colors[recipe_setting_value(w,2)] : 0);
    if(recipe_setting_held(w,3) && (v=recipe_setting_value(w,3)) && v<4U)
        detail_module(list,DETAIL_NAME_BORDER,4U,v,detail_borders[v],
                      recipe_setting_held(w,4) && recipe_setting_value(w,4)<2U ? detail_border_colors[recipe_setting_value(w,4)] : 0);
}
/* The white balance: the mode with its shifts, or the color temperature.
   The icon shows the mode; the short value gives the shifts, such as A2 G1,
   or the temperature. */
static void detail_white_balance(DetailList *list,const u32 *w) {
    u32 field=NATIVE_MODE(w[RECIPE_NATIVE_WORD]), mode, ba, gm, shifted=0;
    char number[12];
    DetailItem item;
    if(!field || field>NATIVE_MODES) return;
    mode=field-1U;
    detail_start(&item,DETAIL_GROUP_WHITE_BALANCE,DETAIL_NAME_WHITE_BALANCE,detail_mode_icons[mode]);
    if(mode==NATIVE_MODE_TEMPERATURE) {
        detail_digits(number,NATIVE_TEMPERATURE_MIN+NATIVE_TEMPERATURE_STEP*NATIVE_TEMPERATURE(w[RECIPE_EXTRA_WORD]));
        detail_both(&item,number);
        detail_both(&item,"K");
        shifted=1;
    }
    else if(detail_modes[mode]) detail_add(&item,detail_modes[mode]);
    else return;
    ba=NATIVE_BA(w[RECIPE_NATIVE_WORD]);
    gm=NATIVE_GM(w[RECIPE_NATIVE_WORD]);
    if(native_mode_shifts[mode] && ba<NATIVE_SHIFTS && ba!=14U) {
        detail_add(&item,shifted ? " " : ", ");
        if(item.m) detail_add_short(&item," ");
        detail_both(&item,ba<14U ? "B" : "A");
        detail_digits(number,ba<14U ? 14U-ba : ba-14U);
        detail_both(&item,number);
        shifted=1;
    }
    if(native_mode_shifts[mode] && gm<NATIVE_SHIFTS && gm!=14U) {
        detail_add(&item,shifted ? " " : ", ");
        if(item.m) detail_add_short(&item," ");
        detail_both(&item,gm<14U ? "G" : "M");
        detail_digits(number,gm<14U ? 14U-gm : gm-14U);
        detail_both(&item,number);
    }
    detail_put(list,&item);
}
/* The corrections that the recipe holds: each stored value + 1, 0 Unset.
   The icon of the value shows it. */
static void detail_corrections_of(DetailList *list,const u32 *w) {
    u32 c, v, first=0;
    char value[16];
    DetailItem item;
    for(c=0;c<NATIVE_CORRECTIONS;first+=native_correction_counts[c],++c) {
        v=NATIVE_CORRECTION(w[RECIPE_EXTRA_WORD],c);
        if(!v || v>native_correction_counts[c] || !detail_label(value,native_correction_labels[c][v-1U],1)) continue;
        detail_start(&item,DETAIL_GROUP_CORRECTIONS,DETAIL_NAME_CORRECTION(c),detail_correction_icons[first+v-1U]);
        detail_add(&item,value);
        detail_put(list,&item);
    }
}
/* The exposure settings that the recipe holds: the compensation in sixths
   of a stop, written to a tenth; the Auto ISO upper limit; the metering;
   and the ND filter. */
static void detail_exposure(DetailList *list,const u32 *w) {
    u32 x=w[NATIVE_EXPOSURE_WORD], ev=NATIVE_EV(x), iso=NATIVE_ISO(x), v;
    char number[12];
    DetailItem item;
    if(ev) {
        int sixths=(int)ev-NATIVE_EV_ZERO;
        /* The tenths, rounded: (a + 3) / 6 as a product with ceil(2^18 / 6),
           exact for these small numbers. The core links no division helper,
           and a compiler makes a loop of subtractions a division. */
        u32 a=(u32)(sixths<0 ? -sixths : sixths)*10U+3U, tenths=a*43691U>>18;
        detail_start(&item,DETAIL_GROUP_EXPOSURE,DETAIL_NAME_COMPENSATION,NATIVE_DETAIL_COMPENSATION_ICON);
        if(tenths) detail_both(&item,sixths<0 ? "-" : "+");
        detail_digits(number,native_div10(tenths));
        detail_both(&item,number);
        detail_both(&item,".");
        detail_digits(number,tenths-10U*native_div10(tenths));
        detail_both(&item,number);
        detail_add(&item," EV");
        detail_put(list,&item);
    }
    if(iso && iso<=NATIVE_ISO_STEPS) {
        detail_start(&item,DETAIL_GROUP_EXPOSURE,DETAIL_NAME_ISO,NATIVE_DETAIL_ISO_ICON);
        detail_add(&item,"ISO ");
        detail_digits(number,detail_iso_values[iso-1U]);
        detail_both(&item,number);
        detail_put(list,&item);
    }
    v=NATIVE_METERING(x);
    if(v && v<=4U) {
        detail_start(&item,DETAIL_GROUP_EXPOSURE,DETAIL_NAME_METERING,detail_metering_icons[v-1U]);
        detail_add(&item,detail_meterings[v-1U]);
        detail_put(list,&item);
    }
    v=NATIVE_ND(x);
    if(v && v<=NATIVE_COUNT(native_nd_labels)) {
        detail_start(&item,DETAIL_GROUP_EXPOSURE,DETAIL_NAME_ND,detail_nd_icons[v-1U]);
        detail_add(&item,native_nd_labels[v-1U]);
        detail_put(list,&item);
    }
}

/* Write the items of the recipe of words `w` into the details block `base`
   and give their number. The items of the modules have no icon yet. */
static u32 recipe_details(volatile u32 *h,const u32 *w,volatile u32 *base) {
    DetailList list;
    list.base=base; list.n=0;
    detail_look(h,&list,w);
    detail_levels_of(h,&list,w);
    detail_native(&list,w);
    detail_modules(&list,w);
    detail_white_balance(&list,w);
    detail_corrections_of(&list,w);
    detail_exposure(&list,w);
    base[DETAIL_COUNT/4]=list.n;
    return list.n;
}
/* Item i of the details block `base`. */
static volatile u32 *detail_item(volatile u32 *base,u32 i) { return DETAIL_ITEM_AT(base,i); }
#endif
