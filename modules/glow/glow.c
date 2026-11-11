/* Glow: independent Bloom and Halation around recorded highlights.

   The glow is light that spreads from the bright parts of the picture. The
   light of a pixel is the light above a soft threshold on its brightest
   channel, with the hue of the pixel kept:
   - Halation: the film base reflects light back into the red layer, and a
     little into the green layer. A white source makes an orange-red rim, a
     red source a red one, and blue little.
   - Bloom: a diffusion filter spreads light of every channel, so the glow
     keeps the color of its source.

   The light comes from the 720 x 480 screennail of the same encode:
   1. The clip map (clip_scan): the part of the main picture under each
      cell of 4 x 4 screennail pixels that is clipped, so that a light
      source of a few main pixels, which the screennail mixes with its dark
      surround, still glows (Look.point).
   2. The cells (coarse_light, coarse_surround): Look.cell screennail pixels
      each, with their mean luma blurred wide. They give the surround weight
      of each place (a light in the dark glows, a bright area much less),
      the area cut (Look.area), the local daylight gate and day_keep
      (Look.gate), and the color of a clipped light (Look.rim).
   3. The fine layer (fine_feed, glow_row): one value for each fine pixel of
      4 x 4 screennail pixels, with a 7-tap binomial blur (sigma 1.2 fine
      pixels) along the rows and the columns. Its rows are made as the
      composite needs them, from a ring of 7 rows.
   4. The composite: each plane gets the glow as Y, Cb, and Cr changes,
      interpolated for every row and every word of 4 pixels from values
      between the fine pixels (grid_row), so that a small glow stays round.
      A change is scaled by the headroom of its pixel,
      (255 - max(R,G,B)) / 256, so a bright area or the source itself gets
      little, and luma is added with saturation.
   The pixel kernels are in glow_kernels.c: NEON for whole blocks (an
   assembly part, because the camera objects build soft-float), C for the
   rest, with the same arithmetic. */
#include "grkit.h"
#include "grkit_module.h"
#include "glow_kernels.h"

#ifdef GRKIT_TEST_H
#define WORDS(address) ((u32 *)(void *)grkit_test_bytes(address))
#define BYTES(address) ((const u8 *)(const void *)grkit_test_bytes(address))
#else
#define WORDS(address) ((u32 *)(address))
#define BYTES(address) ((const u8 *)(address))
#endif



#define FAINT 2U          /* glow below this channel value is 0 */
#define MAX_WORDS 1600U   /* 6400 pixels */
#define MAX_BLOCKS (MAX_WORDS / BLOCK_WORDS)
#define ALL_WORDS (MAX_WORDS + 192U) /* the words of all planes: main (GR IV: 1548), screennail, thumbnail */
#define ALL_BLOCKS (ALL_WORDS / BLOCK_WORDS + 4U)
#define MAX_SOURCE 768U   /* screennail pixels */
#define MAX_FINE (MAX_SOURCE / 4U)
#define MAX_GRID (2U * MAX_FINE)
#define FINE_SHIFT 2U     /* a fine pixel: 4 x 4 screennail pixels */
#define GRID_OFFSET 6144U /* the fine position of grid value 0, of 4096: a quarter of a fine pixel in */
#define RING 7U           /* the rows of the fine blur */
#define MAX_COARSE 64U    /* at least MAX_SOURCE / the smallest Look.cell */
#define MAX_CLIP_COLUMNS (MAX_SOURCE / 4U)
#define CHROMA_FROM 160U  /* Look.chroma: the spread of the channels from which it lifts */
#define LOCAL_GATE_MIN 64U /* local evidence also matters in low-EV interiors */
#define GATE_RANGE 24U    /* 1/16 EV over which the local daylight gate takes over */
#define DAY_FROM 16       /* 1/16 EV above Look.gate from which the glow fades out */
#define DAY_RANGE 32      /* 1/16 EV over which it fades out */
#define DARK_LUMA 50U     /* the mean luma of a coarse cell below which it is dark */
#define DARK_SCENE 218U   /* the dark cells (of 256) of a night scene that meters like day */
#define RIM_LOW 120U      /* Look.rim: the brightest channel from which a pixel tells the color of a light */
#define RIM_FROM 246U     /* Look.rim: the brightest channel from which a pixel takes that color */
#define RIM_MIN 128U      /* Look.rim: the weight of light pixels that a cell needs to tell a color */
#define RIM_DECAY 48U     /* Look.rim: the confidence that a color loses for each cell that it spreads */
#define CLIP_LEVEL 250U   /* the main picture luma (of the white) from which a pixel is clipped */
#define CLIP_SHIFT 2U     /* a clip map cell: 4 x 4 screennail pixels */
#define CLIP_AREA 32U     /* a mean density of 3 x 3 cells from which they are a bright area */
#define MAX_CLIP_BLOCKS (MAX_WORDS * 4U / 16U) /* 16-pixel blocks of a main row */
#define LIFT_FROM 128     /* 1/16 EV (8 EV) up to which Strong takes all of its lift */
#define LIFT_RANGE 32     /* 1/16 EV over which the lift fades out, to none from 10 EV */
#define LARGE_KEEP 224U   /* the blur of the light density of the cells (blur keep) */
#define LARGE_FROM 8U     /* the blurred light density (of 255) from which light is a large area */
#define LARGE_RANGE 32U   /* the density over which it becomes fully large */
#define VEIL_DARK 60U     /* the screennail luma of a receiver up to which the veil guard is full */
#define VEIL_LIGHT 150U   /* the luma from which it does nothing */
#define VEIL_CUT 230U     /* the most (of 256) of the glow that the veil guard takes away */
#define VEIL_SOURCE 200U  /* the screennail luma from which a pixel is part of the light, not of the receiver */

/* Linear light of a channel value, (c / 255)^2.2 x 255. */
static const u8 linear[256] = {
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2,
    3, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 6, 6, 6,
    6, 7, 7, 7, 8, 8, 8, 9, 9, 9, 10, 10, 11, 11, 11, 12,
    12, 13, 13, 13, 14, 14, 15, 15, 16, 16, 17, 17, 18, 18, 19, 19,
    20, 20, 21, 22, 22, 23, 23, 24, 25, 25, 26, 26, 27, 28, 28, 29,
    30, 30, 31, 32, 33, 33, 34, 35, 35, 36, 37, 38, 39, 39, 40, 41,
    42, 43, 43, 44, 45, 46, 47, 48, 49, 49, 50, 51, 52, 53, 54, 55,
    56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71,
    73, 74, 75, 76, 77, 78, 79, 81, 82, 83, 84, 85, 87, 88, 89, 90,
    91, 93, 94, 95, 97, 98, 99, 100, 102, 103, 105, 106, 107, 109, 110, 111,
    113, 114, 116, 117, 119, 120, 121, 123, 124, 126, 127, 129, 130, 132, 133, 135,
    137, 138, 140, 141, 143, 145, 146, 148, 149, 151, 153, 154, 156, 158, 159, 161,
    163, 165, 166, 168, 170, 172, 173, 175, 177, 179, 181, 182, 184, 186, 188, 190,
    192, 194, 196, 197, 199, 201, 203, 205, 207, 209, 211, 213, 215, 217, 219, 221,
    223, 225, 227, 229, 231, 234, 236, 238, 240, 242, 244, 246, 248, 251, 253, 255,
};

/* The glow is in linear light of 16 bits (GLOW_FULL = 1.0). encode_low
   (below 1024) and encode_high (in steps of 64) give its channel value
   through the sRGB curve, whose straight part near black keeps a faint glow
   smooth. */

static const u8 encode_low[1024] = {
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2,
    2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
    2, 2, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
    3, 3, 3, 3, 3, 3, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
    4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 5, 5, 5, 5, 5, 5,
    5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 6, 6,
    6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6,
    6, 6, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
    7, 7, 7, 7, 7, 7, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 10, 10, 10,
    10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10,
    10, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11,
    11, 11, 11, 11, 11, 11, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12,
    12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 13, 13, 13, 13,
    13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13,
    13, 13, 13, 13, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14,
    14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 15, 15, 15, 15,
    15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15,
    15, 15, 15, 15, 15, 15, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16,
    16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16,
    16, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17,
    17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 18, 18,
    18, 18, 18, 18, 18, 18, 18, 18, 18, 18, 18, 18, 18, 18, 18, 18,
    18, 18, 18, 18, 18, 18, 18, 18, 18, 18, 18, 18, 19, 19, 19, 19,
    19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19,
    19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 20, 20, 20, 20, 20,
    20, 20, 20, 20, 20, 20, 20, 20, 20, 20, 20, 20, 20, 20, 20, 20,
    20, 20, 20, 20, 20, 20, 20, 20, 20, 20, 20, 21, 21, 21, 21, 21,
    21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21,
    21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 22, 22, 22, 22,
    22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22,
    22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 23,
    23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23,
    23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23,
    23, 23, 23, 23, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24,
    24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24,
    24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 25, 25, 25, 25, 25, 25,
    25, 25, 25, 25, 25, 25, 25, 25, 25, 25, 25, 25, 25, 25, 25, 25,
    25, 25, 25, 25, 25, 25, 25, 25, 25, 25, 25, 25, 25, 25, 25, 25,
    25, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26,
    26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26,
    26, 26, 26, 26, 26, 26, 26, 26, 26, 27, 27, 27, 27, 27, 27, 27,
    27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27,
    27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27,
    27, 27, 27, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28,
    28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28,
    28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 29,
    29, 29, 29, 29, 29, 29, 29, 29, 29, 29, 29, 29, 29, 29, 29, 29,
    29, 29, 29, 29, 29, 29, 29, 29, 29, 29, 29, 29, 29, 29, 29, 29,
    29, 29, 29, 29, 29, 29, 29, 29, 29, 29, 29, 29, 30, 30, 30, 30,
    30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30,
    30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30,
    30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 31, 31, 31, 31, 31, 31,
    31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31,
    31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31,
    31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 32, 32, 32, 32, 32, 32,
    32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,
    32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,
    32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 33, 33, 33, 33, 33,
    33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33,
    33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33,
    33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 34, 34,
};

static const u8 encode_high[1024] = {
    0, 3, 6, 10, 13, 15, 18, 20, 22, 23, 25, 27, 28, 30, 31, 32,
    34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49,
    49, 50, 51, 52, 53, 53, 54, 55, 56, 56, 57, 58, 58, 59, 60, 61,
    61, 62, 62, 63, 64, 64, 65, 66, 66, 67, 67, 68, 68, 69, 70, 70,
    71, 71, 72, 72, 73, 73, 74, 74, 75, 76, 76, 77, 77, 78, 78, 79,
    79, 79, 80, 80, 81, 81, 82, 82, 83, 83, 84, 84, 85, 85, 85, 86,
    86, 87, 87, 88, 88, 88, 89, 89, 90, 90, 91, 91, 91, 92, 92, 93,
    93, 93, 94, 94, 95, 95, 95, 96, 96, 97, 97, 97, 98, 98, 98, 99,
    99, 99, 100, 100, 101, 101, 101, 102, 102, 102, 103, 103, 103, 104, 104, 104,
    105, 105, 106, 106, 106, 107, 107, 107, 108, 108, 108, 109, 109, 109, 110, 110,
    110, 110, 111, 111, 111, 112, 112, 112, 113, 113, 113, 114, 114, 114, 115, 115,
    115, 115, 116, 116, 116, 117, 117, 117, 118, 118, 118, 118, 119, 119, 119, 120,
    120, 120, 121, 121, 121, 121, 122, 122, 122, 123, 123, 123, 123, 124, 124, 124,
    125, 125, 125, 125, 126, 126, 126, 126, 127, 127, 127, 128, 128, 128, 128, 129,
    129, 129, 129, 130, 130, 130, 130, 131, 131, 131, 131, 132, 132, 132, 133, 133,
    133, 133, 134, 134, 134, 134, 135, 135, 135, 135, 136, 136, 136, 136, 137, 137,
    137, 137, 138, 138, 138, 138, 138, 139, 139, 139, 139, 140, 140, 140, 140, 141,
    141, 141, 141, 142, 142, 142, 142, 143, 143, 143, 143, 143, 144, 144, 144, 144,
    145, 145, 145, 145, 146, 146, 146, 146, 146, 147, 147, 147, 147, 148, 148, 148,
    148, 148, 149, 149, 149, 149, 150, 150, 150, 150, 150, 151, 151, 151, 151, 152,
    152, 152, 152, 152, 153, 153, 153, 153, 153, 154, 154, 154, 154, 155, 155, 155,
    155, 155, 156, 156, 156, 156, 156, 157, 157, 157, 157, 157, 158, 158, 158, 158,
    158, 159, 159, 159, 159, 159, 160, 160, 160, 160, 160, 161, 161, 161, 161, 161,
    162, 162, 162, 162, 162, 163, 163, 163, 163, 163, 164, 164, 164, 164, 164, 165,
    165, 165, 165, 165, 166, 166, 166, 166, 166, 167, 167, 167, 167, 167, 168, 168,
    168, 168, 168, 168, 169, 169, 169, 169, 169, 170, 170, 170, 170, 170, 171, 171,
    171, 171, 171, 171, 172, 172, 172, 172, 172, 173, 173, 173, 173, 173, 173, 174,
    174, 174, 174, 174, 175, 175, 175, 175, 175, 175, 176, 176, 176, 176, 176, 177,
    177, 177, 177, 177, 177, 178, 178, 178, 178, 178, 178, 179, 179, 179, 179, 179,
    179, 180, 180, 180, 180, 180, 181, 181, 181, 181, 181, 181, 182, 182, 182, 182,
    182, 182, 183, 183, 183, 183, 183, 183, 184, 184, 184, 184, 184, 184, 185, 185,
    185, 185, 185, 185, 186, 186, 186, 186, 186, 186, 187, 187, 187, 187, 187, 187,
    188, 188, 188, 188, 188, 188, 189, 189, 189, 189, 189, 189, 190, 190, 190, 190,
    190, 190, 191, 191, 191, 191, 191, 191, 191, 192, 192, 192, 192, 192, 192, 193,
    193, 193, 193, 193, 193, 194, 194, 194, 194, 194, 194, 194, 195, 195, 195, 195,
    195, 195, 196, 196, 196, 196, 196, 196, 197, 197, 197, 197, 197, 197, 197, 198,
    198, 198, 198, 198, 198, 199, 199, 199, 199, 199, 199, 199, 200, 200, 200, 200,
    200, 200, 200, 201, 201, 201, 201, 201, 201, 202, 202, 202, 202, 202, 202, 202,
    203, 203, 203, 203, 203, 203, 203, 204, 204, 204, 204, 204, 204, 204, 205, 205,
    205, 205, 205, 205, 206, 206, 206, 206, 206, 206, 206, 207, 207, 207, 207, 207,
    207, 207, 208, 208, 208, 208, 208, 208, 208, 209, 209, 209, 209, 209, 209, 209,
    210, 210, 210, 210, 210, 210, 210, 211, 211, 211, 211, 211, 211, 211, 212, 212,
    212, 212, 212, 212, 212, 212, 213, 213, 213, 213, 213, 213, 213, 214, 214, 214,
    214, 214, 214, 214, 215, 215, 215, 215, 215, 215, 215, 216, 216, 216, 216, 216,
    216, 216, 216, 217, 217, 217, 217, 217, 217, 217, 218, 218, 218, 218, 218, 218,
    218, 219, 219, 219, 219, 219, 219, 219, 219, 220, 220, 220, 220, 220, 220, 220,
    221, 221, 221, 221, 221, 221, 221, 221, 222, 222, 222, 222, 222, 222, 222, 222,
    223, 223, 223, 223, 223, 223, 223, 224, 224, 224, 224, 224, 224, 224, 224, 225,
    225, 225, 225, 225, 225, 225, 225, 226, 226, 226, 226, 226, 226, 226, 227, 227,
    227, 227, 227, 227, 227, 227, 228, 228, 228, 228, 228, 228, 228, 228, 229, 229,
    229, 229, 229, 229, 229, 229, 230, 230, 230, 230, 230, 230, 230, 230, 231, 231,
    231, 231, 231, 231, 231, 231, 232, 232, 232, 232, 232, 232, 232, 232, 233, 233,
    233, 233, 233, 233, 233, 233, 234, 234, 234, 234, 234, 234, 234, 234, 235, 235,
    235, 235, 235, 235, 235, 235, 236, 236, 236, 236, 236, 236, 236, 236, 236, 237,
    237, 237, 237, 237, 237, 237, 237, 238, 238, 238, 238, 238, 238, 238, 238, 239,
    239, 239, 239, 239, 239, 239, 239, 239, 240, 240, 240, 240, 240, 240, 240, 240,
    241, 241, 241, 241, 241, 241, 241, 241, 241, 242, 242, 242, 242, 242, 242, 242,
    242, 243, 243, 243, 243, 243, 243, 243, 243, 243, 244, 244, 244, 244, 244, 244,
    244, 244, 245, 245, 245, 245, 245, 245, 245, 245, 245, 246, 246, 246, 246, 246,
    246, 246, 246, 246, 247, 247, 247, 247, 247, 247, 247, 247, 248, 248, 248, 248,
    248, 248, 248, 248, 248, 249, 249, 249, 249, 249, 249, 249, 249, 249, 250, 250,
    250, 250, 250, 250, 250, 250, 250, 251, 251, 251, 251, 251, 251, 251, 251, 251,
    252, 252, 252, 252, 252, 252, 252, 252, 252, 253, 253, 253, 253, 253, 253, 253,
    253, 253, 254, 254, 254, 254, 254, 254, 254, 254, 254, 255, 255, 255, 255, 255,
};


