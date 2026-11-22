#ifndef GRKIT_PLAYBACK_PHOTO_H
#define GRKIT_PLAYBACK_PHOTO_H
/* The photo that the playback of one photo shows, for GRKit Info in
   Playback: the poll reads it (runtime/playback_info.h), and the draw shows
   its items only while they belong to it (runtime/viewfinder.h). The
   includer defines u8, u32, WORD, BYTE, HALF, the services, and the target. */

/* The path of the photo that playback opened, with the drive of the card,
   into `path` of PLAYBACK_PATH_BYTES: 1, or 0 without one. A host test
   gives its own path. */
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
/* The folder and file number of the photo that playback shows, from the
   playback metadata: folder << 16 | file. A host test gives its own. */
#ifndef PLAYBACK_SHOWN_NUMBER
#define PLAYBACK_SHOWN_NUMBER() playback_shown_number()
static u32 playback_shown_number(void) {
    u32 metadata=((u32 (*)(void))FW_PLAYBACK_METADATA)();
    return (u32)HALF(metadata,METADATA_FOLDER)<<16 | HALF(metadata,METADATA_FILE);
}
#endif
/* `count` decimal digits at `text`: their value, or 0xffffffff for another
   character. */
static u32 playback_digits(const u8 *text,u32 count) {
    u32 value=0;
    for(;count;--count,++text) {
        if(*text<'0' || *text>'9') return 0xffffffffU;
        value=value*10U+(u32)(*text-'0');
    }
    return value;
}
/* The path of the photo that playback shows into `path` of
   PLAYBACK_PATH_BYTES: 1, or 0 while it is not known. The contents of
   playback keep the photo before the last shot while playback shows that
   shot, when playback opened before the camera saved it. The shot is in
   the folder of that photo, so the file number of the playback metadata
   goes into the 4 digits of the DCF number before the extension
   (C:\DCIM\100RICOH\R0000001.JPG). A shot in another folder is not known. */
static u32 playback_shown(u8 *path) {
    static const u8 dcim[]="C:\\DCIM\\";
    static const u16 places[4]={1000,100,10,1};
    u32 shown=PLAYBACK_SHOWN_NUMBER(), file=shown&0xffffU, n, i, digit;
    if(!PLAYBACK_PATH_SHOWN(path)) return 0;
    for(n=0;n<PLAYBACK_PATH_BYTES && path[n];++n) {}
    for(i=0;i+1U<sizeof(dcim);++i)
        if(path[i]!=dcim[i]) return 0;
    if(n<sizeof(dcim)+3U+8U || path[n-4U]!='.' || playback_digits(path+sizeof(dcim)-1U,3)!=shown>>16 ||
       playback_digits(path+n-8U,4)==0xffffffffU || file>9999U)
        return 0;
    /* No division, which the components lack. */
    for(i=0;i<4U;++i) {
        for(digit=0;file>=places[i];++digit) file-=places[i];
        path[n-8U+i]=(u8)('0'+digit);
    }
    return 1;
}
#endif
