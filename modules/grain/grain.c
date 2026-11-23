/* Film-like grain: a plane filter that draws the grain of a color negative
   printed on paper into each recorded picture, after the grain model of
   spektrafilm by Andrea Volpato (github.com/andreavolpato/spektrafilm,
   src/spektrafilm/model/grain.py). This is a new implementation of the
   published model for the camera, in integers, not a port of that code.

   The System style leaves the grain to the native grain stage, which blends
   one noise plane into Y with the same strength at each tone. In the
   Film-like style the core runs that stage with no amplitude
   (grain_film_like in runtime/picture.c), so the rest of what it does stays
   the same, and this filter draws the grain:

   - Each of the three dye layers (here R, G, and B) has grain of its own,
     independent of the others, so the grain has color, as in a scan of a
     color negative. Mono pictures get the grain of one layer, in Y only.
   - Each layer has three sublayers whose particles are 2, 1, and 0.5 times
     the particle area (spektrafilm's particle_scale_layers), and the layer
     of B has particles twice the area of R and G (particle_scale 1.6, 1.6,
     3.2). In the model a pixel holds a Poisson number of particles, of
     which a binomial share develops: the noise of a sublayer has a variance
     that grows with its particle area, so its weight here is the square root
     of that area. Each sublayer has the blur of its dye clouds, the square
     root of the optical density of one particle in pixels, and then the
     whole grain the blur of spektrafilm's `blur`: 0.5, 0.65 (the default),
     and 1.0 pixels for Small, Medium, and Large. At this pixel size those
     blurs differ by less than a tenth of a pixel, so a layer is one noise
     with their blur, weighted by their variances (grain_texture).
   - The pixel is that of a 36 mm frame across the 6000 samples of a GR III
     picture, 6 um, and the particle area 0.2 um^2 times 0.5, 1, 2, or 4 for
     Weak, Medium, Strong, and Extra Strong: a faster film has larger
     particles (spektrafilm's particle_area_um2), so stronger and wider
     dye clouds.
   - The binomial noise is largest at the middle density of the negative,
     where half of the particles develop, and the print paper shows the
     density of the negative through its curve. grain_tone gives the
     visible grain of each output value for that chain, two halves of a
     parabola fitted to it (test.c checks the fit): a display gamma of
     2.2 over print density, a paper of density 2.3 whose curve is a
     logistic of gamma 2.6, and a negative of density 2.2 with half of its
     particles developed at the middle of the paper curve. The grain is
     strongest in the middle tones and none at paper white or in black.
   - The grain is density: it darkens, so a value moves by its grain times
     its tone, in each of R, G, and B. The filter writes the change of Y,
     Cb, and Cr, so a picture without grain stays exact.

   The model leaves out what this pixel size does not show: its lognormal
   micro-structure acts only below a pixel of about 0.6 um here. The camera
   picture was never a negative, so the tone is a model of a print,
   not of the film of the picture, and the strength is calibrated so that
   Medium is about 9 levels of G at the middle tone (STRENGTH).

   The grain of a layer reads a texture of 256 x 128 samples, a different
   place for each cell of 256 x 128 samples of the picture, and blends the
   cells at their edges so that they keep the statistics of the grain
   (grain_noise). So the grain never repeats. R and G read one texture,
   each at places of its own, and B its own texture of larger particles.
   A smaller plane, the screennail or the thumbnail, gets the grain that a
   reduction of the main picture would keep: the same texture, weaker by
   the ratio of the widths.

   On the camera the hot paths run in NEON in 16 bits (an assembly part, as
   the camera objects build soft-float): the grain of whole blocks of 4
   pairs of samples, in one pass with their chroma for 4:2:2
   (grain_film_blocks) or a pass for each luma row and one for the chroma
   for 4:2:0 (grain_luma_blocks, grain_chroma_blocks), the grain of mono
   pictures in blocks of 8 samples (grain_mono_blocks), and the blends at
   the edges of the cells (grain_hblend, grain_vblend). Inside a cell the
   grain is read from the texture itself, whose rows continue past their
   end. C does the edges of the visible area and the textures, with the
   same arithmetic (neon_parity.c checks it). The tone is two halves of a
   parabola so that NEON computes it for each sample. On the host, and
   with GRAIN_SCALAR, C does all.

   CORE.GRAIN gives the strength and calls the filter (module.toml), and
   CORE.GRAIN_SIZE the size; Auto follows the ISO as the native grain does:
   small to ISO 400, medium to ISO 800, large above. */
#include "grkit_module.h"

#define LAYERS 3U
#define SUBLAYERS 3U
/* The textures: one of the particles of R and G, and one of the twice
   larger particles of B, each a wrapped tile of TILE x TILE samples. */
#define TEXTURES 2U
/* A texture is TILE_W x TILE_H samples, each row followed by a copy of its
   first CELL samples (TILE_STRIDE), so that any CELL samples from any
   column of a row lie in a row. */
#define TILE 256U
#define TILE_W 256U
#define TILE_H 128U
#define TILE_STRIDE (TILE_W + CELL)
/* The standard deviation of the texture of R and G around 128; B gets the
   same scale times its weight, so it keeps its larger grain. */
#define TILE_DEVIATION 36U
/* The standard deviation of the grain that grain_noise gives. */
#define SUM_DEVIATION 36U
/* The cells of the grain (grain_noise): CELL x CELL_ROWS samples, each with its
   own place in the texture, blended with their neighbours over BLEND
   samples on each side of an edge. */
#define CELL 256U
#define CELL_ROWS 128U
#define BLEND 8U
/* The cell columns whose hashes cell_hashes keeps, with one more on each
   side: a picture up to 7680 samples wide. A wider one hashes each cell. */
#define CELL_COLUMNS 32U
/* A number as text, for the assembly. */
#define GRAIN_STR_(x) #x
#define GRAIN_STR(x) GRAIN_STR_(x)

/* The 16-bit arithmetic that the NEON code does, for C:
   qdmulh (VQDMULH.S16) is (2 a b) >> 16, qrdmulh (VQRDMULH.S16) rounds it,
   and both saturate only at -32768 x -32768, which never comes here. */
static int qdmulh(int a, int b) { return (int)(((s32)a * b) >> 15); }
static int qrdmulh(int a, int b) { return (int)(((s32)a * b + (1 << 14)) >> 15); }
static int clamp(int v) { return v < 0 ? 0 : v > 255 ? 255 : v; }

/* The visible grain of a print for each output value v, 0 to 255: two
   halves of a parabola, 255 at TONE_PEAK and 0 at 24 and at 251, that fit
   the model of the comment at the top within 11 of 255 (test.c checks it),
   so that NEON computes it for each sample in 16 bits:
   255 - qdmulh(d^2, K) for d = v - TONE_PEAK, with K = 255 x 32768 / 107^2
   below the peak and 255 x 32768 / 120^2 above, and 0 at least. */
#define TONE_PEAK 131
#define TONE_LEFT 730
#define TONE_RIGHT 580
static int grain_tone(int v)
{
    int d = v - TONE_PEAK, t = 255 - qdmulh(d * d, d < 0 ? TONE_LEFT : TONE_RIGHT);
    return t < 0 ? 0 : t;
}
/* For test.c. */
u32 grain_tone_of(u32 v) { return (u32)grain_tone((int)v); }

/* Gaussian kernels of radius 3 for sigma 0.40 to 1.30 pixels in steps of
   0.05, in 1/4096: the center, then each side. test.c checks them. */
#define KERNELS 19U
const u16 grain_kernel_table[KERNELS][4] = {
    {3766, 165, 0, 0},
    {3502, 297, 0, 0},
    {3222, 436, 1, 0},
    {2956, 566, 4, 0},
    {2718, 678, 11, 0},
    {2514, 769, 22, 0},
    {2336, 841, 39, 0},
    {2178, 896, 62, 1},
    {2042, 935, 90, 2},
    {1922, 962, 121, 4},
    {1816, 979, 154, 7},
    {1718, 989, 188, 12},
    {1636, 991, 221, 18},
    {1558, 989, 254, 26},
    {1486, 984, 285, 36},
    {1424, 975, 314, 47},
    {1366, 965, 340, 60},
    {1312, 953, 365, 74},
    {1266, 940, 387, 88},};

