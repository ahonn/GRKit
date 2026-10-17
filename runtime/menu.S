#include "firmware.h"
#include "hooks.h"
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
    ldr r2, [r1, #ADJ_TITLE]
    cmp r5, r2
    addeq r7, r0, #96
    moveq r9, #1
    beq measure_done
    ldr r2, [r1, #ADJ_DETAIL]
    cmp r5, r2
    addeq r7, r0, #104
    moveq r9, #1
measure_done:
    pop {r0-r3, lr}
    ldr pc, =HOOK_MENU_TEXT_MEASURE_RESUME
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
    ldr r2, [r1, #ADJ_TITLE]
    cmp r4, r2
    addeq r3, r12, #96
    moveq r0, #1
    beq draw_done
    ldr r2, [r1, #ADJ_DETAIL]
    cmp r4, r2
    addeq r3, r12, #104
    moveq r0, #1
draw_done:
    pop {r1, r2, r12, lr}
    ldr pc, =HOOK_MENU_TEXT_DRAW_RESUME
.global menu_event_original
.type menu_event_original, %function
menu_event_original:
    .inst HOOK_MENU_EVENT_STOCK
    ldr pc, =HOOK_MENU_EVENT_RESUME
.global menu_exit_original
.type menu_exit_original, %function
menu_exit_original:
    .inst HOOK_MENU_EXIT_STOCK
    ldr pc, =HOOK_MENU_EXIT_RESUME
.global menu_destroy_original
.type menu_destroy_original, %function
menu_destroy_original:
    mov r12, sp
    ldr pc, =HOOK_MENU_DESTROY_RESUME
.global menu_refresh_original
.type menu_refresh_original, %function
menu_refresh_original:
    mov r12, sp
    ldr pc, =HOOK_MENU_REFRESH_RESUME
.global menu_changed_original
.type menu_changed_original, %function
menu_changed_original:
    mov r12, sp
    ldr pc, =HOOK_MENU_CHANGED_RESUME
.ltorg
#ifdef HOOK_MENU_ICON_WIDTH
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
    ldr pc, =HOOK_MENU_ICON_WIDTH_RESUME
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
    ldr pc, =HOOK_MENU_ICON_HEIGHT_RESUME
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
    ldr pc, =HOOK_MENU_ICON_GUARD_RESUME
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
    ldr pc, =HOOK_MENU_ICON_DESCRIPTOR_RESUME
.ltorg
#endif

#ifdef HOOK_MENU_NATIVE_ADJ_APPLY
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
NATIVE_APPLY menu_native_adj_apply, HOOK_MENU_NATIVE_ADJ_APPLY_RESUME
NATIVE_APPLY menu_native_dialog_apply, HOOK_MENU_NATIVE_DIALOG_APPLY_RESUME

/* OK on the native Image Control page keeps its value and disables the private
   effect. Cancel and shooting exits keep the private effect. */
.global menu_native_page_event
.type menu_native_page_event, %function
menu_native_page_event:
    push {r0, r2}
    ldrh r2, [r1]
    cmp r2, #EV_CONFIRM
    ldr r0, =0x58012340
    moveq r2, #0
    streq r2, [r0, #8]
    streq r2, [r0, #244]
    pop {r0, r2}
    mov r12, sp
    ldr pc, =HOOK_MENU_NATIVE_PAGE_EVENT_RESUME

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
    ldr pc, =HOOK_MENU_NATIVE_PAGE_EXIT_RESUME
.ltorg
#endif

#ifdef HOOK_MENU_CAPTURE_TONE
/* Capture: record the effect that the still build applied (+248) as the Image
   Control tag. Replaces the ImageTone BL in PentaxMakernoteEntryDataSetter. r1
   is the IFD entry. FW_IFD_SET_SHORT(entry, 0, value) stores the u16 tag value. */
.global menu_capture_tone
.type menu_capture_tone, %function
menu_capture_tone:
    ldr r12, =0x58012340
    ldr r12, [r12, #248]
    cmp r12, #0
    ldreq pc, =HOOK_MENU_CAPTURE_TONE_TARGET
    mov r0, r1
    mov r1, #0
    add r12, r12, #0x700
    ldrh r2, [r12, #-8]          @ record + 0x6f8, the effect code
    ldr pc, =FW_IFD_SET_SHORT
#endif

#ifdef HOOK_MENU_PLAYBACK_ICON
/* Playback DISP: keep the native Image Control icon, or show the private icon
   for the code recorded at capture. Replaces the BL to the icon getter, which
   ignores its arguments and returns an icon ID. */
.global menu_playback_icon
.type menu_playback_icon, %function
menu_playback_icon:
    push {r4, lr}
    ldr r12, =HOOK_MENU_PLAYBACK_ICON_TARGET
    blx r12
    cmp r0, #0
    bleq menu_playback_private
    pop {r4, pc}
.ltorg
#endif

#ifdef HOOK_MENU_PLAYBACK_DETAIL
/* Playback detail: show a GRKit photo's recorded adjustments as Standard
   parameters. Replaces the BL to the type getter (r0 = metadata singleton,
   r1 = type byte), which fails for non-native tags. Key 0x25 at METADATA_TONE
   holds such a tag as 0x10000 | (tag - 0x100). GRKit codes have bit 15 set. */
.global menu_playback_detail
.type menu_playback_detail, %function
menu_playback_detail:
    push {r0, r1, r4, lr}
    ldr r12, =HOOK_MENU_PLAYBACK_DETAIL_TARGET
    blx r12
    cmp r0, #0
    bne detail_done
    ldr r2, [sp]
    ldr r2, [r2, #METADATA_TONE]
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
#endif

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

#ifdef HOOK_MENU_VIEWFINDER_ICON
/* Preserve the native mapper, then substitute the active private still icon. */
.global menu_viewfinder_icon
.type menu_viewfinder_icon, %function
menu_viewfinder_icon:
    push {r4,lr}
    ldr r12,=HOOK_MENU_VIEWFINDER_ICON_TARGET
    blx r12
    ldrb r1,[r4,#LIVEVIEW_MOVIE]
    bl menu_viewfinder_icon_select
    pop {r4,pc}
.ltorg
#endif

#ifdef HOOK_MENU_ICON_LOOKUP
/* GR IV icon lookup (id in r0, returns a descriptor). A private ID returns its
   GRKit descriptor. Other IDs continue with the native lookup. */
.global menu_icon_lookup
.type menu_icon_lookup, %function
menu_icon_lookup:
    push {r1-r3, r12, lr}
    push {r0}
    bl menu_icon
    cmp r0, #0
    addne sp, sp, #4
    popne {r1-r3, r12, pc}
    pop {r0}
    pop {r1-r3, r12, lr}
    mov r12, sp
    ldr pc, =HOOK_MENU_ICON_LOOKUP_RESUME
/* GR IV icon draw: replaces `cmp r7, r2` (id, native limit). A private ID
   passes the limit check. */
.global menu_icon_limit
.type menu_icon_limit, %function
menu_icon_limit:
    push {r0-r3, r12, lr}
    mov r0, r7
    bl menu_icon
    cmp r0, #0
    pop {r0-r3, r12, lr}
    movwne r2, #0xffff
    cmp r7, r2
    ldr pc, =HOOK_MENU_ICON_LIMIT_RESUME
/* GR IV icon draw: replaces `ldr r3, [r0, r7, lsl #2]` (table entry). A private
   ID uses its GRKit descriptor, whose first byte is nonzero. */
.global menu_icon_entry
.type menu_icon_entry, %function
menu_icon_entry:
    push {r0-r2, r12, lr}
    mov r0, r7
    bl menu_icon
    movs r3, r0
    pop {r0-r2, r12, lr}
    ldreq r3, [r0, r7, lsl #2]
    ldr pc, =HOOK_MENU_ICON_ENTRY_RESUME
.ltorg
#endif
