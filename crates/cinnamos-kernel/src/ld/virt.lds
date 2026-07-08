OUTPUT_ARCH("riscv")
ENTRY(_start)

MEMORY {
    ram (wxa) : ORIGIN = 0x80200000, LENGTH = 128M
}

SECTIONS {
    PROVIDE(_kernel_start = ORIGIN(ram));
    
    .text : ALIGN(4096) {
        PROVIDE(_text_start = .);
        *(.text.init)
        *(.text .text.*)
    } > ram AT>ram :text

    .rodata : ALIGN(32) {
        *(.rodata .rodata.*)
        PROVIDE(_text_end = .);
    } > ram AT>ram :text

    .data : ALIGN(4096) {
        PROVIDE(_data_start = .);
        *(.sdata .sdata.*)
        *(.data .data.*)
        PROVIDE(_data_end = .);
    } > ram AT>ram :data

    .bss : ALIGN(4096) {
        PROVIDE(_bss_start = .);
        *(.sbss .sbss.*)
        *(.bss .bss.*)
        *(COMMON)
        PROVIDE(_bss_end = .);
    } > ram AT>ram :bss

    . = ALIGN(4096);
    PROVIDE(_kmem_start = .);
    PROVIDE(_stack_start = .);
    . += 0x80000;
    PROVIDE(_stack_end = .);

    PROVIDE(_trap_stack_start = .);
    . += 0x80000;
    PROVIDE(_trap_stack_end = .);
    PROVIDE(_kmem_end = .);

    PROVIDE(_kernel_end = .);
}