static inline u32 encode_glow(u32 v) { return v < 1024 ? encode_low[v] : encode_high[(v + 32) >> 6]; }

/* A look. The glow adds in linear light.
   - fine: the part of the light of each channel R, G, B that spreads in
     the fine layer (of 256), at Medium strength.
   - strength: the scale of `fine` (Q8) at Weak, Medium, and Strong. The
     glow saturates toward its top, so Strong takes more than twice Medium
     to look as far from it as Weak. Strong also lifts the surround weight
     in low-EV scenes (surround_weight) and guards dark receivers next to
     a large light area (veil_guard).
   - cell: the side of a coarse cell in screennail pixels. The surround,
     the area cut, the local daylight gate, and the rim color work in
     cells, so the cell sets their reach in the picture.
   - boost: the maximum highlight gain (at most 16 with 4 x 4 fine pixels,
     so that the sum of a fine pixel stays in 16 bits), reached smoothly at
     white. A clipped picture cannot tell us the original source energy.
   - floor, knee: the linear light (of 255) of the brightest channel of a
     pixel from which it glows, with a soft start from floor - knee to
     floor + knee, so the glow starts from 0 and has no edge. Its channels
     glow alike (scaled by the light above the floor over the light of the
     brightest channel), so the glow keeps the hue of its source.
   - clip: the encoded channel value where the highlight gain starts to
     rise from 1 to boost at 255. The main-plane clip map is independent.
   - red: 0 for a glow in the color of its source (Bloom). Else the red
     light is that of red plus green times red / 256, the green light the
     smaller of green and red, and blue makes none (Halation: the light that
     the film base reflects into the red layer, and into the green one where
     the light also reaches the red layer. The fine channel weights then
     set the rim color: red dominates for white and red sources, while
     blue or cyan sources contribute little).
   - neutral: 0, or 1 for a glow without color: each channel takes the
     light of the brightest one (a White Halation, as black-and-white film
     gives). A look with `red` has no `neutral`.
   - point: how many times the light of a clipped pixel a light source
     gives that the screennail shows mixed with its dark surround (a lamp
     of a few main pixels): its light is that of the clipped part of the
     main picture around it (the clip map), in the hue of the brightest
     screennail pixel there.
   - area: 0, or how much a coarse cell that is mostly light (at floor or
     above) gives less (of 256 at a cell that is all light): the
     inside of a large bright area (a sign, a white coat) gives little, so
     the glow rims it and a small light source keeps its full glow. Up to a
     quarter of a cell may be light without a cut. The cuts are smoothed
     over the cells around, so a rim does not depend on where the edge of
     the area falls in the cell grid. The cut goes into the surround
     (coarse_surround).
   - chroma: 0, or how much the spread of the channels of a pixel (its
     brightest less its darkest) above CHROMA_FROM adds to its brightest
     channel (of 128) where the light starts: a saturated light (a red
     signal, a neon tube) ends below white in the picture, but its
     brightest channel is far above white in the scene.
   - gate: 0, or the scene EV (at ISO 100, in 1/16 EV) from which local
     background rejection grows from LOCAL_GATE_MIN to full over GATE_RANGE.
     From DAY_FROM above it, the whole glow fades out over DAY_RANGE, unless
     the picture is mostly dark (day_keep).
     Local evidence also applies in low-EV interiors and unknown exposures:
     a daylight window is not a point light just because the room is dark.
     Dark surroundings still retain isolated highlights at every scene EV.
   - vivid: 0, or the gain (of 256) of the color of the glow (its Cb and
     Cr changes): film turns the light near a source to its own color, so
     a glow over a pale glare must change its color more than its level.
   - rim: 0, or 1 for a glow in the color of the light around a clipped
     source: the camera clips a colored lamp to a pale or white core, and
     its color shows only in its unclipped fall-off. A pixel near white
     (RIM_FROM) takes that color, at its brightest level (rim_rgb).
   - protect: 0, or 1 to keep a colored glow off saturated cores: the
     headroom of a pixel is measured against the white of the effect, and
     a shared chroma pair of a half-height plane takes the brighter core of
     its two rows and is not tinted across the bottom crop edge. With
     Bloom, a place measures against a white between 255 and that white by
     the part of its light from Halation, so the headroom of Bloom has no
     edge where Halation starts.
   Each pixel takes the glow scaled by its maximum-channel headroom.
   Both chroma components share the mean headroom of their pixel pair. */
typedef struct {
    u32 fine[3];
    u16 strength[3];
    u32 cell;
    u32 boost;
    u32 floor;
    u32 clip;
    u32 knee;
    u32 red;
    u32 neutral;
    u32 point;
    u32 area;
    u32 chroma;
    u32 gate;
    u32 vivid;
    u32 rim;
    u32 protect;
} Look;

/* Halation: a red-orange rim on the dark side of a light source. The
   green weight is low, as encoding linear light makes it more visible than
   its coefficient ratio, but enough that the rim reads as orange-red, not
   as a dark red fringe. Chroma has unit gain: oversaturating the added
   light subtracts blue from the existing picture and turns overlapping
   halos into opaque orange patches. Only light near white glows (floor),
   a saturated light such as a red signal a little below it (chroma), and
   the inside of a large bright area little (area). Local background
   brightness distinguishes isolated highlights from bright surfaces: its
   rejection grows from 10 EV, and the whole glow fades from 11 EV to none
   at 13 EV (day_keep). The film simulation supplies the base color and
   tone response. Weak is lighter than half of Medium, to keep the rim
   subtle on top of a film simulation.

   The colors of the setting HALATION_COLOR differ only in the light of
   each channel. Each keeps about the luma of Orange, so that a strength
   looks alike in every color:
   - Red: almost no green, as a film without an anti-halation backing gives.
   - Orange: the orange-red rim above.
   - Gold: green at about half of red, as a very bright source gives when
     the reflected light also exposes the green layer.
   - White: no color (neutral), as black-and-white film gives.
   A monochrome picture has only luma, so it takes Orange in every color. */
#define HALATION_LOOK(r, g, b, red_mix, no_color) { \
    .fine = {r, g, b}, .strength = {128, 256, 576}, .cell = 13, \
    .boost = 4, .floor = 210, .clip = 224, .knee = 24, .red = red_mix, .neutral = no_color, \
    .point = 4, .area = 240, .chroma = 64, .gate = 160, .vivid = 256, .protect = 1, }
#define HALATION_ORANGE 1U
static const Look halation_looks[] = {
    HALATION_LOOK(700, 14, 0, 26, 0),
    HALATION_LOOK(520, 56, 0, 26, 0),
    HALATION_LOOK(290, 150, 0, 26, 0),
    HALATION_LOOK(140, 140, 140, 0, 1),
};
#define HALATION_COLORS (sizeof halation_looks / sizeof halation_looks[0])
/* Bloom spreads all channels equally in a compact halo, as a highlight
   diffusion filter does, with a high floor. Lit surfaces and mid-tones
   keep their level, and the surround and area cut reduce broad bright
   surfaces more than isolated lights. The glow of a clipped light takes
   the color of its fall-off (rim), and a little more color (vivid), so
   that a colored light does not glow pale or grey. */
static const Look bloom = {
    .fine = {400, 400, 400}, .strength = {144, 256, 576}, .cell = 14,
    .boost = 8, .floor = 200, .clip = 224, .knee = 40,
    .point = 8, .area = 224, .vivid = 320, .rim = 1,
};


/* The part (of 65536) of its light that a place gives with surround
   brightness `top` (the mean luma of the cells around, blurred wide): the
   darkness to the fourth power, so that a light source in the dark glows,
   and a wide bright area (sky, wall) and what is next to it much less. */
/* Strong lifts this weight in a scene up to 10 EV (lift_keep): a place
   gives at least its darkness squared, so that a lamp in a lit room or at
   dusk glows visibly. The lift fades out from 8 EV to 10 EV, so a daylight
   picture keeps the weight above. veil_guard keeps the lifted light of a
   large light area off the dark things next to it. */
INLINE u32 surround_weight(u32 top)
{
    u32 d = 255 - top, q = (d * d + 128) >> 8;
    return q * q + ((q * q) >> 7);
}

#define MAX_LIGHT 16320U /* the light of a channel: 255 times 64 */

/* For each level of the brightest channel of a pixel: the part (of 32768)
   of the light of each of its channels that glows. That is the light above
   Look.floor, with a soft start over 2 x Look.knee, over the light of the
   brightest channel. */
static void light_scale(const Look *look, u16 *scale, u16 *gain)
{
    u32 level, t = look->floor, k = look->knee;
    for (level = 0; level < 256; ++level) {
        u32 m = linear[level], e;
        if (m + k <= t) e = 0;
        else if (m >= t + k) e = m - t;
        else e = divide((m + k - t) * (m + k - t), 4 * k);
        scale[level] = (u16)(m ? divide(e << 15, m) : 0);
        /* Q8 smoothstep avoids a gain jump at clip. Keep the multiplier
           separate from scale so its fractional precision fits in u16. */
        u32 x = level > look->clip ? divide((level - look->clip) << 8, 255 - look->clip) : 0;
        u32 smooth = (x * x * (768 - 2 * x) + 32768) >> 16;
        gain[level] = (u16)(256 + (look->boost - 1) * smooth);
    }
}

/* R, G, B of a pixel scaled to the white of the effect (Fine.norm). */
#define WHITE_RGB(f, rgb) do { if ((f)->normalize) { (rgb)[0] = (f)->norm[0][(rgb)[0]]; \
    (rgb)[1] = (f)->norm[1][(rgb)[1]]; (rgb)[2] = (f)->norm[2][(rgb)[2]]; } } while (0)

/* The light of a look with `red` from the light of R, G, B. */
INLINE void red_light(const Look *look, u32 out[3])
{
    u32 l = out[0] + ((out[1] * look->red) >> 8);
    if (out[1] > out[0]) out[1] = out[0];
    out[0] = l > MAX_LIGHT ? MAX_LIGHT : l;
    out[2] = 0;
}

/* The light of a look with `neutral`: each channel that of the brightest. */
INLINE void neutral_light(u32 out[3])
{
    u32 top = out[0] > out[1] ? out[0] : out[1];
    top = top > out[2] ? top : out[2];
    out[0] = out[1] = out[2] = top;
}

