.syntax unified
.arm
.text
/* Function types let the linker emit BLX for calls from Thumb menu C. */
.global menu_header
.type menu_header, %function
menu_header:
    ldr r0, =0x58012340
    bx lr
.global menu_text_measure
.type menu_text_measure, %function
menu_text_measure:
    ldr r7, [r3, r7, lsl #2]
    push {r0-r3, lr}
    ldr r0, =0x58012340
    ldr r1, [r0, #36]
    cmp r1, #0
    beq measure_done
    ldr r1, [r0, #32]
    ldr r2, [r1, #0x160]
    cmp r5, r2
    addeq r7, r0, #96
    moveq r9, #1
    beq measure_done
    ldr r2, [r1, #0x164]
    cmp r5, r2
    addeq r7, r0, #104
    moveq r9, #1
measure_done:
    pop {r0-r3, lr}
    ldr pc, =0x53396700
.global menu_text_draw
.type menu_text_draw, %function
menu_text_draw:
    ldr r3, [r3, r7, lsl #2]
    push {r1, r2, r12, lr}
    ldr r12, =0x58012340
    ldr r1, [r12, #36]
    cmp r1, #0
    beq draw_done
    ldr r1, [r12, #32]
    ldr r2, [r1, #0x160]
    cmp r4, r2
    addeq r3, r12, #96
    moveq r0, #1
    beq draw_done
    ldr r2, [r1, #0x164]
    cmp r4, r2
    addeq r3, r12, #104
    moveq r0, #1
draw_done:
    pop {r1, r2, r12, lr}
    ldr pc, =0x5339710c
.global menu_event_original
.type menu_event_original, %function
menu_event_original:
    .inst 0xe1d130b0
    ldr pc, =0x531a4dbc
.global menu_exit_original
.type menu_exit_original, %function
menu_exit_original:
    .inst 0xe5d02270
    ldr pc, =0x531a3e8c
.global menu_destroy_original
.type menu_destroy_original, %function
menu_destroy_original:
    mov r12, sp
    ldr pc, =0x5319ff28
.global menu_refresh_original
.type menu_refresh_original, %function
menu_refresh_original:
    mov r12, sp
    ldr pc, =0x531a4118
.global menu_changed_original
.type menu_changed_original, %function
menu_changed_original:
    mov r12, sp
    ldr pc, =0x531a4d00
.ltorg
.global menu_icon_width
.type menu_icon_width, %function
menu_icon_width:
    push {r1-r3, lr}
    mov r0, r1
    bl menu_icon
    cmp r0, #0
    pop {r1-r3, lr}
    movne r0, #40
    bxne lr
    movw r3, #670
    ldr pc, =0x53209fbc
.global menu_icon_height
.type menu_icon_height, %function
menu_icon_height:
    push {r1-r3, lr}
    mov r0, r1
    bl menu_icon
    cmp r0, #0
    pop {r1-r3, lr}
    movne r0, #40
    bxne lr
    movw r3, #670
    ldr pc, =0x53209ff0
.global menu_icon_guard
.type menu_icon_guard, %function
menu_icon_guard:
    push {r0, r2, r3, lr}
    mov r0, r3
    bl menu_icon
    cmp r0, #0
    pop {r0, r2, r3, lr}
    movwne r1, #65535
    movweq r1, #670
    ldr pc, =0x5320a7d0
.global menu_icon_descriptor
.type menu_icon_descriptor, %function
menu_icon_descriptor:
    push {r0-r5, r12, lr}
    mrs r4, cpsr
    mov r0, r3
    bl menu_icon
    mov r5, r0
    pop {r0-r3}
    cmp r5, #0
    movne r3, r5
    ldreq r3, [r2, r3, lsl #2]
    msr APSR_nzcvq, r4
    pop {r4, r5, r12, lr}
    ldr pc, =0x5320a814
.ltorg

/* Native pages apply each highlighted Image Control immediately. Preview it in
   the live view (+244 = 1) and keep the private selection until a native OK. */
.macro NATIVE_APPLY name, resume
.global \name
.type \name, %function
\name:
    push {r0-r2, r12}
    mrs r12, cpsr
    ldr r0, =0x58012340
    ldr r1, [r0, #60]
    cmp r1, #0
    moveq r2, #1
    streq r2, [r0, #244]
    msr APSR_nzcvq, r12
    pop {r0-r2, r12}
    mov r12, sp
    ldr pc, =\resume
.endm
NATIVE_APPLY menu_native_adj_apply, 0x531a0ba4
NATIVE_APPLY menu_native_dialog_apply, 0x531ab99c

/* OK on the native Image Control page keeps its value and disables the private
   effect. Cancel and shooting exits keep the private effect. */
.global menu_native_page_event
.type menu_native_page_event, %function
menu_native_page_event:
    push {r0, r2}
    ldrh r2, [r1]
    cmp r2, #0xd8
    ldr r0, =0x58012340
    moveq r2, #0
    streq r2, [r0, #8]
    streq r2, [r0, #244]
    pop {r0, r2}
    mov r12, sp
    ldr pc, =0x531ac274

/* Leaving the native Image Control page ends its live-view preview. */
.global menu_native_page_exit
.type menu_native_page_exit, %function
menu_native_page_exit:
    push {r0}
    ldr r12, =0x58012340
    mov r0, #0
    str r0, [r12, #244]
    pop {r0}
    mov r12, sp
    ldr pc, =0x531ab1b4
.ltorg

/* Capture: record the effect that the still build applied (+248) as the Image
   Control tag. Replaces BL 0x535c780c in PentaxMakernoteEntryDataSetter. r1 is
   the IFD entry. 0x535d02c8(entry, 0, value) stores the u16 tag value. */
.global menu_capture_tone
.type menu_capture_tone, %function
menu_capture_tone:
    ldr r12, =0x58012340
    ldr r12, [r12, #248]
    cmp r12, #0
    ldreq pc, =0x535c780c
    mov r0, r1
    mov r1, #0
    add r12, r12, #0x700
    ldrh r2, [r12, #-8]          @ record + 0x6f8, the effect code
    ldr pc, =0x535d02c8

/* Playback DISP: keep the native Image Control icon, or show the private icon
   for the code recorded at capture. Replaces BL 0x532b6eec, which ignores its
   arguments and returns an icon ID. */
.global menu_playback_icon
.type menu_playback_icon, %function
menu_playback_icon:
    push {r4, lr}
    ldr r12, =0x532b6eec
    blx r12
    cmp r0, #0
    bleq menu_playback_private
    pop {r4, pc}
.ltorg

/* Playback detail: show a GRKit photo's recorded adjustments as Standard
   parameters. Replaces BL 0x532b44bc (r0 = metadata singleton, r1 = type byte),
   which fails for non-native tags. Key 0x25 at +0x220 holds such a tag as
   0x10000 | (tag - 0x100); GRKit codes have bit 15 set. */
.global menu_playback_detail
.type menu_playback_detail, %function
menu_playback_detail:
    push {r0, r1, r4, lr}
    ldr r12, =0x532b44bc
    blx r12
    cmp r0, #0
    bne detail_done
    ldr r2, [sp]
    ldr r2, [r2, #0x220]
    lsr r3, r2, #16
    cmp r3, #1
    bne detail_done
    add r2, r2, #0x100
    tst r2, #0x8000
    beq detail_done
    ldr r1, [sp, #4]
    mov r2, #0xb
    strb r2, [r1]
    mov r0, #1
detail_done:
    add sp, sp, #8
    pop {r4, pc}
.ltorg

/* Native touch thunks return early for the saved slot, even in private mode. */
.macro NATIVE_TOUCH slot
.global menu_native_touch_\slot
.type menu_native_touch_\slot, %function
menu_native_touch_\slot:
    mov r1, #\slot
    b menu_native_touch
.endm
NATIVE_TOUCH 0
NATIVE_TOUCH 1
NATIVE_TOUCH 2
NATIVE_TOUCH 3
NATIVE_TOUCH 4

/* Preserve the native mapper, then substitute the active private still icon. */
.global menu_viewfinder_icon
.type menu_viewfinder_icon, %function
menu_viewfinder_icon:
    push {r4,lr}
    ldr r12,=0x532a9b28
    blx r12
    ldrb r1,[r4,#0x2d2]
    bl menu_viewfinder_icon_select
    pop {r4,pc}
.ltorg
