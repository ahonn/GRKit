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
.type framing_header, %function
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
/* Replaces the Canvas::Draw BL after its native full-background clear. The
   private layers draw before the native children, and the crop badge after
   them, so that it covers the camera's own (runtime/crop.h). */
.global framing_children
framing_children:
    push {r0-r3,r12,lr}
    bl framing_paint
    ldmia sp, {r0-r3}
    ldr r12,=HOOK_FRAMING_CHILDREN_TARGET
    blx r12
    ldmia sp, {r0-r3}
    bl framing_paint_after
    pop {r0-r3,r12,lr}
    bx lr
/* Replaces the tail call of the live-view grid update to SetVisible of the
   camera's grid view (r0), shown when r1 is not 0. While the live view
   shows module layers, such as a frame or the guides (framing_grid_hidden),
   the grid hides; the camera's setting stays. The camera updates the grid at each refresh of the live-view
   information. */
.global framing_grid
.type framing_grid, %function
framing_grid:
    cmp r1, #0
    beq 1f
    push {r0-r3,r12,lr}
    bl framing_grid_hidden
    cmp r0, #0
    pop {r0-r3,r12,lr}
    movne r1, #0
1:  ldr pc, =HOOK_FRAMING_GRID_TARGET
.ltorg
#ifdef HOOK_FRAMING_CLASSIFY
/* The Framing geometries of the hidden capture aspect ids (runtime/
   framing_ratios.h, from grkit_core::framing): id 2 (16:9) with the heights
   of the wide ratios, such as XPan, and id 1 (4:3) with the widths of the
   narrow ones, such as 5:4 and 3:4, and with the native 1:1 sizes. Each id
   has modes: 0 for its stock sizes, then one for each geometry. These hooks
   switch the GetResolution sizes and the preview insets to the mode:
   shooting follows the Framing ratio, playback the size of the photo that
   the classifier saw last. The family header gives the classifier registers
   at the aspect BL (the width, the class pointer, and the return of its
   frame). */
#if !defined(FRAMING_MODE_INDEXES) || !defined(FRAMING_SIZES)
#error "The family header includes runtime/framing_ratios.h"
#endif
#if !defined(CLASSIFY_WIDTH) || !defined(CLASSIFY_CLASS) || !defined(CLASSIFY_RETURN)
#error "The family header defines the classifier registers"
#endif
/* A table of `count` entries of `size` bytes ends here. */
.macro TABLE_END start, count, size
.if (. - \start) != (\count) * (\size)
.error "A Framing table needs an entry for each mode"
.endif
.balign 4
.endm
/* The mode in r12: GRKIT_HOST_FRAME_FILE is 0 while shooting, else the
   geometry of the photo that the classifier saw last. Shooting follows
   GRKIT_HOST_FRAME_CROP, the crop that applies in the native aspect
   (runtime/framing.c), so a native aspect keeps its stock sizes. Clobbers
   only r12 and the flags. */