/* The light of the channels of a pixel: their linear light, all scaled by
   the part that glows (light_scale), so the light keeps the hue of the
   pixel, with a continuous highlight gain (at most MAX_LIGHT).
   A look with `red` adds some green to red, takes green only up to red,
   and takes no blue. A look with `neutral` gives each channel the light of
   the brightest. */
INLINE void pixel_light(u32 r, u32 g, u32 b, const Look *look, const u16 *scale, const u16 *gain, u32 out[3])
{
    u32 top = r > g ? r : g, source_top, s, times;
    top = top > b ? top : b;
    source_top = top;
    if (look->chroma) {
        u32 low = r < g ? r : g;
        low = low < b ? low : b;
        if (top - low > CHROMA_FROM) top += ((top - low - CHROMA_FROM) * look->chroma) >> 7;
        if (top > 255) top = 255;
    }
    s = scale[top];
    if (!s) { out[0] = out[1] = out[2] = 0; return; }
    /* Saturation may admit a source below the white threshold, but does
       not prove that it is clipped. Keep highlight gain tied to its
       measured maximum channel so colored lights retain brightness steps. */
    times = gain[source_top];
    out[0] = ((((linear[r] * s + 16384) >> 15) * times) + 128) >> 8;
    out[1] = ((((linear[g] * s + 16384) >> 15) * times) + 128) >> 8;
    out[2] = ((((linear[b] * s + 16384) >> 15) * times) + 128) >> 8;
    if (look->red) red_light(look, out);
    else if (look->neutral) neutral_light(out);
}

/* a + b, at most 65535: a sum of the light of several pixels. */
INLINE u16 add_light(u32 a, u32 b) { a += b; return (u16)(a > 65535 ? 65535 : a); }



/* ---- source rows --------------------------------------------------------- */

/* The sums of a coarse cell in coarse_light: R, G, B; the light pixels (at
   Look.floor or above); for Look.gate the luma of the others; and for
   Look.rim the color of the unclipped bright pixels, each by its light. */
typedef struct {
    u32 r, g, b, lit, dim_luma;
    u32 rim[3], rim_weight;
} CellSums;

/* The most coarse cells (MAX_COARSE wide): their mean luma shares the ring
   with the sums of a cell row. */
#define MAX_CELLS 4096U

/* One buffer that the passes use in turn: clip_scan, then coarse_light,
   day_keep, and coarse_surround (the cells), then fine_feed (the rows). */
typedef union {
    u16 rows[RING][3][MAX_FINE]; /* RING source rows of light, R, G, B */
    struct {
        u8 column[MAX_CLIP_BLOCKS]; /* the map column of each block */
        u16 totals[MAX_CLIP_BLOCKS], sum[MAX_CLIP_COLUMNS], pixels[MAX_CLIP_COLUMNS];
        u8 counts[MAX_CLIP_BLOCKS * 16];
    } clip;
    struct { u8 up[MAX_CLIP_COLUMNS], mid[MAX_CLIP_COLUMNS], down[MAX_CLIP_COLUMNS]; } bright; /* the end of clip_scan */
    struct {
        CellSums sums[MAX_COARSE];
        u16 luma64[MAX_CELLS]; /* the mean luma of each cell times 64 */
    } cells;
} Ring;

/* The state of the light of one picture: the fine rows (a ring of RING
   source rows of light R, G, B, blurred along the row, and the next source
   row that goes into the ring), the light model of the look, and the maps
   of the cells and of the clip scan that fine_feed reads. */
typedef struct {
    u32 w, h, next, mono, shift; /* a fine pixel: 1 << shift (FINE_SHIFT) screennail pixels */
    u32 daylight; /* Q8 strength of local background rejection */
    u32 part[3]; /* Look.fine at the strength, after day_keep */
    const Plane *p;
    const Look *look;
    Ring ring;
    u16 light[3][MAX_FINE];
    u8 rgb[3][MAX_FINE]; /* a row of glow changes */
    u32 luma_row[MAX_SOURCE / 4 + 2], chroma_row[MAX_SOURCE / 4 + 2];
    u8 empty[RING]; /* a ring slot without light */
    /* The surround brightness of each cell (coarse_surround), and the
       cell column and fraction of each fine column. */
    const u8 *surround;
    u8 surround_column[MAX_FINE], surround_fraction[MAX_FINE];
    u32 surround_w, surround_h, surround_step;
    /* Look.rim: the color and confidence of each coarse cell (Coarse.rim),
       and the cells of the source row in fine_feed. */
    const u8 *rim, *rim_row;
    /* Strong: the part (of 256) of the lift that the scene EV keeps (0: no
       lift and no veil guard), the lift and the large light share of each
       cell (coarse_surround), and for each fine pixel of the ring rows the
       darkness of its receiver (of 255) blurred along the row, and the
       weight (of 64) of the receivers in that blur (veil_guard). */
    u32 lift_ev;
    const u8 *lift, *large;
    u8 dark[RING][MAX_FINE], dark_weight[RING][MAX_FINE];
    /* The work rows of fine_feed and receiver_row: the luma sum and the
       count of the receiver pixels of each fine pixel, and their darkness.
       They are here, not on the stack: Glow runs on the stack of the
       firmware JPEG encoder, which has little room. */
    u16 seen[MAX_FINE];
    u8 seen_n[MAX_FINE], seen_dark[MAX_FINE];
    u16 recip[65]; /* reciprocal(n) for n up to 64 */
    u32 hot; /* the lowest channel level with light */
    u16 scale[256]; /* light_scale of the look */
    u16 gain[256]; /* Q8 highlight gain, continuous from clip to white */
    u16 inverse[256]; /* 32768 / linear light of a level, 0 for none */
    /* The white of the effect (picture_white): each channel level scaled so
       that the white is 255, when `normalize` is set; and the main picture
       luma from which a pixel is clipped (CLIP_LEVEL of the white). */
    u8 norm[3][256];
    u32 normalize, clip_level;
    /* The clip map (clip_scan): clip_w x clip_h densities, then a flag for
       each map row with a density. 0 wide when there is none. */
    const u8 *clip;
    u32 clip_w, clip_h;
    u8 density[MAX_FINE]; /* of each fine column of the row in clip_density */
    u8 best[4][MAX_FINE]; /* R, G, B, and largest of the brightest pixel */
    /* A screennail row as R, G, B and the largest of them. */
    u8 source[4][MAX_SOURCE];
} Fine;

/* A row of light that adds nothing. */
static const u16 zero_row[MAX_FINE];

/* Copy n bytes of picture memory from `address` with word reads (a byte read
   of picture memory is slow), and return them. */
static const u8 *copy_row(u32 address, u32 n, u32 *buffer)
{
    u32 start = address & ~3U, words = (address - start + n + 3) >> 2, i;
    const u32 *from = WORDS(start);
    for (i = 0; i < words; ++i) buffer[i] = from[i];
    return (const u8 *)buffer + (address - start);
}

/* Screennail row y (from the top of the visible area): luma, and Cb Cr for
   each pair, both from the left of the visible area (chroma from the pair
   of the left pixel). */
static void source_row(Fine *f, u32 y, u32 n, const u8 **luma, const u8 **chroma)
{
    const Plane *p = f->p;
    u32 sy = p->top + y, sx = p->left;
    /* Include complete chroma pairs, without reading an extra pair past
       the final row when the visible width already ends on a pair. */
    u32 chroma_bytes = ((sx + n + 1) & ~1U) - (sx & ~1U);
    *luma = copy_row(p->y + sy * p->pitch + sx, n, f->luma_row);
    *chroma = copy_row(chroma_row(p, sy) + (sx & ~1U), chroma_bytes, f->chroma_row);
}

/* ---- clip map ---------------------------------------------------------- */

/* A count of the clip map holds at most CLIP_ROWS rows. */
#define CLIP_ROWS 255U


/* The counts of the rows since the last flush, added to the sum of each
   map column. */
static void clip_flush(u8 *counts, u16 *totals, const u8 *column, u16 *sum, u32 blocks)
{
    u32 b;
    glow_clip_total(counts, totals, blocks);
    for (b = 0; b < blocks; ++b) sum[column[b]] = (u16)(sum[column[b]] + totals[b]);
}

/* The clip map: for each cell of 1 << CLIP_SHIFT screennail pixels, the
   part (of 255) of the main picture pixels under it that are clipped, from
   every second main row and whole blocks of 16 pixels. Each byte of the
   blocks counts its clipped rows, and the counts go to the map columns
   once for each map row. A light source of a
   few main pixels, which the screennail mixes with its dark surround, shows
   here. A cell in a bright area (the mean of the 3 x 3 cells around it
   CLIP_AREA or more: a window, the sky) is 0: the screennail shows its
   light. The scan uses the ring of `f` (Ring.clip, then Ring.bright). */
static void clip_scan(Fine *f, const Plane *mp, u8 *map)
{
    const Plane *sp = f->p;
    u32 sw = sp->right - sp->left, sh = sp->bottom - sp->top;
    u32 mw = mp->right - mp->left, mh = mp->bottom - mp->top;
    u32 cw = f->clip_w, ch = f->clip_h, blocks = mw >> 4, b, c, y, row = 0, rows = 0, pending = 0;
    u8 *column = f->ring.clip.column, *any = map + cw * ch, *counts = f->ring.clip.counts;
    u16 *totals = f->ring.clip.totals, *sum = f->ring.clip.sum, *pixels = f->ring.clip.pixels;
    if (blocks > MAX_CLIP_BLOCKS) blocks = MAX_CLIP_BLOCKS;
    for (c = 0; c < cw; ++c) sum[c] = pixels[c] = 0;
    for (b = 0; b < blocks << 4; ++b) counts[b] = 0;
    for (b = 0; b < blocks; ++b) {
        c = divide(((b << 4) + 8) * sw, mw) >> CLIP_SHIFT;
        column[b] = (u8)(c < cw ? c : cw - 1);
        pixels[column[b]] = (u16)(pixels[column[b]] + 16);
    }
    for (y = 0;; y += 2) {
        /* Past the last row, r = ch flushes the last map rows. */
        u32 r = y < mh ? divide(y * sh, mh) >> CLIP_SHIFT : ch;
        if (r >= ch && y < mh) r = ch - 1;
        if (r != row || y >= mh) {
            /* The densities of map row `row`. */
            u32 lit = 0;
            if (pending) clip_flush(counts, totals, column, sum, blocks);
            pending = 0;
            for (c = 0; c < cw; ++c) {
                u32 total = rows * pixels[c], d = total ? divide(sum[c] * 255, total) : 0;
                map[row * cw + c] = (u8)(d > 255 ? 255 : d);
                lit |= d;
                sum[c] = 0;
            }
            any[row] = (u8)(lit != 0);
            while (++row < r) {
                for (c = 0; c < cw; ++c) map[row * cw + c] = 0;
                any[row] = 0;
            }
            rows = 0;
            if (y >= mh) break;
        }
        {
            const u8 *line = BYTES(mp->y + (mp->top + y) * mp->pitch + mp->left);
            glow_clip(line, counts, blocks, f->clip_level);
            ++rows;
            if (++pending == CLIP_ROWS) {
                clip_flush(counts, totals, column, sum, blocks);
                pending = 0;
            }
        }
    }
    {
        /* Bright areas out: rows r - 1, r, r + 1 as they were, in the ring. */
        u8 *up = f->ring.bright.up, *mid = f->ring.bright.mid, *down = f->ring.bright.down, *t;
        for (c = 0; c < cw; ++c) up[c] = mid[c] = map[c];
        for (row = 0; row < ch; ++row) {
            u32 lit = 0;
            for (c = 0; c < cw; ++c) down[c] = row + 1 < ch ? map[(row + 1) * cw + c] : mid[c];
            if (any[row])
                for (c = 0; c < cw; ++c) {
                    u32 l = c ? c - 1 : 0, r = c + 1 < cw ? c + 1 : c;
                    u32 a = up[l] + up[c] + up[r] + mid[l] + mid[c] + mid[r] + down[l] + down[c] + down[r];
                    /* Fade point recovery into the ordinary source as a cluster
                       becomes a broad bright area; do not cut it in one step. */
                    u32 start = 9 * CLIP_AREA / 2, end = 9 * CLIP_AREA;
                    u8 d = a >= end ? 0 : a <= start ? mid[c] : (u8)divide(mid[c] * (end - a), end - start);
                    map[row * cw + c] = d;
                    lit |= d;
                }
            any[row] = (u8)(lit != 0);
            t = up; up = mid; mid = down; down = t;
        }
    }
}

/* The mean clip density of the map cells under screennail columns x0 to
   x1 - 1 and rows y0 to y1 - 1. */
static u32 clip_density(const Fine *f, u32 x0, u32 x1, u32 y0, u32 y1)
{
    u32 c0 = x0 >> CLIP_SHIFT, c1 = (x1 - 1) >> CLIP_SHIFT, r0 = y0 >> CLIP_SHIFT, r1 = (y1 - 1) >> CLIP_SHIFT, c, r, s = 0, n;
    if (!f->clip_w || c0 >= f->clip_w || r0 >= f->clip_h) return 0;
    if (c1 >= f->clip_w) c1 = f->clip_w - 1;
    if (r1 >= f->clip_h) r1 = f->clip_h - 1;
    for (r = r0; r <= r1; ++r)
        if (f->clip[f->clip_w * f->clip_h + r])
            for (c = c0; c <= c1; ++c) s += f->clip[r * f->clip_w + c];
    n = (c1 - c0 + 1) * (r1 - r0 + 1);
    return n == 1 ? s : n == 2 ? s >> 1 : n == 4 ? s >> 2 : divide(s, n);
}

/* The light of a light source that the screennail mixes with its dark
   surround: `density` (of 255) of the main pixels there are clipped, and
   each gives Look.point times the light of a clipped channel, in the hue
   of the brightest screennail pixel there (r, g, b, its largest top). */
