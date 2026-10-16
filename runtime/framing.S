#include "abi.h"
.syntax unified
.arm
.text
/* The installer relocates this literal to the private host header. */
.global framing_header
framing_header:
    ldr r0, =0x58012340
    bx lr
/* Replaces `ldrb r1, [r4]` in the still aspect property handler. r1 returns
   the capture request. The next native call reads r0. */
.global framing_capture_request_hook
framing_capture_request_hook:
    push {r0-r3,r12,lr}
    ldrb r0, [r4]
    bl framing_capture_request
    str r0, [sp,#4]
    pop {r0-r3,r12,pc}
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
ENTRY framing_update, framing_bind, 0x53176820
ENTRY framing_exit, framing_leave, 0x53176de8
ENTRY framing_destroy, framing_leave, 0x53170b54
ENTRY framing_canvas, framing_prepare, 0x53391b18
/* Replaces the Canvas::Draw BL after its native full-background clear. */
.global framing_children
framing_children:
    push {r0-r3,r12,lr}
    bl framing_paint
    pop {r0-r3,r12,lr}
    ldr pc,=0x53393d28
/* 65:24 shares capture aspect id 2 with 16:9. These hooks switch the id 2
   geometry: shooting follows the Framing ratio, playback follows the size of
   the photo that the classifier saw last. */
/* r12 = 1 when id 2 means 65:24. Clobbers only r12 and the flags. */
xpan_active:
    ldr r12, =0x58012340
    ldrb r12, [r12, #GRKIT_HOST_FRAME_FILE]
    cmp r12, #0
    beq 1f
    cmp r12, #2
    moveq r12, #1
    movne r12, #0
    bx lr
1:  ldr r12, =0x58012340
    ldr r12, [r12, #GRKIT_HOST_FRAME_RATIO]
    cmp r12, #1
    moveq r12, #1
    movne r12, #0
    bx lr
/* GetResolution id 2 heights replace `mov r3, #height` before a store and an
   unconditional branch to the epilogue, so r12 and the flags are dead. */
.macro HEIGHT name, stock, xpan
.global \name
\name:
    push {lr}
    bl xpan_active
    pop {lr}
    movw r3, #\stock
    cmp r12, #0
    movwne r3, #\xpan
    bx lr
.endm
HEIGHT framing_height_0, 3376, 2216
HEIGHT framing_height_1, 2704, 1776
HEIGHT framing_height_2, 1888, 1240
HEIGHT framing_height_3, 1080, 712
/* Top insets of the 720x480 screennail and 160x120 thumbnail. */
.macro INSET name, stock, xpan
.global \name
\name:
    push {lr}
    bl xpan_active
    pop {lr}
    mov r1, #\stock
    cmp r12, #0
    movne r1, #\xpan
    bx lr
.endm
INSET framing_screennail_inset, 38, 107
INSET framing_thumbnail_inset, 16, 30
/* r0 = width, r1 = height. r12 = 65:24 size class 0..3, or -1.
   Clobbers r0-r3, r12 and the flags. */
xpan_size:
    cmp r0, r1
    movhs r2, r0
    movhs r3, r1
    movlo r2, r1
    movlo r3, r0
    adr r12, xpan_sizes
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
xpan_sizes:
    .hword 6000, 2216, 4800, 1776, 3360, 1240, 1920, 712, 0, 0
/* Replaces the aspect BL in the size classifier 0x532b3988 (frame
   {r4-r7,fp,ip,lr,pc}; r7 = width, r4 = height, r6 = class pointer).
   A 65:24 size returns class and success from the classifier. Other sizes
   record "not 65:24" and continue with the native aspect function. */
.global framing_classify
framing_classify:
    push {r0-r3,lr}
    bl xpan_size
    pop {r0-r3,lr}
    ldr r2, =0x58012340
    cmp r12, #0
    blt 5f
    mov r3, #2
    strb r3, [r2, #GRKIT_HOST_FRAME_FILE]
    cmp r6, #0
    strbne r12, [r6]
    mov r0, #1
    ldm sp, {r4,r5,r6,r7,fp,sp,pc}
5:  mov r3, #1
    strb r3, [r2, #GRKIT_HOST_FRAME_FILE]
    mov r0, r7
    mov r1, r4
    ldr pc, =0x532b29f4
/* Replaces the tail branch of the image aspect getter 0x532b3ca0
   (r0 = width, r1 = height): 65:24 sizes report aspect id 2. */
.global framing_image_aspect
framing_image_aspect:
    push {r0-r3,lr}
    bl xpan_size
    pop {r0-r3,lr}
    cmp r12, #0
    movge r0, #2
    bxge lr
    ldr pc, =0x532b29f4
.ltorg
