#ifndef GRKIT_PERSIST_VIEW_H
#define GRKIT_PERSIST_VIEW_H
/* The complete value view of STATE.TXT. INSTALL captures the restored
   baseline; the App task captures later requests. Accessors select either
   live settings or the shared settings kept while a user-mode set applies.
   The consumer owns the buffer from READY until RELEASE, so capture may
   compare and replace words in place without another full-size buffer. */
static u32 persist_live_core(volatile u32 *h,u32 k) {
    static const u16 words[]={GRKIT_CORE_SETTING_WORDS};
    if(GRKIT_CORE_SETTING_BASE+k==GRKIT_CORE_TONE) {
        u32 active=h[GRKIT_HOST_ACTIVE/4], first=h[GRKIT_HOST_RECORDS/4];
        u32 i, n=h[GRKIT_HOST_COUNT/4];
        if(n>GRKIT_TEXT_RECORDS) return 0;
        for(i=0;i<n;++i)
            if(active==first+i*GRKIT_RECORD_SIZE) return PERSIST_HALF(active,GRKIT_RECORD_CODE);
        return 0;
    }
    return h[words[k]/4];
}
static u32 persist_live_fn(u32 record) {
    u32 value=0, d=9, digit;
    while(d--) {
        digit=PERSIST_BYTE(record,GRKIT_RECORD_PARAMETERS+d);
        value=value*9U+(digit<=8U ? digit : 0U);
    }
    return value;
}
#ifndef PERSIST_CORE_VALUE
#define PERSIST_CORE_VALUE(h,k) persist_live_core(h,k)
#endif
#ifndef PERSIST_FN_VALUE
#define PERSIST_FN_VALUE(h,r,i) persist_live_fn(r)
#endif
#ifndef PERSIST_TONING_VALUE
#define PERSIST_TONING_VALUE(h,r,i) PERSIST_BYTE(r,GRKIT_RECORD_TONING)
#endif
#ifndef PERSIST_MODULE_VALUE
#define PERSIST_MODULE_VALUE(h,t,i) ((t)[SETTINGS_ENTRY(i)+1])
#endif
static u32 persist_put(volatile u32 *out,u32 offset,u32 value) {
    u32 changed=out[offset/4]!=value;
    out[offset/4]=value;
    return changed;
}
/* All unused words are zero. Revision follows effective content, not UI
   counters: temporary edits and recipe changes cannot postpone this file. */
static u32 persist_capture(volatile u32 *h,volatile u32 *out) {
    volatile u32 *t=PERSIST_SETTINGS(h);
    u32 changed=0, i, record, value, n=h[GRKIT_HOST_COUNT/4];
    for(i=0;i<GRKIT_PERSIST_CORE_COUNT;++i)
        changed|=persist_put(out,GRKIT_PERSIST_CORE+4U*i,PERSIST_CORE_VALUE(h,i));
    if(n>GRKIT_TEXT_RECORDS) n=0;
    changed|=persist_put(out,GRKIT_PERSIST_EFFECT_COUNT,n);
    for(i=0;i<GRKIT_TEXT_RECORDS;++i) {
        u32 offset=GRKIT_PERSIST_EFFECTS+4U*GRKIT_PERSIST_EFFECT_WORDS*i;
        record=h[GRKIT_HOST_RECORDS/4]+i*GRKIT_RECORD_SIZE;
        changed|=persist_put(out,offset,i<n ? PERSIST_HALF(record,GRKIT_RECORD_CODE)&0x7fffU : 0);
        changed|=persist_put(out,offset+4U,i<n ? PERSIST_FN_VALUE(h,record,i) : 0);
        value=i<n ? PERSIST_TONING_VALUE(h,record,i) : 0;
        changed|=persist_put(out,offset+8U,value<=8U ? value : 0);
    }
    n=t ? t[0] : 0;
    if(n>GRKIT_MODULE_SETTINGS_MAX) n=0;
    changed|=persist_put(out,GRKIT_PERSIST_MODULE_COUNT,n);
    for(i=0;i<GRKIT_MODULE_SETTINGS_MAX;++i) {
        u32 offset=GRKIT_PERSIST_MODULES+4U*GRKIT_PERSIST_MODULE_WORDS*i;
        value=i<n ? t[SETTINGS_ENTRY(i)] : 0;
        changed|=persist_put(out,offset,value);
        changed|=persist_put(out,offset+4U,value ? PERSIST_MODULE_VALUE(h,t,i) : 0);
    }
    return changed;
}
static __attribute__((unused)) void persist_baseline(volatile u32 *h,volatile u32 *out) {
    persist_capture(h,out);
    out[GRKIT_PERSIST_REVISION/4]=1;
    out[GRKIT_PERSIST_REQUEST/4]=0;
    out[GRKIT_PERSIST_READY/4]=0;
    out[GRKIT_PERSIST_RELEASE/4]=0;
}
#endif