static void point_light(const Fine *f, u32 density, u32 r, u32 g, u32 b, u32 top, u32 out[3])
{
    const Look *look = f->look;
    u32 t = (density * look->point * (255 - look->floor)) >> 8, inv = f->inverse[top];
    if (t > MAX_LIGHT) t = MAX_LIGHT;
    if (!inv) {
        out[0] = out[1] = out[2] = t;
    } else {
        out[0] = (t * ((linear[r] * inv) >> 7)) >> 8;
        out[1] = (t * ((linear[g] * inv) >> 7)) >> 8;
        out[2] = (t * ((linear[b] * inv) >> 7)) >> 8;
    }
    if (look->red) red_light(look, out);
    else if (look->neutral) neutral_light(out);
}

/* R, G, B and their largest of n screennail pixels of a row into
   f->source: NEON for each 16 when the chroma pairs start at the first
   pixel, C for the rest. */
static void pixel_row(Fine *f, const u8 *row, const u8 *crow, u32 n)
{
    u32 x, sx = f->p->left;
    if (!f->mono && !(sx & 1)) {
        GlowRgb out = {f->source[0], f->source[1], f->source[2], f->source[3]};
        glow_rgb(row, crow, &out, n);
        return;
    }
    for (x = 0; x < n; ++x) {
        u32 q = ((sx + x) & ~1U) - (sx & ~1U), rgb[3], top;
        pixel_rgb(row[x], crow[q], crow[q + 1], f->mono, rgb);
        f->source[0][x] = (u8)rgb[0];
        f->source[1][x] = (u8)rgb[1];
        f->source[2][x] = (u8)rgb[2];
        top = rgb[0] > rgb[1] ? rgb[0] : rgb[1];
        f->source[3][x] = (u8)(top > rgb[2] ? top : rgb[2]);
    }
}

/* pixel_row, scaled to the white of the effect. */
static void source_rgb(Fine *f, const u8 *row, const u8 *crow, u32 n)
{
    u32 x;
    pixel_row(f, row, crow, n);
    if (f->normalize)
        for (x = 0; x < n; ++x) {
            u32 r = f->norm[0][f->source[0][x]], g = f->norm[1][f->source[1][x]], b = f->norm[2][f->source[2][x]], top;
            f->source[0][x] = (u8)r;
            f->source[1][x] = (u8)g;
            f->source[2][x] = (u8)b;
            top = r > g ? r : g;
            f->source[3][x] = (u8)(top > b ? top : b);
        }
}

/* The integer square root of v. */
static u32 isqrt(u32 v)
{
    u32 r = 0, b = 1U << 30;
    while (b > v) b >>= 2;
    while (b) {
        if (v >= r + b) { v -= r + b; r = (r >> 1) + b; }
        else r >>= 1;
        b >>= 2;
    }
    return r;
}

/* ---- coarse layer ------------------------------------------------------ */

typedef struct {
    u32 w, h, step, recip; /* recip: 65536 / the pixels taken of a cell */
    u8 *surround; /* Look.area: the cut of each cell; then the surround brightness */
    u8 *around; /* Look.gate: the luma of the pixels below the floor (times dim) */
    u8 *dim; /* Look.gate: the part of the cell below the floor (of 255) */
    u8 *rim; /* Look.rim: R, G, B (the largest 255) and confidence of the light of each cell */
    u8 *lift; /* Strong: the surround weight (of 255) that the lift gives each cell */
    u8 *large; /* Strong: the light part of each cell (of 255); then its large light share */
} Coarse;

/* The cells: the mean luma of each (for the surround and day_keep), the
   cut of a mostly light cell (Look.area), the luma around the light
   (Look.gate), and the color of its light (Look.rim). The sums of a cell
   row go in the ring of `f`, which is not in use yet. */
static void coarse_light(Fine *f, Coarse *m, const Look *look)
{
    u32 cy, cx, i, j, sx = f->p->left, n = m->w * m->step, mono = f->mono, whole = !mono && !(sx & 1);
    /* The pixels taken of a cell: up to 255 (a step up to 30; the camera's
       screennail gives about 10) they count; with more, area and the gate
       make no cut. */
    u32 taken = ((m->step + 1) >> 1) * ((m->step + 1) >> 1), counted = taken <= 255;
    CellSums *sums = f->ring.cells.sums;
    u16 *luma64 = f->ring.cells.luma64;
    for (cy = 0; cy < m->h; ++cy) {
        for (cx = 0; cx < m->w; ++cx) {
            CellSums *c = sums + cx;
            c->r = c->g = c->b = c->lit = c->dim_luma = 0;
            c->rim[0] = c->rim[1] = c->rim[2] = c->rim_weight = 0;
        }
        /* Every second row and column of the cell: the cells are blurred
           wide, so a quarter of the pixels gives the same means. With the
           chroma pairs at the first pixel, the whole row goes through
           pixel_row (NEON) first. */
        for (j = 0; j < m->step; j += 2) {
            const u8 *row, *crow;
            source_row(f, cy * m->step + j, n, &row, &crow);
            if (whole) pixel_row(f, row, crow, n);
            for (cx = 0; cx < m->w; ++cx) {
                CellSums *c = sums + cx;
                u32 x = cx * m->step;
                for (i = 0; i < m->step; i += 2, x += 2) {
                    u32 q = ((sx + x) & ~1U) - (sx & ~1U), rgb[3], top;
                    if (whole) {
                        rgb[0] = f->source[0][x];
                        rgb[1] = f->source[1][x];
                        rgb[2] = f->source[2][x];
                    } else {
                        pixel_rgb(row[x], crow[q], crow[q + 1], mono, rgb);
                    }
                    WHITE_RGB(f, rgb);
                    top = rgb[0] > rgb[1] ? rgb[0] : rgb[1];
                    top = top > rgb[2] ? top : rgb[2];
                    if (look->rim && top >= RIM_LOW && top < RIM_FROM) {
                        /* The color of the light: unclipped pixels, the
                           brighter the more. */
                        u32 w = linear[top];
                        c->rim[0] += linear[rgb[0]] * w;
                        c->rim[1] += linear[rgb[1]] * w;
                        c->rim[2] += linear[rgb[2]] * w;
                        c->rim_weight += w;
                    }
                    if (linear[top] >= look->floor) ++c->lit;
                    else if (look->gate) c->dim_luma += (77 * rgb[0] + 150 * rgb[1] + 29 * rgb[2]) >> 8;
                    c->r += rgb[0];
                    c->g += rgb[1];
                    c->b += rgb[2];
                }
            }
        }
        for (cx = 0; cx < m->w; ++cx) {
            const CellSums *c = sums + cx;
            u32 r = (c->r * m->recip) >> 16, g = (c->g * m->recip) >> 16, b = (c->b * m->recip) >> 16;
            if (look->area) {
                /* The part of the cell that is light (of 256), and the cut
                   of a mostly light cell, kept for coarse_surround. */
                u32 part = (c->lit * m->recip) >> 8, keep = 255;
                if (counted && part > 64) keep = 255 - divide(look->area * (part > 256 ? 192 : part - 64), 192);
                m->surround[cy * m->w + cx] = (u8)keep;
            }
            if (m->large) {
                /* The part of the cell that is light (of 255), for the
                   large light share of the veil guard. */
                u32 part = (c->lit * m->recip) >> 8;
                m->large[cy * m->w + cx] = (u8)(part > 255 ? 255 : part);
            }
            if (look->gate) {
                /* The mean luma of the pixels below the floor times their
                   part of the cell, so that the cells around add up by
                   their pixels below the floor. */
                u32 dim = counted ? taken - c->lit : 0, part = (dim * m->recip) >> 8;
                part = part > 255 ? 255 : part;
                m->dim[cy * m->w + cx] = (u8)part;
                m->around[cy * m->w + cx] = (u8)(dim ? (divide(c->dim_luma, dim) * part + 128) >> 8 : 0);
            }
            /* The mean luma, for the surround and day_keep. */
            luma64[cy * m->w + cx] = (u16)(((77 * r + 150 * g + 29 * b) >> 8) << 6);
            if (look->rim) {
                u8 *q = m->rim + 4 * (cy * m->w + cx);
                u32 top = c->rim[0] > c->rim[1] ? c->rim[0] : c->rim[1];
                top = top > c->rim[2] ? top : c->rim[2];
                q[0] = q[1] = q[2] = q[3] = 0;
                if (c->rim_weight >= RIM_MIN && top) {
                    /* Each channel of 255 at the largest: the sums reach
                       about 2^24, so first take them down to 8 bits. */
                    u32 d = (top >> 8) + 1, t8 = divide(top, d);
                    q[0] = (u8)divide(divide(c->rim[0], d) * 255, t8);
                    q[1] = (u8)divide(divide(c->rim[1], d) * 255, t8);
                    q[2] = (u8)divide(divide(c->rim[2], d) * 255, t8);
                    q[3] = 255;
                }
            }
        }
    }
}

/* Look.rim: spread the color of each cell into the cells around it that
   have none (the clipped inside of a light), forward and back like a
   distance transform. A cell takes the color of its most confident
   neighbor, with RIM_DECAY less confidence, so the color reaches only a
   few cells from where the camera shows it. */
static void rim_take(u8 *c, const u8 *n)
{
    if (n[3] > RIM_DECAY && n[3] - RIM_DECAY > c[3]) {
        c[0] = n[0]; c[1] = n[1]; c[2] = n[2]; c[3] = (u8)(n[3] - RIM_DECAY);
    }
}

static void rim_fill(const Coarse *m)
{
    u32 x, y, w = m->w, h = m->h;
    u8 *r = m->rim;
    for (y = 0; y < h; ++y)
        for (x = 0; x < w; ++x) {
            u8 *c = r + 4 * (y * w + x);
            if (c[3] == 255) continue;
            if (x) rim_take(c, c - 4);
            if (y) {
                rim_take(c, c - 4 * w);
                if (x) rim_take(c, c - 4 * w - 4);
                if (x + 1 < w) rim_take(c, c - 4 * w + 4);
            }
        }
    for (y = h; y-- > 0;)
        for (x = w; x-- > 0;) {
            u8 *c = r + 4 * (y * w + x);
            if (c[3] == 255) continue;
            if (x + 1 < w) rim_take(c, c + 4);
            if (y + 1 < h) {
                rim_take(c, c + 4 * w);
                if (x + 1 < w) rim_take(c, c + 4 * w + 4);
                if (x) rim_take(c, c + 4 * w - 4);
            }
        }
}

/* The level of linear light v (0-255): the lowest with at least v. */
static u32 level_of_light(u32 v)
{
    u32 lo = 0, hi = 255;
    while (lo < hi) {
        u32 mid = (lo + hi) >> 1;
        if (linear[mid] >= v) hi = mid;
        else lo = mid + 1;
    }
    return lo;
}

/* Look.rim: a pixel near white, R, G, B in rgb with its largest `top`,
   takes the color of the light of its cell `q` (Coarse.rim), from
   RIM_FROM to all at 255, by the confidence of that color. Its brightest
   level stays. */
static void rim_rgb(const u8 *q, u32 top, u32 rgb[3])
{
    u32 t, k, l = linear[top];
    if (!q[3] || top < RIM_FROM) return;
    t = divide((top - RIM_FROM + 1) << 8, 256 - RIM_FROM);
    t = (t * q[3] + 128) >> 8;
    for (k = 0; k < 3; ++k) {
        u32 own = linear[rgb[k]], hue = (l * q[k] * 257 + 32768) >> 16;
        rgb[k] = level_of_light((own * (256 - t) + hue * t + 128) >> 8);
    }
}

/* A recursive blur forward and back along rows, then columns. `keep` (of
   256) is the part of the previous cell that each cell keeps. */
static void blur(u16 *map, u32 w, u32 h, u32 keep)
{
    u32 x, y, v;
    for (y = 0; y < h; ++y) {
        u16 *row = map + y * w;
        v = (u32)row[0] << 8;
        for (x = 0; x < w; ++x) { v = ((v * keep) >> 8) + (u32)row[x] * (256 - keep); row[x] = (u16)((v + 128) >> 8); }
        v = (u32)row[w - 1] << 8;
        for (x = w; x-- > 0;) { v = ((v * keep) >> 8) + (u32)row[x] * (256 - keep); row[x] = (u16)((v + 128) >> 8); }
    }
    for (x = 0; x < w; ++x) {
        v = (u32)map[x] << 8;
        for (y = 0; y < h; ++y) { u16 *c = map + y * w + x; v = ((v * keep) >> 8) + (u32)*c * (256 - keep); *c = (u16)((v + 128) >> 8); }
        v = (u32)map[(h - 1) * w + x] << 8;
        for (y = h; y-- > 0;) { u16 *c = map + y * w + x; v = ((v * keep) >> 8) + (u32)*c * (256 - keep); *c = (u16)((v + 128) >> 8); }
    }
}

/* The blur of `blur` for a map of bytes. Each pass rounds to a byte, which
   is enough for the light density that it spreads. */
static void blur_bytes(u8 *map, u32 w, u32 h, u32 keep)
{
    u32 x, y, v;
    for (y = 0; y < h; ++y) {
        u8 *row = map + y * w;
        v = (u32)row[0] << 8;
        for (x = 0; x < w; ++x) { v = ((v * keep) >> 8) + (u32)row[x] * (256 - keep); row[x] = (u8)((v + 128) >> 8); }
        v = (u32)row[w - 1] << 8;
        for (x = w; x-- > 0;) { v = ((v * keep) >> 8) + (u32)row[x] * (256 - keep); row[x] = (u8)((v + 128) >> 8); }
    }
    for (x = 0; x < w; ++x) {
        v = (u32)map[x] << 8;
        for (y = 0; y < h; ++y) { u8 *c = map + y * w + x; v = ((v * keep) >> 8) + (u32)*c * (256 - keep); *c = (u8)((v + 128) >> 8); }
        v = (u32)map[(h - 1) * w + x] << 8;
        for (y = h; y-- > 0;) { u8 *c = map + y * w + x; v = ((v * keep) >> 8) + (u32)*c * (256 - keep); *c = (u8)((v + 128) >> 8); }
    }
}

