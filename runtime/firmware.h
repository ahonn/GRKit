/* Selects the firmware definitions of the build target. The build and the host
   tests define GRKIT_TARGET_<target> (components::Layout::define). A new target
   must add its header here. The build header includes the header of its
   family, runtime/targets/<family>.h. */

/* A firmware address. The installer finds the address
   in the running build of the firmware version and moves each reference
   (crates/grkit-build/src/relocation.rs). The symbol fw_<address> has the
   address of the pinned build. In C the value is opaque, so the compiler keeps
   it in one literal word instead of a direct call or a derived constant. */
#if defined(GRKIT_HOST_ADDRESSES)
/* Host contract tests link the code without the firmware: plain addresses. */
#define FIRMWARE(address) ((unsigned int)(address))
#elif defined(__ASSEMBLER__)
#define FIRMWARE(address) fw_##address
#else
#define FIRMWARE(address) ({ extern char fw_##address[]; unsigned int firmware_; \
    __asm__("" : "=r"(firmware_) : "0"((unsigned int)(__UINTPTR_TYPE__)fw_##address)); firmware_; })
#endif
#if defined(GRKIT_TARGET_GR3_210)
#include "targets/gr3-2.10.h"
#elif defined(GRKIT_TARGET_GR3_221)
#include "targets/gr3-2.21.h"
#elif defined(GRKIT_TARGET_GR4_111)
#include "targets/gr4-1.11.h"
#elif defined(GRKIT_TARGET_GR4_120)
#include "targets/gr4-1.20.h"
#elif defined(GRKIT_TARGET_GR4_120_0528)
#include "targets/gr4-1.20-0528.h"
#elif defined(GRKIT_TARGET_GR4_121)
#include "targets/gr4-1.21.h"
#else
#error "Define GRKIT_TARGET_<target> to select runtime/targets/<target>.h"
#endif

/* Values that each family header gives the shared code. FRAMING_RATIO_ROWS:
   Off, then the Framing ratio value (GRKIT_HOST_FRAME_RATIO) of each row, in
   menu order. The values are 1 XPan, 2 16:9, 3 4:3, 4 2.35:1, 5 1:1, 6 5:4,
   and 7 7:6. A value keeps its meaning across releases, because the camera saves
   it, so a new ratio takes the next value. Every family lists each ratio,
   and Framing crops each one from the native 3:2 (runtime/aspect.h). */
#if !defined(GRKIT_ICON_STRIDE) || !defined(FRAMING_RATIO_ROWS)
#error "The family header defines GRKIT_ICON_STRIDE and FRAMING_RATIO_ROWS"
#endif
