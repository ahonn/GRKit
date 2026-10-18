#include "abi.h"
#include "firmware.h"
#include "hooks.h"
.syntax unified
.arm
.text
/* Picture processing hooks for effect extensions. They link into the Framing
   region, because the processing region has no room left. */

#ifdef HOOK_MODULE_GAMMA_DIRTY
// Z=1 only for an owned record in r0. Preserves all registers.
picture_owned:
    push {r0-r3, lr}
    ldr r1, =0x58012340
    ldr r2, [r1, #GRKIT_HOST_COUNT]
    ldr r1, [r1, #GRKIT_HOST_RECORDS]
    cmp r2, #0
    beq 2f
1:  cmp r0, r1
    beq 3f
    add r1, r1, #GRKIT_RECORD_SIZE
    subs r2, r2, #1
    bne 1b
2:  movs r3, #1
3:  pop {r0-r3, pc}

// Replaces `ldrb r3, [r4, #0xe]` (the gamma stage flag) in the live-view diff.
.global module_gamma_dirty
module_gamma_dirty:
    push {r0-r2, lr}
    ldrb r3, [r4, #0xe]
    ldr r0, [r5, #8]
    bl picture_owned
    movne r0, #0
    // The native diff sets the stage flags from native inputs only. When the
    // live-view record changes, rebuild the curves now, and the color matrix
    // (+5, the monochrome weights) and the YCbCr matrix (+7, the post matrix)
    // on the next update: their stages have already run. This hook runs only
    // for ImageTuningLiveview.
    ldr r1, =0x58012340
    ldr r2, [r1, #GRKIT_HOST_LIVE_RECORD]
    str r0, [r1, #GRKIT_HOST_LIVE_RECORD]
    subs r2, r2, r0
    movne r2, #1
    strbne r2, [r4, #5]
    strbne r2, [r4, #7]
    // Private records and record changes always rebuild the curves.
    orrs r2, r2, r0
    bne dirty_on
    // Native curves must also be rebuilt when leaving a private record.
    ldrb r0, [r5, #0x9b]
    cmp r0, #0
    bne dirty_resume
    ldr r0, [r4, #0x17c]
    add r0, r0, #(GAMMA_BUILT & ~0xff)
    ldr r2, [r0, #(GAMMA_BUILT & 0xff)]
    cmp r2, #0
    bne dirty_resume
dirty_on:
    mov r3, #1
dirty_resume:
    pop {r0-r2, lr}
    ldr pc, =HOOK_MODULE_GAMMA_DIRTY_RESUME
#endif

/* Grain: the native grain stage of DigitalFilterProcess, which runs for
   recorded pictures only. SetEffectParameter reads the grain settings from the
   shot settings (r4 on GR IV, r5 on GR III); these hooks replace each load with
   the Grain level, and Off keeps the native value. The stage gate ignores grain
   for Standard, so the gate wrapper turns the stage on for any level. */
#if defined(HOOK_GRAIN_ENABLE) || defined(HOOK_GRAIN_PARTICLE)
// r3 = the level minus 1, or a value above 2 for Off. Flags change.
grain_level:
    ldr r3, =0x58012340
    ldr r3, [r3, #GRKIT_HOST_GRAIN]
    sub r3, r3, #1
    cmp r3, #2
    bx lr

// Replaces the BL to the stage gate. r0 = plan, r1 = shot settings. The caller
// reloads r0 and r1 after the call.
.global grain_gate
grain_gate:
    push {r0, lr}
    ldr r12, =HOOK_GRAIN_GATE_TARGET
    blx r12
    bl grain_level
    pop {r0, lr}
    movls r3, #1
    strbls r3, [r0, #GRAIN_GATE_FLAG]
    bx lr
#endif

#ifdef HOOK_GRAIN_ENABLE
// GR IV replaces `ldrb r3, [r4, #0x7b5]` (grain on). A cmp of r3 follows each
// replaced load, so the hooks may change the flags.
.global grain_enable
grain_enable:
    push {lr}
    bl grain_level
    pop {lr}
    movls r3, #1
    ldrbhi r3, [r4, #0x7b5]
    bx lr

// Replaces `ldrb r3, [r4, #0x7b7]` (strength 0..2): weak, medium, strong.
.global grain_strength
grain_strength:
    push {lr}
    bl grain_level
    pop {lr}
    ldrbhi r3, [r4, #0x7b7]
    bx lr

// Replaces `ldrb r3, [r4, #0x7b6]` (size 0..2). The size follows the ISO of
// the shot: small to ISO 400, medium to ISO 800, large above. +0xba0 holds the
// float APEX Sv that the EXIF ISO uses. The native ISO index is
// round((Sv - 1) * 3), so ISO 400 ends at Sv 43/6 and ISO 800 at Sv 49/6.
// For a positive float, the bit patterns compare as the values. A negative
// value, an infinity, or a NaN gives small grain.
.global grain_size
grain_size:
    push {lr}
    bl grain_level
    pop {lr}
    ldrbhi r3, [r4, #0x7b6]
    bxhi lr
    ldr r12, [r4, #0xba0]
    ldr r3, =0x7f800000
    cmp r12, r3
    movhs r3, #0
    bxhs lr
    ldr r3, =0x40e55555
    cmp r12, r3
    movlo r3, #0
    bxlo lr
    ldr r3, =0x4102aaab
    cmp r12, r3
    movlo r3, #1
    movhs r3, #2
    bx lr
#endif

#ifdef HOOK_GRAIN_PARTICLE
// GR III replaces `ldrb r3, [r5, #0x795]` (Particle 0..3): the level itself.
.global grain_particle
grain_particle:
    push {lr}
    bl grain_level
    pop {lr}
    addls r3, r3, #1
    ldrbhi r3, [r5, #0x795]
    bx lr
#endif

/* Capture: record the effect of this shot as the Image Control tag. Replaces
   the ImageTone BL in PentaxMakernoteEntryDataSetter. r1 is the IFD entry, and
   r2 the shot settings. FW_IFD_SET_SHORT(entry, 0, value) stores the u16 tag
   value. A shot without a private record keeps the native tag. */
.global picture_capture_tone
picture_capture_tone:
    push {r0-r4, lr}
    mov r0, r2
    bl picture_capture_record
    mov r12, r0
    pop {r0-r4, lr}
    cmp r12, #0
    ldreq pc, =HOOK_PICTURE_CAPTURE_TONE_TARGET
    mov r0, r1
    mov r1, #0
    add r12, r12, #(GRKIT_RECORD_CODE & ~0xff)
    ldrh r2, [r12, #(GRKIT_RECORD_CODE & 0xff)]
    ldr pc, =FW_IFD_SET_SHORT

// JpegEncodeMacroProcess::Execute entry: r0 = process, r1 = message. The stub
// keeps every argument and replays the displaced `mov ip, sp`.
.global picture_encode_entry
picture_encode_entry:
    push {r0-r3, r12, lr}
    bl picture_encode
    pop {r0-r3, r12, lr}
    mov r12, sp
    ldr pc, =HOOK_PICTURE_ENCODE_ENTRY_RESUME
.ltorg
