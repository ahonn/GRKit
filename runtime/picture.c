#include "firmware.h"
#include "abi.h"
#include "core.h"
#include "profile.h"
/* The core of picture.encode, and the post-curve matrix of tuning. The JPEG
   planes, the cache, and the shot ring are firmware and core details, so they
   stay here. Selective color of the effects and the plane filters of the
   modules change the checked planes. */
#ifndef WORD
#define WORD(p,o) (*(volatile u32 *)((p)+(o)))
#endif
#ifndef HALF
#define HALF(p,o) (*(volatile u16 *)((p)+(o)))
#endif
#ifndef BYTE
#define BYTE(p,o) (*(volatile u8 *)((p)+(o)))
#endif
#ifndef PICTURE_YCBCR_CALC
/* YCbCrMatrix calc: object = hue rotation x BT.601, 9-bit coefficients, 256 = 1.0. */
#define PICTURE_YCBCR_CALC(object,context) (((void (*)(u32,u32))FW_YCBCR_CALC)(object,context))
#endif
extern volatile u32 *framing_header(void);

#include "tone.h"
static u32 owned(volatile u32 *h,u32 record) { return record_index(h,record)<GRKIT_TEXT_RECORDS; }
/* Features of an owned record, or 0 for a native one. */
static u32 record_features(volatile u32 *h,u32 record) {
    return owned(h,record) ? WORD(record,GRKIT_RECORD_FEATURES) : 0;
}
/* The tuning context byte of a build, from the Make's third argument (settings +
   0x120): 1 for ImageTuningLiveview, 0 for a still build. module_parameters reads
   the same byte at settings + 0x82ac. */
#define PICTURE_LIVE_CONTEXT 0x818c
/* Replaces the YCbCrMatrix Make call to its calc (0x536e4774). The matrix acts
   on gamma-encoded RGB, so CSP x P applies P after the tone curves, and the hue
   rotation still follows it. The calc stores each coefficient as the low 9 bits
   of a word, and the register writer packs those bits without saturation.
   Selective color changes only recorded pictures, so the live view of such an
   effect drops its chroma rows: the viewfinder shows the luma that the removed
   hues keep in the picture. */
#ifdef NATIVE_SATURATION_BEFORE_CURVES
/* GR IV saturation: a matrix on gamma-encoded RGB, with 9-bit fractions.
   It is RGB x diag(1, f, f) x YCbCr, from the BT.601 matrices with 8-bit
   fractions. f is 1.1 to the power of the level (-4 to 4), rounded to three
   decimals. */