/* The particle area for each strength, in halves of 0.2 um^2: Weak 0.5,
   Medium 1, Strong 2, Extra Strong 4 times. */
static const u8 area_halves[4] = {1, 2, 4, 8};
/* The standard deviation of G where the grain is strongest, in 1/16 levels,
   for each strength: it grows with the square root of the particle area. */
static const u16 strength[4] = {102, 144, 204, 288};
/* spektrafilm's `blur` for Small, Medium, and Large, in 1/100 pixel. */
static const u8 blur_of_size[3] = {50, 65, 100};
/* The particle scale of R, G, and B, and of the sublayers, in 1/10. */
static const u8 layer_scale[LAYERS] = {16, 16, 32};
static const u8 sublayer_scale[SUBLAYERS] = {20, 10, 5};

typedef struct {
    u8 tiles[TEXTURES][TILE_H * TILE_STRIDE];
    u8 line[TILE];
    /* The grain of each layer for one cell of up to two luma rows, around
       128 (grain_noise), and the row below or above a row near the top or
       the bottom edge of its cell. */
    u8 noise[2][LAYERS][CELL];
    u8 other[CELL];
    /* The blended grain at the left and the right edge of a cell row (the
       first and the last BLEND samples), by luma row and layer. */
    u8 edges[2][LAYERS][2U * BLEND];
    /* The grain of R, G, and B of each pair of samples of a cell, in 1/64
       levels, summed over its luma samples, for the change of Cb and Cr. */
    short pairs[LAYERS][CELL / 2U];
    /* The hashes of the cells of the cell rows above, at, and below
       `cell_row` (cell_hashes), by layer: cell column cx at cx + 1. */
    u32 cells[LAYERS][3][CELL_COLUMNS];
    u32 cell_row;
    /* The scalars of the NEON code (grain_constants), and the weights of
       the left and the right edge of a cell (grain_hblend). */
    short constants[16];
    short edge_weights[48];
} Scratch;
_Static_assert(sizeof(Scratch) <= GRKIT_PLANE_SCRATCH, "The grain fits the scratch");

/* splitmix32. */
static u32 mix(u32 *state)
{
    u32 z = *state += 0x9e3779b9U;
    z = (z ^ (z >> 16)) * 0x85ebca6bU;
    z = (z ^ (z >> 13)) * 0xc2b2ae35U;
    return z ^ (z >> 16);
}

/* The integer square root. */
static u32 root(u32 v)
{
    u32 r = 0, b = 1U << 30;
    while (b > v) b >>= 2;
    while (b) {
        if (v >= r + b) {
            v -= r + b;
            r = (r >> 1) + b;
        } else {
            r >>= 1;
        }
        b >>= 2;
    }
    return r;
}


/* The Gaussian blur of `sigma` (1/100 pixel) of the n x n tile, across and
   down, with its edges wrapped. */
static void blur(u8 *tile, u32 w, u32 h, u8 *line, u32 sigma)
{
    u32 index = sigma <= 40U ? 0 : divide(sigma - 40U + 2U, 5U);
    const u16 *k = grain_kernel_table[index < KERNELS ? index : KERNELS - 1U];
    for (u32 pass = 0; pass < 2U; ++pass)
        for (u32 i = 0; i < (pass ? w : h); ++i) {
            u32 step = pass ? w : 1U, n = pass ? h : w;
            u8 *at = tile + (pass ? i : i * w);
            for (u32 j = 0; j < n; ++j) line[j] = at[j * step];
            for (u32 j = 0; j < n; ++j) {
                u32 sum = k[0] * line[j] + 2048U;
                for (u32 d = 1; d <= 3U; ++d) {
                    u32 before = j >= d ? j - d : j + n - d, after = j + d < n ? j + d : j + d - n;
                    sum += k[d] * ((u32)line[before] + line[after]);
                }
                at[j * step] = (u8)(sum >> 12);
            }
        }
}

/* The texture of layer `layer` in the w x h tile, around 128 before its
   scale. Returns the weight of the layer, the square root of the sum of the
   particle scales of its sublayers, x 256 / sqrt(1.6).

   Each sublayer is Gaussian noise (the binomial count of many particles)
   with the variance of its particle area, blurred by its dye clouds and by
   the grain blur. Their sum has the spectrum of the sum of their blurs,
   weighted by their variances; the blurs differ by less than a tenth of a
   pixel here, so the texture is one noise with the blur whose square is the
   weighted mean of their squares. */
static u32 grain_texture(Scratch *s, u8 *tile, u32 w, u32 h, u32 layer, u32 area, u32 size_blur, u32 *state)
{
    u32 weights = 0, blurs = 0;
    for (u32 l = 0; l < SUBLAYERS; ++l) {
        u32 scale = layer_scale[layer] * sublayer_scale[l];
        /* The optical density of one particle is the area of a particle
           times the particles over a sublayer of a third of the density
           2.2: 0.2 um^2 x scale / (36 um^2 / 3) x 2.2 / 3, so the square of
           the dye cloud blur is 0.01222 x scale x area, here in 1/10000
           pixel^2. */
        u32 dye = divide(1222U * scale * area, 2000U);
        weights += scale;
        blurs += scale * (size_blur * size_blur + dye);
    }
    for (u32 i = 0; i < w * h; ++i) {
        u32 r = mix(state);
        tile[i] = (u8)((((r & 255U) + (r >> 8 & 255U) + (r >> 16 & 255U) + (r >> 24)) + 2U) >> 2);
    }
    blur(tile, w, h, s->line, root(divide(blurs, weights)));
    return root(divide(weights * 65536U, 160U));
}

/* The mean and the standard deviation (x 16) of a w x h tile. */
static void texture_stats(const u8 *tile, u32 w, u32 h, u32 *mean, u32 *deviation)
{
    u32 all = 0, squares = 0;
    for (u32 y = 0; y < h; ++y) {
        u32 row = 0, row_squares = 0;
        for (u32 x = 0; x < w; ++x) {
            u32 v = tile[y * w + x];
            row += v;
            row_squares += v * v;
        }
        all += row;
        squares += divide(row_squares, w);
    }
    *mean = divide(all + w * h / 2U, w * h);
    /* The variance from the mean of the squares, x 256. */
    u32 mean_q4 = divide(all * 16U, w * h);
    u32 square_q8 = divide(squares, h) * 256U;
    *deviation = square_q8 > mean_q4 * mean_q4 ? root(square_q8 - mean_q4 * mean_q4) : 0;
}

/* Give each row and then each column of the w x h tile the mean 128,
   twice, so that no row or column of the texture shows as a line. */
static void center(u8 *tile, u32 w, u32 h)
{
    for (u32 round = 0; round < 4U; ++round)
        for (u32 i = 0; i < (round & 1U ? w : h); ++i) {
            u32 step = round & 1U ? w : 1U, n = round & 1U ? h : w, sum = 0;
            u8 *at = tile + (round & 1U ? i : i * w);
            for (u32 j = 0; j < n; ++j) sum += at[j * step];
            int shift = 128 - (int)divide(sum + n / 2U, n);
            if (shift)
                for (u32 j = 0; j < n; ++j) at[j * step] = (u8)clamp(at[j * step] + shift);
        }
}

/* Scale the deviations of a TILE_W x TILE_H tile from `mean` by `scale` /
   4096, around 128, center its rows and columns, and spread its rows to
   TILE_STRIDE, each followed by its first CELL samples. */
static void texture_store(u8 *tile, u32 mean, u32 scale)
{
    for (u32 i = 0; i < TILE_W * TILE_H; ++i) tile[i] = (u8)clamp(128 + ((((int)tile[i] - (int)mean) * (int)scale + 2048) >> 12));
    center(tile, TILE_W, TILE_H);
    for (u32 y = TILE_H; y-- > 0;) {
        u8 *row = tile + y * TILE_STRIDE;
        const u8 *from = tile + y * TILE_W;
        for (u32 x = TILE_W; x-- > 0;) row[x] = from[x];
        for (u32 x = 0; x < CELL; ++x) row[TILE_W + x] = row[x];
    }
}

