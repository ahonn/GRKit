/* Generated from crates/grkit-core/src/aspect/. Do not edit: change the tables
   there, then run GRKIT_UPDATE_GENERATED=1 cargo test -p grkit-core --test
   generated. */
#ifndef GRKIT_FRAMING_RATIOS_H
#define GRKIT_FRAMING_RATIOS_H

/* The Framing ratios by CORE.ASPECT value: 0 Off, 1 XPan 65:24, 2 16:9, 3 4:3,
   4 2.35:1, 5 1:1, 6 5:4, 7 7:6, 8 3:4. Each table here has an entry for each
   value, in value order. */
/* The capture aspect id of a shot and of a RAW development. */
#define FRAMING_RATIO_CAPTURES 0,2,2,1,2,1,1,1,1
#define FRAMING_RATIO_DEVELOP 0,2,2,1,2,3,1,1,1
/* GRKIT_HOST_FRAME_FILE of a RAW development to the ratio: 1 for the stock
   sizes of its capture aspect id. */
#define FRAMING_RATIO_FILES 1,2,1,1,3,1,4,5,6
/* The visible-area id (runtime/picture.c), and the live-view label
   (grkit_core::artwork::OVERLAY_LABELS). */
#define FRAMING_RATIO_AREAS 0,4,2,1,5,3,6,7,8
#define FRAMING_RATIO_LABELS 0,1,2,3,4,5,6,7,18
/* The photo area {left, top, right, bottom} on the 720 x 480 live view. */
#define FRAMING_PHOTO_BOXES {0,0,720,480},{0,107,720,373},{0,37,720,442},{40,0,680,480},{0,87,720,393},{120,0,600,480},{60,0,660,480},{80,0,640,480},{180,0,540,480}
/* GRKIT_HOST_FRAME_FILE: 0 while shooting, 1 for the stock sizes, then each
   Framing geometry: 2 XPan 65:24, 3 2.35:1, 4 5:4, 5 7:6, 6 3:4, 7 5:4 before
   1.6.1, 8 7:6 before 1.6.1. A geometry before 1.6.1 keeps the sizes that the
   releases before 1.6.1 recorded. */
#define FRAMING_FILES 9
/* The capture aspect id and the visible-area id of each file. */
#define FRAMING_FILE_CAPTURES 0,0,2,2,1,1,1,1,1
#define FRAMING_FILE_AREAS 0,0,4,5,6,7,8,6,7
/* The ratio {p, q} of each file that a crop of a ratio has (runtime/crop.h),
   or {0, 0}: a crop has the ratio within FRAMING_CROP_SLACK (p + q), and a
   short side of at least FRAMING_CROP_SHORTEST. */
#define FRAMING_FILE_RATIOS {0,0},{0,0},{65,24},{47,20},{5,4},{7,6},{3,4},{0,0},{0,0}
#define FRAMING_CROP_SLACK 9
#define FRAMING_CROP_SHORTEST 480
/* The geometries of capture aspect id 2 (runtime/framing.S): 0 16:9, 1 XPan
   65:24, 2 2.35:1. The mode of each file, padded to FRAMING_MODE_INDEXES, and
   of each value of GRKIT_HOST_FRAME_CROP. */
#define FRAMING_WIDE_MODES 3
#define FRAMING_WIDE_BY_FILE 0,0,1,2,0,0,0,0,0
#define FRAMING_WIDE_BY_CROP 0,1,0,0,2,0,0,0,0
/* Whether each mode has sizes that no native aspect has (runtime/crop.h:
   crop_resolution_size). */
#define FRAMING_WIDE_FRAMED 0,1,1
/* The top inset of the screennail and of the thumbnail by mode. */
#define FRAMING_WIDE_SCREENNAIL 38,107,87
#define FRAMING_WIDE_THUMBNAIL 16,30,26
/* The geometries of capture aspect id 1 (runtime/framing.S): 0 4:3, 1 1:1, 2
   5:4, 3 7:6, 4 3:4, 5 5:4 before 1.6.1, 6 7:6 before 1.6.1. The mode of each
   file, padded to FRAMING_MODE_INDEXES, and of each value of
   GRKIT_HOST_FRAME_CROP. */
#define FRAMING_NARROW_MODES 7
#define FRAMING_NARROW_BY_FILE 0,0,0,0,2,3,4,5,6
#define FRAMING_NARROW_BY_CROP 0,0,0,0,0,1,2,3,4
/* Whether each mode has sizes that no native aspect has (runtime/crop.h:
   crop_resolution_size). */
#define FRAMING_NARROW_FRAMED 0,0,1,1,1,1,1
/* The left inset of the screennail and of the thumbnail by mode. */
#define FRAMING_NARROW_SCREENNAIL 40,120,60,80,180,60,80
#define FRAMING_NARROW_THUMBNAIL 0,20,6,10,36,6,10
#define FRAMING_MODE_INDEXES 9
/* The insets {left, top} of each visible area in the 720 x 480 screennail and
   the 160 x 120 thumbnail: 3:2, 4:3, 16:9, 1:1, then each Framing geometry. */
