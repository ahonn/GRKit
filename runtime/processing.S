// Private Image Control processing hooks for GR III 2.10. processing.h has
// the parts that each family shares. The header fields are those of abi.h.
// The firmware addresses are FIRMWARE() symbols (hooks.h and
// runtime/firmware.h), which the installer relocates.
// Records are immutable, contiguous GRKIT_RECORD_SIZE allocations retained until reboot.
// The live-view dirty hook lives in picture.S.
#include "firmware.h"
#include "abi.h"
#include "hooks.h"
.syntax unified
.arm
.text

// The context of the parameter build is r5, its transient object r6.
#define PROCESSING_CONTEXT r5
#define PROCESSING_OBJECT r6
#include "processing.h"

module_processing:
    push {r0-r3, r12, lr}
    ldr r0, [r4, #8]
    .word 0xeb000000 | (((.Lowned_record - . - 8) >> 2) & 0x00ffffff)
    bne processing_stock
    pop {r0-r3, r12, lr}
    mov r1, #11
    mov r2, #11
    ldr r12, =FIRMWARE(0x55018cf0)
    add r9, r6, #492
    ldr lr, [r4, #8]
    // A record with hue sectors (feature 8) or a native look (feature 32), 0x28,
    // gives its own MultiAxial set: the native load reads [r12 + 11 * 4], so
    // point r12 44 bytes before the word at
    // RECORD_MULTIAXIAL that holds the set address. With highlight correction on
    // this capture (the native flag at [fp - 0x30], which the function stored
    // before this hook), point it at the next word, the address of the set for
    // half the stage input.
    ldr r0, [lr, #GRKIT_RECORD_FEATURES]
    tst r0, #0x28
.if GRKIT_RECORD_MULTIAXIAL - 44 != 0x714
.error "Update the MultiAxial set offset"
.endif
    beq 1f
    add r12, lr, #0x700
    add r12, r12, #0x14
    ldr r0, [fp, #-0x30]
    cmp r0, #0
    addne r12, r12, #4
1:
    ldr r0, =FIRMWARE(0x55018e4c)
    ldr pc, =FIRMWARE(0x536dc2ac)
processing_stock:
    pop {r0-r3, r12, lr}
    // The eight replaced instructions (a context of target/gr3_210.rs), with
    // literal words for the three type tables instead of MOVW and MOVT pairs.
    ldr r0, =FIRMWARE(0x55018d94)
    ldr r12, =FIRMWARE(0x55018cf0)
    add r9, r6, #0x1ec
    ldr lr, [r0, r1, lsl #2]
    ldr r0, =FIRMWARE(0x55018e4c)
    ldr pc, =FIRMWARE(0x536dc2ac)

module_packet:
    str r3, [r4, #0x90]
    push {r0-r3, r12, lr}
    mrs r12, cpsr
    push {r12, lr}
    ldr r0, [r4, #8]
    .word 0xeb000000 | (((.Lowned_record - . - 8) >> 2) & 0x00ffffff)
    bne packet_stock
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
    ldr r12, =FIRMWARE(0x536f00c4)
    blx r12
    ldr r12, =FIRMWARE(0x536eff34)
    blx r12
    sub sp, r11, #40
    ldm sp, {r4-r11, sp, pc}
packet_stock:
    pop {r12, lr}
    msr cpsr_f, r12
    pop {r0-r3, r12, lr}
    ldr pc, =HOOK_MODULE_PACKET_RESUME

module_gamma_flag:
    push {r0-r2, lr}
    ldr r0, [r4, #8]
    .word 0xeb000000 | (((.Lowned_record - . - 8) >> 2) & 0x00ffffff)
    ldrbne r3, [r4, #0x9b]
    moveq r3, #1
    pop {r0-r2, lr}
    ldr pc, =HOOK_MODULE_GAMMA_FLAG_RESUME

// Replaces the BL to the curve getter at 0x536e87c4 (r4 = packet). r1..r3
// receive the R, G, and B curves. The gamma object is at [fp, #-0x44].
module_gamma_curves:
    push {r0, r5, r12, lr}
    ldr r0, [r4, #8]
    .word 0xeb000000 | (((.Lowned_record - . - 8) >> 2) & 0x00ffffff)
    bne curves_stock
    add r5, r0, #GRKIT_RECORD_CURVES
    str r5, [r1]
    add r5, r5, #GRKIT_RECORD_CURVE_SIZE
    str r5, [r2]
    add r5, r5, #GRKIT_RECORD_CURVE_SIZE
    str r5, [r3]
    mov r1, r0
    ldr r0, [fp, #-0x44]
    mov r2, r4
    .word 0xeb000000 | (((.Lgamma_cached - . - 8) >> 2) & 0x00ffffff)
    popne {r0, r5, r12, pc}
    // The object holds these curves: no table build, and return from the
    // gamma stage as its epilogue does.
    add r0, r0, #(GAMMA_UPLOAD & ~0xff)
    mov r1, #0
    strb r1, [r0, #(GAMMA_UPLOAD & 0xff)]
    sub sp, fp, #40
    ldm sp, {r4-r11, sp, pc}
curves_stock:
    .word 0xeb000000 | (((.Lgamma_forget - . - 8) >> 2) & 0x00ffffff)
    pop {r0, r5, r12, lr}
    ldr pc, =HOOK_MODULE_GAMMA_CURVES_TARGET
.ltorg
