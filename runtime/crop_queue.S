/* The queue completion of a crop shot on GR IV; runtime/framing.S includes
   this file. The back end posts the completion of its queue (the event
   CROP_QUEUE_EVENT, which lets the camera take the next action) when the
   processing of a shot is done and the shutter release has stopped. When
   the release stops after the processing, the release stop checks the
   completion again, but only in continuous and interval drive. A crop shot
   of a Single drive can finish its processing first, so the completion is
   not posted and the camera waits.

   Three hooks give the release stop of a crop shot the same check, once:
   crop_queue_start notes the shot, crop_queue_release runs the completion
   check of the back end when the camera would skip it, and crop_queue_post
   lets one completion through for that shot. The header words
   GRKIT_HOST_CROP_QUEUE_OWNER and GRKIT_HOST_CROP_QUEUE_FLAGS hold the
   state of the last shot. */
#define CROP_QUEUE_POSTED 1
#define CROP_QUEUE_RECOVERED 2
#if defined(HOOK_CROP_QUEUE_START) != defined(HOOK_CROP_QUEUE_RELEASE) \
    || defined(HOOK_CROP_QUEUE_START) != defined(HOOK_CROP_QUEUE_POST) \
    || defined(HOOK_CROP_QUEUE_START) != defined(FW_QUEUE_COMPLETE)
#error "The crop_queue_* hooks and the FW_QUEUE_* functions go together"
#endif
#ifdef HOOK_CROP_QUEUE_START
.arm
/* Replaces `strb r3, [r5, #CROP_BACKEND_RELEASED]` (r3 is 0) at the
   release start of the back end (r5): a new shot, whose events go to the
   owner of the back end. */
