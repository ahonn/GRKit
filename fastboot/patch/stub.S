/* SYS_memovr of a fast-boot firmware build: load a card file into a script
   array, and optionally call it.

   usage: SYS_memovr PATH INDEX [call]
   PATH   a file on the card, such as C:\SCRIPT\APP\CODE.BIN (8.3 names)
   INDEX  a script array, by its number in the order of intdim (0 = first)
   call   any fourth argument: call the array with r0 = its address and
          r1 = the bytes read, in the script task; the call returns to the
          script

   The command reads at most the size of the array. It does nothing when the
   index is not that of an array, the file is empty or missing, or the read
   fails. After the read it cleans the data cache and invalidates the
   instruction cache over the bytes read.

   The official firmware has a memory-overrun test under this name, which
   nothing calls: a script must find this command with SYS_buildinfo
   (probe.S) before it uses it. The build defines the addresses of the
   firmware release: STRTOUL, TTL_TABLE (the script array table, two words
   for each array: the word count, the address), BF_STAT, BF_OPEN, BF_READ,
   and BF_CLOSE (crates/grkit-firmware). */
        .syntax unified
        .arm
        .text
        .global stub
stub:
        push    {r4-r9, lr}
        sub     sp, sp, #76             @ [sp] the read length, [sp+8] the file status (64 bytes); 8-byte aligned with the push
        mov     r4, r0                  @ argc
        mov     r5, r1                  @ argv
        cmp     r4, #3
        blt     done
        ldr     r0, [r5, #8]
        mov     r1, #0
        mov     r2, #0
        ldr     ip, =STRTOUL
        blx     ip
        cmp     r0, #256
        bhs     done
        ldr     r3, =TTL_TABLE
        add     r3, r3, r0, lsl #3
        ldr     r7, [r3]                @ words
        ldr     r6, [r3, #4]            @ address
        cmp     r6, #0
        beq     done
        lsls    r7, r7, #2              @ bytes
        beq     done
        ldr     r0, [r5, #4]
        add     r1, sp, #8
        ldr     ip, =BF_STAT            @ (name, status) -> 0; status word 2 = size
        blx     ip
        cmp     r0, #0
        bne     done
        ldr     r8, [sp, #16]
        cmp     r8, r7
        movhi   r8, r7
        cmp     r8, #0
        beq     done
        ldr     r0, [r5, #4]
        mov     r1, #0                  @ read
        ldr     ip, =BF_OPEN            @ (name, mode) -> fNo, 0 on failure
        blx     ip
        movs    r9, r0
        beq     done
        mov     r1, r6
        mov     r2, r8
        mov     r3, sp
        ldr     ip, =BF_READ            @ (fNo, address, count, &length) -> 0
        blx     ip
        mov     r7, r0
        mov     r0, r9
        ldr     ip, =BF_CLOSE
        blx     ip
        cmp     r7, #0
        bne     done
        bic     r0, r6, #31             @ from the cache line of the first byte
        add     r1, r6, r8
1:      cmp     r0, r1
        mcrlo   p15, 0, r0, c7, c11, 1  @ DCCMVAU
        addlo   r0, r0, #32
        blo     1b
        dsb
        mov     r0, #0
        mcr     p15, 0, r0, c7, c1, 0   @ ICIALLUIS
        mcr     p15, 0, r0, c7, c1, 6   @ BPIALLIS
        dsb
        isb
        cmp     r4, #4
        blt     done
        mov     r0, r6
        mov     r1, r8
        blx     r6
done:
        mov     r0, #0
        add     sp, sp, #76
        pop     {r4-r9, pc}
        .ltorg
