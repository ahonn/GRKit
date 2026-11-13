#ifndef GRKIT_RECIPE_DETAILS_H
#define GRKIT_RECIPE_DETAILS_H
/* The settings of a recipe as short rows of text, for the View page of MENU
   (runtime/menu_recipes.h) and the panel that ADJ shows at the left while
   the cursor is on a recipe (runtime/modules.h, runtime/viewfinder.h).

   The rows hold only what the recipe holds (runtime/recipes.h), in this
   order, each group from a new row:
   - the look: the name of the effect, the English name of the native type,
     or Film Look Off; nothing while the recipe keeps the look;
   - the Fn adjustments of the effect, or the levels of the native type,
     that are not 0, such as "Sat +1  Con -1";
   - the tone, HDR, filter, grain, and solarization of the native type;
   - Grain, Bloom, Halation, and Border (not in the panel, which shows them
     as their status icons);
   - the white balance, such as "WB Auto A2 G1" or "WB 5200K";
   - the corrections, such as "HL Auto  SH Off  Periph Off  NR Low";
   - the exposure settings, such as "EV +0.7  ISO max 3200  Meter Spot".
   The words are English abbreviations, the same in each menu language: the
   live view can write only the glyphs of printable ASCII. A group wraps to
   a new row before an item that would pass DETAIL_WRAP characters.

   The rows are DETAIL_ROWS_MAX rows of DETAIL_UNITS UTF-16 units, each
   NUL-terminated (runtime/services.h: SERVICES_DETAILS). Only the name of an
   effect can hold other characters than printable ASCII.

   The includer defines u8, u16, u32, BYTE, HALF, and WORD, and includes
   runtime/services.h, runtime/native_look.h, and runtime/recipes.h. */

#define DETAIL_WRAP 24U
/* The rows of a mode: all settings, or those that the panel writes. */
#define DETAIL_MENU 0U
#define DETAIL_PANEL 1U
_Static_assert(DETAIL_WRAP<DETAIL_UNITS,"a row holds the longest item and its NUL");

typedef struct {
    volatile u16 *rows;
    u32 n;
    u32 at;
} DetailText;

/* The names of the module settings that a recipe holds, in the order of
   GRKIT_RECIPE_SETTING_IDS: Bloom, Halation, the Halation color, Border,
   and the Border color, with the values of their module text. */
_Static_assert(GRKIT_RECIPE_SETTINGS==5U,"the details name each module setting of a recipe");
static const char *const detail_strengths[4]={"Off","Weak","Medium","Strong"};
static const char *const detail_halation_colors[4]={"Red","Orange","Gold","White"};
static const char *const detail_borders[4]={"Off","Thin","Wide","Instant"};
static const char *const detail_border_colors[2]={"White","Black"};
/* Grain (modules/grain/text/en.toml): its levels and its sizes. */
static const char *const detail_grains[5]={"Off","Weak","Medium","Strong","Extra Strong"};
static const char *const detail_grain_sizes[4]={"Auto","Small","Medium","Large"};
/* The Fn adjustments of an effect and the levels of a native type, in the
   order of NATIVE_PARAMETER_SATURATION to NATIVE_PARAMETER_CLARITYCONTROL. */
static const char *const detail_levels[NATIVE_LEVELS]={"Sat","Hue","Key","Con","HL Con","SH Con","Sharp","Shade","Clarity"};
/* The white balance modes, in the portable order of runtime/native.h. */
static const char *const detail_modes[NATIVE_MODES]={
    "Auto","Multi Auto","Auto Warm","Auto White","Daylight","Shade","Cloudy","Fluor D","Fluor N",
    "Fluor W","Fluor L","Tungsten","CTE","Manual",0,"Custom 1","Custom 2","Custom 3"};
/* The corrections: highlight, shadow, peripheral illumination, and high-ISO
   noise reduction. */
static const char *const detail_corrections[NATIVE_CORRECTIONS]={"HL","SH","Periph","NR"};
static const char *const detail_native_grains[4]={0,"Low","Mid","High"};
static const char *const detail_native_grain_sizes[3]={"Small","Medium","Large"};
static const char *const detail_type_names[NATIVE_TYPES]={NATIVE_TYPE_NAMES};
static const u32 detail_iso_values[]={NATIVE_ISO_VALUES};
_Static_assert(sizeof(detail_iso_values)/sizeof(detail_iso_values[0])>=NATIVE_GR3_ISO_STEPS &&
               sizeof(detail_iso_values)/sizeof(detail_iso_values[0])>=NATIVE_GR4_ISO_STEPS,
               "an ISO for each third stop of the Auto ISO limit");

