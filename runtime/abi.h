#ifndef GRKIT_ABI_H
#define GRKIT_ABI_H

/* Version 1 private memory layout, shared by the loader and ARM hooks. */
#define GRKIT_HOST_HEADER_SIZE 256
#define GRKIT_HOST_COUNT 0
#define GRKIT_HOST_RECORDS 4
#define GRKIT_HOST_ACTIVE 8
#define GRKIT_HOST_LAST_APPLIED 12
#define GRKIT_HOST_MAGIC 16
#define GRKIT_HOST_VERSION 20
#define GRKIT_HOST_OFF_ICON 84
#define GRKIT_HOST_BACK_PENDING 92
/* Words 24..43 belong to the menu: two text descriptors (96..111) and a
   16-word event ring (112..175). */
#define GRKIT_HOST_PLUGINS_ICON 176
#define GRKIT_HOST_TOOLBAR_WIDGET 180
#define GRKIT_HOST_TOOLBAR_OWNER 184
#define GRKIT_HOST_MODULE 188
/* One icon chunk pointer: the runtime capacity fits a single chunk. */
#define GRKIT_HOST_ICON_CHUNKS 192
/* Aspect id 2 variant for the photo shown in playback: 0 none (shooting uses
   the Framing ratio), 1 not 65:24, 2 65:24. */
#define GRKIT_HOST_FRAME_FILE 196
/* Record of the last live-view build: an owned record, or 0 for native processing. */
#define GRKIT_HOST_LIVE_RECORD 200
#define GRKIT_HOST_FRAME_RATIO 208
/* ADJ Framing page preview: 0 none, else the highlighted ratio + 1. */
#define GRKIT_HOST_FRAME_PREVIEW 212
#define GRKIT_HOST_FRAME_DRAWS 216
/* ADJ canvas of the preview. It stays set until a full draw erases the mask. */
#define GRKIT_HOST_FRAME_PREVIEW_CANVAS 220
#define GRKIT_HOST_FRAMING_ICON 224
#define GRKIT_HOST_FRAME_OWNER 228
#define GRKIT_HOST_FRAME_CANVAS 232
#define GRKIT_HOST_FRAME_BITMAP 236
#define GRKIT_HOST_FRAME_SKIPPED 240
/* Live-view-only override: 0 none, 1 native processing, else an owned record. */
#define GRKIT_HOST_PREVIEW 244
/* Record applied by the last non-live-view parameter build, or 0. Capture metadata reads it. */
#define GRKIT_HOST_STILL 248
/* Return to GRKit Image Control after the native parameter page: 1 opening, 2 away. */
#define GRKIT_HOST_RETURN 252

#define GRKIT_RECORD_SIZE 0x700
#define GRKIT_RECORD_COEFFICIENTS 84
#define GRKIT_RECORD_NAME 0x668
#define GRKIT_RECORD_MONOCHROME 0x690
#define GRKIT_RECORD_ICON 0x694
#define GRKIT_RECORD_ID 0x698
#define GRKIT_RECORD_SUMMARY 0x6b8
#define GRKIT_SUMMARY_UNITS 32
/* u16 ImageTone code recorded in photos: 0x8000 | hash of the effect ID. */
#define GRKIT_RECORD_CODE 0x6f8
#define GRKIT_ICON_STRIDE 6408

#endif
