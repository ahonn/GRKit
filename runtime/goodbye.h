#ifndef GRKIT_GOODBYE_H
#define GRKIT_GOODBYE_H
/* Custom Picture of Shutdown Screen (runtime/manage.h: CORE.GOODBYE). When the camera
   turns off, the GoodBye controller asks playback for a JPEG of the firmware
   and shows it for a second. While the setting is on and the card has a
   usable GRKIT/GOODBYE.JPG, a hook gives playback the path of that file
   (runtime/menu.S: menu_goodbye). The firmware reads and decodes it as
   its own picture, with its own text over it. The hook reads the card at
   each power-off, so a file that is missing or not usable shows the picture
   of the firmware.

   The firmware reads the file into a buffer of two bytes for each pixel of
   the size of its record, and its decoder takes the size of the frame from
   the file. So a usable file is no longer than that buffer, and its frame is
   that of the pictures of the firmware: baseline, 720 by 480, with 4:2:0
   chroma (crates/grkit-core/src/goodbye.rs writes it).

   The includer defines u8, u16, u32, and the firmware addresses. */
#define GOODBYE_WIDTH 720U
#define GOODBYE_HEIGHT 480U
#define GOODBYE_BYTES_MAX (GOODBYE_WIDTH*GOODBYE_HEIGHT*2U)
/* The bytes of the start of the file that hold its frame header: the
   pictures of the firmware, and those of the site, have it after the JFIF
   segment and two quantization tables. */
#define GOODBYE_HEAD 184U
static const char goodbye_path[]="C:\\GRKIT\\GOODBYE.JPG";

/* Whether the `n` bytes at the start of a file hold the frame header of a
   picture that the camera shows: SOF0 with three components of 8 bits, the
   size of the screen picture, and 4:2:0 chroma. */
static u32 goodbye_frame(const u8 *head,u32 n) {
    u32 at=2, length;
    if(n<4 || head[0]!=0xff || head[1]!=0xd8) return 0;
    while(at+4<=n && head[at]==0xff) {
        length=(u32)head[at+2]<<8|head[at+3];
        if(head[at+1]==0xc0) {
            const u8 *f=head+at+4;
            return length==17 && at+2+length<=n && f[0]==8 &&
                   ((u32)f[1]<<8|f[2])==GOODBYE_HEIGHT && ((u32)f[3]<<8|f[4])==GOODBYE_WIDTH && f[5]==3 &&
                   f[7]==0x22 && f[10]==0x11 && f[13]==0x11;
        }
        /* Only the segments of an application and of the tables come before
           the frame. */
        if(length<2 || (head[at+1]!=0xdb && head[at+1]!=0xc4 && (head[at+1]&0xf0)!=0xe0)) return 0;
        at+=2+length;
    }
    return 0;
}

#ifndef GOODBYE_FILE
/* The length of the file of `path`, or 0 without it, and its first bytes in
   `head`: the count read. The stat of the RTOS gives the length in word 2 of
   its three words. */
static u32 goodbye_file(const char *path,u8 *head,u32 size,u32 *read) {
    u32 info[3], len=0;
    int file;
    *read=0;
    if(((int (*)(const char *,u32 *))FW_FILE_STAT)(path,info)) return 0;
    file=((int (*)(const u8 *,u32,u32))FW_FILE_OPEN)((const u8 *)path,0,0);
    if(file<0) return 0;
    ((int (*)(u32,void *,u32,u32 *))FW_FILE_READ)((u32)file,head,size,&len);
    ((int (*)(u32))FW_FILE_CLOSE)((u32)file);
    *read=len<=size ? len : 0;
    return info[2];
}
#define GOODBYE_FILE(path,head,size,read) goodbye_file((path),(head),(size),(read))
#endif

/* Whether the power-off shows the picture of the card. */
static u32 goodbye_usable(volatile u32 *h) {
    u8 head[GOODBYE_HEAD];
    u32 read, length;
    if(!goodbye_custom(h)) return 0;
    length=GOODBYE_FILE(goodbye_path,head,sizeof(head),&read);
    return length && length<=GOODBYE_BYTES_MAX && goodbye_frame(head,read);
}

#if FW_GOODBYE_INDEXES
/* A build whose playback loads the record of a picture, its width, its
   height, and the address of its path, from the JPEG table of the firmware
   by the index of the picture, and keeps the address of the path. The
   GoodBye controller asks for one of the pictures of FW_GOODBYE_INDEXES,
   by the edition of the camera, and no other screen asks for those. This
   gives the record of the card picture for such an index, or 0 for the
   record of the table. */
typedef struct { u16 width, height; const char *path; } GoodbyeRecord;
static const GoodbyeRecord goodbye_record={GOODBYE_WIDTH,GOODBYE_HEIGHT,goodbye_path};
A32_ENTRY u32 menu_goodbye_record(u32 index) {
    if(index>=32U || !(FW_GOODBYE_INDEXES>>index&1U)) return 0;
    return goodbye_usable(menu_header()) ? (u32)&goodbye_record : 0;
}
#else
/* A build whose GoodBye controller asks the ending screen service for the
   path, the width, and the height of the picture, and then gives them to
   playback: write those of the card picture, or return 0 for the picture of
   the service. `path` has FW_GOODBYE_PATH bytes. */
_Static_assert(sizeof(goodbye_path)<=FW_GOODBYE_PATH,"The path buffer of the service holds the path");
A32_ENTRY u32 menu_goodbye_select(u8 *path,u32 *width,u32 *height) {
    u32 i;
    if(!goodbye_usable(menu_header())) return 0;
    for(i=0;i<sizeof(goodbye_path);++i) path[i]=(u8)goodbye_path[i];
    *width=GOODBYE_WIDTH;
    *height=GOODBYE_HEIGHT;
    return 1;
}
#endif

/* Show Text of Shutdown Screen (runtime/manage.h: CORE.BYETEXT). The GoodBye
   controller has six views over the picture, a word each from
   GOODBYE_VIEWS: the bar and the text of the shots of today, the bar and the
   text of those with the total, the firmware version, and the text of an
   error, which is a box over the picture. Each view shows until a function
   hides it. One of two functions of the controller shows the text that the
   native setting selects and hides the other views. With the setting off, a
   hook at the entry of each hides the six views in its place
   (runtime/menu.S: menu_goodbye_text_*). A power-off with an error calls
   another function, which shows its text as the camera does. Returns
   whether it hid the views. */
#define GOODBYE_VIEWS 0x160U
#define GOODBYE_TEXT_VIEWS 6U
A32_ENTRY u32 menu_goodbye_text_hide(u32 controller) {
    u32 i;
    if(goodbye_text_shown(menu_header())) return 0;
    for(i=0;i<GOODBYE_TEXT_VIEWS;++i) MENU_VIEW2(FW_VIEW_SET_VISIBLE,MENU_OBJECT_WORD(controller,GOODBYE_VIEWS+4U*i),0);
    return 1;
}
#endif