/* 1 2 1 along the rows, then along the columns: the cuts of Look.area over
   the cells around. */
static void smooth_cells(u8 *a, u32 w, u32 h)
{
    u8 up[MAX_COARSE];
    u32 x, y;
    for (y = 0; y < h; ++y) {
        u8 *row = a + y * w;
        u32 prev = row[0];
        for (x = 0; x < w; ++x) {
            u32 cur = row[x], next = x + 1 < w ? row[x + 1] : cur;
            row[x] = (u8)((prev + 2 * cur + next + 2) >> 2);
            prev = cur;
        }
    }
    for (x = 0; x < w; ++x) up[x] = a[x];
    for (y = 0; y < h; ++y)
        for (x = 0; x < w; ++x) {
            u32 cur = a[y * w + x], down = y + 1 < h ? a[(y + 1) * w + x] : cur;
            a[y * w + x] = (u8)((up[x] + 2 * cur + down + 2) >> 2);
            up[x] = (u8)cur;
        }
}

/* Surround brightness `s` with its darkness weight scaled by `keep` (of
   256): the darkness times the fourth root of keep, which the fourth power
   of surround_weight undoes. */
static u32 surround_cut(u32 s, u32 keep)
{
    u32 root = isqrt(isqrt(keep << 8) << 8);
    return 255 - (((255 - s) * root + 128) >> 8);
}

/* The surround brightness of each cell: its mean luma, blurred wide, made
   brighter by the cuts of Look.area and the local daylight gate, so that
   surround_weight applies them in fine_feed. With Strong, also the lift of
   each cell (surround_weight) and its large light share (veil_guard). */
static void coarse_surround(Fine *f, Coarse *m, const Look *look)
{
    u16 *luma64 = f->ring.cells.luma64;
    u32 i, n = m->w * m->h;
    blur(luma64, m->w, m->h, 236);
    blur(luma64, m->w, m->h, 236);
    if (look->area) {
        smooth_cells(m->surround, m->w, m->h);
        smooth_cells(m->surround, m->w, m->h);
    }
    if (look->gate) {
        smooth_cells(m->around, m->w, m->h);
        smooth_cells(m->around, m->w, m->h);
        smooth_cells(m->dim, m->w, m->h);
        smooth_cells(m->dim, m->w, m->h);
    }
    if (m->lift) {
        blur_bytes(m->large, m->w, m->h, LARGE_KEEP);
        blur_bytes(m->large, m->w, m->h, LARGE_KEEP);
    }
    for (i = 0; i < n; ++i) {
        u32 s = (luma64[i] + 32) >> 6, dark = 255 - s, area = look->area ? m->surround[i] : 255, keep = 256;
        /* Look.area: a mostly light cell counts as a brighter surround, by
           the fourth root of its cut, so that the surround weight (darkness
           to the fourth power) applies the cut once. */
        if (area < 255) s = surround_cut(s, area);
        if (f->daylight) {
            /* Use only non-highlight pixels around the source. A dark
               surround still admits a specular highlight in daylight.
               An entirely bright neighborhood supplies no such evidence. */
            u32 a = m->dim[i] ? divide((u32)m->around[i] << 8, m->dim[i]) : 255;
            u32 local = a <= 96 ? 256 : a >= 208 ? 0 : divide((208 - a) << 8, 112);
            keep = 256 - ((f->daylight * (256 - local) + 128) >> 8);
            s = surround_cut(s, keep);
        }
        if (m->lift) {
            /* The lift: the darkness squared, with the same cuts, each once.
               A light area is large from LARGE_FROM of blurred density. */
            u32 w = (dark * dark + 127) / 255, d = m->large[i];
            w = (((w * area + 127) / 255) * keep) >> 8;
            m->lift[i] = (u8)((w * f->lift_ev) >> 8);
            m->large[i] = (u8)(d <= LARGE_FROM ? 0 : d >= LARGE_FROM + LARGE_RANGE ? 255 : (d - LARGE_FROM) * 255 / LARGE_RANGE);
        }
        m->surround[i] = (u8)s;
    }
}

/* ---- fine layer -------------------------------------------------------- */

/* The two cell rows of the coarse maps around fine row y, and the fraction
   (of 256) of the lower one. */
static u32 cell_rows(const Fine *f, u32 y, u32 *c0, u32 *c1)
{
    int pos = (int)divide((((y << 1) + 1) << f->shift) << 7, f->surround_step) - 128;
    u32 cy;
    if (pos < 0) pos = 0;
    cy = (u32)pos >> 8;
    *c0 = cy < f->surround_h ? cy : f->surround_h - 1;
    *c1 = *c0 + 1 < f->surround_h ? *c0 + 1 : *c0;
    return cy < f->surround_h ? (u32)pos & 255 : 0;
}

/* A coarse map at fine column x, between the cell rows t0 and t1 (fraction
   cf of t1), of 255. */
INLINE u32 cell_value(const Fine *f, const u8 *t0, const u8 *t1, u32 cf, u32 x)
{
    u32 ci = f->surround_column[x], cj = ci + 1 < f->surround_w ? ci + 1 : ci, fx = f->surround_fraction[x];
    u32 a = t0[ci] * (256 - fx) + t0[cj] * fx, b = t1[ci] * (256 - fx) + t1[cj] * fx;
    return (a * (256 - cf) + b * cf + 32768) >> 16;
}

/* Source row y into its ring slot: the light of each fine pixel, at least
   that of the point sources in it (point_light), blurred along the row
   with 1 6 15 20 15 6 1, over 16 (at most 16320 x 4). A row out of the
   picture is 0. Fine sampling is independent of the coarse grid, which
   can miss an isolated light between its samples. */
/* The binomial taps of the fine blur (sum 64). */
static const u8 taps[RING] = {1, 6, 15, 20, 15, 6, 1};
/* 65536 / n, rounded, for the pixel counts and the tap weights of
   receiver_row: the camera has no divide instruction. */
static u16 reciprocal(u32 n) { return (u16)(n > 1 ? divide(65536U + (n >> 1), n) : 65535U); }

/* Strong: the receivers of fine row `slot` for veil_guard. The receiver of a
   fine pixel is the mean screennail luma of its pixels below VEIL_SOURCE:
   the light itself is no receiver, so a fine pixel across the edge of a
   light area is as dark as the dark side of the edge. A fine pixel with
   only light pixels has no receiver. The darkness of the receivers is
   blurred along the row with the fine taps, over the fine pixels that have
   a receiver: so the cut changes smoothly from one fine pixel to the next,
   and a light does not lessen the cut next to it. */
static void receiver_row(Fine *f, u32 slot, const u16 *seen, const u8 *seen_n)
{
    u8 *dark = f->seen_dark;
    u32 x, i;
    for (x = 0; x < f->w; ++x) {
        u32 n = seen_n[x], y = n ? (seen[x] * f->recip[n] + 32768U) >> 16 : 0;
        /* (VEIL_LIGHT - y) * 255 / (VEIL_LIGHT - VEIL_DARK), with 255 / 90 as 2901 / 1024 */
        dark[x] = (u8)(!n || y >= VEIL_LIGHT ? 0 : y <= VEIL_DARK ? 255 : ((VEIL_LIGHT - y) * 2901U + 512U) >> 10);
    }
    for (x = 0; x < f->w; ++x) {
        u32 sum = 0, weight = 0;
        for (i = 0; i < RING; ++i) {
            u32 c = x + i;
            if (c < 3 || c - 3 >= f->w || !seen_n[c - 3]) continue;
            sum += taps[i] * dark[c - 3];
            weight += taps[i];
        }
        f->dark[slot][x] = (u8)(weight ? (sum * f->recip[weight] + 32768U) >> 16 : 0);
        f->dark_weight[slot][x] = (u8)weight;
    }
}

static void fine_feed(Fine *f, u32 y)
{
    u32 x, k, slot = y % RING;
    if (y >= f->h) {
        if (f->empty[slot]) return;
        for (k = 0; k < 3; ++k)
            for (x = 0; x < f->w; ++x) f->ring.rows[slot][k][x] = 0;
        f->empty[slot] = 1;
        return;
    }
    {
        /* Average every screennail pixel in the fine cell. Subsampling here
           makes small lights disappear at particular pixel offsets. Only
           a pixel with a channel at f->hot or above has light. */
        u32 n = 1U << f->shift, j;
        u32 shift = f->shift << 1, any = 0, width = f->w << f->shift;
        u32 hot = f->hot ? (f->hot - 1) * 0x01010101U : 0, points = 0, lit_channel[3];
        u8 *best0 = f->best[0], *best1 = f->best[1], *best2 = f->best[2], *best3 = f->best[3];
        u16 *seen = f->seen; /* Strong: the luma sum of the receiver pixels of each fine pixel (veil_guard) */
        u8 *seen_n = f->seen_n;
        for (k = 0; k < 3; ++k)
            for (x = 0; x < f->w; ++x) f->light[k][x] = 0;
        /* The clip density of each fine pixel, and its brightest pixel. */
        if (f->clip_w) {
            u32 r0 = (y << f->shift) >> CLIP_SHIFT, r1 = (((y + 1) << f->shift) - 1) >> CLIP_SHIFT;
            for (; r0 <= r1 && r0 < f->clip_h; ++r0) points |= f->clip[f->clip_w * f->clip_h + r0];
        }
        if (points) {
            points = 0;
            for (x = 0; x < f->w; ++x) {
                u32 d = clip_density(f, x << f->shift, (x + 1) << f->shift, y << f->shift, (y + 1) << f->shift);
                f->density[x] = (u8)d;
                best3[x] = 0;
                points |= d;
            }
        }
        if (f->lift_ev)
            for (x = 0; x < f->w; ++x) { seen[x] = 0; seen_n[x] = 0; }
        for (j = 0; j < n; ++j) {
            const u8 *row, *crow;
            source_row(f, (y << f->shift) + j, width, &row, &crow);
            /* The receiver: every second pixel of every second row is
               enough for the mean luma of a fine pixel. */
            if (f->lift_ev && !(j & 1))
                for (x = 0; x < width; x += 2)
                    if (row[x] < VEIL_SOURCE) {
                        seen[x >> f->shift] = (u16)(seen[x >> f->shift] + row[x]);
                        ++seen_n[x >> f->shift];
                    }
            if (f->rim) {
                u32 cy = divide((y << f->shift) + j, f->surround_step);
                f->rim_row = f->rim + 4 * f->surround_w * (cy < f->surround_h ? cy : f->surround_h - 1);
            }
            {
                const u8 *r = f->source[0], *g = f->source[1], *b = f->source[2];
                const u32 *top = (const u32 *)(const void *)f->source[3];
                source_rgb(f, row, crow, width);
                if (points)
                    for (x = 0; x < f->w; ++x) {
                        u32 c;
                        if (!f->density[x]) continue;
                        for (c = x << f->shift; c < (x + 1) << f->shift; ++c)
                            if (f->source[3][c] >= best3[x]) {
                                best0[x] = r[c]; best1[x] = g[c]; best2[x] = b[c]; best3[x] = f->source[3][c];
                                if (f->rim) {
                                    /* The brightest pixel of a light source
                                       is the one most often clipped. */
                                    u32 q[3] = {r[c], g[c], b[c]};
                                    rim_rgb(f->rim_row + 4 * f->surround_column[x], best3[x], q);
                                    best0[x] = (u8)q[0]; best1[x] = (u8)q[1]; best2[x] = (u8)q[2];
                                }
                            }
                    }
                /* Light: four pixels at a time, only where one is hot. */
                for (x = 0; x < width; x += 4) {
                    u32 c;
                    if (hot && !uqsub8(top[x >> 2], hot)) continue;
                    for (c = x; c < x + 4 && c < width; ++c) {
                        u32 fx = c >> f->shift;
                        u32 l[3];
                        if (f->rim && f->source[3][c] >= RIM_FROM) {
                            u32 q[3] = {r[c], g[c], b[c]};
                            rim_rgb(f->rim_row + 4 * f->surround_column[fx], f->source[3][c], q);
                            pixel_light(q[0], q[1], q[2], f->look, f->scale, f->gain, l);
                        } else {
                            pixel_light(r[c], g[c], b[c], f->look, f->scale, f->gain, l);
                        }
                        f->light[0][fx] = add_light(f->light[0][fx], l[0]);
                        f->light[1][fx] = add_light(f->light[1][fx], l[1]);
                        f->light[2][fx] = add_light(f->light[2][fx], l[2]);
                    }
                }
            }
        }
        for (k = 0; k < 3; ++k)
            for (x = 0; x < f->w; ++x) f->light[k][x] = (u16)(f->light[k][x] >> shift);
        if (f->lift_ev) receiver_row(f, slot, seen, seen_n);
        if (points)
            for (x = 0; x < f->w; ++x) {
                u32 l[3];
                if (!f->density[x]) continue;
                point_light(f, f->density[x], best0[x], best1[x], best2[x], best3[x], l);
                for (k = 0; k < 3; ++k)
                    if (l[k] > f->light[k][x]) f->light[k][x] = (u16)l[k];
            }
        for (k = 0; k < 3; ++k) {
            u32 a = 0;
            for (x = 0; x < f->w; ++x) a |= f->light[k][x];
            lit_channel[k] = a;
            any |= a;
        }
        f->empty[slot] = (u8)!any;
        if (any) {
            /* The surround at this row: the coarse map, between two cell
               rows. With Strong, the larger of the surround weight and the
               lift. */
            u32 c0, c1, cf = cell_rows(f, y, &c0, &c1), row0 = c0 * f->surround_w, row1 = c1 * f->surround_w;
            for (x = 0; x < f->w; ++x) {
                u32 w = surround_weight(cell_value(f, f->surround + row0, f->surround + row1, cf, x));
                if (f->lift_ev) {
                    u32 lift = cell_value(f, f->lift + row0, f->lift + row1, cf, x) * 257;
                    if (lift > w) w = lift;
                }
                for (k = 0; k < 3; ++k) f->light[k][x] = (u16)((f->light[k][x] * w) >> 16);
            }
        }
        /* A channel without light blurs to 0 (Halation has no blue). The
           columns from 3 to w - 4 have all their taps. */
        for (k = 0; k < 3; ++k) {
            const u16 *s = f->light[k];
            u16 *o = f->ring.rows[slot][k];
            u32 inner = f->w > 6 ? f->w - 3 : 3;
            if (!lit_channel[k]) {
                for (x = 0; x < f->w; ++x) o[x] = 0;
                continue;
            }
            for (x = 3; x < inner; ++x)
                o[x] = (u16)((s[x - 3] + s[x + 3] + 6 * (s[x - 2] + s[x + 2]) + 15 * (s[x - 1] + s[x + 1]) + 20 * s[x]) >> 4);
            for (x = 0; x < f->w; x = x == 2 && inner > 3 ? inner : x + 1) {
                u32 a = x >= 3 ? s[x - 3] : 0, b = x >= 2 ? s[x - 2] : 0, c = x >= 1 ? s[x - 1] : 0;
                u32 d = x + 1 < f->w ? s[x + 1] : 0, e = x + 2 < f->w ? s[x + 2] : 0, g = x + 3 < f->w ? s[x + 3] : 0;
                o[x] = (u16)((a + g + 6 * (b + e) + 15 * (c + d) + 20 * s[x]) >> 4);
            }
        }
    }
}

