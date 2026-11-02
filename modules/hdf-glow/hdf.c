/* HDF Glow: the glow of a highlight diffusion filter on each recorded picture.

   A diffusion filter in front of the lens spreads a little of the light of
   bright areas over their surroundings. The module does the same on the
   YCbCr planes of each JPEG, in three steps:

   1. gather: the light that spills from each cell of a grid over the
      picture, and its colour on a grid half as fine;
   2. spread: blur the grid at three radii, a tight halo and a wide veil;
   3. blend: add the glow to each pixel in linear light as a screen, so
      bright areas never clip harder, and tint it with the colour of the
      lights. HALO adds the red ring of CineStill film around the lights.

   The grids fit the scratch memory of the call: a 24 MP picture uses cells
   of 32 x 32 pixels. The glow is soft, so the cells do not show. The live
   view and DNG files stay as they are. */
#include "grkit.h"
#include "grkit_module.h"
#include "hdf_tables.h"

void hdf_apply(PictureSet *set, u32 strength);
u32 hdf_grid_shift(u32 width, u32 height);

#define MAX_GRID 256U
/* Glow of each strength, x256: Off, then 1/8, 1/4, and 1/2 of a Black Pro-Mist. */
static const u16 AMOUNT[4] = {0, 70, 141, 282};
/* The CineStill halation: red-orange light (1, 0.2, 0.06), its chroma and its
   luma in linear light (x256). */
#define HALO_CB (-50)
#define HALO_CR 70
#define HALO_LUMA 92U

typedef struct {
    u32 left, top, w, h;     /* the visible area */
    u32 shift, gw, gh;       /* cells of 1 << shift pixels; the coarse grid has cells twice as large */
    u32 cw, ch;
    u16 *e, *g;              /* spilled light, then glow, per cell */
    u16 *wt, *nb, *nr;       /* coarse: spilled light, and its Cb and Cr (offset 32768) */
    u16 *line, *row, *rb, *rr;
    u32 *acc, *accw;
    int *accb, *accr;
} Grid;

typedef struct { u32 i0, i1, f; } Lerp;

static u32 align4(u32 n) { return (n + 3U) & ~3U; }
static u32 min_u(u32 a, u32 b) { return a < b ? a : b; }
static u32 clamp8(int v) { return v < 0 ? 0U : v > 255 ? 255U : (u32)v; }
static int sdiv(int n, u32 d) { return n < 0 ? -(int)divide((u32)-n, d) : (int)divide((u32)n, d); }

static u32 grid_bytes(u32 gw, u32 gh, u32 cw, u32 ch)
{
    return align4(gw * gh * 2U) * 2U + align4(cw * ch * 2U) * 3U + align4((gw > gh ? gw : gh) * 2U) +
           align4((gw + 1U) * 2U) + align4((cw + 1U) * 2U) * 2U + gw * 4U + cw * 16U;
}

/* The shift of the finest grid of a width x height area that fits the
   scratch memory, or 32 when none does. */
u32 hdf_grid_shift(u32 width, u32 height)
{
    u32 s;
    for (s = 0; s < 12U; ++s) {
        u32 cell = 1U << s, gw = (width + cell - 1U) >> s, gh = (height + cell - 1U) >> s;
        if (gw <= MAX_GRID && gh <= MAX_GRID &&
            grid_bytes(gw, gh, (gw + 1U) >> 1, (gh + 1U) >> 1) <= GRKIT_PLANE_SCRATCH)
            return s;
    }
    return 32U;
}

