#ifndef GRKIT_SETTING_STORE_H
#define GRKIT_SETTING_STORE_H
/* Set the settings of runtime/settings.h. The menu component sets them, and
   the other components only read them. The includer defines the three
   functions that follow. */
#include "settings.h"
/* A change sends SETTING_CHANGED (runtime/services.h). */
#include "services.h"
/* Set the native still aspect as the native menu does. */
static void aspect_set_native(u32 id);
/* Send the native aspect request again, so the capture hook applies the
   crop. */
static void aspect_request(void);
/* Act on a committed effect. */
static void tone_stored(volatile u32 *h);

/* Commit a value of CORE.TONE and end its live-view preview. */
static __attribute__((unused)) void tone_store(volatile u32 *h,u32 value) {
    h[GRKIT_HOST_ACTIVE/4]=tone_record(h,value);
    h[GRKIT_HOST_PREVIEW/4]=0;
    GRKIT_STATE_CHANGED(h);
    tone_stored(h);
}

/* Publish geometry before requesting a capture change: a shot may start
   before the next live-view bind updates the cached geometry. */
static __attribute__((unused)) void aspect_refresh(volatile u32 *h) {
    h[GRKIT_HOST_FRAME_CROP/4]=capture_aspect_value(h);
    aspect_request();
}

/* Each selected ratio starts from native 3:2, including composition-only
   frames. Off clears the selection without changing the native aspect. */
static __attribute__((unused)) void aspect_store(volatile u32 *h,u32 value) {
    h[GRKIT_HOST_FRAME_RATIO/4]=value;
    if(value && aspect_native()) {
        h[GRKIT_HOST_FRAME_CROP/4]=framing_crop(h);
        aspect_set_native(0);
    }
    aspect_refresh(h);
}

/* Set a setting. A value that the setting does not offer does nothing. A
   change of a module setting counts in the table. */
static __attribute__((unused)) void setting_store(volatile u32 *h,u32 handle,u32 value) {
    volatile u32 *t;
    if(IS_CORE_SETTING(handle)) {
        /* A saved ratio can be hidden by the native aspect. Compare the
           stored selection so clearing that hidden ratio is persisted. */
        u32 old=handle==GRKIT_CORE_ASPECT ? h[GRKIT_HOST_FRAME_RATIO/4] : setting_value(h,handle);
        if(!setting_offered(h,handle,value)) return;
        if(handle==GRKIT_CORE_TONE) tone_store(h,value);
        else if(handle==GRKIT_CORE_ASPECT) aspect_store(h,value);
        else if(handle==GRKIT_CORE_ASPECT_MODE) {
            h[GRKIT_HOST_ASPECT_MODE/4]=value;
            aspect_refresh(h);
        }
        else h[core_setting_words[CORE_SETTING(handle)]/4]=value;
        if(old!=value) {
            GRKIT_STATE_CHANGED(h);
            services_emit(h,SERVICE_EVENT_SETTING_CHANGED,handle,value);
        }
        return;
    }
    t=settings_table(h);
    if(!t || handle>=t[0] || value>=t[SETTINGS_ENTRY(handle)+2] || t[SETTINGS_ENTRY(handle)+1]==value) return;
    t[SETTINGS_ENTRY(handle)+1]=value;
    ++t[1];
    GRKIT_STATE_CHANGED(h);
    services_emit(h,SERVICE_EVENT_SETTING_CHANGED,handle,value);
}
#endif