static const s16 saturation_ycbcr[9]={77,150,29,-43,-85,128,128,-107,-21};
static const s16 saturation_rgb[9]={256,0,359,256,-88,-183,256,454,0};
static const u16 saturation_factors[9]={683,751,826,909,1000,1100,1210,1331,1464};
/* The saturation matrix of a native value, as a Q13 post matrix. */
static void picture_saturation(u32 value,int out[9]) {
    int factor=saturation_factors[value],sum;
    u32 i,j;
    for(i=0;i<3;++i) for(j=0;j<3;++j) {
        sum=saturation_rgb[i*3]*saturation_ycbcr[j]*1000
            +(saturation_rgb[i*3+1]*saturation_ycbcr[3+j]+saturation_rgb[i*3+2]*saturation_ycbcr[6+j])*factor;
        /* 512 / (256 x 256 x 1000), rounded half away from zero. */
        out[i*3+j]=(sum<0 ? -((64000-sum)/128000) : (sum+64000)/128000)*16;
    }
}
#endif
static void post_matrix(volatile u32 *h,u32 object,u32 context,u32 settings) {
    u32 record=WORD(context,8),features,i,j,k,apply=0;
    int csp[9],post[9],sum;
    PICTURE_YCBCR_CALC(object,context);
    features=record_features(h,record);
    if(features&GRKIT_FEATURE_POST) {
        for(i=0;i<9;++i) post[i]=(s16)HALF(record,GRKIT_RECORD_POST+i*2);
        apply=1;
    }
#ifdef NATIVE_SATURATION_BEFORE_CURVES
    /* A native look applies saturation after the tone curves and the post
       matrix, as GR IV does. module_parameters gives the native processing
       the neutral value. */
    if((features&GRKIT_FEATURE_NATIVE_LOOK) && (i=record_parameter(record,0))!=4) {
        int saturation[9],product[9];
        picture_saturation(i,saturation);
        if(apply) {
            for(i=0;i<3;++i) for(j=0;j<3;++j) {
                sum=0;
                for(k=0;k<3;++k) sum+=saturation[i*3+k]*post[k*3+j];
                product[i*3+j]=(sum+4096)>>13;
            }
            for(i=0;i<9;++i) post[i]=product[i];
        } else for(i=0;i<9;++i) post[i]=saturation[i];
        apply=1;
    }
#endif
    if(apply) {
        /* The live view builds this matrix each frame: read each word once. */
        for(i=0;i<9;++i) csp[i]=(int)(WORD(object,i*4)<<23)>>23;
        for(i=0;i<3;++i) for(j=0;j<3;++j) {
            sum=0;
            for(k=0;k<3;++k) sum+=csp[i*3+k]*post[k*3+j];
            sum=(sum+4096)>>13;
            WORD(object,(i*3+j)*4)=(u32)(sum<-256 ? -256 : sum>255 ? 255 : sum);
        }
    }
    if((features&GRKIT_FEATURE_SELECTIVE) && BYTE(settings,PICTURE_LIVE_CONTEXT)==1)
        for(i=3;i<9;++i) WORD(object,i*4)=0;
}
void picture_post(u32 object,u32 context,u32 settings) {
    volatile u32 *h=framing_header();
    u32 start=profile_begin(h);
    post_matrix(h,object,context,settings);
    profile_end(h,start,GRKIT_PROFILE_PICTURE_POST);
}
/* ---- Recorded pictures: the encode host -----------------------------------
   JpegEncodeMacroProcess::Execute (0x535047d4) calls picture_encode before it
   submits the encode. Its image descriptors hold Y at +4, CbCr at +8, then u16
   width, height, and the shared pitch at +0x10. The CbCr plane interleaves
   Cb and Cr for each pair of pixels, in full rows (4:2:2) or half rows (4:2:0). */
#ifndef PICTURE_CACHE_FLUSH
/* Clean and invalidate the data cache for a range. */
#define PICTURE_CACHE_FLUSH(address,length) (((void (*)(u32,u32))FW_CACHE_FLUSH)(address,length))
#endif
#ifndef PICTURE_DATE
/* Local time as DateTime: u16 year, then month, day, hour, minute, second. */
#define PICTURE_DATE(out) (((void (*)(u32,u32))FW_LOCAL_TIME)(WORD(((u32 (*)(void))FW_CLOCK)(),4),(u32)(out)))
#endif
#ifndef PICTURE_NATIVE_ASPECT
#define PICTURE_NATIVE_ASPECT() (((u32 (*)(u32))FW_STILL_ASPECT_GET)(((u32 (*)(void))FW_SETTINGS)()))
#endif
#ifndef PICTURE_NATIVE_ID
/* The family header maps a native aspect to its visible-area id. */
#error "The family header defines PICTURE_NATIVE_ID"
#endif
/* Visible-area id of the capture: 0 3:2, 1 4:3, 2 16:9, 3 1:1, 4 XPan,
   5 2.35:1, 6 5:4, 7 7:6. By crop value: Off, XPan, 16:9, 4:3, 2.35:1, 1:1,
   5:4, 7:6. */
static const u8 picture_area_ids[8]={0,4,2,1,5,3,6,7};
#define PICTURE_CROP_AREA(ratio) ((ratio)<8 ? picture_area_ids[ratio] : 0)
#ifdef FRAMING_XPAN_NATIVE
/* Framing ratio values (runtime/firmware.h) apply to native 3:2 captures.
   XPan and 2.35:1 also apply to native FRAMING_XPAN_NATIVE photos. Other
   native aspects ignore them. */
