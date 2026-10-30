#include "firmware.h"
#include "abi.h"
#include "hooks.h"
/* Each build has the Fn parameter page (PARAMETER_KINDS), the settings page
   (SETTINGS_SUBMENU), and the checkbox options page (FW_CHECK_ITEM) of
   menu.c. The hooks of each page come from the site tables of the target
   (hooks.h). */
#if !defined(PARAMETER_KINDS) || !defined(HOOK_MENU_PARAMETER_START)
#error "The build header defines PARAMETER_KINDS, and the target has the menu_parameter_* hooks"
#endif
#if !defined(SETTINGS_SUBMENU) || !defined(HOOK_MENU_SETTINGS_CHANGE)
#error "The build header defines SETTINGS_SUBMENU, and the target has the menu_settings_* hooks"
#endif
#if !defined(FW_CHECK_ITEM) || !defined(HOOK_MENU_CHECK_FACTORY)
#error "The build header defines FW_CHECK_ITEM, and the target has the menu_check_* hooks"
#endif
.syntax unified
.arm
.text
/* Function types let the linker emit BLX for calls from Thumb menu C. */
.global menu_header
.type menu_header, %function
menu_header:
    ldr r0, =GRKIT_HEADER_REFERENCE
    bx lr
.global menu_text_measure
.type menu_text_measure, %function
menu_text_measure:
    /* r7 is the text ID and r5 the view. A text that menu.c gives
       (menu_text_override) replaces the table entry: the effect name in the
       title of a GRKit parameter page, or a settings page text. Bit 0 of the
       result asks for the font of the menu language. */
    push {r0-r3, r12, lr}
    mov r0, r5
    mov r1, r7
    bl menu_text_override
    movs r1, r0
    beq measure_table
    bic r7, r1, #1
    tst r1, #1
    beq measure_override_done
    bl menu_table_index
    mov r9, r0
measure_override_done:
    pop {r0-r3, r12, lr}
    b measure_resume
