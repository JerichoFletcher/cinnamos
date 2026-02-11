ENTRY(_start)

SECTIONS {
    . = 0x80200000;
    .text   : {
        *(.text.entry)
        *(.text*)
    }

    .rodata : {
        *(.rodata*)
    }

    .data   : {
        *(.data*)
    }

    .bss    : {
        __bss_start = .;
        *(.bss*)
        *(.sbss*)
        *(COMMON)
        __bss_end = .;
    }

    . = ALIGN(16);
    __stack_top = . + 0x4000;
}