OUTPUT_ARCH("riscv")
ENTRY(_start)

PHDRS {
    text    PT_LOAD     FLAGS(5);
    rodata  PT_LOAD     FLAGS(4);
    data    PT_LOAD     FLAGS(6);
    dynamic PT_DYNAMIC  FLAGS(4);
}

SECTIONS {
    . = 0xffffc00000000000;
    PROVIDE(_kernel_start = .);
    
    PROVIDE(_text_start = .);
    .text : AT(0x80200000) {
        *(.text.init)
        *(.text .text.*)
    }
    PROVIDE(_text_end = .);

    . = ALIGN(4K);
    PROVIDE(_rodata_start = .);
    .rodata : {
        *(.rodata .rodata.*)
    }
    .dynamic : { *(.dynamic) } :data :dynamic
    .dynsym : { *(.dynsym) }
    .dynstr : { *(.dynstr) }
    .rela.dyn : { *(.rela.dyn) }
    .gnu.hash : { *(.gnu.hash) }
    .hash : { *(.hash) }
    PROVIDE(_rodata_end = .);

    . = ALIGN(4K);
    PROVIDE(_data_start = .);
    .data : {
        *(.sdata .sdata.*)
        *(.data .data.*)
    }
    .got : { *(.got .got.*) }
    .bss : {
        PROVIDE(_bss_start = .);
        *(.sbss .sbss.*)
        *(.bss .bss.*)
        *(COMMON)
        PROVIDE(_bss_end = .);
    }
    PROVIDE(_data_end = .);

    . = ALIGN(4K);
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