static u32 layout(Grid *gr, const Plane *p, u8 *m)
{
    u32 n;
    if (p->right > p->width || p->bottom > p->height || p->right <= p->left || p->bottom <= p->top) return 0;
    gr->left = p->left; gr->top = p->top;
    gr->w = p->right - p->left; gr->h = p->bottom - p->top;
    gr->shift = hdf_grid_shift(gr->w, gr->h);
    if (gr->shift == 32U) return 0;
    gr->gw = (gr->w + (1U << gr->shift) - 1U) >> gr->shift;
    gr->gh = (gr->h + (1U << gr->shift) - 1U) >> gr->shift;
    gr->cw = (gr->gw + 1U) >> 1; gr->ch = (gr->gh + 1U) >> 1;
    n = align4(gr->gw * gr->gh * 2U);
    gr->e = (u16 *)m; m += n;
    gr->g = (u16 *)m; m += n;
    n = align4(gr->cw * gr->ch * 2U);
    gr->wt = (u16 *)m; m += n;
    gr->nb = (u16 *)m; m += n;
    gr->nr = (u16 *)m; m += n;
    gr->line = (u16 *)m; m += align4((gr->gw > gr->gh ? gr->gw : gr->gh) * 2U);
    gr->row = (u16 *)m; m += align4((gr->gw + 1U) * 2U);
    gr->rb = (u16 *)m; m += align4((gr->cw + 1U) * 2U);
    gr->rr = (u16 *)m; m += align4((gr->cw + 1U) * 2U);
    gr->acc = (u32 *)m; m += gr->gw * 4U;
    gr->accw = (u32 *)m; m += gr->cw * 4U;
    gr->accb = (int *)m; m += gr->cw * 4U;
    gr->accr = (int *)m;
    return 1;
}

/* The samples from `from` to `to` every 1 << sshift pixels. */
static u32 samples(u32 from, u32 to, u32 sshift)
{
    return to > from ? (to - from + (1U << sshift) - 1U) >> sshift : 0U;
}

/* Step 1: the mean spilled light of each cell, in linear light (4095 is
   white, clipped areas up to 3x), and on the coarse grid its Cb and Cr. */
static void gather(const Plane *p, const Grid *gr, u32 mono)
{
    /* Big cells sample every second pixel: at most 16 x 16 samples a cell. */
    u32 sshift = gr->shift > 4U ? gr->shift - 4U : 0U, step = 1U << sshift;
    u32 cell = 1U << gr->shift, right = gr->left + gr->w, bottom = gr->top + gr->h;
    u32 cy, gx, crows = 0;
    for (cy = 0; cy < gr->gh; ++cy) {
        u32 y, y0 = gr->top + (cy << gr->shift), y1 = min_u(y0 + cell, bottom);
        u32 rows = samples(y0, y1, sshift);
        for (gx = 0; gx < gr->gw; ++gx) gr->acc[gx] = 0;
        if (!(cy & 1U)) {
            for (gx = 0; gx < gr->cw; ++gx) {
                gr->accw[gx] = 0;
                gr->accb[gx] = 0;
                gr->accr[gx] = 0;
            }
            crows = 0;
        }
        crows += rows;
        for (y = y0; y < y1; y += step) {
            volatile u8 *luma = GRKIT_BYTES(p->y + y * p->pitch);
            volatile u8 *crow = GRKIT_BYTES(chroma_row(p, y));
            u32 x;
            for (x = gr->left; x < right; x += step) {
                int yv = luma[x], cb = 0, cr = 0;
                u32 r, g, b, mx, spill, e, at;
                if (!mono) {
                    cb = (int)crow[x & ~1U] - 128;
                    cr = (int)crow[(x & ~1U) + 1U] - 128;
                }
                r = clamp8(yv + ((359 * cr) >> 8));
                g = clamp8(yv - ((88 * cb + 183 * cr) >> 8));
                b = clamp8(yv + ((454 * cb) >> 8));
                mx = r > g ? r : g;
                if (b > mx) mx = b;
                /* The brightest channel decides, so coloured lamps and neon spill too. */
                spill = SPILL[mx];
                if (!spill) continue;
                e = (((54U * LIN[r] + 183U * LIN[g] + 19U * LIN[b]) >> 8) * spill) >> 8;
                at = (x - gr->left) >> gr->shift;
                gr->acc[at] += e;
                at >>= 1;
                gr->accw[at] += e;
                gr->accb[at] += (int)e * cb;
                gr->accr[at] += (int)e * cr;
            }
        }
        for (gx = 0; gx < gr->gw; ++gx) {
            u32 x0 = gr->left + (gx << gr->shift);
            gr->e[cy * gr->gw + gx] = (u16)divide(gr->acc[gx], samples(x0, min_u(x0 + cell, right), sshift) * rows);
        }
        if ((cy & 1U) || cy + 1U == gr->gh) {
            u32 c, at = (cy >> 1) * gr->cw;
            for (c = 0; c < gr->cw; ++c) {
                u32 x0 = gr->left + (c << (gr->shift + 1U));
                u32 n = samples(x0, min_u(x0 + 2U * cell, right), sshift) * crows;
                gr->wt[at + c] = (u16)divide(gr->accw[c], n);
                gr->nb[at + c] = (u16)(32768 + (sdiv(gr->accb[c], n) >> 7));
                gr->nr[at + c] = (u16)(32768 + (sdiv(gr->accr[c], n) >> 7));
            }
        }
    }
}

