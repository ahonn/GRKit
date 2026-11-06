#include "firmware.h"
#include "abi.h"
#include "hooks.h"
.syntax unified
.arm
.text
/* Picture processing hooks for effect extensions. They link into the Framing
   region. */

/* Only the DNG writer's crop tags call these stubs. Keep native validation
   and unsupported entry types, then restore the RAW crop of a Framing shot. */
.global picture_dng_origin
picture_dng_origin:
    push {r0,r1,r4,lr}
    ldr r12, =HOOK_PICTURE_DNG_ORIGIN_TARGET
    blx r12
    mov r2, #0
    b .Ldng_crop
.global picture_dng_size
picture_dng_size:
    push {r0,r1,r4,lr}
    ldr r12, =HOOK_PICTURE_DNG_SIZE_TARGET
    blx r12
    mov r2, #1
.Ldng_crop:
    pop {r0,r1,r4,lr}
    b picture_dng_crop

/* C uses soft-float; the native rectangle helper takes crop in s0 and
   aspect on the stack. rect is {width,height,absolute x,absolute y}. */
.fpu vfpv3
.thumb
.thumb_func
.global picture_dng_rect
picture_dng_rect:
    push {r4,lr}
    sub sp, sp, #8
    mov r4, r1
    add r0, r0, #(SHOT_CROP & ~0xff)
    vldr s0, [r0, #(SHOT_CROP & 0xff)]
    movs r0, #0
    str r0, [sp]
    mov r0, r4
    adds r1, r4, #2
    adds r2, r4, #4
    adds r3, r4, #6
    ldr r4, =FW_DNG_NATIVE_RECT
    blx r4
    add sp, sp, #8
    pop {r4,pc}
.arm

#ifdef HOOK_MODULE_GAMMA_DIRTY
// Z=1 only for an owned record in r0. Preserves all registers.
picture_owned:
    push {r0-r3, lr}
    ldr r1, =GRKIT_HEADER_REFERENCE
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
    push {r0-r2, r12, lr}
    ldrb r3, [r4, #0xe]
    ldr r0, [r5, #8]
    bl picture_owned
    movne r0, #0
    // The native diff sets the stage flags from native inputs only. When the
    // live-view record or the adjustments of a private record change, rebuild
    // the curves now, and the color matrix (+5, the monochrome weights and the
    // saturation) and the YCbCr matrix (+7, the post matrix) on the next
    // update: their stages have already run. This hook runs only for
    // ImageTuningLiveview.
    ldr r1, =GRKIT_HEADER_REFERENCE
    ldr r2, [r1, #GRKIT_HOST_LIVE_RECORD]
    str r0, [r1, #GRKIT_HOST_LIVE_RECORD]
    subs r2, r2, r0
    movne r2, #1
    ldr r12, [r1, #GRKIT_HOST_PARAMETER_CHANGES]
    ldr lr, [r1, #GRKIT_HOST_LIVE_PARAMETER_CHANGES]
    str r12, [r1, #GRKIT_HOST_LIVE_PARAMETER_CHANGES]
    cmp r12, lr
    cmpne r0, #0
    movne r2, #1
    cmp r2, #0
    strbne r2, [r4, #5]
    // The MultiAxial stage (+6) holds the hue sectors of a record.
    strbne r2, [r4, #6]
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
    pop {r0-r2, r12, lr}
    ldr pc, =HOOK_MODULE_GAMMA_DIRTY_RESUME
#endif

/* Grain: the native grain stage of DigitalFilterProcess, which runs for
   recorded pictures only. SetEffectParameter reads the grain settings from the
   shot settings (r4 on GR IV, r5 on GR III); these hooks replace each load with
   the Grain level, and Off keeps the native value. The stage gate ignores grain
   for Standard, so the gate wrapper turns the stage on for any level. The gate
   runs before the parameter build of the shot, so for a GRKit effect it reads
   the Shading and Clarity of the native type; the wrapper decides from the
   record instead (picture_gate_flag). Extra Strong (level 4) takes the native
   strong grain, and the amplitude hooks scale its tables by GRAIN_EXTRA / 256. */
#ifndef GRAIN_EXTRA
#define GRAIN_EXTRA 384
#endif
#if defined(HOOK_GRAIN_ENABLE) || defined(HOOK_GRAIN_PARTICLE)
// r3 = the level minus 1: 0 to 3 for Weak to Extra Strong, or a value above 3
// for Off. The flags are those of `cmp r3, #3`. Without a module that uses
// CORE.GRAIN, as after a soft Off of Grain (runtime/manage.h), the level is
// Off.
grain_level:
    ldr r3, =GRKIT_HEADER_REFERENCE
    ldr r3, [r3, #GRKIT_HOST_CAPABILITIES]
    tst r3, #GRKIT_CORE_GRAIN_BIT
    ldrne r3, =GRKIT_HEADER_REFERENCE
    ldrne r3, [r3, #GRKIT_HOST_GRAIN]
    subne r3, r3, #1
    mvneq r3, #0
    cmp r3, #3
    bx lr

// Replaces the BL to the stage gate. r0 = plan, r1 = shot settings. The caller
// reloads r0 and r1 after the call.
.global grain_gate
grain_gate:
    push {r0, lr}
    ldr r12, =HOOK_GRAIN_GATE_TARGET
    blx r12
    ldr r0, [sp]
    ldrb r0, [r0, #GRAIN_GATE_FLAG]
    bl picture_gate_flag
    ldr r1, [sp]
    strb r0, [r1, #GRAIN_GATE_FLAG]
    bl grain_level
    pop {r0, lr}
    movls r3, #1
    strbls r3, [r0, #GRAIN_GATE_FLAG]
    bx lr
#endif

#if defined(HOOK_GRAIN_SIZE) || defined(HOOK_GRAIN_FILL)
// r0 = the float bits of the APEX Sv of the shot. Returns in r0 the size 0
// small, 1 medium, or 2 large of CORE.GRAIN_SIZE: 1 to 3 give 0 to 2, and
// Auto (0) follows the ISO: small to ISO 400, medium to ISO 800, large above.
// The native ISO index is round((Sv - 1) * 3), so ISO 400 ends at Sv 43/6 and
// ISO 800 at Sv 49/6. For a positive float, the bit patterns compare as the
// values. A negative value, an infinity, or a NaN gives small grain. A patch of
// PipelineGrainSize applies only with a module that uses CORE.GRAIN_SIZE, so
// the header word is set. Changes r1 and the flags.
grain_size_of:
    ldr r1, =GRKIT_HEADER_REFERENCE
    ldr r1, [r1, #GRKIT_HOST_GRAIN_SIZE]
    sub r1, r1, #1
    cmp r1, #2
    movls r0, r1
    bxls lr
    ldr r1, =0x7f800000
    cmp r0, r1
    movhs r0, #0
    bxhs lr
    ldr r1, =0x40e55555
    cmp r0, r1
    movlo r0, #0
    bxlo lr
    ldr r1, =0x4102aaab
    cmp r0, r1
    movlo r0, #1
    movhs r0, #2
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
// Extra Strong takes strong.
.global grain_strength
grain_strength:
    push {lr}
    bl grain_level
    pop {lr}
    ldrbhi r3, [r4, #0x7b7]
    bxhi lr
    cmp r3, #2
    movhi r3, #2
    bx lr

#ifdef HOOK_GRAIN_AMPLITUDE
// GR IV replaces `mov r0, #0x200` after the load of the luma amplitude r5 of
// the grain tables (the 3 x 3 tables at 0x53b4bf3c by strength and size).
// Each table entry is the input plus or minus min(input, 255 - input, r5),
// so a larger r5 stays inside 0..255. Extra Strong scales r5.
.global grain_amplitude
grain_amplitude:
    push {r3, lr}
    bl grain_level
    cmp r3, #3
    bne 1f
    mov r3, #GRAIN_EXTRA
    mul r5, r3, r5
    lsr r5, r5, #8
1:  pop {r3, lr}
    mov r0, #0x200
    bx lr
#endif
#endif

#ifdef HOOK_GRAIN_SIZE
// Replaces `ldrb r3, [r4, #0x7b6]` (size 0..2) with the size of
// CORE.GRAIN_SIZE (grain_size_of). +0xba0 holds the float APEX Sv that the
// EXIF ISO uses (0x537fe338).
.global grain_size
grain_size:
    push {r0, r1, lr}
    bl grain_level
    ldrbhi r3, [r4, #0x7b6]
    bhi 1f
    ldr r0, [r4, #0xba0]
    bl grain_size_of
    mov r3, r0
1:  pop {r0, r1, pc}
#endif

#ifdef HOOK_GRAIN_PARTICLE
// GR III replaces `ldrb r3, [r5, #0x795]` (Particle 0..3). The granulation
// scales the noise amplitude by 256, 128, or 64 for Particle 1, 2, or 3
// (0x5367ae28), so level 1..3 maps to Particle 3..1, and Extra Strong to
// Particle 1 (grain_amplitude then scales it). The hook also stores
// the grain of the shot for the noise fill: 0 for the native noise, else
// the size 1 to 3.
.global grain_particle
grain_particle:
    push {r0, r1, lr}
#ifdef HOOK_GRAIN_FILL
    bl grain_level
    movhi r0, #0
    bhi 1f
    ldr r0, [r5, #GRAIN_SV]
    bl grain_size_of
    add r0, r0, #1
1:  ldr r1, =GRKIT_HEADER_REFERENCE
    str r0, [r1, #GRKIT_HOST_GRAIN_SHOT]
#endif
    bl grain_level
    ldrbhi r3, [r5, #0x795]
    bhi 2f
    cmp r3, #2
    movhi r3, #2
    rsb r3, r3, #3
2:  pop {r0, r1, pc}

#ifdef HOOK_GRAIN_AMPLITUDE
// GR III replaces `cmp r5, #2`, the first test of the amplitude choice by the
// Particle r5 (256, 128, or 64), and goes on after it (0x5367ae40) with r6 =
// the amplitude. Extra Strong takes GRAIN_EXTRA. r3 and lr are free here.
.global grain_amplitude
grain_amplitude:
    push {r3, lr}
    bl grain_level
    cmp r3, #3
    moveq r6, #GRAIN_EXTRA
    beq 1f
    cmp r5, #2
    moveq r6, #0x80
    beq 1f
    cmp r5, #3
    moveq r6, #0x40
    movne r6, #0x100
1:  pop {r3, lr}
    ldr pc, =FW_GRAIN_AMPLITUDE_DONE
#endif
#endif

#ifdef HOOK_GRAIN_FILL
// GR III replaces the BL to the noise fill of the granulation: r0 = the noise
// plane, r1 = pitch x height. grain_noise (picture.c) also needs the pitch and
// the height, which the granulation keeps in its frame.
.global grain_fill
grain_fill:
    ldr r2, [fp, #-GRAIN_FILL_PITCH]
    ldr r3, [fp, #-GRAIN_FILL_HEIGHT]
    b grain_noise
#endif

/* Capture: record the effect of this shot as the Image Control tag. Replaces
   the ImageTone BL in PentaxMakernoteEntryDataSetter. r1 is the IFD entry, and
   r2 the shot settings. FW_IFD_SET_SHORT(entry, 0, value) stores the u16 tag
   value. A shot without a private record keeps the native tag. A playback
   edit keeps the effect of its photo (picture_capture_tone_record). */
.global picture_capture_tone
picture_capture_tone:
    push {r0-r4, lr}
    mov r0, r2
    bl picture_capture_tone_record
    mov r12, r0
    pop {r0-r4, lr}
    cmp r12, #0
    ldreq pc, =HOOK_PICTURE_CAPTURE_TONE_TARGET
    mov r0, r1
    mov r1, #0
    add r12, r12, #(GRKIT_RECORD_CODE & ~0xff)
    ldrh r2, [r12, #(GRKIT_RECORD_CODE & 0xff)]
    ldr pc, =FW_IFD_SET_SHORT

/* JPEG header setup of a write and of an in-camera rewrite: the BL to the
   APP1 Exif setup, r0 = writer, r1 = image information. After a successful
   setup, picture_jpeg_xmp (picture.c) adds the XMP segment of the look before
   APP7. The setup result returns. */
.macro JPEG_HEADER_STUB name, target
.global \name
\name:
    push {r0, r1, r4, lr}
    ldr r12, =\target
    blx r12
    movs r4, r0
    beq 1f
    ldr r0, [sp]
    bl picture_jpeg_xmp
1:  mov r0, r4
    add sp, sp, #8
    pop {r4, pc}
.endm
#ifdef HOOK_PICTURE_JPEG_HEADER_TARGET
JPEG_HEADER_STUB picture_jpeg_header, HOOK_PICTURE_JPEG_HEADER_TARGET
#endif
#ifdef HOOK_PICTURE_JPEG_REWRITE_TARGET
JPEG_HEADER_STUB picture_jpeg_rewrite, HOOK_PICTURE_JPEG_REWRITE_TARGET
#endif

/* Capture: the maker-note adjustments (tag 0x247) of a GRKit shot. Replaces
   the BL to the native fill: r0 = the setter + 8, r1 is the IFD entry, and r2
   the shot settings. The native fill runs first, and its result returns. */
.global picture_capture_parameters
picture_capture_parameters:
    push {r1, r2, r4, lr}
    ldr r12, =HOOK_PICTURE_CAPTURE_PARAMETERS_TARGET
    blx r12
    mov r4, r0
    ldr r0, [sp]
    ldr r1, [sp, #4]
    bl picture_record_parameters
    mov r0, r4
    pop {r1, r2, r4, pc}

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

#ifdef HOOK_PICTURE_DEVELOP_TONE
/* The start of a RAW development: this replaces the BL that gives its initial
   Image Control. picture_develop_start (runtime/picture.c) sets the effect and
   clears the Framing ratio of the development, then the native conversion
   runs. */
.global picture_develop_tone
picture_develop_tone:
    push {r0-r3,r12,lr}
    bl picture_develop_start
    pop {r0-r3,r12,lr}
    ldr pc, =HOOK_PICTURE_DEVELOP_TONE_TARGET
#endif