measure_table:
    pop {r0-r3, r12, lr}
    ldr r7, [r3, r7, lsl #2]
    push {r0-r3, lr}
    ldr r0, =GRKIT_HEADER_REFERENCE
    ldr r1, [r0, #GRKIT_HOST_MENU_LEVEL]
    cmp r1, #0
    beq measure_done
    ldr r1, [r0, #GRKIT_HOST_MENU_CONTROLLER]
    ldr r2, [r1, #ADJ_TITLE]
    cmp r5, r2
    addeq r7, r0, #GRKIT_HOST_MENU_TITLE
    beq measure_language
    ldr r2, [r1, #ADJ_DETAIL]
    cmp r5, r2
    bne measure_done
    add r7, r0, #GRKIT_HOST_MENU_DETAIL
measure_language:
    /* r9 is the language table index, which selects the font. menu.c picks
       the index of the text language. r12 may be live after the hook. */
    push {r12}
    bl menu_table_index
    mov r9, r0
    pop {r12}
measure_done:
    pop {r0-r3, lr}
measure_resume:
    ldr pc, =HOOK_MENU_TEXT_MEASURE_RESUME
.global menu_text_draw
.type menu_text_draw, %function
menu_text_draw:
    /* r7 is the text ID, r4 the view, r3 the table, and r0 the language table
       index. As at the measure hook, a text that menu.c gives replaces the
       table entry. It replaces the saved r3, so the pop loads it. */
    push {r0-r3, r12, lr}
    mov r0, r4
    mov r1, r7
    bl menu_text_override
    movs r1, r0
    beq draw_table
    bic r2, r1, #1
    str r2, [sp, #12]
    tst r1, #1
    beq draw_override_done
    bl menu_table_index
    str r0, [sp]
draw_override_done:
    pop {r0-r3, r12, lr}
    b draw_resume
draw_table:
    pop {r0-r3, r12, lr}
    ldr r3, [r3, r7, lsl #2]
    push {r1, r2, r12, lr}
    ldr r12, =GRKIT_HEADER_REFERENCE
    ldr r1, [r12, #GRKIT_HOST_MENU_LEVEL]
    cmp r1, #0
    beq draw_done
    ldr r1, [r12, #GRKIT_HOST_MENU_CONTROLLER]
    ldr r2, [r1, #ADJ_TITLE]
    cmp r4, r2
    addeq r3, r12, #GRKIT_HOST_MENU_TITLE
    beq draw_language
    ldr r2, [r1, #ADJ_DETAIL]
    cmp r4, r2
    bne draw_done
    add r3, r12, #GRKIT_HOST_MENU_DETAIL
draw_language:
    /* r0 is the language table index, as at the measure hook. */
    push {r3, lr}
    bl menu_table_index
    pop {r3, lr}
draw_done:
    pop {r1, r2, r12, lr}
draw_resume:
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
   the live view (GRKIT_HOST_PREVIEW = 1) and keep the private selection until a native OK. */
.macro NATIVE_APPLY name, resume
.global \name
.type \name, %function
\name:
    push {r0-r2, r12}
    mrs r12, cpsr
    ldr r0, =GRKIT_HEADER_REFERENCE
    ldr r1, [r0, #GRKIT_HOST_MENU_REBUILDING]
    cmp r1, #0
    moveq r2, #1
    streq r2, [r0, #GRKIT_HOST_PREVIEW]
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
    ldr r0, =GRKIT_HEADER_REFERENCE
    moveq r2, #0
    streq r2, [r0, #GRKIT_HOST_ACTIVE]
    streq r2, [r0, #GRKIT_HOST_PREVIEW]
    /* A change of the saved effect (GRKIT_STATE_CHANGED). */
    ldreq r2, [r0, #GRKIT_HOST_STATE_CHANGES]
    addeq r2, r2, #1
    streq r2, [r0, #GRKIT_HOST_STATE_CHANGES]
    pop {r0, r2}
    mov r12, sp
    ldr pc, =HOOK_MENU_NATIVE_PAGE_EVENT_RESUME

/* Leaving the native Image Control page ends its live-view preview. */
.global menu_native_page_exit
.type menu_native_page_exit, %function
menu_native_page_exit:
    push {r0}
    ldr r12, =GRKIT_HEADER_REFERENCE
    mov r0, #0
    str r0, [r12, #GRKIT_HOST_PREVIEW]
    pop {r0}
    mov r12, sp
    ldr pc, =HOOK_MENU_NATIVE_PAGE_EXIT_RESUME
.ltorg
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
/* GR IV icon draw: replaces the limit check of the icon ID in r7, `cmp r7, r2`
   with the native limit in r2, or `cmp r7, #NATIVE_ICON_LIMIT` when the
   target defines it. A private ID passes the check: with an immediate limit,
   the native branch after the check is `bhi` or `bhs`, and `cmn r7, #0`
   clears C, so neither is taken. */
.global menu_icon_limit
.type menu_icon_limit, %function
menu_icon_limit:
    push {r0-r3, r12, lr}
    mov r0, r7
    bl menu_icon
    cmp r0, #0
    pop {r0-r3, r12, lr}
#ifdef NATIVE_ICON_LIMIT
    beq 1f
    cmn r7, #0
    ldr pc, =HOOK_MENU_ICON_LIMIT_RESUME
1:  cmp r7, #NATIVE_ICON_LIMIT
#else
    movwne r2, #0xffff
    cmp r7, r2
#endif
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

/* The native parameter page edits the adjustments of a GRKit effect while
   GRKIT_HOST_PARAMETERS holds its record (menu.c). Each hook replaces the
   first instruction, `mov r12, sp`, of an Image Control model getter or setter
   of one Standard adjustment. Only the parameter page calls them. */
.macro PARAMETER_GET index, resume
.global menu_parameter_get_\index
.type menu_parameter_get_\index, %function
/* The getter has no arguments and returns the native value. */
menu_parameter_get_\index:
    push {r1-r3, lr}
    mov r0, #\index
    bl menu_parameter_get
    subs r0, r0, #1
    popge {r1-r3, pc}
    pop {r1-r3, lr}
    mov r12, sp
    ldr pc, =\resume
.endm
.macro PARAMETER_SET index, resume
.global menu_parameter_set_\index
.type menu_parameter_set_\index, %function
/* r0 = the model, r1 = the native value. The setter returns nothing. */
menu_parameter_set_\index:
    push {r0-r3, r12, lr}
    mov r0, #\index
    bl menu_parameter_set
    cmp r0, #0
    pop {r0-r3, r12, lr}
    bxne lr
    mov r12, sp
    ldr pc, =\resume
.endm
PARAMETER_GET 0, HOOK_MENU_PARAMETER_GET_0_RESUME
PARAMETER_GET 1, HOOK_MENU_PARAMETER_GET_1_RESUME
PARAMETER_GET 2, HOOK_MENU_PARAMETER_GET_2_RESUME
PARAMETER_GET 3, HOOK_MENU_PARAMETER_GET_3_RESUME
PARAMETER_GET 4, HOOK_MENU_PARAMETER_GET_4_RESUME
PARAMETER_GET 5, HOOK_MENU_PARAMETER_GET_5_RESUME
PARAMETER_GET 6, HOOK_MENU_PARAMETER_GET_6_RESUME
PARAMETER_GET 7, HOOK_MENU_PARAMETER_GET_7_RESUME
PARAMETER_GET 8, HOOK_MENU_PARAMETER_GET_8_RESUME
.ltorg
PARAMETER_SET 0, HOOK_MENU_PARAMETER_SET_0_RESUME
PARAMETER_SET 1, HOOK_MENU_PARAMETER_SET_1_RESUME
PARAMETER_SET 2, HOOK_MENU_PARAMETER_SET_2_RESUME
PARAMETER_SET 3, HOOK_MENU_PARAMETER_SET_3_RESUME
PARAMETER_SET 4, HOOK_MENU_PARAMETER_SET_4_RESUME
PARAMETER_SET 5, HOOK_MENU_PARAMETER_SET_5_RESUME
PARAMETER_SET 6, HOOK_MENU_PARAMETER_SET_6_RESUME
PARAMETER_SET 7, HOOK_MENU_PARAMETER_SET_7_RESUME
PARAMETER_SET 8, HOOK_MENU_PARAMETER_SET_8_RESUME
.ltorg

/* Replace the BL to the native value read of a list row of the page. The
   read keeps its arguments, so a native read is a tail call. */
.macro PARAMETER_ROW index, target
.global menu_parameter_row_\index
.type menu_parameter_row_\index, %function
menu_parameter_row_\index:
    push {r0-r4, lr}
    mov r0, #\index
    bl menu_parameter_get
    subs r4, r0, #1
    movge r0, r4
    addge sp, sp, #4
    popge {r1-r4, pc}
    pop {r0-r4, lr}
    ldr r12, =\target
    bx r12
.endm
PARAMETER_ROW 0, HOOK_MENU_PARAMETER_ROW_0_TARGET
PARAMETER_ROW 1, HOOK_MENU_PARAMETER_ROW_1_TARGET
PARAMETER_ROW 2, HOOK_MENU_PARAMETER_ROW_2_TARGET
PARAMETER_ROW 3, HOOK_MENU_PARAMETER_ROW_3_TARGET
PARAMETER_ROW 4, HOOK_MENU_PARAMETER_ROW_4_TARGET
PARAMETER_ROW 5, HOOK_MENU_PARAMETER_ROW_5_TARGET
PARAMETER_ROW 6, HOOK_MENU_PARAMETER_ROW_6_TARGET
PARAMETER_ROW 7, HOOK_MENU_PARAMETER_ROW_7_TARGET
PARAMETER_ROW 8, HOOK_MENU_PARAMETER_ROW_8_TARGET
.ltorg

/* The page start method. r0 = the page. */
.global menu_parameter_start
.type menu_parameter_start, %function
menu_parameter_start:
    push {r0-r3, r12, lr}
    bl menu_parameter_page_start
    pop {r0-r3, r12, lr}
    mov r12, sp
    ldr pc, =HOOK_MENU_PARAMETER_START_RESUME

/* Replaces the BL to the method that reads the type and the set kinds of the
   page. r0 = the page. */
.global menu_parameter_kinds
.type menu_parameter_kinds, %function
menu_parameter_kinds:
    push {r4, lr}
    mov r4, r0
    ldr r12, =HOOK_MENU_PARAMETER_KINDS_TARGET
    blx r12
    mov r0, r4
    bl menu_parameter_page_kinds
    pop {r4, pc}

/* Replace the two BLs of the page start to the model method that loads the
   snapshot of the adjustments. r0 = the model. */
.macro PARAMETER_SNAPSHOT index, target
.global menu_parameter_snapshot_\index
.type menu_parameter_snapshot_\index, %function
menu_parameter_snapshot_\index:
    push {r4, lr}
    mov r4, r0
    ldr r12, =\target
    blx r12
    mov r0, r4
    bl menu_parameter_page_snapshot
    pop {r4, pc}
.endm
PARAMETER_SNAPSHOT 0, HOOK_MENU_PARAMETER_SNAPSHOT_0_TARGET
PARAMETER_SNAPSHOT 1, HOOK_MENU_PARAMETER_SNAPSHOT_1_TARGET

/* The page event handler. Replaces `ldrh r3, [r1]`, the event kind. r0 = the
   page, r1 = the event. */
.global menu_parameter_event
.type menu_parameter_event, %function
menu_parameter_event:
    ldrh r3, [r1]
    cmp r3, #DETAIL_EV_SUBPAGE
    bne parameter_event_resume
    push {r0-r3, r12, lr}
    bl menu_parameter_session
    cmp r0, #0
    pop {r0-r3, r12, lr}
    movne r0, #0
    bxne lr
parameter_event_resume:
    ldr pc, =HOOK_MENU_PARAMETER_EVENT_RESUME
.ltorg

/* The native settings page of a GRKit root row (menu.c). ChangeController
   entry: r1 is the controller that opens. */
.global menu_settings_change
.type menu_settings_change, %function
menu_settings_change:
    push {r0-r3, r12, lr}
    mov r0, r1
    bl menu_settings_controller
    /* The C function gives the controller to change to, in r1, or 0: no
       change happens. */
    cmp r0, #0
    beq 1f
    str r0, [sp, #4]
    pop {r0-r3, r12, lr}
    .inst HOOK_MENU_SETTINGS_CHANGE_STOCK
    ldr pc, =HOOK_MENU_SETTINGS_CHANGE_RESUME
1:
    pop {r0-r3, r12, lr}
    bx lr

/* The entry of the controller factory (r0 the factory, r1 the controller
   ID): the checkbox page of an options page (menu_check_controller), else
   the native factory. */
.global menu_check_factory
.type menu_check_factory, %function
menu_check_factory:
    push {r0-r3, r12, lr}
    bl menu_check_controller
    cmp r0, #0
    beq 1f
    add sp, sp, #4
    pop {r1-r3, r12, lr}
    bx lr
1:
    pop {r0-r3, r12, lr}
    .inst HOOK_MENU_CHECK_FACTORY_STOCK
    ldr pc, =HOOK_MENU_CHECK_FACTORY_RESUME

#ifdef HOOK_MENU_INFO_UPDATE
/* The update of the camera information page (menu.c: menu_info_fill). The
   entry hook keeps r0 (the controller) and lr, and returns into
   menu_info_after: the stock prologue copies sp, so its epilogue restores
   this frame and returns here. */
.global menu_info_update
.type menu_info_update, %function
menu_info_update:
    push {r0, lr}
    ldr lr, =menu_info_after
    .inst HOOK_MENU_INFO_UPDATE_STOCK
    ldr pc, =HOOK_MENU_INFO_UPDATE_RESUME
menu_info_after:
    pop {r0, lr}
    push {r4, lr}
    bl menu_info_fill
    pop {r4, pc}
.ltorg
#endif

/* Each replaces the MOVT of the submenu kind table base with a BL, after the
   function has saved lr. The register gets the whole base. Other registers and
   the flags keep their values. A base in r0 to r3 goes to its saved slot. */
.macro SETTINGS_KINDS_SAVED name, slot
.global \name
.type \name, %function
\name:
    push {r0-r3, r12, lr}
    mrs r0, cpsr
    push {r0, r1}
    bl menu_settings_kinds
    str r0, [sp, #8+4*\slot]
    pop {r0, r1}
    msr APSR_nzcvq, r0
    pop {r0-r3, r12, pc}
.endm
.macro SETTINGS_KINDS_REGISTER name, register
.global \name
.type \name, %function
\name:
    push {r0-r3, r12, lr}
    mrs r0, cpsr
    push {r0, r1}
    bl menu_settings_kinds
    mov \register, r0
    pop {r0, r1}
    msr APSR_nzcvq, r0
    pop {r0-r3, r12, pc}
.endm
SETTINGS_KINDS_SAVED menu_settings_kinds_r3_0, 3
SETTINGS_KINDS_SAVED menu_settings_kinds_r2, 2
SETTINGS_KINDS_REGISTER menu_settings_kinds_r5, r5
/* The title of a submenu page: r0 holds its string view, which the next
   call sets to the title ID of the kind entry. The C function resets the
   view first. The base goes to the saved r3. */
.global menu_settings_kinds_r3_1
.type menu_settings_kinds_r3_1, %function
menu_settings_kinds_r3_1:
    push {r0-r3, r12, lr}
    mrs r1, cpsr
    push {r1, r2}
    bl menu_settings_title
    bl menu_settings_kinds
    str r0, [sp, #8+4*3]
    pop {r1, r2}
    msr APSR_nzcvq, r1
    pop {r0-r3, r12, pc}
/* The base in the draw, which has loaded its row list into r6: the C function
   also fits the rows of a GRKit page to the views of the list. */
.global menu_settings_kinds_r8
.type menu_settings_kinds_r8, %function
menu_settings_kinds_r8:
    push {r0-r3, r12, lr}
    mrs r0, cpsr
    push {r0, r1}
    mov r0, r6
    bl menu_settings_kinds_draw
    mov r8, r0
    pop {r0, r1}
    msr APSR_nzcvq, r0
    pop {r0-r3, r12, pc}

/* A submenu built after INSTALL has GRKIT_SUBMENU_ROWS row views instead of
   five (core.h: SUBMENU_ROWS). Its constructor builds the list from five row types on the
   stack, and the stack guard follows at fp - 0x18: the stub replaces the
   store of the fifth and gives the list the types of menu_settings_types. */
.global menu_settings_build
.type menu_settings_build, %function
menu_settings_build:
    ldr r1, =menu_settings_types
    mov r2, #GRKIT_SUBMENU_ROWS
    bx lr
.ltorg
/* The draw hides the rows from the row count to the last view of its list
   (r6): the two compares with the last row. The last view comes from the list,
   because a submenu that the firmware built before INSTALL has five views
   (menu_settings_kinds_draw). Only the flags change. */
.macro SETTINGS_LAST_VIEW name, register
.global \name
.type \name, %function
\name:
    push {r0, r1}
    ldr r0, [r6, #LIST_ROWS]
    ldr r1, [r6, #LIST_ROWS+4]
    sub r1, r1, r0
    lsr r1, r1, #2
    sub r1, r1, #1
    cmp \register, r1
    pop {r0, r1}
    bx lr
.endm
SETTINGS_LAST_VIEW menu_settings_hide_first, r7
SETTINGS_LAST_VIEW menu_settings_hide_next, r4
/* The kind of row r4 from the kind table (r8 its base, r3 the word index of
   the kind). The entry has room for the kinds of five rows: a later row
   takes the kind of the first, whose word index is r3 - r4. */
.global menu_settings_row_kind
.type menu_settings_row_kind, %function
menu_settings_row_kind:
    cmp r4, #5
    ldrlo r1, [r8, r3, lsl #2]
    subhs r1, r3, r4
    ldrhs r1, [r8, r1, lsl #2]
    bx lr
/* The label of row r4 in the draw (r3 + 8 its entry slot, r0 the row view):
   the C function gives the text ID in r1. The entry has room for the labels
   of six rows. */
.global menu_settings_label
.type menu_settings_label, %function
menu_settings_label:
    push {r0, r2, r3, r4, r12, lr}
    mov r1, r0
    ldrh r0, [r3, #8]
    mov r2, r4
    bl menu_settings_row_label
    mov r1, r0
    pop {r0, r2, r3, r4, r12, pc}
/* The controller of the row that OK opens (r2 + 0x14 its entry slot, r0
   the cursor row): the C function gives it in r5. The entry has room for
   the controllers of five rows. */
.global menu_settings_open
.type menu_settings_open, %function
menu_settings_open:
    push {r0-r5, r12, lr}
    mov r1, r0
    ldr r0, [r2, #0x14]
    bl menu_settings_row_controller
    str r0, [sp, #20]
    pop {r0-r5, r12, pc}
/* Each build stores type 0x22 for each of the five stack rows. */
.section .rodata
menu_settings_types:
    .rept GRKIT_SUBMENU_ROWS
    .byte 0x22
    .endr
.text

/* The entry of a value page method: r0 is the page, r1 and r2 the arguments.
   The C function gets r1, r2, and the row of the setting. It returns 0 for
   the native method, else the result + 1. */
.macro SETTINGS_METHOD name, function, setting, stock, resume
.global \name
.type \name, %function
\name:
    push {r0-r3, r12, lr}
    mov r0, r1
    mov r1, r2
    mov r2, #\setting
    bl \function
    subs r3, r0, #1
    blt 1f
    mov r0, r3
    add sp, sp, #4
    pop {r1-r3, r12, pc}
1:
    pop {r0-r3, r12, lr}
    .inst \stock
    ldr pc, =\resume
.endm
SETTINGS_METHOD menu_settings_count_0, menu_settings_count, 0, HOOK_MENU_SETTINGS_COUNT_0_STOCK, HOOK_MENU_SETTINGS_COUNT_0_RESUME
SETTINGS_METHOD menu_settings_draw_0, menu_settings_draw, 0, HOOK_MENU_SETTINGS_DRAW_0_STOCK, HOOK_MENU_SETTINGS_DRAW_0_RESUME
SETTINGS_METHOD menu_settings_apply_0, menu_settings_apply, 0, HOOK_MENU_SETTINGS_APPLY_0_STOCK, HOOK_MENU_SETTINGS_APPLY_0_RESUME
SETTINGS_METHOD menu_settings_current_0, menu_settings_current, 0, HOOK_MENU_SETTINGS_CURRENT_0_STOCK, HOOK_MENU_SETTINGS_CURRENT_0_RESUME
.ltorg
SETTINGS_METHOD menu_settings_count_1, menu_settings_count, 1, HOOK_MENU_SETTINGS_COUNT_1_STOCK, HOOK_MENU_SETTINGS_COUNT_1_RESUME
SETTINGS_METHOD menu_settings_draw_1, menu_settings_draw, 1, HOOK_MENU_SETTINGS_DRAW_1_STOCK, HOOK_MENU_SETTINGS_DRAW_1_RESUME
SETTINGS_METHOD menu_settings_apply_1, menu_settings_apply, 1, HOOK_MENU_SETTINGS_APPLY_1_STOCK, HOOK_MENU_SETTINGS_APPLY_1_RESUME
SETTINGS_METHOD menu_settings_current_1, menu_settings_current, 1, HOOK_MENU_SETTINGS_CURRENT_1_STOCK, HOOK_MENU_SETTINGS_CURRENT_1_RESUME
.ltorg
SETTINGS_METHOD menu_settings_count_2, menu_settings_count, 2, HOOK_MENU_SETTINGS_COUNT_2_STOCK, HOOK_MENU_SETTINGS_COUNT_2_RESUME
SETTINGS_METHOD menu_settings_draw_2, menu_settings_draw, 2, HOOK_MENU_SETTINGS_DRAW_2_STOCK, HOOK_MENU_SETTINGS_DRAW_2_RESUME
SETTINGS_METHOD menu_settings_apply_2, menu_settings_apply, 2, HOOK_MENU_SETTINGS_APPLY_2_STOCK, HOOK_MENU_SETTINGS_APPLY_2_RESUME
SETTINGS_METHOD menu_settings_current_2, menu_settings_current, 2, HOOK_MENU_SETTINGS_CURRENT_2_STOCK, HOOK_MENU_SETTINGS_CURRENT_2_RESUME
.ltorg
SETTINGS_METHOD menu_settings_count_3, menu_settings_count, 3, HOOK_MENU_SETTINGS_COUNT_3_STOCK, HOOK_MENU_SETTINGS_COUNT_3_RESUME
SETTINGS_METHOD menu_settings_draw_3, menu_settings_draw, 3, HOOK_MENU_SETTINGS_DRAW_3_STOCK, HOOK_MENU_SETTINGS_DRAW_3_RESUME
SETTINGS_METHOD menu_settings_apply_3, menu_settings_apply, 3, HOOK_MENU_SETTINGS_APPLY_3_STOCK, HOOK_MENU_SETTINGS_APPLY_3_RESUME
SETTINGS_METHOD menu_settings_current_3, menu_settings_current, 3, HOOK_MENU_SETTINGS_CURRENT_3_STOCK, HOOK_MENU_SETTINGS_CURRENT_3_RESUME
.ltorg
SETTINGS_METHOD menu_settings_count_4, menu_settings_count, 4, HOOK_MENU_SETTINGS_COUNT_4_STOCK, HOOK_MENU_SETTINGS_COUNT_4_RESUME
SETTINGS_METHOD menu_settings_draw_4, menu_settings_draw, 4, HOOK_MENU_SETTINGS_DRAW_4_STOCK, HOOK_MENU_SETTINGS_DRAW_4_RESUME
SETTINGS_METHOD menu_settings_apply_4, menu_settings_apply, 4, HOOK_MENU_SETTINGS_APPLY_4_STOCK, HOOK_MENU_SETTINGS_APPLY_4_RESUME
SETTINGS_METHOD menu_settings_current_4, menu_settings_current, 4, HOOK_MENU_SETTINGS_CURRENT_4_STOCK, HOOK_MENU_SETTINGS_CURRENT_4_RESUME
.ltorg
