.section .rodata
.align 3

.global KERNEL_START
.global KERNEL_END
KERNEL_START:   .dword _kernel_start
KERNEL_END:     .dword _kernel_end