/* The textures. The deviation of the texture of R and G sets the scale,
   and that of B gets it times the weight of B, so B keeps its larger grain.
   A mono picture needs only R and G. */
static void grain_tiles(Scratch *s, u32 textures, u32 area, u32 size_blur, u32 *state)
{
    u32 scale = 0, green = 1, mean, deviation;
    for (u32 x = 0; x < textures; ++x) {
        u8 *tile = s->tiles[x];
        u32 weight = grain_texture(s, tile, TILE_W, TILE_H, x ? 2U : 1U, area, size_blur, state);
        texture_stats(tile, TILE_W, TILE_H, &mean, &deviation);
        if (!x) {
            green = weight;
            scale = deviation ? divide(TILE_DEVIATION * 65536U + deviation / 2U, deviation) : 0;
        }
        texture_store(tile, mean, divide(scale * weight, green));
    }
}

/* The scale of the grain for a plane `width` samples wide, of a main
   picture `main` samples wide, at strength `level`: the grain of G at a
   value is STRENGTH / 16 levels times tone / 255, from noise of the
   deviation SUM_DEVIATION. In 1/64 levels the grain of a sample is
   qdmulh((z - 128) x tone, this scale), so the scale is
   STRENGTH x 64 x 32768 / (16 x SUM_DEVIATION x 255), at most 4112. */
static u32 grain_scale(u32 level, u32 width, u32 main)
{
    return divide(divide(strength[level - 1U] * 4096U * 32U, SUM_DEVIATION * 255U) * width, main);
}

/* The weight of its own cell for a sample d samples inside the edge of its
   cell, in 1/256 (the neighbour gets the rest), and the scale that keeps the
   deviation of the blend of two independent textures,
   4096 / sqrt(w^2 + (1 - w)^2). test.c checks them. */
const u8 grain_blend_weight[BLEND] = {128, 144, 160, 176, 192, 208, 224, 240};
const u16 grain_blend_norm[BLEND] = {5793, 5748, 5620, 5424, 5181, 4912, 4634, 4359};

/* The blend of two grains a and b around 128 with the weights wa and wb in
   1/256 and the scale norm in 1/4096 (grain_blend_norm). wa (a - 128) +
   wb (b - 128) fits 16 bits, as in the NEON code. */
static __attribute__((unused)) u8 blend(int a, int b, int wa, int wb, int norm)
{
    s32 sum = (s32)(short)(wa * (a - 128) + wb * (b - 128));
    return (u8)clamp(128 + ((sum * norm + (1 << 19)) >> 20));
}

/* The place of a cell of a layer in the texture: a hash of the cell, the
   layer, and the seed of the plane. */
static u32 cell_hash(u32 seed, u32 cx, u32 cy, u32 layer)
{
    u32 h = seed + cx * 0x9e3779b1U + cy * 0x85ebca77U + layer * 0xc2b2ae3dU;
    h ^= h >> 15;
    h *= 0x2c1b3c6dU;
    h ^= h >> 12;
    h *= 0x297a2d39U;
    h ^= h >> 15;
    return h;
}

/* Keep the hashes of the cell rows above, at, and below cell row cy, for
   the `first` layer and the `layers` after it. */
static void cell_hashes(Scratch *s, u32 seed, u32 cy, u32 first, u32 layers)
{
    if (s->cell_row == cy) return;
    s->cell_row = cy;
    for (u32 l = first; l < first + layers; ++l)
        for (u32 r = 0; r < 3U; ++r)
            for (u32 c = 0; c < CELL_COLUMNS; ++c) s->cells[l][r][c] = cell_hash(seed, c - 1U, cy + r - 1U, l);
}

/* The hash of a cell, from those that cell_hashes keeps where it can. */
static u32 cell_of(const Scratch *s, u32 seed, u32 cx, u32 cy, u32 layer)
{
    u32 row = cy - s->cell_row + 1U, column = cx + 1U;
    if (row < 3U && column < CELL_COLUMNS) return s->cells[layer][row][column];
    return cell_hash(seed, cx, cy, layer);
}

/* The NEON code (an assembly part, as the camera objects build
   soft-float) and its C, with the same arithmetic:
   - grain_film_blocks: the grain of blocks of 4 pairs of samples of a luma
     row whose pairs have no other row (4:2:2), with their change of Cb and
     Cr (film_pixel, film_chroma);
   - grain_hblend: the blends at the left and the right edge of a cell row;
   - grain_mono_blocks: the grain of blocks of 8 samples of a mono picture,
     in Y (mono_pixel);
   - grain_vblend: n bytes (a multiple of 8) of blend(a, b, wa, wb, norm);
   - grain_luma_blocks: the grain of blocks of 4 pairs of samples of a luma
     row (film_pixel);
   - grain_chroma_blocks: the change of Cb and Cr of blocks of 4 pairs
     (film_chroma).
   The structures give the arguments at the offsets of the assembly. */
typedef struct {
    u8 *y;
    const u8 *c;
    const u8 *z[LAYERS];
    short *sum[LAYERS];
    u32 groups;
    const short *constants;
    /* 0 to start the sums of the pairs (the first luma row of a pair),
       ~0 to add to them. */
    u32 keep;
} GrainBlocks;
typedef struct {
    u8 *c;
    const short *sum[LAYERS];
    u32 groups;
    s32 shift;
} GrainChroma;
_Static_assert(sizeof(void *) != 4U || (sizeof(GrainBlocks) == 44U && sizeof(GrainChroma) == 24U), "The layout of the NEON arguments");
void grain_hblend(u8 *out, const u8 *own_left, const u8 *left, const u8 *own_right, const u8 *right, const short *weights);
void grain_film_blocks(const GrainBlocks *b);
void grain_mono_blocks(const GrainBlocks *b);
void grain_vblend(u8 *out, const u8 *a, const u8 *b, u32 n, u32 weights);
void grain_luma_blocks(const GrainBlocks *b);
void grain_chroma_blocks(const GrainChroma *b);

#if defined(__arm__) && !defined(GRAIN_SCALAR)
#define GRAIN_NEON 1
#else
#define GRAIN_NEON 0
#endif

/* The offsets of R, G, and B from Y for the CbCr sample c: Cr x 1.402,
   -(Cb x 0.344 + Cr x 0.714), and Cb x 1.772 (JPEG, BT.601 full range). */
static void film_offsets(const u8 *c, int *offset)
{
    int cb = (int)c[0] - 128, cr = (int)c[1] - 128;
    offset[0] = cr + qrdmulh(cr, 13173);
    offset[1] = qrdmulh(cb, -11277) + qrdmulh(cr, -23401);
    offset[2] = cb + qrdmulh(cb, 25297);
}

/* The weights of R, G, and B in Y, in 1/32768. */
static const short film_luma[LAYERS] = {9798, 19235, 3736};

/* The grain of one luma sample *y with the offsets of its pair and the
   grain z of each layer around 128, at the scale of the plane: each layer's
   grain in density, in 1/64 levels, with the tone of its own value, added
   to the sums of the pair; Y loses their weighted sum. */
static void film_pixel(u8 *y, const int *offset, const u8 *z, short *sum, int scale)
{
    int v = *y, dy = 0;
    for (u32 l = 0; l < LAYERS; ++l) {
        int d = qdmulh(((int)z[l] - 128) * grain_tone(clamp(v + offset[l])), scale);
        dy = (short)(dy + qdmulh(d, film_luma[l]));
        sum[l] = (short)(sum[l] + d);
    }
    *y = (u8)clamp(v - ((dy + 32) >> 6));
}

/* The change of Cb and Cr of a pair from the sums of its n samples (1, 2,
   or 4): Cb -0.1687 R - 0.3313 G + 0.5 B, Cr 0.5 R - 0.4187 G - 0.0813 B,
   divided by n and 64 with rounding. */
