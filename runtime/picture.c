#include "abi.h"
/* Picture processing for effect extensions: the post-curve matrix, then soft glow
   on recorded pictures. */
typedef unsigned int u32;
typedef unsigned short u16;
typedef unsigned char u8;
typedef short s16;
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
#define PICTURE_YCBCR_CALC(object,context) (((void (*)(u32,u32))0x536e4720)(object,context))
#endif
extern volatile u32 *framing_header(void);

static u32 owned(volatile u32 *h,u32 record) {
    u32 i,base=h[GRKIT_HOST_RECORDS/4];
    for(i=0;i<h[GRKIT_HOST_COUNT/4];++i) if(record==base+i*GRKIT_RECORD_SIZE) return 1;
    return 0;
}
/* Features of an owned record, or 0 for a native one. */
static u32 record_features(volatile u32 *h,u32 record) {
    return owned(h,record) ? WORD(record,GRKIT_RECORD_FEATURES) : 0;
}
/* Replaces the YCbCrMatrix Make call to its calc (0x536e4774). The matrix acts
   on gamma-encoded RGB, so CSP x P applies P after the tone curves, and the hue
   rotation still follows it. The calc stores each coefficient as the low 9 bits
   of a word, and the register writer packs those bits without saturation. */
void picture_post(u32 object,u32 context) {
    volatile u32 *h=framing_header();
    u32 record=WORD(context,8),i,j,k;
    int csp[9],sum;
    PICTURE_YCBCR_CALC(object,context);
    if(!(record_features(h,record)&GRKIT_FEATURE_POST)) return;
    for(i=0;i<9;++i) csp[i]=(int)(WORD(object,i*4)<<23)>>23;
    for(i=0;i<3;++i) for(j=0;j<3;++j) {
        sum=0;
        for(k=0;k<3;++k) sum+=csp[i*3+k]*(s16)HALF(record,GRKIT_RECORD_POST+(k*3+j)*2);
        sum=(sum+4096)>>13;
        WORD(object,(i*3+j)*4)=(u32)(sum<-256 ? -256 : sum>255 ? 255 : sum);
    }
}
/* ---- Recorded pictures: soft glow -------------------------------------------
   JpegEncodeMacroProcess::Execute (0x535047d4) calls picture_encode before it
   submits the encode. Its image descriptors hold Y at +4, CbCr at +8, then u16
   width, height, and the shared pitch at +0x10. The CbCr plane interleaves
   Cb and Cr for each pair of pixels, in full rows (4:2:2) or half rows (4:2:0). */
#ifndef PICTURE_CACHE_FLUSH
/* Clean and invalidate the data cache for a range. */
#define PICTURE_CACHE_FLUSH(address,length) (((void (*)(u32,u32))0x5383c8a8)(address,length))
#endif
#ifndef PICTURE_SCRATCH
/* Glow map after the header and the records, inside the loader's registry array. */
#define PICTURE_SCRATCH(h) ((h)[GRKIT_HOST_RECORDS/4]+(h)[GRKIT_HOST_COUNT/4]*GRKIT_RECORD_SIZE)
#endif
#ifndef PICTURE_MAP
#define PICTURE_MAP(address) ((u8 *)(address))
#endif
#define GLOW_WIDTH 376U

typedef struct { u32 y,c,width,height,pitch,half; } Plane;

/* Shift-subtract division: the freestanding link has no division helper. */
static u32 divide(u32 n,u32 d) {
    u32 q=0,bit=1;
    while(d<n && !(d&0x80000000U)) { d<<=1; bit<<=1; }
    while(bit) { if(n>=d) { n-=d; q|=bit; } d>>=1; bit>>=1; }
    return q;
}
static void flush(const Plane *p) {
    PICTURE_CACHE_FLUSH(p->y,p->pitch*p->height);
    PICTURE_CACHE_FLUSH(p->c,p->pitch*(p->half ? p->height>>1 : p->height));
}
static u32 chroma_row(const Plane *p,u32 y) { return p->c+(p->half ? y>>1 : y)*p->pitch; }

