/* A fast-boot file: the head (head.S), the vector table, and the installer
   with backend.c, linked at the top of the RTOS code region, which the
   firmware leaves unused; the GRKit core grows from the bottom. The head
   loads the bytes up to _edata and clears the rest up to _end. FRUN.BIN
   calls fast_install at its fixed address (runtime::FAST_RUN). */
ENTRY(fast_head)
SECTIONS {
  . = 0x54c00000;
  .head : { *(.head) }
  ASSERT(. <= 0x54c00400, "The head of the fast-boot file passes 1 KB")
  . = 0x54c00400;
  .vectors : { KEEP(*(.vectors)) }
  ASSERT(fast_install == 0x54c00404, "The install vector moved")
  .text : { *(.text*) }
  .rodata : { *(.rodata*) }
  .data : { *(.data*) }
  _edata = .;
  .bss (NOLOAD) : { *(.bss*) *(COMMON) }
  . = ALIGN(16);
  _end = .;
  ASSERT(_end <= 0x55000000, "The fast-boot file passes the end of the code region")
  /DISCARD/ : { *(.ARM.exidx*) *(.ARM.extab*) *(.ARM.attributes) *(.comment) *(.note*) }
}