/* An item: its characters and their number. */
typedef struct {
    char text[DETAIL_UNITS];
    u32 n;
} DetailItem;
static void detail_add(DetailItem *item,const char *text) {
    while(*text && item->n+1U<DETAIL_UNITS) item->text[item->n++]=*text++;
}
/* Add n in decimal. */
static void detail_number(DetailItem *item,u32 n) {
    char digits[10];
    u32 i=0, rest;
    do { rest=native_div10(n); digits[i++]=(char)('0'+n-10U*rest); n=rest; } while(n && i<10U);
    while(i && item->n+1U<DETAIL_UNITS) item->text[item->n++]=digits[--i];
}
/* Add a signed level, such as +1 or -2. */
static void detail_signed(DetailItem *item,int n) {
    detail_add(item,n<0 ? "-" : "+");
    detail_number(item,(u32)(n<0 ? -n : n));
}
/* Whether a label is OFF or Off. */
static u32 detail_off(const char *label) {
    return (label[0]=='O' && (label[1]=='F' || label[1]=='f') && (label[2]=='F' || label[2]=='f') && !label[3]);
}
/* Add a native label: P2 and M2 as +2 and -2, other labels as they are.
   0 for OFF, Off, and no label, which leave the item out. */
static u32 detail_label(DetailItem *item,const char *label) {
    if(!label || !label[0] || detail_off(label)) return 0;
    if((label[0]=='P' || label[0]=='M') && label[1]>='0' && label[1]<='9') {
        detail_add(item,label[0]=='P' ? "+" : "-");
        detail_add(item,label+1);
    } else detail_add(item,label);
    return 1;
}

/* End the row that has text. */
static void detail_break(DetailText *d) {
    if(!d->at) return;
    ++d->n;
    d->at=0;
}
/* Put an item on the row, after two spaces, or on a new row when it would
   pass DETAIL_WRAP. Rows after DETAIL_ROWS_MAX are left out. */
static void detail_put(DetailText *d,const DetailItem *item) {
    volatile u16 *row;
    u32 i;
    if(!item->n) return;
    if(d->at && d->at+2U+item->n>DETAIL_WRAP) detail_break(d);
    if(d->n>=DETAIL_ROWS_MAX) return;
    row=d->rows+d->n*DETAIL_UNITS;
    if(d->at) { row[d->at++]=' '; row[d->at++]=' '; }
    for(i=0;i<item->n && d->at+1U<DETAIL_UNITS;++i) row[d->at++]=(u16)(u8)item->text[i];
    row[d->at]=0;
}
/* Put a name and a value from a table of names, or nothing for a value
   past the table. */
static void detail_named(DetailText *d,const char *name,const char *const *values,u32 count,u32 value) {
    DetailItem item;
    item.n=0;
    if(value>=count || !values[value]) return;
    detail_add(&item,name);
    detail_add(&item," ");
    detail_add(&item,values[value]);
    detail_put(d,&item);
}

/* The look: the name of an effect, as its record has it, or an ASCII name. */
static void detail_look(volatile u32 *h,DetailText *d,const u32 *w) {
    DetailItem item;
    item.n=0;
    u32 type=NATIVE_TYPE(w[RECIPE_NATIVE_WORD]), index, i;
    volatile u16 *row;
    if(RECIPE_LOOK(w[0])==RECIPE_LOOK_EFFECT) {
        index=recipe_effect(h,RECIPE_CODE(w[0]));
        if(index>=GRKIT_TEXT_RECORDS) detail_add(&item,"Effect missing");
        else {
            u32 record=recipe_record(h,index);
            if(d->n>=DETAIL_ROWS_MAX) return;
            row=d->rows+d->n*DETAIL_UNITS;
            for(i=0;i+1U<DETAIL_UNITS && HALF(record,GRKIT_RECORD_NAME+2U*i);++i) row[i]=HALF(record,GRKIT_RECORD_NAME+2U*i);
            row[i]=0;
            d->at=i;
            detail_break(d);
            return;
        }
    }
    else if(type && type<=NATIVE_TYPES) detail_add(&item,detail_type_names[type-1U]);
    else if(RECIPE_LOOK(w[0])==RECIPE_LOOK_OFF) detail_add(&item,"Film Look Off");
    detail_put(d,&item);
    detail_break(d);
}
/* The Fn adjustments or the levels that are not 0: base-9 digits, the
   first lowest, each (value - 4) mod 9 (runtime/recipes.h). A native type
   shows only its own parameters. */
