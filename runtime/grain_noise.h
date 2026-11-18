#ifndef GRKIT_GRAIN_NOISE_H
#define GRKIT_GRAIN_NOISE_H
/* The noise of the GRKit grain (runtime/picture.c: grain_noise): the random
   bytes, the blurred tiles, and the plane. The code reads and writes only
   what its arguments give, so a test can build it alone. The includer
   defines u8, u16, u32, and core_divide. */
/* The camera objects build without an FPU (CAMERA_TARGET in
   crates/grkit-build), so the NEON code of the camera is an assembly part
   (GRAIN_BLOCKS). The native noise fill that grain_noise replaces is NEON
   code, so its task keeps the NEON registers. The assembly part uses d0 to
   d7 and d16 to d29, which a call does not preserve. An ARM host builds the
   same arithmetic from intrinsics (GRAIN_NEON), and other hosts and
   GRAIN_SCALAR build the C code. The tests compare each with one reference:
   tests/runtime/picture_contract.c on the host, and
   synthetic_grain_neon_parity (crates/grkit-build) for the assembly part. */
#if defined(GRAIN_SCALAR)
#elif defined(__arm__)
#define GRAIN_BLOCKS 1
#elif defined(__ARM_NEON)
#include <arm_neon.h>
#define GRAIN_NEON 1
#endif
/* Four xorshift128 generators side by side: generator i gives bytes 4i to
   4i + 3 of each 16 bytes, in little-endian order. */
typedef struct { u32 a[4],b[4],c[4],d[4]; } GrainRandom;
/* A word of a byte buffer. */
typedef u32 __attribute__((may_alias)) GrainWord;
/* splitmix32 of the count, so that close counts give unrelated states. */
static u32 grain_mix(u32 *x) {
    u32 z=(*x+=0x9e3779b9U);
    z=(z^(z>>16))*0x85ebca6bU;
    z=(z^(z>>13))*0xc2b2ae35U;
    return z^(z>>16);
}
static void grain_seed(GrainRandom *r,u32 seed) {
    u32 i;
    for(i=0;i<4;++i) { r->a[i]=grain_mix(&seed); r->b[i]=grain_mix(&seed); r->c[i]=grain_mix(&seed); r->d[i]=grain_mix(&seed)|1U; }
}
/* The sample of the plane for each sum of two half deviations, 0 to 510:
   128 plus the deviation of the sum times sqrt 2, limited to 0..255
   (grain_layers). */
#define GRAIN_SQRT2_PART 13573
#ifdef GRAIN_BLOCKS
/* `blocks` blocks of 16 random bytes (grain_random_row). */
void grain_random_blocks(GrainRandom *r,u8 *row,u32 blocks);
/* `blocks` blocks of 16 samples of the plane from two half deviations
   (grain_add). */
void grain_add_blocks(u8 *out,const u8 *a,const u8 *b,u32 blocks);
/* Copy `blocks` blocks of 16 bytes forwards (grain_repeat). */
void grain_copy_blocks(u8 *out,const u8 *in,u32 blocks);
#define GRAIN_STR_(x) #x
#define GRAIN_STR(x) GRAIN_STR_(x)
/* One step of the four generators: the next 16 bytes from the oldest state
   word A and the newest D, in A and at [r1]. */
#define GRAIN_STEP(A,D) \
    "    vshl.i32 q8, " A ", #11\n" \
    "    veor q8, q8, " A "\n" \
    "    vshr.u32 q9, " D ", #19\n" \
    "    veor q9, q9, " D "\n" \
    "    vshr.u32 " A ", q8, #8\n" \
    "    veor " A ", " A ", q8\n" \
    "    veor " A ", " A ", q9\n" \
    "    vst1.8 {" A "}, [r1]!\n"