/* One box blur of n values `stride` apart, with the edge values repeated. */
static void blur_line(u16 *d, u32 n, u32 stride, u32 r, u16 *tmp, u32 inv)
{
    u32 i, sum;
    for (i = 0; i < n; ++i) tmp[i] = d[i * stride];
    sum = tmp[0] * (r + 1U);
    for (i = 1; i <= r; ++i) sum += tmp[min_u(i, n - 1U)];
    for (i = 0; i < n; ++i) {
        d[i * stride] = (u16)(((unsigned long long)sum * inv + 32768U) >> 16);
        sum += tmp[min_u(i + r + 1U, n - 1U)];
        sum -= tmp[i >= r ? i - r : 0U];
    }
}

/* A Gaussian blur of radius about r cells: three box blurs each way. */
static void blur(u16 *a, u32 w, u32 h, u32 r, u16 *tmp)
{
    /* Rounded, so many passes do not drift: the colour grids sit at 32768. */
    u32 pass, i, inv = divide(65536U + r, 2U * r + 1U);
    for (pass = 0; pass < 3U; ++pass) {
        for (i = 0; i < h; ++i) blur_line(a + i * w, w, 1U, r, tmp, inv);
        for (i = 0; i < w; ++i) blur_line(a + i, h, w, r, tmp, inv);
    }
}

/* A radius given for a grid of 188 cells on its long side, the grid of a
   24 MP picture, so each plane of a shot gets the same glow. */
static u32 radius(u32 base, u32 longest)
{
    u32 r = divide(base * longest + 94U, 188U);
    return r ? r : 1U;
}

/* Step 2: the glow, half a tight halo and half a wide veil, and the colour
   of the light that makes it. */
static void spread(const Grid *gr)
{
    static const u8 BASE[3] = {1, 2, 8}, WEIGHT[3] = {128, 77, 51};
    u32 i, k, r, n = gr->gw * gr->gh, longest = gr->gw > gr->gh ? gr->gw : gr->gh;
    for (i = 0; i < n; ++i) gr->g[i] = 0;
    /* The blurs follow each other, so each radius adds to the last one. */
    for (k = 0; k < 3U; ++k) {
        blur(gr->e, gr->gw, gr->gh, radius(BASE[k], longest), gr->line);
        for (i = 0; i < n; ++i) gr->g[i] = (u16)(gr->g[i] + ((gr->e[i] * WEIGHT[k]) >> 8));
    }
    n = gr->cw * gr->ch;
    r = radius(4U, gr->cw > gr->ch ? gr->cw : gr->ch);
    blur(gr->wt, gr->cw, gr->ch, r, gr->line);
    blur(gr->nb, gr->cw, gr->ch, r, gr->line);
    blur(gr->nr, gr->cw, gr->ch, r, gr->line);
    /* The mean chroma of the light, from its weighted sums. */
    for (i = 0; i < n; ++i) {
        int tb = 0, tr = 0;
        /* Too little light gives no reliable colour: the glow stays neutral. */
        if (gr->wt[i] >= 64U) {
            tb = sdiv(((int)gr->nb[i] - 32768) * 128, gr->wt[i]);
            tr = sdiv(((int)gr->nr[i] - 32768) * 128, gr->wt[i]);
            tb = tb < -127 ? -127 : tb > 127 ? 127 : tb;
            tr = tr < -127 ? -127 : tr > 127 ? 127 : tr;
        }
        gr->nb[i] = (u16)(32768 + tb);
        gr->nr[i] = (u16)(32768 + tr);
    }
}

/* Where pixel `at` falls between the centres of cells of 1 << shift pixels. */
static Lerp lerp(u32 at, u32 shift, u32 n)
{
    Lerp l;
    int pos = (int)((((at << 8) + 128U) >> shift)) - 128;
    if (pos <= 0) {
        l.i0 = l.i1 = 0;
        l.f = 0;
        return l;
    }
    l.i0 = (u32)pos >> 8;
    l.f = (u32)pos & 255U;
    if (l.i0 + 1U >= n) {
        l.i0 = l.i1 = n - 1U;
        l.f = 0;
    } else {
        l.i1 = l.i0 + 1U;
    }
    return l;
}

