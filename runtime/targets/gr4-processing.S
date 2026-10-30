// Private Image Control processing hooks for each GR IV build
// (runtime/processing.S describes the GR III 2.10 version, and processing.h
// has the parts that each family shares). The build header
// gives the frame slot of the type table index (PROCESSING_INDEX_SLOT). The
// comments name the sites of 1.11; crates/grkit-build/src/target has the sites
// of each build.
// The build places this code after the RTOS image. Local calls use BL words. The firmware addresses
// are FIRMWARE() symbols (hooks.h), which the installer relocates.
// The header fields are those of abi.h.
#include "firmware.h"
#include "abi.h"
#include "hooks.h"
.syntax unified
.arm
.text

// The parameter build (1.11: the BL to the normalizer 0x539113d8 at
// 0x53912c70): sb = context (+0x8000), r4 = packet, r5 = transient object,
// r7 = settings. The native Standard branch (1.11 0x53911614) writes one byte
// at +0x1f8, so the fill keeps the byte after it.
#define PROCESSING_CONTEXT sb
#define PROCESSING_OBJECT r5
#define PROCESSING_KEPT_BYTE 0x1f9
#include "processing.h"

// Replaces `mov r1, sl` (1.11 0x53912d40), where every type path joins before the
// type-table getters. sl is the table index. A private record uses the
// Standard index 11 for the other tables, as GR III does.
module_processing:
    push {r0, lr}
    ldr r0, [r4, #8]
    .word 0xeb000000 | (((.Lowned_record - . - 8) >> 2) & 0x00ffffff)
    moveq sl, #11
    streq sl, [fp, #-PROCESSING_INDEX_SLOT]
    mov r1, sl
    pop {r0, pc}

// Replaces `str r0, [r4, #8]` (1.11 0x53912d4c), the record from the first getter.
// A private record set by module_parameters stays.
module_record:
    push {r0, lr}
    ldr r0, [r4, #8]
    .word 0xeb000000 | (((.Lowned_record - . - 8) >> 2) & 0x00ffffff)
    pop {r0, lr}
    strne r0, [r4, #8]
    bx lr

// Replaces `str r0, [r4, #0xc]` (1.11 0x53912d5c), the MultiAxial set from the
// getter (1.11 0x53925fb0), whose table address is fixed in the getter. A private
// record with hue sectors (feature 8) or a native look (feature 32), 0x28, gives
// its own set at RECORD_MULTIAXIAL + 8, or, with highlight correction on this
// capture (the native flag in the frame slot PROCESSING_HIGHLIGHT_SLOT), the set
// whose address is at RECORD_MULTIAXIAL + 4. Other records keep the set of the Standard index. r0 is
// free after this store.
.if GRKIT_RECORD_MULTIAXIAL != 0x740
.error "Update the MultiAxial set offset"
.endif
module_multiaxial:
    push {r1, r2, lr}
    mov r2, r0
    ldr r0, [r4, #8]
    .word 0xeb000000 | (((.Lowned_record - . - 8) >> 2) & 0x00ffffff)
    bne multiaxial_store
    ldr r1, [r0, #GRKIT_RECORD_FEATURES]
    tst r1, #0x28
    beq multiaxial_store
    add r2, r0, #0x740
    add r2, r2, #8
    ldr r1, [fp, #-PROCESSING_HIGHLIGHT_SLOT]
    cmp r1, #0
    ldrne r2, [r0, #0x744]
multiaxial_store:
    str r2, [r4, #0xc]
    pop {r1, r2, pc}

// Replaces `str r3, [r4, #0x90]` (1.11 0x53913188), before the native packet tail.
// The GR IV tail has no special type path, so it continues unchanged.
module_packet:
    str r3, [r4, #0x90]
    push {r0-r3, lr}
    ldr r0, [r4, #8]
    .word 0xeb000000 | (((.Lowned_record - . - 8) >> 2) & 0x00ffffff)
    bne packet_done
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
packet_done:
    pop {r0-r3, pc}

// Replaces `ldrb r1, [r1, #0x9b]` (1.11 0x5391e360, r1 = packet): a private
// record takes the custom-curve path.
module_gamma_flag:
    push {r0, lr}
    ldr r0, [r1, #8]
    .word 0xeb000000 | (((.Lowned_record - . - 8) >> 2) & 0x00ffffff)
    ldrbne r1, [r1, #0x9b]
    moveq r1, #1
    pop {r0, pc}

// Replaces the BL to the curve getter (1.11: 0x53925000 at 0x5391e76c). r1..r3 receive
// the R, G, and B curves. The packet is at [fp, #-0x54], the gamma object at
// [fp, #-0x50].
module_gamma_curves:
    push {r0, r5, r12, lr}
    ldr r0, [fp, #-0x54]
    ldr r0, [r0, #8]
    .word 0xeb000000 | (((.Lowned_record - . - 8) >> 2) & 0x00ffffff)
    bne curves_stock
    add r5, r0, #GRKIT_RECORD_CURVES
    str r5, [r1]
    add r5, r5, #GRKIT_RECORD_CURVE_SIZE
    str r5, [r2]
    add r5, r5, #GRKIT_RECORD_CURVE_SIZE
    str r5, [r3]
    mov r1, r0
    ldr r0, [fp, #-0x50]
    ldr r2, [fp, #-0x54]
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