#define PICTURE_AREA_ID(native,ratio) ((native)==0 || \
    ((native)==FRAMING_XPAN_NATIVE && ((ratio)==1 || (ratio)==4)) ? \
    PICTURE_CROP_AREA(ratio) : PICTURE_NATIVE_ID(native))
#else
/* Framing ratio values (runtime/firmware.h) apply only to native 3:2 captures. */
#define PICTURE_AREA_ID(native,ratio) ((native) ? PICTURE_NATIVE_ID(native) : PICTURE_CROP_AREA(ratio))
#endif
#ifndef PICTURE_SCRATCH
/* Stamp masks after the header and the records, inside the loader's registry array. */
#define PICTURE_SCRATCH(h) ((h)[GRKIT_HOST_TAIL/4])
#endif
#ifndef PICTURE_BARRIER
#define PICTURE_BARRIER() __asm__ volatile("dmb ish" ::: "memory")
#endif
#ifndef PICTURE_MAP
#define PICTURE_MAP(address) ((u8 *)(address))
#endif

/* Core services (runtime/core.h). */
/* Shift-subtract division: the freestanding link has no division helper. */
static u32 core_divide(u32 n,u32 d) {
    u32 q=0,bit=1;
    while(d<n && !(d&0x80000000U)) { d<<=1; bit<<=1; }
    while(bit) { if(n>=d) { n-=d; q|=bit; } d>>=1; bit>>=1; }
    return q;
}
/* The core exports these services to modules (runtime/sdk/grkit.h). */
__attribute__((used)) void grkit_export_local_time(u8 *out) { PICTURE_DATE(out); }
__attribute__((used)) u32 grkit_export_divide(u32 n,u32 d) { return core_divide(n,d); }
static void flush(const Plane *p) {
    PICTURE_CACHE_FLUSH(p->y,p->pitch*p->height);
    PICTURE_CACHE_FLUSH(p->c,p->pitch*(p->half ? p->height>>1 : p->height));
}

static u32 aspect_native(void) { return PICTURE_NATIVE_ASPECT(); }
#include "settings.h"
#include "services.h"
/* The module plane filters that INSTALL registered (GRKIT_HOST_MODULE_FILTERS),
   or 0 for a count out of range. Each has two words: its apply function, and
   its setting handle with its stage in the high half. */
#define MODULE_FILTER_MAX GRKIT_MODULE_FILTERS_MAX
static u32 module_filters(volatile u32 *h) {
    u32 n=h[GRKIT_HOST_MODULE_FILTER_COUNT/4];
    return n<=MODULE_FILTER_MAX ? n : 0;
}
#define MODULE_FILTER(h,i) ((h)+GRKIT_HOST_MODULE_FILTERS/4+(i)*2)
/* Apply a module filter. A host test maps the function address. */
#ifndef MODULE_APPLY
#define MODULE_APPLY(address,set,value) ((void (*)(PictureSet *,u32))(address))(set,value)
#endif
#include "selective.h"