__asm__(
    ".pushsection .text.grain_blocks,\"ax\",%progbits\n"
    ".syntax unified\n"
    ".fpu neon\n"
    ".thumb\n"
    ".p2align 2\n"
    ".global grain_random_blocks\n"
    ".type grain_random_blocks, %function\n"
    ".thumb_func\n"
    "grain_random_blocks:\n"
    "    mov r3, r0\n"
    "    vld1.32 {d0, d1, d2, d3}, [r3]!\n"
    "    vld1.32 {d4, d5, d6, d7}, [r3]\n"
    "    subs r2, r2, #4\n"
    "    blo 2f\n"
    /* Four steps rename the state instead of moving it: a b c d become the
       four outputs. */
    "1:\n"
    GRAIN_STEP("q0", "q3")
    GRAIN_STEP("q1", "q0")
    GRAIN_STEP("q2", "q1")
    GRAIN_STEP("q3", "q2")
    "    subs r2, r2, #4\n"
    "    bhs 1b\n"
    "2:  adds r2, r2, #4\n"
    "    beq 4f\n"
    "3:\n"
    GRAIN_STEP("q0", "q3")
    "    vmov q10, q0\n"
    "    vmov q0, q1\n"
    "    vmov q1, q2\n"
    "    vmov q2, q3\n"
    "    vmov q3, q10\n"
    "    subs r2, r2, #1\n"
    "    bne 3b\n"
    "4:  vst1.32 {d0, d1, d2, d3}, [r0]!\n"
    "    vst1.32 {d4, d5, d6, d7}, [r0]\n"
    "    bx lr\n"
    ".size grain_random_blocks, .-grain_random_blocks\n"
    /* (a + b - 256) (1 + GRAIN_SQRT2_PART / 32768) + 128, limited to
       0..255. */
    ".global grain_add_blocks\n"
    ".type grain_add_blocks, %function\n"
    ".thumb_func\n"
    "grain_add_blocks:\n"
    "    cbz r3, 2f\n"
    "    movw r12, #" GRAIN_STR(GRAIN_SQRT2_PART) "\n"
    "    vdup.16 q13, r12\n"
    "    vmov.i16 q12, #256\n"
    "    vmov.i16 q14, #128\n"
    "1:  vld1.8 {q0}, [r1]!\n"
    "    vld1.8 {q1}, [r2]!\n"
    "    vaddl.u8 q8, d0, d2\n"
    "    vaddl.u8 q9, d1, d3\n"
    "    vsub.i16 q8, q8, q12\n"
    "    vsub.i16 q9, q9, q12\n"
    "    vqrdmulh.s16 q10, q8, q13\n"
    "    vqrdmulh.s16 q11, q9, q13\n"
    "    vadd.i16 q8, q8, q10\n"
    "    vadd.i16 q9, q9, q11\n"
    "    vadd.i16 q8, q8, q14\n"
    "    vadd.i16 q9, q9, q14\n"
    "    vqmovun.s16 d0, q8\n"
    "    vqmovun.s16 d1, q9\n"
    "    vst1.8 {q0}, [r0]!\n"
    "    subs r3, r3, #1\n"
    "    bne 1b\n"
    "2:  bx lr\n"
    ".size grain_add_blocks, .-grain_add_blocks\n"
    ".global grain_copy_blocks\n"
    ".type grain_copy_blocks, %function\n"
    ".thumb_func\n"
    "grain_copy_blocks:\n"
    "    cbz r2, 2f\n"
    "1:  vld1.8 {q0}, [r1]!\n"
    "    vst1.8 {q0}, [r0]!\n"
    "    subs r2, r2, #1\n"
    "    bne 1b\n"
    "2:  bx lr\n"
    ".size grain_copy_blocks, .-grain_copy_blocks\n"
    ".popsection\n"
);
#endif
#ifdef GRAIN_NEON
/* One step of the four generators: the next 16 bytes from the oldest state
   word `a` and the newest `d`. */
