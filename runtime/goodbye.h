#ifndef GRKIT_GOODBYE_H
#define GRKIT_GOODBYE_H
/* Custom Picture of Shutdown Screen (runtime/manage.h: CORE.GOODBYE). When the camera
   turns off, the GoodBye controller asks playback for a JPEG of the firmware
   and shows it for a second. While the setting is on and the card has a
   usable GRKIT/GOODBYE.JPG, a hook gives playback the path of that file
   (runtime/menu.S: menu_goodbye). The firmware reads and decodes it as
   its own picture, with its own text over it. A file that is missing or
   not usable shows the picture of the firmware.

   The power-off does not check the file: the first shooting screen after
   the start checks it once (goodbye_ready), so the camera reads the card at
   the power-off only for a usable file, as the firmware reads its picture.
   When the Wi-Fi service writes or removes the file, it asks for a check
   at once (GRKIT_CONTROL_GOODBYE_CHECK), so the next power-off shows the
   new file. Another change of the file after the check applies at the next
   start. A power-off before a check, after a start in playback or with the
   setting turned on later, checks the file then.

   The firmware reads the file into a buffer of two bytes for each pixel of
   the size of its record, and its decoder takes the size of the frame from
   the file. So a usable file is no longer than that buffer, and its frame is
   that of the pictures of the firmware: baseline, 720 by 480, with 4:2:0
   chroma (crates/grkit-core/src/goodbye.rs writes it, and its `usable` is
   the same check).

   The includer defines u8, u16, u32, and the firmware addresses. */
#define GOODBYE_WIDTH 720U
#define GOODBYE_HEIGHT 480U
#define GOODBYE_BYTES_MAX (GOODBYE_WIDTH*GOODBYE_HEIGHT*2U)
static const char goodbye_path[]="C:\\GRKIT\\GOODBYE.JPG";

/* The check of a file reads all of it, as the decoder of the camera does: a
   decoder that meets data that ends early or does not decode does not end,
   so the camera does not turn off. The check takes the segments that the
   pictures of the firmware and of the site have: SOI, then application
   segments, quantization tables of 8 bits, and Huffman tables of baseline
   sizes around the frame; then one scan of the three components without
   restart intervals, whose Huffman data gives each block of the frame and
   ends with ones in its last byte; then EOI. It takes no comment segment:
   pictures padded with comments have stopped the power-off of a camera. Its
   state is in the services block (SERVICES_GOODBYE). */
#define GOODBYE_MCUS ((GOODBYE_WIDTH/16U)*(GOODBYE_HEIGHT/16U))
#define GOODBYE_READ_BYTES 1024U
typedef struct {
    u32 file, have, at, bits, count, broken;
    /* For each table, DC 0, DC 1, AC 0, AC 1, and each code length: the
       first code, the index of its first value, and the number of codes. */
    u16 first[4][17], index[4][17];
    u8 codes[4][17], defined[4], values[4][256];
    u8 component[3], quant[3], dc[3], ac[3], frame, quantized[4];
    u8 buf[GOODBYE_READ_BYTES];
} GoodbyeScan;
_Static_assert(sizeof(GoodbyeScan)<=GOODBYE_SCRATCH_BYTES,"The services block holds the state of the check");

#ifndef GOODBYE_FILE_OPEN
/* The length of the file of `path`, or 0 without it: the stat of the RTOS
   gives the length in word 2 of its three words. */
static u32 goodbye_file_size(const char *path) {
    u32 info[3];
    return ((int (*)(const char *,u32 *))FW_FILE_STAT)(path,info) ? 0 : info[2];
}
#define GOODBYE_FILE_SIZE(path) goodbye_file_size(path)
#define GOODBYE_FILE_OPEN(path) ((int (*)(const u8 *,u32,u32))FW_FILE_OPEN)((const u8 *)(path),0,0)
#define GOODBYE_FILE_READ(file,buf,size,len) ((int (*)(u32,void *,u32,u32 *))FW_FILE_READ)((file),(buf),(size),(len))
#define GOODBYE_FILE_CLOSE(file) ((int (*)(u32))FW_FILE_CLOSE)(file)
#endif

