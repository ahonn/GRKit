#ifndef GRKIT_PLAYBACK_INFO_H
#define GRKIT_PLAYBACK_INFO_H
/* GRKit Info in Playback (runtime/manage.h: CORE.PBINFO). The detailed
   information of the playback of one photo has two pages; the first one
   shows the icon of each GRKit setting of the photo in a row, in the free
   part of the row of the drive mode, which each Image Control leaves free,
   against its right end, as the camera adds its icons from the left; the
   camera already shows the icon of the film look of the photo in the place
   of its Image Control (menu_playback_private). The settings are those
   of GRKIT_PHOTO_XMP_SETTINGS that grkit:State of the XMP packet of the photo
   holds with a value other than 0, and whose property the packet has: Grain,
   Bloom, Halation, Border, Framing, Crop, and Date Imprint. Framing and Crop
   that only showed a frame have no property, as they did not crop the photo.
   Each is an item of runtime/recipe_details.h with the icon of the setting
   (playback_icon_of, runtime/viewfinder.h: playback_paint). A photo without the state shows
   nothing.

   A poll in the App task reads the photo, every PLAYBACK_POLL_MS while
   playback shows and the setting is On. It reads the file only when the
   first page shows a photo that the items do not belong to: the start of
   the file, then for a JPEG the segment after its Exif segment, where the
   packet is, or for a DNG its packet; DEVELOP_RECORD_SIZE bytes each, into
   the buffer of the record of a RAW development (SERVICES_DEVELOP), which
   a development reads only as it starts, from a screen of its own. Then it
   redraws the canvas of playback. The draw reads no file. With the setting
   Off, the poll stops, and nothing reads or draws.

   The includer defines the helpers of runtime/menu.c: the services, the
   settings, the module icons (detail_module_icon), the recipe details, and
   ui_timer_start. */
#include "xmp.h"
#include "photo_xmp_read.h"
#define PLAYBACK_POLL_MS 100U
#ifndef PLAYBACK_TIMER_START
#define PLAYBACK_TIMER_START(ms) ui_timer_start(ms)
#endif
/* The controller of the playback of one photo, which the controller factory
   keeps. A host test maps it. */
#ifndef PLAYBACK_CONTROLLER_OF
#define PLAYBACK_CONTROLLER_OF() WORD(((u32 (*)(void))FW_CONTROLLER_FACTORY)(),PLAYBACK_FACTORY_SLOT)
#endif
#ifndef PLAYBACK_INVALIDATE
#define PLAYBACK_INVALIDATE(view) CALL1(FW_VIEW_INVALIDATE)(view)
#endif
/* `size` bytes at `offset` of the file at `path` into `buf`: the count read,
   or 0. A host test gives its own file. */
#ifndef PLAYBACK_FILE
static u32 playback_file(const u8 *path,u32 offset,u8 *buf,u32 size) {
    u32 len=0, position[2];
    int file=((int (*)(const u8 *,u32,u32))FW_FILE_OPEN)(path,0,0);
    if(file<0) return 0;
    /* The offset is 64 bits in r2 and r3, from the start of the file. */
    if(!offset || !((int (*)(u32,u32,u32,u32,u32,u32 *))FW_FILE_SEEK)((u32)file,0,offset,0,0,position))
        ((int (*)(u32,void *,u32,u32 *))FW_FILE_READ)((u32)file,buf,size,&len);
    ((int (*)(u32))FW_FILE_CLOSE)((u32)file);
    return len<=size ? len : 0;
}
#define PLAYBACK_FILE(path,offset,buf,size) playback_file((path),(offset),(buf),(size))
#endif
/* The path of the photo that playback opened, with the drive of the card,
   into `path` of PLAYBACK_PATH_BYTES: 1, or 0 without one. */
#ifndef PLAYBACK_PATH_SHOWN
static u32 playback_path_shown(u8 *path) {
    u32 handle=WORD(FW_CONTENTS,CONTENTS_HANDLE), i;
    if(handle<0x40000000U || handle>0xbfffff00U) return 0;
    for(i=1;i<PLAYBACK_PATH_BYTES && (path[i]=BYTE(handle,CONTENTS_PATH+i))!=0;++i) {}
    if(i<2U || i==PLAYBACK_PATH_BYTES) return 0;
    path[0]='C';
    return 1;
}
#define PLAYBACK_PATH_SHOWN(path) playback_path_shown(path)
#endif
static volatile u32 *playback_word(volatile u32 *h,u32 offset) { return services_word(h,SERVICES_PLAYBACK+offset); }

/* The packet of the photo at `path` in `buf` of DEVELOP_RECORD_SIZE bytes:
   its first byte, with its length in *n, or 0. A JPEG walks its segments
   from the start, which the first read holds; the second read starts at the
   first segment that the first one does not hold whole, which is the packet
   after a long Exif segment. */
