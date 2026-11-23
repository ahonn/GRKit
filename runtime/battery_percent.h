#ifndef GRKIT_BATTERY_PERCENT_H
#define GRKIT_BATTERY_PERCENT_H
/* Battery Percentage, a row of Display Settings: the battery icon shows the
   charge as a number in steps of 10 %. The native bars stay full for most of
   the charge.

   The sub-CPU report holds the source, the voltage, the native level, and the
   charge in its first word. The core reads that word and calls nothing. A low
   battery, external power, or a report that does not agree with the native
   level keeps the native icon, so the camera's warnings stay.

   The two icons alternate: a native view redraws only for another icon ID,
   and the icon on screen is never written. Each takes two host icon slots,
   because a battery icon is wider than a slot. The includer defines u32,
   WORD, BYTE, A32_ENTRY, and includes runtime/manage.h and
   runtime/recipe_card.h (card_field). */
#define GRKIT_ICON_BATTERY(n) (0x7ff6U+(n))
#define BATTERY_ICON_WIDTH 60U
#define BATTERY_ICON_HEIGHT 40U
/* The host icons of the two pictures, as offsets from the GRKit icon. */
#define BATTERY_ICON_BASE (GRKIT_HOST_ICON_COUNT-GRKIT_BATTERY_ICON_SLOTS-1U)
#define BATTERY_ICON_PIXELS (GRKIT_ICON_STRIDE-GRKIT_GRF_ICON_SIZE)
_Static_assert(BATTERY_ICON_WIDTH*BATTERY_ICON_HEIGHT*4U+BATTERY_ICON_PIXELS<=2U*GRKIT_ICON_STRIDE,
               "A battery icon fits two icon slots");
_Static_assert(GRKIT_BATTERY_ICON_SLOTS==4U,"Two pictures of two slots");
/* The number is in the bold digits of the label table, white with a black
   outline, centered on the body of the battery. */
#define BATTERY_DIGIT_TOP 11U
#define BATTERY_DIGIT_OUTLINES 2U
#define BATTERY_DIGIT_MARGIN 4U
#define BATTERY_BODY_CENTER 32U
_Static_assert(BATTERY_DIGIT_TOP+GRKIT_BATTERY_DIGIT_HEIGHT<=BATTERY_ICON_HEIGHT,"The number fits the icon");
/* The power report of the sub-CPU: a double buffer in its service. */
#define POWER_SERVICE 4U
#define POWER_INDEX 0xacU
#define POWER_INDEX_BUSY 0x8000U
#define POWER_BUFFERS 0x5cU
#define POWER_REPORT 4U
/* The native levels below this one show a warning icon. */
#define BATTERY_LEVEL_LOW 2U
#define BATTERY_LEVEL_MAX 4U
#define BATTERY_PERCENT_MIN 10U
/* A shown value up to this one has a yellow inside, #D6B020. */
#define BATTERY_PERCENT_LOW 20U
#define BATTERY_FILL (0xff000000U|GRKIT_TILE_FILL*0x010101U)
#define BATTERY_FILL_LOW 0xff20b0d6U
/* A shown value changes only when the charge is this far from it. */
#define BATTERY_PERCENT_HOLD 7U
/* GRKIT_HOST_BATTERY_ICON: the shown value, and the picture that has it. */
#define BATTERY_SHOWN_MASK 0xffU
#define BATTERY_FRONT 0x100U
/* n / 10 for n up to 105, without the division that the core cannot link. */
#define BATTERY_TENS(n) ((n)*6554U>>16)

/* The first word of the power report, or 0 before its service exists. The
   writer marks the index while it fills the other buffer. */
static u32 battery_report(void) {
    u32 service, index, buffer;
    if(!(BYTE(FW_POWER_GUARD,0)&1U)) return 0;
    service=WORD(FW_POWER_HOLDER,POWER_SERVICE);
    if(!service) return 0;
    index=WORD(service,POWER_INDEX);
    if(index&POWER_INDEX_BUSY) --index;
    buffer=WORD(service,POWER_BUFFERS+4U*(index&1U));
    return buffer ? WORD(buffer,POWER_REPORT) : 0;
}
/* The charge of a report from the battery that agrees with the native level
   within one step, or 0. */
static u32 battery_charge(u32 report,u32 level) {
    u32 reported=report>>16&0xffU, charge=report>>24;
    if((report&0xfU)!=1U || charge>100U || level>BATTERY_LEVEL_MAX) return 0;
    if(reported>level+1U || level>reported+1U) return 0;
    return charge;
}
/* The value to show for `charge`: the last one while the charge stays near
   it, so that it does not move between two values. */