static inline __attribute__((always_inline)) uint32x4_t grain_step(uint32x4_t a,uint32x4_t d) {
    uint32x4_t t=veorq_u32(a,vshlq_n_u32(a,11));
    return veorq_u32(veorq_u32(d,vshrq_n_u32(d,19)),veorq_u32(t,vshrq_n_u32(t,8)));
}
#endif
/* `pitch` random bytes; a pitch is a multiple of 16. */
static void grain_random_row(GrainRandom *r,u8 *row,u32 pitch) {
#ifdef GRAIN_BLOCKS
    grain_random_blocks(r,row,pitch/16U);
#elif defined(GRAIN_NEON)
    u32 x=0;
    uint32x4_t a=vld1q_u32(r->a),b=vld1q_u32(r->b),c=vld1q_u32(r->c),d=vld1q_u32(r->d),d1,d2,d3,d4;
    /* Four steps rename the state instead of moving it: a b c d become the
       four outputs. */
    for(;x+64<=pitch;x+=64) {
        d1=grain_step(a,d); d2=grain_step(b,d1); d3=grain_step(c,d2); d4=grain_step(d,d3);
        vst1q_u8(row+x,vreinterpretq_u8_u32(d1)); vst1q_u8(row+x+16,vreinterpretq_u8_u32(d2));
        vst1q_u8(row+x+32,vreinterpretq_u8_u32(d3)); vst1q_u8(row+x+48,vreinterpretq_u8_u32(d4));
        a=d1; b=d2; c=d3; d=d4;
    }
    for(;x+16<=pitch;x+=16) {
        d1=grain_step(a,d);
        a=b; b=c; c=d; d=d1;
        vst1q_u8(row+x,vreinterpretq_u8_u32(d1));
    }
    vst1q_u32(r->a,a); vst1q_u32(r->b,b); vst1q_u32(r->c,c); vst1q_u32(r->d,d);
#else
    u32 x,i,t,v,word=!((u32)row&3U);
    for(x=0;x+16<=pitch;x+=16) for(i=0;i<4;++i) {
        t=r->a[i]^(r->a[i]<<11);
        r->a[i]=r->b[i]; r->b[i]=r->c[i]; r->c[i]=r->d[i];
        v=r->d[i]=r->d[i]^(r->d[i]>>19)^t^(t>>8);
        /* The camera is little-endian, so a word store writes the four bytes. */
        if(word) ((GrainWord *)(void *)(row+x))[i]=v;
        else { row[x+4*i]=(u8)v; row[x+4*i+1]=(u8)(v>>8); row[x+4*i+2]=(u8)(v>>16); row[x+4*i+3]=(u8)(v>>24); }
    }
#endif
}
/* Medium and large grain add two tiles of blurred noise. A blur of the whole
   plane took about half a second for each shot on a GR III (GRKIT/PROFILE),
   and the camera waited for it before playback and power-off. Each shot
   blurs two new tiles of random bytes, 256 and 240 samples wide. Each tile
   repeats across the plane from its own random place, and the plane gets
   their sum. A tile wraps at its edges, so neither has a seam, and the sum
   repeats only every 3840 samples, the least common multiple of the widths.
   Each row and column of a tile then has the mean 128 (grain_center).
   Patches of one tile left seams that showed as a grid in the photos.

   The tile blur is the binomial filter of 1 2 1 or 1 4 6 4 1, down the
   columns and then along the rows. The weights of the two passes sum to 16
   for 3 taps and to 256 for 5 taps, so the mean of random bytes is 128 times
   that sum. The filter scales the standard deviation by the sum of the
   squared weights of one pass, 6 or 70, so the deviation from the mean
   divided by 6 or 70 has the standard deviation of one random byte. The
   division is a multiply by 5461 (1 / 6.0004) or 468 (1 / 70.02). A tile
   keeps 128 plus half of that deviation, so it rarely reaches 0 or 255.
   The plane gets 128 plus the sum of the two half deviations times sqrt 2:
   the sum plus 13573 / 32768 (sqrt 2 - 1) of it, limited to 0..255 once,
   with the standard deviation of one random byte. */
