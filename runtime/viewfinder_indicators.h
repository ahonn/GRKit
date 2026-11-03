#ifndef GRKIT_VIEWFINDER_INDICATORS_H
#define GRKIT_VIEWFINDER_INDICATORS_H
#include "module_icons.h"
/* Each installed entry owns a descriptor in module code and a tile base.
   The same descriptor address identifies its owner to soft-Off services. */
#ifndef INDICATOR_WORD
#define INDICATOR_WORD(address,offset) WORD(address,offset)
#endif
#define INDICATOR_NATIVE_SLOT 1U
#define INDICATOR_STATUS_ROW 2U
#define INDICATOR_IMAGE_CONTROL 1U
#define INDICATOR_ENTRY(h,i) ((h)+GRKIT_HOST_INDICATORS/4+(i)*2)
static GRKIT_MAYBE_UNUSED GRKIT_ICON_COMPACT u32 viewfinder_indicators(volatile u32 *h) {
    u32 count=h[GRKIT_HOST_INDICATOR_COUNT/4];
    return count<=GRKIT_INDICATORS_MAX ? count : 0;
}
static GRKIT_MAYBE_UNUSED GRKIT_ICON_COMPACT u32 viewfinder_indicator_icon(volatile u32 *h,u32 index) {
    volatile u32 *entry=INDICATOR_ENTRY(h,index);
    u32 address=entry[0],count,conditions,i;
    if(!address || services_suspended(h,address) || INDICATOR_WORD(address,0)<24) return GRKIT_ICON_OFF;
    count=INDICATOR_WORD(address,12); conditions=INDICATOR_WORD(address,16);
    if(!count || count>4 || !conditions) return GRKIT_ICON_OFF;
    for(i=0;i<count;++i) {
        u32 condition=conditions+i*12,setting=INDICATOR_WORD(condition,0);
        u32 comparison=INDICATOR_WORD(condition,4),value=INDICATOR_WORD(condition,8),actual;
        if(!setting_limit(h,setting)) return GRKIT_ICON_OFF;
        actual=setting_value(h,setting);
        if((comparison==1 && actual!=value) || (comparison==2 && actual==value) ||
           (comparison!=1 && comparison!=2)) return GRKIT_ICON_OFF;
    }
    return module_icon_value(h,INDICATOR_WORD(address,20),entry[1]);
}
static GRKIT_MAYBE_UNUSED GRKIT_ICON_COMPACT u32 viewfinder_slot_icon(volatile u32 *h,u32 slot,u32 native_icon) {
    u32 i;
    for(i=0;i<viewfinder_indicators(h);++i) {
        u32 address=INDICATOR_ENTRY(h,i)[0],icon;
        if(!address || INDICATOR_WORD(address,4)!=INDICATOR_NATIVE_SLOT || INDICATOR_WORD(address,8)!=slot) continue;
        icon=viewfinder_indicator_icon(h,i);
        return icon==GRKIT_ICON_OFF ? native_icon : icon;
    }
    /* ABI 2.0/2.1 providers predate explicit indicators. Their owner is
       recorded by INSTALL; new modules never acquire an implicit slot. */
    if(slot==INDICATOR_IMAGE_CONTROL && h[GRKIT_HOST_LEGACY_TONE_OWNER/4] &&
       !services_suspended(h,h[GRKIT_HOST_LEGACY_TONE_OWNER/4]) &&
       (h[GRKIT_HOST_CAPABILITIES/4]&GRKIT_CORE_TONE_BIT)) {
        u32 value=tone_value(h);
        if(value) return GRKIT_ICON_EFFECT(value-1);
    }
    return native_icon;
}
static GRKIT_MAYBE_UNUSED GRKIT_ICON_COMPACT u32 viewfinder_status_active(volatile u32 *h) {
    u32 i;
    for(i=0;i<viewfinder_indicators(h);++i) {
        u32 address=INDICATOR_ENTRY(h,i)[0];
        if(address && INDICATOR_WORD(address,4)==INDICATOR_STATUS_ROW &&
           viewfinder_indicator_icon(h,i)!=GRKIT_ICON_OFF) return 1;
    }
    return 0;
}
#endif
