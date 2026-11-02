#include "firmware.h"
#include "abi.h"
#include "core.h"
/* CORE.CUSTOM_SLOTS (runtime/custom_slots.h) on the families that have it.
   The hook of the property setter (custom_slots.S) calls
   custom_slots_property in the task of the Linux property commands. */
#ifdef CUSTOM_SLOT_FIELDS
#define CUSTOM_SLOTS_SETTINGS() (((u32 (*)(void))FW_SETTINGS)())
#define CUSTOM_SLOTS_RELEASE(property) (((void (*)(u32))FW_LINUX_RELEASE)(property))
#define CUSTOM_SLOTS_RESULT(command, n, value) (((void (*)(u32, u32, u32))FW_LINUX_SET_RESULT)(command, n, value))

/* Select set n with its type through the App task, as the native effect
   property does. The call waits for the App task. Return 1 when it did. */
static u32 custom_slots_select(u32 ctx, u32 n, u32 type)
{
    u32 command[CUSTOM_SLOT_COMMAND_SIZE / 4];
    for (u32 i = 0; i < CUSTOM_SLOT_COMMAND_SIZE / 4; ++i) command[i] = 0;
    command[1] = CUSTOM_SLOT_COMMAND_SELECT;
    command[2] = n;
    command[3] = type;
    return ((u32 (*)(u32, u32))FW_APP_COMMAND)(ctx, (u32)command) == 1;
}
#define CUSTOM_SLOTS_SELECT custom_slots_select
#include "custom_slots.h"

extern volatile u32 *framing_header(void);

/* CNTPCT is shared across cores. CNTFRQ is zero on this firmware, so use
   the target's verified counter frequency. */
static void custom_transport_now(u32 *low, u32 *high)
{
    __asm__ volatile("mrrc p15, 0, %0, %1, c14" : "=r"(*low), "=r"(*high));
}
#define CUSTOM_TRANSPORT_STATE (*(CustomTransportState *)((u8 *)framing_header() + GRKIT_HOST_CUSTOM_TRANSPORT))
#define CUSTOM_TRANSPORT_ALLOC(bytes) (((u32 (*)(u32, u32))FW_LINUX_ALLOC)((bytes), 1))
#define CUSTOM_TRANSPORT_NOW(low, high) custom_transport_now(&(low), &(high))
#define CUSTOM_TRANSPORT_EPOCH(h) ((h)[GRKIT_HOST_CUSTOM_EPOCH / 4])
#define CUSTOM_TRANSPORT_TIMEOUT_TICKS (30U * SYSTEM_COUNTER_MHZ * 1000000U)
#include "custom_transport.h"
_Static_assert(sizeof(CustomTransportState) == GRKIT_HOST_CUSTOM_TRANSPORT_WORDS * 4U, "Custom transport fills its header field");

/* r0 and r1 of the property setter. 0: not a request of GRKit. */
u32 custom_slots_property(u32 ctx, u32 command)
{
    return custom_transport_set(framing_header(), ctx, command);
}

u32 custom_slots_get(u32 ctx, u32 command)
{
    return custom_transport_get(framing_header(), ctx, command);
}
#endif