/* The next byte of the file, or -1 at its end or after a read error. */
static int goodbye_byte(GoodbyeScan *s) {
    u32 len=0;
    if(s->at==s->have) {
        if(s->broken) return -1;
        GOODBYE_FILE_READ(s->file,s->buf,GOODBYE_READ_BYTES,&len);
        if(!len || len>GOODBYE_READ_BYTES) { s->broken=1; return -1; }
        s->have=len; s->at=0;
    }
    return s->buf[s->at++];
}
static u32 goodbye_word(GoodbyeScan *s) {
    int a=goodbye_byte(s), b=goodbye_byte(s);
    return a<0 || b<0 ? 0 : (u32)a<<8|(u32)b;
}
/* Skip `n` bytes: 1, or 0 at the end of the file. */
static u32 goodbye_skip(GoodbyeScan *s,u32 n) {
    while(n--) if(goodbye_byte(s)<0) return 0;
    return 1;
}
/* The Huffman tables of a DHT segment of `n` bytes after its length. */
static u32 goodbye_tables(GoodbyeScan *s,u32 n) {
    u32 t, len, total, code, i;
    int b;
    while(n) {
        if((b=goodbye_byte(s))<0 || (b>>4)>1 || (b&15)>1) return 0;
        t=(u32)(b>>4)*2U+(u32)(b&15);
        for(len=1,total=0;len<=16;++len) {
            if((b=goodbye_byte(s))<0) return 0;
            s->codes[t][len]=(u8)b; total+=(u32)b;
        }
        /* Baseline tables: a DC table codes sizes 0 to 11, an AC table runs
           and sizes 1 to 10, with EOB and ZRL. */
        if(total>(t<2U ? 12U : 162U) || n<17U+total) return 0;
        for(i=0;i<total;++i) {
            if((b=goodbye_byte(s))<0 || (t<2U ? b>11 : (b&15)>10)) return 0;
            s->values[t][i]=(u8)b;
        }
        /* The canonical codes: those of each length follow the last code of
           the length before it, shifted left. */
        for(len=1,code=0,i=0;len<=16;++len) {
            s->first[t][len]=(u16)code; s->index[t][len]=(u16)i;
            code+=s->codes[t][len]; i+=s->codes[t][len];
            /* The code of all ones of a length is not a code. */
            if(code>=1U<<len) return 0;
            code<<=1;
        }
        s->defined[t]=1;
        n-=17U+total;
    }
    return 1;
}
/* The next bit of the scan, or -1: 0xff is followed by a stuffed zero in
   the data, as a marker in the scan ends it early. */
static int goodbye_bit(GoodbyeScan *s) {
    int b;
    if(!s->count) {
        if((b=goodbye_byte(s))<0) return -1;
        if(b==0xff && goodbye_byte(s)!=0) return -1;
        s->bits=(u32)b; s->count=8;
    }
    return (int)(s->bits>>--s->count&1U);
}
/* The value of the next code of table `t`, or -1. */
static int goodbye_symbol(GoodbyeScan *s,u32 t) {
    u32 code=0, len;
    int b;
    for(len=1;len<=16;++len) {
        if((b=goodbye_bit(s))<0) return -1;
        code=code<<1|(u32)b;
        if(code-s->first[t][len]<s->codes[t][len]) return s->values[t][s->index[t][len]+code-s->first[t][len]];
    }
    return -1;
}
static u32 goodbye_bits(GoodbyeScan *s,u32 n) {
    while(n--) if(goodbye_bit(s)<0) return 0;
    return 1;
}
/* One block of 64 coefficients with tables `dc` and `ac`. */
static u32 goodbye_block(GoodbyeScan *s,u32 dc,u32 ac) {
    int v=goodbye_symbol(s,dc);
    u32 k=1;
    if(v<0 || v>11 || !goodbye_bits(s,(u32)v)) return 0;
    while(k<64U) {
        if((v=goodbye_symbol(s,2U+ac))<0) return 0;
        if(!v) return 1;
        if(v==0xf0) { k+=16U; continue; }
        if(!(v&15)) return 0;
        k+=(u32)(v>>4);
        if(k>63U || !goodbye_bits(s,(u32)(v&15))) return 0;
        ++k;
    }
    return k==64U;
}
/* The scan after its header: each unit of 16 by 16 pixels has four blocks
   of luma and one of each chroma component, then EOI. */
static u32 goodbye_scan(GoodbyeScan *s) {
    u32 unit, i;
    int b;
    static const u8 order[6]={0,0,0,0,1,2};
    for(unit=0;unit<GOODBYE_MCUS;++unit)
        for(i=0;i<6U;++i)
            if(!goodbye_block(s,s->dc[order[i]],s->ac[order[i]])) return 0;
    if((s->bits&((1U<<s->count)-1U))!=(1U<<s->count)-1U) return 0;
    if(goodbye_byte(s)!=0xff) return 0;
    while((b=goodbye_byte(s))==0xff) {}
    return b==0xd9;
}
/* The frame header after its length: baseline, 8 bits, 720 by 480, three
   components with 4:2:0 chroma. */
