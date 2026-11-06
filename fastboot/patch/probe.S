/* SYS_buildinfo of a fast-boot firmware build: tell a script that the
   firmware has the loader of stub.S (docs/fastboot.md).

   usage: SYS_buildinfo INDEX
   INDEX  a script array of at least 4 words, by its number in the order of
          intdim (0 = first)

   The command writes 4 words to the array: MAGIC, the interface version,
   the project number of the camera (588 GR III, 609 GR IIIx, 636 GR IV; the
   number of its ModeSet file), and the firmware version (210 for 2.10). It
   does nothing for another index or a smaller array.

   The official command prints the build information and ignores its
   arguments, so on official firmware the array keeps its values. A script
   sets word 0 to 0, runs the command, and uses SYS_memovr only if word 0 is
   MAGIC. The build defines STRTOUL, TTL_TABLE, MAGIC, INTERFACE, PROJECT,
   and FIRMWARE (crates/grkit-firmware). */
        .syntax unified
        .arm
        .text
        .global probe
probe:
        push    {r4, lr}
        cmp     r0, #2
        blt     9f
        ldr     r0, [r1, #4]
        mov     r1, #0
        mov     r2, #0
        ldr     ip, =STRTOUL
        blx     ip
        cmp     r0, #256
        bhs     9f
        ldr     r3, =TTL_TABLE
        add     r3, r3, r0, lsl #3
        ldr     r2, [r3]
        cmp     r2, #4
        blo     9f
        ldr     r3, [r3, #4]
        cmp     r3, #0
        beq     9f
        ldr     r2, =MAGIC
        str     r2, [r3]
        mov     r2, #INTERFACE
        str     r2, [r3, #4]
        ldr     r2, =PROJECT
        str     r2, [r3, #8]
        ldr     r2, =FIRMWARE
        str     r2, [r3, #12]
9:
        mov     r0, #0
        pop     {r4, pc}
        .ltorg
