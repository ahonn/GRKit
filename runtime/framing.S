#include "firmware.h"
#include "abi.h"
#include "hooks.h"
/* framing.c syncs the capture aspect with FRAMING_CAPTURE_ASPECT and keeps the
   live view in 3:2 with FW_CAPTURE_ASPECT_GET (target headers). The hooks
   that complete each come from the site tables of the target (hooks.h). */
#if defined(FRAMING_CAPTURE_ASPECT) != defined(HOOK_FRAMING_CAPTURE_REQUEST_HOOK)
#error "FRAMING_CAPTURE_ASPECT and the framing_capture_request_hook hook go together"
#endif
#if defined(FW_CAPTURE_ASPECT_GET) != defined(HOOK_FRAMING_LIVE_MODE)
#error "FW_CAPTURE_ASPECT_GET and the framing_live_* hooks go together"
#endif
.syntax unified
.arm
.text
/* The installer relocates this literal to the private host header. */
.global framing_header
framing_header:
    ldr r0, =GRKIT_HEADER_REFERENCE
    bx lr
#ifdef HOOK_FRAMING_CAPTURE_REQUEST_HOOK
/* Replaces `ldrb r1, [r4]` in the still aspect property handler. r1 returns
   the capture request. The next native call reads r0. */
.global framing_capture_request_hook
framing_capture_request_hook:
    push {r0-r3,r12,lr}
    ldrb r0, [r4]
    bl framing_capture_request
    str r0, [sp,#4]
    pop {r0-r3,r12,pc}
#endif
#ifdef HOOK_FRAMING_LIVE_MODE
/* Replace the capture aspect getter BL in the live-view mode function and in
   the live-view mode check (GR IV). r0 = the capture property. */
.global framing_live_mode
.global framing_live_check
framing_live_mode:
framing_live_check:
    push {r4,lr}
    bl framing_live_aspect
    pop {r4,pc}
#endif
/* Each entry hook preserves the displaced prologue and all incoming arguments. */
.macro ENTRY name, helper, resume
.global \name
\name:
    push {r0-r3,r12,lr}
    bl \helper
    pop {r0-r3,r12,lr}
    mov r12,sp
    ldr pc,=\resume
.endm
ENTRY framing_update, framing_bind, HOOK_FRAMING_UPDATE_RESUME
ENTRY framing_exit, framing_leave, HOOK_FRAMING_EXIT_RESUME
ENTRY framing_destroy, framing_leave, HOOK_FRAMING_DESTROY_RESUME
ENTRY framing_canvas, framing_prepare, HOOK_FRAMING_CANVAS_RESUME
/* Replaces the Canvas::Draw BL after its native full-background clear. */
.global framing_children
framing_children:
    push {r0-r3,r12,lr}
    bl framing_paint
    pop {r0-r3,r12,lr}
    ldr pc,=HOOK_FRAMING_CHILDREN_TARGET
#ifdef HOOK_FRAMING_CLASSIFY
/* XPan (65:24) and 2.35:1 share capture aspect id 2 with 16:9. These hooks
   switch the id 2 geometry: shooting follows the Framing ratio, playback
   follows the size of the photo that the classifier saw last. The family
   header gives the sizes (XPAN_HEIGHT_<n>: the stock 16:9, the 65:24, and the
   2.35:1 height of GetResolution id 2; XPAN_SIZES and SCOPE_SIZES: the 65:24
   and the 2.35:1 sizes, largest first) and the classifier registers at the
   aspect BL (the width, the class pointer, and the return of its frame). */
#if !defined(XPAN_HEIGHT_0) || !defined(XPAN_HEIGHT_1) || !defined(XPAN_HEIGHT_2) || \
    !defined(XPAN_HEIGHT_3) || !defined(XPAN_SIZES) || !defined(SCOPE_SIZES)
#error "The family header defines the XPan and 2.35:1 sizes"
#endif
#if !defined(CLASSIFY_WIDTH) || !defined(CLASSIFY_CLASS) || !defined(CLASSIFY_RETURN)
#error "The family header defines the classifier registers"
#endif
/* The id 2 geometry in r12: 0 = 16:9, 1 = XPan, 2 = 2.35:1. GRKIT_HOST_FRAME_FILE
   is 0 while shooting, else 1 + the geometry of the photo that the classifier
   saw last. Shooting follows GRKIT_HOST_FRAME_CROP, the crop that applies in
   the native aspect (runtime/framing.c). Clobbers only r12 and the flags. */
wide_mode:
    ldr r12, =GRKIT_HEADER_REFERENCE
    ldrb r12, [r12, #GRKIT_HOST_FRAME_FILE]
    cmp r12, #0
    beq 1f
    sub r12, r12, #1
    bx lr
1:  ldr r12, =GRKIT_HEADER_REFERENCE
    ldr r12, [r12, #GRKIT_HOST_FRAME_CROP]
    cmp r12, #1
    bxeq lr
    cmp r12, #4
    moveq r12, #2
    movne r12, #0
    bx lr
/* GetResolution id 2 heights replace `mov r3, #height` before a store and an
   unconditional branch to the epilogue, so r12 and the flags are dead. */
.macro HEIGHT name, stock, xpan, scope
.global \name
\name:
    push {lr}
    bl wide_mode
    pop {lr}
    movw r3, #\stock
    cmp r12, #1
    movweq r3, #\xpan
    cmp r12, #2
    movweq r3, #\scope
    bx lr
.endm
HEIGHT framing_height_0, XPAN_HEIGHT_0
HEIGHT framing_height_1, XPAN_HEIGHT_1
HEIGHT framing_height_2, XPAN_HEIGHT_2
HEIGHT framing_height_3, XPAN_HEIGHT_3
/* Top insets of the 720x480 screennail and 160x120 thumbnail. */
.macro INSET name, stock, xpan, scope
.global \name
\name:
    push {lr}
    bl wide_mode
    pop {lr}
    mov r1, #\stock
    cmp r12, #1
    moveq r1, #\xpan
    cmp r12, #2
    moveq r1, #\scope
    bx lr
.endm
INSET framing_screennail_inset, 38, 107, 87
INSET framing_thumbnail_inset, 16, 30, 26
#ifdef HOOK_FRAMING_WIDTH_0
/* 5:4, 1:1, and 7:6 narrow capture aspect id 1 (4:3). These hooks switch
   the id 1 geometry like the id 2 hooks above. The family header gives the
   widths (NARROW_WIDTH_<n>: the stock 4:3, the 5:4, the 1:1, and the 7:6
   width of GetResolution id 1), the XS height (NARROW_HEIGHT_3: the stock
   4:3 and the 1:1 height), and the 5:4 and 7:6 sizes. Each 1:1 size is a native 1:1 size, so a
   1:1 photo plays back as a native one and needs no playback geometry. */
#if !defined(NARROW_WIDTH_0) || !defined(NARROW_WIDTH_1) || !defined(NARROW_WIDTH_2) || \
    !defined(NARROW_WIDTH_3) || !defined(NARROW_HEIGHT_3) || !defined(NARROW_SIZES)
#error "The family header defines the 5:4, 1:1, and 7:6 sizes"
#endif
/* The id 1 geometry in r12: 0 = 4:3, 1 = 5:4, 2 = 1:1, 3 = 7:6. Shooting
   follows GRKIT_HOST_FRAME_CROP (value 6 for 5:4, 5 for 1:1, 7 for 7:6), so
   the native 4:3 keeps its sizes,
   playback the photo that the classifier saw last (GRKIT_HOST_FRAME_FILE 4
   for 5:4, 5 for 7:6). Clobbers only r12 and the flags. */
narrow_mode:
    ldr r12, =GRKIT_HEADER_REFERENCE
    ldrb r12, [r12, #GRKIT_HOST_FRAME_FILE]
    cmp r12, #0
    beq 1f
    cmp r12, #4
    moveq r12, #1
    bxeq lr
    cmp r12, #5
    moveq r12, #3
    movne r12, #0
    bx lr
1:  ldr r12, =GRKIT_HEADER_REFERENCE
    ldr r12, [r12, #GRKIT_HOST_FRAME_CROP]
    cmp r12, #6
    moveq r12, #1
    bxeq lr
    cmp r12, #7
    moveq r12, #3
    bxeq lr
    cmp r12, #5
    moveq r12, #2
    movne r12, #0
    bx lr
/* GetResolution id 1 widths replace `mov r2, #width` before the height, two
   stores, and a branch to the epilogue, so r12 and the flags are dead. */
.macro WIDTH name, stock, five, square, seven
.global \name
\name:
    push {lr}
    bl narrow_mode
    pop {lr}
    movw r2, #\stock
    cmp r12, #1
    movweq r2, #\five
    cmp r12, #2
    movweq r2, #\square
    cmp r12, #3
    movweq r2, #\seven
    bx lr
.endm
WIDTH framing_width_0, NARROW_WIDTH_0
WIDTH framing_width_1, NARROW_WIDTH_1
WIDTH framing_width_2, NARROW_WIDTH_2
WIDTH framing_width_3, NARROW_WIDTH_3
/* The id 1 XS height replaces `mov r3, #height` after the XS width. The
   native 1:1 XS is smaller than the 4:3 XS height. */
.macro HEIGHT_XS name, stock, square
.global \name
\name:
    push {lr}
    bl narrow_mode
    pop {lr}
    movw r3, #\stock
    cmp r12, #2
    movweq r3, #\square
    bx lr
.endm
HEIGHT_XS framing_narrow_height_3, NARROW_HEIGHT_3
/* The id 1 left inset of the 720x480 screennail replaces `mov r0, #40`. */
.global framing_narrow_screennail
framing_narrow_screennail:
    push {lr}
    bl narrow_mode
    pop {lr}
    mov r0, #40
    cmp r12, #1
    moveq r0, #60
    cmp r12, #2
    moveq r0, #120
    cmp r12, #3
    moveq r0, #80
    bx lr
/* The id 1 thumbnail inset: the stock code stores 0 (r1) at the left inset
   [r2], then at the top inset [r3]. This replaces the second store and
   writes the left inset of the 160x120 thumbnail again. The IIP stores a
   picture with chroma pairs only from an even column, so 5:4 takes 6
   (148 x 120) instead of 5. 1:1 takes 20 (120 x 120), and 7:6 takes 10
   (140 x 120). */
.global framing_narrow_thumbnail
framing_narrow_thumbnail:
    push {r0,lr}
    bl narrow_mode
    mov r0, #0
    cmp r12, #1
    moveq r0, #6
    cmp r12, #2
    moveq r0, #20
    cmp r12, #3
    moveq r0, #10
    strh r0, [r2]
    strh r1, [r3]
    pop {r0,pc}
#endif
/* r0 = width, r1 = height. r12 = the index in wide_sizes: 0..3 for the 65:24
   size classes, 4..7 for the 2.35:1 size classes, 8..11 for the 5:4 size
   classes, 12..15 for the 7:6 size classes, or -1.
   Clobbers r0-r3, r12 and the flags. */
wide_size:
    cmp r0, r1
    movhs r2, r0
    movhs r3, r1
    movlo r2, r1
    movlo r3, r0
    adr r12, wide_sizes
    push {r4}
    mov r4, #0
2:  ldrh r0, [r12], #2
    ldrh r1, [r12], #2
    cmp r0, #0
    beq 3f
    cmp r0, r2
    cmpeq r1, r3
    beq 4f
    add r4, r4, #1
    b 2b
3:  mvn r4, #0
4:  mov r12, r4
    pop {r4}
    bx lr
wide_sizes:
    .hword XPAN_SIZES, SCOPE_SIZES
#ifdef NARROW_SIZES
    .hword NARROW_SIZES
#endif
    .hword 0, 0
/* Replaces the aspect BL in the size classifier (GR III 0x532b3988: frame
   {r4-r7,fp,ip,lr,pc}, r7 = width, r4 = height, r6 = class pointer).
   A 65:24, 2.35:1, 5:4, or 7:6 size records its geometry
   (GRKIT_HOST_FRAME_FILE 2, 3, 4, or 5), then returns its size class and success from the classifier.
   Other sizes record 1 and continue with the native aspect function. */
.global framing_classify
framing_classify:
    push {r0-r3,lr}
    bl wide_size
    pop {r0-r3,lr}
    ldr r2, =GRKIT_HEADER_REFERENCE
    cmp r12, #0
    blt 5f
    mov r3, r12, lsr #2
    add r3, r3, #2
    strb r3, [r2, #GRKIT_HOST_FRAME_FILE]
    and r12, r12, #3
    cmp CLASSIFY_CLASS, #0
    strbne r12, [CLASSIFY_CLASS]
    mov r0, #1
    CLASSIFY_RETURN
5:  mov r3, #1
    strb r3, [r2, #GRKIT_HOST_FRAME_FILE]
    mov r0, CLASSIFY_WIDTH
    mov r1, r4
    ldr pc, =HOOK_FRAMING_CLASSIFY_TARGET
/* Replaces the tail branch of the image aspect getter (GR III 0x532b3ca0,
   r0 = width, r1 = height): 65:24 and 2.35:1 sizes report aspect id 2, and
   5:4 and 7:6 sizes aspect id 1. */
.global framing_image_aspect
framing_image_aspect:
    push {r0-r3,lr}
    bl wide_size
    pop {r0-r3,lr}
    cmp r12, #0
    blt 6f
    cmp r12, #8
    movlo r0, #2
    movhs r0, #1
    bx lr
6:  ldr pc, =HOOK_FRAMING_IMAGE_ASPECT_TARGET
#endif
#ifdef HOOK_FRAMING_LEVEL_THRESHOLD
/* ScreenDevice::OnSetLevelAlarm (runtime/framing.c: highlight warning).
   This replaces `mov r5, #0xfe00`: r5 gets the threshold of the effect. The
   next call reads r0 to r2. */
.global framing_level_threshold
framing_level_threshold:
    push {r0-r3,r12,lr}
    bl framing_level_value
    mov r5, r0
    pop {r0-r3,r12,pc}
/* This replaces `mov r1, r6` before the tail call. r4 is the screen and r6
   the mode, 0 for off. */
.global framing_level_mode
framing_level_mode:
    push {r0-r3,r12,lr}
    mov r0, r4
    mov r1, r6
    bl framing_level_record
    pop {r0-r3,r12,lr}
    mov r1, r6
    bx lr
#endif
#ifdef HOOK_FRAMING_HISTOGRAM
/* LiveViewController's live-view histogram update (runtime/level.h). This
   replaces the BL to the copy of the 256 luma bins into the controller's 128
   bins at r2. The caller then marks the bins as new. */
.global framing_histogram
framing_histogram:
    push {r2,lr}
    ldr r12, =HOOK_FRAMING_HISTOGRAM_TARGET
    blx r12
    pop {r0,lr}
    b framing_histogram_remap
#endif
.ltorg
