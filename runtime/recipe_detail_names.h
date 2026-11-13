#ifndef GRKIT_RECIPE_DETAIL_NAMES_H
#define GRKIT_RECIPE_DETAIL_NAMES_H
/* The English names of the settings of a recipe, which the View page writes
   above their values (runtime/recipe_details.h: the label of an item,
   runtime/viewfinder.h: view_paint). Name 0 is none. The core and the live
   view each include this table, as both read it. */
#define DETAIL_NAME_NONE 0U
#define DETAIL_NAME_FILM_LOOK 1U
#define DETAIL_NAME_IMAGE_CONTROL 2U
/* The levels in the order of NATIVE_PARAMETER_SATURATION to
   NATIVE_PARAMETER_CLARITYCONTROL. */
#define DETAIL_NAME_LEVEL(p) (3U+(p))
#define DETAIL_NAME_TONING 12U
#define DETAIL_NAME_HDR 13U
#define DETAIL_NAME_FILTER 14U
#define DETAIL_NAME_GRAIN_EFFECT 15U
#define DETAIL_NAME_SOLARIZATION 16U
#define DETAIL_NAME_GRAIN 17U
#define DETAIL_NAME_BLOOM 18U
#define DETAIL_NAME_HALATION 19U
#define DETAIL_NAME_BORDER 20U
#define DETAIL_NAME_WHITE_BALANCE 21U
/* The corrections in the order of NATIVE_CORRECTION_NAMES. */
#define DETAIL_NAME_CORRECTION(c) (22U+(c))
#define DETAIL_NAME_COMPENSATION 26U
#define DETAIL_NAME_ISO 27U
#define DETAIL_NAME_METERING 28U
#define DETAIL_NAME_ND 29U
#define DETAIL_NAMES 30U
static const char *const detail_names[DETAIL_NAMES]={
    0,"Film Look","Image Control",
    "Saturation","Hue","High/Low Key","Contrast","Contrast (Highlight)","Contrast (Shadow)","Sharpness",
    "Shading","Clarity",
    "Toning","HDR","Filter Effect","Grain Effect","Solarization",
    "Grain","Bloom","Halation","Border",
    "White Balance",
    "Highlight Correction","Shadow Correction","Peripheral Illumination","High-ISO NR",
    "Exposure Compensation","Auto ISO Upper Limit","Metering","ND Filter"};
#endif
