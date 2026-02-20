OUTPUT_ARCH("riscv")

PROVIDE(_kernel_vma = 0xC0000000);
PROVIDE(_kernel_lma = 0x80200000);

INCLUDE crates/cinnamos-kernel/src/ld/common.ld
