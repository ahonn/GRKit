/* Selects the firmware definitions of the build target. The build and the host
   tests define GRKIT_TARGET_<target> (components::Layout::define). A new target
   must add its header here. */
#if defined(GRKIT_TARGET_GR3_210)
#include "targets/gr3-2.10.h"
#elif defined(GRKIT_TARGET_GR4_111)
#include "targets/gr4-1.11.h"
#else
#error "Define GRKIT_TARGET_<target> to select runtime/targets/<target>.h"
#endif

#ifndef FRAMING_RATIO_COUNT
/* Framing ratios: Off, XPan, 16:9, 4:3. A target can offer only the first ones. */
#define FRAMING_RATIO_COUNT 4U
#endif