/* Visible area of a preview picture: black bars depend on the capture aspect. */
static void visible_area(volatile u32 *h,Plane *p) {
    /* Screennail and thumbnail insets for ids 0 (3:2), 1 (4:3), 2 (16:9), 3 (1:1), then XPan,
       2.35:1, 5:4, and 7:6 (runtime/framing.S). */
    static const u8 screennail[8][2]={{0,0},{40,0},{0,38},{120,0},{0,107},{0,87},{60,0},{80,0}};
    static const u8 thumbnail[8][2]={{0,8},{0,0},{0,16},{20,0},{0,30},{0,26},{6,0},{10,0}};
    /* The capture area: the crop of CORE.ASPECT when a module uses it. */
    u32 ratio=CAPABILITY_USED(h,GRKIT_CORE_ASPECT) ? h[GRKIT_HOST_FRAME_RATIO/4] : 0;
    u32 native=PICTURE_NATIVE_ASPECT(), id;
    const u8 *inset=0;
    id=PICTURE_AREA_ID(native,ratio);
    if(p->width==720 && p->height==480) inset=screennail[id];
    if(p->width==160 && p->height==120) inset=thumbnail[id];
    p->left=inset ? inset[0] : 0; p->top=inset ? inset[1] : 0;
    p->right=p->width-p->left; p->bottom=p->height-p->top;
}
static u32 plane_of(u32 image,u32 code,Plane *p) {
    if(image<0x40000000U || image>0xbfffff00U || code<0x40000000U || code>0xbfffff00U) return 0;
    p->y=WORD(image,4); p->c=WORD(image,8);
    p->width=HALF(image,0x10); p->height=HALF(image,0x12); p->pitch=HALF(image,0x14);
    p->half=BYTE(code,0x10)==2;
    return p->width>=16 && p->height>=16 && p->width<=8192 && p->height<=8192 &&
           p->pitch>=p->width && !(p->width&1) &&
           p->y>=0x40000000U && p->y<0xc0000000U-p->pitch*p->height &&
           p->c>=0x40000000U && p->c<0xc0000000U-p->pitch*p->height;
}
/* A word of the shot ring entry (runtime/abi.h) of the still build of a
   shot: 1 with the word in `out`, or 0 without an entry. Newer entries come
   first. The key is read again after the word, because the build writes the
   ring from another task. */
static u32 shot_word(volatile u32 *h,u32 shot,u32 field,u32 *out) {
    u32 ring=PICTURE_SCRATCH(h)+GRKIT_SHOT_RING, n=WORD(ring,0), i, entry, value;
    for(i=1;shot && i<=GRKIT_SHOT_ENTRIES;++i) {
        entry=ring+GRKIT_SHOT_ENTRY+((n-i)&(GRKIT_SHOT_ENTRIES-1))*GRKIT_SHOT_STRIDE;
        if(WORD(entry,GRKIT_SHOT_KEY)!=shot) continue;
        value=WORD(entry,field);
        PICTURE_BARRIER();
        if(WORD(entry,GRKIT_SHOT_KEY)==shot) { *out=value; return 1; }
    }
    return 0;
}
/* The record that the still build of a shot applied, or 0. */
static u32 shot_record(volatile u32 *h,u32 shot) {
    u32 record;
    return shot_word(h,shot,GRKIT_SHOT_RECORD,&record) ? record : 0;
}
/* The quarter turns clockwise that show the picture of a shot upright, as
   its EXIF Orientation does (1: 0, 6: 1, 3: 2, 8: 3), from the posture of the
   electronic level at its still build (LEVEL_POSTURE in runtime/targets).
   With the lens up or down, or without a posture, the turns of the last shot
   with a known posture stay; the ring word is noise after power-on, so a
   value out of range counts as 0. */
static u32 picture_turns(volatile u32 *h,u32 shot) {
    static const u8 turns[4]={0,2,1,3};
    u32 ring=PICTURE_SCRATCH(h)+GRKIT_SHOT_RING, last=WORD(ring,GRKIT_SHOT_LAST_TURNS), posture;
    if(last>3) last=0;
    if(!shot_word(h,shot,GRKIT_SHOT_POSTURE,&posture) || posture<1 || posture>4) return last;
    WORD(ring,GRKIT_SHOT_LAST_TURNS)=turns[posture-1];
    return turns[posture-1];
}
/* For picture_capture_tone: the record of a shot, or 0. */
u32 picture_capture_record(u32 shot) { return shot_record(framing_header(),shot); }
#ifndef PICTURE_IFD_SET_SHORT
#define PICTURE_IFD_SET_SHORT(entry,index,value) (((void (*)(u32,u32,u32))FW_IFD_SET_SHORT)(entry,index,value))
#endif
/* For picture_capture_parameters: the maker-note adjustments (tag 0x247) of a
   GRKit shot. The native fill reads the settings of the native Image Control
   type. The record's Fn adjustments replace the nine Standard elements:
   saturation, hue, key, contrast, highlight, shadow, sharpness, then shading
   and clarity. An element holds the shown value, the native value less 4
   (-4 to 4), as camera photos show. */
