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
        PROVIDE(_text_end = .);
    } > ram AT>ram :text

    .rodata : ALIGN(32) {
        *(.rodata .rodata.*)
    } > ram AT>ram :text

    . = ALIGN(4096);
    .data : ALIGN(4096) {
        *(.sdata .sdata.*)
        *(.data .data.*)
    } > ram AT>ram :data

    .bss : {
        PROVIDE(_bss_start = .);
        *(.sbss .sbss.*)
        *(.bss .bss.*)
        *(COMMON)
        PROVIDE(_bss_end = .);
    } > ram AT>ram :bss

    . = ALIGN(4096);
    PROVIDE(_stack_start = .);
    PROVIDE(_stack_end = _stack_start + 0x80000);

    PROVIDE(_trap_stack_start = .);
    PROVIDE(_trap_stack_end = _trap_stack_start + 0x80000);

    PROVIDE(_kernel_end = .);
}