/* ---- the rows of the glow ----------------------------------------------- */

/* For one fine row: Y, Cb, Cr changes (times 128) at each fine column. */
typedef struct {
    s16 v[3][MAX_GRID];
    u8 protect[MAX_GRID]; /* the part (of 255) of the light of this sample from a Look.protect stream */
    u32 zero; /* all changes are 0 */
} GlowRow;

/* The composite of one plane: its words, rows, and the per-word state: the
   value and the step of each word (Y, Cb, Cr), the fine column and fraction
   of each word, and the runs of glowing blocks. */
typedef struct {
    const Plane *p;
    u32 x0, words, y, step_y, scale_y, runs, offset;
    s16 *value[3], *step[3];
    u16 *column, *run_first, *run_count;
    u8 *fraction, *original_chroma, *protect;
} Lane;

/* The state of the composite: the words of all planes, the coarse column
   and fraction of each fine column, and the glow rows of the two fine rows
   around the current rows. */
typedef struct {
    s16 value[3][ALL_WORDS], step[3][ALL_WORDS];
    u16 column[ALL_WORDS];
    u8 fraction[ALL_WORDS], protect[ALL_WORDS];
    /* Only half-height color planes reserve original-row storage. */
    u8 *original_chroma;
    u32 chroma_bytes, chroma_used;
    u16 run_first[ALL_BLOCKS], run_count[ALL_BLOCKS];
    GlowRow rows[2];
    Lane lanes[4];
    /* The glow rows of two fine rows (`fine_row`), from which the composite
       makes its rows; the grid of the composite (`grid_w` x `grid_h` glow
       values, twice the fine rows and columns, from a quarter of a fine
       pixel in, at offset GRID_OFFSET of 4096 from the first pixel center). */
    GlowRow *fine[2];
    u32 fine_row[2], fine_glows[2], grid_w, grid_h;
} Rows;


/* Strong: less glow on a dark receiver next to a large light area (a
   backlit window or a white wall in shade beside a dark coat), where the
   lift would otherwise lay a veil. A small light keeps its glow, and so
   does a light receiver (the wall around a lamp). The share of a large
   light area comes from the cells. The darkness of the receivers is
   blurred down the ring rows as along the row (receiver_row), over the
   fine pixels that have a receiver, so the cut follows the receivers
   smoothly and does not draw the fine pixels in the glow. */
static void veil_guard(Fine *f, u32 t)
{
    u32 x, j, k, c0, c1, cf;
    if (t >= f->h) return;
    cf = cell_rows(f, t, &c0, &c1);
    for (x = 0; x < f->w; ++x) {
        u32 large = cell_value(f, f->large + c0 * f->surround_w, f->large + c1 * f->surround_w, cf, x);
        u32 sum = 0, weight = 0, dark, keep;
        if (!large) continue;
        for (j = 0; j < RING; ++j) {
            u32 row = t + j, w;
            if (row < 3 || row - 3 >= f->h) continue;
            w = taps[j] * f->dark_weight[(row - 3) % RING][x];
            sum += w * f->dark[(row - 3) % RING][x];
            weight += w;
        }
        if (!weight) continue;
        dark = weight == 64 * 64 ? (sum + 2048) >> 12 : divide(sum + (weight >> 1), weight);
        keep = 256 - ((((large * dark) >> 8) * VEIL_CUT) >> 8);
        if (keep < 256)
            for (k = 0; k < 3; ++k) f->light[k][x] = (u16)((f->light[k][x] * keep) >> 8);
    }
}

/* Fine row t as linear light, blurred down the ring columns. An empty
   ring returns zero without reading or changing the light buffers. */
static u32 light_row(Fine *f, u32 t)
{
    u32 k, j;
    const u16 *light[RING][3];
    /* The ring holds source rows t - 3 to t + 3. */
    while (f->next <= t + 3) {
        fine_feed(f, f->next);
        ++f->next;
    }
    /* No light in the ring rows: no change. */
    {
        u32 empty = 1;
        for (j = 0; j < RING && empty; ++j) {
            u32 row = t + j;
            if (row >= 3 && row - 3 < f->h) empty = f->empty[(row - 3) % RING];
        }
        if (empty) return 0;
    }
    /* The ring rows of the taps: a row out of the picture adds no light. */
    for (j = 0; j < RING; ++j) {
        u32 row = t + j, inside = row >= 3 && row - 3 < f->h;
        for (k = 0; k < 3; ++k) light[j][k] = inside ? f->ring.rows[(row - 3) % RING][k] : zero_row;
    }
    /* Each channel along the row: the glow (GLOW_FULL = 1.0 linear) in
       f->light, and the change (0-255) in f->rgb. */
    for (k = 0; k < 3; ++k) {
        GlowLight rows;
        for (j = 0; j < RING; ++j) rows.ring[j] = light[j][k];
        rows.out = f->light[k];
        glow_light(&rows, f->part[k], f->w);
    }
    if (f->lift_ev) veil_guard(f, t);
    return 1;
}

/* Both streams read the same unmodified source rows before the composite
   writes them. Add their linear light, then encode once. Chroma gain and
   core protection are weighted by each stream's light: a boolean
   protection would cut Bloom's headroom in steps of fine pixels where
   Halation starts. An empty Halation leaves Bloom unchanged. */
static u32 glow_row(Fine *f, Fine *other, u32 t, GlowRow *out)
{
    u32 first = light_row(f, t), second = other ? light_row(other, t) : 0;
    u32 x, k, any = 0;
    if (!(first | second)) {
        /* A neighboring row can still glow, so interpolation needs zero
           values and protection even when this reused buffer was not empty. */
        for (x = 0; x < f->w; ++x) {
            for (k = 0; k < 3; ++k) out->v[k][x] = 0;
            out->protect[x] = 0;
        }
        out->zero = 1;
        return 0;
    }
    for (x = 0; x < f->w; ++x) {
        u32 energy = 0, extra = 0;
        for (k = 0; k < 3; ++k) {
            u32 a = first ? f->light[k][x] : 0, b = second ? other->light[k][x] : 0;
            u32 sum = a + b, e = encode_glow(sum > GLOW_FULL ? GLOW_FULL : sum);
            energy += a;
            extra += b;
            f->rgb[k][x] = (u8)(e < FAINT ? 0 : e);
            any |= f->rgb[k][x];
        }
        /* Halation's part of the light, at least 1 where it has any. */
        if (other) out->protect[x] = (u8)(extra ? 1 + divide(extra * 254, energy + extra) : 0);
        else out->protect[x] = (u8)(f->look->protect && energy ? 255 : 0);
        /* Save the local chroma gain until glow_ycc has converted the row. */
        f->light[0][x] = (u16)(extra ? divide(energy * f->look->vivid + extra * other->look->vivid, energy + extra) : f->look->vivid);
    }
    {
        GlowYcc rows = {{f->rgb[0], f->rgb[1], f->rgb[2]}, {out->v[0], out->v[1], out->v[2]}};
        glow_ycc(&rows, f->w);
    }
    for (k = 1; k < 3; ++k)
        for (x = 0; x < f->w; ++x) {
            u32 vivid = f->light[0][x];
            if (vivid) {
                int v = (out->v[k][x] * (int)vivid) >> 8;
                out->v[k][x] = (s16)(v > 16256 ? 16256 : v < -16256 ? -16256 : v);
            }
        }
    out->zero = !any;
    return any != 0;
}


/* Derive core protection before either output plane changes. With
   Look.protect the headroom uses the same film white as source detection:
   a print white below 255 must not leave room to tint an already
   saturated core. A pixel with a partial protection (Bloom and Halation)
   takes its brightest channel between the two whites by that part. */
INLINE u32 protect_of(const u8 *protect, u32 x)
{
    u32 a = protect[x >> 2], b = protect[(x >> 2) + 1];
    return a > b ? a : b;
}

INLINE u8 protected_top(u32 top, u32 white_top, u32 part)
{
    /* part 255 takes white_top exactly; white_top >= top. */
    return (u8)(top + (((white_top - top) * (part + (part >> 7)) + 128) >> 8));
}

static void composite_top(const u32 *luma, const u8 *chroma, u8 *top, u32 words, const Fine *f, const u8 *protect)
{
    const u8 *y = (const u8 *)(const void *)luma;
    u32 x, normalize = f->normalize;
    if (chroma) {
        u8 rgb[3][BLOCK_WORDS * 4];
        GlowRgb out = {rgb[0], rgb[1], rgb[2], top};
        glow_rgb(y, chroma, &out, words * 4);
        if (normalize)
            for (x = 0; x < words * 4; ++x) {
                u32 part = protect_of(protect, x);
                if (!part) continue;
                u32 r = f->norm[0][rgb[0][x]], g = f->norm[1][rgb[1][x]], b = f->norm[2][rgb[2][x]];
                u32 v = r > g ? r : g;
                top[x] = protected_top(top[x], b > v ? b : v, part);
            }
        return;
    }
    for (x = 0; x < words * 4; ++x) {
        u32 v = y[x], rgb[3] = {v, v, v}, part = normalize ? protect_of(protect, x) : 0;
        if (part) WHITE_RGB(f, rgb);
        u32 w = rgb[0] > rgb[1] ? rgb[0] : rgb[1];
        if (rgb[2] > w) w = rgb[2];
        top[x] = part ? protected_top(v, w, part) : (u8)w;
    }
}

/* The words of the runs of the last span back to 0, and no runs. Only the
   words in runs are written and stepped, so then all words are 0 (but the
   copy after the last word, which span_values sets each time). */
static void lane_clear(Lane *rw)
{
    u32 r, ch, k;
    for (r = 0; r < rw->runs; ++r) {
        u32 first = rw->run_first[r] * BLOCK_WORDS, end = first + rw->run_count[r] * BLOCK_WORDS;
        if (end > rw->words) end = rw->words;
        for (ch = 0; ch < 3; ++ch)
            for (k = first; k < end; ++k) rw->value[ch][k] = rw->step[ch][k] = 0;
        for (k = first; k < end; ++k) rw->protect[k] = 0;
    }
    rw->runs = 0;
}

/* The values and steps of each word between two glow rows: at row position
   g (of 4096) between them, and for each next row (`step_y`, or
   `chroma_step` for the chroma rows), stepped across the words of each span
   between two fine columns. `glow` has a bit for each fine column with glow
   in either row. Marks the runs of blocks that glow. A word without glow is
   0 (lane_clear), as a glowing word next to it reads its value. */
static void span_values(Lane *rw, const u32 *glow, u32 fw, int g, const GlowRow *up, const GlowRow *down)
{
    u32 k, ch, block, words = rw->words, step_y = rw->step_y, chroma_step = rw->p->half ? step_y << 1 : step_y;
    u32 blocks = (words + BLOCK_WORDS - 1) / BLOCK_WORDS;
    const u16 *column = rw->column;
    u8 active[MAX_BLOCKS];
#define GLOWS(i) ((glow[(i) >> 5] >> ((i) & 31)) & 1U)
    lane_clear(rw);
    for (block = 0; block < blocks; ++block) active[block] = 0;
    for (k = 0; k < words;) {
        u32 i = column[k], j = i + 1 < fw ? i + 1 : i, end = k, f0 = rw->fraction[k], df, n, c;
        while (end < words && column[end] == i) ++end;
        n = end - k;
        if (!(GLOWS(i) | GLOWS(j))) {
            k = end;
            continue;
        }
        /* The preceding word interpolates toward value[k], even when its
           own sample is zero. Keep its block to avoid a 32-pixel cutoff. */
        for (block = (k ? k - 1 : 0) / BLOCK_WORDS; block <= (end - 1) / BLOCK_WORDS; ++block) active[block] = 1;
        {
            u8 part = up->protect[i] > down->protect[i] ? up->protect[i] : down->protect[i];
            if (up->protect[j] > part) part = up->protect[j];
            if (down->protect[j] > part) part = down->protect[j];
            for (c = k; c < end; ++c) rw->protect[c] = part;
        }
        df = n > 1 ? (u32)(rw->fraction[k + 1] - f0) : 0;
        for (ch = 0; ch < 3; ++ch) {
            int rows = (int)(ch ? chroma_step : step_y);
            int ti = up->v[ch][i], tj = up->v[ch][j], bi = down->v[ch][i], bj = down->v[ch][j];
            int vi = ti + (((bi - ti) * g) >> 12), vj = tj + (((bj - tj) * g) >> 12);
            int si = ((bi - ti) * rows) >> 12, sj = ((bj - tj) * rows) >> 12;
            int v = vi * 256 + (vj - vi) * (int)f0, s = si * 256 + (sj - si) * (int)f0;
            int dv = (vj - vi) * (int)df, ds = (sj - si) * (int)df;
            s16 *value = rw->value[ch] + k, *step = rw->step[ch] + k;
            for (c = 0; c < n; ++c) {
                value[c] = (s16)(v >> 8);
                step[c] = (s16)(s >> 8);
                v += dv;
                s += ds;
            }
        }
        k = end;
    }
#undef GLOWS
    /* The word after the last: the same as the last, for the steps between
       words. */
    for (ch = 0; ch < 3; ++ch) {
        rw->value[ch][words] = rw->value[ch][words - 1];
        rw->step[ch][words] = rw->step[ch][words - 1];
    }
    rw->protect[words] = rw->protect[words - 1];
    for (block = 0; block < blocks; ++block) {
        if (!active[block]) continue;
        if (rw->runs && rw->run_first[rw->runs - 1] + rw->run_count[rw->runs - 1] == block) {
            ++rw->run_count[rw->runs - 1];
        } else {
            rw->run_first[rw->runs] = (u16)block;
            rw->run_count[rw->runs++] = 1;
        }
    }
}