#define GRAIN_GAIN(taps) ((taps)==3 ? 5461 : 468)
#define GRAIN_TILE_A 256U
#define GRAIN_TILE_B 240U
#define GRAIN_NOISE_BYTES (GRAIN_TILE_A*GRAIN_TILE_A+GRAIN_TILE_B*GRAIN_TILE_B+6U*GRAIN_TILE_A)
_Static_assert(!(GRAIN_TILE_A%16U) && !(GRAIN_TILE_B%16U) && GRAIN_TILE_B<=GRAIN_TILE_A,
               "Each tile row is whole 16-byte blocks, and the work rows fit both tiles");
/* Copy `n` bytes, a multiple of 16. */
static void grain_copy(u8 *out,const u8 *in,u32 n) {
#ifdef GRAIN_BLOCKS
    grain_copy_blocks(out,in,n/16U);
#elif defined(GRAIN_NEON)
    u32 i;
    for(i=0;i<n;i+=16) vst1q_u8(out+i,vld1q_u8(in+i));
#else
    u32 i;
    for(i=0;i<n;++i) out[i]=in[i];
#endif
}
/* Blur the random n x n tile in place, with its edges wrapped. `keep` holds
   four rows of the random bytes: rows 0 and 1, which the last rows read, and
   the two rows above the current one. `sums` holds the column sums of one
   row. */
static void grain_tile(u8 *tile,u32 n,u8 *keep,u16 *sums,u32 taps) {
    static const u8 k3[3]={1,2,1},k5[5]={1,4,6,4,1};
    const u8 *k=taps==3 ? k3 : k5, *rows[5];
    u32 half=taps/2,x,y,j;
    int s,v;
    grain_copy(keep,tile,2*n);
    for(y=0;y<n;++y) {
        for(j=0;j<taps;++j) {
            int r=(int)y+(int)j-(int)half;
            if(r<0) rows[j]=tile+(u32)(r+(int)n)*n;
            else if((u32)r>=n) rows[j]=keep+((u32)r-n)*n;
            else if((u32)r<y) rows[j]=keep+(2+((u32)r&1))*n;
            else rows[j]=tile+(u32)r*n;
        }
        for(x=0;x<n;++x) {
            u32 c=0;
            for(j=0;j<taps;++j) c+=k[j]*rows[j][x];
            sums[x]=(u16)c;
        }
        /* Row y - 2 has its last reader above. */
        grain_copy(keep+(2+(y&1))*n,tile+y*n,n);
        for(x=0;x<n;++x) {
            /* Column x - half + j, wrapped: a runs from n - half to 2n - 1 + half. */
            u32 a=x+n-half;
            s=-128*(taps==3 ? 16 : 256);
            for(j=0;j<taps;++j,++a) s+=k[j]*sums[a<n ? a : a-n<n ? a-n : a-2*n];
            v=128+((2*s*GRAIN_GAIN(taps)+65536)>>17);
            tile[y*n+x]=(u8)(v<0 ? 0 : v>255 ? 255 : v);
        }
    }
}
/* The rounded mean of n values that sum to `sum`. */
static int grain_mean(int sum,u32 n) {
    return sum<0 ? -(int)core_divide((u32)-sum+n/2U,n) : (int)core_divide((u32)sum+n/2U,n);
}
static u8 grain_shift(u8 v,int d) { int s=(int)v-d; return (u8)(s<0 ? 0 : s>255 ? 255 : s); }
/* Bring the mean of each row, then of each column, of the tile to 128. A
   row of a tile repeats along the whole plane, so the small mean of its
   samples did not average away and showed as a faint line across the photo;
   a column did the same downwards. */