static u32 battery_step(u32 charge,u32 shown) {
    if(shown && charge+BATTERY_PERCENT_HOLD>shown && charge<shown+BATTERY_PERCENT_HOLD) return shown;
    return BATTERY_TENS(charge+5U)*10U;
}
/* The outline of the battery, as the camera draws it, around `fill`. */
static u32 battery_frame(u32 x,u32 y,u32 fill) {
    const u32 clear=0x00ffffffU, edge=0xfe000000U, ink=0xfffefefeU;
    if(x>=4U && x<=10U && y>=13U && y<=26U) {
        u32 row=y<20U ? y-13U : 26U-y;
        if(x>=9U) return ink;
        if(!row) return x<5U ? clear : edge;
        if(row==1U) return x<6U ? edge : ink;
        if(x==4U) return edge;
        return row==2U || x<7U ? ink : BATTERY_FILL;
    }
    if(x<8U || x>56U || y<8U || y>31U) return clear;
    if((x==8U || x==56U) && (y==8U || y==31U)) return clear;
    if(x==8U || x==56U || y==8U || y==31U) return edge;
    if((x==9U || x==55U) && (y==9U || y==30U)) return edge;
    return x<=10U || x>=54U || y<=10U || y>=29U ? ink : fill;
}
/* Paint digit `d` over the picture. Returns its advance: the outlines of two digits share a column. */
static u32 battery_digit(const volatile u8 *table,volatile u32 *to,u32 left,u32 d) {
    u32 entry=GRKIT_BATTERY_DIGIT_FIRST+d, width=card_field(table,entry*8U,2), rows=card_field(table,entry*8U+2U,2);
    const volatile u8 *at=table+card_field(table,entry*8U+4U,4);
    u32 y, spans, first, length, i;
    for(y=0;y<rows;++y) {
        spans=*at++;
        while(spans--) {
            first=card_field(at,0,2); length=card_field(at,2,2); at+=4;
            for(i=0;i<length;++i)
                to[(BATTERY_DIGIT_TOP+y)*BATTERY_ICON_WIDTH+left+first+i]=0xff000000U|at[i]*0x010101U;
            at+=length;
        }
    }
    return width-BATTERY_DIGIT_OUTLINES;
}
/* Write the picture of `value`, 10 to 100: the number over the battery. */
static void battery_write(volatile u32 *h,volatile u32 *to,u32 value) {
    const volatile u8 *table=(const volatile u8 *)GRKIT_LABELS(h);
    u32 digits[3], n=0, width=BATTERY_DIGIT_OUTLINES, tens=BATTERY_TENS(value), x, y, i;
    u32 fill=value<=BATTERY_PERCENT_LOW ? BATTERY_FILL_LOW : BATTERY_FILL;
    if(tens==10U) { digits[n++]=1U; tens=0; }
    digits[n++]=tens;
    digits[n++]=0U;
    for(y=0;y<BATTERY_ICON_HEIGHT;++y)
        for(x=0;x<BATTERY_ICON_WIDTH;++x) to[y*BATTERY_ICON_WIDTH+x]=battery_frame(x,y,fill);
    for(i=0;i<n;++i) width+=card_field(table,(GRKIT_BATTERY_DIGIT_FIRST+digits[i])*8U,2)-BATTERY_DIGIT_OUTLINES;
    if(width>BATTERY_ICON_WIDTH-BATTERY_DIGIT_MARGIN) return;
    x=BATTERY_BODY_CENTER-width/2U;
    for(i=0;i<n;++i) x+=battery_digit(table,to,x,digits[i]);
}
/* The descriptor of a picture, or 0. */
static u32 battery_icon_address(volatile u32 *h,u32 id) {
    u32 grkit=h[GRKIT_HOST_GRKIT_ICON/4];
    return grkit ? grkit+(BATTERY_ICON_BASE+2U*(id-GRKIT_ICON_BATTERY(0)))*GRKIT_ICON_STRIDE : 0;
}
/* The icon ID for the native level, or 0 for the native icon. */
A32_ENTRY u32 menu_battery_percent_icon(u32 level) {
    volatile u32 *h=menu_header();
    u32 state=h[GRKIT_HOST_BATTERY_ICON/4], front=!!(state&BATTERY_FRONT), shown=state&BATTERY_SHOWN_MASK, charge, value, icon;
    charge=battery_percent_on(h) && level>=BATTERY_LEVEL_LOW ? battery_charge(battery_report(),level) : 0;
    if(charge<BATTERY_PERCENT_MIN || !h[GRKIT_HOST_GRKIT_ICON/4]) {
        if(shown) h[GRKIT_HOST_BATTERY_ICON/4]=state&BATTERY_FRONT;
        return 0;
    }
    value=battery_step(charge,shown);
    if(value!=shown) {
        front=!front;
        icon=battery_icon_address(h,GRKIT_ICON_BATTERY(front));
        battery_write(h,(volatile u32 *)(icon+BATTERY_ICON_PIXELS),value);
        h[GRKIT_HOST_BATTERY_ICON/4]=(front ? BATTERY_FRONT : 0U)|value;
    }
    return GRKIT_ICON_BATTERY(front);
}
#endif