static u32 mix(const u16 *v, Lerp l) { return (v[l.i0] * (256U - l.f) + v[l.i1] * l.f) >> 8; }

/* Step 3: add the glow to each pixel, and mix the colour of the light into
   the chroma by the share of the glow in the new light. */
static void blend(const Plane *p, const Grid *gr, u32 amount, u32 halo, u32 mono)
{
    u32 right = gr->left + gr->w, bottom = gr->top + gr->h, y, i;
    for (y = gr->top; y < bottom; ++y) {
        volatile u8 *luma = GRKIT_BYTES(p->y + y * p->pitch);
        volatile u8 *crow = GRKIT_BYTES(chroma_row(p, y));
        u32 chroma = !mono && (!p->half || !(y & 1U)), any = 0, x;
        Lerp ly = lerp(y - gr->top, gr->shift, gr->gh);
        const u16 *a = gr->g + ly.i0 * gr->gw, *b = gr->g + ly.i1 * gr->gw;
        for (i = 0; i < gr->gw; ++i) {
            u32 v = (a[i] * (256U - ly.f) + b[i] * ly.f) >> 8;
            gr->row[i] = (u16)v;
            any |= v;
        }
        if (!any) continue;
        if (chroma) {
            Lerp lc = lerp(y - gr->top, gr->shift + 1U, gr->ch);
            const u16 *b0 = gr->nb + lc.i0 * gr->cw, *b1 = gr->nb + lc.i1 * gr->cw;
            const u16 *r0 = gr->nr + lc.i0 * gr->cw, *r1 = gr->nr + lc.i1 * gr->cw;
            for (i = 0; i < gr->cw; ++i) {
                gr->rb[i] = (u16)((b0[i] * (256U - lc.f) + b1[i] * lc.f) >> 8);
                gr->rr[i] = (u16)((r0[i] * (256U - lc.f) + r1[i] * lc.f) >> 8);
            }
        }
        for (x = gr->left; x < right; ++x) {
            u32 glow = mix(gr->row, lerp(x - gr->left, gr->shift, gr->gw));
            u32 yv, l0, l1, l2, add;
            if (!glow) continue;
            yv = luma[x];
            l0 = LIN[yv];
            add = min_u((glow * amount) >> 8, 4095U);
            l1 = l0 + ((add * (4095U - l0)) >> 12);
            l2 = l1;
            if (halo) {
                u32 h = min_u((((glow * HALO_MASK[yv]) >> 8) * HALO_LUMA) >> 8, 4095U);
                l2 = l1 + ((h * (4095U - l1)) >> 12);
            }
            luma[x] = (u8)SRGB[l2];
            if (chroma && !(x & 1U)) {
                Lerp lc = lerp(x - gr->left, gr->shift + 1U, gr->cw);
                u32 rc = RECIP[l2 >> 4];
                int sg = (int)min_u(((l1 - l0) * rc) >> 12, 256U);
                int sh = (int)min_u(((l2 - l1) * rc) >> 12, 256U);
                int tb = (int)mix(gr->rb, lc) - 32768, tr = (int)mix(gr->rr, lc) - 32768;
                int cb = (int)crow[x] - 128, cr = (int)crow[x + 1U] - 128;
                cb += ((tb - cb) * sg) >> 8;
                cr += ((tr - cr) * sg) >> 8;
                cb += ((HALO_CB - cb) * sh) >> 8;
                cr += ((HALO_CR - cr) * sh) >> 8;
                crow[x] = (u8)clamp8(cb + 128);
                crow[x + 1U] = (u8)clamp8(cr + 128);
            }
        }
    }
}

void hdf_apply(PictureSet *set, u32 strength)
{
    u32 i, halo = setting_get(SETTING_HALO);
    Grid gr;
    if (strength > 3U) strength = 3U;
    for (i = 0; i < set->count; ++i) {
        const Plane *p = &set->planes[i];
        if (!layout(&gr, p, set->scratch)) continue;
        gather(p, &gr, set->mono);
        spread(&gr);
        blend(p, &gr, AMOUNT[strength], halo, set->mono);
    }
}
