/* Selects the firmware definitions of the build target. The build defines
   GRKIT_TARGET_GR4_111 for GR IV 1.11. Host tests use the GR III 2.10 layout. */
#if defined(GRKIT_TARGET_GR4_111)
#include "targets/gr4-1.11.h"
#else
#include "targets/gr3-2.10.h"
#endif

#ifndef FRAMING_RATIO_COUNT
/* Framing ratios: Off, XPan, 16:9, 4:3. A target can offer only the first ones. */
#define FRAMING_RATIO_COUNT 4U
#endif
