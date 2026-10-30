/* Generated from crates/grkit-core/src/native.rs. Do not edit: change the
   tables there, then run GRKIT_UPDATE_GENERATED=1 cargo test -p grkit-core
   --test generated.

   The native picture settings of camera recipes (runtime/recipes.h): the
   portable index of each Image Control type, parameter, white balance mode,
   and correction setting, and the names and labels of each family for
   UserData Set. A label table gives the label of each stored value, 0 for a
   value without one. runtime/targets/<target>-native.h gives the stored
   values and the field offsets of a build. */
#ifndef GRKIT_NATIVE_H
#define GRKIT_NATIVE_H

#define NATIVE_TYPES 21U
#define NATIVE_FIRST_MONOCHROME 14U
#define NATIVE_PARAMETERS 21U
#define NATIVE_LEVELS 9U
#define NATIVE_MODES 18U
#define NATIVE_CORRECTIONS 4U
#define NATIVE_TEMPERATURE_MIN 2500U
#define NATIVE_TEMPERATURE_MAX 10000U
#define NATIVE_TEMPERATURE_STEP 10U
#define NATIVE_PARAMETER_SATURATION 0U
#define NATIVE_PARAMETER_COLORHUE 1U
#define NATIVE_PARAMETER_IMAGEKEY 2U
#define NATIVE_PARAMETER_CONTRAST 3U
#define NATIVE_PARAMETER_CONTRASTHIGHLIGHT 4U
#define NATIVE_PARAMETER_CONTRASTSHADOW 5U
#define NATIVE_PARAMETER_SHARPNESS 6U
#define NATIVE_PARAMETER_SHADING 7U
#define NATIVE_PARAMETER_CLARITYCONTROL 8U
#define NATIVE_PARAMETER_COLORTONEMONOTONE 9U
#define NATIVE_PARAMETER_COLORTONEBLEECHBYPASS 10U
#define NATIVE_PARAMETER_COLORTONERETRO 11U
#define NATIVE_PARAMETER_COLORTONEHDRINTENSE 12U
#define NATIVE_PARAMETER_HDRINTENSE 13U
#define NATIVE_PARAMETER_COLORTONECROSSPROCESS 14U
#define NATIVE_PARAMETER_COLORTONECINEMA 15U
#define NATIVE_PARAMETER_FILTEREFFECTCUSTOM 16U
#define NATIVE_PARAMETER_PARTICLE 17U
#define NATIVE_PARAMETER_PARTICLESIZE 18U
#define NATIVE_PARAMETER_PARTICLESTRENGTH 19U
#define NATIVE_PARAMETER_SOLARIZATIONTYPE 20U

#define NATIVE_TYPE_SETTING "StillImageControlNo0"
#define NATIVE_ACTIVE_SET_SETTING "StillActiveImageControlSet"
#define NATIVE_MAIN_SET "No0"
#define NATIVE_WHITE_BALANCE_SETTING "StillWhiteBalance"
#define NATIVE_COLOR_TEMPERATURE_SETTING "StillWhiteBalanceColorTemp"
#define NATIVE_PREFIXES "StillImageControlNo0Param","AfterReleaseStillImageControlNo0Param","StillFwupImageControlNo0Param"
#define NATIVE_PARAMETER_KEYS "Saturation","ColorHue","ImageKey","Contrast","ContrastHighLight","ContrastShadow","Sharpness","Shading","ClarityControl","ColorToneMonotone","ColorToneBleechBypass","ColorToneRetro","ColorToneHdrIntense","HdrIntense","ColorToneCrossProcess","ColorToneCinema","FilterEffectCustom","Particle","ParticleSize","ParticleStrength","SolarizationType"
#define NATIVE_CORRECTION_NAMES "StillHighlightCorrect","StillShadowCorrect","StillLensShadingCorrection","StillHighIsoNR"
#define NATIVE_SHIFT_BA_LABELS "B14","B13","B12","B11","B10","B9","B8","B7","B6","B5","B4","B3","B2","B1","NORMAL","A1","A2","A3","A4","A5","A6","A7","A8","A9","A10","A11","A12","A13","A14"
#define NATIVE_SHIFT_GM_LABELS "G14","G13","G12","G11","G10","G9","G8","G7","G6","G5","G4","G3","G2","G1","NORMAL","M1","M2","M3","M4","M5","M6","M7","M8","M9","M10","M11","M12","M13","M14"
#define NATIVE_CORRECTION_0_LABELS "Auto","On","Off"
#define NATIVE_CORRECTION_1_LABELS "Auto","Low","Mid","High","Off"
#define NATIVE_CORRECTION_2_LABELS "Off","On"
#define NATIVE_CORRECTION_3_LABELS "Auto","Low","Mid","High","Custom","Off"