static const char *playback_packet(const u8 *path,u8 *buf,u32 *n) {
    u32 len=PLAYBACK_FILE(path,0,buf,DEVELOP_RECORD_SIZE), at=2, base=0, size, reads=1, off, xmp;
    /* A DNG starts with its TIFF block. */
    if(len>=8 && buf[0]=='I' && buf[1]=='I') {
        if(!(off=photo_xmp_tiff(buf,len,n))) return 0;
        if(off<=len && *n<=len-off) return (const char *)buf+off;
        len=PLAYBACK_FILE(path,off,buf,*n<DEVELOP_RECORD_SIZE ? *n : DEVELOP_RECORD_SIZE);
        *n=len;
        return len ? (const char *)buf : 0;
    }
    if(len<4 || buf[0]!=0xff || buf[1]!=0xd8) return 0;
    for(;;) {
        size=at+4U<=len ? (u32)buf[at+2]<<8|buf[at+3] : 0;
        xmp=at+4U<=len && buf[at+1]==0xe1 &&
            photo_xmp_at((const char *)buf,len,at+4U,xmp_namespace,sizeof(xmp_namespace));
        if(at+4U>len || (xmp && at && at+2U+size>len)) {
            if(reads++==2) return 0;
            base+=at;
            len=PLAYBACK_FILE(path,base,buf,DEVELOP_RECORD_SIZE);
            at=0;
            continue;
        }
        /* The image data starts after the segments of the header. */
        if(buf[at]!=0xff || buf[at+1]<0xe0 || buf[at+1]>0xef || size<2U) return 0;
        if(xmp) {
            *n=(at+2U+size<len ? at+2U+size : len)-(at+XMP_SEGMENT_HEAD);
            return (const char *)buf+at+XMP_SEGMENT_HEAD;
        }
        at+=2U+size;
    }
}
/* The value of state file ID `id` in grkit:State of the packet, or 0. */
static u32 playback_state(const char *xmp,u32 len,u32 id) {
    u32 n, at, key, value;
    const char *s=xmp ? photo_xmp_find(xmp,len,PHOTO_XMP_STATE,&n) : 0;
    for(at=0;s && photo_xmp_next(s,n,&at,&key,&value);)
        if(key==id) return value;
    return 0;
}
/* The handle of a setting of the packet: its core handle, or the module
   setting with its state file ID, or PHOTO_XMP_NO_SETTING. */
#define PLAYBACK_NO_SETTING 0xffffffffU
static u32 playback_handle(volatile u32 *h,const PhotoXmpSetting *s) {
    volatile u32 *t=settings_table(h);
    u32 i;
    if(s->handle) return s->handle;
    for(i=0;t && i<t[0];++i) if(t[SETTINGS_ENTRY(i)]==s->id) return i;
    return PLAYBACK_NO_SETTING;
}
/* The items of the settings of the packet, in their order: the property of
   each. */
static const struct { const char *property; u8 name; } playback_settings[]={
    {"Grain",DETAIL_NAME_GRAIN},{"Bloom",DETAIL_NAME_BLOOM},{"Halation",DETAIL_NAME_HALATION},
    {"Border",DETAIL_NAME_BORDER},{"Framing",DETAIL_NAME_FRAMING},{"Crop",DETAIL_NAME_CROP},
    {"DateImprint",DETAIL_NAME_DATE}};
_Static_assert(sizeof(playback_settings)/sizeof(playback_settings[0])<=PLAYBACK_ITEMS_MAX,
               "The block holds each setting");
/* The icon of setting `handle` at `value`: the status icon of its module, as
   the live view shows it, else the tile of the value in ADJ, for a setting
   whose status icon is a text label (Framing, Date Imprint) or that has none
   (Crop); 0 without either. */
static u32 playback_icon_of(volatile u32 *h,u32 handle,u32 value) {
    u32 icon=detail_module_icon(h,handle,value), i, tile;
    if(icon && (icon<GRKIT_ICON_CORE_LABEL(0) || icon>=GRKIT_ICON_EFFECT(0))) return icon;
    for(i=0;i<module_pages(h);++i) {
        tile=h[GRKIT_HOST_MODULE_PAGES/4+i*3+2];
        if(h[GRKIT_HOST_MODULE_PAGES/4+i*3+1]!=handle || tile==MANAGE_NO_TILE) continue;
        value=crop_tile(handle,value);
        return tile<GRKIT_MODULE_TILES_MAX && value<GRKIT_MODULE_TILES_MAX-tile ? GRKIT_ICON_MODULE_TILE(tile+value) : 0;
    }
    return 0;
}
/* The name of property `property` of the namespace of GRKit in `name`. */
#define PLAYBACK_NAME_BYTES 24U
static const char *playback_name(char *name,const char *property) {
    static const char prefix[]="grkit:";
    u32 k;
    for(k=0;k+1U<sizeof(prefix);++k) name[k]=prefix[k];
    for(;*property && k+1U<PLAYBACK_NAME_BYTES;++k) name[k]=*property++;
    name[k]=0;
    return name;
}
static const PhotoXmpSetting *playback_setting(const char *property) {
    u32 i, k;
    for(i=0;i<sizeof(photo_xmp_settings)/sizeof(photo_xmp_settings[0]);++i) {
        for(k=0;property[k] && property[k]==photo_xmp_settings[i].property[k];++k) {}
        if(!property[k] && !photo_xmp_settings[i].property[k]) return &photo_xmp_settings[i];
    }
    return 0;
}
/* Write the items of the photo at `path` into the block of the playback
   (PLAYBACK_DETAILS), and give their number. It keeps its item off the stack
   frame of the poll, which holds the path. */
