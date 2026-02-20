ENTRY(_start)

PROVIDE(_vma_lma_offset = _kernel_vma - _kernel_lma);

SECTIONS {
    . = _kernel_lma;
    .boot   : ALIGN(4096) {
        PROVIDE(_kernel_phys_start = .);
        PROVIDE(_kernel_start = . + _vma_lma_offset);
        PROVIDE(_boot_start = .);
        *(.text.boot.init)
        *(.text.boot)
    }

    .boot.data : ALIGN(4096) {
        PROVIDE(_boot_data_start = .);
        *(.data.boot)
        PROVIDE(_boot_data_end = .);
    }

    .boot_stack (NOLOAD) : ALIGN(4096) {
        PROVIDE(_boot_stack_start = .);
         . += 0x4000;
         PROVIDE(_boot_stack_top = .);
         PROVIDE(_boot_end = .);
    }

    . = _boot_end + _vma_lma_offset;
    .text   : AT(_boot_end) ALIGN(4096) {
        PROVIDE(_text_start = .);
        *(.text.boot)
        *(.text .text.*)
        PROVIDE(_text_end = .);
    }

    .rodata : ALIGN(32) {
        PROVIDE(_rodata_start = .);
        *(.rodata .rodata.*)
        PROVIDE(_rodata_end = .);
    }

    . = ALIGN(4096);
    .data   : {
        PROVIDE(_data_start = .);
        *(.sdata .sdata.*)
        *(.data .data.*)
        PROVIDE(_data_end = .);
    }

    .bss    : {
        PROVIDE(_bss_start = .);
        *(.sbss .sbss.*)
        *(.bss .bss.*)
        *(COMMON)
        PROVIDE(_bss_end = .);
    }

    .kernel_stack (NOLOAD) : ALIGN(4096) {
        PROVIDE(_stack_start = .);
        PROVIDE(_kernel_stack_start = .);
        . += 0x4000;
        PROVIDE(_kernel_stack_top = .);
    }

    .trap_stack (NOLOAD) : ALIGN(32) {
        PROVIDE(_trap_stack_start = .);
        . += 0x4000;
        PROVIDE(_trap_stack_top = .);
        PROVIDE(_stack_end = .);
        PROVIDE(_kernel_end = .);
        PROVIDE(_kernel_phys_end = . - _vma_lma_offset);
    }
}