/* Lane rows of one plane: its words and their fine columns, in the pools
   from word `first` and block `block`. Returns 0 when the plane does not
   fit. */
static u32 lane_init(Lane *lane, Rows *rw, const Plane *p, u32 first, u32 block)
{
    u32 x0 = p->left >> 2, words = ((p->right + 3) >> 2) - x0, tw = p->right - p->left, th = p->bottom - p->top, k, ch;
    u32 step_x;
    int base = (int)(x0 << 2) - (int)p->left;
    /* A plane wider than MAX_WORDS words does not fit the block flags of
       span_values: it gets no glow. */
    if (!tw || !th || words > MAX_WORDS || first + words + 1 > ALL_WORDS || block + words / BLOCK_WORDS + 1 > ALL_BLOCKS) return 0;
    lane->p = p;
    lane->x0 = x0;
    lane->words = words;
    lane->y = p->top;
    lane->runs = 0;
    /* The grid rows (of 4096) for each plane row, and (of 1 << 20) for the
       position of a row: Q12 would lose up to 0.6 of a grid row by the
       middle of a 4000-row plane. */
    lane->step_y = divide(rw->grid_h << 12, th);
    lane->scale_y = divide(rw->grid_h << 20, th);
    lane->offset = GRID_OFFSET;
    for (ch = 0; ch < 3; ++ch) {
        lane->value[ch] = rw->value[ch] + first;
        lane->step[ch] = rw->step[ch] + first;
    }
    for (ch = 0; ch < 3; ++ch)
        for (k = 0; k <= words; ++k) lane->value[ch][k] = lane->step[ch][k] = 0;
    lane->column = rw->column + first;
    lane->fraction = rw->fraction + first;
    lane->protect = rw->protect + first;
    for (k = 0; k <= words; ++k) lane->protect[k] = 0;
    lane->original_chroma = 0;
    if (p->half && rw->original_chroma) {
        if (rw->chroma_used + words * 4 > rw->chroma_bytes) return 0;
        lane->original_chroma = rw->original_chroma + rw->chroma_used;
        rw->chroma_used += words * 4;
    }
    lane->run_first = rw->run_first + block;
    lane->run_count = rw->run_count + block;
    /* The grid columns (of 1 << 20) for each plane column. A word takes the
       value at the center of its first pixel, as the pixel kernels place
       it there and step to the next word over its four pixels. */
    step_x = divide(rw->grid_w << 20, tw);
    for (k = 0; k < words; ++k) {
        int rel = base + (int)(k << 2);
        u32 pos = rel >= 0 ? ((u32)(2 * rel + 1) * step_x) >> 9 : 0, i;
        pos = pos > GRID_OFFSET ? pos - GRID_OFFSET : 0;
        i = pos >> 12;
        lane->column[k] = (u16)(i < rw->grid_w ? i : rw->grid_w - 1);
        lane->fraction[k] = (u8)(i < rw->grid_w ? (pos >> 4) & 255 : 0);
    }
    return 1;
}

/* The rows of a lane in span t (between fine rows t - 1 and t): their
   values at the first row, then the glow added to each row. */
static void lane_span(Lane *lane, u32 grid_w, u32 grid_h, const GlowRow *up, const GlowRow *down, const u32 *glow, u32 glows, u32 t, u32 mono, const Fine *f)
{
    const Plane *p = lane->p;
    u32 first_row = 1, r, o = lane->offset;
    for (; lane->y < p->bottom; ++lane->y) {
        u32 y = lane->y, pos = ((2 * (y - p->top) + 1) * lane->scale_y) >> 9, span = pos >= o ? ((pos - o) >> 12) + 1 : 0;
        u32 *luma, *crow = 0;
        const u8 *source_chroma = 0;
        if (span != t) break;
        if (first_row) {
            u32 a = t ? t - 1 : 0, b = t < grid_h ? t : grid_h - 1;
            first_row = 0;
            if (glows) span_values(lane, glow, grid_w, a == b ? 0 : (int)(pos - o - (a << 12)), up, down);
            else lane_clear(lane);
        }
        luma = WORDS(p->y + y * p->pitch + (lane->x0 << 2));
        if (!mono) {
            const u8 *original = BYTES(chroma_row(p, y) + (lane->x0 << 2));
            if (p->half) {
                /* Cache even inactive rows: an odd row can start a new
                   glow span, or be the first row of a cropped plane. */
                if (!(y & 1) || y == p->top)
                    for (u32 x = 0; x < lane->words * 4; ++x) lane->original_chroma[x] = original[x];
                source_chroma = lane->original_chroma;
            } else source_chroma = original;
            if (!p->half || !(y & 1)) crow = WORDS(chroma_row(p, y) + (lane->x0 << 2));
        }
        if (!lane->runs) continue;
        for (r = 0; r < lane->runs; ++r) {
            u32 first = lane->run_first[r] * BLOCK_WORDS, end = first + lane->run_count[r] * BLOCK_WORDS;
            s16 *const *value = lane->value, *const *step = lane->step;
            if (end > lane->words) end = lane->words;
            for (u32 k = first; k < end; k += BLOCK_WORDS) {
                u32 n = end - k < BLOCK_WORDS ? end - k : BLOCK_WORDS;
                u8 top[BLOCK_WORDS * 4], pair_top[BLOCK_WORDS * 4];
                const u8 *chroma_top = top;
                u32 protected = 0;
                for (u32 x = k; x <= k + n; ++x) protected |= lane->protect[x];
                composite_top(luma + k, source_chroma ? source_chroma + k * 4 : 0, top, n, f, lane->protect + k);
                if (crow && p->half && protected) {
                    /* Both original rows own this chroma. Where Halation
                       contributes, use the brighter core, or no tint at an
                       unpaired crop edge. Other pixels retain Bloom's rule. */
                    if (y + 1 < p->bottom) {
                        const u32 *next = WORDS(p->y + (y + 1) * p->pitch + ((lane->x0 + k) << 2));
                        composite_top(next, source_chroma + k * 4, pair_top, n, f, lane->protect + k);
                    } else {
                        for (u32 x = 0; x < n * 4; ++x) pair_top[x] = 255;
                    }
                    for (u32 x = 0; x < n * 4; ++x) {
                        u32 word = k + (x >> 2);
                        if (!(lane->protect[word] | lane->protect[word + 1]) || top[x] > pair_top[x]) pair_top[x] = top[x];
                    }
                    chroma_top = pair_top;
                }
                if (k == 0 || k + n == lane->words) {
                    u32 left = mono ? p->left : (p->left + 1) & ~1U;
                    u32 right = mono ? p->right : p->right & ~1U;
                    for (u32 x = 0; x < n * 4; ++x) {
                        u32 column = (lane->x0 + k) * 4 + x;
                        if (column < left || column >= right) {
                            top[x] = 255;
                            if (chroma_top == pair_top) pair_top[x] = 255;
                        }
                    }
                }
                if (crow) {
                    GlowChroma a = {value[1] + k, step[1] + k, value[2] + k, step[2] + k, chroma_top};
                    glow_chroma(crow + k, &a, n);
                }
                glow_luma(luma + k, value[0] + k, step[0] + k, n, top);
            }
        }
    }
}

/* The glow row of fine row t, made once (in order of t) into one of the
   two buffers, the one that does not hold fine row `keep`. */
static const GlowRow *fine_glow(Fine *f, Fine *other, Rows *rw, u32 t, u32 keep, u32 *glows)
{
    u32 k = rw->fine_row[0] == t ? 0 : rw->fine_row[1] == t ? 1 : rw->fine_row[0] == keep ? 1 : 0;
    if (rw->fine_row[k] != t) {
        rw->fine_glows[k] = glow_row(f, other, t, rw->fine[k]);
        rw->fine_row[k] = t;
    }
    *glows = rw->fine_glows[k];
    return rw->fine[k];
}

/* Row r of the composite grid. It lies between fine rows r / 2 and
   r / 2 + 1, at 3:1 (r even) or 1:3 (r odd), and each fine column gives
   two values, at 3:1 and 1:3 to the next column (a corner cut), so that
   the steps between fine pixels do not show as corners in the outline of a
   small glow. Returns whether the row glows. */
static u32 grid_row(Fine *f, Fine *other, Rows *rw, u32 r, GlowRow *out)
{
    u32 a = r >> 1, b = a + 1 < f->h ? a + 1 : a, ga, gb, k, x, n = f->w;
    int wa = r & 1 ? 1 : 3, wb = 4 - wa, any = 0;
    const GlowRow *fa = fine_glow(f, other, rw, a, ~0U, &ga), *fb = fine_glow(f, other, rw, b, a, &gb);
    if (!(ga | gb)) {
        if (!out->zero) {
            for (k = 0; k < 3; ++k)
                for (x = 0; x < rw->grid_w; ++x) { out->v[k][x] = 0; out->protect[x] = 0; }
            out->zero = 1;
        }
        return 0;
    }
    out->zero = 0;
    for (x = 0; x < n; ++x) {
        u32 j = x + 1 < n ? x + 1 : x;
        u8 protect = fa->protect[x] > fb->protect[x] ? fa->protect[x] : fb->protect[x];
        if (fa->protect[j] > protect) protect = fa->protect[j];
        if (fb->protect[j] > protect) protect = fb->protect[j];
        out->protect[2 * x] = out->protect[2 * x + 1] = protect;
    }
    for (k = 0; k < 3; ++k) {
        const s16 *pa = fa->v[k], *pb = fb->v[k];
        s16 *o = out->v[k];
        int c = (wa * pa[0] + wb * pb[0] + 2) >> 2;
        for (x = 0; x < n; ++x) {
            int next = x + 1 < n ? (wa * pa[x + 1] + wb * pb[x + 1] + 2) >> 2 : c;
            o[2 * x] = (s16)((3 * c + next + 2) >> 2);
            o[2 * x + 1] = (s16)((c + 3 * next + 2) >> 2);
            any |= o[2 * x] | o[2 * x + 1];
            c = next;
        }
    }
    return any != 0;
}

/* Pass 3: every fine glow row once, from the top, and the rows of all
   planes in each span between two fine rows. A screennail row gets its glow
   after the fine rows have read it. */
static void composite(PictureSet *set, Fine *f, Fine *other, Rows *rw)
{
    u32 i, t, lanes = 0, words = 0, blocks = 0, upper = 0, glows[2] = {0, 0};
    for (i = 0; i < set->count; ++i) {
        Lane *lane = &rw->lanes[lanes];
        if (!lane_init(lane, rw, &set->planes[i], words, blocks)) continue;
        words += lane->words + 1;
        blocks += lane->words / BLOCK_WORDS + 1;
        ++lanes;
    }
    f->next = 0;
    if (other) other->next = 0;
    rw->rows[0].zero = rw->rows[1].zero = 0;
    for (t = 0; t <= rw->grid_h; ++t) {
        /* The glow rows of fine rows a = t - 1 (up) and b = t (down). */
        const GlowRow *up, *down;
        if (t == 0) {
            glows[upper] = grid_row(f, other, rw, 0, &rw->rows[upper]);
            down = up = &rw->rows[upper];
        } else {
            if (t > 1) upper ^= 1; /* the last down row is the new up row */
            up = &rw->rows[upper];
            if (t < rw->grid_h) {
                glows[1 - upper] = grid_row(f, other, rw, t, &rw->rows[1 - upper]);
                down = &rw->rows[1 - upper];
            } else {
                down = up;
                glows[1 - upper] = 0;
            }
        }
        {
            /* The fine columns with glow in either row, as bits. */
            u32 glow[MAX_GRID / 32 + 1], x, any = glows[upper] || (down != up && glows[1 - upper]);
            if (any) {
                for (x = 0; x <= rw->grid_w >> 5; ++x) glow[x] = 0;
                for (x = 0; x < rw->grid_w; ++x)
                    if (up->v[0][x] | up->v[1][x] | up->v[2][x] | down->v[0][x] | down->v[1][x] | down->v[2][x]) glow[x >> 5] |= 1U << (x & 31);
            }
            for (i = 0; i < lanes; ++i) lane_span(&rw->lanes[i], rw->grid_w, rw->grid_h, up, down, glow, any, t, set->mono, f);
        }
    }
}

/* The plane that the light comes from: the 720 x 480 screennail, or else
   the smallest plane at least 240 pixels wide. */
static const Plane *source_plane(const PictureSet *set)
{
    const Plane *best = 0;
    u32 i;
    for (i = 0; i < set->count; ++i) {
        const Plane *p = &set->planes[i];
        if (p->width == 720 && p->height == 480) return p;
        if (p->right - p->left >= 240 && (!best || p->width * p->height < best->width * best->height)) best = p;
    }
    return best;
}

