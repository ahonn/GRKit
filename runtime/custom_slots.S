#include "firmware.h"
#include "abi.h"
#include "hooks.h"
.syntax unified
.arm
.text
/* CORE.CUSTOM_SLOTS (runtime/custom_slots.h). It links into the Framing
   region. */

#ifdef HOOK_CUSTOM_SLOTS_PROPERTY_SET
// The entry of the property setter of PropertyCommandProcessor: r0 = the
// processor context, r1 = the LinuxCommCommand. A request of GRKit returns
// as the setter does: 1 when done, 0 when refused, with the result in the
// command. Any other property runs the setter, after the displaced
// `mov ip, sp`.
.global custom_slots_property_set
custom_slots_property_set:
    push {r0-r3, r12, lr}
    bl custom_slots_property
    cmp r0, #0
    bne 1f
    pop {r0-r3, r12, lr}
    mov r12, sp
    ldr pc, =HOOK_CUSTOM_SLOTS_PROPERTY_SET_RESUME
1:  sub r0, r0, #1
    add sp, sp, #4
    pop {r1-r3, r12, pc}
.ltorg
#endif

#ifdef HOOK_CUSTOM_SLOTS_PROPERTY_GET
.global custom_slots_property_get
custom_slots_property_get:
    push {r0-r3, r12, lr}
    bl custom_slots_get
    cmp r0, #0
    bne 1f
    pop {r0-r3, r12, lr}
    mov r12, sp
    ldr pc, =HOOK_CUSTOM_SLOTS_PROPERTY_GET_RESUME
1:  sub r0, r0, #1
    add sp, sp, #4
    pop {r1-r3, r12, pc}
.ltorg
#endif
