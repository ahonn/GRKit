#ifndef GRKIT_TONE_H
#define GRKIT_TONE_H
/* The capability CORE.TONE (grkit_core::capabilities): the effect of still
   pictures and the live view. Its value is 0 for Off, or n + 1 for effect n.
   The installer writes one record for each effect after the header, and the
   active-record pointer (GRKIT_HOST_ACTIVE) selects one. The installer
   converts the effects only when a loaded module uses CORE.TONE.

   A module page of CORE.TONE, such as that of Image Control, gets its rows
   from the core: the effect names, their icons, the live-view preview of the
   highlighted row, and the Fn adjustments of each effect. The options rows
   of the GRKit section of MENU turn each effect on or off
   (runtime/effects.h). The page gives only its title, and runtime/tone_page.h
   has its rows. runtime/setting_store.h sets the value. The includer defines
   u32 and the header fields. */

/* The index of record pointer `record`, or GRKIT_TEXT_RECORDS for a pointer
   that is not a record, such as a native one. The records are
   GRKIT_RECORD_SIZE = 17 x 256 bytes apart, and the runtime has no division:
   for an offset of at most 40 records, (offset / 256 x 3856) >> 16 is
   offset / GRKIT_RECORD_SIZE exactly, and the product checks the rest. The
   live view asks this several times a frame. */
_Static_assert(GRKIT_RECORD_SIZE==17U*256U && GRKIT_TEXT_RECORDS<4096U/17U,"record_index divides by 17 x 256");
static __attribute__((unused)) u32 record_index(volatile u32 *h,u32 record) {
    u32 offset=record-h[GRKIT_HOST_RECORDS/4], n=h[GRKIT_HOST_COUNT/4], index;
    if(n>GRKIT_TEXT_RECORDS || offset>=n*GRKIT_RECORD_SIZE) return GRKIT_TEXT_RECORDS;
    index=((offset>>8)*3856U)>>16;
    return index*GRKIT_RECORD_SIZE==offset ? index : GRKIT_TEXT_RECORDS;
}
/* The value of CORE.TONE: the row of the active record, or 0 for Off or for a
   pointer that is not a record. */
static __attribute__((unused)) u32 tone_value(volatile u32 *h) {
    u32 i=record_index(h,h[GRKIT_HOST_ACTIVE/4]);
    return i<GRKIT_TEXT_RECORDS ? i+1 : 0;
}
/* The number of values: Off and each effect. */
static __attribute__((unused)) u32 tone_values(volatile u32 *h) {
    u32 n=h[GRKIT_HOST_COUNT/4];
    return n<=GRKIT_TEXT_RECORDS ? n+1 : 1;
}
/* The record of a value, or 0 for Off. */
static __attribute__((unused)) u32 tone_record(volatile u32 *h,u32 value) {
    return value ? h[GRKIT_HOST_RECORDS/4]+(value-1)*GRKIT_RECORD_SIZE : 0;
}
/* The native value 0..8 (4 is neutral) of Fn adjustment `index` of a record:
   (stored + default + 4) mod 9. A stored byte (GRKIT_RECORD_PARAMETERS) is
   the change from the default (GRKIT_RECORD_FN_DEFAULTS), and a byte above 8
   counts as 0. module_parameters (runtime/processing.h) computes the same
   value. Two subtractions take the rest: the runtime has no division. The
   includer defines BYTE. */
static __attribute__((unused)) u32 record_parameter(u32 record,u32 index) {
    u32 value=BYTE(record,GRKIT_RECORD_PARAMETERS+index);
    value=(value>8 ? 0 : value)+BYTE(record,GRKIT_RECORD_FN_DEFAULTS+index)+4;
    if(value>=18) value-=18;
    if(value>=9) value-=9;
    return value;
}
#endif
