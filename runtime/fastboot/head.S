/* The head of a fast-boot file (SCRIPT/GRKIT/F<project><version>.BIN): its
   first 1 KB, then the vector table and the blob (fast.ld).

   The loader script reads the first 1 KB of the file into its first script
   array and calls it there with r0 = that array (SYS_memovr PATH 0 call).
   The head reads the whole file to its link address, clears the BSS, makes
   the code visible to instruction fetch, and calls the scan vector with
   r0 = the array. It does nothing if the file does not have the size of
   this build. The head runs from the script array, so it uses PC-relative
   addresses only, and absolute addresses from its literal pool.

   FRUN.BIN calls the install vector later (runtime::FAST_RUN). */
#include "target.h"
#include "fast.h"
#define FAST_BASE 0x54c00000

        .syntax unified
        .arm
        .section .head, "ax"
        .global fast_head
fast_head:
        push    {r4-r8, lr}
        sub     sp, sp, #72             @ [sp] the read length, [sp+8] the file status
        mov     r7, r0                  @ the script array
        adr     r0, name
        add     r1, sp, #8
        ldr     ip, =RT_FS_STAT_SIZE
        blx     ip
        cmp     r0, #0
        bne     fail
        ldr     r0, [sp, #16]           @ status word 2: the size
        ldr     r1, =(_edata - FAST_BASE)
        cmp     r0, r1
        bne     fail
        adr     r0, name
        mov     r1, #0                  @ read
        ldr     ip, =RT_BF_OPEN
        blx     ip
        movs    r4, r0
        beq     fail
        ldr     r1, =FAST_BASE
        ldr     r2, =(_edata - FAST_BASE)
        mov     r3, sp
        ldr     ip, =RT_BF_READ
        blx     ip
        mov     r5, r0                  @ 0 when the read succeeds
        mov     r0, r4
        ldr     ip, =RT_BF_CLOSE
        blx     ip
        cmp     r5, #0
        bne     fail
        ldr     r0, =_edata
        ldr     r1, =_end
        mov     r2, #0
1:      cmp     r0, r1
        strlo   r2, [r0], #4
        blo     1b
        ldr     r0, =FAST_BASE
        ldr     r1, =_end
2:      cmp     r0, r1
        mcrlo   p15, 0, r0, c7, c11, 1  @ DCCMVAU
        addlo   r0, r0, #32
        blo     2b
        dsb
        mov     r0, #0
        mcr     p15, 0, r0, c7, c1, 0   @ ICIALLUIS
        mcr     p15, 0, r0, c7, c1, 6   @ BPIALLIS
        dsb
        isb
        mov     r0, r7
        ldr     ip, =fast_scan
        blx     ip
        b       done
fail:
        adr     r0, failed
        ldr     ip, =RT_PRINTF
        blx     ip
done:
        add     sp, sp, #72
        pop     {r4-r8, pc}
        .ltorg
name:   .asciz FAST_PATH
failed: .asciz "GRKit: the fast-boot file is not complete\n"
        .balign 4

/* The vector table at FAST_BASE + 0x400: scan, then install. */
        .section .vectors, "ax"
        .global fast_scan, fast_install
fast_scan:
        b       rtos_fastboot_scan
fast_install:
        b       rtos_fastboot_install
