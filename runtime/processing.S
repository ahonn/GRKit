// Private Image Control processing hooks for the pinned GR III 2.10 image.
// HEADER +0..12: count, record base, active record, last applied record.
// HEADER +24/+28: last processing context flags and normalization call count.
// Records are immutable, contiguous GRKIT_RECORD_SIZE allocations retained until reboot.
// The live-view dirty hook lives in picture.S: this region has no room left.
#include "abi.h"
.syntax unified
.arm
.text

// Return Z=1 only for an owned record pointer. Preserve all registers.
.Lowned_record:
    push {r0-r3, lr}
    ldr r1, =HEADER
    ldr r2, [r1]
    ldr r1, [r1, #4]
    cmp r2, #0
    beq owned_no
owned_loop:
    cmp r0, r1
    beq owned_done
    add r1, r1, #GRKIT_RECORD_SIZE
    subs r2, r2, #1
    bne owned_loop
owned_no:
    movs r3, #1
owned_done:
    pop {r0-r3, pc}

// Replaces the BL to the native parameter normalizer, not its shared entry.
// Snapshot activation before constructing this task's packet. Unsupported
// contexts use the native normalizer and cannot accidentally inherit a plugin.
module_parameters:
    push {r0-r3, r12, lr}
    ldr r0, =HEADER
    ldr r2, [r5, #0x2ac]
    str r2, [r0, #24]
    ldr r3, [r0, #28]
    add r3, r3, #1
    str r3, [r0, #28]
    ldr r0, [r0, #8]
    // Context 1 is observed on ImageTuningLiveview; 0 is the still path.
    ldrb r2, [r5, #0x2ac]
    cmp r2, #1
    bhi parameters_stock
    // Menu previews change only the live view. Still captures keep the committed record.
    bne parameters_committed
    ldr r3, =HEADER
    ldr r3, [r3, #GRKIT_HOST_PREVIEW]
    cmp r3, #0
    movne r0, r3
parameters_committed:
    ldrb r2, [r5, #0x2ad]
    cmp r2, #0
    bne parameters_stock
    ldrb r2, [r5, #0x2b2]
    cmp r2, #0
    bne parameters_stock
    .word 0xeb000000 | (((.Lowned_record - . - 8) >> 2) & 0x00ffffff)
    bne parameters_stock
    str r0, [r4, #8]
    // Capture metadata records the effect that a still build applied.
    ldrb r2, [r5, #0x2ac]
    cmp r2, #0
    ldreq r1, =HEADER
    streq r0, [r1, #GRKIT_HOST_STILL]
    // Fill the transient object (r6) like the native Standard branch, from the
    // live settings that the caller computed in r7. Unlike that branch, never
    // write the settings back, so other native types keep their own fields.
    add r1, r6, #0x1e0
    add r0, r6, #0x200
    add r3, r7, #0x700
    adr r12, parameters_sources
parameters_fill:
    ldrb r2, [r12], #1
    cmp r2, #1
    ldrbhi r2, [r3, r2]
    strh r2, [r1], #2
    cmp r1, r0
    bne parameters_fill
    pop {r0-r3, r12, pc}
// Source per halfword at +0x1e0: settings offset - 0x700, or the constant 0 or 1.
// Saturation, hue, key, contrast, highlight, shadow, 1, sharpness, 0, 0,
// shading, clarity, 0, 0, 0, 0.
parameters_sources:
    .byte 0x7d, 0x7e, 0x7f, 0x80, 0x81, 0x82, 1, 0x84, 0, 0, 0x93, 0x94, 0, 0, 0, 0
parameters_stock:
    mov r0, #0
    str r0, [r4, #8]
    // Other non-live-view builds, such as in-camera RAW development, record no effect.
    ldrb r2, [r5, #0x2ac]
    cmp r2, #1
    ldrne r1, =HEADER
    strne r0, [r1, #GRKIT_HOST_STILL]
    pop {r0-r3, r12, lr}
    ldr pc, =0x536db7d8

module_processing:
    push {r0-r3, r12, lr}
    ldr r0, [r4, #8]
    .word 0xeb000000 | (((.Lowned_record - . - 8) >> 2) & 0x00ffffff)
    bne processing_stock
    pop {r0-r3, r12, lr}
    mov r1, #11
    mov r2, #11
    ldr r12, =0x55018cf0
    add r9, r6, #492
    ldr lr, [r4, #8]
    ldr r0, =0x55018e4c
    ldr pc, =0x536dc2ac
processing_stock:
    pop {r0-r3, r12, lr}
    @ ORIGINAL_PROCESSING
    ldr pc, =0x536dc2ac

module_packet:
    str r3, [r4, #0x90]
    push {r0-r3, r12, lr}
    mrs r12, cpsr
    push {r12, lr}
    ldr r0, [r4, #8]
    .word 0xeb000000 | (((.Lowned_record - . - 8) >> 2) & 0x00ffffff)
    bne packet_stock
    ldr r1, =HEADER
    str r0, [r1, #12]
    ldr r1, [r0, #GRKIT_RECORD_MONOCHROME]
    and r1, r1, #1
    strb r1, [r4, #0x79]
    mov r2, #0
    strb r2, [r4, #0x7a]
    strb r2, [r4, #0x55]
    strb r2, [r4, #0x56]
    cmp r1, #0
    movne r1, #0x2000
    strhne r1, [r4, #0x7c]
    strhne r2, [r4, #0x7e]
    strhne r2, [r4, #0x80]
    // Native tail resets the shared gamma helper, then enables the special
    // type-16 path. Only the reset belongs to a Standard-based private packet.
    pop {r12, lr}
    msr cpsr_f, r12
    pop {r0-r3, r12, lr}
    ldr r12, =0x536f00c4
    blx r12
    ldr r12, =0x536eff34
    blx r12
    sub sp, r11, #40
    ldm sp, {r4-r11, sp, pc}
packet_stock:
    pop {r12, lr}
    msr cpsr_f, r12
    pop {r0-r3, r12, lr}
    ldr pc, =0x536dc690

module_gamma_flag:
    push {r0-r2, lr}
    ldr r0, [r4, #8]
    .word 0xeb000000 | (((.Lowned_record - . - 8) >> 2) & 0x00ffffff)
    ldrbne r3, [r4, #0x9b]
    moveq r3, #1
    pop {r0-r2, lr}
    ldr pc, =0x536e8698

module_gamma_curves:
    push {r0, r5, r12, lr}
    ldr r0, [r4, #8]
    .word 0xeb000000 | (((.Lowned_record - . - 8) >> 2) & 0x00ffffff)
    bne curves_stock
    add r5, r0, #0x68
    str r5, [r1]
    add r5, r5, #0x200
    str r5, [r2]
    add r5, r5, #0x200
    str r5, [r3]
    pop {r0, r5, r12, pc}
curves_stock:
    pop {r0, r5, r12, lr}
    ldr pc, =0x536eedf0
.ltorg
