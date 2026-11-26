#ifndef GRKIT_BATTERY_PERCENT_H
#define GRKIT_BATTERY_PERCENT_H
/* Battery Percentage, a row of Display Settings: the battery icon shows the
   charge as a number in steps of 2 %. The native bars stay full for most of
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
#define BATTERY_ICON_WIDTH 60U
#define BATTERY_ICON_HEIGHT 40U
/* Picture n takes host slots GRKIT_ICON_SLOT_BATTERY + 2n and the next one,
   and its icon ID is the ID of its first slot. */
#define BATTERY_ICON(n) GRKIT_ICON_BATTERY(2U*(n))
#define BATTERY_ICON_PIXELS (GRKIT_ICON_STRIDE-GRKIT_GRF_ICON_SIZE)
_Static_assert(BATTERY_ICON_WIDTH*BATTERY_ICON_HEIGHT*4U+BATTERY_ICON_PIXELS<=2U*GRKIT_ICON_STRIDE,
               "A battery icon fits two icon slots");
_Static_assert(GRKIT_ICON_SLOTS_BATTERY==4U,"Two pictures of two slots");
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
/* The least value that shows: the lowest step. A lower charge keeps the
   native icon. */
#define BATTERY_PERCENT_MIN BATTERY_PERCENT_STEP
/* A shown value up to this one has a yellow inside, #D6B020. */
#define BATTERY_PERCENT_LOW 20U
#define BATTERY_FILL (0xff000000U|GRKIT_TILE_FILL*0x010101U)
#define BATTERY_FILL_LOW 0xff20b0d6U
/* The load of the camera moves the reported charge by several points within
   seconds, so the icon follows its average. The shown value falls one step at
   a time, and rises only when the average is this far above it. */
#define BATTERY_PERCENT_STEP 2U
#define BATTERY_PERCENT_RISE 6U
/* The least time between two changes: about 30 s and 60 s, in units of
   battery_now. */
#define BATTERY_FALL_WAIT 36U
#define BATTERY_RISE_WAIT 72U
/* The average takes 1/16 of a new sample each unit of battery_now, about 13 s
   of memory. After a longer gap without a sample, it starts again. */
#define BATTERY_AVERAGE_SHIFT 4U
#define BATTERY_AVERAGE_GAP 32U
/* GRKIT_HOST_BATTERY_ICON: the shown value, the picture that has it, and the
   time of the last change. */
#define BATTERY_SHOWN_MASK 0xffU
#define BATTERY_FRONT 0x100U
#define BATTERY_CHANGE_SHIFT 9U
#define BATTERY_CHANGE_TIME (0xffffffffU>>BATTERY_CHANGE_SHIFT)
/* GRKIT_HOST_BATTERY_AVERAGE: the average in 1/256 points, and the low half
   of the time of its last sample. */
#define BATTERY_AVERAGE_MASK 0xffffU
#define BATTERY_AVERAGE_TIME_SHIFT 16U
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
   within one step, or 0. The level is the low half of its byte: the high half
   is another state. */
static u32 battery_charge(u32 report,u32 level) {
    u32 reported=report>>16&0xfU, charge=report>>24;
    if((report&0xfU)!=1U || charge>100U || level>BATTERY_LEVEL_MAX) return 0;
    if(reported>level+1U || level>reported+1U) return 0;
    return charge;
}
/* The generic timer (40 MHz) in units of 2^25 counts, about 0.84 s. */
static u32 battery_now(void) {
#ifdef __arm__
    u32 low, high;
    __asm__ volatile("mrrc p15, 0, %0, %1, c14" : "=r"(low), "=r"(high));
    return high<<7|low>>25;
#else
    return 0;
#endif
}
/* The average word after a sample of `charge` at time `now`. */
static u32 battery_average(u32 average,u32 charge,u32 now,u32 fresh) {
    u32 gap=(now-(average>>BATTERY_AVERAGE_TIME_SHIFT))&BATTERY_AVERAGE_MASK;
    int value=(int)(average&BATTERY_AVERAGE_MASK), sample=(int)(charge<<8);
    if(fresh || gap>=BATTERY_AVERAGE_GAP) value=sample;
    else value+=(sample-value)>>BATTERY_AVERAGE_SHIFT;
    return now<<BATTERY_AVERAGE_TIME_SHIFT|(u32)value;
}
/* The whole points of an average word. */
static u32 battery_points(u32 average) { return ((average&BATTERY_AVERAGE_MASK)+128U)>>8; }
/* The state for `points` of average at time `now`. Without a shown value,
   the points show at once. */
