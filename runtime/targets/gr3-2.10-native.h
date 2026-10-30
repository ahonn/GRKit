/* Generated from the firmware of gr3-2.10 (crates/grkit-build/src/userdata.rs). Do
   not edit: with the firmware files of the ignored firmware tests, run
   GRKIT_UPDATE_GENERATED=1 cargo test -p grkit-build -- --ignored
   native_headers_match_each_firmware.

   The stored values and the field offsets of the native settings of recipes
   (runtime/native.h). FW_SETTINGS gives the settings object. A field is at
   its offset in the settings record, which is the word at NATIVE_RECORD in
   the object, or the object itself without NATIVE_RECORD. The fields are one
   byte, the color temperature two. Each offset of a parameter that a type
   does not have is 0, and each code of a type or mode that the camera does
   not have is all ones. NATIVE_TYPE_ICONS gives the native icon ID of each
   type, as the native Image Control list shows it, or 0. On GR IV, the byte
   at NATIVE_SENSOR_COLOR_FIELD of the model record (FW_MODEL_RECORD) is 1
   on GR IV Monochrome. */
#ifndef GRKIT_TARGET_NATIVE_H
#define GRKIT_TARGET_NATIVE_H
#define NATIVE_FAMILY_GR3 1
#define NATIVE_RECORD 4U
#define NATIVE_ACTIVE_SET_FIELD 448U
#define NATIVE_TYPE_FIELD 450U
#define NATIVE_TYPE_CODES 11,12,13,14,15,16,22,17,18,19,20,21,255,255,255,255,255,255,255,255,255
#define NATIVE_TYPE_ICONS 358,548,354,357,350,352,23,232,346,356,351,50,0,0,0,0,0,0,0,0,0
#define NATIVE_PARAMETER_FIELDS 456,468,480,498,516,534,552,632,650,0,0,0,0,0,0,0,0,0,0,0,0,458,470,482,500,518,536,554,634,652,0,0,0,0,0,0,0,0,0,0,0,0,0,0,484,502,520,538,556,636,654,570,0,0,0,0,0,0,676,666,0,0,0,0,0,486,504,522,540,558,638,0,572,0,0,0,0,0,0,678,668,0,0,0,0,0,488,506,524,542,560,640,656,574,0,0,0,0,0,0,0,670,0,0,0,0,0,490,508,526,544,562,642,658,576,0,0,0,0,0,0,680,672,0,0,0,2626,2628,2630,2632,2634,2636,2638,2640,2642,0,0,0,0,0,0,0,0,2644,0,0,0,460,472,492,510,528,546,564,644,660,0,0,0,0,0,0,0,0,0,0,0,0,462,474,494,512,530,548,566,646,662,0,578,0,0,0,0,0,0,0,0,0,0,464,476,496,514,532,550,568,648,664,0,0,580,0,0,0,0,0,0,0,0,0,466,478,0,0,0,0,0,0,0,0,0,0,582,674,0,0,0,0,0,0,0,2500,2502,2504,2506,2508,2510,2512,2514,2516,0,0,0,0,0,2518,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
#define NATIVE_MODE_FIELD 376U
#define NATIVE_MODE_CODES 0,1,65535,65535,2,3,4,5,6,7,8,9,17,11,14,255,254,253
#define NATIVE_SHIFT_FIELDS 404,378,406,380,0,0,0,0,408,382,410,384,412,386,414,388,416,390,418,392,420,394,422,396,424,398,426,400,428,402,0,0,0,0,0,0
#define NATIVE_TEMPERATURE_FIELD 430U
#define NATIVE_CORRECTION_FIELDS 1232,1234,1134,1238
#endif