.global crop_queue_start
.type crop_queue_start, %function
crop_queue_start:
    push {r0,r1}
    ldr r0, =GRKIT_HEADER_REFERENCE
    ldr r1, [r5, #CROP_BACKEND_OWNER]
    str r1, [r0, #GRKIT_HOST_CROP_QUEUE_OWNER]
    mov r1, #0
    str r1, [r0, #GRKIT_HOST_CROP_QUEUE_FLAGS]
    dmb ish
    pop {r0,r1}
    strb r3, [r5, #CROP_BACKEND_RELEASED]
    ldr pc, =HOOK_CROP_QUEUE_START_RESUME
.ltorg

/* Replaces `mov r12, sp` at the entry of the event poster (r0 the owner,
   r1 the event). The queue completion of the last shot sets
   CROP_QUEUE_POSTED. After crop_queue_release, only the first completion
   of the shot goes out: a later one returns without a post. */
.global crop_queue_post
.type crop_queue_post, %function
crop_queue_post:
    cmp r1, #CROP_QUEUE_EVENT
    bne 3f
    push {r2-r5}
    ldr r2, =GRKIT_HEADER_REFERENCE
    ldr r3, [r2, #GRKIT_HOST_CROP_QUEUE_OWNER]
    cmp r0, r3
    bne 2f
    add r2, r2, #GRKIT_HOST_CROP_QUEUE_FLAGS
1:  ldrex r3, [r2]
    orr r4, r3, #CROP_QUEUE_POSTED
    strex r5, r4, [r2]
    cmp r5, #0
    bne 1b
    dmb ish
    and r3, r3, #(CROP_QUEUE_POSTED | CROP_QUEUE_RECOVERED)
    cmp r3, #(CROP_QUEUE_POSTED | CROP_QUEUE_RECOVERED)
    pop {r2-r5}
    bxeq lr
    b 3f
2:  pop {r2-r5}
3:  mov r12, sp
    ldr pc, =HOOK_CROP_QUEUE_POST_RESUME
.ltorg

/* Replaces `mov r0, r4` after the drive-mode check of the release stop of
   the back end (r4), where the camera does not check the completion; r0 is
   the drive mode. For a Single shot of a GRKit crop whose completion is not
   posted, check the completion as the continuous drive does, when the queue
   has reached the completion of the shot: no recipe in state 1 or 2, and
   no recipe in state 3, or one that is at or after the completion step of
   its table. CROP_QUEUE_RECOVERED marks the check, at most once a shot. */
.global crop_queue_release
.type crop_queue_release, %function
crop_queue_release:
    push {r0-r7,r12,lr}
    cmp r0, #0
    bne 9f
    ldr r5, =GRKIT_HEADER_REFERENCE
    ldr r0, [r5, #GRKIT_HOST_CROP_NATIVE]
    cmp r0, #0
    beq 9f
    ldrb r0, [r4, #CROP_BACKEND_RELEASED]
    cmp r0, #1
    bne 9f
    ldrb r0, [r4, #CROP_BACKEND_HELD]
    cmp r0, #0
    bne 9f
    ldr r0, [r5, #GRKIT_HOST_CROP_QUEUE_OWNER]
    ldr r1, [r4, #CROP_BACKEND_OWNER]
    cmp r0, r1
    bne 9f
    ldr r0, [r5, #GRKIT_HOST_CROP_QUEUE_FLAGS]
    tst r0, #(CROP_QUEUE_POSTED | CROP_QUEUE_RECOVERED)
    bne 9f
    ldr r6, [r4, #CROP_BACKEND_QUEUE]
    mov r0, r6
    ldr r12, =FW_QUEUE_COUNT1
    blx r12
    cmp r0, #0
    bne 9f
    mov r0, r6
    ldr r12, =FW_QUEUE_COUNT2
    blx r12
    cmp r0, #0
    bne 9f
    mov r0, r6
    ldr r12, =FW_QUEUE_COUNT3
    blx r12
    cmp r0, #0
    beq 5f
    cmp r0, #1
    bne 9f
    /* The recipe in state 3: that of the shot, or the one in the queue. */
    ldr r7, [r4, #CROP_BACKEND_RECIPE]
    cmp r7, #0
    bne 1f
    sub sp, sp, #8
    mov r1, sp
    mov r2, #2
    mov r0, r6
    ldr r12, =FW_QUEUE_RECIPES
    blx r12
    ldr r7, [sp]
    add sp, sp, #8
    cmp r7, #0
    beq 9f
1:  ldr r0, [r7, #CROP_RECIPE_STATE]
    cmp r0, #3
    bne 9f
    ldr r12, =FW_RECIPE_BANK
    blx r12
    ldr r1, [r7, #CROP_RECIPE_TABLE]
    ldr r12, =FW_RECIPE_TABLE
    blx r12
    /* The first step with opcode 1 and argument 8, before opcode 12. */
    ldr r1, [r7, #CROP_RECIPE_PROGRESS]
    mov r2, #0
    add r12, r0, #CROP_TABLE_ARGS
2:  add r3, r0, r2, lsl #2
    ldr r3, [r3, #CROP_TABLE_OPS]
    cmp r3, #12
    beq 9f
    cmp r3, #1
    bne 3f
    add lr, r12, r2, lsl #1
    ldrh lr, [lr]
    cmp lr, #8
    bne 3f
    cmp r2, #0
    beq 9f
    cmp r1, r2
    blt 9f
    b 5f
3:  add r2, r2, #1
    cmp r2, #CROP_TABLE_STEPS
    blo 2b
    b 9f
    /* Mark the check unless a completion came first, then check. */
5:  add r1, r5, #GRKIT_HOST_CROP_QUEUE_FLAGS
6:  ldrex r0, [r1]
    tst r0, #(CROP_QUEUE_POSTED | CROP_QUEUE_RECOVERED)
    bne 8f
    orr r0, r0, #CROP_QUEUE_RECOVERED
    strex r2, r0, [r1]
    cmp r2, #0
    bne 6b
    dmb ish
    mov r0, r4
    ldr r12, =FW_QUEUE_COMPLETE
    blx r12
    b 9f
8:  clrex
9:  pop {r0-r7,r12,lr}
    mov r0, r4
    ldr pc, =HOOK_CROP_QUEUE_RELEASE_RESUME
.ltorg
#endif