static void film_chroma(u8 *c, const short *sum, u32 n)
{
    int shift = 6 + (n == 4U ? 2 : n == 2U ? 1 : 0);
    int cb = (short)(qdmulh(sum[0], -5529) + qdmulh(sum[1], -10855) + qdmulh(sum[2], 16384));
    int cr = (short)(qdmulh(sum[0], 16384) + qdmulh(sum[1], -13720) + qdmulh(sum[2], -2664));
    c[0] = (u8)clamp((int)c[0] - ((cb + (1 << (shift - 1))) >> shift));
    c[1] = (u8)clamp((int)c[1] - ((cr + (1 << (shift - 1))) >> shift));
}

/* The grain of one sample *y of a mono picture with grain z around 128: one
   layer, with the tone of Y. */
static void mono_pixel(u8 *y, int z, int scale)
{
    int v = *y, d = qdmulh((z - 128) * grain_tone(v), scale);
    *y = (u8)clamp(v - ((d + 32) >> 6));
}

#if GRAIN_NEON
/* Registers of grain_luma_blocks: d0 the offset weights, d1 the scale and
   the weights of R, G, and B in Y (s16 scalars), d5 `keep`, q4 TONE_LEFT, q5
   TONE_RIGHT, q6 255 (s16), d14 128 and d15 TONE_PEAK (u8), d16 Y, d17
   CbCr, d18 to d20 the grain, d2 to d4 R, G, B, q3 the change of Y.
   FILM_LAYER: one layer \v with grain \z, weight \k, and sums at \sum. */