#define FRAMING_AREAS 9
#define FRAMING_AREA_SCREENNAILS {0,0},{40,0},{0,38},{120,0},{0,107},{0,87},{60,0},{80,0},{180,0}
#define FRAMING_AREA_THUMBNAILS {0,8},{0,0},{0,16},{20,0},{0,30},{0,26},{6,0},{10,0},{36,0}

#if defined(FRAMING_FAMILY_GR3)
/* Off, then the values of the rows, from the widest ratio. */
#define FRAMING_RATIO_ROWS 0,1,4,2,3,6,7,5,8
/* GetResolution of each size class, L to XS, by mode: the height of id 2,
   the width of id 1, and the XS height of id 1. */
#define FRAMING_WIDE_HEIGHT_0 3376,2216,2552
#define FRAMING_WIDE_HEIGHT_1 2704,1776,2040
#define FRAMING_WIDE_HEIGHT_2 1888,1240,1432
#define FRAMING_WIDE_HEIGHT_3 1080,712,816
#define FRAMING_NARROW_WIDTH_0 5328,4000,4992,4656,2992,5000,4664
#define FRAMING_NARROW_WIDTH_1 4272,3200,4000,3728,2400,4000,3736
#define FRAMING_NARROW_WIDTH_2 2992,2240,2800,2608,1680,2800,2616
#define FRAMING_NARROW_WIDTH_3 1920,1280,1792,1680,1072,1800,1680
#define FRAMING_NARROW_HEIGHT_3 1440,1280,1440,1440,1440,1440,1440
/* The stock width of id 2 and height of id 1 of each size class. */
#define FRAMING_STOCK_WIDTHS 6000,4800,3360,1920
#define FRAMING_STOCK_HEIGHTS 4000,3200,2240,1440
/* The 3:2 height of each size class. */
#define FRAMING_NATIVE_HEIGHTS 4000,3200,2240,1280
/* CORE.CROP (runtime/crop.h): the crop factor of each value as float bits,
   and the camera's own factors of 35 and 50 mm. */
#define CROP_FACTORS 0x3f800000,0x3fa00000,0x3fb6db6e,0x3fe49249,0x40200000
#define CROP_NATIVE_35 0x3fa00000
#define CROP_NATIVE_50 0x3fe49249
/* The sizes {long side, short side} of each file from 2, L to XS, which
   the playback classifier compares. */
#define FRAMING_SIZES 6000,2216,4800,1776,3360,1240,1920,712,6000,2552,4800,2040,3360,1432,1920,816,4992,4000,4000,3200,2800,2240,1792,1440,4656,4000,3728,3200,2608,2240,1680,1440,4000,2992,3200,2400,2240,1680,1440,1072,5000,4000,4000,3200,2800,2240,1800,1440,4664,4000,3736,3200,2616,2240,1680,1440
#elif defined(FRAMING_FAMILY_GR4)
/* Off, then the values of the rows, from the widest ratio. */
#define FRAMING_RATIO_ROWS 0,1,4,2,3,6,7,5,8
/* GetResolution of each size class, L to XS, by mode: the height of id 2,
   the width of id 1, and the XS height of id 1. */
#define FRAMING_WIDE_HEIGHT_0 3480,2288,2632
#define FRAMING_WIDE_HEIGHT_1 2784,1824,2104
#define FRAMING_WIDE_HEIGHT_2 1968,1296,1488
#define FRAMING_WIDE_HEIGHT_3 1080,712,816
#define FRAMING_NARROW_WIDTH_0 5504,4128,5152,4816,3088,5160,4816
#define FRAMING_NARROW_WIDTH_1 4400,3296,4112,3840,2464,4120,3848
#define FRAMING_NARROW_WIDTH_2 3120,2336,2912,2720,1744,2920,2728
#define FRAMING_NARROW_WIDTH_3 1920,1280,1792,1680,1072,1800,1680
#define FRAMING_NARROW_HEIGHT_3 1440,1280,1440,1440,1440,1440,1440
/* The stock width of id 2 and height of id 1 of each size class. */
#define FRAMING_STOCK_WIDTHS 6192,4944,3504,1920
#define FRAMING_STOCK_HEIGHTS 4128,3296,2336,1440
/* The 3:2 height of each size class. */
#define FRAMING_NATIVE_HEIGHTS 4128,3296,2336,1280
/* CORE.CROP (runtime/crop.h): the crop factor of each value as float bits,
   and the camera's own factors of 35 and 50 mm. */
#define CROP_FACTORS 0x3f800000,0x3fa04f89,0x3fb66614,0x3fe23119,0x401f9951
#define CROP_NATIVE_35 0x3fa04f89
#define CROP_NATIVE_50 0x3fe23119
/* The sizes {long side, short side} of each file from 2, L to XS, which
   the playback classifier compares. */
#define FRAMING_SIZES 6192,2288,4944,1824,3504,1296,1920,712,6192,2632,4944,2104,3504,1488,1920,816,5152,4128,4112,3296,2912,2336,1792,1440,4816,4128,3840,3296,2720,2336,1680,1440,4128,3088,3296,2464,2336,1744,1440,1072,5160,4128,4120,3296,2920,2336,1800,1440,4816,4128,3848,3296,2728,2336,1680,1440
#else
#error "Define FRAMING_FAMILY_<family> before runtime/framing_ratios.h"
#endif

#endif