/* gr3. */
#define NATIVE_GR3_TYPE_LABELS "Standard","Vivid","MonotoneGR","SoftMonotone","HardMonotone","HighContrastBW","NegativeFilm","PositiveFilm","BleechBypass_GR","Retro","HdrCuisine","CrossProcess",0,0,0,0,0,0,0,0,0
#define NATIVE_GR3_MODE_LABELS "Auto","Multi",0,0,"Daylight","Shade","Cloudiness","Flu_D","Flu_N","Flu_W","Flu_L","Tungsten","Cte","Manual1","ColorTemp1","Custom1","Custom2","Custom3"
#define NATIVE_GR3_MODE_SHIFTS 1,1,0,0,1,1,1,1,1,1,1,1,1,1,1,0,0,0
#define NATIVE_GR3_TYPE_PREFIXES 0,0,0,0,0,0,1,0,0,0,0,2,0,0,0,0,0,0,0,0,0
#define NATIVE_GR3_LABELS "M4","M3","M2","M1","OFF","P1","P2","P3","P4","M4","M3","M2","M1","OFF","P1","P2","P3","P4","M4","M3","M2","M1","OFF","P1","P2","P3","P4","M4","M3","M2","M1","OFF","P1","P2","P3","P4","M4","M3","M2","M1","OFF","P1","P2","P3","P4","M4","M3","M2","M1","OFF","P1","P2","P3","P4","M4","M3","M2","M1","OFF","P1","P2","P3","P4","M4","M3","M2","M1","OFF","P1","P2","P3","P4","M4","M3","M2","M1","OFF","P1","P2","P3","P4","OFF","Sepia","Red","Green","Blue","Purple",0,"Normal","Cool","Warm","M4","M3","M2","M1","OFF","P1","P2","P3","P4","OFF","BW","Sepia",0,"Low","Mid","High","Basic","Magenta","Yellow","OFF","Custom1","Custom2","Custom3","Custom4","Off","High","Mid","Low"
#define NATIVE_GR3_LABEL_STARTS 0,9,18,27,36,45,54,63,72,81,87,91,100,103,107,110,110,115,119,119,119
#define NATIVE_GR3_LABEL_COUNTS 9,9,9,9,9,9,9,9,9,6,4,9,3,4,3,0,5,4,0,0,0
/* gr4. */
#define NATIVE_GR4_TYPE_LABELS "Standard","Vivid","MonotoneGR","SoftMonotone","HardMonotone","HighContrastBW","NegativeFilm","PositiveFilm","BleechBypass_GR","Retro","HdrCuisine","CrossProcess","CinemaYellow","CinemaGreen","MonochromeStandard","MonochromeSoft","MonochromeHighContrast","MonochromeGrainy","MonochromeSolid","MonochromeHdrCuisine","MonochromeSolarization"
#define NATIVE_GR4_MODE_LABELS "AutoStandard",0,"AutoWarm","AutoWhite","Daylight","Shade","Cloudiness","Flu_D","Flu_N","Flu_W","Flu_L","Tungsten","Cte","Manual1","ColorTemp1","Custom1","Custom2","Custom3"
#define NATIVE_GR4_MODE_SHIFTS 1,0,1,1,1,1,1,1,1,1,1,1,1,1,1,0,0,0
#define NATIVE_GR4_TYPE_PREFIXES 0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,1,1,1,1,1,1
#define NATIVE_GR4_LABELS "M4","M3","M2","M1","OFF","P1","P2","P3","P4","M4","M3","M2","M1","OFF","P1","P2","P3","P4","M4","M3","M2","M1","OFF","P1","P2","P3","P4","M4","M3","M2","M1","OFF","P1","P2","P3","P4","M4","M3","M2","M1","OFF","P1","P2","P3","P4","M4","M3","M2","M1","OFF","P1","P2","P3","P4","M4","M3","M2","M1","OFF","P1","P2","P3","P4","M4","M3","M2","M1","OFF","P1","P2","P3","P4","M4","M3","M2","M1","OFF","P1","P2","P3","P4","OFF","Sepia","Red","Green","Blue","Purple",0,"Normal","Cool","Warm","M4","M3","M2","M1","OFF","P1","P2","P3","P4","OFF","BW","Sepia",0,"Low","Mid","High","Basic","Magenta","Yellow","Low","Mid","High","OFF","Custom1","Custom2","Custom3","Custom4","Off","On","Small","Medium","Large","Low","Mid","High","TYPE1","TYPE2","TYPE3"
#define NATIVE_GR4_LABEL_STARTS 0,9,18,27,36,45,54,63,72,81,87,91,100,103,107,110,113,118,120,123,126
#define NATIVE_GR4_LABEL_COUNTS 9,9,9,9,9,9,9,9,9,6,4,9,3,4,3,3,5,2,3,3,3

#endif