static void grain_center(u8 *tile,u32 n) {
    u32 x,y;
    int s,d;
    for(y=0;y<n;++y) {
        for(x=0,s=0;x<n;++x) s+=(int)tile[y*n+x]-128;
        d=grain_mean(s,n);
        for(x=0;x<n;++x) tile[y*n+x]=grain_shift(tile[y*n+x],d);
    }
    for(x=0;x<n;++x) {
        for(y=0,s=0;y<n;++y) s+=(int)tile[y*n+x]-128;
        d=grain_mean(s,n);
        for(y=0;y<n;++y) tile[y*n+x]=grain_shift(tile[y*n+x],d);
    }
}
/* The samples of the plane by the sum of two half deviations (see
   GRAIN_SQRT2_PART). */
#define GRAIN_SUMS 511U
static u8 grain_sample(u32 sum) {
    int s=(int)sum-256, v=128+s+((2*s*GRAIN_SQRT2_PART+32768)>>16);
    return (u8)(v<0 ? 0 : v>255 ? 255 : v);
}
/* `n` samples of the plane, a multiple of 16, from two half deviations. The
   C code reads the samples from `sums` (grain_sample of each sum). */
static void grain_add(u8 *out,const u8 *a,const u8 *b,u32 n,const u8 *sums) {
#ifdef GRAIN_BLOCKS
    (void)sums;
    grain_add_blocks(out,a,b,n/16U);
#elif defined(GRAIN_NEON)
    u32 x;
    (void)sums;
    for(x=0;x<n;x+=16) {
        uint8x16_t va=vld1q_u8(a+x),vb=vld1q_u8(b+x);
        int16x8_t lo=vsubq_s16(vreinterpretq_s16_u16(vaddl_u8(vget_low_u8(va),vget_low_u8(vb))),vdupq_n_s16(256));
        int16x8_t hi=vsubq_s16(vreinterpretq_s16_u16(vaddl_u8(vget_high_u8(va),vget_high_u8(vb))),vdupq_n_s16(256));
        lo=vaddq_s16(vaddq_s16(lo,vqrdmulhq_s16(lo,vdupq_n_s16(GRAIN_SQRT2_PART))),vdupq_n_s16(128));
        hi=vaddq_s16(vaddq_s16(hi,vqrdmulhq_s16(hi,vdupq_n_s16(GRAIN_SQRT2_PART))),vdupq_n_s16(128));
        vst1q_u8(out+x,vcombine_u8(vqmovun_s16(lo),vqmovun_s16(hi)));
    }
#else
    u32 i;
    for(i=0;i<n;++i) out[i]=sums[a[i]+b[i]];
#endif
}
/* The sum of the two tiles repeats every GRAIN_PERIOD samples along a row,
   and every GRAIN_PERIOD rows down the plane: the least common multiple of
   the tile widths. */
#define GRAIN_PERIOD 3840U
_Static_assert(!(GRAIN_PERIOD%GRAIN_TILE_A) && !(GRAIN_PERIOD%GRAIN_TILE_B) && !(GRAIN_PERIOD%16U),
               "A period holds whole tiles and whole 16-sample blocks");
/* Copy `n` bytes from `in` to `out` forwards, so a copy onto a later part of
   its own source repeats it. A multiple of 16 bytes. */
static void grain_repeat(u8 *out,const u8 *in,u32 n) {
#ifdef GRAIN_BLOCKS
    grain_copy_blocks(out,in,n/16U);
#else
    u32 i;
    if(!(((u32)out|(u32)in)&3U)) {
        GrainWord *o=(GrainWord *)(void *)out;
        const GrainWord *s=(const GrainWord *)(const void *)in;
        for(i=0;i<n/4U;++i) o[i]=s[i];
    } else {
        for(i=0;i<n;++i) out[i]=in[i];
    }
#endif
}
/* Fill the plane with the sum of the two tiles, each from the place that
   `place_a` and `place_b` give: the start column in 16-sample blocks in the
   low half, the start row in the high half. The samples after the first
   period of a row, and the rows after the first period, repeat earlier ones. */
