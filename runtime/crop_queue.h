#ifndef GRKIT_CROP_QUEUE_H
#define GRKIT_CROP_QUEUE_H
/* The state of the last shot of the GR IV back end in
   GRKIT_HOST_CROP_QUEUE_FLAGS: runtime/crop_queue.S writes it, and
   runtime/crop.h reads it (crop_busy). Assembly and C include this file. */
#define CROP_QUEUE_POSTED 1
#define CROP_QUEUE_RECOVERED 2
#define CROP_QUEUE_ARMED 4
/* The back end started the shot; it makes the shot until its queue
   completion (CROP_QUEUE_POSTED). */
#define CROP_QUEUE_STARTED 8
/* The high half holds the time of the start, in units of 2^25 counts of the
   generic timer (about 0.84 s at 40 MHz). */
#define CROP_QUEUE_TIME_SHIFT 16
#endif