static u32 battery_next(u32 state,u32 points,u32 now) {
    u32 shown=state&BATTERY_SHOWN_MASK, front=state&BATTERY_FRONT, target=(points+1U)&~1U;
    u32 since=(now-(state>>BATTERY_CHANGE_SHIFT))&BATTERY_CHANGE_TIME, value=shown;
    if(!shown) value=target;
    else if(target<shown && since>=BATTERY_FALL_WAIT) value=shown-BATTERY_PERCENT_STEP;
    else if(points>=shown+BATTERY_PERCENT_RISE && since>=BATTERY_RISE_WAIT) value=target;
    return value==shown ? state : front|value|now<<BATTERY_CHANGE_SHIFT;
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
/* The digits of `value`, 0 to 100, without a leading zero. Returns their count. */
static u32 battery_digits(u32 value,u32 digits[3]) {
    u32 n=0, tens=BATTERY_TENS(value);
    if(tens==10U) { digits[n++]=1U; digits[n++]=0U; }
    else if(tens) digits[n++]=tens;
    digits[n++]=value-tens*10U;
    return n;
}
/* Write the picture of `value`, BATTERY_PERCENT_MIN to 100: the number over the battery. */
static void battery_write(volatile u32 *h,volatile u32 *to,u32 value) {
    const volatile u8 *table=(const volatile u8 *)GRKIT_LABELS(h);
    u32 digits[3], n=battery_digits(value,digits), width=BATTERY_DIGIT_OUTLINES, x, y, i;
    u32 fill=value<=BATTERY_PERCENT_LOW ? BATTERY_FILL_LOW : BATTERY_FILL;
    for(y=0;y<BATTERY_ICON_HEIGHT;++y)
        for(x=0;x<BATTERY_ICON_WIDTH;++x) to[y*BATTERY_ICON_WIDTH+x]=battery_frame(x,y,fill);
    for(i=0;i<n;++i) width+=card_field(table,(GRKIT_BATTERY_DIGIT_FIRST+digits[i])*8U,2)-BATTERY_DIGIT_OUTLINES;
    if(width>BATTERY_ICON_WIDTH-BATTERY_DIGIT_MARGIN) return;
    x=BATTERY_BODY_CENTER-width/2U;
    for(i=0;i<n;++i) x+=battery_digit(table,to,x,digits[i]);
}
/* The descriptor of a picture, or 0. */
static u32 battery_icon_address(volatile u32 *h,u32 front) {
    return GRKIT_HOST_ICON(h,GRKIT_ICON_SLOT_BATTERY+2U*front);
}
/* The icon ID for the native level, or 0 for the native icon. A report below
   BATTERY_PERCENT_MIN ends the number only through the average, so that a
   moment of load does not bring the native icon back. */
A32_ENTRY u32 menu_battery_percent_icon(u32 level) {
    volatile u32 *h=menu_header();
    u32 state=h[GRKIT_HOST_BATTERY_ICON/4], front=!!(state&BATTERY_FRONT), shown=state&BATTERY_SHOWN_MASK;
    u32 average=h[GRKIT_HOST_BATTERY_AVERAGE/4], charge, now, next, value, icon;
    charge=battery_percent_on(h) && level>=BATTERY_LEVEL_LOW ? battery_charge(battery_report(),level) : 0;
    if(!charge || (!shown && charge<BATTERY_PERCENT_MIN) || !h[GRKIT_HOST_GRKIT_ICON/4]) {
        if(state&~BATTERY_FRONT) h[GRKIT_HOST_BATTERY_ICON/4]=state&BATTERY_FRONT;
        return 0;
    }
    now=battery_now();
    if(shown && average>>BATTERY_AVERAGE_TIME_SHIFT==(now&BATTERY_AVERAGE_MASK)) return BATTERY_ICON(front);
    average=battery_average(average,charge,now,!shown);
    h[GRKIT_HOST_BATTERY_AVERAGE/4]=average;
    if(battery_points(average)<BATTERY_PERCENT_MIN) {
        h[GRKIT_HOST_BATTERY_ICON/4]=state&BATTERY_FRONT;
        return 0;
    }
    next=battery_next(state,battery_points(average),now);
    value=next&BATTERY_SHOWN_MASK;
    if(value!=shown) {
        front=!front;
        icon=battery_icon_address(h,front);
        battery_write(h,(volatile u32 *)(icon+BATTERY_ICON_PIXELS),value);
        next=(next&~BATTERY_FRONT)|(front ? BATTERY_FRONT : 0U);
    }
    if(next!=state) h[GRKIT_HOST_BATTERY_ICON/4]=next;
    return BATTERY_ICON(front);
}
#endif