static void grain_layers(u8 *plane,u32 pitch,u32 height,const u8 *a,const u8 *b,u32 place_a,u32 place_b) {
    u32 y,x,run,row_a=place_a>>16,row_b=place_b>>16,width=pitch<GRAIN_PERIOD ? pitch : GRAIN_PERIOD;
    u8 sums[GRAIN_SUMS];
    for(x=0;x<GRAIN_SUMS;++x) sums[x]=grain_sample(x);
    for(y=0;y<height;++y) {
        const u8 *line_a=a+row_a*GRAIN_TILE_A,*line_b=b+row_b*GRAIN_TILE_B;
        u32 column_a=(place_a&0xffffU)*16U,column_b=(place_b&0xffffU)*16U;
        u8 *out=plane+y*pitch;
        if(y>=GRAIN_PERIOD) grain_repeat(out,out-GRAIN_PERIOD*pitch,pitch);
        else {
            /* Each run ends where the row or a tile ends. */
            for(x=0;x<width;x+=run) {
                run=width-x;
                if(run>GRAIN_TILE_A-column_a) run=GRAIN_TILE_A-column_a;
                if(run>GRAIN_TILE_B-column_b) run=GRAIN_TILE_B-column_b;
                grain_add(out+x,line_a+column_a,line_b+column_b,run,sums);
                column_a+=run; if(column_a==GRAIN_TILE_A) column_a=0;
                column_b+=run; if(column_b==GRAIN_TILE_B) column_b=0;
            }
            if(pitch>width) grain_repeat(out+width,out,pitch-width);
        }
        if(++row_a==GRAIN_TILE_A) row_a=0;
        if(++row_b==GRAIN_TILE_B) row_b=0;
    }
}
/* A place in a tile of width n from a random word. The divisions are by
   constants, which compile to multiplies. */
static u32 grain_place(u32 p,u32 n) {
    return (((p>>8)%n)<<16)|((p&0xffU)%(n/16U));
}
/* Fill the plane of `pitch` x `height` samples with the noise of a seed:
   one random byte each without `work`, as for small grain, else the tiles
   with a blur of `taps` taps, 3 for medium and 5 for large grain, in the
   GRAIN_NOISE_BYTES at `work`. A pitch is a multiple of 16. */
static void grain_plane(u8 *plane,u32 pitch,u32 height,u32 seed,u32 taps,u8 *work) {
    GrainRandom r;
    u32 y;
    grain_seed(&r,seed);
    if(!work) {
        for(y=0;y<height;++y) grain_random_row(&r,plane+y*pitch,pitch);
    } else {
        u8 *a=work,*b=a+GRAIN_TILE_A*GRAIN_TILE_A,*keep=b+GRAIN_TILE_B*GRAIN_TILE_B;
        u16 *sums=(u16 *)(keep+4*GRAIN_TILE_A);
        u32 place_a,place_b;
        for(y=0;y<GRAIN_TILE_A;++y) grain_random_row(&r,a+y*GRAIN_TILE_A,GRAIN_TILE_A);
        for(y=0;y<GRAIN_TILE_B;++y) grain_random_row(&r,b+y*GRAIN_TILE_B,GRAIN_TILE_B);
        grain_tile(a,GRAIN_TILE_A,keep,sums,taps);
        grain_tile(b,GRAIN_TILE_B,keep,sums,taps);
        grain_center(a,GRAIN_TILE_A);
        grain_center(b,GRAIN_TILE_B);
        /* One statement each: the order of the draws is fixed. */
        seed=r.d[0];
        place_a=grain_place(grain_mix(&seed),GRAIN_TILE_A);
        place_b=grain_place(grain_mix(&seed),GRAIN_TILE_B);
        grain_layers(plane,pitch,height,a,b,place_a,place_b);
    }
}
#endif