/* Look.gate: the part (of 256) of the glow that a scene of `ev` (1/16 EV)
   keeps. From DAY_FROM above the gate it fades out over DAY_RANGE, so that
   a daylight picture gets no glow. A picture whose coarse cells are mostly
   dark keeps it all: a lit street or shop window at night can meter like
   daylight. Reads the cell luma that coarse_light leaves, before
   coarse_surround blurs it. */
static u32 day_keep(const Fine *f, const Coarse *m, int ev, int gate)
{
    const u16 *luma64 = f->ring.cells.luma64;
    u32 i, n = m->w * m->h, dark = 0;
    int from = gate + DAY_FROM;
    if (ev <= from) return 256;
    for (i = 0; i < n; ++i) dark += luma64[i] < (DARK_LUMA << 6);
    if (dark << 8 >= n * DARK_SCENE) return 256;
    if (ev >= from + DAY_RANGE) return 0;
    return 256 - divide((u32)(ev - from) << 8, DAY_RANGE);
}

/* Both streams fit together with one clip map and one composite. Their
   coarse maps retain independent cell sizes and thresholds. */
_Static_assert(((sizeof(Rows) + 3) & ~3U) + 2 * ((sizeof(Fine) + 3) & ~3U) + 2 * sizeof(GlowRow) + ALL_WORDS * 4U +
               (720U >> CLIP_SHIFT) * ((480U >> CLIP_SHIFT) + 1) + 4U +
               (720U / 14) * (480U / 14) * 7U + (720U / 13) * (480U / 13) * 5U + 32U <=
               GRKIT_PLANE_SCRATCH, "both glow streams fit the screennail scratch");

/* Look.gate: the strength (Q8) of local background rejection from the
   scene EV of the shot, and that EV (1/16 EV) when the shot has it. */
static u32 shot_gate(const PictureSet *set, const Look *look, u32 *daylight, int *ev)
{
    GrkitShot shot;
    int from = (int)look->gate;
    *daylight = LOCAL_GATE_MIN;
    shot.size = sizeof shot;
    picture_shot(set, &shot);
    if (!(shot.known & GRKIT_SHOT_EXPOSURE)) return 0;
    *ev = shot.scene_ev >> 4;
    if (*ev >= from + (int)GATE_RANGE) *daylight = 256;
    else if (*ev > from) *daylight += divide((u32)(*ev - from) * (256 - LOCAL_GATE_MIN), GATE_RANGE);
    return 1;
}

/* Strong: the part (of 256) of the lift that the scene EV of the shot keeps
   (surround_weight), all of it up to LIFT_FROM, none from LIFT_FROM +
   LIFT_RANGE, and none when the shot has no EV. */
static u32 lift_keep(const PictureSet *set)
{
    GrkitShot shot;
    int ev;
    shot.size = sizeof shot;
    picture_shot(set, &shot);
    if (!(shot.known & GRKIT_SHOT_EXPOSURE)) return 0;
    ev = shot.scene_ev >> 4;
    if (ev <= LIFT_FROM) return 256;
    if (ev >= LIFT_FROM + LIFT_RANGE) return 0;
    return 256 - divide((u32)(ev - LIFT_FROM) << 8, LIFT_RANGE);
}

/* The light model of the look: the light of each level (light_scale), the
   lowest level with light, and the inverse of the linear light. */
static void light_setup(Fine *f, const Look *look)
{
    u32 k;
    light_scale(look, f->scale, f->gain);
    for (f->hot = 0; f->hot < 256 && !f->scale[f->hot]; ++f->hot) {}
    /* Look.chroma: a channel below f->hot can have light, up to the lift
       of a pixel whose darkest channel is 0. */
    if (look->chroma) {
        u32 need = f->hot;
        while (f->hot > CHROMA_FROM + 1 && f->hot - 1 + (((f->hot - 1 - CHROMA_FROM) * look->chroma) >> 7) >= need) --f->hot;
    }
    for (k = 0; k < 256; ++k) f->inverse[k] = (u16)(linear[k] ? divide(32768, linear[k]) : 0);
}

/* The white of the effect: levels scaled so that it is 255, and the clip
   level of the main picture at its luma. */
static void white_setup(Fine *f, const PictureSet *set)
{
    u8 white[3];
    u32 k, v;
    picture_white(set, white);
    f->normalize = 0;
    for (k = 0; k < 3; ++k) {
        u32 scale = divide(255U << 16, white[k] ? white[k] : 255);
        f->normalize |= white[k] < 255;
        for (v = 0; v < 256; ++v) {
            u32 level = (v * scale + 32768) >> 16;
            f->norm[k][v] = (u8)(level > 255 ? 255 : level);
        }
    }
    f->clip_level = divide(((77 * white[0] + 150 * white[1] + 29 * white[2]) >> 8) * CLIP_LEVEL + 127, 255);
}

/* The space for the copies of the chroma rows of half-height planes, in
   bytes: one row of each, at most ALL_WORDS words. */
static u32 chroma_bytes(const PictureSet *set)
{
    u32 k, bytes = 0;
    if (set->mono) return 0;
    for (k = 0; k < set->count; ++k) {
        const Plane *p = &set->planes[k];
        u32 words = ((p->right + 3) >> 2) - (p->left >> 2);
        if (p->half && words <= MAX_WORDS && p->bottom > p->top) bytes += words * 4;
    }
    return bytes > ALL_WORDS * 4 ? ALL_WORDS * 4 : bytes;
}

/* The clip map from the largest plane (the main picture), before the
   composite changes it, at `at`: the bytes that it takes, or 0 without a
   map (Look.point 0, no room, or no main picture). */
static u32 clip_setup(Fine *f, const PictureSet *set, const Look *look, u8 *at, u32 room)
{
    const Plane *src = f->p, *big = 0;
    u32 k;
    for (k = 0; k < set->count; ++k) {
        const Plane *p = &set->planes[k];
        if (!big || (p->right - p->left) * (p->bottom - p->top) > (big->right - big->left) * (big->bottom - big->top)) big = p;
    }
    f->clip = 0;
    f->clip_w = (src->right - src->left + (1U << CLIP_SHIFT) - 1) >> CLIP_SHIFT;
    f->clip_h = (src->bottom - src->top + (1U << CLIP_SHIFT) - 1) >> CLIP_SHIFT;
    if (!look->point || !big || big->right - big->left < 16 || f->clip_w > MAX_CLIP_COLUMNS ||
        f->clip_w * (f->clip_h + 1) + 4 > room) {
        f->clip_w = f->clip_h = 0;
        return 0;
    }
    f->clip = at;
    clip_scan(f, big, at);
    return (f->clip_w * (f->clip_h + 1) + 3) & ~3U;
}

/* The bytes of each coarse cell: 1 of surround, (Look.gate) 2 of the luma
   around, (Look.rim) 4 of color, and (the Strong lift) 1 of lift and 1 of
   large light share. */
static u32 cells_bytes(const Look *look, u32 lift) { return 1 + (look->gate ? 2 : 0) + (look->rim ? 4 : 0) + (lift ? 2 : 0); }

/* The coarse cells at `at`: Look.cell screennail pixels, or larger if they
   do not fit in `room`, cells_bytes for each cell, at most MAX_COARSE wide
   and MAX_CELLS. Returns whether they fit. */
static u32 cells_setup(Coarse *m, const Plane *src, const Look *look, u32 lift, u8 *at, u32 room)
{
    u32 n, bytes = cells_bytes(look, lift);
    for (m->step = look->cell;; ++m->step) {
        m->w = divide(src->right - src->left, m->step);
        m->h = divide(src->bottom - src->top, m->step);
        if (!m->w || !m->h) return 0;
        if (m->w <= MAX_COARSE && m->w * m->h * bytes + 16 <= room && m->w * m->h <= MAX_CELLS) break;
    }
    m->recip = divide(65536, ((m->step + 1) >> 1) * ((m->step + 1) >> 1));
    n = m->w * m->h;
    m->surround = at;
    m->around = m->surround + n;
    m->dim = m->around + n;
    m->rim = m->surround + n + (look->gate ? 2 * n : 0);
    m->lift = lift ? m->surround + n * (bytes - 2) : 0;
    m->large = lift ? m->lift + n : 0;
    return 1;
}

/* Initialize one stream without writing any picture pixels. */
static void stream_setup(Fine *f, const PictureSet *set, const Plane *src, const Look *look, u32 strength)
{
    f->p = src;
    f->mono = set->mono;
    f->look = look;
    f->shift = FINE_SHIFT;
    f->w = (src->right - src->left) >> FINE_SHIFT;
    f->h = (src->bottom - src->top) >> FINE_SHIFT;
    f->daylight = 0;
    f->lift_ev = strength == 3 ? lift_keep(set) : 0;
    if (f->lift_ev)
        for (u32 n = 0; n <= 64; ++n) f->recip[n] = reciprocal(n);
    for (u32 k = 0; k < 3; ++k) f->part[k] = (look->fine[k] * look->strength[strength - 1] + 128) >> 8;
    light_setup(f, look);
    white_setup(f, set);
}

/* Build this stream's surround and color maps. The byte count includes
   alignment for the next stream, or zero when no coarse cells fit. */
static u32 stream_cells(Fine *f, const PictureSet *set, u8 *at, u32 room)
{
    Coarse m;
    const Look *look = f->look;
    int ev = 0;
    u32 known = look->gate ? shot_gate(set, look, &f->daylight, &ev) : 0;
    if (!cells_setup(&m, f->p, look, f->lift_ev, at, room)) return 0;
    coarse_light(f, &m, look);
    if (look->rim) rim_fill(&m);
    if (known) {
        u32 keep = day_keep(f, &m, ev, (int)look->gate);
        for (u32 k = 0; k < 3; ++k) f->part[k] = (f->part[k] * keep + 128) >> 8;
    }
    coarse_surround(f, &m, look);
    for (u32 k = 0; k < RING; ++k) f->empty[k] = 0;
    for (u32 x = 0; x < f->w; ++x) {
        int pos = (int)divide((((x << 1) + 1) << f->shift) << 7, m.step) - 128;
        if (pos < 0) pos = 0;
        f->surround_column[x] = (u8)((u32)pos >> 8 < m.w ? (u32)pos >> 8 : m.w - 1);
        f->surround_fraction[x] = (u8)((u32)pos >> 8 < m.w ? (u32)pos & 255 : 0);
    }
    f->surround = m.surround;
    f->surround_w = m.w;
    f->surround_h = m.h;
    f->surround_step = m.step;
    f->rim = look->rim ? m.rim : 0;
    f->lift = m.lift;
    f->large = m.large;
    return (m.w * m.h * cells_bytes(look, f->lift_ev) + 3) & ~3U;
}

/* The strength of a Glow setting: 1 Weak, 2 Medium, 3 Strong, or 0 for Off
   and a value outside the setting. */
static u32 strength_of(u32 value) { return value <= 3 ? value : 0; }

/* The Halation look of the color setting. A monochrome picture, and a value
   outside the setting, take Orange. */
static const Look *halation_of(const PictureSet *set)
{
    u32 color = setting_get(SETTING_HALATION_COLOR);
    return &halation_looks[set->mono || color >= HALATION_COLORS ? HALATION_ORANGE : color];
}

void glow_apply(PictureSet *set, u32 value)
{
    if (!value) return;
    u32 b = strength_of(setting_get(SETTING_BLOOM)), h = strength_of(setting_get(SETTING_HALATION));
    if (!(b | h)) return;
    const Plane *src = source_plane(set);
    if (!src || src->right - src->left > MAX_SOURCE) return;
    u8 *at = set->scratch, *end = at + GRKIT_PLANE_SCRATCH;
    Rows *rw = (Rows *)(void *)at;
    at += (sizeof(Rows) + 3) & ~3U;
    Fine *f = (Fine *)(void *)at, *other = 0;
    at += (sizeof(Fine) + 3) & ~3U;
    const Look *halation = halation_of(set);
    stream_setup(f, set, src, b ? &bloom : halation, b ? b : h);
    if (b && h) {
        other = (Fine *)(void *)at;
        at += (sizeof(Fine) + 3) & ~3U;
        stream_setup(other, set, src, halation, h);
    }
    rw->chroma_bytes = chroma_bytes(set);
    rw->chroma_used = 0;
    rw->original_chroma = rw->chroma_bytes ? at : 0;
    at += rw->chroma_bytes;
    rw->fine[0] = (GlowRow *)(void *)at;
    rw->fine[1] = rw->fine[0] + 1;
    at += 2 * sizeof(GlowRow);
    rw->fine_row[0] = rw->fine_row[1] = ~0U;
    rw->fine[0]->zero = rw->fine[1]->zero = 0;
    rw->grid_w = f->w << 1;
    rw->grid_h = f->h << 1;
    at += clip_setup(f, set, f->look, at, (u32)(end - at));
    if (other) {
        other->clip = f->clip;
        other->clip_w = f->clip_w;
        other->clip_h = f->clip_h;
    }
    u32 used = stream_cells(f, set, at, (u32)(end - at) / (other ? 2 : 1));
    if (!used) return;
    at += used;
    if (other && !stream_cells(other, set, at, (u32)(end - at))) return;
    /* Daylight can remove Halation completely. Keep the shared maps and
       skip only streams whose three linear-light weights are all zero. */
    if (other && !(other->part[0] | other->part[1] | other->part[2])) other = 0;
    if (!other && !(f->part[0] | f->part[1] | f->part[2])) return;
    composite(set, f, other, rw);
}

/* Each setting gates a core filter, so both Off skips cache cleaning.
   Bloom's callback handles both streams together. The Halation callback only
   handles Halation alone, preserving the combined look and shared scratch. */
void glow_apply_halation(PictureSet *set, u32 value)
{
    if (!strength_of(setting_get(SETTING_BLOOM))) glow_apply(set, value);
}