__asm__(
    ".pushsection .text.grain_neon,\"ax\",%progbits\n"
    ".syntax unified\n"
    ".fpu neon\n"
    ".thumb\n"
    ".macro FILM_EXPAND src, out\n"
    "    vmov d22, \\src\n"
    "    vmov d23, \\src\n"
    "    vzip.16 d22, d23\n"
    "    vaddw.u8 q11, q11, d16\n"
    "    vqmovun.s16 \\out, q11\n"
    ".endm\n"
    ".macro FILM_LAYER_F v, z, k, out\n"
    "    vsubl.u8 q12, \\v, d15\n"
    "    vmul.i16 q13, q12, q12\n"
    "    vclt.s16 q14, q12, #0\n"
    "    vbsl q14, q4, q5\n"
    "    vqdmulh.s16 q13, q13, q14\n"
    "    vqsub.u16 q13, q6, q13\n"
    "    vsubl.u8 q12, \\z, d14\n"
    "    vmul.i16 q12, q12, q13\n"
    "    vqdmulh.s16 q12, q12, d1[0]\n"
    "    vqdmulh.s16 q13, q12, \\k\n"
    "    vadd.i16 q3, q3, q13\n"
    "    vpadd.i16 \\out, d24, d25\n"
    ".endm\n"
    ".macro FILM_LAYER v, z, k, sum\n"
    "    vsubl.u8 q12, \\v, d15\n"
    "    vmul.i16 q13, q12, q12\n"
    "    vclt.s16 q14, q12, #0\n"
    "    vbsl q14, q4, q5\n"
    "    vqdmulh.s16 q13, q13, q14\n"
    "    vqsub.u16 q13, q6, q13\n"
    "    vsubl.u8 q12, \\z, d14\n"
    "    vmul.i16 q12, q12, q13\n"
    "    vqdmulh.s16 q12, q12, d1[0]\n"
    "    vqdmulh.s16 q13, q12, \\k\n"
    "    vadd.i16 q3, q3, q13\n"
    "    vpadd.i16 d26, d24, d25\n"
    "    vld1.16 {d27}, [\\sum]\n"
    "    vand d27, d27, d5\n"
    "    vadd.i16 d27, d27, d26\n"
    "    vst1.16 {d27}, [\\sum]!\n"
    ".endm\n"
    /* FILM_HBLEND: d22 own, d23 other, q8 own weights, q9 other weights,
       q10 norms, d6 128 (u8), q15 128 (s16): the blend into [\\at]. */
    ".macro FILM_HBLEND at\n"
    "    vsubl.u8 q12, d22, d6\n"
    "    vsubl.u8 q13, d23, d6\n"
    "    vmul.i16 q12, q12, q8\n"
    "    vmla.i16 q12, q13, q9\n"
    "    vmull.s16 q13, d24, d20\n"
    "    vmull.s16 q14, d25, d21\n"
    "    vrshr.s32 q13, q13, #20\n"
    "    vrshr.s32 q14, q14, #20\n"
    "    vqmovn.s32 d24, q13\n"
    "    vqmovn.s32 d25, q14\n"
    "    vadd.i16 q12, q12, q15\n"
    "    vqmovun.s16 d24, q12\n"
    "    vst1.8 {d24}, [\\at]\n"
    ".endm\n"
    ".p2align 2\n"
    ".global grain_luma_blocks\n"
    ".type grain_luma_blocks, %function\n"
    ".thumb_func\n"
    "grain_luma_blocks:\n"
    "    push {r4, r5, r6, r7, r8, r9, r10, r11, lr}\n"
    "    vpush {d8-d15}\n"
    "    ldr r9, [r0, #32]\n"
    "    cmp r9, #0\n"
    "    beq 9f\n"
    "    ldr r1, [r0, #0]\n"
    "    ldr r2, [r0, #4]\n"
    "    ldr r3, [r0, #8]\n"
    "    ldr r4, [r0, #12]\n"
    "    ldr r5, [r0, #16]\n"
    "    ldr r6, [r0, #20]\n"
    "    ldr r7, [r0, #24]\n"
    "    ldr r8, [r0, #28]\n"
    "    ldr r11, [r0, #36]\n"
    "    vld1.16 {d0-d1}, [r11]\n"
    "    ldr r10, [r0, #40]\n"
    "    vdup.32 d5, r10\n"
    "    movw r10, #" GRAIN_STR(TONE_LEFT) "\n"
    "    vdup.16 q4, r10\n"
    "    movw r10, #" GRAIN_STR(TONE_RIGHT) "\n"
    "    vdup.16 q5, r10\n"
    "    vmov.i16 q6, #255\n"
    "    vmov.i8 d14, #128\n"
    "    vmov.i8 d15, #" GRAIN_STR(TONE_PEAK) "\n"
    "1:  vld1.8 {d16}, [r1]\n"
    "    vld1.8 {d17}, [r2]!\n"
    "    vld1.8 {d18}, [r3]!\n"
    "    vld1.8 {d19}, [r4]!\n"
    "    vld1.8 {d20}, [r5]!\n"
    "    vsubl.u8 q12, d17, d14\n"
    "    vuzp.16 d24, d25\n"
    "    vqrdmulh.s16 d26, d25, d0[0]\n"
    "    vadd.i16 d26, d26, d25\n"
    "    vqrdmulh.s16 d27, d24, d0[1]\n"
    "    vqrdmulh.s16 d28, d25, d0[2]\n"
    "    vadd.i16 d27, d27, d28\n"
    "    vqrdmulh.s16 d28, d24, d0[3]\n"
    "    vadd.i16 d28, d28, d24\n"
    "    FILM_EXPAND d26, d2\n"
    "    FILM_EXPAND d27, d3\n"
    "    FILM_EXPAND d28, d4\n"
    "    vmov.i16 q3, #0\n"
    "    FILM_LAYER d2, d18, d1[1], r6\n"
    "    FILM_LAYER d3, d19, d1[2], r7\n"
    "    FILM_LAYER d4, d20, d1[3], r8\n"
    "    vrshr.s16 q3, q3, #6\n"
    "    vmovl.u8 q12, d16\n"
    "    vsub.i16 q12, q12, q3\n"
    "    vqmovun.s16 d24, q12\n"
    "    vst1.8 {d24}, [r1]!\n"
    "    subs r9, r9, #1\n"
    "    bne 1b\n"
    "9:  vpop {d8-d15}\n"
    "    pop {r4, r5, r6, r7, r8, r9, r10, r11, pc}\n"
    ".size grain_luma_blocks, .-grain_luma_blocks\n"
    /* grain_film_blocks: as grain_luma_blocks, with the sums of each pair
       in d30, d31, d21 for R, G, B, and then its change of Cb and Cr: d2,
       d3[0] their weights (reloaded from r10 after R, G, B are done), d5
       the rounding shift of a pair of 2 samples (-7). */
    ".p2align 2\n"
    ".global grain_film_blocks\n"
    ".type grain_film_blocks, %function\n"
    ".thumb_func\n"
    "grain_film_blocks:\n"
    "    push {r4, r5, r6, r7, r8, r9, r10, r11, lr}\n"
    "    vpush {d8-d15}\n"
    "    ldr r9, [r0, #32]\n"
    "    cmp r9, #0\n"
    "    beq 9f\n"
    "    ldr r1, [r0, #0]\n"
    "    ldr r2, [r0, #4]\n"
    "    ldr r3, [r0, #8]\n"
    "    ldr r4, [r0, #12]\n"
    "    ldr r5, [r0, #16]\n"
    "    ldr r11, [r0, #36]\n"
    "    vld1.16 {d0-d1}, [r11]!\n"
    "    mov r10, r11\n"
    "    vld1.16 {d2-d3}, [r10]\n"
    "    vdup.16 d5, d3[1]\n"
    "    mov r6, r2\n"
    "    movw r7, #" GRAIN_STR(TONE_LEFT) "\n"
    "    vdup.16 q4, r7\n"
    "    movw r7, #" GRAIN_STR(TONE_RIGHT) "\n"
    "    vdup.16 q5, r7\n"
    "    vmov.i16 q6, #255\n"
    "    vmov.i8 d14, #128\n"
    "    vmov.i8 d15, #" GRAIN_STR(TONE_PEAK) "\n"
    "1:  vld1.8 {d16}, [r1]\n"
    "    vld1.8 {d17}, [r2]!\n"
    "    vld1.8 {d18}, [r3]!\n"
    "    vld1.8 {d19}, [r4]!\n"
    "    vld1.8 {d20}, [r5]!\n"
    "    vsubl.u8 q12, d17, d14\n"
    "    vuzp.16 d24, d25\n"
    "    vqrdmulh.s16 d26, d25, d0[0]\n"
    "    vadd.i16 d26, d26, d25\n"
    "    vqrdmulh.s16 d27, d24, d0[1]\n"
    "    vqrdmulh.s16 d28, d25, d0[2]\n"
    "    vadd.i16 d27, d27, d28\n"
    "    vqrdmulh.s16 d28, d24, d0[3]\n"
    "    vadd.i16 d28, d28, d24\n"
    "    FILM_EXPAND d26, d2\n"
    "    FILM_EXPAND d27, d3\n"
    "    FILM_EXPAND d28, d4\n"
    "    vmov.i16 q3, #0\n"
    "    FILM_LAYER_F d2, d18, d1[1], d30\n"
    "    FILM_LAYER_F d3, d19, d1[2], d31\n"
    "    FILM_LAYER_F d4, d20, d1[3], d21\n"
    "    vrshr.s16 q3, q3, #6\n"
    "    vmovl.u8 q12, d16\n"
    "    vsub.i16 q12, q12, q3\n"
    "    vqmovun.s16 d24, q12\n"
    "    vst1.8 {d24}, [r1]!\n"
    "    vld1.16 {d2-d3}, [r10]\n"
    "    vqdmulh.s16 d22, d30, d2[0]\n"
    "    vqdmulh.s16 d23, d31, d2[1]\n"
    "    vadd.i16 d22, d22, d23\n"
    "    vqdmulh.s16 d23, d21, d2[2]\n"
    "    vadd.i16 d22, d22, d23\n"
    "    vqdmulh.s16 d23, d30, d2[2]\n"
    "    vqdmulh.s16 d24, d31, d2[3]\n"
    "    vadd.i16 d23, d23, d24\n"
    "    vqdmulh.s16 d24, d21, d3[0]\n"
    "    vadd.i16 d23, d23, d24\n"
    "    vrshl.s16 d22, d22, d5\n"
    "    vrshl.s16 d23, d23, d5\n"
    "    vzip.16 d22, d23\n"
    "    vmovl.u8 q12, d17\n"
    "    vsub.i16 q12, q12, q11\n"
    "    vqmovun.s16 d24, q12\n"
    "    vst1.8 {d24}, [r6]!\n"
    "    subs r9, r9, #1\n"
    "    bne 1b\n"
    "9:  vpop {d8-d15}\n"
    "    pop {r4, r5, r6, r7, r8, r9, r10, r11, pc}\n"
    ".size grain_film_blocks, .-grain_film_blocks\n"
    /* grain_mono_blocks: d1[0] the scale, q1 255, q2 TONE_LEFT, q3
       TONE_RIGHT (s16), d16 128 and d17 TONE_PEAK (u8); the grain at z[0].
       It keeps d8 to d15 and r4 to r11. */
    ".p2align 2\n"
    ".global grain_mono_blocks\n"
    ".type grain_mono_blocks, %function\n"
    ".thumb_func\n"
    "grain_mono_blocks:\n"
    "    ldr r3, [r0, #32]\n"
    "    cbz r3, 9f\n"
    "    ldr r1, [r0, #0]\n"
    "    ldr r2, [r0, #8]\n"
    "    ldr r12, [r0, #36]\n"
    "    vld1.16 {d0-d1}, [r12]\n"
    "    movw r12, #" GRAIN_STR(TONE_LEFT) "\n"
    "    vdup.16 q2, r12\n"
    "    movw r12, #" GRAIN_STR(TONE_RIGHT) "\n"
    "    vdup.16 q3, r12\n"
    "    vmov.i16 q1, #255\n"
    "    vmov.i8 d16, #128\n"
    "    vmov.i8 d17, #" GRAIN_STR(TONE_PEAK) "\n"
    "1:  vld1.8 {d18}, [r1]\n"
    "    vld1.8 {d19}, [r2]!\n"
    "    vsubl.u8 q10, d18, d17\n"
    "    vmul.i16 q11, q10, q10\n"
    "    vclt.s16 q12, q10, #0\n"
    "    vbsl q12, q2, q3\n"
    "    vqdmulh.s16 q11, q11, q12\n"
    "    vqsub.u16 q11, q1, q11\n"
    "    vsubl.u8 q10, d19, d16\n"
    "    vmul.i16 q10, q10, q11\n"
    "    vqdmulh.s16 q10, q10, d1[0]\n"
    "    vrshr.s16 q10, q10, #6\n"
    "    vmovl.u8 q11, d18\n"
    "    vsub.i16 q11, q11, q10\n"
    "    vqmovun.s16 d18, q11\n"
    "    vst1.8 {d18}, [r1]!\n"
    "    subs r3, r3, #1\n"
    "    bne 1b\n"
    "9:  bx lr\n"
    ".size grain_mono_blocks, .-grain_mono_blocks\n"
    /* grain_chroma_blocks: d0 and d1[0] the weights of R, G, and B in Cb and
       Cr, d2 the shift (negative, rounding). */
    ".p2align 2\n"
    ".global grain_chroma_blocks\n"
    ".type grain_chroma_blocks, %function\n"
    ".thumb_func\n"
    "grain_chroma_blocks:\n"
    "    push {r4, r5, r6, lr}\n"
    "    ldr r5, [r0, #16]\n"
    "    cmp r5, #0\n"
    "    beq 9f\n"
    "    ldr r1, [r0, #0]\n"
    "    ldr r2, [r0, #4]\n"
    "    ldr r3, [r0, #8]\n"
    "    ldr r4, [r0, #12]\n"
    "    ldr r6, [r0, #20]\n"
    "    rsb r6, r6, #0\n"
    "    vdup.16 d2, r6\n"
    "    movw r6, #0xea67\n"
    "    vmov.16 d0[0], r6\n"
    "    movw r6, #0xd599\n"
    "    vmov.16 d0[1], r6\n"
    "    movw r6, #16384\n"
    "    vmov.16 d0[2], r6\n"
    "    movw r6, #0xca68\n"
    "    vmov.16 d0[3], r6\n"
    "    movw r6, #0xf598\n"
    "    vmov.16 d1[0], r6\n"
    "1:  vld1.16 {d16}, [r2]!\n"
    "    vld1.16 {d17}, [r3]!\n"
    "    vld1.16 {d18}, [r4]!\n"
    "    vqdmulh.s16 d20, d16, d0[0]\n"
    "    vqdmulh.s16 d22, d17, d0[1]\n"
    "    vadd.i16 d20, d20, d22\n"
    "    vqdmulh.s16 d22, d18, d0[2]\n"
    "    vadd.i16 d20, d20, d22\n"
    "    vqdmulh.s16 d21, d16, d0[2]\n"
    "    vqdmulh.s16 d22, d17, d0[3]\n"
    "    vadd.i16 d21, d21, d22\n"
    "    vqdmulh.s16 d22, d18, d1[0]\n"
    "    vadd.i16 d21, d21, d22\n"
    "    vrshl.s16 d20, d20, d2\n"
    "    vrshl.s16 d21, d21, d2\n"
    "    vzip.16 d20, d21\n"
    "    vld1.8 {d24}, [r1]\n"
    "    vmovl.u8 q13, d24\n"
    "    vsub.i16 q13, q13, q10\n"
    "    vqmovun.s16 d24, q13\n"
    "    vst1.8 {d24}, [r1]!\n"
    "    subs r5, r5, #1\n"
    "    bne 1b\n"
    "9:  pop {r4, r5, r6, pc}\n"
    ".size grain_chroma_blocks, .-grain_chroma_blocks\n"
    /* grain_vblend(out, a, b, n, weights): weights holds wa, wb, and norm
       in its bytes 0, 1, and 2 and 3. */
    ".p2align 2\n"
    ".global grain_vblend\n"
    ".type grain_vblend, %function\n"
    ".thumb_func\n"
    "grain_vblend:\n"
    "    push {r4, r5, lr}\n"
    "    ldr r4, [sp, #12]\n"
    "    and r5, r4, #255\n"
    "    vmov.16 d0[0], r5\n"
    "    ubfx r5, r4, #8, #8\n"
    "    vmov.16 d0[1], r5\n"
    "    lsr r5, r4, #16\n"
    "    vmov.16 d0[2], r5\n"
    "    vmov.i8 d6, #128\n"
    "    vmov.i16 q15, #128\n"
    "    lsrs r3, r3, #3\n"
    "    beq 9f\n"
    "1:  vld1.8 {d16}, [r1]!\n"
    "    vld1.8 {d17}, [r2]!\n"
    "    vsubl.u8 q9, d16, d6\n"
    "    vsubl.u8 q10, d17, d6\n"
    "    vmul.i16 q11, q9, d0[0]\n"
    "    vmla.i16 q11, q10, d0[1]\n"
    "    vmull.s16 q12, d22, d0[2]\n"
    "    vmull.s16 q13, d23, d0[2]\n"
    "    vrshr.s32 q12, q12, #20\n"
    "    vrshr.s32 q13, q13, #20\n"
    "    vqmovn.s32 d28, q12\n"
    "    vqmovn.s32 d29, q13\n"
    "    vadd.i16 q14, q14, q15\n"
    "    vqmovun.s16 d16, q14\n"
    "    vst1.8 {d16}, [r0]!\n"
    "    subs r3, r3, #1\n"
    "    bne 1b\n"
    "9:  pop {r4, r5, pc}\n"
    ".size grain_vblend, .-grain_vblend\n"
    /* grain_hblend(out, own_left, left, own_right, right, weights): out[0]
       to out[7] the blend of own_left with left, out[8] to out[15] that of
       own_right with right; weights holds the own and the other weights and
       the norms of the left edge, then of the right edge, 8 s16 each. */
    ".p2align 2\n"
    ".global grain_hblend\n"
    ".type grain_hblend, %function\n"
    ".thumb_func\n"
    "grain_hblend:\n"
    "    push {r4, lr}\n"
    "    ldr r4, [sp, #8]\n"
    "    ldr r12, [sp, #12]\n"
    "    vmov.i8 d6, #128\n"
    "    vmov.i16 q15, #128\n"
    "    vld1.16 {q8-q9}, [r12]!\n"
    "    vld1.16 {q10}, [r12]!\n"
    "    vld1.8 {d22}, [r1]\n"
    "    vld1.8 {d23}, [r2]\n"
    "    FILM_HBLEND r0\n"
    "    add r0, r0, #8\n"
    "    vld1.16 {q8-q9}, [r12]!\n"
    "    vld1.16 {q10}, [r12]\n"
    "    vld1.8 {d22}, [r3]\n"
    "    vld1.8 {d23}, [r4]\n"
    "    FILM_HBLEND r0\n"
    "    pop {r4, pc}\n"
    ".size grain_hblend, .-grain_hblend\n"
    ".purgem FILM_EXPAND\n"
    ".purgem FILM_HBLEND\n"
    ".purgem FILM_LAYER_F\n"
    ".purgem FILM_LAYER\n"
    ".popsection\n");