void picture_record_parameters(u32 entry,u32 shot) {
    static const u8 elements[9]={0,1,2,3,4,5,6,9,10};
    u32 record=picture_capture_record(shot), i;
    if(!record) return;
    for(i=0;i<sizeof(elements);++i)
        PICTURE_IFD_SET_SHORT(entry,elements[i],(record_parameter(record,i)-4)&0xffffU);
}
/* Whether the native Image Control type of a shot makes a monochrome picture,
   such as Monotone or a type of GR IV Monochrome. */
static u32 native_monochrome(u32 shot) {
    u32 type;
    if(shot<0x40000000U || shot>0xbfffff00U) return 0;
    type=BYTE(shot,SETTINGS_IC_TYPE);
#ifdef NATIVE_MONOCHROME_TYPES_HIGH
    if(type>=32) return type<64 && (NATIVE_MONOCHROME_TYPES_HIGH>>(type-32)&1);
#endif
    return type<32 && (NATIVE_MONOCHROME_TYPES>>type&1);
}
/* Once per plane set. The second full-size image (descriptor 3) shares the
   main planes. */
static void encode_shot(volatile u32 *h,u32 message) {
    /* Configuration +0x34 holds the shot settings of the encoded pictures. */
    u32 config=WORD(message,8), record=shot_record(h,WORD(config,0x34));
    u32 done[4], selective, values[MODULE_FILTER_MAX], filters=module_filters(h), any, i, j;
    PictureSet set;
    Plane p;
    set.count=0;
    set.record=0;
    set.mono=0;
    set.scratch=PICTURE_MAP(PICTURE_SCRATCH(h));
    set.turns=picture_turns(h,WORD(config,0x34));
    if(owned(h,record)) {
        set.record=record;
        set.mono=WORD(record,GRKIT_RECORD_MONOCHROME)&1;
    }
    else set.mono=native_monochrome(WORD(config,0x34));
    /* The color of the effect first: selective color of its record. */
    selective=selective_wanted(&set);
    any=selective;
    /* The value of each filter. A filter does not run at 0, or while a soft
       Off stops its module. */
    for(i=0;i<filters;++i) {
        values[i]=services_suspended(h,MODULE_FILTER(h,i)[0]) ? 0 : setting_value(h,MODULE_FILTER(h,i)[1]&0xffffU);
        any|=values[i];
    }
    if(!any) return;
    for(i=0;i<4;++i) {
        u32 seen=0;
        if(!plane_of(WORD(config,0x3c+i*4),WORD(config,0x4c+i*4),&p)) continue;
        for(j=0;j<set.count;++j) seen|=done[j]==p.y;
        if(seen) continue;
        done[set.count]=p.y;
        visible_area(h,&p);
        set.planes[set.count++]=p;
    }
    if(!set.count) return;
    for(i=0;i<set.count;++i) flush(&set.planes[i]);
    if(selective) selective_encode(&set);
    /* The module filters follow the color of the effect, in stage order. */
    for(i=0;i<filters;++i) if(values[i]) {
        u32 start=profile_begin(h);
        MODULE_APPLY(MODULE_FILTER(h,i)[0],&set,values[i]);
        profile_end_module(h,start,MODULE_FILTER(h,i)[0],GRKIT_PROFILE_FILTER);
    }
    for(i=0;i<set.count;++i) flush(&set.planes[i]);
}
/* The encode of one shot, the module filters included. */
void picture_encode(u32 process,u32 message) {
    volatile u32 *h=framing_header();
    u32 start=profile_begin(h);
    (void)process;
    encode_shot(h,message);
    profile_end(h,start,GRKIT_PROFILE_PICTURE_ENCODE);
}

#ifdef GRAIN_FILL_PITCH
/* The noise of the GR III grain stage. The granulation blends one random
   byte into each Y sample, so its grain is one sample wide, and it averages
   away when the picture shows smaller. For a GRKit shot this fill writes the
   random bytes itself: CORE.GRAIN_SIZE small keeps them, medium and large
   use blurred noise with the standard deviation of one random byte, so the
   grain is wider at the same strength (grain_tile). The fill seeds a
   xorshift128 generator from the
   generic timer, so each shot gets its own pattern; the native fill seeds
   from the clock seconds, and shots in the same second repeat it. */
