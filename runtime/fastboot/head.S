/* The head of a fast-boot file (SCRIPT/GRKIT/F<project><version>.BIN): its
   first 1 KB, then the vector table and the blob (fast.ld).

   The loader script reads the first 1 KB of the file into its first script
   array and calls it there with r0 = that array (SYS_memovr PATH 0 call).
   The head reads the whole file to its link address, clears the BSS, makes
   the code visible to instruction fetch, and calls the vector in its entry
   word with r0 = the array. It does nothing if the file does not have the
   size of this build. The head runs from the script array, so it uses
   PC-relative addresses only, and absolute addresses from its literal pool.

   The entry word is the last word of the head (runtime::FAST_ENTRY_OFFSET).
   In this file it is the scan vector. The install file of the release
   (I<project><version>.BIN, runtime::install_file) is this head with the
   install vector there and with the sum of each code page: that head does
   not load the file again. It compares each code page in memory with its
   sum and calls INSTALL in the loaded file only if no page changed after
   SCAN. Otherwise it writes the address of the first changed page and the
   number of changed pages, and the status 997 (runtime::FAST_DETAIL_OFFSET,
   FAST_STATUS_CODE_CHANGED). */
#include "target.h"
#include "fast.h"
/* The build defines FAST_BASE (runtime::FAST_BASE), the address of fast.ld. */

        .syntax unified
        .arm
        .section .head, "ax"
        .global fast_head
fast_head:
        push    {r4-r8, lr}
        sub     sp, sp, #72             @ [sp] the read length, [sp+8] the file status
        mov     r7, r0                  @ the script array
        ldr     r0, code_pages
        cmp     r0, #0
        bne     resident
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
call:
        mov     r0, r7
        ldr     ip, fast_entry
        blx     ip
        b       done
@ The install file: the code is in memory from SCAN. Compare each code page,
@ from the scan vector to the end of the code, with its sum.
resident:
        ldr     r4, =fast_scan          @ the start of a page
        ldr     r5, code_end
        adr     r8, code_sums
        mov     r3, #0                  @ the changed pages
        mov     r6, #0                  @ the first changed page, 0 = none
1:      cmp     r4, r5
        bhs     3f
        mov     lr, r4
        ldr     r1, code_shift
        mov     r2, #1
        add     r2, r4, r2, lsl r1      @ the end of the page
        cmp     r2, r5
        movhi   r2, r5
        mov     r0, #0
2:      ldr     r1, [r4], #4
        add     r0, r1, r0, ror #27     @ runtime::fastboot page_sum
        cmp     r4, r2
        blo     2b
        ldr     r1, [r8], #4
        cmp     r0, r1
        beq     1b
        add     r3, r3, #1
        cmp     r6, #0
        moveq   r6, lr
        b       1b
3:      cmp     r3, #0
        beq     call
        str     r6, [r7, #0x3f0]        @ runtime::FAST_DETAIL_OFFSET
        str     r3, [r7, #0x3f4]
        ldr     r0, =997                @ runtime::FAST_STATUS_CODE_CHANGED
        str     r0, [r7, #0x3f8]
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
/* The sum of each code page (runtime::FAST_SUMS_OFFSET), the number of
   pages, 0 in this file, which loads the file, and the page size as a
   shift: runtime::install_file writes them into the install file. */
        .org    0x200
        .global code_sums
code_sums:
        .space  116 * 4
        .global code_pages
code_pages:
        .word   0
code_shift:
        .word   0
/* The end of the code, which does not change after the load (fast.ld). */
        .global code_end
code_end:
        .word   _code_end
/* The detail words of a changed status: the address of the first changed
   page or block, and the number of changed ones (runtime::FAST_DETAIL_OFFSET). */
        .org    0x3f0
        .word   0
        .word   0
/* The status word: SCAN or INSTALL writes its status here, in the script
   array, for the loader (runtime::FAST_STATUS_OFFSET). It keeps this value
   when the head does not call the vector. */
        .org    0x3f8
        .global fast_status
fast_status:
        .word   998
/* The vector that the head calls: scan here, install in the install file. */
        .org    0x3fc
        .global fast_entry
fast_entry:
        .word   fast_scan

/* The vector table at FAST_BASE + 0x400: scan, then install. */
        .section .vectors, "ax"
        .global fast_scan, fast_install
fast_scan:
        b       rtos_fastboot_scan
fast_install:
        b       rtos_fastboot_install
