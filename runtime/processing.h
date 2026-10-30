// The parts of the Image Control processing hooks that each family shares:
// the record check, the shot ring store, and the parameter build hook. The
// processing source of a family (Layout::processing_source) includes this
// file after it defines the registers of the hooked parameter build:
//   PROCESSING_CONTEXT   the context (+0x2ac holds its kind)
//   PROCESSING_OBJECT    the transient object that the build fills
//   PROCESSING_KEPT_BYTE optional: a byte of the filled halfwords that the
//                        native fill does not write, so the fill keeps it
// r4 is the packet and r7 the shot settings in each family.
#if !defined(PROCESSING_CONTEXT) || !defined(PROCESSING_OBJECT)
#error "Define the registers of the parameter build before processing.h"
#endif

// Return Z=1 only for an owned record pointer in r0. Preserve all registers.
// The live view asks several times a frame. The active record, a record
// whenever it is not 0, is the usual answer, and a pointer outside the
// records, such as a native one, needs no scan.
.if GRKIT_RECORD_SIZE != 17 << 8
.error "The record end is count x 17 x 256"
.endif
.Lowned_record:
    push {r0-r3, lr}
    ldr r1, =GRKIT_HEADER_REFERENCE
    ldr r3, [r1, #GRKIT_HOST_ACTIVE]
    cmp r3, #0
    beq owned_scan
    cmp r0, r3
    beq owned_done
owned_scan:
    ldr r2, [r1, #GRKIT_HOST_COUNT]
    ldr r1, [r1, #GRKIT_HOST_RECORDS]
    cmp r0, r1
    blo owned_no
    add r3, r2, r2, lsl #4
    add r3, r1, r3, lsl #8
    cmp r0, r3
    bhs owned_no
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

// The custom-curve path of the native gamma stage composes the record curves
// with the base curve (the first 512 bytes of the gamma object) on every
// live-view frame, then flags the result (GAMMA_UPLOAD) for the table build.
// For a private record the result depends only on the gamma object, the
// record (only the installer writes its curves), the bypass byte of the packet
// (+0x89), and the base curve. GRKIT_HOST_GAMMA_KEY keeps a hash of these
// inputs for the last composition of a private record.
// r0 = gamma object, r1 = record, r2 = packet. Return Z=1 when the object
// still holds the composition of these inputs: the key matches and the native
// path has not rebuilt the object since (GAMMA_BUILT is 0). Else store the key
// of these inputs. Preserve all registers.
.Lgamma_cached:
    push {r0-r11, lr}
    ldr r3, =0x01000193
    ldrb r4, [r2, #0x89]
    eor r4, r4, r1
    mla r4, r4, r3, r0
    add r5, r0, #0x200
    // Eight words for each loop test: the live view hashes each frame.
gamma_hash:
    ldm r0!, {r1, r2, r6, r7, r8, r9, r10, r11}
    mla r4, r4, r3, r1
    mla r4, r4, r3, r2
    mla r4, r4, r3, r6
    mla r4, r4, r3, r7
    mla r4, r4, r3, r8
    mla r4, r4, r3, r9
    mla r4, r4, r3, r10
    mla r4, r4, r3, r11
    cmp r0, r5
    bne gamma_hash
    // A key is never 0, which marks no composition.
    orr r4, r4, #1
    ldr r0, [sp]
    add r0, r0, #(GAMMA_BUILT & ~0xff)
    ldr r0, [r0, #(GAMMA_BUILT & 0xff)]
    ldr r1, =GRKIT_HEADER_REFERENCE
    ldr r2, [r1, #GRKIT_HOST_GAMMA_KEY]
    str r4, [r1, #GRKIT_HOST_GAMMA_KEY]
    cmp r0, #0
    cmpeq r2, r4
    pop {r0-r11, pc}

// Forget the key: a native custom curve replaces the composition.
.Lgamma_forget:
    push {r0, r1}
    ldr r0, =GRKIT_HEADER_REFERENCE
    mov r1, #0
    str r1, [r0, #GRKIT_HOST_GAMMA_KEY]
    pop {r0, r1}
    bx lr

// Store r0, the record of a non-live-view build (0 for native processing),
// and the posture of the electronic level in the shot ring under the shot
// settings pointer r7. The live view (context kind 1) stores nothing. A reader
// in another task checks the key again after it reads the record, so the key
// is cleared while the entry changes. FW_LEVEL is a static object whose second
// word points to the electronic level, or holds 0 before the level exists.
// Preserves all registers.
.if GRKIT_SHOT_STRIDE != 1 << 4
.error "The shot ring index shift assumes 16-byte entries"
.endif
.Lshot_store:
    push {r0-r3, r12, lr}
    ldrb r2, [PROCESSING_CONTEXT, #0x2ac]
    cmp r2, #1
    beq shot_done
    ldr r1, =GRKIT_HEADER_REFERENCE
    // The shot ring is in the picture scratch, at the start of the tail.
    ldr r3, [r1, #GRKIT_HOST_TAIL]
    add r3, r3, #(GRKIT_SHOT_RING & ~0xff)
    add r3, r3, #(GRKIT_SHOT_RING & 0xff)
    ldr r2, [r3]
    add r12, r2, #1
    str r12, [r3]
    and r2, r2, #(GRKIT_SHOT_ENTRIES - 1)
    add r2, r3, r2, lsl #4
    mov r12, #0
    str r12, [r2, #(GRKIT_SHOT_ENTRY + GRKIT_SHOT_KEY)]
    dmb ish
    str r0, [r2, #(GRKIT_SHOT_ENTRY + GRKIT_SHOT_RECORD)]
    ldr r1, =FW_LEVEL
    ldr r1, [r1, #4]
    cmp r1, #0
    ldrne r1, [r1, #LEVEL_POSTURE]
    str r1, [r2, #(GRKIT_SHOT_ENTRY + GRKIT_SHOT_POSTURE)]
    dmb ish
    str r7, [r2, #(GRKIT_SHOT_ENTRY + GRKIT_SHOT_KEY)]
shot_done:
    pop {r0-r3, r12, pc}

// Replaces the BL to the native parameter normalizer, not its shared entry.
// Snapshot activation before constructing this task's packet. Unsupported
// contexts use the native normalizer and cannot accidentally inherit a
// private record.
module_parameters:
    push {r0-r3, r12, lr}
    ldr r0, =GRKIT_HEADER_REFERENCE
    ldr r3, [r0, #GRKIT_HOST_PROCESSING_CALLS]
    add r3, r3, #1
    str r3, [r0, #GRKIT_HOST_PROCESSING_CALLS]
    // Without a module that uses CORE.TONE, as after a soft Off of Film Look
    // (runtime/manage.h), the native processing runs.
    ldr r3, [r0, #GRKIT_HOST_CAPABILITIES]
    ldr r0, [r0, #GRKIT_HOST_ACTIVE]
    tst r3, #GRKIT_CORE_TONE_BIT
    moveq r0, #0
    // Context 1 is the live view (ImageTuningLiveview); 0 is the still path.
    ldrb r2, [PROCESSING_CONTEXT, #0x2ac]
    cmp r2, #1
    bhi parameters_stock
    // Menu previews change only the live view. Still captures keep the committed record.
    bne parameters_committed
    ldr r3, =GRKIT_HEADER_REFERENCE
    ldr r3, [r3, #GRKIT_HOST_PREVIEW]
    cmp r3, #0
    movne r0, r3
parameters_committed:
    ldrb r2, [PROCESSING_CONTEXT, #0x2ad]
    cmp r2, #0
    bne parameters_stock
    ldrb r2, [PROCESSING_CONTEXT, #0x2b2]
    cmp r2, #0
    bne parameters_stock
    .word 0xeb000000 | (((.Lowned_record - . - 8) >> 2) & 0x00ffffff)
    bne parameters_stock
    str r0, [r4, #8]
    // Capture metadata and the encode read the record of this shot.
    .word 0xeb000000 | (((.Lshot_store - . - 8) >> 2) & 0x00ffffff)
    // Fill the transient object like the native Standard branch, from the
    // adjustments of the record (r0), not from the native settings.
    add r3, r0, #(GRKIT_RECORD_PARAMETERS & ~0xff)
    add r3, r3, #(GRKIT_RECORD_PARAMETERS & 0xff)
#ifdef PROCESSING_KEPT_BYTE
    ldrb r2, [PROCESSING_OBJECT, #PROCESSING_KEPT_BYTE]
    push {r2}
#endif
    add r1, PROCESSING_OBJECT, #0x1e0
    add r0, PROCESSING_OBJECT, #0x200
    adr r12, parameters_sources
parameters_fill:
    ldrb r2, [r12], #1
    cmp r2, #9
    subhs r2, r2, #0x10
    bhs parameters_store
    // A stored byte is the change from the default, and a default is
    // (native - 4) mod 9 (zero without a native look). The native value is
    // (stored + default + 4) mod 9. A stored byte above 8 counts as 0.
    add lr, r3, r2
    ldrb r2, [r3, r2]
    cmp r2, #8
    movhi r2, #0
    ldrb lr, [lr, #(GRKIT_RECORD_FN_DEFAULTS - GRKIT_RECORD_PARAMETERS)]
    add r2, r2, lr
    add r2, r2, #4
    cmp r2, #9
    subhs r2, r2, #9
    cmp r2, #9
    subhs r2, r2, #9
parameters_store:
    strh r2, [r1], #2
    cmp r1, r0
    bne parameters_fill
#ifdef NATIVE_SATURATION_BEFORE_CURVES
    // A native look applies saturation after the tone curves, as GR IV does:
    // picture_post applies it, and the native processing gets the neutral
    // value. 0x20 is GRKIT_FEATURE_NATIVE_LOOK, and r1 is at +0x200.
    ldr r2, [r3, #(GRKIT_RECORD_FEATURES - GRKIT_RECORD_PARAMETERS)]
    tst r2, #0x20
    movne r2, #4
    strhne r2, [r1, #-0x20]
#endif
#ifdef PROCESSING_KEPT_BYTE
    pop {r2}
    strb r2, [PROCESSING_OBJECT, #PROCESSING_KEPT_BYTE]
#endif
    pop {r0-r3, r12, pc}
// Source per halfword at +0x1e0: an index of GRKIT_RECORD_PARAMETERS, or 0x10
// plus a constant. Saturation, hue, key, contrast, highlight, shadow, 1,
// sharpness, 0, 0, shading, clarity, 0, 0, 0, 0.
parameters_sources:
    .byte 0, 1, 2, 3, 4, 5, 0x11, 6, 0x10, 0x10, 7, 8, 0x10, 0x10, 0x10, 0x10
parameters_stock:
    mov r0, #0
    str r0, [r4, #8]
    // Other non-live-view builds, such as in-camera RAW development, record no effect.
    .word 0xeb000000 | (((.Lshot_store - . - 8) >> 2) & 0x00ffffff)
    pop {r0-r3, r12, lr}
    ldr pc, =HOOK_MODULE_PARAMETERS_TARGET