#ifndef GRAIN_NATIVE_FILL
#define GRAIN_NATIVE_FILL(buffer,size) ((void (*)(u32,u32))FW_GRAIN_NOISE)(buffer,size)
#define GRAIN_ALLOCATE(size) ((u32 (*)(u32))FW_ALLOCATE)(size)
#define GRAIN_RELEASE(block) ((void (*)(u32))FW_RELEASE)(block)
#define GRAIN_MAP(address) ((u8 *)(address))
#endif
#ifndef GRAIN_COUNTER
#ifdef __arm__
/* CNTPCT, the 64-bit count of the generic timer. */
static u32 grain_counter(u32 *high) {
    u32 low,hi;
    __asm__ volatile("mrrc p15, 0, %0, %1, c14" : "=r"(low), "=r"(hi));
    *high=hi;
    return low;
}
#else
/* A host build has no camera timer. */
static u32 grain_counter(u32 *high) { *high=0; return 0; }
#endif
#define GRAIN_COUNTER(high) grain_counter(high)
#endif
#ifndef GRAIN_COUNTER_MHZ
#define GRAIN_COUNTER_MHZ SYSTEM_COUNTER_MHZ
#endif
#define GRAIN_ROW_MAX 8192U
/* The camera build has no FPU (CAMERA_TARGET in crates/grkit-build), so the
   camera runs the scalar code. The NEON code runs only in the host test on
   an ARM host (tests/runtime/picture_contract.c), which compares the noise
   with one reference, as it does for the scalar code on other hosts. */
#if defined(__ARM_NEON) && !defined(GRAIN_SCALAR)
#include <arm_neon.h>
#define GRAIN_NEON 1
#endif
/* Four xorshift128 generators side by side: generator i gives bytes 4i to
   4i + 3 of each 16 bytes, in little-endian order. */
typedef struct { u32 a[4],b[4],c[4],d[4]; } GrainRandom;
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
    u32 x=0;
