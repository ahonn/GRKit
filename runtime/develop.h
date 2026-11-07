#ifndef GRKIT_DEVELOP_H
#define GRKIT_DEVELOP_H
/* The Fn adjustments of a GRKit effect in a RAW development.

   The development model has a parameter set for each Image Control, and the
   parameter page of the development (Fn in its Image Control list) edits the
   set of the selected one. An effect row is native Standard to the model
   (runtime/menu.c), so while an effect is chosen, the Standard set holds the
   Fn adjustments of that effect (GRKIT_HOST_DEVELOP_FILLED), and the builds
   of the development read them there (module_parameters in
   runtime/processing.h, and develop_parameter). The Fn adjustments
   that the effect keeps for shooting do not change. When a native row is
   chosen, or a development starts, the set gets its own values back.

   The includer defines u32, u8, BYTE, the header fields, and
   record_parameter (runtime/tone.h). A target without DEVELOP_PARAMETERS has
   no Fn in the development: its builds read the record. */
#ifdef DEVELOP_PARAMETERS
/* The byte of parameter kind `kind` in the Standard set. A host test
   replaces it. */
#ifndef DEVELOP_SET
#define DEVELOP_SET(kind) BYTE(FW_DEVELOP_MODEL,DEVELOP_PARAMETERS+(kind))
#endif
static const u8 develop_kinds[]=PARAMETER_KINDS;
_Static_assert(sizeof(develop_kinds)==9 && GRKIT_HOST_DEVELOP_NATIVE_WORDS*4>=9,
               "GRKIT_RECORD_PARAMETERS holds nine adjustments");
/* Fn adjustment `index` (0..8, 4 neutral) of `record` in a build: from the
   Standard set while a development runs (`running`) and the set holds the
   adjustments of the record, else from the record. */
static __attribute__((unused)) u32 develop_parameter(volatile u32 *h,u32 record,u32 index,u32 running) {
    u32 value;
    if(!running || record<=1 || h[GRKIT_HOST_DEVELOP_FILLED/4]!=record) return record_parameter(record,index);
    value=DEVELOP_SET(develop_kinds[index]);
    return value>8 ? 4 : value;
}
/* Make the Standard set hold the Fn adjustments of `record`, or give it its
   own values back for 0 or 1 (native processing). The first change keeps
   those own values. An effect that leaves the set with changed adjustments
   (the parameter page edited them) keeps them, so that they come back with
   the effect, as a native Image Control keeps the values of its set. The
   development keeps them for the last changed effect. The native choice of
   a row passes the set of the row on to the processing, so call this before
   it. */
static __attribute__((unused)) void develop_hold(volatile u32 *h,u32 record) {
    volatile u8 *native=(volatile u8 *)(h+GRKIT_HOST_DEVELOP_NATIVE/4);
    volatile u8 *changes=(volatile u8 *)(h+GRKIT_HOST_DEVELOP_CHANGES/4);
    u32 filled=h[GRKIT_HOST_DEVELOP_FILLED/4], changed=0, i;
    if(record<=1) record=0;
    if(filled==record) return;
    for(i=0;i<sizeof(develop_kinds);++i) {
        u32 value=DEVELOP_SET(develop_kinds[i]);
        if(!filled) native[i]=(u8)value;
        else if(value!=record_parameter(filled,i)) changed=1;
    }
    if(changed) {
        for(i=0;i<sizeof(develop_kinds);++i) changes[i]=DEVELOP_SET(develop_kinds[i]);
        h[GRKIT_HOST_DEVELOP_CHANGED/4]=filled;
    }
    for(i=0;i<sizeof(develop_kinds);++i)
        DEVELOP_SET(develop_kinds[i])=(u8)(!record ? native[i] :
            h[GRKIT_HOST_DEVELOP_CHANGED/4]==record ? changes[i] : record_parameter(record,i));
    h[GRKIT_HOST_DEVELOP_FILLED/4]=record;
}
#else
#define develop_parameter(h,record,index,running) ((void)(h),(void)(running),record_parameter((record),(index)))
#endif
#endif
