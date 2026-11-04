#ifndef GRKIT_MODULE_ICONS_H
#define GRKIT_MODULE_ICONS_H
/* These helpers have no firmware entrypoints. Thumb keeps their shared
   lookup code within the fixed menu/viewfinder tail on every camera. */
#ifdef __arm__
#define GRKIT_ICON_COMPACT __attribute__((target("thumb")))
#else
#define GRKIT_ICON_COMPACT
#endif
/* Installed GrkitIcon descriptors. The installer resolves their setting
   references; ADJ and the viewfinder use the same committed-value lookup. */
#ifndef MODULE_ICON_WORD
#define MODULE_ICON_WORD(address,offset) WORD(address,offset)
#endif
#define MODULE_ICON_SIZE 16U
#define MODULE_ICON_KIND 4U
#define MODULE_ICON_SETTING 8U
#define MODULE_ICON_FIRST 12U
/* The icon of descriptor `address`. A list row of setting `handle` shows
   the icon of its own `value` before it applies; other icons show the
   committed value of their setting. */
static GRKIT_MAYBE_UNUSED GRKIT_ICON_COMPACT u32 module_icon_row(volatile u32 *h,u32 address,u32 tile_base,u32 handle,u32 row_value) {
    u32 kind,setting,value=0,tile;
    if(!address || MODULE_ICON_WORD(address,0)<MODULE_ICON_SIZE) return GRKIT_ICON_OFF;
    kind=MODULE_ICON_WORD(address,MODULE_ICON_KIND);
    setting=MODULE_ICON_WORD(address,MODULE_ICON_SETTING);
    if(kind==2) {
        if(setting!=GRKIT_CORE_TONE || !(h[GRKIT_HOST_CAPABILITIES/4]&GRKIT_CORE_TONE_BIT)) return GRKIT_ICON_OFF;
        value=tone_value(h);
        return value ? GRKIT_ICON_EFFECT(value-1) : GRKIT_ICON_OFF;
    }
    if(kind!=1 && kind!=3) return GRKIT_ICON_OFF;
    if(setting!=0xffffffffU) {
        if(!setting_limit(h,setting)) return GRKIT_ICON_OFF;
        value=setting==handle ? row_value : setting_value(h,setting);
        if(!setting_offered(h,setting,value)) return GRKIT_ICON_OFF;
    }
    tile=MODULE_ICON_WORD(address,MODULE_ICON_FIRST);
    if(kind==3) return tile<GRKIT_LABEL_COUNT && value<GRKIT_LABEL_COUNT-tile ? GRKIT_ICON_CORE_LABEL(tile+value) : GRKIT_ICON_OFF;
    if(tile_base>=GRKIT_MODULE_TILES_MAX || tile>=GRKIT_MODULE_TILES_MAX-tile_base ||
       value>=GRKIT_MODULE_TILES_MAX-tile_base-tile) return GRKIT_ICON_OFF;
    return GRKIT_ICON_MODULE_TILE(tile_base+tile+value);
}
static GRKIT_MAYBE_UNUSED GRKIT_ICON_COMPACT u32 module_icon_value(volatile u32 *h,u32 address,u32 tile_base) {
    return module_icon_row(h,address,tile_base,0xffffffffU,0);
}
/* Module tile descriptors follow the effects in the icon chunks. */
static GRKIT_MAYBE_UNUSED GRKIT_ICON_COMPACT u32 module_tile_address(volatile u32 *h,u32 id) {
    u32 tile=id-GRKIT_ICON_MODULE_TILE(0),slot,chunk;
    if(tile>=GRKIT_MODULE_TILES_MAX || h[GRKIT_HOST_COUNT/4]>GRKIT_TEXT_RECORDS) return 0;
    slot=GRKIT_HOST_ICON_COUNT+h[GRKIT_HOST_COUNT/4]+tile;
    chunk=slot*1639U>>16;
    if(chunk>=GRKIT_HOST_ICON_CHUNKS_WORDS || !h[GRKIT_HOST_ICON_CHUNKS/4+chunk]) return 0;
    return h[GRKIT_HOST_ICON_CHUNKS/4+chunk]+(slot-chunk*GRKIT_ICONS_PER_CHUNK)*GRKIT_ICON_STRIDE;
}
#endif