/* Spread the highlight map in place: three box passes per axis, radius r. */
static void blur_line(u8 *line,u32 step,u32 n,u32 r) {
    u16 copy[GLOW_WIDTH];
    u32 i,pass,sum,window=2*r+1,reciprocal=divide(65536+window/2,window);
    for(pass=0;pass<3;++pass) {
        for(i=0;i<n;++i) copy[i]=line[i*step];
        sum=copy[0]*(r+1);
        for(i=1;i<=r;++i) sum+=copy[i<n ? i : n-1];
        for(i=0;i<n;++i) {
            line[i*step]=(u8)((sum*reciprocal)>>16);
            sum+=copy[i+r+1<n ? i+r+1 : n-1];
            sum-=copy[i>=r ? i-r : 0];
        }
    }
}
/* Soft glow: highlights above Y 200 spread into a diffuse halo that lifts Y
   toward white and moves chroma toward neutral, like a mist filter. */
static void glow(volatile u32 *h,const Plane *p,u32 strength) {
    static const u8 gains[4]={0,90,154,230};
    u8 *map=PICTURE_MAP(PICTURE_SCRATCH(h));
    u32 f=1,w,ht,x,y,sx,sy,dx,dy,r,step,gain=gains[strength];
    while(p->width>f*GLOW_WIDTH) ++f;
    w=divide(p->width+f-1,f); ht=divide(p->height+f-1,f);
    if(ht>GLOW_WIDTH) return;
    for(sy=0;sy<ht;++sy) for(sx=0;sx<w;++sx) {
        u32 total=0,i,j,px,py;
        for(j=0;j<2;++j) for(i=0;i<2;++i) {
            px=sx*f+((f*(1+2*i))>>2); py=sy*f+((f*(1+2*j))>>2);
            if(px>=p->width) px=p->width-1;
            if(py>=p->height) py=p->height-1;
            total+=BYTE(p->y,py*p->pitch+px);
        }
        total>>=2;
        total=total>200 ? (total-200)*297>>6 : 0;
        map[sy*w+sx]=(u8)(total>255 ? 255 : total);
    }
    r=(w*1966)>>16; if(!r) r=1;
    for(sy=0;sy<ht;++sy) blur_line(map+sy*w,1,w,r);
    for(sx=0;sx<w;++sx) blur_line(map+sx,w,ht,r);
    /* Bilinear between map cells: cell = pixel / f, fraction = (pixel % f) / f,
       counted without division. Cells without glow around them are skipped. */
    step=divide(65536,f);
    for(y=0,sy=0,dy=0;y<p->height;++y) {
        u32 ty=dy*step, cy1=sy+1<ht ? sy+1 : sy;
        u32 crow=chroma_row(p,y), even=!p->half || !(y&1);
        for(x=0,sx=0,dx=0;x<p->width;) {
            u32 tx=dx*step, cx1=sx+1<w ? sx+1 : sx;
            u32 a=map[sy*w+sx],b=map[sy*w+cx1],c=map[cy1*w+sx],d=map[cy1*w+cx1],g,yv,amount;
            if(!(a|b|c|d)) { x+=f-dx; dx=0; ++sx; continue; }
            g=((a*(65536-tx)+b*tx)>>16)*(65536-ty)+((c*(65536-tx)+d*tx)>>16)*ty;
            amount=((g>>16)*gain)>>8;
            yv=BYTE(p->y,y*p->pitch+x);
            BYTE(p->y,y*p->pitch+x)=(u8)(yv+(((255-yv)*amount+128)>>8));
            if(even && !(x&1)) {
                u32 i,k=256-(amount>>1);
                for(i=0;i<2;++i) {
                    int v=(int)BYTE(crow,x+i)-128;
                    BYTE(crow,x+i)=(u8)(128+((v*(int)k)>>8));
                }
            }
            ++x; if(++dx==f) { dx=0; ++sx; }
        }
        if(++dy==f) { dy=0; ++sy; }
    }
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
/* Glow once per plane set. The second full-size image (descriptor 3) shares
   the main planes. */
void picture_encode(u32 process,u32 message) {
    volatile u32 *h=framing_header();
    u32 config=WORD(message,8), record=h[GRKIT_HOST_STILL/4];
    u32 strength=0, done[4], count=0, i, j;
    Plane p;
    (void)process;
    if(record_features(h,record)&GRKIT_FEATURE_GLOW) strength=BYTE(record,GRKIT_RECORD_GLOW);
    if(!strength) return;
    for(i=0;i<4;++i) {
        u32 seen=0;
        if(!plane_of(WORD(config,0x3c+i*4),WORD(config,0x4c+i*4),&p)) continue;
        for(j=0;j<count;++j) seen|=done[j]==p.y;
        if(seen) continue;
        done[count++]=p.y;
        flush(&p);
        glow(h,&p,strength);
        flush(&p);
    }
}
