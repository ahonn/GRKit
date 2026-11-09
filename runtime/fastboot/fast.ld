/* A fast-boot file: the head (head.S), the vector table, and the installer
   with backend.c, linked at the top of the RTOS code region, which the
   firmware leaves unused; the GRKit core grows from the bottom. The head
   loads the bytes up to _edata, clears the rest up to _end, and calls the
   vector in its entry word, the last word of the head
   (runtime::FAST_ENTRY_OFFSET). The code up to _code_end does not change
   after the load: the head of the install file checks it. The data from
   _code_end to _data_end changes only in a call: INSTALL checks it with
   the sums in .fast_sums (backend.c), which must stay out of .data. Each
   INSTALL starts with the .data of the file and a clear .bss, as after a
   load: only .fast_keep (the RAM files and the copy of .data) lasts. */
ENTRY(fast_head)
SECTIONS {
  . = 0x54c00000;
  .head : { *(.head) }
  ASSERT(code_sums == 0x54c00200, "The code sums of the head moved")
  ASSERT(code_pages == 0x54c003d0, "The code pages word of the head moved")
  ASSERT(code_end == 0x54c003d8, "The code end word of the head moved")
  ASSERT(fast_status == 0x54c003f8, "The status word of the head moved")
  ASSERT(fast_entry == 0x54c003fc, "The entry word of the head moved")
  ASSERT(. <= 0x54c00400, "The head of the fast-boot file passes 1 KB")
  . = 0x54c00400;
  .vectors : { KEEP(*(.vectors)) }
  ASSERT(fast_scan == 0x54c00400, "The scan vector moved")
  ASSERT(fast_install == 0x54c00404, "The install vector moved")
  .text : { *(.text*) }
  .rodata : { *(.rodata*) }
  . = ALIGN(4);
  _code_end = .;
  .data : { *(.data*) }
  _edata = .;
  .bss (NOLOAD) : {
    *(.bss*) *(COMMON)
    . = ALIGN(4);
    _keep_start = .;
    *(.fast_keep)
    . = ALIGN(4);
    _data_end = .;
    *(.fast_sums)
    _sums_end = .;
  }
  ASSERT(_data_end > _keep_start, "The kept data is not after .bss")
  ASSERT(_sums_end > _data_end, "The data sums are in the data that they check")
  ASSERT(_edata - _code_end <= 0x14000, "The .data of the fast-boot file passes its copy")
  . = ALIGN(16);
  _end = .;
  ASSERT(_end <= 0x55000000, "The fast-boot file passes the end of the code region")
  /DISCARD/ : { *(.ARM.exidx*) *(.ARM.extab*) *(.ARM.attributes) *(.comment) *(.note*) }
}