#else
void grain_hblend(u8 *out, const u8 *own_left, const u8 *left, const u8 *own_right, const u8 *right, const short *weights)
{
    for (u32 i = 0; i < 8U; ++i) {
        out[i] = blend(own_left[i], left[i], weights[i], weights[8 + i], weights[16 + i]);
        out[8 + i] = blend(own_right[i], right[i], weights[24 + i], weights[32 + i], weights[40 + i]);
    }
}
void grain_mono_blocks(const GrainBlocks *b)
{
    for (u32 x = 0; x < b->groups * 8U; ++x) mono_pixel(b->y + x, b->z[0][x], b->constants[4]);
}
void grain_film_blocks(const GrainBlocks *b)
{
    for (u32 j = 0; j < b->groups * 4U; ++j) {
        int offset[LAYERS];
        short sum[LAYERS] = {0, 0, 0};
        film_offsets(b->c + 2U * j, offset);
        for (u32 i = 0; i < 2U; ++i) {
            u32 x = 2U * j + i;
            u8 z[LAYERS] = {b->z[0][x], b->z[1][x], b->z[2][x]};
            film_pixel(b->y + x, offset, z, sum, b->constants[4]);
        }
        film_chroma((u8 *)b->c + 2U * j, sum, 2);
    }
}
void grain_vblend(u8 *out, const u8 *a, const u8 *b, u32 n, u32 weights)
{
    int wa = (int)(weights & 255U), wb = (int)(weights >> 8 & 255U), norm = (int)(weights >> 16);
    for (u32 i = 0; i < n; ++i) out[i] = blend(a[i], b[i], wa, wb, norm);
}
void grain_luma_blocks(const GrainBlocks *b)
{
    for (u32 j = 0; j < b->groups * 4U; ++j) {
        int offset[LAYERS];
        short sum[LAYERS];
        film_offsets(b->c + 2U * j, offset);
        for (u32 l = 0; l < LAYERS; ++l) sum[l] = b->keep ? b->sum[l][j] : 0;
        for (u32 i = 0; i < 2U; ++i) {
            u32 x = 2U * j + i;
            u8 z[LAYERS] = {b->z[0][x], b->z[1][x], b->z[2][x]};
            film_pixel(b->y + x, offset, z, sum, b->constants[4]);
        }
        for (u32 l = 0; l < LAYERS; ++l) b->sum[l][j] = sum[l];
    }
}
void grain_chroma_blocks(const GrainChroma *b)
{
    u32 n = b->shift == 8 ? 4U : 2U;
    for (u32 j = 0; j < b->groups * 4U; ++j) {
        short sum[LAYERS] = {b->sum[0][j], b->sum[1][j], b->sum[2][j]};
        film_chroma(b->c + 2U * j, sum, n);
    }
}
#endif

/* The scalars of grain_luma_blocks for a plane of `scale` (grain_scale),
   d0 and d1. */
static void grain_constants(Scratch *s, u32 scale)
{
    short *k = s->constants;
    k[0] = 13173;
    k[1] = -11277;
    k[2] = -23401;
    k[3] = 25297;
    k[4] = (short)scale;
    k[5] = film_luma[0];
    k[6] = film_luma[1];
    k[7] = film_luma[2];
    k[8] = -5529;
    k[9] = -10855;
    k[10] = 16384;
    k[11] = -13720;
    k[12] = -2664;
    k[13] = -7;
    k[14] = k[15] = 0;
    for (u32 i = 0; i < 8U; ++i) {
        s->edge_weights[i] = grain_blend_weight[i];
        s->edge_weights[8 + i] = (short)(256 - grain_blend_weight[i]);
        s->edge_weights[16 + i] = (short)grain_blend_norm[i];
        s->edge_weights[24 + i] = grain_blend_weight[7 - i];
        s->edge_weights[32 + i] = (short)(256 - grain_blend_weight[7 - i]);
        s->edge_weights[40 + i] = (short)grain_blend_norm[7 - i];
    }
}