static u32 goodbye_frame(GoodbyeScan *s,u32 n) {
    u8 f[15];
    u32 i;
    int b;
    if(n!=15U) return 0;
    for(i=0;i<15U;++i) { if((b=goodbye_byte(s))<0) return 0; f[i]=(u8)b; }
    if(f[0]!=8 || ((u32)f[1]<<8|f[2])!=GOODBYE_HEIGHT || ((u32)f[3]<<8|f[4])!=GOODBYE_WIDTH || f[5]!=3 ||
       f[7]!=0x22 || f[10]!=0x11 || f[13]!=0x11) return 0;
    for(i=0;i<3U;++i) {
        if(f[8+3*i]>3U) return 0;
        s->component[i]=f[6+3*i]; s->quant[i]=f[8+3*i];
    }
    if(f[6]==f[9] || f[6]==f[12] || f[9]==f[12]) return 0;
    s->frame=1;
    return 1;
}
/* The scan header after its length, then the scan. */
static u32 goodbye_start_scan(GoodbyeScan *s,u32 n) {
    u32 i;
    int b, t;
    if(!s->frame || n!=10U || goodbye_byte(s)!=3) return 0;
    for(i=0;i<3U;++i) {
        if(goodbye_byte(s)!=s->component[i] || (t=goodbye_byte(s))<0 || (t>>4)>1 || (t&15)>1 ||
           !s->defined[t>>4] || !s->defined[2+(t&15)] || !s->quantized[s->quant[i]]) return 0;
        s->dc[i]=(u8)(t>>4); s->ac[i]=(u8)(t&15);
    }
    if(goodbye_byte(s)!=0 || goodbye_byte(s)!=63 || (b=goodbye_byte(s))!=0) return 0;
    return goodbye_scan(s);
}
/* The quantization tables of a DQT segment of `n` bytes after its length:
   8 bits each. */
static u32 goodbye_quantization(GoodbyeScan *s,u32 n) {
    int b;
    while(n) {
        if(n<65U || (b=goodbye_byte(s))<0 || (b>>4)!=0 || (b&15)>3 || !goodbye_skip(s,64)) return 0;
        s->quantized[b&15]=1;
        n-=65U;
    }
    return 1;
}
/* Whether the open file is a picture that the camera shows. */
static u32 goodbye_picture(GoodbyeScan *s) {
    int b;
    u32 n;
    if(goodbye_byte(s)!=0xff || goodbye_byte(s)!=0xd8) return 0;
    for(;;) {
        if(goodbye_byte(s)!=0xff) return 0;
        while((b=goodbye_byte(s))==0xff) {}
        if(b<0 || (n=goodbye_word(s))<2U) return 0;
        n-=2U;
        if(b==0xda) return goodbye_start_scan(s,n);
        if(b==0xc0 ? s->frame || !goodbye_frame(s,n) : b==0xc4 ? !goodbye_tables(s,n) :
           b==0xdb ? !goodbye_quantization(s,n) : (b&0xf0)==0xe0 ? !goodbye_skip(s,n) : 1)
            return 0;
    }
}
/* Whether the card has a picture that the camera shows. */
static u32 goodbye_file_usable(volatile u32 *h) {
    GoodbyeScan *s;
    u32 length, i, usable;
    int file;
    if(!h[GRKIT_HOST_SERVICES/4]) return 0;
    length=GOODBYE_FILE_SIZE(goodbye_path);
    if(!length || length>GOODBYE_BYTES_MAX || (file=GOODBYE_FILE_OPEN(goodbye_path))<0) return 0;
    s=(GoodbyeScan *)(services_word(h,SERVICES_GOODBYE)+1);
    for(i=0;i<sizeof(*s);++i) ((volatile u8 *)s)[i]=0;
    s->file=(u32)file;
    usable=goodbye_picture(s);
    GOODBYE_FILE_CLOSE((u32)file);
    return usable;
}
/* The states of SERVICES_GOODBYE. */
#define GOODBYE_UNCHECKED 0U
#define GOODBYE_NONE 1U
#define GOODBYE_USABLE 2U
static u32 goodbye_check(volatile u32 *h) {
    u32 state=goodbye_file_usable(h) ? GOODBYE_USABLE : GOODBYE_NONE;
    if(h[GRKIT_HOST_SERVICES/4]) services_word(h,SERVICES_GOODBYE)[0]=state;
    return state;
}
/* The first shooting screen after the start: check the file while the
   setting is on. */
static void goodbye_ready(volatile u32 *h) {
    if(goodbye_custom(h)) goodbye_check(h);
}
/* Whether the power-off shows the picture of the card. */
static u32 goodbye_usable(volatile u32 *h) {
    u32 state;
    if(!goodbye_custom(h)) return 0;
    state=h[GRKIT_HOST_SERVICES/4] ? services_word(h,SERVICES_GOODBYE)[0] : GOODBYE_UNCHECKED;
    if(state==GOODBYE_UNCHECKED) state=goodbye_check(h);
    return state==GOODBYE_USABLE;
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
