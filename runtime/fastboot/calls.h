#ifndef GRKIT_FASTBOOT_CALLS_H
#define GRKIT_FASTBOOT_CALLS_H
/* The calls of the RTOS backend (backend.c) beyond the Linux system calls,
   which installer.c makes only when it is built with RTOS_NATIVE. */

/* (path, u32 out[2]): 0 with the size and the stamp of the times of a card
   file, from the listing of its folder. -2 when the folder does not have
   the file, -38 when no listing with sizes exists. The stamp changes when
   the file is written. */
#define FAST_FILE_STAMP 0xf1001U

#endif
