.section .rodata
.align 3

.global KERNEL_START
.global KERNEL_END
KERNEL_START:       .dword _kernel_start
KERNEL_END:         .dword _kernel_end

.global TEXT_START
.global TEXT_END
TEXT_START:         .dword _text_start
TEXT_END:           .dword _text_end

.global DATA_START
.global DATA_END
DATA_START:         .dword _data_start
DATA_END:           .dword _data_end

.global BSS_START
.global BSS_END
BSS_START:          .dword _bss_start
BSS_END:            .dword _bss_end

.global KMEM_START
.global KMEM_END
KMEM_START:         .dword _kmem_start
KMEM_END:           .dword _kmem_end

.global STACK_START
.global STACK_END
STACK_START:        .dword _stack_start
STACK_END:          .dword _stack_end

.global TRAP_STACK_START
.global TRAP_STACK_END
TRAP_STACK_START:   .dword _trap_stack_start
TRAP_STACK_END:     .dword _trap_stack_end