static __attribute__((noinline)) u32 playback_info_build(volatile u32 *h,const u8 *path) {
    DetailList list;
    DetailItem item;
    const PhotoXmpSetting *s;
    const char *xmp;
    u8 *buf=(u8 *)(services_word(h,SERVICES_DEVELOP)+DEVELOP_COPY_RECORD/4);
    u32 n=0, i, value, handle, len;
    char name[PLAYBACK_NAME_BYTES];
    list.base=playback_word(h,PLAYBACK_DETAILS); list.n=0; list.max=PLAYBACK_ITEMS_MAX;
    xmp=playback_packet(path,buf,&n);
    for(i=0;xmp && i<sizeof(playback_settings)/sizeof(playback_settings[0]);++i) {
        if(!(s=playback_setting(playback_settings[i].property)) || !(value=playback_state(xmp,n,s->id)) ||
           !photo_xmp_find(xmp,n,playback_name(name,playback_settings[i].property),&len) || !len) continue;
        detail_start(&item,DETAIL_GROUP_MODULES,playback_settings[i].name,0);
        handle=playback_handle(h,s);
        if(handle!=PLAYBACK_NO_SETTING) item.icon=playback_icon_of(h,handle,value);
        detail_put(&list,&item);
    }
    list.base[DETAIL_COUNT/4]=list.n;
    return list.n;
}

/* The poll: while playback shows, publish its controller for the live view
   (PLAYBACK_CONTROLLER), and on the first page of the detailed information
   give the items of the photo that shows. A path that differs from that of
   the items reads the photo; the empty path marks the items as changing, so
   that the live view does not paint them meanwhile. Returns whether the
   poll goes on. */
#ifndef PLAYBACK_BARRIER
#define PLAYBACK_BARRIER() __asm__ volatile("dmb ish" ::: "memory")
#endif
static u32 playback_info_poll(volatile u32 *h) {
    volatile u32 *p=playback_word(h,0);
    volatile u8 *kept=(volatile u8 *)playback_word(h,PLAYBACK_PATH);
    u32 current=services_word(h,SERVICES_MENU+MENU_ROWS_CURRENT)[0], c, i;
    u8 path[PLAYBACK_PATH_BYTES];
    if(!playback_info_on(h) || current>>24!=PLAYBACK_SINGLE>>24) {
        p[PLAYBACK_CONTROLLER/4]=0;
        return 0;
    }
    c=PLAYBACK_CONTROLLER_OF();
    p[PLAYBACK_CONTROLLER/4]=c;
    if(!c || WORD(c,CONTROLLER_ID)!=PLAYBACK_SINGLE || BYTE(c,PLAYBACK_DISPLAY)!=PLAYBACK_DETAIL ||
       BYTE(c,PLAYBACK_DISPLAY+1)!=PLAYBACK_INFO_PAGE || !PLAYBACK_PATH_SHOWN(path)) return 1;
    for(i=0;i<PLAYBACK_PATH_BYTES && kept[i]==path[i] && path[i];++i) {}
    if(i<PLAYBACK_PATH_BYTES && kept[i]==path[i]) return 1;
    kept[0]=0;
    PLAYBACK_BARRIER();
    playback_info_build(h,path);
    PLAYBACK_BARRIER();
    for(i=PLAYBACK_PATH_BYTES;i--;) kept[i]=path[i];
    PLAYBACK_INVALIDATE(WORD(c,ADJ_CANVAS));
    return 1;
}
static void playback_info_start(volatile u32 *h) {
    volatile u32 *timer=playback_word(h,PLAYBACK_TIMER);
    if(timer[0] || !playback_info_poll(h)) return;
    timer[0]=PLAYBACK_TIMER_START(PLAYBACK_POLL_MS);
}
/* A controller change to `id` starts the poll in playback. */
static void playback_info_controller(volatile u32 *h,u32 id) {
    if(h[GRKIT_HOST_SERVICES/4] && id>>24==PLAYBACK_SINGLE>>24) playback_info_start(h);
}
static u32 playback_info_fired(volatile u32 *h,u32 id) {
    volatile u32 *timer=playback_word(h,PLAYBACK_TIMER);
    if(!id || id!=timer[0]) return 0;
    timer[0]=0;
    playback_info_start(h);
    return 1;
}
#endif
