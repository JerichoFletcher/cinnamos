OUTPUT_ARCH("riscv")
ENTRY(_start)

MEMORY {
    ram (wxa) : ORIGIN = 0x80200000, LENGTH = 128M
}

PHDRS {
    text PT_LOAD;
    data PT_LOAD;
    bss PT_LOAD;
}

SECTIONS {
    .text   : {
        PROVIDE(_text_start = .);
        *(.text.init)
        *(.text .text.*)
        PROVIDE(_text_end = .);
    } >ram AT>ram :text

    .rodata : {
        . = ALIGN(32);
        PROVIDE(_rodata_start = .);
        *(.rodata .rodata.*)
        PROVIDE(_rodata_end = .);
    } >ram AT>ram :text

    . = ALIGN(4096);
    .data   : {
        PROVIDE(_data_start = .);
        *(.sdata .sdata.*)
        *(.data .data.*)
        PROVIDE(_data_end = .);
    } >ram AT>ram :data

    .bss    : {
        PROVIDE(_bss_start = .);
        *(.sbss .sbss.*)
        *(.bss .bss.*)
        *(COMMON)
        PROVIDE(_bss_end = .);
    } >ram AT>ram :bss

    . = ALIGN(32);
    PROVIDE(_stack_start = .);
    .boot_stack (NOLOAD) : {
        PROVIDE(_boot_stack_start = .);
        . += 0x4000;
        PROVIDE(_boot_stack_top = .);
    }

    . = ALIGN(32);
    .trap_stack (NOLOAD) : {
        PROVIDE(_trap_stack_start = .);
        . += 0x4000;
        PROVIDE(_trap_stack_top = .);
    }
    PROVIDE(_stack_end = .);

    PROVIDE(_heap_start = _stack_end);
    PROVIDE(_heap_size = _mem_end - _heap_start);

    PROVIDE(_mem_start = ORIGIN(ram));
    PROVIDE(_mem_end = ORIGIN(ram) + LENGTH(ram));
}