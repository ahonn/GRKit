#ifndef GRKIT_GRF_H
#define GRKIT_GRF_H

#define GRKIT_GRF_SIZE 8192U

/* Return one only for a complete, supported GRFX0001 record. */
int grkit_grf_validate(const unsigned char *data, unsigned long size);

#endif