static void detail_levels_of(DetailText *d,const u32 *w) {
    u32 digits=w[RECIPE_FN_WORD], p, rest, digit;
    u32 type=NATIVE_TYPE(w[RECIPE_NATIVE_WORD]), effect=RECIPE_LOOK(w[0])==RECIPE_LOOK_EFFECT;
    if(!effect && !(type && type<=NATIVE_TYPES)) return;
    for(p=0;p<NATIVE_LEVELS;++p) {
        DetailItem item;
    item.n=0;
        rest=native_div9(digits);
        digit=digits-9U*rest;
        digits=rest;
        if(!digit || (!effect && !native_field(type-1U,p))) continue;
        detail_add(&item,detail_levels[p]);
        detail_add(&item," ");
        detail_signed(&item,digit<=4U ? (int)digit : (int)digit-9);
        detail_put(d,&item);
    }
    detail_break(d);
}
/* The tone, HDR, filter, grain, and solarization of the native type. */
static void detail_native(DetailText *d,const u32 *w) {
    u32 type=NATIVE_TYPE(w[RECIPE_NATIVE_WORD]), t, p, s;
    DetailItem item;
    if(RECIPE_LOOK(w[0])==RECIPE_LOOK_EFFECT || !type || type>NATIVE_TYPES) return;
    t=type-1U;
    p=native_tone(t);
    item.n=0;
    detail_add(&item,"Tone ");
    if(p<NATIVE_PARAMETERS && detail_label(&item,native_label(p,NATIVE_TONE(w[RECIPE_NATIVE_WORD])))) detail_put(d,&item);
    item.n=0;
    detail_add(&item,"HDR ");
    if(native_field(t,NATIVE_PARAMETER_HDRINTENSE) &&
       detail_label(&item,native_label(NATIVE_PARAMETER_HDRINTENSE,NATIVE_INTENSITY(w[RECIPE_NATIVE_WORD])))) detail_put(d,&item);
    item.n=0;
    detail_add(&item,"Filter ");
    if(native_field(t,NATIVE_PARAMETER_FILTEREFFECTCUSTOM) &&
       detail_label(&item,native_label(NATIVE_PARAMETER_FILTEREFFECTCUSTOM,NATIVE_FILTER(w[RECIPE_NATIVE_WORD])))) detail_put(d,&item);
    s=NATIVE_GRAIN(w[RECIPE_EXTRA_WORD]);
    if(s && s<4U) {
        item.n=0;
        detail_add(&item,"IC Grain ");
        detail_add(&item,detail_native_grains[s]);
        if(native_grain_sized(t) && NATIVE_GRAIN_SIZE(w[RECIPE_EXTRA_WORD])<3U) {
            detail_add(&item," ");
            detail_add(&item,detail_native_grain_sizes[NATIVE_GRAIN_SIZE(w[RECIPE_EXTRA_WORD])]);
        }
        detail_put(d,&item);
    }
    item.n=0;
    detail_add(&item,"Solar ");
    if(native_field(t,NATIVE_PARAMETER_SOLARIZATIONTYPE) &&
       detail_label(&item,native_label(NATIVE_PARAMETER_SOLARIZATIONTYPE,NATIVE_SOLARIZATION(w[RECIPE_EXTRA_WORD])))) detail_put(d,&item);
    detail_break(d);
}
/* Grain, Bloom, Halation, and Border, as the recipe holds them. */
static void detail_modules(DetailText *d,const u32 *w) {
    DetailItem item;
    item.n=0;
    u32 level;
    if(w[0]&RECIPE_GRAIN) {
        level=RECIPE_GRAIN_LEVEL(w[0]);
        if(level<5U) {
            detail_add(&item,"Grain ");
            detail_add(&item,detail_grains[level]);
            if(level && RECIPE_GRAIN_SIZE(w[0])) {
                detail_add(&item," ");
                detail_add(&item,detail_grain_sizes[RECIPE_GRAIN_SIZE(w[0])]);
            }
            detail_put(d,&item);
        }
    }
    if(recipe_setting_held(w,0)) detail_named(d,"Bloom",detail_strengths,4U,recipe_setting_value(w,0));
    if(recipe_setting_held(w,1) && recipe_setting_value(w,1)<4U) {
        item.n=0;
        detail_add(&item,"Halation ");
        detail_add(&item,detail_strengths[recipe_setting_value(w,1)]);
        if(recipe_setting_value(w,1) && recipe_setting_held(w,2) && recipe_setting_value(w,2)<4U) {
            detail_add(&item," ");
            detail_add(&item,detail_halation_colors[recipe_setting_value(w,2)]);
        }
        detail_put(d,&item);
    }
    if(recipe_setting_held(w,3) && recipe_setting_value(w,3)<4U) {
        item.n=0;
        detail_add(&item,"Border ");
        detail_add(&item,detail_borders[recipe_setting_value(w,3)]);
        if(recipe_setting_value(w,3) && recipe_setting_held(w,4) && recipe_setting_value(w,4)<2U) {
            detail_add(&item," ");
            detail_add(&item,detail_border_colors[recipe_setting_value(w,4)]);
        }
        detail_put(d,&item);
    }
    detail_break(d);
}
/* The white balance: the mode with its shifts, or the color temperature. */
static void detail_white_balance(DetailText *d,const u32 *w) {
    u32 field=NATIVE_MODE(w[RECIPE_NATIVE_WORD]), mode, ba, gm;
    DetailItem item;
    item.n=0;
    if(!field || field>NATIVE_MODES) return;
    mode=field-1U;
    detail_add(&item,"WB ");
    if(mode==NATIVE_MODE_TEMPERATURE) {
        detail_number(&item,NATIVE_TEMPERATURE_MIN+NATIVE_TEMPERATURE_STEP*NATIVE_TEMPERATURE(w[RECIPE_EXTRA_WORD]));
        detail_add(&item,"K");
    }
    else if(detail_modes[mode]) detail_add(&item,detail_modes[mode]);
    else return;
    ba=NATIVE_BA(w[RECIPE_NATIVE_WORD]);
    gm=NATIVE_GM(w[RECIPE_NATIVE_WORD]);
    if(native_mode_shifts[mode] && ba<NATIVE_SHIFTS && ba!=14U) {
        detail_add(&item,ba<14U ? " B" : " A");
        detail_number(&item,ba<14U ? 14U-ba : ba-14U);
    }
    if(native_mode_shifts[mode] && gm<NATIVE_SHIFTS && gm!=14U) {
        detail_add(&item,gm<14U ? " G" : " M");
        detail_number(&item,gm<14U ? 14U-gm : gm-14U);
    }
    detail_put(d,&item);
    detail_break(d);
}
/* The corrections that the recipe holds: each stored value + 1, 0 Unset. */
static void detail_corrections_of(DetailText *d,const u32 *w) {
    u32 c, v;
    for(c=0;c<NATIVE_CORRECTIONS;++c) {
        v=NATIVE_CORRECTION(w[RECIPE_EXTRA_WORD],c);
        if(v) detail_named(d,detail_corrections[c],native_correction_labels[c],native_correction_counts[c],v-1U);
    }
    detail_break(d);
}
/* The exposure settings that the recipe holds: the compensation in sixths
   of a stop, written to a tenth; the Auto ISO upper limit; the metering;
   and the ND filter. */
