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
#include "develop.h"
/* A RAW development encodes the DNG in playback, not a new shot. Its picture
   takes the time of that DNG from the playback metadata (runtime/targets). */
#ifndef PICTURE_DEVELOPING
#ifdef FW_DEVELOP_MODEL
#define PICTURE_DEVELOPING() ((BYTE(FW_DEVELOP_GUARD,0)&1U) && BYTE(FW_DEVELOP_MODEL,DEVELOP_STATE))
#define PICTURE_PHOTO_TIME(out) do { \
    u32 data_=((u32 (*)(void))FW_PLAYBACK_METADATA)(), i_; \
    for(i_=0;i_<8;++i_) (out)[i_]=BYTE(data_,METADATA_TIME+i_); \
} while(0)
/* The EXIF Orientation of that DNG. */
#define PICTURE_PHOTO_ORIENTATION() BYTE(((u32 (*)(void))FW_PLAYBACK_METADATA)(),METADATA_ORIENTATION)
#else
#define PICTURE_DEVELOPING() 0
#define PICTURE_PHOTO_TIME(out) PICTURE_DATE(out)
#define PICTURE_PHOTO_ORIENTATION() 1
#endif
#endif
static u32 owned(volatile u32 *h,u32 record) { return record_index(h,record)<GRKIT_TEXT_RECORDS; }
/* Features of an owned record, or 0 for a native one. */
static u32 record_features(volatile u32 *h,u32 record) {
    return owned(h,record) ? WORD(record,GRKIT_RECORD_FEATURES) : 0;
}
/* The record whose ImageTone code is `code`, or 0. The installer gives each
   record a distinct code with bit 15 set. */
