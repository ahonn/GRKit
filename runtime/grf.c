/* Shared GRFX0001 validation for the installer and host tools. Bytes 84..127
   hold the optional extension: u32 features at 84, an i16[9] post matrix at 88,
   and selective color at 106 (i16 cos and sin, four u16 tangents, two u8 chroma
   limits). Bytes 120..127 are reserved. */
#include "grf.h"
#ifndef GRKIT_GRF_EMBEDDED
typedef unsigned char u8;
typedef unsigned int u32;
#endif
#ifdef GRKIT_GRF_EMBEDDED
#define grf_get32 get32
#define grf_equal equal
#else
static u32 grf_get32(const u8 *p)
{ return p[0] | p[1] << 8 | p[2] << 16 | (u32)p[3] << 24; }
static int grf_equal(volatile const u8 *a, const u8 *b, u32 n)
{ for (u32 i=0; i<n; ++i) if (a[i] != b[i]) return 0; return 1; }
#endif
static u32 crc(const u8 *p)
{
    u32 value=~0U;
    for (u32 i=0; i<8192; ++i) {
        value ^= (i>=12 && i<16) ? 0 : p[i];
        for (u32 bit=0; bit<8; ++bit) value=(value>>1)^((0U-(value&1))&0xedb88320U);
    }
    return ~value;
}
static int zeros(const u8 *p, u32 n) { for(u32 i=0;i<n;++i) if(p[i]) return 0; return 1; }
#define GRF_FEATURE_POST 1U
#define GRF_FEATURE_SELECTIVE 2U
static u32 grf_get16(const u8 *p) { return p[0] | p[1] << 8; }
/* The bounds of grkit_core::selective: a Q12 unit direction, inner < outer <= 1024
   on each side, and c0 < c1 <= 181. */
static int valid_selective(const u8 *p)
{
    int c=(short)grf_get16(p), s=(short)grf_get16(p+2);
    u32 square=(u32)(c*c)+(u32)(s*s);
    if (square<15000000U || square>18000000U) return 0;
    for (u32 side=0; side<2; ++side) {
        u32 inner=grf_get16(p+4+side*4), outer=grf_get16(p+6+side*4);
        if (inner>=outer || outer>1024) return 0;
    }
    return p[12]<p[13] && p[13]<=181;
}
/* Each feature bit owns its field, and an unused field stays zero. An effect
   without features has zeros in the whole extension. A monochrome effect has
   no features. */
static int valid_extension(const u8 *p)
{
    u32 features=grf_get32(p+84);
    if ((features & ~(GRF_FEATURE_POST|GRF_FEATURE_SELECTIVE)) || !zeros(p+120,8)) return 0;
    if (!(features&GRF_FEATURE_POST) && !zeros(p+88,18)) return 0;
    if (!(features&GRF_FEATURE_SELECTIVE) && !zeros(p+106,14)) return 0;
    if (features && grf_get32(p+80)==1) return 0;
    if ((features&GRF_FEATURE_SELECTIVE) && !valid_selective(p+106)) return 0;
    return 1;
}
static int valid_profile(const u8 *p)
{
    if (!grf_equal(p,(const u8 *)"GRFX0001",8) || grf_get32(p+8)!=8192 || grf_get32(p+12)!=crc(p)) return 0;
    u32 n=0;
    while (n<32 && p[16+n]) {
        u8 c=p[16+n];
        if (!((c>='a'&&c<='z')||(c>='0'&&c<='9')||(n&&c=='-'))) return 0;
        ++n;
    }
    if (!n || n>31 || !zeros(p+16+n,32-n)) return 0;
    n=0;
    while (n<32 && p[48+n]) { if(p[48+n]<32 || p[48+n]>126) return 0; ++n; }
    if (!n || n>16 || p[48]==' ' || p[47+n]==' ' || !zeros(p+48+n,32-n)) return 0;
    if (grf_get32(p+80)>1 || !valid_extension(p) ||
        !zeros(p+146,2) || !zeros(p+8084,108)) return 0;
    for (u32 i=148;i<1684;i+=2) if (((u32)p[i]|p[i+1]<<8)>16383) return 0;
    if(grf_get32(p+80)==1 && (!grf_equal(p+128,p+134,6) || !grf_equal(p+128,p+140,6) ||
       !grf_equal(p+148,p+660,512) || !grf_equal(p+148,p+1172,512))) return 0;
    for (u32 i=1687;i<8084;i+=4) if(p[i]!=0 && p[i]!=255) return 0;
    return 1;
}

#ifndef GRKIT_GRF_EMBEDDED
int grkit_grf_validate(const unsigned char *data, unsigned long size)
{
    return data && size == GRKIT_GRF_SIZE && valid_profile(data);
}
#endif