.macro MODE name, by_file, by_crop
\name:
    push {r0}
    ldr r0, =GRKIT_HEADER_REFERENCE
    ldrb r12, [r0, #GRKIT_HOST_FRAME_FILE]
    cmp r12, #0
    ldreq r12, [r0, #GRKIT_HOST_FRAME_CROP]
    adrne r0, \by_file
    adreq r0, \by_crop
    cmp r12, #FRAMING_MODE_INDEXES
    ldrblo r12, [r0, r12]
    movhs r12, #0
    pop {r0}
    bx lr
.endm
MODE wide_mode, wide_by_file, wide_by_crop
wide_by_file:
    .byte FRAMING_WIDE_BY_FILE
TABLE_END wide_by_file, FRAMING_MODE_INDEXES, 1
wide_by_crop:
    .byte FRAMING_WIDE_BY_CROP
TABLE_END wide_by_crop, FRAMING_MODE_INDEXES, 1
/* `out` = the halfword of the mode in the table that follows. Each hook
   replaces an instruction that sets `out` to a size or an inset, so r12 and
   the flags are dead. */
.macro LOOKUP name, mode, out
.global \name
\name:
    push {lr}
    bl \mode
    pop {lr}
    adr \out, \name\()_modes
    add \out, \out, r12, lsl #1
    ldrh \out, [\out]
    bx lr
\name\()_modes:
.endm
/* GetResolution id 2 heights replace `mov r3, #height` before a store and an
   unconditional branch to the epilogue. */
LOOKUP framing_height_0, wide_mode, r3
    .hword FRAMING_WIDE_HEIGHT_0
TABLE_END framing_height_0_modes, FRAMING_WIDE_MODES, 2
LOOKUP framing_height_1, wide_mode, r3
    .hword FRAMING_WIDE_HEIGHT_1
TABLE_END framing_height_1_modes, FRAMING_WIDE_MODES, 2
LOOKUP framing_height_2, wide_mode, r3
    .hword FRAMING_WIDE_HEIGHT_2
TABLE_END framing_height_2_modes, FRAMING_WIDE_MODES, 2
LOOKUP framing_height_3, wide_mode, r3
    .hword FRAMING_WIDE_HEIGHT_3
TABLE_END framing_height_3_modes, FRAMING_WIDE_MODES, 2
/* Top insets (r1) of the 720x480 screennail and 160x120 thumbnail. */
LOOKUP framing_screennail_inset, wide_mode, r1
    .hword FRAMING_WIDE_SCREENNAIL
TABLE_END framing_screennail_inset_modes, FRAMING_WIDE_MODES, 2
LOOKUP framing_thumbnail_inset, wide_mode, r1
    .hword FRAMING_WIDE_THUMBNAIL
TABLE_END framing_thumbnail_inset_modes, FRAMING_WIDE_MODES, 2
#ifdef HOOK_FRAMING_WIDTH_0
MODE narrow_mode, narrow_by_file, narrow_by_crop
narrow_by_file:
    .byte FRAMING_NARROW_BY_FILE
TABLE_END narrow_by_file, FRAMING_MODE_INDEXES, 1
narrow_by_crop:
    .byte FRAMING_NARROW_BY_CROP
TABLE_END narrow_by_crop, FRAMING_MODE_INDEXES, 1
/* GetResolution id 1 widths replace `mov r2, #width` before the height, two
   stores, and a branch to the epilogue. The pipeline crops the width from
   the center. */
LOOKUP framing_width_0, narrow_mode, r2
    .hword FRAMING_NARROW_WIDTH_0
TABLE_END framing_width_0_modes, FRAMING_NARROW_MODES, 2
LOOKUP framing_width_1, narrow_mode, r2
    .hword FRAMING_NARROW_WIDTH_1
TABLE_END framing_width_1_modes, FRAMING_NARROW_MODES, 2
LOOKUP framing_width_2, narrow_mode, r2
    .hword FRAMING_NARROW_WIDTH_2
TABLE_END framing_width_2_modes, FRAMING_NARROW_MODES, 2
LOOKUP framing_width_3, narrow_mode, r2
    .hword FRAMING_NARROW_WIDTH_3
TABLE_END framing_width_3_modes, FRAMING_NARROW_MODES, 2
/* The id 1 XS height replaces `mov r3, #height` after the XS width. The
   native 1:1 XS is smaller than the 4:3 XS height. */
LOOKUP framing_narrow_height_3, narrow_mode, r3
    .hword FRAMING_NARROW_HEIGHT_3
TABLE_END framing_narrow_height_3_modes, FRAMING_NARROW_MODES, 2
/* The id 1 left inset of the 720x480 screennail replaces `mov r0, #40`. */
LOOKUP framing_narrow_screennail, narrow_mode, r0
    .hword FRAMING_NARROW_SCREENNAIL
TABLE_END framing_narrow_screennail_modes, FRAMING_NARROW_MODES, 2
/* The id 1 thumbnail inset: the stock code stores 0 (r1) at the left inset
   [r2], then at the top inset [r3]. This replaces the second store and
   writes the left inset of the 160x120 thumbnail again. The IIP stores a
   picture with chroma pairs only from an even column, so each inset is
   even (grkit_core::framing). */
.global framing_narrow_thumbnail
framing_narrow_thumbnail:
    push {r0,lr}
    bl narrow_mode
    adr r0, framing_narrow_thumbnail_modes
    add r0, r0, r12, lsl #1
    ldrh r0, [r0]
    strh r0, [r2]
    strh r1, [r3]
    pop {r0,pc}
framing_narrow_thumbnail_modes:
    .hword FRAMING_NARROW_THUMBNAIL
TABLE_END framing_narrow_thumbnail_modes, FRAMING_NARROW_MODES, 2
#endif
/* Replaces the aspect BL in the size classifier (GR III 0x532b3988: frame
   {r4-r7,fp,ip,lr,pc}, r7 = width, r4 = height, r6 = class pointer). Each
   size records its geometry (framing_size_file, runtime/framing.c) in
   GRKIT_HOST_FRAME_FILE. A Framing size then returns its size class and
   success from the classifier. A crop of a ratio, which has no size class,
   and another size continue with the native aspect function. */
.global framing_classify
framing_classify:
    push {r0-r4,lr}
    bl framing_size_file
    mov r12, r0
    pop {r0-r4,lr}
    ldr r2, =GRKIT_HEADER_REFERENCE
    strb r12, [r2, #GRKIT_HOST_FRAME_FILE]
    lsrs r12, r12, #8
    beq 5f
    sub r12, r12, #1
    cmp CLASSIFY_CLASS, #0
    strbne r12, [CLASSIFY_CLASS]
    mov r0, #1
    CLASSIFY_RETURN
5:  mov r0, CLASSIFY_WIDTH
    mov r1, r4
    ldr pc, =HOOK_FRAMING_CLASSIFY_TARGET
/* Replaces the tail branch of the image aspect getter (GR III 0x532b3ca0,
   r0 = width, r1 = height): a Framing size and a crop of a ratio report
   the capture aspect id of their geometry (framing_size_capture). */
.global framing_image_aspect
framing_image_aspect:
    push {r0-r3,r12,lr}
    bl framing_size_capture
    str r0, [sp, #16]
    pop {r0-r3,r12,lr}
    cmp r12, #0
    movne r0, r12
    bxne lr
    ldr pc, =HOOK_FRAMING_IMAGE_ASPECT_TARGET
#ifdef HOOK_FRAMING_DEVELOP_ASPECT
#if !defined(DEVELOP_MODEL) || !defined(DEVELOP_ASPECT)
#error "The family header defines the RAW development registers"
#endif
/* RAW development of a DNG (runtime/framing.c). This replaces the BL that
   gives the initial development aspect for the recorded capture aspect.
   framing_develop_select returns the development aspect, or -1 for the
   native choice. */
.global framing_develop_aspect
framing_develop_aspect:
    mov r0, DEVELOP_MODEL
    mov r1, DEVELOP_ASPECT
    push {r0,r1,r4,lr}
    bl framing_develop_select
    cmn r0, #1
    popne {r1,r2,r4,pc}
    pop {r0,r1,r4,lr}
    ldr pc, =HOOK_FRAMING_DEVELOP_ASPECT_TARGET
/* These replace the BLs that turn the development aspect (r1, model r0) into
   a capture aspect: where the development sends it to the pipeline, and
   where it gives the size of the developed photo. framing_develop_geometry
   sets the geometry, and framing_develop_capture gives the capture aspect of
   a Framing ratio. The GetResolution hooks above then give the size of the
   developed photo. */
.macro DEVELOP_CAPTURE name, target
.global \name
\name:
    push {r0,r1,r4,lr}
    bl framing_develop_geometry
    pop {r0,r1}
    ldr r12, =\target
    blx r12
    bl framing_develop_capture
    pop {r4,pc}
.endm
DEVELOP_CAPTURE framing_develop_push, HOOK_FRAMING_DEVELOP_PUSH_TARGET
DEVELOP_CAPTURE framing_develop_size, HOOK_FRAMING_DEVELOP_SIZE_TARGET
.ltorg
#endif
#ifdef HOOK_FRAMING_RESIZE_SOURCE
#ifndef RESIZE_SIZE
#error "The family header names the register of the resize source size"
#endif
/* Resize of the photo in playback (runtime/framing.c). This replaces
   `mov r0, RESIZE_SIZE` at the end of the getter of the size {width, height}
   that the camera looks up in its resize size table. */
.global framing_resize_source
framing_resize_source:
    push {r0-r3,r12,lr}
    mov r0, RESIZE_SIZE
    bl framing_resize_stock
    pop {r0-r3,r12,lr}
    mov r0, RESIZE_SIZE
    bx lr
#endif
#ifdef HOOK_FRAMING_PLAY_ASPECT
/* This replaces the first instruction (`mov ip, sp`) of the function that
   gives the aspect of an image size {width, height} (r0) in playback edits.
   It runs the function, then framing_resize_play_aspect corrects the result
   for a Framing size. */
.global framing_play_aspect
framing_play_aspect:
    push {r0,lr}
    adr lr, 1f
    mov ip, sp
    ldr pc, =HOOK_FRAMING_PLAY_ASPECT_RESUME
1:  pop {r1,lr}
    b framing_resize_play_aspect
#endif
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
#include "crop.S"
#include "crop_queue.S"
#ifdef HOOK_CROP_RESOLUTION
.fpu vfpv3
.arm
/* GetResolution(translator r0, size r1, aspect r2, u16 *width r3,
   u16 *height at [sp]) with the crop factor in s0. Replaces `mov r12, sp`
   after its first instruction. The camera knows only its native factors.
   A crop of CORE.CROP (runtime/crop.h) takes the size of
   crop_resolution_size (runtime/framing.c), in each aspect and Framing
   geometry, else the size of the camera at the factor of its crop step.
   The C code uses no VFP register, so s0 stays. */
.global crop_resolution
.type crop_resolution, %function
crop_resolution:
    vmov r12, s0
    cmp r12, #0x3f800000
    beq 2f
    push {r0-r4,lr}
    mov r0, r1
    mov r1, r2
    mov r2, r12
    bl crop_resolution_size
    cmp r0, #0
    beq 1f
    ldr r1, [sp, #12]
    strh r0, [r1]
    lsr r0, r0, #16
    ldr r1, [sp, #24]
    strh r0, [r1]
    pop {r0-r4,lr}
    bx lr
    /* The factor of the crop step: that of 50 mm from it, else that of 35
       mm. Positive floats compare as their bits. */
1:  vmov r12, s0
    ldr r4, =CROP_NATIVE_50
    cmp r12, r4
    ldrlo r4, =CROP_NATIVE_35
    vmov s0, r4
    pop {r0-r4,lr}
2:  mov r12, sp
    ldr pc, =HOOK_CROP_RESOLUTION_RESUME
.ltorg
#endif
#ifdef HOOK_CROP_IMAGE_CROP
/* GetImageCrop with the crop factor in s0 (GR III): the step of the maps of
   the factor, 0, 1, or 2. Replaces `mov r12, sp` after its first
   instruction. It knows only the native factors; another factor of
   CORE.CROP takes the maps of its crop step. */
.global crop_image_crop
.type crop_image_crop, %function
crop_image_crop:
    push {r4,r5}
    vmov r4, s0
    ldr r5, =0x3f800000
    cmp r4, r5
    ldrne r5, =CROP_NATIVE_35
    cmpne r4, r5
    ldrne r5, =CROP_NATIVE_50
    cmpne r4, r5
    beq 1f
    ldr r5, =CROP_NATIVE_50
    vmov s1, r5
    vcmp.f32 s0, s1
    vmrs APSR_nzcv, fpscr
    movge r0, #2
    movlt r0, #1
    pop {r4,r5}
    bx lr
1:  pop {r4,r5}
    mov r12, sp
    ldr pc, =HOOK_CROP_IMAGE_CROP_RESUME
.ltorg
#endif
#ifdef HOOK_CROP_SHOT
/* After the copy of the capture settings of a shot into the shot request
   object (r4): the factor of a 40 or 70 mm crop replaces the factor of its
   crop step in that copy (crop_shot_copy, runtime/framing.c), so that the
   window, the size, the Exif focal length, and the DNG crop of the shot
   follow it. A RAW development makes its photo through the same copy, with
   the factor of the crop of its DNG. Then the displaced instruction. */
.global crop_shot
.type crop_shot, %function
crop_shot:
    push {r0-r3,r12,lr}
    add r0, r4, #(CROP_SHOT_COPY & ~0xff)
    ldr r0, [r0, #(CROP_SHOT_COPY & 0xff)]
    cmp r0, #0
    beq 1f
    add r0, r0, #(SHOT_CROP & ~0xff)
    add r0, r0, #(SHOT_CROP & 0xff)
    bl crop_shot_copy
1:  pop {r0-r3,r12,lr}
    CROP_SHOT_DISPLACED
    bx lr
.ltorg
#endif
#ifdef HOOK_CROP_LIVE_0
/* Replaces each BL to the live-view crop factor (s0) before the live view
   scales it by 8192 for its zoom: GRKIT_HOST_CROP_LIVE, when it is not 0,
   takes the place of the camera's factor. The sites (four on GR III, five
   on GR IV) call the same function. */
.fpu vfpv3
.arm
.global crop_live_0
.global crop_live_1
.global crop_live_2
.global crop_live_3
.global crop_live_4
.type crop_live_0, %function
crop_live_0:
crop_live_1:
crop_live_2:
crop_live_3:
crop_live_4:
    push {r4, lr}
    ldr r12, =HOOK_CROP_LIVE_0_TARGET
    blx r12
    ldr r3, =GRKIT_HEADER_REFERENCE
    ldr r3, [r3, #GRKIT_HOST_CROP_LIVE]
    cmp r3, #0
    vmovne s0, r3
    pop {r4, pc}
.ltorg
#endif
#ifdef HOOK_CROP_WINDOW
/* The live-view window of a zoom (r3, the factor times 8192), at the entry
   of the function that makes it, which `mov r12, sp` starts. A caller that
   gives the zoom of the camera's crop factor (GRKIT_HOST_CROP_CAMERA_ZOOM,
   within 2) gets the zoom of the live factor (GRKIT_HOST_CROP_LIVE_ZOOM)
   while the core has one. GRKIT_HOST_CROP_LIVE_SEEN keeps the zoom of the
   last window of a crop, up to CROP_ZOOM_MOST: a larger zoom, such as that
   of the focus magnifier, is not a crop. */
#define CROP_ZOOM_MOST 22938
.arm
.global crop_window
.type crop_window, %function
crop_window:
    push {r4, r5}
    ldr r4, =GRKIT_HEADER_REFERENCE
    ldr r5, [r4, #GRKIT_HOST_CROP_LIVE_ZOOM]
    cmp r5, #0
    beq 1f
    ldr r12, [r4, #GRKIT_HOST_CROP_CAMERA_ZOOM]
    subs r12, r3, r12
    rsbmi r12, r12, #0
    cmp r12, #2
    movls r3, r5
1:  movw r5, #CROP_ZOOM_MOST
    cmp r3, r5
    strls r3, [r4, #GRKIT_HOST_CROP_LIVE_SEEN]
    pop {r4, r5}
    mov r12, sp
    ldr pc, =HOOK_CROP_WINDOW_RESUME
.ltorg
#endif
#ifdef HOOK_CROP_BADGE
.fpu vfpv3
.arm
/* Replaces the BL to the crop badge icon of a crop factor (s0) in the
   live-view crop badge update. Where the core draws its own badge
   (runtime/crop.h), the icon is that of no crop, CROP_BADGE_OFF. */
.global crop_badge
.type crop_badge, %function
crop_badge:
    push {r0-r3,r12,lr}
    vpush {d0}
    bl crop_badge_native
    vpop {d0}
    cmp r0, #0
    pop {r0-r3,r12,lr}
    ldrne pc, =HOOK_CROP_BADGE_TARGET
    mov r0, #CROP_BADGE_OFF
    bx lr
.ltorg
#endif