#ifdef GRAIN_NEON
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
    u32 i,t,v;
    for(;x+16<=pitch;x+=16) for(i=0;i<4;++i) {
        t=r->a[i]^(r->a[i]<<11);
        r->a[i]=r->b[i]; r->b[i]=r->c[i]; r->c[i]=r->d[i];
        v=r->d[i]=r->d[i]^(r->d[i]>>19)^t^(t>>8);
        row[x+4*i]=(u8)v; row[x+4*i+1]=(u8)(v>>8); row[x+4*i+2]=(u8)(v>>16); row[x+4*i+3]=(u8)(v>>24);
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
#define GRAIN_SQRT2_PART 13573
#define GRAIN_TILE_A 256U
#define GRAIN_TILE_B 240U
#define GRAIN_NOISE_BYTES (GRAIN_TILE_A*GRAIN_TILE_A+GRAIN_TILE_B*GRAIN_TILE_B+6U*GRAIN_TILE_A)
_Static_assert(!(GRAIN_TILE_A%16U) && !(GRAIN_TILE_B%16U) && GRAIN_TILE_B<=GRAIN_TILE_A,
               "Each tile row is whole 16-byte blocks, and the work rows fit both tiles");
/* Copy `n` bytes, a multiple of 16. */
static void grain_copy(u8 *out,const u8 *in,u32 n) {
    u32 i;
#ifdef GRAIN_NEON
    for(i=0;i<n;i+=16) vst1q_u8(out+i,vld1q_u8(in+i));
#else
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
/* 16 samples of the plane from two half deviations. */
static void grain_add16(u8 *out,const u8 *a,const u8 *b) {
#ifdef GRAIN_NEON
    uint8x16_t va=vld1q_u8(a),vb=vld1q_u8(b);
    int16x8_t lo=vsubq_s16(vreinterpretq_s16_u16(vaddl_u8(vget_low_u8(va),vget_low_u8(vb))),vdupq_n_s16(256));
    int16x8_t hi=vsubq_s16(vreinterpretq_s16_u16(vaddl_u8(vget_high_u8(va),vget_high_u8(vb))),vdupq_n_s16(256));
    lo=vaddq_s16(vaddq_s16(lo,vqrdmulhq_s16(lo,vdupq_n_s16(GRAIN_SQRT2_PART))),vdupq_n_s16(128));
    hi=vaddq_s16(vaddq_s16(hi,vqrdmulhq_s16(hi,vdupq_n_s16(GRAIN_SQRT2_PART))),vdupq_n_s16(128));
    vst1q_u8(out,vcombine_u8(vqmovun_s16(lo),vqmovun_s16(hi)));
#else
    u32 i;
    int v;
    for(i=0;i<16;++i) {
        int s=(int)a[i]+(int)b[i]-256;
        v=128+s+((2*s*GRAIN_SQRT2_PART+32768)>>16);
        out[i]=(u8)(v<0 ? 0 : v>255 ? 255 : v);
    }
#endif
}
/* Fill the plane with the sum of the two tiles, each from the place that
   `place_a` and `place_b` give: the start column in 16-sample blocks in the
   low half, the start row in the high half. */
static void grain_layers(u8 *plane,u32 pitch,u32 height,const u8 *a,const u8 *b,u32 place_a,u32 place_b) {
    u32 y,x,row_a=place_a>>16,row_b=place_b>>16;
    for(y=0;y<height;++y) {
        const u8 *line_a=a+row_a*GRAIN_TILE_A,*line_b=b+row_b*GRAIN_TILE_B;
        u32 column_a=(place_a&0xffffU)*16U,column_b=(place_b&0xffffU)*16U;
        u8 *out=plane+y*pitch;
        for(x=0;x<pitch;x+=16) {
            grain_add16(out+x,line_a+column_a,line_b+column_b);
            column_a+=16; if(column_a==GRAIN_TILE_A) column_a=0;
            column_b+=16; if(column_b==GRAIN_TILE_B) column_b=0;
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
/* Replaces the noise fill of the granulation (grain_fill in picture.S). The
   profile counts the microseconds of the generic timer, which all cores
   share: the task can move to another core, whose cycle counter is another
   one (runtime/profile.h). */
void grain_noise(u32 buffer,u32 size,u32 pitch,u32 height) {
    volatile u32 *h=framing_header();
    u32 shot=h[GRKIT_HOST_GRAIN_SHOT/4],table=h[GRKIT_HOST_PROFILE/4],high,low,block,y;
    GrainRandom r;
    if(!shot || shot>3 || !pitch || pitch%16 || pitch>GRAIN_ROW_MAX || pitch*height!=size || height<1) {
        GRAIN_NATIVE_FILL(buffer,size);
        return;
    }
    low=GRAIN_COUNTER(&high);
    grain_seed(&r,low^high*0x27d4eb2dU);
    block=shot==1 ? 0 : GRAIN_ALLOCATE(GRAIN_NOISE_BYTES);
    if(!block) {
        /* Small, or no memory for the tiles: one random byte each. */
        for(y=0;y<height;++y) grain_random_row(&r,GRAIN_MAP(buffer)+y*pitch,pitch);
    } else {
        u8 *a=GRAIN_MAP(block),*b=a+GRAIN_TILE_A*GRAIN_TILE_A,*keep=b+GRAIN_TILE_B*GRAIN_TILE_B;
        u16 *sums=(u16 *)(keep+4*GRAIN_TILE_A);
        u32 taps=shot==2 ? 3 : 5,seed,place_a,place_b;
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
        grain_layers(GRAIN_MAP(buffer),pitch,height,a,b,place_a,place_b);
        GRAIN_RELEASE(block);
    }
    if(table) profile_add(table,GRKIT_PROFILE_PICTURE_GRAIN,core_divide(GRAIN_COUNTER(&high)-low,GRAIN_COUNTER_MHZ));
}
#endif