static u32 record_of_code(volatile u32 *h,u32 code) {
    u32 n=h[GRKIT_HOST_COUNT/4], i, record;
    if(!(code&0x8000U) || n>GRKIT_TEXT_RECORDS) return 0;
    for(i=0;i<n;++i) {
        record=h[GRKIT_HOST_RECORDS/4]+i*GRKIT_RECORD_SIZE;
        if(HALF(record,GRKIT_RECORD_CODE)==code) return record;
    }
    return 0;
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
    if((features&GRKIT_FEATURE_NATIVE_LOOK) && (i=develop_parameter(h,record,0,PICTURE_DEVELOPING()))!=4) {
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
/* Visible-area id of the capture: 0 3:2, 1 4:3, 2 16:9, 3 1:1, then each
   Framing geometry (runtime/framing_ratios.h), by crop value. */
static const u8 picture_area_ids[GRKIT_CORE_ASPECT_VALUES]={FRAMING_RATIO_AREAS};
#define PICTURE_CROP_AREA(ratio) ((ratio)<GRKIT_CORE_ASPECT_VALUES ? picture_area_ids[ratio] : 0)
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
/* The core exports these services to modules (runtime/sdk/grkit.h). The local
   time is the time of the picture: the clock for a shot, the time of the DNG
   for a RAW development, so that Date Imprint prints the day of the photo. */
/* Whether a GrkitDateTime holds a date and a time that the camera can set. */
static u32 picture_time_valid(const u8 *t) {
    u32 year=t[0]|(u32)t[1]<<8;
    return year>=2000 && year<=2099 && t[2]>=1 && t[2]<=12 && t[3]>=1 && t[3]<=31 &&
           t[4]<24 && t[5]<60 && t[6]<60;
}
__attribute__((used)) void grkit_export_local_time(u8 *out) {
    if(PICTURE_DEVELOPING()) {
        PICTURE_PHOTO_TIME(out);
        if(picture_time_valid(out)) return;
    }
    PICTURE_DATE(out);
}
__attribute__((used)) u32 grkit_export_divide(u32 n,u32 d) { return core_divide(n,d); }
/* The white of the effect of a recorded picture: input 255 of the curves of
   its record, after its post matrix, as R, G, B levels, and 255 each without
   a record. The Fn adjustments are not in it. runtime/level.h gives the luma
   of the same white to the live-view highlight warning. */
__attribute__((used)) void grkit_export_picture_white(const PictureSet *set,u8 *white) {
    u32 record=set->record, post, k, c;
    int rgb[3], v;
    if(!record) { white[0]=white[1]=white[2]=255; return; }
    post=WORD(record,GRKIT_RECORD_FEATURES)&GRKIT_FEATURE_POST;
    for(c=0;c<3;++c) rgb[c]=(int)HALF(record,GRKIT_RECORD_CURVES+c*GRKIT_RECORD_CURVE_SIZE+255*2);
    for(k=0;k<3;++k) {
        v=rgb[k];
        if(post) {
            v=0;
            for(c=0;c<3;++c) v+=(s16)HALF(record,GRKIT_RECORD_POST+(k*3+c)*2)*rgb[c];
            v=(v+4096)>>13;
        }
        v=v<0 ? 0 : v>16383 ? 16383 : v;
        white[k]=(u8)(((u32)v*255U+8192U)>>14);
    }
}
/* picture_shot: the shooting data of the shot of a plane set, in the
   layout of GrkitShot (GRKIT_SHOT_INFO_* in abi.h). */
#define SHOT_INVALID ((int)0x80000000)
/* An IEEE single in 1/256 units, rounded, or SHOT_INVALID when it is not
   finite or not below 2^23. */
static int shot_q8(u32 bits) {
    u32 e=(bits>>23)&255, m=(bits&0x7fffffU)|0x800000U;
    int shift=(int)e-142, v;
    if(!(bits&0x7fffffffU)) return 0;
    if(e==255 || shift>7) return SHOT_INVALID;
    if(shift>=0) v=(int)(m<<shift);
    else if(shift<-25) v=0;
    else v=(int)((m+(1U<<(-shift-1)))>>(-shift));
    return bits>>31 ? -v : v;
}
/* base x 2^(x/256), rounded, at most 0xffffffff. */
static u32 shot_exp2(u32 base,int x) {
    /* 2^(k/32) in Q16, k = 0 to 32. */
    static const u32 table[33]={
        65536,66971,68438,69936,71468,73032,74632,76266,77936,79642,81386,
        83169,84990,86851,88752,90696,92682,94711,96785,98905,101070,103283,
        105545,107856,110218,112631,115098,117618,120194,122825,125515,128263,131072};
    int i=x>>8;
    u32 f=(u32)x&255, k=f>>3, t=f&7;
    u32 m=table[k]+(((table[k+1]-table[k])*t+4)>>3);
    unsigned long long r=(unsigned long long)base*m;
    int shift=16-i;
    if(shift<=0) return shift<-16 || (r<<(-shift))>>32 ? 0xffffffffU : (u32)(r<<(-shift));
    if(shift>=64) return 0;
    r=(r+(1ULL<<(shift-1)))>>shift;
    return r>>32 ? 0xffffffffU : (u32)r;
}
/* log2(v) in 1/256 units, for v > 0. */
static int shot_log2(u32 v) {
    unsigned long long y;
    int n=31, k, q=0;
    while(!(v>>n)) --n;
    /* y in [1, 2) as Q30. */
    y=n>=30 ? (unsigned long long)(v>>(n-30)) : (unsigned long long)v<<(30-n);
    for(k=0;k<8;++k) {
        y=(y*y)>>30;
        q<<=1;
        if(y>=(2ULL<<30)) { y>>=1; q|=1; }
    }
    return n*256+q;
}
static void info_half(u8 *out,u32 at,u32 v) { out[at]=(u8)v; out[at+1]=(u8)(v>>8); }
static void info_word(u8 *out,u32 at,u32 v) { info_half(out,at,v&0xffff); info_half(out,at+2,v>>16); }
__attribute__((used)) void grkit_export_picture_shot(const PictureSet *set,u8 *out) {
    static const u8 program[]={SHOT_PROGRAM_CODES}, metering[]={5,2,3};
    u8 info[GRKIT_SHOT_INFO_BYTES];
    u32 shot=set->shot, size=out[0]|out[1]<<8|out[2]<<16|(u32)out[3]<<24, known=0, i, v;
    for(i=0;i<GRKIT_SHOT_INFO_BYTES;++i) info[i]=0;
    if(shot>=0x40000000U && shot<=0xbff00000U) {
        int av=shot_q8(WORD(shot,SHOT_APEX)), tv=shot_q8(WORD(shot,SHOT_APEX+4)), sv=shot_q8(WORD(shot,SHOT_APEX+8));
        int xv=shot_q8(WORD(shot,SHOT_APEX+12)), focal=shot_q8(WORD(shot,SHOT_FOCAL)), crop=shot_q8(WORD(shot,SHOT_CROP));
        u32 time=0, mode=BYTE(shot,SHOT_LONG_MODE);
        if(mode-SHOT_LONG_FIRST<2U) {
            /* Time or bulb: the time decides the shutter value. */
            unsigned long long t=(unsigned long long)WORD(shot,SHOT_TIME)|(unsigned long long)WORD(shot,SHOT_TIME+4)<<32;
            time=t>>32 ? 0xffffffffU : (u32)t;
            if(time) tv=19*256+238-shot_log2(time); /* log2(1e6) = 19.93 */
        }
        else if(BYTE(shot,SHOT_TIME_SET)) time=WORD(shot,SHOT_TIME);
        /* Av about -1 to 12, Tv -12 to 16, Sv 2 to 20: else no exposure. */
        if(av>-256 && av<12*256 && tv>-12*256 && tv<16*256 && sv>2*256 && sv<20*256) {
            if(!time) time=shot_exp2(1000000,-tv);
            info_word(info,GRKIT_SHOT_INFO_AV,(u32)av);
            info_word(info,GRKIT_SHOT_INFO_TV,(u32)tv);
            info_word(info,GRKIT_SHOT_INFO_SV,(u32)sv);
            info_word(info,GRKIT_SHOT_INFO_SCENE_EV,(u32)(av+tv-sv+5*256));
            info_word(info,GRKIT_SHOT_INFO_EXPOSURE_US,time);
            info_half(info,GRKIT_SHOT_INFO_F_NUMBER,shot_exp2(10,av>>1));
            v=shot_exp2(100,sv-5*256);
            info_half(info,GRKIT_SHOT_INFO_ISO,v>65535 ? 65535 : v);
            known|=GRKIT_SHOT_INFO_KNOWN_EXPOSURE;
        }
        if(xv>-16*256 && xv<16*256) { info_word(info,GRKIT_SHOT_INFO_COMPENSATION,(u32)xv); known|=GRKIT_SHOT_INFO_KNOWN_COMPENSATION; }
        if(focal>0 && focal<1000*256) {
            /* In 1/10 mm, and as 35 mm film: times 1.5339 (Q16 100526) and the crop. */
            info_half(info,GRKIT_SHOT_INFO_FOCAL,(u32)(focal*10+128)>>8);
            if(crop>0 && crop<8*256) info_half(info,GRKIT_SHOT_INFO_FOCAL_35MM,(u32)((((unsigned long long)focal*100526U*(u32)crop>>16)+32768)>>16));
            known|=GRKIT_SHOT_INFO_KNOWN_FOCAL;
        }
        v=BYTE(shot,SHOT_PROGRAM);
        if(v<sizeof(program)) { info[GRKIT_SHOT_INFO_PROGRAM]=program[v]; known|=GRKIT_SHOT_INFO_KNOWN_PROGRAM; }
        v=BYTE(shot,SHOT_METERING);
        if(v<sizeof(metering)) { info[GRKIT_SHOT_INFO_METERING]=metering[v]; known|=GRKIT_SHOT_INFO_KNOWN_METERING; }
        /* EXIF Flash: off 0x10; on 8, with red-eye reduction 0x48; fired +1. */
        v=!BYTE(shot,SHOT_FLASH_ON) ? 0x10U : (BYTE(shot,SHOT_FLASH_MODE)|2)==3 ? 0x48U : 8U;
        if(v!=0x10U && BYTE(shot,SHOT_FLASH_FIRED)) v|=1;
        info_half(info,GRKIT_SHOT_INFO_FLASH,v);
        known|=GRKIT_SHOT_INFO_KNOWN_FLASH;
        v=HALF(shot,SHOT_TAKEN);
        if(v>=2000 && v<2200 && BYTE(shot,SHOT_TAKEN+2)-1U<12U && BYTE(shot,SHOT_TAKEN+3)-1U<31U) {
            info_half(info,GRKIT_SHOT_INFO_TAKEN,v);
            info[GRKIT_SHOT_INFO_TAKEN+2]=BYTE(shot,SHOT_TAKEN+2);
            info[GRKIT_SHOT_INFO_TAKEN+3]=BYTE(shot,SHOT_TAKEN+3);
            info[GRKIT_SHOT_INFO_TAKEN+4]=BYTE(shot,SHOT_TAKEN+5);
            info[GRKIT_SHOT_INFO_TAKEN+5]=BYTE(shot,SHOT_TAKEN+6);
            info[GRKIT_SHOT_INFO_TAKEN+6]=BYTE(shot,SHOT_TAKEN+7);
            known|=GRKIT_SHOT_INFO_KNOWN_TAKEN;
        }
    }
    /* The fields that the module's GrkitShot has: its size, as the module
       set it, and the core's, the smaller. */
    if(size>GRKIT_SHOT_INFO_BYTES) size=GRKIT_SHOT_INFO_BYTES;
    if(size<8) return;
    info_word(info,GRKIT_SHOT_INFO_SIZE,size);
    info_word(info,GRKIT_SHOT_INFO_KNOWN,known);
    for(i=0;i<size;++i) out[i]=info[i];
}
static void flush(const Plane *p) {
    PICTURE_CACHE_FLUSH(p->y,p->pitch*p->height);
    PICTURE_CACHE_FLUSH(p->c,p->pitch*(p->half ? p->height>>1 : p->height));
}

static u32 aspect_native(void) { return PICTURE_NATIVE_ASPECT(); }
#include "settings.h"
#include "services.h"
#include "grm_format.h"
#include "xmp.h"
#include "photo_xmp.h"
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

/* The visible-area id of a shot: the capture aspect, with the crop of
   CORE.ASPECT when a module uses it. */
static u32 shot_area(volatile u32 *h) {
    u32 ratio=framing_crop(h);
    return PICTURE_AREA_ID(PICTURE_NATIVE_ASPECT(),ratio);
}
/* The visible-area id of a RAW development, from the size of its main
   picture: a Framing size (GRKIT_HOST_FRAME_FILE 2 and up, runtime/framing.c),
   else the nearest native aspect. The settings for shots do not apply. */
static u32 developed_area(volatile u32 *h,const Plane *main) {
    static const u8 file_area[FRAMING_FILES]={FRAMING_FILE_AREAS};
    /* Width x 100 / height of 3:2, 4:3, 16:9, and 1:1, in area id order. */
    static const u8 ratios[4]={150,133,178,100};
    u32 file=((volatile u8 *)h)[GRKIT_HOST_FRAME_FILE], w=main->width, hh=main->height, ratio, best=0, i;
    if(file>=2 && file<FRAMING_FILES) return file_area[file];
    if(w<hh) { u32 t=w; w=hh; hh=t; }
    ratio=core_divide(w*100,hh);
    for(i=1;i<4;++i) {
        u32 d=ratio>ratios[i] ? ratio-ratios[i] : ratios[i]-ratio;
        u32 b=ratio>ratios[best] ? ratio-ratios[best] : ratios[best]-ratio;
        if(d<b) best=i;
    }
    return best;
}
/* Visible area of a preview picture: black bars depend on the area id. */
static void visible_area(u32 id,Plane *p) {
    /* Screennail and thumbnail insets for ids 0 (3:2), 1 (4:3), 2 (16:9), 3 (1:1), then each
       Framing geometry (runtime/framing.S). */
    static const u8 screennail[FRAMING_AREAS][2]={FRAMING_AREA_SCREENNAILS};
    static const u8 thumbnail[FRAMING_AREAS][2]={FRAMING_AREA_THUMBNAILS};
    const u8 *inset=0;
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
   ring from another task. The first barrier acquires the published fields;
   the second keeps the key check after those reads. */
#if defined(__arm__)
__attribute__((target("thumb"),minsize))
#endif
static u32 shot_word(volatile u32 *h,u32 shot,u32 field,u32 *out) {
    u32 ring=PICTURE_SCRATCH(h)+GRKIT_SHOT_RING, n=WORD(ring,0), i, entry, value;
    for(i=1;shot && i<=GRKIT_SHOT_ENTRIES;++i) {
        entry=ring+GRKIT_SHOT_ENTRY+((n-i)&(GRKIT_SHOT_ENTRIES-1))*GRKIT_SHOT_STRIDE;
        if(WORD(entry,GRKIT_SHOT_KEY)!=shot) continue;
        PICTURE_BARRIER();
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

/* The native sensor rectangle of aspect 3:2 for a crop factor, the bits of
   a float. The assembly bridge supplies the firmware's hard-float calling
   convention. */
#ifndef PICTURE_DNG_RECT
void picture_dng_rect(u32 factor,u16 *rect);
#define PICTURE_DNG_RECT(factor,rect) picture_dng_rect((factor),(rect))
#endif
#ifndef PICTURE_IFD_SET_LONG
#define PICTURE_IFD_SET_LONG(entry,index,value) (((void (*)(u32,u32,u32))FW_IFD_SET_LONG)(entry,index,value))
#endif
/* The crop factor of the whole sensor, 1.0f. */
#define PICTURE_FACTOR_ONE 0x3f800000U
/* Whether Uncropped RAW is On (runtime/manage.h: CORE.RAW): the copy in
   the header holds its value + 1. */
static u32 picture_raw_uncropped(volatile u32 *h) {
    return h[GRKIT_HOST_MANAGE_COPY/4+GRKIT_MANAGE_COPY_RAW]==2U;
}
/* {p, q} of each Framing ratio, by CORE.ASPECT value - 1 (runtime/sdk/grkit.h). */
static const u8 picture_ratios[GRKIT_CORE_ASPECT_VALUES-1][2]={
    {65,24},{16,9},{4,3},{47,20},{1,1},{5,4},{7,6},{3,4}};
/* The crop of a shot in the DNG: x, y, width, and height, as the
   DefaultCrop tags hold them (relative to ActiveArea, whose origin is at
   +8/+12 of the information `info`). A shot captured with a Framing crop
   keeps the 3:2 picture of its crop factor in `kept`, and its JPEG has the
   centered box of its ratio in `shot_crop`: 1. Another shot has the native
   values of the information (+0x18/+0x1c for the origin, +0x20/+0x24 for
   the size, as the native setter takes them) in `shot_crop`: 0. */
static u32 picture_shot_frame(volatile u32 *h,u32 shot,u32 info,u32 *kept,u32 *shot_crop) {
    u32 crop, p, q, w, hh;
    u16 rect[4];
    if(!shot_word(h,shot,GRKIT_SHOT_FRAME_CROP,&crop) || !crop || crop>=GRKIT_CORE_ASPECT_VALUES) {
        shot_crop[0]=WORD(info,0x18)-WORD(info,8);
        shot_crop[1]=WORD(info,0x1c)-WORD(info,12);
        shot_crop[2]=WORD(info,0x20);
        shot_crop[3]=WORD(info,0x24);
        return 0;
    }
    PICTURE_DNG_RECT(WORD(shot,SHOT_CROP),rect);
    kept[0]=rect[2]-WORD(info,8); kept[1]=rect[3]-WORD(info,12); kept[2]=rect[0]; kept[3]=rect[1];
    w=rect[0]; hh=rect[1]; p=picture_ratios[crop-1][0]; q=picture_ratios[crop-1][1];
    if(w*q>hh*p) w=core_divide(hh*p,q); else hh=core_divide(w*q,p);
    shot_crop[0]=kept[0]+((rect[0]-w)>>1);
    shot_crop[1]=kept[1]+((rect[1]-hh)>>1);
    shot_crop[2]=w;
    shot_crop[3]=hh;
    return 1;
}
/* The whole 3:2 picture of the sensor, as picture_shot_frame gives a crop. */
static void picture_whole(u32 info,u32 *whole) {
    u16 rect[4];
    PICTURE_DNG_RECT(PICTURE_FACTOR_ONE,rect);
    whole[0]=rect[2]-WORD(info,8); whole[1]=rect[3]-WORD(info,12); whole[2]=rect[0]; whole[3]=rect[1];
}
/* Called after the native DNG tag setter of DefaultCropOrigin (`size` 0) or
   DefaultCropSize (`size` 1). A Framing shot keeps the 3:2 picture of its
   crop factor. With Uncropped RAW, each DNG keeps the whole 3:2 picture of
   the sensor; its XMP packet has the crop of the shot as a Camera Raw crop
   (picture_dng_xmp_add). JPEG geometry, the source descriptor, and embedded
   previews keep their recorded composition. */
/* This metadata path uses Thumb to fit beside the pixel-processing hooks. */
#if defined(__arm__)
__attribute__((target("thumb"),minsize))
#endif
void picture_dng_crop(u32 setter,u32 entry,u32 size) {
    volatile u32 *h=framing_header();
    u32 info=WORD(setter,DNG_SETTER_INFO), kept[4], shot_crop[4], i, *values=kept;
    if(HALF(entry,6)!=4 || WORD(entry,8)!=2) return;
    if(picture_raw_uncropped(h)) picture_whole(info,values=shot_crop);
    else if(!picture_shot_frame(h,WORD(setter,DNG_SETTER_SHOT),info,kept,shot_crop)) return;
    for(i=0;i<2;++i) PICTURE_IFD_SET_LONG(entry,i,values[(size ? 2U : 0U)+i]);
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
#ifdef FW_DEVELOP_MODEL
#ifndef PICTURE_PLAYBACK_TONE
/* Key 0x25 of the playback metadata: a non-native Image Control tag as
   0x10000 | (tag - 0x100). */
#define PICTURE_PLAYBACK_TONE() WORD(((u32 (*)(void))FW_PLAYBACK_METADATA)(),METADATA_TONE)
#endif
/* The crop of the DNG (runtime/framing.c: framing_develop_crop). */
#ifndef PICTURE_DEVELOP_CROP
#if defined(DEVELOP_SETTINGS) && defined(CROP_FOCALS)
void framing_develop_crop(void);
#define PICTURE_DEVELOP_CROP() framing_develop_crop()
#else
#define PICTURE_DEVELOP_CROP() ((void)0)
#endif
#endif
/* The start of a RAW development: its effect is the effect that the DNG
   records (the ImageTone code of the record, picture_capture_tone), or
   native processing for a native photo or an effect that the camera no
   longer has. It develops the native aspect of the DNG until its aspect list
   selects a Framing ratio (runtime/menu.c), and the crop of the DNG. */
/* The longest path of a photo, as "C:\DCIM\100RICOH\R0000001.DNG". */
#define PICTURE_PATH_MAX 64U
#ifndef PICTURE_DEVELOP_FILE
#ifdef FW_CONTENTS
/* `size` bytes at `offset` of the file that playback opened, the DNG of a
   development, in `buf`: the count read, or 0. */
static u32 picture_develop_file(u32 offset,u8 *buf,u32 size) {
    u8 path[PICTURE_PATH_MAX];
    u32 handle=WORD(FW_CONTENTS,CONTENTS_HANDLE), len=0, position[2], i;
    int file;
    if(handle<0x40000000U || handle>0xbfffff00U) return 0;
    /* The path has no drive letter: the photos of a development are on the
       card. */
    for(i=1;i<sizeof(path) && (path[i]=BYTE(handle,CONTENTS_PATH+i))!=0;++i) {}
    if(i==sizeof(path)) return 0;
    path[0]='C';
    file=((int (*)(const u8 *,u32,u32))FW_FILE_OPEN)(path,0,0);
    if(file<0) return 0;
    /* The offset is 64 bits in r2 and r3, from the start of the file. */
    if(!offset || !((int (*)(u32,u32,u32,u32,u32,u32 *))FW_FILE_SEEK)((u32)file,0,offset,0,0,position))
        ((int (*)(u32,void *,u32,u32 *))FW_FILE_READ)((u32)file,buf,size,&len);
    ((int (*)(u32))FW_FILE_CLOSE)((u32)file);
    return len<=size ? len : 0;
}
#define PICTURE_DEVELOP_FILE(offset,buf,size) picture_develop_file((offset),(buf),(size))
#else
#define PICTURE_DEVELOP_FILE(offset,buf,size) ((void)(offset),(void)(buf),(void)(size),0U)
#endif
#endif
/* The most bytes of an XMP packet that a development searches for the
   state. */
#define PICTURE_XMP_SEARCH 0x10000U
_Static_assert(PHOTO_XMP_STATE_BYTES+1U<=DEVELOP_RECORD_SIZE,"The buffer holds the state property with its value");
/* The settings of the DNG of a development (runtime/photo_xmp.h: grkit:State
   of the XMP packet of IFD0). The camera writes the packet at the start of
   the file, within the buffer. Lightroom moves it to the end of the file when
   it saves metadata there: then this reads the packet in parts, which
   overlap by the length of the property name, and reads again from the
   property, so that its value is in the buffer. */
static void picture_develop_record(volatile u32 *h) {
    u8 *buf;
    u32 len, n=0, off, at, where;
    if(!h[GRKIT_HOST_SERVICES/4]) return;
    buf=(u8 *)(services_word(h,SERVICES_DEVELOP)+DEVELOP_COPY_RECORD/4);
    len=PICTURE_DEVELOP_FILE(0,buf,DEVELOP_RECORD_SIZE);
    if(!(off=photo_xmp_tiff(buf,len,&n))) return;
    if(off<=len && n<=len-off) {
        photo_xmp_apply(h,(const char *)buf+off,n);
        return;
    }
    if(n>PICTURE_XMP_SEARCH) n=PICTURE_XMP_SEARCH;
    for(at=0;at<n;at+=DEVELOP_RECORD_SIZE-sizeof(PHOTO_XMP_STATE)) {
        len=PICTURE_DEVELOP_FILE(off+at,buf,n-at<DEVELOP_RECORD_SIZE ? n-at : DEVELOP_RECORD_SIZE);
        if(!len) return;
        if(!(where=photo_xmp_where((const char *)buf,len,PHOTO_XMP_STATE))) continue;
        at+=where-1U;
        len=PICTURE_DEVELOP_FILE(off+at,buf,n-at<DEVELOP_RECORD_SIZE ? n-at : DEVELOP_RECORD_SIZE);
        photo_xmp_apply(h,(const char *)buf,len);
        return;
    }
}
/* The Framing ratio that the photo of the development records, or 0
   (runtime/framing.c: framing_develop_select). The development asks for its
   initial aspect and its initial Image Control (picture_develop_start) in
   an order that this code does not depend on: each reads the record. */
u32 picture_develop_ratio(void) {
    volatile u32 *h=framing_header(), *c;
    if(!h[GRKIT_HOST_SERVICES/4]) return 0;
    c=services_word(h,SERVICES_DEVELOP);
    c[DEVELOP_COPY_ASPECT/4]=0;
    picture_develop_record(h);
    return c[DEVELOP_COPY_ASPECT/4];
}
void picture_develop_start(void) {
    volatile u32 *h=framing_header();
    u32 value=PICTURE_PLAYBACK_TONE(), record;
    PICTURE_DEVELOP_CROP();
    h[GRKIT_HOST_DEVELOP_RATIO/4]=0;
    h[GRKIT_HOST_DEVELOP/4]=1;
    /* The model gives each parameter set its values after this, so the
       Standard set holds no effect's adjustments, and no effect keeps changed
       ones (runtime/develop.h). */
    h[GRKIT_HOST_DEVELOP_FILLED/4]=0;
    h[GRKIT_HOST_DEVELOP_CHANGED/4]=0;
    h[GRKIT_HOST_DEVELOP_TONED/4]=0;
    develop_copy_start(h,settings_table(h));
    picture_develop_record(h);
    if(value>>16==1 && (record=record_of_code(h,(value+0x100)&0xffffU))) h[GRKIT_HOST_DEVELOP/4]=record;
}
#endif
/* A header word of a core setting that the picture stages read: that of
   the copy while a RAW development runs (runtime/develop_settings.h). For
   runtime/picture.S. */
u32 picture_header_word(u32 offset) {
    volatile u32 *h=framing_header(), *copy=develop_copy_header(h,offset);
    return copy ? *copy : h[offset/4];
}
/* The state file ID of GRAIN.STYLE (grkit_core::runtime::state_key_id), the
   setting of modules/grain: 0 System, 1 Film-like. */
#define GRAIN_STYLE_ID 252484306U
/* Whether the Grain module draws the grain itself (Film-like, the plane
   filter of modules/grain/grain.c). Without the setting, as with an older
   Grain module, the grain is System. A RAW development reads the value of
   its copy (setting_value). */
static u32 grain_style_film(volatile u32 *h) {
    volatile u32 *t=settings_table(h);
    u32 i;
    for(i=0;t && i<t[0];++i)
        if(t[SETTINGS_ENTRY(i)]==GRAIN_STYLE_ID) return setting_value(h,i)==1U;
    return 0;
}
/* Whether the Grain module draws the grain of this shot: Film-like, and a
   level other than Off. While the module is Off, the native stage keeps the
   default grain of a look (picture_grain_level). */
static u32 grain_module_film(volatile u32 *h) {
    return (h[GRKIT_HOST_CAPABILITIES/4]&GRKIT_CORE_GRAIN_BIT) && picture_header_word(GRKIT_HOST_GRAIN) &&
        grain_style_film(h);
}
/* For grain_module_draws (runtime/picture.S): in the Film-like style the
   native grain stage runs as in the System style, with no amplitude, so
   that everything else that stage of the firmware does stays the same
   (such as the brightness that Glow's highlights depend on), and the plane
   filter of the Grain module adds the grain. */
u32 grain_film_like(void) { return grain_module_film(framing_header()); }
/* The quarter turns clockwise of an EXIF Orientation (1: 0, 6: 1, 3: 2, 8: 3). */
static u32 orientation_turns(u32 orientation) {
    return orientation==6 ? 1 : orientation==3 ? 2 : orientation==8 ? 3 : 0;
}
#include "canvas_shot.h"
/* The record of a shot, or 0: the Fn adjustments of the shot (tag 0x247). */
u32 picture_capture_record(u32 shot) { return shot_record(framing_header(),shot); }
/* For picture_capture_tone: the record whose code the picture records, or 0.
   A capture or a RAW development has the entry of its build in the shot
   ring. A write without one in playback (GRKIT_HOST_FRAME_FILE is not 0
   there, and 0 while shooting) is an edit of the photo shown, such as Resize,
   which does not run Image Control: it keeps the GRKit effect that the photo
   records. The native tag would record Standard. */
u32 picture_capture_tone_record(u32 shot) {
    volatile u32 *h=framing_header();
    u32 record;
    if(shot_word(h,shot,GRKIT_SHOT_RECORD,&record)) return record;
#ifdef FW_DEVELOP_MODEL
    if(((volatile u8 *)h)[GRKIT_HOST_FRAME_FILE]) {
        u32 value=PICTURE_PLAYBACK_TONE();
        if(value>>16==1) return record_of_code(h,(value+0x100)&0xffffU);
    }
#endif
    return 0;
}
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
    volatile u32 *h=framing_header();
    u32 record=picture_capture_record(shot), running=PICTURE_DEVELOPING(), i;
    if(!record) return;
    for(i=0;i<sizeof(elements);++i)
        PICTURE_IFD_SET_SHORT(entry,elements[i],(develop_parameter(h,record,i,running)-4)&0xffffU);
}
/* For grain_gate (runtime/picture.S): the stage flag of the grain stage of a
   recorded picture. The stage gate runs before the parameter build of the
   shot, so it reads the Shading and Clarity of the native Image Control type.
   A GRKit effect gets the rule of the native Standard type with the values of
   its record: the stage runs when Shading or Clarity (adjustments 7 and 8) is
   not neutral. The record is the one that the build takes (module_parameters
   in runtime/processing.h): the effect of a RAW development, or the
   committed effect. Native processing keeps `flag`, the native result. */
/* The grain that the look of this shot has by itself (GRKIT_RECORD_LOOK_GRAIN). */
static u32 look_grain(volatile u32 *h) {
    u32 record;
    if(!(h[GRKIT_HOST_CAPABILITIES/4]&GRKIT_CORE_TONE_BIT)) return 0;
    record=PICTURE_DEVELOPING() ? h[GRKIT_HOST_DEVELOP/4] : h[GRKIT_HOST_ACTIVE/4];
    return owned(h,record) ? BYTE(record,GRKIT_RECORD_LOOK_GRAIN) : 0;
}
/* The level of CORE.GRAIN, 0 without a module that uses it. */
static u32 grain_setting(volatile u32 *h) {
    return h[GRKIT_HOST_CAPABILITIES/4]&GRKIT_CORE_GRAIN_BIT ? picture_header_word(GRKIT_HOST_GRAIN) : 0;
}
/* 1 while the Grain module is Off and this shot takes the default grain of its
   look: Hi-Contrast, and Grainy on a camera without the Rough grain. */
static u32 look_default_grain(volatile u32 *h) {
    u32 grain=look_grain(h);
#ifdef GRKIT_ROUGH_GRAIN
    if(grain==1) return 0;
#endif
    return grain && !grain_setting(h);
}
/* For runtime/picture.S: the grain level of this shot, 0 Off to 4 Extra Strong.
   The Grain module wins. While it is Off, the default grain of a look is strong,
   the native default of Hi-Contrast on GR IV Monochrome. */
u32 picture_grain_level(void) {
    volatile u32 *h=framing_header();
    return look_default_grain(h) ? 3 : grain_setting(h);
}
/* For runtime/picture.S: 1 while this shot has the default grain of its look. */
u32 picture_look_grain_on(void) {
    return look_default_grain(framing_header());
}
/* For runtime/picture.S: the value of CORE.GRAIN_SIZE for this shot. The grain
   of a look has medium size. */
u32 picture_grain_size(void) {
    return look_default_grain(framing_header()) ? 2 : picture_header_word(GRKIT_HOST_GRAIN_SIZE);
}
#ifdef GRKIT_ROUGH_GRAIN
/* Whether this shot takes the Rough grain of the firmware: its look is Grainy. */
u32 picture_rough(void) {
    return look_grain(framing_header())==1;
}
#endif
u32 picture_gate_flag(u32 flag) {
    volatile u32 *h=framing_header();
    u32 record, running;
    if(!(h[GRKIT_HOST_CAPABILITIES/4]&GRKIT_CORE_TONE_BIT)) return flag;
#ifdef GRKIT_ROUGH_GRAIN
    if(picture_rough()) return 1;
#endif
    running=PICTURE_DEVELOPING();
    record=running ? h[GRKIT_HOST_DEVELOP/4] : h[GRKIT_HOST_ACTIVE/4];
    if(!owned(h,record)) return flag;
    return develop_parameter(h,record,7,running)!=4 || develop_parameter(h,record,8,running)!=4;
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
   main planes. `canvas` is the table entry of a shot with a border
   (runtime/canvas_shot.h), or 0. */
static void encode_shot(volatile u32 *h,u32 message,const u32 *canvas) {
    /* Configuration +0x34 holds the shot settings of the encoded pictures. */
    u32 config=WORD(message,8), record=shot_record(h,WORD(config,0x34));
    u32 done[4], selective, values[MODULE_FILTER_MAX], filters=module_filters(h), any, i, j;
    PictureSet set;
    Plane p;
    set.count=0;
    set.record=0;
    set.mono=0;
    set.scratch=PICTURE_MAP(PICTURE_SCRATCH(h));
    /* A RAW development turns as its DNG records, not as the camera is held. */
    set.turns=PICTURE_DEVELOPING() ? orientation_turns(PICTURE_PHOTO_ORIENTATION()) : picture_turns(h,WORD(config,0x34));
    /* The shooting data (picture_shot); in a RAW development, that of the DNG. */
    set.shot=WORD(config,0x34);
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
        set.planes[set.count++]=p;
    }
    if(!set.count) return;
    {
        u32 area, main=0;
        for(i=1;i<set.count;++i)
            if(set.planes[i].width*set.planes[i].height>set.planes[main].width*set.planes[main].height) main=i;
        area=PICTURE_DEVELOPING() ? developed_area(h,&set.planes[main]) : shot_area(h);
        for(i=0;i<set.count;++i) visible_area(area,&set.planes[i]);
    }
#if CANVAS_ENABLED
    if(canvas) canvas_inner(canvas,&set);
#else
    (void)canvas;
#endif
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
#if CANVAS_ENABLED
    u32 canvas[CANVAS_E_WORDS];
    u32 bordered=canvas_encode(h,WORD(message,8),canvas);
#endif
    (void)process;
#if CANVAS_ENABLED
    encode_shot(h,message,bordered ? canvas : 0);
#else
    encode_shot(h,message,0);
#endif
    profile_end(h,start,GRKIT_PROFILE_PICTURE_ENCODE);
}
#ifdef CANVAS_POOL
/* After the encode of one shot (runtime/picture.S): its canvas gives back
   its pool block. */
void picture_encode_done(u32 process,u32 message) {
    (void)process;
    canvas_encoded(framing_header(),WORD(message,8));
}
#endif

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
#include "grain_noise.h"
/* Replaces the noise fill of the granulation (grain_fill in picture.S). The
   profile counts the microseconds of the generic timer, which all cores
   share: the task can move to another core, whose cycle counter is another
   one (runtime/profile.h). */
void grain_noise(u32 buffer,u32 size,u32 pitch,u32 height) {
    volatile u32 *h=framing_header();
    u32 shot=h[GRKIT_HOST_GRAIN_SHOT/4],table=h[GRKIT_HOST_PROFILE/4],high,low,block;
    if(!shot || shot>3 || !pitch || pitch%16 || pitch>GRAIN_ROW_MAX || pitch*height!=size || height<1) {
        GRAIN_NATIVE_FILL(buffer,size);
        return;
    }
    /* Film-like: the Grain module draws the grain, and the native stage
       blends neutral noise (with no amplitude, grain_amplitude). */
    if(grain_module_film(h)) {
        u8 *plane=GRAIN_MAP(buffer);
        u32 i;
        for(i=0;i<size;++i) plane[i]=128;
        return;
    }
    low=GRAIN_COUNTER(&high);
    /* Small, or no memory for the tiles: one random byte each. */
    block=shot==1 ? 0 : GRAIN_ALLOCATE(GRAIN_NOISE_BYTES);
    grain_plane(GRAIN_MAP(buffer),pitch,height,low^high*0x27d4eb2dU,shot==2 ? 3 : 5,block ? GRAIN_MAP(block) : 0);
    if(block) GRAIN_RELEASE(block);
    if(table) profile_add(table,GRKIT_PROFILE_PICTURE_GRAIN,core_divide(GRAIN_COUNTER(&high)-low,GRAIN_COUNTER_MHZ));
}
#endif

/* The JPEG header writer of the firmware: +0x18 the header buffer, +0x1c its
   size, and +0x48 the offset of the APP7 preview segment. The header starts
   with SOI, the APP1 marker and length, and "Exif\0\0", so the TIFF block
   starts at JPEG_TIFF. GR III and GR IV share the layout, also that of the
   image information. */
#define JPEG_WRITER_BUFFER 0x18
#define JPEG_WRITER_SIZE 0x1c
#define JPEG_WRITER_APP7 0x48
#define JPEG_TIFF 12U
/* The image information of the header setup holds the shot settings
   pointer of a capture at +0x98, the key of its shot ring entry. Another
   write, such as an edit in playback, has no entry there. */
#define JPEG_INFO_SHOT 0x98
#ifndef PICTURE_JPEG_MAP
#define PICTURE_JPEG_MAP(address,size) ((volatile u8 *)(address))
#endif
#define XMP_NAME_UNITS ((GRKIT_RECORD_MONOCHROME-GRKIT_RECORD_NAME)/2)
/* The name of the look of record `record` in `name`. */
static void picture_record_name(u32 record,u16 *name) {
    u32 i;
    for(i=0;i<XMP_NAME_UNITS;++i) name[i]=HALF(record,GRKIT_RECORD_NAME+i*2);
}
/* The XMP entry (tag 700) of IFD0 of a DNG (runtime/picture.S:
   picture_dng_xmp), with the packet of the photo (runtime/photo_xmp.h):
   `factory` makes entries, and `ifd` takes them in tag order. The look is
   the GRKit effect of the shot; a native look has no name here, as the
   entry has no ImageTone code to read. The writer does not have the shot
   yet: with Uncropped RAW, the packet gets the Camera Raw crop of the whole
   picture, which picture_dng_xmp_set makes the crop of the shot. */
#define PICTURE_XMP_TAG 700U
/* Whether an address is in the RAM of the camera. */
#define PICTURE_RAM(address) ((address)>=0x40000000U && (address)<0xc0000000U)
/* The DNG specification and exiftool take tag 700 as BYTE. */
#define PICTURE_TIFF_BYTE 1U
void picture_dng_xmp_add(u32 factory,u32 ifd) {
#ifdef FW_IFD_APPEND
    volatile u32 *h=framing_header(), *stash;
    u8 *packet;
    u16 name[XMP_NAME_UNITS];
    u32 n, entry, record, crop_at=0, full;
    if(!h[GRKIT_HOST_SERVICES/4] || !h[GRKIT_HOST_CAPABILITIES/4]) return;
    stash=services_word(h,SERVICES_DNG_CROP);
    stash[DNG_CROP_ENTRY/4]=0;
    if((record=h[GRKIT_HOST_ACTIVE/4])) picture_record_name(record,name);
    full=picture_raw_uncropped(h);
    packet=(u8 *)services_word(h,SERVICES_DEVELOP+DEVELOP_COPY_RECORD);
    n=photo_xmp_build(h,packet,DEVELOP_RECORD_SIZE-(sizeof(xmp_end)-1),record ? name : 0,XMP_NAME_UNITS,1,
                      full ? &crop_at : 0);
    if(!n) return;
    n=xmp_put(packet,n,xmp_end,sizeof(xmp_end)-1);
    entry=((u32 (*)(u32,u32,u32,u32,u32))WORD(WORD(factory,0),0xc))(factory,PICTURE_XMP_TAG,PICTURE_TIFF_BYTE,n,0);
    if(!entry) return;
    ((void (*)(u32,u32))FW_IFD_APPEND)(ifd,entry);
    ((void (*)(u32,u32,u32))FW_IFD_ENTRY_DATA)(entry,(u32)packet,n);
    if(full) {
        stash[DNG_CROP_AT/4]=crop_at;
        stash[DNG_CROP_ENTRY/4]=entry;
    }
#else
    (void)factory; (void)ifd;
#endif
}
/* The data of an entry. A host test maps it. */
#ifndef PICTURE_DNG_DATA
#define PICTURE_DNG_DATA(address,size) ((u8 *)(address))
#endif
/* For picture_dng_set (runtime/picture.S): the tag setter of the DNG writer
   gets `entry`. For the XMP entry of picture_dng_xmp_add with a Camera Raw
   crop, the crop of the whole picture becomes the crop of the shot, in
   place: the setter has the information and the shot of the DNG by now. */
void picture_dng_xmp_set(u32 setter,u32 entry) {
    volatile u32 *h, *stash;
    u32 info, shot, data, count, at, crop[8], kept[4];
    if(HALF(entry,4)!=PICTURE_XMP_TAG) return;
    h=framing_header();
    if(!h[GRKIT_HOST_SERVICES/4]) return;
    stash=services_word(h,SERVICES_DNG_CROP);
    if(stash[DNG_CROP_ENTRY/4]!=entry) return;
    stash[DNG_CROP_ENTRY/4]=0;
    info=WORD(setter,DNG_SETTER_INFO); shot=WORD(setter,DNG_SETTER_SHOT);
    data=WORD(entry,0x10); count=WORD(entry,8); at=stash[DNG_CROP_AT/4];
    if(!PICTURE_RAM(info) || !PICTURE_RAM(shot) || !data || at>=count) return;
    picture_shot_frame(h,shot,info,kept,crop);
    picture_whole(info,crop+4);
    photo_xmp_crop_values(PICTURE_DNG_DATA(data,count),at,count,crop);
}
/* For picture_jpeg_header and picture_jpeg_rewrite (runtime/picture.S),
   after the firmware set up the APP1 Exif segment: the XMP segment of the
   photo goes at +0x48, and +0x48 moves past it. The APP7 setup, the
   maker-note offset of APP7, and the place of the main image all follow
   +0x48. A capture and a RAW development record their settings. An edit in
   playback, such as Resize, records only the look: the current settings are
   not those of the photo. Playback can open before the JPEG of the last
   shot is written; that JPEG is still a capture, as its shot settings
   (`info`, the image information, at JPEG_INFO_SHOT) are in the shot ring.
   No segment is written while no module runs (the GRKit switch is Off, or
   safe mode), or into a buffer without room for the largest APP7. */
void picture_jpeg_xmp(u32 writer,u32 info) {
    volatile u32 *h=framing_header();
    u32 buffer=WORD(writer,JPEG_WRITER_BUFFER), size=WORD(writer,JPEG_WRITER_SIZE), at=WORD(writer,JPEG_WRITER_APP7);
    u32 code, record, segment, units=XMP_NAME_UNITS, room, length, settings, entry;
    u16 name[XMP_NATIVE_UNITS>XMP_NAME_UNITS ? XMP_NATIVE_UNITS : XMP_NAME_UNITS];
    const u16 *look=name;
    volatile u8 *out;
    if(!h || !h[GRKIT_HOST_CAPABILITIES/4] || !buffer || at<=JPEG_TIFF || at>size || (at&(XMP_ALIGN-1))) return;
    if(!xmp_tone_code(PICTURE_JPEG_MAP(buffer+JPEG_TIFF,at-JPEG_TIFF),at-JPEG_TIFF,&code)) look=0;
    else if((record=record_of_code(h,code))) picture_record_name(record,name);
    else if(xmp_native_name(code,name,XMP_NATIVE_UNITS)) units=XMP_NATIVE_UNITS;
    else look=0;
    settings=!((volatile u8 *)h)[GRKIT_HOST_FRAME_FILE] || PICTURE_DEVELOPING() ||
             (PICTURE_RAM(info) && shot_word(h,WORD(info,JPEG_INFO_SHOT),GRKIT_SHOT_RECORD,&entry));
    /* The packet goes in place: room for it, its end, and the largest APP7. */
    room=size-at;
    if(room<=XMP_APP7_RESERVE+XMP_SEGMENT_HEAD+sizeof(xmp_end)-1) return;
    room-=XMP_APP7_RESERVE+XMP_SEGMENT_HEAD+sizeof(xmp_end)-1;
    if(room>0xffffU) room=0xffffU;
    out=PICTURE_JPEG_MAP(buffer+at,size-at);
    length=photo_xmp_build(h,(u8 *)out+XMP_SEGMENT_HEAD,room,look,units,settings,0);
    segment=xmp_segment_size(length);
    if(!segment || segment>size-at || size-at-segment<XMP_APP7_RESERVE) return;
    xmp_finish_segment(out,segment,length);
    WORD(writer,JPEG_WRITER_APP7)=at+segment;
}