static void detail_exposure(DetailText *d,const u32 *w) {
    u32 x=w[NATIVE_EXPOSURE_WORD], ev=NATIVE_EV(x), iso=NATIVE_ISO(x), v;
    DetailItem item;
    item.n=0;
    if(ev) {
        int sixths=(int)ev-NATIVE_EV_ZERO;
        /* The tenths, rounded: (a + 3) / 6 as a product with ceil(2^18 / 6),
           exact for these small numbers. The core links no division helper,
           and a compiler makes a loop of subtractions a division. */
        u32 a=(u32)(sixths<0 ? -sixths : sixths)*10U+3U, tenths=a*43691U>>18;
        detail_add(&item,"EV ");
        if(tenths) detail_add(&item,sixths<0 ? "-" : "+");
        detail_number(&item,native_div10(tenths));
        detail_add(&item,".");
        detail_number(&item,tenths-10U*native_div10(tenths));
        detail_put(d,&item);
    }
    if(iso && iso<=NATIVE_ISO_STEPS) {
        item.n=0;
        detail_add(&item,"ISO max ");
        detail_number(&item,detail_iso_values[iso-1U]);
        detail_put(d,&item);
    }
    v=NATIVE_METERING(x);
    if(v) detail_named(d,"Meter",native_metering_labels,NATIVE_COUNT(native_metering_labels),v-1U);
    v=NATIVE_ND(x);
    if(v) detail_named(d,"ND",native_nd_labels,NATIVE_COUNT(native_nd_labels),v-1U);
    detail_break(d);
}

/* Write the rows of the recipe of words `w` into `rows`, all of them or
   those of the panel (DETAIL_PANEL), and give their number. */
static u32 recipe_details(volatile u32 *h,const u32 *w,u32 mode,volatile u16 *rows) {
    DetailText d;
    d.rows=rows; d.n=0; d.at=0;
    detail_look(h,&d,w);
    detail_levels_of(&d,w);
    detail_native(&d,w);
    if(mode!=DETAIL_PANEL) detail_modules(&d,w);
    detail_white_balance(&d,w);
    detail_corrections_of(&d,w);
    detail_exposure(&d,w);
    /* A recipe that keeps each setting still has a row. */
    if(!d.n) {
        DetailItem item;
        item.n=0;
        detail_add(&item,"No settings");
        detail_put(&d,&item);
        detail_break(&d);
    }
    return d.n;
}
#endif
