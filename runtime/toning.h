#ifndef GRKIT_TONING_H
#define GRKIT_TONING_H
/* The Toning of a native look whose red or blue curve differs from green, as
   GR IV Cinema has: the weight of a GR IV curve set moves the red and blue
   curves toward green. A Toning value t (-4..4, the stored byte of
   GRKIT_RECORD_TONING) makes each of them green + (own - green) x (2 + t) / 2:
   0 is the look's own toning, -1 and +1 are the GR IV levels 1 and 3, -2
   makes the curves gray, and lower values turn the tint the other way. INSTALL
   keeps the look's own red and blue curves in a tail slot
   (GRKIT_TONE_CURVES), and the record curves hold the toned ones. The
   includer defines u16 and u32. */

/* The value -4..4 of a stored byte: (value) mod 9, and a byte above 8 counts as 0. */
static __attribute__((unused)) int toning_value(u32 stored) {
    return stored>8 ? 0 : stored<=4 ? (int)stored : (int)stored-9;
}

/* The u16 at `address`. The includer may map it, as the installer does. */
#ifndef TONING_HALF
#define TONING_HALF(address) (*(volatile u16 *)(address))
#endif
/* curves: the address of the record curves (red, green, blue, 256 samples
   each); own: the address of the look's own red, then blue curve. */
static __attribute__((unused)) void toning_curves(u32 curves, u32 own, int value) {
    for (u32 c=0; c<2; ++c)
        for (u32 i=0; i<256; ++i) {
            int green=TONING_HALF(curves+512+i*2);
            int v=green+(((int)TONING_HALF(own+c*512+i*2)-green)*(2+value))/2;
            TONING_HALF(curves+c*1024+i*2)=(u16)(v<0 ? 0 : v>16383 ? 16383 : v);
        }
}
#endif