/* Where the grain of a layer comes from along one row of a cell: z(x) =
   part[0][t] for t = x - cell below BLEND (the left edge), part[1][t]
   inside, and part[2][t] from CELL - BLEND on (the right edge). */
typedef struct {
    const u8 *part[3];
} NoiseRow;
static u32 part_of(u32 t) { return t < BLEND ? 0 : t < CELL - BLEND ? 1U : 2U; }

/* The texture row of a cell with hash h at the row ty of the cell (ty may
   be up to BLEND outside the cell, for its neighbours), and its
   column at the start of the cell: TILE_STRIDE keeps CELL + BLEND samples
   from there in the row. */
static const u8 *texture_at(const u8 *texture, u32 h, s32 ty, u32 column)
{
    return texture + (((u32)ty + (h >> 8)) & (TILE_H - 1U)) * TILE_STRIDE + (column & (TILE_W - 1U));
}

/* The grain of `layer` for cell cx of the picture row y, around 128.

   Each cell of CELL x CELL_ROWS samples reads the texture at a place of its own,
   so the grain never repeats. Near the left or the right edge of its cell a
   sample blends the texture of the cell with that of its neighbour, and a
   row near the top or the bottom edge blends that row with the row of the
   cell above or below, with the weights of grain_blend_weight and the scale
   of grain_blend_norm, which keeps the deviation of the sum of two
   independent textures; a corner blends four. This is the tiling and
   blending of Heitz and Neyret (2018), which keeps the statistics of a
   Gaussian texture such as the grain. A neighbour reads its texture where
   its own samples would continue, so a blend joins the two textures.

   Inside the cell of a row away from its top and bottom the grain is the
   texture itself: out->part[1] points into it. The edges go to `edge`
   (2 BLEND bytes), and the whole row of a row near the top or the bottom to
   `full` (CELL bytes). */
static void grain_noise(Scratch *s, NoiseRow *out, u8 *edge, u8 *full, u32 layer, u32 seed, u32 y, u32 cx)
{
    const u8 *texture = s->tiles[layer == 2U ? 1U : 0];
    u32 cy = y / CELL_ROWS, ty = y % CELL_ROWS, rows = 1, cell_row[2] = {cy, cy}, d = 0;
    s32 row_y[2] = {(s32)ty, (s32)ty};
    const u8 *own[2];
    if (ty < BLEND || ty >= CELL_ROWS - BLEND) {
        d = ty < BLEND ? ty : CELL_ROWS - 1U - ty;
        rows = 2;
        cell_row[1] = ty < BLEND ? cy - 1U : cy + 1U;
        row_y[1] = ty < BLEND ? (s32)(ty + CELL_ROWS) : (s32)ty - (s32)CELL_ROWS;
    }
    for (u32 r = 0; r < rows; ++r) {
        u32 h = cell_of(s, seed, cx, cell_row[r], layer);
        u32 left = cell_of(s, seed, cx - 1U, cell_row[r], layer), right = cell_of(s, seed, cx + 1U, cell_row[r], layer);
        own[r] = texture_at(texture, h, row_y[r], h);
        /* The cell on the left continues at its sample CELL, the cell on
           the right starts BLEND samples before its sample 0. */
        grain_hblend(r ? s->other : edge, own[r], texture_at(texture, left, row_y[r], left + CELL), own[r] + CELL - BLEND,
                     texture_at(texture, right, row_y[r], right + TILE_W - BLEND), s->edge_weights);
    }
    if (rows == 1U) {
        out->part[1] = own[0];
    } else {
        u32 w = grain_blend_weight[d], weights = w | (256U - w) << 8 | (u32)grain_blend_norm[d] << 16;
        grain_vblend(edge, edge, s->other, 2U * BLEND, weights);
        grain_vblend(full + BLEND, own[0] + BLEND, own[1] + BLEND, CELL - 2U * BLEND, weights);
        out->part[1] = full;
    }
    out->part[0] = edge;
    out->part[2] = edge + BLEND - (CELL - BLEND);
}

/* The blocks of 8 samples of a cell that the visible area x0 to x1 holds
   whole, b0 to b1 (none: both CELL_BLOCKS), and the samples before and
   after them, x0 to n0 and n1 to x1, which C does. Each cell row is
   CELL_BLOCKS blocks: the first and the last read the blended edges, the
   others the texture or the blended row (grain_noise). */
#define CELL_BLOCKS (CELL / 8U)
_Static_assert(BLEND == 8U, "An edge of a cell is one block of 8 samples");
static const u32 part_blocks[3][2] = {{0, 1}, {1, CELL_BLOCKS - 1U}, {CELL_BLOCKS - 1U, CELL_BLOCKS}};
typedef struct {
    u32 cell, x0, x1, b0, b1, n0, n1;
} CellSpan;
static void cell_span(CellSpan *c, const Plane *p, u32 cx)
{
    c->cell = cx * CELL;
    c->x0 = c->cell > p->left ? c->cell : p->left;
    c->x1 = c->cell + CELL < p->right ? c->cell + CELL : p->right;
    c->b0 = (c->x0 - c->cell + 7U) >> 3;
    c->b1 = (c->x1 - c->cell) >> 3;
    if (c->b0 >= c->b1) c->b0 = c->b1 = CELL_BLOCKS;
    c->n0 = c->b0 < CELL_BLOCKS ? c->cell + (c->b0 << 3) : c->x1;
    c->n1 = c->b0 < CELL_BLOCKS ? c->cell + (c->b1 << 3) : c->x1;
}

/* The grain of a mono picture: one layer, in Y, the blocks in NEON
   (grain_mono_blocks) and the samples around them in C. */
static void grain_mono(const Plane *p, Scratch *s, u32 seed, int scale)
{
    for (u32 y = p->top; y < p->bottom; ++y) {
        u8 *row = (u8 *)GRKIT_BYTES(p->y + y * p->pitch);
        cell_hashes(s, seed, y / CELL_ROWS, 1, 1);
        for (u32 cx = p->left / CELL; cx * CELL < p->right; ++cx) {
            CellSpan c;
            NoiseRow z;
            cell_span(&c, p, cx);
            grain_noise(s, &z, s->edges[0][1], s->noise[0][1], 1, seed, y, cx);
            for (u32 part = 0; part < 3U; ++part) {
                u32 from = c.b0 > part_blocks[part][0] ? c.b0 : part_blocks[part][0];
                u32 to = c.b1 < part_blocks[part][1] ? c.b1 : part_blocks[part][1];
                if (from >= to) continue;
                GrainBlocks b;
                b.y = row + c.cell + (from << 3);
                b.z[0] = z.part[part] + (from << 3);
                b.groups = to - from;
                b.constants = s->constants;
                grain_mono_blocks(&b);
            }
            u32 runs[2][2] = {{c.x0, c.n0}, {c.n1, c.x1}};
            for (u32 run = 0; run < 2U; ++run)
                for (u32 x = runs[run][0]; x < runs[run][1]; ++x) {
                    u32 t = x - c.cell;
                    mono_pixel(row + x, z.part[part_of(t)][t], scale);
                }
        }
    }
}

/* The grain of the luma samples x from a to b (excluded) of row k of a cell
   in C: the pairs that NEON does not take, at the edges of the visible
   area. With `first` the sums of each pair start. */
