/* A fast-boot file: the head (head.S), the vector table, and the installer
   with backend.c, linked at the top of the RTOS code region, which the
   firmware leaves unused; the GRKit core grows from the bottom. The head
   loads the bytes up to _edata, clears the rest up to _end, and calls the
   vector in its entry word, the last word of the head
   (runtime::FAST_ENTRY_OFFSET). */
ENTRY(fast_head)
SECTIONS {
  . = 0x54c00000;
  .head : { *(.head) }
  ASSERT(fast_status == 0x54c003f8, "The status word of the head moved")
  ASSERT(fast_entry == 0x54c003fc, "The entry word of the head moved")
  ASSERT(. <= 0x54c00400, "The head of the fast-boot file passes 1 KB")
  . = 0x54c00400;
  .vectors : { KEEP(*(.vectors)) }
  ASSERT(fast_scan == 0x54c00400, "The scan vector moved")
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