static void film_pairs_c(Scratch *s, u8 *row, const u8 *c, const NoiseRow *z, u32 cell, u32 a, u32 b, u32 first, int scale)
{
    for (u32 j = a >> 1; j < (b + 1U) >> 1; ++j) {
        int offset[LAYERS];
        short sum[LAYERS];
        u32 pair = j - (cell >> 1);
        film_offsets(c + (j << 1), offset);
        for (u32 l = 0; l < LAYERS; ++l) sum[l] = first ? 0 : s->pairs[l][pair];
        for (u32 x = j << 1; x < (j << 1) + 2U; ++x) {
            if (x < a || x >= b) continue;
            u32 t = x - cell, part = part_of(t);
            u8 zs[LAYERS] = {z[0].part[part][t], z[1].part[part][t], z[2].part[part][t]};
            film_pixel(row + x, offset, zs, sum, scale);
        }
        for (u32 l = 0; l < LAYERS; ++l) s->pairs[l][pair] = sum[l];
    }
}

/* The change of Cb and Cr of the pairs of the samples a to b (excluded) of
   a cell in C, from their sums, with `rows` luma rows. */
static void film_chroma_c(Scratch *s, u8 *c, u32 cell, u32 a, u32 b, u32 rows)
{
    for (u32 j = a >> 1; j < (b + 1U) >> 1; ++j) {
        u32 n = 0, pair = j - (cell >> 1);
        short sum[LAYERS] = {s->pairs[0][pair], s->pairs[1][pair], s->pairs[2][pair]};
        for (u32 x = j << 1; x < (j << 1) + 2U; ++x) n += x >= a && x < b;
        film_chroma(c + (j << 1), sum, n * rows);
    }
}

/* The grain of a color picture works on the pairs of luma samples that
   share a CbCr sample (one pair in each luma row, two rows with `half`):
   each luma sample gets the grain of R, G, and B in density with the tone
   of its own R, G, and B, and its change of Y (film_pixel); the grain of
   each layer is summed over the samples of the pair, and the pair gets the
   mean change of Cb and Cr (film_chroma). NEON does the blocks of 4 pairs
   (8 samples) of a cell that the visible area holds whole (cell_span):
   with one luma row in a single pass (grain_film_blocks), with two in a
   pass for each row and one for the chroma. C does the samples around
   them. */
static void grain_color(const Plane *p, Scratch *s, u32 seed, int scale)
{
    u32 first = p->half ? p->top >> 1 : p->top, last = p->half ? (p->bottom + 1U) >> 1 : p->bottom;
    for (u32 r = first; r < last; ++r) {
        u32 rows = p->half ? 2U : 1U, y0 = p->half ? r << 1 : r, line[2];
        u8 *luma[2];
        u32 count = 0;
        for (u32 k = 0; k < rows; ++k) {
            u32 y = y0 + k;
            if (y < p->top || y >= p->bottom) continue;
            luma[count] = (u8 *)GRKIT_BYTES(p->y + y * p->pitch);
            line[count++] = y;
        }
        if (!count) continue;
        u8 *c = (u8 *)GRKIT_BYTES(p->c + r * p->pitch);
        for (u32 cx = p->left / CELL; cx * CELL < p->right; ++cx) {
            CellSpan span;
            cell_span(&span, p, cx);
            u32 cell = span.cell, x0 = span.x0, x1 = span.x1, b0 = span.b0, b1 = span.b1, n0 = span.n0, n1 = span.n1;
            NoiseRow z[2][LAYERS];
            for (u32 k = 0; k < count; ++k) {
                cell_hashes(s, seed, line[k] / CELL_ROWS, 0, LAYERS);
                for (u32 l = 0; l < LAYERS; ++l) grain_noise(s, &z[k][l], s->edges[k][l], s->noise[k][l], l, seed, line[k], cx);
            }
            for (u32 k = 0; k < count; ++k) {
                for (u32 part = 0; part < 3U; ++part) {
                    u32 from = b0 > part_blocks[part][0] ? b0 : part_blocks[part][0];
                    u32 to = b1 < part_blocks[part][1] ? b1 : part_blocks[part][1];
                    if (from >= to) continue;
                    GrainBlocks b;
                    b.y = luma[k] + cell + (from << 3);
                    b.c = c + cell + (from << 3);
                    for (u32 l = 0; l < LAYERS; ++l) {
                        b.z[l] = z[k][l].part[part] + (from << 3);
                        b.sum[l] = s->pairs[l] + (from << 2);
                    }
                    b.groups = to - from;
                    b.constants = s->constants;
                    b.keep = k ? ~0U : 0;
                    if (count == 1U) grain_film_blocks(&b);
                    else grain_luma_blocks(&b);
                }
                film_pairs_c(s, luma[k], c, z[k], cell, x0, n0, !k, scale);
                film_pairs_c(s, luma[k], c, z[k], cell, n1, x1, !k, scale);
            }
            if (count == 2U && b0 < b1) {
                GrainChroma b;
                b.c = c + cell + (b0 << 3);
                for (u32 l = 0; l < LAYERS; ++l) b.sum[l] = s->pairs[l] + (b0 << 2);
                b.groups = b1 - b0;
                b.shift = 8;
                grain_chroma_blocks(&b);
            }
            film_chroma_c(s, c, cell, x0, n0, count);
            film_chroma_c(s, c, cell, n1, x1, count);
        }
    }
}

/* The grain size of a shot for CORE.GRAIN_SIZE Auto: by its ISO. */
static u32 size_of_iso(const PictureSet *set)
{
    GrkitShot shot;
    shot.size = sizeof(GrkitShot);
    shot.known = 0;
    picture_shot(set, &shot);
    if (!(shot.known & GRKIT_SHOT_EXPOSURE)) return 1;
    return shot.iso <= 400U ? 1U : shot.iso <= 800U ? 2U : 3U;
}

/* The seed of a picture: some samples of the main picture and the shot, so
   that each picture gets its own grain, and the same picture the same. */
static u32 seed_of(const PictureSet *set, const Plane *main)
{
    GrkitShot shot;
    u32 seed = 0x6a09e667U;
    shot.size = sizeof(GrkitShot);
    shot.known = 0;
    picture_shot(set, &shot);
    /* Only what the core filled: the rest of `shot` is undefined. */
    if (shot.known & GRKIT_SHOT_EXPOSURE) seed ^= shot.exposure_us * 0x9e3779b1U ^ (u32)shot.sv << 7 ^ (u32)shot.av << 19;
    if (shot.known & GRKIT_SHOT_TAKEN)
        seed ^= (u32)shot.taken.second << 24 ^ (u32)shot.taken.minute << 16 ^ (u32)shot.taken.hour << 8;
    for (u32 i = 1; i <= 64U; ++i) {
        u32 y = main->top + divide((main->bottom - main->top) * i, 65U);
        u32 x = main->left + divide((main->right - main->left) * i, 65U);
        seed = (seed ^ GRKIT_BYTES(main->y + y * main->pitch + x)[0]) * 0x01000193U;
    }
    return seed;
}

/* Film-like grain of `level` 1 to 4 and `size` 1 to 3 on each plane of the
   set. */
void grain_draw(PictureSet *set, u32 level, u32 size)
{
    if (!level || level > 4U || !size || size > 3U || !set->count) return;
    const Plane *main = &set->planes[0];
    for (u32 i = 1; i < set->count; ++i)
        if (set->planes[i].width > main->width) main = &set->planes[i];
    if (main->right <= main->left || main->bottom <= main->top) return;
    Scratch *s = (Scratch *)(void *)set->scratch;
    u32 state = seed_of(set, main);
    grain_tiles(s, set->mono ? 1U : TEXTURES, area_halves[level - 1U], blur_of_size[size - 1U], &state);
    for (u32 i = 0; i < set->count; ++i) {
        const Plane *p = &set->planes[i];
        if (p->right <= p->left || p->bottom <= p->top) continue;
        u32 seed = mix(&state), scale = grain_scale(level, p->width, main->width);
        s->cell_row = ~0U;
        grain_constants(s, scale);
        if (set->mono) grain_mono(p, s, seed, (int)scale);
        else grain_color(p, s, seed, (int)scale);
    }
}

/* The filter of CORE.GRAIN: the core calls it with the strength 1 to 4.
   In the System style the native grain stage draws the grain. */
void grain_apply(PictureSet *set, u32 level)
{
    if (setting_get(SETTING_STYLE) != 1U) return;
    u32 size = setting_get(USED_CORE_GRAIN_SIZE);
    grain_draw(set, level, size ? size : size_of_iso(set));
}
